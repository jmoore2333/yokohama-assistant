// background.js - the MV3 service worker: the only place that talks to the
// local relay.
//
// Content scripts hand it captures; it delivers them to
// `http://127.0.0.1:8787/capture/<table_id>`. It never talks to BGA.
//
// Delivery differs by kind, on purpose:
//
//   packets  (the automatic stream, arriving whenever the page fetches its
//            notification log) -> POST to the relay; if that fails, park the
//            item in a DURABLE outbox and raise the toolbar badge. Downloading
//            a file per packet would bury the owner in downloads, and dropping
//            it would lose the very history H2 replays.
//
//   snapshot (the popup's Take snapshot button, always a deliberate act) ->
//            probe /health first; if the relay is not up, hand the item straight
//            back to the page to save as a .json. That file is a single NDJSON
//            line in the relay's own format, so it can simply be appended to the
//            table's capture file.
//
// The outbox is durable because an MV3 service worker is not: Chrome recycles
// it after about thirty seconds idle, so anything held only in a module
// variable is gone by the next capture. The same fact is why the session
// counters below live in storage rather than in a `let`.

const RELAY_ORIGIN = "http://127.0.0.1:8787"; // must match bridge-relay's --port
const OUTBOX_KEY = "outbox";
const SESSION_KEY = "session";
const RELAY_KEY = "relay";

// `chrome.storage.local` gives about 10 MB without the `unlimitedStorage`
// permission, which this extension deliberately does not ask for. The bound
// below leaves headroom inside that. Reaching it means the relay has been down
// for a long time - the badge says so, and BGA's history endpoint is
// cumulative, so reloading the tab once the relay is back re-fetches what was
// missed.
const OUTBOX_MAX_ITEMS = 400;
const OUTBOX_MAX_BYTES = 6 * 1024 * 1024;

const REQUEST_TIMEOUT_MS = 5000;

// ------------------------------------------------------------ serializing ---

// One at a time. Two captures arriving together would otherwise read, modify,
// and write the outbox on top of each other.
//
// ONLY entry points serialize (the message listener, the poll, the wake).
// Nothing inside a serialized task may call `serialize` again - that would wait
// on a chain it is itself holding, which is a deadlock, not a queue.
let chain = Promise.resolve();
function serialize(task) {
  const run = chain.then(task, task);
  chain = run.catch(() => {});
  return run;
}

// ------------------------------------------------------------------ relay ---

// Keep this in step with `capture::safe_table_id` in the relay. Sanitizing here
// rather than percent-encoding is deliberate: an id the relay would refuse
// costs us a capture, and a capture is worth more than a faithful table name.
//
// "In step" is the load-bearing word (six-lens lens-6). The relay's rule is the
// allowlist PLUS a leading-alphanumeric requirement (`-rf`, `.ssh`, `_private`
// all refused there), so an id that only satisfies the allowlist here — leading
// `-`/`.`/`_` — would be emitted, refused 400 by the relay, and DROPPED from the
// outbox silently. The leading-alphanumeric check closes that divergence; the
// `"unknown"` fallback is a value the relay accepts. Real BGA table ids are
// numeric, so nothing legitimate is excluded.
function safeTable(raw) {
  const cleaned = String(raw || "")
    .replace(/[^A-Za-z0-9._-]/g, "-")
    .slice(0, 64);
  return /^[A-Za-z0-9]/.test(cleaned) ? cleaned : "unknown";
}

async function withTimeout(url, options) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function relayIsUp() {
  try {
    const response = await withTimeout(`${RELAY_ORIGIN}/health`, { method: "GET" });
    return response.ok;
  } catch (_) {
    return false;
  }
}

// Returns "stored" | "retry" | "refused".
//
// "refused" is a 4xx: the relay understood the item and will not take it (bad
// envelope, bad table id). Re-sending it forever would only fill the outbox
// with something that can never land, so it is dropped - loudly, in the service
// worker console, which is where the owner is told to look.
async function postToRelay(item) {
  const url = `${RELAY_ORIGIN}/capture/${safeTable(item.table)}`;
  let response;
  try {
    response = await withTimeout(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ kind: item.kind, payload: item.payload }),
    });
  } catch (_) {
    return "retry"; // relay not running, or not answering in time
  }
  if (response.ok) return "stored";
  const detail = await response.text().catch(() => "");
  console.error(`[yokohama-bridge] relay refused a ${item.kind} (${response.status}): ${detail}`);
  return response.status >= 400 && response.status < 500 ? "refused" : "retry";
}

// ----------------------------------------------------------------- outbox ---

const readOutbox = async () => (await chrome.storage.local.get(OUTBOX_KEY))[OUTBOX_KEY] || [];
const writeOutbox = (items) => chrome.storage.local.set({ [OUTBOX_KEY]: items });

// PURE, and the ONE definition of "full" (review N-1). `enqueue` reads it to
// decide whether to DROP, and the badge reads it to SAY so; a second copy of
// the rule would be a badge that reports "parked" about captures that were in
// fact thrown away. Full by EITHER bound - the item cap or the byte cap, since
// both drop.
function outboxIsFull(items) {
  const list = Array.isArray(items) ? items : [];
  const bytes = list.reduce((n, i) => n + ((i && i.bytes) || 0), 0);
  return list.length >= OUTBOX_MAX_ITEMS || bytes >= OUTBOX_MAX_BYTES;
}

// Returns "parked" | "dropped" - the caller records which, because a dropped
// capture is the one thing here that is unrecoverable and the panel says so.
async function enqueue(item) {
  const items = await readOutbox();
  if (outboxIsFull(items)) {
    console.error(
      "[yokohama-bridge] outbox is full - this capture was DROPPED. Start bridge-relay and " +
        "reload the BGA tab; the notification history endpoint is cumulative, so a full " +
        "re-read recovers what was missed."
    );
    return "dropped";
  }
  item.bytes = JSON.stringify(item.payload).length;
  items.push(item);
  await writeOutbox(items);
  return "parked";
}

// Drain oldest-first, so the capture file keeps the order the page saw. Stops
// at the first item that needs a retry: if one POST failed because the relay is
// down, the next hundred will too.
async function flushOutbox() {
  let items = await readOutbox();
  if (items.length === 0) return;
  let reachable = null;
  while (items.length > 0) {
    const outcome = await postToRelay(items[0]);
    if (outcome === "retry") {
      reachable = false;
      break;
    }
    reachable = true; // the relay answered, even if it answered "no"
    const item = items.shift();
    await recordSession({
      outcome: outcome === "stored" ? "delivered" : "refused",
      kind: item.kind,
      table: item.table,
    });
  }
  await writeOutbox(items);
  if (reachable !== null) await noteRelay(reachable);
}

async function pendingCount() {
  return (await readOutbox()).length;
}

// -------------------------------------------------------- the indicator ---

// THE CONNECTED INDICATOR (owner-requested, 2026-08-08). Two surfaces over one
// piece of state:
//
//   the BADGE   - relay reachable? and how deep is the outbox? - at a glance,
//   the POPUP   - the same, plus the table number and this session's counts.
//
// HOW THE BADGE STAYS TRUE, and what it costs. Relay state is learned from
// three places, in descending order of how cheap it is:
//
//   1. free - every packets delivery already POSTs to the relay, so its
//      outcome IS a reachability observation. During play this is what keeps
//      the badge honest, at zero extra requests.
//   2. a 20s poll, armed only while a boardgamearena.com tab exists, and
//      re-armed only while one still does. It costs one loopback GET per 20s
//      of a game you are sitting in, and nothing at all otherwise.
//   3. the popup, which force-probes on open. A click is a deliberate act, so
//      it is allowed to spend a request, and it is why the PANEL is the
//      authority and the badge is the hint.
//
// And ONE thing keeps all three honest: a stored reading EXPIRES
// (`relayBeliefAt`). Whatever the badge paints, it paints from evidence no
// older than two poll intervals, or it paints "?". Freshness is not something
// the three mechanisms are trusted to maintain - it is checked at the moment
// of painting.
//
// The poll is a plain `setTimeout` chain and NOT `chrome.alarms`, because
// `alarms` is a permission and this extension's permission list is a promise to
// the owner.
//
// THE CADENCE IS LOAD-BEARING, and it used to be wrong (review SHOULD-1). It
// was 30s - EXACTLY Chrome's MV3 idle-termination window. A pending
// `setTimeout` is an in-memory timer that dies with its worker, so a timer
// armed for the same 30s Chrome is counting down usually loses the race, and it
// loses it precisely in the case it was written for: a BGA tab open with
// nothing being captured. At 20s the timer fires with ~10s to spare.
//
// The knock-on, stated rather than pocketed: the poll's own `chrome.*` calls
// RESET that idle timer, so while a boardgamearena.com tab is open this worker
// now stays resident instead of being recycled every half-minute. That is a
// deliberate trade - one loopback GET and one live worker per open table,
// against a badge that can be checked. It is bounded by the tab: with no BGA
// tab open the poll does not re-arm, nothing resets the timer, and the worker
// is recycled as before.
//
// What the poll still cannot do is repaint a badge while no worker is running.
// So the residual is a FROZEN badge (a photograph from the last worker), never
// a badge that keeps asserting something it has stopped checking - that is what
// the staleness rule below is for.
const HEALTH_TTL_MS = 15000; // a probe younger than this is reused
const HEALTH_POLL_MS = 20000; // the idle cadence, while a BGA tab is open
const BGA_TAB_MATCH = ["*://*.boardgamearena.com/*"];

// Exposed as a function because `bridge/tests/connected_indicator.js` loads this
// file into a `vm` context, where top-level `const`s are NOT visible on the
// context object but function declarations are. The test reads the shipped
// cadence rather than duplicating it, and asserts it is inside the idle window.
function healthPollMs() {
  return HEALTH_POLL_MS;
}

const BADGE_LIVE = "#1b7f3b"; // green   - relay answering, nothing waiting
const BADGE_DRAINING = "#9a6400"; // amber   - relay answering, backlog going out
const BADGE_PARKING = "#b3261e"; // red     - relay down, captures piling up
const BADGE_UNSURE = "#5f6368"; // grey    - relay down or not yet asked

// PURE, and the whole badge policy - THE ONLY COPY OF IT (review N-2). `popup.js`
// used to re-derive the same four-state decision to pick its CSS class, so the
// policy lived in two places and only one of them was tested. It now reads the
// `state` name returned here and supplies wording for it; deciding WHICH state
// we are in happens exactly once, and it happens here.
//
// `relayUp` is true / false / null (never asked). `pending` is the outbox depth.
// `full` comes from `outboxIsFull` - never re-derived from `pending`, because
// the outbox is also full by BYTES.
//
// The three rules worth stating out loud:
//   * a nonzero outbox ALWAYS renders as its count. That was the badge's only
//     meaning before this feature and it is the number the owner already knows
//     how to read; the indicator adds colour to it rather than taking it away.
//   * no state renders empty. An empty badge used to mean "fine" and also
//     "this worker has never run" and also "the relay died an hour ago", which
//     is the ambiguity the whole feature exists to remove.
//   * a FULL outbox says DROPPED, not parked. At the cap the count stops being
//     a backlog that will drain and becomes a loss that will not - and the two
//     were previously the same red badge saying the same word (review N-1).
function badgeFor(state) {
  const pending = (state && Number(state.pending)) || 0;
  const relayUp = state ? state.relayUp : null;
  const full = !!(state && state.full);

  if (pending > 0) {
    if (relayUp === true) {
      // No FULL wording on this arm even at the cap: with the relay answering,
      // a capture is POSTed directly (and the backlog flushed first), so
      // nothing is being dropped - the depth is draining, not overflowing.
      return {
        state: "draining",
        text: String(pending),
        color: BADGE_DRAINING,
        title: `Yokohama bridge - relay connected, sending ${pending} queued capture(s)`,
      };
    }
    // Red either way: a backlog wants attention whether or not we know why it
    // is there. The TOOLTIP is where the two part company - the depth is a
    // local fact and is always true, but "the relay is not answering" is a
    // READING, and with `relayUp === null` there is no reading to say it from.
    const holding = full
      ? `outbox FULL at ${pending} - new captures are being DROPPED`
      : `${pending} capture(s) parked`;
    return relayUp === false
      ? {
          state: "parking",
          text: String(pending),
          color: BADGE_PARKING,
          title: `Yokohama bridge - RELAY NOT ANSWERING on ${RELAY_ORIGIN}; ${holding}. Start bridge-relay.`,
        }
      : {
          state: "waiting",
          text: String(pending),
          color: BADGE_PARKING,
          title: `Yokohama bridge - ${
            full ? holding : `${pending} capture(s) waiting`
          }; relay state unchecked. Open the panel to check ${RELAY_ORIGIN}.`,
        };
  }
  if (relayUp === true) {
    return {
      state: "live",
      text: "●",
      color: BADGE_LIVE,
      title: `Yokohama bridge - relay connected on ${RELAY_ORIGIN}, recording`,
    };
  }
  if (relayUp === false) {
    return {
      state: "offline",
      text: "○",
      color: BADGE_UNSURE,
      title: `Yokohama bridge - relay NOT answering on ${RELAY_ORIGIN} (nothing parked yet)`,
    };
  }
  return {
    state: "unknown",
    text: "?",
    color: BADGE_UNSURE,
    title: "Yokohama bridge - relay not checked yet; open the panel to check",
  };
}

// A reading older than this stops being evidence and goes back to "unknown".
// Two poll intervals: one missed poll is ordinary (a recycle, a busy machine),
// two in a row means nothing is checking.
const BADGE_STALE_AFTER_MS = 2 * HEALTH_POLL_MS;

// PURE, and the rule that stops the badge asserting more than it knows (review
// SHOULD-1). `badgeFor` reads a stored answer and has no idea how old it is, so
// before this a worker waking on an hour-old `{up: true}` painted a green
// "connected, recording" having probed nothing - and `wake()` deliberately
// spends no request, so nothing corrected it until the next capture or poll.
//
// Returns true / false / null, where null is "I do not know" - which the badge
// already renders honestly as "?".
//
// A reading dated in the FUTURE is unknown rather than believed: the clock
// moved, so the age is not measurable, and unknown is the safe direction.
function relayBeliefAt(relay, nowMs) {
  if (!relay || typeof relay.up !== "boolean") return null;
  const age = (nowMs || 0) - (relay.at_ms || 0);
  if (age < 0 || age > BADGE_STALE_AFTER_MS) return null;
  return relay.up;
}

// PURE. A session's counters, zeroed. A "session" starts at browser startup or
// when the extension is (re)loaded, and it lives in storage rather than in a
// module variable because this worker is recycled after ~30 idle seconds and a
// counter that resets every half minute is worse than no counter at all.
function emptySession(nowMs, reason) {
  return {
    started_at_ms: nowMs || 0,
    started_because: reason || "unknown",
    packets_delivered: 0,
    snapshots_delivered: 0,
    snapshots_saved: 0, // relay was down; the snapshot became a download
    captures_parked: 0,
    captures_dropped: 0, // outbox full, or a snapshot that could not be saved
    captures_refused: 0, // the relay answered 4xx: it will never take this one
    last_table: null,
    last_kind: null,
    last_event_at_ms: null,
  };
}

// PURE. One delivery outcome folded into a session. Returns a NEW object; the
// input is never touched, because the caller holds the value it read from
// storage and a mutation there would be invisible until the next write.
//
// An outcome this does not recognise moves no counter - the table and timestamp
// still update, since knowing which table the worker last saw is useful even
// when the outcome is one a future version added.
function sessionAfter(session, event) {
  const base = session && typeof session === "object" ? session : emptySession(event && event.atMs, "first capture");
  const next = { ...base };
  const kind = event && event.kind;

  switch (event && event.outcome) {
    case "delivered":
      if (kind === "snapshot") next.snapshots_delivered = (base.snapshots_delivered || 0) + 1;
      else next.packets_delivered = (base.packets_delivered || 0) + 1;
      break;
    case "saved":
      next.snapshots_saved = (base.snapshots_saved || 0) + 1;
      break;
    case "parked":
      next.captures_parked = (base.captures_parked || 0) + 1;
      break;
    case "dropped":
      next.captures_dropped = (base.captures_dropped || 0) + 1;
      break;
    case "refused":
      next.captures_refused = (base.captures_refused || 0) + 1;
      break;
    default:
      break;
  }

  if (event && event.table) next.last_table = String(event.table);
  if (kind) next.last_kind = kind;
  next.last_event_at_ms = (event && event.atMs) || Date.now();
  return next;
}

const readSession = async () => (await chrome.storage.local.get(SESSION_KEY))[SESSION_KEY] || null;

// Inside a serialized task already - never wraps itself in `serialize`.
//
// This is one storage write per capture, where a healthy delivery previously
// did none. That is the price of counters that survive a recycle, and its
// worst case is bounded by something that already exists: `main-world.js`'s
// socket volume guard caps captures at 200 per 10s, and ordinary play is
// orders of magnitude below that.
async function recordSession(event) {
  const next = sessionAfter(await readSession(), { atMs: Date.now(), ...event });
  await chrome.storage.local.set({ [SESSION_KEY]: next });
  return next;
}

async function startSession(reason) {
  await chrome.storage.local.set({ [SESSION_KEY]: emptySession(Date.now(), reason) });
}

const readRelay = async () => (await chrome.storage.local.get(RELAY_KEY))[RELAY_KEY] || null;

// Written only when the answer CHANGES or the record has gone stale, so a busy
// game does not turn every capture into a storage write. `at_ms` therefore
// under-claims freshness by up to HEALTH_TTL_MS, which is the safe direction:
// the panel may say "checked 14s ago" about a 2s-old observation, never the
// reverse.
//
// `force` is the ONE exception, and it exists for the Recheck relay button
// (review N-6). Under the suppression rule a deliberate re-probe that came back
// with the same answer within 15s left `at_ms` alone, so the panel still read
// "up · 14s ago" and the click looked like it had done nothing. A forced probe
// really did just happen, so it is allowed to say so.
async function noteRelay(up, options) {
  const reachable = !!up; // callers pass a comparison result, not always a bool
  const previous = await readRelay();
  const now = Date.now();
  const force = !!(options && options.force);
  if (!force && previous && previous.up === reachable && now - (previous.at_ms || 0) < HEALTH_TTL_MS) return previous;
  const record = { up: reachable, at_ms: now };
  await chrome.storage.local.set({ [RELAY_KEY]: record });
  return record;
}

async function probeRelay(options) {
  const up = await relayIsUp();
  await noteRelay(up, options);
  return up;
}

// The badge is painted through the staleness rule, ALWAYS - including on the
// wake, which is the paint that used to carry an arbitrarily old claim. The
// panel does not need it: `statusReport` re-probes anything older than
// HEALTH_TTL_MS before it renders, so what it shows is at most 15s old and is
// dated on screen.
async function currentBadge() {
  const relay = await readRelay();
  // ONE outbox read, and both facts come out of it: the depth, and whether the
  // outbox is at a bound. Asking `pendingCount()` and then re-deriving fullness
  // from the number it returned would miss the byte bound entirely.
  const items = await readOutbox();
  return badgeFor({
    pending: items.length,
    full: outboxIsFull(items),
    relayUp: relayBeliefAt(relay, Date.now()),
  });
}

async function updateBadge() {
  const badge = await currentBadge();
  await chrome.action.setBadgeBackgroundColor({ color: badge.color });
  await chrome.action.setBadgeTextColor({ color: "#ffffff" });
  await chrome.action.setBadgeText({ text: badge.text });
  await chrome.action.setTitle({ title: badge.title });
  return badge;
}

// What the popup renders. `force` spends a /health request; without it a probe
// still happens once the cached one is older than HEALTH_TTL_MS, so the panel
// is never showing a reading it cannot date.
async function statusReport(options) {
  const forced = !!(options && options.force);
  const cached = await readRelay();
  const stale = !cached || Date.now() - (cached.at_ms || 0) >= HEALTH_TTL_MS;
  if (forced || stale) await probeRelay({ force: forced });

  // Repainting here is the point of opening the panel: whatever the badge was
  // showing, it now shows what the panel is about to say.
  const badge = await updateBadge();
  const relay = await readRelay();
  const pending = await pendingCount();
  // Started at 0, NOT at `Date.now()` (review N-5). There is no session here;
  // stamping the synthetic one with the current time made the panel report
  // "Recording since 0s" on a worker that had been up for an hour and recorded
  // nothing. Zero renders as "—", which is the truth.
  const session = (await readSession()) || emptySession(0, "not started");

  return {
    relay_origin: RELAY_ORIGIN,
    relay_up: relay ? relay.up : null,
    relay_checked_at_ms: relay ? relay.at_ms : null,
    pending,
    outbox_max_items: OUTBOX_MAX_ITEMS,
    badge,
    session,
    now_ms: Date.now(),
  };
}

// ------------------------------------------------------------- the poll ---

let pollTimer = null;

// The `url` filter works on the strength of the boardgamearena.com HOST
// permission - no `tabs` permission is taken for it. If a future Chrome were to
// ignore the filter without that permission, this degrades to "poll while any
// tab is open", which costs one extra loopback GET per poll interval and breaks
// nothing.
async function anyBgaTabOpen() {
  try {
    const tabs = await chrome.tabs.query({ url: BGA_TAB_MATCH });
    return !!(tabs && tabs.length > 0);
  } catch (_) {
    // Only reachable if the host permission were withdrawn; treat it as "stop
    // polling" rather than as "poll forever".
    return false;
  }
}

function armPoll() {
  if (pollTimer !== null) return;
  pollTimer = setTimeout(() => {
    pollTimer = null;
    serialize(async () => {
      if (!(await anyBgaTabOpen())) return false; // no table open: stop, do not re-arm
      await probeRelay();
      await flushOutbox(); // the relay came back? then the backlog goes now
      await updateBadge();
      return true;
    })
      .then((again) => {
        if (again) armPoll();
      })
      .catch((e) => console.warn("[yokohama-bridge] health poll failed:", e));
  }, HEALTH_POLL_MS);
}

// --------------------------------------------------------------- delivery ---

async function deliverPackets(item) {
  await flushOutbox();
  const outcome = await postToRelay(item);
  await noteRelay(outcome !== "retry"); // a 4xx still means the relay is THERE
  if (outcome === "retry") {
    await recordSession({ outcome: await enqueue(item), kind: item.kind, table: item.table });
  } else {
    await recordSession({
      outcome: outcome === "stored" ? "delivered" : "refused",
      kind: item.kind,
      table: item.table,
    });
  }
  await updateBadge();
}

async function deliverSnapshot(item, sender) {
  await flushOutbox();
  if (await probeRelay()) {
    const outcome = await postToRelay(item);
    if (outcome === "stored") {
      console.info(`[yokohama-bridge] snapshot delivered for table ${item.table}`);
      await recordSession({ outcome: "delivered", kind: "snapshot", table: item.table });
      await updateBadge();
      return;
    }
    if (outcome === "refused") {
      // already logged; a download would not help
      await recordSession({ outcome: "refused", kind: "snapshot", table: item.table });
      await updateBadge();
      return;
    }
  }
  const saved = await offerDownload(item, sender);
  await recordSession({ outcome: saved ? "saved" : "dropped", kind: "snapshot", table: item.table });
  await updateBadge();
}

// The relay-is-down path for a deliberate snapshot. The file written is ONE
// NDJSON line in exactly the relay's format, so recovering it is a `cat` onto
// the table's capture file rather than a conversion.
async function offerDownload(item, sender) {
  const line =
    JSON.stringify({
      received_at_ms: Date.now(),
      kind: item.kind,
      payload: item.payload,
    }) + "\n";
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const filename = `${safeTable(item.table)}-${item.kind}-${stamp}.ndjson.json`;

  const tabId = sender && sender.tab && sender.tab.id;
  if (tabId == null) {
    console.error("[yokohama-bridge] no tab to save through; snapshot lost");
    return false;
  }
  try {
    // Sent to the TOP frame (frameId 0), not the game iframe: a download
    // started from a nested frame can be blocked by the browser, and the top
    // frame has no such restriction.
    await chrome.tabs.sendMessage(tabId, { type: "download", filename, text: line }, { frameId: 0 });
    console.warn(
      `[yokohama-bridge] relay is not answering on ${RELAY_ORIGIN} - saved ${filename} instead. ` +
        "Append it to the table's .ndjson capture file."
    );
    return true;
  } catch (e) {
    console.error("[yokohama-bridge] download fallback could not reach the page:", e);
    return false;
  }
}

// What the toolbar CLICK used to do, verbatim: flush the backlog, then ask the
// page for a `gamedatas` snapshot. It now hangs off the popup's Take snapshot
// button, because an action with a `default_popup` never fires `onClicked` -
// the popup is the click. One extra keystroke buys the panel; the button is
// autofocused so Enter still works the moment it opens.
async function requestSnapshot(tabId) {
  await flushOutbox();
  await updateBadge();
  if (tabId == null) {
    return { ok: false, detail: "No active tab to snapshot. Open your BGA table tab and try again." };
  }
  try {
    // No frameId: every frame is asked, and only the one holding
    // `gameui.gamedatas` answers.
    await chrome.tabs.sendMessage(tabId, { type: "snapshot-request" });
    return { ok: true, detail: "Snapshot requested." };
  } catch (e) {
    console.warn(
      "[yokohama-bridge] no bridge content script in that tab - is it a boardgamearena.com " +
        "page, and was it loaded after the extension?",
      e
    );
    return {
      ok: false,
      detail: "No bridge in that tab - is it a boardgamearena.com page, and was it loaded AFTER the extension? Reload the tab.",
    };
  }
}

// ----------------------------------------------------------------- wiring ---

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || typeof message.type !== "string") return undefined;

  if (message.type === "capture") {
    const item = { table: message.table, kind: message.kind, payload: message.payload };

    // A THUNK, not a promise. Handing `serialize` an already-started promise
    // would leave the chain nothing to hold back: the delivery's read of the
    // outbox would have happened before its turn came, so a burst arriving in
    // one tick would read the same array N times over and keep only the last
    // write. `bridge/tests/outbox_race.js` is the regression - it reported 1/5
    // survived against that shape.
    const work = () =>
      item.kind === "snapshot" ? deliverSnapshot(item, sender) : deliverPackets(item);

    // `return true` keeps the message port open, and an open port keeps this
    // service worker alive until the delivery finishes. Without it Chrome may
    // recycle the worker mid-POST.
    serialize(work)
      .then(() => {
        armPoll(); // a capture means a table is open; keep the badge honest
        sendResponse({ ok: true });
      })
      .catch((e) => {
        console.error("[yokohama-bridge] delivery failed:", e);
        sendResponse({ ok: false });
      });
    return true;
  }

  // --- the popup's two questions ------------------------------------------

  if (message.type === "status") {
    serialize(() => statusReport({ force: !!message.force }))
      .then((report) => {
        armPoll();
        sendResponse({ ok: true, report });
      })
      .catch((e) => {
        console.error("[yokohama-bridge] status failed:", e);
        sendResponse({ ok: false, detail: String(e) });
      });
    return true;
  }

  if (message.type === "snapshot-now") {
    serialize(() => requestSnapshot(message.tabId))
      .then((result) => {
        armPoll();
        sendResponse(result);
      })
      .catch((e) => {
        console.error("[yokohama-bridge] snapshot request failed:", e);
        sendResponse({ ok: false, detail: String(e) });
      });
    return true;
  }

  return undefined;
});

chrome.runtime.onStartup.addListener(() => serialize(() => startSession("browser start").then(updateBadge)));
chrome.runtime.onInstalled.addListener(() =>
  serialize(() => startSession("extension installed or reloaded").then(updateBadge))
);

// Every worker START runs this - the install, the browser launch, and each of
// the many wakes in between. It repaints from stored state (so a recycled
// worker never shows a badge left over from a different truth) and re-arms the
// poll (which died with the previous worker). Deliberately NO probe: a wake is
// not a reason to spend a request, and the poll or the next delivery will have
// the real answer within ~20s.
//
// The repaint is safe to make WITHOUT a probe only because `currentBadge` runs
// the stored reading through `relayBeliefAt` first. This paint is exactly where
// the review found an hour-old "connected, recording": stored state is not a
// fact, it is a dated observation, and the date is checked here.
serialize(async () => {
  await updateBadge();
  armPoll();
});
