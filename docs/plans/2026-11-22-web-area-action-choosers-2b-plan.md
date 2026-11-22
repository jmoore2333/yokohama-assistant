# The card choosers: the Port and the Laboratory, and the terminal's flag counts — implementation plan 2b (2026-11-22)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** At the Port the main list reads as one entry and the decline, and the entry opens a chooser in the REPL (one card or two switched on by their printed numbers), which the browser draws with its own card faces. At the Laboratory the list reads as one row per technology, and a technology with more than one listed row opens the numbered pick the REPL already has. The terminal shows the per-country flag counts: in full in its `players` view, and compactly on the dashboard's Mercenaries row.

**Architecture:** The Port's chooser is a third model in the CLI's pure module `chooser.rs`, built by looking moves up in the legal list; the REPL holds the selection as a `SubPrompt`, and `confirm` commits the one listed move the selection names, or opens the existing numbered pick when two listed moves take the same cards (the second card paid either way). The web layer serializes the model as one new `PromptState` arm, `port`, and the browser draws it. The Laboratory's fold adds no sub-prompt and no prompt kind. Nothing computes a rule: reach, hand room, the two-card limit and what a second card costs are all answered by finding a listed move.

**Tech stack:** Rust 1.97.1 (`yokohama-cli`, `yokohama-web`, `yokohama-worker`), TypeScript, React, vitest, Playwright 1.62.

**Spec:** `docs/plans/2026-11-21-web-area-action-choosers-design.md` (accepted 2026-11-21): §5.1, §5.4, §5.5, §5.6's `port` arm, §5.7, §5.8, §6, §7. Read it before any task; where this plan and the design disagree, stop and report. This is the second of its two plans (§10).

**Read with it:** plan 2a's last section, "The run … and what it taught, for plan 2b" (`docs/plans/2026-11-21-web-area-action-choosers-2a-plan.md`): the names this plan builds on, the eight lessons, and the four things this plan owes. Each lesson is written into the constraints and cells below; the section is the reason for them.

**Branch:** `web-choosers-cards`, on the lab, cut from plan 2a's branch at `f46e8f76`. `BASE` below is that commit. Tasks land as new commits; nothing reaches origin.

## Global constraints

- **The lane's fence** (`docs/web-lane.md` §1): `yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and `gen-data` byte-identical; `SEMANTICS_REV` 7; no bot behaviour moved. A task that finds it needs an edit under any of the five STOPS and reports. Check at every task's end: `git diff --stat BASE -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` prints nothing.
- **The web's line fence does not move**: `git diff --stat BASE -- crates/yokohama-web/src/grammar.rs` prints nothing, its test module included.
- **The legal list is the only legality authority.** Which cards are in reach, whether a card can be switched on, whether a selection can be confirmed and how many ways it can be paid: each is answered by finding listed moves. No reach, hand-room, two-card or payment rule is restated in `chooser.rs`, `repl.rs`, `client.rs` or `web/`: none of them reads `port_accessible_zones`, the hand's length, the seat's yen or its imported goods to decide a flag. `PortBoard::zone_of` and the acting board's slots are read for what is DISPLAYED only (a card's zone and its place in the list).
- **The REPL's behaviour moves on purpose, and only where a task says so.** The guide pair (`docs/cli-guide.md` and `docs/cli-guide.html`) moves in the same commit, word-identical; `guide_sync` stays green. An in-source golden that moves is quoted in the report, before and after; none is edited without its diff shown.
- **The words are fixed in "The words" below.** That section is the single source for every screen, entry and refusal; the cells and the guide quote it.
- **TDD**: the failing cell first, run red, then the code. Suites green and `cargo clippy --workspace --all-targets -- -D warnings` clean after every task. No `cargo fmt`.
- **A cell that says "read off the list" names the rival rule and carries a fixture or a one-fact clone that separates the two** (plan 2a, lesson 1). The tasks name the rival for each such cell. A real corpus fixture co-varies its facts, so a client cell that reads a flag uses the fixture AND a JSON round-trip clone of it with ONE field changed.
- **A payload in a client cell comes from `web/src/wire/`**, never from a builder.
- **A fixture a `Session` applies a move from passes `check_invariants`** (plan 2a, lesson 3). Build a position as `chooser::test_support::issue_position()` is built: move pieces and cards that exist, invent none.
- **A driver refuses what no seed of its game reaches**, with a message that says why; it grows no branch nobody can run (rulings R25, R28 of plan 2a).
- **No agent opens an image**: no `.png`, `.webp` or `.jpg` anywhere, the card and token art (`crates/yokohama-web/assets/art/`) included. A layout claim is a measured box; how the cards look is the owner's reading.
- **Every invisible non-ASCII code point in code is a `\u` escape.** `—`, `●` and `→` are visible and may be raw.
- **Git**: stage with explicit paths, never `git add -A`; new commits only, never an amend; implementers do not push. A commit message ends with ONE attribution trailer naming the model that wrote the commit (`Co-Authored-By: Claude <its own model> <noreply@anthropic.com>`) and carries no `Claude-Session:` line and no claude.ai address (the owner, 2026-11-22).
- **Dispatch rules** (every implementer and reviewer): never `rm -rf`, `rm -r` or a bulk `rm -f`; long runs in the foreground or polled with short reads, never a Monitor or a wait on an event; the full report goes to `.superpowers/sdd/2026-11-22-web-area-action-choosers-2b-plan/<role>-task<N>.md` and the reply is the status line, the counts and that path; a reviewer works on an export (`git archive`) with its own `target/`, never in the checkout; a mutant is restored by `git checkout` or `touch`, never by a copy that keeps the file's mtime.
- **Probe before reporting.** A code block here is the single source: an implementer who finds it wrong corrects the plan's block in the same commit, transcribes it, and reports DONE_WITH_CONCERNS naming the correction. That covers test lines this plan dictates: the implementer runs each task's mutants as a probe and says which cell caught which. The task reviewer runs them on an export; the whole-branch review runs them again. The overlap is accepted.
- **Two stale-module traps.** `npm run test:wasm` (in `web/`) and the Worker's pool (`npm test` in `worker/`) replay the corpus through a COMPILED module: after any Rust change run `cd web && npm run build:wasm` before the first and `cd worker && npm run build:module:debug` before the second.
- **The suites of a task that moves Rust** ("the Rust suites" below): `cargo test -p yokohama-cli`, `-p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`; `cd worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test`; the lint gate; both fence checks. A task that moves no Rust file runs the web commands its steps name and the fence checks. **One suite at a time, never two beside each other**: a red read under a parallel run is re-run alone, its output saved first.
- **The browser suite is red from Task 2 until Task 5 lands, by design.** Once the Port folds, a walk in `web/e2e/` meets a prompt its policy cannot answer until the panel (Task 4) and the policy (Task 5) exist. No task between them runs `npm run e2e`; Task 5 runs it whole.
- **Counts are readings.** Re-count every suite at your own HEAD and state the counts in the report; a count that differs from the board's is a finding to explain.
- **A sentence says how its figure was got.** A figure in a comment, a cell name or a document was measured, or it says it was reasoned.

## Review focus

Five conditions the design implies and none of its §6 cells exercises; each has its cell in the task named.

1. **A third card, and a number that is no card in reach.** With two cards on, a third is refused and the selection kept; a number naming a card on the board but outside the listed takes, or no card at all, is refused with the cards that are (Task 2 cells 3 and 4).
2. **Port B.** At three and four players there are two ports. The chooser reads the ACTING board: its heading says `Port B`, and a card that sits on Port A is not in it (Task 1 cell 7; Task 2 cell 8).
3. **`cancel` inside the payment pick** returns to the main list and keeps nothing: the next opening of the chooser starts empty (Task 2 cell 6).
4. **A mercenary's sub-turn** at a Port and at a Laboratory folds and commits as the President's turn does (Task 2 cell 9; Task 3 cell 5).
5. **A title's rows are not next to each other in the engine's list.** Measured (below): with both copies of a title on one board, the first copy's row, another technology's rows and the second copy's rows come in slot order. The entry sits where the title first appears and its pick holds every row of both copies (Task 3 cells 1 and 2).

## What was measured before this plan was written

Readings by a throwaway harness outside the tree, at `f46e8f76`; the plan's independent review re-measures them before any task runs.

- **The engine's Port list** (`generate_port_actions`): the singles in board slot order, then every pair with its first card before its second in slot order, each pair once per way it can be paid (2 yen, then 1 imported good), then the decline. At a two-player position (setup seed 7, the dealt Port A `[O10, O34 | O28, O18 | O02, O13]`): power 2 lists 4 takes over 2 cards; power 3 lists 16 over 4; power 5 lists 36 over 6 (6 singles, 15 pairs each payable both ways with 5 yen and 1 imported good), 37 rows with the decline. With 2 orders in hand only the 6 singles are listed; with 3 none is, and the decline stands alone. With 5 yen and no imported good, or 1 yen and 1 imported good, each pair is listed once (21 takes); with 1 yen and no imported good, no pair is.
- **The engine's Laboratory list** (`generate_lab_actions`) is in slot order, a slot's payment mixes together. Ten of the twenty titles exist as two cards. With both copies of Ball on one board (slots 1 and 4) between Mining Technology and Tram, the list read: Mining Technology, Ball (the slot-1 copy, 1 row), Tram (3 rows), Ball (the slot-4 copy, 2 rows). Ball's three rows are NOT contiguous. Owning three technologies doubles most rows (the 4th-card fee paid either way): Ball read 6 rows.
- **The corpus walk's game** (the two-player `wire` record, 1,036 events): 95 area-action prompts with no choice queued; 3 Church folds (plan 2a's); **11 Port lists with more than one take** (events 90, 117, 145, 274, 356, 447, 707, 736, 821, 917, 1024) and none with exactly one; the walk's draws there are three pairs (events 90, 356, 1024, each payable one way), five singles and three declines. **No drawn pair is payable both ways, so the walk never opens the payment pick.** The Laboratory is met twice, once with a title of two rows (event 50, Ball; the draw is one of them).
- **At three players Port B exists and Laboratory B does not; at four both exist.**
- **The longest `order_prompt_line` in the registry is 79 characters** (O06); the widest row of the Port's screen is 97 columns, and a sub-prompt's screen is printed unwrapped under a frame 175 to 200 columns wide.
- **Whenever a pair is listed, both of its singles are** (the engine lists singles while the hand holds fewer than three and pairs while it holds at most one). So at the Port a selection is either empty or listed: with one card on, the preview is always legal.
- **By lines from a fresh table** (the additional actions ended, Option B's two assistants at a Port, the President placed there: power 3) every table reaches a folded Port prompt with four cards in reach and six pairs. The seat holds 3 yen and no imported good at every player count, so **no pair payable both ways is reachable by lines at the opening**. On the table the Worker's restore cells use (seed 20260831) the entry is `1` and the cards are 34, 19, 12 and 4.
- **The walk gains 19 rows at Task 2 and 1 at Task 3**, counted from its draws (three pairs at three more lines each, five singles at two more, three declines at none; one Laboratory pick): `walk.json` 2,222, then 2,241, then 2,242. The kept list's rows 2194 to 2206 and 2221 sit after the first folded Port prompt and shift; its earlier rows and `walk-host.json`'s `after` indices do not.
- **Event 50's two Laboratory rows are the two COPIES of Ball** (cards 12 and 11, slots 1 and 2): `Laboratory: acquire Ball` and `Laboratory: acquire Ball, slot fee: 2 yen`. The walk's draw is the first.
- **Inside a chooser `?` opens the card index** and returns to the chooser with its selection kept; `ports` and `hand` open their views too; `view`, `help` and `board` are refused (`Repl::step` answers `?` ahead of every sub-prompt, and plan 2a pinned it).
- **The numbered pick prints its title followed by a colon** (`{title}:`), and the browser shows the title as its asking line.
- **The terminal's frame has no row to spare for a flags line** (measured by adding one): the opening frame is 51 rows at two players, 50 at three and 49 at four, against a 50-row floor that the three-player frame sits on. A `Flags` line per seat reads 53, 51 and 50; with a line for the flags in hand, 54, 53 and 52; five frame cells read red. Joining the orders line does not fit either: the two-player orders line already reaches the dashboard's 74 columns, and a seat's column below the board is 48 columns at four players. The hand panel below the board is one row and already names each card's country.
- **These readings were first taken on states the engine's invariants refuse** (assistants and cards invented, not moved) and were reproduced by the plan's review on valid ones. A valid build: the assistants come out of the seat's hand; an extra order in hand is popped off the order deck; a Laboratory card is swapped with the tech deck, and an emptied slot's card goes back into its deck.

## The words

Entries on the main list. `{r}` is the count of distinct cards over the listed Port moves; `{n}` is the count of listed Laboratory rows for the title.

```
Port — take 1 or 2 order cards ({r} in reach)
Port — take 1 order card ({r} in reach)
Laboratory: acquire {title} — {n} ways to pay
```

The first Port line when some listed move takes two cards, the second when none does. A Laboratory title with one listed row keeps that row, in `describe_move`'s words, as today. When the acting Laboratory board holds both copies of a title, each of that title's rows names its copy's slot after the name, as the `labs` view and the browser's lab strip number it: `Laboratory: acquire Ball (slot 2)` (Task 3b, on the owner's word of 2026-11-22: a player has to know which of the two cards a row takes).

Opening the chooser prints no line of its own.

**The Port's screen.** One row per card in reach, in the acting board's slot order. The zone's first row is prefixed `  LOW   `, `  MID   ` or `  HIGH  ` (eight columns), every other row eight spaces; then `[N] ` with the card's bare number right-aligned to two columns; then `order_prompt_line`'s words for the card; then, when the card is selected, four spaces and `●`. The heading names the acting board, `Port A` or `Port B`.

```
Port A — take 1 card, or 2 (toggle by number):
  LOW   [10] O10 [Britain]  needs 2 copper + 1 tea + 1 fish  ->  4 VP + 2 move    ●
        [34] O34 [France]  needs 2 copper + 3 tea  ->  9 VP + 1 import    ●
  MID   [28] O28 [America]  needs 1 silk + 4 fish  ->  3 VP + 1 import + 1 warehouse
        [18] O18 [America]  needs 2 copper + 3 fish  ->  6 VP + 1 import
  HIGH  [ 2] O02 [France]  needs 2 tea + 1 fish  ->  3 VP
        [13] O13 [Britain]  needs 2 copper + 1 silk + 2 fish  ->  8 VP + 1 import
Selected: O10, O34
  -> legal — 2 ways to pay, asked at confirm:
       Port: take O10, O34, paying 2 yen
       Port: take O10, O34, paying 1 imported good
> a card number toggles it, 'confirm' to take, or 'cancel'
```

This is Task 1's fixture with O10 and O34 on. When no listed move takes two cards the heading reads `Port A — take 1 card (toggle by number):`.

`Selected` lists the cards switched on in slot order, as `O{nn}`. The lines under it:

| The listed moves taking exactly the selection | Lines |
|---|---|
| nothing selected | `Selected: (none)` over `  -> pick a card to preview it` |
| one | `  -> legal — {describe_move's words}`, for example `  -> legal — Port: take O10` |
| two | the three lines above: `  -> legal — 2 ways to pay, asked at confirm:` and each move's words indented seven spaces, in list order |
| none, with something selected | `  -> NOT currently offered (adjust the selection)` |

**The two picks.** Both are the REPL's existing numbered pick (`SubPrompt::Pick`), its rows `describe_move`'s words, its hint and its `cancel` (`Selection cancelled — back to the main options.`) unchanged. Their titles, passed WITHOUT a colon, since the pick prints one after its title and the browser shows the title as it is:

```
Port: take O10, O34 — choose the payment
Laboratory: acquire {title} — choose the payment
```

The Port's title lists the cards as `describe_move` does for those moves.

**Refusals and cancels in the Port's chooser** (`{card}` is the card as `O{nn}`):

| Input | Line |
|---|---|
| a card in reach that the list holds no take for with the selection (a third card; a second with no room in hand or no way to pay it) | `! the list holds no take with {card} added from here` |
| a number, or `O{nn}`, that is no card in reach | `! that is not a card in reach here — pick one of the listed numbers, 'confirm', or 'cancel'` |
| `confirm` with nothing selected | `! pick a card first, or 'cancel'` |
| `confirm` on a selection the list does not hold | `! that selection is not currently available — adjust it or 'cancel'.` (Option A's own line) |
| anything else, bare `back` included | `! toggle a card by its number, 'confirm', or 'cancel' ('?' shows the cards; cancel first for help or the view menu)` |
| `cancel` | `Port take cancelled — back to the main options.` |

A card is named by its bare number (`11`), which is what the browser sends, and in the terminal also as it is displayed (`O11`, `o11`).

`?` opens the order-card index over the chooser and returns to it with the selection kept, as it does over the Church's and Chinatown's (design §9: "following what the sub-prompts do today"). The other two choosers' refusal lines say "(cancel first to use views/help)", which is half true (`?`, `ports` and `hand` open from inside); they are plan 2a's, built, and are recorded for the cycle's close.

Two lines of the table above cannot be reached at the Port by lines, since a selection there is always empty or listed (measured): `  -> NOT currently offered (adjust the selection)` and the `confirm` refusal for a selection the list does not hold. They are written, so that the chooser does not rest on how the engine builds its list; no driver hunts for them, and the model's cell reads them on a list with a pair's single removed.

**The terminal's flag counts** (Task 8), in the `players` view (`render::players_panel`), one line per seat after its `completed:` line:

```
  flags: GB 2  US 1  FR 0  DE 0  NL 1
```

The five in `Country::ALL`'s order. For a seat whose hand is shown and holds at least one flag, a second line:

```
  flags in hand (not yet counting): GB 1  NL 1
```

listing the non-zero countries only. A hidden hand prints no such line.

In the always-visible dashboard the face-up counts ride the **Mercenaries row** of each seat, at no cost in rows, since every second flag of one country is what earns a mercenary. Each country's code is followed by its count with no space, the five separated by single spaces, a zero count dimmed:

```
  Mercenaries  0 unused · 0 used · GB1 US0 FR0 DE1 NL1
  Mercenaries  0 unused · GB1 US0 FR0 DE1 NL1
```

The first form wherever the row fits its width (the two-player dashboard's 74 columns; a three-player seat's 56). The second where it does not: the `used` count yields FIRST and whole, never clipped, as `next take` yields on the seat's last row; it stays in the `players` view. That is always so in a four-player seat's 48 columns. **Where the second form does not fit either** (corrected at Task 8, measured over every legal row by `the_used_count_yields_first_and_whole`: the second form reaches 50 columns, against a four-player seat's 48, with two-digit counts in four countries or in three with all ten mercenaries unused; the widest row at 74 columns is 59 and the widest second form at 56 is 50, so neither of the other two widths reaches this), the countries at zero yield next, all of them and whole, so a row that names some countries and not others has none of the others:

```
  Mercenaries  9 unused · GB14 US12 FR12 DE10
  Mercenaries  10 unused
```

and where no country is at zero, the counts yield together and the row reads its unused count alone, the last form above: no country is ever dropped while another stays, since that would read as a zero, and nothing is ever clipped. The flags in hand are not on the dashboard: its hand panel already names each card's country. **Ruled by the owner on 2026-11-22**, after reading the mock: see "The owner's ruling: where the terminal shows the flags".

**The browser's words.** The Port panel's asking line is `The Port — take one card, or two`, or `The Port — take one card` when the arm's `max_cards` is 1. The help's and the guide's three sentences (Task 9):

- P1. `At the Port, when more than one take is on offer, the list shows a single entry beside the decline, and that entry opens a chooser: you switch on one card or two.`
- P2. `A card you have not switched on is live only where the engine's list holds a take with it added, so a second card goes dark when your hand has no room for it or you cannot pay for it; when you could pay for the second card either with 2 yen or with an imported good, Confirm asks which.`
- P3. `At the Laboratory a technology that can be paid for in more than one way is a single row, and pressing it opens the list of those ways.`

## The files

- `crates/yokohama-cli/src/chooser.rs` — the Port's model, text and matcher beside the Church's and Chinatown's; three fixtures in its `test_support` (Task 1).
- `crates/yokohama-cli/src/repl.rs` — `PromptOption::Port`, `PromptOption::Laboratory`, `SubPrompt::Port`, `step_port`, the fold in `group_area_action`, one shared close-then-commit helper, `chooser_screen()` (Tasks 2, 3).
- `crates/yokohama-cli/src/render.rs` (`players_panel`), `panel_render.rs` (the Mercenaries row in both arrangements), `screen_model.rs`, the two committed frame captures — the flag counts and the country codes with their pin (Task 8).
- `crates/yokohama-cli/src/describe.rs` — the Laboratory row names its slot when the acting board holds both copies of its title (Task 3b).
- `crates/yokohama-cli/tests/piped_dogfood.rs` — the driver learns the Port and the Laboratory's pick; a floor on what it counts (Tasks 2, 3).
- `crates/yokohama-web/src/client.rs`, `api.rs`, `lib.rs`, `tests/play_a_whole_game.rs`, `tests/wire_fixtures.rs` — the `port` arm, the strip by identity, every driver, the corpus (Tasks 2, 3, 7).
- `web/src/wire/` — four new fixtures (`port.json`, `port_selected.json`, `port_pair.json`, `lab_pick.json`); `web/src/wire-script/` regenerated; `web/src/wire/contract.ts`, `contract.test.ts`, `web/src/types.ts` (Tasks 2, 3).
- `worker/scripts/replay/fixtures/production-door-kept-rows.txt`, `worker/scripts/live-replay.test.mjs` — the kept list's indices and the pinned literal move with the walk (ruling R18 of plan 2a) in every task that regenerates it (Tasks 2, 3).
- `web/src/Decision.tsx`, `styles.css`, `Decision.test.tsx` — an interim branch for `port` and the unknown-kind cells' kind (Task 2); the Port's panel, `isChooser` (Task 4).
- `worker/RUNBOOK.md`, `docs/web-lane.md` — the walk rows they cite by number ("walk row 2204", "rows 2204 and 2205") move with the walk (Tasks 2, 3).
- `web/e2e/harness.ts`, `shared_table.ts`, `the_table.spec.ts` — the policy, the layout and the keyboard cells (Task 5).
- `crates/yokohama-worker/src/picker.rs` (its test module only) — the restore cells (Task 6).
- `docs/cli-guide.md`, `docs/cli-guide.html`, `docs/web-guide.md`, `web/src/Help.tsx`, `web/src/Help.test.tsx`, `docs/web-lane.md` — the documents (Tasks 2, 3, 8, 9).

### Task 1: The Port's chooser, as a pure module

**impl: sonnet · review: opus.**

**Files:** modify `crates/yokohama-cli/src/chooser.rs`.

**Interfaces.** Consumes: `describe::describe_move`, `describe::acting_area` (both `pub(crate)`), the engine's `order_prompt_line`, `PortBoard::zone_of`, `Rules::orders`. Produces, for Tasks 2, 4 and 6:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortCard {
    /// What to send: the card's bare id.
    pub number: u8,
    pub id: OrderCardId,
    /// The zone of the card's slot on the acting board. Displayed, never a rule.
    pub zone: PortZone,
    /// The card's face, as the ports panel carries it.
    pub printed: OrderCard,
    /// `order_prompt_line`'s words.
    pub words: String,
    pub selected: bool,
    /// The list holds a take of the selection with this card added.
    pub can_add: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortPreview {
    /// Some listed move takes exactly the selection.
    pub legal: bool,
    /// `describe_move`'s words of every listed move taking exactly the selection, in list order: none, one or two.
    pub ways: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortModel {
    /// Which port is acting, as the ports panel names its boards.
    pub board_b: bool,
    /// The most cards one listed move takes: 1 or 2.
    pub max_cards: usize,
    /// The cards some listed move takes, in the acting board's slot order.
    pub cards: Vec<PortCard>,
    pub preview: PortPreview,
}

/// The cards a Port move takes; `None` for any other move.
pub fn port_cards(mv: &Move) -> Option<&[OrderCardId]>;
/// Every listed move taking exactly `selected`, whatever the order of either, in list order.
pub fn port_matches(legal: &[Move], selected: &[OrderCardId]) -> Vec<Move>;
/// The distinct cards over the listed Port moves.
pub fn port_reach(legal: &[Move]) -> Vec<OrderCardId>;
/// The most cards one listed Port move takes; 0 when none is listed.
pub fn port_most_cards(legal: &[Move]) -> usize;
pub fn port_model(state: &GameState, rules: &Rules, legal: &[Move], selected: &[OrderCardId]) -> PortModel;
/// The terminal's screen ("The words"), ending in a newline.
pub fn port_text(model: &PortModel) -> String;
```

`can_add` is false for a selected card. A card of `selected` that the acting board does not hold is ignored by the model. In `test_support`, each passing `check_invariants` and built as "What was measured" says a valid build is (the assistants out of the seat's hand, an extra order popped off the order deck):

- `port_position() -> GameState`: two players, setup seed 7, the seat to move with its President and four assistants on Port A (power 5), its hand as dealt (one order), 5 yen and 1 imported good. Measured: 36 Port moves, 6 singles and 15 pairs each listed twice.
- `port_position_with(assistants: u8, hand: usize, yen: u32, imports: u32) -> GameState`: the same board with those four facts set.
- `port_b_position() -> GameState`: three players, the seat to move on Port B at power 5 with room for two cards, 5 yen and no imported good, so each pair is listed once.

- [ ] **Step 1: the cells, red.** Each names the rival rule its data is chosen against.
  1. `the_fixtures_are_what_the_plan_measured`: on `port_position()`, 36 Port moves, 6 of one card, 30 of two, every pair's cards listed twice, `port_reach` of 6, `port_most_cards` 2; and `check_invariants` passes on all three fixtures.
  2. `every_listed_take_matches_itself_whatever_its_order`: for every listed Port move, `port_matches(legal, cards)` holds that move; with the cards reversed it holds the same moves; for a pair on `port_position()` it holds exactly the two listed moves, in list order.
  3. `can_add_is_read_off_the_list`: for every selection of zero, one or two cards of the reach on each position below, each unselected card's `can_add` equals `!port_matches(legal, selection + card).is_empty()`. The positions separate the list from each restated rule: `port_position_with(4, 2, 5, 1)` (two orders in hand: rival "two cards while the hand has room for one"; every `can_add` with one card on is false); `port_position_with(4, 1, 1, 0)` (rival "a second card is free"; the same); `port_position_with(1, 1, 5, 1)` (power 2: rival "every card on the board"; two cards in the model, four not); `port_position_with(4, 3, 5, 1)` (three orders in hand, so the list holds the decline alone: rival "the cards of the zones the power opens", which is `port_accessible_zones`' rule; every zone is open and the model holds no card). And on `port_position()`'s list with both takes of one pair removed, with that pair's first card on, its second reads `can_add` false and another card true (rival: "a second card whenever the list holds any pair"; the engine lists every pair in reach or none, so only a cut list separates the two).
  4. `a_third_card_cannot_be_added`: on `port_position()` with two cards on, every other card's `can_add` is false, and both selected cards read `selected: true`, `can_add: false`.
  5. `the_preview_is_describe_s`: on `port_position()`, one card on gives `ways` equal to `[describe_move(the listed single)]`; two cards give the two listed moves' words in list order, the first ending `, paying 2 yen` and the second `, paying 1 imported good`; nothing on gives `legal: false` and no way. On `port_position_with(4, 1, 5, 0)` a pair gives one way (rival: "two ways whenever two cards are on"). On `port_position()`'s list with one single REMOVED, that card alone reads `legal: false`, no way, and `port_text` prints `  -> NOT currently offered (adjust the selection)` (the engine never lists it so; the model must not rest on that).
  6. `the_cards_are_in_slot_order_with_their_zones`: on `port_position()` the numbers read `[10, 34, 28, 18, 2, 13]` with zones Low, Low, Mid, Mid, High, High, `printed` and `words` the registry's for each id; a clone of the state with two slots swapped reads the swapped order (rival: "sorted by id"); and on a list with the first card's single removed, so that the list first names another card, the order is still the board's (rival: "the order the list names them").
  7. `port_b_is_the_acting_board`: on `port_b_position()`, `board_b` is true, every card of the model sits on Port B, and no card of Port A is in it (rival: "always Port A"); and with a three-player seat acting on Port A, `board_b` is false, the heading opens `Port A` and every card sits on Port A (rival: "Port B whenever the table has one"; at two players there is no Port B to tell the two apart).
  8. `the_screen_is_the_plan_s`: `port_text` on `port_position()` with O10 and O34 on equals "The words"' block byte for byte; with nothing on its last three lines are `Selected: (none)`, `  -> pick a card to preview it` and the prompt line; `port_text` on `port_position_with(4, 2, 5, 1)` opens `Port A — take 1 card (toggle by number):`.
  9. `max_cards_is_read_off_the_list`: 2 on `port_position()`, 1 on `port_position_with(4, 2, 5, 1)` and on `port_position_with(4, 1, 1, 0)` (rival: "2 whenever the hand has room").
- [ ] **Step 2:** `cargo test -p yokohama-cli --lib chooser::` reads the nine red (they do not compile; that is the red).
- [ ] **Step 3:** the code. One pass over the legal list builds a map from a sorted card set to its listed moves; `can_add`, the preview and `max_cards` all read that map.
- [ ] **Step 4:** green; `cargo test -p yokohama-cli`; the lint gate; both fence checks.
- [ ] **Step 5:** commit.

**Mutants:** `can_add` from the hand's length alone (cell 3, the 1-yen position, where the hand has room and the list says no); `can_add` from the seat's yen and imported goods alone (cell 3, the two-orders position, where the seat can pay and the list says no); the cards from the board's slots without the list (cell 3 alone, at the power-2 position, where the model would hold six cards and the list reaches two; cell 7 cannot see it, since every card of Port B's fixture is in reach); the cards from `port_accessible_zones` (cell 3, the three-orders position); Port B whenever the table has one (cell 7, the three-player Port A seat); `can_add` from the list's longest take (cell 3, the cut pair); `port_matches` comparing in order (cell 2); `ways` built from the selection instead of `describe_move` (cell 5); the acting board fixed to Port A (cell 7); the cards sorted by id (cell 6).

### Task 2: The Port folds, in the REPL and on the wire

**impl: opus · review: opus.**

**Files:** modify `crates/yokohama-cli/src/repl.rs`, `crates/yokohama-cli/tests/piped_dogfood.rs`, `crates/yokohama-web/src/client.rs`, `api.rs`, `lib.rs`, `tests/play_a_whole_game.rs`, `tests/wire_fixtures.rs`, `web/src/types.ts`, `web/src/wire/contract.ts`, `contract.test.ts`, `web/src/Decision.tsx` (one interim branch, below), `web/src/Decision.test.tsx` (the unknown-kind cells), the walk's regenerated files, the Worker's two kept-list files, `worker/RUNBOOK.md` and `docs/web-lane.md` where they cite a walk row by number, `docs/cli-guide.md`, `docs/cli-guide.html`; every CLI and web-crate test that answers a Port prompt by row (search, below).

**Interfaces.** Consumes Task 1's. Produces:

- `PromptOption::Port { reach: usize, pairs: bool }` and `SubPrompt::Port { selected: Vec<OrderCardId> }` (the cards in the order switched on).
- `group_area_action` folds the Port family when more than one Port move is listed, the entry where the family's first move sits; one listed Port move stays a direct row.
- `step_port`: a number or `O{nn}` toggles; `confirm` reads `chooser::port_matches`: none refuses and keeps the selection; one closes the chooser and THEN commits that listed move; two closes the chooser and opens `enter_pick_subprompt` over the two moves, titled as "The words" says.
- **One helper for `cancel`, `quit` and the close-then-commit `confirm`**, called by `step_church`, `step_chinatown` and `step_port`. The two existing choosers' behaviour and every one of their cells are unmoved.
- `chooser_screen()` answers for the Port's chooser too.
- `PromptState::Port(yokohama_cli::chooser::PortModel)`, kind `port`; `prompt_kind` answers it from `repl.sub` with no model built; `api.rs` strips the Port's screen from a reply's messages by identity, as it strips the other two.
- In `web/src/types.ts` the `port` arm (`PortCard`, `PortPreview`), with `OrderCard` and the zone typed as the ports panel types them. In `Decision.tsx`, `port` joins `DRAWN_KINDS` and is drawn, until Task 4, by an INTERIM branch of its own: the panel's asking line ("The words") and Cancel, under `.chooser.port`, never the unknown-kind node (`.unknown-prompt`), so the client has no dead end between the two tasks and no known kind is drawn as an unknown one. In `Decision.test.tsx` the unknown-kind cells stop using `"port"` as their unknown kind (it is a known kind from this task on) and take a name no server sends; the comment in `Decision.tsx` that calls `port` "the next" is corrected. `npm test` stays green at this task's end.
- `index_target`'s exhaustive match gains the `SubPrompt::Port` arm, answering as the other choosers' arms do, so `?` opens the order index over the chooser.
- **The drivers' `port` arm** (`play_a_whole_game.rs`'s `choose`, and the piped-dogfood driver from its screen): while fewer than `max_cards` cards are on and some unselected card's `can_add` is true, switch one on when the driver's own draw says so; else Confirm when the preview is legal; else Cancel. An arm that confirms as soon as the preview is legal takes a single every time, because one card is always legal (measured), and no pair and no payment pick is ever reached.
- `client::test_support::to_the_port(repl: &mut Repl) -> String` (a real game by lines to a folded Port prompt; returns the entry's number) and `port_entry(repl: &Repl) -> Option<String>`.

**What answers a Port prompt by row today.** Search, do not trust this list: `grep -rn "Port: take\|AreaAction::Port" crates/yokohama-cli/src crates/yokohama-cli/tests crates/yokohama-web/src crates/yokohama-web/tests crates/yokohama-worker/src web/e2e web/src/local`. Found by the plan's review, which planted a crude fold and read what went red: fourteen cells of `yokohama-cli` that answer a Port take by its index in the legal list (`repl.rs`'s `sim_mode_port_take_applies_immediately_with_no_reveal_prompt` and the two-card helper three index cells share; `reveal.rs`'s two helpers and seven of its cells, `live_port_take_collects_reveals_then_applies_them` and `the_guide_reveal_captures_are_what_the_fixtures_render` among them); `crates/yokohama-web/src/lib.rs`'s `play_until_a_draw` and its Port callers, which look for a row starting `Port: take`; `play_a_whole_game.rs`'s `choose`, which panics on a kind it does not know; the piped-dogfood driver; the corpus walk; `Decision.test.tsx`'s unknown-kind cells. `drive_to_double_assistant_move_cascade` and `view_render.rs` name a Port move and read green under the fold. No capture golden and no `guide_sync` cell quotes a Port row. Each site learns the chooser, or names its move another way, in this task; the report lists every site found and what was done at each. **The live-mode cells park at a reveal after the take**: a take committed through the chooser must leave them parking exactly as before (cell 10b).

**The corpus.** The walk plays the SAME game: at a folded Port prompt it draws its index over `frame.decision.options`, as it does at a folded Church prompt, and names the drawn move: a move the fold left as a direct row (the decline) by that row's number; a take by the entry's number, each card's bare number in the listed move's order, then `confirm`. A drawn pair that two listed moves take would need the pick's number; the walk's game never draws one (measured above), so the walk REFUSES that case with a panic that says so, and grows no branch for it. Three captures at the one folded Port prompt whose draw is a pair with more than two cards in reach (event 1024 by the measurement: four cards in reach, six pairs each payable one way; the implementer confirms the event). The walk's capture keeps the FIRST occurrence of a name, and event 447 opens a four-card chooser first (its draw is a single), so the condition is stated in the walk and not left to order: `port` (the chooser just opened), `port_selected` (one card on, legal, one way), `port_pair` (two cards on, legal, one way). The first fold (event 90) has two cards in reach, so a pair there leaves no unselected card and its flags would say nothing.

- [ ] **Step 1: the REPL's cells, red** (in `repl.rs`'s tests, on Task 1's fixtures).
  1. `the_main_list_folds_the_port`: on `port_position()` the list is two rows, `Port — take 1 or 2 order cards (6 in reach)` and the decline; on `port_position_with(4, 2, 5, 1)` the entry reads `Port — take 1 order card (6 in reach)`; on a position whose list holds exactly one Port move (two orders in hand at power 2 with one LOW slot emptied into the removed pile) that move is a direct row in `describe_move`'s words (rival: "fold whenever the President is at a Port"); and on `port_position_with(1, 1, 5, 1)` (power 2: six cards on the board, two in the listed takes) the entry reads `Port — take 1 or 2 order cards (2 in reach)` (rival: "the cards on the board").
  2. `every_listed_take_commits_through_the_chooser`: for each of `port_position()`'s 36 listed takes, the lines that name it (the entry, its cards, `confirm`, and for a pair the pick's row) commit exactly that move, compared as a `Move`; the cards are sent once in the listed order and once reversed. Half the takes name their cards as `O{nn}`. The same over `port_position_with(4, 1, 5, 0)`'s 21 takes, where each pair is listed once: `confirm` commits it with no pick (rival: "the pick opens whenever two cards are on").
  3. `a_third_card_is_refused_and_the_selection_kept`: two cards on, a third answers `! the list holds no take with O28 added from here`, and `confirm` then opens the pick over the first two. And design §6's own case, a second card with room in hand for one: on `port_position_with(4, 2, 5, 1)` one card on, a second answers the same line for its card, nothing is applied, and `confirm` takes the first alone. The same on `port_position_with(4, 1, 1, 0)`, where the hand has room for two and the seat cannot pay for a second (rival: "a second card while the hand has room"). And a card that is on is switched off by its number in either form, whatever its `can_add` reads: with one card on, its number leaves `Selected: (none)` and `confirm` answers the pick-a-card line; with two on, the first card's number is taken and leaves the second alone (rival: "a number only ever switches a card on").
  4. `a_number_that_is_no_card_in_reach_is_refused`: on `port_position_with(1, 1, 5, 1)` (power 2), `28` (on the board, out of reach), `99` and `0` each answer the not-in-reach line and change nothing.
  5. `confirm_with_one_way_closes_then_commits`: one card, `confirm`: the reply holds no line of the chooser's screen and ends in the next prompt, and the state is the listed move's (equal to applying that move to a twin). That a take's reply reads as the old direct row's did is Step 3's to witness: every fixture committed before this task stays byte-identical.
  6. `cancel_in_the_payment_pick_keeps_nothing`: two cards, `confirm`, `cancel`: the main list; opening the entry again shows `Selected: (none)`.
  7. `the_refusals_are_the_plan_s`: `confirm` with nothing on; `back`; `back 1`; `more`; `help`; a tile's id; `cancel`. Each line is "The words"', and none but `cancel` changes the selection.
  7b. `the_card_index_opens_over_the_port_s_chooser_and_returns_to_it`: with a card on, `?` shows the order-card index and closing it shows the chooser with that card still on (beside plan 2a's cell of the same shape).
  8. `port_b_folds_and_commits`: on `port_b_position()` the screen opens `Port B —` and a take commits.
  9. `a_mercenary_s_port_folds_and_commits`: a position in a mercenary's sub-turn at a Port (built as plan 2a's mercenary Church cell builds its own): the fold, one take committed, the sub-turn's next phase reached.
  10. `the_shared_helper_moved_nothing`: no new assertion; the Church's and Chinatown's existing cells and both sweeps read green unedited. The report states their counts before and after.
  10b. `a_live_port_take_through_the_chooser_parks_at_its_reveal`: in live mode a take committed through the chooser parks at the reveal as the old direct row's did; the existing live cells, moved to the chooser's lines, assert what they asserted.
  11. `a_self_play_sweep_commits_every_take`: over self-play positions at two, three and four players (seeds and game counts sized to stay per-edit, as plan 2a's sweeps are), at every folded Port prompt every listed take commits through the chooser, and every model's `can_add` equals the list's answer. The report states how many positions and takes it reached, and how many pairs were payable both ways (the floor is one; if the sweep reaches none, enlarge it and say so).
- [ ] **Step 2: the web layer's cells, red** (`client.rs`, `api.rs`).
  12. `the_port_arm_is_the_repl_s_model`: at `to_the_port`, the acting seat's prompt is `port`, equal to `chooser::port_model` of the same `Repl`; another seat's is `watching`; a caller with no seat gets no arm.
  13. `a_port_reply_carries_no_screen_line_and_keeps_a_refusal`: a toggle's reply has no line of `chooser_screen()`; a refused third card's reply carries the refusal line.
  14. `prompt_kind_is_the_built_prompt_s_kind_at_every_kind`: the existing cell, extended to `port`.
  15. `the_expect_check_tells_the_port_from_the_main_list`: a line sent with `expect: "options"` while the chooser is open answers 409, as it does for the other two.
  16. `every_line_the_client_sends_for_the_port_passes_the_fence`: each bare number 1 to 36, `confirm` and `cancel` pass `is_gameplay_line` as it stands; `O11` and `o11` do not (the terminal's form, which the client never sends).
- [ ] **Step 3:** the code, the drivers, the walk and the captures; regenerate (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then the plain run) and READ the diff: every fixture committed before this task is byte-identical, `log.json` among them; the walk gains rows only at the folded Port prompts; the three captures are new. The kept list's header counts, indices and footer and `live-replay.test.mjs`'s literal move with the walk (no kept row's text changes, none is added or dropped); both diffs quoted in the report.
- [ ] **Step 4: the contract.** `wire.port`, `wire.portSelected`, `wire.portPair` join `EVERY_STATE`; `KEYS.portCard`, `KEYS.portPreview` and the arm's own top-level key list; one cell per fixture states what it is (`port`: nothing selected, not legal, no way; `portSelected`: one card selected, legal, one way; `portPair`: two selected, legal, one way, every other card's `can_add` false); `wire_fixtures`' variant count goes to 12.
- [ ] **Step 5: the piped-dogfood driver** answers the Port's chooser from its screen (a card the screen lists; `confirm` when the screen says legal; the pick by number), counts takes confirmed through the chooser and payment picks opened, and asserts **a floor of one on its Church count and on its Port count**, each measured before it is set and stated in the report.
- [ ] **Step 6: the guide pair.** §4 gains the Port's paragraph and screen (the block in "The words"), the two forms of a card's number, and the payment pick; a `guide_sync` cell ties the guide's quoted screen to `port_text` on the fixture, as plan 2a's fix wave tied the other two.
- [ ] **Step 7:** the Rust suites, one at a time; commit. This task is the plan's largest, and its two halves cannot each be green: with the REPL's fold alone the web crate's corpus cell reads red (measured). If it lands as two commits, the first (the REPL, its drivers in the CLI crate and the guide) says in its message that the web suites are red until the next; the task's END is green, and the walk rows `worker/RUNBOOK.md` and `docs/web-lane.md` cite by number are corrected to the regenerated walk's.

**Mutants:** the fold at one listed Port move (cell 1); `confirm` committing a move BUILT from the selection (cell 2, the reversed order, where a built pair is not the listed one); `confirm` with two matches committing the first (cells 2 and 3); the payment pick opened whenever two cards are on (cell 2's second position); commit before close (cell 5); the selection kept across the pick's `cancel` (cell 6); `O{nn}` not parsed (cell 2); a reach rule in `step_port` (cell 4); a hand-room and two-card rule in `step_port` in place of `can_add` (cell 3, the 1-yen position); the entry's count taken from the board's cards (cell 1, the power-2 position); a selected card's number doing nothing (cell 3); the arm sent to another seat (cell 12); the screen left in `messages` (cell 13); `prompt_kind` building the model (cell 14 has no red against that: its proof is the mutant, as in plan 2a); the walk drawing over the prompt's rows instead of the frame's options (Step 3: `log.json` moves).

### Task 3: The Laboratory folds by title

**impl: opus · review: opus.**

**Files:** modify `crates/yokohama-cli/src/repl.rs`, `crates/yokohama-cli/tests/piped_dogfood.rs` if its driver names a Laboratory row, `crates/yokohama-web/src/lib.rs` (the `Laboratory: acquire` caller of `play_until_a_draw`), `tests/wire_fixtures.rs`, the walk's regenerated files and the Worker's two kept-list files, `web/src/wire/contract.ts`, `contract.test.ts`, `docs/cli-guide.md`, `docs/cli-guide.html`; every test that answers a Laboratory prompt by row (search: `grep -rn "Laboratory: acquire\|AreaAction::Laboratory"` over the same trees as Task 2).

**Interfaces.** Produces `PromptOption::Laboratory { title: String, moves: Vec<Move> }`. `group_area_action` takes only the legal list today; it gains what it needs to name a technology (the rules' tech registry, threaded from `group_options`), and the report quotes the new signature. In it, the Laboratory rows are grouped by the technology's name (`describe::tech_name`'s), a title with one listed row stays a direct row, and a title with more becomes one entry at the place of its FIRST row, holding every row of that title in list order. Choosing it calls `enter_pick_subprompt` with the title in "The words". No new `SubPrompt`, no new prompt kind, no client change: on the wire the entry is an ordinary row with `opens_picker: true` and the pick is `pick`.

- [ ] **Step 1: the cells, red.** Fixtures in `chooser::test_support` (each passing `check_invariants`): `lab_twin_position()`, both copies of one title on Laboratory A in slots 1 and 4, another technology between them with more than one way to pay, the seat able to pay every row; `lab_twin_position_owning_three()`, the same with three other technologies owned.
  1. `the_main_list_reads_one_row_per_title`: on `lab_twin_position()`, the rows are, in order, the single-row technology as today's direct row, `Laboratory: acquire {twin} — 3 ways to pay` at the place of the twin's first row, the other technology's entry with its count, and the decline. No title appears twice (rival: "group runs of adjacent rows", under which the twin's title appears twice).
  2. `the_twin_s_pick_holds_every_row_of_both_copies`: the entry opens a pick whose title is `Laboratory: acquire {twin} — choose the payment` (drawn with the pick's own colon) and whose rows are the three listed moves in list order, in `describe_move`'s words, the slot-1 copy's row without a slot fee and the slot-4 copy's with one (rival: "group by card id", under which the entry holds one copy's rows).
  3. `every_listed_row_commits_through_its_title`: on both fixtures, for every listed Laboratory move, the entry's number (or the direct row's) and then the pick's row commit exactly that move, compared as a `Move`.
  4. `a_title_with_one_row_stays_direct`: its row's words and its number's effect are today's. (A Laboratory prompt with an acquisition always lists the decline too, so there is no forced single option to keep here.)
  5. `a_mercenary_s_laboratory_folds_and_commits`.
  6. `a_self_play_sweep_commits_every_row`: at two, three and four players, at every Laboratory prompt with a title of more than one row, every listed row commits through its title, and the main list reads as a list laid out BY HAND from the legal list (each title where its first row stands, a one-row title as its own row, the decline last), never through the fold under test (rival: "the entries gathered after the direct rows", which every built fixture agrees with, since their one direct row stands before every folded title; the sweep's positions do not); the report states the positions and rows reached, how many had a one-row title after a folded one, and whether any had both copies of a title on one board.
  7. On the wire (`client.rs`): at a folded Laboratory prompt the entry's row has `opens_picker: true` and its `words` are the entry's; the pick's prompt is `pick` with the title and the rows.
- [ ] **Step 2:** red, then the code and the drivers. `play_until_a_draw`'s Laboratory caller answers the pick when the row it pressed opened one.
- [ ] **Step 3: the corpus.** The walk names a drawn Laboratory row by its title's entry and then the pick's number, the same game played (`log.json` and every earlier fixture byte-identical). One capture, `lab_pick`, at the walk's folded Laboratory prompt (event 50 by the measurement), which is the corpus's two-copies case: both copies of Ball, one row each. Its contract cell reads kind `pick`, the title, and the two rows' words, `Laboratory: acquire Ball` and `Laboratory: acquire Ball, slot fee: 2 yen`. The walk's draw is the first row. The kept list and the cited walk rows move with the walk as in Task 2 (one row).
- [ ] **Step 4: the guide pair** gains the Laboratory's paragraph with the entry and the pick's screen.
- [ ] **Step 5:** the Rust suites; commit.

**Mutants:** grouping adjacent runs (cell 1); grouping by card id (cell 2); the entry placed at the title's last row (cell 1); the entries gathered after the direct rows (cell 6's layout); the pick's rows sorted (cell 2 and cell 3); a fold at one row (cell 4); the entry's count taken from the cards on the board (cell 1: the twin reads 3 ways over 2 cards, the other technology 3 over 1).

### Task 3b: Two copies of a technology are told apart by their slot

**impl: opus · review: opus.** Added during the run on the owner's word (2026-11-22): a player needs to know which of the two cards a row takes.

**Why.** Ten of the twenty technology titles exist as two cards, and both copies can be on one Laboratory board. A row's words name the technology and what is paid on top of the action's power; when the two copies sit in slots that add the same (slot pairs 1/2, 3/4 and 5/6), their rows read exactly alike, in the Laboratory's pick and, before Task 3, on the flat list. Task 3's sweep read that at 8 of its 21 folded positions.

**Files:** modify `crates/yokohama-cli/src/describe.rs` (the Laboratory arm of `describe_area_action`), its tests; `crates/yokohama-cli/src/chooser.rs` (`test_support`, fixtures only) and `crates/yokohama-cli/src/repl.rs` (cells; the sweep); `crates/yokohama-web/tests/wire_fixtures.rs` only if a cell there quotes the words; the regenerated corpus; `web/src/wire/contract.test.ts` (the `lab_pick` cell's two rows); `docs/cli-guide.md`, `docs/cli-guide.html`; any golden or capture that words a Laboratory row on a board holding both copies (search, below).

**The words.** When the ACTING Laboratory board holds another card of the same name as the card a move acquires, the row names the card's slot after the technology's name, numbered as the terminal's `labs` view and the browser's lab strip label it (the first slot is `slot 1`):

```
Laboratory: acquire Ball (slot 2)
Laboratory: acquire Ball (slot 5), slot fee: 2 yen
```

In every other case the row's words are today's, byte for byte. The entry (`Laboratory: acquire {title} — {n} ways to pay`) and the pick's title (`Laboratory: acquire {title} — choose the payment`) do not change. The words are `describe_move`'s, so the marker is the same wherever a move is worded: the main list's direct rows, the pick's rows, the log, `suggest` and `why`, and the browser, which draws those words.

**Interfaces.** The Laboratory arm of `describe_area_action` reads the acting board (`describe::acting_area`, as the Port's chooser finds its board) for DISPLAY only: where the card sits, and whether another card of its name sits on the same board. It decides nothing about legality. No new public item is needed; if one is, the report names it.

- [ ] **Step 1: the cells, red.** Each names the rival rule its data is chosen against.
  1. `a_twin_s_row_names_its_slot`: on `lab_twin_position()`, each of the twin's three rows names its own copy's slot, and the number equals the label the `labs` view prints for that slot (rival: "the slot's index from zero"). The rows of the pick read the same words.
  2. `a_single_copy_keeps_today_s_words`: on the same position the other technologies' rows hold no `(slot`; and on a one-fact clone with the twin's second copy swapped back into the tech deck (a valid state: `check_invariants`), the remaining copy's row is today's (rivals: "always name the slot"; "name the slot for any title that exists twice").
  3. `two_copies_in_slots_that_add_the_same_read_apart`: on a valid position with both copies in a slot pair that adds the same, every pair of the title's rows differs, and rows that differ only by the copy differ only in the slot's number (the owner's case; before this task they were identical).
  4. `the_other_board_s_copy_does_not_count`: at four players, with one copy on Laboratory A and the other on Laboratory B, the acting board's row holds no marker (rival: "two copies anywhere on the table").
  5. `no_two_rows_of_a_laboratory_prompt_read_alike`: Task 3's sweep also asserts, at every Laboratory prompt it reaches, that no two listed Laboratory rows have the same words, with a floor on the positions where both copies stood on the acting board and on those where they stood in slots that add the same, each measured before it is set.
  6. `the_log_names_the_slot_too`: an applied twin acquire's log line carries the marker (it is `describe`'s words for the move at the state it was made from).
- [ ] **Step 2:** red (saved), then the code.
- [ ] **Step 3: what words a Laboratory row today.** Search, do not trust a list: `grep -rn "Laboratory: acquire" crates web/src worker docs --include='*.rs' --include='*.ts' --include='*.tsx' --include='*.json' --include='*.md' --include='*.html' --include='*.txt' -l`. A golden, capture or fixture whose board holds both copies moves; every one that moves is quoted in the report, before and after. One that words a single copy must NOT move.
- [ ] **Step 4: the corpus.** Regenerate (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then the plain run) and READ the diff. Expected, and anything else is a finding: `lab_pick.json`'s two rows gain their slots (event 50, both copies of Ball); every other changed byte is that same marker on the line of the acquire the walk made there (the log's line, and any frame that quotes it); `walk.json`'s input rows do not change (2,242), so the kept list and the cited walk rows do not move. The `lab_pick` contract cell reads the two new rows.
- [ ] **Step 5: the guide pair.** The Laboratory paragraph's sentence about two copies now says that each row names its slot when both copies are on the board, with the example, word-identical in both formats; `guide_sync` green. A doc comment that says the copies can read alike is corrected.
- [ ] **Step 6:** the Rust suites, one at a time; the lint gate; both fence checks; commit.

**Mutants:** the marker on every Laboratory row (cell 2); the marker for any title that exists twice, whatever the board holds (cell 2's clone); the slot's index from zero (cell 1); copies counted across both boards (cell 4); the marker only when the two slots add the same (cell 1, whose copies sit in slots that add different amounts); the marker read from the FIRST card of that name on the board instead of the move's own card (cells 1 and 3: both rows would name one slot).

### Task 4: The Port's panel in the browser

**impl: sonnet · review: opus.**

**Files:** modify `web/src/Decision.tsx`, `web/src/styles.css`, `web/src/Decision.test.tsx`.

**Interfaces.** Consumes the `port` arm, `OrderCardView` (`web/src/Cards.tsx`), `ChooserControls`. Produces `.chooser.port`: the asking line; `.zones` › one `.zone[data-zone]` per zone that has a card, its heading `LOW`, `MID` or `HIGH`, then one `button.card-pick[data-number][aria-pressed][aria-label="{words}"]` per card holding `OrderCardView` over the arm's `printed`; `.verdict[role=status]`, `.ok` with each way's words (and `asked at Confirm` when there are two) or `.bad`; `ChooserControls`. And `isChooser(prompt)`, one helper that the chooser guard and its callers read in place of the kind comparisons. The accepted mockup's Port panel is the reference for structure and class names (`chooser-mockups.html` beside the ledger; read as text).

- A card sends its `number` as the line. Confirm sends `confirm`; Cancel sends `cancel`.
- `disabled` with `data-unavailable` says what the payload says: an unselected card whose `can_add` is false; Confirm while the preview is not legal. While a request is in flight a card is `aria-disabled`, sends nothing on a press and KEEPS the keyboard focus (plan 2a's ruling R45); Confirm and Cancel are `disabled` while busy, which the walks' settle wait rests on.
- The interim branch Task 2 left for `port` is replaced by the panel; `DRAWN_KINDS` keeps `port`.

- [ ] **Step 1: the cells, red** (vitest; each payload from `web/src/wire/`).
  1. From `wire.port`: the asking line, every zone's heading, one button per card in the arm's order with its `data-number` and its accessible name, none pressed, Confirm disabled with `data-unavailable`, Cancel live.
  2. From `wire.portSelected` and `wire.portPair`: the pressed cards; the verdict's words equal `preview.ways`. In every fixture the selected cards stand first, so a clone of `wire.port` with its LAST card's `selected` true reads that card alone pressed (rival: "the first cards are the pressed ones"). With nothing selected and the preview not legal the verdict is empty, as the Church's and Chinatown's panels are.
  3. **`can_add` is the payload's**: on `wire.portPair` every unselected card is disabled with `data-unavailable`; a one-fact clone with one unselected card's `can_add` true draws that card live though two are selected (rival: "disable every other card once two are on"); a clone of `wire.portSelected` with one card's `can_add` false disables that card alone (rival: "live while fewer than two are on").
  4. **`legal` is the payload's**: a clone of `wire.portSelected` with `preview.legal` false disables Confirm though a card is selected; a clone of `wire.port` with `legal` true and one way enables it with nothing selected.
  5. **Two ways**: a clone of `wire.portPair` with a second string in `ways` draws both lines and `asked at Confirm`.
  6. **The order and the zones are the payload's**: the zones are drawn in the order the payload first names them, and the cards in the payload's order within each. A clone with `cards` reversed draws them reversed, the zones' headings too; a clone with one card's `zone` changed draws it under that zone's heading; a clone with one zone's two cards swapped draws them swapped (the one that tells the payload's order from a sort by number, since the fixture reversed IS ascending).
  7. A press on a card sends its number once; a press while busy sends nothing; Confirm and Cancel send their words; a spectator's and a non-acting seat's page has no chooser.
  8. `max_cards` 1 in a clone reads `The Port — take one card`.
  9. `isChooser` is true for the three chooser kinds and false for every other kind in `EVERY_STATE`.
- [ ] **Step 2:** red; the panel; green. `npm run typecheck && npm test` in `web/`; the fence checks; commit.

**Mutants:** a card disabled when two are selected, by count (cell 3's first clone); Confirm enabled by "something is selected" (cell 4); the line sent is the card's index (cell 7, since no fixture's numbers are 1 to 6 in order); the verdict built from the selected cards' ids (cell 2 and cell 5); cards sorted by number (cells 1 to 3, where the fixture's LOW holds 36 before 14, and cell 6's swapped pair; the reversed clone cannot part it); the pressed state taken from a card's position (cell 2's clone); the zones in a fixed order (cell 6's reversed clone); a busy card `disabled` (cell 7's focus half is Task 5's browser cell; here the attribute).

### Task 5: The browser's walks and the layout

**impl: opus · review: opus. The only Playwright runner while it works.**

**Files:** modify `web/e2e/harness.ts`, `web/e2e/shared_table.ts`, `web/e2e/the_table.spec.ts`; `web/src/styles.css` only as a cell below proves it needs.

- The harness's `CHOOSER_ADDS` gains the Port's unpressed, enabled cards. The policy at `port` follows the Chinatown policy's shape: add a card while one can be added and the policy's own draw says so, else Confirm when it is live, else Cancel; never a pressed card. The pick that follows a Confirm is the `pick` the walks already answer.
- [ ] **Step 1: the cells, red** (in `the_table.spec.ts`, beside plan 2a's).
  1. **The boxes at 390 × 844 and 1440 × 900**: every card button inside the answer deck's box, no two overlapping, Confirm and Cancel inside the viewport and under their own centres.
  2. **Landscape, 844 × 390 and 667 × 375** (plan 2a's ruling R38): Confirm and Cancel inside the viewport and under their own centres; whatever is cut can be scrolled into view inside the deck.
  3. **A take through the screen**: open the entry, press a card, Confirm; the log's last line is the take's; the hand has the card.
  4. **Two cards**: press two, Confirm; when the pick opens, answer it; both cards in hand. The cell reads the arm's flags to find a second card that can be added and skips, saying so, on a table where none can.
  5. **The keyboard**: a card keeps the focus across a press and a second press, and a Confirm reached by Tab applies the take.
  6. **The policy never presses a pressed card** (a reading over a walked game, as plan 2a's minus cell reads).
- [ ] **Step 2:** `cd web && npm run build:wasm:debug && node scripts/e2e-build.mjs`, the new cells one at a time, then **the whole suite once** (`npm run e2e`): the count and the wall in the report.
- [ ] **Step 3:** commit.

**Mutants:** the settle wait ignoring a chooser's payload-disabled cards (a walk times out at the first Port chooser); the policy pressing a pressed card (cell 6); the landscape rule removed for the Port (cell 2); a busy card `disabled` (cell 5).

### Task 6: A Port chooser rebuilt on a new Worker instance

**impl: sonnet · review: opus.**

**Files:** modify `crates/yokohama-worker/src/picker.rs` (its test module only).

- [ ] **Step 1: the cells, red**, beside plan 2a's three and built as they are (lines sent to a table, a new instance restored, the prompt read).
  1. `restore_picker_rebuilds_a_port_chooser_with_its_cards`: the lines switch a card on, a second on, the first off and a third on, so that a restore feeding each distinct line once reads another selection. On the cells' table the entry is `1` and the cards are 34, 19, 12 and 4 (measured), so the entry's line equals no card's; the implementer confirms that and says which lines it chose.
  2. `a_refused_card_is_not_replayed_into_a_selection`: a third card refused, then the restore: the two cards, nothing else.
  3. `cancel_clears_the_port_s_lines`.
  4. No cell for the payment pick here: on this table no pair is payable both ways by lines (the seat holds no imported good at the opening, measured). A `confirm` on a pair commits; the pick is carried by the REPL's cells and by Task 7's game, and a pick's restore is the mechanism the route and Telegram picks already prove.
- [ ] **Step 2:** red, then green with NO change outside the test module (the mechanism covers a new sub-prompt unchanged; a change it needs is a STOP). `cargo test -p yokohama-worker`; `cd worker && npm run build:module:debug && npm test`; commit.

**Mutants** (planted in the export's `picker.rs` and `repl.rs`): the restore de-duplicating lines (cell 1); a refused card entering the selection (cell 2, read straight after the refusal); a refused line NOT recorded (cell 2's reading of the stored lines: recording a refused line is the mechanism's design, pinned by plan 2a's refused cell, so "a refused line recorded" is no mutant); the restore feeding the lines out of order (cell 2, after the restore); `cancel` leaving the list (cell 3).

### Task 7: A whole game at three players through the web layer, and what a press costs

**impl: opus · review: opus.** Test code and one document paragraph; no product code.

**Files:** modify `crates/yokohama-web/tests/play_a_whole_game.rs`, `crates/yokohama-web/src/api.rs` (one test), `crates/yokohama-cli/src/repl.rs` (one test), `docs/web-lane.md`.

- [ ] **Step 1: the third arm of `play_a_whole_game.rs`**: a bot-free three-player game, one caller holding every seat, played to a winner through the API. `choose` gains the `chinatown` arm in place of its refusal, since a seed now runs that branch (Confirm when `preview.legal`, else a row with `can_add`, else Cancel: every flag the arm's own) and uses the `port` arm Task 2 gave it (a card on a draw while one can be added, else Confirm, else Cancel), and answers the picks as it answers any pick. The arm counts and prints: Chinatown confirms, Port confirms, payment picks opened, Laboratory picks opened. **Floors of one on each of the four**, measured on the arm's fixed seed before they are set; if the seed reaches no payment pick, choose a seed that does and say which were tried. An input cap sized from the measured length, stated in the test.
- [ ] **Step 2: the two-player arms** learned `port` in Task 2 and cannot meet `chinatown` (that board has none); this step re-reads them green. `lib.rs`'s two-player `play` walk keeps its refusal of `chinatown`.
- [ ] **Step 3: design §9's cost, measured and recorded.** Two cells that print milliseconds and assert nothing. In `yokohama-cli`'s tests (`repl.rs`): opening the chooser and one add at `chooser::test_support::rich_position()` (2,002 listed moves; the plan's review read about 12 ms each in debug). That is design §9's figure, the longest Chinatown list, and it is the REPL's cost, since the fixture is `pub(crate)` to the CLI's own test build and the web crate cannot reach it. In `yokohama-web`'s tests (`api.rs`): one add through `api::input` at `to_the_chinatown` (127 combinations) and one card at `to_the_port`, which is the layer's cost at the lists lines reach. Run both in debug and with `--release`; write the four readings, the machine and each list's length into `docs/web-lane.md` beside the corpus's figures, as readings and not bounds, each named for what it is. The private Worker's reading is the orchestrator's, at the deploy.
- [ ] **Step 4:** `cargo test -p yokohama-web` (state the arm's wall); the lint gate; commit.

**Mutants:** the `chinatown` arm pressing a row whose `can_add` is false (the game stalls under the input cap); the `port` arm confirming while not legal (the same); a floor set above what the seed reaches (red, as it should be: the floors are readings).

### Task 8: The terminal's flag counts: the `players` view and the Mercenaries row

**impl: opus · review: opus.** Design §5.8 as the owner ruled it on 2026-11-22 (choice D of the ruling's section): the `players` view in full, and the counts on the dashboard's Mercenaries row.

**Files:** modify `crates/yokohama-cli/src/render.rs` (`players_panel` and its tests), `crates/yokohama-cli/src/panel_render.rs` (`mercenaries_line`, the two-player arrangement's; `mercenaries_row`, the seat column's), `crates/yokohama-cli/src/screen_model.rs` (the codes and their pin), the committed frame captures that quote the row (`t3-frame-2p.txt`, `t10-frame-3p.txt`), `docs/cli-guide.md`, `docs/cli-guide.html`.

**Interfaces.** Consumes `PlayerModel::flags` (the dashboard), `screen_model::face_up_flags(p, rules)` and `screen_model::order_flags(ids, rules)` (the `players` view, which reads the state), all cycle 1's (no new count), and the view's own mask, `SeatDisplay::hides_hand`. Produces `screen_model::country_code(Country) -> &'static str` (`GB`, `US`, `FR`, `DE`, `NL`): the engine's own table is private, so this is a mirror with a drift pin, the `placement_fee` precedent, pinned against `order_compact_line`'s country token for one registry card of each country.

**What a mock already read** (an export with the row patched, run in a real terminal at 200 columns, 2026-11-22): the opening frames at three and four players and a two-player game 700 events in, where the two seats read `GB1 US0 FR0 DE1 NL1` and `GB1 US1 FR1 DE1 NL0`. The frame's heights did not move (51, 50, 49). `cargo test -p yokohama-cli --lib` on the mock read 885 passed and 2 failed, the two committed frame captures, which quote the row.

- [ ] **Step 1: the cells, red.**
  1. The pin: for a card of each of the five countries, `order_compact_line`'s second token equals `country_code`.
  2. `the_players_view_lists_each_seat_s_flags`: on a state whose seat has completed orders and technologies giving five DISTINCT non-zero counts, that seat's line is "The words"' with those counts in `Country::ALL`'s order (rival: two countries swapped); a seat with nothing face up reads five zeros; a seat whose hand is hidden still carries the line.
  3. `the_flags_in_hand_are_apart_and_masked`: a shown hand with orders prints the second line with the non-zero countries of the HAND; a seat with non-zero face-up flags and an empty shown hand prints no second line (rival: reading the face-up counts); a hidden hand that holds orders prints no second line (rival: printing it for every seat).
  4. `the_mercenaries_row_carries_the_face_up_flags`: in both arrangements the row ends with the five counts of `PlayerModel::flags`, on a model with five distinct non-zero counts (rival: swapped countries) and on a seat whose hand is hidden (the counts are face up and never masked); a zero count is dimmed and a non-zero one is not.
  5. `the_used_count_yields_first_and_whole`: at a three-player seat's width the row keeps `· N used`; at a four-player seat's it reads without it and is never clipped; the two-player line always keeps it. Then the widest LEGAL row, measured as `the_widest_legal_seat_tail_row_still_fits_its_column` measures its row: the largest mercenary and flag counts a seat can hold, at each width. If the widest row still outruns a four-player column after `used` has yielded, the policy's next step is that the countries at zero yield, and "The words" says so in the same commit; the report states the measured widths either way.
  6. The frame's heights are unmoved: `screen`'s opening-frame and game-walk cells read green UNEDITED. The two committed captures are regenerated (`UPDATE_CAPTURES=1`) and their diffs READ: one row per seat changes and nothing else; both diffs are quoted in the report.
- [ ] **Step 2:** red; the code; every golden that moves quoted before and after.
- [ ] **Step 3:** the guide pair: the `players` view passage gains the two lines, and the dashboard's passage gains the row with its yield. `cargo test -p yokohama-cli`; `cargo test -p yokohama-web` (the frame's models are the wire's and this task changes no model: `wire_fixtures` reads unmoved, or its diff is read and explained); the lint gate; both fence checks; commit.

**Mutants:** the in-hand line reading the face-up counts (cell 3); two countries swapped (cells 2 and 4); the code table drifted from the engine's (cell 1); the in-hand line printed for a hidden hand (cell 3); the flags masked with a hidden hand (cell 4); the row clipped at four players in place of yielding (cell 5); `used` dropped at three players too (cell 5).

### Task 9: The browser's words and the lane document

**impl: sonnet · review: opus.**

**Files:** modify `web/src/Help.tsx`, `web/src/Help.test.tsx`, `docs/web-guide.md`, `docs/web-lane.md`.

- [ ] **Step 1:** P1, P2 and P3 of "The words" enter the help and the guide after plan 2a's two sentences, word for word in both; the help's cell holds them on both renders (with and without pictures) and in order, as it holds S1 and S2.
- [ ] **Step 2: try each clause against the product before committing** (plan 2a, lesson 7), and say in the report what was tried for each: "beside the decline"; "one card or two"; "goes dark when your hand has no room"; "or you cannot pay for it"; "when you could pay … either … Confirm asks which" (only when BOTH can be afforded); "a single row … opens the list of those ways". A clause that is false is corrected in "The words" first, then transcribed.
- [ ] **Step 3: `docs/web-lane.md`**: §1's exception sentence names the Port's chooser and the Laboratory's fold; the prompt kinds where it lists them (`port` is the corpus's eleventh kind and `PromptState`'s twelfth variant; re-count both); §2's suite counts and the corpus's figures re-counted at this head (the pool's stale 298 among them); the production-door list's figures.
- [ ] **Step 4:** `npm run typecheck && npm test` in `web/`; `cargo test -p yokohama-web --test lane_document`; `cd worker && npm run test:scripts`; commit.

**Mutants:** a sentence in the help alone, or in the guide alone (the help's cell and its absence from the guide are both read; the guide's copy is pinned by no test, which the report says); the order of the sentences (the order cell).

## How a player gets this release

Confirmed by reading the client's update path (`web/src/swRegister.ts`, `sw.ts`, `UpdateLine.tsx`, `docs/web-guide.md` §2) and pinned by the browser cells of `web/e2e/offline.spec.ts` ("the update: a second release waits, nothing switches until the click", "the door takes a release by itself", "the update offered to a tab NO worker controls", "the tab that did not click"), which read green in plan 2a's gates. This plan changes none of it.

- **A browser that has never held the site** gets the new client from the network.
- **A browser that holds an earlier release** opens on that release, because its service worker answers a page load from what it holds. The page then finds the new release and installs it in the background. **On the front page the release is taken without a click**: the door reloads itself once the install has finished. Both "You against the bot" and "Play with friends" start from that page. A lobby or a game opened by its own link does not pass the front page and shows the line below instead.
- **In the lobby or a game**, the line `A new version is ready. Reload to use it` appears and waits for the click; nothing reloads by itself while someone plays. An open tab asks for a new release every fifteen minutes and whenever it comes back into view.
- **An ordinary refresh of a lobby or game tab does not take a waiting release.** This one is reasoned and not pinned: the worker hands over only when a tab asks (the line's click, or the door), or, by the browser's own rule, once every tab of the site is closed, and none of the cells above reloads such a tab while a release waits. A hard reload goes round the worker and loads the release from the network.
- **A game against the bot** runs in the tab, engine included, so a tab on the earlier release plays the earlier lists with no dead end; the game is stored in the browser and is there after the reload.
- **A table with friends** is answered by the new server from the moment of the deploy. A tab that was already at a table across it is the window the owner accepted on 2026-11-22 (`docs/web-lane.md` §7; his word was that a refresh is required, and in the client that refresh is the line's click): the click brings it up to date, and from plan 2a's client on a kind it cannot draw reads `This screen needs a newer version.` with a Cancel. A mechanic that makes open tabs refresh is issue #78.

## The owner's ruling: where the terminal shows the flags (D, 2026-11-22)

Design §5.8 (`docs/plans/2026-11-21-web-area-action-choosers-design.md`, lines 145 to 147): "The terminal's players panel draws the five per-country counts of face-up flags for every seat (`screen_model::face_up_flags`) and, for an open hand, the flags of the orders in hand apart from them (`HandPanel::flags_in_hand`), marked as not yet counting."

The terminal has two things a reader could call its players panel: the always-visible dashboard's PLAYERS panel (`panel_render.rs`) and the `players` view (`render::players_panel`). The dashboard has no row to spare. `crates/yokohama-cli/src/screen.rs`, the record above `FRAME_HEIGHTS` (lines 338 to 360): the two-player opening frame "CROSSED THE FLOOR: 49 → 51" when the Orders line was added, "The 3- and 4-player openings, which are ceilings, are still inside the floor", and "The owner adjudicates whether the row is worth its cost; nothing here may be raised further to keep a run quiet."

Measured by the plan's review, the opening frame's rows against the 50-row floor:

| | today | a `Flags` line per seat | and a line for the flags in hand |
|---|---|---|---|
| two players | 51 | 53 | 54 |
| three players | 50 | 51 | 53 |
| four players | 49 | 50 | 52 |

What follows from each choice:

- **A. The `players` view only.** No frame row moves; the counts are one command away (`players`), for every seat, with the flags in hand apart for a shown hand. The dashboard below the board already names each hand card's country.
- **B. A row per seat in the dashboard.** The three-player opening leaves a 200x50 window for the first time (51 rows) and the two-player one goes to 53. `FRAME_HEIGHTS`, the game-walk bounds, two committed frame captures and the guide's §2 figures are re-measured, and the hosted terminal's 200x50 floor no longer shows the whole three-player screen. The in-hand line has no place below the board (that hand panel is one row).
- **C. Both: the `players` view, and the dashboard's row at two players only**, where the frame is already outside the floor and its height was never a ceiling (53 rows).

- **D. The `players` view in full, and the counts on the dashboard's Mercenaries row (the plan's proposal).** No frame row moves. Found by looking for a row the counts belong on: every second flag of one country is what earns a mercenary, so the row reads the cause beside the effect. A mock was built on an export and read in a real terminal: `Mercenaries  0 unused · 0 used · GB1 US0 FR0 DE1 NL1` at two and three players, and at four players, where a seat's column is 48 wide, `Mercenaries  0 unused · GB0 US0 FR0 DE0 NL0`, the `used` count yielding as `next take` already yields on the seat's last row. The frame's heights read 51, 50 and 49, unmoved; the only cells that changed were the two committed frame captures, which quote the row. Its cost: at four players the `used` count is read in the `players` view, and the compact form (`GB1`) is denser than the rest of the panel.

**Ruled: D.** The owner, asked on 2026-11-22, leaned to the `players` view and found the dashboard useful; shown the mock in a real terminal, he ruled for the Mercenaries row. Task 8 is written for D. A, B and C are recorded as what was weighed.

## The plan's review (2026-11-22)

An independent reviewer measured this plan against the tree on an export before any task ran: one Critical finding, five Important, seventeen Minor. Every figure of "What was measured" reproduced; every fixture can be built as a state the engine's invariants accept. All are settled in the text above; the Critical was settled by the owner's ruling in the section before this one.

- **The terminal's flag line does not fit the frame** (Critical, measured by adding it): neither a row per seat nor the orders line. Task 8 now reads the `players` view and, after a mock was read in a real terminal at every player count, the dashboard's Mercenaries row, which costs no row; the owner ruled for it.
- **`?` cannot be refused inside the Port's chooser**: the REPL answers it ahead of every sub-prompt and plan 2a pinned that. It opens the order index over the chooser, which design §9 asks for.
- **A driver that confirms as soon as the preview is legal never takes a pair**: one card is always legal. The drivers' arm now adds a card on a draw first, and the payment pick's floor can be reached.
- **The longest Chinatown fixture is out of the web crate's reach**: design §9's cost is timed in the CLI crate at 2,002 listed moves, and the layer's cost at the lists lines reach, each named for what it is.
- **vitest would have gone red at Task 2**: the unknown-kind cells used `port` as their unknown kind. Task 2 now moves those cells and draws `port` by an interim branch of its own.
- **The pick prints a colon after its title**: the two titles are passed without one.
- From the Minors: the plan's own readings were first taken on invalid states and are now said to be so, with the valid build; the walk's capture keeps the first occurrence, so the Port captures' prompt is named by a condition; event 50 is the two-copies case; two of the Port's lines cannot be reached by lines and are read on a list with a single removed; the hand-room case has its REPL cell; `group_area_action` needs the tech registry; the runbook's cited walk rows move with the walk; Task 2's halves cannot each be green; the update path's refresh sentence is reasoned, not pinned, and a hard reload is the exception; three mutant attributions and one vacuous clause corrected.

## After the tasks — the orchestrator's, in order

By the owner's direction of 2026-11-22 this plan includes the deploy to the private test Worker, the verification there and, if it passes, the documents a separate session needs to take the work to main and to the production hosts (his words are in the run's ledger). That direction covers the run of this plan, ONE target for a deploy (the private test Worker, `yokohama-test.moore.nyc`) and the documents of step 5. It covers nothing on origin, no tag, no version bump and no production host: those are the wrap's, in a separate session, on the owner's word there.

1. **The whole-branch review**, on an export: a Workflow of about three agents led by Fable (the whole diff against the design; the mutants re-run at the head, the only Playwright runner; the wire, the mask and every host). One fix wave and a scoped re-review.
2. **The gates at the branch's head.** Every per-edit suite re-counted. The fifteen release gates as ONE uninterrupted sequence with the release binaries pre-built, the wall machine-tagged. No fenced crate moves, so fourteen are expected to read as at plan 2a's head; the piped-dogfood gate is the one this plan changes.
3. **The private test Worker carries the reviewed head**, by `worker/RUNBOOK.md` §26 (its own keyless client build with the four values of `docs/web-lane.md` §14, `npm run build` in `worker/`, `npm run deploy:private` through both guards), recorded in §26 as a new record. The owner's word above is the word for this deploy, and for a second one there if step 4 finds a fault that the run then fixes. A deploy of this change drops a picker left open there. The version before it is the rollback target (`eba04eca-d30c-427c-9651-ede6693e3f00`, plan 2a's head).
4. **The verification on the deployed private Worker.** It PASSES when every line below reads as stated; a line that cannot be read is reported as not read, never as passed.
   - The recipe's readings over WARP: `/api/health` 200 at the workspace version and `semantics_rev` 7 with `cache-control: no-store`; `/api/me` the caller under Access; `/` 200 with the enforced header set; `sw.js` naming the new release; the client bundle carrying the Port panel's asking line.
   - **Chinatown, unmoved**: a four-player solo table dealt with the walk's `wire4` seed and lines (the 2026-11-22 record's method and script), whose two chooser replies equal the committed `chinatown.json` and `chinatown_selected.json` in every top-level key but the table's `id`.
   - **The Port's chooser on a real host**: a two-player solo table, every seat the caller's, with the corpus walk's seed (20260831): the additional actions ended, Option B's two assistants at Port A, the President placed there; the entry (`1`) answers a `port` prompt with four cards in reach (34, 19, 12 and 4 by the measurement); one card's reply has it selected and the preview legal with one way; a second card's has two selected; `confirm` applies the take and the log's last line is the take's; the table is then abandoned. Each reply's `messages` carries no line of the terminal's screen.
   - **The Laboratory's fold on a real host**, if a title with more than one row is reachable by lines on a table of the orchestrator's own; if it is not, said so (the corpus replay in the Worker's pool carries `lab_pick`).
   - **One press timed** there (Task 7's reading for the Worker): the wall of one Chinatown add and one Port card, as read by the caller.
   - **The update path**, in a browser that holds plan 2a's release if the orchestrator's browser can hold one (the front page takes the new release with no click; a game tab shows the line and its click reloads); otherwise it is named as the owner's reading and does not hold the pass.
   - The live replay is NOT run: it is the release window's.
   If the verification does not pass: the fault is fixed on the branch through a reviewed fix (a task-sized fix has its review; a deploy follows only when the suites it touches are green), or the run stops and reports. Step 5 does not start on a failed verification.
5. **If, and only if, the verification passes: the documents the wrap needs**, so that a separate session can take the work to origin and to production from a handover alone.
   - **The cycle's close** (`docs/plans/<date>-web-area-action-choosers-close.md`): what cycle 2 built in its two plans; both whole-branch reviews; the gates at this plan's head; the register of rulings from BOTH plans' ledgers, every id the tree cites resolved; what plan 2a's last section recorded for the close; the deferred minors; what the wrap owes.
   - **`CLAUDE.md`**: the grammars line (the Port's and Chinatown's and the Church's choosers, the Laboratory's fold), and the standing sentence "every model is `screen_model`'s, every word `describe`'s" made exact (the choosers' models are `chooser`'s). Standing rules only, by that file's own rule.
   - **The board** (`docs/handover.md`): Track 6 rewritten for "cycle 2 closed on the lab; the wrap next".
   - **The wrap's handover** (`docs/plans/<date>-track6-wrap-handover.md`), written for a session that has read nothing else. It carries: the three lab branches, their heads and what each closes (#75 and #76; #77); how they reach origin under the two-repository rule ("a branch reaches origin (public) ONCE, when that review is clean, as the pull request that merges it", master moving only by a merge commit), with the choice the owner asked for, one clean branch to main or one pull request per cycle, laid out with what each costs and a recommendation, for his ruling there; the release number by `docs/web-lane.md` §7 and the tag; the production deploy on the three tiers in the owner's window, by the two runbooks, the NAS first as the canary; the live replay from the release's tree (the kept list and the cited walk rows moved in this cycle); the help's pictures re-cut once on the final tree and the README's pictures; the decision-log rows owed; the issues to close (#75, #76, #77) and the one left open (#78); the lane document's §2 counts; the commit trailers before plan 2a's fix wave, left as pushed; every item cycle 1's close and this cycle's close say the wrap owes; and what the owner decides there.
   - Nothing in this step pushes to origin, tags, bumps a version or touches a production host.

Not this plan's: origin, the release, the production deploy, the help's pictures and the decision-log rows (the wrap, in its own session); the forced-refresh mechanic (issue #78).
