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
//   snapshot (the toolbar button, always a deliberate act) -> probe /health
//            first; if the relay is not up, hand the item straight back to the
//            page to save as a .json. That file is a single NDJSON line in the
//            relay's own format, so it can simply be appended to the table's
//            capture file.
//
// The outbox is durable because an MV3 service worker is not: Chrome recycles
// it after about thirty seconds idle, so anything held only in a module
// variable is gone by the next capture.

const RELAY_ORIGIN = "http://127.0.0.1:8787"; // must match bridge-relay's --port
const OUTBOX_KEY = "outbox";

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
function safeTable(raw) {
  const cleaned = String(raw || "")
    .replace(/[^A-Za-z0-9._-]/g, "-")
    .slice(0, 64);
  return cleaned && cleaned !== "." && cleaned !== ".." ? cleaned : "unknown";
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

async function enqueue(item) {
  const items = await readOutbox();
  const bytes = items.reduce((n, i) => n + (i.bytes || 0), 0);
  if (items.length >= OUTBOX_MAX_ITEMS || bytes >= OUTBOX_MAX_BYTES) {
    console.error(
      "[yokohama-bridge] outbox is full - this capture was DROPPED. Start bridge-relay and " +
        "reload the BGA tab; the notification history endpoint is cumulative, so a full " +
        "re-read recovers what was missed."
    );
    return;
  }
  item.bytes = JSON.stringify(item.payload).length;
  items.push(item);
  await writeOutbox(items);
}

// Drain oldest-first, so the capture file keeps the order the page saw. Stops
// at the first item that needs a retry: if one POST failed because the relay is
// down, the next hundred will too.
async function flushOutbox() {
  let items = await readOutbox();
  if (items.length === 0) return;
  while (items.length > 0) {
    const outcome = await postToRelay(items[0]);
    if (outcome === "retry") break;
    items.shift(); // "stored" or "refused" - either way it leaves the queue
  }
  await writeOutbox(items);
}

async function pendingCount() {
  return (await readOutbox()).length;
}

async function updateBadge() {
  const pending = await pendingCount();
  await chrome.action.setBadgeBackgroundColor({ color: "#b3261e" });
  await chrome.action.setBadgeText({ text: pending > 0 ? String(pending) : "" });
}

// --------------------------------------------------------------- delivery ---

async function deliverPackets(item) {
  await flushOutbox();
  const outcome = await postToRelay(item);
  if (outcome === "retry") await enqueue(item);
  await updateBadge();
}

async function deliverSnapshot(item, sender) {
  await flushOutbox();
  if (await relayIsUp()) {
    const outcome = await postToRelay(item);
    if (outcome === "stored") {
      console.info(`[yokohama-bridge] snapshot delivered for table ${item.table}`);
      await updateBadge();
      return;
    }
    if (outcome === "refused") return; // already logged; a download would not help
  }
  await offerDownload(item, sender);
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
    return;
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
  } catch (e) {
    console.error("[yokohama-bridge] download fallback could not reach the page:", e);
  }
}

// ----------------------------------------------------------------- wiring ---

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || message.type !== "capture") return undefined;

  const item = { table: message.table, kind: message.kind, payload: message.payload };

  // A THUNK, not a promise. Handing `serialize` an already-started promise
  // would leave the chain nothing to hold back: the delivery's read of the
  // outbox would have happened before its turn came, so a burst arriving in one
  // tick would read the same array N times over and keep only the last write.
  // `bridge/tests/outbox_race.js` is the regression - it reported 1/5 survived
  // against that shape.
  const work = () =>
    item.kind === "snapshot" ? deliverSnapshot(item, sender) : deliverPackets(item);

  // `return true` keeps the message port open, and an open port keeps this
  // service worker alive until the delivery finishes. Without it Chrome may
  // recycle the worker mid-POST.
  serialize(work)
    .then(() => sendResponse({ ok: true }))
    .catch((e) => {
      console.error("[yokohama-bridge] delivery failed:", e);
      sendResponse({ ok: false });
    });
  return true;
});

chrome.action.onClicked.addListener(async (tab) => {
  await serialize(flushOutbox);
  await updateBadge();
  if (tab && tab.id != null) {
    try {
      // No frameId: every frame is asked, and only the one holding
      // `gameui.gamedatas` answers.
      await chrome.tabs.sendMessage(tab.id, { type: "snapshot-request" });
    } catch (e) {
      console.warn(
        "[yokohama-bridge] no bridge content script in that tab - is it a boardgamearena.com " +
          "page, and was it loaded after the extension?",
        e
      );
    }
  }
});

chrome.runtime.onStartup.addListener(() => serialize(updateBadge));
chrome.runtime.onInstalled.addListener(() => serialize(updateBadge));
