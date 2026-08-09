//! **Scenario suite** (Phase 3.3 T9; spec §6, admitted under §0.2) — twelve
//! hand-built rules situations the evaluator rung has to get right, each one a
//! single falsifiable property with a rules citation, a WHY line, and a
//! demonstrated failing counterpart.
//!
//! # The two waves, and which table each one is written against
//!
//! **Scenarios 1–9 are suite v1** (Phase 3.3 T9), authored against the frozen
//! [`WeightTable::V1`] — the table the bot plays today. **Scenarios 10–12 are
//! the Phase-3.5c round-1 growth** (protocol
//! `docs/plans/2026-08-09-phase3.5c-evaluator-v2-protocol.md` §4.2 rung L3 and
//! §6's growth list), and they are written against
//! [`WeightTable::V2_DRAFT`] — the ITERATION CANDIDATE that becomes V2 at the
//! freeze commit — because L3's whole job is that *"every adopted change has at
//! least one scenario that lands red-then-green against it"*. A round's
//! scenario is therefore authored BEFORE the weights move, fails against the
//! unmoved draft, and turns green in the adoption commit; its counterfactual is
//! then the round's own edit REVERTED rather than a zeroed term.
//!
//! Nothing about scenarios 1–9 changed when 10–12 landed. The never-delete rule
//! below covers both waves alike.
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
//! carries nine scenarios; *the explanation's top term names X* carries
//! country-set completion's second clause and both round-1 re-pricing scenarios'
//! leak clause (the explanation must attribute the margin to a NAMED set of
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
//! # What this suite does NOT cover (T9 review S3)
//!
//! Spec §6's category (3), end-trigger timing, names TWO clocks: the church
//! track filling, and the order deck running near-empty so that unbanked cards
//! have to be converted. The delivered scenario discharges the category through
//! the CHURCH clock alone, per T4's routing of the church sign-flip question to
//! this suite. **Nothing here exercises the deck half.** Scenario 5 sets
//! `order_deck_len` and scenario 9 leaves a shortened deck behind, but both use
//! it purely as STAGE-SETTING — a stage proxy the position is dated by, never a
//! decision about banking before the deck runs out. Recorded here so suite
//! growth has a named target, and so no reader takes "end-trigger timing:
//! covered" for more than the one clock it is.
//!
//! # Cost, and why most scenarios drive no game
//!
//! Eleven of the twelve are EVAL-ORDERING properties over constructed views (ten
//! over [`score`], one over a feature function): the evaluator is a pure
//! function of a [`PlayerView`], so specifying its input directly is both the
//! cheapest and the most precise way to state "this position must outrank that
//! one". Only the null-placement trap is genuinely BEHAVIOURAL — "the bot does
//! not spend the game placing nothing" is a statement about picks, not about
//! scores — and it pays for one short driven segment (40 decisions, twice:
//! shipped and mutated).
//!
//! The constructed views are DOCTORED: built by `observe`ing a real game and
//! then overwriting the fields the scenario is about. Several are not states
//! the engine could reach (a hand-set track, a truncated deck), and that is
//! fine and stated — the evaluator's entire input is the view, so a doctored
//! view is a complete specification of an evaluation, and the engine's own
//! reachability is pinned by four other suites. The two scenarios whose claim
//! involves the ENGINE's agreement (the D1 dummy's majority, the terminal
//! ranking) build real `GameState`s instead and cross-check against
//! `final_result`.
//!
//! # Determinism
//!
//! Every scenario is deterministic. The eleven eval-ordering ones consume no
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
//! tests) — see below for the figure and for why it did not move.
//!
//! The split is the interesting figure and it is why the shapes were chosen
//! this way: the eight eval-ordering scenarios plus the registry pin run in
//! **0.00 s** (below the timer's resolution — one shared [`rules`] build, nine
//! setup-and-`observe` fixtures, fourteen [`check_invariants`] fixture audits,
//! and about forty `score` calls), and the null-placement scenario's two
//! 40-decision Greedy-v1 segments are **0.05 s**, i.e. effectively the whole
//! binary. Order-seconds was the budget; the suite came in two orders under it,
//! and the room that buys is for the driven scenarios a later revision will
//! want.
//!
//! **Round 1's three additions spend that room and barely touch the figure**
//! (0.06 / 0.05 / 0.05 s for the whole binary at the round-1 close, same
//! machine, same method): all three are constructed-view properties with no
//! driven segment, so each costs one setup-and-`observe` plus a handful of
//! `score`/`explain` calls. The driven scenario is still effectively the whole
//! binary, and a future round wanting a BEHAVIOURAL scenario (scenario 5's doc
//! names the one it would most like) still has the budget for it.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use yokohama_core::game::actions::calculate_church_faith;
use yokohama_core::game::eval::{
    achievement_progress, achievement_progress_postclaim_sum, clock_ppm, explain, interpolate,
    place_signal, progress_ppm, score, terminal_standing, view_is_terminal, Feature, ScoreStructure,
    WeightTable, HAND_CORE_RESERVE, PROGRESS_FULL_PPM, TERMINAL_MARGIN_STEP, TERMINAL_RANK_STEP,
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

/// **THE TWELVE SCENARIOS.** Indices 0–8 are suite v1 — spec §6's eight
/// categories plus the terminal-scoring scenario that section adds. Indices
/// 9–11 are the Phase-3.5c round-1 growth, one per adopted change: the early
/// re-mix, the warehouse flatten (protocol scenario 16), and the T3 post-claim
/// filter (protocol scenario 17).
///
/// The length is pinned by the type. Adding a thirteenth scenario is a
/// deliberate edit to this array and its `#[test]`; removing one of these twelve
/// is the thing the never-delete rule forbids, and it will not compile.
const SCENARIOS: [(&str, ScenarioFn); 12] = [
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
];

/// Spec §6's never-delete rule, plus the registry hygiene the twelve `#[test]`s
/// lean on: each test names both its index and its scenario name, so a
/// reordering that silently re-pointed a test at a different scenario fails
/// here and there.
#[test]
fn the_registry_holds_twelve_uniquely_named_scenarios() {
    assert_eq!(
        SCENARIOS.len(),
        12,
        "spec §6's eight categories plus the terminal-scoring scenario (suite v1's nine), \
         plus the Phase-3.5c round-1 growth's three; scenarios are never deleted (this \
         file's header states the rule)",
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

/// The viewer's own raw value for one feature — the same number [`score`]
/// multiplies by a weight, read without a weight so a fixture can state what it
/// built (`ScoreStructure` is the authority on all sixteen; ten are field reads
/// and five are re-derived end scoring).
fn viewer_value(view: &PlayerView, rules: &Rules, feature: Feature) -> i64 {
    ScoreStructure::derive(view, rules).value_of(own_row(view), feature)
}

/// Which features' EXPLAINED products separate two views under `w` — the leak
/// check both re-pricing scenarios lean on, in [`Feature::ALL`] order.
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

/// The round-1 EARLY re-mix, REVERTED inside the candidate table: the five early
/// slots the round moved, put back to [`WeightTable::V1`]'s values, and nothing
/// else touched.
///
/// This is §0.2's targeted mutation for scenario 10, in the shape an ADOPTION
/// round needs. Suite v1's mutations zero one term because a v1 scenario asks
/// "does the shipped table price this term at all"; a round-1 scenario asks
/// "does the round's own edit carry this position", so the mutation is the edit
/// undone. Zeroing a term here would prove only that the term matters — which
/// scenarios 1, 2 and 8 already prove for three of these five — and would say
/// nothing about the re-mix.
///
/// Each slot's LATE weight is taken from the DRAFT, not from `V1`, so the
/// mutation is exactly the EARLY re-mix and not a whole-column revert. (For
/// these five the two late values coincide — the round moved only their early
/// entries — so this is a statement about what the code MEANS rather than about
/// what it currently computes, and it stays correct if a later round moves one
/// of their late weights.)
fn early_remix_reverted() -> WeightTable {
    let mut w = WeightTable::V2_DRAFT;
    for feature in EARLY_REMIX_SLOTS {
        let (v1_early, _) = WeightTable::V1.get(feature);
        let (_, draft_late) = WeightTable::V2_DRAFT.get(feature);
        w.set(feature, v1_early, draft_late);
    }
    w
}

/// The five EARLY slots round 1 re-mixed, in [`Feature::ALL`] order — the
/// round's change-set (a) made a list, and the exact set scenario 10's fixture
/// is built to separate on.
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
/// round-1 candidate that reads
/// `2·(−12) + 18·4 + 12·10 + 5·3 + 6·(−3) = −24 + 72 + 120 + 15 − 18 = +165`;
/// under V1's early weights it reads
/// `10·(−12) + 12·4 + 6·10 + 4·3 + 14·(−3) = −120 + 48 + 60 + 12 − 42 = −42`.
/// The same two positions, ranked opposite ways by the round's edit — which is
/// what "the re-mix moves the MIX, not the scale" buys (R0 finding F8: halving
/// the whole early vector is play-identical, so only a change of RATIOS can move
/// a decision).
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
/// (`docs/plans/2026-08-09-phase3.5c-round1-evidence.md`). This scenario is
/// protocol §4.2 rung L3 for that change: the DECISION the re-priced ratios
/// make, pinned so a later round cannot quietly undo it without answering here.
///
/// **What it does NOT claim.** Not that the engine position WINS the game — this
/// is one constructed pair, not a corpus, and the corpus evidence is an AUC over
/// held-out games, quoted where it lives. What is pinned is that the shipped
/// rung, handed these two positions early, prefers the builder.
///
/// **COUNTERFACTUAL — the five re-mixed EARLY slots reverted to V1
/// (`vp 10, ctry 12, tech 6, excess 4, board 14`; [`early_remix_reverted`]).**
/// Observed failing line, verbatim:
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
        "the five re-mixed EARLY slots reverted to V1 (vp 10, ctry 12, tech 6, excess 4, board 14)",
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
        return Err(format!(
            "fixture: the shipped sum must value the two positions EQUALLY and nonzero (that \
             equality is the double-count blindness the filter removes), got {u_live} and \
             {u_dead}"
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
