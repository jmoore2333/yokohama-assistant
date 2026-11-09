# BGA bridge — install and run (Phase 3.5b, H0)

Capture your own Board Game Arena Yokohama table to a local file. Two halves: a
Chrome extension that watches the page, and a relay that writes what it sees.

**Read-only, always.** The extension never submits a move, a chat message, or
any other write to BGA. It copies responses the page has already received and
snapshots the page's own game state. Nothing leaves your machine.

Design authority: `docs/plans/2026-08-07-bga-bridge-design.md`.

---

## 1. Start the relay

From the repository's root:

```bash
cargo run -p yokohama-bridge --bin bridge-relay
```

It prints where it is listening and where it writes:

```
bridge-relay listening on http://127.0.0.1:8787
  captures -> /path/to/yokohama-assistant/bridge-captures
  health   -> curl http://127.0.0.1:8787/health
  read-only: this process never talks to Board Game Arena.
```

Leave it running. Options: `--dir <path>` to write elsewhere, `--port <n>`,
`--help` for the rest. It binds loopback only and refuses to do otherwise.

**Changing the port means changing it in three more places**, and missing any
one leaves the extension silently unable to reach a relay that is running fine:

| Where | What |
|---|---|
| `bridge/extension/background.js` | `RELAY_ORIGIN` — the address it POSTs to |
| `bridge/extension/manifest.json` | the `http://127.0.0.1:8787/*` host permission |
| `crates/yokohama-bridge/src/lib.rs` | `DEFAULT_PORT`, so the flag is not needed next time |

After editing the extension files, reload it in `chrome://extensions` **and**
reload the BGA tab.

Confirm it is alive:

```bash
curl http://127.0.0.1:8787/health
```

## 2. Load the extension

1. Open `chrome://extensions`.
2. Turn on **Developer mode** (top right).
3. **Load unpacked** → select the `bridge/extension` folder of your clone.
4. Pin "Yokohama BGA bridge (capture)" to the toolbar so you can see its badge.

It asks for two hosts and one permission: `boardgamearena.com` (to watch the
page), `127.0.0.1:8787` (to reach the relay), and `storage` (to hold captures
if the relay is down). The connected indicator (§3a) added no permission and no
host to that list.

> **Already had it loaded?** The connected indicator ships in extension version
> **0.4.0**. Reload the extension once in `chrome://extensions` to get it — and
> then reload the BGA tab, because reloading the extension orphans the wrapper
> in any page that was already open.

## 3. Open the table

Open your BGA Yokohama table — e.g. `https://boardgamearena.com/table?table=895608640`
and enter the game. **If the tab was already open, reload it**: the capture
wrapper installs at page load and cannot attach to a page that is already
running.

Captures start as soon as the page fetches its move log, **and live moves then
bank continuously as they happen** — the extension watches BGA's realtime socket
as well as the history fetch, so you do not need to sit reloading during play.

Both of those depend on the same thing: **the tab must have been loaded after the
extension was**. The wrapper installs at page load and cannot attach to a page
already running. Reloading is also still worth doing **once at game end** — the
history endpoint is cumulative, so a final reload re-banks the whole game and
covers anything the socket missed. Cheap, and it is the belt to the socket's
braces.

Watch captures arrive:

```bash
tail -f bridge-captures/895608640.ndjson
```

The relay's own console prints a line per capture:

```
capture table=895608640 +18422 B (file now 214883 B)
```

## 3a. Is it recording? (the connected indicator)

The toolbar button answers that without you looking at a single file.

**The badge, at a glance:**

| Badge | Colour | Means |
|---|---|---|
| ● | green | Relay connected, nothing waiting. This is the "it is recording" state. |
| ○ | grey | Relay is **not** answering — and nothing has been captured yet either. Start the relay. |
| `17` | red | Relay is **not** answering and 17 captures are **parked** in the outbox. Start the relay; they go out on the next capture. |
| `17` | amber | Relay **is** answering and the 17-deep backlog is going out now. Transient. |
| ? | grey | Not checked yet — the extension has just started. Click the button to check. |

**Click it** for the detail: the table number, captures delivered this session,
snapshots taken, outbox depth, and the relay state with the age of the reading.
It also warns about the things you would otherwise only find later — captures
dropped because the outbox filled, captures the relay refused, and snapshots
that ended up in `~/Downloads` instead.

Two buttons: **Take snapshot** (§4 — the primary one, and it is focused when
the panel opens, so click-then-Enter is the whole gesture) and **Recheck relay**.

How current is the badge? Relay state is refreshed by every delivery (free —
each capture already POSTs to the relay), by a 30-second poll while a BGA tab
is open, and by opening the panel, which always probes. Chrome recycles the
extension's service worker after about thirty idle seconds, and a recycled
worker keeps whatever badge it last painted — so if the relay dies while
nothing is happening, the badge can lag. **The panel is the authority; the
badge is the hint.** Opening it always gives you a fresh answer.

## 4. Take a snapshot

Click the extension's toolbar button, then **Take snapshot** (it is already
focused, so Enter works). It captures `gameui.gamedatas` — the full
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
- `kind: "packets"` — `payload.body_text` is a notification response or a
  realtime socket frame **verbatim, as a string**, plus its `url` and
  `captured_at_ms`. `payload.source` says which transport it came from:
  `"fetch"` / `"xhr"` for the history, `"websocket"` for live moves. A binary
  socket frame is recorded as a stub (`note: "non-text frame"` with a
  `byte_length`) rather than decoded, and a runaway socket parks one
  `note: "volume guard tripped"` line and pauses for ten seconds.

`received_at_ms` is when the relay took delivery; `payload.captured_at_ms` is
when the page saw it. They differ when a capture waited in the extension's
outbox, so order a replay by the capture time, not the receive time.

**Delivery is at-least-once, so the file may contain duplicates.** The extension
re-sends anything it is not sure landed, and Chrome can recycle its service
worker mid-send. This is deliberate — losing a capture is unrecoverable, sending
one twice is not — but it means anything reading the file must dedupe on the
payload's own identity, never on `received_at_ms`, which differs across a
re-send by construction.

---

## 6. Live-game runsheet (H3 — the sheet a gate run follows)

Everything above, in the order a real game needs it, plus the three
disciplines the captured corpus proved matter. The plan's measured reason for
step 3 (plan §0.5 / §6a): a snapshot taken mid-game cannot recover the initial
Port and Laboratory cards backwards, so **a snapshot before move 1 is what
makes a move-1 replay possible at all**.

1. **Before sitting down**: start the relay (§1). Load — or if it was already
   loaded, **reload** — the extension in `chrome://extensions` (§2). Reloading
   the extension invalidates any BGA tab that was already open, so do it
   before the table tab exists, not after.
2. **Open the table tab** (§3), *after* the extension is loaded. If in any
   doubt about the ordering, reload the tab: the wrapper installs at page load
   and can never attach to a page already running. Confirm capture lines are
   arriving — the toolbar badge should be a **green ●** (§3a), and
   `tail -f bridge-captures/<table>.ndjson` or the relay console shows them
   landing.
3. **Snapshot at table open, BEFORE move 1** (§4): click the toolbar button
   and press **Take snapshot** while the table is still in setup. This banks
   the initial Port A six and Lab A six — the one observation the notification
   feeds never repeat. A game whose first snapshot is mid-game imports as a
   bounded replay, not a move-1 replay. The panel's **Snapshots taken
   (session)** count going **up by one** is your confirmation it happened —
   it counts the whole browser session, not this table, so it reads 0 → 1 only
   for the first snapshot after a browser start or an extension reload.
4. **Start the follower** in a repo terminal:

   ```bash
   cargo run -p yokohama-cli -- import bridge-captures/<table>.ndjson --follow --out bga-<table>.json
   ```

   It renders the game as a read-only screen and redraws as moves land. A
   `HOLD` line is not a crash: it names what the pipeline cannot yet pass
   (snapshot not yet arrived, evidence pending, a documented rules
   divergence), prints once, and the follower resumes on its own when the
   capture provides more. Leave it running; Ctrl-C and re-running it later
   reaches the identical state from the same file.
5. **Extension reload discipline, mid-game**: do not reload the extension
   during play unless something is broken. If you must (or Chrome updates
   it), **reload the BGA tab immediately after** — an extension reload
   orphans the page's wrapper ("Extension context invalidated"), and until
   the tab reloads, nothing is being captured. After any tab reload, take a
   fresh snapshot (a free checkpoint anchor; the reload also re-banks the
   cumulative history, covering whatever the gap missed).
6. **The reload-at-end ritual**: when the game ends, reload the tab **once**
   and take a final snapshot. The history endpoint is cumulative, so the
   reload re-banks the entire game in one response — the belt to the socket's
   braces — and the end-of-game snapshot plus BGA's `finalScoring` table are
   what the final-score cross-check reads. The follower prints that
   cross-check (engine total vs BGA total, per seat) when the replay reaches
   game end, and exits.
7. **Afterwards**: the capture file is the irreplaceable artifact — it stays
   in `bridge-captures/` (gitignored). The follower's `--out` record is a
   normal live-mode save: `cargo run -p yokohama-cli -- bga-<table>.json`
   opens it in the driver.

---

## Verifying a change to the bridge

Both halves have their own suite, and neither runs the other's:

```bash
cargo test -p yokohama-bridge                              # the relay
node bridge/tests/outbox_race.js \
  && node bridge/tests/wrapper_transparency.js \
  && node bridge/tests/safe_table_parity.js \
  && node bridge/tests/connected_indicator.js              # the extension
```

The `node` four need no dependencies and exit nonzero on failure. They cover
the outbox's behaviour when the relay is down, that the page cannot tell its
`fetch`/`XMLHttpRequest`/`WebSocket` have been wrapped, that table ids the
relay would refuse are never emitted, and the connected indicator (badge map,
session counters — including that they survive a service-worker recycle — the
snapshot-on-click contract, and that no new permission was taken).

---

## Troubleshooting

**The toolbar badge shows a RED number.** That many captures are parked because
the relay is not answering. Start it — the backlog flushes on the next capture,
within 30 seconds of the poll, or immediately if you open the panel and press
**Recheck relay**. If the badge keeps climbing, check that the relay's port
matches `RELAY_ORIGIN` in `background.js`. (An **amber** number is the same
backlog on its way out with the relay up, and needs nothing from you.)

**The badge is a grey ○ or ?.** The relay is not answering, or has not been
asked yet. Start it (§1) and press **Recheck relay** in the panel.

**The badge says green ● but nothing is landing.** The badge reports the
RELAY; captures come from the PAGE. Open the panel — if **Captures delivered
(session)** is not moving, it is the page half: reload the BGA tab (next
entry).

**Nothing is being captured.** Reload the BGA tab. The wrapper installs at page
load, so a tab opened before the extension was loaded (or before it was
reloaded) has no wrapper in it.

**The panel says "Service worker not answering".** Chrome recycled it and it
did not come back. Reload the extension in `chrome://extensions`, then reload
the BGA tab.

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

The relay half was verified against the running binary (2026-08-07). The
extension half was proven live on 2026-08-08 against table #895724349: the
fetch/XHR, snapshot, and WebSocket paths all banked a complete real game end
to end (execution ledger, H0 rows). §6's runsheet is the H3 procedure the
gate run follows; its snapshot-before-move-1 step is the one thing no later
diligence can recover.

§3a's connected indicator (extension 0.4.0, 2026-08-08) is verified by
`bridge/tests/connected_indicator.js` and by rendering the panel in a real
Chrome in both colour schemes. Read that second clause narrowly: the eye
confirmed the panel LAYS OUT in both schemes, and it missed a primary button
at 1.95:1 in the dark one (review N-4, fixed) — which is why the contrast of
both schemes is now COMPUTED by the test rather than looked at. It has **not**
yet been watched during a live game — the first recorded game after this
change is what confirms the badge reads true against a real table, and the
reading to distrust first is a green ● that lags a relay killed while nothing
was being captured (the recycle case §3a names).

## Recording future games (the steady-state process, owner-ruled 2026-08-08)

Per game, any number of sittings (async-friendly — every table-page load banks a full
cumulative history fetch):

1. Relay up: `cargo run -p yokohama-bridge --bin bridge-relay` (leave running while playing).
2. Open the table; check the badge is a **green ●** (§3a), then **snapshot (toolbar
   button → Take snapshot) before move 1** — the game-start anchor.
3. Play. Realtime moves ride the socket; async sittings re-bank history on every open.
   Glance at the badge when you sit down: green ● is recording, a red number is a relay
   you forgot to start.
4. At the final score screen: snapshot again, then reload the tab once (belt-and-braces
   cumulative copy). Each table auto-writes its own `bridge-captures/<table>.ndjson`.
5. Later, in any session: `yokohama-cli import bridge-captures/<table>.ndjson`. Whatever the
   translator cannot express **holds with a named refusal** — that named list IS the
   "updates required for future inspection" report; bring it in and the holds become the
   next fix list, exactly as the 2026-08-08 gate games' seven discoveries did.

Caveat: with the relay DOWN, the extension's outbox parks at most 400 items / 6 MB
(drop-newest) — enough for a sitting's history fetches, not days of socket traffic. If in
doubt, reload the tab once with the relay up: cumulative history recovers everything for
turn-based play. The badge now tells you before it matters: a red number IS the parked
count, and the panel says so in words.

The connected indicator this section used to carry as a deferred item **shipped in extension
version 0.4.0** (2026-08-08) and is documented at **§3a** — badge state for relay
reachability, click for the table number and this session's capture/snapshot counts.
Reload the extension once in `chrome://extensions` to get it.
