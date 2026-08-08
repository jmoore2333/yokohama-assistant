// The capture wrapper's two obligations, tested under a stubbed page.
//
//   node bridge/tests/wrapper_transparency.js [path-to-main-world.js]
//
// Exits 0 on pass, 1 on failure. No dependencies - plain `node`.
//
// 1. TRANSPARENCY. `main-world.js` replaces the page's own `fetch` and
//    `XMLHttpRequest`. The page must not be able to tell. That means the same
//    Response object comes back, a rejection rejects with the SAME error, the
//    page's own body read still works (we clone, never consume), and a failure
//    anywhere on OUR side is swallowed rather than surfaced to the page. This
//    is the property that makes the extension safe to leave running on a live
//    game: at worst we lose a capture, never the game.
//
// 2. COMPLETENESS. The two H0-review findings this file was written for:
//      S-6 - a `fetch(new URL(...))` argument was missed entirely, because a
//            URL instance has `.href`, not `.url`.
//      S-1 - the XHR load listener was registered per `send()`, so a REUSED
//            XHR object (which the API permits, and BGA's transport does)
//            captured its second response twice and its third three times.
//
// The environment below is a stub, not a browser. It models the parts the
// wrapper actually touches: a one-shot response body, and an XHR whose listener
// list is observable.

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const target =
  process.argv[2] || path.join(__dirname, "..", "extension", "main-world.js");

const CAPTURED_URL = "https://boardgamearena.com/4/yokohama/yokohama/notificationHistory.html?table=1";
const UNMATCHED_URL = "https://boardgamearena.com/somethingelse";

const posted = [];

/** A Response with the real one-shot body rule, and optional sabotage. */
class FakeResponse {
  constructor(body, { failClone = false, failText = false } = {}) {
    this._body = body;
    this._failClone = failClone;
    this._failText = failText;
    this._read = false;
    this.status = 200;
    this.url = CAPTURED_URL;
  }
  clone() {
    if (this._failClone) throw new Error("clone() blew up");
    return new FakeResponse(this._body, { failText: this._failText });
  }
  async text() {
    if (this._failText) throw new Error("text() blew up");
    if (this._read) throw new Error("body already consumed");
    this._read = true;
    return this._body;
  }
}

/** An XHR whose listeners are a real, observable list - so reuse is visible. */
class FakeXHR {
  constructor() {
    this._listeners = [];
    this.responseType = "";
    this.status = 200;
    this.responseText = '{"logs":[]}';
    this.responseURL = "";
  }
  open(method, url) {
    this._method = method;
    this._url = url;
  }
  send() {
    this.responseURL = this._url;
    // A copy, because a handler that registers another handler mid-dispatch
    // would otherwise mutate the list we are walking.
    this._listeners.slice().forEach((fn) => fn.call(this));
  }
  addEventListener(event, fn) {
    if (event === "load") this._listeners.push(fn);
  }
}

let nativeFetchImpl = null;
const sandboxWindow = {
  location: {
    search: "?table=895608640",
    href: "https://boardgamearena.com/4/yokohama?table=895608640",
  },
  fetch: (...args) => nativeFetchImpl(...args),
  postMessage: (message) => posted.push(message),
  addEventListener: () => {},
  gameui: null,
  XMLHttpRequest: FakeXHR,
};

const context = {
  window: sandboxWindow,
  console: { ...console, warn: () => {}, debug: () => {} },
  URLSearchParams,
  Date,
  JSON,
  Promise,
  String,
  Object,
  setTimeout,
};
context.globalThis = context;
vm.createContext(context);
vm.runInContext(fs.readFileSync(target, "utf8"), context, { filename: target });

const wrappedFetch = context.window.fetch;
const WrappedXHR = context.window.XMLHttpRequest;

let failures = 0;
const check = (label, ok, detail) => {
  if (!ok) failures += 1;
  console.log(`  [${ok ? "PASS" : "FAIL"}] ${label}${detail ? "  - " + detail : ""}`);
};
const settle = () => new Promise((resolve) => setTimeout(resolve, 10));

(async () => {
  console.log(`capture wrapper transparency + completeness\n  target: ${target}\n`);
  console.log("--- fetch: the page must not be able to tell ---");

  const boom = new Error("network down");
  nativeFetchImpl = () => Promise.reject(boom);
  let caught = null;
  try {
    await wrappedFetch(CAPTURED_URL);
  } catch (e) {
    caught = e;
  }
  check("a rejected fetch still rejects, with the SAME error", caught === boom);

  const original = new FakeResponse('{"logs":[1,2,3]}');
  nativeFetchImpl = () => Promise.resolve(original);
  const got = await wrappedFetch(CAPTURED_URL);
  check("the page receives the identical Response object", got === original);

  let pageBody = null;
  let readErr = null;
  try {
    pageBody = await got.text();
  } catch (e) {
    readErr = e;
  }
  check(
    "the page can still read the body after we captured it",
    pageBody === '{"logs":[1,2,3]}',
    readErr ? `page got: ${readErr.message}` : ""
  );

  await settle();
  check("...and the capture was emitted", posted.some((m) => m.kind === "packets"));

  posted.length = 0;
  const cloneBomb = new FakeResponse("x", { failClone: true });
  nativeFetchImpl = () => Promise.resolve(cloneBomb);
  let pageSaw = null;
  let pageErr = null;
  try {
    pageSaw = await wrappedFetch(CAPTURED_URL);
  } catch (e) {
    pageErr = e;
  }
  check("clone() throwing does not break the page's fetch", pageSaw === cloneBomb && !pageErr);

  nativeFetchImpl = () => Promise.resolve(new FakeResponse("x", { failText: true }));
  pageErr = null;
  try {
    await wrappedFetch(CAPTURED_URL);
  } catch (e) {
    pageErr = e;
  }
  await settle();
  check("text() rejecting does not reject the page's promise", !pageErr);

  posted.length = 0;
  nativeFetchImpl = () => Promise.resolve(new FakeResponse("{}"));
  await wrappedFetch(UNMATCHED_URL);
  await settle();
  check("an unmatched URL is not captured", posted.length === 0, `${posted.length} posted`);

  console.log("\n--- fetch: every argument shape reaches the matcher (S-6) ---");

  for (const [label, resource] of [
    ["a string", CAPTURED_URL],
    ["a Request-like (.url)", { url: CAPTURED_URL }],
    ["a URL instance (.href, no .url)", new URL(CAPTURED_URL)],
    ["a stringifiable object", { toString: () => CAPTURED_URL }],
  ]) {
    posted.length = 0;
    nativeFetchImpl = () => Promise.resolve(new FakeResponse("{}"));
    await wrappedFetch(resource);
    await settle();
    check(`${label} is captured`, posted.length === 1, `${posted.length} capture(s)`);
  }

  console.log("\n--- XHR: one capture per response, however the object is reused (S-1) ---");

  posted.length = 0;
  const xhr = new WrappedXHR();
  const counts = [];
  for (let send = 0; send < 3; send += 1) {
    const before = posted.length;
    xhr.open("GET", CAPTURED_URL);
    xhr.send();
    await settle();
    counts.push(posted.length - before);
  }
  check(
    "three sends on ONE reused XHR object produce exactly one capture each",
    counts.every((n) => n === 1),
    `captures per send: ${counts.join(", ")} (total ${posted.length} for 3 responses)`
  );

  // Reuse across a URL change: the listener is registered once per object, so
  // it must re-check the URL when it fires rather than trust registration time.
  posted.length = 0;
  const reused = new WrappedXHR();
  reused.open("GET", CAPTURED_URL);
  reused.send();
  await settle();
  const afterMatching = posted.length;
  reused.open("GET", UNMATCHED_URL);
  reused.send();
  await settle();
  check(
    "reusing that object for an UNMATCHED url captures nothing further",
    afterMatching === 1 && posted.length === 1,
    `${afterMatching} then ${posted.length}`
  );

  console.log(`\n${failures === 0 ? "PASS" : "FAIL"} - ${failures} failing check(s)`);
  process.exit(failures === 0 ? 0 : 1);
})();
