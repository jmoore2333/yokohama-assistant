# `bridge/` — the BGA→CLI bridge, browser half

- `extension/` — the MV3 Chrome extension, loaded unpacked. Read-only toward
  Board Game Arena: it copies responses the page already receives and snapshots
  the page's own `gamedatas`. It never submits a move, a chat message, or any
  other write.
- `console-snippet.js` — the no-extension debugging harness. Paste it into the
  game iframe's console to save one `gamedatas` snapshot.

The Rust half is `crates/yokohama-bridge` (the capture relay). Install and run
instructions: **`docs/bridge-setup.md`**. Design authority:
`docs/plans/2026-08-07-bga-bridge-design.md`.
