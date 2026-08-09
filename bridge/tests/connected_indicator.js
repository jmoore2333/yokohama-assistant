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
//
//   5. THE BADGE CANNOT OUTLIVE ITS EVIDENCE (review fix wave, SHOULD-1). A
//      green "recording" painted from an hour-old reading is worse than no
//      badge, because the whole feature exists to answer "is it recording?".
//      Two things are pinned: the staleness rule that turns an old reading back
//      into "unknown", and that the health poll's cadence sits INSIDE Chrome's
//      ~30s MV3 idle window - a `setTimeout` scheduled at or past that window
//      is a timer whose worker is usually dead before it fires, i.e. a
//      correction mechanism that cannot run in the case it was written for.
//
//   6. EVERY REQUEST IS LOOPBACK (review fix wave, SHOULD-3). The old pins
//      asserted the relay origin the worker REPORTS and matched health hits
//      with a tail-only `/\/health$/`, so a worker pointed at
//      `https://telemetry.example.com/health` passed all four suites. Both the
//      requests actually made and the URL literals in the source are checked
//      now. `host_permissions` is not this pin: it governs whether a response
//      can be READ, not where a request may go.
//
//   7. A SNAPSHOT IS COUNTED ON BOTH PATHS (review fix wave, SHOULD-2). §6 of
//      docs/bridge-setup.md tells the owner to confirm the pre-move-1 anchor by
//      watching "Snapshots taken" go 0 -> 1, and that anchor is the one
//      observation no later diligence recovers. The counters were correct and
//      untested: stripping BOTH `recordSession` calls out of `deliverSnapshot`
//      left all four suites green.

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

// Every worker this file boots, so the egress pin in §5 can sweep the requests
// ALL of them made rather than the ones one cell happened to look at.
const workers = [];
const boot = (options) => {
  const worker = loadWorker(options);
  workers.push(worker);
  return worker;
};

const { context } = boot({ target });
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

// A backlog with NO reading behind it. The count is a local fact and is always
// shown; what the tooltip may not do is invent the reason for it.
const unsurePark = badgeFor({ relayUp: null, pending: 4 });
const knownPark = badgeFor({ relayUp: false, pending: 4 });
check("a backlog with no relay reading still shows its COUNT", unsurePark.text === "4", JSON.stringify(unsurePark.text));
check("a KNOWN-down backlog says the relay is not answering", /not answering/i.test(knownPark.title), knownPark.title);
check(
  "...and an UNKNOWN-state backlog does not say that, because it does not know",
  !/not answering/i.test(unsurePark.title) && unsurePark.title !== knownPark.title,
  unsurePark.title
);

// ------------------------------------------ 1b. what the badge may believe ---
//
// THE FIX-WAVE PIN (SHOULD-1). `badgeFor` reads a stored reading; nothing in it
// knows how OLD that reading is, so a worker waking on an hour-old `up: true`
// painted "connected, recording" having probed nothing. These two exports are
// the rule that stops it and the cadence that makes the rule rarely bite.

const relayBeliefAt = context.relayBeliefAt;
const healthPollMs = context.healthPollMs;
const IDLE_KILL_MS = 30000; // Chrome recycles an idle MV3 service worker at ~30s

const staleRuleExposed = typeof relayBeliefAt === "function" && typeof healthPollMs === "function";
check(
  "background.js exposes the badge's staleness rule and its poll cadence",
  staleRuleExposed,
  `relayBeliefAt=${typeof relayBeliefAt} healthPollMs=${typeof healthPollMs}`
);

// Read from the worker rather than duplicated here, so this file cannot drift
// from the shipped cadence. A `const` in a vm context is not visible on the
// context object, which is why the worker exports it as a function.
const POLL_MS = staleRuleExposed ? healthPollMs() : IDLE_KILL_MS;

if (staleRuleExposed) {
  check(
    "the health poll fires INSIDE Chrome's ~30s idle window, so it can actually run",
    POLL_MS > 0 && POLL_MS < IDLE_KILL_MS,
    `poll at ${POLL_MS}ms vs a ~${IDLE_KILL_MS}ms idle kill - a timer at or past the window dies with its worker`
  );

  const now = 1000000;
  const fresh = { up: true, at_ms: now - 1000 };
  const ancient = { up: true, at_ms: now - 2 * POLL_MS - 1 };
  check("a reading taken a second ago is believed", relayBeliefAt(fresh, now) === true);
  check(
    "a reading older than two poll intervals is NOT believed",
    relayBeliefAt(ancient, now) === null,
    String(relayBeliefAt(ancient, now))
  );
  check(
    "...and a stale relay-DOWN reading is equally unknown, not sticky",
    relayBeliefAt({ up: false, at_ms: now - 2 * POLL_MS - 1 }, now) === null
  );
  check("a stale reading is unknown, which is a DIFFERENT badge from either answer", (() => {
    const painted = badgeFor({ relayUp: relayBeliefAt(ancient, now), pending: 0 });
    return painted.text === unknown.text && painted.text !== live.text;
  })());
  check("no reading at all is unknown", relayBeliefAt(null, now) === null);
  check("a malformed reading is unknown rather than a crash", (() => {
    try {
      return relayBeliefAt({ up: "yes" }, now) === null && relayBeliefAt(undefined, now) === null;
    } catch (_) {
      return false;
    }
  })());
  check(
    "a reading dated in the FUTURE (the clock moved) is unknown, not believed",
    relayBeliefAt({ up: true, at_ms: now + 60000 }, now) === null
  );
}

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

// The relay this extension is allowed to talk to, and the ONE health URL. Both
// spelled in full: the old pins matched `/\/health$/`, which a request to
// `https://telemetry.example.com/health` also satisfies.
const LOOPBACK = "http://127.0.0.1:8787";
const HEALTH_URL = `${LOOPBACK}/health`;

(async () => {
  // --- a cold wake paints the badge without touching the network -----------
  const cold = boot({ target, relay: "up" });
  await settle();
  check("a cold worker paints the badge from stored state", cold.badge.text === unknown.text, `badge=${JSON.stringify(cold.badge.text)}`);
  check(
    "...without a health probe (a wake is not a reason to spend a request)",
    cold.requests.length === 0,
    `${cold.requests.length} request(s): ${cold.requests.map((r) => r.url).join(", ")}`
  );
  check(
    `...and arms the health poll at its shipped cadence (${POLL_MS}ms)`,
    cold.timers.some((t) => t.ms === POLL_MS),
    `timers at: ${cold.timers.map((t) => t.ms).join(", ") || "(none)"}`
  );

  cold.fireTimer(POLL_MS);
  await settle();
  const healthHits = cold.requests.filter((r) => r.url === HEALTH_URL);
  check("the poll probes /health ON THE LOOPBACK RELAY", healthHits.length === 1, `${healthHits.length} hit(s) of ${cold.requests.length} request(s)`);
  check("...and the badge turns connected", cold.badge.text === live.text && cold.badge.color === live.color, `${JSON.stringify(cold.badge.text)} ${cold.badge.color}`);
  check(
    "...and re-arms itself while a BGA tab is open",
    cold.timers.filter((t) => t.ms === POLL_MS).length === 2,
    `${cold.timers.filter((t) => t.ms === POLL_MS).length} poll timer(s)`
  );

  const noTabs = boot({ target, relay: "up", tabs: [] });
  await settle();
  noTabs.fireTimer(POLL_MS);
  await settle();
  check(
    "with no BGA tab open the poll stops rather than running forever",
    noTabs.timers.filter((t) => t.ms === POLL_MS).length === 1 && noTabs.requests.length === 0,
    `${noTabs.timers.filter((t) => t.ms === POLL_MS).length} poll timer(s), ${noTabs.requests.length} request(s)`
  );

  // --- waking on an old reading (SHOULD-1, end to end) ---------------------
  //
  // The exact probe from the review: storage holds "connected", taken an hour
  // ago; the relay is in fact down. The wake deliberately spends no request, so
  // the ONLY thing standing between the owner and a green "recording" badge
  // that is an hour out of date is the staleness rule.
  const stale = boot({
    target,
    relay: "down",
    store: { relay: { up: true, at_ms: Date.now() - 3600000 } },
  });
  await settle();
  check(
    "a worker waking on an HOUR-OLD 'connected' reading does not paint CONNECTED",
    stale.badge.text !== live.text,
    `badge=${JSON.stringify(stale.badge.text)} ${stale.badge.color} - ${stale.badge.title}`
  );
  check(
    "...it paints UNKNOWN, which is what it actually knows",
    stale.badge.text === unknown.text && stale.badge.color === unknown.color,
    `${JSON.stringify(stale.badge.text)} ${stale.badge.color}`
  );
  check(
    "...and still spends no request to say so (the wake's cost is unchanged)",
    stale.requests.length === 0,
    `${stale.requests.length} request(s)`
  );

  const recent = boot({ target, relay: "down", store: { relay: { up: true, at_ms: Date.now() - 2000 } } });
  await settle();
  check(
    "a two-second-old reading is still trusted, so the rule ages readings and does not just discard them",
    recent.badge.text === live.text,
    `badge=${JSON.stringify(recent.badge.text)}`
  );

  // --- the relay is down: park, and SAY so ---------------------------------
  const down = boot({ target, relay: "down" });
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
  const recycled = boot({ target, relay: "up", store: down.store });
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
      "a forced status probes /health on the loopback relay, so the panel is never stale",
      recycled.requests.some((r) => r.url === HEALTH_URL),
      recycled.requests.map((r) => r.url).join(", ")
    );
  }

  // --- a snapshot is counted on BOTH paths (SHOULD-2) ----------------------
  //
  // §6 of docs/bridge-setup.md hangs the pre-move-1 anchor on "Snapshots taken
  // going 0 -> 1", and that anchor is the one observation no later diligence
  // recovers. Both arms of `deliverSnapshot` are exercised, because the
  // relay-is-down arm is the one the owner meets when the anchor matters most.
  const snapUp = boot({ target, relay: "up" });
  await settle();
  await snapUp.send({ type: "capture", table: "895724349", kind: "snapshot", payload: { gamedatas: true } });
  await settle();
  check(
    "a snapshot delivered to a live relay counts as a SNAPSHOT, not as a packet",
    snapUp.store.session.snapshots_delivered === 1 && snapUp.store.session.packets_delivered === 0,
    JSON.stringify(snapUp.store.session)
  );
  check(
    "...and it went to the loopback relay's capture endpoint for that table",
    snapUp.requests.some((r) => r.method === "POST" && r.url === `${LOOPBACK}/capture/895724349`),
    snapUp.requests.map((r) => `${r.method} ${r.url}`).join(", ")
  );

  const snapDown = boot({ target, relay: "down" });
  await settle();
  await snapDown.send({ type: "capture", table: "895724349", kind: "snapshot", payload: { gamedatas: true } });
  await settle();
  check(
    "a snapshot with the relay down is handed back to the PAGE to save",
    snapDown.tabMessages.some((m) => m.message && m.message.type === "download" && /\.ndjson\.json$/.test(m.message.filename)),
    JSON.stringify(snapDown.tabMessages.map((m) => m.message && m.message.type))
  );
  check(
    "...and is STILL counted as a snapshot taken - the count the owner confirms the anchor with",
    snapDown.store.session.snapshots_saved === 1,
    JSON.stringify(snapDown.store.session)
  );
  check(
    "...and it does not go into the outbox, which is the packets stream's queue",
    (snapDown.store.outbox || []).length === 0,
    `${(snapDown.store.outbox || []).length} parked`
  );

  // --- the snapshot-on-click contract -------------------------------------
  const clicker = boot({ target, relay: "down" });
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

  const noTab = boot({ target, relay: "up", tabSendFails: true });
  await settle();
  const failed = await noTab.send({ type: "snapshot-now", tabId: 9 }, {});
  check(
    "a snapshot asked of a tab with no bridge in it fails LOUDLY, not silently",
    !!(failed && failed.ok === false && typeof failed.detail === "string" && failed.detail.length > 10),
    failed ? JSON.stringify(failed.detail) : "no answer"
  );

  // ------------------------------------ 3b. everything went to the loopback ---
  //
  // THE FIX-WAVE PIN (SHOULD-3). Every worker booted above is swept, so this
  // covers the health probe, the capture POSTs, both snapshot arms and the
  // drain - whichever cell happened to drive them. Repointing either fetch site
  // at a remote host used to pass all four suites: the only things resembling a
  // pin were `report.relay_origin`, which is a string the worker REPORTS, and a
  // tail-only `/\/health$/`, which `https://anywhere/health` satisfies.
  //
  // `host_permissions` does not cover this. It decides whether a response can
  // be READ, not where a request may be SENT - a request to an unlisted origin
  // still leaves the machine.
  const originOf = (url) => {
    try {
      return new URL(url).origin;
    } catch (_) {
      return `(unparseable: ${url})`;
    }
  };
  const everyRequest = workers.flatMap((w) => w.requests);
  const offOrigin = everyRequest.filter((r) => originOf(r.url) !== LOOPBACK);
  check(
    "the workers made requests at all, so the sweep below is not vacuous",
    everyRequest.length > 0,
    `${everyRequest.length} request(s) across ${workers.length} worker(s)`
  );
  check(
    "EVERY request any worker made went to the loopback relay, and nowhere else",
    offOrigin.length === 0,
    offOrigin.map((r) => `${r.method} ${r.url}`).join(", ") || "none off-origin"
  );

  // The static half, so an arm no cell drives cannot smuggle one in. Read from
  // `target` for the worker (it is the code under test - a mutation run must be
  // caught here) and from the shipped popup, which is not.
  const scanned = [
    ["background.js", fs.readFileSync(target, "utf8")],
    ["popup.js", fs.readFileSync(path.join(extensionDir, "popup.js"), "utf8")],
  ];
  for (const [name, source] of scanned) {
    const urls = source.match(/https?:\/\/[^\s"'`)]+/g) || [];
    const foreign = urls.filter((u) => originOf(u) !== LOOPBACK);
    check(
      `every absolute URL in ${name} - comments included - is the loopback relay`,
      foreign.length === 0,
      foreign.join(", ") || `${urls.length} URL(s), all loopback`
    );
  }

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
