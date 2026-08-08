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

/**
 * A WebSocket whose listener list is observable, which can deliver a frame on
 * demand, and which counts `send` — nothing in this extension may ever send.
 */
class FakeWebSocket {
  constructor(url, protocols) {
    this.url = url;
    this.protocols = protocols;
    this._listeners = [];
    FakeWebSocket.instances.push(this);
  }
  addEventListener(type, fn) {
    if (type === "message") this._listeners.push(fn);
  }
  send() {
    FakeWebSocket.sendCalls += 1;
  }
  /** Deliver one frame to every registered listener, in registration order. */
  _fire(data) {
    this._listeners.slice().forEach((fn) => fn.call(this, { data }));
  }
  /** Deliver a frame whose `data` accessor blows up. */
  _fireHostile() {
    const event = {};
    Object.defineProperty(event, "data", {
      get() {
        throw new Error("reading data blew up");
      },
    });
    this._listeners.slice().forEach((fn) => fn.call(this, event));
  }
}
FakeWebSocket.instances = [];
FakeWebSocket.sendCalls = 0;
FakeWebSocket.CONNECTING = 0;
FakeWebSocket.OPEN = 1;
FakeWebSocket.CLOSING = 2;
FakeWebSocket.CLOSED = 3;

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
  WebSocket: FakeWebSocket,
};

const context = {
  window: sandboxWindow,
  console: { ...console, warn: () => {}, debug: () => {} },
  URLSearchParams,
  URL,
  Date,
  JSON,
  Promise,
  Proxy,
  Reflect,
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

  console.log("\n--- WebSocket: the realtime feed (H0.1) ---");

  // H0.1's reason to exist: the 2026-08-08 live run proved that during realtime
  // play BGA streams moves over a WebSocket and NO fetch/XHR fires per move, so
  // a completed turn produced zero captures. The wrapper below is what makes a
  // live game bank continuously.
  const WS_URL = "wss://r2-do1.boardgamearena.net/socket.io/?EIO=4&transport=websocket";
  const WrappedWebSocket = context.window.WebSocket;

  posted.length = 0;
  const seenByPage = [];
  const socket = new WrappedWebSocket(WS_URL, ["proto-1"]);
  socket.addEventListener("message", (event) => seenByPage.push(event.data));

  check(
    "(a) the page gets the REAL socket back, not a stand-in",
    socket === FakeWebSocket.instances[FakeWebSocket.instances.length - 1] &&
      socket instanceof FakeWebSocket,
    `identity ${socket === FakeWebSocket.instances.at(-1)}, instanceof ${socket instanceof FakeWebSocket}`
  );
  check(
    "(a) the constructor's own arguments arrive untouched",
    socket.url === WS_URL && socket.protocols[0] === "proto-1"
  );
  check(
    "(a) the static constants survive the wrap",
    WrappedWebSocket.OPEN === 1 && WrappedWebSocket.CLOSED === 3,
    `OPEN=${WrappedWebSocket.OPEN} CLOSED=${WrappedWebSocket.CLOSED}`
  );

  // A frame with the awkward characters a verbatim claim has to survive.
  const FRAME = '42["notify",{"type":"playerTurn","args":{"n":"a\\"quoted\\" \\n line"}}]';
  socket.ateFired = true;
  socket._fire(FRAME);
  await settle();

  check("(a) the page's own listener still fires", seenByPage.length === 1 && seenByPage[0] === FRAME);

  const wsCaptures = posted.filter((m) => m.payload && m.payload.source === "websocket");
  check("(b) a text frame is captured", wsCaptures.length === 1, `${wsCaptures.length} capture(s)`);
  if (wsCaptures.length === 1) {
    const payload = wsCaptures[0].payload;
    check("(b) the frame is recorded VERBATIM", payload.body_text === FRAME);
    check("(b) the observed socket url is recorded", payload.url === WS_URL, `url: ${payload.url}`);
    check("(b) it rides the packets envelope", wsCaptures[0].kind === "packets");
    check("(b) it carries a capture timestamp", typeof payload.captured_at_ms === "number");
  }

  // (c) Non-text frames are STATED, never decoded — verbatim-or-stated.
  posted.length = 0;
  socket._fire(new ArrayBuffer(64));
  await settle();
  const binary = posted.filter((m) => m.payload && m.payload.source === "websocket");
  check("(c) a non-text frame produces exactly one stub", binary.length === 1);
  if (binary.length === 1) {
    check(
      "(c) the stub states what it is and how big, and decodes nothing",
      binary[0].payload.note === "non-text frame" &&
        binary[0].payload.byte_length === 64 &&
        binary[0].payload.body_text === null,
      JSON.stringify(binary[0].payload)
    );
  }

  // (d) Sockets that are not BGA's are not our business.
  posted.length = 0;
  const seenByOtherPage = [];
  const foreign = new WrappedWebSocket("wss://chat.example.com/socket");
  foreign.addEventListener("message", (event) => seenByOtherPage.push(event.data));
  foreign._fire("something private");
  await settle();
  check("(d) a non-BGA socket produces zero captures", posted.length === 0, `${posted.length} posted`);
  check("(d) ...and that page's own listener still fires", seenByOtherPage.length === 1);

  // A hostname that merely CONTAINS the domain must not pass as a suffix.
  posted.length = 0;
  const lookalike = new WrappedWebSocket("wss://boardgamearena.net.evil.example/socket");
  lookalike._fire("nope");
  await settle();
  check("(d) a lookalike hostname does not match", posted.length === 0, `${posted.length} posted`);

  // (e) Our failure is never the page's failure.
  //
  // A FRESH socket, whose only page listener guards its own read of the
  // hostile accessor — so the one thing that can break this dispatch is OUR
  // handler letting the throw out. Our handler is registered first (at
  // construction), so if it does not swallow, `_fire` unwinds and the page
  // listener below never runs at all.
  posted.length = 0;
  const hostileSocket = new WrappedWebSocket(WS_URL);
  const seenAfterThrow = [];
  hostileSocket.addEventListener("message", (event) => {
    try {
      seenAfterThrow.push(event.data);
    } catch (_) {
      seenAfterThrow.push("(page read it and threw, which is the page's business)");
    }
  });
  let escaped = null;
  try {
    hostileSocket._fireHostile();
  } catch (e) {
    escaped = e;
  }
  check(
    "(e) a throw in the capture path does not escape to the page",
    escaped === null,
    escaped ? `escaped: ${escaped.message}` : ""
  );
  check("(e) ...and the page's listener still ran", seenAfterThrow.length === 1);

  // The volume guard: BGA sockets also carry heartbeats and chat. Content is
  // NOT filtered (H2 dedupes and filters — fidelity first), but a runaway is
  // capped, and the cap announces itself rather than going quiet.
  posted.length = 0;
  const chatty = new WrappedWebSocket(WS_URL);
  for (let i = 0; i < 260; i += 1) chatty._fire(`frame-${i}`);
  await settle();
  const captured = posted.filter((m) => m.payload && m.payload.source === "websocket");
  const notes = captured.filter((m) => m.payload.note === "volume guard tripped");
  check(
    "the volume guard caps a runaway burst",
    captured.length < 260 && captured.length > 100,
    `${captured.length} captures from 260 frames`
  );
  check("the guard says so, exactly once", notes.length === 1, `${notes.length} note(s)`);

  check("NOTHING was ever sent on any socket", FakeWebSocket.sendCalls === 0,
    `${FakeWebSocket.sendCalls} send call(s)`);

  console.log(`\n${failures === 0 ? "PASS" : "FAIL"} - ${failures} failing check(s)`);
  process.exit(failures === 0 ? 0 : 1);
})();
