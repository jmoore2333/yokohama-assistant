//! **Scenario suite** (Phase 3.3 T9; spec §6, admitted under §0.2) — nineteen
//! hand-built rules situations the evaluator rung has to get right, each one a
//! single falsifiable property with a rules citation, a WHY line, and a
//! demonstrated failing counterpart.
//!
//! # The waves, and which table each one is written against
//!
//! **Scenarios 1–9 are suite v1** (Phase 3.3 T9), authored against the frozen
//! [`WeightTable::V1`] — the table the bot plays today. **Scenarios 10–12 are
//! the Phase-3.5c round-1 growth and scenario 13 is round 2's** (protocol
//! `docs/plans/2026-08-09-phase3.5c-evaluator-v2-protocol.md` §4.2 rung L3 and
//! §6's growth list), and they are written against
//! [`WeightTable::V2_DRAFT`] — the ITERATION CANDIDATE that becomes V2 at the
//! freeze commit — because L3's whole job is that *"every adopted change has at
//! least one scenario that lands red-then-green against it"*. A round's
//! scenario is therefore authored BEFORE the weights move, fails against the
//! unmoved draft, and turns green in the adoption commit; its counterfactual is
//! then the round's own edit REVERTED rather than a zeroed term.
//!
//! **Scenarios 14 onward are the W4 GROWTH WAVE** (protocol §6's growth list,
//! landing with the freeze package), and they are a THIRD kind of scenario
//! again: they guard no round's edit, because the iteration loop stopped
//! adopting at round 3. Each one instead pins a property the evaluator is
//! required to have *by the rules* or *by owner canon*, against whichever table
//! that property is a statement about — [`WeightTable::V1`] where the claim
//! holds table-independently (which spares the freeze commit a re-point),
//! [`WeightTable::V2_DRAFT`] where the claim is genuinely about the
//! shipped-candidate's own pricing. Every W4 scenario's doc carries a **WHICH
//! TABLE** section naming its choice and its reason, and a canon-judgment
//! scenario additionally quotes the owner ruling it encodes and names its
//! protocol LABEL (guidingpath's 2026-08-09 canon-batch row; protocol §6's
//! naming amendment). **W4 landed its RULES-DERIVABLE pair first** — D5's dummy
//! in the technology majority, and the deck clock's banking decision, neither of
//! which needs canon — **and then its CANON JUDGMENT band**, one scenario per
//! ruling of the owner's 2026-08-09 batch (labels 11, 12+13, 14, 15). Two of
//! those rulings REVERSED the drafts they were asked about and one INVERTED its
//! draft, so each judgment doc quotes the ruling it encodes verbatim rather than
//! paraphrasing a scenario title that no longer means what it once said.
//!
//! Nothing about scenarios 1–9 changed when 10–12 landed, and no scenario's
//! CODE or property changed when 13 landed or when the W4 wave landed.
//! Scenario 10's counterfactual helper was RE-DOCUMENTED at round 2
//! ([`early_remix_reverted`]) because round 2 moved three of its five slots a
//! second time, so "restore V1" and "undo round 1" stopped being the same
//! sentence — a re-pointing of the doc, which is exactly what the never-delete
//! rule below asks for when the evaluator legitimately moves under a standing
//! scenario.
//!
//! # THE NEVER-DELETE RULE (map §5-3.3, spec §6)
//!
//! **A scenario in this file is never deleted.** It may be re-pointed when the
//! evaluator legitimately changes — a re-priced weight table, a new feature, a
//! different candidate scheme — and the re-pointing rewrites the property and
//! its recorded counterfactual in the same commit. What it may not do is
//! vanish: the whole value of a scenario suite is that it accumulates, so that
//! a rung three revisions from now still answers for the situations this one
//! did. [`SCENARIOS`] is a fixed-length array for exactly that reason —
//! deleting an entry is a compile error, not a quiet diff.
//!
//! # The record shape (spec §6)
//!
//! Every scenario carries, in its own function's doc comment:
//!
//! | field | where |
//! |---|---|
//! | name | [`SCENARIOS`] and the `#[test]` that runs it |
//! | rules citation | the doc's **RULES** line |
//! | builder | the fixture code, from [`base_view`] or a constructed state |
//! | seat | stated in the doc; seat 0 everywhere unless said otherwise |
//! | falsifiable property | the doc's **PROPERTY** line, and the `check` closure |
//! | WHY | the doc's **WHY** line |
//! | authoring proof | the doc's **COUNTERFACTUAL** block |
//!
//! §6 names three property SHAPES and all three are in use: `eval(A) > eval(B)`
//! carries ten scenarios; *the explanation's top term names X* carries
//! country-set completion's second clause and the three re-pricing scenarios'
//! leak clauses (the explanation must attribute the margin to a NAMED set of
//! terms and to nothing else, which doubles as each fixture's leak check); and
//! *chosen move ∈ set* carries the null-placement trap, in its aggregate form —
//! see that scenario's doc for why a count over a segment is the right reading
//! of it under a randomised tie-break.
//!
//! Round 1 adds a FOURTH shape, recorded as an addition rather than folded into
//! the three: scenario 12's subject is a FEATURE FUNCTION, not a weight, so its
//! property is `f(A) > f(B)` over the candidate feature plus an arithmetic
//! identity against the shipped one, and its counterfactual substitutes the
//! shipped function for the candidate instead of mutating a table. That shape
//! exists because the T3 post-claim filter lands ADDITIVELY in this round — the
//! `score`/`explain` call-site switch is the freeze commit's business — so there
//! is no weight for a scenario to move yet, and a scenario that waited for the
//! switch would be the change landing unguarded.
//!
//! **Round 2 adds NO new shape.** Scenario 13 is the first shape with the
//! second's leak clause, exactly like scenarios 10 and 11 — what is new about it
//! is that its two positions are an EXCHANGE of the same table material rather
//! than two different holdings, so its margin is a pure statement about the
//! ORDER of two weights (see its doc's arithmetic).
//!
//! # §0.2's authoring anti-vacuity rule, made executable
//!
//! A scenario is admitted only with a demonstrated failing counterpart under a
//! **targeted minimal mutation of the term or behavior the scenario names** —
//! country-term-zeroed for the country scenario, and so on. One global
//! degenerate ([`WeightTable::ZERO`]) may NOT discharge the rule for every
//! scenario: that would prove only "this can fail", never "this tests what it
//! claims". `policy_diversity`'s mutation-by-policy-swap discipline, applied
//! per scenario.
//!
//! Two things follow, and both are deliberate:
//!
//! 1. **The counterfactual SHIPS.** Each scenario evaluates its property twice
//!    — once under the shipped evaluator and once under its own targeted
//!    mutation — and [`admit`] fails the test if the mutation does NOT break
//!    it, or breaks a clause OTHER than the one the scenario names. A
//!    recorded-only mutation rots the day someone widens a fixture; a shipped
//!    one cannot. The clause check is the second half of "targeted": most
//!    scenarios assert several things (a fixture guard, a control, a second
//!    inequality), and a mutation that tripped one of those instead would
//!    satisfy a bare "the mutated run failed" while proving nothing.
//! 2. **The observed failing line is quoted verbatim in the doc AND printed at
//!    run time.** `cargo test -p yokohama-core --test scenario_suite --
//!    --nocapture` prints every counterfactual's failure line, so the quoted
//!    text is auditable rather than asserted. Where the targeted mutation is a
//!    SOURCE edit rather than a weight (the terminal scenario — §2.6 short-
//!    circuits the weight vector entirely, so no table can reach it), the doc
//!    names the source edit and quotes the line it produced at authoring, and
//!    the shipped counterfactual is the same rejected reading computed locally.
//!
//! # What this suite does NOT cover (T9 review S3) — and what W4 closed
//!
//! Spec §6's category (3), end-trigger timing, names TWO clocks: the church
//! track filling, and the order deck running near-empty so that unbanked cards
//! have to be converted. Suite v1 discharged the category through the CHURCH
//! clock alone, per T4's routing of the church sign-flip question to this
//! suite, and this section recorded the gap in as many words: *"nothing here
//! exercises the deck half"* — scenario 5 set `order_deck_len` and scenario 9
//! left a shortened deck behind, but both used it purely as STAGE-SETTING, a
//! stage proxy the position is dated by, never a decision about banking before
//! the deck runs out.
//!
//! **The W4 wave CLOSES that named target: [`deck_clock_banking_beats_holding`]
//! is the deck half**, and the deck clock is what DATES its two positions and
//! what its margin is a function of, rather than stage-setting a position merely
//! sits inside. Read "decision" there in this file's own sense and no wider: it
//! is an EVAL-ORDERING property over two constructed views (the same seat with
//! the order banked and with it still held), not a driven pick — the one
//! genuinely behavioural scenario here is still the null-placement trap. The gap
//! paragraph is kept rather than deleted, because the discipline it
//! demonstrates — name the target, do not claim the category — is the reason the
//! gap was closable at all.
//!
//! What is still NOT covered, stated in the same spirit: the BUILDING clock
//! (`clock_ppm`'s fourth component) dates no scenario here, and no scenario
//! makes a decision about firing the 8-shophouse or 4-trading-house trigger.
//! Scenario 5 is the nearest thing and its own doc records that it reduces to a
//! weight ordering with no rules arithmetic exercised.
//!
//! # Cost, and why most scenarios drive no game
//!
//! Eighteen of the nineteen are EVAL-ORDERING properties over constructed views
//! or states (seventeen over [`score`], one over a feature function): the
//! evaluator is a pure function of a [`PlayerView`], so specifying its input
//! directly is both the cheapest and the most precise way to state "this
//! position must outrank that one". Only the null-placement trap is genuinely
//! BEHAVIOURAL — "the bot does not spend the game placing nothing" is a
//! statement about picks, not about scores — and it pays for one short driven
//! segment (40 decisions, twice: shipped and mutated).
//!
//! The constructed views are DOCTORED: built by `observe`ing a real game and
//! then overwriting the fields the scenario is about. Several are not states
//! the engine could reach (a hand-set track, a truncated deck), and that is
//! fine and stated — the evaluator's entire input is the view, so a doctored
//! view is a complete specification of an evaluation, and the engine's own
//! reachability is pinned by four other suites. The THREE scenarios whose claim
//! involves the ENGINE's agreement (the D1 dummy's majority, the terminal
//! ranking, and W4's D5 dummy terminal) build real `GameState`s instead and
//! cross-check against `final_result`.
//!
//! # Determinism
//!
//! Every scenario is deterministic. The eighteen eval-ordering ones consume no
//! randomness at all — `score` is a pure function. The driven one fixes both
//! agent seeds ([`DRIVEN_SEEDS`]), which fixes the per-decision sampler seed
//! and the §3.2 tie-break draw, so the segment is byte-identical run to run;
//! it asserts a COUNT (assistants placed > 0), which is additionally robust to
//! any tie-break landing.
//!
//! # Tier and measured cost
//!
//! **Per-edit, never `#[ignore]`d** (spec §0.2/§10.1). Measured at the T9 close,
//! DEBUG, binary run directly, three runs on a quiet Apple M5:
//! **0.05 / 0.05 / 0.05 s** for the whole binary (10 tests: nine scenarios plus
//! the registry pin). RE-MEASURED unchanged at the T9 review close, with the
//! fixture invariant audits that wave added inside the figure. RE-MEASURED
//! AGAIN at the 3.5c round-1 close with the three new scenarios inside it (13
//! tests), and AGAIN at round 2 with the fourteenth (14 tests) — see below for
//! the figures and for why they did not move.
//!
//! The split is the interesting figure and it is why the shapes were chosen
//! this way: the twelve eval-ordering scenarios plus the registry pin run in
//! **0.00 s** (below the timer's resolution — one shared [`rules`] build,
//! per-scenario setup-and-`observe` fixtures, the per-fixture
//! [`check_invariants`] audits, and order-tens of `score` calls), and the
//! null-placement scenario's two 40-decision Greedy-v1 segments are **0.05 s**,
//! i.e. effectively the whole binary. Order-seconds was the budget; the suite
//! came in two orders under it, and the room that buys is for the driven
//! scenarios a later revision will want.
//!
//! **Round 1's three additions do not move the figure** — re-measured at the
//! round-1 close, same machine, same method, 13 tests: a **0.05–0.06 s** band
//! (load-variant in the last digit; the T9 reading sits inside it).
//! All three additions are constructed-view properties with no driven segment,
//! so each costs one setup-and-`observe` plus a handful of `score`/`explain`
//! calls, and the driven scenario is still effectively the whole binary. A
//! future round wanting a BEHAVIOURAL scenario (scenario 5's doc names the one
//! it would most like) still has the whole budget for it.
//!
//! **Round 2's one addition does not move it either** — re-measured at the
//! round-2 change-set commit, same machine, same method, 14 tests: the same
//! **0.05–0.06 s** band. Scenario 13 is a constructed-view property too, and its
//! fixture is one `base_view` plus two clones.
//!
//! **W4's growth re-measures the figure at every commit it makes**, and reports
//! it as a BAND rather than a single reading, because the binary's wall is
//! dominated by one driven scenario and sits at the timer's resolution
//! everywhere else. **The W4 rules-derivable pair, 16 tests: 0.05–0.07 s**, the
//! same band the round-2 close read. Scenario 14 builds two finished
//! `GameState`s and pays two `check_invariants` audits and two `final_result`
//! calls, which is the most expensive non-driven fixture in the file and still
//! invisible against the driven segment. **The canon judgment band, 20 tests:
//! 0.05–0.07 s** — unmoved, and for the same reason: its four scenarios are
//! constructed-view properties costing one `base_view` and a handful of
//! `score`/`explain` calls apiece, against a driven segment that is still
//! effectively the whole binary.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use yokohama_core::game::actions::calculate_church_faith;
use yokohama_core::game::eval::{
    achievement_progress, achievement_progress_postclaim_sum, clock_ppm, explain, initial_order_deck_len,
    interpolate, place_signal, progress_ppm, score, terminal_standing, view_is_terminal, Feature, ScoreStructure,
    WeightTable, HAND_CORE_RESERVE, MAX_SEATS, PROGRESS_FULL_PPM, TERMINAL_MARGIN_CLAMP,
    TERMINAL_MARGIN_STEP, TERMINAL_RANK_STEP,
};
use yokohama_core::game::greedy::{GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use yokohama_core::{
    apply_move_unchecked, check_invariants, final_result, finalize_setup, is_game_over, new_game,
    observe, AchievementCard, AchievementType, ArenaAgent, ChurchSlot, ChurchTrack, Country,
    CustomsSlot, CustomsTrack, EndGameTrigger, GameConfig, GameState, HandCard, Move, OrderCardId,
    PlayerId, PlayerView, Rules, StartingOrderChoice, TechCardId, TurnPhase, VisiblePlayer,
};

// ---------------------------------------------------------------------------
// The registry (the never-delete rule's teeth)
// ---------------------------------------------------------------------------

/// One scenario: its stable name and the function that runs BOTH halves of
/// §0.2's admission — the shipped property and its targeted counterfactual.
type ScenarioFn = fn(&Rules) -> Result<(), String>;

/// **THE NINETEEN SCENARIOS.** Indices 0–8 are suite v1 — spec §6's eight
/// categories plus the terminal-scoring scenario that section adds. Indices
/// 9–11 are the Phase-3.5c round-1 growth, one per adopted change: the early
/// re-mix, the warehouse flatten (protocol scenario 16), and the T3 post-claim
/// filter (protocol scenario 17). Index 12 is round 2's, guarding that round's
/// two dominant re-pricings together (the late church/five-power crossing).
/// Indices 13–14 open the **W4 growth wave** with its RULES-DERIVABLE pair —
/// the D5 dummy deciding a terminal, and the deck clock's banking decision
/// (this file's own named coverage gap, closed). Indices 15–18 are W4's
/// **CANON JUDGMENT band**, one per owner ruling of the 2026-08-09 batch:
/// track-racing (protocol label 11), the customs race with the church side
/// folded in (labels 12 and 13, both re-aimed), achievement proximity as
/// self-guidance (label 14, re-aimed), and the end-trigger brake the ruling
/// INVERTED (label 15).
///
/// **These are REGISTRY INDICES, and the protocol's scenario NUMBERS are
/// tension LABELS** — the two do not line up and are not meant to (protocol §6's
/// 2026-08-09 naming amendment: this array is dense, the labels are not). Each
/// judgment scenario's doc names the label it answers for.
///
/// The length is pinned by the type. Adding a twentieth scenario is a
/// deliberate edit to this array and its `#[test]`; removing one of these
/// nineteen is the thing the never-delete rule forbids, and it will not compile.
const SCENARIOS: [(&str, ScenarioFn); 19] = [
    ("country-set completion", country_set_completion),
    ("order economics: fulfill now vs hold", order_economics_fulfill_now_vs_hold),
    ("end-trigger timing: the church clock", end_trigger_timing_church_clock),
    ("church/customs majority: the D1 dummy", customs_majority_with_the_d1_dummy),
    ("building race", building_race_trading_house_over_shophouse),
    ("retreat/recovery discipline", recovery_discipline_stops_at_the_reserve),
    ("faith arithmetic", faith_arithmetic_buys_the_rightmost_church_space),
    ("the null-placement trap", the_null_placement_trap),
    ("terminal scoring", terminal_scoring_prefers_the_bonus_win),
    ("early engine over early VP", early_engine_over_early_vp),
    ("warehouse decay flattened", warehouse_decay_flattened),
    ("post-claim achievement progress", post_claim_achievement_progress),
    ("late bonus re-pricing: tokens over church", late_bonus_repricing_tokens_over_church),
    ("D5 dummy decides the technology majority", d5_dummy_decides_the_technology_majority),
    ("deck clock: bank before the deck runs out", deck_clock_banking_beats_holding),
    ("track-racing: extending a winnable church race", track_racing_extends_a_winnable_church_race),
    ("customs race: the rival's progress", customs_race_and_the_rivals_progress),
    ("achievement proximity as self-guidance", achievement_proximity_as_self_guidance),
    ("behind at the end trigger: keep building", behind_at_the_end_trigger_keeps_building),
];

/// Spec §6's never-delete rule, plus the registry hygiene the nineteen
/// `#[test]`s lean on: each test names both its index and its scenario name, so
/// a reordering that silently re-pointed a test at a different scenario fails
/// here and there.
#[test]
fn the_registry_holds_nineteen_uniquely_named_scenarios() {
    assert_eq!(
        SCENARIOS.len(),
        19,
        "spec §6's eight categories plus the terminal-scoring scenario (suite v1's nine), \
         plus the Phase-3.5c round-1 growth's three, round 2's one, and the W4 growth wave's \
         rules-derivable pair and canon judgment band; scenarios are never deleted (this file's \
         header states the rule)",
    );
    let mut names: Vec<&str> = SCENARIOS.iter().map(|(n, _)| *n).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(names.len(), total, "two scenarios share a name");
}

// ---------------------------------------------------------------------------
// §0.2's admission helper
// ---------------------------------------------------------------------------

/// Admit a scenario: the property must HOLD as shipped, and must FAIL under the
/// named targeted mutation **on the clause the scenario is about**.
///
/// `expected` is a distinctive fragment of the failure line each scenario's
/// **COUNTERFACTUAL** block quotes, and matching it is what makes §0.2's
/// admission a statement about the TARGET rather than about any breakage at
/// all (T9 review S2). Most scenarios check more than one clause — a fixture
/// guard, a control, a second inequality — and a mutation that tripped one of
/// those instead would have satisfied a bare "the mutated run returned Err"
/// while proving nothing about the term the doc names. It would also let the
/// quoted line silently rot: the fragment is lifted from that quote, so a
/// mutation whose failure text moved fails here rather than leaving the doc
/// describing a run that no longer happens.
///
/// The counterfactual's failure line is printed rather than swallowed, so the
/// text quoted in each scenario's doc comment can be re-checked with
/// `cargo test -p yokohama-core --test scenario_suite -- --nocapture` instead
/// of being taken on trust.
fn admit(
    name: &str,
    counterfactual: &str,
    expected: &str,
    shipped: Result<(), String>,
    mutated: Result<(), String>,
) -> Result<(), String> {
    shipped.map_err(|line| format!("[{name}] the SHIPPED evaluator failed the scenario: {line}"))?;
    match mutated {
        Ok(()) => Err(format!(
            "[{name}] VACUOUS: the property still holds under its targeted mutation \
             ({counterfactual}). Spec §0.2 admits a scenario only with a demonstrated \
             failing counterpart under a mutation of the term the scenario NAMES — \
             either the fixture stopped exercising that term, or the property is \
             about something else now."
        )),
        Err(line) if !line.contains(expected) => Err(format!(
            "[{name}] MIS-TARGETED: the mutation ({counterfactual}) did break something, but \
             not the clause this scenario names — the failure was expected to contain \
             `{expected}` and read: {line}"
        )),
        Err(line) => {
            println!("[{name}] counterfactual ({counterfactual}) failed as required: {line}");
            Ok(())
        }
    }
}

/// [`WeightTable::V1`] with ONE feature's pair zeroed — §0.2's targeted
/// mutation, expressed as data so no scenario needs a second code path.
fn zeroed(feature: Feature) -> WeightTable {
    let mut w = WeightTable::V1;
    w.set(feature, 0, 0);
    w
}

/// `Err` carrying `msg` unless `cond`. The scenarios' whole vocabulary: a
/// property that returns `Result` can be evaluated twice — once shipped, once
/// mutated — from one piece of code, which is what makes the counterfactual a
/// test rather than a comment.
fn require(cond: bool, msg: String) -> Result<(), String> {
    if cond {
        // The message carries the scenario's own figures, so echoing it on
        // SUCCESS is what makes each shipped margin auditable rather than
        // quoted-on-trust: `--nocapture` prints every property that held and
        // the numbers it held by. Invisible under a plain `cargo test`.
        println!("  held: {msg}");
        Ok(())
    } else {
        Err(msg)
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The standard registries, built once for the whole binary.
fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(Rules::standard)
}

/// A finalized game with every seat keeping its first dealt Order, so every
/// seat is viewable (`observe` refuses a seat whose own deal is unrecorded).
fn keep_game(players: u8, seed: u64, rules: &Rules) -> GameState {
    let pending = new_game(GameConfig::for_player_count(players), rules, seed)
        .expect("setup must succeed at a printed player count");
    let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
    let state = finalize_setup(pending, &choices).expect("finalize");
    check_invariants(&state, rules).expect("a finalized setup is a state the engine accepts");
    state
}

/// Every constructed `GameState` in this file goes through the engine's own
/// invariant net before anything reads it (T9 review M2).
///
/// The doctored states are hand-built OUTSIDE `new_game`, which is exactly the
/// construction `check_invariants` was widened to audit — its I9 and I11
/// clauses say so in as many words ("Phase 3 constructs states outside
/// `new_game`, so this moves the guarantee into the ... net"). A fixture that
/// files cards into one pile without taking them out of another, or that
/// vacates a track dummy, is not a state the engine would ever hand an
/// evaluator, so an ordering measured on it says nothing about the rung. The
/// terminal fixtures below did precisely that at authoring: five country cards
/// written into a completed pile while still sitting in the deck and the boxed
/// pile, for a 41-card 36-card game.
///
/// Doctored VIEWS are deliberately NOT covered by this and cannot be — a
/// `PlayerView` is the evaluator's whole input, so specifying one directly is a
/// complete specification of an evaluation (this file's header states the
/// reasoning, and [`recovery_discipline_stops_at_the_reserve`] states the one
/// place a view's arithmetic departs from a reachable state's). A `GameState`
/// is the engine's own object and has to obey the engine's own rules.
fn engine_valid(state: &GameState, rules: &Rules, label: &str) -> Result<(), String> {
    check_invariants(state, rules)
        .map_err(|e| format!("fixture '{label}' is not a state the engine would accept: {e}"))
}

/// Take one Order card out of whichever pile the setup deal left it in, so a
/// fixture may file it under a `completed_orders` pile without minting a 37th
/// card.
///
/// At the seed the terminal scenario uses, three of its five country cards sit
/// in the order deck and two in the boxed `orders_removed` pile; the port arm
/// covers the third place a card can legitimately be. The two error arms are
/// the point of writing this as a function rather than a `retain`: a deal that
/// moved onto the country set is a fixture repair, and raiding a HAND to make
/// room would strand that seat's `starting_order` (invariant I11) or make the
/// seat unobservable.
fn take_order_out_of_play(state: &mut GameState, id: OrderCardId) -> Result<(), String> {
    if let Some(i) = state.order_deck.iter().position(|c| *c == id) {
        state.order_deck.remove(i);
        return Ok(());
    }
    if let Some(i) = state.orders_removed.iter().position(|c| *c == id) {
        state.orders_removed.remove(i);
        return Ok(());
    }
    let ports =
        std::iter::once(&mut state.port_state.port_a).chain(state.port_state.port_b.as_mut());
    for slot in ports.flat_map(|port| port.slots.iter_mut()) {
        if *slot == Some(id) {
            *slot = None;
            return Ok(());
        }
    }
    if let Some(p) = state.players.iter().find(|p| p.order_cards_in_hand.contains(&id)) {
        return Err(format!(
            "fixture: order {} was DEALT to seat {} — the setup deal moved onto this \
             scenario's country set, so re-pick the set rather than raiding a hand",
            id.0, p.id.0,
        ));
    }
    Err(format!("fixture: order {} sits in no pile — conservation was broken before this", id.0))
}

/// Take one Technology card out of whichever pile the setup left it in, so a
/// fixture may file it under a seat's `technologies` without minting a 31st
/// card — [`take_order_out_of_play`]'s twin, against the tech-conservation
/// clause of [`check_invariants`] (`deck + labs + owned == 30`).
///
/// Techs have PER-COPY ids (30 physical cards, 20 titles), so the two error
/// arms below name genuinely different fixture repairs: an already-owned card
/// means two fixtures collided on one physical copy, and a card in no pile means
/// conservation was already broken upstream.
fn take_tech_out_of_play(state: &mut GameState, id: TechCardId) -> Result<(), String> {
    if let Some(i) = state.tech_deck.iter().position(|c| *c == id) {
        state.tech_deck.remove(i);
        return Ok(());
    }
    let labs = std::iter::once(&mut state.lab_state.lab_a).chain(state.lab_state.lab_b.as_mut());
    for slot in labs.flat_map(|lab| lab.slots.iter_mut()) {
        if *slot == Some(id) {
            *slot = None;
            return Ok(());
        }
    }
    if let Some(p) = state.players.iter().find(|p| p.technologies.contains(&id)) {
        return Err(format!(
            "fixture: technology {} is already owned by seat {} — two fixtures picked the same \
             physical copy, so re-pick rather than double-filing it",
            id.0, p.id.0,
        ));
    }
    Err(format!("fixture: technology {} sits in no pile — conservation was broken before this", id.0))
}

/// Every physical Technology copy of one country printing one gear value, by
/// id — the raw material for a holding whose INDUSTRY TOTAL is chosen and whose
/// COUNTRY ICONS are held constant.
///
/// Read off the registry rather than retyped, on the same discipline as
/// [`one_tech_per_country`]: the caller states the gear value it wants and
/// checks how many copies came back, so a data edit is a named fixture failure
/// rather than a silently different holding.
fn techs_of_country_at_gear(rules: &Rules, country: Country, gear: u8) -> Vec<TechCardId> {
    let mut ids: Vec<TechCardId> = rules
        .techs
        .iter()
        .filter(|c| c.country == country && c.industry_cost == gear)
        .map(|c| c.id)
        .collect();
    ids.sort_unstable_by_key(|id| id.0);
    ids
}

/// Seat 0's view of a fresh game — the base every doctored fixture starts from.
fn base_view(players: u8, seed: u64, rules: &Rules) -> PlayerView {
    observe(&keep_game(players, seed, rules), PlayerId(0)).expect("seat 0 is viewable")
}

/// The viewer's own row, mutably.
fn viewer_mut(view: &mut PlayerView) -> &mut VisiblePlayer {
    let me = view.viewer;
    view.players.iter_mut().find(|p| p.id == me).expect("the view contains its own seat")
}

/// One order id per country, in [`Country::ALL`] order, plus a SECOND id of the
/// first country — the raw material for a five-country set and for a
/// four-country set of the same size.
fn one_order_per_country(rules: &Rules) -> ([OrderCardId; 5], OrderCardId) {
    let pick = |country: Country, skip: usize| -> OrderCardId {
        let mut ids: Vec<OrderCardId> =
            rules.orders.iter().filter(|c| c.country == country).map(|c| c.id).collect();
        ids.sort_unstable_by_key(|id| id.0);
        ids[skip]
    };
    let set = [
        pick(Country::English, 0),
        pick(Country::American, 0),
        pick(Country::French, 0),
        pick(Country::German, 0),
        pick(Country::Dutch, 0),
    ];
    (set, pick(Country::English, 1))
}

/// One Technology per country, lowest id first, for the first `n` countries of
/// [`Country::ALL`] — a technology holding whose country icons are DISTINCT by
/// construction, and whose printed gear costs the caller reads off the registry
/// rather than retyping.
///
/// Countries with no printed technology are skipped rather than faked, so the
/// caller checks the returned LENGTH (all five have techs today; the check is
/// what would notice if a data edit changed that).
fn one_tech_per_country(rules: &Rules, n: usize) -> Vec<TechCardId> {
    Country::ALL
        .iter()
        .take(n)
        .filter_map(|country| {
            let mut ids: Vec<TechCardId> =
                rules.techs.iter().filter(|c| c.country == *country).map(|c| c.id).collect();
            ids.sort_unstable_by_key(|id| id.0);
            ids.first().copied()
        })
        .collect()
}

/// The viewer's own row. A free function rather than a closure because callers
/// need the borrow to outlive the call (a closure returning a reference derived
/// from its argument cannot express that).
fn own_row(view: &PlayerView) -> &VisiblePlayer {
    view.players.iter().find(|p| p.id == view.viewer).expect("the view contains its seat")
}

/// One player's slot in a [`MAX_SEATS`]-wide seat array, or `None` — a MIRROR of
/// `eval.rs`'s own `seat_index`, and deliberately as narrow: the dummy sentinel
/// has no slot, and neither has any seat number outside the printed range (only
/// a hand-built view can carry one).
///
/// It exists so [`terminal_value_with_dummy_gear`] — a REJECTED READING of the
/// shipped terminal ladder — indexes seats exactly where the shipped code
/// indexes them. A control that dropped only the dummy would be a subtly
/// different function from the one it is the counterfactual to, and on this
/// file's hand-built views that difference is reachable rather than theoretical.
fn seat_slot(player: &VisiblePlayer) -> Option<usize> {
    if player.id.is_dummy() {
        return None;
    }
    let index = usize::from(player.id.0);
    (index < MAX_SEATS).then_some(index)
}

/// The viewer's own raw value for one feature — the same number [`score`]
/// multiplies by a weight, read without a weight so a fixture can state what it
/// built (`ScoreStructure` is the authority on all sixteen; ten are field reads
/// and five are re-derived end scoring).
fn viewer_value(view: &PlayerView, rules: &Rules, feature: Feature) -> i64 {
    ScoreStructure::derive(view, rules).value_of(own_row(view), feature)
}

/// Which features' EXPLAINED products separate two views under `w` — the leak
/// check the three re-pricing scenarios lean on, in [`Feature::ALL`] order.
///
/// A re-pricing scenario's whole claim is "these two positions differ on the
/// slots the round moved and on nothing else", and a doctored fixture is exactly
/// the place a stray term creeps in (a tech card carries a country icon; a
/// completed order feeds an achievement leg). Naming the separating terms turns
/// that claim into a printed list rather than a reviewer's trust.
fn separating_terms(
    a: &PlayerView,
    b: &PlayerView,
    rules: &Rules,
    w: &WeightTable,
) -> Vec<&'static str> {
    explain(a, rules, w)
        .terms
        .iter()
        .zip(explain(b, rules, w).terms.iter())
        .filter(|(x, y)| x.product != y.product)
        .map(|(x, _)| x.feature.name())
        .collect()
}

// ---------------------------------------------------------------------------
// 1. Country-set completion
// ---------------------------------------------------------------------------

/// **RULES.** End scoring, country sets: a player forms as many sets as their
/// face-up country icons (completed Orders + owned Technologies) allow, and a
/// set of 2/3/4/5 DISTINCT countries is worth 2/4/8/12 VP. A duplicate icon
/// does not enlarge a set — four English icons are one English country, four
/// times over (`game/scoring.rs`; rulebook end-scoring, KB
/// `01-rules-reference.md`).
///
/// **SEAT.** 0, in a 2-player game.
///
/// **PROPERTY**, in two clauses of one claim. (a) `score(five completed
/// orders, five DISTINCT countries)` > `score(five completed orders, four
/// distinct — one country doubled)`; and (b) the EXPLANATION attributes that
/// whole margin to `country_bonus_now` and to no other term. Both fixtures
/// hold the same NUMBER of completed orders, so the second clause is also the
/// fixture's own leak check — any other term the doctoring moved would be named
/// by clause (b) rather than left to a reader's trust. Measured at T9 under the
/// frozen table: **144 against 96**, country weight 12 early.
///
/// **WHY.** `country_bonus_now` is the evaluator's best EARLY separator (R1-3,
/// AUC 0.6212 against running VP's 0.5625), and it is the term that replaced
/// both `completed_orders` and `distinct_countries` (§2.2's cluster winner). A
/// rung that priced completed orders by COUNT would rank these two positions
/// equal and would take the wrong Order at every Port decision in the opening.
///
/// **COUNTERFACTUAL — `country_bonus_now` zeroed (§0.2's targeted mutation).**
/// With that one pair set to `(0, 0)` the two fixtures are indistinguishable to
/// every remaining term. Observed failing line, verbatim:
///
/// ```text
/// the five-country set must outscore the four-country set: 12 VP of bonus
/// against 8, but the evaluator scored them 0 and 0 (country term 0/0)
/// ```
fn country_set_completion(rules: &Rules) -> Result<(), String> {
    let (distinct, duplicate_english) = one_order_per_country(rules);
    // Anti-vacuity on the fixture itself: the "five distinct" hand must really
    // be five countries, and the other must really be four.
    let countries = |ids: &[OrderCardId]| -> Vec<Country> {
        let mut cs: Vec<Country> =
            ids.iter().filter_map(|id| rules.orders.get(*id)).map(|c| c.country).collect();
        cs.sort_unstable_by_key(|c| Country::ALL.iter().position(|x| x == c).unwrap_or(usize::MAX));
        cs.dedup();
        cs
    };
    let four: Vec<OrderCardId> = distinct[..4].iter().copied().chain([duplicate_english]).collect();
    if countries(&distinct).len() != 5 || countries(&four).len() != 4 {
        return Err(format!(
            "fixture: expected a 5-country and a 4-country hand, got {} and {}",
            countries(&distinct).len(),
            countries(&four).len(),
        ));
    }

    let base = base_view(2, 11, rules);
    let with_orders = |ids: &[OrderCardId]| -> PlayerView {
        let mut v = base.clone();
        viewer_mut(&mut v).completed_orders = ids.to_vec();
        v
    };
    let five_set = with_orders(&distinct);
    let four_set = with_orders(&four);

    let check = |w: &WeightTable| -> Result<(), String> {
        let a = score(&five_set, rules, w);
        let b = score(&four_set, rules, w);
        let (ce, cl) = w.get(Feature::CountryBonusNow);
        require(
            a > b,
            format!(
                "the five-country set must outscore the four-country set: 12 VP of bonus \
                 against 8, but the evaluator scored them {a} and {b} (country term {ce}/{cl})"
            ),
        )?;
        // The EXPLANATION shape of the same claim (spec §6's third property
        // form): the margin must be attributed to the country term and to
        // nothing else. This is also the fixture's own leak check — if
        // doctoring `completed_orders` moved any other term, its name shows up
        // here instead of in a reviewer's imagination.
        let attributed: Vec<&str> = explain(&five_set, rules, w)
            .terms
            .iter()
            .zip(explain(&four_set, rules, w).terms.iter())
            .filter(|(x, y)| x.product != y.product)
            .map(|(x, _)| x.feature.name())
            .collect();
        require(
            attributed == ["country_bonus_now"],
            format!(
                "the explanation must attribute the whole margin to country_bonus_now, it \
                 named {attributed:?}"
            ),
        )
    };

    admit(
        SCENARIOS[0].0,
        "country_bonus_now zeroed",
        "the five-country set must outscore the four-country set",
        check(&WeightTable::V1),
        check(&zeroed(Feature::CountryBonusNow)),
    )
}

#[test]
fn scenario_01_country_set_completion() {
    run(0, "country-set completion");
}

// ---------------------------------------------------------------------------
// 2. Order economics — fulfill now vs hold
// ---------------------------------------------------------------------------

/// **RULES.** An Order card is completed by paying its printed trade-good
/// requirement, whereupon it moves face-up in front of the player and pays its
/// printed rewards; an Order still in hand scores NOTHING at game end and its
/// country icon counts for nothing. Achievement cards, by contrast, reward
/// HOLDING goods (Copper 5 / Silk 6 / Tea 7 / Seafood 7 — KB
/// `01-rules-reference.md`, achievement categories). Order #21 (NL) prints
/// `seafood 2 · tea 2 · silk 2 → 11 VP`.
///
/// **SEAT.** 0, in a 2-player game.
///
/// **PROPERTY.** `score(order #21 fulfilled)` > `score(order #21 still in
/// hand)`, on a fixture where the goods it costs are exactly the goods feeding
/// three achievement cards the seat currently sits AT threshold on. Fulfilling
/// banks 11 VP and releases a hand commitment, and gives up 6 goods' worth of
/// achievement progress (20 → 14). Measured at T9 under the frozen table:
/// **175 against 74**.
///
/// **WHY.** This is the rung's most common real decision — the goods are
/// always doing something else — and it is the one the feature list can most
/// easily get backwards: `assistants`, `imported_goods` and
/// `achievement_progress_sum` all price HOLDING, and only `victory_points`
/// and `country_bonus_now` price the conversion. A table that under-priced VP
/// would hoard goods and finish with a hand full of unfulfilled commitments,
/// which is exactly what `hand_size`'s measured negative sign (R1-7,
/// 0.4745 → 0.3753) says losing players do.
///
/// **COUNTERFACTUAL — `victory_points` zeroed (§0.2's targeted mutation).**
/// The printed 11 VP is what pays for the goods, so zeroing that one term
/// leaves the achievement loss unopposed and the seat prefers to hoard — the
/// margin goes from +101 to −9. Observed failing line, verbatim:
///
/// ```text
/// fulfilling order O21 for its printed 11 VP must beat holding it, even
/// though the goods it costs were feeding three achievements: fulfilled 25,
/// held 34 (vp term 0/0, achievement progress 14 vs 20)
/// ```
fn order_economics_fulfill_now_vs_hold(rules: &Rules) -> Result<(), String> {
    // The card is named by its printed corner index, and its printed numbers
    // are read from the registry rather than retyped.
    let id = OrderCardId(21);
    let card = rules.orders.get(id).ok_or("fixture: order 21 is not in the registry")?;
    let req = card.requirements;
    let vp = card.rewards.victory_points;
    if req.seafood == 0 || req.tea == 0 || req.silk == 0 || vp == 0 {
        return Err(format!("fixture: order 21's printed numbers moved: {req:?} -> {vp} VP"));
    }

    let mut base = base_view(2, 12, rules);
    // The three achievements this scenario turns on, set explicitly so the
    // fixture does not depend on which three the deal happened to reveal.
    base.achievements = [
        AchievementCard::new(AchievementType::Seafood7),
        AchievementCard::new(AchievementType::Tea7),
        AchievementCard::new(AchievementType::Silk6),
    ];
    {
        let me = viewer_mut(&mut base);
        me.completed_orders.clear();
        me.technologies.clear();
        me.victory_points = 4;
        // Exactly at all three thresholds: every good the order costs is a
        // point of achievement progress the seat gives up.
        me.seafood = 7;
        me.tea = 7;
        me.silk = 6;
        me.copper = 0;
        me.hand = vec![HandCard::Known(id)];
    }

    let held = base.clone();
    let mut fulfilled = base.clone();
    {
        let me = viewer_mut(&mut fulfilled);
        me.hand.clear();
        me.completed_orders = vec![id];
        me.victory_points += u32::from(vp);
        me.seafood -= u32::from(req.seafood);
        me.tea -= u32::from(req.tea);
        me.silk -= u32::from(req.silk);
        me.imported_goods += u32::from(card.rewards.imported_goods);
        me.money += u32::from(card.rewards.money);
    }

    let progress = |v: &PlayerView| -> i64 {
        let me = v.players.iter().find(|p| p.id == v.viewer).expect("own row");
        ScoreStructure::derive(v, rules).value_of(me, Feature::AchievementProgressSum)
    };
    let (held_progress, done_progress) = (progress(&held), progress(&fulfilled));
    // Anti-vacuity: the goods cost must actually cost achievement progress, or
    // the scenario is a plain "more VP is better" restatement.
    if done_progress >= held_progress {
        return Err(format!(
            "fixture: fulfilling must COST achievement progress ({held_progress} -> \
             {done_progress}); the three cards are not the ones the order pays with"
        ));
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let a = score(&fulfilled, rules, w);
        let b = score(&held, rules, w);
        let (ve, vl) = w.get(Feature::VictoryPoints);
        require(
            a > b,
            format!(
                "fulfilling order O{} for its printed {vp} VP must beat holding it, even though \
                 the goods it costs were feeding three achievements: fulfilled {a}, held {b} \
                 (vp term {ve}/{vl}, achievement progress {done_progress} vs {held_progress})",
                id.0,
            ),
        )
    };

    admit(
        SCENARIOS[1].0,
        "victory_points zeroed",
        "must beat holding it",
        check(&WeightTable::V1),
        check(&zeroed(Feature::VictoryPoints)),
    )
}

#[test]
fn scenario_02_order_economics_fulfill_now_vs_hold() {
    run(1, "order economics: fulfill now vs hold");
}

// ---------------------------------------------------------------------------
// 3. End-trigger timing — the church clock
// ---------------------------------------------------------------------------

/// **RULES.** The game ends when a trigger fires; one of them is the Church
/// management board filling to the player-count threshold (4/5/6 REAL pawns at
/// 2/3/4 players — the 2P dummy does not count, finding D1;
/// `game/end_game.rs`, `GameConfig::track_trigger`). Church places pay 6 VP to
/// the most pawns and 3 to the second, count ties broken by the rightmost
/// occupied space (rulebook p.9).
///
/// **SEAT.** 0, in a 4-player game — deliberately not 2P, because at two
/// players a single church pawn already reads the clock at 25% and the early
/// stage this scenario needs cannot be constructed.
///
/// **PROPERTY.** The value of a church LEAD flips sign with the church clock.
/// Let `d(view_pair) = score(viewer leads the church) − score(the same pawns
/// belonging to an opponent)`; then `d(one pawn on the track) < 0 < d(five
/// pawns on the track)`. The two members of each pair differ ONLY in who owns
/// the pawns, so every other term contributes identically and the stage proxy
/// is the same within a pair. Measured at T9 under the frozen table:
/// **d_early = −6** at p = 166,666 (church weight −1) and **d_late = +72** at
/// p = 833,333 (weight +12), with the church clock asserted to be the one
/// dating both positions.
///
/// **WHY.** This is T4's routed open question made executable. The weight
/// freeze confirmed `church_bonus_now`'s measured sign FLIP (0.4411 early →
/// 0.5754 late, both intervals excluding 0.5, under a corpus that ends via
/// `ChurchTrackFull` in 1,999 of 2,000 games) — but what that discharges is
/// SIGN-STABILITY under a ChurchTrackFull-dominated ending regime, and the
/// artifact says so in as many words: *"whether DELIBERATE early track-racing
/// pays remains the research notes' §4.6 open question, routed to the scenario
/// suite and 3.5"* (`docs/plans/2026-07-27-phase3.3-weight-freeze.md`, the
/// church-cluster row). This is the scenario-suite half of that routing, and it
/// states the DECISION rather than the correlation: early, racing the church
/// hands the evaluator a negative term — the bot must be paying for the pawn,
/// not for the place — while late the printed 6/3 is real VP and the lead is
/// worth having. §2.3 interpolates rather than switching stages precisely so
/// this term can cross zero smoothly, and this is the crossing.
///
/// **What it does NOT do, stated so the routing is not over-claimed.** It does
/// not settle whether deliberate early track-racing pays — that is a strength
/// question and it belongs to 3.5's tuning, where weights stop being hand-set.
/// What it pins is the shipped ANSWER: this rung declines an early church lead
/// and wants a late one, and any re-pricing that reverses either half has to
/// come through this scenario to do it.
///
/// **COUNTERFACTUAL — `church_bonus_now` zeroed (§0.2's targeted mutation).**
/// Zeroed, both halves of the pair collapse to identical scores and no sign
/// survives to flip. Observed failing line, verbatim:
///
/// ```text
/// EARLY (one pawn, p=166666, church weight 0): a church lead must be priced
/// NEGATIVE — leader 0, follower 0, d=0
/// ```
fn end_trigger_timing_church_clock(rules: &Rules) -> Result<(), String> {
    let base = base_view(4, 13, rules);
    if base.church_track.total_assistants() != 0 {
        return Err("fixture: a fresh 4P church track must start empty".into());
    }

    // (index -> owner). The pairs are mirror images: the SAME pawns on the SAME
    // spaces, owned by the viewer in one member and by seat 1 in the other.
    let with_pawns = |owners: &[(usize, u8)]| -> PlayerView {
        let mut v = base.clone();
        for (index, seat) in owners {
            v.church_track.slots[*index] =
                Some(ChurchSlot { player: PlayerId(*seat), arrival_order: *index as u8 + 1 });
        }
        v
    };
    let early_leader = with_pawns(&[(0, 0)]);
    let early_follower = with_pawns(&[(0, 1)]);
    // Five pawns: three for the leader, one each for the two remaining seats,
    // so second place is a real contest decided on the rightmost space.
    let late_leader = with_pawns(&[(0, 0), (1, 0), (2, 0), (3, 2), (4, 1)]);
    let late_follower = with_pawns(&[(0, 1), (1, 1), (2, 1), (3, 2), (4, 0)]);

    // The clock is the scenario's subject, so it is asserted rather than
    // assumed: the church occupancy must be the term that dates the position in
    // both pairs, and the two stages must be the ones named in the doc.
    let stage = |a: &PlayerView, b: &PlayerView, label: &str, expect: u32| -> Result<(), String> {
        let clocks = clock_ppm(a, rules);
        let (pa, pb) = (progress_ppm(a, rules), progress_ppm(b, rules));
        require(
            pa == expect && pb == pa && clocks[1] == pa,
            format!(
                "{label}: the church clock must date BOTH members of the pair at {expect} ppm \
                 (the mirror moves ownership, never occupancy), got {pa} and {pb} with clocks \
                 {clocks:?}"
            ),
        )
    };
    stage(&early_leader, &early_follower, "early", 166_666)?;
    stage(&late_leader, &late_follower, "late", 833_333)?;

    let check = |w: &WeightTable| -> Result<(), String> {
        let (ce, cl) = w.get(Feature::ChurchBonusNow);
        let early_w = interpolate(ce, cl, 166_666);
        let late_w = interpolate(ce, cl, 833_333);
        let d_early = score(&early_leader, rules, w) - score(&early_follower, rules, w);
        let d_late = score(&late_leader, rules, w) - score(&late_follower, rules, w);
        require(
            d_early < 0,
            format!(
                "EARLY (one pawn, p=166666, church weight {early_w}): a church lead must be \
                 priced NEGATIVE — leader {}, follower {}, d={d_early}",
                score(&early_leader, rules, w),
                score(&early_follower, rules, w),
            ),
        )?;
        require(
            d_late > 0,
            format!(
                "LATE (five pawns, p=833333, church weight {late_w}): a church lead must be \
                 priced POSITIVE — leader {}, follower {}, d={d_late}",
                score(&late_leader, rules, w),
                score(&late_follower, rules, w),
            ),
        )
    };

    admit(
        SCENARIOS[2].0,
        "church_bonus_now zeroed",
        "a church lead must be priced NEGATIVE",
        check(&WeightTable::V1),
        check(&zeroed(Feature::ChurchBonusNow)),
    )
}

#[test]
fn scenario_03_end_trigger_timing_church_clock() {
    run(2, "end-trigger timing: the church clock");
}

// ---------------------------------------------------------------------------
// 4. Church/customs majority — the D1 dummy
// ---------------------------------------------------------------------------

/// **RULES.** Finding D1 (rulebook p.9 NOTE, Jeremy's edition; guidingpath's
/// 2026-07-21 decision-log rows). In a 2-player game a neutral dummy pawn sits
/// on Church faith-7 and Customs space-4, and it is a real COMPETITOR in both
/// majority bonuses: it can take a place — which is then awarded to nobody
/// while still being consumed — and it wins a count tie on the rightmost
/// occupied space. Customs pays 8 VP to the most pawns and 4 to the second.
///
/// **SEAT.** 0, in a 2-player game (the only count with a dummy).
///
/// **PROPERTY.** With one customs pawn each and the dummy on space 4,
/// `score(viewer on space 5)` > `score(viewer on space 3)` — landing to the
/// RIGHT of the dummy wins the count tie and takes first place (8 VP), landing
/// to its left loses the tie and collects second (4 VP). Both fixtures put the
/// same NUMBER of pawns on the track, so the customs clock and every other term
/// are identical. The 8 and the 4 are read off the ENGINE's own per-player
/// breakdown before the evaluator is asked. Measured at T9 under the frozen
/// table: **56 against 28**, customs weight 7 at p = 250,000.
///
/// **WHY.** This is the rule an evaluator is most likely to get wrong by
/// omission, because it is invisible at 3 and 4 players: a mirror that simply
/// ranked the real seats would score both fixtures 8 VP and the bot would spend
/// its customs visits landing behind a pawn it cannot beat. The engine prices
/// the dummy exactly (`game/scoring.rs`) and §2.2 requires the view-side mirror
/// to agree; this scenario is that agreement stated as a DECISION rather than as
/// a value.
///
/// **CONTROL (live, in-test).** The same two fixtures with the dummy removed
/// score EQUAL (56 and 56) — so the property is genuinely dummy-driven and not
/// an artifact of the space index.
///
/// **COUNTERFACTUAL — `customs_bonus_now` zeroed (§0.2's targeted mutation).**
/// Observed failing line, verbatim:
///
/// ```text
/// landing right of the D1 dummy (space 5, 8 VP) must outscore landing left of
/// it (space 3, 4 VP): 0 vs 0 (customs term 0/0)
/// ```
fn customs_majority_with_the_d1_dummy(rules: &Rules) -> Result<(), String> {
    // A REAL state here, not a doctored view alone: half the claim is that the
    // engine's own final scoring agrees, and that needs `final_result`.
    let mut state = keep_game(2, 14, rules);
    let dummy_space = CustomsTrack::space_to_index(4).expect("space 4 exists");
    if !state.customs_track.slots[dummy_space].is_some_and(|s| s.player.is_dummy()) {
        return Err("fixture: the 2P setup must place the customs dummy on space 4".into());
    }
    state.customs_track.slots.iter_mut().for_each(|slot| {
        *slot = slot.filter(|s| s.player.is_dummy());
    });
    engine_valid(&state, rules, "2P customs track cleared back to the dummy alone")?;

    let placed = |space: u8| -> Result<GameState, String> {
        let mut s = state.clone();
        s.customs_track.slots[CustomsTrack::space_to_index(space).expect("1..=7")] =
            Some(CustomsSlot { player: PlayerId(0), arrival_order: 1 });
        engine_valid(&s, rules, &format!("seat 0 on customs space {space}"))?;
        Ok(s)
    };
    let (right, left) = (placed(5)?, placed(3)?);

    // The engine's own breakdown is what "8 VP" and "4 VP" mean here.
    let engine_customs = |s: &GameState| -> i64 {
        i64::from(
            final_result(s, rules)
                .standings
                .iter()
                .find(|st| st.player == PlayerId(0))
                .expect("seat 0 places")
                .breakdown
                .customs,
        )
    };
    if (engine_customs(&right), engine_customs(&left)) != (8, 4) {
        return Err(format!(
            "fixture: the engine must score these 8 and 4 (D1), got {} and {}",
            engine_customs(&right),
            engine_customs(&left),
        ));
    }

    let view = |s: &GameState| observe(s, PlayerId(0)).expect("seat 0 is viewable");
    let (right_view, left_view) = (view(&right), view(&left));

    // The live control: with the dummy gone, both landings are a lone first
    // place and the evaluator must be indifferent.
    let without_dummy = |v: &PlayerView| -> PlayerView {
        let mut v = v.clone();
        v.customs_track.slots[dummy_space] = None;
        v
    };
    let (a, b) = (
        score(&without_dummy(&right_view), rules, &WeightTable::V1),
        score(&without_dummy(&left_view), rules, &WeightTable::V1),
    );
    require(
        a == b,
        format!(
            "control: with the dummy removed both landings are a lone first place and must \
             score equal, got {a} and {b} — the fixture is testing the space index, not D1"
        ),
    )?;

    let check = |w: &WeightTable| -> Result<(), String> {
        let a = score(&right_view, rules, w);
        let b = score(&left_view, rules, w);
        let (ce, cl) = w.get(Feature::CustomsBonusNow);
        require(
            a > b,
            format!(
                "landing right of the D1 dummy (space 5, 8 VP) must outscore landing left of \
                 it (space 3, 4 VP): {a} vs {b} (customs term {ce}/{cl})"
            ),
        )
    };

    admit(
        SCENARIOS[3].0,
        "customs_bonus_now zeroed",
        "landing right of the D1 dummy",
        check(&WeightTable::V1),
        check(&zeroed(Feature::CustomsBonusNow)),
    )
}

#[test]
fn scenario_04_church_customs_majority_d1_dummy() {
    run(3, "church/customs majority: the D1 dummy");
}

// ---------------------------------------------------------------------------
// 5. Building race
// ---------------------------------------------------------------------------

/// **RULES.** Two of the end-game triggers are building races: ONE player
/// building all 8 Shophouses, or ONE player building all 4 Trading Houses
/// (`game/end_game.rs`). Trading Houses are the scarcer token (4 against 8),
/// cost 4/5/6/7 Yen at the Employment Agency where Shophouses cost 2, and pay
/// the larger printed reward (KB `01-rules-reference.md`; design §5.2's take
/// progression).
///
/// **SEAT.** 0, in a 2-player game, with the order deck run down to 2 cards so
/// the DECK clock (800,000 ppm) dates the position and neither building clock
/// moves the stage between the two fixtures.
///
/// **PROPERTY.** `score(one Trading House built in area X)` > `score(one
/// Shophouse built in the same area X)`. Same area, so the two fixtures give
/// identical achievement progress (a building in an area counts once, whichever
/// kind it is) and identical everything else. Measured at T9 under the frozen
/// table: **23 against 18**, the two interpolated weights at p = 800,000.
///
/// **WHY.** The two buildings are the same ACTION at different prices, and the
/// evaluator's only lever for telling them apart is the weight pair. A rung
/// that priced them equally would build whichever token was cheapest to reach
/// and would never run the four-token race — the faster of the two end
/// triggers, and the one a leading position wants to reach.
///
/// **WHAT IT ACTUALLY REDUCES TO — this is the suite's WEAKEST property, and
/// it is recorded rather than dressed up (T9 review S1).** The two fixtures are
/// identical apart from which token stands in the same area, so the review
/// verified that only the two building terms differ between them and the
/// inequality collapses to `w_trading(p) > w_shophouse(p)` at one stage. NO
/// rules arithmetic is exercised: nothing here computes a take cost, a printed
/// reward, or a distance to either trigger. The scarcity and price rationale in
/// the RULES line above lives entirely in the WEIGHTS, and under the frozen
/// table the ordering holds at EVERY `p` — 15 against 12 early and 25 against
/// 20 late, linearly interpolated, so no crossing exists to find. What this
/// scenario therefore pins is the weight ORDERING, and a re-pricing that
/// inverted it would have to come through here to do it. Kept under the
/// never-delete rule and flagged for strengthening at v2, where the property
/// worth having is behavioural — does the rung actually RUN the four-token
/// race? — and needs the lookahead 3.4 adds to be worth asserting.
///
/// **COUNTERFACTUAL — `trading_houses_built` zeroed (§0.2's targeted
/// mutation).** Observed failing line, verbatim:
///
/// ```text
/// a Trading House must outprice a Shophouse in the same area (4 tokens to the
/// end trigger against 8, at the larger printed reward): 0 vs 18 (trading term
/// 0/0 against shophouse 12/20)
/// ```
fn building_race_trading_house_over_shophouse(rules: &Rules) -> Result<(), String> {
    let mut base = base_view(2, 15, rules);
    // Run the deck down so the DECK clock, not either building clock, sets the
    // stage — otherwise the two fixtures interpolate at different p and the
    // comparison would not be about the building terms alone.
    base.order_deck_len = 2;
    let area = *base.board_layout.areas.keys().next().ok_or("fixture: an empty board")?;

    let mut trading = base.clone();
    viewer_mut(&mut trading).trading_houses.built = vec![area];
    let mut shophouse = base.clone();
    viewer_mut(&mut shophouse).shophouses.built = BTreeMap::from([(area, 0u8)]);

    let stage = |v: &PlayerView| progress_ppm(v, rules);
    if stage(&trading) != 800_000 || stage(&shophouse) != 800_000 {
        return Err(format!(
            "fixture: both positions must sit at the same deck-dated stage, got {} and {}",
            stage(&trading),
            stage(&shophouse),
        ));
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let a = score(&trading, rules, w);
        let b = score(&shophouse, rules, w);
        let (te, tl) = w.get(Feature::TradingHousesBuilt);
        let (se, sl) = w.get(Feature::ShophousesBuilt);
        require(
            a > b,
            format!(
                "a Trading House must outprice a Shophouse in the same area (4 tokens to the \
                 end trigger against 8, at the larger printed reward): {a} vs {b} (trading \
                 term {te}/{tl} against shophouse {se}/{sl})"
            ),
        )
    };

    admit(
        SCENARIOS[4].0,
        "trading_houses_built zeroed",
        "a Trading House must outprice a Shophouse in the same area",
        check(&WeightTable::V1),
        check(&zeroed(Feature::TradingHousesBuilt)),
    )
}

#[test]
fn scenario_05_building_race() {
    run(4, "building race");
}

// ---------------------------------------------------------------------------
// 6. Retreat/recovery discipline
// ---------------------------------------------------------------------------

/// **RULES.** Movement Option B: instead of moving the President, a player may
/// recover ANY NUMBER of their own Assistants from the areas the President
/// passes back to their hand (the `r` retreat grammar; rulebook p.14, KB
/// `01-rules-reference.md`). Assistants are placed FROM HAND ONLY — never from
/// the warehouse or the management boards — so a seat with an empty hand cannot
/// place, cannot visit the Church and cannot visit Customs.
///
/// **SEAT.** 0, in a 2-player game, holding 4 Assistants on one area and an
/// empty hand.
///
/// **FIXTURE HONESTY — the assistant TOTAL (T9 review S4).** These three views
/// are the only fixtures in the file that change the seat's assistant total.
/// The 2P deal is 8 in hand + 12 in the warehouse = the engine's 20; the
/// fixtures below overwrite the hand and board legs with 4 tokens between them
/// and leave the warehouse at 12, so each view carries **16**, not 20. That is
/// deliberate and harmless to the property — all three carry the SAME 16, the
/// warehouse leg is byte-identical across them, and the claim is an ordering
/// over the hand/board split alone — but these are doctored views and not
/// reachable states, which is what the header's doctoring declaration covers
/// and what keeps `check_invariants` (whose `total() != 20` clause these would
/// fail) off the view side. Said out loud rather than left for a reader to
/// notice.
///
/// **PROPERTY.** The recovery optimum is INTERIOR:
/// `score(recover 2)` > `score(recover 0)` and `score(recover 2)` >
/// `score(recover 4)`. Recovering back to the hand reserve beats leaving the
/// hand empty, and beats emptying the board. Measured at T9 under the frozen
/// table: **recover-2 = 4, recover-0 = −28, recover-4 = −16** — margins of +32
/// and +20 around an interior optimum.
///
/// **WHY.** Spec §2.5's falsifier says a retreat must SOMETIMES pay; L14 says
/// the reason it can is the piecewise hand split, because a single linear
/// in-hand weight makes "a retreat sometimes pays" and "a placement ever pays"
/// the same inequality with opposite signs. The interior optimum is what that
/// split buys, and it is the shape the rung needs: under eval-v0's linear
/// `on_board(18)` over `in_hand(8)` a nonzero recovery was argmax 0 times in
/// 48,458 movement picks (notes §4.3) — the `RetreatAverse` pathology arriving
/// from the evaluator side, which `policy_diversity`'s own gate exists to catch.
///
/// **NOTE ON THE MUTATION NAMED IN THE SPEC.** §0.2's illustrative list says
/// "the recovery scenario fails under warehouse-weight-zeroed". It does not,
/// and the reason is the PATH rather than any scarcity of warehouse writers.
/// A recovery moves board → HAND. The engine has THREE production writers of
/// `assistants.in_warehouse` — the Employment Agency take
/// (`game/apply_move.rs:887`), the University additional action
/// (`game/tech_effects.rs:175`) and the `WarehouseTake` choice resolution
/// (`game/choices.rs:230`) — and not one of them sits on the retreat path:
/// all three are warehouse → hand DRAINS at other decision points. The
/// warehouse leg is consequently identical across all three fixtures below, so
/// zeroing its weight cannot change a retreat-vs-not ordering, and a mutation
/// that cannot break the property is not a targeted one. The term this scenario
/// actually names is the hand split L14 identifies as the recovery calculus's
/// pivot, and that is what is mutated below. Deviation recorded here rather
/// than papered over. (T9 review M1 corrected the writer count: this note
/// previously called the Employment Agency the engine's ONLY warehouse writer,
/// which is false. The deviation itself was re-verified and stands — what makes
/// it sound is that no writer is on the retreat path, not how many there are.)
///
/// **COUNTERFACTUAL — `assistants_in_hand_core` zeroed (§0.2's targeted
/// mutation).** With the reserve priced at nothing the board term dominates and
/// leaving every assistant out is best — eval-v0's pathology, restored.
/// Observed failing line, verbatim:
///
/// ```text
/// recovering to the hand reserve must beat recovering nothing (an empty hand
/// cannot place, visit the Church or visit Customs): recover-2 4, recover-0 32
/// (core term 0/0, board 14/10, excess 4/2)
/// ```
fn recovery_discipline_stops_at_the_reserve(rules: &Rules) -> Result<(), String> {
    let base = base_view(2, 16, rules);
    let area = *base.board_layout.areas.keys().next().ok_or("fixture: an empty board")?;

    // The four assistants are the SAME four in all three fixtures; only where
    // they sit moves, so every other term contributes identically. The seat's
    // TOTAL is 16 in all three, not the engine's 20 — the untouched 12-token
    // warehouse plus these 4. See the doc's fixture-honesty note.
    let after_recovering = |n: u8| -> PlayerView {
        let mut v = base.clone();
        let me = viewer_mut(&mut v);
        me.assistants.in_hand = n;
        me.assistants.on_board = BTreeMap::new();
        if n < 4 {
            me.assistants.on_board.insert(area, 4 - n);
        }
        v
    };
    let (none, reserve, all) = (after_recovering(0), after_recovering(2), after_recovering(4));

    let check = |w: &WeightTable| -> Result<(), String> {
        let (n, r, a) =
            (score(&none, rules, w), score(&reserve, rules, w), score(&all, rules, w));
        let (core_e, core_l) = w.get(Feature::AssistantsInHandCore);
        let (board_e, board_l) = w.get(Feature::AssistantsOnBoard);
        let (exc_e, exc_l) = w.get(Feature::AssistantsInHandExcess);
        let terms = format!(
            "(core term {core_e}/{core_l}, board {board_e}/{board_l}, excess {exc_e}/{exc_l})"
        );
        require(
            r > n,
            format!(
                "recovering to the hand reserve must beat recovering nothing (an empty hand \
                 cannot place, visit the Church or visit Customs): recover-2 {r}, recover-0 \
                 {n} {terms}"
            ),
        )?;
        require(
            r > a,
            format!(
                "recovering to the hand reserve must beat emptying the board (assistants past \
                 the reserve are worth more working than held): recover-2 {r}, recover-4 {a} \
                 {terms}"
            ),
        )
    };

    admit(
        SCENARIOS[5].0,
        "assistants_in_hand_core zeroed",
        "must beat recovering nothing",
        check(&WeightTable::V1),
        check(&zeroed(Feature::AssistantsInHandCore)),
    )
}

#[test]
fn scenario_06_retreat_recovery_discipline() {
    run(5, "retreat/recovery discipline");
}

// ---------------------------------------------------------------------------
// 7. Faith arithmetic
// ---------------------------------------------------------------------------

/// **RULES.** Church Faith scales LINEARLY with action power (1,2,3,4,5, capped
/// at 5) plus 1 per DISTINCT payment type contributed, one each from copper,
/// silk, tea, seafood, Yen and imported good (`calculate_church_faith`;
/// Jeremy-adjudicated 2026-07-17 — the rulebook's "2 Power: 1 Faith" line is a
/// typo). The pawn then goes on any empty Faith space at or below that total,
/// and the Church majority's count tie is broken by the RIGHTMOST occupied
/// space (rulebook p.9). At 2 players the dummy holds Faith 7 (finding D1).
///
/// **SEAT.** 0, in a 2-player game; the opponent already sits on Faith 5.
///
/// **PROPERTY.** `score(viewer on the space a power-5 visit paying THREE types
/// reaches)` > `score(viewer on the space the same visit paying ONE type
/// reaches)` — i.e. Faith 8 beats Faith 6, where both numbers come from
/// `calculate_church_faith` rather than from the fixture author. At Faith 8 the
/// viewer is rightmost and takes first place (6 VP); at Faith 6 the DUMMY on
/// Faith 7 is rightmost, takes a place that is awarded to nobody, and the
/// viewer collects second (3 VP). Measured at T9 under the frozen table:
/// **33 against 15**, church weight +6 at p = 500,000 (two real pawns of the
/// 2P trigger's four).
///
/// **WHY.** The faith formula is the only place in the rules where paying MORE
/// KINDS beats paying more of one kind, and it is the arithmetic this project
/// had to adjudicate against its own rulebook text. Its decision consequence is
/// entirely positional — two extra distinct goods buy two spaces, and two
/// spaces are the difference between 6 VP and 3 — so an evaluator that priced
/// church presence without pricing DEPTH would treat the two visits as the same
/// move.
///
/// **COUNTERFACTUAL — `church_bonus_now` zeroed (§0.2's targeted mutation).**
/// Both fixtures put one pawn on the track, so with the church term zeroed
/// nothing distinguishes Faith 8 from Faith 6. Observed failing line, verbatim:
///
/// ```text
/// a power-5 church visit paying 3 distinct types reaches Faith 8 and takes
/// first place (6 VP); paying 1 type reaches Faith 6, behind the D1 dummy on
/// Faith 7, for second (3 VP): -3 vs -3 (church term 0/0 at p=500000)
/// ```
fn faith_arithmetic_buys_the_rightmost_church_space(rules: &Rules) -> Result<(), String> {
    // The arithmetic itself, from the engine's own function.
    let rich = calculate_church_faith(5, true, true, false, false, true, false);
    let poor = calculate_church_faith(5, true, false, false, false, false, false);
    if (rich, poor) != (8, 6) {
        return Err(format!(
            "fixture: power 5 with 3 distinct payment types must reach Faith 8 and with 1 type \
             Faith 6 (linear power + 1 per type), got {rich} and {poor}"
        ));
    }

    let mut base = base_view(2, 17, rules);
    let dummy = ChurchTrack::faith_to_index(7).expect("faith 7 exists");
    if !base.church_track.slots[dummy].is_some_and(|s| s.player.is_dummy()) {
        return Err("fixture: the 2P setup must place the church dummy on Faith 7".into());
    }
    base.church_track.slots[ChurchTrack::faith_to_index(5).expect("faith 5")] =
        Some(ChurchSlot { player: PlayerId(1), arrival_order: 1 });

    let landing = |faith: u8| -> PlayerView {
        let mut v = base.clone();
        v.church_track.slots[ChurchTrack::faith_to_index(faith).expect("4..=11")] =
            Some(ChurchSlot { player: PlayerId(0), arrival_order: 2 });
        v
    };
    let (deep, shallow) = (landing(rich), landing(poor));
    let p = progress_ppm(&deep, rules);
    if p != 500_000 || p != progress_ppm(&shallow, rules) {
        return Err(format!("fixture: both landings must sit at p=500000, got {p}"));
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let a = score(&deep, rules, w);
        let b = score(&shallow, rules, w);
        let (ce, cl) = w.get(Feature::ChurchBonusNow);
        require(
            a > b,
            format!(
                "a power-5 church visit paying 3 distinct types reaches Faith {rich} and takes \
                 first place (6 VP); paying 1 type reaches Faith {poor}, behind the D1 dummy on \
                 Faith 7, for second (3 VP): {a} vs {b} (church term {ce}/{cl} at p={p})"
            ),
        )
    };

    admit(
        SCENARIOS[6].0,
        "church_bonus_now zeroed",
        "a power-5 church visit paying 3 distinct types",
        check(&WeightTable::V1),
        check(&zeroed(Feature::ChurchBonusNow)),
    )
}

#[test]
fn scenario_07_faith_arithmetic() {
    run(6, "faith arithmetic");
}

// ---------------------------------------------------------------------------
// 8. The null-placement trap  (the one DRIVEN scenario)
// ---------------------------------------------------------------------------

/// The two agent seeds the driven segment runs on. Fixed, so the segment — and
/// therefore each decision's sampler seed and §3.2 tie-break draw — is
/// byte-identical run to run.
const DRIVEN_SEEDS: [u64; 2] = [0xA3E1_2026_0727_0091, 0xA3E1_2026_0727_0092];

/// How many decisions the segment plays. Sized to cover several full turns at
/// 2 players (both seats reach Placement repeatedly) while staying a fraction
/// of a second in debug — see this file's header for the measurement.
const DRIVEN_DECISIONS: usize = 40;

/// **RULES.** The Placement step is optional in the sense that placing ZERO
/// assistants is always legal — the engine emits `PlaceThreeAreas {
/// placements: [] }` and emits it FIRST (`placement_iter`'s documented
/// emission order). Assistants are placed from hand, 1 Yen per assistant to
/// each opposing President's owner in the area (Option B's two-into-one-area
/// costs 2, not 1; KB `01-rules-reference.md`).
///
/// **SEAT.** Both, in a driven 2-player Greedy-v1 self-play segment.
///
/// **PROPERTY (behavioural — the only one here that is).** Over
/// [`DRIVEN_DECISIONS`] decisions from a fresh game, the shipped rung applies at
/// least one placement that actually puts assistants on the board. Measured at
/// T9 under the frozen table: **14 assistants in 5 real placements**, out of 40
/// decisions.
///
/// The AGGREGATE form is deliberate. The §3.2 tie-break is a uniform draw, so
/// "the chosen move at decision *k* is a real placement" is a statement about
/// where one draw landed; a count over a whole segment is robust to every tie
/// individually, while the fixed [`DRIVEN_SEEDS`] additionally make the whole
/// segment reproducible. Both halves of the brief's determinism requirement,
/// rather than one.
///
/// **WHY.** This is the failure mode Phase 3.3 has walked into twice from two
/// different directions, and neither time did any VALUE test notice. R2 found
/// it from the tie-break: the null placement is emitted first, evaluators tie on
/// ~30% of decisions, so first-listed-wins plays "place nothing" → nothing on
/// the board → the routes family is empty at every Movement point → an
/// absorbing no-op loop (θ̂ 0.2633 against 0.9292 for a uniform draw over the
/// tied set, same master and evaluator). T3 found the same absorbing loop from
/// the WEIGHTS: the pre-split table priced in-hand 8 over on-board 7, every
/// placement scored strictly negative, and 2P self-play returned
/// `CapReached { turns: 401 }` having placed 0 assistants (spec L14). A bot that
/// places nothing is not a weak bot, it is a stuck one — and the shape of the
/// bug is that every arithmetic pin stays green while it happens, which is why
/// this scenario drives a game instead of scoring a view.
///
/// **COUNTERFACTUAL — `assistants_on_board` zeroed (§0.2's targeted
/// mutation).** At one ply a placement moves `k` assistants from hand to board
/// and does nothing else the positional vector can see, so with the board term
/// at zero every placement is a strict loss of `k · w_hand` and the anchored
/// empty placement is the argmax at every Placement point — T3's degeneracy,
/// reconstructed from the other side. Observed failing line, verbatim:
///
/// ```text
/// Greedy-v1 must escape the null-placement trap: over 40 decisions it placed
/// 0 assistants in 0 real placements (board term 0/0)
/// ```
fn the_null_placement_trap(rules: &Rules) -> Result<(), String> {
    let check = |w: &WeightTable| -> Result<(), String> {
        let (placements, assistants) = drive_greedy_2p(rules, w)?;
        let (be, bl) = w.get(Feature::AssistantsOnBoard);
        require(
            assistants > 0,
            format!(
                "Greedy-v1 must escape the null-placement trap: over {DRIVEN_DECISIONS} \
                 decisions it placed {assistants} assistants in {placements} real placements \
                 (board term {be}/{bl})"
            ),
        )
    };

    admit(
        SCENARIOS[7].0,
        "assistants_on_board zeroed",
        "must escape the null-placement trap",
        check(&WeightTable::V1),
        check(&zeroed(Feature::AssistantsOnBoard)),
    )
}

/// Play [`DRIVEN_DECISIONS`] decisions of 2P Greedy-v1 self-play under
/// `weights`, returning `(real placements applied, assistants placed)`.
///
/// The honest chain is the agent's own (`observe` → sampler → handle → argmax);
/// this loop only supplies the seat, exactly as `arena::drive` does
/// (`let seat = state.current_player`). `apply_move_unchecked` is the
/// precedent `greedy.rs`'s own driven pins use — the move came off the agent's
/// legal families, and engine legality is four other suites' subject.
fn drive_greedy_2p(rules: &Rules, weights: &WeightTable) -> Result<(usize, usize), String> {
    let mut state = keep_game(2, 0xA3E1_2026_0727_0090, rules);
    let mut agents: Vec<GreedyAgent> = DRIVEN_SEEDS
        .iter()
        .map(|seed| GreedyAgent::with_weights(rules, *weights, DEFAULT_RETREAT_SAMPLES, *seed))
        .collect();

    let (mut decisions, mut placements, mut assistants) = (0usize, 0usize, 0usize);
    while decisions < DRIVEN_DECISIONS && !is_game_over(&state) {
        let seat = state.current_player;
        let view = observe(&state, seat).map_err(|e| format!("observe seat {}: {e}", seat.0))?;
        let report = agents[usize::from(seat.0)].choose(&view, rules)?;
        let placed = assistants_placed_by(&report.mv);
        if placed > 0 {
            placements += 1;
            assistants += placed;
        }
        state = apply_move_unchecked(&state, &report.mv, rules)
            .map_err(|e| format!("the chosen move failed to apply: {e}"))?;
        decisions += 1;
    }
    // Anti-vacuity on the SEGMENT rather than on the property: "it placed
    // nothing in 40 decisions" and "it placed nothing in the 3 decisions before
    // the game ended" are very different claims, and only the first is the one
    // this scenario makes. A short segment is an error, not a pass.
    require(
        decisions == DRIVEN_DECISIONS,
        format!(
            "the driven segment must run its full {DRIVEN_DECISIONS} decisions and ran \
             {decisions} — a segment cut short by the game ending would make the placement \
             count a claim about a shorter game rather than about the trap"
        ),
    )?;
    Ok((placements, assistants))
}

/// Assistants one applied move put on the board — the two Placement encodings
/// and nothing else (`greedy.rs`'s `assistants_placed`, per move).
fn assistants_placed_by(mv: &Move) -> usize {
    match mv {
        Move::PlaceThreeAreas { placements } => placements.len(),
        Move::PlaceTwoSameArea { adjacent_bonus, .. } => 2 + usize::from(adjacent_bonus.is_some()),
        _ => 0,
    }
}

#[test]
fn scenario_08_the_null_placement_trap() {
    run(7, "the null-placement trap");
}

// ---------------------------------------------------------------------------
// 9. Terminal scoring
// ---------------------------------------------------------------------------

/// **RULES.** End scoring is running VP PLUS the unbanked end bonuses —
/// country sets, the technology industry-value majority (10/5), the Church
/// (6/3) and Customs (8/4) majorities, and the leftover conversions — with the
/// winner decided on the total and ties broken clockwise from the start player
/// (`game/scoring.rs`, `final_result`). A card still in hand scores nothing.
///
/// **SEAT.** 0, in two finished 2-player games.
///
/// **PROPERTY.** `score(the finished game seat 0 WINS on bonuses while trailing
/// on running VP)` > `score(the finished game seat 0 LOSES on bonuses while
/// leading on running VP)`. In the first, seat 0 has 20 running VP and a
/// five-country set (32 total) against 30; in the second it has 30 against an
/// opponent's 20-plus-the-same-set (32). The engine's `final_result` is asserted
/// to agree on both winners before the evaluator is asked, and the view-side
/// standing is asserted to place the viewer 1st and 2nd respectively. Measured
/// at T9: **+1,002,000,000 against −1,002,000,000** — one rank step either side
/// of zero plus the clamped ±2-point margin.
///
/// **WHY.** §2.6 exists because the obvious reading — "VP totals are visible in
/// the view, rank on those" — mis-ranks exactly the states the terminal term is
/// for: at the moment the game stops, the bonuses are unbanked and can be worth
/// more than everything on the VP track. Both spec review lenses rejected that
/// reading. A search reaching a terminal successor must prefer the one it WINS,
/// and this pair is the smallest position where winning and leading disagree.
///
/// **COUNTERFACTUAL — ranking on running VP alone.** The targeted mutation here
/// is a SOURCE edit, not a weight: §2.6 short-circuits the weight vector
/// entirely on a terminal view, so no [`WeightTable`] can reach this property
/// and a weight mutation would be untargeted by construction. At authoring,
/// `eval.rs`'s `terminal_standing_of` was changed to read
/// `structure.base_vp[..]` where it reads `structure.final_total(..)` — the two
/// lines that define `viewer_total` and each rival's `total`, reverted
/// immediately after — and this scenario failed with, verbatim:
///
/// ```text
/// the terminal the viewer WINS (20 running VP + a 12-VP country set = 32) must
/// outrank the terminal it loses (30 running VP against 32): won -1010000000,
/// lost 1010000000
/// ```
///
/// Two consequences of that run are built into the code below rather than left
/// as prose. First, the fixture guard reads `ScoreStructure::final_total`
/// directly instead of `terminal_standing`'s output — a guard that consumed the
/// mutated function fired FIRST and masked the property, which is how the line
/// above came to be captured on the second attempt. Second, the rank
/// cross-check runs AFTER the score comparison, so a broken ranking reports the
/// score line rather than the rank line. The shipped counterfactual is that same
/// rejected reading computed locally from `ScoreStructure::base_vp` (it prints
/// `ranked on running VP alone: won -1010000000, lost 1010000000` — the same two
/// numbers), so the control cannot rot with the source edit that produced them.
fn terminal_scoring_prefers_the_bonus_win(rules: &Rules) -> Result<(), String> {
    let (set, _) = one_order_per_country(rules);

    // `bonus_holder` is the seat given the five-country set; the other leads on
    // running VP and loses.
    let finished = |bonus_holder: u8| -> Result<GameState, String> {
        let mut state = keep_game(2, 18, rules);
        force_terminal(&mut state);
        for p in state.players.iter_mut() {
            // Zero every remainder leg so the two totals are exactly the two
            // numbers this scenario is about. Both card piles are empty at a
            // fresh setup, so the two `clear()`s are no-ops — and if that ever
            // stopped being true they would destroy cards, which is exactly
            // what the conservation guard at the end of this closure catches.
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
            p.completed_orders.clear();
            p.technologies.clear();
            p.victory_points = if p.id.0 == bonus_holder { 20 } else { 30 };
        }
        // CONSERVATION (T9 review M2). Filing five country cards under a
        // completed pile without taking them out of the piles the deal left
        // them in MINTS five cards: at this seed three sit in the order deck
        // and two in the boxed pile, and the state that reached the evaluator
        // carried 41 of a 36-card game. `check_invariants` rejects it, and
        // before this loop existed nothing here asked.
        for id in set {
            take_order_out_of_play(&mut state, id)?;
        }
        state.players[usize::from(bonus_holder)].completed_orders = set.to_vec();
        engine_valid(&state, rules, &format!("terminal game, seat {bonus_holder} holds the set"))?;
        Ok(state)
    };
    let (won, lost) = (finished(0)?, finished(1)?);

    for (label, state, winner) in [("won", &won, 0u8), ("lost", &lost, 1)] {
        let result = final_result(state, rules);
        if result.winner != PlayerId(winner) {
            return Err(format!(
                "fixture: the engine must call seat {winner} the winner of the '{label}' game, \
                 it called seat {}",
                result.winner.0,
            ));
        }
    }

    let view = |s: &GameState| observe(s, PlayerId(0)).expect("seat 0 is viewable");
    let (won_view, lost_view) = (view(&won), view(&lost));
    if !view_is_terminal(&won_view) || !view_is_terminal(&lost_view) {
        return Err("fixture: both views must be finished games".into());
    }
    // Anti-vacuity, and deliberately NOT read off `terminal_standing`: the
    // authoring mutation below rewrites exactly that function, and a fixture
    // guard that consumed its output would fire first and mask the property.
    // `ScoreStructure::final_total` is the untouched arithmetic.
    let totals = |v: &PlayerView| -> (i64, i64, i64) {
        let s = ScoreStructure::derive(v, rules);
        (s.base_vp[0], s.final_total(0), s.final_total(1))
    };
    let (won_running, won_mine, won_theirs) = totals(&won_view);
    let (lost_running, lost_mine, lost_theirs) = totals(&lost_view);
    if !(won_mine > won_theirs && lost_mine < lost_theirs && lost_running > won_running) {
        return Err(format!(
            "fixture: the viewer must WIN the first game on bonuses ({won_mine} vs \
             {won_theirs}) and LOSE the second while LEADING on RUNNING VP ({lost_running} \
             against the first game's {won_running}; finals {lost_mine} vs {lost_theirs})"
        ));
    }
    let standing = |v: &PlayerView| terminal_standing(v, rules).expect("a terminal view stands");
    let shipped = (|| -> Result<(), String> {
        let (a, b) =
            (score(&won_view, rules, &WeightTable::V1), score(&lost_view, rules, &WeightTable::V1));
        require(
            a > b,
            format!(
                "the terminal the viewer WINS ({won_running} running VP + a 12-VP country set \
                 = {won_mine}) must outrank the terminal it loses ({lost_running} running VP \
                 against {lost_theirs}): won {a}, lost {b}"
            ),
        )?;
        // The same claim read off the standing rather than off the ladder — the
        // engine agreed on both winners above, so the mirror must agree on both
        // places. Second, not first, so that the SCORE line is what a broken
        // ranking reports.
        let (won_rank, lost_rank) = (standing(&won_view).rank, standing(&lost_view).rank);
        require(
            (won_rank, lost_rank) == (1, 2),
            format!(
                "the view-side standing must place the viewer 1st in the game it wins and 2nd \
                 in the game it loses, got {won_rank} and {lost_rank}"
            ),
        )
    })();

    // The rejected reading, computed locally: the same ladder over BASE VP
    // instead of the full final total.
    let by_running_vp = |v: &PlayerView| -> i64 {
        let structure = ScoreStructure::derive(v, rules);
        let mine = structure.base_vp[usize::from(v.viewer.0)];
        let best_other = v
            .players
            .iter()
            .filter(|p| p.id != v.viewer && !p.id.is_dummy())
            .map(|p| structure.base_vp[usize::from(p.id.0)])
            .max()
            .unwrap_or(mine);
        let rank = 1 + u8::from(best_other > mine);
        TERMINAL_RANK_STEP * place_signal(rank) + TERMINAL_MARGIN_STEP * (mine - best_other)
    };
    let mutated = {
        let (a, b) = (by_running_vp(&won_view), by_running_vp(&lost_view));
        require(a > b, format!("ranked on running VP alone: won {a}, lost {b}"))
    };

    admit(
        SCENARIOS[8].0,
        "terminal ranking on running VP alone",
        "ranked on running VP alone",
        shipped,
        mutated,
    )
}

/// Force `state` into the shape the engine calls finished — end-game
/// triggered, final round, and the turn resting at the end of the last seat's
/// turn (`tests/eval_core.rs`'s builder verbatim).
fn force_terminal(state: &mut GameState) {
    let n = state.config.player_count;
    state.end_game_triggered = true;
    state.final_round = true;
    state.trigger_reason = Some(EndGameTrigger::ChurchTrackFull);
    state.trigger_player = Some(state.config.start_player);
    state.current_player = PlayerId((state.config.start_player.0 + n - 1) % n);
    state.turn_phase = TurnPhase::TurnEnd;
    assert!(is_game_over(state), "the fixture must actually be a finished game");
}

#[test]
fn scenario_09_terminal_scoring() {
    run(8, "terminal scoring");
}

// ---------------------------------------------------------------------------
// 10. Early engine over early VP  (3.5c round 1 — the early re-mix)
// ---------------------------------------------------------------------------

/// The PRE-V2 early mix, restored inside the candidate table: the five early
/// slots this scenario is built on, put back to [`WeightTable::V1`]'s values, and
/// nothing else touched.
///
/// This is §0.2's targeted mutation for scenario 10, in the shape an ADOPTION
/// round needs. Suite v1's mutations zero one term because a v1 scenario asks
/// "does the shipped table price this term at all"; a re-pricing scenario asks
/// "does the round's own edit carry this position", so the mutation is the edit
/// undone. Zeroing a term here would prove only that the term matters — which
/// scenarios 1, 2 and 8 already prove for three of these five — and would say
/// nothing about the re-mix.
///
/// **RE-DOCUMENTED AT ROUND 2, and the meaning has genuinely changed** (round
/// 1's Task-F review NOTE-4, which asked for exactly this re-visit if a later
/// round moved these slots again — it did). At round 1 this helper reverted ONE
/// round's edit, so "restore V1" and "undo round 1" named the same thing. Round
/// 2's early extension then moved THREE of these five slots a second time
/// (`vp 2→1`, `ctry 18→20`, `tech 12→14`), so what the helper restores today is
/// **the ORIGINAL (V1) early mix — "the mix the evaluator was frozen with", not
/// "round 1 undone"**. The code is unchanged and deliberately so: `V1` is the
/// right authority either way, because the scenario's claim is about the
/// evaluator's early RATIOS against the ones T4 froze, and every round that
/// re-mixes them has to answer here. The margin the scenario reports grows with
/// each such round rather than resetting (+165 at round 1, +205 at round 2), and
/// the counterfactual line below is INVARIANT across them — it is computed from
/// V1's five early weights, which no round moves.
///
/// Each slot's LATE weight is taken from the DRAFT, not from `V1`, so the
/// mutation is exactly the EARLY re-mix and not a whole-column revert. (For
/// these five the two late values still coincide — neither round has moved their
/// late entries — so this is a statement about what the code MEANS rather than
/// about what it currently computes, and it stays correct the day a round moves
/// one of their late weights.)
fn early_remix_reverted() -> WeightTable {
    let mut w = WeightTable::V2_DRAFT;
    for feature in EARLY_REMIX_SLOTS {
        let (v1_early, _) = WeightTable::V1.get(feature);
        let (_, draft_late) = WeightTable::V2_DRAFT.get(feature);
        w.set(feature, v1_early, draft_late);
    }
    w
}

/// The five EARLY slots round 1 re-mixed, in [`Feature::ALL`] order — that
/// round's change-set (a) made a list, and the exact set scenario 10's fixture
/// is built to separate on. Round 2's early extension re-priced three of them
/// again (`vp`, `ctry`, `tech`) without widening or narrowing the set, which is
/// why this list did not move with it.
const EARLY_REMIX_SLOTS: [Feature; 5] = [
    Feature::VictoryPoints,
    Feature::CountryBonusNow,
    Feature::TechBonusNow,
    Feature::AssistantsInHandExcess,
    Feature::AssistantsOnBoard,
];

/// **RULES.** Nothing an early position banks is worth what it will be worth at
/// the end, and the two piles that pay at the end are face-up: completed Orders
/// and owned Technologies carry the COUNTRY ICONS a set is formed from
/// (2/3/4/5 distinct countries = 2/4/8/12 VP), and owned Technologies also carry
/// the printed GEAR values the 10/5 technology majority is ranked on — where at
/// two players the setup dummy competes at industry value exactly 7.5 (D5, owner
/// ruling 2026-08-08, rulebook p.9 final-scoring NOTE). Running VP, by contrast,
/// is already banked and compounds no further. Assistants placed FROM HAND buy
/// action power now; assistants held past the reserve are tempo
/// ([`HAND_CORE_RESERVE`]).
///
/// **SEAT.** 0, in a 2-player game, at `p = 0` — the EARLY vector exactly.
///
/// **PROPERTY.** `score(the ENGINE position)` > `score(the VP-AND-BOARD leader)`
/// under [`WeightTable::V2_DRAFT`], in two clauses. (a) The two positions
/// separate on EXACTLY the five slots round 1 re-mixed
/// ([`EARLY_REMIX_SLOTS`]) and on no other term — the fixture's own leak check,
/// and what makes clause (b) a statement about the re-mix rather than about
/// whatever else the doctoring moved. (b) The engine position outranks.
///
/// **THE ARITHMETIC, stated because the margin is the point.** Both positions
/// share one base view, so the margin is `Σ w(0) · (v_engine − v_leader)` over
/// the separating terms alone. The five deltas are `vp −12`, `country_bonus_now
/// +4` (three distinct country icons = a 3-set), `tech_bonus_now +10` (industry
/// 9 doubles to 18, clear of the 2P dummy's 15, so the majority's first place is
/// taken outright while the leader owns no technology and does not place at
/// all), `assistants_in_hand_excess +3`, `assistants_on_board −3`. Under the
/// CURRENT candidate (round 2's `M*`) that reads
/// `1·(−12) + 20·4 + 14·10 + 5·3 + 6·(−3) = −12 + 80 + 140 + 15 − 18 = +205`;
/// under V1's early weights it reads
/// `10·(−12) + 12·4 + 6·10 + 4·3 + 14·(−3) = −120 + 48 + 60 + 12 − 42 = −42`.
/// The same two positions, ranked opposite ways by the re-mix — which is what
/// "the re-mix moves the MIX, not the scale" buys (R0 finding F8: halving the
/// whole early vector is play-identical, so only a change of RATIOS can move a
/// decision).
///
/// **The margin's history, since two rounds have now moved these ratios.** Round
/// 1 landed it at
/// `2·(−12) + 18·4 + 12·10 + 5·3 + 6·(−3) = +165`; round 2's early extension
/// (`vp 2→1`, `ctry 18→20`, `tech 12→14`) took it to `+205`. Each round pushes
/// the same direction, so the scenario does not merely survive the second
/// round — it is one of the two places round 2's early component is guarded at
/// all (the other being the round's own L2/L4 evidence). The counterfactual is
/// UNCHANGED at −42 across both, because it is computed from V1's five early
/// weights.
///
/// **FIXTURE HONESTY — the assistant total.** Both positions carry the engine's
/// own 20 tokens (leader 6 on board + 2 in hand + 12 warehouse; engine 3 + 5 +
/// 12), so unlike scenario 6 this pair needs no total caveat. The doctoring that
/// IS unreachable-as-written is the leader's 12 running VP beside an empty
/// completed-order pile — a real seat would have banked those VP through cards
/// that also carry icons. That is deliberate: pricing the two piles TOGETHER is
/// what the round is arguing about, so the fixture separates them.
///
/// **WHY.** R0's headline finding, and the roundest number in the whole report:
/// EVAL TOTAL early measures **0.5314** in the SHIPPED regime — near chance —
/// and the two terms it is most weighted on are the two the corpus
/// CONTRADICTED, `victory_points` at 0.4651 and `assistants_on_board` at 0.4464
/// (`docs/plans/2026-08-09-phase3.5c-r0-report.md` findings F1 and F2: both
/// point the wrong way early, i.e. the early leader on those terms is the seat
/// that goes on to LOSE). The owner independently confirmed the reading as
/// canon, in as many words — early VP leaders lose to engine builders, "it
/// holds" (guidingpath's 2026-08-09 canon-batch row). The L2 pass then picked
/// the magnitudes: `m22` is the held-out EARLY argmax on the shipped-regime
/// corpus and the best cell on all six corpus×stage columns
/// (`docs/plans/2026-08-09-phase3.5c-round1-evidence.md`), and round 2's
/// pre-registered follow-up step moved three of them one notch further along the
/// same monotone axis, improving held-out EARLY again
/// (`docs/plans/2026-08-09-phase3.5c-round2-evidence.md`). This scenario is
/// protocol §4.2 rung L3 for that change: the DECISION the re-priced ratios
/// make, pinned so a later round cannot quietly undo it without answering here.
///
/// **What it does NOT claim.** Not that the engine position WINS the game — this
/// is one constructed pair, not a corpus, and the corpus evidence is an AUC over
/// held-out games, quoted where it lives. What is pinned is that the shipped
/// rung, handed these two positions early, prefers the builder.
///
/// **COUNTERFACTUAL — the five re-mixed EARLY slots restored to the PRE-V2 (V1)
/// mix (`vp 10, ctry 12, tech 6, excess 4, board 14`;
/// [`early_remix_reverted`], whose doc records why "restore V1" and "undo round
/// 1" stopped being the same sentence at round 2).** Observed failing line,
/// verbatim — RE-VERIFIED unchanged at the round-2 change-set commit, which is
/// itself a small fact about the fixture: the five reverted weights are V1's
/// either way, and every other term cancels between the two positions:
///
/// ```text
/// early ENGINE over early VP: the country/tech/spare-hand position must
/// outrank the VP-and-board leader in the EARLY vector — engine 138, leader
/// 180, margin -42 (early weights: vp 10, country 12, tech 6, excess 4,
/// board 14)
/// ```
///
/// The two absolute scores carry the base view's shared terms; the MARGIN is
/// the figure the arithmetic above predicts, and it reads −42 exactly.
fn early_engine_over_early_vp(rules: &Rules) -> Result<(), String> {
    let techs = one_tech_per_country(rules, 3);
    if techs.len() != 3 {
        return Err(format!(
            "fixture: three countries must each print a technology, got {} — the country \
             registry or the tech data moved",
            techs.len(),
        ));
    }
    let industry: u32 =
        techs.iter().filter_map(|id| rules.techs.get(*id)).map(|c| u32::from(c.industry_cost)).sum();
    // D5's dummy contends at industry 7.5, compared as a doubled integer (15).
    // The holding must clear it, or the majority this scenario is about is the
    // dummy's and the tech delta collapses from 10 to 5.
    if 2 * industry <= 15 {
        return Err(format!(
            "fixture: the three technologies must out-industry the 2P dummy's 7.5 (D5), \
             their printed gears total {industry}"
        ));
    }

    let mut base = base_view(2, 19, rules);
    // Set explicitly rather than taking the deal: two of the twelve achievement
    // types count completed ORDERS and owned TECHNOLOGIES, and either would give
    // the engine position a sixth separating term and break clause (a) for a
    // reason that has nothing to do with the re-mix.
    base.achievements = [
        AchievementCard::new(AchievementType::Copper5),
        AchievementCard::new(AchievementType::Silk6),
        AchievementCard::new(AchievementType::Tea7),
    ];
    let area = *base.board_layout.areas.keys().next().ok_or("fixture: an empty board")?;
    {
        let me = viewer_mut(&mut base);
        me.completed_orders.clear();
        me.technologies.clear();
        me.victory_points = 0;
    }

    // The VP-and-board leader: 12 banked VP and six assistants working, holding
    // exactly the reserve in hand.
    let mut leader = base.clone();
    {
        let me = viewer_mut(&mut leader);
        me.victory_points = 12;
        me.assistants.in_hand = HAND_CORE_RESERVE;
        me.assistants.on_board = BTreeMap::from([(area, 6u8)]);
    }
    // The engine: three technologies (three country icons AND the industry
    // majority), three assistants working, three spare in hand past the reserve.
    let mut engine = base.clone();
    {
        let me = viewer_mut(&mut engine);
        me.technologies = techs.clone();
        me.assistants.in_hand = HAND_CORE_RESERVE + 3;
        me.assistants.on_board = BTreeMap::from([(area, 3u8)]);
    }

    // The stage is the scenario's premise, so it is asserted: at p = 0 the
    // weights ARE the early vector, with no interpolation to reason about.
    let (pa, pb) = (progress_ppm(&engine, rules), progress_ppm(&leader, rules));
    if (pa, pb) != (0, 0) {
        return Err(format!(
            "fixture: both positions must sit at p=0 so the EARLY vector governs exactly, \
             got {pa} and {pb}"
        ));
    }
    // The three end-scoring values the re-mix aims weight at, read off the
    // structure rather than assumed from the doctoring.
    let built = [
        (Feature::CountryBonusNow, viewer_value(&engine, rules, Feature::CountryBonusNow), 4),
        (Feature::TechBonusNow, viewer_value(&engine, rules, Feature::TechBonusNow), 10),
        (Feature::CountryBonusNow, viewer_value(&leader, rules, Feature::CountryBonusNow), 0),
        (Feature::TechBonusNow, viewer_value(&leader, rules, Feature::TechBonusNow), 0),
    ];
    for (feature, got, want) in built {
        if got != want {
            return Err(format!(
                "fixture: {} must read {want} on the position it was built for, got {got}",
                feature.name(),
            ));
        }
    }
    // Anti-vacuity on the hand split: both positions must hold the FULL reserve,
    // so `assistants_in_hand_core` cancels and the only hand term that separates
    // them is the excess one the round re-priced.
    let core = |v: &PlayerView| viewer_value(v, rules, Feature::AssistantsInHandCore);
    if core(&engine) != core(&leader) {
        return Err(format!(
            "fixture: both positions must hold the whole {HAND_CORE_RESERVE}-token reserve so \
             the core term cancels, got {} and {}",
            core(&engine),
            core(&leader),
        ));
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let separating = separating_terms(&engine, &leader, rules, w);
        let expected: Vec<&str> = EARLY_REMIX_SLOTS.iter().map(|f| f.name()).collect();
        require(
            separating == expected,
            format!(
                "the two positions must separate on EXACTLY the five re-mixed early slots \
                 {expected:?}, they separated on {separating:?}"
            ),
        )?;
        let a = score(&engine, rules, w);
        let b = score(&leader, rules, w);
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, 0)
        };
        require(
            a > b,
            format!(
                "early ENGINE over early VP: the country/tech/spare-hand position must outrank \
                 the VP-and-board leader in the EARLY vector — engine {a}, leader {b}, margin \
                 {} (early weights: vp {}, country {}, tech {}, excess {}, board {})",
                a - b,
                at(Feature::VictoryPoints),
                at(Feature::CountryBonusNow),
                at(Feature::TechBonusNow),
                at(Feature::AssistantsInHandExcess),
                at(Feature::AssistantsOnBoard),
            ),
        )
    };

    admit(
        SCENARIOS[9].0,
        "the five re-mixed EARLY slots restored to the pre-V2 (V1) mix (vp 10, ctry 12, tech 6, \
         excess 4, board 14)",
        "must outrank the VP-and-board leader in the EARLY vector",
        check(&WeightTable::V2_DRAFT),
        check(&early_remix_reverted()),
    )
}

#[test]
fn scenario_10_early_engine_over_early_vp() {
    run(9, "early engine over early VP");
}

// ---------------------------------------------------------------------------
// 11. Warehouse decay flattened  (3.5c round 1 — protocol scenario 16, T2)
// ---------------------------------------------------------------------------

/// The round-1 WAREHOUSE FLATTEN, REVERTED inside the candidate table: the one
/// feature pair the round's change-set (b) moved, put back to
/// [`WeightTable::V1`]'s `(−10, −4)`, and nothing else touched.
///
/// §0.2's targeted mutation for scenario 11. The EARLY half of the pair is
/// −10 in both tables — the flatten is a LATE-only edit — so this reverts
/// exactly one number, which is as targeted as a mutation gets.
fn warehouse_decay_restored() -> WeightTable {
    let mut w = WeightTable::V2_DRAFT;
    let (early, late) = WeightTable::V1.get(Feature::AssistantsInWarehouse);
    w.set(Feature::AssistantsInWarehouse, early, late);
    w
}

/// **RULES.** Assistants live in exactly three places: the seat's HAND, the
/// main board, and the WAREHOUSE. Placements come from hand only, so a token in
/// the warehouse does nothing at all until it is taken out — and the only ways
/// out are the Employment Agency's printed 4/5/6/7 take progression, the
/// University additional action, and the warehouse-icon choice, each of which
/// costs a decision point that could have been spent acting. Nothing in the
/// rules ever moves a token INTO the warehouse mid-game, so a clogged warehouse
/// is a seat that never spent those decisions (KB `01-rules-reference.md`;
/// design §5.2's take progression).
///
/// **SEAT.** 0, in a 2-player game, with the order deck run out so the DECK
/// clock reads `p = 1,000,000` — the LATE vector exactly. (A view at full
/// progress is not a finished game: the real trigger also needs an empty Port
/// slot, a conjunct [`clock_ppm`] deliberately drops, and the position is scored
/// positionally rather than through the terminal ladder.)
///
/// **PROPERTY.** `score(the CLEAN warehouse)` > `score(the CLOGGED warehouse)`
/// under [`WeightTable::V2_DRAFT`], in two clauses: (a) the two positions
/// separate on exactly `assistants_in_hand_excess`, `assistants_in_warehouse`
/// and `imported_goods` — the leak check — and (b) the clean position outranks.
///
/// **THE ARITHMETIC, and why the flatten is the ONLY thing that decides it.**
/// Both positions carry the engine's own 20 tokens, so the six tokens that are
/// stranded in one are IN HAND in the other: clean holds `warehouse 2 / hand 18`
/// against clogged's `warehouse 8 / hand 12`, and the clogged seat is paid for
/// its stranding with six more imported goods. The margin is
/// `w_excess·(16−10) + w_warehouse·(2−8) + w_imported·(0−6)`. Two of those three
/// weights are IDENTICAL in the candidate and in the reverted table (late excess
/// 2, late imported 8), so they contribute `+12 − 48 = −36` either way and
/// cannot decide anything; the whole decision is the third term, which reads
/// `(−10)·(−6) = +60` flattened and `(−4)·(−6) = +24` decayed. Totals **+24**
/// and **−12** — one number, one sign flip.
///
/// **WHY.** T2, the warehouse-decay tension, is one of the two magnitude
/// tensions the T4 freeze RECORDED rather than resolved, and its own words are
/// the argument: the term's separation *"does not decay late (0.2742), which the
/// −10 → −4 weight decay does. Sign confirmed, magnitude shape unsupported and
/// left alone by the one-pass rule; recorded for 3.5"*
/// ([`WeightTable`]'s doc, from `docs/plans/2026-07-27-phase3.3-weight-freeze.md`).
/// That quote is the rev-5 record; the REV-7 corpora reproduce the shape and are
/// what the disposition actually rests on (protocol §2's rule): greedy
/// |AUC−.5| late 0.223 vs early 0.175, lookahead late 0.190 vs early 0.161 —
/// non-decaying in BOTH regimes (`docs/plans/2026-08-09-phase3.5c-r0-report.md`).
/// Round 1 is the 3.5 the note was deferring to, and the L2 pass chose the
/// flatten from a pre-registered three-point grid (`−8/−8`, `−10/−10`, `−10/−7`)
/// on held-out AUC — `m22` holds the early magnitude and undoes the late decay
/// (`docs/plans/2026-08-09-phase3.5c-round1-evidence.md`). The DECISION content
/// is this: late, a rung with the decayed weight will accept a stranded
/// warehouse for a modest consolation elsewhere, and a rung with the flat weight
/// will not.
///
/// **COUNTERFACTUAL — the warehouse pair restored to V1's `(−10, −4)`
/// ([`warehouse_decay_restored`]).** Observed failing line, verbatim:
///
/// ```text
/// warehouse decay flattened: the CLEAN warehouse must outrank the CLOGGED one
/// that paid for its stranding with 6 imported goods — clean 60, clogged 72,
/// margin -12 (late weights: warehouse -4, excess 2, imported 8)
/// ```
fn warehouse_decay_flattened(rules: &Rules) -> Result<(), String> {
    let mut base = base_view(2, 20, rules);
    // The deck clock at full: `initial_order_deck_len` is DERIVED from the
    // registry, so emptying the view's deck is what dates the position, and the
    // maximum over the four clocks is then 1,000,000 whatever the tracks read.
    base.order_deck_len = 0;
    {
        let me = viewer_mut(&mut base);
        me.assistants.on_board = BTreeMap::new();
        me.imported_goods = 0;
    }

    // Twenty tokens either way — the six the clogged seat left stranded are the
    // six the clean seat is holding.
    let with_warehouse = |stranded: u8, imported: u32| -> PlayerView {
        let mut v = base.clone();
        let me = viewer_mut(&mut v);
        me.assistants.in_warehouse = stranded;
        me.assistants.in_hand = 20 - stranded;
        me.imported_goods = imported;
        v
    };
    let clean = with_warehouse(2, 0);
    let clogged = with_warehouse(8, 6);

    let total = |v: &PlayerView| -> u32 {
        let me = v.players.iter().find(|p| p.id == v.viewer).expect("own row");
        u32::from(me.assistants.in_warehouse)
            + u32::from(me.assistants.in_hand)
            + me.assistants.on_board.values().map(|n| u32::from(*n)).sum::<u32>()
    };
    if total(&clean) != total(&clogged) {
        return Err(format!(
            "fixture: the two positions must hold the SAME tokens in different places, got \
             {} and {} — otherwise the clogged seat is simply a richer one",
            total(&clean),
            total(&clogged),
        ));
    }
    let (pa, pb) = (progress_ppm(&clean, rules), progress_ppm(&clogged, rules));
    if (pa, pb) != (PROGRESS_FULL_PPM, PROGRESS_FULL_PPM) {
        return Err(format!(
            "fixture: both positions must sit at full progress so the LATE vector governs \
             exactly, got {pa} and {pb}"
        ));
    }
    if view_is_terminal(&clean) || view_is_terminal(&clogged) {
        return Err("fixture: a full-progress view must still be scored POSITIONALLY — the \
                    terminal ladder short-circuits the weight vector this scenario is about"
            .into());
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let separating = separating_terms(&clean, &clogged, rules, w);
        require(
            separating
                == ["assistants_in_hand_excess", "assistants_in_warehouse", "imported_goods"],
            format!(
                "the two positions must separate on the stranded tokens, the hand they came \
                 from, and the goods that paid for the stranding — they separated on \
                 {separating:?}"
            ),
        )?;
        let a = score(&clean, rules, w);
        let b = score(&clogged, rules, w);
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, PROGRESS_FULL_PPM)
        };
        require(
            a > b,
            format!(
                "warehouse decay flattened: the CLEAN warehouse must outrank the CLOGGED one \
                 that paid for its stranding with 6 imported goods — clean {a}, clogged {b}, \
                 margin {} (late weights: warehouse {}, excess {}, imported {})",
                a - b,
                at(Feature::AssistantsInWarehouse),
                at(Feature::AssistantsInHandExcess),
                at(Feature::ImportedGoods),
            ),
        )
    };

    admit(
        SCENARIOS[10].0,
        "the warehouse pair restored to V1's (-10, -4)",
        "the CLEAN warehouse must outrank the CLOGGED one",
        check(&WeightTable::V2_DRAFT),
        check(&warehouse_decay_restored()),
    )
}

#[test]
fn scenario_11_warehouse_decay_flattened() {
    run(10, "warehouse decay flattened");
}

// ---------------------------------------------------------------------------
// 12. Post-claim achievement progress  (3.5c round 1 — protocol scenario 17, T3)
// ---------------------------------------------------------------------------

/// **RULES.** The three Achievement cards in play are claimed by placing an
/// assistant on the card once its printed requirement is met; the claim pays its
/// printed VP and the goods are NOT spent, so a seat keeps holding whatever it
/// held. A seat may not claim the SAME card twice — the engine's claim gate
/// refuses exactly that repeat and nothing else — while a SECOND seat may still
/// claim the same card for the printed subsequent value (KB
/// `01-rules-reference.md`, achievement categories; the gate is
/// `can_claim_achievement`).
///
/// **SEAT.** 0, in a 2-player game, holding a claimed Tea 7 beside an unclaimed
/// Seafood 7 — two cards with the SAME printed threshold, which is what lets the
/// two positions carry identical shipped progress.
///
/// **PROPERTY**, two clauses over the CANDIDATE FEATURE FUNCTION rather than
/// over a weight (see below for why). (i) **The ordering content:** a position
/// holding LIVE progress toward the unclaimed card reads strictly above an
/// otherwise-identical position holding only DEAD progress on the card this seat
/// has already claimed — 7 against 0 — while the SHIPPED unfiltered sum reads
/// both positions **7**, i.e. cannot tell them apart at all. (ii) **The
/// arithmetic identity:** [`achievement_progress_postclaim_sum`] equals the
/// shipped sum minus exactly the claimed card's CAPPED leg, on both positions
/// (`0 = 7 − 7` on the dead one, `7 = 7 − 0` on the live one), which is what
/// says the filter drops a leg and rewrites no other arithmetic.
///
/// Clause (i) is checked FIRST on purpose, and scenario 9's doc records the same
/// reasoning for the same reason: the counterfactual substitutes the shipped sum
/// for the candidate one, which breaks clause (ii) as well — so a fixture that
/// asked the identity first would report the identity's line and the
/// counterfactual would be admitted on the wrong clause.
///
/// **WHY THIS SCENARIO OWNS NO WEIGHT, and what happens at the freeze.** T3's
/// post-claim filter is the ONE sanctioned feature change of the v2 protocol
/// (§4.1(ii), §5's T3 row) and it lands in round 1 **additively**: the function
/// exists, nothing calls it, `score` and `explain` still read the shipped sum,
/// and `EVAL_IMPL_VERSION` does not move. The call-site switch and its blast
/// radius — goldens, drift pins, the version bump, every controller const that
/// embeds the evaluator — are the FREEZE commit's business (protocol §7's
/// mechanics), so the switch stays one auditable event instead of a behavior
/// change smuggled inside a re-pricing round. **This scenario re-points to
/// `score` there:** clause (i) becomes `score(live) > score(dead)`, and the
/// additive-only guard below (which asserts today that the shipped score ranks
/// the two EQUAL) becomes the thing that has to be rewritten rather than
/// deleted. The re-point is the never-delete rule working as designed, and it is
/// written down here so the freeze commit does not have to rediscover it.
///
/// **WHY the change at all.** The shipped sum keeps paying a seat for progress
/// toward a card it has already taken. The cap stops that leg GROWING; it does
/// not stop it being COUNTED, and the VP the claim paid has already landed in
/// `victory_points`, which [`Feature::VictoryPoints`] already prices — so the
/// evaluator charges the same banked reward twice and, worse, cannot see the
/// difference between a seat with a live claim in front of it and a seat with
/// none. The corpus column is supporting rather than load-bearing and is read
/// honestly in `docs/plans/2026-08-09-phase3.5c-round1-evidence.md`: under the
/// shipped LOOKAHEAD regime the filtered column separates better EARLY and MID
/// (0.5198 and 0.5358, against the unfiltered feature's no-signal 0.4996 and
/// 0.5024) and worse LATE (0.5312 against 0.5601) — and that late advantage IS
/// the double count, progress at cap on cards already claimed proxying for
/// banked VP, so it is the complaint rather than a defence. T3 therefore stands
/// on the structural argument, which is protocol §4.2 L1's named-structural-
/// argument arm, with the early/mid improvement as regime evidence beside it.
///
/// **SCOPE — the filter is the VIEWER'S OWN claim, and that is a decision.** A
/// card an OPPONENT has claimed leaves this seat's leg fully live, because the
/// rules let a second seat claim the same card for the printed subsequent value;
/// zeroing it would price a reachable reward at nothing. Pinned against the
/// engine's own gate in `tests/eval_core.rs`, not against a reading of the
/// rulebook, and exercised here by the third card in play.
///
/// **COUNTERFACTUAL — the SHIPPED unfiltered sum substituted for the candidate
/// one.** A source-shaped mutation rather than a weight one, because no weight
/// can reach a function nothing calls: the property is evaluated twice, once
/// through [`achievement_progress_postclaim_sum`] and once through the value
/// `ScoreStructure` computes for [`Feature::AchievementProgressSum`], which IS
/// the shipped arithmetic. Observed failing line, verbatim:
///
/// ```text
/// post-claim achievement progress (the SHIPPED unfiltered sum): LIVE progress
/// toward an unclaimed card must outrank DEAD progress on a card this seat has
/// already claimed — live 7, dead 7, and the shipped feature reads both 7
/// ```
fn post_claim_achievement_progress(rules: &Rules) -> Result<(), String> {
    let mut base = base_view(2, 21, rules);
    let mut claimed = AchievementCard::new(AchievementType::Tea7);
    claimed.claimed_by = vec![PlayerId(0)];
    // Tea 7 and Seafood 7 print the SAME threshold, so the two positions below
    // can carry identical shipped progress on different cards. Money 10 is the
    // third card in play and is untouched by either position — it is what makes
    // the filtered sum a SUM over the cards rather than a single leg.
    base.achievements = [
        claimed,
        AchievementCard::new(AchievementType::Seafood7),
        AchievementCard::new(AchievementType::Money10),
    ];
    {
        let me = viewer_mut(&mut base);
        me.money = 0;
        me.copper = 0;
        me.silk = 0;
        me.tea = 0;
        me.seafood = 0;
    }

    let mut dead = base.clone();
    viewer_mut(&mut dead).tea = 7;
    let mut live = base.clone();
    viewer_mut(&mut live).seafood = 7;

    let filtered = |v: &PlayerView| achievement_progress_postclaim_sum(v, own_row(v));
    let unfiltered = |v: &PlayerView| viewer_value(v, rules, Feature::AchievementProgressSum);
    // The claimed card's own capped leg, per position — the quantity clause (ii)
    // says the filter subtracts.
    let claimed_leg = |v: &PlayerView| -> i64 {
        let (current, needed) = achievement_progress(v, own_row(v), AchievementType::Tea7);
        i64::from(current.min(needed))
    };

    // THE FIXTURE'S PREMISE, and the blindness this scenario exists to make
    // visible: the shipped feature values the two positions identically.
    let (u_live, u_dead) = (unfiltered(&live), unfiltered(&dead));
    if u_live != u_dead || u_dead == 0 {
        // This guard runs FIRST, so it is also the line the FREEZE author will
        // meet: switching Feature::AchievementProgressSum onto the filtered sum
        // makes `unfiltered` read the filter through the call site and trips
        // this premise before the additive-only guard below is reached (Task-F
        // review SHOULD-1). Hence the instruction rides here too.
        return Err(format!(
            "fixture: the shipped sum must value the two positions EQUALLY and nonzero (that \
             equality is the double-count blindness the filter removes), got {u_live} and \
             {u_dead}. If this fired because the freeze switched the feature's call site onto \
             the filtered sum: RE-POINT this scenario at `score` per its doc — never delete it"
        ));
    }
    // The claimed leg must be AT ITS CAP on the dead position, or "dead progress"
    // is really "progress that could still grow".
    let (current, needed) = achievement_progress(&dead, own_row(&dead), AchievementType::Tea7);
    if current < needed {
        return Err(format!(
            "fixture: the claimed card's leg must sit AT its cap on the dead position, got \
             {current} of {needed}"
        ));
    }
    if !base.achievements[0].claimed_by.contains(&PlayerId(0))
        || !base.achievements[1].claimed_by.is_empty()
    {
        return Err("fixture: card 0 must be claimed by the viewer and card 1 unclaimed".into());
    }
    // THE ADDITIVE-ONLY GUARD (protocol §4.1(ii)): nothing calls the candidate
    // function yet, so the shipped evaluator must rank these two positions
    // EXACTLY equal. **AT THE FREEZE COMMIT this becomes `score(&live) >
    // score(&dead)` and clause (i) re-points onto `score`** — see the doc.
    let (s_live, s_dead) =
        (score(&live, rules, &WeightTable::V2_DRAFT), score(&dead, rules, &WeightTable::V2_DRAFT));
    if s_live != s_dead {
        return Err(format!(
            "the T3 filter is ADDITIVE in this round: `score` must still read the shipped \
             unfiltered sum and rank these two positions equal, got {s_live} and {s_dead} — if \
             the call site has legitimately switched, re-point this guard and clause (i) onto \
             `score` (protocol §7's mechanics), do not delete them"
        ));
    }

    let check = |sum: &dyn Fn(&PlayerView) -> i64, label: &str| -> Result<(), String> {
        let (l, d) = (sum(&live), sum(&dead));
        require(
            l > d,
            format!(
                "post-claim achievement progress ({label}): LIVE progress toward an unclaimed \
                 card must outrank DEAD progress on a card this seat has already claimed — \
                 live {l}, dead {d}, and the shipped feature reads both {u_dead}"
            ),
        )?;
        for (name, view, value) in [("live", &live, l), ("dead", &dead, d)] {
            let (shipped, leg) = (unfiltered(view), claimed_leg(view));
            require(
                value == shipped - leg,
                format!(
                    "the post-claim sum must be the shipped sum minus exactly the claimed \
                     card's capped leg on the {name} position: {value} against {shipped} − {leg}"
                ),
            )?;
        }
        Ok(())
    };

    admit(
        SCENARIOS[11].0,
        "the SHIPPED unfiltered achievement_progress_sum substituted for the candidate",
        "must outrank DEAD progress on a card this seat has already claimed",
        check(&filtered, "the candidate post-claim filter"),
        check(&unfiltered, "the SHIPPED unfiltered sum"),
    )
}

#[test]
fn scenario_12_post_claim_achievement_progress() {
    run(11, "post-claim achievement progress");
}

// ---------------------------------------------------------------------------
// 13. Late bonus re-pricing  (3.5c round 2 — the church/five-power crossing)
// ---------------------------------------------------------------------------

/// The two LATE slots round 2's change-set CROSSED, in [`Feature::ALL`] order —
/// the round's dominant pair made a list, and the exact set scenario 13's
/// fixture is built to separate on.
///
/// `church_bonus_now` late `16 → 8` (the C-axis winner, c1) and
/// `five_power_tokens` late `10 → 14` (c4) are one decision here rather than
/// two, because the scenario's whole margin is a statement about their ORDER:
/// under the round-1 table the church weight is the larger of the pair, under
/// the round-2 candidate the five-power weight is.
const LATE_REPRICED_SLOTS: [Feature; 2] = [Feature::ChurchBonusNow, Feature::FivePowerTokens];
/// The round-2 LATE re-pricing, REVERTED inside the candidate table: the two
/// slots of [`LATE_REPRICED_SLOTS`], put back to the prices the round-1 table
/// `m22` carried, and nothing else touched.
///
/// §0.2's targeted mutation for scenario 13, in the shape an ADOPTION round
/// needs — the round's own edit undone, exactly as [`early_remix_reverted`] and
/// [`warehouse_decay_restored`] are for rounds 1's two re-pricings.
///
/// **Where the reverted numbers come from, and why that is exact.** `m22` is not
/// a constant in this tree — it was the previous VALUE of
/// [`WeightTable::V2_DRAFT`], which round 2's adoption commit overwrote — so the
/// in-source authority for the two reverted late weights is
/// [`WeightTable::V1`], and it is the RIGHT authority here because round 1 moved
/// NEITHER of these slots: `m22` inherited `church_bonus_now` late 16 and
/// `five_power_tokens` late 10 from V1 unchanged, so "restore V1's late values"
/// and "restore m22's late values" name the same two numbers today.
///
/// **Read that as a dated statement, not a permanent one** (round 1's Task-F
/// review NOTE-4, generalized): the day a LATER round moves either slot again,
/// this helper reverts TWO rounds of edits rather than one, and its meaning
/// becomes "the ORIGINAL (V1) pricing" rather than "round 2 undone". That is
/// exactly what happened to [`early_remix_reverted`] at this round, and its doc
/// was re-written to say so rather than its code changed. Re-document this one
/// the same way when it happens.
///
/// Each slot's EARLY weight is taken from the DRAFT, not from `V1`, so the
/// mutation is exactly the LATE re-pricing and not a whole-column revert. (For
/// these two the early values coincide in both tables — round 2 moved only their
/// late entries — so this is a statement about what the code MEANS rather than
/// about what it currently computes.)
fn late_bonus_reprice_reverted() -> WeightTable {
    let mut w = WeightTable::V2_DRAFT;
    for feature in LATE_REPRICED_SLOTS {
        let (_, m22_late) = WeightTable::V1.get(feature);
        let (draft_early, _) = WeightTable::V2_DRAFT.get(feature);
        w.set(feature, draft_early, m22_late);
    }
    w
}

/// **RULES.** Two printed end-scoring rewards, priced against each other.
/// (a) The CHURCH majority pays 6 VP to the most pawns on the Church management
/// board and 3 to the second, a count tie broken by the RIGHTMOST occupied space
/// (rulebook p.9; `game/scoring.rs`) — and at two players the setup dummy on
/// faith 7 is a real competitor that can take a place, which is then awarded to
/// nobody (finding D1, guidingpath's 2026-07-21 rows). (b) A 5-POWER TOKEN is
/// taken by acting in an area at Power exactly 5 while that area's face-up token
/// is still there; taking it resolves the token's printed one-off effect and
/// leaves the area with none (`apply_move.rs`'s five-power arm, which refuses
/// any other power and any empty area). The effects are goods, imported goods or
/// yen and nothing else (`types/board.rs`'s `FivePowerEffect`, six variants), so
/// a held token is not itself worth VP — the held COUNT is a record of how many
/// areas this seat has dominated, and [`Feature::FivePowerTokens`] is that count.
///
/// **SEAT.** 0, in a 2-player game, with the order deck run out so the DECK
/// clock reads `p = 1,000,000` — the LATE vector exactly. (As in scenario 11, a
/// view at full progress is not a finished game: the real trigger also needs an
/// empty Port slot, a conjunct [`clock_ppm`] deliberately drops, so the position
/// is scored positionally rather than through the terminal ladder.)
///
/// **PROPERTY.** `score(the FIVE-POWER position)` > `score(the CHURCH-MAJORITY
/// position)` under [`WeightTable::V2_DRAFT`], in two clauses. (a) The two
/// positions separate on EXACTLY the two slots round 2 re-priced
/// ([`LATE_REPRICED_SLOTS`]) and on no other term — the fixture's own leak
/// check. (b) The five-power position outranks.
///
/// **THE FIXTURE IS AN EXCHANGE, which is what makes the arithmetic clean.** The
/// two positions hold the SAME table material with the two seats swapped. In the
/// church-majority position the viewer's lone church pawn sits on faith 10 and
/// the opponent's on faith 9 — one pawn each, so the majority is the D1 tie-break
/// and the viewer's rightmost space takes it (6 to the viewer, 3 to the
/// opponent, the dummy on faith 7 losing the same tie) — while the opponent
/// banks four 5-Power tokens to the viewer's one. The five-power position is
/// that position with BOTH holdings exchanged: the opponent on faith 10, the
/// viewer on faith 9, and the four tokens in the viewer's pile. Both positions
/// therefore put the same number of pawns on the same two spaces, so the
/// viewer's assistant commitment and the church clock are identical by
/// construction and there is no "who spent more" caveat to make.
///
/// **THE ARITHMETIC, stated because the margin is the point.** The evaluator is
/// relative, so each term reads `w · (mine − theirs)`. Church: `+3` in one
/// position and `−3` in the other. Five-power: `−3` and `+3`. Every other term is
/// shared. So
/// `score(five-power) − score(church) = 6 · (w_five_power − w_church)` at
/// `p = 1,000,000` — the margin is SIX TIMES THE GAP BETWEEN THE TWO LATE
/// WEIGHTS, and nothing else. Under the round-2 candidate that reads
/// `6 · (14 − 8) = +36`; under the round-1 prices it reads
/// `6 · (10 − 16) = −36`. The same two positions, ranked opposite ways by the
/// round's edit, because the edit CROSSES the two weights.
///
/// **WHY.** R0's shipped-regime evidence on both members, finding F7
/// (`docs/plans/2026-08-09-phase3.5c-r0-report.md`). Under lookahead —
/// the regime that ships — `church_bonus_now` LATE is **NO-SIGNAL at 0.5180
/// [0.4915, 0.5437]** while carrying a +16 late weight, more than the term this
/// scenario prices it against: the church race is near-universal in this regime
/// (ChurchTrackFull is
/// 5-for-5 across every corpus in existence), so the awarded bonus stops telling
/// winners from losers, and paying +16 for it prices a near-constant.
/// `five_power_tokens` LATE, by contrast, measures **0.8058 [0.7874, 0.8236]** —
/// the second-strongest late member of the whole table after `victory_points`
/// — on a weight of only 10. Round 2's pre-registered two-stage L2 then chose
/// the magnitudes: c1 (`chur_l 16→8`) is the round's dominant single, positive on
/// every column of both corpora, and c4 (`5pw_l 10→14`) is its second admission;
/// the merged candidate `M*` leads all six (corpus × stage) held-out columns
/// (`docs/plans/2026-08-09-phase3.5c-round2-evidence.md`). This scenario is
/// protocol §4.2 rung L3 for that pair: the DECISION the crossing makes, pinned
/// so a later round cannot quietly undo it without answering here.
///
/// **What the round's OTHER components ride, stated honestly.** Round 2's
/// change-set also softens `hand_size` (`−3/−6 → −1/−2`, c3) and extends the
/// early re-mix (`vp 2→1, ctry 18→20, tech 12→14`, d1). Neither gets a
/// discriminating fixture of its own here, and neither is claimed to have one:
/// c3 was admitted on a MARGINAL held-out read (+0.0001 on its home stage) that
/// the round's evidence artifact records as riding the pre-registered merge rule
/// rather than its own demonstration, so a scenario asserting a decision it
/// turns would be inventing evidence; d1's DIRECTION is already guarded — it
/// moves three of the five slots scenario 10 pins, and `M*` strengthens that
/// scenario's margin rather than weakening it (+165 → +205). Both ride the
/// round's L2 evidence and its L4 screens, which is what the ladder is for; this
/// scenario carries the change-set's L3 obligation for the two components that
/// DO have a demonstrable decision between them.
///
/// **What it does NOT claim.** Not that the five-power position wins the game —
/// this is one constructed exchange, not a corpus, and the corpus evidence is an
/// AUC over held-out games, quoted where it lives. What is pinned is that the
/// shipped rung, handed this exchange late, takes the tokens.
///
/// **COUNTERFACTUAL — the two re-priced LATE slots restored to m22's
/// `(church 16, five-power 10)` ([`late_bonus_reprice_reverted`]).** Observed
/// failing line, verbatim:
///
/// ```text
/// late bonus RE-PRICING: the five-power position must outrank the
/// church-majority position in the LATE vector — five-power -18, church 18,
/// margin -36 (late weights: church 16, five_power 10)
/// ```
///
/// The MARGIN is the figure the arithmetic above predicts and it reads −36
/// exactly. The two absolute scores are ±18 — symmetric around zero — because
/// the exchange is the fixture's ONLY doctoring and a fresh 2P view is otherwise
/// seat-symmetric, so every one of the other fourteen terms contributes exactly
/// 0 to both. `16·3 − 10·3 = +18` and its mirror is the whole of each score.
fn late_bonus_repricing_tokens_over_church(rules: &Rules) -> Result<(), String> {
    /// Faith 7 — the D1 dummy's 2P setup space, and the pawn both positions beat
    /// on the rightmost-space tie-break.
    const DUMMY_FAITH: u8 = 7;
    /// The winning and losing spaces of the one-pawn-each count tie.
    const HIGH_FAITH: u8 = 10;
    const LOW_FAITH: u8 = 9;
    /// The 5-Power holdings the two positions exchange.
    const TOKENS_HELD: u8 = 4;
    const TOKENS_TRAILING: u8 = 1;

    let mut base = base_view(2, 22, rules);
    // The deck clock at full dates both positions (scenario 11's device): the
    // church occupancy is IDENTICAL across the pair by construction, so it could
    // not date them differently, but the deck is what puts them in the LATE
    // vector at all.
    base.order_deck_len = 0;

    let dummy_index = ChurchTrack::faith_to_index(DUMMY_FAITH).expect("faith 7 is on the track");
    if !base.church_track.slots[dummy_index].is_some_and(|s| s.player.is_dummy()) {
        return Err(format!(
            "fixture: the 2P setup must place the church dummy on faith {DUMMY_FAITH} (D1) — \
             without it the count tie below is not the rule this scenario cites"
        ));
    }
    if base.church_track.total_assistants() != 0 {
        return Err("fixture: a fresh 2P church track must carry no REAL pawn".into());
    }

    // The five tokens the two seats hold between them come OFF the board, so the
    // doctored view stays coherent about a token pool that is finite by
    // construction. Nothing in the evaluator reads the board's token map — the
    // feature is a field read on the player — so this buys coherence, not a
    // measurement; it is done because a fixture that mints tokens is a fixture a
    // reader has to take on trust.
    let held_total = usize::from(TOKENS_HELD + TOKENS_TRAILING);
    let taken: Vec<_> = base
        .board_layout
        .five_power_tokens
        .iter()
        .filter(|(_, token)| token.is_some())
        .map(|(area, _)| *area)
        .take(held_total)
        .collect();
    if taken.len() != held_total {
        return Err(format!(
            "fixture: the 2P board must carry at least {held_total} face-up 5-Power tokens for \
             the two seats to have taken them, it carries {}",
            taken.len(),
        ));
    }
    for area in &taken {
        base.board_layout.five_power_tokens.insert(*area, None);
    }

    // The exchange: `church_first` takes the rightmost church space (and with it
    // the 6), `token_holder` banks the four 5-Power tokens. The two positions are
    // `(0, 1)` and `(1, 0)` — the same material, held the other way round.
    let position = |church_first: u8, token_holder: u8| -> PlayerView {
        let mut v = base.clone();
        for (faith, seat, arrival) in
            [(HIGH_FAITH, church_first, 1u8), (LOW_FAITH, 1 - church_first, 2u8)]
        {
            v.church_track.slots[ChurchTrack::faith_to_index(faith).expect("faith 4..=11")] =
                Some(ChurchSlot { player: PlayerId(seat), arrival_order: arrival });
        }
        for player in v.players.iter_mut() {
            if player.id == PlayerId(token_holder) {
                player.five_power_tokens = TOKENS_HELD;
            } else if player.id == PlayerId(1 - token_holder) {
                player.five_power_tokens = TOKENS_TRAILING;
            }
        }
        v
    };
    let church_lead = position(0, 1);
    let five_power = position(1, 0);

    // The stage is the scenario's premise, so it is asserted: at p = FULL the
    // weights ARE the late vector, and the DECK clock is the one that put them
    // there.
    for (label, view) in [("church-majority", &church_lead), ("five-power", &five_power)] {
        let p = progress_ppm(view, rules);
        let clocks = clock_ppm(view, rules);
        if p != PROGRESS_FULL_PPM || clocks[0] != PROGRESS_FULL_PPM {
            return Err(format!(
                "fixture: the {label} position must sit at full progress on the DECK clock so \
                 the LATE vector governs exactly, got p={p} with clocks {clocks:?}"
            ));
        }
        if view_is_terminal(view) {
            return Err(format!(
                "fixture: the {label} position must still be scored POSITIONALLY — the terminal \
                 ladder short-circuits the weight vector this scenario is about"
            ));
        }
    }

    // The exchange, made executable: both positions must occupy the SAME church
    // spaces with the same number of real pawns, so the pair differs in WHO holds
    // what and never in how much was spent.
    let occupied = |v: &PlayerView| -> Vec<usize> {
        v.church_track
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_some())
            .map(|(index, _)| index)
            .collect()
    };
    if occupied(&church_lead) != occupied(&five_power)
        || church_lead.church_track.total_assistants() != 2
    {
        return Err(format!(
            "fixture: the two positions must occupy the same church spaces with two REAL pawns \
             (the exchange swaps owners, never occupancy), got {:?} at {} real and {:?} at {} \
             real",
            occupied(&church_lead),
            church_lead.church_track.total_assistants(),
            occupied(&five_power),
            five_power.church_track.total_assistants(),
        ));
    }

    // Both seats' derived values, read off the structure rather than assumed
    // from the doctoring — this is where D1's tie-break and the token exchange
    // become facts instead of intentions.
    let value_of = |v: &PlayerView, seat: u8, feature: Feature| -> i64 {
        let row = v
            .players
            .iter()
            .find(|p| p.id == PlayerId(seat))
            .expect("a 2P view contains both seats");
        ScoreStructure::derive(v, rules).value_of(row, feature)
    };
    let built = [
        ("church-majority", &church_lead, Feature::ChurchBonusNow, 6, 3),
        ("five-power", &five_power, Feature::ChurchBonusNow, 3, 6),
        (
            "church-majority",
            &church_lead,
            Feature::FivePowerTokens,
            i64::from(TOKENS_TRAILING),
            i64::from(TOKENS_HELD),
        ),
        (
            "five-power",
            &five_power,
            Feature::FivePowerTokens,
            i64::from(TOKENS_HELD),
            i64::from(TOKENS_TRAILING),
        ),
    ];
    for (label, view, feature, mine, theirs) in built {
        let (got_mine, got_theirs) = (value_of(view, 0, feature), value_of(view, 1, feature));
        if (got_mine, got_theirs) != (mine, theirs) {
            return Err(format!(
                "fixture: on the {label} position {} must read {mine} for the viewer and \
                 {theirs} for the opponent, got {got_mine} and {got_theirs}",
                feature.name(),
            ));
        }
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let separating = separating_terms(&five_power, &church_lead, rules, w);
        let expected: Vec<&str> = LATE_REPRICED_SLOTS.iter().map(|f| f.name()).collect();
        require(
            separating == expected,
            format!(
                "the two positions must separate on EXACTLY the two re-priced late slots \
                 {expected:?}, they separated on {separating:?}"
            ),
        )?;
        let a = score(&five_power, rules, w);
        let b = score(&church_lead, rules, w);
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, PROGRESS_FULL_PPM)
        };
        require(
            a > b,
            format!(
                "late bonus RE-PRICING: the five-power position must outrank the church-majority \
                 position in the LATE vector — five-power {a}, church {b}, margin {} (late \
                 weights: church {}, five_power {})",
                a - b,
                at(Feature::ChurchBonusNow),
                at(Feature::FivePowerTokens),
            ),
        )
    };

    admit(
        SCENARIOS[12].0,
        "the two re-priced LATE slots restored to m22's (church 16, five-power 10)",
        "must outrank the church-majority position in the LATE vector",
        check(&WeightTable::V2_DRAFT),
        check(&late_bonus_reprice_reverted()),
    )
}

#[test]
fn scenario_13_late_bonus_repricing() {
    run(12, "late bonus re-pricing: tokens over church");
}

// ---------------------------------------------------------------------------
// 14. The D5 dummy decides a terminal  (3.5c W4 — rules-derivable, no canon)
// ---------------------------------------------------------------------------

/// **RULES.** Finding D5 (owner ruling 2026-08-08, rulebook p.9 final-scoring
/// NOTE; guidingpath's 2026-08-08 decision-log rows, `SEMANTICS_REV` 5→7). At
/// TWO players the setup dummy competes in the TECHNOLOGY majority at industry
/// value exactly **7.5** — not merely in the church and customs majorities
/// finding D1 covers. The majority pays 10 VP to the largest total industry
/// value and 5 to the second, ties broken clockwise from the start player, and
/// a place the dummy takes is CONSUMED while being awarded to nobody. The
/// engine and the evaluator's mirror both compare DOUBLED industry values so
/// the dummy is the integer 15 and a real seat's total is always even — a tie
/// against the dummy is unrepresentable by construction, which is what makes
/// "7.5" a decision procedure rather than a rounding question.
///
/// **SEAT.** 0, in two finished 2-player games (the only count with a dummy).
///
/// **ITS NUMBER IS A REGISTRY POSITION, NOT A PROTOCOL LABEL.** This is the
/// file's fourteenth scenario — [`SCENARIOS`] index 13 — and it is **not**
/// protocol scenario 14. The protocol's numbers are TENSION LABELS over a sparse
/// list while this array is dense (protocol §6's 2026-08-09 naming amendment),
/// and labels 14 and 15 name two of the CANON-ruled judgment scenarios
/// (achievement proximity, the end-trigger brake) that land later in this wave.
/// This one and its neighbour carry no protocol label at all: §6's growth list
/// names them by description — "the rules-derivable two" — precisely because
/// they need no canon.
///
/// **PROPERTY**, in three clauses of one claim, over two REAL `GameState`s
/// rather than doctored views — half the claim is that the ENGINE's own final
/// scoring agrees, and that needs `final_result`. Seat 0 holds two English
/// technologies in both games and seat 1 holds none; the only difference is the
/// printed gear total.
///
/// (a) **The fixture is the rule.** At gear **7** (doubled 14 < 15) seat 0
///     LOSES the majority to the dummy and collects the second place, 5 VP; at
///     gear **8** (doubled 16 > 15) it TAKES the first, 10 VP. Both legs are
///     read off [`ScoreStructure`] and both winners are read off the engine's
///     `final_result` before the evaluator is asked.
/// (b) **The ordering.** `score(the gear-8 game)` > `score(the gear-7 game)`.
///     Running VP is 20 for seat 0 and 26 for seat 1 in BOTH games, so the 5 VP
///     the dummy's 7.5 moves is exactly what decides the winner: 30 against 26
///     at gear 8, 25 against 26 at gear 7. Measured under the frozen table:
///     **+1,004,000,000 against −1,001,000,000** — a win one side of the rank
///     ladder and a loss the other, with the margin refinement inside each.
/// (c) **The view-side standing agrees with the engine**, 1st and 2nd
///     respectively. Checked AFTER the ordering, so a broken ranking reports
///     the score line rather than the rank line (scenario 9's ordering rule,
///     for scenario 9's reason).
///
/// **WHICH TABLE, and why it is not the draft.** [`WeightTable::V1`], because
/// §2.6 short-circuits the weight vector entirely on a terminal view: no table
/// reaches this property, so naming the draft would imply a dependence that
/// does not exist. The claim is rules-derivable and table-independent, which is
/// also why this scenario needs no canon ruling — D5 is a printed rule the owner
/// adjudicated, not a judgment about play.
///
/// **WHY.** D5 landed at `SEMANTICS_REV` 7 as one of the five BGA-parity
/// rulings, and it is the one with no scenario behind it: the suite covers D1's
/// dummy in the CUSTOMS majority (scenario 4) and the church tie-break (scenario
/// 7), but nothing here made the technology dummy decide anything. It is also
/// the rule most easily lost by omission in exactly the direction that matters —
/// a mirror that ranked only the real seats would hand seat 0 the 10 VP at gear
/// 7 as well, i.e. would call the LOST game won, and a search valuing a terminal
/// successor would then play toward a finish it actually loses. Scenario 9 pins
/// that the terminal ladder ranks on FINAL totals rather than running VP; this
/// one pins that one of those totals is computed with the dummy in the field.
///
/// **COUNTERFACTUAL — the technology majority recomputed with the dummy priced
/// at gear 7 (doubled 14) instead of 7.5.** The targeted mutation here is a
/// SOURCE-shaped one, not a weight, for scenario 9's reason: no
/// [`WeightTable`] can reach a terminal view. Priced at 14 the gear-7 seat TIES
/// the dummy on doubled industry and takes the tie on clockwise distance (the
/// dummy's distance key is unreachable), so it collects the first place, its
/// final total becomes 30, and BOTH games are wins by the same margin of 4 —
/// the ordering collapses to equality. The shipped counterfactual is that
/// rejected reading computed LOCALLY, from [`ScoreStructure`]'s public per-seat
/// legs with only the technology leg re-derived, so the control cannot rot with
/// a source edit. Observed failing line, verbatim:
///
/// ```text
/// the D5 dummy priced at gear 7 instead of 7.5: gear-8 1004000000, gear-7
/// 1004000000
/// ```
///
/// **The equivalent SOURCE edit, and why it is not the shipped control.**
/// Changing `eval.rs`'s `technology_majority` to `consider((DUMMY_SEAT, 14,
/// i32::MAX))` produces the same reading — but it also rewrites the arithmetic
/// clause (a) reads, so the fixture guard fires FIRST and masks the property.
/// That is scenario 9's masking hazard met a second time, and it is exactly why
/// the shipped counterfactual is computed locally instead of being a recorded
/// source edit. Verified at authoring by making the edit and reverting it; what
/// it produced was the guard, verbatim:
///
/// ```text
/// fixture: the gear-8 holding must TAKE the technology majority (10) and the
/// gear-7 holding must lose it to the dummy's 7.5 and collect the second place
/// (5), the structure read 10 and 10
/// ```
///
/// Read that as the guard doing its job in both directions: it is anti-vacuity
/// for the fixture AND the reason the counterfactual could not be a source edit.
fn d5_dummy_decides_the_technology_majority(rules: &Rules) -> Result<(), String> {
    // Two English gear-4 copies and one English gear-3 copy, read off the
    // registry. Both holdings are TWO ENGLISH technologies, so the country-set
    // bonus (one distinct country, no set) and the owned-card count are equal
    // across the pair and the only thing that moves is the gear total.
    let gear4 = techs_of_country_at_gear(rules, Country::English, 4);
    let gear3 = techs_of_country_at_gear(rules, Country::English, 3);
    if gear4.len() < 2 || gear3.is_empty() {
        return Err(format!(
            "fixture: England must print at least two gear-4 technologies and one gear-3, the \
             registry has {} and {}",
            gear4.len(),
            gear3.len(),
        ));
    }
    let high = vec![gear4[0], gear4[1]];
    let low = vec![gear3[0], gear4[0]];
    let industry = |holding: &[TechCardId]| -> u32 {
        holding.iter().filter_map(|id| rules.techs.get(*id)).map(|c| u32::from(c.industry_cost)).sum()
    };
    if (industry(&low), industry(&high)) != (7, 8) {
        return Err(format!(
            "fixture: the two holdings must print gear 7 and gear 8 — the values that straddle \
             the D5 dummy's 7.5 — they print {} and {}",
            industry(&low),
            industry(&high),
        ));
    }
    // The RULE the two holdings have to obey to be holdings at all: a player may
    // own only ONE Technology of a given NAME (10 of the 20 titles circulate as
    // two physical copies so two seats can each own one). [`techs_of_country_at_gear`]
    // selects by country and printed gear, which are not the title — England
    // prints exactly one gear-4 copy of each of two DIFFERENT titles today, so
    // `high` is legal, and this guard is what would notice the day a data edit
    // made it two copies of one title instead.
    for holding in [&high, &low] {
        let mut titles: Vec<&str> = holding
            .iter()
            .filter_map(|id| rules.techs.get(*id))
            .map(|card| card.name.as_str())
            .collect();
        let owned = titles.len();
        titles.sort_unstable();
        titles.dedup();
        if titles.len() != owned || owned != holding.len() {
            return Err(format!(
                "fixture: a seat may own only one Technology of a NAME, and this holding of {} \
                 cards carries {} distinct titles ({titles:?}) — re-pick rather than filing an \
                 illegal holding under a seat",
                holding.len(),
                titles.len(),
            ));
        }
    }

    let finished = |holding: &[TechCardId]| -> Result<GameState, String> {
        let mut state = keep_game(2, 23, rules);
        force_terminal(&mut state);
        for p in state.players.iter_mut() {
            // Zero every remainder leg so the two final totals are exactly the
            // numbers this scenario is about (scenario 9's device, same reason).
            p.money = 0;
            p.copper = 0;
            p.silk = 0;
            p.tea = 0;
            p.seafood = 0;
            p.imported_goods = 0;
            p.completed_orders.clear();
            p.technologies.clear();
            p.victory_points = if p.id.0 == 0 { 20 } else { 26 };
        }
        // CONSERVATION: a technology filed under a seat while still sitting in
        // the deck or a lab slot mints a 31st card, which `check_invariants`
        // rejects (scenario 9's order-side lesson, applied to techs).
        for id in holding {
            take_tech_out_of_play(&mut state, *id)?;
        }
        state.players[0].technologies = holding.to_vec();
        engine_valid(&state, rules, &format!("terminal 2P game, seat 0 at gear {}", industry(holding)))?;
        Ok(state)
    };
    let (gear8, gear7) = (finished(&high)?, finished(&low)?);

    // The ENGINE's own verdict, before the evaluator is asked: the gear-8 game
    // is seat 0's and the gear-7 game is seat 1's.
    for (label, state, winner) in [("gear-8", &gear8, 0u8), ("gear-7", &gear7, 1)] {
        let result = final_result(state, rules);
        if result.winner != PlayerId(winner) {
            return Err(format!(
                "fixture: the engine must call seat {winner} the winner of the '{label}' game, \
                 it called seat {}",
                result.winner.0,
            ));
        }
    }

    let view = |s: &GameState| observe(s, PlayerId(0)).expect("seat 0 is viewable");
    let (gear8_view, gear7_view) = (view(&gear8), view(&gear7));
    if !view_is_terminal(&gear8_view) || !view_is_terminal(&gear7_view) {
        return Err("fixture: both views must be finished games".into());
    }
    // Clause (a): the 10/5 split, and the totals it produces, read off the
    // structure rather than assumed from the doctoring. This is the guard the
    // equivalent source edit would trip first — see the doc.
    let legs = |v: &PlayerView| -> (i64, i64, i64) {
        let s = ScoreStructure::derive(v, rules);
        (s.tech[0], s.final_total(0), s.final_total(1))
    };
    let (gear8_tech, gear8_mine, gear8_theirs) = legs(&gear8_view);
    let (gear7_tech, gear7_mine, gear7_theirs) = legs(&gear7_view);
    if (gear8_tech, gear7_tech) != (10, 5) {
        return Err(format!(
            "fixture: the gear-8 holding must TAKE the technology majority (10) and the gear-7 \
             holding must lose it to the dummy's 7.5 and collect the second place (5), the \
             structure read {gear8_tech} and {gear7_tech}"
        ));
    }
    if !(gear8_mine > gear8_theirs && gear7_mine < gear7_theirs) {
        return Err(format!(
            "fixture: the 5 VP the dummy moves must decide the winner — gear-8 {gear8_mine} \
             against {gear8_theirs} and gear-7 {gear7_mine} against {gear7_theirs}"
        ));
    }

    let standing = |v: &PlayerView| terminal_standing(v, rules).expect("a terminal view stands");
    let shipped = (|| -> Result<(), String> {
        let (a, b) =
            (score(&gear8_view, rules, &WeightTable::V1), score(&gear7_view, rules, &WeightTable::V1));
        require(
            a > b,
            format!(
                "a seat at printed gear 8 TAKES the technology majority over the D5 dummy's 7.5 \
                 (10 VP, final {gear8_mine} against {gear8_theirs}) and one at gear 7 loses it \
                 (5 VP, final {gear7_mine} against {gear7_theirs}): gear-8 {a}, gear-7 {b}"
            ),
        )?;
        let (won_rank, lost_rank) = (standing(&gear8_view).rank, standing(&gear7_view).rank);
        require(
            (won_rank, lost_rank) == (1, 2),
            format!(
                "the view-side standing must place the viewer 1st in the gear-8 game and 2nd in \
                 the gear-7 game, got {won_rank} and {lost_rank}"
            ),
        )
    })();

    // The rejected reading, computed locally: the same terminal ladder with the
    // technology leg re-derived against a dummy priced at gear 7.
    let mutated = {
        let (a, b) =
            (terminal_value_with_dummy_gear(&gear8_view, rules, 14), terminal_value_with_dummy_gear(&gear7_view, rules, 14));
        require(a > b, format!("the D5 dummy priced at gear 7 instead of 7.5: gear-8 {a}, gear-7 {b}"))
    };

    admit(
        SCENARIOS[13].0,
        "the technology majority recomputed with the dummy at gear 7 (doubled 14) instead of 7.5",
        "the D5 dummy priced at gear 7 instead of 7.5",
        shipped,
        mutated,
    )
}

/// §2.6's terminal ladder, recomputed with the 2-player technology dummy priced
/// at `dummy_doubled` instead of the shipped 15 — the rejected reading scenario
/// 14's counterfactual runs.
///
/// Everything except the technology leg comes from the shipped
/// [`ScoreStructure`] (its per-seat legs are public), so the mutation really is
/// the ONE number the doc names and not a second implementation of end scoring.
/// The ranking key and the ladder magnitudes are the shipped ones
/// ([`TERMINAL_RANK_STEP`], [`TERMINAL_MARGIN_STEP`],
/// [`TERMINAL_MARGIN_CLAMP`], [`place_signal`]).
///
/// **Seat indexing narrows exactly as the shipped mirrors narrow.** `eval.rs`'s
/// `seat_index` drops the dummy sentinel AND any seat number outside
/// `0..MAX_SEATS`; [`seat_slot`] below is that predicate, and every index into a
/// seat-wide array here goes through it — a control that indexed on
/// `is_dummy()` alone would be a slightly DIFFERENT function from the one it is
/// the rejected reading of, on exactly the hand-built views this file makes.
fn terminal_value_with_dummy_gear(view: &PlayerView, rules: &Rules, dummy_doubled: u32) -> i64 {
    let structure = ScoreStructure::derive(view, rules);
    let seats = i32::from(view.config.player_count);
    let distance = |p: PlayerId| {
        (i32::from(p.0) + seats - i32::from(view.config.start_player.0)).rem_euclid(seats)
    };

    // (seat or None for the dummy, DOUBLED industry, clockwise distance).
    let mut field: Vec<(Option<usize>, u32, i32)> = Vec::new();
    for player in &view.players {
        let Some(seat) = seat_slot(player) else { continue };
        let gears: u32 = player
            .technologies
            .iter()
            .filter_map(|id| rules.techs.get(*id))
            .map(|card| u32::from(card.industry_cost))
            .sum();
        // A seat owning no technology never places — the shipped rule.
        if gears == 0 {
            continue;
        }
        field.push((Some(seat), 2 * gears, distance(player.id)));
    }
    if view.config.player_count == 2 {
        field.push((None, dummy_doubled, i32::MAX));
    }
    field.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));

    let mut tech = [0i64; MAX_SEATS];
    for (place, vp) in [(0usize, 10i64), (1usize, 5i64)] {
        if let Some((Some(seat), _, _)) = field.get(place).copied() {
            tech[seat] += vp;
        }
    }
    let total = |seat: usize| structure.final_total(seat) - structure.tech[seat] + tech[seat];

    let mine = view
        .players
        .iter()
        .find(|p| p.id == view.viewer)
        .and_then(seat_slot)
        .unwrap_or(0);
    let my_total = total(mine);
    let mut rank = 1u8;
    let mut best_other: Option<i64> = None;
    for player in &view.players {
        let Some(seat) = seat_slot(player) else { continue };
        if seat == mine {
            continue;
        }
        let theirs = total(seat);
        best_other = Some(best_other.map_or(theirs, |b: i64| b.max(theirs)));
        if theirs > my_total
            || (theirs == my_total && distance(player.id) < distance(view.viewer))
        {
            rank += 1;
        }
    }
    let margin = (my_total - best_other.unwrap_or(my_total))
        .clamp(-TERMINAL_MARGIN_CLAMP, TERMINAL_MARGIN_CLAMP);
    TERMINAL_RANK_STEP * place_signal(rank) + TERMINAL_MARGIN_STEP * margin
}

#[test]
fn scenario_14_d5_dummy_technology_majority() {
    run(13, "D5 dummy decides the technology majority");
}

// ---------------------------------------------------------------------------
// 15. The deck clock's banking decision  (3.5c W4 — the file's named gap)
// ---------------------------------------------------------------------------

/// [`WeightTable::V1`] with the `victory_points` pair FLATTENED to its EARLY
/// value — the stage dependence of the one term scenario 15's margin turns on,
/// removed, and nothing else touched.
///
/// §0.2's targeted mutation for scenario 15, and it is deliberately NOT
/// "victory_points zeroed" (scenario 2's mutation, which would prove only that
/// printed VP is worth something — a claim scenario 2 already carries, on a
/// fixture of very nearly this shape). This scenario's claim is about the DECK
/// CLOCK: banking gets BETTER as the deck runs out, and it does so because the
/// term paying for the conversion is the one that RISES with the stage. So the
/// mutation removes the LATENESS from `victory_points` rather than the term —
/// held at its early 10, a banked VP is priced the same at a full deck as at an
/// empty one, and the margin stops growing with the clock.
fn victory_points_flattened_to_early() -> WeightTable {
    let mut w = WeightTable::V1;
    let (early, _) = WeightTable::V1.get(Feature::VictoryPoints);
    w.set(Feature::VictoryPoints, early, early);
    w
}

/// **RULES.** One of the end-game triggers is the order DECK running out with
/// an empty Port slot (`game/end_game.rs`; [`clock_ppm`]'s first component is
/// its clock, which deliberately drops the port conjunct). An Order card still
/// in a player's hand at the end scores NOTHING and its country icon counts for
/// nothing — only a COMPLETED order pays its printed rewards and carries an icon
/// into a country set. Order #14 (DE) prints `copper 1 · silk 2 · tea 2 → 6 VP,
/// 2 Yen` and nothing else, so fulfilling it is exactly a goods-for-VP
/// conversion with no other board effect. Achievement cards, by contrast, reward
/// HOLDING goods (Copper 5 / Silk 6 / Tea 7 — KB `01-rules-reference.md`).
///
/// **SEAT.** 0, in a 2-player game with **one card left in the order deck** —
/// the deck clock at 900,000 ppm, and asserted to be the clock that dates the
/// position (no track carries a real pawn and nobody has built).
///
/// **ITS NUMBER IS A REGISTRY POSITION, NOT A PROTOCOL LABEL.** This is the
/// file's fifteenth scenario — [`SCENARIOS`] index 14 — and it is **not**
/// protocol scenario 15, which names the END-TRIGGER BRAKE the owner ruled on
/// (and inverted) and which lands later in this wave as its own canon-judgment
/// scenario. The protocol's numbers are tension LABELS over a sparse list while
/// this array is dense (protocol §6's 2026-08-09 naming amendment); this
/// scenario is one of §6's "rules-derivable two" and carries no label.
///
/// **WHICH TABLE, and why it is not the draft.** [`WeightTable::V1`], on this
/// file's rule for W4 scenarios: the claim is TABLE-INDEPENDENT — both tables
/// price a banked VP at 40 late and both grow it with the stage, which is the
/// whole of what clause (c) asserts — so it rides the frozen table and the
/// freeze commit has one fewer scenario to re-point. **Recorded at authoring,
/// NOT asserted:** under [`WeightTable::V2_DRAFT`] the same pair reads **+202**
/// with one card left against **−3** at a full deck, i.e. the draft's round-1
/// early-VP re-pricing makes the early half of the contrast a preference for
/// HOLDING; under V1 the early half still prefers banking (+53). The GROWTH is
/// the property both tables share, and the growth is what is pinned.
///
/// **PROPERTY**, three clauses. (a) The two positions separate on EXACTLY
/// `victory_points`, `achievement_progress_sum` and `hand_size` — the fixture's
/// own leak check, and what makes the rest a statement about the conversion
/// rather than about whatever else the doctoring moved. (b) `score(the order
/// BANKED)` > `score(the order still HELD)` with one card left. (c) **THE DECK
/// CLOCK'S OWN CONTENT:** that margin is strictly LARGER than the one the same
/// pair reads with a FULL deck — the closer the deck is to running out, the
/// better banking gets. Measured under the frozen table: **+212 against +53**.
///
/// **THE ARITHMETIC, and why the deck clock is the subject.** Both positions
/// share one base view, so the margin is `Σ w(p) · (v_banked − v_held)` over the
/// three separating terms: `vp +6` (the card's printed reward),
/// `achievement_progress_sum −5` (every one of the five goods the order costs
/// was a point of progress, all three legs sitting strictly below their caps),
/// `hand_size −1` (the commitment left the hand). At the deck-dated
/// `p = 900,000` the interpolated weights are `vp 37`, `achievement 3`,
/// `hand_size −5`, so the margin is
/// `37·6 + 3·(−5) + (−5)·(−1) = 222 − 15 + 5 = +212`; with a FULL deck (`p = 0`)
/// they are `vp 10`, `achievement 2`, `hand_size −3` and it is
/// `10·6 + 2·(−5) + (−3)·(−1) = 60 − 10 + 3 = +53`. Both read exactly.
///
/// **WHY THE GROWTH IS THE RIGHT CLAUSE, and not the ordering alone.** Under the
/// frozen table banking this order beats holding it at EVERY stage, so an
/// ordering-only property here would restate scenario 2 with a shorter deck and
/// pin nothing about the clock. What is distinctive is the DERIVATIVE: the same
/// conversion is worth four times as much one card from the end as it is at a
/// full deck, and the reason is the rules content — a held order is worth
/// nothing at all when the game stops, so the nearer the deck is to out, the
/// less an unfulfilled commitment can ever be worth. That is the same fact
/// `hand_size`'s measured negative sign reports from the corpus side (R1-7,
/// 0.4745 → 0.3753), read from the decision side.
///
/// **WHY.** This file's header recorded, from the T9 review onward, that spec
/// §6's end-trigger category names two clocks and that *"nothing here exercises
/// the deck half"*. This is the deck half: the position is DATED by the deck (a
/// fixture guard, not a hope) and the margin is a function of it. It is an
/// eval-ORDERING property over two constructed views, like twelve of its
/// neighbours — the header says so where it retires the gap, and "decision" here
/// means the ranking of two positions, never a driven pick.
///
/// **FIXTURE HONESTY — the goods sit OFF the threshold corner, deliberately.**
/// The seat holds 3 copper / 3 silk / 3 tea against cards printing 5 / 6 / 7, so
/// every leg is strictly below its cap and the five goods the order costs are
/// five points of progress given up whichever way the cap would have rounded.
/// An earlier draft of this fixture sat the seat exactly AT all three thresholds
/// (5/6/7); the margins are identical either way, and the corner was moved off
/// because sitting at three simultaneous claim thresholds is precisely the shape
/// round 3 identified the shipped progress SUM as blind to — the
/// `achievement_race_proximity` finding (+0.054 early / +0.094 mid,
/// `docs/plans/2026-08-09-phase3.5c-round3-evidence.md`), which is the named
/// first candidate of the post-freeze V3 cycle. A required suite property should
/// not be resting on the one corner a future feature is chartered to re-price.
///
/// **What it does NOT claim.** Not that fulfilling is right at any particular
/// real decision point — the goods are always doing something else, and the
/// evaluator sees one ply. What is pinned is that with the deck one card from
/// out, this rung prefers the banked position, and prefers it MORE than it would
/// have with the deck full.
///
/// **COUNTERFACTUAL — the `victory_points` pair flattened to its early value
/// (10/10) ([`victory_points_flattened_to_early`]).** With the term's stage
/// dependence gone the deck clock buys the conversion nothing: the late margin
/// falls to +50 while the full-deck margin is unmoved at +53 — so banking still
/// beats holding (clause (b) survives, as it must: the mutation does not touch
/// whether a VP is worth having) and the GROWTH clause, the one this scenario is
/// about, inverts. Observed failing line, verbatim:
///
/// ```text
/// the deck clock must make banking BETTER, not merely acceptable: with ONE card
/// left the margin is 50 and with a FULL deck the same pair reads 53 (p=900000,
/// deck-dated weights vp 10, achievement 3, hand_size -5)
/// ```
///
/// Both figures are what the arithmetic above predicts once `vp` stops moving:
/// `10·6 + 3·(−5) + (−5)·(−1) = +50` late against the unchanged `+53` early.
/// That the EARLY number does not move is not a coincidence worth hiding — the
/// flatten sets `vp` to the value it already had there, so the full-deck half of
/// the contrast is by construction the same number in both runs. It is the
/// deck-dated half that moves, +212 → +50, which is exactly the claim.
fn deck_clock_banking_beats_holding(rules: &Rules) -> Result<(), String> {
    // The card is named by its printed corner index and every number below is
    // read off the registry rather than retyped.
    let id = OrderCardId(14);
    let card = rules.orders.get(id).ok_or("fixture: order 14 is not in the registry")?;
    let (req, rewards) = (card.requirements, card.rewards);
    if req.copper == 0 || req.silk == 0 || req.tea == 0 || req.seafood != 0 {
        return Err(format!(
            "fixture: order 14 must cost copper, silk and tea and no seafood (three types, one \
             per achievement card in play), it costs {req:?}"
        ));
    }
    // A pure goods-for-VP conversion: any other printed reward would move a
    // fourth term and break the leak clause for a reason that has nothing to do
    // with the deck.
    if rewards.victory_points == 0
        || (rewards.imported_goods, rewards.warehouse_tokens, rewards.any_trade_good) != (0, 0, 0)
        || (rewards.copper, rewards.silk, rewards.tea, rewards.seafood) != (0, 0, 0, 0)
        || rewards.movement != 0
    {
        return Err(format!(
            "fixture: order 14 must pay VP (and Yen, which is not a feature) and nothing else — \
             its printed rewards are {rewards:?}"
        ));
    }

    let mut base = base_view(2, 24, rules);
    // ONE card left. The deck clock is then 900,000 ppm and, with no pawn on
    // either track and nothing built, it is the clock that dates the position.
    base.order_deck_len = 1;
    // Set explicitly rather than taking the deal: these three are the cards the
    // order's three goods types feed. The holding sits STRICTLY BELOW all three
    // printed thresholds (3 against 5 / 6 / 7) — see the doc's fixture-honesty
    // note for why the threshold corner is deliberately avoided — so every good
    // the order spends is a point of progress given up with no cap in the way.
    base.achievements = [
        AchievementCard::new(AchievementType::Copper5),
        AchievementCard::new(AchievementType::Silk6),
        AchievementCard::new(AchievementType::Tea7),
    ];
    {
        let me = viewer_mut(&mut base);
        me.completed_orders.clear();
        me.technologies.clear();
        me.victory_points = 0;
        me.copper = 3;
        me.silk = 3;
        me.tea = 3;
        me.seafood = 0;
        me.hand = vec![HandCard::Known(id)];
    }

    let held = base.clone();
    let mut banked = base.clone();
    {
        let me = viewer_mut(&mut banked);
        me.hand.clear();
        me.completed_orders = vec![id];
        me.victory_points += u32::from(rewards.victory_points);
        me.money += u32::from(rewards.money);
        me.copper -= u32::from(req.copper);
        me.silk -= u32::from(req.silk);
        me.tea -= u32::from(req.tea);
    }
    // The same pair with a FULL deck — clause (c)'s comparator.
    // `initial_order_deck_len` is derived from the registry, so restoring the
    // view's deck to it is what puts p back at 0.
    let with_full_deck = |v: &PlayerView| -> PlayerView {
        let mut v = v.clone();
        v.order_deck_len = initial_order_deck_len(&v, rules).unwrap_or(0);
        v
    };
    let (held_early, banked_early) = (with_full_deck(&held), with_full_deck(&banked));

    // THE STAGE IS THE SCENARIO'S SUBJECT, so it is asserted rather than
    // assumed: the DECK clock must be the maximum, and the other three must be
    // strictly behind it, or the position is dated by something else and the
    // scenario is not about the deck at all.
    let clocks = clock_ppm(&banked, rules);
    let p = progress_ppm(&banked, rules);
    if p != 900_000 || clocks[0] != p || clocks[1..].iter().any(|c| *c >= p) {
        return Err(format!(
            "fixture: one card left in a 2P deck must date the position at 900000 ppm on the \
             DECK clock alone, got p={p} with clocks {clocks:?}"
        ));
    }
    if progress_ppm(&held, rules) != p {
        return Err("fixture: banking must not move the stage (the deck length is the same in \
                    both positions — only the hand and the piles differ)"
            .into());
    }
    if progress_ppm(&banked_early, rules) != 0 || progress_ppm(&held_early, rules) != 0 {
        return Err("fixture: clause (c)'s full-deck comparator must sit at p=0".into());
    }
    for (label, view) in
        [("banked", &banked), ("held", &held), ("banked-full-deck", &banked_early)]
    {
        if view_is_terminal(view) {
            return Err(format!(
                "fixture: the {label} position must still be scored POSITIONALLY — a near-empty \
                 deck is not a finished game, and the terminal ladder short-circuits the weight \
                 vector this scenario is about"
            ));
        }
    }
    // Anti-vacuity: the conversion must actually COST achievement progress, or
    // the scenario is a plain "more VP is better" restatement.
    let progress = |v: &PlayerView| viewer_value(v, rules, Feature::AchievementProgressSum);
    let (held_progress, banked_progress) = (progress(&held), progress(&banked));
    if held_progress - banked_progress != i64::from(req.total()) {
        return Err(format!(
            "fixture: every one of the order's {} goods must be a point of achievement progress \
             given up, progress went {held_progress} -> {banked_progress}",
            req.total(),
        ));
    }
    // OFF THE THRESHOLD CORNER, asserted rather than described (the doc's
    // fixture-honesty note): every leg must sit strictly UNDER its printed
    // requirement in the richer of the two positions, so no cap is doing any of
    // the arithmetic above and the fixture is not resting on the simultaneous
    // triple-claim corner the V3 proximity candidate is chartered to re-price.
    for card in &held.achievements {
        let (current, needed) = achievement_progress(&held, own_row(&held), card.achievement_type);
        if current >= needed {
            return Err(format!(
                "fixture: every achievement leg must sit strictly below its threshold on the \
                 HELD position, {:?} reads {current} of {needed}",
                card.achievement_type,
            ));
        }
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let separating = separating_terms(&banked, &held, rules, w);
        require(
            separating == ["victory_points", "achievement_progress_sum", "hand_size"],
            format!(
                "the two positions must separate on the printed VP, the achievement progress the \
                 goods were feeding, and the commitment that left the hand — they separated on \
                 {separating:?}"
            ),
        )?;
        let (a, b) = (score(&banked, rules, w), score(&held, rules, w));
        let early_margin = score(&banked_early, rules, w) - score(&held_early, rules, w);
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, p)
        };
        require(
            a > b,
            format!(
                "with ONE card left in the order deck, banking the completable order must beat \
                 holding it: banked {a}, held {b}, margin {} (p={p}, weights vp {}, achievement \
                 {}, hand_size {})",
                a - b,
                at(Feature::VictoryPoints),
                at(Feature::AchievementProgressSum),
                at(Feature::HandSize),
            ),
        )?;
        // THE DECK CLOCK'S OWN CONTENT (clause (c)): the ordering above holds at
        // every stage under the frozen table, so what makes this the DECK
        // scenario rather than a restatement of scenario 2 is that the margin
        // GROWS as the deck runs out.
        require(
            a - b > early_margin,
            format!(
                "the deck clock must make banking BETTER, not merely acceptable: with ONE card \
                 left the margin is {} and with a FULL deck the same pair reads {early_margin} \
                 (p={p}, deck-dated weights vp {}, achievement {}, hand_size {})",
                a - b,
                at(Feature::VictoryPoints),
                at(Feature::AchievementProgressSum),
                at(Feature::HandSize),
            ),
        )
    };

    admit(
        SCENARIOS[14].0,
        "the victory_points pair flattened to its early value (10/10)",
        "the deck clock must make banking BETTER",
        check(&WeightTable::V1),
        check(&victory_points_flattened_to_early()),
    )
}

#[test]
fn scenario_15_deck_clock_banking() {
    run(14, "deck clock: bank before the deck runs out");
}

// ---------------------------------------------------------------------------
// 16. Track-racing: extending a WINNABLE race  (3.5c W4 — protocol label 11)
// ---------------------------------------------------------------------------

/// **RULES.** The Church management board is one of the end-game clocks: the
/// game ends when the track fills to the player-count threshold (4/5/6 REAL
/// pawns at 2/3/4 players — the 2P dummy does not count, finding D1;
/// `game/end_game.rs`, `GameConfig::track_trigger`). The board itself pays a
/// MAJORITY at the end — 6 VP to the most pawns and 3 to the second, a count tie
/// broken by the RIGHTMOST occupied space (rulebook p.9) — and at two players
/// the neutral dummy on faith 7 is a real competitor that can take a place,
/// which is then consumed while being awarded to nobody (finding D1,
/// guidingpath's 2026-07-21 rows). A pawn that goes on the track LEAVES the
/// board: it is spent, it stops earning action power, and it does not come back.
///
/// **SEAT.** 0, in a 2-player game, with the order deck run out so the DECK
/// clock reads `p = 1,000,000` — the LATE vector exactly, in all four positions,
/// so the one-pawn occupancy difference inside each pair cannot move the stage.
/// (Scenario 11's device: a view at full progress is not a finished game, since
/// the real trigger also needs an empty Port slot — a conjunct [`clock_ppm`]
/// deliberately drops.)
///
/// **THE CANON IT ENCODES — protocol label 11, ruled CONDITIONALLY YES.** The
/// owner, 2026-08-09 (guidingpath's canon-batch row): *"sometimes forcing the
/// end game sooner does make sense"* — a church rush for a short game is
/// anecdotally popular among BGA 2P players, it is the counter to
/// engine-building, and it is *"hard to do totally alone without losing to an
/// engine"*. **The CONDITIONAL is the load-bearing half**, and it is what this
/// scenario makes executable: the fixture is sized so the race is WINNABLE, and
/// the control is the same physical extension in a race where it wins nothing.
///
/// **PROPERTY**, three clauses. (a) **WINNABLE:** `score(extend)` >
/// `score(abandon)`, where extending puts a second pawn on the track and takes
/// the majority (6 VP to the viewer, 3 to the rival) that one pawn behind the
/// rival's collected nothing at all. (b) **DECIDED — the conditional's other
/// side:** the SAME conversion, in a field where the viewer's single pawn
/// already holds first place, reads strictly BELOW leaving the assistant in its
/// area, because the majority is 6/3 either way and the pawn only stops working.
/// (c) The winnable pair separates on EXACTLY `church_bonus_now` and
/// `assistants_on_board` — the fixture's leak check.
///
/// **THE TWO FIELDS ARE THE SAME MATERIAL, one space index apart.** The rival
/// holds one pawn on faith 8 in all four positions and the dummy sits on faith
/// 7. In the WINNABLE field the viewer's first pawn is on faith 5 — behind both,
/// so at one pawn each the count-1 tie goes to the rival's rightmost space and
/// the viewer collects nothing; the second pawn (faith 6) wins the count
/// outright and takes the 6. In the DECIDED field that first pawn is on faith 11
/// instead — rightmost, so it already takes the 6 — and the second pawn (faith
/// 10) changes no award. Nothing else about the two fields differs, which is
/// what makes the flip a statement about the race rather than about pawns.
///
/// **THE ARITHMETIC.** Every term but two is identical inside a pair, so the
/// margin is `w_church · Δ(mine − theirs) + w_board · Δ(mine)` at
/// `p = 1,000,000`. WINNABLE: the church relative value goes from `0 − 6` to
/// `6 − 3`, i.e. `+9`, and the board loses one assistant, so under the frozen
/// table `16·9 + 10·(−1) = 144 − 10 = +134`. DECIDED: the church relative value
/// is `6 − 3` in BOTH positions, so the whole margin is the board's
/// `10·(−1) = −10`.
///
/// **FIXTURE HONESTY — the twenty tokens stay twenty.** A pawn that extends the
/// race has left `assistants.on_board` and arrived on `assistants.on_church`, so
/// every position here is asserted to hold the seat's whole printed supply
/// (`AssistantState::total`) rather than
/// quietly dropping a token the way a fixture that edited only `on_board` would.
/// Both seats' hands are equal across each pair by construction, so the three
/// hand terms cancel out of every margin.
///
/// **WHICH TABLE.** [`WeightTable::V1`], on this file's W4 rule: the claim holds
/// table-independently, so it rides the frozen table and the freeze commit has
/// one fewer scenario to re-point. Recorded at authoring, not asserted: under
/// [`WeightTable::V2_DRAFT`] the same two margins read **+62** and **−10** — the
/// winnable margin is smaller because round 2 down-priced the late church bonus
/// (16 → 8), and the conditional is unchanged.
///
/// **WHAT IS NOT EXPRESSIBLE AT THIS GRAIN, said out loud.** The canon's own
/// words are about *forcing the end game sooner*, and the evaluator has no term
/// for that: a clock only RE-WEIGHTS the position, it never prices "the game
/// stops earlier" as such. This fixture holds the stage FIXED (the deck clock
/// dates all four positions) precisely so the margin is the majority arithmetic
/// and nothing else, which is the half of the ruling this rung can answer for.
/// The other half — whether racing the clock to shorten the game is right —
/// is a strength question that belongs to the arena, not to a constructed pair.
///
/// **COUNTERFACTUAL — `church_bonus_now` zeroed (§0.2's targeted mutation).**
/// With the majority priced at nothing, extending is only ever an assistant
/// leaving the board, so the winnable field ranks the way the decided one does.
/// Note the clause ORDER below: the ordering clauses run BEFORE the leak clause,
/// because a zeroed term stops separating the pair at all and a leak clause
/// asked first would fire on the mutation instead of the property (scenario 9's
/// masking hazard, met from the other direction). Observed failing line,
/// verbatim:
///
/// ```text
/// extending a WINNABLE church race must outrank abandoning it — the second
/// pawn takes the 6/3 majority the first one lost to the rival's rightmost
/// space: extend 22, abandon 32, margin -10 (late weights: church 0, board 10)
/// ```
fn track_racing_extends_a_winnable_church_race(rules: &Rules) -> Result<(), String> {
    /// The D1 dummy's 2P church space.
    const DUMMY_FAITH: u8 = 7;
    /// The rival's single pawn, to the RIGHT of the dummy.
    const RIVAL_FAITH: u8 = 8;

    let mut base = base_view(2, 25, rules);
    base.order_deck_len = 0;
    let area = *base.board_layout.areas.keys().next().ok_or("fixture: an empty board")?;

    let dummy_index = ChurchTrack::faith_to_index(DUMMY_FAITH).expect("faith 7 is on the track");
    if !base.church_track.slots[dummy_index].is_some_and(|s| s.player.is_dummy()) {
        return Err(format!(
            "fixture: the 2P setup must place the church dummy on faith {DUMMY_FAITH} (D1) — the \
             majority arithmetic this scenario cites is the dummy's rule"
        ));
    }
    if base.church_track.total_assistants() != 0 {
        return Err("fixture: a fresh 2P church track must carry no REAL pawn".into());
    }
    let supply = own_row(&base).assistants.total();

    // One position: the viewer's pawns on `mine`, the rival's on `theirs`, and
    // the viewer's remaining working assistants in one area. The hand absorbs
    // the rest, so every seat still holds the whole printed supply.
    let position = |mine: &[u8], theirs: &[u8], on_board: u8| -> Result<PlayerView, String> {
        let mut v = base.clone();
        for (faiths, seat) in [(mine, 0u8), (theirs, 1u8)] {
            for (n, faith) in faiths.iter().enumerate() {
                let index = ChurchTrack::faith_to_index(*faith)
                    .ok_or_else(|| format!("fixture: faith {faith} is not a church space"))?;
                if v.church_track.slots[index].is_some() {
                    return Err(format!("fixture: faith {faith} is occupied twice"));
                }
                v.church_track.slots[index] =
                    Some(ChurchSlot { player: PlayerId(seat), arrival_order: n as u8 + 1 });
            }
        }
        for player in v.players.iter_mut() {
            let (pawns, board) = if player.id == PlayerId(0) {
                (mine.len() as u8, on_board)
            } else {
                (theirs.len() as u8, 0)
            };
            player.assistants.on_church = pawns;
            player.assistants.on_board = BTreeMap::new();
            if board > 0 {
                player.assistants.on_board.insert(area, board);
            }
            player.assistants.in_hand = supply
                .checked_sub(player.assistants.in_warehouse + board + pawns)
                .ok_or("fixture: the position spends more assistants than a seat owns")?;
        }
        Ok(v)
    };

    let winnable_abandon = position(&[5], &[RIVAL_FAITH], 4)?;
    let winnable_extend = position(&[5, 6], &[RIVAL_FAITH], 3)?;
    let decided_abandon = position(&[11], &[RIVAL_FAITH], 4)?;
    let decided_extend = position(&[10, 11], &[RIVAL_FAITH], 3)?;

    // The stage is the premise, so it is asserted: the DECK clock puts all four
    // positions in the LATE vector, and none of them is a finished game.
    let all = [
        ("winnable-abandon", &winnable_abandon),
        ("winnable-extend", &winnable_extend),
        ("decided-abandon", &decided_abandon),
        ("decided-extend", &decided_extend),
    ];
    for (label, view) in all {
        let (p, clocks) = (progress_ppm(view, rules), clock_ppm(view, rules));
        if p != PROGRESS_FULL_PPM || clocks[0] != PROGRESS_FULL_PPM {
            return Err(format!(
                "fixture: the {label} position must sit at full progress on the DECK clock so the \
                 LATE vector governs exactly and the extra pawn cannot move the stage, got p={p} \
                 with clocks {clocks:?}"
            ));
        }
        if view_is_terminal(view) {
            return Err(format!(
                "fixture: the {label} position must still be scored POSITIONALLY — the terminal \
                 ladder short-circuits the weight vector this scenario is about"
            ));
        }
        for player in &view.players {
            if !player.id.is_dummy() && player.assistants.total() != supply {
                return Err(format!(
                    "fixture: the {label} position must leave seat {} holding the whole {supply}-\
                     token supply (a pawn on the track left the board and arrived on `on_church`), \
                     it holds {}",
                    player.id.0,
                    player.assistants.total(),
                ));
            }
        }
    }

    // THE CONDITIONAL, read off the structure rather than assumed from the
    // doctoring: the extension must WIN a place in one field and change nothing
    // in the other, or the two clauses below are not the pair the doc claims.
    let awards = |v: &PlayerView| -> (i64, i64) {
        let structure = ScoreStructure::derive(v, rules);
        let value = |seat: u8| {
            let row = v.players.iter().find(|p| p.id == PlayerId(seat)).expect("both seats");
            structure.value_of(row, Feature::ChurchBonusNow)
        };
        (value(0), value(1))
    };
    let built = [
        ("winnable-abandon", &winnable_abandon, (0, 6)),
        ("winnable-extend", &winnable_extend, (6, 3)),
        ("decided-abandon", &decided_abandon, (6, 3)),
        ("decided-extend", &decided_extend, (6, 3)),
    ];
    for (label, view, want) in built {
        let got = awards(view);
        if got != want {
            return Err(format!(
                "fixture: the {label} position's church majority must read {want:?} for (viewer, \
                 rival) — the WINNABLE field is the one where the second pawn takes a place and \
                 the DECIDED field the one where it takes none — it read {got:?}"
            ));
        }
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, PROGRESS_FULL_PPM)
        };
        let (church, board) = (at(Feature::ChurchBonusNow), at(Feature::AssistantsOnBoard));
        let (a, b) = (score(&winnable_extend, rules, w), score(&winnable_abandon, rules, w));
        require(
            a > b,
            format!(
                "extending a WINNABLE church race must outrank abandoning it — the second pawn \
                 takes the 6/3 majority the first one lost to the rival's rightmost space: extend \
                 {a}, abandon {b}, margin {} (late weights: church {church}, board {board})",
                a - b,
            ),
        )?;
        let (c, d) = (score(&decided_extend, rules, w), score(&decided_abandon, rules, w));
        require(
            c < d,
            format!(
                "the SAME extension in a race the viewer has ALREADY won must read below leaving \
                 the assistant working — the majority is 6/3 either way, so the pawn buys nothing \
                 and stops earning: extend {c}, abandon {d}, margin {} (late weights: church \
                 {church}, board {board})",
                c - d,
            ),
        )?;
        // The leak clause runs LAST on purpose — see the doc's counterfactual
        // note: the targeted mutation zeroes the term the pair separates on, so
        // a leak clause asked first would fire on the mutation rather than on
        // the property.
        let separating = separating_terms(&winnable_extend, &winnable_abandon, rules, w);
        require(
            separating == ["church_bonus_now", "assistants_on_board"],
            format!(
                "the winnable pair must separate on the majority it wins and the assistant it \
                 spends, and on nothing else — it separated on {separating:?}"
            ),
        )
    };

    admit(
        SCENARIOS[15].0,
        "church_bonus_now zeroed",
        "extending a WINNABLE church race must outrank abandoning it",
        check(&WeightTable::V1),
        check(&zeroed(Feature::ChurchBonusNow)),
    )
}

#[test]
fn scenario_16_track_racing_winnable_church_race() {
    run(15, "track-racing: extending a winnable church race");
}

// ---------------------------------------------------------------------------
// 17. The customs RACE and the rival's progress  (3.5c W4 — protocol labels
//     12 and 13, both re-aimed)
// ---------------------------------------------------------------------------

/// **RULES.** The Customs management board is seven SINGLE-OCCUPANCY spaces,
/// taken by paying their printed imported-goods cost (1,1,2,2,3,3,4) for their
/// printed VP (5,4,10,9,16,15,22) — so the cost-4 space is a thing exactly ONE
/// player can ever have — and it is the second of the two track end-clocks
/// (4/5/6 REAL pawns at 2/3/4 players). At the end the board pays a MAJORITY: 8
/// VP to the most pawns and 4 to the second, a count tie broken by the RIGHTMOST
/// occupied space, with the 2P dummy on space 4 competing for a place that is
/// then consumed and awarded to nobody (finding D1; `game/scoring.rs`, rulebook
/// p.9). The printed per-space VP is paid WHEN THE SPACE IS TAKEN, so it is
/// already inside `victory_points`; the majority is the part that is still
/// contested, and it is what [`Feature::CustomsBonusNow`] mirrors.
///
/// **SEAT.** 0, in a 2-player game, deck run out so `p = 1,000,000` — the LATE
/// vector exactly, in all four positions (scenario 11's device again).
///
/// **THE CANON IT ENCODES — protocol labels 12 and 13, and BOTH were RE-AIMED
/// away from the drafts they started as** (the owner, 2026-08-09; guidingpath's
/// canon-batch row). Label 13, customs: **REFRAMED AS A RACE, NOT DENIAL** —
/// *"only one player can have the 4-track"*, **the AWARENESS of the opponent's
/// customs progress is the valuable thing**, assistants committed there
/// handicap everything else, and most of the track is lower-value redemptions.
/// Label 12, church denial: **NOT CANON** — *"denial as a strategy is fairly
/// advanced and probably not the most optimal place a player can take"*, so the
/// drafted denial scenario must NOT encode denial-beats-neutral, while *"the
/// majority ARITHMETIC being priced correctly is a different, narrower property
/// and stays fair game"*. This scenario carries both: the customs half as the
/// race, and the church half as the CONTROL that says the evaluator prices the
/// arithmetic of a place and never the denial of a space.
///
/// **PROPERTY**, four clauses. (a) **THE RACE, contested:** with the rival
/// holding a customs space, committing an assistant to the track reads ABOVE
/// leaving it working — it takes the 8 and leaves the rival the 4, where before
/// the rival held the 8 and the viewer had nothing. (b) **THE SAME COMMITMENT,
/// uncontested:** with the rival absent from the track and the viewer's single
/// pawn already holding first place, a second pawn reads strictly BELOW leaving
/// the assistant working — the majority is 8 either way and the token only stops
/// earning. The two fields differ in EXACTLY the rival's customs progress, which
/// is the awareness the canon calls valuable, made executable. (c) **THE CHURCH
/// SIDE, per label 12's re-aim:** two church landings whose majority awards are
/// IDENTICAL score EQUAL, whichever space they occupy — there is no denial term,
/// and the suite does not grow one. (d) The contested pair separates on exactly
/// `customs_bonus_now` and `assistants_on_board`.
///
/// **THE ARITHMETIC.** Inside each pair everything but two terms is identical,
/// so the margin is `w_customs · Δ(mine − theirs) + w_board · Δ(mine)` at
/// `p = 1,000,000`. CONTESTED: the customs relative value goes from `0 − 8` to
/// `8 − 4`, i.e. `+12`, so under the frozen table `20·12 + 10·(−1) = 240 − 10 =
/// +230`. UNCONTESTED: the relative value is `8 − 0` in both positions, so the
/// margin is the board's `10·(−1) = −10`.
///
/// **WHY THIS IS NOT DENIAL, stated precisely, because the line matters.** The
/// evaluator is relative — every term is `w · (mine − theirs)` — so a rival's
/// award always enters a margin. What the canon rules out is treating denial as
/// a GOAL: a term that paid for taking an option away from a rival independently
/// of what it wins. Clause (c) is the executable form of "there is no such
/// term": the viewer's own award and the rival's are both held fixed while the
/// occupied SPACE moves, and the two positions must score identically. Clause
/// (a)'s margin, by contrast, is entirely the arithmetic of places actually
/// changing hands, which is exactly the *"narrower property"* the ruling leaves
/// fair game.
///
/// **WHICH TABLE.** [`WeightTable::V1`], on this file's W4 rule — the claim is
/// table-independent, and the two tables price both terms this scenario turns on
/// identically late (`customs 20`, `board 10`), so the margins above are the
/// same figure under the draft. Recorded at authoring: **+230 and −10 under both
/// tables**, and the church control is an equality under any table at all.
///
/// **COUNTERFACTUAL — `customs_bonus_now` zeroed (§0.2's targeted mutation).**
/// With the majority priced at nothing, committing an assistant to the track is
/// only ever a token leaving the board, so the contested field ranks the way the
/// uncontested one does. The leak clause runs last for scenario 16's reason.
/// Observed failing line, verbatim:
///
/// ```text
/// in a CONTESTED customs race the commitment must outrank leaving the
/// assistant working — it takes the 8 and leaves the rival the 4, where the
/// abandoned position had the rival on the 8 and the viewer on nothing: commit
/// 24, abandon 34, margin -10 (late weights: customs 0, board 10)
/// ```
fn customs_race_and_the_rivals_progress(rules: &Rules) -> Result<(), String> {
    /// The D1 dummy's 2P customs space.
    const DUMMY_SPACE: u8 = 4;
    /// The rival's space when it is contesting — right of the dummy, so at one
    /// pawn each it wins the count tie against everything to its left.
    const RIVAL_SPACE: u8 = 5;
    /// The viewer's first and second landings, both right of the rival's.
    const MINE_FIRST: u8 = 7;
    const MINE_SECOND: u8 = 6;

    let mut base = base_view(2, 26, rules);
    base.order_deck_len = 0;
    let area = *base.board_layout.areas.keys().next().ok_or("fixture: an empty board")?;

    let dummy_index = CustomsTrack::space_to_index(DUMMY_SPACE).expect("space 4 exists");
    if !base.customs_track.slots[dummy_index].is_some_and(|s| s.player.is_dummy()) {
        return Err(format!(
            "fixture: the 2P setup must place the customs dummy on space {DUMMY_SPACE} (D1)"
        ));
    }
    if base.customs_track.total_assistants() != 0 {
        return Err("fixture: a fresh 2P customs track must carry no REAL pawn".into());
    }
    let supply = own_row(&base).assistants.total();

    // One position, on the customs side of scenario 16's constructor: the
    // viewer's pawns on `mine`, the rival's on `theirs`, the viewer's remaining
    // working assistants in one area, and both seats still holding the whole
    // printed supply.
    let position = |mine: &[u8], theirs: &[u8], on_board: u8| -> Result<PlayerView, String> {
        let mut v = base.clone();
        for (spaces, seat) in [(mine, 0u8), (theirs, 1u8)] {
            for (n, space) in spaces.iter().enumerate() {
                let index = CustomsTrack::space_to_index(*space)
                    .ok_or_else(|| format!("fixture: space {space} is not a customs space"))?;
                if v.customs_track.slots[index].is_some() {
                    return Err(format!("fixture: customs space {space} is occupied twice"));
                }
                v.customs_track.slots[index] =
                    Some(CustomsSlot { player: PlayerId(seat), arrival_order: n as u8 + 1 });
            }
        }
        for player in v.players.iter_mut() {
            let (pawns, board) = if player.id == PlayerId(0) {
                (mine.len() as u8, on_board)
            } else {
                (theirs.len() as u8, 0)
            };
            player.assistants.on_customs = pawns;
            player.assistants.on_board = BTreeMap::new();
            if board > 0 {
                player.assistants.on_board.insert(area, board);
            }
            player.assistants.in_hand = supply
                .checked_sub(player.assistants.in_warehouse + board + pawns)
                .ok_or("fixture: the position spends more assistants than a seat owns")?;
        }
        Ok(v)
    };

    let contested_abandon = position(&[], &[RIVAL_SPACE], 4)?;
    let contested_commit = position(&[MINE_FIRST], &[RIVAL_SPACE], 3)?;
    let uncontested_abandon = position(&[MINE_FIRST], &[], 4)?;
    let uncontested_commit = position(&[MINE_FIRST, MINE_SECOND], &[], 3)?;

    let all = [
        ("contested-abandon", &contested_abandon),
        ("contested-commit", &contested_commit),
        ("uncontested-abandon", &uncontested_abandon),
        ("uncontested-commit", &uncontested_commit),
    ];
    for (label, view) in all {
        let (p, clocks) = (progress_ppm(view, rules), clock_ppm(view, rules));
        if p != PROGRESS_FULL_PPM || clocks[0] != PROGRESS_FULL_PPM {
            return Err(format!(
                "fixture: the {label} position must sit at full progress on the DECK clock so the \
                 LATE vector governs exactly and the extra pawn cannot move the stage, got p={p} \
                 with clocks {clocks:?}"
            ));
        }
        if view_is_terminal(view) {
            return Err(format!(
                "fixture: the {label} position must still be scored POSITIONALLY — the terminal \
                 ladder short-circuits the weight vector this scenario is about"
            ));
        }
        for player in &view.players {
            if !player.id.is_dummy() && player.assistants.total() != supply {
                return Err(format!(
                    "fixture: the {label} position must leave seat {} holding the whole {supply}-\
                     token supply (a pawn on the track left the board and arrived on \
                     `on_customs`), it holds {}",
                    player.id.0,
                    player.assistants.total(),
                ));
            }
        }
    }

    // The two FIELDS, read off the structure: the commitment must win a place
    // where the rival contests and win none where the rival is absent.
    let awards = |v: &PlayerView| -> (i64, i64) {
        let structure = ScoreStructure::derive(v, rules);
        let value = |seat: u8| {
            let row = v.players.iter().find(|p| p.id == PlayerId(seat)).expect("both seats");
            structure.value_of(row, Feature::CustomsBonusNow)
        };
        (value(0), value(1))
    };
    let built = [
        ("contested-abandon", &contested_abandon, (0, 8)),
        ("contested-commit", &contested_commit, (8, 4)),
        ("uncontested-abandon", &uncontested_abandon, (8, 0)),
        ("uncontested-commit", &uncontested_commit, (8, 0)),
    ];
    for (label, view, want) in built {
        let got = awards(view);
        if got != want {
            return Err(format!(
                "fixture: the {label} position's customs majority must read {want:?} for (viewer, \
                 rival) — the CONTESTED field is the one where the commitment takes a place from \
                 the rival and the UNCONTESTED field the one where it takes none — it read {got:?}"
            ));
        }
    }

    // Clause (c)'s pair — the church side of the re-aim. Both seats hold one
    // church pawn; only the SPACE the viewer's sits on moves, and both landings
    // are right of the rival's, so the 6/3 awards are identical by construction.
    let church_landing = |mine: u8| -> Result<PlayerView, String> {
        let mut v = base.clone();
        for (faith, seat) in [(mine, 0u8), (8, 1u8)] {
            let index = ChurchTrack::faith_to_index(faith)
                .ok_or_else(|| format!("fixture: faith {faith} is not a church space"))?;
            if v.church_track.slots[index].is_some() {
                return Err(format!("fixture: faith {faith} is occupied twice"));
            }
            v.church_track.slots[index] =
                Some(ChurchSlot { player: PlayerId(seat), arrival_order: seat + 1 });
        }
        for player in v.players.iter_mut() {
            if !player.id.is_dummy() {
                player.assistants.on_church = 1;
                player.assistants.in_hand -= 1;
            }
        }
        Ok(v)
    };
    let (blocking, neutral) = (church_landing(10)?, church_landing(9)?);
    let church_awards = |v: &PlayerView| -> (i64, i64) {
        let structure = ScoreStructure::derive(v, rules);
        let value = |seat: u8| {
            let row = v.players.iter().find(|p| p.id == PlayerId(seat)).expect("both seats");
            structure.value_of(row, Feature::ChurchBonusNow)
        };
        (value(0), value(1))
    };
    if church_awards(&blocking) != (6, 3) || church_awards(&neutral) != (6, 3) {
        return Err(format!(
            "fixture: both church landings must award the viewer the 6 and the rival the 3, or \
             clause (c) is comparing two different majorities rather than two different spaces — \
             they read {:?} and {:?}",
            church_awards(&blocking),
            church_awards(&neutral),
        ));
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, PROGRESS_FULL_PPM)
        };
        let (customs, board) = (at(Feature::CustomsBonusNow), at(Feature::AssistantsOnBoard));
        let (a, b) = (score(&contested_commit, rules, w), score(&contested_abandon, rules, w));
        require(
            a > b,
            format!(
                "in a CONTESTED customs race the commitment must outrank leaving the assistant \
                 working — it takes the 8 and leaves the rival the 4, where the abandoned position \
                 had the rival on the 8 and the viewer on nothing: commit {a}, abandon {b}, margin \
                 {} (late weights: customs {customs}, board {board})",
                a - b,
            ),
        )?;
        let (c, d) = (score(&uncontested_commit, rules, w), score(&uncontested_abandon, rules, w));
        require(
            c < d,
            format!(
                "the SAME commitment with the rival ABSENT from the track must read below leaving \
                 the assistant working — the majority is 8 either way, so the second pawn only \
                 handicaps everything else: commit {c}, abandon {d}, margin {} (late weights: \
                 customs {customs}, board {board})",
                c - d,
            ),
        )?;
        let (e, f) = (score(&blocking, rules, w), score(&neutral, rules, w));
        require(
            e == f,
            format!(
                "the church side of the re-aim: two landings with IDENTICAL majority awards must \
                 score EQUAL — this rung prices the arithmetic of a place, never the denial of a \
                 space: faith 10 {e}, faith 9 {f}"
            ),
        )?;
        let separating = separating_terms(&contested_commit, &contested_abandon, rules, w);
        require(
            separating == ["customs_bonus_now", "assistants_on_board"],
            format!(
                "the contested pair must separate on the majority it wins and the assistant it \
                 spends, and on nothing else — it separated on {separating:?}"
            ),
        )
    };

    admit(
        SCENARIOS[16].0,
        "customs_bonus_now zeroed",
        "in a CONTESTED customs race the commitment must outrank leaving the assistant working",
        check(&WeightTable::V1),
        check(&zeroed(Feature::CustomsBonusNow)),
    )
}

#[test]
fn scenario_17_customs_race_and_the_rivals_progress() {
    run(16, "customs race: the rival's progress");
}

// ---------------------------------------------------------------------------
// 18. Achievement proximity as SELF-guidance  (3.5c W4 — protocol label 14,
//     re-aimed)
// ---------------------------------------------------------------------------

/// **RULES.** Three Achievement cards are in play for the whole game. A card is
/// claimed by placing an assistant on it once its printed requirement is MET,
/// the goods are not spent, and a seat may not claim the same card twice (KB
/// `01-rules-reference.md`, achievement categories; the gate is
/// `can_claim_achievement`). The three goods cards in this fixture print Copper
/// 5, Silk 6 and Tea 7. [`Feature::AchievementProgressSum`] mirrors the seat's
/// nearness to those claims as a SUM of legs, **each leg capped at its own
/// threshold** — capped because a seat sitting on 12 copper is no closer to the
/// 5-copper card than one sitting on 5, and because the VP of a card already
/// taken has landed in `victory_points` (see [`achievement_progress`]).
///
/// **SEAT.** 0, in a 2-player game at `p = 0` — a fresh deck and empty tracks,
/// so the EARLY vector governs exactly.
///
/// **THE CANON IT ENCODES — protocol label 14, RE-AIMED.** The drafted scenario
/// was achievement DENIAL and the owner ruled it **NOT CANON** (2026-08-09,
/// guidingpath's canon-batch row): achievements are **AGENDA-SETTERS** — *"use
/// them as guides to help set your agenda"*: which orders to take, where to
/// build, whether to chase 5-power tokens — and denial *"forgoes something
/// better"*; the anecdote attached to the ruling is a friend who wins by
/// building his whole strategy around achieving all three. The re-aim is
/// therefore **proximity as SELF-guidance**, and this scenario is that at the
/// grain the SHIPPED evaluator can express it.
///
/// **PROPERTY**, three clauses. (a) **THE EXPRESSIBLE HALF:** at EQUAL total
/// goods, a holding spread across LIVE legs reads above one that has overflowed
/// a single card past its threshold — 8 points of progress against 5, for the
/// same eight goods. (b) **THE BLIND SPOT, pinned as an EQUALITY:** two holdings
/// with the same capped SUM but very different nearness-to-claim — four copper,
/// one short of the Copper 5 card, against two copper and two silk, which is
/// three and four short of two cards — score EXACTLY EQUAL. The shipped feature
/// cannot tell them apart, and that is recorded here rather than wished away.
/// (c) The clause-(a) pair separates on exactly `achievement_progress_sum`.
///
/// **THE ARITHMETIC.** Trade goods are not features (§2.2 excludes them as
/// uninformative, R1-7) and the leftover conversions are end-scoring only, so
/// the ONLY term either pair moves is the progress sum: clause (a)'s margin is
/// `w_achievement(0) · (8 − 5) = 2 · 3 = +6`, and clause (b)'s is
/// `2 · (4 − 4) = 0`.
///
/// **WHAT THE SHIPPED GRAIN CANNOT SAY, and the V3 re-point.** "Nearer to claim
/// reads better at equal cost" is NOT expressible as a capped sum wherever the
/// caps do not bite: five copper and a spread 2/2/1 are the same 5 to this
/// feature, though one is a claim and the other is three separate errands. That
/// is exactly what round 3 MEASURED — `achievement_race_proximity` separates
/// **+0.054 EARLY and +0.094 MID** over the shipped progress sum in the shipped
/// regime, the one genuine find of the round's L1 pass, recorded as the named
/// FIRST CANDIDATE of the post-freeze V3 cycle and deliberately not admitted
/// pre-freeze (`docs/plans/2026-08-09-phase3.5c-round3-evidence.md`). Clause (b)
/// is the scenario's honest half: it pins what the rung does TODAY, so that the
/// day a proximity feature lands, this is the clause that gets RE-POINTED —
/// rewritten to an inequality in the same commit as the feature — and never
/// deleted (the file's never-delete rule; scenario 12 carries the same
/// instruction for the T3 filter, and it is the pattern this follows).
///
/// **WHICH TABLE.** [`WeightTable::V1`]. Both tables price this term at exactly
/// `2 → 4` — no round moved it — so the claim is table-independent and rides the
/// frozen table, sparing the freeze commit a re-point.
///
/// **COUNTERFACTUAL — `achievement_progress_sum` zeroed (§0.2's targeted
/// mutation).** With the term priced at nothing the two holdings of clause (a)
/// are indistinguishable, which is the same blindness clause (b) records — just
/// arrived at by pricing rather than by capping. Observed failing line,
/// verbatim:
///
/// ```text
/// achievement proximity as SELF-guidance: at eight goods either way, progress
/// on LIVE legs must outrank a holding that has overflowed one card past its
/// printed threshold — live 0, overflow 0, margin 0 (achievement weight 0,
/// capped progress 8 against 5)
/// ```
fn achievement_proximity_as_self_guidance(rules: &Rules) -> Result<(), String> {
    let mut base = base_view(2, 27, rules);
    // Set explicitly rather than taking the deal: three GOODS cards, so no leg
    // reads completed orders, owned technologies or buildings and the fixture's
    // only lever is the goods it hands the seat.
    base.achievements = [
        AchievementCard::new(AchievementType::Copper5),
        AchievementCard::new(AchievementType::Silk6),
        AchievementCard::new(AchievementType::Tea7),
    ];
    {
        let me = viewer_mut(&mut base);
        me.completed_orders.clear();
        me.technologies.clear();
        me.victory_points = 0;
        me.copper = 0;
        me.silk = 0;
        me.tea = 0;
        me.seafood = 0;
    }
    let holding = |copper: u32, silk: u32| -> PlayerView {
        let mut v = base.clone();
        let me = viewer_mut(&mut v);
        me.copper = copper;
        me.silk = silk;
        v
    };
    // Eight goods either way: five copper (a claim) plus three silk against
    // eight copper, three of which are past the Copper 5 cap and buy nothing.
    let (live, overflow) = (holding(5, 3), holding(8, 0));
    // Four goods either way, and the same capped sum: one copper short of a
    // claim, against a spread that is short of two.
    let (near, smeared) = (holding(4, 0), holding(2, 2));

    let goods = |v: &PlayerView| -> u32 {
        let me = own_row(v);
        me.copper + me.silk + me.tea + me.seafood
    };
    let progress = |v: &PlayerView| viewer_value(v, rules, Feature::AchievementProgressSum);
    for (label, a, b) in [("clause (a)", &live, &overflow), ("clause (b)", &near, &smeared)] {
        if goods(a) != goods(b) {
            return Err(format!(
                "fixture: {label}'s two holdings must carry the SAME number of goods — otherwise \
                 the richer one is simply richer and the comparison says nothing about nearness — \
                 they carry {} and {}",
                goods(a),
                goods(b),
            ));
        }
    }
    if (progress(&live), progress(&overflow)) != (8, 5) {
        return Err(format!(
            "fixture: the LIVE holding must read 8 points of capped progress and the OVERFLOW one \
             5 (three of its eight copper sit past the Copper 5 cap), they read {} and {}",
            progress(&live),
            progress(&overflow),
        ));
    }
    if progress(&near) != progress(&smeared) {
        return Err(format!(
            "fixture: clause (b)'s two holdings must read the SAME capped progress — that equality \
             IS the blind spot the clause records — they read {} and {}",
            progress(&near),
            progress(&smeared),
        ));
    }
    // The nearness the shipped sum cannot see, asserted so the clause is not
    // quietly comparing two equally-distant holdings: one is a single good from
    // a claim, the other is three and four goods from two.
    let short_of = |v: &PlayerView, card: AchievementType| -> i64 {
        let (current, needed) = achievement_progress(v, own_row(v), card);
        i64::from(needed) - i64::from(current.min(needed))
    };
    if short_of(&near, AchievementType::Copper5) != 1
        || short_of(&smeared, AchievementType::Copper5) != 3
        || short_of(&smeared, AchievementType::Silk6) != 4
    {
        return Err(format!(
            "fixture: clause (b)'s NEAR holding must sit one good from the Copper 5 claim and its \
             SMEARED twin three and four from two claims, they sit {} / {} and {}",
            short_of(&near, AchievementType::Copper5),
            short_of(&smeared, AchievementType::Copper5),
            short_of(&smeared, AchievementType::Silk6),
        ));
    }
    for (label, view) in [("live", &live), ("near", &near)] {
        let p = progress_ppm(view, rules);
        if p != 0 {
            return Err(format!(
                "fixture: the {label} position must sit at p=0 so the EARLY vector governs \
                 exactly, got {p}"
            ));
        }
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let (early, late) = w.get(Feature::AchievementProgressSum);
        let weight = interpolate(early, late, 0);
        let (a, b) = (score(&live, rules, w), score(&overflow, rules, w));
        require(
            a > b,
            format!(
                "achievement proximity as SELF-guidance: at eight goods either way, progress on \
                 LIVE legs must outrank a holding that has overflowed one card past its printed \
                 threshold — live {a}, overflow {b}, margin {} (achievement weight {weight}, \
                 capped progress 8 against 5)",
                a - b,
            ),
        )?;
        let (c, d) = (score(&near, rules, w), score(&smeared, rules, w));
        require(
            c == d,
            format!(
                "the BLIND SPOT this rung still has, pinned rather than wished away: a holding one \
                 good from a claim and one that is three and four goods from two claims carry the \
                 same capped sum and must score EQUAL here — near {c}, smeared {d}. Re-point this \
                 clause to an inequality in the same commit that admits a proximity feature \
                 (round 3's `achievement_race_proximity`, +0.054 early / +0.094 mid); never \
                 delete it"
            ),
        )?;
        let separating = separating_terms(&live, &overflow, rules, w);
        require(
            separating == ["achievement_progress_sum"],
            format!(
                "the clause-(a) pair must separate on the progress sum alone — trade goods are not \
                 features — it separated on {separating:?}"
            ),
        )
    };

    admit(
        SCENARIOS[17].0,
        "achievement_progress_sum zeroed",
        "progress on LIVE legs must outrank a holding that has overflowed",
        check(&WeightTable::V1),
        check(&zeroed(Feature::AchievementProgressSum)),
    )
}

#[test]
fn scenario_18_achievement_proximity() {
    run(17, "achievement proximity as self-guidance");
}

// ---------------------------------------------------------------------------
// 19. Behind at the end trigger, keep building  (3.5c W4 — protocol label 15,
//     INVERTED by the ruling)
// ---------------------------------------------------------------------------

/// **RULES.** End scoring is running VP PLUS the unbanked end bonuses, and the
/// technology bonus is one of them: 10 VP to the largest total printed industry
/// value and 5 to the second, ties broken clockwise, with the 2P setup dummy
/// competing at industry value exactly **7.5** (finding D5, owner ruling
/// 2026-08-08, rulebook p.9 final-scoring NOTE) — so a 2P seat needs a printed
/// gear total of 8 to take the majority outright, and a seat owning no
/// technology never places at all. An Order card, by contrast, pays its printed
/// VP the moment it is completed and compounds no further; Order #3 (GB) prints
/// `seafood 1 · copper 2 → 4 VP, 1 Yen`.
///
/// **SEAT.** 0, in a 2-player game, deck run out so `p = 1,000,000` — the LATE
/// vector, i.e. the stage at which `victory_points` carries its LARGEST weight
/// in the whole table (40). This is the hardest place to make this scenario's
/// claim, which is why it is made here.
///
/// **THE CANON IT ENCODES — protocol label 15, and the ruling INVERTED the
/// drafted scenario.** The draft was an end-trigger BRAKE: a seat behind near
/// the end should cash what it can. The owner ruled **NO** (2026-08-09,
/// guidingpath's canon-batch row): a behind player **KEEPS BUILDING** — *"focus
/// on the flags from your orders and tech cards, build more orders, look for
/// synergies, make each turn efficient"* — because scores diverge and then
/// *"swing enormously in the final two turns on end bonuses (order flags
/// included)"*, so being behind mid-game is not decisive. This scenario is the
/// inverted ruling: near the end trigger and behind, the end-scoring structure
/// outranks the printed cash.
///
/// **PROPERTY**, three clauses. (a) **BEHIND, LATE:** `score(the two
/// technologies that clear the D5 dummy and take the 10/5 majority)` >
/// `score(the same seat having banked Order #3's printed 4 VP instead)`, with
/// the rival 20 VP ahead in both. (b) **NO BRAKE EXISTS, and that is the
/// ruling's content:** the SAME pair with the viewer AHEAD instead of behind
/// produces the IDENTICAL margin — the preference does not depend on the deficit
/// at all, so there is no end-trigger brake for the deficit to arm. (c) The pair
/// separates on exactly `victory_points` and `tech_bonus_now`.
///
/// **THE ARITHMETIC, and the boundary it draws.** The margin is
/// `w_tech · 10 − w_vp · Δvp` at `p = 1,000,000`, i.e. `20·10 − 40·4 = 200 − 160
/// = +40` under the frozen table. Read the boundary off that rather than the
/// single number: at this stage the 10-VP majority outprices ANY printed cash
/// below 5 VP and loses to anything above it, because a banked VP is priced at
/// 40 and an end-scoring VP inside the technology bonus at 20. The scenario
/// pins the side of that boundary the canon is about; it does not claim the
/// boundary is in the right place, which is a magnitude question and therefore
/// 3.5c's tuning lane rather than a scenario's.
///
/// **WHY CLAUSE (b) IS STRUCTURAL, said plainly.** Every term of this evaluator
/// is `w · (mine − theirs)`, so a rival's running VP enters BOTH positions of a
/// pair identically and cancels out of the margin. The deficit therefore CANNOT
/// change this preference, whatever the table says — which is exactly the
/// ruling ("being behind is not a reason to change plan"), and exactly why the
/// clause is worth pinning: a future term keyed to the seat's own deficit — the
/// brake the drafted scenario 15 assumed — would have to break this clause to
/// exist, and would then have to answer for the canon here.
///
/// **FIXTURE HONESTY.** The two positions are constructed to isolate the two
/// end-scoring shapes; they are not a claim that these are the two legal moves
/// at some real decision point (the goods and gears they cost are different
/// currencies, and the evaluator sees one ply). Scenario 10 states the same
/// caveat for the same reason. The CASH position banks 4 VP beside an empty
/// completed-order pile, which no real seat would have — deliberately, because
/// pricing the icons and the VP together is the thing this pair is separating.
/// Both holdings are ENGLISH technologies, so the country-set bonus is 0 on both
/// sides (one distinct country forms no set) and the pair cannot leak into it.
///
/// **WHICH TABLE.** [`WeightTable::V1`], on this file's W4 rule: the two tables
/// carry identical LATE weights for both terms (`vp 40`, `tech 20`), so the
/// claim is table-independent and the margin is +40 under the draft too.
///
/// **COUNTERFACTUAL — `tech_bonus_now` zeroed (§0.2's targeted mutation).** With
/// the majority priced at nothing, the structure is worth nothing and the printed
/// cash wins by the whole of its weighted 4 VP. Observed failing line, verbatim:
///
/// ```text
/// BEHIND at the end trigger, the engine must still outrank the cash: the two
/// technologies that clear the D5 dummy take the 10/5 majority, where banking
/// order 3's printed 4 VP compounds no further — build -800, cash -640, margin
/// -160 (late weights: vp 40, tech 0)
/// ```
fn behind_at_the_end_trigger_keeps_building(rules: &Rules) -> Result<(), String> {
    /// The rival's running VP in the BEHIND field, and the viewer's.
    const RIVAL_VP: u32 = 30;
    const MY_VP: u32 = 10;

    // The cash side's magnitude is a PRINTED reward, read off the registry.
    let cash_card = OrderCardId(3);
    let card = rules.orders.get(cash_card).ok_or("fixture: order 3 is not in the registry")?;
    let cash_vp = u32::from(card.rewards.victory_points);
    if cash_vp == 0 || cash_vp >= 5 {
        return Err(format!(
            "fixture: order 3's printed VP must sit strictly under 5 — the boundary the doc's \
             arithmetic draws — it prints {cash_vp}"
        ));
    }
    // The build side: two ENGLISH gear-4 technologies, printed gear total 8, so
    // the doubled 16 clears the D5 dummy's 15 and takes the majority outright.
    let techs = techs_of_country_at_gear(rules, Country::English, 4);
    if techs.len() < 2 {
        return Err(format!(
            "fixture: England must print at least two gear-4 technologies, the registry has {}",
            techs.len(),
        ));
    }
    let holding = vec![techs[0], techs[1]];
    let industry: u32 = holding
        .iter()
        .filter_map(|id| rules.techs.get(*id))
        .map(|c| u32::from(c.industry_cost))
        .sum();
    if 2 * industry <= 15 {
        return Err(format!(
            "fixture: the holding must out-industry the 2P dummy's 7.5 (D5), its printed gears \
             total {industry}"
        ));
    }
    let mut titles: Vec<&str> =
        holding.iter().filter_map(|id| rules.techs.get(*id)).map(|c| c.name.as_str()).collect();
    let owned = titles.len();
    titles.sort_unstable();
    titles.dedup();
    if titles.len() != owned {
        return Err(format!(
            "fixture: a seat may own only one Technology of a NAME — this holding carries {titles:?}"
        ));
    }

    let mut base = base_view(2, 28, rules);
    // The deck run out: the LATE vector, and the end trigger one empty Port slot
    // away (scenario 11's note on the conjunct `clock_ppm` drops).
    base.order_deck_len = 0;
    // Three GOODS cards, so neither the completed-order count nor the owned-tech
    // count feeds an achievement leg and clause (c) stays a two-term claim.
    base.achievements = [
        AchievementCard::new(AchievementType::Copper5),
        AchievementCard::new(AchievementType::Silk6),
        AchievementCard::new(AchievementType::Tea7),
    ];
    {
        let me = viewer_mut(&mut base);
        me.completed_orders.clear();
        me.technologies.clear();
        me.victory_points = MY_VP;
    }

    // `rival_vp` is the ONLY thing that separates the behind field from the
    // ahead one — clause (b)'s whole point.
    let field = |rival_vp: u32| -> (PlayerView, PlayerView) {
        let mut base = base.clone();
        for player in base.players.iter_mut() {
            if player.id == PlayerId(1) {
                player.victory_points = rival_vp;
            }
        }
        let mut cash = base.clone();
        viewer_mut(&mut cash).victory_points += cash_vp;
        let mut build = base.clone();
        viewer_mut(&mut build).technologies = holding.clone();
        (cash, build)
    };
    let (behind_cash, behind_build) = field(RIVAL_VP);
    let (ahead_cash, ahead_build) = field(0);

    for (label, view) in [
        ("behind-cash", &behind_cash),
        ("behind-build", &behind_build),
        ("ahead-cash", &ahead_cash),
        ("ahead-build", &ahead_build),
    ] {
        let (p, clocks) = (progress_ppm(view, rules), clock_ppm(view, rules));
        if p != PROGRESS_FULL_PPM || clocks[0] != PROGRESS_FULL_PPM {
            return Err(format!(
                "fixture: the {label} position must sit at full progress on the DECK clock so the \
                 LATE vector governs exactly, got p={p} with clocks {clocks:?}"
            ));
        }
        if view_is_terminal(view) {
            return Err(format!(
                "fixture: the {label} position must still be scored POSITIONALLY — the terminal \
                 ladder short-circuits the weight vector this scenario is about"
            ));
        }
    }
    // The premise of the whole scenario, asserted: BEHIND means behind on
    // running VP in both positions of the field, and ahead means ahead.
    let standing = |v: &PlayerView| -> (i64, i64) {
        let structure = ScoreStructure::derive(v, rules);
        (structure.base_vp[0], structure.base_vp[1])
    };
    for (label, view) in [("behind-cash", &behind_cash), ("behind-build", &behind_build)] {
        let (mine, theirs) = standing(view);
        if mine >= theirs {
            return Err(format!(
                "fixture: the {label} position must leave the viewer BEHIND on running VP — that \
                 is the ruling's premise — it reads {mine} against {theirs}"
            ));
        }
    }
    for (label, view) in [("ahead-cash", &ahead_cash), ("ahead-build", &ahead_build)] {
        let (mine, theirs) = standing(view);
        if mine <= theirs {
            return Err(format!(
                "fixture: the {label} position must leave the viewer AHEAD on running VP, it reads \
                 {mine} against {theirs}"
            ));
        }
    }
    // The two end-scoring values, read off the structure rather than assumed:
    // the build takes the majority outright, the cash position places nowhere,
    // and neither forms a country set.
    let value = |v: &PlayerView, feature: Feature| viewer_value(v, rules, feature);
    let built = [
        ("behind-build", &behind_build, Feature::TechBonusNow, 10),
        ("behind-cash", &behind_cash, Feature::TechBonusNow, 0),
        ("behind-build", &behind_build, Feature::CountryBonusNow, 0),
        ("behind-cash", &behind_cash, Feature::CountryBonusNow, 0),
    ];
    for (label, view, feature, want) in built {
        let got = value(view, feature);
        if got != want {
            return Err(format!(
                "fixture: {} must read {want} on the {label} position, it read {got}",
                feature.name(),
            ));
        }
    }

    let check = |w: &WeightTable| -> Result<(), String> {
        let at = |f: Feature| {
            let (e, l) = w.get(f);
            interpolate(e, l, PROGRESS_FULL_PPM)
        };
        let (vp, tech) = (at(Feature::VictoryPoints), at(Feature::TechBonusNow));
        let (a, b) = (score(&behind_build, rules, w), score(&behind_cash, rules, w));
        require(
            a > b,
            format!(
                "BEHIND at the end trigger, the engine must still outrank the cash: the two \
                 technologies that clear the D5 dummy take the 10/5 majority, where banking order \
                 3's printed {cash_vp} VP compounds no further — build {a}, cash {b}, margin {} \
                 (late weights: vp {vp}, tech {tech})",
                a - b,
            ),
        )?;
        let (c, d) = (score(&ahead_build, rules, w), score(&ahead_cash, rules, w));
        require(
            a - b == c - d,
            format!(
                "there is no END-TRIGGER BRAKE for a deficit to arm: the same pair with the viewer \
                 AHEAD must produce the IDENTICAL margin, since every term is relative and the \
                 rival's running VP cancels — behind {} , ahead {}",
                a - b,
                c - d,
            ),
        )?;
        let separating = separating_terms(&behind_build, &behind_cash, rules, w);
        require(
            separating == ["victory_points", "tech_bonus_now"],
            format!(
                "the pair must separate on the banked VP and the technology majority, and on \
                 nothing else — it separated on {separating:?}"
            ),
        )
    };

    admit(
        SCENARIOS[18].0,
        "tech_bonus_now zeroed",
        "BEHIND at the end trigger, the engine must still outrank the cash",
        check(&WeightTable::V1),
        check(&zeroed(Feature::TechBonusNow)),
    )
}

#[test]
fn scenario_19_behind_at_the_end_trigger() {
    run(18, "behind at the end trigger: keep building");
}

// ---------------------------------------------------------------------------
// The runner
// ---------------------------------------------------------------------------

/// Run scenario `index`, first checking that the registry still holds the
/// scenario this test is named for — so a reordering of [`SCENARIOS`] cannot
/// silently re-point a test at a different situation.
fn run(index: usize, expected: &str) {
    let (name, scenario) = SCENARIOS[index];
    assert_eq!(
        name, expected,
        "SCENARIOS[{index}] is `{name}`, but this test is written for `{expected}` — the \
         registry was reordered without re-pointing the tests",
    );
    if let Err(failure) = scenario(rules()) {
        panic!("{failure}");
    }
}
