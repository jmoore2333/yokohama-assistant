//! Standing honesty suites for Phase 3.0 (spec §4, T1/T3/T4/T5/T6).
//!
//! Two tiers, per the repo's gate policy: everything here runs per-edit at a
//! capped size, and `observation_honesty_release_sweep` re-runs it uncapped
//! at 2/3/4P as a release gate.
//!
//! ANTI-VACUITY IS PER FAMILY, not global. Several variant families degenerate
//! exactly where games spend their time -- a deck reshuffle is a no-op once
//! the deck is empty, and the deck emptying IS how nearly every game ends --
//! so a bare "we made thousands of comparisons" would certify a suite that
//! varied nothing.
//!
//! VIEWER = CURRENT PLAYER (load-bearing, do not "simplify" to seat 0). The
//! sampler only guarantees the VIEWER's hand is exact; opponents' hands are
//! materialized guesses. `get_legal_moves` computes the CURRENT player's legal
//! list, which reads the current player's hand -- so T4-on-samples
//! (`get_legal_moves(world) == legal`) holds only because the seat we observe
//! from IS the current player. For the same reason `hidden_variants` takes the
//! viewer: the `hidden_order_swap` family must perturb a NON-viewer opponent,
//! or it would rewrite the observing seat's OWN visible starting order and fail
//! T1 for a reason that is not an honesty leak.

use std::collections::HashMap;

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game,
    observe, run_honest_game, view_digest, AgentPathTally, BeliefSampler, DecisionHandle,
    FamilySize, GameConfig, GameState, Move, MoveFamily, PlayerId, Rules, StartingOrderChoice,
    TrivialHonestAgent,
};

// Cross-pin (Task 6 carry-forward): the engine's honest-driver turn cap
// (`HONEST_MAX_TURNS`) and the shared self-play driver's cap (`common::MAX_TURNS`)
// are the same non-termination bound and must move together. `common/mod.rs`
// carries `#![allow(dead_code)]`, so pulling it in for the one constant is clean.
mod common;

const FAMILIES: [&str; 5] = [
    "deck_reshuffle",
    "deck_removed_repartition",
    "hidden_order_swap",
    "tech_permutation",
    "setup_seed_change",
];

/// Per-edit tier: 2P and 3P, seeds 0-1, capped at 300 decision points.
const PER_EDIT_COUNTS: [u8; 2] = [2, 3];
const PER_EDIT_SEEDS: [u64; 2] = [0, 1];
const PER_EDIT_MAX_STEPS: usize = 300;
const PER_EDIT_FAMILY_FLOOR: usize = 50;
const PER_EDIT_DECK_DIFF_FLOOR: usize = 200;

fn keep_game(players: u8, seed: u64, rules: &Rules) -> GameState {
    let pending = new_game(GameConfig::for_player_count(players), rules, seed).unwrap();
    let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
    finalize_setup(pending, &choices).unwrap()
}

/// Variants of `state` that differ ONLY in what `viewer` cannot see. Sizes are
/// always preserved, so every variant is a state the seat could not rule out.
fn hidden_variants(
    state: &GameState,
    viewer: PlayerId,
    rng: &mut ChaCha8Rng,
) -> Vec<(&'static str, GameState)> {
    use rand::seq::SliceRandom;
    let mut out = Vec::new();

    let mut a = state.clone();
    a.order_deck.shuffle(rng);
    out.push(("deck_reshuffle", a));

    let mut b = state.clone();
    {
        let mut pool: Vec<_> = b.order_deck.iter().chain(b.orders_removed.iter()).copied().collect();
        pool.shuffle(rng);
        let d = b.order_deck.len();
        b.order_deck = pool[..d].to_vec();
        b.orders_removed = pool[d..].to_vec();
    }
    out.push(("deck_removed_repartition", b));

    let mut c = state.clone();
    {
        // Swap an OPPONENT's unfulfilled starting order with a boxed card. The
        // opponent is relative to the actual `viewer` -- perturbing the viewer's
        // own deal would move something the viewer legitimately sees.
        let opp = c.players.iter().position(|p| {
            p.id != viewer
                && p.starting_order.is_some_and(|s| p.order_cards_in_hand.contains(&s))
        });
        if let (Some(oi), false) = (opp, c.orders_removed.is_empty()) {
            let old = c.players[oi].starting_order.unwrap();
            let bi = rng.gen_range(0..c.orders_removed.len());
            let new = c.orders_removed[bi];
            let hi = c.players[oi].order_cards_in_hand.iter().position(|x| *x == old).unwrap();
            c.players[oi].order_cards_in_hand[hi] = new;
            c.players[oi].starting_order = Some(new);
            c.orders_removed[bi] = old;
        }
    }
    out.push(("hidden_order_swap", c));

    let mut d = state.clone();
    d.tech_deck.shuffle(rng);
    out.push(("tech_permutation", d));

    let mut e = state.clone();
    e.config.setup_seed = e.config.setup_seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    out.push(("setup_seed_change", e));

    out
}

struct Tally {
    effective: HashMap<&'static str, usize>,
    deck_differing: usize,
    comparisons: usize,
}

/// The whole T1/T4/T5/T6 body, run at a given size. Returns the tally so both
/// tiers can assert their own floors.
fn run_honesty_sweep(
    counts: &[u8],
    seeds: &[u64],
    max_steps: Option<usize>,
    sample_every: usize,
    rules: &Rules,
) -> Tally {
    let mut tally = Tally {
        effective: FAMILIES.iter().map(|f| (*f, 0usize)).collect(),
        deck_differing: 0,
        comparisons: 0,
    };

    for &players in counts {
        for &seed in seeds {
            let mut state = keep_game(players, seed, rules);
            let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xA5A5);
            let mut step = 0usize;

            while !is_game_over(&state) {
                if let Some(cap) = max_steps {
                    if step >= cap {
                        break;
                    }
                }
                let legal = get_legal_moves(&state, rules);
                assert!(!legal.is_empty(), "dead end at {players}P seed {seed} step {step}");

                if step.is_multiple_of(sample_every) {
                    let seat = state.current_player;
                    let view = observe(&state, seat).unwrap();
                    let digest = view_digest(&view);

                    // T1: no hidden variation may move the view.
                    for (family, variant) in hidden_variants(&state, seat, &mut rng) {
                        if variant != state {
                            *tally.effective.get_mut(family).unwrap() += 1;
                        }
                        let vv = observe(&variant, seat).unwrap();
                        assert_eq!(vv, view, "T1 {family}: hidden state leaked into the view");
                        assert_eq!(view_digest(&vv), digest, "T1 {family}: digest moved");

                        // T4: legality is view-measurable -- the variant's legal
                        // list must be identical, order included.
                        assert_eq!(
                            get_legal_moves(&variant, rules),
                            legal,
                            "T4 {family}: hidden state moved the legal list"
                        );
                    }

                    // T3/T5/T6 over sampled worlds.
                    let mut sampler =
                        BeliefSampler::from_view(&view, rules, 77 + step as u64).unwrap();
                    for _ in 0..3 {
                        let world = sampler.sample();
                        tally.comparisons += 1;
                        if world.order_deck != state.order_deck {
                            tally.deck_differing += 1;
                        }

                        // T5: exact multiset partition + a valid state.
                        assert_multiset_partition(&world, rules);
                        assert!(check_invariants(&world, rules).is_ok(), "T5: sampled world invalid");
                        let mut canon = world.orders_removed.clone();
                        canon.sort_by_key(|c| c.0);
                        assert_eq!(world.orders_removed, canon, "T5: box must be canonical");

                        // T4 again, on the sample this time.
                        assert_eq!(
                            get_legal_moves(&world, rules),
                            legal,
                            "T4: a sampled world's legal list must equal the true one"
                        );

                        // T6a: round-trip.
                        assert_eq!(observe(&world, seat).unwrap(), view, "T6a: sample changed the view");

                        // T6b: public fidelity (Keep-shaped truth only -- every
                        // game in this corpus is Keep-shaped; the LIVE cells are
                        // covered by the live matrix test).
                        assert_public_fidelity(&world, &state);
                    }

                    // T3: determinism from (view, seed), independent of the truth.
                    let mut s1 = BeliefSampler::from_view(&view, rules, 5).unwrap();
                    let mut s2 = BeliefSampler::from_view(&view, rules, 5).unwrap();
                    assert_eq!(s1.sample(), s2.sample(), "T3: same (view, seed) must repeat");
                }

                let pick = legal[rng.gen_range(0..legal.len())].clone();
                state = apply_move_unchecked(&state, &pick, rules).unwrap();
                step += 1;
            }
        }
    }
    tally
}

fn assert_multiset_partition(world: &GameState, rules: &Rules) {
    let mut orders: Vec<u8> = Vec::new();
    orders.extend(world.order_deck.iter().map(|c| c.0));
    orders.extend(world.orders_removed.iter().map(|c| c.0));
    for slot in world.port_state.port_a.slots.iter().flatten() {
        orders.push(slot.0);
    }
    if let Some(b) = &world.port_state.port_b {
        for slot in b.slots.iter().flatten() {
            orders.push(slot.0);
        }
    }
    for p in &world.players {
        orders.extend(p.order_cards_in_hand.iter().map(|c| c.0));
        orders.extend(p.completed_orders.iter().map(|c| c.0));
    }
    orders.sort_unstable();
    let mut expect: Vec<u8> = rules.orders.iter_sorted_by_id().iter().map(|c| c.id.0).collect();
    expect.sort_unstable();
    assert_eq!(orders, expect, "order ids must partition exactly once each");

    let mut techs: Vec<u8> = world.tech_deck.iter().map(|c| c.0).collect();
    for slot in world.lab_state.lab_a.slots.iter().flatten() {
        techs.push(slot.0);
    }
    if let Some(b) = &world.lab_state.lab_b {
        for slot in b.slots.iter().flatten() {
            techs.push(slot.0);
        }
    }
    for p in &world.players {
        techs.extend(p.technologies.iter().map(|c| c.0));
    }
    techs.sort_unstable();
    let mut expect_t: Vec<u8> = rules.techs.iter_sorted_by_id().iter().map(|c| c.id.0).collect();
    expect_t.sort_unstable();
    assert_eq!(techs, expect_t, "tech ids must partition exactly once each");
}

/// T6b: a sampled world must match the TRUE state everywhere a seat can see,
/// after canonicalizing the five hidden members away. Round-tripping the view
/// alone cannot catch a public field that BOTH sides drop or default.
fn assert_public_fidelity(world: &GameState, truth: &GameState) {
    let mut a = world.clone();
    let mut b = truth.clone();
    for s in [&mut a, &mut b] {
        s.config.setup_seed = 0;
        s.order_deck = vec![];
        s.tech_deck = vec![];
        s.orders_removed = vec![];
        for p in &mut s.players {
            // `starting_order` is never public state the sampler can reconstruct.
            // While UNFULFILLED it is hidden member 5, held in hand -- canonicalize
            // that hand slot so an opponent's materialized guess does not read as a
            // public divergence from the real card. Once FULFILLED the card sits in
            // the face-up `completed_orders` while `starting_order` keeps pointing at
            // it (I11 is write-only: no legal move clears it, game/invariants.rs:58),
            // but `observe` masks that residual to `None` for opponents and carries
            // no trace of WHICH completed order was the starting one -- so the sampler
            // legitimately drops it (a `None` residual is I11-valid too). Null the
            // field in BOTH tiers unconditionally so this benign residual, which no
            // seat can observe (T6a passes), never counts against public fidelity.
            if let Some(c) = p.starting_order {
                if let Some(i) = p.order_cards_in_hand.iter().position(|x| *x == c) {
                    p.order_cards_in_hand[i] = yokohama_core::OrderCardId(0);
                }
                p.starting_order = None;
            }
        }
    }
    // Deck SIZES are public and must still match.
    assert_eq!(world.order_deck.len(), truth.order_deck.len(), "T6b: deck size");
    assert_eq!(world.tech_deck.len(), truth.tech_deck.len(), "T6b: tech deck size");
    assert_eq!(a, b, "T6b: a sampled world diverged from truth on PUBLIC state");
}

#[test]
fn observation_honesty_per_edit() {
    let rules = Rules::standard();
    let tally =
        run_honesty_sweep(&PER_EDIT_COUNTS, &PER_EDIT_SEEDS, Some(PER_EDIT_MAX_STEPS), 1, &rules);

    // Anti-vacuity, per family: a suite that varied nothing must not pass.
    for f in FAMILIES {
        let n = tally.effective[f];
        assert!(
            n >= PER_EDIT_FAMILY_FLOOR,
            "family {f} was effective only {n} times (floor {PER_EDIT_FAMILY_FLOOR})"
        );
    }
    assert!(
        tally.deck_differing >= PER_EDIT_DECK_DIFF_FLOOR,
        "only {} samples drew a different deck (floor {PER_EDIT_DECK_DIFF_FLOOR}) -- a truth-copying sampler would pass vacuously",
        tally.deck_differing
    );
    assert!(tally.comparisons > 0);
}

#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): 30 uncapped games, sampling every 25th decision point; run with cargo test --release -p yokohama-core --test observation_honesty -- --ignored before merges/phase closes"]
fn observation_honesty_release_sweep() {
    let rules = Rules::standard();
    let seeds: Vec<u64> = (0..10).collect();
    let tally = run_honesty_sweep(&[2, 3, 4], &seeds, None, 25, &rules);
    for f in FAMILIES {
        let n = tally.effective[f];
        assert!(n >= 500, "family {f} effective only {n} times (release floor 500)");
    }
    assert!(tally.deck_differing >= 2_000, "deck-differing samples: {}", tally.deck_differing);
}

/// Gate item 2 (spec §9 / §4-T7) at the RELEASE tier: the AGENT path, uncapped.
///
/// `observation_honesty_release_sweep` above drives moves by uniform-random on
/// the true state -- it never touches `run_honest_game` or `TrivialHonestAgent`,
/// so it is not evidence that the observe -> sample -> pick loop plays a full
/// game. This test is: it runs `TrivialHonestAgent`s through `run_honest_game`
/// across 2/3/4P x seeds 0..10, uncapped to completion, and asserts each game
/// finished, named an end trigger, and ranked every seat.
///
/// NON-VACUITY (the load-bearing part -- parked finding agent.rs:92). A sweep
/// that completed every game while the sampler silently fell back on every call
/// would prove nothing about the belief path, because the fallback arm resolves
/// to the same true legal list. The per-agent `AgentPathTally` makes the two
/// arms observable. `sampled > 0` pins that the sampler arm actually drove
/// decisions; `fallback == 0` pins that it drove ALL of them -- provable for
/// these all-`Keep` games, where the sampled and true legal lists coincide
/// (spec F5), so a nonzero fallback would be an F5 regression surfacing through
/// the agent path.
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): 30 uncapped agent-driven full games (run_honest_game + TrivialHonestAgent) at 2/3/4P x seeds 0..10; run with cargo test --release -p yokohama-core --test observation_honesty -- --ignored before merges/phase closes"]
fn trivial_honest_agents_complete_full_games_release_sweep() {
    let rules = Rules::standard();
    let mut tallies: Vec<AgentPathTally> = Vec::new();
    let mut games = 0u32;

    for &players in &[2u8, 3, 4] {
        for seed in 0..10u64 {
            let stats = run_honest_game(seed, players, &rules, &mut |seat, _n| {
                let agent =
                    TrivialHonestAgent::new(seed.wrapping_mul(131).wrapping_add(seat.0 as u64));
                tallies.push(agent.tally());
                Box::new(agent)
            })
            .unwrap_or_else(|e| panic!("{players}P seed {seed} did not complete: {e}"));

            assert_eq!(stats.players, players);
            assert!(stats.moves_applied > 0, "{players}P seed {seed}: no moves applied");
            assert!(
                stats.trigger.is_some(),
                "{players}P seed {seed}: a completed game names its end trigger"
            );
            assert_eq!(
                stats.result.standings.len(),
                players as usize,
                "{players}P seed {seed}: standings must rank every seat"
            );
            games += 1;
        }
    }
    assert_eq!(games, 30, "2/3/4P x seeds 0..10 is 30 games");

    let sampled: u64 = tallies.iter().map(|t| t.sampled_pick_count()).sum();
    let fallback: u64 = tallies.iter().map(|t| t.fallback_count()).sum();
    eprintln!(
        "agent-path release sweep: {games} games, {} agents, sampler-arm decisions {sampled}, fallback {fallback}",
        tallies.len()
    );
    assert!(
        sampled > 0,
        "the belief-sampler arm never drove a decision across 30 games -- the sweep is vacuous"
    );
    assert_eq!(
        fallback, 0,
        "these games are all Keep-shaped, where the sampled and true legal lists coincide (F5); \
         a nonzero fallback ({fallback}) means the sampler arm was skipped -- an F5 regression \
         surfacing through the agent path"
    );
}

// ---------------------------------------------------------------------------
// T4, extended to the FACTORED surface (Phase 3.2 Task 11; spec §10.3, §4).
//
// The T4 assertions above measure `get_legal_moves` — the exhaustive list. The
// arena's weighted draw never calls it. What that draw actually reads is the
// factored surface: which family TAGS `families()` emits, each family's SIZE
// (the two exact sizes at a Movement point ARE the weights), and the moves each
// family serves. Spec §4 states the property the draw needs as PER-FAMILY
// INVARIANCE — identical tags and identical sizes across every world consistent
// with the seat's view — and says in terms that it is strictly stronger than
// union-set equality: a factored source that split the same union differently
// across two consistent worlds would pass every assertion above while making
// the retreat-vs-routes weight a function of hidden state.
//
// `placement_upper_len` is the concrete gap this closes. Nothing in the honesty
// suites read it before: the union check cannot, because Placement's size is an
// UPPER BOUND that deliberately over-counts the realized moves wherever a fee
// gate punches a hole. It is a real input to the volume metric and to any
// size-weighted draw, so it needs its own invariance evidence.
//
// WHY A DEDICATED WALK rather than more assertions inside `run_honesty_sweep`:
// this check materializes EVERY family at EVERY compared state (~9 surfaces per
// sampled point), which is a different cost class from the sweep's one
// `get_legal_moves`. Folding it in would tax both existing tiers for a property
// that is per-edit by design (spec §12). It reuses the sweep's machinery where
// the machinery is the point — `keep_game`, `hidden_variants` (all five hidden
// members, viewer-relative) and `BeliefSampler` determinization — and only the
// walk is its own.
// ---------------------------------------------------------------------------

/// 2P and 4P: the two counts that bracket the surface. 4P has the densest
/// boards (more assistants on board, so the biggest retreat products) and 2P the
/// largest boxed pile, which is where the hidden set is largest.
const FACTORED_COUNTS: [u8; 2] = [2, 4];
const FACTORED_SEEDS: [u64; 2] = [0, 1];
const FACTORED_MAX_STEPS: usize = 240;
/// Every 3rd decision point. The walk itself is uniform-random, so the sampled
/// points are spread across phases without paying nine surfaces at every step.
const FACTORED_SAMPLE_EVERY: usize = 3;
/// Tripwire, not a branch (the `action_source_equivalence.rs` precedent): a
/// state whose factored volume exceeds this fails loudly rather than silently
/// downgrading to a weaker comparison. Organic play stays far below it.
const FACTORED_VOLUME_CAP: u64 = 50_000;

/// The factored action surface at one decision point — everything the arena's
/// draw reads, captured for comparison.
#[derive(Debug)]
struct FactoredSurface {
    /// `families()`' tags in ITS order. The retreat tag carries its `on_board`
    /// payload, so comparing tags compares that map too.
    families: Vec<MoveFamily>,
    /// `DecisionHandle::size` per family — the value AND its exactness, which is
    /// what makes `placement_upper_len` a compared quantity.
    sizes: Vec<FamilySize>,
    /// The moves each family serves, in `iter`'s order.
    moves: Vec<Vec<Move>>,
}

fn factored_surface(state: &GameState, rules: &Rules) -> FactoredSurface {
    let handle = DecisionHandle::new(state, rules);
    let families: Vec<MoveFamily> = handle.families().to_vec();
    let sizes: Vec<FamilySize> = families.iter().map(|f| handle.size(f)).collect();
    let (volume, _) = handle.volume();
    assert!(
        volume <= FACTORED_VOLUME_CAP,
        "factored volume {volume} at {:?} exceeds the materialize cap {FACTORED_VOLUME_CAP} — \
         investigate before raising it",
        handle.turn_phase(),
    );
    let moves: Vec<Vec<Move>> = families.iter().map(|f| handle.iter(f).collect()).collect();
    FactoredSurface { families, sizes, moves }
}

/// Assert two surfaces are the same surface, naming WHICH of the three layers
/// moved. Move lists are compared element for element, which is strictly
/// stronger than the multiset equality the property needs — every one of these
/// generators is a pure function of the state, so a reordering across two
/// consistent worlds would be a real defect too, and an ordered compare reports
/// the first differing index instead of a count.
fn assert_same_surface(got: &FactoredSurface, want: &FactoredSurface, ctx: &str) {
    assert_eq!(
        got.families, want.families,
        "{ctx}: hidden state moved the family TAGS ({:?} vs {:?})",
        got.families, want.families,
    );
    for (i, (g, w)) in got.sizes.iter().zip(&want.sizes).enumerate() {
        assert_eq!(
            g, w,
            "{ctx}: hidden state moved the size of {:?} ({g:?} vs {w:?})",
            want.families[i],
        );
    }
    for (i, (g, w)) in got.moves.iter().zip(&want.moves).enumerate() {
        assert_eq!(
            g.len(),
            w.len(),
            "{ctx}: {:?} served {} moves, not {}",
            want.families[i],
            g.len(),
            w.len(),
        );
        if let Some(at) = g.iter().zip(w).position(|(a, b)| a != b) {
            panic!(
                "{ctx}: {:?} served a different move at index {at} of {}:\n  variant: {:?}\n  truth:   {:?}",
                want.families[i],
                g.len(),
                g[at],
                w[at],
            );
        }
    }
}

#[derive(Default)]
struct FactoredTally {
    effective: HashMap<&'static str, usize>,
    points: usize,
    comparisons: usize,
    deck_differing: usize,
    /// Points at which some family reported an `UpperBound` size — i.e. points
    /// where `placement_upper_len` was one of the compared quantities.
    upper_bound_points: usize,
    /// Points with more than one family: the Movement shape, where the two exact
    /// sizes ARE the weighted draw's weights.
    multi_family_points: usize,
    moves_compared: usize,
}

/// Spec §4's per-family-invariance requirement, executable (§10.3): across every
/// hidden variant and every sampled world consistent with a seat's view, the
/// factored surface is IDENTICAL — same family tags, same `FamilySize` values
/// (exactness included, so `placement_upper_len` is covered), same served moves
/// in the same order.
///
/// This is half the evidence for that requirement; `observation_leakage.rs`'s
/// `family_weighted_weight_vector_is_invariant_under_hidden_variation` is the
/// other half, aimed at the two Movement sizes specifically.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit). Adding the
/// hidden deck's TOP CARD to every exact size in `factored_surface` —
/// `FamilySize::Exact(n + state.order_deck.first().map_or(0, |c| c.0) as u64)` —
/// fails at the FIRST sampled point:
///
/// > hidden variant deck_reshuffle (2P seed 0 step 0): hidden state moved the
/// > size of AdditionalAction (Exact(18) vs Exact(32))
///
/// while `observation_honesty_per_edit` stayed GREEN in the same run — which is
/// precisely the coverage gap this test closes.
///
/// The first attempt at that mutation used the deck's LENGTH and this test
/// passed, correctly: deck length is PUBLIC (`PlayerView::order_deck_len`), every
/// variant family preserves it by construction, and the sampler conserves it. A
/// mutation has to read hidden CONTENT to be a leak at all — worth recording,
/// because a reviewer re-running the proof with the obvious `.len()` would
/// otherwise read the green as a vacuous test rather than as the boundary
/// working.
#[test]
fn families_iter_and_len_are_invariant_across_hidden_variants_and_sampled_worlds() {
    let rules = Rules::standard();
    let mut tally = FactoredTally {
        effective: FAMILIES.iter().map(|f| (*f, 0usize)).collect(),
        ..Default::default()
    };

    for &players in &FACTORED_COUNTS {
        for &seed in &FACTORED_SEEDS {
            let mut state = keep_game(players, seed, &rules);
            let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xF00D);
            let mut step = 0usize;

            while !is_game_over(&state) && step < FACTORED_MAX_STEPS {
                let legal = get_legal_moves(&state, &rules);
                assert!(!legal.is_empty(), "dead end at {players}P seed {seed} step {step}");

                if step.is_multiple_of(FACTORED_SAMPLE_EVERY) {
                    let seat = state.current_player;
                    let view = observe(&state, seat).unwrap();
                    let base = factored_surface(&state, &rules);

                    tally.points += 1;
                    if base.sizes.iter().any(|s| matches!(s, FamilySize::UpperBound(_))) {
                        tally.upper_bound_points += 1;
                    }
                    if base.families.len() > 1 {
                        tally.multi_family_points += 1;
                    }
                    tally.moves_compared += base.moves.iter().map(Vec::len).sum::<usize>();

                    // Hidden VARIANTS: states the seat could not rule out.
                    for (family, variant) in hidden_variants(&state, seat, &mut rng) {
                        if variant != state {
                            *tally.effective.get_mut(family).unwrap() += 1;
                        }
                        assert_same_surface(
                            &factored_surface(&variant, &rules),
                            &base,
                            &format!("hidden variant {family} ({players}P seed {seed} step {step})"),
                        );
                        tally.comparisons += 1;
                    }

                    // Sampled WORLDS: the determinizations the arena's agent
                    // actually builds its handle over.
                    let mut sampler =
                        BeliefSampler::from_view(&view, &rules, 101 + step as u64).unwrap();
                    for _ in 0..3 {
                        let world = sampler.sample();
                        if world.order_deck != state.order_deck {
                            tally.deck_differing += 1;
                        }
                        assert_same_surface(
                            &factored_surface(&world, &rules),
                            &base,
                            &format!("sampled world ({players}P seed {seed} step {step})"),
                        );
                        tally.comparisons += 1;
                    }
                }

                let pick = legal[rng.gen_range(0..legal.len())].clone();
                state = apply_move_unchecked(&state, &pick, &rules).unwrap();
                step += 1;
            }
        }
    }

    eprintln!(
        "factored per-family invariance: {} decision points, {} comparisons, {} moves compared; \
         {} points carried an UpperBound size, {} were multi-family; {} samples drew a different deck",
        tally.points,
        tally.comparisons,
        tally.moves_compared,
        tally.upper_bound_points,
        tally.multi_family_points,
        tally.deck_differing,
    );

    // ANTI-VACUITY, per family (the module header's rule): a run that varied
    // nothing must not pass.
    for f in FAMILIES {
        let n = tally.effective[f];
        assert!(n >= 50, "family {f} was effective only {n} times (floor 50)");
    }
    assert!(
        tally.deck_differing >= 200,
        "only {} samples drew a different deck — a truth-copying sampler would pass vacuously",
        tally.deck_differing,
    );
    // ...and per COMPARED QUANTITY: each of the three layers must have been
    // exercised on something. Without these, a walk that only ever reached
    // single-family points with exact sizes would report a green
    // "per-family invariance proven" having never compared an upper bound or a
    // weight pair.
    assert!(
        tally.upper_bound_points >= 50,
        "only {} sampled points carried an UpperBound size — placement_upper_len is the quantity \
         this test exists to cover",
        tally.upper_bound_points,
    );
    assert!(
        tally.multi_family_points >= 20,
        "only {} sampled points had >1 family — the Movement pair IS the weighted draw's weight \
         vector, so a run that never reached one proves nothing about it",
        tally.multi_family_points,
    );
    assert!(
        tally.moves_compared >= 5_000,
        "only {} moves compared — the iterated-moves layer needs real volume",
        tally.moves_compared,
    );
}

/// Task 6 carry-forward: the honest game driver's non-termination cap
/// (`HONEST_MAX_TURNS`) and the shared self-play driver's cap
/// (`common::MAX_TURNS`) are the same bound. Pin them to each other so a change
/// to one forces a look at the other, and anchor the current value.
#[test]
fn honest_and_common_turn_caps_agree() {
    assert_eq!(
        yokohama_core::HONEST_MAX_TURNS,
        common::MAX_TURNS,
        "the honest driver and the self-play driver must share one turn cap"
    );
    assert_eq!(yokohama_core::HONEST_MAX_TURNS, 10_000);
}
