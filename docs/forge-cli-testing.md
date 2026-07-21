# Forge MCP — visual CLI testing (adopted 2026-07-21)

How agents *see* the CLI the way a tester does. `yokohama-cli` gates color and box-drawing on a
real TTY; piped Bash output is deliberately plain (byte-identical guarantee), so an agent
running the CLI through ordinary shell tools is blind to the render layer it is editing. The
**Forge MCP** (github.com/ferodrigop/forge) fixes that: it spawns real PTY sessions the agent
can type into and read back — colors on, layout exact, wizard scriptable.

Everything below was live-verified 2026-07-21 against Forge v0.9.0 on this machine.

## Setup facts (this machine)

- Server: HTTP MCP at `http://127.0.0.1:3141/mcp`, configured **project-scoped** in
  `~/.claude.json` under this repo's path — any Claude Code session in this repo sees the
  `mcp__forge__*` tools. The server must be running first (Jeremy starts it; `health_check`
  answers if it's up).
- **Live dashboard for humans: `http://127.0.0.1:3141/`** — every terminal an agent drives is
  watchable there in real time. This is the parallel-beta-testing hook: Jeremy can watch a
  game as the agent plays it.
- Install is a **bun** global (`~/.bun/bin/forge`, node-pty 1.1.0 prebuilds).

### Known failure + fix: `posix_spawnp failed`

If every `create_terminal` fails with `Error: posix_spawnp failed`, the exec bit fell off
node-pty's macOS spawn helper (bun's installer drops it; a forge update/reinstall can bring it
back). Fix, no server restart needed:

```bash
chmod +x ~/.bun/install/global/node_modules/node-pty/prebuilds/darwin-*/spawn-helper
```

(Same disease as the repo's rustup story: a toolchain binary that isn't executable. Diagnosis
trail: MCP transport fine / `health_check` fine / only PTY spawn failing.)

## The two read models

- `read_terminal` — **raw incremental stream** since your last read: includes ANSI codes
  (`[31m` = red, `[1m` = bold), token-efficient. Use it to *verify color emission* and to
  follow move-by-move output.
- `read_screen` — the rendered viewport, ANSI stripped: exactly what a human sees, geometry
  intact. Use it to *judge layout* (the staggered board, column alignment, page furniture).

Use both: `read_screen` to see what the tester sees, `read_terminal` to prove the SGR codes
behind it. `grep_terminal` regex-searches the whole session buffer (returns line numbers) —
good for asserting "the width warning printed" without re-reading everything.

## Verified session recipe

```
create_terminal  {command: "/bin/zsh", cwd: <repo root>, cols: 140, rows: 50,
                  name: "yokohama-cli", tags: ["yokohama"]}
write_terminal   {id, input: "cargo run -p yokohama-cli"}     # prebuild first — see notes
wait_for         {id, pattern: "Mode\\? \\(sim/live\\)"}
write_terminal   {id, input: "sim\n2\nred\nblue\n0\n42\n<scratchpad>/game.json\n0\n0"}
wait_for         {id, pattern: "confirm / edit"}
write_terminal   {id, input: "confirm"}
wait_for         {id, pattern: "Turn 1"}
# ... then one write_terminal per REPL command, read_terminal after each ...
write_terminal   {id, input: "quit"}
close_terminal   {id}
```

- **The whole setup wizard scripts in one multi-line write** (answers queue in the PTY line
  buffer; the wizard consumes them one question at a time). Sim 2P answer order: mode, players,
  color×seats, start seat, seed, save path, keep×seats. 4P: same with 4 colors / 4 keeps.
  Batching REPL moves (`"0\n0\n0"`) works the same way — but only batch moves whose prompts you
  can predict; when unsure, one write per read.
- **Point the save path at the session scratchpad**, never the repo root (autosave writes after
  every move; a blank answer would drop `yokohama-game-<seed>.json` into the repo).
- **`cargo build -p yokohama-cli` (plain Bash) before the session** so `cargo run` inside the
  PTY starts in ~0.1s instead of compiling; every `wait_for` above then returns instantly.
- Sizing: ≥120 cols renders every current board without shear (2P 86 / 3P 120 / 4P 103; the
  staggered renderer will change these — re-check its width warning). 140×50 was comfortable.

## Housekeeping rules

- **Always `close_terminal` when done** (`close_group` by tag after a multi-terminal run).
  Sessions idle out after 30 min, max 10 concurrent — don't rely on the timeout.
- Ring buffer is 1 MB/session by default; a long full-game session can overflow it (watch
  `droppedBytes` in `read_terminal`; raise `bufferSize` at create, max 10 MB).
- Visual checks **complement** the automated gates, never replace them: the debug suites,
  release gates, and `guide_sync` stay the merge bar. Forge is how you *look at* what you
  built; honesty properties (NO_COLOR, piped-plain byte-identical, engine-legality matching)
  are proven by tests, not by looking.

## Baseline evidence captured at adoption (pre-topology-fix)

From the live 2026-07-21 session — the "before" pictures the board-topology + CLI-graphics PR
changes, and the grounded UX findings behind its backlog (plan §7):

- 2P board renders as a **2×5 lattice**, 4P as **3×6** (Canal mid-bottom row) — the defect the
  PR replaces with staggered 4/3/2/1 and 6/5/4/3 pyramids.
- Color today is thin: player names (`[31mRed`, `[34mBlue`) and a bold phase name are the ONLY
  SGR output; wizard, board grid, option lists, and views are monochrome. Board cells are code
  strings (`sh ---- | th -`).
- First Placement is **186 flat options over 10 pages** — the `a`/`b` grammar is currently the
  only usable path (why "grammar-optional guided pickers" is on the backlog).
- Turn-1 ceremony: 4 of 7 inputs were forced single-option no-ops (End-AA ×2, Skip 5-Power,
  Skip building).
- Area-action preview can read as a bug: `Produce copper (power 2 -> +0 copper)` — faithful to
  tile data (differential-proven) but with no threshold context.
- 2P dummy on `tracks` is a bare `X` at faith-7 / customs-4 — finding D1 makes it a scoring
  occupant, so the display must name it, not just block it.
