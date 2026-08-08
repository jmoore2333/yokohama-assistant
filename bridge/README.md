# `bridge/` — the BGA→CLI bridge, browser half

- `extension/` — the MV3 Chrome extension, loaded unpacked. Read-only toward
  Board Game Arena: it copies responses the page already receives and snapshots
  the page's own `gamedatas`. It never submits a move, a chat message, or any
  other write.
- `console-snippet.js` — the no-extension debugging harness. Paste it into the
  game iframe's console to save one `gamedatas` snapshot.
- `tests/` — the extension's test suite (below).

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
node bridge/tests/outbox_race.js && node bridge/tests/wrapper_transparency.js
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

Both load the shipped file in a stubbed environment — a stub models only what
the wrapper touches, so a green run is evidence about this logic, not proof the
extension works in Chrome. That is what the first live capture is for.

The Rust half is `crates/yokohama-bridge` (the capture relay). Install and run
instructions: **`docs/bridge-setup.md`**. Design authority:
`docs/plans/2026-08-07-bga-bridge-design.md`.
