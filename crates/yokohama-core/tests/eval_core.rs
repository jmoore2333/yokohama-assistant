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

use yokohama_core::game::eval::{
    achievement_progress as view_achievement_progress, achievement_progress_postclaim_sum,
    achievement_progress_sum as prefreeze_achievement_sum, area_bit, areas_in, clock_ppm, explain,
    initial_order_deck_len, interpolate, order_cards_to_remove, place_signal,
    progress_ppm, raw_value, realizable_power, score, terminal_standing, track_trigger,
    view_is_terminal, Explanation, Feature, ScoreStructure, WeightTable, PROGRESS_FULL_PPM,
    TERMINAL_MARGIN_CLAMP, TERMINAL_MARGIN_STEP, TERMINAL_RANK_STEP,
};
use yokohama_core::{
    achievement_progress, apply_move_unchecked, calculate_power, final_result, finalize_setup,
    get_legal_moves, is_game_over, is_possible_action_area_full, new_game, observe, AreaId,
    AreaType, ChurchSlot, ChurchTrack, CustomsSlot, CustomsTrack, DecisionHandle, GameConfig,
    GameState, HandCard, Move, OrderCardId, PlayerId, PlayerView, PortBoard, PresidentLocation,
    RolloutPolicy, Rules, ScoreBreakdown, ShophouseState, StartingOrderChoice, TradingHouseState,
    TurnPhase, VisiblePlayer,
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
/// and its reward at the power the seat would arrive with is asserted nonzero,
/// so "the gate excluded it" and "the gate was never tested" cannot look alike.
#[test]
fn production_access_is_the_engines_exported_reward_at_realizable_power() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 81, &rules);
    let areas = production_areas(&state);
    assert_eq!(areas.len(), 4, "a 2P board carries one tile of each production type");

    // Three occupied at different strengths, one deliberately empty.
    place_assistants(&mut state, 0, areas[0], 1);
    place_assistants(&mut state, 0, areas[1], 2);
    place_assistants(&mut state, 0, areas[2], 4);
    assert_eq!(token_stock(&state, 0), (20, 8, 4), "the fixture moved pawns; it must not invent them");

    let expected: i64 = read_s(&state, &rules, PlayerId(0), |_, view, player| {
        let mut total = 0i64;
        for area in &areas[..3] {
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
    let unoccupied_would_pay = read_s(&state, &rules, PlayerId(0), |_, view, player| {
        let power = realizable_power(view, player, areas[3]);
        yokohama_core::actions::exported_reward(view.board_layout.areas[&areas[3]].area_type, power)
            .expect("a production tile is in the dispatcher's domain")
    });
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |_, _, player| player.assistants.in_area(areas[3])),
        0,
        "the fourth tile is the ungated control and must be empty"
    );
    println!(
        "S1 = {s1}; the ungated fourth tile would have paid {unoccupied_would_pay} at its arriving power"
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
            s.employment_agency_access(view, player)
        })
    };
    assert_eq!(read(&before), 3, "three assistants plus the arriving president allows three tokens");
    assert_eq!(read(&after), 0, "the drained agency no longer satisfies the presence gate");
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

    // No board assistants at all: the declared 0.
    assert_eq!(
        read_s(&state, &rules, PlayerId(0), |s, _, p| s.president_local_access(p)),
        0,
        "a seat with no board presence has no re-entry"
    );

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

/// Every function a Group-S read executes, by its source-text head. The list is
/// the SUBJECT of the two scans below, and it is hand-maintained on purpose: a
/// new S helper that is not added here is a helper neither scan covers, which is
/// exactly the omission an eighth feature would make silently.
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
    const FORBIDDEN_READS: [&str; 8] = [
        ".money",
        ".imported_goods",
        ".hand",
        ".completed_orders",
        ".copper",
        ".silk",
        ".tea",
        ".seafood",
    ];
    for head in S_HELPER_HEADS {
        let source = s_helper_source(head);
        for needle in FORBIDDEN_READS {
            assert!(
                !source.contains(needle),
                "`{head}` reads `{needle}` — Group S prices presence, power and position; \
                 money/goods/orders belong to Groups E and O, and only Group X reads both"
            );
        }
    }
    // The needles are real field reads somewhere, or this scan proves nothing.
    let control = s_helper_source("fn remainder_bonus(");
    let found = FORBIDDEN_READS.iter().filter(|n| control.contains(**n)).count();
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
/// # WHAT THIS CELL DOES NOT DO, and why — carried forward
///
/// §7's contract names a SECOND comparator for this row: the harness's own
/// view-side mirror, `visible_power`. It is a PRIVATE function of
/// `driven_families.rs` (declared at :1489), and this batch's charter fences
/// every `driven*` file against the adjudicator lane, so the comparison cannot
/// be written here without a one-token visibility change this batch may not
/// make. The pin is therefore OWED FORWARD, with the unblock recorded: make
/// `visible_power` `pub`, then assert it against this reading on
/// president-at-the-area fixtures. The content lost meanwhile is small and
/// worth stating exactly — `visible_power` is a term-for-term mirror of
/// `calculate_power`, which is the authority this cell already compares
/// against, and it carries no drift pin of its own today, so the cross-pin's
/// real value is that it would give the HARNESS mirror one.
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
