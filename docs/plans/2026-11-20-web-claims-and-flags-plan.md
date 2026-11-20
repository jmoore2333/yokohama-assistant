# The claim on the card and the flag tally — implementation plan (2026-11-20)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A claimed achievement shows the claimants' cubes on the card and lights in the walk (#75), and every seat shows its per-country flag counts, the viewer's own row with the flags still in hand (#76).

**Architecture:** The flag counts are made in `screen_model` (the CLI crate) from the registries and held to the engine's public outputs by a drift pin; they ride the wire as two new fields and the browser draws them. The claim is client only: one component draws a band on the card, and the frame differ names the card whose claims grew. No fenced crate's bytes move.

**Tech stack:** Rust 1.97.1 (`yokohama-cli`, `yokohama-web`), TypeScript, React, vitest, Playwright 1.62.

**Spec:** `docs/plans/2026-11-20-web-claims-and-flags-design.md` (accepted 2026-11-20). Read it before any task; where this plan and the design disagree, stop and report.

**Branch:** `web-evaluator-feedback`, on the lab. Tasks land as new commits there; nothing reaches origin before the whole-branch review is clean.

## Global constraints

- **The lane's fence** (`docs/web-lane.md` §1): `yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and `gen-data` byte-identical; `SEMANTICS_REV` 7; no bot behaviour moved. A task that finds it needs an edit under any of the five STOPS and reports. Check at every task's end: `git diff --stat e600f5f4 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` prints nothing.
- **The terminal's rendering does not move**: `cargo test -p yokohama-cli` keeps its capture goldens and `guide_sync` green with no golden edited. `docs/cli-guide.md` and `.html` are not touched.
- **TDD**: the failing cell first, run red, then the code. Suites green and `cargo clippy --workspace --all-targets -- -D warnings` clean after every task. No `cargo fmt`.
- **A payload in a client cell comes from `web/src/wire/`**, never from a builder. A JSON round-trip clone with one field changed is the existing practice for frame pairs (`web/src/diffFrames.test.ts`) and for an absent-field cell.
- **No agent opens an image**: no `.png`, `.webp` or `.jpg` anywhere, the card art (`crates/yokohama-web/assets/art/`), the help pictures (`web/public/help/`, `web/help-shots/`), the README's (`worker/docs/img/`) and Playwright's output (`web/test-results/`) included. A layout claim is a measured box; a picture is the owner's reading.
- **Every invisible non-ASCII code point in code is a `\u` escape.** The dash in the claim's words is U+2014, visible, and may be raw.
- **Git**: stage with explicit paths, never `git add -A`; new commits only, never an amend; implementers do not push. Commit messages end with `Co-Authored-By: Claude Mythos 5.2 <noreply@anthropic.com>`.
- **Dispatch rules** (every implementer and reviewer): never `rm -rf`, `rm -r` or a bulk `rm -f`; long runs in the foreground or polled with short reads, never a Monitor or a wait on an event; the full report goes to `.superpowers/sdd/2026-11-20-web-claims-and-flags-plan/<role>-task<N>.md` and the reply is the status line, the counts and that path; a reviewer works on an export (`git archive`) with its own `target/`, never in the checkout.
- **Probe before reporting.** A code block here is the single source: an implementer who finds it wrong corrects the plan's block in the same commit, transcribes it, and reports DONE_WITH_CONCERNS naming the correction. Each task lists its mutants: the implementer runs them as a probe, the task reviewer runs them on an export, the whole-branch review runs them again. The overlap is accepted.
- **Two stale-module traps.** `npm run test:wasm` (in `web/`) and the Worker's pool (`npm test` in `worker/`) replay the corpus through a COMPILED module: after any Rust change run `cd web && npm run build:wasm` before the first and `cd worker && npm run build:module:debug` before the second, or a stale module reads as a corpus mismatch.
- **Counts are readings.** Re-count every suite at your own HEAD and state the counts in the report; a count that differs from the board's (`docs/handover.md`, Gates at HEAD) is a finding to explain, not a number to fix.

## Review focus

Five conditions the design implies and no cell of its §6 exercises; each has its cell in the task named.

1. **An order id the registry does not know** contributes no flag and does not panic (Task 1, cell 6).
2. **A caller holding several seats, or waiting on another seat**: `+n` is drawn on the row of the seat whose hand the frame carries, never on the acting seat's row because it is acting (Task 2, cell 3).
3. **A screen reader on the tally**: the row reads as words, counts and in-hand flags included (Task 2, cell 6).
4. **A claimant the colours array does not name** reads `seat N`, as the retired chips did, and still gets its cube (Task 4, cell 5).
5. **A phone's width**: the five flag cells with `+n` do not widen a seat's row at 390 px (Task 6, cell 2).

## The files

| File | Responsibility in this plan | Tasks |
|---|---|---|
| `crates/yokohama-cli/src/screen_model.rs` | `FlagCounts`, the two counting functions, the two fields, their cells and the drift pin | 1 |
| `crates/yokohama-web/tests/wire_fixtures.rs` | the regenerated corpus; the in-process `goals_claimed_4p` | 1, 3 |
| `web/src/wire/*.json` | the corpus: two keys added (Task 1), one file added (Task 3) | 1, 3 |
| `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts` | the wire's types and key sets | 1, 3 |
| `web/src/local/corpus.node.test.ts`, `worker/test/corpus.test.ts` | the two exact-set lists of files no script names | 3 |
| `web/src/Dashboard.tsx` | the tally on a seat's row; `changedGoals` to the Achievements panel | 2, 5 |
| `web/src/Cards.tsx` | the claim band; the card's lit class | 4, 5 |
| `web/src/diffFrames.ts`, `web/src/walk.ts` | `Changes.goals` and its union | 5 |
| `web/src/App.tsx`, `web/src/TableSections.tsx`, `web/src/Strips.tsx` | the set handed down to the Goals strip | 5 |
| `web/src/styles.css` | the tally's and the band's rules | 2, 4 |
| `web/e2e/the_table.spec.ts` | the two layout cells | 6 |
| `web/src/Help.tsx`, `docs/web-guide.md`, `docs/web-lane.md` | the words | 3, 7 |
| `web/e2e/shots.spec.ts`, `web/e2e/readme_shots.spec.ts`, `web/public/help/*.webp` | the pictures | 8, 9 |

---

### Task 1: The counts, from the model to the wire contract

**Tier:** impl opus · review opus.

**Files:**
- Modify: `crates/yokohama-cli/src/screen_model.rs`
- Regenerate: `web/src/wire/*.json`
- Modify: `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`

**Interfaces — produces:**

```rust
/// One count per country, in `Country::ALL` order — the shape `MercenarySupply` has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagCounts { pub english: u8, pub american: u8, pub french: u8, pub german: u8, pub dutch: u8 }
impl FlagCounts { pub fn as_array(&self) -> [u8; 5] }   // English, American, French, German, Dutch

/// The countries of a list of order ids, read from `rules.orders`.
pub fn order_flags(ids: &[OrderCardId], rules: &Rules) -> FlagCounts
/// A seat's face-up flags: its `completed_orders` and its `technologies`.
pub fn face_up_flags(p: &PlayerState, rules: &Rules) -> FlagCounts

pub struct PlayerModel { /* … */ pub flags: FlagCounts }                  // the LAST field
pub struct HandPanel   { /* … */ pub flags_in_hand: Option<FlagCounts> }  // the LAST field; None whenever `hidden`
```

```ts
export interface FlagCounts { english: number; american: number; french: number; german: number; dutch: number }
// PlayerModel gains `flags: FlagCounts`; HandPanel gains `flags_in_hand: FlagCounts | null`
```

The fields go LAST in each struct: serde writes keys in field order, and the corpus diff measured for this design is those two keys appended.

- [ ] **Step 1: Write the cells** in `screen_model.rs`'s test module.

  1. `the_face_up_flags_count_fulfilled_orders_and_technologies_by_country`: a seat given `completed_orders` 1, 12, 2, 3 reads american 2, french 1, english 1, german 0, dutch 0 (the ids' countries are the ones `game/scoring.rs`'s cells use: {1, 12} American, {2, 5} French, {3, 10} English); then one technology pushed raises exactly its own country's count by one, the country read from `rules.techs` and named in the assertion's message.
  2. `the_face_up_flags_do_not_count_an_order_in_hand`: a seat whose only card of some country is in `order_cards_in_hand` reads zero for it.
  3. `the_face_up_flags_ride_a_seat_whose_hand_is_hidden`: `build_players` under `SeatFacts { hand_hidden: true, .. }` for every seat still carries `flags` equal to `face_up_flags`.
  4. `the_flags_in_hand_count_the_hand_and_include_the_starting_order`: `build_hand(.., hidden = false)` on a fresh seat (its hand is the kept starting order alone) reads `Some` with exactly one flag, in that card's country.
  5. Extend `a_masked_hand_carries_no_card_identity_across_the_wire`: the hidden panel's `flags_in_hand` is `None`, its JSON contains `"flags_in_hand":null`, and the open panel's is `Some`.
  6. `an_order_id_the_registry_does_not_know_counts_as_no_flag`: `order_flags` over any id under `Rules::empty_for_tests()` is `FlagCounts::default()`.
  7. The drift pin, verbatim but for the name of the counting function it calls:

```rust
/// **"Each seat's five counts equal the engine's"** (design §6), with no engine
/// function opened: three legs against public outputs, at every position.
/// The mercenary leg is the one that says WHICH country a flag belongs to —
/// the bonus is symmetric in the countries and cannot.
#[test]
fn the_models_flags_agree_with_the_engines_mercenaries_and_country_bonus() {
    let rules = Rules::standard();
    let (mut compared, mut nonzero, mut supply_out) = (0usize, 0usize, 0usize);
    for n in [2u8, 3, 4] {
        for seed in 0..6u64 {
            let pending = new_game(GameConfig::for_player_count(n), &rules, seed).unwrap();
            let mut state =
                finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); n as usize]).unwrap();
            let mut k: u64 = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
            for _ in 0..4000 {
                let engine = final_result(&state, &rules);
                for p in &state.players {
                    let arr = face_up_flags(p, &rules).as_array();
                    let sum: usize = arr.iter().map(|&x| x as usize).sum();
                    assert_eq!(sum, p.completed_orders.len() + p.technologies.len());
                    let b = &engine.standings.iter().find(|s| s.player == p.id).unwrap().breakdown;
                    assert_eq!(yokohama_core::eval::set_bonus(arr), i64::from(b.country));
                    for (i, c) in Country::ALL.into_iter().enumerate() {
                        let held = p.mercenaries.available.get(&c).copied().unwrap_or(0)
                            + p.mercenaries.used.get(&c).copied().unwrap_or(0);
                        let supply = match c {
                            Country::English => state.mercenary_supply.english,
                            Country::American => state.mercenary_supply.american,
                            Country::French => state.mercenary_supply.french,
                            Country::German => state.mercenary_supply.german,
                            Country::Dutch => state.mercenary_supply.dutch,
                        };
                        if supply > 0 {
                            assert_eq!(held, arr[i] / 2, "seat {:?} {c:?}", p.id);
                        } else {
                            supply_out += 1;
                            assert!(held <= arr[i] / 2, "seat {:?} {c:?}", p.id);
                        }
                    }
                    compared += 1;
                    if sum > 0 {
                        nonzero += 1;
                    }
                }
                if is_game_over(&state) {
                    break;
                }
                let legal = get_legal_moves(&state, &rules);
                k ^= k << 13;
                k ^= k >> 7;
                k ^= k << 17;
                state = apply_move(&state, &legal[(k % legal.len() as u64) as usize], &rules).unwrap();
            }
        }
    }
    // Anti-vacuity: an empty sweep, or one that never met a spent supply, proves nothing.
    assert!(compared > 100_000, "compared {compared}");
    assert!(nonzero > 50_000, "nonzero {nonzero}");
    assert!(supply_out > 1_000, "supply-out rows {supply_out}");
}
```

  The design's review ran this on an export at `bf3d182c`: 211,700 rows compared, 161,735 non-zero, 35,969 with a supply out, 4.22 s. The three floors are about half of those readings; if your reading is far from them, report it rather than moving a floor.

- [ ] **Step 2: Run red.** `cargo test -p yokohama-cli --lib screen_model` — expected: does not compile, naming `FlagCounts` and `face_up_flags`.
- [ ] **Step 3: Implement** the type, the two functions and the two fields. `face_up_flags` is `order_flags` over `completed_orders` plus one count per technology whose card `rules.techs` knows. `build_hand` sets `flags_in_hand` to `None` when `hidden`, else `Some(order_flags(&p.order_cards_in_hand, rules))`.
- [ ] **Step 4: Run green.** `cargo test -p yokohama-cli` — every binary green, no golden edited.
- [ ] **Step 5: Regenerate the corpus and read the diff.** `cargo test -p yokohama-web --test wire_fixtures` reads red (the corpus is stale); then `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then the plain run green. The diff must be the two keys and nothing else — check it mechanically, then read it:

```bash
git diff --stat -- web/src/wire | tail -1        # the review's reading: 27 files
python3 - <<'EOF'
import json, subprocess
def strip(o):
    if isinstance(o, dict):
        return {k: strip(v) for k, v in o.items() if k not in ("flags", "flags_in_hand")}
    return [strip(v) for v in o] if isinstance(o, list) else o
files = subprocess.run(["git", "diff", "--name-only", "--", "web/src/wire"], capture_output=True, text=True).stdout.split()
for f in files:
    was = json.loads(subprocess.run(["git", "show", f"HEAD:{f}"], capture_output=True, text=True).stdout)
    assert strip(json.load(open(f))) == was, f"{f}: moved by more than the two keys"
print(len(files), "files, each moved by the two keys alone")
EOF
```

- [ ] **Step 6: The client's contract.** In `types.ts` the interface and the two fields. In `contract.ts`: `"flags"` appended to `PLAYER`; a new root `HAND = ["seat", "hidden", "cards", "card_count", "flags_in_hand"]` with its `KeysOf<HandPanel, …>` assertion and its `KEYS.hand` entry, in the manner of `PLAYER`. In `contract.test.ts`, beside the existing run-time root checks:
  - the hand root: every Hand panel of every state's `Object.keys` equals `KEYS.hand`;
  - `a hand's flag count is the countries of its own cards`: for every state with a Hand panel, recounting `cards[].printed?.country` gives `flags_in_hand` exactly;
  - `no seat's row carries an in-hand count`: no `PlayerModel` of any state has the key `flags_in_hand`.
- [ ] **Step 7: The suites.** `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`; `cd worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test`; the lint gate; the fence check.
- [ ] **Step 8: Commit** the Rust file, the corpus files (by `git add web/src/wire`, a directory this task owns whole) and the three TypeScript files.

**Mutants:** `flags_in_hand` computed when `hidden` (cell 5). `face_up_flags` reading `order_cards_in_hand` (cells 2, 7). The starting order filtered from the hand's count (cell 4). One technology dropped from the count (cell 7, the sum leg). English and American swapped in `as_array` or in the count (cell 7, the mercenary leg; cell 1).

---

### Task 2: The tally on a seat's row

**Tier:** impl sonnet · review opus.

**Files:**
- Modify: `web/src/Dashboard.tsx`, `web/src/styles.css`
- Test: `web/src/Dashboard.test.tsx`

**Interfaces — consumes:** `PlayerModel.flags`, `HandPanel.flags_in_hand` (Task 1). **Produces** (Task 6 measures these): inside each `.seat`, one `.flags` row of five `.flag` cells in the order of `Dashboard.tsx`'s existing `MERCENARIES` table, each with `data-country` set to that table's wire key (`english` …), its count in `.flag-n`, the class `none` on a cell whose count is zero, and, where flags are in hand, a `.flag-hand` reading `+n`.

- [ ] **Step 1: Write the cells** (`describe("a seat's flags")`), each over a corpus payload through `wire` and `playersOf` / `handOf` (`web/src/test/panels.ts`):
  1. Five cells per seat in the engine's country order, each count equal to that seat's own `flags` value in the payload, on a payload where some seat has a non-zero count (precondition asserted, not assumed).
  2. A zero is drawn, with the class `none`.
  3. `+n` is drawn on the row whose seat is the Hand panel's `seat`, for each country with a non-zero `flags_in_hand`, and on no other row — on `wire.options` (the viewer acts) and on `wire.watching` (another seat acts: the row is the hand's seat, not the current one).
  4. A spectator's payload draws no `.flag-hand`.
  5. A clone with `flags` removed from every seat and `flags_in_hand` from the hand draws no `.flags` row and does not throw.
  6. The row's accessible name is `Flags: Britain 2, America 0, France 1, Germany 0, Netherlands 0` with `; in hand: Britain 1` appended when flags are in hand (the country words are `MERCENARIES`' third column; the numbers the payload's), and each `.flag-hand` has the title `1 in hand — counts when the order is fulfilled` (the number its own).
- [ ] **Step 2: Run red.** `cd web && npx vitest run src/Dashboard.test.tsx` — expected: the six cells fail, no `.flags` in the document.
- [ ] **Step 3: Implement.** `Dashboard` finds the Hand panel in `panels` and hands `Players` a `handFlags?: { seat: PlayerId; flags: FlagCounts }` only when `flags_in_hand` is non-null; `Players` draws the row only when `p.flags` is present. Each cell's mark is the country's word; the mercenary token's art beside it is the builder's choice (design §9) and, if drawn, has an empty `alt`.
- [ ] **Step 4: Run green**, then `npm run typecheck && npm test` whole.
- [ ] **Step 5: Commit.**

**Mutants:** `+n` drawn on the `is_current` row (cell 3, `wire.watching`). Zero cells dropped (cell 2). The cells in alphabetical order (cell 1, 6). The row drawn from an absent `flags` (cell 5).

---

### Task 3: The four-claim fixture

**Tier:** impl opus · review opus.

**Files:**
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (`constructed()` and its doc, which counts its own entries)
- Create: `web/src/wire/goals_claimed_4p.json` (by regeneration)
- Modify: `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts` (where it lists the corpus's files), `web/src/local/corpus.node.test.ts` (`NOT_SERVED` and the comment above it), `worker/test/corpus.test.ts` (`THIRD_COLUMN`, alphabetical: after `foreign_bot_409`), `docs/web-lane.md` (§2: the corpus's "**110** JSON files" and the pool's "twenty files no script names" each rise by one)

**Interfaces — produces:**

```ts
/** A 4P position's Achievements and Players panels, built in process. */
export interface GoalsClaimedFixture { achievements: { cards: AchievementModel[] }; players: { players: PlayerModel[] } }
// contract.ts: wire.goalsClaimed4p: GoalsClaimedFixture
```

The fixture's content is fixed here, because two of Task 4's mutants survive any other shape: **card 0 claimed by seats 2, 0, 3, 1 in that order; card 1 claimed by seat 3 alone; card 2 unclaimed.**

- [ ] **Step 1: Add the builder** and its entry in `constructed()`:

```rust
/// `goals_claimed_4p`: the claim band's fixture (issue #75). The corpus's walk
/// is a two-player game whose cards carry at most one claim, and no short
/// deterministic game reaches four, so the claim order is set on the cards of
/// a fresh 4P position and the panels are the model's own builders'. The order
/// is 2, 0, 3, 1 on purpose: neither seat order nor "the lowest seat first"
/// can pass for claim order. Constructed, not routed: a third-column file.
fn goals_claimed_4p() -> Value {
    use yokohama_cli::screen_model::{build_achievements, build_players, SeatFacts};
    use yokohama_core::{finalize_setup, new_game, GameConfig, PlayerColor, PlayerId, Rules, StartingOrderChoice};
    let rules = Rules::standard();
    let mut config = GameConfig::for_player_count(4);
    config.player_colors =
        vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
    config.start_player = PlayerId::new(0);
    let pending = new_game(config, &rules, SETUP_SEED).expect("a fixture game");
    let mut state =
        finalize_setup(pending, &[StartingOrderChoice::Keep(0); 4]).expect("a fixture game");
    state.achievements[0].claimed_by = [2, 0, 3, 1].map(PlayerId::new).to_vec();
    state.achievements[1].claimed_by = vec![PlayerId::new(3)];
    serde_json::json!({
        "achievements": build_achievements(&state),
        "players": build_players(&state, &rules, &SeatFacts::all_open(4)),
    })
}
```

- [ ] **Step 2: Run red, regenerate, run green.** The plain `wire_fixtures` run refuses an emitted file that is not committed; `UPDATE_WIRE_FIXTURES=1` writes it; `git status --short web/src/wire` then shows one new file and no modified one. Run the WHOLE binary: its leakage pins and its count cells enumerate the corpus.
- [ ] **Step 3: The lists and the type.** The interface in `types.ts`; the import and `wire.goalsClaimed4p` in `contract.ts`, and the file's name wherever `contract.test.ts` lists the corpus; `"goals_claimed_4p"` in `NOT_SERVED` and in `THIRD_COLUMN`; the two counts in `docs/web-lane.md` §2, each with this task's commit named as its reading.
- [ ] **Step 4: The suites** of Task 1's Step 7 (the node replay and the pool each assert their exact set).
- [ ] **Step 5: Commit.**

**Mutants:** the fixture's first card claimed in seat order (Task 4's cells then cannot fail on order: the reviewer confirms the committed JSON reads `[2, 0, 3, 1]`). The name left out of either list (that list's cell).

---

### Task 4: The claim band on the card

**Tier:** impl opus · review opus.

**Files:**
- Modify: `web/src/Cards.tsx`, `web/src/styles.css`
- Test: `web/src/Strips.test.tsx`, `web/src/Dashboard.test.tsx`, `web/src/Index.test.tsx`

**Interfaces — consumes:** `wire.goalsClaimed4p` (Task 3). **Produces** (Tasks 5 and 6 rely on these): on a claimed card, over the picture box, one `.claims` node with `role="img"` and an `aria-label`; inside it one `.claim` per claimant in claim order, each holding the board's `CubeIcon` and carrying the seat's colour class (`seatClass`) and a `title`; the first has the class `first`; `.claims-vp.first` holds `first_place_vp` and, only when a later claim exists, `.claims-vp.later` holds `subsequent_vp` after a divider.

**The words, exactly:** the first claim is `Red claimed first — 6 VP`, a later one `Blue claimed — 3 VP`; the colour word is `colors[seat]`, or `seat N` where the array has none; the numbers are the model's. Each is its cube's `title`; the band's `aria-label` is all of them in claim order joined by ` · `.

- [ ] **Step 1: Write the cells.**
  1. Four claims (`wire.goalsClaimed4p.achievements.cards[0]`, its colours from the fixture's `players`), in the Goals strip and in the dashboard's Achievements panel: four `.claim` in the order seats 2, 0, 3, 1 by colour class; only the first has `first`; the band's `aria-label` and each `title` as above; both VP numbers shown.
  2. One claim (`cards[1]`, seat 3): the ringed place holds seat 3's colour; no `.claims-vp.later`, no divider.
  3. No claim (`cards[2]`): no `.claims`; the `unclaimed` chip is there.
  4. Each of 1–3 again with `art={null}`: the band is drawn over the blank.
  5. A claimant the colours array does not name (cell 1's card with a two-colour array): its words read `seat 2 claimed first — …` and its cube is drawn.
  6. The goals index (`Index.test.tsx`), on the corpus's own index payload, whose claimed card is claimed by seat 1: the band is there with one cube in the ringed place.
  7. No caption carries `first ` or `also ` chips: `.chip.claimed` is gone from all three renderings.
- [ ] **Step 2: Run red.** `cd web && npx vitest run src/Strips.test.tsx src/Dashboard.test.tsx src/Index.test.tsx`.
- [ ] **Step 3: Implement** in `AchievementCardView`. The band is positioned against the picture box, so the picture and the blank share one wrapper; the band is a single row that never wraps, its cubes shrinking before they overflow. Existing cells that assert the retired chips move on purpose, each named in the report.
- [ ] **Step 4: Run green**, then `npm run typecheck && npm test` whole.
- [ ] **Step 5: Commit.**

**Mutants:** the cubes sorted by seat (cell 1). `first` given to the lowest seat (cells 1, 2). The band dropped when there is no art (cell 4). The `aria-label` dropped, or the cubes left exposed to it (cell 1). `.claims-vp.later` drawn with one claim (cell 2).

---

### Task 5: The walk lights the claimed card

**Tier:** impl sonnet · review opus.

**Files:**
- Modify: `web/src/diffFrames.ts`, `web/src/walk.ts`, `web/src/App.tsx`, `web/src/TableSections.tsx`, `web/src/Strips.tsx`, `web/src/Dashboard.tsx`, `web/src/Cards.tsx`
- Test: `web/src/diffFrames.test.ts`, `web/src/walk.test.ts`, `web/src/Strips.test.tsx`, `web/src/Dashboard.test.tsx`, `web/src/TableSections.test.tsx`

**Interfaces — produces:**

```ts
// diffFrames.ts
export interface Changes { /* … */ goals: Set<AchievementType>; any: boolean }
// a card is in `goals` when its `claimed_by` is LONGER in `b` than in `a`
// Cards.tsx: AchievementCardView gains `changed?: boolean` → the class `changed` on its figure
// Strips.tsx: GoalsStrip gains `changed?: Set<AchievementType>`
// Dashboard.tsx: Dashboard gains `changedGoals?: Set<AchievementType>`, handed to Achievements
// TableSections.tsx: gains `changedGoals?: Set<AchievementType>`, handed to GoalsStrip
```

- [ ] **Step 1: Write the cells.**
  - `diffFrames.test.ts`, one edit per fresh clone of `wire.atK`: a claim pushed onto one card names that card and no other; a claim popped names nothing; a claim's seat changed at the same length names nothing; with a claim pushed and nothing else moved, `any` is true; the self-diff cell's sum gains `goals`.
  - `walk.test.ts`: `heldChanges` carries the union of the mover's steps' `goals` (the file had no `heldChanges` cell before this task: the run is `buildRun`'s over clones of a corpus frame, one claim pushed per step).
  - `Strips.test.tsx`, `Dashboard.test.tsx`: the card the set names has the class `changed` and no other card does; with no set, none does.
  - `TableSections.test.tsx`: `changedGoals` reaches the Goals strip.
- [ ] **Step 2: Run red.**
- [ ] **Step 3: Implement.** `goals` is read from the two frames' Achievements panels and nothing else; it joins `any`; `heldChanges` unions it; `App` hands `highlights?.goals` to `TableSections` and to `Dashboard`. The module's head comment and its paragraph on which sets are fixed and which only grow gain `goals`: a claim, once made, is never withdrawn in play, and the read is still "longer in `b`".
- [ ] **Step 4: Run green**, then `npm run typecheck && npm test` whole.
- [ ] **Step 5: Commit.**

**Mutants:** `goals` reading "differs" (the popped and the changed-seat cells). `goals` left out of `any`. `heldChanges` not unioning it. The Achievements panel lighting from `changedSeats`.

---

### Task 6: The browser's layout cells

**Tier:** impl opus · review opus.

**Files:**
- Modify: `web/e2e/the_table.spec.ts` (and `web/e2e/harness.ts` only for a helper two cells share)

Layout is real only in Chromium. Both cells ASSERT boxes and RECORD the rest, in the manner of this file's width cells; no screenshot is taken.

- [ ] **Step 1: Cell 1 — four cubes on one line.** A four-seat table at the open door. `page.route` rewrites the game-state reply so its Achievements panel is the committed `goals_claimed_4p.json`'s (read from `web/src/wire/` with `readFileSync`; its cards are a server's own). At each of the widths this file's set-table cell walks (390, 1024, 1201, 1440): in the Goals strip's first card the four `.claim` boxes share a top within 1 px, each lies inside the `.claims` box, the `.claims` box and both `.claims-vp` lie inside the picture box, and a cube's width is at least the floor. **The floor is measured before it is set**: record the cube's width at 390 px, set the floor at half that reading, and state both in the test. If the state does not arrive by a reply `page.route` can rewrite (a local game answers from the Web Worker), STOP and report: do not build a page by hand.
- [ ] **Step 2: Cell 2 — the tally at a phone's width.** At 390 px on a running game: every `.flags` box lies inside its `.seat` box, and the page's horizontal overflow (`readOverflow`) is what it is without the row — assert the page delta, record the rest.
- [ ] **Step 3: Anti-vacuity.** Each cell first asserts the thing it measures is on screen (four `.claim` nodes; five `.flag` cells per seat).
- [ ] **Step 4: Run** `cd web && npm run e2e` whole, in the foreground; state the count and the wall. Every spec keeps its `test.skip(!haveBrowser, …)` and its `./sw` import (`every_spec_can_skip`, `every_spec_declares_its_worker`).
- [ ] **Step 5: Commit.**

**Mutants (CSS):** the band allowed to wrap (cell 1's shared top). A fixed cube size that overflows at 390 (cell 1's containment). The flag row given a fixed width (cell 2).

---

### Task 7: The words

**Tier:** impl sonnet · review opus.

**Files:**
- Modify: `web/src/Help.tsx` (and `web/src/Help.test.tsx` where it pins a sentence), `docs/web-guide.md`, `docs/web-lane.md` (any list of the model's fields)

- [ ] **Step 1: Place two sentences** where the existing text describes the goals and a seat's row, in both `Help.tsx` and `docs/web-guide.md`, and correct any neighbouring sentence they make false (one that says a claim is listed under the card, for instance):
  - "A claimed goal shows the claimant's assistant on the card. The first to claim stands in the ringed place beside the first-place points; later claimants follow beside theirs."
  - "Each player's row counts their flags by country: the flags on fulfilled orders and on technologies, which earn mercenaries and score the country sets. On your own row, `+n` is the flags on orders still in your hand; they count once the order is fulfilled."
- [ ] **Step 2: Run** `cd web && npm test && npm run test:wasm` (its document-link cells read these files) and `cargo test -p yokohama-web --test lane_document`.
- [ ] **Step 3: Commit.**

---

### Task 8: The help panel's pictures

**Tier:** impl opus · gate: the owner's reading. **The last task before the whole-branch review**, because it ends with files the owner must read before they are committed.

**Files:**
- Modify: `web/e2e/shots.spec.ts`, `web/e2e/readme_shots.spec.ts` (the text sidecar only), `web/src/Help.tsx` (alts and captions)
- Regenerate: `web/public/help/*.webp` (five)

- [ ] **Step 1: The sidecar.** Beside every picture each generator writes, it also writes the page's accessibility snapshot as text (`await page.locator("body").ariaSnapshot()`) under the same name with `.aria.txt`, under the gitignored `web/help-shots/` (the help panel's beside their PNGs; the README's under `web/help-shots/readme/`, since that spec removes its own scratch directory when it ends). Commit this step alone: it changes no picture.
- [ ] **Step 2: Re-cut the five**, both commands, in this order, opening none of the output:

```bash
cd web && npm run build && YOKOHAMA_WEB_SHOTS=1 npx playwright test shots.spec.ts
python3 docs/tools/mkhelpart.py        # from the repository root
```

- [ ] **Step 3: Re-read every `alt` and `caption`** of the five `Shot`s in `web/src/Help.tsx` against that picture's `.aria.txt`: every run photographs a different game, so each claim about what is in frame (a prompt's words, a seat, a count, a name) is checked against the text and corrected where it no longer holds. Add what this cycle put in frame (the flag row; a claim band only if the snapshot shows one).
- [ ] **Step 4: Report, and stop without committing Steps 2–3.** The report lists, per picture: each claim checked against the text and its verdict; each claim only the picture can settle (position, scroll, what is cut off), for the owner; and `git status --short`. The orchestrator puts the five pictures and the report before the owner and commits on the owner's word.
- [ ] **Step 5 (the orchestrator):** `cd web && npm run build && npm run test:wasm` (the precache inventory holds the help pictures), then commit the five `.webp` and `Help.tsx`.

---

## After the tasks — the orchestrator's, in order

1. **The whole-branch review**, on an export: a Workflow of about three agents led by Fable (the owner's cap for a light cycle), one fix wave, a scoped re-review.
2. **The gates at the branch's head**: every per-edit suite re-counted; the fifteen release gates as ONE uninterrupted sequence, the release binaries pre-built, the wall machine-tagged. No fenced crate moved, so the roster is expected to read as at master.
3. **Origin**: the pull request (closing #75 and #76), `web.yml` dispatched on it, a merge commit, master pushed to the lab.
4. **The release**, by `docs/web-lane.md` §7 and the two runbooks: the owner's window, not planned here. A deploy of this change drops a picker left open on the Worker (design §10).
5. **Task 9 — the README's ten pictures**, once the Worker runs the release. They are the Worker's own, so they are cut live: the quota guard first (`worker/RUNBOOK.md` §7), then `YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test readme_shots`; the alts in `worker/README.md` §1.4 re-read against each picture's `.aria.txt` as in Task 8; the ten pictures the owner's reading; landed as its own small pull request.
6. **The close**: `docs/plans/2026-11-20-web-claims-and-flags-close.md` with the readings; a decision-log row in `guidingpath.md`; the board; and the terminal's flag counts recorded where the chooser cycle will find them (design §11).
