// mv3_stub.js - the stubbed MV3 world that every `background.js` test loads the
// shipped worker into.
//
// It exists as one module rather than one copy per test because the stub is a
// model of the CHROME SURFACE the worker touches, and that surface grows: the
// connected indicator added `chrome.action.setTitle`, `setBadgeTextColor` and
// `chrome.tabs.query` to it. Three private copies would mean a worker that
// loads under one test and throws under another, which is a test failure that
// says nothing about the worker.
//
// It is a STUB, not Chrome. It models only what the worker touches, and only
// the properties a test depends on:
//
//   * `chrome.storage.local` get/set are ASYNCHRONOUS (the property the outbox
//     race depends on - two deliveries in one tick can interleave across the
//     await),
//   * `chrome.action.*` is observable, so a badge assertion can read the last
//     text/colour/title actually painted,
//   * `setTimeout` is recorded AND unref'd, so a test can assert the health
//     poll's cadence, fire it by hand, and still let node exit,
//   * `fetch` is whatever the test says it is - that is how "the relay is down"
//     and "the relay came back" are expressed.
//
// A green run here is evidence about this LOGIC. It is not proof the extension
// works in Chrome; that is what the first live capture is for.

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const DEFAULT_TARGET = path.join(__dirname, "..", "extension", "background.js");

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

/**
 * Load `background.js` into a fresh stubbed MV3 world.
 *
 * Options:
 *   target       - path to the worker (default: the shipped one)
 *   relay        - "down" (default) | "up" | a function (url, options) => Response-ish
 *   store        - initial `chrome.storage.local` contents
 *   asyncStorage - default true; false makes get/set resolve without a tick
 *   tabs         - what `chrome.tabs.query` answers (default: one BGA tab)
 *   quiet        - default true; silences the worker's (correct, noisy) logging
 */
function loadWorker(options = {}) {
  const target = options.target || DEFAULT_TARGET;
  const source = fs.readFileSync(target, "utf8");
  const asyncStorage = options.asyncStorage !== false;

  const store = { ...(options.store || {}) };
  const storage = {
    async get(keys) {
      if (asyncStorage) await tick(); // where the interleave happens
      const wanted = Array.isArray(keys) ? keys : [keys];
      const out = {};
      for (const key of wanted) out[key] = store[key];
      return out;
    },
    async set(object) {
      if (asyncStorage) await tick();
      Object.assign(store, object);
    },
  };

  // --- the relay, as the test wants it -------------------------------------
  const requests = [];
  let relay = options.relay || "down";
  const setRelay = (next) => {
    relay = next;
  };
  const fetchStub = (url, init) => {
    requests.push({ url: String(url), method: (init && init.method) || "GET", body: init && init.body });
    if (typeof relay === "function") return Promise.resolve(relay(String(url), init));
    if (relay === "up") return Promise.resolve({ ok: true, status: 200, text: async () => "" });
    return Promise.reject(new Error("ECONNREFUSED (relay not running)"));
  };

  // --- the toolbar action, observable --------------------------------------
  const badge = { text: null, color: null, textColor: null, title: null };
  const action = {
    setBadgeBackgroundColor: async ({ color }) => {
      badge.color = color;
    },
    setBadgeText: async ({ text }) => {
      badge.text = text;
    },
    setBadgeTextColor: async ({ color }) => {
      badge.textColor = color;
    },
    setTitle: async ({ title }) => {
      badge.title = title;
    },
    onClicked: { addListener: (fn) => listeners.clicked.push(fn) },
  };

  // --- timers, recorded and unref'd ----------------------------------------
  const timers = [];
  const stubSetTimeout = (fn, ms) => {
    const handle = setTimeout(fn, ms);
    if (handle && typeof handle.unref === "function") handle.unref();
    timers.push({ ms, fn, handle });
    return handle;
  };

  const listeners = { message: [], startup: [], installed: [], clicked: [] };
  const tabMessages = [];
  let tabsAnswer =
    options.tabs === undefined ? [{ id: 1, url: "https://boardgamearena.com/8/yokohama?table=895724349" }] : options.tabs;

  const chrome = {
    storage: { local: storage },
    runtime: {
      onMessage: { addListener: (fn) => listeners.message.push(fn) },
      onStartup: { addListener: (fn) => listeners.startup.push(fn) },
      onInstalled: { addListener: (fn) => listeners.installed.push(fn) },
    },
    action,
    tabs: {
      async query() {
        return tabsAnswer;
      },
      async sendMessage(tabId, message, opts) {
        tabMessages.push({ tabId, message, opts });
        if (options.tabSendFails) throw new Error("Could not establish connection.");
      },
    },
  };

  const quiet = options.quiet !== false;
  const context = {
    chrome,
    fetch: fetchStub,
    console: quiet ? { ...console, error: () => {}, warn: () => {}, info: () => {}, debug: () => {} } : console,
    setTimeout: stubSetTimeout,
    clearTimeout,
    AbortController,
    Promise,
    JSON,
    String,
    Number,
    Object,
    Array,
    Math,
    Date,
    RegExp,
  };
  context.globalThis = context;

  // A FRESH context per load, not `runInThisContext`: the worker declares its
  // constants at top level, so a second load into a shared global would fail
  // with "Identifier 'RELAY_ORIGIN' has already been declared". A fresh context
  // is also exactly what a service-worker RECYCLE looks like - same
  // `chrome.storage.local`, brand new module scope - which is how the tests
  // check that counters survive one.
  vm.createContext(context);
  vm.runInContext(source, context, { filename: target });

  /** Hand the worker a runtime message and resolve with what it answered. */
  const send = (message, sender) =>
    new Promise((resolve) => {
      let answered = false;
      const reply = (response) => {
        answered = true;
        resolve(response);
      };
      let held = false;
      for (const listener of listeners.message) {
        if (listener(message, sender || { tab: { id: 1 } }, reply) === true) held = true;
      }
      // A listener that neither answered nor asked to hold the port is a
      // message this worker does not handle; do not hang the test on it.
      if (!held && !answered) resolve(undefined);
    });

  return {
    target,
    context,
    store,
    badge,
    timers,
    requests,
    tabMessages,
    listeners,
    send,
    setRelay,
    setTabs: (next) => {
      tabsAnswer = next;
    },
    /** Fire the recorded timer scheduled at exactly `ms`, most recent first. */
    fireTimer(ms) {
      for (let i = timers.length - 1; i >= 0; i -= 1) {
        if (timers[i].ms === ms) {
          clearTimeout(timers[i].handle);
          timers[i].fn();
          return true;
        }
      }
      return false;
    },
  };
}

/** Let every pending microtask and zero-delay timer drain. */
const settle = (ms = 300) => new Promise((resolve) => setTimeout(resolve, ms));

/** The tiny check harness the bridge tests share. */
function reporter(title) {
  let failures = 0;
  console.log(title);
  return {
    check(name, ok, detail) {
      if (!ok) failures += 1;
      console.log(`  [${ok ? "PASS" : "FAIL"}] ${name}${detail ? "  - " + detail : ""}`);
      return ok;
    },
    done() {
      console.log(`\n${failures === 0 ? "PASS" : "FAIL"} - ${failures} failing check(s)`);
      process.exit(failures === 0 ? 0 : 1);
    },
  };
}

module.exports = { loadWorker, settle, reporter, DEFAULT_TARGET };
