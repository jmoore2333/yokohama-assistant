# BGA bridge — install and run (Phase 3.5b, H0)

Capture your own Board Game Arena Yokohama table to a local file. Two halves: a
Chrome extension that watches the page, and a relay that writes what it sees.

**Read-only, always.** The extension never submits a move, a chat message, or
any other write to BGA. It copies responses the page has already received and
snapshots the page's own game state. Nothing leaves your machine.

Design authority: `docs/plans/2026-08-07-bga-bridge-design.md`.

---

## 1. Start the relay

```bash
cd /Users/jmoore/Documents/Github/yokohama-assistant
cargo run -p yokohama-bridge --bin bridge-relay
```

It prints where it is listening and where it writes:

```
bridge-relay listening on http://127.0.0.1:8787
  captures -> /Users/jmoore/Documents/Github/yokohama-assistant/bridge-captures
  health   -> curl http://127.0.0.1:8787/health
  read-only: this process never talks to Board Game Arena.
```

Leave it running. Options: `--dir <path>` to write elsewhere, `--port <n>` to
move the port (change `RELAY_ORIGIN` in `bridge/extension/background.js` to
match), `--help` for the rest. It binds loopback only and refuses to do
otherwise.

Confirm it is alive:

```bash
curl http://127.0.0.1:8787/health
```

## 2. Load the extension

1. Open `chrome://extensions`.
2. Turn on **Developer mode** (top right).
3. **Load unpacked** → select
   `/Users/jmoore/Documents/Github/yokohama-assistant/bridge/extension`.
4. Pin "Yokohama BGA bridge (capture)" to the toolbar so you can see its badge.

It asks for two hosts and one permission: `boardgamearena.com` (to watch the
page), `127.0.0.1:8787` (to reach the relay), and `storage` (to hold captures
if the relay is down).

## 3. Open the table

Open your BGA Yokohama table — e.g. `https://boardgamearena.com/table?table=895608640`
and enter the game. **If the tab was already open, reload it**: the capture
wrapper installs at page load and cannot attach to a page that is already
running.

Captures start as soon as the page fetches its move log. Watch them arrive:

```bash
tail -f bridge-captures/895608640.ndjson
```

The relay's own console prints a line per capture:

```
capture table=895608640 +18422 B (file now 214883 B)
```

## 4. Take a snapshot

Click the extension's toolbar button. It captures `gameui.gamedatas` — the full
own-perspective game state, roughly 30 KB — and delivers it to the relay.

If the relay is not running, the snapshot downloads as a `.ndjson.json` file
instead. That file is one line in the relay's exact format, so recover it with:

```bash
cat ~/Downloads/895608640-snapshot-*.ndjson.json >> bridge-captures/895608640.ndjson
```

## 5. What the capture file holds

One JSON object per line:

```json
{"received_at_ms":1754582400123,"kind":"snapshot","payload":{ … }}
{"received_at_ms":1754582401044,"kind":"packets","payload":{ … }}
```

- `kind: "snapshot"` — `payload.gamedatas` is the page's game state, plus
  `captured_at_ms` and `href`.
- `kind: "packets"` — `payload.body_text` is a notification response **verbatim,
  as a string**, plus its `url`, `status` and `captured_at_ms`.

`received_at_ms` is when the relay took delivery; `payload.captured_at_ms` is
when the page saw it. They differ when a capture waited in the extension's
outbox, so order a replay by the capture time, not the receive time.

---

## Verifying a change to the bridge

Both halves have their own suite, and neither runs the other's:

```bash
cargo test -p yokohama-bridge                                             # the relay
node bridge/tests/outbox_race.js && node bridge/tests/wrapper_transparency.js   # the extension
```

The `node` pair needs no dependencies and exits nonzero on failure. It covers
the outbox's behaviour when the relay is down, and that the page cannot tell its
`fetch`/`XMLHttpRequest` have been wrapped.

---

## Troubleshooting

**The toolbar badge shows a number.** That many captures are parked because the
relay is not answering. Start it — the backlog flushes on the next capture, or
immediately if you click the button. If the badge keeps climbing, check that
the relay's port matches `RELAY_ORIGIN` in `background.js`.

**Nothing is being captured.** Reload the BGA tab. The wrapper installs at page
load, so a tab opened before the extension was loaded (or before it was
reloaded) has no wrapper in it.

**"Extension context invalidated" in the page console.** You reloaded the
extension while the tab was open. Reload the tab.

**Look at the logs.** `chrome://extensions` → the extension's **service worker**
link opens the console where delivery problems are reported. Page-side problems
appear in the game iframe's own console.

**No extension at all.** `bridge/console-snippet.js` saves one snapshot from the
DevTools console. Switch the console's frame selector from `top` to the game
frame (`…/yokohama?table=…`) first — `gameui` lives in the iframe. It cannot
capture the move log; that is what the extension is for.

---

## Status of this sheet

The relay half is verified: routes, refusals, and the NDJSON output were
exercised against the running binary (2026-08-07). **The extension half has not
yet been loaded in Chrome against a live table** — that is H0's owner-in-the-loop
step, and the first run should confirm that captures appear for table
#895608640 before anything downstream is built on them.
