# The goods choosers: the Church and Chinatown — implementation plan 2a (2026-11-21)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** At the Church and at Chinatown the main list reads as one entry and the decline; the entry opens a chooser in the REPL (kinds switched on and off at the Church, exchanges counted up and down at Chinatown), and the browser draws that chooser with goods and yen tokens.

**Architecture:** A new pure module in the CLI crate, `chooser.rs`, builds each chooser's model by looking moves up in the legal list; the REPL holds the selection as a `SubPrompt`, draws the terminal's screen from the model and commits only the listed move the selection names. The web layer serializes the same model as two new `PromptState` arms, and the browser draws them. Nothing computes a rule: every "offered", "can add" and "legal" is a lookup in `session.legal()`.

**Tech stack:** Rust 1.97.1 (`yokohama-cli`, `yokohama-web`, `yokohama-worker`), TypeScript, React, vitest, Playwright 1.62.

**Spec:** `docs/plans/2026-11-21-web-area-action-choosers-design.md` (accepted 2026-11-21). Read it before any task; where this plan and the design disagree, stop and report. This is the first of its two plans (§10): the Port, the Laboratory and the terminal's flag counts are plan 2b's and no task here touches them.

**Branch:** `web-choosers-goods`, on the lab, cut from cycle 1's lab head. `BASE` below is the commit the branch was cut from; the ledger records it. Tasks land as new commits; nothing reaches origin.

## Global constraints

- **The lane's fence** (`docs/web-lane.md` §1): `yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and `gen-data` byte-identical; `SEMANTICS_REV` 7; no bot behaviour moved. A task that finds it needs an edit under any of the five STOPS and reports. Check at every task's end: `git diff --stat BASE -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` prints nothing.
- **The web's line fence does not move**: `git diff --stat BASE -- crates/yokohama-web/src/grammar.rs` prints nothing, its test module included.
- **The legal list is the only legality authority.** Whether a kind is offered, whether one more exchange can be added, whether a selection can be confirmed: each is answered by finding a listed move. No affordability, power or track rule is restated in `chooser.rs`, `repl.rs`, `client.rs` or `web/`. The engine's public helpers `calculate_power`, `calculate_church_faith` and `chinatown_rate` are read for what is DISPLAYED only, as `describe` reads them today.
- **The REPL's behaviour moves on purpose, and only where a task says so.** The guide pair (`docs/cli-guide.md` and `docs/cli-guide.html`) moves in the same commit, word-identical; `guide_sync` stays green. An in-source golden that moves is quoted in the report, before and after; none is edited without its diff shown.
- **The words are fixed in "The words" below.** That section is the single source for every screen, entry and refusal; the cells and the guide quote it.
- **TDD**: the failing cell first, run red, then the code. Suites green and `cargo clippy --workspace --all-targets -- -D warnings` clean after every task. No `cargo fmt`.
- **A payload in a client cell comes from `web/src/wire/`**, never from a builder. A JSON round-trip clone with one field changed is the existing practice.
- **No agent opens an image**: no `.png`, `.webp` or `.jpg` anywhere, the token art (`crates/yokohama-web/assets/art/`) included. A layout claim is a measured box; how the tokens look is the owner's reading.
- **Every invisible non-ASCII code point in code is a `\u` escape.** `—`, `●` and `→` are visible and may be raw.
- **Git**: stage with explicit paths, never `git add -A`; new commits only, never an amend; implementers do not push. Commit messages end with `Co-Authored-By: Claude Mythos 5.2 <noreply@anthropic.com>`, whichever model writes the commit.
- **Dispatch rules** (every implementer and reviewer): never `rm -rf`, `rm -r` or a bulk `rm -f`; long runs in the foreground or polled with short reads, never a Monitor or a wait on an event; the full report goes to `.superpowers/sdd/2026-11-21-web-area-action-choosers-2a-plan/<role>-task<N>.md` and the reply is the status line, the counts and that path; a reviewer works on an export (`git archive`) with its own `target/`, never in the checkout.
- **Probe before reporting.** A code block here is the single source: an implementer who finds it wrong corrects the plan's block in the same commit, transcribes it, and reports DONE_WITH_CONCERNS naming the correction. Each task lists its mutants: the implementer runs them as a probe, the task reviewer runs them on an export, the whole-branch review runs them again. The overlap is accepted.
- **A cell's data must tell the right answer from the nearest wrong one** (cycle 1's close, §7). Where a cell could pass under a plausible wrong rule, its fixture is chosen so it cannot; the tasks below name the wrong rule each fixture is chosen against.
- **Two stale-module traps.** `npm run test:wasm` (in `web/`) and the Worker's pool (`npm test` in `worker/`) replay the corpus through a COMPILED module: after any Rust change run `cd web && npm run build:wasm` before the first and `cd worker && npm run build:module:debug` before the second.
- **The suites of a task that moves Rust** ("the Rust suites" below): `cargo test -p yokohama-cli`, `-p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`; `cd worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test`; the lint gate; both fence checks. A task that moves no Rust file runs the web commands its steps name and the fence checks.
- **The browser suite is red from Task 3 until Task 7 lands, by design.** Once a list folds, a walk in `web/e2e/` meets a prompt it cannot answer until the panels (Tasks 5, 6) and the walks' policy (Task 7) exist. No task between them runs `npm run e2e`; Task 7 runs it whole.
- **Counts are readings.** Re-count every suite at your own HEAD and state the counts in the report; a count that differs from the board's is a finding to explain.

## Review focus

Five conditions the design implies and no cell of its §6 exercises; each has its cell in the task named.

1. **A number outside the chooser's range** (`0` or `7` at the Church, `10` at Chinatown) is refused with the range, and the selection is kept (Task 3 cell 4; Task 4 cell 3).
2. **The queued church-space choice after a confirm**: the chooser closes and the next prompt is the engine's space choice, not the chooser again (Task 3 cell 5).
3. **`undo` after a chooser's commit** returns to the main list with the entry folded, not into the chooser (Task 3 cell 7).
4. **Someone who is not the acting seat while a chooser is open** (the other player, a watcher) receives `watching` and no chooser arm (Task 3 cell 13).
5. **A phone's width**: with Chinatown's nine rows open at 390 px, Confirm is on screen and can be pressed (Task 7 cell 2).

## The words

Entries on the main list (`{n}` is the count of listed moves in the family, `{k}` the most exchanges in one listed move):

```
Church — choose what to pay ({n} payment sets)
Chinatown — choose exchanges (up to {k}; {n} combinations)
```

Opening a chooser prints no line of its own: the screen's first line says what to do.

**The Church's screen.** Six rows in a fixed order. A row is `  [N] ` then the kind's words padded to 17 columns, then `have H` padded to 10, then a mark: `●` when selected, `(not offered)` when no listed move pays that kind; trailing spaces trimmed.

```
Church — pay any of these, at most one of each (toggle by number):
  [1] copper           have 1
  [2] silk             have 5    ●
  [3] tea              have 2    ●
  [4] seafood          have 3
  [5] 2 yen            have 2    ●
  [6] imported good    have 1
Selected: silk, tea, 2 yen
  -> legal — Church: pay silk, tea, 2 yen -> faith 8 (power 5 + 3 kinds)
> type a number to toggle, 'confirm' to pay, or 'cancel'
```

With nothing selected the line reads `Selected: (none)`. That empty selection can itself be a listed move: at power 4 or more with an empty space the power alone reaches, the chooser opens on `  -> legal — Church: pay nothing -> faith 4 (power 4 + 0 kinds)` and `confirm` pays nothing. A selection the list does not hold reads:

```
Selected: silk
  -> NOT currently offered (adjust the selection)
     the list offers faith 8 to 11 here
> type a number to toggle, 'confirm' to pay, or 'cancel'
```

The third line is `the list offers faith {lo} here` when the lowest and highest listed faith are equal.

**Chinatown's screen.** Nine rows in the engine's order. Row 1 is prefixed `  sell  `, row 5 `  buy   `, every other row eight spaces; then `[N] `, the exchange's words padded to 28 columns, then a mark: `x{count}` when chosen, `(not offered)` when no listed move uses that exchange; trailing spaces trimmed.

```
Chinatown — up to 3 exchanges (add by number):
  sell  [1] 1 copper -> 2 yen           x1
        [2] 1 silk -> 2 yen
        [3] 1 tea -> 1 yen
        [4] 1 seafood -> 1 yen
  buy   [5] 2 yen -> 1 copper
        [6] 2 yen -> 1 silk             x2
        [7] 1 yen -> 1 tea
        [8] 1 yen -> 1 seafood
        [9] 4 yen -> 1 imported good
Selected (3 of 3): 1 copper -> 2 yen, 2 yen -> 1 silk, 2 yen -> 1 silk
  -> legal — Chinatown: 1 copper -> 2 yen, 2 yen -> 1 silk, 2 yen -> 1 silk
> a number adds one, 'back <number>' takes one back, 'confirm', or 'cancel'
```

This is the screen of Task 2's fixture with one copper sale and two silk purchases chosen. No row is marked there: with three exchanges even the silk sale is listed, behind a silk purchase. With the same stock at power 1, rows 2 and 9 carry the mark, row 2 reading `        [2] 1 silk -> 2 yen             (not offered)`.
There the heading reads `Chinatown — up to 1 exchange (add by number):`, in the singular, over `Selected (0 of 1): (none)` when nothing is chosen.

`Selected` lists the chosen exchanges in row order, each repeated by its count. With nothing chosen: `Selected (0 of 3): (none)` over `  -> add an exchange to preview it`. A selection the list does not hold: `  -> NOT currently offered (adjust the selection)`.

**Refusals and cancels** (`{words}` is the kind's or the exchange's words):

| Where | Input | Line |
|---|---|---|
| Church | a kind no listed move pays | `! {words} is not in any payment the list offers here` |
| Church | a number outside 1 to 6 | `! pick a kind 1-6, 'confirm', or 'cancel'` |
| Church | anything else | `! toggle a kind by number, 'confirm', or 'cancel' (cancel first to use views/help)` |
| Church | `cancel` | `Church payment cancelled — back to the main options.` |
| Chinatown | one more that no listed move holds | `! the list holds no set with one more "{words}" from here` |
| Chinatown | a number outside 1 to 9, alone or after `back` | `! pick an exchange 1-9, 'back <number>', 'confirm', or 'cancel'` |
| Chinatown | `back N` with none of N chosen | `! no "{words}" is selected to take back` |
| Chinatown | `confirm` with nothing chosen | `! add an exchange first, or 'cancel'` |
| Chinatown | anything else, bare `back` included | `! add by number, 'back <number>', 'confirm', or 'cancel' (cancel first to use views/help)` |
| Chinatown | `cancel` | `Chinatown exchanges cancelled — back to the main options.` |
| both | `confirm` on a selection the list does not hold | `! that selection is not currently available — adjust it or 'cancel'.` (Option A's own line) |

`HELP_TEXT`'s `cancel` line gains `, an area-action chooser` at its end.

## The files

| File | Responsibility in this plan | Tasks |
|---|---|---|
| `crates/yokohama-cli/src/chooser.rs` (new) | the chooser models, the matchers, the terminal's two screens | 1, 2 |
| `crates/yokohama-cli/src/screen_model.rs` | `CHURCH_YEN_COST`, the mirror | 1 |
| `crates/yokohama-cli/src/describe.rs` | the kind's words from one table; two helpers made `pub(crate)` | 1, 2 |
| `crates/yokohama-cli/src/repl.rs` | the fold, the two sub-prompts, their input, `chooser_screen`, `HELP_TEXT` | 3, 4 |
| `crates/yokohama-cli/tests/piped_dogfood.rs` | the driver learns the Church's screen | 3 |
| `docs/cli-guide.md`, `docs/cli-guide.html` | the choosers, in the commits that move the REPL | 3, 4 |
| `crates/yokohama-web/src/client.rs`, `api.rs` | the two `PromptState` arms; a chooser's screen left out of `messages` | 3, 4 |
| `crates/yokohama-web/src/lib.rs` (tests), `tests/play_a_whole_game.rs`, `tests/wire_fixtures.rs` | every driver that answers by row number; the corpus walk | 3, 4 |
| `web/src/wire/*.json`, `web/src/wire-script/*.json` | the corpus: five files added | 3, 4 |
| `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts` | the wire's types and key sets | 3, 4 |
| `docs/web-lane.md` | §1's fence sentence, §2's counts, the prompt kinds | 3, 4 |
| `web/src/Token.tsx` (new), `web/src/Decision.tsx`, `web/src/App.tsx`, `web/src/styles.css` | the token and the two panels | 5, 6 |
| `web/e2e/harness.ts`, `web/e2e/the_table.spec.ts` | the walks' policy, the layout cells, the chooser walks | 7 |
| `crates/yokohama-worker/src/picker.rs` (tests) | a chooser rebuilt on a new instance | 8 |
| `web/src/Help.tsx`, `docs/web-guide.md` | the browser's words | 9 |

---

### Task 1: The Church's chooser, as a pure module

**Tier:** impl sonnet · review opus.

**Files:**
- Create: `crates/yokohama-cli/src/chooser.rs`
- Modify: `crates/yokohama-cli/src/lib.rs` (`pub mod chooser;`), `crates/yokohama-cli/src/screen_model.rs`, `crates/yokohama-cli/src/describe.rs`

Nothing folds in this task and no prompt changes: it adds functions and their cells.

**Interfaces — produces:**

```rust
// screen_model.rs
/// What the Church's yen payment kind costs. The engine holds it as a literal
/// inside `validate_church_action` (`p.money < 2`); this is its mirror, held to
/// the engine by `the_church_yen_kind_costs_what_an_apply_charges`
/// (the `placement_fee` precedent: a mirror plus a pin, no engine edit).
pub const CHURCH_YEN_COST: u32 = 2;

// describe.rs — the ONE table of a payment kind's words; the Church arm of
// `describe_move` reads it, so "2 yen" is written in one place.
pub(crate) fn church_kind_words(good: crate::chooser::Good) -> String;  // copper, silk, tea, seafood, "2 yen", imported good
pub(crate) fn acting_area(state: &GameState) -> Option<AreaId>;         // the existing helper, visibility widened

// chooser.rs
/// A thing a chooser spends or gains: a trade good, yen, or an imported good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Good { Copper, Silk, Tea, Seafood, Yen, Imports }

/// The Church's six payment kinds, in the order of `AreaAction::Church`'s fields.
pub const CHURCH_KINDS: [Good; 6] =
    [Good::Copper, Good::Silk, Good::Tea, Good::Seafood, Good::Yen, Good::Imports];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount { pub good: Good, pub amount: u32 }

/// What stands under a chooser's selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    /// The legal list holds a move the selection names.
    pub legal: bool,
    /// `describe_move`'s words for that move; `None` when `legal` is false.
    pub words: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchKind {
    pub number: usize,   // what to send: 1 to 6
    pub good: Good,
    pub amount: u32,     // 1, or CHURCH_YEN_COST for yen
    pub words: String,   // church_kind_words
    pub have: u32,       // the acting seat's stock of `good`
    pub selected: bool,
    pub offered: bool,   // some listed Church move pays this kind
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchModel {
    pub kinds: Vec<ChurchKind>,          // always six, in CHURCH_KINDS order
    pub preview: Preview,
    pub offered_faiths: Option<String>,  // "the list offers faith 8 to 11 here"
}

/// The acting seat's (`state.current_player`'s) stock of `good`.
pub fn stock(state: &GameState, good: Good) -> u32;
/// A listed Church move's six flags in CHURCH_KINDS order; `None` for any other move.
pub fn church_flags(mv: &Move) -> Option<[bool; 6]>;
/// The listed move whose six flags equal `selected`.
pub fn church_match(legal: &[Move], selected: [bool; 6]) -> Option<Move>;
pub fn church_model(state: &GameState, rules: &Rules, legal: &[Move], selected: [bool; 6]) -> ChurchModel;
/// The terminal's screen for the model — "The words", byte for byte.
pub fn church_text(model: &ChurchModel) -> String;
```

`offered_faiths` is `None` when the preview is legal or the list holds no Church move. Otherwise it is the sentence over the lowest and highest `calculate_church_faith(power, …)` among the listed Church moves, with `power = calculate_power(state, state.current_player, area)` for `acting_area(state)`, exactly as `describe`'s Church arm reads it.

**The fixture all Church cells share** (`issue_position()`, in `chooser.rs`'s test module): a 2P state (`new_game` + `finalize_setup`, any seed), then by state surgery in the manner of `describe.rs`'s `church_option_previews_faith_matching_the_engine_queue`: the current player's president on the Church, 4 of its assistants on that area (power 5), stock copper 1, silk 5, tea 2, seafood 3, money 2, imported goods 1, the track's faith 4, 5 and 6 spaces held by the other seat's assistants (taken from its warehouse, so its twenty are conserved; a copy of the dummy's faith-7 slot there would fail the engine's invariant I9 the moment a Session applies a move), `turn_phase = AreaAction`, `current_action_power = Some(calculate_power(..))`. Measured on the engine for this plan: `get_legal_moves` there holds 43 moves, 42 of them Church moves.

- [ ] **Step 1: Write the cells** in `chooser.rs`'s test module.
  1. `the_issue_position_lists_42_church_moves`: the count above (the fixture's anti-vacuity).
  2. `a_selection_the_list_holds_previews_in_describe_s_words`: silk, tea and yen selected: `preview.legal`, `preview.words` equals `describe_move` of `church_match`'s move AND the literal `Church: pay silk, tea, 2 yen -> faith 8 (power 5 + 3 kinds)`; `offered_faiths` is `None`.
  3. `a_selection_one_kind_short_is_not_offered`: silk and tea selected (faith 7, a taken space): not legal, `words` `None`, `offered_faiths` is `Some("the list offers faith 8 to 11 here")`. Two kinds, not one: a rule "any two kinds will do" passes a one-kind fixture's refusal and fails this one.
  4. `every_flag_set_matches_exactly_when_it_is_listed`: over all 64 flag arrays, `church_match` is `Some` for exactly 42, and for each listed move `church_match(legal, church_flags(mv).unwrap()) == Some(mv)`.
  5. `offered_is_read_from_the_list_not_from_the_stock`: the fixture with money 1: the yen kind reads `have: 1, offered: false`, the other five `offered: true`. Then with copper 0: copper reads `have: 0, offered: false`. Then with tea, seafood, money and imported goods all 0, where the list holds no Church move: copper reads `have: 1, offered: false`, silk `have: 5, offered: false`, and `offered_faiths` is `None`. The first two cases sit where a stock rule and the list agree; this is where they part, so a rule `have >= amount` fails it.
  6. `the_model_s_six_kinds_carry_their_numbers_costs_and_words`: numbers 1 to 6, goods in `CHURCH_KINDS` order, amounts `[1, 1, 1, 1, CHURCH_YEN_COST, 1]`, words `copper, silk, tea, seafood, 2 yen, imported good`, `have` `[1, 5, 2, 3, 2, 1]`. Then on the fixture with six different stocks (copper 1, silk 5, tea 2, seafood 3, money 4, imported goods 6), `have` reads `[1, 5, 2, 3, 4, 6]`: the fixture's own values repeat (copper and imported goods 1, tea and money 2), so two swapped fields in `stock` would pass the first reading.
  7. `the_church_screen_is_the_plan_s`: `church_text` equals each of the two literal screens in "The words" (the legal one, and silk alone with its three-line verdict), and `Selected: (none)` with nothing selected.
  8. `the_church_yen_kind_costs_what_an_apply_charges` (in `screen_model.rs`'s test module): on a Church position with power 3, an empty track but for the dummy and money exactly `CHURCH_YEN_COST`, the yen-only payment is listed, and `apply_move` of it leaves money at 0 and every good unchanged; with money `CHURCH_YEN_COST - 1` no listed move pays yen.
  9. `describe_words_a_church_payment_from_the_one_table` (in `describe.rs`): `church_kind_words(Good::Yen) == format!("{CHURCH_YEN_COST} yen")`, and `describe_move` of the all-six move still reads `Church: pay copper, silk, tea, seafood, 2 yen, imported good -> faith 11 (power 5 + 6 kinds)` on the fixture.
  10. `a_list_offering_one_faith_says_so`: the fixture with faith 8, 9 and 10 filled as well, so only space 11 is empty and the six-kind payment is the one Church move listed (assert that count): with silk alone selected, `offered_faiths` is `Some("the list offers faith 11 here")`.
- [ ] **Step 2: Run red.** `cargo test -p yokohama-cli --lib chooser` — expected: does not compile, naming `chooser`.
- [ ] **Step 3: Implement.** `describe`'s Church arm builds its parts from `church_kind_words` over `CHURCH_KINDS` and the move's flags; its output must not change by a byte.
- [ ] **Step 4: Run green.** `cargo test -p yokohama-cli` whole: every binary green, no golden moved.
- [ ] **Step 5: The Rust suites, then commit** the four files.

**Mutants:** the yen kind's `amount` written as 1 (cell 6; cell 8 reads the constant and the engine, never the model). `offered` computed as `have > 0` (cell 5, the money-1 case). `church_match` ignoring the imported-good flag (cell 4's count). `offered_faiths` returned when the preview is legal (cell 2). `CHURCH_YEN_COST` set to 3 (cells 8, 9). The preview's words built by `format!` in `chooser.rs` instead of `describe_move`: the reviewer changes one word in `describe`'s Church arm and cell 2's first assertion must still hold while its literal fails. `offered` computed as `have >= amount` (cell 5's third case). Two fields swapped in `stock` (cell 6's second reading). The one-faith sentence printed as `faith 11 to 11` (cell 10). The sentence returned with no Church move listed (cell 5's third case).

---

### Task 2: Chinatown's chooser, as a pure module

**Tier:** impl sonnet · review opus.

**Files:**
- Modify: `crates/yokohama-cli/src/chooser.rs`, `crates/yokohama-cli/src/describe.rs` (`chinatown_exchange_text` made `pub(crate)`)

**Interfaces — consumes:** `Good`, `Amount`, `Preview`, `stock`, `CHURCH_KINDS` (Task 1); `screen_model::ChinatownOffer::{all, index}` and the engine's `chinatown_rate` (existing). **Produces:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChinatownRow {
    pub number: usize,   // what to send to add one: 1 to 9
    pub sells: bool,     // the exchange gains yen
    pub spend: Amount,   // chinatown_rate's, at the printed rate
    pub gain: Amount,
    pub words: String,   // describe's: "1 copper -> 2 yen"
    pub count: u8,       // how many of it are chosen
    pub can_add: bool,   // the list holds the selection with one more of it
    pub offered: bool,   // some listed move uses it
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChinatownModel {
    pub max_exchanges: usize,  // the most exchanges in one listed move
    pub chosen: usize,         // the selection's total
    pub stock: Vec<Amount>,    // the acting seat's six holdings, in CHURCH_KINDS order
    pub rows: Vec<ChinatownRow>,  // always nine, in ChinatownOffer::all() order
    pub preview: Preview,
}

/// A listed Chinatown move's count per exchange, by `ChinatownOffer::index`; `None` for any other move.
pub fn chinatown_counts(mv: &Move) -> Option<[u8; 9]>;
/// The listed move holding exactly `counts`, in whatever order the engine listed it.
pub fn chinatown_match(legal: &[Move], counts: [u8; 9]) -> Option<Move>;
pub fn chinatown_model(state: &GameState, rules: &Rules, legal: &[Move], counts: [u8; 9]) -> ChinatownModel;
pub fn chinatown_text(model: &ChinatownModel) -> String;
```

`stock` and `sells` are this plan's additions to the design's list of fields, as `words` on a Church kind is: the accepted mockup draws a stock row and a Sell and Buy split, and the arm carries both so the client reads neither from elsewhere.

`chinatown_model` walks the list once into a set of count arrays and answers `can_add`, `offered` and the match from that set. The engine's `ChinatownGood` maps to `Good` by a match with no `_` arm (`Money` is `Yen`, `Imported` is `Imports`). The Mining bonus is not shown: `gain` is the printed rate, as the words are (design §2).

**The fixture** (`ordinary_position()`): a 3P state, the current player's president on Chinatown with 2 of its assistants there (power 3), stock copper 1, silk 0, tea 2, seafood 1, money 3, imported goods 0, `turn_phase = AreaAction`, `current_action_power` set. Measured for this plan: 103 listed moves, 102 of them Chinatown moves, the longest of 3 exchanges.

- [ ] **Step 1: Write the cells.**
  1. `the_ordinary_position_lists_102_exchange_sets_of_at_most_three`.
  2. `every_listed_set_matches_itself_whatever_its_order`: for each listed Chinatown move, `chinatown_match(legal, chinatown_counts(mv).unwrap()) == Some(mv)`; and the same move with its exchanges reversed has the same counts; and a list holding every Chinatown move with its exchanges reversed still matches each by its counts and returns the move as listed (the engine lists some sets out of row order, `[YenToSilk, SilkToYen]` among them: assert that one is listed; so the real list already refuses a row-order vector compare, and the reversed list holds every set out of its listed order).
  3. `one_more_can_be_added_exactly_when_the_list_holds_it`: from the empty selection, `can_add` for rows 1 to 9 is `[true, false, true, true, true, true, true, true, false]` (no silk to sell; 3 yen does not buy an imported good). After one copper sold (`counts[0] = 1`), row 9 reads `can_add: true`. This is the cell that says selling first makes a purchase addable.
  4. `a_selection_left_by_taking_the_sale_back_is_not_offered`: `counts` with row 9 alone: `preview.legal` false, `words` `None`.
  5. `offered_is_read_from_the_list`: on the ordinary position all nine rows read `offered: true`: row 9 is listed behind a sale and row 2 behind a silk purchase, while `can_add` from the empty selection is false for both. That pair tells `offered` from `can_add`. On the same stock with no assistant on the tile (power 1; measured: 7 listed sets), rows 2 and 9 read `offered: false` and the other seven `true`.
  6. `the_rows_carry_the_engine_s_rates_and_describe_s_words`: for all nine rows, `spend` and `gain` equal `chinatown_rate`'s amounts and goods, `words` equals `describe`'s, and `words == format!("{} -> {}", …)` rebuilt from `spend` and `gain` in `describe`'s unit words, so the tokens and the words cannot disagree. `sells` is true for rows 1 to 4 only.
  7. `the_stock_is_the_acting_seat_s_six_holdings`: amounts `[1, 0, 2, 1, 3, 0]` in `CHURCH_KINDS` order. Then with six different holdings (copper 1, silk 2, tea 3, seafood 4, money 5, imported goods 6) the amounts read `[1, 2, 3, 4, 5, 6]` and the goods are `CHURCH_KINDS` in order: the position's own holdings repeat (copper and seafood 1, silk and imported goods 0), so two entries out of order would pass the first reading.
  8. `the_chinatown_screen_is_the_plan_s`: on the ordinary position with one copper sale and two silk purchases chosen (measured: listed, in the engine's order copper sale, silk, silk), `chinatown_text` equals the literal screen in "The words"; the empty selection's two lines; and on the power-1 position row 2's line with its mark. On that position the heading is in the singular, `Chinatown — up to 1 exchange (add by number):`, and the empty selection reads `Selected (0 of 1): (none)`.
  9. `a_long_list_costs_what_it_costs`: on a 4P position with power 5 and stock 5, 5, 5, 5, money 20, imported goods 2 (measured for this plan: 2,001 listed Chinatown moves), build the model with one exchange chosen: the list's count is 2,001, the model has nine rows and `chosen` is 1. The elapsed time is printed and not asserted; the report states the reading (design §9).
  10. `the_flags_and_the_heading_are_read_from_the_list_handed_in`: on the ordinary position with every listed move that holds a tea sale left out of the slice handed to the model, row 3 reads `can_add: false` and `offered: false` from the empty selection though the seat holds 2 tea, and the other eight rows' `can_add` read as in cell 3. With the slice cut to its one-exchange moves, `max_exchanges` reads 1 at power 3, and after one copper sale no row reads `can_add: true`. A rule worked from the stock, the rate or the power passes cells 3 and 5 and fails this one. With the slice cut to the sets that repeat an exchange, `max_exchanges` reads 3 while no set there holds more than 2 different exchanges (assert both), so a count of different exchanges fails it.
  11. `the_model_previews_every_listed_set_in_describe_s_words`: on the ordinary position, for every listed Chinatown move, `chinatown_model` over that move's counts has `preview` equal to `Preview { legal: true, words: Some(describe_move(..)) }` of THAT listed move. The list holds sets out of row order (cell 2 asserts one), so a match made in row order, or words built from the selection instead of the listed move, fails here.
- [ ] **Step 2: Run red**, **Step 3: implement**, **Step 4: `cargo test -p yokohama-cli` green**, **Step 5: the Rust suites, then commit.**

**Mutants:** `can_add` computed from the stock and the rate (cell 10's first reading; cells 3 and 5 pass under it). `offered` computed from the stock (cell 10's first reading). `chinatown_match` comparing exchange vectors in order (cell 2's first loop: the list's own out-of-row-order sets refuse it). `offered` set equal to `can_add` (cell 5). `gain` read with the Mining bonus added (cell 6). `max_exchanges` taken from `current_action_power` (cell 10's second reading). The model's `stock` entries out of `CHURCH_KINDS` order (cell 7's second reading). The heading left plural at one exchange (cell 8's power-1 reading). The model's preview matched by a row-order vector compare, or worded from a move built from the selection (cell 11). `max_exchanges` counted as the most different exchanges in one set (cell 10's third reading).

---

### Task 3: The Church folds, in the REPL and on the wire

**Tier:** impl opus · review opus.

**Files:**
- Modify: `crates/yokohama-cli/src/repl.rs`, `crates/yokohama-cli/tests/piped_dogfood.rs`, `docs/cli-guide.md`, `docs/cli-guide.html`
- Modify: `crates/yokohama-web/src/client.rs`, `crates/yokohama-web/src/api.rs`, `crates/yokohama-web/src/lib.rs` (its test helpers), `crates/yokohama-web/tests/play_a_whole_game.rs`, `crates/yokohama-web/tests/wire_fixtures.rs`
- Regenerate: `web/src/wire/*.json`, `web/src/wire-script/*.json`
- Modify: `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `docs/web-lane.md`
- Modify (added at Task 3, ruling R18): `worker/scripts/replay/fixtures/production-door-kept-rows.txt` and `worker/scripts/live-replay.test.mjs` — they pin the walk script's row count and the production door's kept rows' indices, which the chooser's lines at the three folded prompts shift (`walk.json` 2,201 → 2,216 rows; the same 26 rows kept)

This is the task where the list changes shape, so everything that answers an area action by row number moves with it, in the same task, or a suite reads red.

**Interfaces — consumes:** Task 1's `chooser` functions. **Produces:**

```rust
// repl.rs
enum PromptOption {
    // …
    /// The Church's payment family collapsed: more than one `AreaAction::Church`
    /// move is listed. `count` is how many.
    Church { count: usize },
}
pub enum SubPrompt {
    // …
    /// The Church chooser: which of `chooser::CHURCH_KINDS` are switched on.
    Church { selected: [bool; 6] },
}
impl Repl {
    /// The area-action main list: a family with more than one listed move
    /// becomes one entry at its first move's place; every other move stays `Direct`.
    fn group_area_action(legal: Vec<Move>) -> Vec<PromptOption>;
    fn step_church(&mut self, words: &[&str]) -> StepOutput;
    /// The screen an open chooser appended to the last reply, exactly as `out`
    /// appended it; `None` when no chooser is open.
    pub fn chooser_screen(&self) -> Option<String>;
}

// client.rs — serde's tag is `church`; the fields are the model's own.
pub enum PromptState { /* … */ Church(yokohama_cli::chooser::ChurchModel) }
```

```ts
// types.ts
export type ChooserGood = "Copper" | "Silk" | "Tea" | "Seafood" | "Yen" | "Imports";
export interface ChooserAmount { good: ChooserGood; amount: number }
export interface ChooserPreview { legal: boolean; words: string | null }
export interface ChurchKind { number: number; good: ChooserGood; amount: number; words: string; have: number; selected: boolean; offered: boolean }
// PromptState gains: { kind: "church"; kinds: ChurchKind[]; preview: ChooserPreview; offered_faiths: string | null }
// contract.ts: wire.church, wire.churchSelected, wire.churchLegal
```

**How it behaves.**
- `group_options` sends the AreaAction phase with no choice queued to `group_area_action`. The decline stays the last row. A Church with one listed payment, or none, reads as today.
- The entry's number opens `SubPrompt::Church` with nothing selected and prints no line before the screen.
- A number 1 to 6 switches its kind. Switching ON a kind that is not `offered` is refused; switching off is always taken. `confirm` closes the chooser and resets the page and THEN commits `church_match`'s move through `commit`, in `step_place_a`'s order; with no match it refuses and keeps the selection. The order matters: `out` appends the main prompt only when no sub-prompt is open, so only this order makes the reply read as the old row's did. `cancel` closes it. `quit` and `q` behave as in the placement pickers.
- `index_target` answers `None` for a chooser: no card registry is being asked about.
- Three doc comments in `repl.rs` say the area-action list is one row per move (on `PromptOption`, on `option_rows`, on `display_options`). The fold makes them false; correct them in this task.
- `api.rs`: where a chooser is open after the step, the reply's text loses `chooser_screen()`'s suffix by identity, as the main prompt's is lost today; what stands before it (a refusal, a cancel line) is kept. The placement pickers' and the pick's replies are not touched.

**The drivers that answer by number** (found by a search of the tree for this plan; run the suites to find any other, and name it in the report):
- `repl.rs`'s test fixture `drive_to_double_assistant_move_cascade` commits a Church payment by its position in the legal list. It now opens the entry (`display_number_containing(r, "Church — choose what to pay")`), sends `1`, `2`, `5` and `confirm`.
- `crates/yokohama-web/tests/play_a_whole_game.rs`'s `choose` panics on a kind it does not know. Its rule for `church`: `confirm` when `preview.legal`; else the number of a kind drawn at random among those `offered` and not `selected`; `cancel` when there is none.
- `crates/yokohama-web/src/lib.rs` has two test walks that answer the first row (near `:8047` and `:8518`). Give the second the same rule with "the first" for "a random"; change the first only if it reads red.
- `crates/yokohama-cli/tests/piped_dogfood.rs`: on a screen holding `type a number to toggle, 'confirm' to pay`: `confirm` when it holds `-> legal`; else a random shown number, at most six times in one visit; then `cancel`.
- `crates/yokohama-web/tests/wire_fixtures.rs`'s walk, below.

**The corpus walk plays the same game.** The walk draws a random row at every main prompt. With the fold a Church prompt has two rows where it had dozens, so a naive walk would play a different game from its first Church visit on and every later fixture would change. Instead, at a main prompt where the Church family is folded (the prompt has fewer rows than `frame.decision.options`, and one of those options is a Church move), the walk draws its index over `frame.decision.options`, the list the rows were one-per-move over before, and then plays THAT move: for a Church move it opens the entry (the row whose `opens_picker` is true), sends the number of each kind the move's flags hold, and confirms; for the decline it sends the row whose `opens_picker` is false. A Church move reads `{"ExecuteAreaAction":{"action":{"Church":{"copper":true,…}}}}` and the decline `{"ExecuteAreaAction":{"action":"Skip"}}`; the flags are read BY FIELD NAME (copper 1, silk 2, tea 3, seafood 4, yen 5, imported_good 6), since the corpus sorts an object's keys. The draw consumes the generator exactly as before, so the game is the one the committed corpus already holds.
  - Captures, first sighting each: `church` (the chooser as it opens), `church_selected` (after its first kind is switched on) and `church_legal` (the first `church` prompt whose preview is legal, so the corpus holds a legal preview with `describe`'s words: without it the contract's "a legal preview has words" reads nothing at these seeds). All three join the walk's `owed` list.
  - Measured by the plan's review on an export: the walk meets 95 area-action prompts, the rows there equal `frame.decision.options` one for one, three of them fold (events 18, 251 and 387), and all three draws land on a Church move. So all three captures are reached at the seeds as they are, and the walk's decline branch never runs at these seeds: say so in its comment. If a capture is not reached, STOP and report; do not move a seed.
  - The witness is `choice_ChurchSpace.json`: it holds the reply to the event-18 payment, so it is byte-identical exactly when the chooser's `confirm` answers as the old row did.

- [ ] **Step 1: Write the REPL's cells** in `repl.rs`'s test module, on Task 1's `issue_position()` (lift it to a helper both modules reach) unless a cell says otherwise. The fixture is already reachable as `crate::chooser::test_support::issue_position()`. **Corrected at Task 3:** as Task 1 first built it, with copies of the dummy's slot on faith 4 to 6, committing any payment from it answered `LOCKDOWN — INVARIANT VIOLATION after apply` and recorded nothing, because `Session::apply` runs the engine's invariants and their I9 clause allows the 2P dummy on faith 7 alone; the fixture now holds those spaces with the other seat's assistants, and lists the same 43 moves.
  1. `the_church_folds_to_one_entry_and_the_decline`: two displayed entries, the first worded `Church — choose what to pay (42 payment sets)`, the second the decline in `describe`'s words.
  2. `a_church_with_one_payment_stays_a_direct_row`: a position listing exactly one Church move reads two `Direct` rows whose words are `describe`'s; a position listing only the decline reads one row and bare Enter applies it.
  3. `a_number_switches_its_kind_on_and_off`: `2` then `2` again leaves nothing selected; the screen is `church_text` of the model each time.
  4. `a_kind_not_offered_and_a_number_out_of_range_are_refused_and_keep_the_selection`: with money 1, after `2`, the line `5` answers the not-offered refusal with `2 yen` in it; `0` and `7` answer the range refusal; the selection is still silk alone after each, and `6` is taken.
  5. `confirm_commits_the_listed_move_and_the_space_choice_follows`: `2`, `3`, `4`, `5`, `confirm` (four kinds, faith 9, so spaces 8 and 9 both qualify): the record's last event is exactly `church_match`'s `Move`, `r.sub` is `SubPrompt::None`, the queue's head is `PendingChoice::ChurchSpace { total_faith: 9 }`, and the next prompt lists two `ChooseChurchSpace` moves. Three kinds would reach one space, which the engine places without asking, and the cell would say nothing about the queued choice.
  6. `confirm_on_a_selection_the_list_does_not_hold_applies_nothing`: `2`, `3`, `confirm` (faith 7): Option A's refusal line, no new event, the chooser open with silk and tea selected.
  7. `cancel_and_undo_return_to_the_folded_list`: `cancel` prints the cancel line over the main prompt; after a committed payment, `undo` shows the two-entry list and `r.sub` is `None`.
  8. `words_that_are_not_an_answer_are_refused_inside_the_chooser`: `back`, `more`, `list`, `help` and `undo` each answer the catch-all refusal, apply nothing and leave the chooser open.
  9. `the_card_index_opens_over_the_chooser_and_returns_to_it`: `2`, `?`, `cancel`: the chooser is open with silk selected.
  10. `a_mercenary_s_sub_turn_at_the_church_folds_too`: the fixture with the President moved to another area, `is_mercenary_action = true`, `mercenary_target_area = Some(church)`, and `current_action_power` set again from `calculate_power` (measured: it reads 4 and 22 Church moves are listed; left at 5 the engine and `describe` would disagree about the power): the entry reads `Church — choose what to pay (22 payment sets)`.
  11. `every_listed_church_move_commits_through_the_chooser`, the sweep (design §6's first cell). Seeded random self-play at 2, 3 and 4 players, in the manner of `screen_model.rs`'s `the_models_flags_agree_with_the_engines_mercenaries_and_country_bonus`. At every position in the AreaAction phase with no choice queued and more than one listed Church move, for EVERY listed Church move: a fresh `Repl::new(Session::for_test_from_state(..))`, the entry, the number of each kind its flags hold, `confirm`; the record's last event equals that move. At the same positions, for the empty selection and after each toggle of the walk to one sampled move, every kind's `offered` equals "some listed move's flags hold it". Then the issue position in full, all 42 (design §6). Count positions and moves, print both, and set two anti-vacuity floors at about half of what your run reads; size the seeds so the cell stays under about ten seconds in a debug build, and state the readings.
  12. `the_empty_selection_can_be_a_listed_payment`: at power 4 with space 4 empty, the chooser opens on `-> legal — Church: pay nothing -> faith 4 (power 4 + 0 kinds)` and `confirm` commits the move whose six flags are false. On the issue position the empty selection is NOT listed (faith 5, spaces 4 to 7 taken), and `confirm` there is refused.
- [ ] **Step 2: Write the web layer's cells** (`client.rs`'s and `api.rs`'s test modules; use the table helpers those modules already have to reach a Church chooser through `api::input`).
  13. `a_chooser_is_the_acting_seat_s_alone`: with the chooser open, the acting seat's payload has `prompt.kind == "church"`, and the other seat's and a watcher's have `watching`.
  14. `the_church_arm_is_the_repl_s_model_by_value`: the arm's JSON equals `serde_json::to_value(chooser::church_model(..))` with `"kind": "church"` added.
  15. `a_chooser_reply_carries_no_line_of_the_terminal_s_screen`: the reply that opens the chooser has empty `messages`; the reply to a refused kind carries exactly the refusal line; in neither does any line of `chooser_screen()` appear. And a `place_a` reply still carries its `> type an area id to toggle` line.
  16. `every_line_the_client_sends_at_the_church_passes_the_fence`: `1` to `6`, `confirm` and `cancel` pass `grammar::is_gameplay_line`.
- [ ] **Step 3: Run red.** `cargo test -p yokohama-cli --lib repl` — expected: does not compile, naming `SubPrompt::Church`.
- [ ] **Step 4: Implement the REPL**, then `HELP_TEXT`'s `cancel` line. `cargo test -p yokohama-cli`: the `repl.rs` cells green. `guide_sync` now reads red on the §5 block: that is Step 7's.
- [ ] **Step 5: Implement the web layer**: the arm in `prompt_of`, `kind()`, `api.rs`'s suffix; in `wire_fixtures.rs` `every_prompt_variant`, `prompt_ordinal` and the literal count in `the_variant_list_is_every_variant_exactly_once` (9 today, 10 after this task); the drivers' rules; the walk.
- [ ] **Step 6: Regenerate the corpus and read the diff.** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then the plain run green. Then:

```bash
git status --short web/src/wire web/src/wire-script   # expected: three untracked files under wire/, NO modified file there; the walk scripts modified
git diff --stat -- web/src/wire                        # expected: nothing
```

  No committed fixture is expected to move: the walk plays the same game and a chooser's `confirm` answers as the old row did. A modified fixture is a finding, named in the report with its reason. A moved `log.json` (a different game) or a moved `choice_ChurchSpace.json` (a different reply) is a STOP.
- [ ] **Step 7: The guide pair.** In §4 of both formats, a paragraph after the placement pickers' on the area-action choosers: what the entry reads, the Church's screen quoted from "The words", that a kind the list does not offer is marked, what `confirm` and `cancel` do, and that the legal list still decides. §5's command block takes `HELP_TEXT`'s new `cancel` line. `cargo test -p yokohama-cli --test guide_sync` green.
- [ ] **Step 8: The client's contract.** The four types and the arm in `types.ts`. In `contract.ts`: the two imports with their `Widen<ClientState>` and `PromptOf<"church">` assertions, `wire.church`, `wire.churchSelected` and `wire.churchLegal`, all added to `EVERY_STATE` so the corpus-wide key-set and hand-mask cells cover them, and key sets `KEYS.churchKind` and `KEYS.chooserPreview` in the manner of `KEYS.optionRow`. In `contract.test.ts`: every `church` state has six kinds whose `Object.keys` equal `KEYS.churchKind` and whose numbers are 1 to 6; `wire.church` has no kind selected and `wire.churchSelected` exactly one, and neither preview is legal, so each has `words` `null`; `wire.churchLegal` has a legal preview with words.
- [ ] **Step 9: The lane document.** §1's fence sentence gains this cycle as the one that moved the REPL on purpose, naming the design. §2: the prompt kinds where it lists them. The corpus's JSON count is a second commit that names the first, as §2's counts do. §2's suite counts are the cycle's close's, as cycle 1 left them. No suite pins the corpus count or the prompt kinds in that file (`lane_document` reads §13 and §16 only): read the count with `ls web/src/wire/*.json | wc -l` and state it.
- [ ] **Step 10: The suites.** The Rust suites; then the piped-dogfood gate, `cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored`, and state how many Church payments the transcript confirmed through the chooser (a count, not an assertion).
- [ ] **Step 11: Commit** by explicit paths (`web/src/wire` and `web/src/wire-script` are directories this task owns whole).

**Mutants:** the fold applied when one Church move is listed (cell 2). The decline folded into the entry (cell 1). `confirm` committing a `Move` built from the flags instead of the listed one: the reviewer makes `church_match` return a clone with one flag flipped and cell 5 and the sweep must fail. A not-offered kind accepted at the toggle (cell 4). The chooser left open after a commit (cell 5). `prompt_of_seat`'s scoping bypassed for the new arm (cell 13). `chooser_screen` returning `None` so the screen rides `messages` (cell 15). `confirm` committing before it closes the chooser (Step 6: `choice_ChurchSpace.json` moves). The walk drawing over `prompt.rows` at a folded prompt (Step 6's `log.json` check). `church_legal` captured at the first `church` prompt whether or not its preview is legal (the regeneration refuses it first: its payload is `church.json`'s again, the answer to no request of its own; the contract's cell on `wire.churchLegal` is shown to fail on a hand-placed payload).

---

### Task 4: Chinatown folds, in the REPL and on the wire

**Tier:** impl opus · review opus.

**Files:** the same as Task 3's, less `crates/yokohama-cli/tests/piped_dogfood.rs` and `docs/web-lane.md` §1. With them (added at Task 4, ruling R19): `worker/scripts/replay/fixtures/production-door-kept-rows.txt` and `worker/scripts/live-replay.test.mjs` — the six lines the walk posts to `wire4`, a solo table the production seed rule keeps, JOIN the rows a production-door run sends, so the list grows from 26 kept rows and the cell's title and literal move with it.

**Interfaces — consumes:** Task 2's functions; Task 3's `group_area_action`, `chooser_screen` and the walk's folded-prompt rule. **Produces:**

```rust
enum PromptOption { /* … */ Chinatown { count: usize, longest: usize } }
pub enum SubPrompt { /* … */ Chinatown { counts: [u8; 9] } }
impl Repl { fn step_chinatown(&mut self, words: &[&str]) -> StepOutput; }
pub enum PromptState { /* … */ Chinatown(yokohama_cli::chooser::ChinatownModel) }
```

```ts
export interface ChinatownRow { number: number; sells: boolean; spend: ChooserAmount; gain: ChooserAmount; words: string; count: number; can_add: boolean; offered: boolean }
// PromptState gains: { kind: "chinatown"; max_exchanges: number; chosen: number; stock: ChooserAmount[]; rows: ChinatownRow[]; preview: ChooserPreview }
// contract.ts: wire.chinatown, wire.chinatownSelected
```

**How it behaves.** `group_area_action` folds a Chinatown family of more than one listed move the same way. A number 1 to 9 adds one of its exchange when `can_add`, and is refused otherwise. `back N` takes one of exchange N back when one is chosen. `confirm` commits `chinatown_match`'s move, the engine's own listed order. Everything else is Task 3's shape. `chooser_screen` serves both choosers, so `api.rs` needs no further change.

**The drivers.** `choose` and the `lib.rs` walk play two-player games, whose board has no Chinatown, so neither gains an arm that could never run: each refuses a `chinatown` prompt with a panic that says so (ruling R25's rule, applied here when Task 4's implementer found both arms dead). The piped-dogfood driver is not changed in this task: its gate plays two-player games, whose board has no Chinatown, so an arm there could never run. Chinatown's whole-game coverage in the terminal is cell 8's sweep. Through the web layer it is answered at one position by cells 13 to 15, on the corpus's `wire4` route, in Task 7's four-player browser cell and in Task 8's restore; no whole-game gate plays a table with a Chinatown.

**The corpus.** Chinatown is not on the two-player board, so its fixtures are routed on the walk's four-player table `wire4`, after `options_4p` and `areas_4p` are captured: `1` (end the additional phase); `b` and the id of the tile whose `area_type` is `Chinatown`, read from the payload; the Movement row whose words name that tile; then the row whose `opens_picker` is true, captured as `chinatown`; then `1`, captured as `chinatown_selected`; then `confirm`. The walk's `send_as` and `as_actor` are fixed to the table `wire`, so these lines are posted to `/api/games/wire4/input` with `call_as(Some(WALKER), …)`. Seat 0 opens with one of each good and 3 yen, so at power 3 the entry reads `Chinatown — choose exchanges (up to 3; 127 combinations)` (measured for this plan); assert those words in the walk. The walk's folded-prompt rule stays the Church's alone: its game is two-player and that board has no Chinatown, so the walk refuses a folded Chinatown prompt there with a panic that says so, rather than carry a branch no seed can run.

- [ ] **Step 1: Write the REPL's cells**, on Task 2's `ordinary_position()`.
  1. `chinatown_folds_to_one_entry_and_the_decline`: `Chinatown — choose exchanges (up to 3; 102 combinations)` and the decline.
  2. `a_number_adds_one_and_back_takes_one_back`: `1`, `6`, `6` reads counts 1, 0, 0, 0, 0, 2; `back 6` reads one silk purchase; the screen is `chinatown_text` of the model each time.
  3. `what_the_list_does_not_hold_is_refused_and_keeps_the_selection`: from empty, `9` answers the one-more refusal with `4 yen -> 1 imported good` in it; `2` likewise; `10`, `0` and `back 10` answer the range refusal; `back 3` with no tea sale chosen answers the take-back refusal; the selection is unchanged after each.
  4. `taking_back_the_sale_that_paid_for_a_purchase_cannot_be_confirmed`: `1`, `9`, `back 1`: the preview reads NOT currently offered, `confirm` answers Option A's refusal line, no event is recorded, and the chooser is open with the imported-good purchase alone.
  5. `confirm_commits_the_engine_s_listed_order`: `6`, `1`, `6` (a purchase sent before the sale that pays for its second) then `confirm`: the record's last event is `chinatown_match`'s `Move`, by value.
  6. `confirm_with_nothing_chosen_is_refused`; `bare_back_and_other_words_are_refused_inside_the_chooser` (`back`, `more`, `list`, `help`, `undo`); `cancel_returns_to_the_folded_list`.
  7. `back_and_a_number_at_the_main_prompt_applies_nothing`: at the folded main list, `back 3` answers `Unrecognized input` and records no event.
  8. `every_listed_chinatown_move_commits_through_the_chooser`, the sweep, at 3 and 4 players (and 2 with a precondition that no Chinatown is met). At each folded position: every listed move when the family holds at most 64, else 64 spread evenly with the first and the last; each is named by sending its exchanges' numbers in its own listed order, then `confirm`; the record's last event equals it. At each step of each walk, every row's `can_add` equals "the list holds the selection with one more of it". Then the ordinary position in full, all 102, and Task 2's `rich_position()` (power 5, 2,001 listed sets) by the same spread: **corrected at Task 4** — self-play at 3 and 4 players over seeds 0..4 met 51 folded positions and no five-exchange set at power 5 (measured), so the rich position is where the sweep meets one (41 of its 64 spread moves), and the cell asserts it met at least one. Floors and sizing as Task 3's cell 11.
- [ ] **Step 2: Write the web layer's cells**: Task 3's cells 13 to 15 for `chinatown`, and `every_line_the_client_sends_at_chinatown_passes_the_fence` (`1` to `9`, `back 1` to `back 9`, `confirm`, `cancel`).
- [ ] **Step 3: Run red**, **Step 4: implement the REPL**, **Step 5: implement the web layer and the walk** (the variant count's literal reads 11 after this task).
- [ ] **Step 6: Regenerate and read the diff**, as Task 3's Step 6: two new files (`chinatown.json`, `chinatown_selected.json`), the walk scripts, and no committed fixture modified. A lobby or table fixture that lists `wire4` and moved is named with its reason.
- [ ] **Step 7: The guide pair**: the paragraph gains Chinatown's screen, `back <number>`, and the sentence that selling first is what makes a purchase addable. **Step 8: the client's contract** (the two fixtures in `EVERY_STATE`; `KEYS.chinatownRow`; nine rows numbered 1 to 9; `wire.chinatown` has `chosen` 0 and `wire.chinatownSelected` has 1; `stock` has six entries; rows 1 to 4 are the ones with `sells`). **Step 9: the lane document's counts and prompt kinds.** **Step 10: the suites. Step 11: commit.**

**Mutants:** an exchange added without the lookup (cell 3). `back N` dropping below zero or taking a different row (cells 2, 3). `confirm` committing the exchanges in the order typed (cell 5). The match made on the order typed (the sweep still passes, since it types the listed order: cell 5 is the catcher). `back 3` at the main prompt treated as `undo` (cell 7). The sweep sampling only the first 64 rows (the reviewer plants a matcher that fails for moves of five exchanges; the spread must catch it on a power-5 position, so assert the sweep met one).

---

### Task 5: The token and the Church's panel

**Tier:** impl sonnet · review opus.

**Files:**
- Create: `web/src/Token.tsx`, `web/src/Token.test.tsx`
- Modify: `web/src/Decision.tsx`, `web/src/App.tsx` (hands `art` to `Decision`), `web/src/styles.css`
- Test: `web/src/Decision.test.tsx`

**Interfaces — consumes:** `wire.church`, `wire.churchSelected`, `wire.churchLegal`, the `church` arm's types (Task 3); `tokenArt` (`web/src/art.ts`). **Produces** (Task 7 measures these):

```tsx
/** `amount` copies of one good's token, read as ONE node: role="img", aria-label
 *  "2 yen". Art from `tokenArt` where the manifest has it; else a plain mark. */
export function Tokens(props: { good: ChooserGood; amount: number; art: ArtManifest | null; small?: boolean; decorative?: boolean }): JSX.Element
// Decision gains the prop `art?: ArtManifest | null`, default `null`, so no existing caller changes.
```

- Art keys: `Copper` `good_copper`, `Silk` `good_silk`, `Tea` `good_tea`, `Seafood` `good_seafood`, `Imports` `good_imports`, `Yen` `yen_1`. The plain mark is a ring in the good's colour (`--copper` and its five siblings in `styles.css`) holding `Cu`, `Si`, `Te`, `Sf`, `Im` or `¥`.
- The label's noun: `copper`, `silk`, `tea`, `seafood`, `yen`, `imported good`. Each inner picture or mark is `aria-hidden`. The label is `tokenLabel(good, amount)`, exported for Task 6's stock row: the amount and the noun, `imported goods` in the plural when the amount is not 1.
- With `decorative`, the node is `aria-hidden` and carries no role and no label: for a place whose reading is something else's (Chinatown's stock entry, whose own label says how many are held, draws one decorative token).
- The Church's DOM: `.decision` › `.chooser.church` › `.kinds` › six `button.kind[data-good]` with `aria-pressed`, each holding `Tokens` and a `.lbl` of the kind's `words` over a `.have` reading `you have N`; then `.verdict[role="status"]`. Layout as the accepted mockup: a three-column grid of buttons at least 48 px tall.
- A kind button's accessible name is exactly `{words}, you have {N}`, set as its `aria-label`, so the token's own label is not read a second time inside it.
- A kind is disabled when `busy` or not `offered`. A click sends the kind's `number`.
- **A control disabled by what the payload says carries `data-unavailable`** (a kind not offered; Confirm while the preview is not legal), whether or not a request is in flight. A control disabled only because a request is in flight does not. Cancel never carries it. Task 7's walks tell a busy chooser from an idle one by this.
- The verdict: `legal — {preview.words}`, or `not offered — adjust the selection` with `offered_faiths` on its own line under it when present.
- Confirm and Cancel serve the new kind. Confirm is disabled unless `preview.legal`; the server still decides.
- The asking line: `The Church — what do you pay?`

- [ ] **Step 1: Write the cells.** `Token.test.tsx`:
  1. With the corpus's manifest (`art_manifest.json`, which holds the key: assert it), two yen draw two `img` elements with empty `alt` inside one `role="img"` node named `2 yen`. The group's own text is empty: no plain mark is drawn beside a picture.
  2. With `art` null, or a manifest without the key, the node holds two plain marks reading `¥` and the same name; no `img`. Each mark is `aria-hidden`.
  3. One imported good is named `1 imported good`. `tokenLabel` reads `2 imported goods`, `0 imported goods`, `4 yen` and `2 copper`.
  3a. For each of the six goods, with the corpus's manifest, the picture's address is `tokenArt(manifest, key)` for that good's own key (`good_copper`, `good_silk`, `good_tea`, `good_seafood`, `good_imports`, `yen_1`), and the six addresses differ (assert it); with `art` null its mark reads `Cu`, `Si`, `Te`, `Sf`, `Im`, `¥`.
  3b. With `decorative`, the node is `aria-hidden`, has no `role` and no `aria-label`, and still draws its copies.

  `Decision.test.tsx`, `describe("the Church's chooser")`:
  4. Six kind buttons in the payload's order, each with the accessible name exactly `{words}, you have {N}` from the payload, and a `.have` reading `you have N`. On a clone of `wire.church` with one kind's `words` changed, that button's name and its `.lbl` read the changed words: the words are the payload's, not rebuilt from the good and the amount.
  5. A click on a kind sends its `number` and nothing else; on `wire.churchSelected` the selected kind has `aria-pressed="true"` and the rest `false`.
  6. On `wire.church` the imported-good kind is not offered (assert it): its button is disabled with `data-unavailable`, and a click sends nothing. With `busy`, an offered kind is disabled and carries no `data-unavailable`, and the not-offered kind still carries it. On a clone of `wire.church` with silk's `offered` set false, silk's button is disabled with `data-unavailable` though the seat holds one: the fixture's own not-offered kind is also the one the seat has none of, so a rule worked from `have` passes the first reading and fails this one.
  7. Confirm is disabled with `data-unavailable` on `wire.churchSelected`, whose preview is not legal though a kind is selected (assert both), and enabled on `wire.churchLegal`, whose preview is legal (assert it); pressing it there sends `confirm`; Cancel sends `cancel` and never carries `data-unavailable`. With `busy` on `wire.churchLegal`, Confirm is disabled and carries no `data-unavailable`. On a clone of `wire.churchLegal` with its preview set to `{ legal: false, words: null }`, Confirm is disabled with `data-unavailable` though two kinds are selected and `offered_faiths` is null: the two fixtures differ in more than legality, so Confirm worked out from the selection or from the faith line passes the first readings and fails this one. Cancel carries no `data-unavailable` on `wire.churchSelected` either, where Confirm does.
  8. On `wire.churchLegal` the verdict reads `legal — ` and the payload's words; on `wire.churchSelected` it reads `not offered — adjust the selection` and, on its own line, the payload's `offered_faiths` (`the list offers faith 4 to 7 here`: assert the fixture holds it). On a clone of `wire.churchSelected` with `offered_faiths` null, the second line is absent. The verdict carries the class `ok` when legal and `bad` when not. On `wire.church`, with nothing selected, the verdict is `bad` and carries the faith line: the Church's verdict is never the empty one.
  9. A watcher's render (`spectating`) draws no kind button and no Confirm.
  10. No `button.option` is drawn for a `church` prompt, and the asking line is the one above.
  11. With `art` set to the corpus's manifest, a kind button holds a picture, so the manifest reaches the token through `Decision`.
- [ ] **Step 2: Run red.** `cd web && npx vitest run src/Token.test.tsx src/Decision.test.tsx`.
- [ ] **Step 3: Implement.** **Step 4: run green**, then `npm run typecheck && npm test` whole. **Step 5: commit.**

**Mutants:** the click sending the kind's index or its good's name (cell 5). A not-offered kind left enabled (cell 6). Confirm enabled whenever anything is selected (cell 7: `wire.churchSelected` has a kind selected and is not legal). The token's inner pictures exposed with their own `alt` (cell 1's name). Confirm drawn for a watcher (cell 9). A kind button named by its content, so the name repeats the token's label (cell 4's exact name). `imported good` left singular at 2 (`Token.test.tsx` cell 3). A kind disabled from `have` (cell 6's clone). Confirm worked out from the number selected or from the faith line (cell 7's clone). Cancel mirroring Confirm's `data-unavailable` (cell 7). One good's art key pointing at another's (`Token.test.tsx` cell 3a). A mark drawn beside a picture (`Token.test.tsx` cell 1). `Decision` handing the token no art (cell 11). A kind's words rebuilt in the client (cell 4's clone). The verdict's class fixed at `ok` (cell 8). The Church's verdict drawn empty when nothing is selected (cell 8 on `wire.church`).

---

### Task 6: Chinatown's panel

**Tier:** impl sonnet · review opus.

**Files:** Modify `web/src/Decision.tsx`, `web/src/styles.css`; test `web/src/Decision.test.tsx`.

**Interfaces — consumes:** `Tokens` (Task 5); `wire.chinatown`, `wire.chinatownSelected` (Task 4). **Produces:** `.decision` › `.chooser.chinatown` › `.stock` (six entries: a small token and the amount as text); a `.side-h` reading `Sell` over the rows with `sells`, one reading `Buy` over the rest; each row `.ex[data-number]` (class `on` when its `count` is above zero) holding `.what` (the spend's tokens, an arrow, the gain's tokens, `.words`) and `.step` (`button.minus`, `.n` with the count, `button.plus`); then `.verdict[role="status"]`.

- Plus is disabled when `busy` or not `can_add`, and sends the row's `number`. Minus is disabled when `busy` or the count is zero, and sends `back {number}`. A plus that cannot add and a minus at zero carry `data-unavailable`, by Task 5's rule. Their names are `one more: {words}` and `one fewer: {words}`.
- The verdict is empty when `chosen` is 0; otherwise Task 5's two forms (with no faith line).
- The asking line: `Chinatown — up to {max_exchanges} exchanges · {chosen} of {max_exchanges} chosen`. With `max_exchanges` 1 it reads `exchange`, in the singular, as the terminal's heading does.
- The rows are the deck's scrolling box while the chooser is open, as `.options.scrolls` is for a list; Confirm and Cancel stay outside it.

- [ ] **Step 1: Write the cells**, `describe("Chinatown's chooser")`:
  1. Nine rows; the four with `sells` under `Sell` and the five others under `Buy`, in the payload's order; each row's words the payload's. Each row's pictures and arrow are inside one `aria-hidden` wrapper, so the row's words are its one reading. On a clone of `wire.chinatown` with its rows reversed, the four rows with `sells` are still under `Sell`: the split reads the flag, not the position. On a clone with row 1's `words` changed and nothing else, the row's `.words` and its plus and minus names read the changed words and its gain token still reads `2 yen`; on a second clone with row 1's `gain` amount set to 3 and its words untouched, its gain token is labelled `3 yen` and draws three copies. Two clones, one fact each: one clone changing both lets an amount parsed from the words pass. On a clone with one row's `sells` flipped, that row sits under the other heading.
  2. Plus sends the row's `number`; minus sends `back ` and the number; on `wire.chinatown` every minus is disabled; on `wire.chinatownSelected` exactly the chosen row's minus is enabled and its `.n` reads its count. On the clone with the rows reversed, row 8's plus sends `8`: the number, not the place. On a clone of `wire.chinatownSelected` with row 6's `count` set to 2, row 6's minus sends `back 6`: the number, not the count.
  3. A row whose `can_add` is false has its plus disabled with `data-unavailable`, and a click sends nothing. `wire.chinatown` has one, the imported-good purchase, which is `offered`; `wire.chinatownSelected` has the copper sale already chosen. Assert both preconditions. On `wire.chinatownSelected`, row 1's plus is disabled with `data-unavailable` though the stock still shows 1 copper, and row 9's plus is enabled though the stock shows 3 yen against a spend of 4: a rule worked from the stock and the spend fails both. On a clone of `wire.chinatown` with `chosen` set equal to `max_exchanges` and nothing else, row 1's plus is still enabled: plus reads `can_add` alone.
  4. The stock row reads the payload's six amounts in its order. Each entry is one `role="img"` node named by `tokenLabel` (`3 yen`, `0 imported goods` on the fixtures) holding one small token and the amount. On a clone with the six stock amounts set to 1, 2, 3, 4, 5 and 6, the entries read those in the payload's order: four of the fixture's own amounts are 1. The fixture's six labels are asserted as literals (`1 copper`, `1 silk`, `1 tea`, `1 seafood`, `3 yen`, `0 imported goods`), and the token inside an entry is decorative: it carries no label of its own.
  5. Confirm is disabled on `wire.chinatown` and enabled on `wire.chinatownSelected`; the verdict is empty on the first and reads `legal — ` with the payload's words on the second. On a clone of `wire.chinatownSelected` with its preview set to `{ legal: false, words: null }`, Confirm is disabled with `data-unavailable` though `chosen` is 1, and the verdict reads `not offered — adjust the selection`: Confirm worked out from `chosen` passes the two fixtures and fails this one. Cancel carries no `data-unavailable` there. On a clone of `wire.chinatown` with `chosen` set to 1 and nothing else, the verdict reads `not offered — adjust the selection`, not the empty one: the verdict reads `chosen`, not the rows' counts.
  6. The asking line on both fixtures, with their own numbers. On a clone of `wire.chinatown` with `max_exchanges` 1 it reads `Chinatown — up to 1 exchange · 0 of 1 chosen`. On a clone of `wire.chinatownSelected` with `chosen` set to 2 and nothing else, it reads `Chinatown — up to 3 exchanges · 2 of 3 chosen`: `chosen` is the payload's, not a sum.
  7. A watcher's render draws no stepper and no Confirm.
- [ ] **Step 2: run red. Step 3: implement. Step 4: green, then `npm run typecheck && npm test`. Step 5: commit.**

**Mutants:** minus sending the bare number (cell 2). Plus enabled from `offered` instead of `can_add` (cell 3: the fixture's imported-good row is offered and cannot be added). Rows split at index 4 instead of by `sells` (the reviewer reverses a clone's rows; cell 1 must still group by the flag). Confirm enabled at `chosen` 0 (cell 5).
The asking line left plural at one exchange (cell 6's clone). A row's pictures read aloud beside its words (cell 1's accessible reading).
Plus worked out from the stock and the spend (cell 3 on `wire.chinatownSelected`). Confirm live whenever `chosen` is above zero (cell 5's clone). The stock entries out of the payload's order (cell 4's clone). A row's words or amounts rebuilt in the client (cell 1's clones). Cancel mirroring Confirm's `data-unavailable` (cell 5's clone). A row's gain amount parsed from its words (cell 1's second clone). `chosen` summed from the rows' counts (cells 5 and 6's clones). The Sell and Buy split made by the gain's good (cell 1's flipped clone). A plus or a minus sending its place (cell 2 on the reversed clone). The minus sending `back` and the count (cell 2's clone). Plus also gated on `chosen` below the most (cell 3's clone). The stock entry's token labelled (cell 4).

---

### Task 7: The browser's walks and the layout

**Tier:** impl opus · review opus.

**Files:** Modify `web/e2e/harness.ts`, `web/e2e/the_table.spec.ts`.

**Interfaces — consumes:** the DOM of Tasks 5 and 6.

**Two things stop a walk at a chooser.**
- **The settle wait.** `readScreen` waits until no button in the decision zone is disabled (`harness.ts`, the `waitForFunction` at its head), which today means "no request is in flight", since every `disabled` in `Decision.tsx` was `busy`. A chooser always has some control disabled by what the payload says, so that wait would time out after 15 s at the first chooser every walk meets. The wait ignores a button carrying `data-unavailable` (Tasks 5 and 6). Cancel never carries it, so a busy chooser still reads busy.
- **The policy.** `act` has no arm for a chooser. `readScreen` gains `chooser` (a `.chooser` is on screen) and `adds` (the count of enabled, unpressed `.chooser .kind` buttons plus enabled `.chooser .plus` buttons). `act`, before its generic Confirm arm: with a chooser on screen, press Confirm when it is enabled and either `adds` is 0 or a coin says so; else press one of the adds; else Cancel. Adds only ever grow a selection, so a visit ends. `Affordance` gains `"chooser"`.

- [ ] **Step 1: Write the cells** in `the_table.spec.ts`. Reach the positions through the screen as that spec's helpers do: on a two-player game, end the additional phase, Option B on the Church's tile, the Movement row naming the Church, then the entry; on a four-player game the same with Chinatown.
  1. **The Church's boxes**, at 390 and 1440 px wide: each of the six `.kind` boxes lies inside `.chooser` on all four edges (0.5 px of rounding, stated in the test); no two overlap; each is at least 44 px tall; no `.lbl` is clipped (`scrollWidth <= clientWidth + 1`); Confirm and Cancel lie inside the viewport.
  2. **Chinatown's boxes**, at 390 and 1440 px: each `.step` lies inside its `.ex` row; plus and minus are at least 28 px square; each row's first token lies inside the row; and with the rows scrolled to their top, Confirm lies inside the viewport and `elementFromPoint` at its centre is the button.
  3. **A Church payment through the screen**: press kinds until the verdict reads `legal — `, keep its words, Confirm; the turn's account then holds those words.
  4. **A Chinatown exchange through the screen**, on the four-player game: plus on the first row with an enabled plus; the verdict is legal; minus on it and Confirm is disabled; plus again; Confirm; the account holds `Chinatown: ` and that row's words.
  5. **The policy**, a browser cell (`act` takes a page): at the opened Chinatown chooser, where Confirm is disabled, `readScreen` returns within its wait with `chooser` true and `adds` above zero, and `act` presses an add and returns `"chooser"`; at the Church chooser of cell 3 with every offered kind pressed, `adds` reads 0 and `act` presses Confirm.
- [ ] **Step 2: Implement the settle wait and the policy**, then run the cells. Cells 1 to 4 measure the product as built: a failing measurement there is a finding, so STOP with the numbers and do not edit `web/src/` or loosen the assertion. A cell that cannot reach its chooser is the harness's and is yours to fix.
- [ ] **Step 3: Run the five cells green.**
- [ ] **Step 4: The whole browser suite**, `cd web && npm run e2e`: every walk still reaches its end. State the count and the wall.
- [ ] **Step 5: Commit.**

**Mutants:** the settle wait left as it was (every cell times out at its chooser). A `.kind` grid of six columns at 390 px (cell 1's overlap or label clip). Confirm placed inside the scrolling box (cell 2 at 390 px). The policy pressing a PRESSED kind (the visit can then loop: the reviewer plants it and the whole-game walk's own move bound must fail). The policy removed (a whole-game walk reports a dead end).

---

### Task 8: A chooser rebuilt on a new Worker instance

**Tier:** impl sonnet · review opus.

**Files:** Modify `crates/yokohama-worker/src/picker.rs` (its test module only).

**Interfaces — consumes:** the two choosers through `api::input` (Tasks 3, 4). The mechanism itself must not need a change: `is_open` and `close` test `SubPrompt::None` only. If a cell cannot pass without one, STOP and report.

- [ ] **Step 1: Write the cells**, in the manner of `restore_picker_rebuilds_a_place_a_picker_so_confirm_commits_the_intended_move`.
  1. `restore_picker_rebuilds_a_church_chooser_with_its_selection`: a two-player table driven to the Church chooser (`1`; `b` and the Church's id; the Movement row naming the Church; the entry's number); two kinds switched on; the list's JSON taken from the reply; a fresh instance at the same record restored from it; the acting seat's payload reads `church` with the same two kinds selected; `confirm` commits the move those kinds name.
  2. `restore_picker_rebuilds_a_chinatown_chooser_with_its_counts`: the same on a four-player table whose seats one caller holds, at its opening: in the chooser `1`, `6`, `back 6`; after the restore the counts read one copper sale and nothing else, and `confirm` commits it.
  3. `a_refused_toggle_is_refed_and_changes_nothing`: at the Church on turn 1 the seat holds no imported good, so `6` is refused; the refusal is a 200 that leaves the picker open, so the line sits in the list, and the restored selection equals the one before the restore.
- [ ] **Step 2: Run.** These cells are expected green on the code as Tasks 3 and 4 left it; that is the finding (design §3: "the mechanism covers it unchanged"). **Step 3:** `cargo test -p yokohama-worker`, then `cd worker && npm run build:module:debug && npm test`. **Step 4: commit.**

**Mutants** (the reviewer's, on an export, since the task adds no product code): `chooser_screen`'s sub-prompts treated as closed by `is_open` (cells 1, 2). The list's first line dropped at restore (the chooser never opens: cells 1, 2).

---

### Task 9: The browser's words

**Tier:** impl sonnet · review opus.

**Files:** Modify `web/src/Help.tsx`, `web/src/Help.test.tsx`, `docs/web-guide.md`.

- [ ] **Step 1: The sentences.** In `Help.tsx`, after the sentence on the placement picker, and in `docs/web-guide.md` after its counterpart, the same two sentences:
  - S1: `At the Church and at Chinatown the list is one entry that opens a chooser: at the Church you switch on what you pay, at Chinatown you add exchanges with plus and take them back with minus.`
  - S2: `A control is live only where the engine's list holds that choice, so selling first is what lights a purchase you could not yet afford, and Confirm is live once the list holds exactly what you have chosen.`
- [ ] **Step 2: The cell**, in `Help.test.tsx`: needles only these sentences carry (`opens a chooser`, `take them back with minus`, `selling first is what lights a purchase`, `holds exactly what you have chosen`), and each cut sentence fails it.
- [ ] **Step 3: Re-read the neighbours.** Any sentence in either file that the choosers make false is corrected, and named in the report. The pictures are the wrap's and are not re-cut.
- [ ] **Step 4:** `cd web && npm run typecheck && npm test && npm run test:wasm` (the last holds the documents' link check). **Step 5: commit.**

**Mutants:** S2 cut from `Help.tsx` (the cell). The two files' sentences differing by a word (the reviewer diffs them).

---

## After the tasks — the orchestrator's, in order

1. **The whole-branch review**, on an export: a Workflow of about three agents led by Fable (the whole diff against the design; the mutants re-run at the head, the only Playwright runner; the wire, the mask and every host). One fix wave and a scoped re-review.
2. **The gates at the branch's head.** Every per-edit suite re-counted. The fifteen release gates as ONE uninterrupted sequence with the release binaries pre-built, the wall machine-tagged. No fenced crate moved, so fourteen are expected to read as at cycle 1's head; the piped-dogfood gate is the one this plan changes.
3. **The private test Worker** carries the head for the owner's reading, by `worker/RUNBOOK.md` §26, on the owner's word. A deploy of this change drops a picker left open there.
4. **The board** (`docs/handover.md`), and what this build taught recorded for plan 2b, which is written against this branch's reviewed head.

Not this plan's: the Port, the Laboratory, the terminal's flag counts (plan 2b); `CLAUDE.md`'s grammars line and the cycle's close (after 2b); the help's pictures, origin, the release and the decision-log rows (the wrap).

## The plan's review (2026-11-21)

An independent reviewer measured this plan against the tree on an export before any task ran: no Critical finding, three Important, eleven Minor, and every figure quoted here reproduced. All are settled in the text above.

- **The corpus walk does play the same game** (measured, Task 3): 95 area-action prompts, the rows equal to the frame's options one for one, three folded prompts, all three draws on a Church move.
- **The browser walks would have timed out at their first chooser**: `readScreen` waits for no disabled button in the decision zone, and a chooser always has one. Tasks 5 and 6 now mark a control the payload disables, and Task 7 changes the wait.
- **`confirm`'s order was left open**: only "close, then commit" makes the reply read as the old row's did. Task 3 now fixes the order, and its Step 6 holds every committed fixture unmoved, `choice_ChurchSpace.json` as the witness.
- **Task 7's policy cell cited a precedent that does not exist** and is now a browser cell.
- From the Minors: the mercenary cell recomputes its power; the empty Church selection can be a listed payment and has its cell; the reported position's 42 sets go through the REPL's lines; the walk reads a move's flags by field name; the variant count's literal, `EVERY_STATE` and three stale comments in `repl.rs` are named; the piped-dogfood driver takes no Chinatown arm, since its games have no Chinatown; `wire4`'s lines are posted to that table's own route.
