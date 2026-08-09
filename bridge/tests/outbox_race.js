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
// The environment is `mv3_stub.js` - a stub, not Chrome. It reproduces the ONE
// property this race depends on (that `chrome.storage.local` get/set are
// asynchronous) and nothing else, and the relay is DOWN throughout, so every
// capture MUST end up parked.

const path = require("path");
const { loadWorker, reporter } = require("./mv3_stub");

const target = process.argv[2] || path.join(__dirname, "..", "extension", "background.js");

/** Deliver `markers` as one burst and report which survived, in order. */
async function burst(markers) {
  const worker = loadWorker({ target, relay: "down" });
  for (const marker of markers) {
    // Not awaited, on purpose: the whole point is that they arrive together.
    worker.send({ type: "capture", table: "895608640", kind: "packets", payload: { marker } });
  }
  await new Promise((resolve) => setTimeout(resolve, 500));
  return (worker.store.outbox || []).map((item) => item.payload.marker);
}

(async () => {
  const { check, done } = reporter(`outbox race regression (M-1)\n  target: ${target}\n`);

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

  done();
})();
