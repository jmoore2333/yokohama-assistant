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
    achievement_race_proximity,
    achievement_progress_sum as prefreeze_achievement_sum, clock_ppm, explain,
    initial_order_deck_len, interpolate, order_cards_to_remove, place_signal,
    progress_ppm, raw_value, score, terminal_standing, track_trigger, view_is_terminal,
    Explanation, Feature, ScoreStructure, WeightTable, PROGRESS_FULL_PPM, TERMINAL_MARGIN_CLAMP,
    TERMINAL_MARGIN_STEP, TERMINAL_RANK_STEP,
};
use yokohama_core::{
    achievement_progress, apply_move_unchecked, final_result, finalize_setup, is_game_over,
    new_game, observe, ChurchSlot, ChurchTrack, CustomsSlot, CustomsTrack, DecisionHandle,
    GameConfig, GameState, OrderCardId, PlayerId, PlayerView, PortBoard, RolloutPolicy, Rules,
    ScoreBreakdown, ShophouseState, StartingOrderChoice, TradingHouseState, TurnPhase,
    VisiblePlayer,
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
/// and trimmed. `Rules` is the one permitted engine dependency: `score` needs
/// the order registry's size for §2.3's deck clock and T2's mirrors need the
/// order/tech registries. Widening this list is a deliberate act with grounds —
/// which is the whole point of asserting it (spec §2.1: the crate-internal
/// module COULD import anything, so the guard is executable, not a compiler
/// fact).
const GAME_IMPORT_ALLOWLIST: [&str; 1] = ["use crate::game::rules::Rules;"];

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
///
/// `achievement_race_proximity` MEASURED at its admission (2026-08-19, V3 T4) on
/// this same sweep: nonzero on **1,584 of 1,598 views**, floored at 792 by the
/// same measure-then-halve discipline. It is the densest arm in the table and
/// that is structural rather than lucky — the value is zero only for a seat
/// standing on a claimable card or one that has claimed all three, so a floor
/// here is a check that the sweep RAN, not that the event is rare.
//     [vp,  ctry, tech, chur, cust, hCORE, hEXC, ware, brd, shop, trad, 5pw, imp, achv, merc, hnd#, prox]
const FEATURE_NONZERO_FLOORS: [usize; Feature::COUNT] =
    [689, 489, 375, 387, 84, 634, 133, 855, 835, 431, 30, 27, 120, 809, 317, 823, 792];

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
// V3 — `achievement_race_proximity`, the seventeenth feature
//
// The arithmetic MOVED here from `examples/eval_probe.rs` at the V3 adoption
// (spec §5, the C2 contraction): a candidate column that graduates becomes ONE
// authored function in `eval.rs` and the probe's column re-points at it, so the
// screened arithmetic and the shipped arithmetic can never be two things. The
// probe's own fixture pins stay where they are and now read through this
// function; these are the shipped-side pins.
// ---------------------------------------------------------------------------

/// A 4P fixture whose seats carry KNOWN, unequal gaps toward the three cards in
/// play, so "the NEAREST unclaimed gap" has something to choose between and a
/// claim has something to remove. Built on [`postclaim_fixture`]'s board for
/// its reason (the AND-shaped legs need five production areas), then given
/// three DIFFERENT cards rather than three copies of one.
fn proximity_fixture(rules: &Rules) -> GameState {
    let mut state = postclaim_fixture(&rules.clone(), yokohama_core::AchievementType::Copper5);
    state.achievements = [
        // Seat 0 holds 7 money (postclaim_fixture) -> gap 3.
        yokohama_core::AchievementCard::new(yokohama_core::AchievementType::Money10),
        // Seat 0 holds 9 copper -> over the cap, gap 0 is NOT wanted here, so
        // the third card carries the nearest-but-nonzero leg instead.
        yokohama_core::AchievementCard::new(yokohama_core::AchievementType::Orders5),
        yokohama_core::AchievementCard::new(yokohama_core::AchievementType::Technologies4),
    ];
    state
}

/// The helper IS the nearest unclaimed gap, negated — computed from the mirror
/// rather than hand-typed, so a cap change cannot leave this asserting a stale
/// constant. Every seat, so the claim is about the arithmetic and not about one
/// lucky row.
#[test]
fn the_race_proximity_helper_is_the_nearest_unclaimed_gap_negated() {
    let rules = Rules::standard();
    let state = proximity_fixture(&rules);
    for seat in 0..state.config.player_count {
        let id = PlayerId(seat);
        let view = observe(&state, id).unwrap();
        let me = seat_of(&view, id);
        let expected = view
            .achievements
            .iter()
            .filter(|card| !card.claimed_by.contains(&id))
            .map(|card| {
                let (current, needed) =
                    view_achievement_progress(&view, me, card.achievement_type);
                i64::from(needed.saturating_sub(current.min(needed)))
            })
            .min()
            .unwrap_or(0);
        assert_eq!(
            achievement_race_proximity(&view, me),
            -expected,
            "seat {seat}: the column is minus the SMALLEST remaining gap",
        );
    }
}

/// The viewer's OWN claim drops that card's leg and the next-nearest takes
/// over; an OPPONENT'S claim leaves it live (the engine's claim gate refuses
/// only a repeat claim by the same seat, so an opponent's card is still worth
/// its printed subsequent value here).
#[test]
fn a_self_claim_drops_a_leg_from_the_race_while_an_opponent_claim_leaves_it_live() {
    let rules = Rules::standard();
    let base = proximity_fixture(&rules);
    let view = observe(&base, PlayerId(0)).unwrap();
    let before = achievement_race_proximity(&view, seat_of(&view, PlayerId(0)));

    // Which card is currently NEAREST — the one a self-claim must remove.
    let nearest = view
        .achievements
        .iter()
        .min_by_key(|card| {
            let (current, needed) =
                view_achievement_progress(&view, seat_of(&view, PlayerId(0)), card.achievement_type);
            needed.saturating_sub(current.min(needed))
        })
        .expect("three cards are in play")
        .achievement_type;

    let mut opponent_claimed = base.clone();
    for card in opponent_claimed.achievements.iter_mut() {
        if card.achievement_type == nearest {
            card.claimed_by.push(PlayerId(1));
        }
    }
    let view = observe(&opponent_claimed, PlayerId(0)).unwrap();
    assert_eq!(
        achievement_race_proximity(&view, seat_of(&view, PlayerId(0))),
        before,
        "an opponent's claim must not move this seat's race — the card is still claimable",
    );

    let mut self_claimed = base.clone();
    for card in self_claimed.achievements.iter_mut() {
        if card.achievement_type == nearest {
            card.claimed_by.push(PlayerId(0));
        }
    }
    let view = observe(&self_claimed, PlayerId(0)).unwrap();
    let after = achievement_race_proximity(&view, seat_of(&view, PlayerId(0)));
    assert!(
        after < before,
        "the viewer's own claim must drop the nearest leg, leaving a strictly larger gap \
         (before {before}, after {after})",
    );
}

/// The FEATURE reads the helper, for every seat, through the structure the
/// score path derives once — the drift-pin idiom applied to the new column.
#[test]
fn the_race_proximity_feature_reads_the_helper_at_every_seat() {
    let rules = Rules::standard();
    let state = proximity_fixture(&rules);
    let mut any_nonzero = false;
    for seat in 0..state.config.player_count {
        let id = PlayerId(seat);
        let view = observe(&state, id).unwrap();
        let structure = ScoreStructure::derive(&view, &rules);
        for player in &view.players {
            any_nonzero |= achievement_race_proximity(&view, player) != 0;
            assert_eq!(
                structure.value_of(player, Feature::AchievementRaceProximity),
                achievement_race_proximity(&view, player),
                "the structure's proximity row must be the helper's answer",
            );
        }
        let me = seat_of(&view, id);
        assert_eq!(
            raw_value(&view, me, Feature::AchievementRaceProximity, &rules),
            achievement_race_proximity(&view, me),
            "the probe-facing accessor must agree with the structure",
        );
    }
    assert!(
        any_nonzero,
        "every proximity read zero, so the agreement above is two zeroes agreeing",
    );
}

/// **Proximity is not points.** `final_total` is the quantity §2.6 RANKS, and a
/// positional nearness term inside it would move a finished game's standings.
/// Asserted as an identity against the six components rather than as a wish.
#[test]
fn the_race_proximity_column_is_not_a_component_of_the_final_total() {
    let rules = Rules::standard();
    let state = proximity_fixture(&rules);
    let view = observe(&state, PlayerId(0)).unwrap();
    let structure = ScoreStructure::derive(&view, &rules);
    let mut any_nonzero = false;
    for seat in 0..usize::from(state.config.player_count) {
        assert_eq!(
            structure.final_total(seat),
            structure.base_vp[seat]
                + structure.country[seat]
                + structure.tech[seat]
                + structure.church[seat]
                + structure.customs[seat]
                + structure.remainder[seat],
            "seat {seat}: the final total is the six SCORING components and nothing else",
        );
        any_nonzero |= structure.race_proximity[seat] != 0;
    }
    assert!(any_nonzero, "the fixture must carry a nonzero proximity, or this pins nothing");
}

/// **The retention contract, measured** (spec §5): a `(0, 0)` seventeenth slot
/// contributes a zero product at every stage, so both retained tables score
/// exactly what they scored before the width move. Over organic views of a real
/// game, both tables, every seat — the totals are the sixteen products they
/// always were.
#[test]
fn the_padded_seventeenth_slot_is_inert_in_both_retained_tables() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 31, &rules);
    let mut rng = ChaCha8Rng::seed_from_u64(0x5E17_2026_0819);
    let mut views = 0usize;
    for _ in 0..60 {
        if is_game_over(&state) {
            break;
        }
        for seat in 0..state.config.player_count {
            let view = observe(&state, PlayerId(seat)).unwrap();
            for weights in [WeightTable::V1, WeightTable::V2] {
                let e: Explanation = explain(&view, &rules, &weights);
                assert_eq!(e.terms.len(), Feature::COUNT);
                let padded = e.terms.last().expect("one term per feature");
                assert_eq!(padded.feature, Feature::AchievementRaceProximity);
                assert_eq!(padded.weight, 0, "a retained table prices the new slot at zero");
                assert_eq!(padded.product, 0, "a zero weight contributes a zero product");
                assert_eq!(
                    e.terms[..Feature::COUNT - 1].iter().map(|t| t.product).sum::<i64>(),
                    score(&view, &rules, &weights),
                    "the score is the SIXTEEN products it always was",
                );
            }
            views += 1;
        }
        let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
        let handle = DecisionHandle::new(&state, &rules);
        let Some(mv) = policy.pick(&handle, &mut rng) else { break };
        state = apply_move_unchecked(&state, &mv, &rules).expect("a sampled move applies");
    }
    assert!(views >= 20, "the inertness pin barely ran: {views} views");
    // ANTI-VACUITY: the slot is inert because its WEIGHT is zero, not because
    // its VALUE is. A table that PRICES it moves the score — on a position where
    // the seats' proximities actually differ, since every term is relative and
    // two seats racing equally cancel by construction.
    let view = observe(&proximity_fixture(&rules), PlayerId(0)).unwrap();
    let me = seat_of(&view, PlayerId(0));
    assert_ne!(
        raw_value(&view, me, Feature::AchievementRaceProximity, &rules),
        0,
        "the fixture's viewer must actually be chasing something",
    );
    assert_ne!(
        score(&view, &rules, &WeightTable::V3_DRAFT),
        score(&view, &rules, &WeightTable::V2),
        "the draft PRICES the slot, so it must not score identically to padded V2",
    );
}
