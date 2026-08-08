// Regression test for the H0 review's M-1: the outbox must not lose captures
// when several arrive in the same tick with the relay down.
//
//   node bridge/tests/outbox_race.js [path-to-background.js]
//
// Exits 0 on pass, 1 on failure. No dependencies - plain `node`.
//
// WHY THIS EXISTS. `background.js` reads the outbox, appends to it, and writes
// it back, all across `await` points. Two captures delivered in the same tick
// therefore both read the SAME starting array and the second write clobbers the
// first, unless the deliveries are serialized. They are meant to be, by
// `serialize()` - but the original code called `deliverPackets(item)` and
// passed the resulting PROMISE to `serialize`, which starts the work
// immediately and leaves the chain with nothing to hold back. The fix is the
// thunk form (`serialize(() => …)`). This file is what tells the difference:
// against the defect it reports 1/2 survived, against the fix 2/2.
//
// The environment below is a stub, not Chrome. It reproduces the ONE property
// the race depends on - that `chrome.storage.local` get/set are asynchronous -
// and nothing else.

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const target =
  process.argv[2] || path.join(__dirname, "..", "extension", "background.js");
const source = fs.readFileSync(target, "utf8");

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

/** Load background.js in a stubbed MV3 world with the relay DOWN. */
function loadWorker() {
  const store = {};
  const storage = {
    async get(key) {
      await tick(); // a real read is async; this is where the interleave happens
      return { [key]: store[key] };
    },
    async set(object) {
      await tick();
      Object.assign(store, object);
    },
  };

  let messageListener = null;
  const chrome = {
    storage: { local: storage },
    runtime: {
      onMessage: { addListener: (fn) => (messageListener = fn) },
      onStartup: { addListener: () => {} },
      onInstalled: { addListener: () => {} },
    },
    action: {
      setBadgeBackgroundColor: async () => {},
      setBadgeText: async () => {},
      onClicked: { addListener: () => {} },
    },
    tabs: { sendMessage: async () => {} },
  };

  // A FRESH context per load, not `runInThisContext`: the worker declares its
  // constants at top level, so a second load into a shared global would fail
  // with "Identifier 'RELAY_ORIGIN' has already been declared".
  const context = {
    chrome,
    // Every delivery attempt fails, so every capture MUST end up parked.
    fetch: () => Promise.reject(new Error("ECONNREFUSED (relay not running)")),
    // The worker logs loudly on delivery failure, which is correct behaviour
    // and pure noise here.
    console: { ...console, error: () => {}, warn: () => {}, info: () => {} },
    setTimeout,
    clearTimeout,
    AbortController,
    Promise,
    JSON,
    String,
    Object,
    Date,
  };
  context.globalThis = context;
  vm.createContext(context);
  vm.runInContext(source, context, { filename: target });

  return { store, deliver: messageListener };
}

/** Deliver `markers` as one burst and report which survived, in order. */
async function burst(markers) {
  const { store, deliver } = loadWorker();
  for (const marker of markers) {
    deliver(
      { type: "capture", table: "895608640", kind: "packets", payload: { marker } },
      { tab: { id: 1 } },
      () => {}
    );
  }
  await new Promise((resolve) => setTimeout(resolve, 500));
  return (store.outbox || []).map((item) => item.payload.marker);
}

let failures = 0;
const check = (label, ok, detail) => {
  if (!ok) failures += 1;
  console.log(`  [${ok ? "PASS" : "FAIL"}] ${label}${detail ? "  - " + detail : ""}`);
};

(async () => {
  console.log(`outbox race regression (M-1)\n  target: ${target}\n`);

  const pair = ["packet-A", "packet-B"];
  const parkedPair = await burst(pair);
  check(
    "two captures in one tick both survive",
    parkedPair.length === pair.length,
    `${parkedPair.length}/${pair.length} parked: ${parkedPair.join(", ") || "(none)"}`
  );

  // A longer burst, because a two-item race can in principle be survived by
  // luck while a five-item one cannot.
  const many = ["p1", "p2", "p3", "p4", "p5"];
  const parkedMany = await burst(many);
  check(
    "a five-capture burst survives intact",
    parkedMany.length === many.length,
    `${parkedMany.length}/${many.length} parked: ${parkedMany.join(", ") || "(none)"}`
  );

  // Order is not cosmetic: the capture file is replayed in file order, so an
  // outbox that reordered a burst would hand H2 a history the game never had.
  check(
    "the burst is parked in arrival order",
    parkedMany.join(",") === many.join(","),
    `got: ${parkedMany.join(", ") || "(none)"}`
  );

  console.log(`\n${failures === 0 ? "PASS" : "FAIL"} - ${failures} failing check(s)`);
  process.exit(failures === 0 ? 0 : 1);
})();
