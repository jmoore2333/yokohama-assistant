//! Per-edit tests for evaluator v1's core (Phase 3.3 Tasks T1 and T2) — spec
//! §2.1's source-text guard, §2.2's relative form and score-structure DRIFT
//! PIN, §2.3's stage interpolation, §2.6's terminal term, and §0.3(a)'s
//! two-path `score`/`explain` requirement.
//!
//! WHY THIS LIVES IN AN INTEGRATION BINARY AND NOT IN `game/eval.rs`'s own
//! `#[cfg(test)] mod tests`. The §2.1 guard `include_str!`s `eval.rs` and
//! asserts the forbidden identifiers do not OCCUR in it. A guard living inside
//! the file it scans would match its own needles, so the needles would have to
//! be spelled in pieces (`concat!("Game", "State")`) — a cute trick a reader
//! has to decode before trusting the guard. Keeping the scanner in a different
//! file makes the needles literal. The same reason applies to everything else
//! here: these tests need `new_game`/`observe`/`apply_move_unchecked`, and
//! importing those INSIDE `eval.rs` (even under `#[cfg(test)]`) would put
//! `use crate::game::` lines in the scanned file and force the allowlist open.
//! Only the pure-arithmetic unit tests stay in the module.
//!
//! POINTER, because it is easy to miss: the T4 corpus instrument
//! (`examples/eval_probe.rs`) carries its OWN unit tests over its statistics —
//! the AUC estimator, the cluster-bootstrap multiplicity, the stage terciles,
//! §2.4's materiality rule — and `cargo test -p yokohama-core` does NOT run an
//! example's test harness. They run under
//! `cargo test -p yokohama-core --example eval_probe` (or `--all-targets`), and
//! that command is named in the probe's module doc and in
//! `docs/plans/2026-07-27-phase3.3-weight-freeze.md` §2. A test no documented
//! command runs is a test that never runs, so it is written down in three
//! places on purpose.
//!
//! Tier: per-edit, never `#[ignore]`d. Re-measured at the T2 close (debug,
//! three runs): 0.54 / 0.53 / 0.53 s, up from T1's 0.16 s. The added cost was
//! A/B'd rather than guessed, because the obvious attribution is the wrong one:
//! deleting `check_drift` from the sweep moves the sweep test only 0.55 → 0.50 s,
//! so §2.2's drift pin is ~10% of the increase and the rest is `check_view`'s
//! own `raw_value` calls, which are deliberately the CONVENIENCE accessor —
//! each derives a whole `ScoreStructure` to answer about one feature, 16
//! features x (1 + opponents) per view (15 before T3's in-hand split). That is
//! the price of checking `explain`
//! against an independently-assembled value instead of against the structure it
//! was built from, and it is worth paying here; `score` itself derives once per
//! call and is unaffected (0 allocations, below). Still cheap overall because
//! this file does NOT run `check_invariants` per move: the engine is not its
//! subject and four other per-edit suites already carry that net.
//!
//! §0.3(a)'s allocation figures, re-measured at T3: `score` 0 allocations / 0
//! peak bytes, `explain` 1 allocation / 640 peak bytes (SIXTEEN 40-byte terms;
//! 600 for fifteen before the split — the delta is exactly one term, which is
//! the check that the split added a feature and not a heap block). The
//! `Explanation` grew an `Option<TerminalStanding>` at T2 and the allocation
//! COUNT did not move — the standing rides in the struct, not in the heap
//! block.

// The shared counting allocator, included the same way
// `tests/action_source_worstcase.rs` includes it (each includer links its own
// private `#[global_allocator]`; no conflict with the lib-only probe in
// `game/search_actions.rs`). Used by exactly one test here — the §0.3(a)
// two-path pin, which is a statement about ALLOCATION and cannot be made any
// other way.
#[path = "action_cost_meter/mod.rs"]
mod meter;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use yokohama_core::game::driven_families::visible_power;
use yokohama_core::actions::exported_reward;
use yokohama_core::game::eval::{
    achievement_progress as view_achievement_progress, achievement_progress_postclaim_sum,
    achievement_progress_sum as prefreeze_achievement_sum, area_bit, areas_in, clock_ppm,
    countries_in, country_bit, explain, initial_order_deck_len, interpolate, lab_bit, order_bit,
    order_cards_to_remove,
    place_signal, production_good, progress_ppm, raw_value, realizable_power, score, set_bonus,
    terminal_standing,
    track_trigger, view_is_terminal, CountrySet, Explanation, Feature, OrderSet, ScoreStructure,
    WeightTable, LAB_SLOTS, PROGRESS_FULL_PPM, TERMINAL_MARGIN_CLAMP, TERMINAL_MARGIN_STEP,
    TERMINAL_RANK_STEP,
};
use yokohama_core::{
    achievement_progress, apply_move_unchecked, calculate_power, final_result, finalize_setup,
    get_legal_moves, is_game_over, is_possible_action_area_full, new_game, observe, AreaAction,
    AreaId, AreaType, ChurchSlot, ChurchTrack, Country, CustomsSlot, CustomsTrack, DecisionHandle,
    GameConfig, GameState, HandCard, Move, OrderCardId, PlayerId, PlayerView, PortBoard,
    PresidentLocation, RolloutPolicy, Rules, ScoreBreakdown, ShophouseState, StartingOrderChoice,
    TechCardId, TechEffect, TradingHouseState, TurnPhase, VisiblePlayer,
};

/// A Keep-shaped game: every seat's deal is recorded, so EVERY seat is
/// viewable (`observe` refuses a seat whose own deal is unrecorded), which is
/// what lets the sweep below observe all seats at every sampled decision point.
fn keep_game(players: u8, seed: u64, rules: &Rules) -> GameState {
    let pending = new_game(GameConfig::for_player_count(players), rules, seed).unwrap();
    let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
    finalize_setup(pending, &choices).unwrap()
}

// ---------------------------------------------------------------------------
// §2.1 — the source-text guard
// ---------------------------------------------------------------------------

/// Every `crate::game::` occurrence `eval.rs` is allowed to contain, verbatim
/// and trimmed. Widening this list is a deliberate act with grounds — which is
/// the whole point of asserting it (spec §2.1: the crate-internal module COULD
/// import anything, so the guard is executable, not a compiler fact).
///
/// **Entry 1, `Rules`** — the original permitted engine dependency: `score`
/// needs the order registry's size for §2.3's deck clock, and T2's mirrors need
/// the order/tech registries.
///
/// **Entry 2, the printed reward helpers — WIDENED AT V4 §3.2's Group S, and
/// the grounds are the spec's own.** §3.2 registers the engine paths its
/// candidate features may take (`crate::game::actions::*`, "private modules
/// re-exported"), and S1 / S5 / S6 are each defined AS an engine-priced reward:
/// S1 is "the engine's exported reward at realizable power", S5 is
/// `bank_reward`, S6 is `employment_agency_tokens_allowed` under a warehouse
/// minimum. Restating those tables inside `eval.rs` would put a second copy of
/// the rulebook's printed numbers in the crate — precisely what §13's
/// `exported_reward` dispatcher was built to prevent ("one job, one object").
///
/// **What the widening does NOT open.** All three imported functions are pure
/// `u8 -> u8` arithmetic over a POWER: none takes engine state, so none can
/// carry hidden information into a module whose whole contract is that it
/// cannot see any. The honesty boundary this guard defends is the STATE
/// boundary, and the forbidden-identifier scan below — which is whole-file and
/// untouched — is what holds it. This entry buys three reward tables, not a
/// state handle.
const GAME_IMPORT_ALLOWLIST: [&str; 2] = [
    "use crate::game::rules::Rules;",
    "use crate::game::actions::{bank_reward, employment_agency_tokens_allowed, exported_reward};",
];

/// Identifiers whose mere presence means the evaluator has stopped being a
/// function of a seat's view. `PendingSetup` is here because it carries the
/// pre-`finalize_setup` deals — the one setup-time object that would hand an
/// evaluator every seat's hidden starting order at once.
const FORBIDDEN_IDENTIFIERS: [&str; 2] = ["GameState", "PendingSetup"];

/// Spec §2.1's executable half. NOT a compiler fact and the doc says so: any
/// module inside `yokohama-core` can name any type in the crate, so the
/// signature guarantee (`score(&PlayerView, ..)`) is what a caller can rely on,
/// and THIS is what keeps the implementation from quietly reaching around it.
/// It is a TRIPWIRE against the ordinary spellings of a reach, not a proof —
/// the signature plus review discipline are the boundary; this makes the
/// casual violation loud.
///
/// The reach needles are scanned over the NON-TEST portion of the file only
/// (the in-module `mod tests` legitimately writes `use super::*;`), split at
/// the first `#[cfg(test)]` line, whose presence the test asserts so the split
/// cannot silently scan nothing. The forbidden IDENTIFIERS are whole-file.
///
/// MUTATION-PROVEN AT AUTHORING and re-proven at the T1-review hardening (each
/// arm reverted, observed messages quoted):
///   - `use crate::types::GameState;` — FAILED: `eval.rs:63 names the
///     forbidden identifier `GameState` … Line: use crate::types::GameState;`
///   - `use crate::game::scoring::calculate_remainder_bonus;` — FAILED:
///     reaches into `crate::game::` outside the allowlist.
///   - T1-review bypass (a): `let _sneaky = super::semantics::SEMANTICS_REV;`
///     inside a non-test fn — previously SILENT (the review demonstrated it
///     passing all six tests); now FAILED on the `super::` needle.
///   - T1-review bypass (b): `use crate::game as engine;` + an
///     `engine::semantics::…` read — previously SILENT; now FAILED on the
///     `crate::game` needle at the alias declaration.
#[test]
fn eval_source_text_names_no_engine_state_and_only_allowlisted_game_imports() {
    let src = include_str!("../src/game/eval.rs");

    // The reach scan stops where the test module starts; assert the marker
    // exists so a renamed/moved test module re-opens this decision instead of
    // quietly shrinking the scanned region to everything (fine) or the file
    // gaining code BELOW the marker (not fine — hence the single-marker pin).
    let marker = "#[cfg(test)]";
    let marker_count = src.matches(marker).count();
    assert_eq!(
        marker_count, 1,
        "eval.rs must contain exactly one `#[cfg(test)]` marker (found {marker_count}); \
         the guard scans reach needles only above it, so a second marker or code below \
         the test module would escape the scan — restructure deliberately, not incidentally"
    );
    let non_test = src.split(marker).next().unwrap();

    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        let trimmed = line.trim();
        for needle in FORBIDDEN_IDENTIFIERS {
            assert!(
                !line.contains(needle),
                "eval.rs:{n} names the forbidden identifier `{needle}` — the evaluator is a \
                 function of a PlayerView (spec §2.1). Line: {trimmed}"
            );
        }
    }

    // Reach needles, NON-TEST region. Substring over OCCURRENCES, not
    // `starts_with("use ")`: an inline path (`crate::game::scoring::foo(..)`)
    // is the same reach-around with no `use` line to catch. The set covers the
    // spellings the T1 review demonstrated as silent bypasses of the original
    // `crate::game::`-only needle:
    //   `super::`     — from `game/eval.rs`, `super::` IS `crate::game`;
    //   `crate::game` — direct paths AND `use crate::game as engine;` (no
    //                   trailing `::` required, unlike the original needle);
    //   `game as`     — the brace-grouped alias `use crate::{game as g}`;
    //   `game::`      — a bare-path read after `use crate::*;`.
    // Wording note for eval.rs editors: these are TEXT needles, so even a doc
    // comment containing one trips the scan — write around them (the guard bit
    // its own author's module doc at T1, by design).
    const REACH_NEEDLES: [&str; 4] = ["super::", "crate::game", "game as", "game::"];
    for (i, line) in non_test.lines().enumerate() {
        let n = i + 1;
        let trimmed = line.trim();
        for needle in REACH_NEEDLES {
            if line.contains(needle) {
                assert!(
                    GAME_IMPORT_ALLOWLIST.contains(&trimmed),
                    "`eval.rs:{n}` reaches toward the engine (needle `{needle}`) outside the \
                     allowlist {GAME_IMPORT_ALLOWLIST:?}. Line: {trimmed}"
                );
            }
        }
    }

    // Anti-vacuity: a scan that found nothing to scan proves nothing. The
    // allowlisted line must actually be present, so a future edit that drops
    // the `Rules` import (and with it the guard's only positive case) has to
    // update this list rather than silently emptying it.
    for allowed in GAME_IMPORT_ALLOWLIST {
        assert!(
            src.lines().any(|l| l.trim() == allowed),
            "the allowlist names `{allowed}`, which eval.rs no longer contains"
        );
    }
    assert!(src.contains("&PlayerView"), "eval.rs must still take views");
}

// ---------------------------------------------------------------------------
// §2.3 — the progress proxy's denominators, pinned against the engine
// ---------------------------------------------------------------------------

/// `eval.rs` MIRRORS two player-count functions (`GameConfig::track_trigger`
/// and `GameConfig::order_cards_to_remove`) because a `ViewConfig` exposes
/// neither and constructing a `GameConfig` to ask would both allocate (killing
/// §0.3(a)'s allocation-free path) and require the view to carry a config it
/// deliberately does not. This is the repo's standard answer to a mirror: pin
/// it (the `move_legality` / T2 drift-pin discipline).
#[test]
fn the_player_count_mirrors_match_the_engines_own_functions() {
    for players in [2u8, 3, 4] {
        let config = GameConfig::for_player_count(players);
        let view = observe(&keep_game(players, 5, &Rules::standard()), PlayerId(0)).unwrap();
        assert_eq!(
            track_trigger(view.config.player_count),
            config.track_trigger(),
            "{players}P church/customs trigger threshold"
        );
        assert_eq!(
            order_cards_to_remove(view.config.player_count),
            config.order_cards_to_remove(),
            "{players}P setup order-card removal"
        );
    }

    // The other two structural constants the deck clock and the building clock
    // are built on, pinned against the token/board supplies rather than
    // restated as prose: a port board's slot count IS the initial fill (see
    // `new_game`'s `refill_port`), and 8 shophouses / 4 trading houses are the
    // completion denominators `end_game.rs` triggers on.
    assert_eq!(PortBoard::new().slots.len(), 6, "port board slot count");
    let sh = ShophouseState::new();
    assert_eq!(sh.in_hand + sh.in_warehouse, 8, "shophouse supply == the building-race goal");
    assert_eq!(
        TradingHouseState::new().in_warehouse,
        4,
        "trading-house supply == the building-race goal"
    );
}

/// §2.3(a): the initial order-deck size is DERIVED, never tabulated — and the
/// derivation is pinned against what a fresh game actually deals out at every
/// player count. A fresh view therefore reads progress 0 on every clock, which
/// is the other half of the pin (a wrong denominator shows up as a nonzero
/// deck clock at turn 1).
#[test]
fn the_derived_initial_deck_size_matches_a_fresh_game_at_every_player_count() {
    let rules = Rules::standard();
    // 36 printed orders − setup removal − 2 dealt per seat − 6 per port board.
    for (players, expect) in [(2u8, 10usize), (3, 13), (4, 16)] {
        let state = keep_game(players, 9, &rules);
        let view = observe(&state, PlayerId(0)).unwrap();
        assert_eq!(view.order_deck_len, expect, "{players}P fresh deck");
        assert_eq!(
            initial_order_deck_len(&view, &rules),
            Some(expect),
            "{players}P derived initial deck size must equal the fresh deck"
        );
        assert_eq!(
            progress_ppm(&view, &rules),
            0,
            "{players}P: a fresh game has run none of the end-game clocks"
        );
        // The 2P dummy sits on faith-7 and space-4 from setup (finding D1) and
        // must NOT read as progress: the trigger counts real pawns only.
        assert_eq!(view.config.has_port_b, view.port_state.port_b.is_some());
    }
}

/// A view whose registry is empty (`Rules::empty_for_tests`) has no derivable
/// deck denominator. The clock is then UNAVAILABLE rather than wrong: total
/// over a degenerate input, and the remaining clocks still work.
#[test]
fn an_empty_order_registry_makes_the_deck_clock_unavailable_not_wrong() {
    let rules = Rules::standard();
    let view = observe(&keep_game(2, 9, &rules), PlayerId(0)).unwrap();
    let empty = Rules::empty_for_tests();
    assert_eq!(initial_order_deck_len(&view, &empty), None);
    assert_eq!(progress_ppm(&view, &empty), 0);
    // And a score is still produced (no panic, no divide by zero).
    let _ = score(&view, &empty, &WeightTable::V1);
}

// ---------------------------------------------------------------------------
// §0.3(a) — the two paths, and what makes the exact-sum property non-vacuous
// ---------------------------------------------------------------------------

/// §0.3(a)'s "score is the allocation-free hot path", made executable.
///
/// The property this defends is not performance for its own sake: it is that
/// `score` is NOT `explain().total`, which is the tautology the spec says the
/// exact-sum property would otherwise be dischargeable by. An `Explanation`
/// cannot be built without allocating its `Vec`, so `allocs == 0` on `score`
/// and `allocs > 0` on `explain` is a structural statement about the two paths.
///
/// Consequence for T2, stated here because that task will feel it: the
/// view-side score-structure mirrors must stay allocation-free too (fixed
/// arrays and counters, not `Vec`s). Relaxing this bound is allowed with
/// recorded grounds; silently letting it drift is not.
#[test]
fn score_allocates_nothing_and_explain_allocates() {
    let rules = Rules::standard();
    let weights = WeightTable::V1;
    // 4P: the widest player loop and the largest board — if any arm allocates,
    // it allocates here.
    let view = observe(&keep_game(4, 3, &rules), PlayerId(1)).unwrap();

    let (s, hot) = meter::measure(|| score(&view, &rules, &weights));
    assert_eq!(hot.allocs, 0, "score must not allocate (measured {} allocations)", hot.allocs);

    let (e, cold) = meter::measure(|| explain(&view, &rules, &weights));
    assert!(cold.allocs > 0, "explain builds a Vec; 0 allocations means it is not the cold path");
    assert_eq!(e.total, s, "the two paths must agree on the same view");

    // Bytes are RECORDED, never asserted — the meter's own contract (machine
    // and allocator dependent, unlike the deterministic event count).
    println!(
        "score: {} allocs / {} peak bytes | explain: {} allocs / {} peak bytes",
        hot.allocs, hot.peak_bytes, cold.allocs, cold.peak_bytes
    );
}

// ---------------------------------------------------------------------------
// The organic sweep: exact sum, orientation, stage bounds, monotonicity
// ---------------------------------------------------------------------------

/// What the sweep saw, so the assertions below can be floored against
/// MEASURED counts rather than hopes (the repo's anti-vacuity discipline: a
/// property that never met a nonzero case certifies nothing).
#[derive(Default, Debug)]
struct Tally {
    views: usize,
    /// Views by stage band, so the sweep proves it covered BOTH ends of the
    /// interpolation it is checking.
    early_zero: usize,
    late_half: usize,
    full: usize,
    /// Per-clock nonzero counts (deck / church / customs / building).
    clock_nonzero: [usize; 4],
    /// Terms whose product was nonzero, and terms whose selected opponent was
    /// NOT the raw maximum (i.e. a negative weight genuinely flipped the pick).
    nonzero_terms: usize,
    negative_weight_picks: usize,
    /// Decision points at which every seat's view agreed on the proxy.
    seat_agreements: usize,
    /// Views on which each feature's VIEWER value was nonzero, by
    /// `Feature::index()`. The five score-structure features are floored off
    /// this (T2 anti-vacuity): a mirror that is right about zero everywhere is
    /// not pinned to anything.
    nonzero_by_feature: [usize; Feature::COUNT],
    /// `(state, seat)` pairs whose five re-derivations were compared against
    /// the engine's own breakdown, and the pairs among those where the 2P dummy
    /// was actually holding a majority place.
    drift_pairs: usize,
    drift_dummy_pairs: usize,
    /// Terminal views scored through the §2.6 path.
    terminal_views: usize,
    /// The largest |positional score| the sweep produced — the measured input
    /// to §2.6's dominance claim (recorded AND asserted against the rank step).
    max_abs_positional: i64,
    /// (`seat`, card) legs the v2 post-claim filter actually DROPPED across the
    /// drift sweep — anti-vacuity for the re-derivation above. Without it the
    /// filtered and unfiltered expectations would agree everywhere and the
    /// re-derived clause would be pinning the pre-freeze arithmetic under a new
    /// name. Organic play must claim achievements for the filter to bite, and
    /// this counter is what says it did.
    postclaim_drops: usize,
    /// Views on which the PRE-FREEZE (unfiltered) achievement sum was nonzero —
    /// the same-run comparator for `nonzero_by_feature[13]`, so the freeze's own
    /// delta on this sweep is a measured difference rather than an inference.
    nonzero_prefreeze_achievement: usize,
}

/// One organic game: play it with the shipped weighted rung and check every
/// evaluator property at every sampled decision point, from EVERY seat.
fn sweep_game(
    players: u8,
    seed: u64,
    max_steps: Option<usize>,
    sample_every: usize,
    rules: &Rules,
    weights: &WeightTable,
    tally: &mut Tally,
) {
    let mut state = keep_game(players, seed, rules);
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x3F3F);
    let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
    let mut step = 0usize;
    let mut last_progress = 0u32;

    while !is_game_over(&state) {
        if max_steps.is_some_and(|cap| step >= cap) {
            break;
        }
        // The handle is scoped so its borrow of `state` ends before the state
        // is replaced below — the arena driver's shape, minus the recording.
        let mv = {
            let handle = DecisionHandle::new(&state, rules);
            policy
                .pick(&handle, &mut rng)
                .unwrap_or_else(|| panic!("dead end at {players}P seed {seed} step {step}"))
        };

        if step.is_multiple_of(sample_every) {
            let mut proxies = Vec::with_capacity(players as usize);
            for i in 0..players {
                let view = observe(&state, PlayerId(i)).unwrap();
                let p = check_view(&view, rules, weights, tally);
                proxies.push(p);
            }
            // §2.2's drift pin, riding the same traversal (one engine
            // `final_result` per decision point, every seat compared).
            check_drift(&state, rules, tally);
            // A running game must NOT take the §2.6 path — the terminal
            // predicate's negative side, checked as often as its positive one.
            let view = observe(&state, PlayerId(0)).unwrap();
            assert!(
                !view_is_terminal(&view),
                "the view called a running {players}P game finished at seed {seed} step {step}"
            );
            // The proxy is a function of PUBLIC state only, so every seat must
            // read the same clock. (A term that leaked a seat-relative
            // quantity into the proxy would show up here.)
            assert!(
                proxies.windows(2).all(|w| w[0] == w[1]),
                "the stage proxy is seat-relative at {players}P seed {seed} step {step}: {proxies:?}"
            );
            let p = proxies[0];
            // Monotone over SIM play: the deck only shrinks, tracks and
            // buildings only fill. (The assertion is SCOPED to self-play only
            // because live-tracking states are not swept here — not because the
            // deck can grow on the live path: `arrange_orders_on_top` is a
            // length-PRESERVING exchange, each boxed card swapping with a deck
            // donor, so `order_deck.len()` cannot rise there either. T1 review
            // corrected the earlier claim to the contrary.)
            assert!(
                p >= last_progress,
                "progress went backwards at {players}P seed {seed} step {step}: \
                 {last_progress} -> {p}"
            );
            last_progress = p;
            tally.seat_agreements += 1;
        }

        state = apply_move_unchecked(&state, &mv, rules).unwrap();
        step += 1;
    }

    // End of a COMPLETED game: whichever trigger fired drove its own clock to
    // full, so the proxy must read exactly full. This is the cross-check that
    // the mirrored denominators are the trigger's own (a church clock over 5
    // instead of 4 at 2P reads 0.8 here, not 1.0).
    if is_game_over(&state) {
        let view = observe(&state, PlayerId(0)).unwrap();
        assert_eq!(
            progress_ppm(&view, rules),
            PROGRESS_FULL_PPM,
            "a finished {players}P game (trigger {:?}) must read full progress",
            view.trigger_reason
        );
        tally.full += 1;
        // The §2.6 path, on the only states that can exercise it organically.
        check_drift(&state, rules, tally);
        check_terminal(&state, players, rules, weights, tally);
    }
}

/// §2.2's DRIFT PIN: every view-side re-derivation, for every seat, against the
/// engine's OWN per-player breakdown at the same state.
///
/// The comparison target is `final_result`, which is the engine's authority on
/// all five components at once — no new scoring surface had to be opened for
/// this pin, so `game/scoring.rs` is untouched by T2 and the differential gates
/// keep whatever they were already proving about it.
///
/// The pin runs at NON-terminal states too, deliberately: `final_result` is a
/// pure function of the position, the mirrors are read mid-game as `*_now`
/// features, and a mirror that only agreed at game end would be wrong exactly
/// where the evaluator actually reads it.
fn check_drift(state: &GameState, rules: &Rules, tally: &mut Tally) {
    let engine = final_result(state, rules);
    let dummy_on_track = state.church_track.slots.iter().flatten().any(|s| s.player.is_dummy())
        || state.customs_track.slots.iter().flatten().any(|s| s.player.is_dummy());

    for seat in 0..state.config.player_count {
        let id = PlayerId(seat);
        let view = observe(state, id).unwrap();
        let me = view.players.iter().find(|p| p.id == id).expect("viewer is seated");
        let structure = ScoreStructure::derive(&view, rules);
        assert_eq!(
            structure.seats,
            usize::from(state.config.player_count),
            "the structure must carry one row per seat"
        );
        let breakdown: &ScoreBreakdown = &engine
            .standings
            .iter()
            .find(|s| s.player == id)
            .expect("every seat has a standing")
            .breakdown;

        for (feature, expected, label) in [
            (Feature::CountryBonusNow, breakdown.country, "country"),
            (Feature::TechBonusNow, breakdown.technology, "technology"),
            (Feature::ChurchBonusNow, breakdown.church, "church"),
            (Feature::CustomsBonusNow, breakdown.customs, "customs"),
        ] {
            assert_eq!(
                structure.value_of(me, feature),
                i64::from(expected),
                "seat {seat}: view-side {label} bonus drifted from the engine's breakdown \
                 {breakdown:?}"
            );
            // `raw_value` is the probe-facing accessor and must agree with the
            // structure it is a convenience over.
            assert_eq!(
                raw_value(&view, me, feature, rules),
                structure.value_of(me, feature),
                "seat {seat}: raw_value disagreed with ScoreStructure for {label}"
            );
        }

        // The achievement mirror is pinned per CARD, `(current, needed)` and
        // all, not just through the summed feature — a sum can agree while two
        // legs are wrong in opposite directions.
        //
        // RE-DERIVED AT THE v2 FREEZE, and the shape of the change matters: the
        // PER-LEG half below is untouched (it is a mirror pin against the
        // engine's own progress arithmetic, which T3 did not touch), while the
        // SUM the feature reports is now the POST-CLAIM sum — so the expected
        // total skips the cards THIS seat has already claimed. Both totals are
        // accumulated so the pre-freeze one can still be asserted against
        // `prefreeze_achievement_sum`, which keeps the two arithmetics honest
        // against each other over organic play rather than only on a fixture.
        let mut expected_sum = 0i64;
        let mut expected_prefreeze = 0i64;
        for card in &view.achievements {
            let mine = view_achievement_progress(&view, me, card.achievement_type);
            let theirs = achievement_progress(state, id, card.achievement_type);
            assert_eq!(
                mine, theirs,
                "seat {seat}: {:?} progress drifted from the engine",
                card.achievement_type
            );
            let leg = i64::from(mine.0.min(mine.1));
            expected_prefreeze += leg;
            if !card.claimed_by.contains(&id) {
                expected_sum += leg;
            } else {
                tally.postclaim_drops += 1;
            }
        }
        assert_eq!(
            structure.value_of(me, Feature::AchievementProgressSum),
            expected_sum,
            "seat {seat}: the summed achievement feature is not the POST-CLAIM capped sum of \
             its legs (T3's filter, shipped at the v2 freeze)"
        );
        assert_eq!(
            prefreeze_achievement_sum(&view, me),
            expected_prefreeze,
            "seat {seat}: the pre-freeze sum is not the unfiltered capped sum of its legs"
        );

        // The full final total — the quantity §2.6 ranks — against the engine's
        // own. Every scored component is face-up, so this is an equality and
        // not an approximation (see `TerminalStanding`'s residual note).
        assert_eq!(
            structure.final_total(usize::from(seat)),
            i64::from(breakdown.total),
            "seat {seat}: the re-derived final total drifted: {breakdown:?}"
        );

        tally.drift_pairs += 1;
        if dummy_on_track {
            tally.drift_dummy_pairs += 1;
        }
    }
}

/// §2.6 on an organically-finished game: the terminal predicate, the ranking,
/// and the ladder's magnitude, each against the engine's own answer.
fn check_terminal(
    state: &GameState,
    players: u8,
    rules: &Rules,
    weights: &WeightTable,
    tally: &mut Tally,
) {
    let engine = final_result(state, rules);
    for seat in 0..players {
        let id = PlayerId(seat);
        let view = observe(state, id).unwrap();
        assert!(view_is_terminal(&view), "seat {seat}: a finished game must read terminal");

        let standing = terminal_standing(&view, rules).expect("terminal view has a standing");
        let engine_row =
            engine.standings.iter().find(|s| s.player == id).expect("every seat ranks");
        assert_eq!(
            standing.rank, engine_row.rank,
            "seat {seat}: view-side finishing place drifted from the engine's ranking"
        );
        assert_eq!(standing.seats, players, "every seat is ranked");
        assert_eq!(standing.viewer_total, i64::from(engine_row.breakdown.total));

        // The ladder: a winner scores positive, everyone else negative, and the
        // engine's own winner is the only seat that may read rank 1.
        let s = score(&view, rules, weights);
        assert_eq!(s, standing.value, "score must return the standing's value verbatim");
        let e = explain(&view, rules, weights);
        assert_eq!(e.total, s, "explain().total != score() on a terminal view");
        assert_eq!(e.terminal, Some(standing), "explain must carry the standing");
        assert!(
            e.terms.iter().all(|t| t.weight == 0 && t.product == 0),
            "a terminal explanation must report zero positional weights (§2.6)"
        );
        assert_eq!(
            e.terms.iter().map(|t| t.product).sum::<i64>() + standing.value,
            e.total,
            "the exact-sum identity must hold with the terminal contribution folded in"
        );
        if id == engine.winner {
            assert_eq!(standing.rank, 1);
            assert!(s > 0, "the winner's terminal score must be positive: {s}");
        } else {
            assert!(standing.rank > 1);
            assert!(s < 0, "a non-winner's terminal score must be negative: {s}");
        }
        // The ladder's arithmetic, spelled out, plus the property the clamp
        // exists for: the margin refinement can never cross a finishing place.
        let place = place_signal(standing.rank);
        let margin = (standing.viewer_total - standing.best_other_total)
            .clamp(-TERMINAL_MARGIN_CLAMP, TERMINAL_MARGIN_CLAMP);
        assert_eq!(s, TERMINAL_RANK_STEP * place + TERMINAL_MARGIN_STEP * margin);
        assert!(
            (s - TERMINAL_RANK_STEP * place).abs() * 2 < TERMINAL_RANK_STEP,
            "two margins must not span a rank step: {s} vs place {place}"
        );
        tally.terminal_views += 1;
    }
}

/// Every per-view property, returning the view's stage proxy.
fn check_view(
    view: &PlayerView,
    rules: &Rules,
    weights: &WeightTable,
    tally: &mut Tally,
) -> u32 {
    let p = progress_ppm(view, rules);
    assert!(p <= PROGRESS_FULL_PPM, "proxy out of range: {p}");

    let s = score(view, rules, weights);
    let e: Explanation = explain(view, rules, weights);

    // §0.3(a): the two paths agree EXACTLY, and the terms account for all of it.
    assert_eq!(e.total, s, "explain().total != score()");
    assert_eq!(
        e.terms.iter().map(|t| t.product).sum::<i64>(),
        e.total,
        "term products do not sum to the total"
    );
    assert_eq!(e.progress_ppm, p, "explain reported a different stage than progress_ppm");
    assert_eq!(e.terms.len(), Feature::COUNT, "one term per feature, always");

    let me = view.players.iter().find(|q| q.id == view.viewer).expect("viewer is seated");
    for (slot, term) in e.terms.iter().enumerate() {
        assert_eq!(term.feature, Feature::ALL[slot], "terms must be in canonical order");

        let (early, late) = weights.get(term.feature);
        assert_eq!(term.weight, interpolate(early, late, p), "term weight is not w(p)");
        assert_eq!(
            term.viewer_value,
            raw_value(view, me, term.feature, rules),
            "{}: viewer raw value",
            term.feature.name()
        );
        assert_eq!(
            term.product,
            term.weight * (term.viewer_value - term.opponent_value),
            "{}: product is not w(p)·(mine − theirs)",
            term.feature.name()
        );

        // §2.2's orientation rule: the opponent extreme is selected by the
        // SIGNED weight, so a negative weight picks the SMALLEST raw value.
        let opponents: Vec<&VisiblePlayer> =
            view.players.iter().filter(|q| q.id != view.viewer).collect();
        let raws: Vec<i64> =
            opponents.iter().map(|q| raw_value(view, q, term.feature, rules)).collect();
        if term.weight > 0 {
            assert_eq!(
                term.opponent_value,
                *raws.iter().max().unwrap(),
                "{}: a positive weight must take the opponents' maximum",
                term.feature.name()
            );
        } else if term.weight < 0 {
            assert_eq!(
                term.opponent_value,
                *raws.iter().min().unwrap(),
                "{}: a negative weight must take the opponents' minimum",
                term.feature.name()
            );
            if raws.iter().max() != raws.iter().min() {
                tally.negative_weight_picks += 1;
            }
        }
        if term.product != 0 {
            tally.nonzero_terms += 1;
        }
        // T2 anti-vacuity: every feature — the five re-derivations especially —
        // must actually be nonzero somewhere in the sweep, or its arm is
        // certified by nothing. Floored in the test below off measured counts.
        if term.viewer_value != 0 {
            tally.nonzero_by_feature[term.feature.index()] += 1;
        }
        // SAME-RUN ATTRIBUTION for the v2 freeze's one feature change: the
        // shipped (filtered) count rides `nonzero_by_feature` above, and this is
        // the count the PRE-FREEZE arithmetic would have produced on the
        // identical trajectories. Their difference is the T3 filter's whole
        // footprint on this sweep, measured rather than reasoned about — which
        // is what lets this test's re-recorded doc separate the freeze's own
        // delta from the rules-rev drift the trajectories already carried.
        if term.feature == Feature::AchievementProgressSum
            && prefreeze_achievement_sum(view, me) != 0
        {
            tally.nonzero_prefreeze_achievement += 1;
        }
    }

    tally.max_abs_positional = tally.max_abs_positional.max(s.abs());
    tally.views += 1;
    if p == 0 {
        tally.early_zero += 1;
    }
    if p >= PROGRESS_FULL_PPM / 2 {
        tally.late_half += 1;
    }
    for (i, c) in clock_ppm(view, rules).iter().enumerate() {
        if *c > 0 {
            tally.clock_nonzero[i] += 1;
        }
    }
    p
}

/// THE per-edit property (§0.3(a)) over ORGANICALLY SAMPLED views: three full
/// games, one per player count, under the shipped
/// `RolloutPolicy::FAMILY_WEIGHTED_DEFAULT`, observed from every seat.
///
/// Full games rather than a step cap ON PURPOSE: the interpolation is the thing
/// under test and a capped game never leaves the early band (a 4P game runs
/// ~1,700 moves), so a capped sweep would check `w(0)` a thousand times and
/// call it coverage.
///
/// SINCE T2 this sweep also carries §2.2's DRIFT PIN and §2.6's terminal path
/// — see [`check_drift`] and [`check_terminal`], which ride the same traversal
/// rather than paying for a second one.
///
/// MEASURED at the T2 close, RE-MEASURED at T3's in-hand split, and RE-RECORDED
/// at the Phase-3.5c v2 freeze (2026-08-10), with the floors set at roughly HALF
/// each measured count (the sweep is fully deterministic — fixed seeds, seeded
/// ChaCha8 — so the figures are exact, and the halving is headroom for benign
/// drift rather than protection against flake). The figures AT HEAD:
///
/// ```text
/// views 1598 · early_zero 132 · late_half 853 · full 3
/// clocks [deck 1415, church 1353, customs 195, building 1153]
/// nonzero_terms 14267 · negative_weight_picks 1903 · decision points 507
/// nonzero_by_feature
///   [vp 1165, ctry 780, tech 665, chur 667, cust 105, hCORE 1240, hEXC 363,
///    ware 1470, brd 1557, shop 746, trad 77, 5pw 327, imp 395, achv 1475,
///    merc 503, hnd# 1458]
/// drift_pairs 1607 · drift_dummy_pairs 270 · terminal_views 9
/// max |positional score| 2675  (rank step 1e9 clears it 373,831x — the
///   test's own integer division; 1e9/2675 = 373,831.7)
/// postclaim_drops 238 · nonzero_prefreeze_achievement 1475
/// ```
///
/// WHAT MOVED AND WHY, since a re-measure that does not account for its own
/// deltas is just a new number — and here the honest answer has TWO parts,
/// only one of which belongs to the freeze.
///
/// **1. Most of it is RULES-REV DRIFT that was already in the tree.** The T3
/// block above was recorded at `SEMANTICS_REV` 5. The sweep plays under
/// `FAMILY_WEIGHTED_DEFAULT`, which never consults the evaluator, so no
/// evaluator change can move a trajectory — but the five 2026-08-08 parity
/// rulings (revs 6 and 7) changed reachable states, and every trajectory count
/// moved with them: views 1,711 → 1,598, `late_half` 1,134 → 853, the customs
/// clock 390 → 195, `drift_pairs` 1,720 → 1,607. Those deltas predate this
/// commit; the halved floors absorbed them silently, which is the floors working
/// as designed and also the reason a re-record was overdue.
///
/// **2. The FREEZE's own footprint is measured, not inferred, and it is
/// small.** T3's post-claim filter dropped **238** (seat × card) legs across the
/// drift sweep (`postclaim_drops`, the anti-vacuity counter for the re-derived
/// achievement clause — a filter that dropped nothing would leave that clause
/// pinning the pre-freeze arithmetic under a new name). Its effect on the
/// per-feature anti-vacuity count is **zero**: `nonzero_by_feature[achv]` and
/// the same-run pre-freeze comparator `nonzero_prefreeze_achievement` BOTH read
/// 1,475, because a seat that has claimed one card still holds live progress on
/// another. The two figures this run cannot separate are `nonzero_terms` and
/// `max_abs_positional`, which carry the rev drift and the filter together; they
/// are recorded as measured and nothing is claimed about their split. NOTE that
/// this sweep still evaluates with `WeightTable::V1` on purpose — its subject is
/// the assembly (`explain` == `score`), not the shipped pricing, and holding the
/// table fixed is what makes even this much attribution possible.
///
/// T1 → T2's earlier delta stands recorded too: the five score-structure
/// features began computing real values, which changed every score.
///
/// MUTATION-PROVEN AT AUTHORING (all reverted; the point is that this sweep
/// catches the slips it exists for, not merely that `assert!` works):
///   - `score` skipping one feature (`if feature == Feature::HandSize
///     { continue; }`) — FAILED: `assertion left == right failed:
///     explain().total != score(); left: 68, right: 62`. That is the §0.3(a)
///     exact-sum property doing its job across two assemblies.
///   - `opponent_extreme` comparing RAW values instead of `weight · value`
///     (i.e. dropping the orientation rule) — FAILED: `hand_size: a negative
///     weight must take the opponents' minimum; left: 2, right: 1`.
///   - T2: the church majority awarding its places to the ranking WITHOUT the
///     dummy contender — see the constructed dummy test below for the recorded
///     failure; the organic half of the pin catches it too.
#[test]
fn explain_reconstructs_score_exactly_over_organic_views() {
    let rules = Rules::standard();
    let weights = WeightTable::V1;
    let mut tally = Tally::default();

    for (players, seed) in [(2u8, 0u64), (3, 1), (4, 2)] {
        sweep_game(players, seed, None, 8, &rules, &weights, &mut tally);
    }
    // One capped extra game per count, off a different seed: cheap variety in
    // the early band, where most of a game's decisions live.
    for (players, seed) in [(2u8, 11u64), (3, 12), (4, 13)] {
        sweep_game(players, seed, Some(120), 4, &rules, &weights, &mut tally);
    }

    println!("eval organic sweep: {tally:?}");
    assert!(tally.views >= 850, "too few views to call this organic: {tally:?}");
    assert!(tally.early_zero >= 60, "no early-band coverage: {tally:?}");
    assert!(tally.late_half >= 550, "no late-band coverage: {tally:?}");
    assert_eq!(tally.full, 3, "all three uncapped games must have finished: {tally:?}");
    assert!(tally.nonzero_terms >= 4_000, "terms are mostly zero — vacuous: {tally:?}");
    assert!(
        tally.negative_weight_picks >= 850,
        "the negative-weight orientation arm never bit (spec §2.2 is unexercised): {tally:?}"
    );
    // Each of the four clocks must have run somewhere in the sweep, or the
    // maximum defining the proxy is a maximum over fewer terms than it claims.
    for (i, name) in ["deck", "church", "customs", "building"].iter().enumerate() {
        assert!(tally.clock_nonzero[i] >= 150, "the {name} clock barely advanced: {tally:?}");
    }

    // T2 — ANTI-VACUITY, per feature. Floors measured first, then set at half
    // (the same discipline as the counts above). The five score-structure
    // features are the point: a mirror that reads zero on every swept view is
    // pinned to nothing, however green the drift comparison looks.
    for feature in Feature::ALL {
        let seen = tally.nonzero_by_feature[feature.index()];
        let floor = FEATURE_NONZERO_FLOORS[feature.index()];
        assert!(
            seen >= floor,
            "{} was nonzero on only {seen} of {} views (floor {floor}) — its arm is \
             effectively untested: {tally:?}",
            feature.name(),
            tally.views
        );
    }

    // T2 — the drift pin ran, and ran on states where the 2P dummy was
    // genuinely competing (the D1 arm has a constructed test of its own, but a
    // pin whose organic half never met a dummy would be half a pin).
    assert!(tally.drift_pairs >= 850, "the drift pin barely ran: {tally:?}");
    assert!(tally.drift_dummy_pairs >= 120, "no organic 2P-dummy coverage: {tally:?}");
    // v2 FREEZE anti-vacuity: the drift pin's achievement clause now expects the
    // POST-CLAIM sum, and that expectation is only a different statement from
    // the pre-freeze one where organic play actually claimed something. Measured
    // 238, floored at half in this file's standing discipline.
    assert!(
        tally.postclaim_drops >= 119,
        "T3's filter dropped almost no legs, so the re-derived achievement clause is pinning \
         the PRE-FREEZE arithmetic under a new name: {tally:?}"
    );
    assert_eq!(
        tally.terminal_views,
        2 + 3 + 4,
        "every seat of all three finished games must have been scored terminally: {tally:?}"
    );

    // §2.6's DOMINANCE, as a measurement rather than a claim: the largest
    // positional score anywhere in the sweep, against the rank step a terminal
    // place is worth. The bound a future weight table must keep: a positional
    // total under TERMINAL_RANK_STEP/2 (5e8) is what makes a win outscore every
    // running position. The provisional table clears it by ~4 orders.
    println!(
        "max |positional score| {} vs rank step {TERMINAL_RANK_STEP} \
         (headroom x{})",
        tally.max_abs_positional,
        TERMINAL_RANK_STEP / tally.max_abs_positional.max(1)
    );
    assert!(tally.max_abs_positional > 0, "no positional score was nonzero: {tally:?}");
    assert!(
        tally.max_abs_positional < TERMINAL_RANK_STEP / 2,
        "a positional score reached {} — at or past half a rank step, so a terminal \
         win no longer dominates every running position (§2.6)",
        tally.max_abs_positional
    );
}

/// Per-feature nonzero floors for the sweep above, by `Feature::index()`.
/// MEASURED-THEN-SET at the T2 close and RE-MEASURED at the T3 in-hand split
/// (the observed counts are in that test's doc block), each at half the observed
/// count rounded down — the same discipline every other floor in this file uses.
/// The two smallest arms (`trading_houses_built` 61 and `five_power_tokens` 110)
/// are genuinely rare events, not weak coverage: four trading houses END the
/// game.
///
/// The split's two floors were measured the same way and are NOT symmetric, by
/// construction: `core` is nonzero whenever a seat holds any assistant at all
/// (1,269 views — the old unsplit feature's count exactly, which is the check
/// that the split moved nothing else), while `excess` needs a hand above
/// `HAND_CORE_RESERVE` and fires on 267. The excess arm being ~5x rarer is the
/// measurement, not a weakness: a hand over the reserve is what a placement
/// spends, and a seat holding one is the position the split exists to price.
///
/// `five_power_tokens` RE-MEASURED at `SEMANTICS_REV` 6 (2026-08-08, the
/// rules-parity ruling set — reachable-state behaviour changes, so the swept
/// trajectories legitimately moved): observed 54 on the rev-6 sweep against
/// the old floor of 55 (from T3's observed 110), re-halved to 27 by the same
/// measure-then-halve discipline. Still a genuinely rare event, same reason
/// as before. Every other arm stayed above its standing floor at rev 6, so
/// only this entry moved.
//     [vp,  ctry, tech, chur, cust, hCORE, hEXC, ware, brd, shop, trad, 5pw, imp, achv, merc, hnd#]
const FEATURE_NONZERO_FLOORS: [usize; Feature::COUNT] =
    [689, 489, 375, 387, 84, 634, 133, 855, 835, 431, 30, 27, 120, 809, 317, 823];

// ---------------------------------------------------------------------------
// §2.2 — the D1 dummy, constructed
// ---------------------------------------------------------------------------

/// A finalized 2P game with both real players' church and customs pawns
/// cleared, leaving only the setup dummy (Church faith-7 / Customs space-4).
fn dummy_fixture(rules: &Rules) -> GameState {
    let mut state = keep_game(2, 7, rules);
    assert!(
        state.church_track.slots.iter().flatten().any(|s| s.player.is_dummy()),
        "the 2P setup must place a dummy — finding D1"
    );
    state
        .church_track
        .slots
        .iter_mut()
        .for_each(|slot| *slot = slot.filter(|s| s.player.is_dummy()));
    state
        .customs_track
        .slots
        .iter_mut()
        .for_each(|slot| *slot = slot.filter(|s| s.player.is_dummy()));
    state
}

/// FINDING D1, view-side: the 2P dummy is a real competitor in both majorities
/// — it can TAKE a place (awarded to nobody) and it can WIN a count tie on the
/// rightmost space — and the evaluator's mirrors must price it exactly as the
/// engine does, in both directions.
///
/// The two scenarios are the engine's own D1 reproducers (`game/scoring.rs`'s
/// `church_majority_2p_dummy_takes_first_place` and
/// `customs_majority_2p_dummy_wins_tie_via_rightmost`) read back through the
/// view, plus their controls where the dummy is beaten and changes nothing.
/// That contrast is the anti-vacuity: the same fixture family produces 3 VP
/// with the dummy competing and 6 VP without, so a mirror that ignored the
/// dummy would be caught by a value, not by an absence.
///
/// MUTATION-PROVEN AT AUTHORING (reverted): dropping the dummy from
/// `majority_bonus`'s tally (skipping `is_dummy()` owners outright, the obvious
/// "the dummy scores nothing so why count it" simplification) — FAILED here on
/// `denied: view-side church bonus drifted from the engine; left: 3, right: 0`
/// (P0 wrongly promoted into the place the dummy holds), and independently in
/// the organic sweep on `seat 1: view-side church bonus drifted from the
/// engine's breakdown …; left: 6, right: 3`. Both halves of the pin bite it,
/// which is the point of having both.
#[test]
fn the_2p_dummy_competes_in_the_view_side_majorities_exactly_as_the_engine_does() {
    let rules = Rules::standard();
    let church7 = ChurchTrack::faith_to_index(7).unwrap();
    let customs4 = CustomsTrack::space_to_index(4).unwrap();

    // (label, church placements, customs placements, what the dummy does)
    let mut denied = dummy_fixture(&rules);
    // Both real pawns LEFT of faith-7: the dummy is rightmost at count 1, takes
    // 1st (nobody scores it), and P1 gets only 2nd.
    denied.church_track.slots[ChurchTrack::faith_to_index(4).unwrap()] =
        Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 1 });
    denied.church_track.slots[ChurchTrack::faith_to_index(5).unwrap()] =
        Some(ChurchSlot { player: PlayerId::new(1), arrival_order: 2 });
    // Customs: P0 on space-3, dummy on space-4 — a COUNT TIE the dummy wins on
    // rightmost, dropping P0 from 8 VP to 4.
    denied.customs_track.slots[CustomsTrack::space_to_index(3).unwrap()] =
        Some(CustomsSlot { player: PlayerId::new(0), arrival_order: 1 });
    denied.customs_track.slots[CustomsTrack::space_to_index(1).unwrap()] =
        Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 2 });

    // The control: both real players out-COUNT the lone dummy, so it places
    // third and changes nothing — 6/3 and 8/4 go to real players.
    let mut beaten = dummy_fixture(&rules);
    for (faith, player) in [(4u8, 0u8), (8, 0), (5, 1), (9, 1)] {
        beaten.church_track.slots[ChurchTrack::faith_to_index(faith).unwrap()] =
            Some(ChurchSlot { player: PlayerId::new(player), arrival_order: faith });
    }
    for (space, player) in [(5u8, 0u8), (6, 0), (1, 1), (2, 1)] {
        beaten.customs_track.slots[CustomsTrack::space_to_index(space).unwrap()] =
            Some(CustomsSlot { player: PlayerId::new(player), arrival_order: space });
    }

    for (label, state) in [("denied", &denied), ("beaten", &beaten)] {
        assert!(
            state.church_track.slots[church7].is_some_and(|s| s.player.is_dummy())
                && state.customs_track.slots[customs4].is_some_and(|s| s.player.is_dummy()),
            "{label}: the fixture must still hold the dummy on its printed spaces"
        );
        let engine = final_result(state, &rules);
        for seat in 0..2u8 {
            let id = PlayerId(seat);
            let view = observe(state, id).unwrap();
            let me = view.players.iter().find(|p| p.id == id).unwrap();
            let structure = ScoreStructure::derive(&view, &rules);
            let breakdown = &engine
                .standings
                .iter()
                .find(|s| s.player == id)
                .unwrap()
                .breakdown;
            assert_eq!(
                structure.value_of(me, Feature::ChurchBonusNow),
                i64::from(breakdown.church),
                "{label}: view-side church bonus drifted from the engine"
            );
            assert_eq!(
                structure.value_of(me, Feature::CustomsBonusNow),
                i64::from(breakdown.customs),
                "{label}: view-side customs bonus drifted from the engine"
            );
        }
    }

    // ANTI-VACUITY: the two fixtures must actually DISAGREE about the same
    // seat's bonus, or the pair proves only that both sides compute the same
    // constant. The denied case is the D1 effect itself: P1 collects 3 (2nd
    // behind the dummy) where the control collects 6 (1st outright), and P0
    // collects 4 in customs where the control collects 8.
    let value = |state: &GameState, seat: u8, feature: Feature| {
        let view = observe(state, PlayerId(seat)).unwrap();
        let me = view.players.iter().find(|p| p.id == PlayerId(seat)).unwrap().clone();
        ScoreStructure::derive(&view, &rules).value_of(&me, feature)
    };
    assert_eq!(value(&denied, 1, Feature::ChurchBonusNow), 3, "P1 is 2nd behind the dummy");
    assert_eq!(value(&beaten, 1, Feature::ChurchBonusNow), 6, "P1 is 1st with the dummy beaten");
    assert_eq!(value(&denied, 0, Feature::ChurchBonusNow), 0, "P0 is beaten by both");
    assert_eq!(value(&denied, 0, Feature::CustomsBonusNow), 4, "the dummy wins the rightmost tie");
    assert_eq!(value(&beaten, 0, Feature::CustomsBonusNow), 8, "P0 out-counts the dummy");
}

// ---------------------------------------------------------------------------
// §2.6 — the terminal term, constructed
// ---------------------------------------------------------------------------

/// Force `state` into the shape the engine calls finished: end-game triggered,
/// final round, and the turn resting at the end of the last seat's turn.
fn force_terminal(state: &mut GameState) {
    let n = state.config.player_count;
    state.end_game_triggered = true;
    state.final_round = true;
    state.trigger_reason = Some(yokohama_core::EndGameTrigger::ChurchTrackFull);
    state.trigger_player = Some(state.config.start_player);
    state.current_player = PlayerId((state.config.start_player.0 + n - 1) % n);
    state.turn_phase = TurnPhase::TurnEnd;
    assert!(is_game_over(state), "the fixture must actually be a finished game");
}

/// §2.6's WHOLE POINT, constructed so that running VP and the true final
/// standing DISAGREE: P0 leads on victory points and LOSES the game, because
/// P1's five completed orders are a full country set worth 12 unbanked VP.
///
/// A terminal term that scored "VP totals visible in-view" — the reading both
/// spec review lenses rejected — would rank P0 first here. The bonus-inclusive
/// re-derivation ranks P1 first, and the engine agrees.
///
/// MUTATION-PROVEN AT AUTHORING (reverted): ranking on `base_vp` alone instead
/// of `final_total` — i.e. exactly the "VP totals visible in-view" reading —
/// FAILED here on `20 running VP + a 12-VP country set; left: 20, right: 32`
/// and, independently, in the organic sweep's terminal check on
/// `standing.viewer_total vs the engine's total; left: 46, right: 87`.
#[test]
fn a_terminal_view_is_ranked_by_the_full_final_score_not_by_running_vp() {
    let rules = Rules::standard();
    let mut state = dummy_fixture(&rules);
    force_terminal(&mut state);

    for p in state.players.iter_mut() {
        // Zero every remainder leg so the fixture's arithmetic is exactly the
        // two numbers this test is about.
        p.money = 0;
        p.copper = 0;
        p.silk = 0;
        p.tea = 0;
        p.seafood = 0;
        p.imported_goods = 0;
        p.completed_orders.clear();
        p.technologies.clear();
    }
    state.players[0].victory_points = 30;
    state.players[1].victory_points = 20;
    // O01 American, O02 French, O03 English, O04 German, O06 Dutch: five
    // distinct countries = one full set = 12 VP, which is more than the 10 VP
    // P0 leads by.
    state.players[1].completed_orders = [1u8, 2, 3, 4, 6].map(OrderCardId).to_vec();

    let engine = final_result(&state, &rules);
    assert_eq!(engine.winner, PlayerId(1), "the fixture must make the VP leader lose");

    let loser = observe(&state, PlayerId(0)).unwrap();
    let winner = observe(&state, PlayerId(1)).unwrap();
    assert!(view_is_terminal(&loser) && view_is_terminal(&winner));

    let loser_standing = terminal_standing(&loser, &rules).unwrap();
    let winner_standing = terminal_standing(&winner, &rules).unwrap();
    assert_eq!(loser_standing.viewer_total, 30);
    assert_eq!(winner_standing.viewer_total, 32, "20 running VP + a 12-VP country set");
    assert_eq!(winner_standing.rank, 1, "the bonus winner takes first");
    assert!(
        loser_standing.rank > winner_standing.rank,
        "the loser must not outrank the winner: left: {}, right: {}",
        loser_standing.rank,
        winner_standing.rank
    );

    // The ladder at 2P: dominant and margin-refined. (At 2P, and ONLY at 2P,
    // the two place values happen to be mirror images (+1/−1); at 3–4P the
    // ladder is deliberately ASYMMETRIC — every non-first place is negative —
    // and eval.rs documents that a symmetric place ladder would be a bug. The
    // T2 review caught this comment calling the ladder "symmetric" as if that
    // were the design; the VALUE assertions below are what pin the design.)
    let weights = WeightTable::V1;
    let win = score(&winner, &rules, &weights);
    let loss = score(&loser, &rules, &weights);
    assert_eq!(win, TERMINAL_RANK_STEP + 2 * TERMINAL_MARGIN_STEP, "2P first place, margin +2");
    assert_eq!(loss, -TERMINAL_RANK_STEP - 2 * TERMINAL_MARGIN_STEP, "2P second place, margin −2");
    assert!(win > 0 && loss < 0, "first positive, last negative (2P mirror is incidental)");

    // A hostile margin cannot lift second above first: the clamp is what makes
    // that structural rather than a fact about typical scores.
    let mut blowout = state.clone();
    blowout.players[1].victory_points = 100_000;
    let blown = observe(&blowout, PlayerId(1)).unwrap();
    let blown_standing = terminal_standing(&blown, &rules).unwrap();
    assert_eq!(blown_standing.rank, 1);
    assert_eq!(
        score(&blown, &rules, &weights),
        TERMINAL_RANK_STEP + TERMINAL_MARGIN_STEP * TERMINAL_MARGIN_CLAMP,
        "the margin saturates at the clamp"
    );
    let blown_loser = observe(&blowout, PlayerId(0)).unwrap();
    assert!(
        score(&blown_loser, &rules, &weights) < score(&blown, &rules, &weights),
        "no margin may invert the finishing order"
    );
}

/// The terminal PREDICATE's four conjuncts, each falsified on its own. The
/// organic sweep pins the positive side (three finished games) and the negative
/// side (every sampled running position); this pins the boundary — which is
/// where a mirror of someone else's predicate actually drifts.
#[test]
fn the_view_side_terminal_predicate_mirrors_the_engine_conjunct_by_conjunct() {
    let rules = Rules::standard();
    let mut state = keep_game(3, 21, &rules);
    force_terminal(&mut state);
    let seat = PlayerId(0);
    assert!(view_is_terminal(&observe(&state, seat).unwrap()), "the fixture is terminal");

    for (label, mutate) in [
        ("trigger cleared", (|s: &mut GameState| s.end_game_triggered = false) as fn(&mut GameState)),
        ("final round cleared", |s: &mut GameState| s.final_round = false),
        ("trigger player cleared", |s: &mut GameState| s.trigger_player = None),
        ("phase rewound", |s: &mut GameState| s.turn_phase = TurnPhase::Recovery),
        ("wrong seat on turn", |s: &mut GameState| s.current_player = s.config.start_player),
    ] {
        let mut broken = state.clone();
        mutate(&mut broken);
        let view = observe(&broken, seat).unwrap();
        assert_eq!(
            view_is_terminal(&view),
            is_game_over(&broken),
            "{label}: the view-side predicate disagreed with the engine"
        );
        assert!(!view_is_terminal(&view), "{label} must not read as a finished game");
        // And a non-terminal view scores positionally, with no standing.
        assert_eq!(terminal_standing(&view, &rules), None);
    }
}

/// The achievement mirror across ALL TWELVE card types and both cap boundaries.
///
/// WHY THIS EXISTS, recorded because it is the anti-vacuity lesson of this
/// task: only three achievement cards are in play per game, so the organic
/// sweep meets at most a handful of types and essentially never meets a leg
/// ABOVE its own cap. Mutating `Prod3Comm2`'s commercial leg from `.min(2)` to
/// uncapped SURVIVED the whole rest of this file — the sweep is broad, not
/// deep, and a broad pin is not a substitute for the boundary. This cell walks
/// every type against the engine's own progress arithmetic and puts both
/// Category-C legs strictly over their caps.
///
/// MUTATION-PROVEN AT AUTHORING (each reverted):
///   - `Prod3Comm2`'s commercial leg uncapped — FAILED: `both over cap:
///     Prod3Comm2 progress drifted from the engine; left: (6, 5),
///     right: (5, 5)`. This is the mutation that survived every OTHER test in
///     this file, which is why the cell exists.
///   - `Tokens5` counting only face-up mercenaries (`unused_count()` in place
///     of `total()` — the ruling-A2 arm, where face-DOWN mercenaries count
///     toward "own 5 tokens") — FAILED: `one leg short: Tokens5 progress
///     drifted from the engine; left: (5, 5), right: (6, 5)`.
#[test]
fn the_view_side_achievement_progress_mirrors_the_engine_at_every_card_and_cap() {
    let rules = Rules::standard();
    let base = keep_game(4, 31, &rules);

    let production: Vec<_> = base
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.is_production())
        .map(|(a, _)| *a)
        .collect();
    let commercial: Vec<_> = base
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.is_commercial())
        .map(|(a, _)| *a)
        .collect();
    assert!(
        production.len() >= 5 && commercial.len() >= 3,
        "the 4P board must offer enough areas to push both Category-C legs over \
         their caps: {} production, {} commercial",
        production.len(),
        commercial.len()
    );

    // (label, production areas held, commercial areas held). The first cell is
    // the engine's own worked counterexample — a RAW total (6) above `needed`
    // (5) that must still not read as met, because the commercial leg (1) is
    // under its own threshold (2). The second puts BOTH legs over their caps,
    // which is what catches an uncapped commercial leg.
    let mut distinct = ALL_ACHIEVEMENTS.to_vec();
    distinct.sort_by_key(|a| format!("{a:?}"));
    distinct.dedup();
    assert_eq!(distinct.len(), 12, "the card list must name twelve DISTINCT types");

    for (label, prod_n, comm_n) in [("one leg short", 5usize, 1usize), ("both over cap", 5, 3)] {
        let mut state = base.clone();
        let p = &mut state.players[0];
        p.trading_houses.built = production[..prod_n].to_vec();
        for area in &commercial[..comm_n] {
            p.shophouses.built.insert(*area, 0);
        }
        // Category A and B, each pushed strictly PAST its threshold so the
        // `min(current, needed)` cap in the summed feature is load-bearing too.
        p.copper = 9;
        p.silk = 9;
        p.tea = 9;
        p.seafood = 9;
        p.money = 25;
        p.five_power_tokens = 4;
        p.mercenaries.available.insert(yokohama_core::Country::English, 1);
        p.mercenaries.used.insert(yokohama_core::Country::French, 1);
        p.completed_orders = [1u8, 2, 3, 4, 6, 8].map(OrderCardId).to_vec();
        p.technologies = (1..=5u8).map(yokohama_core::TechCardId).collect();

        for card in ALL_ACHIEVEMENTS {
            state.achievements = [
                yokohama_core::AchievementCard::new(card),
                yokohama_core::AchievementCard::new(card),
                yokohama_core::AchievementCard::new(card),
            ];
            let view = observe(&state, PlayerId(0)).unwrap();
            let me = view.players.iter().find(|q| q.id == PlayerId(0)).unwrap();

            let mine = view_achievement_progress(&view, me, card);
            let theirs = achievement_progress(&state, PlayerId(0), card);
            assert_eq!(mine, theirs, "{label}: {card:?} progress drifted from the engine");

            // The summed feature is the CAPPED sum, three identical cards over.
            assert_eq!(
                ScoreStructure::derive(&view, &rules)
                    .value_of(me, Feature::AchievementProgressSum),
                3 * i64::from(mine.0.min(mine.1)),
                "{label}: {card:?} — the feature is not the capped sum of its legs"
            );
        }

        // The boundary itself, on the engine's own numbers.
        let view = observe(&state, PlayerId(0)).unwrap();
        let me = view.players.iter().find(|q| q.id == PlayerId(0)).unwrap();
        let (current, needed) =
            view_achievement_progress(&view, me, yokohama_core::AchievementType::Prod3Comm2);
        assert_eq!(needed, 5, "{label}: 3 production cap + 2 commercial cap");
        if comm_n == 1 {
            assert_eq!(current, 4, "{label}: min(5,3) + min(1,2), NOT the raw 6");
            assert!(current < needed, "{label}: the short leg must keep the claim unmet");
        } else {
            assert_eq!(current, 5, "{label}: min(5,3) + min(3,2), NOT the raw 8");
        }
    }
}

/// Every achievement type, so the mirror above cannot quietly stop covering
/// one. The engine's own enum is matched exhaustively on both sides, so a new
/// variant is a compile error there; this list is the TEST's half and the
/// duplicate check in the cell is what stops it degenerating into twelve copies
/// of one card.
const ALL_ACHIEVEMENTS: [yokohama_core::AchievementType; 12] = {
    use yokohama_core::AchievementType::*;
    [
        Copper5,
        Silk6,
        Tea7,
        Seafood7,
        Money10,
        Tokens5,
        Orders5,
        Technologies4,
        Production4,
        Commercial4,
        Prod3Comm2,
        Prod2Comm3,
    ]
};

// ---------------------------------------------------------------------------
// T2-review fix wave (orchestrator): three pins the review showed were missing
// ---------------------------------------------------------------------------

/// The clamp inequality that makes the terminal ladder's ORDER structural is
/// pinned by NOTHING but this test (T2 review: sweeping
/// `TERMINAL_MARGIN_CLAMP` 400 → 600/1000/10000 left the whole suite green
/// while at 600 rank 3's best case, −2e9 + 6e8, genuinely outscored rank 2's
/// worst, −1e9 − 6e8). Two pins, both pure arithmetic over the shipped
/// constants:
///   1. the global inequality `2·CLAMP·MARGIN_STEP < RANK_STEP`;
///   2. the property it exists to buy, stated directly — at every supported
///      player count, the WORST value a place can take strictly exceeds the
///      BEST value of the place below it.
///
/// At clamp 600 both fail; at the shipped 400 both hold with 2e8 headroom.
#[test]
fn the_terminal_ladder_cannot_invert_adjacent_places_by_construction() {
    // The global inequality is a compile-time fact about three consts, so it
    // is pinned as one (the repo's L12 preference — the strongest pins are the
    // ones the compiler carries): clamp 600 makes this a COMPILE error here.
    const {
        assert!(2 * TERMINAL_MARGIN_CLAMP * TERMINAL_MARGIN_STEP < TERMINAL_RANK_STEP);
    }
    for players in 2u8..=4 {
        for rank in 1..players {
            let worst_here = TERMINAL_RANK_STEP * place_signal(rank)
                - TERMINAL_MARGIN_STEP * TERMINAL_MARGIN_CLAMP;
            let best_below = TERMINAL_RANK_STEP * place_signal(rank + 1)
                + TERMINAL_MARGIN_STEP * TERMINAL_MARGIN_CLAMP;
            assert!(
                worst_here > best_below,
                "{players}P: rank {rank}'s worst case ({worst_here}) must strictly exceed rank \
                 {}'s best case ({best_below}) — the clamp is what buys this",
                rank + 1,
            );
        }
    }
}

/// An area holding BOTH a shophouse and a trading house for the same player
/// counts ONCE in the building-area achievement legs — the engine has a
/// dedicated unit test for exactly this shape
/// (`test_count_production_buildings_does_not_double_count_area_with_both`),
/// and the T2 constructed cell made the overlap structurally unreachable by
/// segregating building types by area class, so the de-duplication arm in
/// `building_areas` had no test teeth (T2 review: mutating the
/// `!contains_key` guard to always-true SURVIVED the whole suite; against
/// this cell it fails).
#[test]
fn an_area_holding_both_building_types_counts_once_in_view_side_progress() {
    let rules = Rules::standard();
    let mut state = keep_game(4, 31, &rules);

    let production: Vec<_> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.is_production())
        .map(|(a, _)| *a)
        .collect();
    assert!(production.len() >= 3, "need three production areas for the overlap fixture");

    // Player 0 holds a trading house AND a shophouse in production[0], plus a
    // trading house alone in production[1] and a shophouse alone in
    // production[2]: three DISTINCT built areas, four buildings. A
    // double-counting mirror reads four areas.
    let p = &mut state.players[0];
    p.trading_houses.built = production[..2].to_vec();
    p.shophouses.built.insert(production[0], 0);
    p.shophouses.built.insert(production[2], 1);

    for card in ALL_ACHIEVEMENTS {
        state.achievements = [
            yokohama_core::AchievementCard::new(card),
            yokohama_core::AchievementCard::new(card),
            yokohama_core::AchievementCard::new(card),
        ];
        let view = observe(&state, PlayerId(0)).unwrap();
        let me = view.players.iter().find(|q| q.id == PlayerId(0)).unwrap();
        assert_eq!(
            view_achievement_progress(&view, me, card),
            achievement_progress(&state, PlayerId(0), card),
            "overlap area double-counted: {card:?} drifted from the engine",
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 3.5c — T3's post-claim filter (protocol §4.1(ii) / §5 T3)
//
// It landed ADDITIVELY in round 1 (nothing called it, so these tests were the
// only thing holding its arithmetic) and the FREEZE COMMIT switched
// `Feature::AchievementProgressSum`'s call site onto it. These tests moved with
// the switch: `shipped_sum` below reads through the FEATURE, so it now reports
// the filter, and the double-count fact they were written to demonstrate is
// re-pointed onto `prefreeze_achievement_sum` — the pre-filter arithmetic, which
// eval.rs still exports for exactly this purpose.
// ---------------------------------------------------------------------------

/// The viewer's row of a view, by seat.
fn seat_of(view: &PlayerView, seat: PlayerId) -> &VisiblePlayer {
    view.players.iter().find(|p| p.id == seat).expect("seat is in the view")
}

/// The SHIPPED sum, read where the evaluator reads it — through the FEATURE
/// rather than through either underlying function, so these tests compare
/// against what actually prices a position today. Since the freeze that is the
/// post-claim filter, and asserting so is one of this file's jobs.
fn shipped_sum(view: &PlayerView, rules: &Rules, seat: PlayerId) -> i64 {
    ScoreStructure::derive(view, rules)
        .value_of(seat_of(view, seat), Feature::AchievementProgressSum)
}

/// The PRE-FREEZE sum — the same capped-leg arithmetic with no claim filter.
/// It is what `Feature::AchievementProgressSum` read until 2026-08-10, and it is
/// kept reachable so the double count T3 removed can still be DEMONSTRATED
/// rather than only described.
fn prefreeze_sum(view: &PlayerView, seat: PlayerId) -> i64 {
    prefreeze_achievement_sum(view, seat_of(view, seat))
}

/// One card's capped leg for one seat — the quantity the post-claim filter
/// removes, computed from the mirror rather than hand-typed so a cap change
/// cannot leave these tests asserting a stale constant.
fn capped_leg(view: &PlayerView, seat: PlayerId, card: yokohama_core::AchievementType) -> i64 {
    let (current, needed) = view_achievement_progress(view, seat_of(view, seat), card);
    i64::from(current.min(needed))
}

/// A state where BOTH seats under test carry real, UNEQUAL progress toward the
/// cards in play, so a post-claim exclusion has something to remove on either
/// side and the surviving legs have something to keep. Seat 0 is over the copper
/// cap and under the money one; seat 1 is the reverse.
///
/// FOUR seats, on the achievement mirror's own board and for its reason: only
/// the 4P board carries the five production plus one commercial area the
/// AND-shaped counterexample below needs (the 2P board cannot hold five
/// production areas at all). The filter under test is per-seat, so the seat
/// count is a fixture detail rather than part of the property.
fn postclaim_fixture(rules: &Rules, card: yokohama_core::AchievementType) -> GameState {
    let mut state = keep_game(4, 31, rules);
    state.players[0].copper = 9;
    state.players[0].money = 7;
    state.players[1].copper = 2;
    state.players[1].money = 25;

    let production: Vec<_> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.is_production())
        .map(|(a, _)| *a)
        .collect();
    let commercial: Vec<_> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.is_commercial())
        .map(|(a, _)| *a)
        .collect();
    assert!(
        production.len() >= 5 && !commercial.is_empty(),
        "the 4P board must offer 5 production and 1 commercial area for the AND-shaped leg",
    );
    // Seat 0: five production areas and ONE commercial — the AND-shaped
    // counterexample shape (raw 6, capped 3 + 1 = 4, `needed` 5, claim unmet).
    state.players[0].trading_houses.built = production[..5].to_vec();
    state.players[0].shophouses.built.insert(commercial[0], 0);

    state.achievements = [
        yokohama_core::AchievementCard::new(card),
        yokohama_core::AchievementCard::new(card),
        yokohama_core::AchievementCard::new(card),
    ];
    state
}

/// With nothing claimed the candidate IS the shipped computation — over every
/// card type and both seats, so "reproduces it exactly" is a statement about
/// the arithmetic and not about one lucky fixture.
#[test]
fn the_postclaim_sum_reproduces_the_shipped_sum_while_nothing_is_claimed() {
    let rules = Rules::standard();
    for card in ALL_ACHIEVEMENTS {
        let state = postclaim_fixture(&rules, card);
        for seat in [PlayerId(0), PlayerId(1)] {
            let view = observe(&state, seat).unwrap();
            assert!(
                view.achievements.iter().all(|c| c.claimed_by.is_empty()),
                "the fixture must start unclaimed",
            );
            assert_eq!(
                achievement_progress_postclaim_sum(&view, seat_of(&view, seat)),
                shipped_sum(&view, &rules, seat),
                "{card:?} seat {seat:?}: the unclaimed candidate must equal the shipped sum",
            );
        }
    }
}

/// The one thing the candidate changes: a card THE SEAT BEING VALUED has
/// already claimed contributes 0, and exactly that card's capped leg leaves the
/// sum. The shipped sum is asserted NOT to move in the same breath — that
/// standing difference is the double-count protocol §5 T3 names.
#[test]
fn the_postclaim_sum_drops_a_card_the_viewer_has_already_claimed() {
    let rules = Rules::standard();
    let card = yokohama_core::AchievementType::Copper5;
    let mut state = postclaim_fixture(&rules, card);

    let view = observe(&state, PlayerId(0)).unwrap();
    let before = achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0)));
    let leg = capped_leg(&view, PlayerId(0), card);
    let prefreeze_before = prefreeze_sum(&view, PlayerId(0));
    assert!(leg > 0, "the fixture's copper leg must carry weight for the drop to be visible");
    assert_eq!(before, 3 * leg, "three identical cards, one leg each");

    // ONE of the three cards is claimed by the viewer.
    state.achievements[0].claimed_by.push(PlayerId(0));
    let view = observe(&state, PlayerId(0)).unwrap();
    assert_eq!(
        achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0))),
        before - leg,
        "exactly the claimed card's capped leg leaves the filtered sum",
    );
    // THE DOUBLE COUNT, still demonstrated rather than merely described — on the
    // PRE-FREEZE arithmetic, which is where it lives now.
    assert_eq!(
        prefreeze_sum(&view, PlayerId(0)),
        prefreeze_before,
        "the PRE-FREEZE sum kept paying for a claimed card — the double-count T3 names",
    );
    // …and the SHIPPED feature no longer does. This is the call-site switch
    // itself, pinned at the one place the evaluator reads the value.
    assert_eq!(
        shipped_sum(&view, &rules, PlayerId(0)),
        before - leg,
        "since the freeze `Feature::AchievementProgressSum` IS the filtered sum",
    );
    assert_ne!(
        shipped_sum(&view, &rules, PlayerId(0)),
        prefreeze_before,
        "anti-vacuity: the two arithmetics must genuinely differ on this fixture",
    );

    // …and claiming all three empties it.
    state.achievements[1].claimed_by.push(PlayerId(0));
    state.achievements[2].claimed_by.push(PlayerId(0));
    let view = observe(&state, PlayerId(0)).unwrap();
    assert_eq!(achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0))), 0);
}

/// **The scope boundary, and the rules fact under it.** An OPPONENT's claim
/// leaves the viewer's leg fully live, because the rules let a second seat claim
/// the same card for the printed subsequent VP — asserted here against the
/// engine's own claim gate, not against a reading of the rulebook.
#[test]
fn an_opponent_claim_leaves_the_viewers_leg_fully_live() {
    let rules = Rules::standard();
    let card = yokohama_core::AchievementType::Copper5;
    let mut state = postclaim_fixture(&rules, card);

    let view = observe(&state, PlayerId(0)).unwrap();
    let viewer_before = achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0)));
    let opponent_before = achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(1)));
    let opponent_leg = capped_leg(&view, PlayerId(1), card);
    assert!(opponent_leg > 0, "seat 1 must hold copper for its own drop to be visible");

    state.achievements[0].claimed_by.push(PlayerId(1));
    let view = observe(&state, PlayerId(0)).unwrap();
    assert_eq!(
        achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0))),
        viewer_before,
        "an opponent's claim must not touch the VIEWER's sum",
    );
    assert_eq!(
        achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(1))),
        opponent_before - opponent_leg,
        "the exclusion is per-seat: it fires for the claimant and for nobody else",
    );

    // The rules fact the scope rests on: seat 0 can still claim the card seat 1
    // took, so its leg is a live claim rather than dead weight.
    assert!(
        state.players[0].assistants.in_hand >= 1,
        "the claim gate needs an assistant in hand; the fixture must supply one",
    );
    assert!(
        yokohama_core::can_claim_achievement(&state, PlayerId(0), card),
        "an opponent-claimed card stays claimable — which is why the filter is per-seat",
    );
}

/// The AND-shaped cards' per-leg caps survive the filter: on a card the viewer
/// has NOT claimed the candidate reports the same capped arithmetic the shipped
/// mirror does (raw 6 → capped 4, never the raw sum), and a claim removes
/// exactly that capped value.
#[test]
fn the_postclaim_sum_preserves_the_and_shaped_cards_cap_arithmetic() {
    let rules = Rules::standard();
    let card = yokohama_core::AchievementType::Prod3Comm2;
    let mut state = postclaim_fixture(&rules, card);

    let view = observe(&state, PlayerId(0)).unwrap();
    let (current, needed) = view_achievement_progress(&view, seat_of(&view, PlayerId(0)), card);
    assert_eq!((current, needed), (4, 5), "min(5,3) + min(1,2) = 4, NOT the raw 6");
    assert_eq!(
        achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0))),
        3 * 4,
        "three unclaimed AND-shaped cards, each capped at 4",
    );

    state.achievements[2].claimed_by.push(PlayerId(0));
    let view = observe(&state, PlayerId(0)).unwrap();
    assert_eq!(
        achievement_progress_postclaim_sum(&view, seat_of(&view, PlayerId(0))),
        2 * 4,
        "the claimed card leaves at its CAPPED value, not at its raw one",
    );
}

/// §0.3(a)'s allocation bound holds on the TERMINAL branch too (T2 review
/// note: the shipped pin measured a fresh view only, while a search evaluates
/// terminal successors at its leaves — the branch with the extra standing
/// work is exactly the one that was unmeasured).
#[test]
fn score_allocates_nothing_on_the_terminal_branch_too() {
    let rules = Rules::standard();
    let weights = WeightTable::V1;
    let mut state = dummy_fixture(&rules);
    force_terminal(&mut state);
    let view = observe(&state, PlayerId(0)).unwrap();
    assert!(view_is_terminal(&view), "the fixture must read terminal from the view");

    let (s, hot) = meter::measure(|| score(&view, &rules, &weights));
    assert_eq!(
        hot.allocs, 0,
        "terminal score must not allocate (measured {} allocations)",
        hot.allocs
    );
    // The value itself is rank-dominated — sanity, not the subject.
    assert!(s.abs() >= TERMINAL_RANK_STEP - TERMINAL_MARGIN_STEP * TERMINAL_MARGIN_CLAMP);
    println!("terminal score: {} allocs / {} peak bytes", hot.allocs, hot.peak_bytes);
}

// ---------------------------------------------------------------------------
// V4 §3.2 — `realizable_power`, the ARRIVING-POWER oracle
// ---------------------------------------------------------------------------

/// The president-relocated READ-ONLY clone the §7 pin compares against: this
/// same position, with `seat`'s president standing at `area`.
///
/// **Its only consumer is `calculate_power`**, which reads the president
/// location, the assistant counts, the two building maps and the station token
/// and nothing else. Nothing is applied to this clone, replayed from it, scored
/// on it or handed to an agent — so it does not have to be a position the rules
/// could reach (two presidents may land on one area here). It has to be the
/// position whose power `realizable_power` CLAIMS to report, which is the
/// definition of the oracle this pins against.
fn president_relocated(state: &GameState, seat: PlayerId, area: AreaId) -> GameState {
    let mut moved = state.clone();
    moved.players[seat.0 as usize].president_location = PresidentLocation::OnBoard(area);
    moved
}

/// V4 spec §3.2's shared arithmetic, pinned as §7's audit contract part 4 asks:
/// `realizable_power(seat, a)` is the power the seat would ARRIVE at `a` with,
/// i.e. the engine's own full-state power function evaluated on this position
/// with that seat's president CONSTRUCTED AT `a`.
///
/// The sweep is over ORGANIC play at 2P and 3P, every seat, every area, so the
/// two readings meet at president-present areas (where the engine already adds
/// the term), at president-elsewhere areas (where the view-side mirror adds it
/// and the engine does not), and at the president-in-hand states that open
/// every game.
#[test]
fn realizable_power_is_the_arriving_power_oracle_over_organic_play() {
    let rules = Rules::standard();
    let mut checked = 0usize;
    let mut with_president_here = 0usize;
    for (players, seed) in [(2u8, 41u64), (3, 43)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x5A5A);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..120usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(7) {
                for i in 0..players {
                    let seat = PlayerId(i);
                    let view = observe(&state, seat).unwrap();
                    let player =
                        view.players.iter().find(|p| p.id == seat).expect("the viewer has a row");
                    for area in view.board_layout.areas.keys().copied() {
                        let oracle =
                            calculate_power(&president_relocated(&state, seat, area), seat, area);
                        assert_eq!(
                            realizable_power(&view, player, area),
                            oracle,
                            "{players}P seed {seed} step {step}: seat {i} at {area:?}"
                        );
                        checked += 1;
                        if player.president_location == PresidentLocation::OnBoard(area) {
                            with_president_here += 1;
                        }
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    // Anti-vacuity in BOTH directions: the sweep must have met areas where the
    // president actually stands (the branch on which the two readings agree for
    // the engine's own reason) and areas where it does not.
    assert!(checked > 500, "the sweep only made {checked} comparisons");
    assert!(with_president_here > 0, "no sampled area ever held the viewer's president");
    assert!(with_president_here < checked, "every sampled area held the viewer's president");
    println!("realizable_power oracle: {checked} comparisons, {with_president_here} president-here");
}

/// The one clause that makes this NOT a copy of the engine's power function:
/// the president term is UNCONDITIONAL (spec §3.2, "with the president term
/// unconditional"). Two positions differing ONLY in where this seat's president
/// stands must read the same realizable power at the queried area — that is the
/// whole content of "arriving".
///
/// The declared consequence rides here too: the president's CURRENT area is
/// barred as next turn's destination by the no-return-to-start rule and STILL
/// receives the +1 (§3.2's declared shape, positive and known).
#[test]
fn realizable_power_keeps_the_president_term_wherever_the_president_stands() {
    let rules = Rules::standard();
    let state = keep_game(2, 45, &rules);
    let seat = PlayerId(0);
    let areas: Vec<AreaId> = observe(&state, seat).unwrap().board_layout.areas.keys().copied().collect();
    let (here, elsewhere) = (areas[0], areas[1]);

    let at_here = observe(&president_relocated(&state, seat, here), seat).unwrap();
    let at_elsewhere = observe(&president_relocated(&state, seat, elsewhere), seat).unwrap();
    let row = |view: &PlayerView| view.players.iter().find(|p| p.id == seat).unwrap().clone();

    assert_eq!(
        realizable_power(&at_here, &row(&at_here), here),
        realizable_power(&at_elsewhere, &row(&at_elsewhere), here),
        "the arriving reading must not move with the president"
    );
    // ... and the divergence from the engine's own reading is real, so this is
    // not a pin that would pass on a plain mirror.
    assert_ne!(
        calculate_power(&president_relocated(&state, seat, here), seat, here),
        calculate_power(&president_relocated(&state, seat, elsewhere), seat, here),
        "the engine's conditional term must differ across the pair, or the cell is vacuous"
    );
}

/// The cap is applied LAST, over the summed terms (spec §3.2, "cap LAST";
/// the engine's own ordering).
///
/// The fixture moves five of the seat's eight hand assistants onto one area —
/// `in_hand` 8 → 3, `on_board[area]` 0 → 5, so the seat's twenty-pawn
/// conservation identity is untouched — and the sixth term is the president
/// the arriving reading always adds.
#[test]
fn realizable_power_caps_last_at_five() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 47, &rules);
    let seat = PlayerId(0);
    let area = *observe(&state, seat).unwrap().board_layout.areas.keys().next().unwrap();

    let pawns = &mut state.players[0].assistants;
    assert_eq!(pawns.in_hand, 8, "the setup deals eight assistants to hand");
    assert_eq!(pawns.on_board.get(&area).copied().unwrap_or(0), 0);
    pawns.in_hand -= 5;
    pawns.on_board.insert(area, 5);

    let view = observe(&state, seat).unwrap();
    let player = view.players.iter().find(|p| p.id == seat).unwrap();
    assert_eq!(
        realizable_power(&view, player, area),
        5,
        "five assistants plus the arriving president is six, capped to five"
    );
}

/// **THE PER-TERM CELL, and it exists because the organic sweep does not cover
/// its own subject.** Of the 1116 comparisons
/// `realizable_power_is_the_arriving_power_oracle_over_organic_play` makes, the
/// trading-house term is live in ZERO and the station term in ZERO (the
/// shophouse term in six): organic 2P/3P play under the weighted rung rarely
/// builds and never places the Station token in the sampled window. A field
/// typo in either term — a shophouse map consulted where a trading-house set
/// was meant, say — would pass every other cell in this file. So the terms get
/// a CONSTRUCTED position that carries all five power sources at one area.
///
/// Every stock is MOVED, never invented: the seat's twenty assistants, eight
/// shophouses and four trading houses are asserted equal before and after, so
/// the fixture is a position the engine could hold rather than a convenient
/// pile of fields.
///
/// The reading is pinned against the engine at every variant, on exactly the
/// organic cell's oracle — `calculate_power` on the president-relocated clone.
#[test]
fn realizable_power_counts_the_shophouse_the_trading_house_and_the_station() {
    let rules = Rules::standard();
    let base = keep_game(2, 49, &rules);
    let seat = PlayerId(0);
    let area = *observe(&base, seat).unwrap().board_layout.areas.keys().next().unwrap();

    // What the fixture claims to start from, asserted rather than assumed — a
    // setup that already carried one of these would make the deltas below
    // measure something other than the term they name.
    assert_eq!(base.players[0].assistants.on_board.get(&area).copied().unwrap_or(0), 0);
    assert!(!base.players[0].shophouses.built.contains_key(&area));
    assert!(!base.players[0].trading_houses.built.contains(&area));
    assert_eq!(base.station_location, None);
    assert_ne!(
        base.players[0].president_location,
        PresidentLocation::OnBoard(area),
        "the president must be ELSEWHERE, so the term this reading adds is the unconditional one"
    );

    let mut full = base.clone();
    {
        let p = &mut full.players[0];
        p.assistants.in_hand -= 1;
        p.assistants.on_board.insert(area, 1);
        p.shophouses.in_warehouse -= 1;
        p.shophouses.built.insert(area, 0);
        p.trading_houses.in_warehouse -= 1;
        p.trading_houses.built.push(area);
    }
    full.station_location = Some(area);

    // The constructibility mandate, made executable: tokens moved, never
    // invented (20 assistants / 8 shophouses / 4 trading houses).
    let stock = |s: &GameState| {
        let p = &s.players[0];
        let assistants = u32::from(p.assistants.in_hand)
            + u32::from(p.assistants.in_warehouse)
            + p.assistants.on_board.values().map(|n| u32::from(*n)).sum::<u32>()
            + u32::from(p.assistants.on_achievements)
            + u32::from(p.assistants.on_church)
            + u32::from(p.assistants.on_customs);
        let shophouses = u32::from(p.shophouses.in_hand)
            + u32::from(p.shophouses.in_warehouse)
            + p.shophouses.built.len() as u32;
        let trading = u32::from(p.trading_houses.in_hand)
            + u32::from(p.trading_houses.in_warehouse)
            + p.trading_houses.built.len() as u32;
        (assistants, shophouses, trading)
    };
    assert_eq!(stock(&full), stock(&base), "the fixture moved tokens; it must not invent them");
    assert_eq!(stock(&full), (20, 8, 4), "the printed per-seat token supplies");

    let read = |state: &GameState| {
        let view = observe(state, seat).unwrap();
        let player = view.players.iter().find(|p| p.id == seat).expect("the viewer has a row");
        realizable_power(&view, player, area)
    };
    let oracle =
        |state: &GameState| calculate_power(&president_relocated(state, seat, area), seat, area);

    // 1 president + 1 assistant + 1 shophouse + 1 trading house + 1 station = 5,
    // which is the cap exactly, so no term is hidden behind it.
    assert_eq!(read(&full), 5, "all five power sources at one area");
    assert_eq!(read(&full), oracle(&full));

    // PER-TERM: drop one source at a time; the reading must fall by exactly
    // one, and must still agree with the engine. This is what the two dead
    // terms have instead of organic coverage.
    let mut without: Vec<(&str, GameState)> = Vec::new();

    let mut no_assistant = full.clone();
    no_assistant.players[0].assistants.on_board.remove(&area);
    no_assistant.players[0].assistants.in_hand += 1;
    without.push(("assistant", no_assistant));

    let mut no_shophouse = full.clone();
    no_shophouse.players[0].shophouses.built.remove(&area);
    no_shophouse.players[0].shophouses.in_warehouse += 1;
    without.push(("shophouse", no_shophouse));

    let mut no_trading_house = full.clone();
    no_trading_house.players[0].trading_houses.built.retain(|a| *a != area);
    no_trading_house.players[0].trading_houses.in_warehouse += 1;
    without.push(("trading house", no_trading_house));

    let mut no_station = full.clone();
    no_station.station_location = None;
    without.push(("station", no_station));

    for (term, state) in &without {
        assert_eq!(stock(state), (20, 8, 4), "the {term} variant lost a token");
        assert_eq!(read(state), 4, "dropping the {term} did not cost exactly one power");
        assert_eq!(read(state), oracle(state), "the {term} variant disagrees with the engine");
    }
    // Names what it exercised, so a future reader can see the coverage claim is
    // about four separate terms rather than one sum.
    println!(
        "realizable_power per-term: {} terms each worth exactly 1 at the cap boundary",
        without.len()
    );
}

// ---------------------------------------------------------------------------
// V4 §3.2 — the view context the candidate features ride
// ---------------------------------------------------------------------------

/// §3.2's allocation contract names the shape the ten view-context features
/// must ride: "counters / fixed arrays / bitsets" inside
/// [`ScoreStructure::derive`], "indexed by `AreaId` (0..18 at 4P)". This cell
/// pins the BOARD half of that context against the layout it is derived from,
/// class by class, at every printed player count.
///
/// The per-count numbers are the rulebook's own removal rules
/// (`board_gen::tiles_for_player_count`): 10 / 14 / 18 tiles, with 2P losing
/// Port B, Lab B, Chinatown, the CANAL and one duplicate of each of the four
/// production types, and 3P losing Lab B and three production duplicates.
/// **The 2P row is the one worth reading twice: a 2P board carries no Canal
/// tile at all**, so every Canal clause in S3/S4/S7 is structurally inert on
/// exactly the boards V4's fixture package is built from — which is why the
/// 3P and 4P rows are here rather than a 2P-only cell.
#[test]
fn view_context_masks_every_area_class_at_every_player_count() {
    let rules = Rules::standard();
    for (players, seed, tiles, production, has_canal) in
        [(2u8, 61u64, 10u32, 4u32, false), (3, 63, 14, 5, true), (4, 65, 18, 8, true)]
    {
        let view = observe(&keep_game(players, seed, &rules), PlayerId(0)).unwrap();
        let c = ScoreStructure::derive(&view, &rules).context;

        assert_eq!(c.present.count_ones(), tiles, "{players}P tile count");
        assert_eq!(
            c.placeable.count_ones(),
            if has_canal { tiles - 1 } else { tiles },
            "{players}P placeable areas are the board minus the Canal"
        );
        assert_eq!(c.production.count_ones(), production, "{players}P production tiles");
        assert_eq!(c.tracks.count_ones(), 2, "{players}P Church + Customs");
        assert_eq!(c.bank.count_ones(), 1, "{players}P Bank");
        assert_eq!(c.employment_agency.count_ones(), 1, "{players}P Employment Agency");

        // Every mask is a subset of the board the layout carries — a bit set
        // outside `present` would be a phantom area every consumer inherits.
        for (name, mask) in [
            ("placeable", c.placeable),
            ("production", c.production),
            ("tracks", c.tracks),
            ("bank", c.bank),
            ("employment_agency", c.employment_agency),
        ] {
            assert_eq!(mask & !c.present, 0, "{players}P: {name} names an area off the board");
        }

        // ... and area by area against the layout itself, so the counts above
        // cannot be right for the wrong reason.
        for (area, config) in &view.board_layout.areas {
            let bit = area_bit(*area);
            assert_ne!(c.present & bit, 0, "{area:?} is on the board");
            assert_eq!(
                c.placeable & bit != 0,
                config.area_type.allows_placement(),
                "{players}P {area:?} placeable"
            );
            assert_eq!(
                c.production & bit != 0,
                config.area_type.is_production(),
                "{players}P {area:?} production"
            );
            assert_eq!(
                c.tracks & bit != 0,
                matches!(config.area_type, AreaType::Church | AreaType::Customs),
                "{players}P {area:?} track"
            );
            assert_eq!(
                c.bank & bit != 0,
                config.area_type == AreaType::Bank,
                "{players}P {area:?} bank"
            );
            assert_eq!(
                c.employment_agency & bit != 0,
                config.area_type == AreaType::EmploymentAgency,
                "{players}P {area:?} employment agency"
            );
        }
    }
}

/// FREE adjacency is `adjacency` MINUS `canal_crossings` (spec §3.2's S4 card,
/// and S7's step relation) — a REAL subtraction, not a defensive one: the board
/// generator adds each tolled crossing pair to BOTH `adjacency` and
/// `canal_crossings` (`board_gen::derive_canal_crossings`'s doc: "The caller
/// adds each returned pair to both"), so a free-edge reading that skipped the
/// subtraction would walk S7's president across the Canal for free.
///
/// ANTI-VACUITY IN BOTH DIRECTIONS, because the milestone's own fixture package
/// is 2P: the sweep asserts a 2P board tolls NOTHING (it has no Canal tile) and
/// floors the total number of tolled crossings the 3P/4P boards contributed, so
/// this cell cannot pass by never meeting a crossing.
#[test]
fn view_context_free_adjacency_is_adjacency_minus_the_tolled_canal_crossings() {
    let rules = Rules::standard();
    let mut tolled = 0usize;
    for (players, seed) in [(2u8, 67u64), (3, 69), (4, 71)] {
        let view = observe(&keep_game(players, seed, &rules), PlayerId(0)).unwrap();
        let c = ScoreStructure::derive(&view, &rules).context;

        for (area, neighbours) in &view.board_layout.adjacency {
            let mut expected: u32 = 0;
            for n in neighbours {
                if !view.board_layout.is_canal_crossing(*area, *n) {
                    expected |= area_bit(*n);
                }
            }
            assert_eq!(
                c.free_adjacent(*area),
                expected,
                "{players}P {area:?}: free adjacency must drop exactly the tolled pairs"
            );
        }

        if players == 2 {
            assert!(
                view.board_layout.canal_crossings.is_empty(),
                "a 2P board carries no Canal tile, so it can toll nothing"
            );
        }
        tolled += view.board_layout.canal_crossings.len();
    }
    assert!(tolled > 0, "no sampled board tolled a crossing — the subtraction was never exercised");
    println!("free adjacency: {tolled} tolled crossings dropped across the sampled boards");
}

/// The SEAT half of the context: which areas hold each seat's assistants, and
/// where each seat's president stands. Both are read by S3/S4/S7 for the
/// VIEWER and — the registered cross-seat read (§5.2 family 5(a)) — for every
/// OPPONENT, whose president bars S4's re-entry targets and S7's counted set.
///
/// Swept over organic play so the two branches of `president_location` both
/// occur, with floors on each: a fixture that only ever met in-hand presidents
/// would certify neither.
#[test]
fn view_context_records_every_seats_presence_and_president() {
    let rules = Rules::standard();
    let (mut occupied_seen, mut on_board_seen, mut in_hand_seen) = (0usize, 0usize, 0usize);
    for (players, seed) in [(2u8, 73u64), (3, 75)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x2C2C);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..90usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(6) {
                let view = observe(&state, PlayerId(0)).unwrap();
                let c = ScoreStructure::derive(&view, &rules).context;
                for player in &view.players {
                    let seat = usize::from(player.id.0);
                    for area in view.board_layout.areas.keys().copied() {
                        let held = player.assistants.in_area(area) >= 1;
                        assert_eq!(
                            c.occupied[seat] & area_bit(area) != 0,
                            held,
                            "{players}P seed {seed} step {step}: seat {seat} presence at {area:?}"
                        );
                        if held {
                            occupied_seen += 1;
                        }
                    }
                    match player.president_location {
                        PresidentLocation::OnBoard(a) => {
                            assert_eq!(c.president[seat], area_bit(a), "seat {seat} president");
                            on_board_seen += 1;
                        }
                        PresidentLocation::InHand => {
                            assert_eq!(c.president[seat], 0, "seat {seat} president is in hand");
                            in_hand_seen += 1;
                        }
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(occupied_seen > 20, "only {occupied_seen} occupied areas met");
    assert!(on_board_seen > 0, "no sampled seat ever had its president on the board");
    assert!(in_hand_seen > 0, "no sampled seat ever had its president in hand");
    println!(
        "seat context: {occupied_seen} presences, {on_board_seen} on-board / {in_hand_seen} in-hand presidents"
    );
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Group S: shared fixture machinery
// ---------------------------------------------------------------------------

/// Read one Group-S helper off a state, through the whole honest chain the
/// production caller uses: observe the seat, derive the structure once, hand
/// the helper that seat's own row.
///
/// The closure shape is what keeps the borrow of `view` inside one scope while
/// still letting each cell name the helper it means.
fn read_s<T>(
    state: &GameState,
    rules: &Rules,
    seat: PlayerId,
    f: impl Fn(&ScoreStructure, &PlayerView, &VisiblePlayer) -> T,
) -> T {
    let view = observe(state, seat).expect("the fixture seat is observable");
    let structure = ScoreStructure::derive(&view, rules);
    let player = view.players.iter().find(|p| p.id == seat).expect("the viewer has a row");
    f(&structure, &view, player)
}

/// The single area of `wanted` type. Every printed board carries at most one
/// Bank / Employment Agency / Church / Customs House, so "the" is exact for the
/// types these cells ask for.
fn area_typed(state: &GameState, wanted: AreaType) -> AreaId {
    state
        .board_layout
        .areas
        .iter()
        .find(|(_, c)| c.area_type == wanted)
        .map(|(a, _)| *a)
        .unwrap_or_else(|| panic!("this board carries no {wanted:?}"))
}

/// Every production area, ascending.
fn production_areas(state: &GameState) -> Vec<AreaId> {
    state
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.is_production())
        .map(|(a, _)| *a)
        .collect()
}

/// Move `n` of `seat`'s HAND assistants onto `area`.
///
/// MOVED, never invented — the fixture discipline the arriving-power cells
/// already follow: a seat owns twenty pawns and a cell that conjures a
/// twenty-first is not a position the engine could hold.
fn place_assistants(state: &mut GameState, seat: usize, area: AreaId, n: u8) {
    let pawns = &mut state.players[seat].assistants;
    assert!(pawns.in_hand >= n, "seat {seat} has only {} assistants in hand", pawns.in_hand);
    pawns.in_hand -= n;
    *pawns.on_board.entry(area).or_insert(0) += n;
}

/// The reverse: `n` of `seat`'s assistants leave `area` for hand — the shape
/// the automatic Recovery step has at every boundary cell below.
fn recover_assistants(state: &mut GameState, seat: usize, area: AreaId, n: u8) {
    let pawns = &mut state.players[seat].assistants;
    let here = pawns.on_board.get(&area).copied().unwrap_or(0);
    assert!(here >= n, "seat {seat} holds only {here} assistants at {area:?}");
    if here == n {
        pawns.on_board.remove(&area);
    } else {
        pawns.on_board.insert(area, here - n);
    }
    pawns.in_hand += n;
}

/// The seat's twenty assistants, eight shophouses and four trading houses,
/// counted across every location — the conservation identity every fixture
/// here asserts before it asserts anything else.
fn token_stock(state: &GameState, seat: usize) -> (u32, u32, u32) {
    let p = &state.players[seat];
    let assistants = u32::from(p.assistants.in_hand)
        + u32::from(p.assistants.in_warehouse)
        + p.assistants.on_board.values().map(|n| u32::from(*n)).sum::<u32>()
        + u32::from(p.assistants.on_achievements)
        + u32::from(p.assistants.on_church)
        + u32::from(p.assistants.on_customs);
    let shophouses = u32::from(p.shophouses.in_hand)
        + u32::from(p.shophouses.in_warehouse)
        + p.shophouses.built.len() as u32;
    let trading = u32::from(p.trading_houses.in_hand)
        + u32::from(p.trading_houses.in_warehouse)
        + p.trading_houses.built.len() as u32;
    (assistants, shophouses, trading)
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S1 `ProductionAccess`
// ---------------------------------------------------------------------------

/// §7 audit part 4, S1's row: "S1 vs the production reward exports".
///
/// The expected value is rebuilt here from the FOUR SHIPPED HELPERS by name —
/// `copper_mine_reward` and its three siblings — rather than from the
/// dispatcher S1 itself calls, so this cell would catch a dispatcher whose arms
/// were wired to the wrong helper as well as an S1 that summed the wrong areas.
///
/// The PRESENCE GATE rides the same fixture: one production area is left empty,
/// and its reward at the power the seat would arrive with is ASSERTED nonzero,
/// so "the gate excluded it" and "the gate was never tested" cannot look alike.
///
/// **Which tile is left empty is a choice, not an accident.** At the empty tile
/// the arriving power is 1 — the president alone — and at power 1 a Copper Mine
/// and a Silk Mill both export NOTHING. Leaving either of those empty would give
/// a control that reads 0 for the tile's own reason rather than for the gate's,
/// which is exactly the inert control this cell used to have. So the empty tile
/// is the Tea Plantation: 1:1 in power, and therefore worth 1 at the very
/// bottom of its range.
#[test]
fn production_access_is_the_engines_exported_reward_at_realizable_power() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 81, &rules);
    let all = production_areas(&state);
    assert_eq!(all.len(), 4, "a 2P board carries one tile of each production type");

    let empty = area_typed(&state, AreaType::TeaPlantation);
    let occupied: Vec<AreaId> = all.iter().copied().filter(|a| *a != empty).collect();
    assert_eq!(occupied.len(), 3, "the other three production tiles");

    // Three occupied at different strengths, the Tea Plantation deliberately empty.
    place_assistants(&mut state, 0, occupied[0], 1);
    place_assistants(&mut state, 0, occupied[1], 2);
    place_assistants(&mut state, 0, occupied[2], 4);
    assert_eq!(token_stock(&state, 0), (20, 8, 4), "the fixture moved pawns; it must not invent them");

    let expected: i64 = read_s(&state, &rules, PlayerId(0), |_, view, player| {
        let mut total = 0i64;
        for area in &occupied {
            let power = realizable_power(view, player, *area);
            let printed = match view.board_layout.areas[area].area_type {
                AreaType::CopperMine => yokohama_core::actions::copper_mine_reward(power),
                AreaType::SilkMill => yokohama_core::actions::silk_mill_reward(power),
                AreaType::TeaPlantation => yokohama_core::actions::tea_plantation_reward(power),
                AreaType::FishingGround => yokohama_core::actions::fishing_ground_reward(power),
                other => panic!("{other:?} is not a production type"),
            };
            total += i64::from(printed);
        }
        total
    });
    assert!(expected > 0, "the fixture must price something, or the cell is vacuous");

    let s1 = read_s(&state, &rules, PlayerId(0), |s, view, player| s.production_access(view, player));
    assert_eq!(s1, expected, "S1 must be the summed printed exports at the arriving power");

    // The gate is legality, not a scope cut: the empty tile WOULD pay.
    let (unoccupied_would_pay, present) = read_s(&state, &rules, PlayerId(0), |_, view, player| {
        let power = realizable_power(view, player, empty);
        let pay = yokohama_core::actions::exported_reward(
            view.board_layout.areas[&empty].area_type,
            power,
        )
        .expect("a production tile is in the dispatcher's domain");
        (pay, player.assistants.in_area(empty))
    });
    assert_eq!(present, 0, "the control tile is the ungated one and must be empty");
    assert!(
        unoccupied_would_pay > 0,
        "the control tile would pay {unoccupied_would_pay} at its arriving power — a control \
         that pays nothing cannot show the gate excluded anything"
    );
    println!(
        "S1 = {s1}; the ungated Tea Plantation would have paid {unoccupied_would_pay} at its arriving power"
    );
}

/// §3.2's cited SPARSITY fact, executable: `copper_mine_reward(2) == 0`. A lone
/// assistant on a Copper Mine is a REAL presence — the gate passes, the power is
/// 2, and the printed export is still nothing.
///
/// This is the cell that separates "S1 is zero because the seat is not there"
/// from "S1 is zero because the tile pays nothing at that strength", which are
/// different facts about the same number and the reason the §13 dispatcher
/// returns `Some(0)` rather than `None` here.
#[test]
fn production_access_reads_zero_at_the_copper_mine_sparsity_floor() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 83, &rules);
    let mine = area_typed(&state, AreaType::CopperMine);
    place_assistants(&mut state, 0, mine, 1);

    let (power, s1, present) = read_s(&state, &rules, PlayerId(0), |s, view, player| {
        (
            realizable_power(view, player, mine),
            s.production_access(view, player),
            player.assistants.in_area(mine),
        )
    });
    assert_eq!(present, 1, "the presence gate must be SATISFIED here");
    assert_eq!(power, 2, "one assistant plus the arriving president");
    assert_eq!(yokohama_core::actions::copper_mine_reward(2), 0, "the printed sparsity");
    assert_eq!(s1, 0, "a satisfied gate over a tile that pays nothing at power 2");
}

/// S1's boundary paragraph, executable: "acting produces goods … and the SAME
/// turn's automatic Recovery drains the acted area's presence to HAND — the
/// access is spent to realize the yield".
///
/// Before: a sole assistant on a Tea Plantation, S1 > 0. After the Recovery that
/// ends the turn: the pawn is in hand, and S1 is 0. **The conservation half —
/// that the goods gained pay for the access lost — is family 11(a)'s weight-side
/// gate, not this cell's**; what is pinned here is the STRUCTURE of the drain.
#[test]
fn production_access_drains_to_zero_when_recovery_empties_the_acted_area() {
    let rules = Rules::standard();
    let mut before = keep_game(2, 85, &rules);
    let tea = area_typed(&before, AreaType::TeaPlantation);
    place_assistants(&mut before, 0, tea, 1);

    let mut after = before.clone();
    recover_assistants(&mut after, 0, tea, 1);
    assert_eq!(token_stock(&after, 0), token_stock(&before, 0), "Recovery moves a pawn, nothing else");

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| s.production_access(view, player))
    };
    assert!(read(&before) > 0, "a Tea Plantation pays its power, so the before side must be nonzero");
    assert_eq!(read(&after), 0, "the drained area no longer satisfies the presence gate");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S2 `TrackAccess`
// ---------------------------------------------------------------------------

/// S2 is `Σ realizable_power` over the two TRACK areas the seat occupies —
/// power-steps, not a reward table (spec §3.2's S2 card).
///
/// The two tracks are priced INDEPENDENTLY and the fixture gives them different
/// strengths, so a helper that summed one twice, or that took a maximum instead
/// of a sum, fails here rather than passing on a symmetric board.
#[test]
fn track_access_sums_the_arriving_power_over_the_two_occupied_tracks() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 87, &rules);
    let church = area_typed(&state, AreaType::Church);
    let customs = area_typed(&state, AreaType::Customs);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| s.track_access(view, player))
    };
    assert_eq!(read(&state), 0, "no presence at either track, so no access");

    place_assistants(&mut state, 0, church, 1);
    let church_only = read(&state);
    place_assistants(&mut state, 0, customs, 3);
    let both = read(&state);
    assert_eq!(token_stock(&state, 0), (20, 8, 4));

    let (church_power, customs_power) = read_s(&state, &rules, PlayerId(0), |_, view, player| {
        (realizable_power(view, player, church), realizable_power(view, player, customs))
    });
    assert_eq!(church_power, 2, "one assistant plus the arriving president");
    assert_eq!(customs_power, 4, "three assistants plus the arriving president");
    assert_eq!(church_only, i64::from(church_power), "the church leg alone");
    assert_eq!(both, i64::from(church_power) + i64::from(customs_power), "S2 sums the two legs");
}

/// **S2's BOUNDARY TWIN, named by §7: the D6 full-collapse.** "a track placement
/// CONSUMES a board assistant from the activated area whenever one stands there
/// — total collapse at exactly-one".
///
/// The after side is the D6 area-first transition written out (`tracks.rs`'s
/// `place_church_assistant`, 296-330: the pawn leaves the ACTING AREA and lands
/// on `on_church`), with the seat's twenty-pawn conservation asserted across the
/// pair so the collapse is a MOVE and not a deletion. Exactly-one is the whole
/// point: the seat still owns the pawn, it is simply no longer at the Church, so
/// the access the placement was taken FOR is gone at the instant it is realized.
#[test]
fn track_access_collapses_when_a_d6_placement_takes_the_last_assistant() {
    let rules = Rules::standard();
    let mut before = keep_game(2, 89, &rules);
    let church = area_typed(&before, AreaType::Church);
    place_assistants(&mut before, 0, church, 1);

    let mut after = before.clone();
    {
        let pawns = &mut after.players[0].assistants;
        pawns.on_board.remove(&church);
        pawns.on_church += 1;
    }
    assert_eq!(token_stock(&after, 0), token_stock(&before, 0), "the placement MOVES the pawn");

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| {
            (s.track_access(view, player), s.assistant_areas_distinct(player))
        })
    };
    assert_eq!(read(&before).0, 2, "one assistant plus the arriving president");
    assert_eq!(read(&after).0, 0, "the sole board assistant is now ON the track, not at the area");

    // S3's boundary paragraph names this same instant — "−1 at … every D6 track
    // placement emptying one" — so the second half of that paragraph is closed
    // here rather than in a fixture that would only rebuild this one.
    assert_eq!(read(&before).1, 1, "S3 counts the occupied Church before the placement");
    assert_eq!(read(&after).1, 0, "and steps down by exactly one after it");
}

/// **THE OPTIMISTIC-PROXY DECLARATION, pinned rather than repaired** (spec §3.2:
/// "S2 can read > 0 at states where the track action is Skip-only … S2 is an
/// ACCESS potential, deliberately not a legality mirror; the declared positive
/// bias is a §5.4-inspection DECLARED SHAPE").
///
/// Both named degeneracies are made executable against the engine's OWN
/// eligibility functions, so this cell would notice if the engine ever stopped
/// having the shape the declaration describes:
///
/// * Church below total faith 4 — `eligible_church_spaces` returns EMPTY at the
///   faith a payment-free visit earns, while S2 prices the visit anyway.
/// * Customs at zero imports — `customs_max_imported_goods` is bounded by power
///   but the seat has nothing to pay, so no cost is affordable at any power.
///
/// A future edit that "fixed" S2 into a legality mirror breaks this cell, which
/// is the point: the shape is registered, and un-registering it is a spec change.
#[test]
fn track_access_is_an_access_potential_and_reads_positive_where_the_action_is_skip_only() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 91, &rules);
    let church = area_typed(&state, AreaType::Church);
    let customs = area_typed(&state, AreaType::Customs);
    place_assistants(&mut state, 0, church, 1);
    place_assistants(&mut state, 0, customs, 4);
    // A payment-free visit: no goods, no yen, no imports to spend.
    {
        let p = &mut state.players[0];
        p.money = 0;
        p.copper = 0;
        p.silk = 0;
        p.tea = 0;
        p.seafood = 0;
        p.imported_goods = 0;
    }

    let (church_power, customs_power, s2) = read_s(&state, &rules, PlayerId(0), |s, view, player| {
        (
            realizable_power(view, player, church),
            realizable_power(view, player, customs),
            s.track_access(view, player),
        )
    });

    // Church: the faith a payment-free visit earns cannot reach the track's
    // first space (4).
    let faith = yokohama_core::actions::calculate_church_faith(
        church_power,
        false,
        false,
        false,
        false,
        false,
        false,
    );
    assert!(faith < 4, "a payment-free visit earns faith {faith}, below the track's first space");
    assert!(
        yokohama_core::actions::eligible_church_spaces(&state.church_track, faith).is_empty(),
        "the engine offers no church space at faith {faith}"
    );

    // Customs: power would allow imports, and the seat holds none.
    assert!(
        yokohama_core::actions::customs_max_imported_goods(customs_power) > 0,
        "the fixture's customs power is above the hard gate, so the zero is the STOCK's"
    );
    assert_eq!(state.players[0].imported_goods, 0, "nothing to pay with");

    assert_eq!(
        s2,
        i64::from(church_power) + i64::from(customs_power),
        "S2 prices both visits at full arriving power although BOTH are Skip-only"
    );
    println!("S2 declared-shape cell: church faith {faith}, S2 = {s2} at a Skip-only pair");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S5 `BankAccess`
// ---------------------------------------------------------------------------

/// §7 audit part 4, S5's row: "S5 vs `bank_reward`".
///
/// The Bank pays 1..5 Yen linearly in power (manual p.12), so the sweep walks
/// the seat's presence from one assistant to four and pins every step against
/// the shipped helper — a cell at a single strength would pass on a helper that
/// returned a constant.
#[test]
fn bank_access_is_the_engines_bank_reward_at_realizable_power() {
    let rules = Rules::standard();
    let base = keep_game(2, 93, &rules);
    let bank = area_typed(&base, AreaType::Bank);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| {
            (s.bank_access(view, player), realizable_power(view, player, bank))
        })
    };
    assert_eq!(read(&base).0, 0, "no presence at the Bank, so no access");

    let mut seen = Vec::new();
    for pawns in 1..=4u8 {
        let mut state = base.clone();
        place_assistants(&mut state, 0, bank, pawns);
        assert_eq!(token_stock(&state, 0), (20, 8, 4));
        let (s5, power) = read(&state);
        assert_eq!(power, pawns + 1, "assistants plus the arriving president");
        assert_eq!(
            s5,
            i64::from(yokohama_core::actions::bank_reward(power)),
            "S5 must be the printed bank reward at the arriving power"
        );
        seen.push(s5);
    }
    assert_eq!(seen, vec![2, 3, 4, 5], "the Bank's linear yen ladder, capped at 5");
}

/// S5's boundary paragraph: "acting converts the access into E1 AND drains the
/// presence at Recovery — family 11(b)". The conservation half is 11(b)'s,
/// weight-side; the STRUCTURE of the drain is pinned here.
#[test]
fn bank_access_drains_to_zero_when_recovery_empties_the_bank() {
    let rules = Rules::standard();
    let mut before = keep_game(2, 95, &rules);
    let bank = area_typed(&before, AreaType::Bank);
    place_assistants(&mut before, 0, bank, 2);
    let mut after = before.clone();
    recover_assistants(&mut after, 0, bank, 2);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| s.bank_access(view, player))
    };
    assert_eq!(read(&before), 3, "two assistants plus the arriving president, paid 1:1");
    assert_eq!(read(&after), 0, "the drained Bank no longer satisfies the presence gate");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S6 `EmploymentAgencyAccess`
// ---------------------------------------------------------------------------

/// §7 audit part 4, S6's row: "S6 vs `employment_agency_tokens_allowed` + the
/// warehouse-min".
///
/// Both sides of the `min` are exercised on purpose. At a FULL warehouse the
/// allowance binds (0/0/1/2/3/4 per power, manual p.14); at a nearly-empty one
/// the SUPPLY binds, which is the clause §3.2 says "repairs the structural zero
/// — the warehouse is one-way and only drains, so an ungated read over-credits
/// monotonically".
#[test]
fn employment_agency_access_is_the_allowance_capped_by_the_warehouse_supply() {
    let rules = Rules::standard();
    let base = keep_game(2, 97, &rules);
    let ea = area_typed(&base, AreaType::EmploymentAgency);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| {
            (s.employment_agency_access(view, player), realizable_power(view, player, ea))
        })
    };
    assert_eq!(read(&base).0, 0, "no presence at the Employment Agency, so no access");

    // The ALLOWANCE side: a full warehouse (12 assistants + 6 shophouses + 4
    // trading houses = 22 tokens) can never be the binding half.
    let mut allowance_seen = Vec::new();
    for pawns in 1..=4u8 {
        let mut state = base.clone();
        place_assistants(&mut state, 0, ea, pawns);
        let (s6, power) = read(&state);
        let allowed = yokohama_core::actions::employment_agency_tokens_allowed(power);
        let warehouse = i64::from(state.players[0].assistants.in_warehouse)
            + i64::from(state.players[0].shophouses.in_warehouse)
            + i64::from(state.players[0].trading_houses.in_warehouse);
        assert!(warehouse > i64::from(allowed), "the allowance must be the binding half here");
        assert_eq!(s6, i64::from(allowed), "S6 must be the printed allowance at power {power}");
        allowance_seen.push(s6);
    }
    assert_eq!(allowance_seen, vec![1, 2, 3, 4], "the printed 0/0/1/2/3/4 ladder at powers 2..5");

    // The SUPPLY side: one token left in the whole warehouse, power 5.
    let mut starved = base.clone();
    place_assistants(&mut starved, 0, ea, 4);
    {
        let p = &mut starved.players[0];
        p.assistants.in_hand += p.assistants.in_warehouse - 1;
        p.assistants.in_warehouse = 1;
        p.shophouses.in_hand += p.shophouses.in_warehouse;
        p.shophouses.in_warehouse = 0;
        p.trading_houses.in_hand += p.trading_houses.in_warehouse;
        p.trading_houses.in_warehouse = 0;
    }
    assert_eq!(token_stock(&starved, 0), (20, 8, 4), "the warehouse drained INTO hand, nothing lost");
    let (s6, power) = read(&starved);
    assert_eq!(power, 5, "four assistants plus the arriving president");
    assert_eq!(yokohama_core::actions::employment_agency_tokens_allowed(5), 4, "the allowance is 4");
    assert_eq!(s6, 1, "the one token left in the warehouse is the binding half");
}

/// **S6's OPTIMISTIC-PROXY DECLARATION, pinned rather than repaired** (spec
/// §3.2: "the warehouse-min gates supply, not AFFORDABILITY — a warehouse
/// holding only trading-house tokens against an empty purse is Skip-only while
/// S6 reads > 0 … the declared positive bias is a §5.4 DECLARED SHAPE").
///
/// The fixture is exactly the sentence: every assistant and shophouse out of the
/// warehouse, four trading-house tokens still in it, and a purse of zero against
/// a printed first take of 4 Yen (`TradingHouseState::take_cost(0)`).
#[test]
fn employment_agency_access_gates_supply_and_not_affordability() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 99, &rules);
    let ea = area_typed(&state, AreaType::EmploymentAgency);
    place_assistants(&mut state, 0, ea, 2);
    {
        let p = &mut state.players[0];
        p.assistants.in_hand += p.assistants.in_warehouse;
        p.assistants.in_warehouse = 0;
        p.shophouses.in_hand += p.shophouses.in_warehouse;
        p.shophouses.in_warehouse = 0;
        p.money = 0;
    }
    assert_eq!(token_stock(&state, 0), (20, 8, 4));
    assert_eq!(state.players[0].trading_houses.in_warehouse, 4, "only trading houses remain");
    assert_eq!(
        TradingHouseState::take_cost(0),
        4,
        "the first trading-house take costs 4 Yen, which the seat has not got"
    );

    let s6 = read_s(&state, &rules, PlayerId(0), |s, view, player| {
        s.employment_agency_access(view, player)
    });
    assert_eq!(s6, 2, "the allowance at power 3 binds; affordability is deliberately not read");
}

/// The second half of S6's boundary paragraph — "presence drains at Recovery" —
/// closed for symmetry with S1's and S5's drain cells. The first half (the take
/// converting warehouse stock into hand stock) is the SUPPLY side of
/// `employment_agency_access_is_the_allowance_capped_by_the_warehouse_supply`,
/// where exactly that conversion drops the reading to the remaining token.
///
/// The conservation half of both is family 11(c)'s, weight-side, and is not
/// this cell's to assert.
///
/// # The third state exists to KILL the presence gate, and nothing else did
///
/// A drained agency reads 0 for TWO reasons at once, and only one of them is the
/// gate: with no assistants the arriving power is 1, and
/// `employment_agency_tokens_allowed(1)` is 0 — so deleting the gate from the
/// helper leaves every other fixture in this file green.
///
/// The `stationed` state separates them. The station token raises the arriving
/// power to 2, where the printed allowance is one token, at an agency the seat
/// has still never touched and with a full warehouse behind it. An UNGATED S6
/// reads 1 there; the gated one must read 0, and that assert is the gate's only
/// executable witness in the repo.
#[test]
fn employment_agency_access_drains_to_zero_when_recovery_empties_the_agency() {
    let rules = Rules::standard();
    let mut before = keep_game(2, 139, &rules);
    let agency = area_typed(&before, AreaType::EmploymentAgency);
    place_assistants(&mut before, 0, agency, 3);
    let mut after = before.clone();
    recover_assistants(&mut after, 0, agency, 3);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, view, player| {
            (s.employment_agency_access(view, player), realizable_power(view, player, agency))
        })
    };
    assert_eq!(
        read(&before).0,
        3,
        "three assistants plus the arriving president allows three tokens"
    );
    assert_eq!(read(&after).0, 0, "the drained agency no longer satisfies the presence gate");

    let mut stationed = after.clone();
    stationed.station_location = Some(agency);
    let (s6, power) = read(&stationed);
    assert_eq!(
        stationed.players[0].assistants.in_area(agency),
        0,
        "the gate-killing state must have NO presence at the agency"
    );
    assert_eq!(power, 2, "the arriving president plus the station token");
    assert_eq!(
        yokohama_core::actions::employment_agency_tokens_allowed(power),
        1,
        "an ungated read would find an allowance of 1 here — that is the mutant this kills"
    );
    assert!(
        stationed.players[0].assistants.in_warehouse
            + stationed.players[0].shophouses.in_warehouse
            + stationed.players[0].trading_houses.in_warehouse
            > 0,
        "the warehouse must be stocked, or the min would hide the gate again"
    );
    assert_eq!(s6, 0, "presence is the gate, and the seat has none at this agency");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S3 `AssistantAreasDistinct`
// ---------------------------------------------------------------------------

/// S3 is a COUNT of non-Canal areas holding at least one own assistant — the
/// spread axis (spec §3.2's S3 card).
///
/// The fixture stacks one area and spreads two others, so a helper that summed
/// pawns instead of counting areas reads 5 where this reads 3.
#[test]
fn assistant_areas_distinct_counts_areas_and_not_pawns() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 101, &rules);
    let areas = production_areas(&state);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, player| s.assistant_areas_distinct(player))
    };
    assert_eq!(read(&state), 0, "an empty board holds no areas");

    place_assistants(&mut state, 0, areas[0], 3);
    place_assistants(&mut state, 0, areas[1], 1);
    place_assistants(&mut state, 0, areas[2], 1);
    assert_eq!(token_stock(&state, 0), (20, 8, 4));
    assert_eq!(read(&state), 3, "three distinct areas, five pawns");
}

/// S3's boundary paragraph: "−1 at every Recovery of a sole-pawn area … no cliff
/// (a count)".
///
/// Both halves in one pair: draining the STACKED area by one moves nothing, and
/// draining the sole-pawn area moves it by exactly one. That is what "no cliff"
/// means for a count, and it is the property that distinguishes S3 from every
/// gated access term beside it.
#[test]
fn assistant_areas_distinct_steps_by_one_and_never_cliffs() {
    let rules = Rules::standard();
    let mut base = keep_game(2, 103, &rules);
    let areas = production_areas(&base);
    place_assistants(&mut base, 0, areas[0], 3);
    place_assistants(&mut base, 0, areas[1], 1);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, player| s.assistant_areas_distinct(player))
    };
    assert_eq!(read(&base), 2);

    let mut thinned = base.clone();
    recover_assistants(&mut thinned, 0, areas[0], 1);
    assert_eq!(read(&thinned), 2, "a stacked area keeps its bit — no cliff");

    let mut emptied = base.clone();
    recover_assistants(&mut emptied, 0, areas[1], 1);
    assert_eq!(read(&emptied), 1, "the sole-pawn area is worth exactly one step");
}

/// S3's Canal clause, and the honest note about where it can bite.
///
/// **On every board the engine can produce this clause is inert**: 2P carries no
/// Canal tile at all, and at 3P/4P no pawn may ever be placed on the one that
/// exists (`is_possible_action_area_full` movement.rs:71-73 refuses it). The
/// organic half below asserts exactly that — every occupied area is placeable
/// across sampled 3P play.
///
/// The second half is a CONSTRUCTED position the rules cannot reach, and it is
/// here for the same reason `president_relocated` is: the clause has to be shown
/// LOAD-BEARING in the code rather than decorative, and the only way to show a
/// filter removes something is to hand it something to remove. Nothing is
/// applied to, replayed from, or scored on this state beyond the one count.
#[test]
fn assistant_areas_distinct_excludes_the_canal() {
    let rules = Rules::standard();
    let mut state = keep_game(3, 105, &rules);
    let canal = area_typed(&state, AreaType::Canal);
    let production = production_areas(&state);

    // Organic half: the engine never puts a pawn on the Canal.
    let mut rng = ChaCha8Rng::seed_from_u64(0x0CA9A1);
    let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
    let mut organic = state.clone();
    for _ in 0..60usize {
        if is_game_over(&organic) {
            break;
        }
        for player in &organic.players {
            assert_eq!(
                player.assistants.in_area(canal),
                0,
                "the engine placed a pawn on the Canal, which movement.rs:71-73 refuses"
            );
        }
        let mv = {
            let handle = DecisionHandle::new(&organic, &rules);
            policy.pick(&handle, &mut rng).expect("the rung always finds a move")
        };
        organic = apply_move_unchecked(&organic, &mv, &rules).expect("a sampled move applies");
    }

    // Constructed half: the filter removes what it is handed.
    place_assistants(&mut state, 0, production[0], 1);
    let without_canal = read_s(&state, &rules, PlayerId(0), |s, _, p| s.assistant_areas_distinct(p));
    place_assistants(&mut state, 0, canal, 1);
    let with_canal = read_s(&state, &rules, PlayerId(0), |s, _, p| s.assistant_areas_distinct(p));
    assert_eq!(without_canal, 1);
    assert_eq!(with_canal, 1, "the Canal pawn must not raise the count");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S4 `PresidentLocalAccess`
// ---------------------------------------------------------------------------

/// S4's ON-BOARD branch: `own_assistants(a)` plus the same sum over the
/// FREE-adjacent ring (spec §3.2's S4 card).
///
/// Three things are separated in one fixture: the ANCHOR term (pawns under the
/// president), the RING term (pawns one free step away), and the areas that are
/// neither — a pawn two steps out must not count, or "local" means nothing.
#[test]
fn president_local_access_sums_the_anchor_and_the_free_adjacent_ring() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 107, &rules);
    let context = {
        let view = observe(&state, PlayerId(0)).unwrap();
        ScoreStructure::derive(&view, &rules).context
    };

    // An anchor with a neighbour and a non-neighbour, read off the layout.
    let anchor = *state.board_layout.areas.keys().next().unwrap();
    let ring: Vec<AreaId> = areas_in(context.free_adjacent(anchor)).collect();
    assert!(!ring.is_empty(), "every area on a staggered board has a neighbour");
    let far = *state
        .board_layout
        .areas
        .keys()
        .find(|a| **a != anchor && context.free_adjacent(anchor) & area_bit(**a) == 0)
        .expect("a 10-tile board has a non-neighbour");

    state.players[0].president_location = PresidentLocation::OnBoard(anchor);
    place_assistants(&mut state, 0, anchor, 2);
    place_assistants(&mut state, 0, ring[0], 3);
    place_assistants(&mut state, 0, far, 3);
    assert_eq!(token_stock(&state, 0), (20, 8, 4));

    let s4 = read_s(&state, &rules, PlayerId(0), |s, _, p| s.president_local_access(p));
    assert_eq!(s4, 5, "2 under the president + 3 one free step away; the 3 two steps out do not count");
}

/// The FREE-edge subtraction inside S4's ring, at the only player counts that
/// can show it: a pawn across a TOLLED Canal crossing is adjacent in the layout
/// and is NOT in the ring.
///
/// This is the cell the milestone's own 2P fixture package cannot contain — a 2P
/// board has no Canal tile, so the subtraction is inert there. The sweep walks
/// 3P and 4P boards until it finds a crossing whose endpoint it can anchor on,
/// and FLOORS the number of crossings it exercised.
#[test]
fn president_local_access_does_not_reach_across_a_tolled_canal_crossing() {
    let rules = Rules::standard();
    let mut exercised = 0usize;
    for (players, seed) in [(3u8, 109u64), (4, 111)] {
        let base = keep_game(players, seed, &rules);
        let crossings = base.board_layout.canal_crossings.clone();
        assert!(!crossings.is_empty(), "{players}P boards carry a Canal");
        for (a, b) in crossings {
            let mut state = base.clone();
            state.players[0].president_location = PresidentLocation::OnBoard(a);
            place_assistants(&mut state, 0, b, 2);
            let s4 = read_s(&state, &rules, PlayerId(0), |s, _, p| s.president_local_access(p));
            assert!(
                base.board_layout.adjacent_areas(a).contains(&b),
                "a tolled pair is adjacent in the layout — that is what makes this cell non-vacuous"
            );
            assert_eq!(s4, 0, "{players}P {a:?}->{b:?}: the tolled edge is not a FREE edge");
            exercised += 1;
        }
    }
    assert!(exercised > 0, "no crossing was exercised");
    println!("S4 free-edge subtraction: {exercised} tolled crossings checked");
}

/// S4's IN-HAND branch: the maximum local sum over the seat's own-assistant,
/// non-Canal, opponent-President-free areas — "the `PlacePresident` target class
/// MODULO the placement fee".
///
/// Three clauses, one fixture: the maximum is taken (not the first or the sum),
/// an area blocked by the OPPONENT's President is not a candidate even though
/// the seat stands there, and a seat with no board assistants reads 0.
#[test]
fn president_local_access_in_hand_maximises_over_the_reentry_targets() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 113, &rules);
    state.players[0].president_location = PresidentLocation::InHand;

    // No board assistants at all: the declared 0 — for S4 and, on the same
    // state and for the same reason, for S7's in-hand branch, whose card
    // carries the identical "0 with no board assistants" sentence.
    let (s4_empty, s7_empty) = read_s(&state, &rules, PlayerId(0), |s, _, p| {
        (s.president_local_access(p), s.president_reachable_areas(p))
    });
    assert_eq!(s4_empty, 0, "a seat with no board presence has no re-entry");
    assert_eq!(s7_empty, 0, "and no area is actionable in principle from a hand it cannot leave");

    let context = {
        let view = observe(&state, PlayerId(0)).unwrap();
        ScoreStructure::derive(&view, &rules).context
    };
    let areas: Vec<AreaId> = state.board_layout.areas.keys().copied().collect();
    let weak = areas[0];
    let strong = *areas
        .iter()
        .find(|a| **a != weak && context.free_adjacent(weak) & area_bit(**a) == 0)
        .expect("a non-neighbour of the weak candidate");
    // Isolated from BOTH candidates: a blocked area adjacent to one of them
    // would still pay into that one's ring, and the cell would then be about
    // ring arithmetic instead of about the exclusion it names.
    let blocked = *areas
        .iter()
        .find(|a| {
            **a != weak
                && **a != strong
                && context.free_adjacent(weak) & area_bit(**a) == 0
                && context.free_adjacent(strong) & area_bit(**a) == 0
        })
        .expect("a third area isolated from both candidates");

    place_assistants(&mut state, 0, weak, 1);
    place_assistants(&mut state, 0, strong, 3);
    place_assistants(&mut state, 0, blocked, 4);
    state.players[1].president_location = PresidentLocation::OnBoard(blocked);
    assert_eq!(token_stock(&state, 0), (20, 8, 4));

    let s4 = read_s(&state, &rules, PlayerId(0), |s, _, p| s.president_local_access(p));
    assert_eq!(
        s4, 3,
        "the maximum over the re-entry class: the 4-pawn area is barred by the opponent's President"
    );
}

/// **S4's BOUNDARY TWIN, named by §7: the recall branch-switch.** "recall
/// switches to the in-hand form".
///
/// One position, two President locations, and the two branches answer
/// DIFFERENTLY — an on-board president anchored on a thin area reads its own
/// local sum, while recalling it lets the in-hand branch pick the seat's best
/// re-entry instead. A helper that ran one branch for both would pass every
/// other S4 cell and fail this one.
#[test]
fn president_local_access_switches_branches_when_the_president_is_recalled() {
    let rules = Rules::standard();
    let mut base = keep_game(2, 115, &rules);
    let context = {
        let view = observe(&base, PlayerId(0)).unwrap();
        ScoreStructure::derive(&view, &rules).context
    };
    let areas: Vec<AreaId> = base.board_layout.areas.keys().copied().collect();
    let thin = areas[0];
    let rich = *areas
        .iter()
        .find(|a| **a != thin && context.free_adjacent(thin) & area_bit(**a) == 0)
        .expect("a non-neighbour");
    place_assistants(&mut base, 0, thin, 1);
    place_assistants(&mut base, 0, rich, 4);

    let mut on_board = base.clone();
    on_board.players[0].president_location = PresidentLocation::OnBoard(thin);
    let mut recalled = base.clone();
    recalled.players[0].president_location = PresidentLocation::InHand;

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, p| s.president_local_access(p))
    };
    assert_eq!(read(&on_board), 1, "anchored on the thin area, the ring reaches nothing");
    assert_eq!(read(&recalled), 4, "in hand, the best re-entry is the rich area");

    // The paragraph's OTHER half — "movement re-centers" — is the same position
    // with the president standing somewhere else. The fees that movement really
    // costs are E1's; what S4 shows is the re-centring itself.
    let mut recentred = base.clone();
    recentred.players[0].president_location = PresidentLocation::OnBoard(rich);
    assert_eq!(read(&recentred), 4, "moving the president moves the centre, and the value with it");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — S7 `PresidentReachableAreas`
// ---------------------------------------------------------------------------

/// A three-link chain on whatever board the seed produced: `start`, a free
/// neighbour `near`, and a free neighbour of `near` that is NOT a neighbour of
/// `start`. Derived from the layout rather than hand-listed, so the cells below
/// do not depend on which tiles the shuffle put where.
fn chain_of_three(state: &GameState, context: &yokohama_core::game::eval::ViewContext) -> [AreaId; 3] {
    for start in state.board_layout.areas.keys().copied() {
        for near in areas_in(context.free_adjacent(start)) {
            for far in areas_in(context.free_adjacent(near)) {
                if far != start && context.free_adjacent(start) & area_bit(far) == 0 {
                    return [start, near, far];
                }
            }
        }
    }
    panic!("this board has no two-step chain, which a staggered pyramid always has")
}

/// S7's ON-BOARD branch walks the ASSISTANT CHAIN: an area is reachable only by
/// stepping onto areas the seat occupies (spec §3.2's S7 card, mirroring
/// `can_step_onto` movement.rs:256-260).
///
/// The pair is the whole property: with the middle link occupied both areas are
/// reachable; with the middle link EMPTY the far one is not, even though it
/// still holds the seat's pawns. A helper that counted own-assistant areas
/// without walking would read 1 on the second side instead of 0.
///
/// Read in the other direction, this pair IS S7's boundary paragraph: placement
/// on the middle link extends the frontier by two areas at once, and a Recovery
/// that empties it shrinks the frontier back to nothing. The recall branch of
/// the same paragraph has its own cell below.
#[test]
fn president_reachable_areas_walks_the_assistant_chain_along_free_edges() {
    let rules = Rules::standard();
    let mut base = keep_game(2, 117, &rules);
    let context = {
        let view = observe(&base, PlayerId(0)).unwrap();
        ScoreStructure::derive(&view, &rules).context
    };
    let [start, near, far] = chain_of_three(&base, &context);
    base.players[0].president_location = PresidentLocation::OnBoard(start);

    let mut linked = base.clone();
    place_assistants(&mut linked, 0, near, 1);
    place_assistants(&mut linked, 0, far, 1);
    let mut broken = base.clone();
    place_assistants(&mut broken, 0, far, 1);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, p| s.president_reachable_areas(p))
    };
    assert_eq!(read(&linked), 2, "the chain reaches both links");
    assert_eq!(read(&broken), 0, "a gap in the chain stops the walk at the president's own area");
}

/// **The declared narrowing that is easiest to get backwards**: an opponent's
/// President blocks STOPPING, never PASS-THROUGH (spec §3.2's S7 card, citing
/// `can_step_onto` movement.rs:256-260 — which has no President clause — against
/// `is_possible_action_area_full` movement.rs:92-101, which does).
///
/// So the middle link stays walkable while ceasing to be countable: the far area
/// is still reached THROUGH it, and the count drops by exactly the blocked area.
#[test]
fn president_reachable_areas_transits_an_opponent_president_without_counting_it() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 119, &rules);
    let context = {
        let view = observe(&state, PlayerId(0)).unwrap();
        ScoreStructure::derive(&view, &rules).context
    };
    let [start, near, far] = chain_of_three(&state, &context);
    state.players[0].president_location = PresidentLocation::OnBoard(start);
    place_assistants(&mut state, 0, near, 1);
    place_assistants(&mut state, 0, far, 1);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, p| {
            (s.president_reachable_areas(p), s.president_reachable_set(p))
        })
    };
    let (open, open_set) = read(&state);
    assert_eq!(open, 2);

    state.players[1].president_location = PresidentLocation::OnBoard(near);
    let (blocked, blocked_set) = read(&state);
    assert_eq!(blocked, 1, "the blocked link is no longer countable");
    assert_ne!(blocked_set & area_bit(far), 0, "the far area is still reached THROUGH the block");
    assert_eq!(blocked_set & area_bit(near), 0, "the blocked area itself is not counted");
    assert_eq!(open_set & area_bit(near), area_bit(near), "the open side counted it");
}

/// **S7's BOUNDARY TWIN, named by §7 beside S4's: the recall branch-switch**,
/// and the DECLARED structural step it carries — "a DECLARED structural
/// +1-class step in the RECALLED direction (the on-board branch subtracts the
/// barred own area; the in-hand PAA count does not) … named rather than denied".
///
/// One position, two President locations: on the board the president's own area
/// is barred as next turn's destination (`apply_move.rs:241-250`) and drops out
/// of the count; recalled, the same area is an ordinary `PlacePresident` target
/// and the count is one higher. The step is asserted to be exactly +1 here, so
/// the declaration is a measurement rather than a description.
#[test]
fn president_reachable_areas_steps_up_by_one_when_the_president_is_recalled() {
    let rules = Rules::standard();
    let mut base = keep_game(2, 121, &rules);
    let context = {
        let view = observe(&base, PlayerId(0)).unwrap();
        ScoreStructure::derive(&view, &rules).context
    };
    let [start, near, _] = chain_of_three(&base, &context);
    place_assistants(&mut base, 0, start, 1);
    place_assistants(&mut base, 0, near, 1);

    let mut on_board = base.clone();
    on_board.players[0].president_location = PresidentLocation::OnBoard(start);
    let mut recalled = base.clone();
    recalled.players[0].president_location = PresidentLocation::InHand;

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, p| {
            (s.president_reachable_areas(p), s.president_reachable_set(p))
        })
    };
    let (standing, standing_set) = read(&on_board);
    let (in_hand, _) = read(&recalled);
    assert_eq!(standing_set & area_bit(start), 0, "the president's own area is barred");
    assert_eq!(standing, 1, "only the neighbour is actionable next turn");
    assert_eq!(in_hand, 2, "recalled, the seat's own area rejoins the placement class");
    assert_eq!(in_hand - standing, 1, "the DECLARED +1-class step in the recalled direction");
}

/// §7 audit part 4, S7's first row: "S7's counted set ⊆ the engine's
/// `is_possible_action_area_full` set on president-present fixtures".
///
/// Swept over organic 2P and 3P play at every seat whose President is on the
/// board. The containment is one-directional BY DESIGN and the cell says so: the
/// engine's set is every area the seat may act at, and S7 narrows it twice more
/// — to the areas the assistant chain REACHES, and away from the seat's own
/// barred area. The count of strictly-narrower readings is floored, so a helper
/// that had quietly become the engine's own set would fail here rather than pass
/// a containment it satisfies trivially.
#[test]
fn president_reachable_set_is_contained_in_the_engines_possible_action_areas() {
    let rules = Rules::standard();
    let (mut compared, mut strictly_narrower) = (0usize, 0usize);
    for (players, seed) in [(2u8, 123u64), (3, 125)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x7E7E);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..120usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(5) {
                for i in 0..players {
                    let seat = PlayerId(i);
                    let counted = read_s(&state, &rules, seat, |s, _, p| {
                        match p.president_location {
                            PresidentLocation::OnBoard(_) => Some(s.president_reachable_set(p)),
                            PresidentLocation::InHand => None,
                        }
                    });
                    let Some(counted) = counted else { continue };
                    let mut engine: u32 = 0;
                    for area in state.board_layout.areas.keys().copied() {
                        if is_possible_action_area_full(&state, seat, area, Some(&rules.techs), false)
                        {
                            engine |= area_bit(area);
                        }
                    }
                    assert_eq!(
                        counted & !engine,
                        0,
                        "{players}P seed {seed} step {step} seat {i}: S7 counted an area the \
                         engine refuses as a possible action area"
                    );
                    compared += 1;
                    if counted != engine {
                        strictly_narrower += 1;
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(compared > 20, "only {compared} president-present comparisons");
    assert!(strictly_narrower > 0, "S7 never narrowed the engine's set — containment is vacuous");
    println!(
        "S7 containment: {compared} comparisons, {strictly_narrower} strictly narrower than the engine's set"
    );
}

/// §7 audit part 4, S7's second row: "the in-hand count = the `PlacePresident`
/// destination count modulo fee".
///
/// The fee is the "modulo": the generator drops a destination the seat cannot
/// pay for (`legal_moves.rs:275-282`), and §3.2 declares that filter unmodeled,
/// so the fixture funds the seat well past any fee and the two counts must then
/// agree exactly. The opponent's President sits on one of the seat's own areas,
/// so the pin has something to exclude on BOTH sides rather than agreeing by
/// having nothing to disagree about.
#[test]
fn president_reachable_areas_in_hand_is_the_place_president_destination_count() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 127, &rules);
    let areas: Vec<AreaId> = state.board_layout.areas.keys().copied().collect();
    place_assistants(&mut state, 0, areas[0], 1);
    place_assistants(&mut state, 0, areas[1], 2);
    place_assistants(&mut state, 0, areas[2], 1);
    state.players[1].president_location = PresidentLocation::OnBoard(areas[2]);
    state.players[0].president_location = PresidentLocation::InHand;
    state.players[0].money = 50;
    state.current_player = PlayerId(0);
    state.turn_phase = TurnPhase::Movement;

    let destinations = get_legal_moves(&state, &rules)
        .iter()
        .filter(|mv| matches!(mv, Move::PlacePresident { .. }))
        .count();
    let s7 = read_s(&state, &rules, PlayerId(0), |s, _, p| s.president_reachable_areas(p));

    assert_eq!(destinations, 2, "two of the seat's three areas are free of the opponent's President");
    assert_eq!(s7, destinations as i64, "the in-hand count IS the placement class");
}

// ---------------------------------------------------------------------------
// V4 §7 audit contract — Group S, the cross-cutting parts
// ---------------------------------------------------------------------------

/// The Group-S FEATURE BODIES, by source-text head — the subject of the two
/// scans below. Hand-maintained on purpose: a new S helper not added here is a
/// helper neither scan covers, which is exactly the omission an eighth feature
/// would make silently.
///
/// **This is not every function a Group-S read executes, and the difference
/// matters when reading what the scans certify.** Four kinds of code run below
/// these bodies and are deliberately outside the list, because each already has
/// a guard that suits it better than a text scan:
///
/// * `realizable_power` — pinned as arithmetic against the engine's own power
///   function by the four cells at the end of this file, which is a stronger
///   statement than "its text mentions no money".
/// * `ViewContext`'s accessors (`free_adjacent`, `opponent_presidents`,
///   `occupied_placeable`, `reentry_targets`) and the `derive` pass that fills
///   them — pinned VALUE-wise by the three `view_context_*` cells against the
///   board layout they are derived from.
/// * `area_bit` / `areas_in` / `seat_index` — index arithmetic over an `AreaId`
///   or a seat number, with no player row in scope to read a field of.
/// * The engine's three printed-reward helpers, which take a `u8` and return a
///   `u8`.
///
/// So the cleanliness scan's claim is precise: **the feature bodies name no
/// money, goods or order field.** Nothing beneath them could supply one — that
/// half rests on the guards named above, not on this list.
/// The ECONOMIC side of §2's partition, as source-text needles: money, imported
/// goods, the order pipeline and the four trade-good stocks.
///
/// Declared once and shared by all three cleanliness clauses. "No S helper reads
/// these", "Group E is what reads these", and "only X reads BOTH sides" are only
/// statements about one partition if the three scans look for the same words; a
/// per-test copy would let the closer drift away from the clause it closes.
const ECONOMIC_READS: [&str; 8] = [
    ".money",
    ".imported_goods",
    ".hand",
    ".completed_orders",
    ".copper",
    ".silk",
    ".tea",
    ".seafood",
];

/// The SPATIAL side of the same partition: power, presence, position and the
/// board. See [`ECONOMIC_READS`] for why both lists live at file scope.
const SPATIAL_READS: [&str; 7] = [
    "realizable_power(",
    ".assistants",
    "president",
    ".occupied",
    ".built",
    "station_location",
    "board_layout",
];

const S_HELPER_HEADS: [&str; 9] = [
    "    pub fn production_access(",
    "    pub fn track_access(",
    "    pub fn assistant_areas_distinct(",
    "    pub fn president_local_access(",
    "    fn local_assistant_mass(",
    "    pub fn bank_access(",
    "    pub fn employment_agency_access(",
    "    pub fn president_reachable_areas(",
    "    pub fn president_reachable_set(",
];

/// One function's DECLARATION AND BODY, from its head to the `}` at the head's
/// OWN indentation — so the same extractor serves an `impl` method and the
/// top-level control function below.
///
/// Doc comments sit above the head and are deliberately outside the slice: a
/// scan that swallowed them would trip over the prose that NAMES the things the
/// code may not read.
fn s_helper_source(head: &str) -> &'static str {
    const SRC: &str = include_str!("../src/game/eval.rs");
    let indent: String = head.chars().take_while(|c| *c == ' ').collect();
    let closing = format!("\n{indent}}}");
    let start = SRC.find(head).unwrap_or_else(|| panic!("eval.rs declares `{head}`"));
    let rest = &SRC[start..];
    let end = rest.find(&closing).expect("the function closes at its own indentation");
    &rest[..end]
}

/// §7 audit part 1 for Group S: **view honesty, at the type level.**
///
/// The whole-file guard above already refuses the engine-state identifiers
/// anywhere in `eval.rs`. This is the narrower, positive statement §7 asks for:
/// every Group-S helper's parameter list names ONLY a view, a seat's public row,
/// and an area — so the honesty claim is readable off the signature rather than
/// inferred from the absence of a forbidden word.
///
/// (Group O's substitution-invariance half of part 1 belongs to the batch that
/// builds O1; Group S reads no hidden-slot-bearing field at all.)
#[test]
fn every_group_s_helper_takes_only_a_view_a_public_row_and_an_area() {
    const ALLOWED_PARAM_TYPES: [&str; 3] = ["&PlayerView", "&VisiblePlayer", "AreaId"];
    for head in S_HELPER_HEADS {
        let source = s_helper_source(head);
        let open = source.find('(').expect("a declaration has an argument list");
        let close = source.find(')').expect("a declaration closes its argument list");
        let params = &source[open + 1..close];
        let mut named = 0usize;
        for (i, param) in params.split(',').map(str::trim).filter(|p| !p.is_empty()).enumerate() {
            if i == 0 {
                assert_eq!(param, "&self", "`{head}` must be a method on the structure");
                continue;
            }
            let ty = param.split(": ").nth(1).unwrap_or_else(|| panic!("`{param}` has no type"));
            assert!(
                ALLOWED_PARAM_TYPES.contains(&ty),
                "`{head}` takes `{ty}`, which is not one of {ALLOWED_PARAM_TYPES:?} — a Group-S \
                 helper is a function of a seat's VIEW"
            );
            named += 1;
        }
        assert!(named > 0, "`{head}` names no argument beyond `&self`");
    }
}

/// §7's cross-group cleanliness assert, S half: **no S helper reads money,
/// imported goods, or order cards.**
///
/// Group S prices presence, power, position and the board; the resource and
/// order legs are Group E's and Group O's, and §3.2 registers the interaction
/// cell (Group X) as the ONLY group that reads both sides. A leak here would not
/// break a test — it would quietly make an S/E partition duty unmeasurable,
/// which is the failure mode §2's partition table exists to prevent.
///
/// ANTI-VACUITY: the same needles are asserted to be FINDABLE in `eval.rs` —
/// `remainder_bonus` reads every one of them — so a scan that had stopped
/// matching anything at all fails here rather than passing everywhere.
#[test]
fn no_group_s_helper_reads_money_imports_or_order_cards() {
    for head in S_HELPER_HEADS {
        let source = s_helper_source(head);
        for needle in ECONOMIC_READS {
            assert!(
                !source.contains(needle),
                "`{head}` reads `{needle}` — Group S prices presence, power and position; \
                 money/goods/orders belong to Groups E and O, and only Group X reads both"
            );
        }
    }
    // The needles are real field reads somewhere, or this scan proves nothing.
    let control = s_helper_source("fn remainder_bonus(");
    let found = ECONOMIC_READS.iter().filter(|n| control.contains(**n)).count();
    assert!(
        found >= 5,
        "the control function matched only {found} of the forbidden needles — the scan has \
         stopped being able to find a real read"
    );
}

/// §7 audit part 2(a) for Group S: **INDEX-PERMUTATION INVARIANCE** — relabeling
/// the seats permutes every feature's per-seat values identically (§5.2 family
/// 5(a), which restates seat-locality in exactly these terms).
///
/// **The registered cross-seat reads, named as family 5(a) requires them to be:**
/// S4's IN-HAND branch and S7 read ALL seats' President locations — an
/// opponent's President bars a re-entry target and un-counts a reachable area.
/// So this is not the trivial statement that each feature reads one row; it is
/// the statement that the cross-seat reads follow the relabeling too.
///
/// The relabel is FULL: the two seats' whole rows trade places, and every other
/// `PlayerId` the state carries — track slot owners, achievement claimants, the
/// current player, the trigger player, the start player — is remapped with them,
/// so the permuted state is a genuinely relabeled table rather than a fixture
/// with half its references stale.
#[test]
fn group_s_values_permute_with_a_full_seat_relabel() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 129, &rules);
    let areas: Vec<AreaId> = state.board_layout.areas.keys().copied().collect();
    // A position where the two seats differ in EVERY input Group S reads, and
    // where seat 1's President blocks one of seat 0's areas (the cross-seat read).
    place_assistants(&mut state, 0, areas[0], 2);
    place_assistants(&mut state, 0, areas[1], 3);
    place_assistants(&mut state, 1, areas[2], 1);
    place_assistants(&mut state, 1, areas[3], 4);
    state.players[0].president_location = PresidentLocation::InHand;
    state.players[1].president_location = PresidentLocation::OnBoard(areas[1]);
    state.players[0].shophouses.built.insert(areas[0], 0);
    state.players[0].shophouses.in_warehouse -= 1;
    state.station_location = Some(areas[1]);

    let relabelled = relabel_seats_0_and_1(&state);
    let before = group_s_row(&state, &rules);
    let after = group_s_row(&relabelled, &rules);

    assert_ne!(before[0], before[1], "the two seats must differ, or the permutation is invisible");
    assert_eq!(after[0], before[1], "seat 0 of the relabeled table is the old seat 1");
    assert_eq!(after[1], before[0], "seat 1 of the relabeled table is the old seat 0");
    println!("seat permutation: {before:?} -> {after:?}");
}

/// The seven Group-S values of each seat, as read from that seat's OWN view.
fn group_s_row(state: &GameState, rules: &Rules) -> [[i64; 7]; 2] {
    let mut out = [[0i64; 7]; 2];
    for (seat, row) in out.iter_mut().enumerate() {
        *row = read_s(state, rules, PlayerId(seat as u8), |s, view, player| {
            [
                s.production_access(view, player),
                s.track_access(view, player),
                s.assistant_areas_distinct(player),
                s.president_local_access(player),
                s.bank_access(view, player),
                s.employment_agency_access(view, player),
                s.president_reachable_areas(player),
            ]
        });
    }
    out
}

/// Swap the LABELS of seats 0 and 1: their rows trade places and every
/// `PlayerId` the rest of the state names is remapped to match.
///
/// The row swap keeps `id` aligned with the index (`observe` addresses a seat by
/// index, so rows and ids cannot drift apart) — everything else about the player
/// travels, which is what makes this a relabeling rather than a field edit.
fn relabel_seats_0_and_1(state: &GameState) -> GameState {
    let swap = |p: PlayerId| match p.0 {
        0 => PlayerId(1),
        1 => PlayerId(0),
        _ => p,
    };
    let mut out = state.clone();
    out.players.swap(0, 1);
    out.players[0].id = PlayerId(0);
    out.players[1].id = PlayerId(1);
    for slot in out.church_track.slots.iter_mut().flatten() {
        slot.player = swap(slot.player);
    }
    for slot in out.customs_track.slots.iter_mut().flatten() {
        slot.player = swap(slot.player);
    }
    for card in out.achievements.iter_mut() {
        for claimant in card.claimed_by.iter_mut() {
            *claimant = swap(*claimant);
        }
    }
    out.current_player = swap(out.current_player);
    out.config.start_player = swap(out.config.start_player);
    out.trigger_player = out.trigger_player.map(swap);
    out
}

/// §7 audit part 2(b) for Group S: **CROSS-VIEW NEGATION at 2P**, on a fixture
/// of §5.2's constructible no-hidden-slot class (both starting orders
/// FULFILLED, so neither view carries a masked hand slot).
///
/// Two statements, and the first is the stronger one: every Group-S value is
/// IDENTICAL whichever seat's view it is read from — Group S reads nothing that
/// masking can touch. The registered identity, that the relative difference
/// negates across the two views, follows and is asserted in its own right
/// because it is the form §5.2(b) names.
#[test]
fn group_s_differences_negate_across_the_two_2p_views() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 131, &rules);
    // The two seats are given DIFFERENT area classes on purpose: without a
    // production tile and a Bank on one side and the tracks and the agency on
    // the other, four of the seven features read zero for both seats and the
    // negation identity holds for the wrong reason.
    let production = production_areas(&state);
    let bank = area_typed(&state, AreaType::Bank);
    let church = area_typed(&state, AreaType::Church);
    let agency = area_typed(&state, AreaType::EmploymentAgency);
    place_assistants(&mut state, 0, production[0], 2);
    place_assistants(&mut state, 0, production[1], 1);
    place_assistants(&mut state, 0, bank, 1);
    place_assistants(&mut state, 1, church, 2);
    place_assistants(&mut state, 1, agency, 2);
    state.players[0].president_location = PresidentLocation::OnBoard(production[0]);
    state.players[1].president_location = PresidentLocation::InHand;

    // The no-hidden-slot class: each seat's starting order moves from the hand
    // into the face-up completed pile, which is exactly what `observe` keys the
    // mask on ("Mask ONLY while unfulfilled").
    for p in state.players.iter_mut() {
        let card = p.starting_order.expect("the Keep shape records a starting order");
        p.order_cards_in_hand.retain(|c| *c != card);
        p.completed_orders.push(card);
    }

    let views = [
        observe(&state, PlayerId(0)).expect("seat 0 observes"),
        observe(&state, PlayerId(1)).expect("seat 1 observes"),
    ];
    for view in &views {
        for player in &view.players {
            assert!(
                !player.hand.iter().any(|c| matches!(c, HandCard::HiddenStartingOrder)),
                "the fixture must carry no masked slot: seat {:?}",
                player.id
            );
        }
    }

    let mut moved = 0usize;
    for (v, view) in views.iter().enumerate() {
        let structure = ScoreStructure::derive(view, &rules);
        let other = &views[1 - v];
        let other_structure = ScoreStructure::derive(other, &rules);
        for seat in 0..2u8 {
            let row = |view: &PlayerView| {
                view.players.iter().find(|p| p.id == PlayerId(seat)).expect("a row").clone()
            };
            let (mine, theirs) = (row(view), row(other));
            let here = [
                structure.production_access(view, &mine),
                structure.track_access(view, &mine),
                structure.assistant_areas_distinct(&mine),
                structure.president_local_access(&mine),
                structure.bank_access(view, &mine),
                structure.employment_agency_access(view, &mine),
                structure.president_reachable_areas(&mine),
            ];
            let there = [
                other_structure.production_access(other, &theirs),
                other_structure.track_access(other, &theirs),
                other_structure.assistant_areas_distinct(&theirs),
                other_structure.president_local_access(&theirs),
                other_structure.bank_access(other, &theirs),
                other_structure.employment_agency_access(other, &theirs),
                other_structure.president_reachable_areas(&theirs),
            ];
            assert_eq!(here, there, "seat {seat} reads differently from the two views");
        }
    }

    // The registered form: the relative difference negates.
    let rows = group_s_row(&state, &rules);
    for (f, (mine, theirs)) in rows[0].iter().zip(rows[1].iter()).enumerate() {
        let from_zero = mine - theirs;
        let from_one = theirs - mine;
        assert_eq!(from_zero, -from_one, "feature {f} does not negate across the views");
        if from_zero != 0 {
            moved += 1;
        }
    }
    assert!(moved >= 5, "only {moved} of the seven features separated the seats");
    println!("cross-view negation: {moved} of the seven Group-S features separated the seats");

    // THE DOMAIN PINS. Every cell above this one reads a feature on a fixture
    // built to exercise THAT feature, so a helper whose area mask had been
    // widened — S2 summing power over every occupied area rather than the two
    // tracks, S5 over every area rather than the Bank, S6 over every area
    // rather than the agency — would still pass all of them. This fixture is
    // the one place two seats hold DIFFERENT area classes at once (seat 0 the
    // production tiles and the Bank, seat 1 the Church and the agency), so
    // pinning both rows at their exact values is what makes a widened domain
    // visible: each widening moves a zero off zero.
    //
    // The constants are MEASURED, not derived — the board is a shuffle, so the
    // production types and the adjacency behind S1/S4/S7 are the seed's to
    // choose. The order is S1..S7.
    assert_eq!(
        rows[0],
        [4, 0, 3, 2, 2, 0, 0],
        "seat 0 holds production tiles and the Bank: S2 and S6 MUST be zero"
    );
    assert_eq!(
        rows[1],
        [0, 3, 2, 2, 0, 2, 2],
        "seat 1 holds the Church and the agency: S1 and S5 MUST be zero"
    );
}

/// §3.2's ALLOCATION CONTRACT, the Group-S half: every helper is
/// allocation-free.
///
/// The existing `score_allocates_nothing_and_explain_allocates` pin already
/// covers [`ScoreStructure::derive`]'s new view context — `score` derives once
/// per call, so the context rides that pin whether or not anything reads it.
/// This cell covers the READERS, which that pin cannot: the helpers are not on
/// `score`'s path until §7's step 8 moves `Feature::ALL`, so without this the
/// S7 walk could grow a `Vec` and no cell in the repo would notice.
///
/// 4P for the widest board and the deepest reachable set.
#[test]
fn every_group_s_helper_allocates_nothing() {
    let rules = Rules::standard();
    let mut state = keep_game(4, 133, &rules);
    // Placeable only: a 4P board carries a Canal, and a fixture that stood a
    // pawn on it would be measuring an unreachable position.
    let areas: Vec<AreaId> = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, c)| c.area_type.allows_placement())
        .map(|(a, _)| *a)
        .collect();
    for (i, area) in areas.iter().take(6).enumerate() {
        place_assistants(&mut state, 0, *area, if i == 0 { 3 } else { 1 });
    }
    state.players[0].president_location = PresidentLocation::OnBoard(areas[0]);
    state.players[1].president_location = PresidentLocation::OnBoard(areas[2]);

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let player = view.players.iter().find(|p| p.id == PlayerId(0)).expect("a row").clone();

    let (sum, stats) = meter::measure(|| {
        structure.production_access(&view, &player)
            + structure.track_access(&view, &player)
            + structure.assistant_areas_distinct(&player)
            + structure.president_local_access(&player)
            + structure.bank_access(&view, &player)
            + structure.employment_agency_access(&view, &player)
            + structure.president_reachable_areas(&player)
    });
    assert_eq!(stats.allocs, 0, "a Group-S helper allocated ({} allocations)", stats.allocs);
    assert!(sum > 0, "the fixture must exercise real values, or the meter measured nothing");
    println!("Group S: {sum} summed over 4P, {} allocs / {} peak bytes", stats.allocs, stats.peak_bytes);
}

/// §7 audit part 4's FIRST row, in the form this batch can execute:
/// `realizable_power` against the engine's own power function AT the fixtures
/// where the conditional and unconditional readings must coincide — the areas
/// where the seat's President already stands.
///
/// The organic oracle cell above compares against a president-RELOCATED clone at
/// every area; this one takes no clone at all, so it is the equality §3.2 states
/// in its own terms: "it keeps the conditional president term, so equality holds
/// only there".
///
/// # Two divergences, documented because §3.2 asks for them
///
/// * `driven::arriving_power` (`driven.rs:2063-2065`) is
///   `(calculate_power + 1).min(5)`, so where the President STANDS it adds the
///   term the engine already added — it agrees with neither reading there. That
///   divergence stays harness-side, exactly as §3.2 says.
/// * Away from the President, `calculate_power` is one lower by construction;
///   the cell counts those cases and floors them, so the equality above cannot
///   pass by the two readings being the same function.
///
/// # The second comparator, landed at the batch boundary
///
/// §7's contract names a SECOND comparator for this row: the harness's own
/// view-side mirror, `visible_power`. It was PRIVATE to `driven_families.rs`
/// and this batch's charter fenced every `driven*` file, so the S3-T1 batch
/// delivered this cell alone and recorded the cross-pin as owed forward. The
/// orchestrator landed the one-token unblock at the T1 boundary (the fence's
/// reason — the adjudicator lane — had merged), and the cross-pin now lives in
/// [`realizable_power_and_the_harness_mirror_agree_where_the_president_stands`]
/// directly below, which is the HARNESS mirror's first drift pin.
#[test]
fn realizable_power_equals_the_engines_reading_where_the_president_stands() {
    let rules = Rules::standard();
    let (mut coincident, mut divergent) = (0usize, 0usize);
    for (players, seed) in [(2u8, 135u64), (3, 137)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x1D1D);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..120usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(4) {
                for i in 0..players {
                    let seat = PlayerId(i);
                    let view = observe(&state, seat).expect("the seat observes");
                    let player = view.players.iter().find(|p| p.id == seat).expect("a row");
                    for area in view.board_layout.areas.keys().copied() {
                        let mine = realizable_power(&view, player, area);
                        let engine = calculate_power(&state, seat, area);
                        if player.president_location == PresidentLocation::OnBoard(area) {
                            assert_eq!(
                                mine, engine,
                                "{players}P seed {seed} step {step}: seat {i} at its own {area:?}"
                            );
                            coincident += 1;
                        } else if mine != engine {
                            divergent += 1;
                        }
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(coincident > 0, "no sampled area ever held the queried seat's President");
    assert!(divergent > 0, "the two readings never diverged — the equality above is vacuous");
    println!(
        "president-at-area equality: {coincident} coincident, {divergent} divergent elsewhere"
    );
}

/// **The §7 cross-pin, both halves exact**: `realizable_power` vs the harness's
/// own view-side mirror, `driven_families::visible_power` — landed at the S3-T1
/// boundary once the one-token `pub` unblock was made (the cell above records
/// the deferral).
///
/// §7 asks for equality "on president-at-the-queried-area fixtures" with the
/// divergence documented. This cell pins MORE than the ask, because the
/// divergence is not noise but an exact identity: `visible_power` keeps the
/// CONDITIONAL president term, so
///
/// * president at the area  ⇒ `realizable == visible` (both carry the +1), and
/// * president anywhere else ⇒ `realizable == min(5, visible + 1)` — the
///   unconditional term added to the mirror's reading, capped LAST exactly as
///   both functions cap.
///
/// Asserting the second half as an identity (rather than counting divergences)
/// means a drift in EITHER function's term list or cap placement fails the
/// walk at the first state it touches. Same organic walk as the cell above, so
/// the two pins sample the same distribution; both branches assert they fired.
#[test]
fn realizable_power_and_the_harness_mirror_agree_where_the_president_stands() {
    let rules = Rules::standard();
    let (mut at_area, mut elsewhere) = (0usize, 0usize);
    for (players, seed) in [(2u8, 135u64), (3, 137)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x2E2E);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..120usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(4) {
                for i in 0..players {
                    let seat = PlayerId(i);
                    let view = observe(&state, seat).expect("the seat observes");
                    let player = view.players.iter().find(|p| p.id == seat).expect("a row");
                    for area in view.board_layout.areas.keys().copied() {
                        let mine = realizable_power(&view, player, area);
                        let mirror = visible_power(&view, player, area);
                        if player.president_location == PresidentLocation::OnBoard(area) {
                            assert_eq!(
                                mine, mirror,
                                "{players}P seed {seed} step {step}: seat {i} at its own {area:?}"
                            );
                            at_area += 1;
                        } else {
                            assert_eq!(
                                mine,
                                (mirror + 1).min(5),
                                "{players}P seed {seed} step {step}: seat {i} away from {area:?}"
                            );
                            elsewhere += 1;
                        }
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(at_area > 0, "no sampled area ever held the queried seat's President");
    assert!(elsewhere > 0, "every sampled area held the President — the +1 half never fired");
    println!("harness-mirror cross-pin: {at_area} at-area, {elsewhere} elsewhere");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Groups E-resource and E-order: the order-card view context
// ---------------------------------------------------------------------------

/// One order card's face in O1's own HALF-VP units — `2·VP + 2·imports +
/// money` (§3.2's O1 card, whose doubling is what keeps the two amortized
/// terminal identities — 1 import ≡ 1 VP, 2¥ ≡ 1 VP — integer).
///
/// A FIXTURE reader, not the helper: it folds ONE printed card and knows
/// nothing about hands, pools or masked slots. Every expectation below is
/// built from it, so a cell states the manifest's arithmetic rather than a
/// number somebody typed.
fn half_vp_face(rules: &Rules, card: OrderCardId) -> i64 {
    let o = rules.orders.get(card).expect("a printed order card");
    i64::from(2 * u32::from(o.rewards.victory_points)
        + 2 * u32::from(o.rewards.imported_goods)
        + u32::from(o.rewards.money))
}

/// One order card's ICON faces — movement plus warehouse (§3.2's O6 card).
/// The same kind of reader as [`half_vp_face`], for the same reason.
fn icon_faces(rules: &Rules, card: OrderCardId) -> i64 {
    let o = rules.orders.get(card).expect("a printed order card");
    i64::from(u32::from(o.rewards.movement) + u32::from(o.rewards.warehouse_tokens))
}

/// One order card's printed requirements as `[copper, silk, tea, seafood]`.
fn requirement_vector(rules: &Rules, card: OrderCardId) -> [u32; 4] {
    let r = rules.orders.get(card).expect("a printed order card").requirements;
    [u32::from(r.copper), u32::from(r.silk), u32::from(r.tea), u32::from(r.seafood)]
}

/// One seat's public row inside `view`.
fn row_of(view: &PlayerView, seat: PlayerId) -> VisiblePlayer {
    view.players.iter().find(|p| p.id == seat).expect("the seat has a row").clone()
}

/// One Group-E helper, as a value — the shape §7's family-5(c) cell folds over
/// so its three features read through one loop rather than three copies of it.
type EReader = fn(&ScoreStructure, &VisiblePlayer, &Rules) -> i64;

/// The eight Group-E values of one row, in §3.2's LABEL order —
/// E1, E2, E3, O1, O2, O3, O5, O6 (the order `CandidateFeature` declares, so a
/// printed row here reads against the fixture package's own axis).
fn e_values(structure: &ScoreStructure, player: &VisiblePlayer, rules: &Rules) -> [i64; 8] {
    [
        structure.money_stock(player),
        structure.trade_goods_stock(player),
        structure.building_tokens_ready(player),
        structure.held_order_value(player, rules),
        structure.order_matched_goods(player, rules),
        structure.executable_orders(player, rules),
        structure.flag_icon_marginal(player, rules),
        structure.held_order_icons(player, rules),
    ]
}

/// The eight Group-E values of each seat, as read from that seat's OWN view.
fn group_e_row(state: &GameState, rules: &Rules) -> [[i64; 8]; 2] {
    let mut out = [[0i64; 8]; 2];
    for (seat, row) in out.iter_mut().enumerate() {
        *row = read_s(state, rules, PlayerId(seat as u8), |s, _, p| e_values(s, p, rules));
    }
    out
}

/// A Keep game with every ORDER-CARD site EMPTIED — the port display, every
/// hand, every completed pile, and the boxed setup discards — and all
/// thirty-six cards gathered into the deck, so a fixture below states exactly
/// which card sits where instead of inheriting a shuffle.
///
/// `starting_order` is cleared with the hand, deliberately: the field records a
/// card the seat can SEE, and O1's pool locates it for that reason, so leaving
/// it pointing at a card this helper has just pushed back into the deck would
/// make the fixture claim a deck card is located — a state no `observe` can
/// produce. The two helpers below put a starting order back, in whichever of
/// the two registered conditions a cell needs.
fn cleared_order_table(players: u8, seed: u64, rules: &Rules) -> GameState {
    let mut state = keep_game(players, seed, rules);
    let mut gathered: Vec<OrderCardId> = std::mem::take(&mut state.orders_removed);
    for slot in state.port_state.port_a.slots.iter_mut() {
        if let Some(id) = slot.take() {
            gathered.push(id);
        }
    }
    if let Some(b) = state.port_state.port_b.as_mut() {
        for slot in b.slots.iter_mut() {
            if let Some(id) = slot.take() {
                gathered.push(id);
            }
        }
    }
    for p in state.players.iter_mut() {
        gathered.append(&mut p.order_cards_in_hand);
        gathered.append(&mut p.completed_orders);
        p.starting_order = None;
    }
    state.order_deck.extend(gathered);
    assert_eq!(state.order_deck.len(), 36, "every printed card is now free");
    state
}

/// Take `card` out of the only sites a fixture may take one from — the deck,
/// the boxed discards, or the port display. A card in a hand or a completed
/// pile is NOT free: it is already somebody's, and a cell that moved it would
/// be rewriting the position rather than building one.
fn take_free_card(state: &mut GameState, card: OrderCardId) {
    if let Some(i) = state.order_deck.iter().position(|c| *c == card) {
        state.order_deck.remove(i);
        return;
    }
    if let Some(i) = state.orders_removed.iter().position(|c| *c == card) {
        state.orders_removed.remove(i);
        return;
    }
    for slot in state.port_state.port_a.slots.iter_mut() {
        if *slot == Some(card) {
            *slot = None;
            return;
        }
    }
    if let Some(b) = state.port_state.port_b.as_mut() {
        for slot in b.slots.iter_mut() {
            if *slot == Some(card) {
                *slot = None;
                return;
            }
        }
    }
    panic!("{card:?} is in no free site — it is already in a hand or a completed pile");
}

/// Set `seat`'s four trade-good stocks OUTRIGHT, as `[copper, silk, tea,
/// seafood]`. Every seat starts with ONE of each good, so a fixture that set
/// only the two it cares about would be measuring those two plus a setup
/// constant — which is exactly how an E2 expectation drifts two off.
fn set_stocks(state: &mut GameState, seat: usize, stocks: [u32; 4]) {
    let p = &mut state.players[seat];
    p.copper = stocks[0];
    p.silk = stocks[1];
    p.tea = stocks[2];
    p.seafood = stocks[3];
}

/// Empty both lab displays. The labs are a TECHNOLOGY site, so
/// [`cleared_order_table`] leaves them alone; an O5 fixture that wants its
/// attainable set to be the order sources alone has to say so, because an open
/// lab slot is a flag on the table like any other.
fn clear_labs(state: &mut GameState) {
    for slot in state.lab_state.lab_a.slots.iter_mut() {
        *slot = None;
    }
    if let Some(b) = state.lab_state.lab_b.as_mut() {
        for slot in b.slots.iter_mut() {
            *slot = None;
        }
    }
}

/// Move `card` into `seat`'s hand.
fn deal_card(state: &mut GameState, seat: usize, card: OrderCardId) {
    take_free_card(state, card);
    state.players[seat].order_cards_in_hand.push(card);
}

/// Move `card` into `seat`'s face-up completed pile.
fn complete_card(state: &mut GameState, seat: usize, card: OrderCardId) {
    take_free_card(state, card);
    state.players[seat].completed_orders.push(card);
}

/// Move `card` onto port A's slot `index`.
fn offer_card(state: &mut GameState, index: usize, card: OrderCardId) {
    take_free_card(state, card);
    assert!(state.port_state.port_a.slots[index].is_none(), "slot {index} is already offered");
    state.port_state.port_a.slots[index] = Some(card);
}

/// Give `seat` a still-UNFULFILLED starting order: the card in the hand with
/// the residual field naming it — the one state `observe` masks from every
/// other seat ("Mask ONLY while unfulfilled", `observe.rs:169-171`).
fn unfulfilled_starting_order(state: &mut GameState, seat: usize, card: OrderCardId) {
    deal_card(state, seat, card);
    state.players[seat].starting_order = Some(card);
}

/// Give `seat` a FULFILLED starting order: the card face-up in the completed
/// pile with the residual field still naming it. §5.2(b)'s constructible
/// no-hidden-slot class, built rather than played into.
fn fulfilled_starting_order(state: &mut GameState, seat: usize, card: OrderCardId) {
    complete_card(state, seat, card);
    state.players[seat].starting_order = Some(card);
}

/// Replace `seat`'s still-unfulfilled starting order with `card`, which must be
/// FREE; the displaced card takes the substitute's exact place.
///
/// Card counts, hand length, deck length and the hand INDEX of the masked slot
/// are all preserved, so two states built this way differ in exactly one thing
/// — the hidden identity — and in nothing an opponent's view can name.
fn substitute_starting_order(state: &GameState, seat: usize, card: OrderCardId) -> GameState {
    let mut out = state.clone();
    let old = out.players[seat].starting_order.expect("the seat holds a starting order");
    let hand_at = out.players[seat]
        .order_cards_in_hand
        .iter()
        .position(|c| *c == old)
        .expect("the starting order is still unfulfilled");
    if let Some(i) = out.order_deck.iter().position(|c| *c == card) {
        out.order_deck[i] = old;
    } else {
        take_free_card(&mut out, card);
        out.order_deck.push(old);
    }
    out.players[seat].order_cards_in_hand[hand_at] = card;
    out.players[seat].starting_order = Some(card);
    out
}

/// §7 audit part 4 for O1's POOL, in its strongest available form: the
/// allocation-free located bitset and its two accumulators against
/// `PlayerView::unseen_orders` — the crate's INDEPENDENT authority on what a
/// seat cannot locate, written for the belief sampler and tested in its own
/// module.
///
/// Three statements, and the first is the one that matters: the pool is the
/// registry MINUS the located set, as a SET and not merely as a count. A
/// derivation that located the right NUMBER of wrong cards would pass a count
/// check and fail here.
///
/// The two readings agree even though they are built differently — this one
/// adds the viewer's own `starting_order`, which `unseen_orders` does not. That
/// term is redundant on every view `observe` can produce (a kept starting order
/// sits in the hand until it is fulfilled and in `completed_orders` after), and
/// §3.2's O1 card names it anyway for the live-tracking case where a seat's own
/// deal was recorded before its hand was reconciled. This cell is what keeps
/// the redundancy true.
#[test]
fn view_context_locates_every_order_the_table_shows_and_pools_the_rest() {
    let rules = Rules::standard();
    let registry: OrderSet = rules.orders.iter().map(|c| order_bit(c.id)).fold(0, |a, b| a | b);
    assert_eq!(registry.count_ones(), 36, "the printed manifest is thirty-six order cards");

    let (mut samples, mut masked_slots, mut widest_pool) = (0usize, 0usize, 0u32);
    for (players, seed) in [(2u8, 201u64), (3, 203), (4, 205)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x0117);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..80usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(8) {
                for i in 0..players {
                    let seat = PlayerId(i);
                    let view = observe(&state, seat).expect("the seat observes");
                    let context = ScoreStructure::derive(&view, &rules).context;
                    let unseen = view.unseen_orders(&rules);
                    let mut expected: OrderSet = 0;
                    for id in &unseen {
                        expected |= order_bit(*id);
                    }
                    assert_eq!(
                        registry & !context.located_orders,
                        expected,
                        "{players}P seed {seed} step {step} seat {i}: the pooled SET"
                    );
                    assert_eq!(
                        context.unseen_orders as usize,
                        unseen.len(),
                        "{players}P seed {seed} step {step} seat {i}: the pool count"
                    );
                    let halfvp: i64 = unseen.iter().map(|id| half_vp_face(&rules, *id)).sum();
                    assert_eq!(
                        context.unseen_order_halfvp, halfvp,
                        "{players}P seed {seed} step {step} seat {i}: the pooled half-VP sum"
                    );
                    assert!(
                        context.located_orders.count_ones() > 0,
                        "the port display alone locates cards; a zero located set is a dead scan"
                    );
                    masked_slots += view.hidden_slot_count();
                    widest_pool = widest_pool.max(context.unseen_orders);
                    samples += 1;
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(samples > 0, "the sweep sampled nothing");
    assert!(masked_slots > 0, "no sampled view carried a masked slot — O1's pool is never read");
    println!("order-card context: {samples} samples, {masked_slots} masked slots, widest pool {widest_pool}");
}

/// The pool's FLOOR — O1's reading for a masked slot — and its boundary: an
/// empty pool reads 0 rather than dividing by zero.
///
/// The organic arm brackets the floor two-sidedly (`count·floor ≤ sum <
/// count·(floor+1)`), which is the statement "integer division of the pool's
/// mean" makes without restating the division.
#[test]
fn the_unseen_pool_floor_is_the_pools_mean_and_zero_when_nothing_is_unseen() {
    let rules = Rules::standard();
    let view = observe(&keep_game(2, 207, &rules), PlayerId(0)).expect("seat 0 observes");
    let context = ScoreStructure::derive(&view, &rules).context;
    let (count, sum, floor) =
        (i64::from(context.unseen_orders), context.unseen_order_halfvp, context.unseen_order_floor());
    assert!(count > 0, "an opening 2P table cannot have located every card");
    assert_eq!(floor, sum / count, "the floor IS the pool's integer mean");
    assert!(count * floor <= sum, "the floor is not above the mean");
    assert!(sum < count * (floor + 1), "the floor is the LARGEST such integer");
    println!("opening 2P pool: {count} cards, {sum} half-VP, floor {floor}");

    // THE BOUNDARY: every printed card located. Reachable only by construction
    // — a real table always has a deck — and the answer must be 0, not a panic.
    let mut state = cleared_order_table(2, 209, &rules);
    let all: Vec<OrderCardId> = rules.orders.iter().map(|c| c.id).collect();
    state.order_deck.clear();
    state.orders_removed.clear();
    state.players[0].completed_orders = all;
    let empty = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let empty_context = ScoreStructure::derive(&empty, &rules).context;
    assert_eq!(empty_context.unseen_orders, 0, "the fixture locates all thirty-six");
    assert_eq!(empty_context.unseen_order_halfvp, 0, "an empty pool carries no value");
    assert_eq!(empty_context.unseen_order_floor(), 0, "an empty pool's floor is 0");
}

/// §3.2's O1 card names the pool's fourth term as **the VIEWER's OWN starting
/// order**, and this is the cell that makes the word "own" load-bearing.
///
/// # Why it needs its own fixture, and why no other cell can carry it
///
/// `observe` fills `starting_order` for no seat but the viewer
/// (`observe.rs:197`), so on every view the engine can produce, "the viewer's
/// row" and "every row" are the SAME SET and the two readings agree exactly.
/// Every other cell in this file reaches the evaluator through `observe`, which
/// means a widened read is invisible to all of them — the narrowing would rest
/// on the code and a comment alone.
///
/// The class the card cites is the one built here: a HAND-BUILT view carrying an
/// opponent-row `starting_order`. The edit is in-idiom for this file (§5.2(c)'s
/// cell edits a row's `hand` the same way), and it is precisely the shape a
/// live-tracking or fixture author could produce by accident. A widened read
/// would locate a card that seat is not entitled to point at, shrinking the pool
/// and moving the masked opponent's O1 through the floor.
///
/// # Anti-vacuity, permanent rather than by mutation record
///
/// The planted card is chosen to be one the table CANNOT already point at AND
/// one whose removal from the pool actually MOVES the floor — searched for, with
/// the search asserted to succeed. So the pins below cannot pass by the
/// counterfactual being identical; the closing assertion states that
/// counterfactual as arithmetic. The masked slot is asserted present too, since
/// without one O1 never reads the pool at all.
///
/// MUTATION-PROVEN at authoring: re-widening the derivation to
/// `p.id == view.viewer || true` turns exactly this cell red (message in the
/// batch report) and leaves the other 82 green. The mutant was never committed.
#[test]
fn the_pool_locates_the_viewers_own_starting_order_and_no_other_seats() {
    let rules = Rules::standard();
    let state = keep_game(2, 265, &rules);
    let honest = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let honest_structure = ScoreStructure::derive(&honest, &rules);
    let honest_context = honest_structure.context;
    let floor = honest_context.unseen_order_floor();

    assert!(
        row_of(&honest, PlayerId(1)).hand.contains(&HandCard::HiddenStartingOrder),
        "the opponent must carry a masked slot, or O1 never reads the pool"
    );

    // A card the table cannot point at, AND one whose removal moves the floor.
    //
    // `iter_sorted_by_id`, NOT `iter`: the registry is `HashMap`-backed and its
    // iteration order is process-randomized, so a `find` over `iter` would plant
    // a DIFFERENT card on every run — the cell would still pass, and its printed
    // identity would be irreproducible, which is the silent failure
    // `observe.rs:220-222` names in exactly these words.
    let planted = rules
        .orders
        .iter_sorted_by_id()
        .into_iter()
        .map(|c| c.id)
        .filter(|id| honest_context.located_orders & order_bit(*id) == 0)
        .find(|id| {
            let count = i64::from(honest_context.unseen_orders) - 1;
            let sum = honest_context.unseen_order_halfvp - half_vp_face(&rules, *id);
            count > 0 && sum / count != floor
        })
        .expect("some unlocated card's removal would move the pool floor");

    let mut widened = honest.clone();
    let opponent =
        widened.players.iter_mut().find(|p| p.id == PlayerId(1)).expect("the opponent's row");
    assert_eq!(
        opponent.starting_order, None,
        "`observe` fills this field for no seat but the viewer — the fixture PLANTS it"
    );
    opponent.starting_order = Some(planted);

    let widened_structure = ScoreStructure::derive(&widened, &rules);
    let widened_context = widened_structure.context;
    assert_eq!(
        widened_context.located_orders, honest_context.located_orders,
        "an opponent-row `starting_order` located a card the viewer cannot see"
    );
    assert_eq!(widened_context.unseen_orders, honest_context.unseen_orders, "the pool shrank");
    assert_eq!(
        widened_context.unseen_order_halfvp, honest_context.unseen_order_halfvp,
        "the pool's value moved"
    );
    assert_eq!(widened_context.unseen_order_floor(), floor, "the floor moved");

    // BOTH rows, because the pool is table-wide: a widened read moves the masked
    // seat's reading through the floor and leaves the viewer's alone, so pinning
    // both says which is which rather than only that something is unmoved.
    for seat in [PlayerId(0), PlayerId(1)] {
        assert_eq!(
            widened_structure.held_order_value(&row_of(&widened, seat), &rules),
            honest_structure.held_order_value(&row_of(&honest, seat), &rules),
            "seat {seat:?}'s O1 moved with a field that seat is not entitled to read"
        );
    }

    // THE COUNTERFACTUAL, as arithmetic: had the planted card been located, the
    // pins above would have failed. Without this the cell could pass by the
    // fixture being unable to tell the two readings apart.
    let would_count = i64::from(honest_context.unseen_orders) - 1;
    let would_sum = honest_context.unseen_order_halfvp - half_vp_face(&rules, planted);
    assert_ne!(would_sum / would_count, floor, "the planted card's removal must move the floor");
    println!(
        "viewer-only pool: planted {planted:?} (face {}), floor {floor} vs the widened {}",
        half_vp_face(&rules, planted),
        would_sum / would_count
    );
}

/// §7 audit part 4 for O5's SOURCES: the attainable-country scan is exactly the
/// seat's KNOWN hand, the face-up port offers, and the open lab titles the seat
/// does not already own by EFFECT equality — the engine's own ownership test
/// (`actions/cards.rs:268-270`).
///
/// The fixture makes all four clauses bite at once and makes the ownership
/// clause PER SEAT: seat 0 owns one copy of the two-copy Gaslight title, so the
/// lab's other Gaslight copy is unattainable for seat 0 and attainable for
/// seat 1. Two seats reading DIFFERENT sets off one table is what separates a
/// per-seat ownership test from a table-wide one.
#[test]
fn view_context_attainable_countries_are_the_hand_the_port_and_the_unowned_lab_titles() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 211, &rules);
    clear_labs(&mut state);
    for p in state.players.iter_mut() {
        p.technologies.clear();
    }

    // The two Gaslight copies (French) — the rulebook's one-per-title rule is
    // what makes a second copy unattainable to a seat already holding one.
    let gaslight: Vec<TechCardId> = rules
        .techs
        .iter()
        .filter(|t| t.effect == TechEffect::Gaslight)
        .map(|t| t.id)
        .collect();
    assert_eq!(gaslight.len(), 2, "Gaslight circulates as two physical copies");
    let brickyard = rules
        .techs
        .iter()
        .find(|t| t.effect == TechEffect::Brickyard)
        .expect("the German Brickyard")
        .id;

    deal_card(&mut state, 0, OrderCardId(3)); // English
    offer_card(&mut state, 0, OrderCardId(1)); // American
    state.lab_state.lab_a.slots[0] = Some(brickyard); // German, unowned by both
    state.lab_state.lab_a.slots[1] = Some(gaslight[1]); // French
    state.players[0].technologies.push(gaslight[0]); // ... the OTHER copy

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let context = ScoreStructure::derive(&view, &rules).context;
    assert_eq!(
        context.attainable(0),
        country_bit(Country::English) | country_bit(Country::American) | country_bit(Country::German),
        "seat 0: its own English hand card, the American offer, the unowned German title — and \
         NOT the French copy of a title it already owns"
    );
    assert_eq!(
        context.attainable(1),
        country_bit(Country::American) | country_bit(Country::German) | country_bit(Country::French),
        "seat 1 holds no card and owns no technology: the offer and BOTH lab titles, and not \
         seat 0's hand card"
    );
    assert_eq!(
        context.attainable(0) & country_bit(Country::Dutch),
        0,
        "no source shows a Dutch flag — the scan must not invent one"
    );

    // THE ROUND TRIP, pinned locally. O5 walks a mask back into countries to
    // ask `set_bonus` what each one buys, so `country_bit` and the walk's
    // inverse have to agree — a permuted inverse would price the wrong flag's
    // step. Two O5 cells below catch that today, but only as a value they
    // happen to disagree about; this says it directly.
    const EVERY_COUNTRY: [Country; 5] = [
        Country::English,
        Country::American,
        Country::French,
        Country::German,
        Country::Dutch,
    ];
    let all: CountrySet = EVERY_COUNTRY.iter().copied().map(country_bit).fold(0, |a, b| a | b);
    assert_eq!(all.count_ones(), 5, "five flags, five distinct bits");
    for country in EVERY_COUNTRY {
        let one: Vec<Country> = countries_in(country_bit(country)).collect();
        assert_eq!(one, vec![country], "a one-flag mask walks back to that flag");
    }
    assert_eq!(countries_in(all).count(), 5, "and the full mask walks back to all five");
    assert_eq!(countries_in(0).count(), 0, "the empty mask walks back to nothing");
}

/// §13's `set_bonus` refactor: ONE derivation, both callers.
///
/// The printed ladder is pinned directly (2/4/8/12 for 2/3/4/5 distinct
/// countries, and a set forms as often as the icons allow), and then the
/// INCUMBENT identity: `CountryBonusNow` over organic play equals `set_bonus`
/// applied to the icon multiset rebuilt here from the two face-up piles. That
/// second half is what makes the extraction a refactor rather than a second
/// implementation — the incumbent feature and O5's marginal now read the same
/// function, and a divergence fails here.
#[test]
fn set_bonus_is_the_one_derivation_behind_the_country_bonus_and_the_flag_marginal() {
    assert_eq!(set_bonus([0, 0, 0, 0, 0]), 0, "no icons");
    assert_eq!(set_bonus([1, 0, 0, 0, 0]), 0, "one distinct country scores nothing");
    assert_eq!(set_bonus([4, 0, 0, 0, 0]), 0, "four of ONE country is still one country");
    assert_eq!(set_bonus([1, 1, 0, 0, 0]), 2, "two distinct");
    assert_eq!(set_bonus([1, 1, 1, 0, 0]), 4, "three distinct");
    assert_eq!(set_bonus([1, 1, 1, 1, 0]), 8, "four distinct");
    assert_eq!(set_bonus([1, 1, 1, 1, 1]), 12, "five distinct");
    assert_eq!(set_bonus([2, 2, 2, 2, 2]), 24, "two full sets");
    assert_eq!(set_bonus([2, 1, 1, 0, 0]), 4, "the second round has one country left");
    assert_eq!(set_bonus([3, 3, 1, 0, 0]), 4 + 2 + 2, "a three-set, then two two-sets");

    let rules = Rules::standard();
    let mut compared = 0usize;
    let mut nonzero = 0usize;
    for (players, seed) in [(2u8, 213u64), (3, 215)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x0B05);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for step in 0..90usize {
            if is_game_over(&state) {
                break;
            }
            if step.is_multiple_of(6) {
                let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
                for player in &view.players {
                    let mut icons = [0u8; 5];
                    for id in &player.completed_orders {
                        let c = rules.orders.get(*id).expect("a printed card").country;
                        icons[country_slot(c)] += 1;
                    }
                    for id in &player.technologies {
                        let c = rules.techs.get(*id).expect("a printed card").country;
                        icons[country_slot(c)] += 1;
                    }
                    let incumbent =
                        raw_value(&view, player, Feature::CountryBonusNow, &rules);
                    assert_eq!(incumbent, set_bonus(icons), "one derivation, both callers");
                    compared += 1;
                    if incumbent > 0 {
                        nonzero += 1;
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(compared > 0, "the sweep compared nothing");

    // The organic arm is the DRIFT pin and it is where the two derivations meet
    // under real play; it is not where the identity is exercised at a nonzero
    // value, because a rung game rarely completes orders of two countries and a
    // sweep that HAPPENED to would be a flaky floor. The constructed arm carries
    // that half: icons out of BOTH face-up piles, across the whole ladder.
    let mut state = cleared_order_table(2, 217, &rules);
    for p in state.players.iter_mut() {
        p.technologies.clear();
    }
    let english_tech = rules
        .techs
        .iter()
        .find(|t| t.country == Country::English)
        .expect("an English technology")
        .id;
    let steps = [
        (vec![OrderCardId(3)], false, 0),                                   // one country
        (vec![OrderCardId(3), OrderCardId(1)], false, 2),                   // two
        (vec![OrderCardId(3), OrderCardId(1), OrderCardId(2)], false, 4),   // three
        (vec![OrderCardId(3), OrderCardId(1), OrderCardId(2)], true, 4),    // + a 2nd English
        (
            vec![OrderCardId(3), OrderCardId(1), OrderCardId(2), OrderCardId(4)],
            false,
            8,
        ), // four
        (
            vec![OrderCardId(3), OrderCardId(1), OrderCardId(2), OrderCardId(4), OrderCardId(6)],
            false,
            12,
        ), // five
    ];
    for (completed, owns_tech, expected) in steps {
        let mut fixture = state.clone();
        for card in &completed {
            complete_card(&mut fixture, 0, *card);
        }
        if owns_tech {
            fixture.players[0].technologies.push(english_tech);
        }
        let read = read_s(&fixture, &rules, PlayerId(0), |s, _, p| {
            s.value_of(p, Feature::CountryBonusNow)
        });
        assert_eq!(read, expected, "the incumbent ladder at {completed:?} (tech: {owns_tech})");
        compared += 1;
        if read > 0 {
            nonzero += 1;
        }
    }
    assert!(nonzero >= 5, "only {nonzero} comparisons were nonzero — the identity is vacuous");
    println!("set_bonus identity: {compared} rows compared, {nonzero} nonzero");
}

/// The five-wide icon array's slot for a country, restated on the TEST side so
/// the identity above is a comparison of two independent derivations rather
/// than of one function with itself. The loop `set_bonus` runs is symmetric in
/// the countries, so any bijection serves.
fn country_slot(country: Country) -> usize {
    match country {
        Country::English => 0,
        Country::American => 1,
        Country::French => 2,
        Country::German => 3,
        Country::Dutch => 4,
    }
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Group E-resource (E1, E2, E3)
// ---------------------------------------------------------------------------

/// **E1 `MoneyStock`** — the purse, linear, and §3.2's boundary: every spend
/// class debits it AT THE INSTANT PAID (the F4 repair).
///
/// The boundary is read off organic play rather than a constructed
/// subtraction, and the claim is exactly as wide as the cell: E1 equals the
/// purse at EVERY sampled state of two rung games, and the purse is observed to
/// move in both directions, so "at the instant paid" is witnessed rather than
/// asserted.
///
/// **What this cell deliberately does NOT do is enumerate the spend CLASSES**
/// §3.2's E1 paragraph lists (placement and movement fees, the church 2¥ leg,
/// the lab supplements, Employment-Agency costs, the port's second card): the
/// counters below say a debit happened, not which class produced it. A
/// per-class cell would be vacuous here in a way it is not for the access
/// terms — E1 IS `player.money`, so every class debits the same field by
/// construction, and a fixture per class would be re-testing the engine's own
/// pricing under this feature's name. What could break — E1 reading a stale
/// copy, or a different field — is what the equality across every row catches.
#[test]
fn money_stock_is_the_purse_and_moves_by_exactly_what_a_spend_debits() {
    let rules = Rules::standard();
    let (mut debits, mut credits, mut samples) = (0usize, 0usize, 0usize);
    for (players, seed) in [(2u8, 217u64), (3, 219)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x0E1E);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        let mut previous: Option<Vec<i64>> = None;
        for _ in 0..120usize {
            if is_game_over(&state) {
                break;
            }
            let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
            let structure = ScoreStructure::derive(&view, &rules);
            let now: Vec<i64> = view
                .players
                .iter()
                .map(|p| {
                    let e1 = structure.money_stock(p);
                    assert_eq!(e1, i64::from(p.money), "E1 is the purse, exactly");
                    e1
                })
                .collect();
            if let Some(before) = &previous {
                for (a, b) in before.iter().zip(now.iter()) {
                    if b < a {
                        debits += 1;
                    } else if b > a {
                        credits += 1;
                    }
                }
            }
            previous = Some(now);
            samples += 1;
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(samples > 0, "the sweep sampled nothing");
    assert!(debits > 0, "no seat ever paid — the boundary half is unwitnessed");
    assert!(credits > 0, "no seat was ever paid — the fixture is not a real economy");
    println!("E1: {samples} samples, {debits} debits, {credits} credits");
}

/// **E2 `TradeGoodsStock`** — the four-good sum, linear, over organic play.
#[test]
fn trade_goods_stock_is_the_four_good_sum_over_organic_play() {
    let rules = Rules::standard();
    let (mut samples, mut nonzero) = (0usize, 0usize);
    let mut state = keep_game(2, 221, &rules);
    let mut rng = ChaCha8Rng::seed_from_u64(0x0E2E);
    let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
    for _ in 0..120usize {
        if is_game_over(&state) {
            break;
        }
        let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
        let structure = ScoreStructure::derive(&view, &rules);
        for p in &view.players {
            let expected = i64::from(p.copper) + i64::from(p.silk) + i64::from(p.tea) + i64::from(p.seafood);
            assert_eq!(structure.trade_goods_stock(p), expected, "E2 is the four-good sum");
            samples += 1;
            if expected > 0 {
                nonzero += 1;
            }
        }
        let mv = {
            let handle = DecisionHandle::new(&state, &rules);
            policy.pick(&handle, &mut rng).expect("the rung always finds a move")
        };
        state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
    }
    assert!(samples > 0, "the sweep sampled nothing");
    assert!(nonzero > 0, "every reading was 0 — the sum is never exercised");
    println!("E2: {samples} rows, {nonzero} nonzero");
}

/// **E2's boundary twin** — "fulfillment consumes exactly the printed
/// requirements before rewards land" (§3.2's E2 card).
///
/// Constructed at the instant, because that is what the boundary IS: the drop
/// equals the card's own `requirements.total()` read off the registry, not a
/// number this cell chooses.
#[test]
fn trade_goods_stock_drops_by_exactly_the_printed_requirements_at_a_fulfillment() {
    let rules = Rules::standard();
    const FULFILLED: OrderCardId = OrderCardId(27); // English, requires 2 copper + 2 seafood
    let mut state = cleared_order_table(2, 223, &rules);
    deal_card(&mut state, 0, FULFILLED);
    deal_card(&mut state, 0, OrderCardId(3));
    set_stocks(&mut state, 0, [3, 0, 0, 2]);

    let before = read_s(&state, &rules, PlayerId(0), |s, _, p| s.trade_goods_stock(p));
    let requirements = requirement_vector(&rules, FULFILLED);
    let total: i64 = requirements.iter().map(|n| i64::from(*n)).sum();
    assert_eq!(total, 4, "card 27's printed requirement total");

    // The instant: the goods leave, the card joins the face-up pile.
    state.players[0].copper -= requirements[0];
    state.players[0].silk -= requirements[1];
    state.players[0].tea -= requirements[2];
    state.players[0].seafood -= requirements[3];
    state.players[0].order_cards_in_hand.retain(|c| *c != FULFILLED);
    state.players[0].completed_orders.push(FULFILLED);

    let after = read_s(&state, &rules, PlayerId(0), |s, _, p| s.trade_goods_stock(p));
    assert_eq!(before, 5, "three copper and two seafood");
    assert_eq!(before - after, total, "E2 drops by exactly the printed requirements");
}

/// §7 audit part 4 for **E3 `BuildingTokensReady`** — the two IN-HAND token
/// counts and nothing else.
///
/// The widening control is the point: warehouse stock is S6's visible supply
/// and built tokens are the incumbents', so moving a token between those two
/// sites must leave E3 unmoved. Without that half, an E3 that summed the whole
/// token pool would pass a fixture that only ever varied `in_hand`.
#[test]
fn building_tokens_ready_is_exactly_the_two_in_hand_token_counts() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 225, &rules);
    let area = production_areas(&state)[0];
    for (shop, trade) in [(0u8, 0u8), (2, 0), (0, 3), (2, 3), (1, 1)] {
        state.players[0].shophouses.in_hand = shop;
        state.players[0].trading_houses.in_hand = trade;
        let e3 = read_s(&state, &rules, PlayerId(0), |s, _, p| s.building_tokens_ready(p));
        assert_eq!(e3, i64::from(shop) + i64::from(trade), "E3 is the two in-hand counts");
    }
    // The partition control: neither the warehouse nor the built sites move it.
    state.players[0].shophouses.in_hand = 1;
    state.players[0].trading_houses.in_hand = 1;
    let base = read_s(&state, &rules, PlayerId(0), |s, _, p| s.building_tokens_ready(p));
    state.players[0].shophouses.in_warehouse += 3;
    state.players[0].trading_houses.in_warehouse += 2;
    state.players[0].trading_houses.built.push(area);
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.building_tokens_ready(p)),
        base,
        "warehouse stock is S6's and built tokens are the incumbents' — E3 reads neither"
    );
}

/// **E3's boundary twin** — "the FREE build realizes the token into
/// `ShophousesBuilt`/`TradingHousesBuilt`" (§3.2's E3 card, and the reason the
/// term is owner-ruled IN: without it an EA retrieval trades S6-visible
/// warehouse stock for nothing until the build lands).
#[test]
fn building_tokens_ready_hands_the_token_to_the_built_incumbent_at_a_free_build() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 227, &rules);
    let area = production_areas(&state)[0];
    state.players[0].shophouses.in_hand = 2;
    state.players[0].trading_houses.in_hand = 1;

    let read = |state: &GameState| {
        let view = observe(state, PlayerId(0)).expect("seat 0 observes");
        let structure = ScoreStructure::derive(&view, &rules);
        let row = row_of(&view, PlayerId(0));
        (
            structure.building_tokens_ready(&row),
            structure.value_of(&row, Feature::ShophousesBuilt),
        )
    };
    let (e3_before, built_before) = read(&state);
    assert_eq!(e3_before, 3, "two shophouses and one trading house, all in hand");

    // The build: the token leaves the hand for the board, free.
    state.players[0].shophouses.in_hand -= 1;
    state.players[0].shophouses.built.insert(area, 0);

    let (e3_after, built_after) = read(&state);
    assert_eq!(e3_after, e3_before - 1, "E3 gives the token up");
    assert_eq!(built_after, built_before + 1, "the incumbent takes it — no double count");
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Group E-order (O1, O2, O3, O5, O6)
// ---------------------------------------------------------------------------

/// **O1 `HeldOrderValue`** over a fully KNOWN hand: the summed HALF-VP face,
/// `2·VP + 2·imports + money` per card, read off the registry.
///
/// The three cards are chosen so all three terms are live — one pure-VP, one
/// carrying money, one carrying an import — because a helper that dropped the
/// money or the import term would still match a pure-VP hand.
#[test]
fn held_order_value_is_the_half_vp_face_of_every_known_card() {
    let rules = Rules::standard();
    const PURE_VP: OrderCardId = OrderCardId(21); // 11 VP, no money, no import
    const WITH_MONEY: OrderCardId = OrderCardId(27); // 1 VP + 2 money
    const WITH_IMPORT: OrderCardId = OrderCardId(5); // 1 VP + 1 import
    assert_eq!(half_vp_face(&rules, PURE_VP), 22);
    assert_eq!(half_vp_face(&rules, WITH_MONEY), 4);
    assert_eq!(half_vp_face(&rules, WITH_IMPORT), 4);

    let mut state = cleared_order_table(2, 229, &rules);
    for card in [PURE_VP, WITH_MONEY, WITH_IMPORT] {
        deal_card(&mut state, 0, card);
    }
    let o1 = read_s(&state, &rules, PlayerId(0), |s, _, p| s.held_order_value(p, &rules));
    assert_eq!(o1, 22 + 4 + 4, "the summed half-VP face");

    // An empty hand is 0, not the pool floor: only a MASKED slot reads the pool.
    state.players[0].order_cards_in_hand.clear();
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.held_order_value(p, &rules)),
        0,
        "an empty hand holds no value"
    );
}

/// **O1's registered epistemic asymmetry**: an opponent's `HiddenStartingOrder`
/// slot is priced at the FLOOR of the unseen pool's mean, and the rest of that
/// hand at its true face.
#[test]
fn held_order_value_prices_a_masked_slot_at_the_unseen_pool_floor() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 231, &rules);
    unfulfilled_starting_order(&mut state, 1, OrderCardId(27));
    deal_card(&mut state, 1, OrderCardId(21)); // a known card beside the masked one

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let opponent = row_of(&view, PlayerId(1));
    assert_eq!(
        opponent.hand.iter().filter(|c| **c == HandCard::HiddenStartingOrder).count(),
        1,
        "the fixture must carry exactly one masked slot"
    );
    let known: i64 = opponent
        .hand
        .iter()
        .filter_map(|c| match c {
            HandCard::Known(id) => Some(half_vp_face(&rules, *id)),
            HandCard::HiddenStartingOrder => None,
        })
        .sum();
    let floor = structure.context.unseen_order_floor();
    assert!(floor > 0, "the opening pool is rich enough for the floor to be visible");
    assert_eq!(
        structure.held_order_value(&opponent, &rules),
        known + floor,
        "the known faces plus ONE pool floor for the masked slot"
    );
}

/// §7 audit part 1's Group-O half: **O1's SUBSTITUTION INVARIANCE.**
///
/// Three states differing ONLY in the identity of the opponent's hidden
/// starting order (two substitutions off the dealt card) read identical O1 from
/// the opposite seat — for BOTH rows, since the pool is table-wide.
///
/// # Why the cell is not vacuous, permanently
///
/// Two controls ride here rather than only in a one-off mutation:
///
/// * the substituted cards' TRUE faces are asserted to take at least two
///   distinct values, at least one of them different from the pool floor — so a
///   reader that peeked at the hidden identity WOULD separate these states; and
/// * the SAME substitution is asserted to MOVE the opponent's own reading of
///   itself, which is the registered epistemic asymmetry stated as a contrast:
///   the card is not hidden from its holder, and O1 says so.
///
/// The view equality asserted first is the honesty precondition — it is what
/// makes "differing only in the hidden identity" a checked claim about the
/// fixture rather than a description of it.
///
/// # MUTATION-PROVEN at authoring, both directions, message observed
///
/// A helper here CANNOT peek — no argument it takes carries a hidden identity,
/// which is the type-level half of §7 part 1 and is pinned separately by
/// [`every_group_e_helper_takes_only_a_public_row_and_the_registries`]. So the
/// mutation is applied to the READER this cell uses, which is where the
/// distinction between a view-honest evaluator and a state-peeking one lives:
///
///   - MUTANT (the masked slot valued at the substituted card's TRUE face,
///     read off the state): FAILED — `assertion left == right failed: O1 moved
///     with an identity seat 0 cannot see; left: [0, 8]  right: [0, 22]` (8 and
///     22 being the substitute's and the dealt card's faces).
///   - RESTORED (through `held_order_value`): passes, printing
///     `faces [22, 8, 4], floor 12, opposite [0, 12], own [22, 8, 4]` — the
///     opposite seat's reading pinned at the floor while the holder's own
///     tracks every substituted face.
///
/// A second mutation, on the HELPER (a masked slot valued 0 rather than at the
/// floor), leaves THIS cell green — 0 is substitution-invariant too — and is
/// caught by four sibling cells, which is the division of labour intended: this
/// one says the value cannot depend on the hidden identity, and the floor cells
/// say what the value is.
#[test]
fn held_order_value_is_invariant_under_substitution_of_the_hidden_identity() {
    let rules = Rules::standard();
    let base = {
        let mut state = cleared_order_table(2, 233, &rules);
        unfulfilled_starting_order(&mut state, 1, OrderCardId(21));
        state
    };
    let dealt = base.players[1].starting_order.expect("the fixture records one");
    // Two substitutes from the DECK, so nothing an opponent can see moves.
    let substitutes: Vec<OrderCardId> = base
        .order_deck
        .iter()
        .copied()
        .filter(|c| half_vp_face(&rules, *c) != half_vp_face(&rules, dealt))
        .take(2)
        .collect();
    assert_eq!(substitutes.len(), 2, "the deck must offer two differently-faced substitutes");

    let states: Vec<GameState> = std::iter::once(base.clone())
        .chain(substitutes.iter().map(|c| substitute_starting_order(&base, 1, *c)))
        .collect();
    let faces: Vec<i64> = states
        .iter()
        .map(|s| half_vp_face(&rules, s.players[1].starting_order.expect("a starting order")))
        .collect();
    assert!(
        faces.iter().collect::<std::collections::BTreeSet<_>>().len() >= 2,
        "the substitutions must change the TRUE face, or a peeking reader would agree too"
    );

    let mut opposite: Vec<[i64; 2]> = Vec::new();
    let mut own: Vec<i64> = Vec::new();
    for state in &states {
        let view = observe(state, PlayerId(0)).expect("seat 0 observes");
        let structure = ScoreStructure::derive(&view, &rules);
        opposite.push([
            structure.held_order_value(&row_of(&view, PlayerId(0)), &rules),
            structure.held_order_value(&row_of(&view, PlayerId(1)), &rules),
        ]);
        let mine = observe(state, PlayerId(1)).expect("seat 1 observes");
        let my_structure = ScoreStructure::derive(&mine, &rules);
        own.push(my_structure.held_order_value(&row_of(&mine, PlayerId(1)), &rules));
    }

    let first = observe(&states[0], PlayerId(0)).expect("seat 0 observes");
    for state in &states[1..] {
        assert_eq!(
            observe(state, PlayerId(0)).expect("seat 0 observes"),
            first,
            "the substitution must be invisible from the other seat"
        );
    }
    for row in &opposite[1..] {
        assert_eq!(*row, opposite[0], "O1 moved with an identity seat 0 cannot see");
    }
    assert!(
        own.iter().collect::<std::collections::BTreeSet<_>>().len() >= 2,
        "the holder's OWN reading did not move — the fixture proves nothing about masking"
    );
    let floor = ScoreStructure::derive(&first, &rules).context.unseen_order_floor();
    assert!(faces.iter().any(|f| *f != floor), "every true face equalled the floor by accident");
    println!("O1 substitution: faces {faces:?}, floor {floor}, opposite {:?}, own {own:?}", opposite[0]);
}

/// **O1's boundary twin**: the helper iterates the hand WITHOUT assuming its
/// length. The rules cap a hand at three, and a LIVE-TRACKING view can carry
/// FOUR — the synthesized `HiddenStartingOrder` slot ahead of three
/// publicly-witnessed Port takes (`observe.rs:160-166`).
#[test]
fn held_order_value_iterates_a_four_card_live_tracking_hand() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 235, &rules);
    // The live `Unknown` fiction on seat 1: the dealt card is in NEITHER
    // storage site, so it sits in the removed pile and the slot is synthesized
    // AHEAD of the witnessed takes rather than replacing one of them.
    take_free_card(&mut state, OrderCardId(2));
    state.players[1].starting_order_hidden = true;
    state.orders_removed.push(OrderCardId(2));
    let known = [OrderCardId(21), OrderCardId(27), OrderCardId(5)];
    for card in known {
        deal_card(&mut state, 1, card);
    }

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let opponent = row_of(&view, PlayerId(1));
    assert_eq!(opponent.hand.len(), 4, "one masked slot plus three witnessed takes");
    let expected: i64 = known.iter().map(|c| half_vp_face(&rules, *c)).sum::<i64>()
        + structure.context.unseen_order_floor();
    assert_eq!(
        structure.held_order_value(&opponent, &rules),
        expected,
        "all four entries priced — three faces and one floor"
    );
}

/// §5.2 family 5(a)'s NAMED cross-seat read: **O1's pool is p-INDEPENDENT** —
/// it reads all located cards, so every masked slot at a table reads the SAME
/// floor regardless of which seat's row is being valued.
///
/// 3P, where two opponents carry masked slots at once; the two rows are given
/// different KNOWN halves so the equality below is not two identical rows
/// agreeing with themselves.
#[test]
fn held_order_value_reads_one_table_wide_pool_for_every_seat() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(3, 237, &rules);
    unfulfilled_starting_order(&mut state, 1, OrderCardId(27));
    unfulfilled_starting_order(&mut state, 2, OrderCardId(26));
    deal_card(&mut state, 1, OrderCardId(21));

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let floor = structure.context.unseen_order_floor();
    let mut residuals = Vec::new();
    for seat in [PlayerId(1), PlayerId(2)] {
        let row = row_of(&view, seat);
        let masked = row.hand.iter().filter(|c| **c == HandCard::HiddenStartingOrder).count();
        assert_eq!(masked, 1, "seat {seat:?} must carry exactly one masked slot");
        let known: i64 = row
            .hand
            .iter()
            .filter_map(|c| match c {
                HandCard::Known(id) => Some(half_vp_face(&rules, *id)),
                HandCard::HiddenStartingOrder => None,
            })
            .sum();
        residuals.push(structure.held_order_value(&row, &rules) - known);
    }
    assert_ne!(
        row_of(&view, PlayerId(1)).hand.len(),
        row_of(&view, PlayerId(2)).hand.len(),
        "the two rows must differ, or the shared floor is invisible"
    );
    assert_eq!(residuals[0], floor, "seat 1's masked slot reads the table's pool");
    assert_eq!(residuals[1], floor, "and so does seat 2's — the pool is not seat-relative");
}

/// §7 audit part 4 for **O2 `OrderMatchedGoods`**: `Σ_g min(stock_g,
/// total_req_g)` over KNOWN held orders — the cap is per good and SUMMED across
/// the hand, which is what makes `O2 ≤ E2` true.
///
/// The fixture discriminates the summed cap from a per-card one: two cards each
/// needing two copper against a stock of three read `min(3, 4) = 3`, where a
/// per-card reading would double-count to `2 + 2 = 4`.
#[test]
fn order_matched_goods_caps_each_good_against_the_summed_requirement() {
    let rules = Rules::standard();
    const A: OrderCardId = OrderCardId(3); // 2 copper, 1 seafood
    const B: OrderCardId = OrderCardId(27); // 2 copper, 2 seafood
    assert_eq!(requirement_vector(&rules, A), [2, 0, 0, 1]);
    assert_eq!(requirement_vector(&rules, B), [2, 0, 0, 2]);

    let mut state = cleared_order_table(2, 239, &rules);
    deal_card(&mut state, 0, A);
    deal_card(&mut state, 0, B);
    set_stocks(&mut state, 0, [3, 0, 0, 1]);

    let (o2, e2) = read_s(&state, &rules, PlayerId(0), |s, _, p| {
        (s.order_matched_goods(p, &rules), s.trade_goods_stock(p))
    });
    assert_eq!(o2, 4, "min(3, 4) copper + min(1, 3) seafood — the SUMMED cap, not 2 + 2 + 1");
    assert_eq!(e2, 4, "at these stocks every good is matched and O2 meets its ceiling");

    // Hidden slots contribute NOTHING to the requirement sum: seat 1 holds a
    // masked card that needs two copper and nine copper to spend on it, and
    // seat 0 still reads 0, because it can see no requirement to match.
    let mut masked = cleared_order_table(2, 239, &rules);
    unfulfilled_starting_order(&mut masked, 1, B);
    masked.players[1].copper = 9;
    let view = observe(&masked, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    assert_eq!(
        structure.order_matched_goods(&row_of(&view, PlayerId(1)), &rules),
        0,
        "a masked slot names no requirement — O2 reads 0 for it"
    );
}

/// §3.2's declared partition duty `{O2, X2, E2}` in its checkable half:
/// **O2 ≤ E2 everywhere**, because each good's contribution is capped by that
/// good's own stock.
#[test]
fn order_matched_goods_never_exceeds_the_trade_goods_stock() {
    let rules = Rules::standard();
    let (mut rows, mut strict, mut equal) = (0usize, 0usize, 0usize);
    for (players, seed) in [(2u8, 241u64), (3, 243)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x0022);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for _ in 0..120usize {
            if is_game_over(&state) {
                break;
            }
            for i in 0..players {
                let view = observe(&state, PlayerId(i)).expect("the seat observes");
                let structure = ScoreStructure::derive(&view, &rules);
                for p in &view.players {
                    let (o2, e2) = (structure.order_matched_goods(p, &rules), structure.trade_goods_stock(p));
                    assert!(o2 <= e2, "O2 {o2} exceeded E2 {e2} — the per-good cap leaked");
                    rows += 1;
                    if e2 > 0 && o2 < e2 {
                        strict += 1;
                    } else if e2 > 0 && o2 == e2 {
                        equal += 1;
                    }
                }
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
        }
    }
    assert!(rows > 0, "the sweep sampled nothing");
    assert!(strict > 0, "the inequality was never strict at a nonzero stock");
    assert!(equal > 0, "the inequality was never tight — the bound is never approached");
    println!("O2 <= E2: {rows} rows, {strict} strict, {equal} tight");
}

/// §7 audit part 4 for **O3 `ExecutableOrders`**: the count of KNOWN held
/// orders whose four requirements the stock meets, INDEPENDENTLY — and
/// imports and money never substitute for a trade good.
#[test]
fn executable_orders_counts_the_hands_whose_four_requirements_the_stock_meets() {
    let rules = Rules::standard();
    const MET: OrderCardId = OrderCardId(3); // 2 copper, 1 seafood
    const SHORT: OrderCardId = OrderCardId(27); // 2 copper, 2 seafood
    let mut state = cleared_order_table(2, 245, &rules);
    deal_card(&mut state, 0, MET);
    deal_card(&mut state, 0, SHORT);
    set_stocks(&mut state, 0, [3, 0, 0, 1]);

    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.executable_orders(p, &rules)),
        1,
        "one card met, one a seafood short"
    );

    // Imports and money NEVER substitute (§3.2's O3 card).
    state.players[0].money = 99;
    state.players[0].imported_goods = 99;
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.executable_orders(p, &rules)),
        1,
        "a full purse buys no trade good"
    );

    // The count is INDEPENDENT, not a joint allocation: one seafood serves both
    // cards' arithmetic separately, which is what "uncapped multi-fulfil" means
    // for a static read.
    state.players[0].seafood = 2;
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.executable_orders(p, &rules)),
        2,
        "both cards independently met at three copper and two seafood"
    );
}

/// §5.2 family 6(a)'s row for the order group: **fulfillment RETIRES the card's
/// contribution** to O2, O3 and O6 at the instant the rewards land.
///
/// One instant, three features, and both mechanisms visible: the card leaves the
/// hand (O6's icons and the requirement sum go with it) AND the goods it
/// consumed leave the stock (which is what takes the OTHER card out of O3 too).
#[test]
fn fulfillment_retires_the_cards_contribution_to_o2_o3_and_o6() {
    let rules = Rules::standard();
    const FULFILLED: OrderCardId = OrderCardId(27); // 2 copper + 2 seafood, one warehouse icon
    const KEPT: OrderCardId = OrderCardId(3); // 2 copper + 1 seafood, icon-free
    assert_eq!(icon_faces(&rules, FULFILLED), 1, "card 27 carries one warehouse icon");
    assert_eq!(icon_faces(&rules, KEPT), 0, "card 3 is icon-free");

    let mut state = cleared_order_table(2, 247, &rules);
    deal_card(&mut state, 0, FULFILLED);
    deal_card(&mut state, 0, KEPT);
    set_stocks(&mut state, 0, [3, 0, 0, 2]);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, p| {
            [
                s.order_matched_goods(p, &rules),
                s.executable_orders(p, &rules),
                s.held_order_icons(p, &rules),
            ]
        })
    };
    let before = read(&state);
    assert_eq!(before, [5, 2, 1], "min(3,4) copper + min(2,3) seafood; both executable; one icon");

    let requirements = requirement_vector(&rules, FULFILLED);
    state.players[0].copper -= requirements[0];
    state.players[0].seafood -= requirements[3];
    state.players[0].order_cards_in_hand.retain(|c| *c != FULFILLED);
    state.players[0].completed_orders.push(FULFILLED);

    let after = read(&state);
    assert_eq!(after, [1, 0, 0], "one copper against the kept card's two; nothing executable; no icons");
    assert!(after.iter().zip(before.iter()).all(|(a, b)| a < b), "all three retired");
}

/// §7 audit part 4 for **O5 `FlagIconMarginal`**: the `set_bonus` STEP the best
/// attainable country buys, over `icons(seat)` = the country multiset of
/// `completed_orders + technologies`.
///
/// Both arms of the max fire: a country the seat already holds buys nothing,
/// the missing fourth buys the 4→8 step, and the helper returns the larger.
#[test]
fn flag_icon_marginal_is_the_set_bonus_step_the_best_attainable_country_buys() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 249, &rules);
    clear_labs(&mut state);
    state.players[0].technologies.clear();
    // Three distinct realized countries: English, American, French.
    for card in [OrderCardId(3), OrderCardId(1), OrderCardId(2)] {
        complete_card(&mut state, 0, card);
    }
    let base = read_s(&state, &rules, PlayerId(0), |s, _, p| {
        s.value_of(p, Feature::CountryBonusNow)
    });
    assert_eq!(base, 4, "three distinct countries");

    // A country already held: the marginal is 0.
    offer_card(&mut state, 0, OrderCardId(10)); // English
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.flag_icon_marginal(p, &rules)),
        0,
        "a fourth English icon adds no distinct country"
    );

    // The missing fourth: 4 -> 8.
    offer_card(&mut state, 1, OrderCardId(4)); // German
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.flag_icon_marginal(p, &rules)),
        4,
        "the MAX over the attainable set takes the German step, not the English zero"
    );
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| {
            s.flag_icon_marginal(p, &rules) + s.value_of(p, Feature::CountryBonusNow)
        }),
        8,
        "the step lands exactly where the incumbent would read it"
    );
}

/// §7's NAMED boundary twin for O5: **0 on an empty attainable set** — and it
/// is a real zero, not an absent one. The seat here holds four distinct
/// countries, so a fifth would be worth 4; with no hand, no offer and no open
/// lab title there is nowhere to get one, and the term reads 0.
#[test]
fn flag_icon_marginal_is_zero_on_an_empty_attainable_set() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 251, &rules);
    clear_labs(&mut state);
    state.players[0].technologies.clear();
    for card in [OrderCardId(3), OrderCardId(1), OrderCardId(2), OrderCardId(4)] {
        complete_card(&mut state, 0, card);
    }

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let row = row_of(&view, PlayerId(0));
    assert_eq!(structure.context.attainable(0), 0, "no source shows any flag");
    assert_eq!(structure.value_of(&row, Feature::CountryBonusNow), 8, "four distinct countries");
    assert_eq!(structure.flag_icon_marginal(&row, &rules), 0, "nowhere to get the fifth");

    // The CONTROL: one Dutch offer, and the same position reads the 8 -> 12 step.
    offer_card(&mut state, 0, OrderCardId(6)); // Dutch
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.flag_icon_marginal(p, &rules)),
        4,
        "the zero above was the empty SET, not a missing step"
    );
}

/// **O5's boundary twin**: realization moves the step into `CountryBonusNow`
/// (§5.2 families 6(c)/4(e)), and the marginal collapses to 0 with the source
/// still standing — so the collapse is the CURVATURE's truth, not the source
/// disappearing.
#[test]
fn flag_icon_marginal_realizes_into_the_country_bonus_when_the_icon_lands() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 253, &rules);
    clear_labs(&mut state);
    state.players[0].technologies.clear();
    for card in [OrderCardId(3), OrderCardId(1), OrderCardId(2), OrderCardId(4)] {
        complete_card(&mut state, 0, card);
    }
    deal_card(&mut state, 0, OrderCardId(8)); // Dutch, and it STAYS in hand

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, _, p| {
            (s.flag_icon_marginal(p, &rules), s.value_of(p, Feature::CountryBonusNow))
        })
    };
    let (o5_before, country_before) = read(&state);
    assert_eq!((o5_before, country_before), (4, 8), "the fifth country is one step away");

    // A DIFFERENT Dutch card completes; the hand card that made Dutch
    // attainable is untouched.
    complete_card(&mut state, 0, OrderCardId(6));
    let (o5_after, country_after) = read(&state);
    assert_eq!(country_after - country_before, o5_before, "the step landed where O5 priced it");
    assert_eq!(o5_after, 0, "post-set shrinkage: a sixth flag of five countries buys nothing");
    assert_ne!(
        read_s(&state, &rules, PlayerId(0), |s, _, _| s.context.attainable(0)),
        0,
        "the Dutch source is still on the table — the collapse is curvature, not absence"
    );
}

/// §7 audit part 4 for **O6 `HeldOrderIcons`**: the manifest's icon census, and
/// the summed faces over a KNOWN hand in ICON units.
///
/// The census is a GENERATED-REGISTRY pin (`game/data.rs` is emitted by
/// `gen-data` from the YAML manifest): §3.2 states 2/36 movement and 5/36
/// warehouse, and a manifest edit that moved either number would land here
/// rather than silently re-scale a fitted weight.
#[test]
fn held_order_icons_matches_the_generated_manifests_icon_census() {
    let rules = Rules::standard();
    let movement: Vec<OrderCardId> =
        rules.orders.iter().filter(|c| c.rewards.movement > 0).map(|c| c.id).collect();
    let warehouse: Vec<OrderCardId> =
        rules.orders.iter().filter(|c| c.rewards.warehouse_tokens > 0).map(|c| c.id).collect();
    assert_eq!(movement.len(), 2, "§3.2's census: two of thirty-six carry a movement face");
    assert_eq!(warehouse.len(), 5, "§3.2's census: five of thirty-six carry a warehouse face");
    assert!(
        rules.orders.iter().all(|c| c.rewards.any_trade_good == 0),
        "the any-good face is structurally unreachable from an order (§3.1)"
    );

    const MOVES: OrderCardId = OrderCardId(10); // movement 2
    const STORES: OrderCardId = OrderCardId(27); // warehouse 1
    assert_eq!(icon_faces(&rules, MOVES), 2);
    assert_eq!(icon_faces(&rules, STORES), 1);

    let mut state = cleared_order_table(2, 255, &rules);
    deal_card(&mut state, 0, MOVES);
    deal_card(&mut state, 0, STORES);
    deal_card(&mut state, 0, OrderCardId(3)); // icon-free
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.held_order_icons(p, &rules)),
        3,
        "movement and warehouse faces summed in ICON units, never VP-mapped"
    );

    // Hidden slots contribute 0 — even a masked card that DOES carry a face.
    let mut masked = cleared_order_table(2, 255, &rules);
    unfulfilled_starting_order(&mut masked, 1, OrderCardId(31)); // movement 2
    let view = observe(&masked, PlayerId(0)).expect("seat 0 observes");
    assert_eq!(
        ScoreStructure::derive(&view, &rules).held_order_icons(&row_of(&view, PlayerId(1)), &rules),
        0,
        "a masked slot shows no face"
    );
}

// ---------------------------------------------------------------------------
// V4 §7 audit contract — Group E, the cross-cutting parts
// ---------------------------------------------------------------------------

/// The Group-E FEATURE BODIES (E-resource and E-order both), by source-text
/// head — the subject of the two scans below, hand-maintained for the same
/// reason [`S_HELPER_HEADS`] is: a ninth helper not added here is a helper
/// neither scan covers.
///
/// The same four kinds of code sit beneath these as beneath the S list, with
/// the same guards: `ScoreStructure::derive`'s order-card pass is pinned
/// VALUE-wise against `PlayerView::unseen_orders`, `set_bonus` is pinned
/// against the incumbent country bonus, and `order_bit` / `country_bit` /
/// `seat_index` are index arithmetic with no player row in scope.
const E_HELPER_HEADS: [&str; 8] = [
    "    pub fn money_stock(",
    "    pub fn trade_goods_stock(",
    "    pub fn building_tokens_ready(",
    "    pub fn held_order_value(",
    "    pub fn order_matched_goods(",
    "    pub fn executable_orders(",
    "    pub fn flag_icon_marginal(",
    "    pub fn held_order_icons(",
];

/// §7 audit part 1 for Group E: **view honesty, at the type level.**
///
/// Every Group-E helper's parameter list names ONLY a seat's public row and the
/// static registries. Not one takes a `&PlayerView` — the group needs no board,
/// no station token and no track — so the honesty claim here is even narrower
/// than Group S's, and readable off the signature.
#[test]
fn every_group_e_helper_takes_only_a_public_row_and_the_registries() {
    const ALLOWED_PARAM_TYPES: [&str; 2] = ["&VisiblePlayer", "&Rules"];
    for head in E_HELPER_HEADS {
        let source = s_helper_source(head);
        let open = source.find('(').expect("a declaration has an argument list");
        let close = source.find(')').expect("a declaration closes its argument list");
        let params = &source[open + 1..close];
        let mut named = 0usize;
        for (i, param) in params.split(',').map(str::trim).filter(|p| !p.is_empty()).enumerate() {
            if i == 0 {
                assert_eq!(param, "&self", "`{head}` must be a method on the structure");
                continue;
            }
            let ty = param.split(": ").nth(1).unwrap_or_else(|| panic!("`{param}` has no type"));
            assert!(
                ALLOWED_PARAM_TYPES.contains(&ty),
                "`{head}` takes `{ty}`, which is not one of {ALLOWED_PARAM_TYPES:?} — a Group-E \
                 helper is a function of a seat's PUBLIC ROW and the printed registries"
            );
            named += 1;
        }
        assert!(named > 0, "`{head}` names no argument beyond `&self`");
    }
}

/// §7's cross-group cleanliness assert, E half: **no E helper reads power or
/// presence.**
///
/// # The scan's scope, stated because it is a decision
///
/// The needles are the POWER/PRESENCE ones, and they are applied to all EIGHT
/// helpers — §3.2's Group E-resource AND Group E-order, which §7's sentence
/// ("no E helper reads power/presence") covers together. The S-half needles
/// (money, goods, order cards) are deliberately NOT scanned here: those fields
/// are exactly what this group exists to price, and scanning for them would
/// assert the opposite of the spec. The third clause — "only X reads both" —
/// belongs to the batch that builds Group X, which is the only group that can
/// make it true or false.
///
/// ANTI-VACUITY: every needle is asserted to be FINDABLE in the Group-S bodies
/// and `realizable_power`, which read all of them — so a scan that had stopped
/// matching anything fails here rather than passing everywhere.
#[test]
fn no_group_e_helper_reads_power_or_presence() {
    for head in E_HELPER_HEADS {
        let source = s_helper_source(head);
        for needle in SPATIAL_READS {
            assert!(
                !source.contains(needle),
                "`{head}` reads `{needle}` — Group E prices money, goods and the order \
                 pipeline; power and presence belong to Group S, and only Group X reads both"
            );
        }
    }
    // The needles are real reads somewhere, or this scan proves nothing.
    let control: String = S_HELPER_HEADS
        .iter()
        .chain(std::iter::once(&"pub fn realizable_power("))
        .map(|h| s_helper_source(h))
        .collect::<Vec<_>>()
        .join("\n");
    for needle in SPATIAL_READS {
        assert!(
            control.contains(needle),
            "the control corpus no longer contains `{needle}` — the scan has stopped being \
             able to find a real read"
        );
    }
}

/// §7 audit part 2(a) for Group E: **INDEX-PERMUTATION INVARIANCE** under a FULL
/// seat relabel (§5.2 family 5(a)).
///
/// **The registered cross-seat read, named as family 5(a) requires:** O1's pool
/// reads ALL located cards — every seat's completed pile, every seat's known
/// hand, and the port display — so O1 is emphatically not a function of one
/// row. It is p-INDEPENDENT rather than seat-local, which is why relabeling
/// must leave the pool alone while the rows trade places.
#[test]
fn group_e_values_permute_with_a_full_seat_relabel() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 257, &rules);
    deal_card(&mut state, 0, OrderCardId(10)); // English, movement 2
    deal_card(&mut state, 0, OrderCardId(3)); // English, 2 copper + 1 seafood
    deal_card(&mut state, 1, OrderCardId(21)); // Dutch, pure VP
    offer_card(&mut state, 0, OrderCardId(4)); // German offer, an O5 source
    state.players[0].money = 7;
    set_stocks(&mut state, 0, [3, 0, 0, 1]);
    state.players[0].shophouses.in_hand = 2;
    state.players[0].trading_houses.in_hand = 1;
    complete_card(&mut state, 0, OrderCardId(1));
    complete_card(&mut state, 0, OrderCardId(2));
    state.players[1].money = 1;
    set_stocks(&mut state, 1, [0, 1, 2, 0]);
    state.players[1].shophouses.in_hand = 0;
    state.players[1].trading_houses.in_hand = 0;
    complete_card(&mut state, 1, OrderCardId(5));

    let relabelled = relabel_seats_0_and_1(&state);
    let before = group_e_row(&state, &rules);
    let after = group_e_row(&relabelled, &rules);

    assert_ne!(before[0], before[1], "the two seats must differ, or the permutation is invisible");
    assert_eq!(after[0], before[1], "seat 0 of the relabeled table is the old seat 1");
    assert_eq!(after[1], before[0], "seat 1 of the relabeled table is the old seat 0");

    // The pool is a property of the TABLE: relabeling the seats cannot move it.
    let pool = |state: &GameState| {
        let view = observe(state, PlayerId(0)).expect("seat 0 observes");
        let c = ScoreStructure::derive(&view, &rules).context;
        (c.located_orders, c.unseen_orders, c.unseen_order_halfvp)
    };
    assert_eq!(pool(&state), pool(&relabelled), "O1's pool is p-independent");
    println!("seat permutation: {before:?} -> {after:?}");
}

/// §7 audit part 2(b) for Group E: **CROSS-VIEW NEGATION at 2P**, on §5.2's
/// constructible no-hidden-slot class — both starting orders FULFILLED, so
/// neither view carries a masked slot and every Group-E value is a function of
/// public state alone.
///
/// Two statements, the first the stronger: with nothing masked, every value is
/// IDENTICAL whichever seat's view it is read from. The registered identity —
/// that the relative difference negates — follows, and is asserted in its own
/// right because it is the form §5.2(b) names.
#[test]
fn group_e_differences_negate_across_the_two_2p_views() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 259, &rules);
    // The no-hidden-slot class: each seat's starting order sits FULFILLED in
    // the face-up completed pile, the residual field still naming it.
    clear_labs(&mut state); // the attainable sets are ORDER sources alone here
    fulfilled_starting_order(&mut state, 0, OrderCardId(1)); // American
    fulfilled_starting_order(&mut state, 1, OrderCardId(4)); // German
    deal_card(&mut state, 0, OrderCardId(10)); // English, movement 2
    deal_card(&mut state, 0, OrderCardId(3)); // English, 2 copper + 1 seafood
    deal_card(&mut state, 1, OrderCardId(36)); // German — a country seat 1 HAS
    offer_card(&mut state, 0, OrderCardId(25)); // a German offer, the shared O5 source
    state.players[0].money = 7;
    set_stocks(&mut state, 0, [3, 0, 0, 1]);
    state.players[0].shophouses.in_hand = 2;
    state.players[0].trading_houses.in_hand = 1;
    state.players[1].money = 1;
    set_stocks(&mut state, 1, [0, 0, 2, 0]);
    state.players[1].shophouses.in_hand = 0;
    state.players[1].trading_houses.in_hand = 0;

    let views = [
        observe(&state, PlayerId(0)).expect("seat 0 observes"),
        observe(&state, PlayerId(1)).expect("seat 1 observes"),
    ];
    for view in &views {
        for player in &view.players {
            assert!(
                !player.hand.iter().any(|c| matches!(c, HandCard::HiddenStartingOrder)),
                "the fixture must carry no masked slot: seat {:?}",
                player.id
            );
        }
    }

    for (v, view) in views.iter().enumerate() {
        let structure = ScoreStructure::derive(view, &rules);
        let other = &views[1 - v];
        let other_structure = ScoreStructure::derive(other, &rules);
        for seat in 0..2u8 {
            let here = e_values(&structure, &row_of(view, PlayerId(seat)), &rules);
            let there = e_values(&other_structure, &row_of(other, PlayerId(seat)), &rules);
            assert_eq!(here, there, "seat {seat} reads differently from the two views");
        }
    }

    let rows = group_e_row(&state, &rules);
    let mut moved = 0usize;
    for (f, (mine, theirs)) in rows[0].iter().zip(rows[1].iter()).enumerate() {
        assert_eq!(mine - theirs, -(theirs - mine), "feature {f} does not negate across the views");
        if mine != theirs {
            moved += 1;
        }
    }
    assert_eq!(moved, 8, "only {moved} of the eight features separated the seats");
    println!("cross-view negation: {moved} of eight separated; rows {rows:?}");

    // THE FIELD PINS. Every cell above this one reads a feature on a fixture
    // built to exercise THAT feature, so a helper reading the WRONG FIELD of
    // the right kind — E2 summing the warehouse, E3 the built tokens, O2 the
    // stock instead of the match, O6 the VP star instead of the icon — would
    // still pass all of them. This is the one fixture where both seats hold
    // different values of all eight at once, so pinning both rows exactly is
    // what makes a mis-wired field visible.
    //
    // Every constant is DERIVED from the printed manifest, and the derivation
    // is the comment beside it — the board is a shuffle, but nothing in Group E
    // reads the board. The order is E1, E2, E3, O1, O2, O3, O5, O6.
    assert_eq!(
        rows[0],
        [
            7,  // E1: the purse
            4,  // E2: 3 copper + 1 seafood
            3,  // E3: two shophouse tokens and one trading-house token in hand
            17, // O1: card 10's face 8 + card 3's face 9
            4,  // O2: min(3, 4) copper + min(0, 1) tea + min(1, 2) seafood
            1,  // O3: card 3 met; card 10 wants a tea the seat has not got
            2,  // O5: one American icon, and either attainable flag makes a pair
            2,  // O6: card 10's two movement faces, card 3 icon-free
        ],
        "seat 0 holds two English cards against three copper and one seafood"
    );
    assert_eq!(
        rows[1],
        [
            1,  // E1
            2,  // E2: two tea
            0,  // E3: no token in hand — the warehouse holds them, and E3 is not S6
            12, // O1: card 36's face 2·5 + 2
            2,  // O2: min(2, 2) tea; the four silk it also wants are not held
            0,  // O3: four silk short
            0,  // O5: German realized, German attainable — one country is no pair
            1,  // O6: card 36's one warehouse face
        ],
        "seat 1 holds one German card in a German position: its O5 step is CLOSED"
    );
}

/// §7 audit part 2(c): the **DECLARED-ASYMMETRY pins** for O1, O2, O3 and O6 —
/// every hidden-slot-0 or Known-only reader. The per-feature cross-view gap
/// equals EXACTLY the masked slot's own term, and nothing else moves.
///
/// The term is computed independently, by evaluating the same helper on the
/// holder's OWN row with the masked card REMOVED from the hand. So the identity
/// is not "the difference is some difference" but "the masking view reads
/// exactly what the holder would read without that card" — and for O1, exactly
/// that plus one pool floor.
///
/// O5 is deliberately NOT in this list, and §5.2's own (c) clause leaves it out
/// for the reason the arithmetic gives: O5 is a MAX over a set, not a sum over a
/// hand, so a masked card has no additive term to be the gap. Its cross-view
/// behaviour is a set difference, covered by the attainable-source cell above.
#[test]
fn the_masked_slot_term_is_exactly_the_cross_view_gap_for_o1_o2_o3_and_o6() {
    let rules = Rules::standard();
    const MASKED: OrderCardId = OrderCardId(27); // 2 copper + 2 seafood, one warehouse icon
    const OPEN: OrderCardId = OrderCardId(3); // 2 copper + 1 seafood, icon-free
    let mut state = cleared_order_table(2, 261, &rules);
    unfulfilled_starting_order(&mut state, 1, MASKED);
    deal_card(&mut state, 1, OPEN);
    set_stocks(&mut state, 1, [3, 0, 0, 2]);

    let mine = observe(&state, PlayerId(1)).expect("seat 1 observes");
    let my_structure = ScoreStructure::derive(&mine, &rules);
    let full = row_of(&mine, PlayerId(1));
    let mut without = full.clone();
    without.hand.retain(|c| *c != HandCard::Known(MASKED));
    assert_eq!(without.hand.len(), full.hand.len() - 1, "exactly one card removed");

    let theirs = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let their_structure = ScoreStructure::derive(&theirs, &rules);
    let masked_row = row_of(&theirs, PlayerId(1));
    assert!(
        masked_row.hand.contains(&HandCard::HiddenStartingOrder),
        "seat 1's starting order must still be unfulfilled"
    );

    // O2 / O3 / O6: the masking view reads EXACTLY the without-the-card value.
    let readers: [(&str, EReader); 3] = [
        ("O2", |s, p, r| s.order_matched_goods(p, r)),
        ("O3", |s, p, r| s.executable_orders(p, r)),
        ("O6", |s, p, r| s.held_order_icons(p, r)),
    ];
    for (name, read) in readers {
        let with = read(&my_structure, &full, &rules);
        let term = with - read(&my_structure, &without, &rules);
        assert_ne!(term, 0, "{name}: the masked card contributes nothing — the pin is vacuous");
        assert_eq!(
            read(&their_structure, &masked_row, &rules),
            read(&my_structure, &without, &rules),
            "{name}: the masking view must read the hand WITHOUT the masked card"
        );
        assert_eq!(
            read(&their_structure, &masked_row, &rules) - with,
            -term,
            "{name}: the cross-view gap is exactly the masked slot's own term"
        );
    }

    // O1: the same, plus ONE pool floor in place of the card's true face.
    let floor = their_structure.context.unseen_order_floor();
    let face = half_vp_face(&rules, MASKED);
    assert_eq!(
        their_structure.held_order_value(&masked_row, &rules),
        my_structure.held_order_value(&without, &rules) + floor,
        "O1: the rest of the hand at its face, and the masked slot at the pool floor"
    );
    assert_eq!(
        their_structure.held_order_value(&masked_row, &rules)
            - my_structure.held_order_value(&full, &rules),
        floor - face,
        "O1: the cross-view gap is exactly floor minus the masked card's true face"
    );
    assert_ne!(floor, face, "the fixture must make the substitution visible in O1");
}

/// §3.2's ALLOCATION CONTRACT, the Group-E half: every helper is
/// allocation-free.
///
/// `score`'s own allocs==0 pin already covers `ScoreStructure::derive`'s new
/// order-card pass — `score` derives once per call, so the pool and the
/// attainable scan ride it whether or not anything reads them. This cell covers
/// the READERS, which that pin cannot: the eight are not on `score`'s path until
/// §7's step 8 moves `Feature::ALL`, so without this an O5 that collected its
/// attainable countries into a `Vec` would go unnoticed.
///
/// 4P for the widest seat loop, with a hand, stocks, tokens and a completed pile
/// on the measured row so no helper short-circuits to zero.
#[test]
fn every_group_e_helper_allocates_nothing() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(4, 263, &rules);
    deal_card(&mut state, 0, OrderCardId(10));
    deal_card(&mut state, 0, OrderCardId(3));
    state.players[0].money = 5;
    set_stocks(&mut state, 0, [3, 0, 0, 2]);
    state.players[0].shophouses.in_hand = 2;
    state.players[0].trading_houses.in_hand = 1;
    complete_card(&mut state, 0, OrderCardId(1));
    complete_card(&mut state, 0, OrderCardId(2));
    // A SECOND, masked row, so O1's pool-floor branch and the three
    // hidden-slot zeros are measured too — not only the Known path.
    unfulfilled_starting_order(&mut state, 1, OrderCardId(27));
    offer_card(&mut state, 0, OrderCardId(6));

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let player = row_of(&view, PlayerId(0));
    let masked = row_of(&view, PlayerId(1));
    assert!(masked.hand.contains(&HandCard::HiddenStartingOrder), "the masked row is masked");

    let (sum, stats) = meter::measure(|| {
        let read = |p: &VisiblePlayer| {
            structure.money_stock(p)
                + structure.trade_goods_stock(p)
                + structure.building_tokens_ready(p)
                + structure.held_order_value(p, &rules)
                + structure.order_matched_goods(p, &rules)
                + structure.executable_orders(p, &rules)
                + structure.flag_icon_marginal(p, &rules)
                + structure.held_order_icons(p, &rules)
        };
        read(&player) + read(&masked)
    });
    assert_eq!(stats.allocs, 0, "a Group-E helper allocated ({} allocations)", stats.allocs);
    assert!(sum > 0, "the fixture must exercise real values, or the meter measured nothing");
    println!("Group E: {sum} summed over 4P, {} allocs / {} peak bytes", stats.allocs, stats.peak_bytes);
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Group X: the interaction cell's fixtures
// ---------------------------------------------------------------------------

/// The Group-X FEATURE BODIES, by source-text head — hand-maintained for the
/// same reason [`S_HELPER_HEADS`] and [`E_HELPER_HEADS`] are, and now backed by
/// [`the_three_group_lists_name_every_feature_helper_eval_rs_declares`], which
/// makes an unlisted helper a failure rather than a silence.
const X_HELPER_HEADS: [&str; 3] = [
    "    pub fn lab_takeable(",
    "    pub fn port_takeable(",
    "    pub fn matched_production_access(",
];

/// Every method head `eval.rs` declares inside one §3.2 group section — from the
/// section's banner comment to the ruler that opens the next section.
///
/// Method heads only: a free function sits at column zero and is deliberately
/// outside every list (`half_vp_face`, `port_offers` and their kin are guarded
/// by the value pins their callers carry, not by a text scan).
fn helper_heads_in_section(banner: &str) -> Vec<String> {
    const SRC: &str = include_str!("../src/game/eval.rs");
    let start = SRC.find(banner).unwrap_or_else(|| panic!("eval.rs carries the banner `{banner}`"));
    let after_banner = &SRC[start..];
    let body = after_banner
        .find("\nimpl ScoreStructure {")
        .unwrap_or_else(|| panic!("`{banner}` is not followed by an `impl ScoreStructure` block"));
    let section = &after_banner[body..];
    let end = section.find("\n// ------").unwrap_or(section.len());
    section[..end]
        .lines()
        .filter(|line| line.starts_with("    pub fn ") || line.starts_with("    fn "))
        .map(|line| line[..line.find('(').expect("a declaration opens its arguments") + 1].to_string())
        .collect()
}

/// Put technologies into one lab board's six slots, leftmost first — board 0 is
/// Lab A, board 1 Lab B.
///
/// Written OUTRIGHT, the way [`clear_labs`] empties them: the tech deck is not
/// conserved by this file (unlike the thirty-six order cards, which
/// [`cleared_order_table`] gathers and [`take_free_card`] hands out one at a
/// time). What matters for Group X is which titles sit at which POSITIONS, and
/// the assertion below is what keeps a fixture from planting one title twice by
/// accident — the one lab state the rules cannot produce for a single-copy
/// title.
fn set_lab(state: &mut GameState, board: usize, ids: [Option<TechCardId>; 6]) {
    let named: Vec<TechCardId> = ids.iter().flatten().copied().collect();
    for (i, id) in named.iter().enumerate() {
        assert!(!named[i + 1..].contains(id), "{id:?} is planted in two slots at once");
    }
    let lab = match board {
        0 => &mut state.lab_state.lab_a,
        _ => state.lab_state.lab_b.as_mut().expect("this board carries no Lab B"),
    };
    lab.slots = ids;
}

/// Put order cards into one port board's six slots, leftmost first, returning
/// whatever stood there to the deck — board 0 is Port A, board 1 Port B.
///
/// POSITIONS are the point: X2's zone test is positional, so a fixture that
/// could not say "this offer sits at index 4" could not exercise it. Cards come
/// through [`take_free_card`], so the thirty-six-card conservation
/// [`cleared_order_table`] asserts survives.
fn set_port(state: &mut GameState, board: usize, ids: [Option<OrderCardId>; 6]) {
    fn board_of(state: &mut GameState, board: usize) -> &mut PortBoard {
        match board {
            0 => &mut state.port_state.port_a,
            _ => state.port_state.port_b.as_mut().expect("this table carries no Port B"),
        }
    }
    let returned: Vec<OrderCardId> =
        board_of(state, board).slots.iter_mut().filter_map(|slot| slot.take()).collect();
    state.order_deck.extend(returned);
    for (index, id) in ids.into_iter().enumerate() {
        if let Some(id) = id {
            take_free_card(state, id);
            board_of(state, board).slots[index] = Some(id);
        }
    }
}

/// Give `seat` the named technologies outright — the owned-title list X1's
/// EFFECT-equality filter and the 4th-card duty both read.
fn own_techs(state: &mut GameState, seat: usize, ids: &[TechCardId]) {
    state.players[seat].technologies = ids.to_vec();
}

/// `n` technology cards with PAIRWISE DISTINCT effects and the named printed
/// gear cost, skipping every effect in `avoid`.
///
/// **`iter_sorted_by_id`, never the bare registry iterator.** Registry iteration
/// is `HashMap`-backed and process-randomized (`observe.rs:220-222` names the
/// failure in those words), so a cell that SEARCHES for a card off `iter` plants
/// a different one every run: it passes each time and pins nothing. Every cell
/// below prints the identities this returns, and they are the same in every
/// process.
fn distinct_effect_techs(
    rules: &Rules,
    n: usize,
    cost: u8,
    avoid: &[TechEffect],
) -> Vec<TechCardId> {
    let mut taken: Vec<TechEffect> = avoid.to_vec();
    let mut out = Vec::new();
    for tech in rules.techs.iter_sorted_by_id() {
        if out.len() == n {
            break;
        }
        if tech.industry_cost == cost && !taken.contains(&tech.effect) {
            taken.push(tech.effect);
            out.push(tech.id);
        }
    }
    assert_eq!(out.len(), n, "the manifest prints fewer than {n} distinct titles at gear {cost}");
    out
}

/// The FIRST title the manifest prints TWICE, as its two physical copies plus
/// the effect they share.
///
/// Ten of the twenty titles circulate as two cards precisely so two seats can
/// each own one (`actions/cards.rs:263-270`), and effect equality is the
/// engine's own ownership-by-title test. Same `iter_sorted_by_id` discipline as
/// above, and the caller prints what it resolved.
fn two_copies_of_one_title(rules: &Rules) -> (TechCardId, TechCardId, TechEffect) {
    let sorted = rules.techs.iter_sorted_by_id();
    for (i, first) in sorted.iter().enumerate() {
        for second in &sorted[i + 1..] {
            if second.effect == first.effect {
                return (first.id, second.id, first.effect);
            }
        }
    }
    panic!("the manifest prints no title twice — ten of the twenty are supposed to");
}

/// Does the ENGINE accept SOME payment for this Lab-A slot?
///
/// The exhaustive search over `validate_lab_action`'s own payment shapes — every
/// extra-Yen amount the purse could cover (odd ones included, so the exact-cover
/// refusal is exercised rather than assumed), every import count, and all three
/// answers to the extra-technology surcharge. That makes the comparison below an
/// ACCEPT/REJECT pin against the engine rather than against a restatement of its
/// arithmetic.
fn engine_accepts_lab_slot(
    state: &GameState,
    seat: PlayerId,
    slot: usize,
    power: u8,
    rules: &Rules,
) -> bool {
    let row = state.player(seat);
    let (money, imports) = (row.money.min(40) as u8, row.imported_goods.min(20) as u8);
    for extra_yen in 0..=money {
        for extra_imported in 0..=imports {
            for payment in [None, Some(true), Some(false)] {
                let ok = yokohama_core::actions::validate_lab_action(
                    state,
                    seat,
                    &state.lab_state.lab_a,
                    slot,
                    extra_yen,
                    extra_imported,
                    payment,
                    power,
                    &rules.techs,
                )
                .is_ok();
                if ok {
                    return true;
                }
            }
        }
    }
    false
}

/// The three Group-X values of one row, in §3.2's LABEL order — X1, X2, X3.
fn x_values(
    structure: &ScoreStructure,
    view: &PlayerView,
    player: &VisiblePlayer,
    rules: &Rules,
) -> [i64; 3] {
    [
        structure.lab_takeable(view, player),
        structure.port_takeable(view, player, rules),
        structure.matched_production_access(view, player, rules),
    ]
}

/// The three Group-X values of each seat, as read from that seat's OWN view.
fn group_x_row(state: &GameState, rules: &Rules) -> [[i64; 3]; 2] {
    let mut out = [[0i64; 3]; 2];
    for (seat, row) in out.iter_mut().enumerate() {
        *row = read_s(state, rules, PlayerId(seat as u8), |s, view, p| x_values(s, view, p, rules));
    }
    out
}

/// One order card's printed requirement TOTAL against a stock vector — the
/// per-offer cap X2 applies, computed here from the registry rather than from
/// the helper it checks.
fn matched_against(rules: &Rules, card: OrderCardId, stock: [u32; 4]) -> i64 {
    let req = requirement_vector(rules, card);
    (0..4).map(|g| i64::from(stock[g].min(req[g]))).sum()
}

/// §7 audit part 4 for X1, the SURCHARGE half: the context's all-in industry
/// cost is the card's printed gear cost plus `lab_position_surcharge` at ALL SIX
/// 0-based slot indices.
///
/// The engine's helper is the authority on the magnitudes (`0|1 → +0`,
/// `2|3 → +1`, `4|5 → +2`; board art, not p.14 prose), and it is called here
/// rather than restated, so the pin is a comparison and not a second copy of the
/// same six numbers. The lab is filled with six EQUAL-gear titles precisely so
/// that every difference across the six slots is the surcharge and nothing else.
#[test]
fn the_lab_context_prices_every_open_slot_at_gear_plus_the_engines_surcharge() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 301, &rules);
    let techs = distinct_effect_techs(&rules, 6, 3, &[]);
    println!("lab slots 0..5 = {techs:?}");
    set_lab(&mut state, 0, [
        Some(techs[0]),
        Some(techs[1]),
        Some(techs[2]),
        Some(techs[3]),
        Some(techs[4]),
        Some(techs[5]),
    ]);

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let context = ScoreStructure::derive(&view, &rules).context;
    for slot in 0..LAB_SLOTS {
        let card = rules.techs.get(techs[slot]).expect("a printed technology");
        assert_eq!(
            context.lab_industry_cost[0][slot],
            card.industry_cost + yokohama_core::actions::lab_position_surcharge(slot),
            "slot {slot}: gear {} plus the engine's own surcharge",
            card.industry_cost
        );
    }
    // Non-vacuity: the six all-in costs are NOT one number, or the surcharge
    // could be zero everywhere and this cell would still pass.
    assert_eq!(
        context.lab_industry_cost[0],
        [3, 3, 4, 4, 5, 5],
        "six gear-3 titles under the +0/+0/+1/+1/+2/+2 board art"
    );
    // Lab B is absent at 2P, and an absent board prices nothing.
    assert_eq!(context.lab_areas[1], 0, "a 2P table carries no Lab B area");
    assert_eq!(context.lab_industry_cost[1], [0; LAB_SLOTS], "an absent board prices nothing");
}

/// §7 audit part 4 for X1, the OWNERSHIP half: a title is owned by EFFECT
/// EQUALITY, on the fixture that separates that reading from an id comparison —
/// the two PHYSICAL COPIES of one title, side by side in slots 0 and 1.
///
/// Ten of the twenty titles circulate as two cards so two seats can each own
/// one, and the engine refuses a purchase whose EFFECT the buyer already holds
/// (`actions/cards.rs:263-273`). So a seat holding copy A must see BOTH slots
/// closed; a seat holding neither must see both open. An implementation
/// comparing card ids would leave the twin open and fail here — which is the
/// whole reason the fixture is built out of a two-copy title.
#[test]
fn the_lab_context_closes_both_physical_copies_of_a_title_the_seat_owns() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 303, &rules);
    let (first, second, effect) = two_copies_of_one_title(&rules);
    println!("two-copy title {effect:?} = {first:?} + {second:?}");
    let others = distinct_effect_techs(&rules, 4, 3, &[effect]);
    set_lab(&mut state, 0, [
        Some(first),
        Some(second),
        Some(others[0]),
        Some(others[1]),
        Some(others[2]),
        Some(others[3]),
    ]);
    // Seat 0 owns ONE physical copy; seat 1 owns nothing.
    own_techs(&mut state, 0, &[first]);

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let context = ScoreStructure::derive(&view, &rules).context;
    let open = |seat: usize, slot: usize| context.lab_unowned[seat] & lab_bit(0, slot) != 0;

    assert!(!open(0, 0), "the seat holds this very card");
    assert!(!open(0, 1), "the TWIN is the same title — effect equality, not id equality");
    assert!(open(1, 0) && open(1, 1), "the other seat owns neither copy");
    for slot in 2..LAB_SLOTS {
        assert!(open(0, slot) && open(1, slot), "slot {slot} is a title neither seat owns");
    }
    // The engine agrees, on its own refusal path.
    let refusal = yokohama_core::actions::validate_lab_action(
        &state,
        PlayerId(0),
        &state.lab_state.lab_a,
        1,
        0,
        0,
        None,
        5,
        &rules.techs,
    );
    assert_eq!(
        refusal,
        Err("Already own a Technology card with this name"),
        "the engine refuses the twin for the same reason"
    );
}

/// **X1 `LabTakeable`** — the count of open lab slots the seat could take ALL
/// IN, and the pooled 2¥-or-1-import arithmetic that decides it.
///
/// Six gear-3 titles at power 2 give deficits `[1, 1, 2, 2, 3, 3]` — the
/// surcharge ladder, read through the feature. The budget is
/// `floor(money/2) + imports`, and the cell walks it one step at a time, which
/// is what makes the POOLING exact rather than incidental: two Yen and one
/// import buy the same single point of industry, so `(money 4, imports 0)`,
/// `(money 2, imports 1)` and `(money 0, imports 2)` must all read the same
/// count.
#[test]
fn lab_takeable_counts_the_slots_the_pooled_all_in_purse_clears() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 305, &rules);
    let lab = area_typed(&state, AreaType::LaboratoryA);
    let techs = distinct_effect_techs(&rules, 6, 3, &[]);
    set_lab(&mut state, 0, [
        Some(techs[0]),
        Some(techs[1]),
        Some(techs[2]),
        Some(techs[3]),
        Some(techs[4]),
        Some(techs[5]),
    ]);
    place_assistants(&mut state, 0, lab, 1); // realizable power 1 + 1 = 2

    let read = |state: &GameState| read_s(state, &rules, PlayerId(0), |s, v, p| s.lab_takeable(v, p));
    for (money, imports, expected) in [
        (0u32, 0u32, 0i64),
        (1, 0, 0), // one Yen buys no industry — the halving is a FLOOR
        (2, 0, 2),
        (3, 0, 2),
        (0, 1, 2),
        (4, 0, 4),
        (2, 1, 4),
        (0, 2, 4),
        (6, 0, 6),
        (2, 2, 6),
        (0, 3, 6),
        (99, 99, 6), // never more than the open slots
    ] {
        state.players[0].money = money;
        state.players[0].imported_goods = imports;
        assert_eq!(read(&state), expected, "money {money}, imports {imports}");
    }
}

/// §7's NAMED NEGATIVE boundary twin for X1: **the zero-deficit, duty-only
/// refusal.**
///
/// Power over-covers every printed cost, so the industry deficit is ZERO at all
/// six slots — and the seat still takes nothing, because it owns three
/// technologies and the FOURTH card owes a surcharge of 2 Yen or 1 import that
/// POWER MAY NOT PAY ("excess industry value is lost"). The six-lens round's
/// counterexample, verbatim: power 5, cost ≤ 5, money 0, imports 0, three techs
/// owned → 0 takeable.
///
/// Two controls keep the zero from being an accident of the fixture: dropping to
/// TWO owned technologies (no duty) reads 6, and keeping three but adding two
/// Yen reads 6 again. And the engine is asked directly — every payment shape
/// `validate_lab_action` prices is refused at the duty-only state and accepted
/// once the duty can be paid.
#[test]
fn lab_takeable_is_zero_on_a_duty_only_deficit_the_engine_also_refuses() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 307, &rules);
    let lab = area_typed(&state, AreaType::LaboratoryA);
    let open = distinct_effect_techs(&rules, 6, 3, &[]);
    let owned_effects: Vec<TechEffect> =
        open.iter().map(|id| rules.techs.get(*id).expect("a printed technology").effect).collect();
    let owned = distinct_effect_techs(&rules, 3, 4, &owned_effects);
    println!("open {open:?}; owned {owned:?}");
    set_lab(&mut state, 0, [
        Some(open[0]),
        Some(open[1]),
        Some(open[2]),
        Some(open[3]),
        Some(open[4]),
        Some(open[5]),
    ]);
    place_assistants(&mut state, 0, lab, 4); // realizable power 1 + 4 = 5
    state.players[0].money = 0;
    state.players[0].imported_goods = 0;
    own_techs(&mut state, 0, &owned);

    let read = |state: &GameState| read_s(state, &rules, PlayerId(0), |s, v, p| s.lab_takeable(v, p));
    assert_eq!(read(&state), 0, "power over-covers the cost and still cannot pay the 4th-card duty");
    for slot in 0..LAB_SLOTS {
        assert!(
            !engine_accepts_lab_slot(&state, PlayerId(0), slot, 5, &rules),
            "the engine must refuse slot {slot} at the duty-only state"
        );
    }

    // Control 1: the duty is what closed it — two owned technologies, not three.
    let mut two = state.clone();
    own_techs(&mut two, 0, &owned[..2]);
    assert_eq!(read(&two), 6, "with no duty due, every zero-deficit slot is takeable");

    // Control 2: the duty is PAYABLE, and two Yen is exactly its price.
    let mut paid = state.clone();
    paid.players[0].money = 2;
    assert_eq!(read(&paid), 6, "two Yen pays the duty at every zero-deficit slot");
    for slot in 0..LAB_SLOTS {
        assert!(
            engine_accepts_lab_slot(&paid, PlayerId(0), slot, 5, &rules),
            "the engine must accept slot {slot} once the duty can be paid"
        );
    }
}

/// §7 audit part 4 for X1, the ACCEPT/REJECT half: over a grid of purses,
/// powers and owned-title counts, **X1 equals the number of open slots the
/// ENGINE would accept some payment for.**
///
/// Not a restatement of the arithmetic — `engine_accepts_lab_slot` searches
/// `validate_lab_action`'s own payment space, so every clause the engine applies
/// (the exact-cover rule, the odd-Yen refusal, the surcharge's two prices, the
/// purse and import checks, the owned-title refusal) is inside the comparison.
///
/// The grid spans the duty on and off, deficits from negative to unpayable, and
/// a purse split every way between Yen and imports. Both verdicts are asserted
/// to occur, so the identity cannot pass by everything being accepted.
#[test]
fn lab_takeable_matches_the_engines_accept_reject_over_a_purchase_grid() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 309, &rules);
    let lab = area_typed(&state, AreaType::LaboratoryA);
    let open = distinct_effect_techs(&rules, 6, 3, &[]);
    let open_effects: Vec<TechEffect> =
        open.iter().map(|id| rules.techs.get(*id).expect("a printed technology").effect).collect();
    let owned_pool = distinct_effect_techs(&rules, 3, 4, &open_effects);
    set_lab(&mut state, 0, [
        Some(open[0]),
        Some(open[1]),
        Some(open[2]),
        Some(open[3]),
        Some(open[4]),
        Some(open[5]),
    ]);
    place_assistants(&mut state, 0, lab, 4);

    let (mut accepted, mut refused, mut cells) = (0usize, 0usize, 0usize);
    // Power 1 is not in the grid and cannot be: `validate_lab_action` takes power
    // as an ARGUMENT and has no presence clause, while X1's presence gate means
    // an occupied lab always reads at least 1 (president) + 1 (assistant). The
    // grid therefore spans exactly the powers X1 can report at an occupied lab.
    for pawns in 1..=4u8 {
        let mut sized = state.clone();
        recover_assistants(&mut sized, 0, lab, 4 - pawns);
        let power = 1 + pawns;
        for owned in 0..=3usize {
            own_techs(&mut sized, 0, &owned_pool[..owned]);
            for money in [0u32, 1, 2, 3, 4, 6] {
                for imports in [0u32, 1, 2] {
                    sized.players[0].money = money;
                    sized.players[0].imported_goods = imports;
                    let engine = (0..LAB_SLOTS)
                        .filter(|slot| {
                            engine_accepts_lab_slot(&sized, PlayerId(0), *slot, power, &rules)
                        })
                        .count() as i64;
                    let mine =
                        read_s(&sized, &rules, PlayerId(0), |s, v, p| s.lab_takeable(v, p));
                    assert_eq!(
                        mine, engine,
                        "power {power}, owned {owned}, money {money}, imports {imports}"
                    );
                    cells += 1;
                    accepted += engine as usize;
                    refused += LAB_SLOTS - engine as usize;
                }
            }
        }
    }
    assert!(accepted > 0 && refused > 0, "the grid must span both verdicts");
    println!("X1 vs validate_lab_action: {cells} cells, {accepted} accepts, {refused} refusals");
}

/// X1's PRESENCE gate and its family-11(d) boundary: the count is zero without
/// an assistant at the lab, and it collapses to zero again at the Recovery that
/// drains the acted area.
///
/// §3.2's presence sentence is the engine's own action-legality condition, and
/// §3.2's X1 boundary paragraph says the purchase's presence drains at Recovery
/// so X1 reads 0 at the following hand-off. Both halves are the same assertion
/// from the two directions, which is why they sit in one cell.
#[test]
fn lab_takeable_needs_a_presence_and_drains_to_zero_at_recovery() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 311, &rules);
    let lab = area_typed(&state, AreaType::LaboratoryA);
    let techs = distinct_effect_techs(&rules, 2, 3, &[]);
    set_lab(&mut state, 0, [Some(techs[0]), Some(techs[1]), None, None, None, None]);
    state.players[0].money = 99;

    let read = |state: &GameState| read_s(state, &rules, PlayerId(0), |s, v, p| s.lab_takeable(v, p));
    assert_eq!(read(&state), 0, "no assistant at the lab: no access to price");
    place_assistants(&mut state, 0, lab, 1);
    assert_eq!(read(&state), 2, "both open slots clear at an all-in purse");
    recover_assistants(&mut state, 0, lab, 1);
    assert_eq!(read(&state), 0, "family 11(d): the presence drains and the access goes with it");
}

/// **X2 `PortTakeable`** — the goods a face-up offer would match, summed over
/// the offers the seat's power actually reaches, each capped by its OWN printed
/// requirement.
///
/// The per-offer cap is what makes this a portfolio rather than a total: three
/// Yen-worth of seafood against three cards each wanting one seafood matches
/// three, not one. And the ZONES step with power exactly as the board prints
/// them, which is the other half of the term.
///
/// Every expectation is computed from the registry by [`matched_against`] and
/// then HARD-PINNED as a number, so a drift in either the helper or the fixture
/// has to move both to stay green.
#[test]
fn port_takeable_sums_each_offers_own_capped_match_over_the_open_zones() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 313, &rules);
    let port = area_typed(&state, AreaType::PortA);
    const OFFERS: [OrderCardId; 6] = [
        OrderCardId(1),
        OrderCardId(2),
        OrderCardId(3),
        OrderCardId(4),
        OrderCardId(5),
        OrderCardId(6),
    ];
    set_port(&mut state, 0, OFFERS.map(Some));
    const STOCK: [u32; 4] = [1, 1, 1, 1];
    set_stocks(&mut state, 0, STOCK);

    // The six per-offer matches, off the registry and then pinned.
    let matches: Vec<i64> = OFFERS.iter().map(|c| matched_against(&rules, *c, STOCK)).collect();
    assert_eq!(matches, vec![2, 2, 2, 3, 2, 3], "the printed requirements against one of each good");

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, v, p| s.port_takeable(v, p, &rules))
    };
    assert_eq!(read(&state), 0, "no assistant at the port: no access to price");
    for (pawns, expected, why) in [
        (1u8, 4i64, "power 2 opens LOW alone: slots 0 and 1"),
        (2, 9, "power 3 adds MID: slots 2 and 3"),
        (3, 14, "power 4 adds HIGH: slots 4 and 5"),
        (4, 14, "power 5 opens nothing further"),
    ] {
        let mut sized = state.clone();
        place_assistants(&mut sized, 0, port, pawns);
        assert_eq!(read(&sized), expected, "{why}");
    }
}

/// §7's NAMED boundary twin for X2, first half: **the hand-cap zeroing.**
///
/// The manual refuses a take into a hand already holding three orders, so the
/// whole term collapses — not one offer's worth, all of them — and stays
/// collapsed until a fulfillment empties a slot. §5.2's 4(f) is the pair that
/// prices the collapse; this is the cell that says it happens at all.
///
/// The engine is asked in its own words: `validate_port_action` refuses the
/// cheapest possible take at the cap.
#[test]
fn port_takeable_is_zero_at_the_three_card_hand_cap() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 315, &rules);
    let port = area_typed(&state, AreaType::PortA);
    set_port(&mut state, 0, [
        Some(OrderCardId(1)),
        Some(OrderCardId(2)),
        None,
        None,
        None,
        None,
    ]);
    set_stocks(&mut state, 0, [1, 1, 1, 1]);
    place_assistants(&mut state, 0, port, 1);
    deal_card(&mut state, 0, OrderCardId(10));
    deal_card(&mut state, 0, OrderCardId(11));

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, v, p| s.port_takeable(v, p, &rules))
    };
    assert_eq!(read(&state), 4, "a two-card hand may still take");
    let take = yokohama_core::actions::validate_port_action(
        &state,
        PlayerId(0),
        &state.port_state.port_a,
        &[OrderCardId(1)],
        None,
        2,
    );
    assert!(take.is_ok(), "the engine allows the take at hand size 2: {take:?}");

    deal_card(&mut state, 0, OrderCardId(12));
    assert_eq!(state.players[0].order_cards_in_hand.len(), 3, "the fixture is AT the cap");
    assert_eq!(read(&state), 0, "the whole term collapses at the cap, not one offer's worth");
    assert_eq!(
        yokohama_core::actions::validate_port_action(
            &state,
            PlayerId(0),
            &state.port_state.port_a,
            &[OrderCardId(1)],
            None,
            2,
        ),
        Err("Would exceed Order card hand limit of 3"),
        "the engine refuses for the reason the term collapses"
    );

    // PERSISTENT until a fulfillment: putting a card back re-opens the term.
    let mut fulfilled = state.clone();
    let card = fulfilled.players[0].order_cards_in_hand.pop().expect("a card to retire");
    fulfilled.players[0].completed_orders.push(card);
    assert_eq!(read(&fulfilled), 4, "the collapse lifts exactly when the hand drops below three");
}

/// §7's NAMED boundary twin for X2, second half: **a dry-deck `None` gap, and
/// the zone read positionally across it.**
///
/// Once the order deck runs out the port stops refilling, so a `None` may sit at
/// any index while cards remain to its right (`refill_port` compacts LEFT and
/// fills from the right). The zone of an offer is its SLOT's zone — index 1 is
/// LOW even when index 0 is empty — so the term must iterate positions, never
/// the flattened list of present cards.
///
/// The fixture is chosen so the two readings DIFFER: with a gap at 0 and offers
/// at 1 (LOW) and 5 (HIGH), a flatten-then-count implementation would call both
/// of them LOW and read 5 at power 2 where the truth is 2.
#[test]
fn port_takeable_reads_the_slot_position_across_a_dry_deck_gap() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 317, &rules);
    let port = area_typed(&state, AreaType::PortA);
    set_port(&mut state, 0, [
        None,
        Some(OrderCardId(1)),
        None,
        None,
        None,
        Some(OrderCardId(6)),
    ]);
    const STOCK: [u32; 4] = [1, 1, 1, 1];
    set_stocks(&mut state, 0, STOCK);
    let low = matched_against(&rules, OrderCardId(1), STOCK);
    let high = matched_against(&rules, OrderCardId(6), STOCK);
    assert_eq!((low, high), (2, 3), "the two offers match different amounts");

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, v, p| s.port_takeable(v, p, &rules))
    };
    for (pawns, expected, why) in [
        (1u8, low, "power 2: the LOW slot 1 only — the gap at 0 is not an offer"),
        (2, low, "power 3: slot 5 is still HIGH"),
        (3, low + high, "power 4: the HIGH slot 5 opens"),
    ] {
        let mut sized = state.clone();
        place_assistants(&mut sized, 0, port, pawns);
        assert_eq!(read(&sized), expected, "{why}");
    }
    // The discriminator, stated: flattening would read both offers as LOW.
    assert_ne!(low, low + high, "the fixture must separate the two readings");
}

/// §7 audit part 4 for X2: **the zones the term opens are exactly the zones the
/// engine's own `port_zone_accessible` opens**, at every power, over a full
/// board.
///
/// The expectation is rebuilt per power from the engine's gate and the registry
/// — never from a second copy of the 2/3/4 ladder — so this is the comparison
/// §7 asks for.
///
/// **RECORDED, because the card states two gates and only one of them can ever
/// fire.** §3.2 gates each board on `own_assistants >= 1` AND on
/// `realizable_power >= 2`; but `realizable_power` is
/// `min(5, own_assistants + 1 + …)`, so a board the seat has a presence at reads
/// at least 2 by construction and the power gate is implied by the presence gate
/// on every view. Both are implemented, because the card says both; the sweep
/// below therefore starts at power 2, which is the lowest power an occupied port
/// can report, rather than pretending to exercise power 1.
#[test]
fn port_takeable_opens_exactly_the_zones_the_engines_gate_opens() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 319, &rules);
    let port = area_typed(&state, AreaType::PortA);
    const OFFERS: [OrderCardId; 6] = [
        OrderCardId(1),
        OrderCardId(2),
        OrderCardId(3),
        OrderCardId(4),
        OrderCardId(5),
        OrderCardId(6),
    ];
    set_port(&mut state, 0, OFFERS.map(Some));
    const STOCK: [u32; 4] = [2, 1, 1, 2];
    set_stocks(&mut state, 0, STOCK);
    place_assistants(&mut state, 0, port, 1);

    let mut seen = std::collections::BTreeSet::new();
    for pawns in 0..=4u8 {
        let mut sized = state.clone();
        place_assistants(&mut sized, 0, port, pawns);
        // President + the standing assistant + the rest, capped where the engine
        // caps. The zone ladder saturates at 4, so the cap changes no verdict —
        // it is written because the reading it mirrors applies it.
        let power = (1 + 1 + pawns).min(5);
        let expected: i64 = OFFERS
            .iter()
            .enumerate()
            .filter(|(slot, _)| {
                yokohama_core::actions::port_zone_accessible(power, PortBoard::zone_of(*slot))
            })
            .map(|(_, card)| matched_against(&rules, *card, STOCK))
            .sum();
        let mine =
            read_s(&sized, &rules, PlayerId(0), |s, v, p| s.port_takeable(v, p, &rules));
        assert_eq!(mine, expected, "power {power}");
        seen.insert(mine);
    }
    assert!(seen.len() >= 3, "the sweep must move the value, or the gate is untested: {seen:?}");
    println!("X2 vs port_zone_accessible: values {seen:?}");
}

/// X2's PER-BOARD gates, on the one table that carries two port boards: at 3P
/// the presence and the power are asked BOARD BY BOARD, and an offer on a board
/// the seat does not stand at is worth nothing however rich it is.
///
/// This is the trap the term's shape exists to avoid — a flattened view of both
/// displays would credit Port B's offers to a seat that only reaches Port A.
#[test]
fn port_takeable_gates_presence_and_power_board_by_board() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(3, 321, &rules);
    let port_a = area_typed(&state, AreaType::PortA);
    let port_b = area_typed(&state, AreaType::PortB);
    const A_OFFERS: [OrderCardId; 6] = [
        OrderCardId(1),
        OrderCardId(2),
        OrderCardId(3),
        OrderCardId(4),
        OrderCardId(5),
        OrderCardId(6),
    ];
    const B_OFFERS: [OrderCardId; 6] = [
        OrderCardId(7),
        OrderCardId(8),
        OrderCardId(9),
        OrderCardId(10),
        OrderCardId(11),
        OrderCardId(12),
    ];
    set_port(&mut state, 0, A_OFFERS.map(Some));
    set_port(&mut state, 1, B_OFFERS.map(Some));
    const STOCK: [u32; 4] = [1, 1, 1, 1];
    set_stocks(&mut state, 0, STOCK);

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, v, p| s.port_takeable(v, p, &rules))
    };
    let a_low: i64 = A_OFFERS[..2].iter().map(|c| matched_against(&rules, *c, STOCK)).sum();
    let b_low: i64 = B_OFFERS[..2].iter().map(|c| matched_against(&rules, *c, STOCK)).sum();
    assert!(b_low > 0, "Port B's LOW zone must be worth something, or the gate is untested");

    place_assistants(&mut state, 0, port_a, 1);
    assert_eq!(read(&state), a_low, "Port B's offers belong to a board the seat does not stand at");
    place_assistants(&mut state, 0, port_b, 1);
    assert_eq!(read(&state), a_low + b_low, "both boards, each at its OWN power");
}

/// One seat's four trade-good stocks, in the `[copper, silk, tea, seafood]`
/// order every Group-E and Group-X reader uses.
fn stocks_of(state: &GameState, seat: PlayerId) -> [u32; 4] {
    let p = state.player(seat);
    [p.copper, p.silk, p.tea, p.seafood]
}

/// §7 audit part 4 for X3, the MAP half: the good `production_good` credits an
/// area type to is the good the ENGINE's own production action pays there.
///
/// Read off ORGANIC play at 2/3/4P: every applied `ExecuteAreaAction` in the
/// production family is checked against the printed reward the §13 dispatcher
/// gives for the acting area's type at the power the engine itself recorded, and
/// the named good must have risen by at least that much. A transposed map (tea
/// for seafood, say) fails at the first tile that pays anything.
///
/// **The narrowing, stated rather than hidden:** the other three goods are NOT
/// asserted unmoved. The same applied move can carry the automatic 5-Power
/// bonus, whose token pays goods of its own choosing (`FivePowerEffect::Copper`
/// and friends), so an equality there would be pinning the token and not the
/// map. The `>=` form still discriminates, because a wrong good would have to be
/// paid the right amount by an unrelated bonus at every sampled tile.
#[test]
fn the_production_good_map_is_the_good_the_engines_own_action_pays() {
    let rules = Rules::standard();
    let mut fired = [0usize; 4];
    for (players, seed) in [(2u8, 401u64), (3, 403), (4, 405)] {
        let mut state = keep_game(players, seed, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x3F3F);
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        for _ in 0..900usize {
            if is_game_over(&state) {
                break;
            }
            let mv = {
                let handle = DecisionHandle::new(&state, &rules);
                policy.pick(&handle, &mut rng).expect("the rung always finds a move")
            };
            let producing = matches!(
                &mv,
                Move::ExecuteAreaAction { action } if matches!(
                    action,
                    AreaAction::ProduceCopper
                        | AreaAction::ProduceSilk
                        | AreaAction::ProduceTea
                        | AreaAction::ProduceSeafood
                )
            );
            let actor = state.current_player;
            let acting_area = if state.is_mercenary_action {
                state.mercenary_target_area
            } else {
                match state.player(actor).president_location {
                    PresidentLocation::OnBoard(a) => Some(a),
                    PresidentLocation::InHand => None,
                }
            };
            let power = state.current_action_power.unwrap_or(0);
            let before = stocks_of(&state, actor);
            let next = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
            if producing {
                let area = acting_area.expect("a production action has an acting area");
                let kind = state
                    .board_layout
                    .areas
                    .get(&area)
                    .expect("the acting area is on the board")
                    .area_type;
                let good = production_good(kind).expect("a production area names its good");
                let reward = exported_reward(kind, power).expect("the dispatcher prices it");
                let after = stocks_of(&next, actor);
                assert!(
                    after[good] >= before[good] + u32::from(reward),
                    "{kind:?} at power {power} pays {reward} of good {good}: {before:?} -> {after:?}"
                );
                if reward > 0 {
                    fired[good] += 1;
                }
            }
            state = next;
        }
    }
    for (good, count) in fired.iter().enumerate() {
        assert!(*count > 0, "good {good} was never paid a positive reward — its arm is unexercised");
    }
    println!("production-good map: paying activations per good {fired:?}");
    // Every non-production type is OUTSIDE the map's domain, the same way it is
    // outside the dispatcher's.
    for kind in [
        AreaType::Bank,
        AreaType::PortA,
        AreaType::LaboratoryA,
        AreaType::EmploymentAgency,
        AreaType::Church,
        AreaType::Customs,
        AreaType::Canal,
    ] {
        assert_eq!(production_good(kind), None, "{kind:?} exports no trade good");
    }
}

/// **X3 `MatchedProductionAccess`** — the export a production tile would pay,
/// capped by what the seat's KNOWN held orders still NEED of that good.
///
/// Three tiles at three different readings in one fixture: a Tea Plantation
/// whose export is worth every good of it, a Fishing Ground whose need runs out
/// before its export does, and a Copper Mine at the power where the engine pays
/// nothing at all (§3.2's cited sparsity fact — a satisfied presence gate can
/// still contribute zero).
///
/// Then the boundary the card names: the need closes AS THE STOCK FILLS, one
/// good at a time and with no cliff, because the term is a minimum.
#[test]
fn matched_production_access_is_the_export_capped_by_the_residual_need() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(2, 323, &rules);
    let tea = area_typed(&state, AreaType::TeaPlantation);
    let fishing = area_typed(&state, AreaType::FishingGround);
    let copper = area_typed(&state, AreaType::CopperMine);
    // Card 26: French, 3 tea + 1 seafood.
    const CARD: OrderCardId = OrderCardId(26);
    assert_eq!(requirement_vector(&rules, CARD), [0, 0, 3, 1], "the fixture's printed requirements");
    deal_card(&mut state, 0, CARD);
    set_stocks(&mut state, 0, [0, 0, 0, 0]);
    place_assistants(&mut state, 0, tea, 2); // power 3 -> tea export 3
    place_assistants(&mut state, 0, fishing, 1); // power 2 -> seafood export 2
    place_assistants(&mut state, 0, copper, 1); // power 2 -> copper export 0

    let read = |state: &GameState| {
        read_s(state, &rules, PlayerId(0), |s, v, p| s.matched_production_access(v, p, &rules))
    };
    // tea min(3, 3) + seafood min(2, 1) + copper min(0, 0)
    assert_eq!(read(&state), 4, "three tiles, three different readings");

    for (stock, expected, why) in [
        ([0u32, 0, 1, 0], 3i64, "one tea banked: the tea need falls to 2"),
        ([0, 0, 3, 0], 1, "the tea need closes; the seafood tile still reads"),
        ([0, 0, 3, 1], 0, "both needs closed — the order is executable and X3 is 0"),
        ([9, 9, 9, 9], 0, "a stock past every requirement stays at 0, never negative"),
    ] {
        let mut filled = state.clone();
        set_stocks(&mut filled, 0, stock);
        assert_eq!(read(&filled), expected, "{why}");
    }

    // The other boundary: a fulfillment retires the need with the card.
    let mut fulfilled = state.clone();
    let card = fulfilled.players[0].order_cards_in_hand.pop().expect("the dealt card");
    fulfilled.players[0].completed_orders.push(card);
    assert_eq!(read(&fulfilled), 0, "family 6(a): the need leaves the hand with the card");
}

/// §7 audit part 4 for X3: **the registered 10(d) fixture, read through the
/// package's own public surface**, with the arithmetic re-derived from the
/// engine's dispatcher and the printed requirements rather than restated.
///
/// `pair_fixtures.rs` is not touched by this batch — the frozen package is
/// LOADED here as test input, which is what a registered gate fixture is for.
/// 10(d) is X3's gate: one Fishing Ground pawn at realizable power 2, zero
/// stocks, holding card 26 (which still needs a seafood) against card 5 (which
/// does not).
#[test]
fn matched_production_access_reads_the_registered_10d_fixture_exactly() {
    use yokohama_core::game::pair_fixtures::{
        build_registered_pair, Side, StageEndpoint, FAMILY_10D_MATCHED_CARD,
        FAMILY_10D_UNMATCHED_CARD, PAIR_10D,
    };
    let rules = Rules::standard();
    let fixture = build_registered_pair(&PAIR_10D, StageEndpoint::Early, &rules)
        .expect("10(d) builds at its registered stage");
    let subject = fixture.subject();

    let read = |side: Side| {
        let view = fixture.view(side);
        let structure = ScoreStructure::derive(view, &rules);
        let row = row_of(view, subject);
        (structure.matched_production_access(view, &row, &rules), structure, row)
    };
    let (greater, structure, row) = read(Side::Greater);
    let (lesser, _, _) = read(Side::Lesser);

    // The summation domain IS one tile, or the arithmetic below is about the
    // wrong sum.
    let occupied_production: Vec<AreaId> = areas_in(
        structure.context.occupied[usize::from(subject.0)] & structure.context.production,
    )
    .collect();
    assert_eq!(occupied_production.len(), 1, "10(d) registers ONE production presence");
    let area = occupied_production[0];
    let view = fixture.view(Side::Greater);
    let kind = view.board_layout.areas.get(&area).expect("the tile is on the board").area_type;
    assert_eq!(kind, AreaType::FishingGround, "10(d)'s registered tile");

    // The two factors, each from its own authority.
    let power = realizable_power(view, &row, area);
    assert_eq!(power, 2, "10(d) registers realizable power 2");
    let export = exported_reward(kind, power).expect("a production tile is priced");
    assert_eq!(export, 2, "the dispatcher's Fishing Ground reading at power 2");
    let need = requirement_vector(&rules, FAMILY_10D_MATCHED_CARD)[3];
    assert_eq!(need, 1, "card 26 still needs one seafood at zero stocks");
    assert_eq!(requirement_vector(&rules, FAMILY_10D_UNMATCHED_CARD)[3], 0, "card 5 needs none");

    assert_eq!(greater, i64::from(export.min(need as u8)), "min(export 2, residual need 1)");
    assert_eq!(greater, 1, "10(d)'s registered `x3_greater`");
    assert_eq!(lesser, 0, "10(d)'s registered `x3_lesser`");
    println!("10(d): X3 greater {greater}, lesser {lesser}, at {kind:?} power {power}");
}

/// A 2P table where the two seats differ in ALL THREE Group-X readings and
/// NEITHER carries a masked slot — §5.2(b)'s constructible no-hidden-slot class,
/// built rather than played into, and shared by the two cross-cutting cells
/// below so they are read off one position.
///
/// Both seats stand at the lab, the port and the Tea Plantation; what differs is
/// the purse, the power at each board, the hand and the owned title.
fn group_x_fixture(seed: u64, rules: &Rules) -> GameState {
    let mut state = cleared_order_table(2, seed, rules);
    let lab = area_typed(&state, AreaType::LaboratoryA);
    let port = area_typed(&state, AreaType::PortA);
    let tea = area_typed(&state, AreaType::TeaPlantation);
    let techs = distinct_effect_techs(rules, 6, 3, &[]);
    set_lab(&mut state, 0, [
        Some(techs[0]),
        Some(techs[1]),
        Some(techs[2]),
        Some(techs[3]),
        Some(techs[4]),
        Some(techs[5]),
    ]);
    set_port(&mut state, 0, [
        Some(OrderCardId(1)),
        Some(OrderCardId(2)),
        Some(OrderCardId(3)),
        Some(OrderCardId(4)),
        Some(OrderCardId(5)),
        Some(OrderCardId(6)),
    ]);
    fulfilled_starting_order(&mut state, 0, OrderCardId(21));
    fulfilled_starting_order(&mut state, 1, OrderCardId(22));

    place_assistants(&mut state, 0, lab, 1);
    place_assistants(&mut state, 0, port, 1);
    place_assistants(&mut state, 0, tea, 2);
    state.players[0].money = 6;
    state.players[0].imported_goods = 0;
    set_stocks(&mut state, 0, [1, 1, 1, 1]);
    deal_card(&mut state, 0, OrderCardId(7));
    own_techs(&mut state, 0, &[techs[5]]);

    place_assistants(&mut state, 1, lab, 1);
    place_assistants(&mut state, 1, port, 3);
    place_assistants(&mut state, 1, tea, 1);
    state.players[1].money = 0;
    state.players[1].imported_goods = 0;
    set_stocks(&mut state, 1, [1, 1, 1, 1]);
    deal_card(&mut state, 1, OrderCardId(5));
    state
}

/// §7 audit part 1 for Group X: **view honesty, at the type level.**
///
/// Every Group-X helper's parameter list names only a seat's view, a seat's
/// PUBLIC row, and the printed registries. The group reads both sides of §2's
/// partition, which is exactly why the claim is worth making off the
/// declaration: what it may not take is engine state, and no argument carries
/// any.
#[test]
fn every_group_x_helper_takes_only_a_view_a_public_row_and_the_registries() {
    const ALLOWED_PARAM_TYPES: [&str; 3] = ["&PlayerView", "&VisiblePlayer", "&Rules"];
    for head in X_HELPER_HEADS {
        let source = s_helper_source(head);
        let open = source.find('(').expect("a declaration has an argument list");
        let close = source.find(')').expect("a declaration closes its argument list");
        let params = &source[open + 1..close];
        let mut named = 0usize;
        for (i, param) in params.split(',').map(str::trim).filter(|p| !p.is_empty()).enumerate() {
            if i == 0 {
                assert_eq!(param, "&self", "`{head}` must be a method on the structure");
                continue;
            }
            let ty = param.split(": ").nth(1).unwrap_or_else(|| panic!("`{param}` has no type"));
            assert!(
                ALLOWED_PARAM_TYPES.contains(&ty),
                "`{head}` takes `{ty}`, which is not one of {ALLOWED_PARAM_TYPES:?} — a Group-X \
                 helper is a function of a seat's VIEW, its public row and the registries"
            );
            named += 1;
        }
        assert!(named > 0, "`{head}` names no argument beyond `&self`");
    }
}

/// §7's cross-group cleanliness assert, THE CLOSER: **only Group X reads both
/// sides.**
///
/// The other two clauses are already executable — no S helper reads money,
/// imports or orders; no E helper reads power or presence — and each is a
/// NEGATIVE statement. On their own they are consistent with a third group that
/// reads neither side, or with Group X being a fourth copy of Group S. This is
/// the POSITIVE half: every Group-X body names at least one needle from EACH
/// list, so the interaction cell is the interaction cell.
///
/// The three cells share [`ECONOMIC_READS`] and [`SPATIAL_READS`] rather than
/// each declaring its own copy — the claim "only X reads both" is only about one
/// partition if all three clauses are scanned for the same needles.
#[test]
fn every_group_x_helper_reads_both_sides_of_the_partition() {
    for head in X_HELPER_HEADS {
        let source = s_helper_source(head);
        let economic: Vec<&str> =
            ECONOMIC_READS.iter().copied().filter(|n| source.contains(n)).collect();
        let spatial: Vec<&str> =
            SPATIAL_READS.iter().copied().filter(|n| source.contains(n)).collect();
        assert!(
            !economic.is_empty(),
            "`{head}` reads no money, goods or order field — an interaction term that reads only \
             one side belongs in Group S"
        );
        assert!(
            !spatial.is_empty(),
            "`{head}` reads no power, presence or board field — an interaction term that reads \
             only one side belongs in Group E"
        );
        println!("{head}: economic {economic:?}, spatial {spatial:?}");
    }
}

/// The cleanliness sweep's EXHAUSTIVENESS half: every feature helper `eval.rs`
/// declares inside the three `§3.2` group sections is named by exactly one of
/// the three lists.
///
/// Without this the three scans are only as complete as three hand-maintained
/// arrays, and a nineteenth helper added to a group would be scanned by nobody —
/// the omission the group lists' own docs warn about, made impossible rather
/// than merely warned about.
#[test]
fn the_three_group_lists_name_every_feature_helper_eval_rs_declares() {
    let groups: [(&str, &str, &[&str]); 3] = [
        ("Group S", "// V4 §3.2 — Group S", &S_HELPER_HEADS),
        ("Group E", "// V4 §3.2 — Group E", &E_HELPER_HEADS),
        ("Group X", "// V4 §3.2 — Group X", &X_HELPER_HEADS),
    ];
    let mut total = 0usize;
    for (name, banner, listed) in groups {
        let declared = helper_heads_in_section(banner);
        assert!(!declared.is_empty(), "{name}'s section declares no helper at all");
        for head in &declared {
            assert!(
                listed.contains(&head.as_str()),
                "{name} declares `{head}`, which no cleanliness list names — add it, or the \
                 scans do not cover it"
            );
        }
        for head in listed {
            assert!(
                declared.contains(&head.to_string()),
                "{name}'s list names `{head}`, which its section no longer declares"
            );
        }
        total += declared.len();
        println!("{name}: {} helpers", declared.len());
    }
    assert_eq!(total, 20, "nine Group-S bodies, eight Group-E, three Group-X");
}

/// §7 audit part 2(a) for Group X: **INDEX-PERMUTATION INVARIANCE** under a FULL
/// seat relabel (§5.2 family 5(a)).
///
/// **Group X has NO registered cross-seat read** — unlike S4's in-hand branch and
/// S7 (which read every seat's President) and O1's pool (which is a property of
/// the table). What travels here instead is a per-SEAT context field: the lab's
/// unowned-title mask is derived once per seat inside `derive`, so a relabel that
/// moved the rows but not the mask would show up as a Group-X value attached to
/// the wrong seat. That is what this cell is for, and the fixture gives the two
/// seats DIFFERENT masks so the failure would be visible.
#[test]
fn group_x_values_permute_with_a_full_seat_relabel() {
    let rules = Rules::standard();
    let state = group_x_fixture(325, &rules);
    let relabelled = relabel_seats_0_and_1(&state);

    let masks = |state: &GameState| {
        let view = observe(state, PlayerId(0)).expect("seat 0 observes");
        ScoreStructure::derive(&view, &rules).context.lab_unowned
    };
    let before_masks = masks(&state);
    assert_ne!(
        before_masks[0], before_masks[1],
        "the two seats must own different titles, or the mask's travel is invisible"
    );
    let after_masks = masks(&relabelled);
    assert_eq!(after_masks[0], before_masks[1], "the unowned mask travels with the seat");
    assert_eq!(after_masks[1], before_masks[0], "and so does the other");

    let before = group_x_row(&state, &rules);
    let after = group_x_row(&relabelled, &rules);
    assert_ne!(before[0], before[1], "the two seats must differ, or the permutation is invisible");
    assert_eq!(after[0], before[1], "seat 0 of the relabeled table is the old seat 1");
    assert_eq!(after[1], before[0], "seat 1 of the relabeled table is the old seat 0");
    println!("seat permutation: {before:?} -> {after:?}");
}

/// §7 audit part 2(b) for Group X: **CROSS-VIEW NEGATION at 2P**, on §5.2's
/// constructible no-hidden-slot class.
///
/// With nothing masked, every Group-X value is a function of public state alone
/// and must read IDENTICALLY from either seat's view; the registered identity —
/// that the relative difference negates — follows and is asserted in its own
/// right because it is the form §5.2(b) names.
///
/// All three features are required to separate the seats, which is what keeps
/// the negation from holding trivially on a row of zeros.
#[test]
fn group_x_differences_negate_across_the_two_2p_views() {
    let rules = Rules::standard();
    let state = group_x_fixture(327, &rules);
    let views = [
        observe(&state, PlayerId(0)).expect("seat 0 observes"),
        observe(&state, PlayerId(1)).expect("seat 1 observes"),
    ];
    for view in &views {
        for player in &view.players {
            assert!(
                !player.hand.iter().any(|c| matches!(c, HandCard::HiddenStartingOrder)),
                "the fixture must carry no masked slot: seat {:?}",
                player.id
            );
        }
    }

    for (v, view) in views.iter().enumerate() {
        let structure = ScoreStructure::derive(view, &rules);
        let other = &views[1 - v];
        let other_structure = ScoreStructure::derive(other, &rules);
        for seat in 0..2u8 {
            let here = x_values(&structure, view, &row_of(view, PlayerId(seat)), &rules);
            let there =
                x_values(&other_structure, other, &row_of(other, PlayerId(seat)), &rules);
            assert_eq!(here, there, "seat {seat} reads differently from the two views");
        }
    }

    let rows = group_x_row(&state, &rules);
    let mut moved = 0usize;
    for (f, (mine, theirs)) in rows[0].iter().zip(rows[1].iter()).enumerate() {
        assert_eq!(mine - theirs, -(theirs - mine), "feature {f} does not negate across the views");
        if mine != theirs {
            moved += 1;
        }
    }
    assert_eq!(moved, 3, "only {moved} of the three features separated the seats");
    // THE FIELD PINS, the same discipline the Group-E row carries: every cell
    // above reads one feature on a fixture built for it, so a helper reading the
    // wrong field of the right kind would still pass them. Here both rows are
    // pinned exactly, and the arithmetic is spelled out in the message.
    assert_eq!(
        rows[0],
        [5, 4, 2],
        "seat 0: five of six slots clear at budget 3; LOW alone at power 2; min(tea 3, need 2)"
    );
    assert_eq!(
        rows[1],
        [0, 14, 1],
        "seat 1: an empty purse clears nothing; all six offers at power 4; min(tea 2, need 1)"
    );
}

/// §7 audit part 2(c) for Group X: the **DECLARED-ASYMMETRY pin for X3**, the
/// group's one Known-only reader (§5.2's (c) clause names it in that list).
///
/// The gap is computed INDEPENDENTLY, by evaluating the same helper on the
/// holder's OWN row with the masked card removed from the hand — so the identity
/// is not "the difference is some difference" but "the masking view reads
/// exactly what the holder would read without that card".
///
/// X1 and X2 are deliberately not in the list and the cell says why by asserting
/// it: X1 reads no hand at all, and X2 reads the hand's LENGTH, which is public
/// for every seat (`observe` masks a starting order's IDENTITY, never its slot).
/// Both therefore read the SAME from either view, and that equality is the
/// scoping statement.
#[test]
fn the_masked_slot_term_is_exactly_the_cross_view_gap_for_x3() {
    let rules = Rules::standard();
    const MASKED: OrderCardId = OrderCardId(7); // 1 copper + 3 tea
    const OPEN: OrderCardId = OrderCardId(5); // 1 copper + 2 tea
    let mut state = cleared_order_table(2, 329, &rules);
    let tea = area_typed(&state, AreaType::TeaPlantation);
    place_assistants(&mut state, 1, tea, 2); // power 3 -> tea export 3
    set_stocks(&mut state, 1, [0, 0, 0, 0]);
    unfulfilled_starting_order(&mut state, 1, MASKED);
    deal_card(&mut state, 1, OPEN);

    let mine = observe(&state, PlayerId(1)).expect("seat 1 observes");
    let my_structure = ScoreStructure::derive(&mine, &rules);
    let full = row_of(&mine, PlayerId(1));
    let mut without = full.clone();
    without.hand.retain(|c| *c != HandCard::Known(MASKED));
    assert_eq!(without.hand.len(), full.hand.len() - 1, "exactly one card removed");

    let theirs = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let their_structure = ScoreStructure::derive(&theirs, &rules);
    let masked_row = row_of(&theirs, PlayerId(1));
    assert!(
        masked_row.hand.contains(&HandCard::HiddenStartingOrder),
        "seat 1's starting order must still be unfulfilled"
    );

    let with = my_structure.matched_production_access(&mine, &full, &rules);
    let hidden = my_structure.matched_production_access(&mine, &without, &rules);
    let term = with - hidden;
    assert_ne!(term, 0, "the masked card contributes nothing — the pin is vacuous");
    assert_eq!(
        their_structure.matched_production_access(&theirs, &masked_row, &rules),
        hidden,
        "X3: the masking view must read the hand WITHOUT the masked card"
    );
    assert_eq!(
        their_structure.matched_production_access(&theirs, &masked_row, &rules) - with,
        -term,
        "X3: the cross-view gap is exactly the masked slot's own term"
    );

    // The scoping half: the other two read the same from either view.
    assert_eq!(
        my_structure.lab_takeable(&mine, &full),
        their_structure.lab_takeable(&theirs, &masked_row),
        "X1 reads no hand — a masked slot cannot move it"
    );
    assert_eq!(
        my_structure.port_takeable(&mine, &full, &rules),
        their_structure.port_takeable(&theirs, &masked_row, &rules),
        "X2 reads the hand's public LENGTH, which a masked slot does not change"
    );
}

/// §3.2's ALLOCATION CONTRACT, the Group-X half: every helper is
/// allocation-free.
///
/// `score`'s own allocs==0 pin covers `ScoreStructure::derive`'s lab pass — the
/// context is derived once per call whether or not anything reads it. This cell
/// covers the READERS, which that pin cannot until §7's step 8 moves
/// `Feature::ALL`.
///
/// 4P for the widest table: TWO lab boards and TWO port boards, so both the
/// per-board loops and the second board's slots are measured rather than
/// short-circuited.
#[test]
fn every_group_x_helper_allocates_nothing() {
    let rules = Rules::standard();
    let mut state = cleared_order_table(4, 331, &rules);
    let lab_a = area_typed(&state, AreaType::LaboratoryA);
    let lab_b = area_typed(&state, AreaType::LaboratoryB);
    let port_a = area_typed(&state, AreaType::PortA);
    let port_b = area_typed(&state, AreaType::PortB);
    let production = production_areas(&state);
    let techs = distinct_effect_techs(&rules, 12, 3, &[]);
    set_lab(&mut state, 0, [
        Some(techs[0]),
        Some(techs[1]),
        Some(techs[2]),
        Some(techs[3]),
        Some(techs[4]),
        Some(techs[5]),
    ]);
    set_lab(&mut state, 1, [
        Some(techs[6]),
        Some(techs[7]),
        Some(techs[8]),
        Some(techs[9]),
        Some(techs[10]),
        Some(techs[11]),
    ]);
    set_port(&mut state, 0, [
        Some(OrderCardId(1)),
        Some(OrderCardId(2)),
        Some(OrderCardId(3)),
        Some(OrderCardId(4)),
        Some(OrderCardId(5)),
        Some(OrderCardId(6)),
    ]);
    set_port(&mut state, 1, [
        Some(OrderCardId(8)),
        Some(OrderCardId(9)),
        Some(OrderCardId(11)),
        Some(OrderCardId(12)),
        Some(OrderCardId(13)),
        Some(OrderCardId(14)),
    ]);
    for area in [lab_a, lab_b, port_a, port_b] {
        place_assistants(&mut state, 0, area, 2);
    }
    for area in production.iter().take(4) {
        place_assistants(&mut state, 0, *area, 1);
    }
    state.players[0].money = 9;
    state.players[0].imported_goods = 2;
    set_stocks(&mut state, 0, [1, 1, 1, 1]);
    deal_card(&mut state, 0, OrderCardId(7));
    // A SECOND, masked row so the hidden-slot paths are measured too.
    unfulfilled_starting_order(&mut state, 1, OrderCardId(27));

    let view = observe(&state, PlayerId(0)).expect("seat 0 observes");
    let structure = ScoreStructure::derive(&view, &rules);
    let player = row_of(&view, PlayerId(0));
    let masked = row_of(&view, PlayerId(1));
    assert!(masked.hand.contains(&HandCard::HiddenStartingOrder), "the masked row is masked");

    let (sum, stats) = meter::measure(|| {
        let read = |p: &VisiblePlayer| {
            structure.lab_takeable(&view, p)
                + structure.port_takeable(&view, p, &rules)
                + structure.matched_production_access(&view, p, &rules)
        };
        read(&player) + read(&masked)
    });
    assert_eq!(stats.allocs, 0, "a Group-X helper allocated ({} allocations)", stats.allocs);
    assert!(sum > 0, "the fixture must exercise real values, or the meter measured nothing");
    println!("Group X: {sum} summed over 4P, {} allocs / {} peak bytes", stats.allocs, stats.peak_bytes);
}
