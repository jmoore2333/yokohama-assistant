// popup.js - the click detail.
//
// It owns NO state and it never talks to the relay or to BGA. Every number it
// shows comes from one `status` message to the service worker, which is the
// only place that knows anything; the panel is a renderer. That matters because
// the worker is recycled constantly and the panel is not - a panel that cached
// would be showing a different session's numbers within a minute.
//
// It also carries the toolbar click's old job. An action with a
// `default_popup` never fires `chrome.action.onClicked`, so "click the button
// to snapshot" becomes "click the button, press Take snapshot" - which is why
// that button is the primary one and is autofocused: with the panel open, Enter
// takes the snapshot.

"use strict";

const $ = (id) => document.getElementById(id);

const REFRESH_MS = 2000;

/** Ask the service worker something. A dead worker answers by not answering. */
const ask = (message) =>
  new Promise((resolve) => {
    try {
      chrome.runtime.sendMessage(message, (response) => {
        if (chrome.runtime.lastError) {
          resolve({ ok: false, detail: chrome.runtime.lastError.message });
          return;
        }
        resolve(response || { ok: false, detail: "the service worker did not answer" });
      });
    } catch (e) {
      resolve({ ok: false, detail: String(e && e.message ? e.message : e) });
    }
  });

/** The tab the button will snapshot. Resolved HERE because "current window"
 *  means the window this panel hangs off, which the service worker cannot know. */
async function activeTabId() {
  const query = async (q) => {
    try {
      return (await chrome.tabs.query(q)) || [];
    } catch (_) {
      return [];
    }
  };
  let tabs = await query({ active: true, currentWindow: true });
  if (tabs.length === 0) tabs = await query({ active: true, lastFocusedWindow: true });
  return tabs.length > 0 ? tabs[0].id : null;
}

function ago(ms, nowMs) {
  if (ms == null) return "never";
  const seconds = Math.max(0, Math.round((nowMs - ms) / 1000));
  if (seconds < 2) return "just now";
  if (seconds < 90) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 90) return `${minutes} min ago`;
  return `${Math.round(minutes / 60)} h ago`;
}

function since(ms, nowMs) {
  if (!ms) return "—";
  const seconds = Math.max(0, Math.round((nowMs - ms) / 1000));
  if (seconds < 90) return `${seconds}s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 90) return `${minutes} min`;
  return `${Math.round(minutes / 60)} h`;
}

// The badge's state, said in words.
//
// It is READ from `report.badge.state`, not re-decided here (review N-2). This
// function used to re-implement `badgeFor`'s policy from `pending` and
// `relay_up`, which meant the four-state rule existed twice and the copy in
// this file was covered by no test at all - the two agreed, but nothing said
// they had to. What lives here now is WORDING and a class name, which is the
// panel's own business; WHICH state we are in is decided once, in the worker.
//
// An unrecognised state falls back to unknown rather than throwing: this file
// and background.js ship together, so a mismatch means a half-applied reload,
// and "not checked yet" is the honest thing to say when the answer is
// unreadable.
const STATES = {
  draining: { klass: "state-draining", text: (r) => `Connected — sending ${r.pending}` },
  parking: { klass: "state-parking", text: (r) => `Relay down — ${r.pending} parked` },
  waiting: { klass: "state-parking", text: (r) => `${r.pending} waiting — relay unchecked` },
  live: { klass: "state-live", text: () => "Connected — recording" },
  offline: { klass: "state-offline", text: () => "Relay not answering" },
  unknown: { klass: "state-unknown", text: () => "Relay not checked yet" },
};

function stateOf(report) {
  const name = report.badge && STATES[report.badge.state] ? report.badge.state : "unknown";
  return { klass: STATES[name].klass, text: STATES[name].text(report) };
}

function render(report) {
  const now = report.now_ms;
  const session = report.session || {};
  const state = stateOf(report);

  $("state").className = `state ${state.klass}`;
  $("state-text").textContent = state.text;

  $("table").textContent = session.last_table ? `#${session.last_table}` : "none seen yet";
  $("delivered").textContent = String((session.packets_delivered || 0) + (session.snapshots_delivered || 0));
  $("snapshots").textContent = String((session.snapshots_delivered || 0) + (session.snapshots_saved || 0));
  $("outbox").textContent =
    report.pending > 0 ? `${report.pending} waiting (max ${report.outbox_max_items})` : "empty";
  $("relay").textContent =
    report.relay_up === null
      ? report.relay_origin
      : `${report.relay_up ? "up" : "down"} · ${ago(report.relay_checked_at_ms, now)}`;
  // `started_because` is surfaced rather than merely stored (review N-5): it is
  // what distinguishes "these counts are low because the browser just started"
  // from "…because the extension was reloaded mid-game".
  $("session").textContent = session.started_at_ms
    ? `${since(session.started_at_ms, now)} · ${session.started_because || "unknown"}`
    : "—";

  // Only shown when there is something to say, so its presence is the signal.
  const trouble = [];
  if (session.captures_dropped) trouble.push(`${session.captures_dropped} capture(s) DROPPED — outbox was full.`);
  if (session.captures_refused) trouble.push(`${session.captures_refused} capture(s) refused by the relay.`);
  if (session.snapshots_saved) trouble.push(`${session.snapshots_saved} snapshot(s) saved to Downloads instead.`);
  if (report.relay_up === false && report.pending === 0) {
    trouble.push(`No relay on ${report.relay_origin}. Run: cargo run -p yokohama-bridge --bin bridge-relay`);
  }
  $("trouble").hidden = trouble.length === 0;
  $("trouble").textContent = trouble.join(" ");
}

function say(text, bad) {
  $("message").textContent = text || "";
  $("message").className = bad ? "message bad" : "message";
}

async function refresh(force) {
  const answer = await ask({ type: "status", force: !!force });
  if (!answer || !answer.ok || !answer.report) {
    $("state").className = "state state-unknown";
    $("state-text").textContent = "Service worker not answering";
    say(answer && answer.detail ? answer.detail : "Reload the extension.", true);
    return;
  }
  render(answer.report);
}

$("snapshot").addEventListener("click", async () => {
  const button = $("snapshot");
  button.disabled = true;
  say("Asking the page…");
  const tabId = await activeTabId();
  const result = await ask({ type: "snapshot-now", tabId });
  say(result && result.detail ? result.detail : "Done.", !(result && result.ok));
  button.disabled = false;
  // The snapshot travels page -> content script -> worker, so the counters move
  // a moment after the button does.
  setTimeout(() => refresh(false), 700);
  setTimeout(() => refresh(false), 1800);
});

$("recheck").addEventListener("click", async () => {
  say("Checking…");
  await refresh(true);
  say("");
});

refresh(true);
setInterval(() => refresh(false), REFRESH_MS);
