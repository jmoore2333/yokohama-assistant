# CLI Branch-4 Touchup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Execution model per the owner: Opus implementers (xhigh), a Fable review stage per task, the orchestrator adjudicating.

**Goal:** Land the six branch-4 CLI items plus the owner's orders-filled panel line and the
`areas` reference view, with guides, Forge validation, and a README/screenshot pass — as
local commits on `cli-branch4-touchup`, never pushed until the Phase-C lane lands.

**Architecture:** Every change lives in `crates/yokohama-cli` (render/describe/panel/why/
suggest/view layers) plus the doc surfaces. The engine is read, never edited. New facts
render from engine-exported data (`completed_orders`, `EndGameTrigger`, `counter_rows`,
per-power reward functions); no CLI-side rule math.

**Tech Stack:** Rust (toolchain pinned 1.97.1), cargo test per-edit tier, Forge MCP for
visual validation, chromium render for README PNGs.

**Spec:** `docs/plans/2026-08-18-cli-branch4-touchup-spec.md` — the plan argues from it;
executors read both. The spec's §6 records the Fable review this plan already absorbs.

## Global Constraints

- **NO push, NO PR, NO gh writes.** Local commits on `cli-branch4-touchup` only (spec §1).
- **Engine crates untouched.** A change that seems to need one is a STOP-and-rescope.
- **Guide pair rule:** any user-visible change updates `docs/cli-guide.md` AND
  `docs/cli-guide.html` word-identically (modulo markup) in the SAME commit; `guide_sync`
  must pass per edit.
- **Honesty properties:** engine sole authority; piped output byte-identical plain text
  (`style::paint` is identity off-TTY); `NO_COLOR` honored; widths measured through
  `screen::visible_width`; glyphs only via `style.rs`'s vocabulary macro, never inline.
- **Per-edit gate after every task:** `cargo test -p yokohama-cli` (baseline 771 pass +
  2 ignored: 743 lib · 9 bga_import · 18 guide_sync · 1 dogfood; lib wall ~22.5s on this
  Mac) and `cargo clippy --workspace --all-targets -- -D warnings`. Task 7 runs the other
  suites once (they must not move).
- **TDD:** the failing test is written and SEEN to fail before the implementation, every
  cycle. Commit messages follow repo style (`feat(cli): …` / `docs: …`), each ending with
  the `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer.
- Height/width pins move only DELIBERATELY: a changed `WALK_HEIGHTS` / `FRAME_HEIGHTS` /
  panel-sweep constant is re-measured, asserted, and named in the commit message.

---

### Task 0: Branch + baseline

**Files:** none created; git state only.

- [ ] **Step 0.1:** `git checkout master && git status --short` — MUST be clean and at/past
  `3021dc5`. `git checkout -b cli-branch4-touchup`.
- [ ] **Step 0.2:** Confirm baseline: `cargo test -p yokohama-cli 2>&1 | grep "test result"`
  → 743/9/18/1 all ok (the 2 dogfood ignores expected). No commit.

---

### Task 1: End-trigger banner (spec §2.1)

**Files:**
- Modify: `crates/yokohama-cli/src/render.rs` (the `END TRIGGERED: {:?}` delta line at
  ~659; new banner + reason-words functions near `end_trigger_line` ~390)
- Modify: `crates/yokohama-cli/src/style.rs` — **U+2550 `═` is NOT in the vocabulary: T3
  REMOVED it from the crate entirely** (style.rs:188-196: "the character is GONE rather
  than converted … no longer spelled anywhere in the crate"). The banner ADDS it as a new
  named glyph via the `vocabulary!` macro (East-Asian-Width Ambiguous, the review-pinned
  class), which also means updating the module doc: the name/code-point count arithmetic
  (~:103, ~:114), the emitted-census sentence (~:168), and the DEFERRED CONVERSIONS
  narrative whose "no longer spelled anywhere" claim stops being true. The opening-frame
  pin at screen.rs:854-875 (no U+2550 in the composed frame) is scoped to its own frame
  and does NOT trip on the banner — say so in the test doc rather than discovering it.
- Modify: `crates/yokohama-cli/src/session.rs` (flip detection in `apply` ~462 ONLY —
  see step 1.7's `apply_standalone` note)
- Modify: `crates/yokohama-cli/src/repl.rs` (banner drain into the post-apply output
  stream ~504-516; load-time notice; bot-review inclusion)
- Test: in-module `#[cfg(test)]` blocks of `render.rs`, `session.rs`, `repl.rs` (the
  repo's pattern — `Repl::step` drives command-level behavior)

**Interfaces:**
- Produces: `render::trigger_reason_words(t: &EndGameTrigger) -> String` (rulebook words,
  no `Debug`); `render::end_trigger_banner(t: &EndGameTrigger, width: usize) -> String`
  (the bordered block, `Tone::Warn`, rule from `═` vocabulary const);
  `Session::take_end_trigger_banner(&mut self) -> Option<EndGameTrigger>` (set on every
  observed false→true flip of `end_game_triggered`, cleared on take — session-scoped,
  never serialized).
- Consumes: `EndGameTrigger`'s five variants (`types/state.rs:142-153`:
  `TradingHousesComplete(PlayerId)`, `ShophousesComplete(PlayerId)`, `OrderDeckEmpty`,
  `ChurchTrackFull`, `CustomsTrackFull`); `state.final_round` for the load notice.

- [x] **Step 1.1: Failing tests for the words mapping.** In `render.rs` tests:
  `trigger_reason_words_cover_every_variant_in_rulebook_words` — exhaustive match over all
  five variants (no `_` arm in the IMPL, so a sixth variant is a compile error); asserts
  each string is non-empty, contains no `{:?}` artifacts (`assert!(!s.contains("("))` is
  wrong — `TradingHousesComplete` names the PLAYER, e.g. "RED built all 4 Trading Houses";
  assert the seat name appears), and `OrderDeckEmpty`'s words state BOTH halves ("order
  deck is empty" AND "a Port slot cannot refill" — the necessary-not-sufficient nuance
  `end_trigger_line`'s doc carries).
- [x] **Step 1.2:** Run `cargo test -p yokohama-cli trigger_reason_words` → FAIL
  (unresolved function).
- [x] **Step 1.3:** Implement `trigger_reason_words` (takes `&GameState` too if seat names
  are needed — match the existing describe helpers' signature style, e.g.
  `describe::area_label(state, …)`). Run → PASS.
- [x] **Step 1.4: Failing tests for the banner block.**
  `end_trigger_banner_is_bordered_full_width_and_states_the_two_phase_rule`: renders at
  width 80; asserts top/bottom rules are exactly 80 columns of the `═` vocabulary char
  (via `screen::visible_width`), body contains `"END TRIGGERED"`, the reason words, and
  the verified rule clause `"The current round finishes, then every player gets one full
  final round"`; and `visible_width(line) <= 80` for every line.
  `the_banner_is_plain_text_when_piped`: with color OFF (the default in tests), the output
  contains no ESC byte.
- [x] **Step 1.5:** Run → FAIL. Implement `end_trigger_banner` (wrap via the existing
  prose-wrap helper — `suggest::wrap_to` is the precedent; border from the vocabulary
  const; `paint(Tone::Warn, …)` per line). Run → PASS.
- [x] **Step 1.6: Failing test for the delta-line reword.** NO existing test pins the
  delta `END TRIGGERED` line (the string appears exactly once in the repo, render.rs:660)
  — write a FRESH `delta_summary` flip test using the trigger-state constructors around
  render.rs:~1123 and the delta-fixture pattern at ~1679-1751:
  `the_delta_line_speaks_words_not_debug` asserts the delta window line for a flip equals
  the short form built from `trigger_reason_words` and contains no `Some(` / `{:?}` residue.
  Run → FAIL. Replace `format!("END TRIGGERED: {:?}", …)` at ~659-661. Run → PASS.
- [x] **Step 1.7: Failing tests for session flip detection.** In `session.rs` tests, drive
  a fixture to the trigger (the repo's session tests build small states — reuse the
  existing fixture pattern near session.rs:1674 which already constructs
  `end_game_triggered` states): `apply_sets_the_pending_banner_exactly_on_the_flip` —
  None before, `Some(reason)` after the flipping move, None again after take, and a
  SECOND non-flipping apply leaves it None. `undo_rearm`: after the flip + take, unwind
  (drive the session's existing undo path), re-apply a flipping move → `Some` again
  (transition-fired semantics, spec §2.1). Run → FAIL. Implement (compare
  `before.end_game_triggered` / `after.end_game_triggered` around the staged apply; both
  states are in hand in `apply`). **Detection lives in `apply` ONLY — `apply_standalone`'s
  one accepted event (`RevealStartingOrder`) moves card identity and cannot flip the
  trigger (only `end_game::check_end_game_triggers` on the move path sets it); leave a
  one-line comment saying so instead of dead detection code.** Run → PASS.
- [x] **Step 1.8: Failing tests for the REPL surfaces.** Via `Repl::step` fixtures:
  `the_banner_reaches_the_inline_stream_once` (flip move → output contains the banner
  block; next step's output does not); `the_bot_review_carries_the_banner_when_the_flip
  _happened_inside_the_run` (bot run whose pump crosses the trigger → the review/narrative
  text contains `"END TRIGGERED"`); `a_loaded_triggered_save_prints_the_branched_notice`
  (load fixtures with `end_game_triggered && !final_round` → notice contains
  `"finishing the current round"`; with `final_round` → `"FINAL ROUND in progress"`; and
  the full banner does NOT render on load).
- [x] **Step 1.9:** Run → FAIL. Wire: repl drains `take_end_trigger_banner` where it
  formats `{warnings}{narrative}{prompt}` (~repl.rs:516) and inside the bot-review branch
  (~511-514); the load path emits the branched notice once. Run → PASS.
- [x] **Step 1.10: Guides.** `docs/cli-guide.md` + `.html`: the end-triggers strip section
  gains the banner behavior paragraph (word-identical pair). Run the full per-edit gate
  (cli suite + clippy) → green.
- [x] **Step 1.11: Commit** `feat(cli): loud one-time end-trigger banner — transition-fired,
  bot-run and load-notice aware (branch-4 item 5)`.

---

### Task 2: Lab slot-surcharge reword (spec §2.4)

**Files:**
- Modify: `crates/yokohama-cli/src/describe.rs:629-649` (+ its test block; hoist/reuse a
  `plural()` — `card_render.rs:604` has the `pub(crate)` one)
- Modify: `docs/cli-guide.md:287`-area §5 block + `.html:244` twin

**Interfaces:** none new — the describe string changes shape:
`", slot fee: {n} yen"` / `", slot fee: {n} imported good[s]"` / both-nonzero composed
`", slot fee: {a} yen + {b} imported good[s]"`.

- [ ] **Step 2.1: Failing tests.** In describe.rs's test block (the existing Laboratory
  fixtures sit at ~1566-1600): `the_slot_surcharge_reads_as_a_fee_not_a_gain` — three cases:
  yen-only (`extra_yen: 2` → contains `"slot fee: 2 yen"`, NOT `"+2 yen"`), imported-only
  (`extra_imported: 1` → `"slot fee: 1 imported good"`, no `"good(s)"`, no `+`),
  both-nonzero (`extra_yen: 1, extra_imported: 1` → exactly one `"slot fee:"` occurrence,
  `"1 yen + 1 imported good"`). Run → FAIL.
- [ ] **Step 2.2:** Implement the composed clause. Run → PASS. Sweep for other pinned
  sites: `grep -rn '"+.* yen"' crates/yokohama-cli` and fix any fixture asserting the old
  wording.
- [ ] **Step 2.3:** Update both guide blocks (word-identical). Per-edit gate → green.
- [ ] **Step 2.4: Commit** `feat(cli): lab slot surcharge reads as the fee it is
  (branch-4 item 4)`.

---

### Task 3: Players-panel orders line (spec §2.5)

**Files:**
- Modify: `crates/yokohama-cli/src/screen_model.rs` (`build_player` ~1822 — the per-player
  model gains `orders_filled: Vec<OrderCardId>` in completion order, straight from
  `player.completed_orders`; the in-hand count already modeled for the President line
  moves with it)
- Modify: `crates/yokohama-cli/src/panel_render.rs` (2P block ~500-600: new `Orders` line
  beside `Tech`; President line drops its order clause ~592; 3-4P tail row ~1261-1310:
  `N filled` count only, elision order re-derived; the width-sweep pin re-measured)
- Modify: `crates/yokohama-cli/src/screen.rs` (`WALK_HEIGHTS` ~410, `FRAME_HEIGHTS` ~344 —
  re-measured, changes named in the commit)
- Modify: `docs/cli-guide.md`/`.html` (§2 heights if pinned there — `guide_sync`'s
  `section2_frame_heights_match_the_composed_screen` tells you; panel description prose)
- Test: panel_render + screen + guide_sync existing pins move; new tests below

**Interfaces:**
- Produces (2P line, exact shape): `Orders  2 in hand · 3 filled (O23, O08, O12)` —
  completion order (the Vec's own order), `O{:02}` codes, `0 filled` with no parenthetical,
  in-hand count keeps the existing hidden-hand treatment on this line. 3-4P tail-row fact:
  `N filled` (count only, never codes).
- Consumes: `Player::completed_orders: Vec<OrderCardId>` (public, mode-independent —
  written at `apply_move.rs:1346`).

- [ ] **Step 3.1: Failing tests, 2P.** In panel_render tests:
  `the_orders_line_states_hand_count_and_filled_codes_in_completion_order` — fixture with
  completed_orders `[23, 8, 12]` and 2 hand orders → panel contains
  `Orders  2 in hand · 3 filled (O23, O08, O12)` (exact substring, completion order NOT
  sorted); `zero_filled_has_no_parenthetical` → `0 filled` and no `(`;
  `the_president_line_no_longer_carries_the_order_clause` → the President line matches
  location-only shape; `a_hidden_bot_hand_hides_the_count_not_the_filled_clause` → the
  in-hand half renders the existing `(hidden)` treatment, filled clause intact;
  **`the_widest_legal_orders_line_survives_its_budget`** — the beside-panel line clips at
  `DASH_W = 74` (panel_render.rs:159-164), and an unbounded codes parenthetical would clip
  mid-code from ~7 filled with the hidden marker, so the OVERFLOW TREATMENT IS SPECIFIED,
  not discovered: **when the parenthetical would overflow the line budget it elides with
  the vocabulary's ellipsis mark (the one context-row elision already uses) — codes drop
  from the END, the paren always closes, and the `N filled` count stays EXACT** (e.g.
  `12 filled (O23, O08, …)`); the sweep drives the widest legal joint case (in-hand +
  filled bounded by the 36-card deck) the way the module's other widest-legal pins do
  (achievements :2303, tracks :2388, seat tail :2451). Run → FAIL.
- [ ] **Step 3.2:** Implement model + 2P render. Run → PASS, EXPECT height pins to fail —
  that failure is the deliberate re-measure trigger.
- [ ] **Step 3.3:** Re-measure `WALK_HEIGHTS` (and `FRAME_HEIGHTS` only if the composed
  frame actually grew — the 2P panel sits beside the board; verify whether the +1 line
  overflows the board height before touching the frame constant). Update pins + guide §2
  numbers if `guide_sync` says so. Run cli suite → green.
- [ ] **Step 3.4: Failing tests, 3-4P.** `the_seat_tail_row_carries_the_filled_count` —
  4P fixture, tail row contains `3 filled` and NEVER a code parenthetical at any fixture
  width. **The four-fact worst case overflows the 47/48-column budget (~58 columns even
  under the joint in-hand+filled ≤ 36 bound), and NO elision machinery exists on this row
  today — the yield policy is a DECISION, made here: when the swept row exceeds its
  column, `next take ¥N` drops FIRST and alone (its information is recoverable at the
  Employment Agency decision preview); the orders, locked, and filled facts never yield.**
  This conditionally re-opens the T10/T11 KEEP verdict on that fact (kept when the budget
  for a disambiguating word was zero; a fourth fact changes the arithmetic) — recorded in
  spec §5 for owner adjudication, and the fn doc at panel_render.rs:1261-1297 (the
  three-facts/47-column claims) is REWRITTEN with the re-measure. RE-MEASURE the sweep pin
  (`the_widest_legal_seat_tail_row_still_fits_its_column`) over the constrained joint
  domain — the new worst case is computed, asserted ≤ the column budget, and stated in
  the test doc. Run → FAIL → implement → PASS.
- [ ] **Step 3.5:** Guides: panel section prose (both formats). Per-edit gate → green.
- [ ] **Step 3.6: Commit** `feat(cli): players panel shows orders filled — 2P codes in
  completion order, 3-4P counts by measured necessity (owner item 7)`; the height/width
  pin moves named in the body.

---

### Task 4: `why` — forced-move lead, full counters, nonzero-product aid (spec §2.2/2.3/2.6)

**Files:**
- Modify: `crates/yokohama-cli/src/why.rs` (lead line + counter block at the notes
  builder ~440-461)
- Modify: `crates/yokohama-cli/src/suggest.rs` (the shared table row machinery ~101-130;
  summary line under the table)
- Modify: `crates/yokohama-cli/src/style.rs` (vocabulary macro ~197-244: second name
  `NONZERO_MARK` for U+25B6 per the `SEPARATOR`/`EMPTY_SLOT` precedent)
- Modify: `docs/cli-guide.md`/`.html` (the quoted `why`/`suggest` screens + §5 blocks)
- Test: why.rs's own battery (the refusal-table + fixture pattern is there ~532-1147)

**Interfaces:**
- Produces: lead line `Only legal option — nothing was weighed: the position offered
  exactly one move.` gated on `get_legal_moves(&state, rules).len() == 1` at the
  reconstructed decision state (BOTH arms; runner-ups omitted when it fires); counter
  block rendered from `yokohama_core::driven_runner::counter_rows(&stats)` (via the crate
  glob — verify the re-export path compiles, `lib.rs:10` is `pub use game::*`), names
  verbatim, grouped into a search line + a budget line, with
  `the_counter_block_names_all_fifteen_rows` pinning every `counter_rows` name appears;
  table rows gain the `glyph::NONZERO_MARK` lead column (nonzero product) / same-width
  blank, `Tone::Dim` zero rows, and the summary line
  `N of M features carry a nonzero product — they are this move's whole story.`
- Consumes: `get_legal_moves`, `counter_rows`, `Explanation`'s per-row product values
  (the table builder already has them — the marker derives from the SAME value the
  `product` cell prints, one source).

- [ ] **Step 4.1: Failing test, forced lead.** Reuse why's fixture machinery to a
  1-legal-move decision (a forced point exists in the every-move pin's game — or construct
  via the pending-choice singleton pattern why's fixtures already use):
  `a_forced_move_leads_with_only_legal_option_and_omits_runner_ups` — output STARTS the
  account body with the lead sentence, contains no runner-ups heading;
  `an_unforced_move_has_no_forced_lead` control; and
  `a_forced_greedy_move_composes_the_lead_with_its_one_ply_account` — a GREEDY contestant
  at its forced first move (why.rs:622-623's own fixture doc: "the bot's first move of a
  game is the forced End additional-action phase"; the greedy fixture pattern exists at
  `a_greedy_mover_renders_the_leaner_block`, why.rs:~1181) renders the lead line AND the
  ONE_PLY_ACCOUNT sentence together — composition, not replacement (spec §2.2's
  both-arms clause, red-greened rather than asserted in prose). Run → FAIL.
- [ ] **Step 4.2:** Implement (the account path reconstructs `state` already; call
  `get_legal_moves` there once). **Implement the lead + runner-ups omission in WHY's
  account path, NOT in the shared builder: suggest.rs:694 renders its own forced-point
  message ("No runner-up: this decision point offered exactly one move.") with its own
  pin at suggest.rs:1145 — both stay untouched** (spec §0's why-is-the-vetted-surface
  note). Run → PASS.
- [ ] **Step 4.3: Failing test, counters.** `the_counter_block_names_all_fifteen_rows` —
  drive a lookahead why; for each `(name, _)` in `counter_rows(&stats)` assert the block
  contains the name; plus `the_two_line_grouping_holds` (search line + budget line, both
  ≤ frame width via `visible_width`). Run → FAIL. Implement from `counter_rows` with an
  unknown-name error arm. Run → PASS.
- [ ] **Step 4.4: Failing test, marker.** `nonzero_product_rows_carry_the_mark_and_zeros_
  dim` — a fixture explanation with known zero/nonzero rows: marked rows exactly the
  nonzero-product ones, column alignment unchanged (`visible_width` equal across rows),
  summary line counts match, TOTAL row unmarked; `piped_output_is_escape_free` (paint
  identity off-TTY — assert no ESC). Run → FAIL. Add `NONZERO_MARK` via the vocabulary
  macro — **and this trips an existing pin the implementer must move DELIBERATELY:
  `the_glyph_table_records_every_constant_truthfully` (style.rs:945-951) asserts U+00B7
  is the vocabulary's ONLY shared code point by exact equality. Extend its sanctioned
  list to include `(0x25B6, vec!["ACTIVE", "NONZERO_MARK"])` as the second deliberate
  pair, and update the module-doc arithmetic the pin's comment references ("Twenty-one
  names over twenty code points" ~:103, "eighteen of the twenty-one" ~:114) plus the doc
  table row for the new name — the commit message names the vocabulary move.** Implement
  row lead + dim + summary. Run → PASS.
- [ ] **Step 4.5:** Guides: re-capture the quoted screens (the guide quotes captured
  output; `guide_sync` holds label constants — update both formats and any moved label).
  Per-edit gate → green (why's every-move pin ~22.5s is the long pole; it must stay green
  UNCHANGED in meaning — these are additive renders).
- [ ] **Step 4.6: Commit** `feat(cli): why leads forced moves honestly, surfaces all 15
  search counters, and marks the nonzero-product rows (branch-4 items 1, 2, 6)`.

---

### Task 5: The `areas` view (spec §2.7) — the key component

**Files:**
- Modify: `crates/yokohama-cli/src/screen_model.rs` (new `AreasPanel` +
  `build_areas(state, rules) -> AreasPanel` near the other builders ~1816+)
- Modify: `crates/yokohama-cli/src/view_render.rs` (new `areas_view(&AreasPanel,
  ViewContext) -> String`, the card-surface pattern `ports_view`/`labs_view` follow)
- Modify: `crates/yokohama-cli/src/repl.rs` (ViewSurface gains `Areas` — enum ~123,
  `command()` ~135, the parse site ~249, the view-menu labels ~1693, `HELP_TEXT` ~3322)
- Modify: `docs/cli-guide.md`/`.html` (§5 command blocks + a view section)
- Test: screen_model + view_render + repl test blocks; `guide_sync` enforces the §5 half

**Interfaces:**
- Produces: `AreasPanel { rows: Vec<Vec<AreaCard>> }` grouped by `grid_position` board
  rows (no assumed shape — the `render::board_grid` rule); `AreaCard` carries: id + name +
  type; the payout ladder as `Vec<String>` (one line per power tier 1..=5, 5-Power bonus
  called out); site slot lines with FULL printed-reward words (the `sites`-view
  vocabulary); occupancy (president owner, assistant counts, Station). Special areas
  (Church/Customs/Port/Lab/Employment/Chinatown/etc.) get action-in-words lines and a
  pointer line to their dedicated view where one exists (`tracks`/`ports`/`labs`).
- Consumes — **the callable-ladder inventory (spec §2.7 item 2), assembled at
  implementation start and recorded in the module doc:** `game/actions/production.rs`
  `copper_mine_reward`/`silk_mill_reward`/`tea_plantation_reward`/`fishing_ground_reward`
  (power); `game/actions/commercial.rs` `bank_reward`, `chinatown_rate`; rules data for printed
  5-Power bonuses and site rewards (the `sites` view's own source); anything else
  exported that `describe.rs:605-617`'s Bank-preview pattern reaches. Hand-worded lines
  ONLY where no callable exists, each with a pin against apply behavior.

- [ ] **Step 5.1: Failing tests, model.** `build_areas_groups_by_board_rows_any_geometry`
  (fixture-geometry safe, like the board_grid tests); `production_ladders_are_computed_not_
  transcribed` — for each production area, the ladder line at power p quotes EXACTLY
  `<area>_reward(p)`'s value (the test CALLS the engine fn and compares — transcription
  cannot pass a data change); `the_bank_ladder_matches_bank_reward`;
  `special_areas_state_their_action_and_point_at_their_view`. Run → FAIL.
- [ ] **Step 5.2:** Implement `AreaCard`/`AreasPanel`/`build_areas`. Run → PASS.
- [ ] **Step 5.3: Failing tests, render.** `areas_view_renders_every_area_exactly_once`;
  `board_rows_chunk_rather_than_truncate` — at a 200-column context no line exceeds the
  width (via `visible_width`) and every card's reward words appear unelided;
  `the_view_is_allowed_to_exceed_screen_height` (asserts the full surface prints —
  scrollback is the stated behavior, spec §2.7). Run → FAIL. Implement `areas_view` over
  `card_render`. Run → PASS.
- [ ] **Step 5.4: Failing tests, plumbing.** Repl: `areas_opens_and_toggles_like_the_other_
  surfaces` (`step("areas")` shows it, second `step("areas")` closes, `view` menu lists
  it); HELP_TEXT mentions `areas`. Run → FAIL. Add the `Areas` arm everywhere the compiler
  and the menu tests point. Run → PASS.
- [ ] **Step 5.5:** Guides: §5 blocks in both formats + a short view section
  (`guide_sync::help_text_commands_are_documented_in_both_guides` enforces the floor).
  Per-edit gate → green.
- [ ] **Step 5.6: Commit** `feat(cli): the areas view — per-area action-payout reference,
  computed from the engine's own reward functions (branch-4 item 3 + owner item 8)`.

---

### Task 6: README + screenshots (spec §2.8)

**Files:**
- Modify: `README.md` (Status section → post-Phase-B two-liner; features list gains the
  banner + `areas` view + orders line; image references)
- Create: `docs/images/cli-frame-touchup.png`, `docs/images/cli-areas.png`,
  `docs/images/cli-banner.png`, `docs/images/cli-why-counters.png` (names final at
  capture; replace stale W5 images where layouts moved, keep `cli-suggest.png` if still
  accurate)

**Process (the a106934 precedent):** drive the built binary in a Forge PTY
(`docs/forge-cli-testing.md` — the recipe, incl. the `spawn-helper` chmod gotcha), capture
the frames (seed recorded), convert the captured ANSI to HTML, render via the chromium MCP
at a monospace font, screenshot to PNG, commit under `docs/images/`. Capture seeds and
commands go in the commit message.

- [ ] **Step 6.1:** Build release-ish binary (`cargo build -p yokohama-cli`), Forge-drive
  a 2P sim to capture: the composed frame with the Orders line; the banner moment (drive
  to a trigger — a church-track fixture save from Task 1's tests can seed a near-trigger
  position, or replay `docs/temp/owner-games/game3.json` to its trigger); the `areas`
  view; a `why` screen with the counter block.
- [ ] **Step 6.2:** Chromium-render the four captures to PNG; verify each PNG opens and is
  legible (Read the file — it renders as an image).
- [ ] **Step 6.3:** README edits: status refresh (short — deep status lives in
  guidingpath), feature bullets, image swaps with honest alt text.
- [ ] **Step 6.4: Commit** `docs: README touchup pass — status refresh + captures of the
  banner, orders line, areas view, and why counters`.

---

### Task 7: Validation wave + close-out

**Files:**
- Modify: `docs/plans/2026-08-09-phase3.5c-execution-ledger.md` (one appended row: the
  branch, its items, its counts — append-only, rebase-friendly)
- Modify: `guidingpath.md` (one appended decision-log row, same discipline)
- Modify: `docs/plans/2026-08-18-cli-branch4-touchup-spec.md` (§6 gains the execution
  outcome line)

- [ ] **Step 7.1: Forge validation of every visual change** (per `docs/forge-cli-testing.md`):
  banner fires loud at a real trigger and NOT on reload (both notice branches driven);
  Orders line at 2P and the 4P tail row; `areas` at the 200×50 floor per-screenful; forced
  `why` lead at one of the owner's two recorded moments (replay
  `docs/temp/owner-games/game1.json` / `game3.json` via `why <n>` — the saves are the
  fixtures of record, spec §3); the marker/dim table on a real suggest/why.
- [ ] **Step 7.2: Full per-edit sweep at branch HEAD:** cli + core + oracle + difftest +
  bridge + eval_probe + clippy. Core/oracle/difftest/bridge must read UNCHANGED
  (1090+29+1 · 191 · 52+5 · 81); any moved count is a stop-and-look.
- [ ] **Step 7.3:** Ledger + guidingpath rows; spec §6 outcome line. Commit
  `docs(cli-touchup): close-out rows — the branch's record`.
- [ ] **Step 7.4:** Report to the owner: per-item evidence, the decisions list (spec §5),
  the held-back push, and the rebase plan (append-only doc conflicts expected at the
  ledger/guidingpath only).

## Self-review record

Spec coverage: §2.1→T1, §2.2/2.3/2.6→T4, §2.4→T2, §2.5→T3, §2.7→T5, §2.8→T6, §3
cross-cuts→global constraints + per-task gates, §4 model→header + task order, §5/§6→T7.
Item 8's option survey is spec-side (no task — it is a record, already written). No
placeholder steps; interfaces name exact functions/paths; the two known risky couplings
(height pins at T3, guide quotes at T4/T5) are called inside their tasks.
