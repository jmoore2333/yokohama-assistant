# Forge MCP — visual CLI testing (adopted 2026-07-21)

How agents *see* the CLI the way a tester does. `yokohama-cli` gates color and box-drawing on a
real TTY; piped Bash output is deliberately plain (byte-identical guarantee), so an agent
running the CLI through ordinary shell tools is blind to the render layer it is editing. The
**Forge MCP** (github.com/ferodrigop/forge) fixes that: it spawns real PTY sessions the agent
can type into and read back — colors on, layout exact, wizard scriptable.

Everything below was live-verified 2026-07-21 against Forge v0.9.0 on this machine; the
recipe and the sizing were re-verified against the shipped binary 2026-08-06 (Phase-3.5a T11).

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
create_terminal  {command: "/bin/zsh", cwd: <repo root>, cols: 200, rows: 50,
                  name: "yokohama-cli", tags: ["yokohama"]}
write_terminal   {id, input: "cargo run -p yokohama-cli"}     # prebuild first — see notes
wait_for         {id, pattern: "Mode\\? \\(sim/live\\)"}
write_terminal   {id, input: "sim\n2\nred\nblue\n0\n42\n<scratchpad>/game.json\n1\n1\n\n"}
                  # keep answers are 1 or 2 (T5's 1-based ruling), one per seat;
                  # the trailing blank answers the "Bot seats?" question (3.2) — blank = none
wait_for         {id, pattern: "confirm / edit"}
write_terminal   {id, input: "confirm"}
wait_for         {id, pattern: "Turn 1"}
# ... then one write_terminal per REPL command, read_terminal after each ...
write_terminal   {id, input: "quit"}
close_terminal   {id}
```

- **The whole setup wizard scripts in one multi-line write** (answers queue in the PTY line
  buffer; the wizard consumes them one question at a time). Sim 2P answer order: mode, players,
  color×seats, start seat, seed, save path, keep×seats, bot seats. 4P: same with 4 colors / 4
  keeps. Batching REPL moves (`"1\n1\n1"`) works the same way — but only batch moves whose
  prompts you can predict; when unsure, one write per read.
- **Every numbered list starts at `[1]`** (T5's ruling, `screen_model::NUMBER_BASE`), so a
  scripted answer of `0` is now out of range everywhere except the guided retreat's `[0-n]`,
  which asks for a QUANTITY and where `0` legitimately means "recover none".
- **Tile, token and achievement rosters carry STABLE numbers** (T7): a type's number is its
  place in the whole set, fixed for the pass, and it does NOT close up as the list shrinks. A
  script that repeated the same number to walk down a menu was reading the pre-T7 behaviour —
  repeating `1` now re-picks the same type until its copies run out, and is then refused by
  name. Script the numbers you actually want, or type names.
- **Point the save path at the session scratchpad**, never the repo root (autosave writes after
  every move; a blank answer would drop `yokohama-game-<seed>.json` into the repo).
- **`cargo build -p yokohama-cli` (plain Bash) before the session** so `cargo run` inside the
  PTY starts in ~0.1s instead of compiling; every `wait_for` above then returns instantly.
- Sizing (re-measured 2026-08-06 against the redesigned frame, and pinned by `guide_sync`'s
  `section2_frame_widths_match_the_composed_screen` / `..._heights_...`): the COMPOSED FRAME —
  not the board alone — is **198 cols × 48 rows at 2 players, 175 × 49 at 3, and 200 × 48 at 4**
  (the board grids themselves are 120 / 150 / 180 cols). So **create terminals at 200×50**,
  which is the design floor the whole redesign is drawn to; the 140×50 earlier revisions of this
  file recommended shears every count. In play the 3- and 4-player frames hold at 60 rows by
  construction, while the 2-player one is ragged and keeps growing (67 rows on the deepest walk
  the tests run) — `panel` and `screen` buy those rows back.
- **The full-screen surfaces replace the frame rather than sitting under it**, so `read_screen`
  after `ports` / `labs` / `tracks` / `sites` / `goals` / `hand` / `log` / `?` shows that
  surface alone on a cleared screen. Press Enter (or `cancel`) to come back to the prompt
  underneath — and note that any OTHER line closes the view and then RUNS, so a stray digit
  applies an option.

## Capturing a session to a file (the T12 visual-pass recipe)

`read_screen` is for *looking*. When the output has to become a committed
artifact, do not transcribe it — run the binary under `script` INSIDE the Forge
PTY, so the CLI still sees a real terminal and every byte is logged:

```
write_terminal {id, input: "script -q <scratch>/raw.log cargo run -q -p yokohama-cli"}
# ...drive the session as usual...
write_terminal {id, input: "quit"}
```

Then split the log on the frame boundary and strip the escapes. Every composed
frame opens with `ESC[2J ESC[H`, so that pair is the split point and each piece
is one screen; `docs/plans/2026-08-05-phase3.5a-mocks/captures/t12-*.txt` were
cut exactly this way. Measure widths on the EMITTED lines rather than on the
viewport: `script` records what the program wrote, before the terminal wrapped
it, which is the only way to see an overflow instead of its symptom.

**Sizing surprises worth knowing before you conclude anything from a wide run.**
The card indexes adapt above the floor through `screen::index_width`, but the
shipped card data hides it at the obvious test size: at **240** columns nothing
repacks, because a 49-column tech/site card fits 4 per row at both 200 and 240,
and the order index — which would take 9 — has no flag group larger than 8. The
discriminating cell is the **site index at 250**, where the 24-card ungrouped
run goes 4 per row to 5. A wide run that "looks identical" is arithmetic, not a
broken measurement.

## Standing PTY checks (properties no `cargo test` can reach)

Almost every honesty property in this driver is pinned by a test. These are the ones that
**cannot** be: they are about what a real terminal does with bytes, and the suite has no
terminal. Re-run them in a PTY whenever `crates/yokohama-cli/src/input.rs` or the `rustyline`
declaration in `crates/yokohama-cli/Cargo.toml` changes.

- **Type-ahead survives.** The whole wizard in ONE write must consume every answer:

  ```
  write_terminal {id, input: "sim\r2\rred\rblue\r1\r31337\r"}
  ```

  Six answers, one write, no waiting between them — the session must reach the **save-path**
  question. If it stops at an earlier question, the leftover bytes were dropped with the read
  buffer and `rustyline`'s `buffer-redux` feature is off (T9 review, MUST-1). What the suite
  *can* say about this is only that the feature is declared —
  `input::the_editor_arm_keeps_its_read_buffer_between_lines` — because the type that changes
  shape lives behind a private module in rustyline. The behaviour is this check.

- **↑ recall, and Ctrl-C not resurrecting.** Type a line, Enter, then ↑ — the line comes back.
  Type a second line, press **Ctrl-C** instead of Enter, then ↑ — the abandoned line must NOT
  be offered, and the game must still be alive at its prompt. The queue-and-history logic
  itself is pinned without a terminal
  (`input::a_delivered_line_enters_the_history_and_an_abandoned_one_never_does`, which drives
  the shipped loop through a scripted stand-in); what needs the PTY is that rustyline's key
  bindings reach that logic at all.

- **A multi-line paste is answered line by line.** Paste (⌘V, not a scripted write — bracketed
  paste is what makes this different) a two-line block such as `sim` + `2` at the mode
  question. Both questions must be answered, exactly as the same two lines behave in a pipe.
  The SPLIT is pinned without a terminal
  (`input::a_pasted_block_reaches_the_parser_the_way_a_pipe_would`); the PTY is what proves the
  block arrives as one bracketed paste in the first place.

- **The frame is undisturbed by editing.** Insert mid-line, backspace, and use ↑ across the
  wizard→REPL boundary: the composed frame above the prompt must never repaint or shear (the
  empty `EDIT_PROMPT` geometry, pinned at the frame end by
  `input::the_editor_prompt_is_empty_because_the_frame_ends_at_column_zero`).

## Housekeeping rules

- **Always `close_terminal` when done** (`close_group` by tag after a multi-terminal run).
  Sessions idle out after 30 min, max 10 concurrent — don't rely on the timeout.
- Ring buffer is 1 MB/session by default; a long full-game session can overflow it (watch
  `droppedBytes` in `read_terminal`; raise `bufferSize` at create, max 10 MB).
- Visual checks **complement** the automated gates, never replace them: the debug suites,
  release gates, and `guide_sync` stay the merge bar. Forge is how you *look at* what you
  built; honesty properties (NO_COLOR, piped-plain byte-identical, engine-legality matching)
  are proven by tests, not by looking.

## Terminal lifecycle (from the Forge project's skill)

Condensed from the Forge project's own "Terminal Lifecycle Management" skill (provided by
Jeremy 2026-07-21). The source brief lived in the working area under `.superpowers/sdd/`,
which is gitignored in full, so it is NOT in a clone and is deliberately not cited as a
retrievable source — the rules below are the whole of what this project relies on:

- **Before `create_terminal`, always `list_terminals` first** and reuse a session when one fits
  — filter by tag (`list_terminals({ tag: "yokohama" })`). Don't accumulate idle sessions.
- **Tag on create.** `name` is the human-readable label; `tags` group sessions for bulk
  operations — `close_group({ tag })` kills a whole group and `list_terminals({ tag })` filters.
  Agent / worktree / `run_command` sessions are auto-tagged.
- **Four read models, each for its job:** `read_terminal` = incremental new output since your
  last read (progress checks, polling, command results — token-efficient); `read_screen` =
  current viewport snapshot, ANSI stripped (TUI / redraw apps and any cursor-based UI, where
  incremental reads don't make sense); `read_multiple` = several sessions in one call
  (dashboard-style status); `grep_terminal` = regex over the whole buffer (find errors/warnings
  or extract a port/URL/count without re-reading everything). For this repo's CLI, that maps to
  "The two read models" above — `read_screen` to judge layout, `read_terminal` to prove the SGR.
- **Clean up what you no longer need** — `close_terminal({ id })` for one, `close_group({ tag })`
  for a run; `run_command` sessions auto-clean.
- `list_templates` / `create_from_template` spin up pre-configured sessions.

## Baseline evidence captured at adoption (pre-topology-fix)

**Historical (2026-07-21, pre-fix).** The staggered renderer and the composed frontend redesign
LANDED on branch `board-topology-cli-graphics` (see `guidingpath.md` Current state and
`docs/cli-guide.md` for the current frontend). The items below are the "before" state, kept for
reference — the lattice board, thin color, 186-flat-option Placement, and bare-`X` dummy are all
superseded.

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
