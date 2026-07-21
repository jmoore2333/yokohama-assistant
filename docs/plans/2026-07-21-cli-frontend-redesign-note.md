# CLI frontend redesign — design note (Task 6, 2026-07-21)

The owner's mandate: the CLI is no longer an engine tester, it is the game's v1-beta
frontend. Full redesign, not a facelift; the engine stays the sole authority. This note
fixes the interaction architecture and screen design before feature code (brief, Process
requirement 1).

## 1. Interaction architecture: line-based composed frames (not raw-mode TUI)

**Chosen: line-based command protocol + full-frame redraw on TTY.** After every input
line, the TTY sees a freshly composed screen (clear + banner strip + board + side panel +
message zone + prompt); piped stdin/stdout sees exactly the line protocol it sees today —
plain, complete, scriptable.

The tradeoff, named: a raw-mode TUI (crossterm) would give key-level navigation (arrow
keys, live cursors) at the cost of a new dependency, terminal-state management
(raw/cooked, resize, signal restore), a second input path to keep honest with the piped
protocol, and a test story that can't reuse the existing `Repl::step(&str) -> StepOutput`
suite (~170 line-protocol tests). The composed-frame route keeps ONE input grammar for
both TTY and pipes, keeps `Repl` a pure function of input lines (everything stays
testable as plain strings), and still delivers the "arranged screen" the owner asked for.
Interactivity comes from guided pickers (sub-prompts over the engine's legal list), not
from cursor movement. A future PR can layer raw-mode navigation on top without undoing
any of this.

**No new dependencies.** Frame composition is ANSI clear-screen + cursor-home
(`ESC[2J ESC[H`), emitted only in TTY mode by the same discipline `style.rs` uses for
color: a `screen::init_from_tty()` switch that `main.rs` flips once; every library path
and every piped run stays byte-plain. `NO_COLOR` keeps governing color only; frames
without color are still frames (toggle `screen` to get rolling text on a TTY).

## 2. Screen regions (TTY frame)

```
════════════════ Y O K O H A M A ═══════════════════════════   banner strip
┌────────────────┐┌────────────────┐┌───────────────┐
│ A00 Silk Mill  ││ A01 Church   * │ ... staggered  │           board: one box per tile,
│ P: R      [St] ││ P: -           │     pyramid    │           rows offset half a tile
│ R2 B1          ││ -              │                │           (brick pattern, widest row
│ sh ·B·R  th ·  ││ sh ····  th R  │                │           on top, from grid_position)
└────────────────┘└────────────────┘└───────────────┘
         ┌────────────────┐┌────────────────┐                ─ Players ────────────────
         │ ~~~~~~~~~~~~~~ ││ A05 Bank       │                 ▶ Red    3 yen   0 VP
         │ ~~~ CANAL ~~~~ ││ ...            │                   goods c1 s1 t1 f1 i1 ...
         │ ~ toll 1 yen ~ ││                │                 (side panel: right of board
         └────────────────┘└────────────────┘                  at 2P/3P, below it at 4P)
   * = unclaimed 5-Power token · [St] Station · ~ = canal      legend
────────────────────────────────────────────────────────────
Red: +1 copper                                                 message zone (delta lines /
Phase: Placement -> Movement                                   errors from the last input)
Turn 1 | Red | Movement | 3 yen 0 VP | c1 s1 t1 f1 i1 | hand 2
[0] Move President to Church (A01) — free                      prompt: numbered options
[1] Retreat President to hand — guided recovery
> enter a number, 'r' to retreat, or a command ('help')        hint
```

- **Board** (`render::board_grid`, rewritten): independent bordered boxes per tile,
  positioned purely from `AreaConfig::grid_position` (row, col) — row r indents by
  r × half-a-box, rows stack top row first. Works for ANY geometry the engine hands us
  (the real pyramid or a test fixture's 2×2), no pyramid assumption baked in. Tile box:
  header `A{id} {name} {*}`, presidents line (+ `[St]` station marker), assistants line,
  build marks. The Canal tile renders as water (`~` fill, `toll 1 yen`) — it has no site,
  no tokens. Box pitch 18 → board widths 72 (2P) / 90 (3P) / 108 (4P); the one-time
  width note re-derives from the rendered width exactly as today (>100 → 4P only).
- **Canal transits vs plain adjacency**: touching tiles ARE plain adjacency (free);
  crossings are the through-canal pairs, which the board legend names (`~ = canal:
  president may cross shore-to-shore, 1 yen to the bank`) and `sites` lists pair-by-pair
  (`canal->A07`) from `board_layout.canal_crossings` — never re-derived.
- **Side panel**: compact per-player box (~40 cols): name (▶ marks current player),
  yen/VP, goods, assistant zones, buildings + next TH take cost, President location, all
  painted. Right of the board when the board is ≤100 cols (2P/3P), below it at 4P (no
  terminal-size probing exists in std; the board's own width is the only honest signal).
  `panel` toggles it; `screen` toggles the whole composed mode.
- **Message zone**: the last step's delta/error lines — same strings as piped mode, so
  nothing exists only in the frame.

## 3. Privacy model

One thing in this game is genuinely hidden: starting orders (and hands contain them).
`privacy` toggles a hide flag; **default ON in live mode, OFF in sim**. When ON: hand
CONTENTS are masked everywhere (players panel and `hand` show counts + `(hidden —
'privacy' to toggle)`), so the laptop can be passed around a live table without leaking
the operator's kept starting order. Public surfaces (board, tracks, buildings, deltas of
public reveals, port displays) never mask — port takes happen face-up at the table.
Sim-mode setup previews both seats' deals by design (hot-seat simulation is a testing
surface, not a hidden-information game); the guide documents that.

## 4. Guided pickers (grammar-optional, engine-authoritative)

All pickers are `SubPrompt` entry helpers on the existing pattern (route/retreat): they
only ever assemble a candidate and exact-match it against `session.legal()` before
commit. No picker constructs state or prices anything the engine didn't already offer.

- **Placement**: the 186-option flat wall is replaced by grouped entries —
  `[0] Option A — pick up to 3 areas` (toggle areas by id off the board view; running
  fee preview + live legality check against the cached legal list; `confirm`),
  `[1] Option B — 2 assistants in one area` (pick area; if Telegram variants are legal,
  a follow-up numbered pick; fee preview inline), `[2] Place no assistants`. The
  `a`/`b` grammars remain first-class (never removed; pickers never the only path).
- **Assistant-move**: pending `AssistantMove` options group by SOURCE (`hand`, each
  occupied area) into one entry each + Decline; picking a source opens a numbered
  destination list. `m <from> <to>` grammar unchanged.
- **Retreat**: the existing guided flow, reworded into the same visual language.
- Fee/cost previews shown by pickers come from `describe_move` (see §5), so the picker,
  the grammar echo, and any numbered list can never disagree.

## 5. Decision-surface previews (single-sourced, test-pinned)

- **Placement fee** on Option A/B texts: `— fee N yen (1 to Blue, ...)`, computed by the
  same public-state read `credit_placement_fees` does (1 yen per assistant placed into
  an area holding an opponent President, per owner); a test pins the previewed number
  against the payer's actual money delta from a scratch apply, so drift with the engine
  is caught mechanically.
- **Employment Agency**: option text gains `— cost N yen` (2/shophouse + the printed
  4/5/6/7 TH progression via `TradingHouseState::take_cost`); same scratch-apply pin.
- **Church**: option text gains `-> faith N (power P + K payment kinds)` via the
  engine's own `calculate_power` + `calculate_church_faith`; pinned against the
  `ChurchSpace { total_faith }` choice the engine actually queues.
- **Production threshold**: `Produce copper (power 2 -> +0 copper, next +1 at power 3)`
  — the "next" clause probes the same engine reward fn upward to the next yield step.

## 6. Ceremony reduction

Bare Enter on a prompt whose displayed option list is exactly ONE entry applies that
entry through the normal commit path (recorded honestly in the log/record — owner chose
this over silent auto-advance). The prompt advertises it: `only option — press Enter`.
Numbered `0` keeps working identically (scripts unaffected). Empty input on multi-option
prompts stays a re-prompt.

## 7. What piped (non-TTY) mode looks like

Exactly today's shape: rolling plain text, zero ANSI bytes, every prompt ending in the
`> ` hint line, every action reachable (numbered options, grammars, picker sub-prompts —
all line-based). Deliberate text changes (picker entries, previews, Enter advertisement,
renamed board) land with updated tests + guides. A new integration test drives the real
binary through a FULL 2-player sim game over process pipes — adaptively, reading prompts
and answering — and asserts the game reaches WINNER with zero escape bytes in the entire
transcript. That test is the piped-dogfood honesty gate.

## 8. D1 dummy + views restyle

The 2P dummy renders as a named `Dummy` occupant (dim) on both tracks, with a one-line
note that it competes in majority scoring (finding D1). All views (`sites`/`goals`/
`tracks`/`labs`/`ports`/`players`/`score`/`log`) get one shared design language: a
`─ Title ─...` rule header, aligned columns, player letters/names and goods painted
everywhere they appear (paint-after-layout discipline unchanged, `strip_ansi(colored) ==
plain` stays pinned).

## 9. Module layout

- `render.rs` — pure state views (staggered board, panel, views). Stays paint-safe.
- `screen.rs` (new) — TTY-gated frame composition: banner, board+panel join by visible
  width, clear-screen chrome, `strip_ansi` promoted from test-support to crate-internal.
- `repl.rs` — pickers as new `SubPrompt` variants, bare-Enter, `panel`/`privacy`/
  `screen` commands, HELP_TEXT.
- `describe.rs` — fee/cost/faith/threshold previews.
- `main.rs` — stays thin: init switches, banner, wraps step output in a frame when
  frames are on. Wizard and session/engine boundary untouched.

## 10. Dependency decisions

None added. crossterm rejected for v1 (see §1); ANSI emitted remains confined to
`style.rs` (SGR) and `screen.rs` (clear/home), both behind TTY-gated switches that
default OFF, so libraries, tests, and pipes stay byte-plain without opting in.
