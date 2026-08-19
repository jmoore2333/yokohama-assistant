# CLI branch-4 touchup — spec

**Date 2026-08-18 · branch `cli-branch4-touchup` off master `3021dc5` · LOCAL COMMITS ONLY, no
push and no PR until the concurrent Phase-C lane lands (owner directive; we rebase onto their
work when it does).** Engine crates untouched by fence: every item here lives in
`crates/yokohama-cli` plus the doc surfaces. `SEMANTICS_REV` 7 does not move.

## 0. Provenance and scope

The work list is the **branch-4 CLI backlog** — routed non-blocking at the D7 triage and
carried into Phase C — plus two owner additions from the 2026-08-18 direction message:

| # | Item | Source |
|---|------|--------|
| 1 | Forced-move `why` leads with "only legal option — nothing was weighed" | owner-games handover §5.1 |
| 2 | `you`/`best opp` first-contact aid: surface nonzero-product rows | handover §5.2 |
| 3 | Browsable per-area action-payout reference (`areas` view) | handover §5.3 |
| 4 | Lab "acquire X, +2 yen" reads as a gain; it is the slot surcharge PAID | handover §5.4 |
| 5 | End-trigger firing gets a loud one-time banner | handover §5.5 |
| 6 | `why` surfaces the full SearchStats counter line (2 of 15 shown today) | D7 triage §2, orchestrator candidate |
| 7 | Players panel shows orders FILLED per player (count + card indexes) | owner 2026-08-18 (post-review feedback) |
| 8 | Enhanced-board readability exploration (tiles/rewards legibility at scale) | owner 2026-08-18 — **explored in §2.7's option survey; recommendation: the `areas` view carries it** |
| 9 | README pass: refresh status + re-capture screenshots at this build | owner 2026-08-18 |

References: `docs/plans/2026-08-17-phase-b-owner-games-handover.md` §5,
`docs/plans/2026-08-17-phase-b-d7-triage.md` §2 (~line 155), guidingpath's 2026-08-17/18 rows,
the execution ledger's PHASE B ITEM-3 row, the close handover §4's carry line.

**A verification that settles an owner question (recorded here so it reaches the PR):**
`suggest` is NOT stale. `suggest.rs:326` builds `LookaheadAgent::new(rules, stream_seed)` —
constructor defaults — and `bot.rs:325` documents that construction as unified with
`PROVISIONAL_DEFAULT_BOT = Contestant::LOOKAHEAD_V1_DEFAULT` (the W5 re-point; both ride
`WeightTable::V2` through the shared evaluator since the freeze). What IS true: only `why`
carries the every-move reconstruction pin, so `why` is the *vetted* surface. Consequence for
item 2: the explain-table change lands in the shared `suggest.rs` table machinery (which `why`
renders through), with the new tests written against **`why`'s** output path; `suggest`
inherits the improvement without becoming a separately-vetted surface.

## 1. Non-goals

- **Dynamic in-frame tile scaling** (growing the composed frame's board tiles with terminal
  size). Touches every frame consumer, the guide width pins, and Forge legibility; the
  readability ask is served by §2.7's full-screen surface instead — see §2.7's option
  survey, which records what scaling would cost and why it is future work, so the deferred
  PR review adjudicates an explored decision rather than a bare merge.
- No `suggest` rework, no live-mode bots, no difficulty levels (v1 fences stand).
- No engine-crate edits. If an item turns out to need one, STOP and re-scope — that is a
  hidden-complexity upgrade, not a quiet widening.
- No push, no PR, no gh operations. The 15-gate release roster is NOT run per-edit; it runs
  once at the (deferred) merge point, per the two-tier policy.

## 2. Item specs

### 2.1 End-trigger banner (item 5) — the headline

Current state: the only in-flow evidence is one delta-window line
`END TRIGGERED: {:?}` (`render.rs:659-661`, Debug formatting) and the quiet
`· FINAL ROUND` strip suffix (`render.rs:400-402`). The owner missed both live.

Spec:

- **A loud one-time banner** renders in the message zone when `end_game_triggered` flips
  false→true across an applied event, regardless of actor (the human's move, a bot-run move,
  a live-entered move, a choice resolution). Shape: a full-width bordered block, `Tone::Warn`
  on TTY, e.g.

  ```
  ══════════════════════════════════════════════════════════════════
    END TRIGGERED — the Church track filled. The current round
    finishes, then every player gets one full final round, then the
    game ends and final scoring runs.
  ══════════════════════════════════════════════════════════════════
  ```

  **The end-rule clause above states the rule the engine actually implements**
  (`apply_move.rs:252-289`, rulebook p.9: the trigger round completes first, THEN one full
  final round — equal turns for all; the engine tracks the two phases as the separate bools
  `end_game_triggered` and `final_round`). The implementer still cross-checks the KB before
  freezing the string, but the shape above is the verified one — and "dealer" is not this
  game's vocabulary (the engine's round boundary is `config.start_player`). The reason
  clause comes from a new words-not-Debug mapping over `trigger_reason` (each variant gets
  rulebook words; the deck reason states the deck-empty + unfillable-Port-slot conjunction
  honestly). The border rule's U+2550 `═` is NOT currently in the vocabulary — T3 REMOVED the
  character from the crate entirely — so the banner ADDS it as a NEW named glyph via the
  `vocabulary!` macro (with the module-doc census updated), never spelled inline, per the
  closed-vocabulary rule. (The plan-review correction of this spec's earlier
  already-in-vocabulary claim; the plan's Task 1 carries the mechanics.)
- **Bot-run flips surface after the run**: when the flip happens inside a bot's turn, the
  banner appears with the bot-run summary (and in the load-time TURN REVIEW screen if the
  run is being replayed there), not silently swallowed.
- **Transition-fired semantics**: the banner fires on every false→true flip observed
  in-session — which makes the `undo` interaction principled rather than accidental: an
  undo that unwinds the trigger re-arms it, and a later line that re-triggers (possibly for
  a DIFFERENT reason) fires a fresh banner, because that is genuinely news. No sticky
  once-per-session flag; nothing persists to the save format.
- **A save reloaded after the trigger** does not re-fire the banner; the load path prints a
  one-line notice in the message zone once, **branched on `state.final_round`** (both
  reachable save states are real): `END TRIGGERED — <reason>; finishing the current round`
  when `final_round` is still false, `FINAL ROUND in progress — <reason>` once it is true.
- **The delta line is reworded** from `END TRIGGERED: {:?}` to the same words the banner
  uses (short form), so the log's historical fold reads honestly too — and it still speaks
  on a post-undo re-trigger, banner or no banner.
- Piped runs print the banner as plain text (the piped-plain byte-identity property is
  TTY-color-stripping, not content suppression). The piped-dogfood pins that assert output
  shapes get updated alongside, deliberately, in the same commit.

### 2.2 Forced-move `why` lead (item 1)

Detection: **`get_legal_moves(&state, rules).len() == 1` at the reconstructed decision
state** — an engine-authority-honest statement about the POSITION, which is what the
sentence claims. (NOT the roots-table length: the greedy arm's `ranking` is `Vec::new()` by
construction and its `stats` is `None` (bot.rs:453-461), so no search artifact can carry
this fact on both arms; the legal list can, and `why` already reconstructs the state it
would be called on.) When it fires, the account LEADS with a first line before any table:

> `Only legal option — nothing was weighed: the position offered exactly one move.`

The runner-ups block is omitted for that case (there are none), the explain table stays (it
is still the truthful account of what the position was worth), and the line renders for both
contestant kinds that reach `why` (lookahead and the grandfathered greedy path's
ONE_PLY_ACCOUNT variant — the lead line composes with, not replaces, that sentence).

### 2.3 Nonzero-product row aid (item 2)

In the shared explain table (`suggest.rs`, headings `feature · you · best opp · weight ·
product`): **canonical row order does not move** (cross-referencing against `eval::explain`
and older screenshots must keep working). Instead:

- Rows with a nonzero `product` get a leading marker — **a second glyph-vocabulary name for
  U+25B6 (e.g. `glyph::NONZERO_MARK`), added per the `SEPARATOR`/`EMPTY_SLOT` precedent**
  (style.rs:103-108: a call site reads better naming which JOB it means; the code point is
  already vocabulary as `glyph::ACTIVE`, whose Ambiguous-width acceptance rationale at
  style.rs:110-131 is the alignment justification — cite it, don't re-argue it; the
  shared-code-point pin `the_glyph_table_records_every_constant_truthfully` moves
  DELIBERATELY to sanction the second pair, per the plan's Task 4). Present on
  TTY and piped alike, so the two stay byte-identical modulo color; zero-product rows get
  the same-width blank.
- Zero-product rows render `Tone::Dim` on TTY.
- A summary line under the table: `N of M features carry a nonzero product — they are this
  move's whole story.` (exact wording fixed at implementation; the count is computed, never
  hand-maintained).
- The TOTAL row is exempt from the marker column's semantics (it is a sum, not a story row).

Tests target `why`'s rendering path (§0's vetting note). `guide_sync` quotes of the table
headings/labels update in the same commit if any quoted string moves.

### 2.4 Lab slot-surcharge reword (item 4)

`describe.rs:632-637`: `", +{n} yen"` / `", +{n} imported"` become one composed fee clause —
`", slot fee: {n} yen"` / `", slot fee: {n} imported good[s]"` (plurals via the repo's
existing `plural()` helper, never a `(s)` parenthetical), and when BOTH fields are nonzero a
single clause `", slot fee: {a} yen + {b} imported good[s]"` (never `slot fee:` twice on one
line) — the same fee grammar the 4th+ clause two
lines down already uses (`", 4th+ tech fee: 2 yen"`), so one describe line can carry both
clauses without reading as a gain anywhere. Every fixture/test string pinning the old
wording moves in the same commit; the known guide pin is the §5 block at `cli-guide.md:287`
/ `.html:244`, and any quoted previews update with it.

### 2.5 Players-panel orders line (item 7)

Engine already exposes everything needed: `Player::completed_orders: Vec<OrderCardId>`
(public, also on `PlayerView`). Spec:

- **2P layout** (full panel): a dedicated `Orders` line joins the block next to `Tech`:
  `Orders  2 in hand · 3 filled (O23, O08, O12)` — filled codes in **completion order**
  (the Vec's order IS the chronology; the owner's example was not sorted, and chronology is
  the informative reading), `O{:02}` format matching every other order-code site. Zero
  filled renders `0 filled` with no parenthetical. The in-hand count MOVES here from the
  President line (which keeps location only — one fact, one home); for a hidden-hand bot
  seat the in-hand count keeps its existing `(hidden)` treatment on the new line, and the
  filled clause is always public.
- **3-4P compressed layout: counts only, by measured necessity.** The tail row is the
  column layout's tightest — 47 columns against 48 at 4P, swept and pinned
  (`the_widest_legal_seat_tail_row_still_fits_its_column`), with its own doc recording that
  the budget for even one added word was zero — and `SEAT_ROWS = 10` is fixed with the 3P
  frame exactly on the 50-row floor, so no new row exists either. The design is therefore:
  the tail row carries `N filled` (never the codes parenthetical, at any width), the
  worst-case elision order is RE-DERIVED across the now-four facts and the sweep pin
  re-measured, and **the codes half of the owner's ask is recorded as under-delivered at
  3-4P by this measurement** — the 2P layout delivers the owner's example in full, and the
  owner adjudicates the residue at PR review.
- Live mode: filled orders are public knowledge (fulfilment is a public act with the card
  face up), so the line renders identically for live-tracked opponents.

### 2.6 `why` full counter line (item 6)

Replace the two-counter "Search cost" note (`why.rs:453-461`) with the full 15-counter
surface, **rendered FROM `driven_runner::counter_rows(&SearchStats) -> [(&str, u64); 15]`**
— the existing public surface whose no-`..` destructure makes a sixteenth counter a COMPILE
error in core, built precisely because "why printing 2 of 15 is exactly the gap this table
exists to close" (its own doc). The `why` renderer iterates `counter_rows`' names through a
display grouping whose unknown-name arm is an error, and a pin asserts the rendered block
mentions all 15 names — so core growth breaks `counter_rows`' compile, which grows the
array, which trips the unknown-name arm here: the totality chain has no silent link.
**`counter_rows`' names are the vocabulary, verbatim** (one vocabulary across the
instruments); the grouping is display-only, e.g. a search line and a budget line. Rendered
only where stats exist (the lookahead path), unchanged refusal behavior elsewhere. D7's
mandatory step-one set (binds/starved/ply/free-cap) must all be present and named — that
absence is the defect this item fixes.

### 2.7 The `areas` view (items 3+8 merged)

A seventh `ViewSurface` (`areas`), following the `ports`/`labs`/`sites` full-screen
card-surface pattern over `card_render`, opened/closed by the `areas` command and listed in
the `view` menu and `HELP_TEXT`.

Content — one rich card per area, **grouped in board rows** (by `grid_position`, the same
no-assumed-shape rule `render::board_grid` follows), each card carrying:

1. Header: `A07 SILK MILL` + area type.
2. **The action-payout ladder**: what the action yields at each power tier, spelled out in
   words (the reference the owner asked for — previews only price *offered* actions today).
   **Ladders are COMPUTED by calling the engine's exported per-power reward functions and
   rules data wherever a callable source exists** — the engine already exports them and the
   CLI already imports several (`production.rs`: `copper_mine_reward` / `silk_mill_reward` /
   `tea_plantation_reward` / `fishing_ground_reward(power)`; `commercial.rs`: `bank_reward`,
   `chinatown_rate`; `describe.rs:605-617` already computes the Bank preview by calling
   `bank_reward` per power — that is the pattern). The plan inventories every callable
   source at planning time. Hand-worded lines are permitted ONLY where no callable exists
   (prose glue; any 5-Power printed bonus rules data does not carry), and each such line
   carries a pin against apply behavior — the pin language applies to that residue only.
3. Site slots with FULL printed rewards in words (the compressed `✱1 ✱2¥1` glyphs the owner
   found illegible expand to the `sites`-view vocabulary), built-owner marks.
4. Current occupancy: president/assistants/Station, matching the live tiles' facts.
5. Special-area cards (Church/Customs/ports/labs/Employment/Chinatown/Stock/University etc.)
   state their action in words and point at their dedicated view where one exists
   (`tracks`, `ports`, `labs`) rather than duplicating those surfaces.

Layout: adaptive to the terminal via the existing full-screen-view width machinery (the
card index already uses the real terminal width, `screen.rs:187`); at the 200-column floor
the cards render in board-row groups, **chunking a board row into successive card rows when
its cards cannot share the width at full words — reward words are never truncated to force
a fit**; a wider terminal gets wider or more-per-row cards. **Height: the surface will
exceed 50 rows at 3-4P (10–18 areas of rich cards) and is ALLOWED to — it prints in full
and relies on terminal scrollback, the existing full-screen-view behavior, stated here
rather than assumed. Forge validation proves PER-SCREENFUL legibility at the floor size**
(the buffer is read in screen-sized slices), not whole-surface capture.

**The option survey behind this design (item 8 — the owner asked for an exploration, so
the explored options are on record):**

- **(a) Full-screen reference view + width-adaptive cards — CHOSEN.** Zero risk to the
  composed frame (no board_grid/tile-grammar edits, no guide width-pin churn), serves both
  the payout-reference ask and the tiles/rewards readability ask, and scales with whatever
  the terminal gives it — which is the only scaling a terminal app honestly controls (font
  size belongs to the terminal emulator).
- **(b) STEPPED frame tiers — the owner's own refinement (2026-08-18): "View Medium" (the
  current 200×50-floor frame) and "View Large" (a second, richer fixed layout the frame
  upgrades to when the terminal offers the room — e.g. shrunk text or a fullscreen window,
  where a 2P frame today leaves most of the screen open).** Materially more tractable than
  continuous scaling (two fixed layouts, not a continuum) — but still a second full layout
  for the LIVE frame, the most-pinned surface in the repo (frame widths/heights,
  `guide_sync` §2, panel join geometry, mockgen, Forge captures — all single-tier), so it
  is roughly an areas-view-sized project layered onto the riskiest existing code.
  **DEFERRED to its own session by orchestrator call, owner-accepted in-session**, on
  three grounds: this branch's brief is the small CLI PR; a Large frame reads WORSE
  through the Forge MCP (the floor partly exists for agent legibility, the owner's own
  concern); and the owner's stated endgame — a web/Tauri frontend over the API-friendly
  backend once 3.5 progresses — owns rich display, leaving the CLI's value concentrated
  in TTY/agent testing. The revisit is cheap: `screen::init_from_tty` already reads the
  terminal size, so the future PR is pure layout work, no plumbing. Pick up from this
  paragraph.
- **(c) Single-area zoom command** (`area A07` prints one rich card inline): cheap, but it
  is a lookup, not the browsable reference the owner described, and (a) subsumes it.

The owner saw this survey and refined it in-session (the (b) rewrite above); (a) proceeds
now and — at the owner's shrunk-text resolution — already uses the freed width for bigger
cards, the cheapest real taste of Large. The PR review adjudicates the record either way.

### 2.8 README pass (item 9)

- Status section refreshed to the post-Phase-B state (short — the deep status lives in
  guidingpath; README stays the GitHub-optimized front door).
- Screenshots re-captured at this branch's build via the established pipeline (Forge PTY
  capture → chromium render → `docs/images/*.png`, the a106934 precedent): the composed 2P
  frame (now with the Orders line), the `areas` view, the end-trigger banner moment, and a
  `why` account with the full counter block. Old images replaced where they show stale
  layouts, kept where still accurate. Capture seeds/commands recorded in the commit message.
- The features list mentions the new view and the banner where the README already enumerates
  CLI capabilities.

## 3. Cross-cutting constraints (all items)

- **Guide pair**: every user-visible change updates `docs/cli-guide.md` + `.html`
  word-identically in the SAME commit; `guide_sync` (18 tests) is the enforcement floor, not
  the ceiling — prose sections describing changed behavior update too.
- **Honesty properties held**: engine as sole authority (no CLI-side rule math beyond
  rendering engine facts — the payout ladder derives from engine/rules data), piped output
  byte-identical plain text, `NO_COLOR` honored, no width regressions (`screen::visible_width`
  guards; new surfaces measure through it). **Height pins are re-measured deliberately, never
  incidentally**: any panel line added at 2P moves `WALK_HEIGHTS` (screen.rs:410) and may
  press `FRAME_HEIGHTS` (screen.rs:344, the 3P frame sits ON the 50-row floor) — each such
  move is named in its commit message with the new measurement.
- **Glyphs come from the closed vocabulary** (style.rs): new jobs get NAMES (second names
  for existing code points where that is the fit, per the `SEPARATOR`/`EMPTY_SLOT`
  precedent); no surface spells a non-ASCII glyph inline.
- **TDD**: failing test first, per item, in the same red-green discipline the repo's history
  keeps. The cli lib wall is ~23s (the `why` every-move pin) — budget it per edit.
- **Per-edit suites after every task**: `cargo test -p yokohama-cli` + `cargo clippy
  --workspace --all-targets -- -D warnings`. Core/oracle/difftest/bridge run once at branch
  close (nothing here should move them; a moved count is a stop-and-look signal).
- The owner-games saves (`docs/temp/owner-games/game1.json`, `game3.json`, the notes) are
  the validation fixtures of record: the banner and forced-`why` items get re-driven at the
  actual events the owner flagged (game 3's end-trigger moment; the two forced-move `why`
  hunts) once implemented.

## 4. Execution model

- Branch `cli-branch4-touchup` off master `3021dc5`. Local commits only.
- Task order (shared-file contention says sequential): **T1** banner (§2.1) → **T2**
  surcharge reword (§2.4) → **T3** orders line (§2.5) → **T4** `why` lead + counters +
  table aid (§2.2/2.3/2.6) → **T5** `areas` view (§2.7) → **T6** Forge validation wave +
  screenshots + README (§2.8) + guide prose pass + ledger/guidingpath rows.
- Dispatch per the owner's rulings (standing + the 2026-08-18 refinement): **Opus
  implementers (xhigh), a Fable review stage per task**, the orchestrator adjudicating.
- Document reviews are SINGLE-reviewer: this spec and the implementation plan each get one
  **Fable max review** before execution, findings adjudicated by the orchestrator (the
  owner re-scoped the original opus+fable pair mid-run — pairs are for implementation
  waves, not document reviews).

## 5. Decisions taken here for owner review at PR time

1. Item 8 explored via §2.7's option survey and RESOLVED in-session: the owner refined the
   future direction to STEPPED tiers (View Medium / View Large, §2.7 option (b)) and
   accepted its deferral to a dedicated session; the `areas` view (a) carries the
   readability ask in this PR.
2. Banner semantics: transition-fired (every observed false→true flip — so an undo that
   unwinds the trigger re-arms it and a re-trigger fires fresh, possibly with a different
   reason); a reloaded save gets a one-line notice branched on `state.final_round`, no
   re-fire (§2.1).
3. Filled-order codes render in completion order, not sorted (§2.5).
4. The in-hand order count moves from the President line to the new Orders line at 2P; the
   3-4P tail row carries `N filled` counts only — the codes half is under-delivered at 3-4P
   by the measured 47/48-column budget, recorded for owner adjudication (§2.5).
5. Explain-table row order stays canonical; the aid is a vocabulary-named marker + dim +
   summary count (§2.3).
6. `suggest` verified current; item-2 tests target `why` (§0).
7. Forced-move detection is `get_legal_moves(...).len() == 1` at the reconstructed position,
   not a search artifact — valid on both contestant arms (§2.2).
8. The `why` counter block renders from `driven_runner::counter_rows`, names verbatim — the
   totality chain (core compile guard → array growth → unknown-name arm) has no silent link
   (§2.6).
9. The 4P seat tail row's four-fact worst case overflows its measured 47/48-column budget,
   so a yield policy exists: `next take ¥N` drops FIRST and alone at the swept corner (its
   information is recoverable at the Employment Agency decision preview) — a CONDITIONAL
   re-open of the T10/T11 keep verdict on that fact, for owner adjudication (§2.5, plan
   Task 3.4).
10. The 2P Orders line's codes parenthetical elides with the vocabulary ellipsis when it
    would overflow the line budget — codes drop from the end, the paren always closes, the
    `N filled` count stays exact (§2.5, plan Task 3.1).
11. **The 2P opening frame crossed the 200×50 design floor: 49 → 51 rows (`FLOOR_H + 1`,
    pinned EXACT so both further growth and a future return inside fail loudly).** The
    cost is inherent to decision 4's dedicated Orders line (two seats, one row each).
    `FLOOR_H` itself is unmoved; the 3P/4P openings stay inside it; guide §2 no longer
    claims a 200×50 window shows the whole opening at every count. Mitigating fact,
    measured: the 2P in-play walk already reached 63 rows before this change (65 after) —
    the dashboard grows a row per owned tech from the second onward — so mid-game 2P
    never fit 50 rows anyway. T3 implementer deviation 2 + review finding 1; the owner
    adjudicates whether the row is worth the crossing (the alternatives: compress another
    2P panel line, or fold orders back onto the President line).
12. T4's three recorded deviations, adjudicated shipped-shape-over-spec-letter: the counter
    grouping's unknown-name arm renders a labelled defect line ("Counters this build does
    not group (please report):") rather than erroring — the totality purpose is fully
    served by the all-15-names pin plus core's compile guard, and a `why` screen that
    panics on future counter growth is worse than one that names the gap; the summary
    counts "rows" not "features" (the terminal `final standing` row is a story row —
    without it a finished game would read "0 of 16" over a table whose whole content sat
    in the skipped row); the second counter line is labelled "Budget and guards:" so D7's
    mandatory step-one set lands on one line. The dated owner-walkthrough capture in
    docs/temp/ keeps the old two-counter line as the historical record it is.
13. **The strip suffix contradicts the new notice during the trigger round** (T7 validation
    anomaly A): `render::end_trigger_line` appends `· FINAL ROUND` gated on
    `end_game_triggered`, so at `end_game_triggered && !final_round` the frame shows
    `· FINAL ROUND` directly above the precise `END TRIGGERED — <reason>; finishing the
    current round` notice. Pre-existing and pinned; branch 4 did not cause it but its
    precise wording is what makes the shorthand read as a contradiction. UNCHANGED on this
    branch — owner call. Recommendation: a follow-up gates the suffix (`· END TRIGGERED`
    during the trigger round, `· FINAL ROUND` once `final_round` sets), moving the strip
    pin and the guide sentence with it.

## 6. Review trail

- 2026-08-18: single-Fable max review (owner re-scoped the pair mid-run: document reviews
  get one reviewer; the opus/fable split is for implementation waves). Verdict FIXABLE — 3
  MUST, 3 SHOULD, 4 NOTE, all adopted; the MUSTs: the banner's end-rule wording corrected to
  the engine's verified two-phase rule with the load notice branched on `final_round`; the
  3-4P orders clause redesigned counts-only against the measured zero-budget tail row; the
  forced-move detection re-sourced to the legal list. Adjudicated by the orchestrator; this
  revision is the amended spec.
- 2026-08-19: **EXECUTION COMPLETE — the branch is ready and HELD, unpushed, awaiting the
  Phase-C lane.** T1–T5 each ran opus-implement (xhigh) → fable-review (max) → orchestrator
  adjudication, with fix waves where findings were real: T1 `6d0e025`+`9b3606c` (banner;
  review PASS_WITH_NOTES, undo-drain + GAME-OVER-notice fixes) · T2 `82aac18` (surcharge;
  PASS_WITH_NOTES, nothing to fix) · T3 `613c5d3`+`fd5f902` (orders line; PASS_WITH_NOTES,
  growth-mechanism doc + rename) · T4 `a3a8fca`+`aa704fb` (why trio; PASS_WITH_NOTES,
  decision 12) · T5 `f001783`+`ddb5815` (areas view; FIXABLE — the footer/gloss prose
  transcription findings, all fixed with computed/pinned figures) · T6 `bf422a1` (README +
  four chromium-rendered captures, the banner shot from a REAL played transition) · T7
  validation (Forge): forced-`why` at the owner's own e28/e42 anchors, all THREE
  load-notice arms witnessed live, 4P tail row, 18/18 areas at the floor with zero
  over-width lines, the one-time banner property verified over a whole session buffer;
  anomaly B ("1 imported good") fixed on-branch, anomaly A recorded as decision 13.
  Cross-suite sweep at HEAD: core 1090+29+1 · oracle 191 · difftest 52+5 · bridge 81 ·
  eval_probe 36 · node 4/4 · clippy clean — every non-cli count UNCHANGED; cli grew
  743→796 lib (+2 dogfood ignores unchanged). The 15-gate release roster is deliberately
  NOT run here — it runs at the (deferred) merge close per the two-tier policy.
- 2026-08-18: single-Fable max review of the PLAN. Verdict FIXABLE — 1 MUST (the
  `NONZERO_MARK` addition trips the only-one-shared-code-point glyph pin; whitelisted
  deliberately), 4 SHOULD (U+2550 is GONE from the crate, not converted — this spec's
  §2.1 corrected with it; the 4P yield policy and 2P ellipsis treatment decided, decisions
  9-10 above; a greedy-composition test added), 4 NOTE (why-side gating vs suggest's own
  forced-point pin; `apply_standalone` cannot flip the trigger; anchor precision;
  baseline independently reproduced 743/9/18/1 at clean master). All adopted; the plan is
  amended in place. Execution proceeds under the owner's dispatch: Opus implementers
  (xhigh), Fable review stage per task, orchestrator adjudicating.
