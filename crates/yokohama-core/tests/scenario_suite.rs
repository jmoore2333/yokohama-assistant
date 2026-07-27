//! **Scenario suite v1** (Phase 3.3 T9; spec §6, admitted under §0.2) — nine
//! hand-built rules situations the evaluator rung has to get right, each one a
//! single falsifiable property with a rules citation, a WHY line, and a
//! demonstrated failing counterpart.
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
//! carries seven scenarios; *the explanation's top term names X* carries
//! country-set completion's second clause (the explanation must attribute the
//! whole margin to `country_bonus_now` and to nothing else, which doubles as
//! that fixture's leak check); and *chosen move ∈ set* carries the
//! null-placement trap, in its aggregate form — see that scenario's doc for why
//! a count over a segment is the right reading of it under a randomised
//! tie-break.
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
//! Eight of the nine are EVAL-ORDERING properties over constructed views: the
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
//! reachability is pinned by four other suites. The two scenarios whose claim
//! involves the ENGINE's agreement (the D1 dummy's majority, the terminal
//! ranking) build real `GameState`s instead and cross-check against
//! `final_result`.
//!
//! # Determinism
//!
//! Every scenario is deterministic. The eight eval-ordering ones consume no
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
//! fixture invariant audits that wave added inside the figure.
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

use std::collections::BTreeMap;
use std::sync::OnceLock;

use yokohama_core::game::actions::calculate_church_faith;
use yokohama_core::game::eval::{
    clock_ppm, explain, interpolate, place_signal, progress_ppm, score, terminal_standing,
    view_is_terminal, Feature, ScoreStructure, WeightTable, TERMINAL_MARGIN_STEP,
    TERMINAL_RANK_STEP,
};
use yokohama_core::game::greedy::{GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use yokohama_core::{
    apply_move_unchecked, check_invariants, final_result, finalize_setup, is_game_over, new_game,
    observe, AchievementCard, AchievementType, ArenaAgent, ChurchSlot, ChurchTrack, Country,
    CustomsSlot, CustomsTrack, EndGameTrigger, GameConfig, GameState, HandCard, Move, OrderCardId,
    PlayerId, PlayerView, Rules, StartingOrderChoice, TurnPhase, VisiblePlayer,
};

// ---------------------------------------------------------------------------
// The registry (the never-delete rule's teeth)
// ---------------------------------------------------------------------------

/// One scenario: its stable name and the function that runs BOTH halves of
/// §0.2's admission — the shipped property and its targeted counterfactual.
type ScenarioFn = fn(&Rules) -> Result<(), String>;

/// **THE NINE SCENARIOS OF SUITE v1.** Spec §6's eight categories plus the
/// terminal-scoring scenario that section adds.
///
/// The length is pinned by the type. Adding a tenth scenario is a deliberate
/// edit to this array and its `#[test]`; removing one of these nine is the
/// thing the never-delete rule forbids, and it will not compile.
const SCENARIOS: [(&str, ScenarioFn); 9] = [
    ("country-set completion", country_set_completion),
    ("order economics: fulfill now vs hold", order_economics_fulfill_now_vs_hold),
    ("end-trigger timing: the church clock", end_trigger_timing_church_clock),
    ("church/customs majority: the D1 dummy", customs_majority_with_the_d1_dummy),
    ("building race", building_race_trading_house_over_shophouse),
    ("retreat/recovery discipline", recovery_discipline_stops_at_the_reserve),
    ("faith arithmetic", faith_arithmetic_buys_the_rightmost_church_space),
    ("the null-placement trap", the_null_placement_trap),
    ("terminal scoring", terminal_scoring_prefers_the_bonus_win),
];

/// Spec §6's never-delete rule, plus the registry hygiene the nine `#[test]`s
/// lean on: each test names both its index and its scenario name, so a
/// reordering that silently re-pointed a test at a different scenario fails
/// here and there.
#[test]
fn the_registry_holds_nine_uniquely_named_scenarios() {
    assert_eq!(
        SCENARIOS.len(),
        9,
        "spec §6 is eight categories plus the terminal-scoring scenario; scenarios are \
         never deleted (this file's header states the rule)",
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
