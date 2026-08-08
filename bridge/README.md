# `bridge/` — the BGA→CLI bridge, browser half

- `extension/` — the MV3 Chrome extension, loaded unpacked. Read-only toward
  Board Game Arena: it copies responses the page already receives and snapshots
  the page's own `gamedatas`. It never submits a move, a chat message, or any
  other write.
- `console-snippet.js` — the no-extension debugging harness. Paste it into the
  game iframe's console to save one `gamedatas` snapshot.
- `tests/` — the extension's test suite (below).

## Tests

The extension's JavaScript is not covered by any `cargo` command, so it gets its
own. Plain `node`, no dependencies, nonzero exit on failure:

```bash
node bridge/tests/outbox_race.js && node bridge/tests/wrapper_transparency.js
```

- `outbox_race.js` — the H0 review's **M-1** regression: several captures
  arriving in one tick with the relay down must all survive. Against the defect
  it reported 1/5; against the fix, 5/5 in arrival order.
- `wrapper_transparency.js` — that the page cannot tell its `fetch` and
  `XMLHttpRequest` have been wrapped (same Response object, same rejection, its
  own body read still works, our failures never surface to it), plus the two
  completeness findings: **S-6**, a `URL` instance argument has `.href` and not
  `.url` and was being missed; **S-1**, a reused XHR object captured its second
  response twice and its third three times.

Both load the shipped file in a stubbed environment — a stub models only what
the wrapper touches, so a green run is evidence about this logic, not proof the
extension works in Chrome. That is what the first live capture is for.

The Rust half is `crates/yokohama-bridge` (the capture relay). Install and run
instructions: **`docs/bridge-setup.md`**. Design authority:
`docs/plans/2026-08-07-bga-bridge-design.md`.
