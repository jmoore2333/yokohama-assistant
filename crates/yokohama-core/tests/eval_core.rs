//! Per-edit tests for evaluator v1's core (Phase 3.3 Task T1) — spec §2.1's
//! source-text guard, §2.2's relative form, §2.3's stage interpolation, and
//! §0.3(a)'s two-path `score`/`explain` requirement.
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
//! Tier: per-edit, never `#[ignore]`d. Measured at the T1 close (debug, three
//! runs): 0.16 / 0.16 / 0.16 s — the organic sweep (three full games plus three
//! capped ones, ~3,600 applied moves) is nearly the whole cost. It is that
//! cheap because it does NOT run `check_invariants` per move: the engine is not
//! this file's subject and four other per-edit suites already carry that net.

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
    clock_ppm, explain, initial_order_deck_len, interpolate, order_cards_to_remove, progress_ppm,
    raw_value, score, track_trigger, Explanation, Feature, WeightTable, PROGRESS_FULL_PPM,
};
use yokohama_core::{
    apply_move_unchecked, finalize_setup, is_game_over, new_game, observe, DecisionHandle,
    GameConfig, GameState, PlayerId, PlayerView, PortBoard, RolloutPolicy, Rules, ShophouseState,
    StartingOrderChoice, TradingHouseState, VisiblePlayer,
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
///
/// MUTATION-PROVEN AT AUTHORING (both arms, each reverted, observed lines
/// quoted verbatim):
///   - added `use crate::types::GameState;` to `eval.rs` — FAILED:
///     `eval.rs:63 names the forbidden identifier `GameState` — the evaluator
///     is a function of a PlayerView (spec §2.1). Line: use
///     crate::types::GameState;`
///   - added `use crate::game::scoring::calculate_remainder_bonus;` — FAILED:
///     ``eval.rs:63` reaches into `crate::game::` outside the allowlist ["use
///     crate::game::rules::Rules;"]. Line: use
///     crate::game::scoring::calculate_remainder_bonus;`
#[test]
fn eval_source_text_names_no_engine_state_and_only_allowlisted_game_imports() {
    let src = include_str!("../src/game/eval.rs");

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
        // Substring, not `starts_with("use ")`: an inline path
        // (`crate::game::scoring::foo(..)`) is the same reach-around with no
        // `use` line to catch, so the scan is over OCCURRENCES.
        if line.contains("crate::game::") {
            assert!(
                GAME_IMPORT_ALLOWLIST.contains(&trimmed),
                "`eval.rs:{n}` reaches into `crate::game::` outside the allowlist \
                 {GAME_IMPORT_ALLOWLIST:?}. Line: {trimmed}"
            );
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
    let _ = score(&view, &empty, &WeightTable::PROVISIONAL_V1);
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
    let weights = WeightTable::PROVISIONAL_V1;
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
            // The proxy is a function of PUBLIC state only, so every seat must
            // read the same clock. (A term that leaked a seat-relative
            // quantity into the proxy would show up here.)
            assert!(
                proxies.windows(2).all(|w| w[0] == w[1]),
                "the stage proxy is seat-relative at {players}P seed {seed} step {step}: {proxies:?}"
            );
            let p = proxies[0];
            // Monotone over SIM play: the deck only shrinks, tracks and
            // buildings only fill. (The live-tracking reveal path can hand
            // boxed cards BACK to the deck via `arrange_orders_on_top`, so
            // this is a property of self-play, not of the type.)
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

        // T1 staging (spec §2.2 + this task's brief): the five score-structure
        // features are declared, weighted 0 in BOTH vectors, and compute 0
        // until T2 lands the view-side re-derivations.
        if term.feature.is_score_structure() {
            assert_eq!((early, late), (0, 0), "{} must be staged at 0", term.feature.name());
            assert_eq!(term.viewer_value, 0, "{} is staged", term.feature.name());
            assert_eq!(term.product, 0, "{} is staged", term.feature.name());
        }
    }

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
/// MEASURED at the T1 close, and the floors are set at roughly HALF each
/// measured count (the sweep is fully deterministic — fixed seeds, seeded
/// ChaCha8 — so the figures are exact, and the halving is headroom for benign
/// drift rather than protection against flake):
///
/// ```text
/// views 1711 · early_zero 132 · late_half 1134 · full 3
/// clocks [deck 1462, church 1466, customs 390, building 1216]
/// nonzero_terms 8648 · negative_weight_picks 1767 · decision points 537
/// ```
///
/// MUTATION-PROVEN AT AUTHORING (both reverted; the point is that this sweep
/// catches the two slips it exists for, not merely that `assert!` works):
///   - `score` skipping one feature (`if feature == Feature::HandSize
///     { continue; }`) — FAILED: `assertion left == right failed:
///     explain().total != score(); left: 68, right: 62`. That is the §0.3(a)
///     exact-sum property doing its job across two assemblies.
///   - `opponent_extreme` comparing RAW values instead of `weight · value`
///     (i.e. dropping the orientation rule) — FAILED: `hand_size: a negative
///     weight must take the opponents' minimum; left: 2, right: 1`.
#[test]
fn explain_reconstructs_score_exactly_over_organic_views() {
    let rules = Rules::standard();
    let weights = WeightTable::PROVISIONAL_V1;
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
}
