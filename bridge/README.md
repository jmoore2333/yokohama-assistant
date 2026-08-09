# `bridge/` — the BGA→CLI bridge, browser half

- `extension/` — the MV3 Chrome extension, loaded unpacked. Read-only toward
  Board Game Arena: it copies responses the page already receives and snapshots
  the page's own `gamedatas`. It never submits a move, a chat message, or any
  other write.
  - `main-world.js` sees the page's objects, `bridge.js` couriers between the
    worlds, `background.js` is the only half that talks to the relay, and
    `popup.html`/`.js`/`.css` are the click detail (below).
- `console-snippet.js` — the no-extension debugging harness. Paste it into the
  game iframe's console to save one `gamedatas` snapshot.
- `tests/` — the extension's test suite (below).

## The connected indicator

The toolbar button says whether the bridge is recording, without opening a
file. Green ● connected · grey ○ relay not answering · a **red** number is that
many captures parked because the relay is down · **amber** is the same backlog
going out with the relay up · grey ? not checked yet. The count semantics the
badge already had are kept — the indicator adds colour and a never-empty state
to them rather than replacing them.

Clicking opens a small panel: table number, captures delivered this session,
snapshots taken, outbox depth, relay state with the age of the reading, and a
warning block for the things you would otherwise only discover later (drops,
refusals, snapshots that fell back to `~/Downloads`).

An action with a `default_popup` never fires `chrome.action.onClicked`, so the
toolbar click's old **snapshot + flush** moved onto the panel's primary button,
which is autofocused — click to open, Enter to snapshot. The worker path
underneath it is unchanged.

Relay state is learned free from every delivery's own POST outcome, by a 30s
poll armed only while a boardgamearena.com tab is open, and by the panel, which
force-probes `/health` on open. That is a plain `setTimeout` chain and **not**
`chrome.alarms`, because `alarms` is a permission: the extension still asks for
exactly `storage` plus the two hosts it always had. The cost of that choice,
stated rather than hidden — Chrome recycles the service worker after ~30 idle
seconds and a recycled worker keeps whatever badge it last painted, so the
badge can lag a relay that died while nothing was happening. The panel probes;
the badge hints. Session counters live in `chrome.storage.local` for the same
recycle reason.

Owner-facing description: **`docs/bridge-setup.md` §3a**.

## The two feeds

The extension captures from two transports, and they are not interchangeable.
`fetch`/`XMLHttpRequest` carry the **notification history** — the whole move log,
fetched when the page loads and re-fetched cumulatively on every reload.
The **WebSocket** carries moves as they happen during realtime play. The
2026-08-08 live run measured a completed turn producing zero fetch/XHR captures,
which is why both are wrapped: without the socket, a game only banks at reloads;
without the history, a game that started before the extension did is lost.

Socket frames ride the same envelope as everything else (`kind: "packets"`) with
`payload.source = "websocket"` and the observed socket URL, so H2 learns the real
host shape from data rather than from a guess made in the wrapper. Text frames
are recorded verbatim; a binary frame is recorded as a stub stating its size,
never decoded with an assumed encoding. Content is deliberately **not** filtered
— heartbeats and chat are captured too, because fidelity comes first and H2
dedupes — but a runaway is capped at 200 frames per 10 seconds, and the cap
announces itself with a note capture rather than going quiet.

## Tests

The extension's JavaScript is not covered by any `cargo` command, so it gets its
own. Plain `node`, no dependencies, nonzero exit on failure:

```bash
node bridge/tests/outbox_race.js \
  && node bridge/tests/wrapper_transparency.js \
  && node bridge/tests/safe_table_parity.js \
  && node bridge/tests/connected_indicator.js
```

- `outbox_race.js` — the H0 review's **M-1** regression: several captures
  arriving in one tick with the relay down must all survive. Against the defect
  it reported 1/5; against the fix, 5/5 in arrival order.
- `wrapper_transparency.js` — that the page cannot tell its `fetch`,
  `XMLHttpRequest` and `WebSocket` have been wrapped (same Response object, same
  socket instance, same rejection, its own body read still works, our failures
  never surface to it), plus the completeness findings: **S-6**, a `URL` instance
  argument has `.href` and not `.url` and was being missed; **S-1**, a reused XHR
  object captured its second response twice and its third three times; and
  **H0.1**'s socket pins — verbatim text frames, the stated (never decoded)
  binary stub, non-BGA and lookalike hostnames capturing nothing, and the volume
  guard.
- `safe_table_parity.js` — that `safeTable` is "in step" with the relay's
  `capture::safe_table_id` as its comment claims: every id the relay refuses
  (leading `-`/`.`/`_`, path separators) maps to one the relay accepts, so no
  capture the extension emits is silently 400'd and dropped from the outbox.
- `connected_indicator.js` — the badge map as a pure table (including that a
  parked outbox still shows its count in the colour it always used, and that no
  state paints an empty badge); the session counters as a pure reducer **and**
  through real deliveries, with the recycle cell that catches the obvious wrong
  build — counters in a module variable, which Chrome resets every thirty idle
  seconds; that the toolbar click's snapshot-and-flush still happens now that
  the popup has silenced `onClicked`; and the manifest's permissions and hosts
  asserted verbatim, because the cheap way to poll health is `alarms` and the
  cheap way to find the active tab is `tabs`, and neither is taken.

`mv3_stub.js` is the shared MV3 environment the three `background.js` tests
load the worker into — one model of the chrome surface rather than one per
file, since that surface grows.

They all load the shipped file in a stubbed environment — a stub models only
what the code touches, so a green run is evidence about this logic, not proof
the extension works in Chrome. That is what the first live capture is for.

The Rust half is `crates/yokohama-bridge` (the capture relay). Install and run
instructions: **`docs/bridge-setup.md`**. Design authority:
`docs/plans/2026-08-07-bga-bridge-design.md`.
