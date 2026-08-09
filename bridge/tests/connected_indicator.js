// The CONNECTED INDICATOR (owner-requested, 2026-08-08): the toolbar badge
// says whether the relay is reachable at a glance, and the click reveals the
// table number, this session's counts, and the outbox depth.
//
//   node bridge/tests/connected_indicator.js [path-to-background.js]
//
// Exits 0 on pass, 1 on failure. Plain `node`, no dependencies.
//
// WHAT IS PINNED HERE, and why each pin is worth its line:
//
//   1. THE BADGE MAP is a pure function of (relay reachable?, outbox depth),
//      so it is tested as one - a table, not a screenshot. Two of its rows
//      carry a promise made to the owner: a nonzero outbox STILL shows its
//      count (the pre-existing semantics are integrated, not clobbered), and
//      relay-up is never confusable with relay-down at the same depth.
//
//   2. THE COUNTERS are a pure reducer over delivery outcomes. The reducer is
//      tested directly AND through real deliveries, because the interesting
//      failure is not arithmetic - it is a counter that lives in a module
//      variable and therefore resets every time Chrome recycles the service
//      worker, which is roughly every thirty idle seconds. The recycle test
//      (a second worker loaded over the same storage) is the one that catches
//      that, and it is the reason the session lives in `chrome.storage.local`.
//
//   3. THE CLICK CONTRACT SURVIVES. Adding a popup means `action.onClicked`
//      stops firing, so the snapshot-and-flush the toolbar button used to do
//      has to be reachable from the popup instead - same flush, same
//      `snapshot-request` to the tab. That is a behavioural pin, not prose.
//
//   4. NO NEW PERMISSIONS. The manifest's `permissions` and `host_permissions`
//      are asserted verbatim, because the cheapest way to build a health poll
//      is to reach for `alarms`, and the cheapest way to build a popup that
//      knows which tab you are on is to reach for `tabs`. Neither is taken.

const fs = require("fs");
const path = require("path");
const { loadWorker, settle, reporter } = require("./mv3_stub");

const target = process.argv[2] || path.join(__dirname, "..", "extension", "background.js");

// The manifest and popup checks below read the SHIPPED extension, never
// `dirname(target)`: they are assertions about what is loaded into Chrome, and
// a mutation run (which points `target` at an edited copy in /tmp to prove a
// cell discriminates) must not have them blow up beside it.
const extensionDir = path.join(__dirname, "..", "extension");

const { check, done } = reporter(`connected indicator\n  target: ${target}\n`);

const { context } = loadWorker({ target });
const badgeFor = context.badgeFor;
const sessionAfter = context.sessionAfter;
const emptySession = context.emptySession;

if (typeof badgeFor !== "function" || typeof sessionAfter !== "function" || typeof emptySession !== "function") {
  console.log("  [FAIL] background.js must expose badgeFor / sessionAfter / emptySession as function declarations");
  console.log("\nFAIL - 1 failing check(s)");
  process.exit(1);
}

// ------------------------------------------------------------- 1. the map ---

const PARKED_RED = "#b3261e"; // the colour the count-only badge already used

const unknown = badgeFor({ relayUp: null, pending: 0 });
const live = badgeFor({ relayUp: true, pending: 0 });
const offline = badgeFor({ relayUp: false, pending: 0 });
const parking = badgeFor({ relayUp: false, pending: 7 });
const draining = badgeFor({ relayUp: true, pending: 3 });

check("an unchecked relay reads as unknown, not as connected", unknown.text !== live.text, `${JSON.stringify(unknown.text)} vs ${JSON.stringify(live.text)}`);
check("relay up with an empty outbox is a distinct, NON-EMPTY badge", live.text.length > 0 && live.text !== offline.text, `up=${JSON.stringify(live.text)} down=${JSON.stringify(offline.text)}`);
check("relay up and relay down differ in COLOUR as well as glyph", live.color !== offline.color, `${live.color} vs ${offline.color}`);
check(
  "a parked outbox still shows its COUNT (the pre-existing semantics)",
  parking.text === "7" && draining.text === "3",
  `down=${JSON.stringify(parking.text)} up=${JSON.stringify(draining.text)}`
);
check("...in the colour the count-only badge already used", parking.color === PARKED_RED, parking.color);
check(
  "relay-down parking is distinguishable from relay-up draining at the same depth",
  badgeFor({ relayUp: false, pending: 3 }).color !== draining.color,
  `${badgeFor({ relayUp: false, pending: 3 }).color} vs ${draining.color}`
);
check(
  "every state carries a tooltip that names the relay state",
  [unknown, live, offline, parking, draining].every((b) => typeof b.title === "string" && b.title.length > 10),
  [unknown, live, offline, parking, draining].map((b) => (b.title || "").slice(0, 24)).join(" | ")
);
check(
  "no state paints an EMPTY badge (an empty badge is the state this feature removes)",
  [unknown, live, offline, parking, draining].every((b) => typeof b.text === "string" && b.text.length > 0)
);
check("a badge is total over junk input", (() => {
  try {
    const b = badgeFor(undefined);
    return typeof b.text === "string" && typeof b.color === "string";
  } catch (_) {
    return false;
  }
})());

// --------------------------------------------------------- 2. the counters ---

const seed = emptySession(1000, "test");
const before = JSON.stringify(seed);
const afterOne = sessionAfter(seed, { outcome: "delivered", kind: "packets", table: "895724349", atMs: 2000 });
check("sessionAfter does not mutate its input", JSON.stringify(seed) === before);
check("sessionAfter returns a new object", afterOne !== seed);
check("a delivered packets capture counts once", afterOne.packets_delivered === 1 && afterOne.snapshots_delivered === 0);
check("...and records the table it was for", afterOne.last_table === "895724349");

const afterSnap = sessionAfter(afterOne, { outcome: "delivered", kind: "snapshot", table: "895724349", atMs: 3000 });
check("a delivered snapshot counts as a snapshot, not as a packet", afterSnap.snapshots_delivered === 1 && afterSnap.packets_delivered === 1);

const afterSaved = sessionAfter(afterSnap, { outcome: "saved", kind: "snapshot", table: "895724349", atMs: 4000 });
check(
  "a snapshot that fell back to a DOWNLOAD is still a snapshot taken",
  afterSaved.snapshots_saved === 1 && afterSaved.snapshots_delivered + afterSaved.snapshots_saved === 2
);

const afterParked = sessionAfter(afterSaved, { outcome: "parked", kind: "packets", table: "895724349", atMs: 5000 });
check("a parked capture counts as parked, not as delivered", afterParked.captures_parked === 1 && afterParked.packets_delivered === 1);

const afterDropped = sessionAfter(afterParked, { outcome: "dropped", kind: "packets", table: "895724349", atMs: 6000 });
check("a dropped capture is counted, because it is the one that is GONE", afterDropped.captures_dropped === 1);

const afterJunk = sessionAfter(afterDropped, { outcome: "who-knows", kind: "packets", table: "999", atMs: 7000 });
check(
  "an outcome the reducer does not know moves no counter",
  afterJunk.packets_delivered === 1 &&
    afterJunk.captures_parked === 1 &&
    afterJunk.captures_dropped === 1 &&
    afterJunk.snapshots_delivered === 1
);
check("...but the table it named is still the last table seen", afterJunk.last_table === "999");

// ------------------------------------------------- 3. end to end, in a stub ---

(async () => {
  // --- a cold wake paints the badge without touching the network -----------
  const cold = loadWorker({ target, relay: "up" });
  await settle();
  check("a cold worker paints the badge from stored state", cold.badge.text === unknown.text, `badge=${JSON.stringify(cold.badge.text)}`);
  check(
    "...without a health probe (a wake is not a reason to spend a request)",
    cold.requests.length === 0,
    `${cold.requests.length} request(s): ${cold.requests.map((r) => r.url).join(", ")}`
  );
  check(
    "...and arms a 30s health poll",
    cold.timers.some((t) => t.ms === 30000),
    `timers at: ${cold.timers.map((t) => t.ms).join(", ") || "(none)"}`
  );

  cold.fireTimer(30000);
  await settle();
  const healthHits = cold.requests.filter((r) => /\/health$/.test(r.url));
  check("the poll probes /health", healthHits.length === 1, `${healthHits.length} hit(s)`);
  check("...and the badge turns connected", cold.badge.text === live.text && cold.badge.color === live.color, `${JSON.stringify(cold.badge.text)} ${cold.badge.color}`);
  check(
    "...and re-arms itself while a BGA tab is open",
    cold.timers.filter((t) => t.ms === 30000).length === 2,
    `${cold.timers.filter((t) => t.ms === 30000).length} poll timer(s)`
  );

  const noTabs = loadWorker({ target, relay: "up", tabs: [] });
  await settle();
  noTabs.fireTimer(30000);
  await settle();
  check(
    "with no BGA tab open the poll stops rather than running forever",
    noTabs.timers.filter((t) => t.ms === 30000).length === 1 && noTabs.requests.length === 0,
    `${noTabs.timers.filter((t) => t.ms === 30000).length} poll timer(s), ${noTabs.requests.length} request(s)`
  );

  // --- the relay is down: park, and SAY so ---------------------------------
  const down = loadWorker({ target, relay: "down" });
  await settle();
  for (const marker of ["a", "b"]) {
    down.send({ type: "capture", table: "895724349", kind: "packets", payload: { marker } });
  }
  await settle();
  check("two captures park with the relay down", (down.store.outbox || []).length === 2, `${(down.store.outbox || []).length} parked`);
  check("the badge shows the depth in the parking colour", down.badge.text === "2" && down.badge.color === PARKED_RED, `${down.badge.text} ${down.badge.color}`);
  check("the session counted them as parked, not delivered", down.store.session.captures_parked === 2 && down.store.session.packets_delivered === 0);
  check("the session knows which table", down.store.session.last_table === "895724349");

  // --- the relay comes back ------------------------------------------------
  down.setRelay("up");
  down.send({ type: "capture", table: "895724349", kind: "packets", payload: { marker: "c" } });
  await settle();
  check("the backlog drains on the next capture", (down.store.outbox || []).length === 0, `${(down.store.outbox || []).length} still parked`);
  check("all three count as delivered", down.store.session.packets_delivered === 3, `${down.store.session.packets_delivered}`);
  check("the parked pair is still remembered as having been parked", down.store.session.captures_parked === 2);
  check("the badge turns connected", down.badge.text === live.text && down.badge.color === live.color, `${JSON.stringify(down.badge.text)} ${down.badge.color}`);

  // --- the recycle: the point of putting counters in storage ---------------
  const recycled = loadWorker({ target, relay: "up", store: down.store });
  await settle();
  recycled.send({ type: "capture", table: "895724349", kind: "packets", payload: { marker: "d" } });
  await settle();
  check(
    "counters SURVIVE a service-worker recycle",
    recycled.store.session.packets_delivered === 4,
    `${recycled.store.session.packets_delivered} (a module-variable counter would read 1)`
  );

  // --- the click detail ----------------------------------------------------
  const answer = await recycled.send({ type: "status", force: true }, {});
  const report = answer && answer.report;
  check("the worker answers a status request", !!(answer && answer.ok && report));
  if (report) {
    check("the report names the relay origin", report.relay_origin === "http://127.0.0.1:8787", report.relay_origin);
    check("the report carries relay reachability", report.relay_up === true);
    check("the report carries the outbox depth", report.pending === 0);
    check("the report carries the table number", report.session.last_table === "895724349", String(report.session.last_table));
    check("the report carries this session's delivered count", report.session.packets_delivered === 4);
    check("the report carries the badge the toolbar is showing", !!(report.badge && report.badge.text === live.text));
    check(
      "a forced status probes /health, so the panel is never stale",
      recycled.requests.some((r) => /\/health$/.test(r.url)),
      recycled.requests.map((r) => r.url).join(", ")
    );
  }

  // --- the snapshot-on-click contract -------------------------------------
  const clicker = loadWorker({ target, relay: "down" });
  await settle();
  clicker.send({ type: "capture", table: "895724349", kind: "packets", payload: { marker: "parked" } });
  await settle();
  clicker.setRelay("up");
  const clicked = await clicker.send({ type: "snapshot-now", tabId: 1 }, {});
  await settle();
  check("the popup's snapshot button reaches the worker", !!(clicked && clicked.ok), clicked ? clicked.detail : "no answer");
  check(
    "...and it asks the TAB for a snapshot, exactly as the old toolbar click did",
    clicker.tabMessages.some((m) => m.tabId === 1 && m.message && m.message.type === "snapshot-request"),
    JSON.stringify(clicker.tabMessages.map((m) => m.message && m.message.type))
  );
  check(
    "...and it flushes the backlog first, exactly as the old toolbar click did",
    (clicker.store.outbox || []).length === 0,
    `${(clicker.store.outbox || []).length} still parked`
  );

  const noTab = loadWorker({ target, relay: "up", tabSendFails: true });
  await settle();
  const failed = await noTab.send({ type: "snapshot-now", tabId: 9 }, {});
  check(
    "a snapshot asked of a tab with no bridge in it fails LOUDLY, not silently",
    !!(failed && failed.ok === false && typeof failed.detail === "string" && failed.detail.length > 10),
    failed ? JSON.stringify(failed.detail) : "no answer"
  );

  // ------------------------------------------- 4. no new permissions, ever ---

  const manifest = JSON.parse(fs.readFileSync(path.join(extensionDir, "manifest.json"), "utf8"));
  check(
    "the manifest still asks for exactly one permission",
    JSON.stringify(manifest.permissions) === JSON.stringify(["storage"]),
    JSON.stringify(manifest.permissions)
  );
  check(
    "...and exactly the two hosts it already had",
    JSON.stringify(manifest.host_permissions) ===
      JSON.stringify(["*://*.boardgamearena.com/*", "http://127.0.0.1:8787/*"]),
    JSON.stringify(manifest.host_permissions)
  );
  check("the action declares a popup", typeof manifest.action.default_popup === "string", String(manifest.action.default_popup));

  const popupPath = path.join(extensionDir, manifest.action.default_popup || "popup.html");
  const popupExists = fs.existsSync(popupPath);
  check("...and the popup file exists", popupExists, popupPath);
  if (popupExists) {
    const html = fs.readFileSync(popupPath, "utf8");
    check("the popup offers a snapshot control", /id="snapshot"/.test(html));
    check("the popup autofocuses it, so it stays one keystroke from open", /autofocus/.test(html));
    check("the popup loads its script from a file (MV3 forbids inline script)", /<script src="/.test(html) && !/<script>/.test(html));
    check(
      "the popup pulls NOTHING from the network",
      !/(src|href)="https?:/.test(html),
      (html.match(/(?:src|href)="https?:[^"]*"/g) || []).join(", ")
    );
    for (const asset of (html.match(/(?:src|href)="([^"]+)"/g) || []).map((m) => m.replace(/^.*?="/, "").replace(/"$/, ""))) {
      check(`the popup's ${asset} ships with the extension`, fs.existsSync(path.join(extensionDir, asset)));
    }
  }

  done();
})();
