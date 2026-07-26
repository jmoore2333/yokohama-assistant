//! T2 (counterfactual-leakage invariance) and the targeted corners (spec T8).
//!
//! WHY A CANARY: `TrivialHonestAgent`'s decision cannot possibly move under
//! T2's variants -- legality is view-measurable, so its inputs are invariant
//! by construction. Running only that agent would give a green "leakage
//! invariance proven" with no detector behind it. The canary's decision
//! depends on the SAMPLED hidden content, so a sampler or a plumbing bug that
//! leaks truth moves its pick. The cheating-sampler test below proves the
//! detector actually detects.
//!
//! VIEWER = CURRENT PLAYER (load-bearing, do not "simplify" to seat 0). Every
//! game-walk observes and canary-picks at `current_player`: the sampler only
//! guarantees the VIEWER's hand is exact, and `get_legal_moves` reads the
//! current player's hand, so a canary keyed off a sample at any other seat
//! would false-fail T2 for a reason that is not a leak.
//!
//! PHASE 3.2 (Task 11, spec §2's honesty thread) adds the weighted policy to
//! this file, and with it two channels 3.0 never had to think about. Pick
//! invariance certifies what a decider READS; it certifies neither where the
//! decider's rng SEED came from (a seed folded from hidden state produces moves
//! that are individually unimpeachable) nor which WEIGHTS the decider drew from
//! (two runs can agree on a pick while disagreeing on the distribution it came
//! out of). Each gets its own cell, and each carries its own catch evidence —
//! the seed one by flagging a leaky agent and an honest one in the SAME run,
//! the weight one by a documented mutation proof.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game,
    observe, ArenaAgent, BeliefError, BeliefSampler, DecisionHandle, GameConfig, GameState,
    HandCard, Move, MoveFamily, PlayerId, PlayerView, PolicyAgent, RolloutPolicy, Rules,
    StartingOrderChoice, ViewError,
};

fn keep_game(players: u8, seed: u64, rules: &Rules) -> GameState {
    let pending = new_game(GameConfig::for_player_count(players), rules, seed).unwrap();
    let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
    finalize_setup(pending, &choices).unwrap()
}

/// The viewer (seat 0) keeps; each seat listed in `unknown_seats` is a live
/// `Unknown` (the shape a physically-tracked game produces). Every other seat
/// keeps.
fn live_game(players: u8, seed: u64, unknown_seats: &[u8], rules: &Rules) -> GameState {
    let pending = new_game(GameConfig::for_player_count(players), rules, seed).unwrap();
    let choices: Vec<_> = (0..players)
        .map(|i| {
            if unknown_seats.contains(&i) {
                StartingOrderChoice::Unknown
            } else {
                StartingOrderChoice::Keep(0)
            }
        })
        .collect();
    finalize_setup(pending, &choices).unwrap()
}

/// Fold hidden content into a number. Any leak changes this.
fn hidden_fingerprint(s: &GameState) -> usize {
    s.order_deck.iter().map(|c| c.0 as usize).sum::<usize>() * 31
        + s.orders_removed.iter().map(|c| c.0 as usize).sum::<usize>() * 17
        + s.players.iter().filter_map(|p| p.starting_order).map(|c| c.0 as usize).sum::<usize>()
}

/// The canary's decision: index into `legal` by the hidden content of a world.
/// `cheat = None` uses an HONEST sample (blind to truth by construction);
/// `cheat = Some(truth)` simulates a sampler that peeked.
fn canary_pick(
    view: &PlayerView,
    legal: &[Move],
    rules: &Rules,
    sampler_seed: u64,
    cheat: Option<&GameState>,
) -> Move {
    let fp = match cheat {
        Some(truth) => hidden_fingerprint(truth),
        None => {
            let mut s = BeliefSampler::from_view(view, rules, sampler_seed).unwrap();
            hidden_fingerprint(&s.sample())
        }
    };
    legal[fp % legal.len()].clone()
}

/// Counterfactual worlds that differ from `state` ONLY in what no seat can see:
/// the deck ORDER, the deck/box PARTITION, and the setup SEED. All three are
/// hidden members `observe` masks, so every variant observes back to the same
/// view. NOTE (Task-7 carry-forward): this helper deliberately perturbs NO
/// starting order -- the one hidden member that needs seat-relative targeting
/// (perturbing the viewer's own deal would move something it legitimately
/// sees). That is why the game-walks can observe from `current_player` with no
/// viewer argument here: nothing this function touches is seat-relative.
fn variants(state: &GameState, rng: &mut ChaCha8Rng) -> Vec<GameState> {
    use rand::seq::SliceRandom;
    let mut out = Vec::new();

    let mut a = state.clone();
    a.order_deck.shuffle(rng);
    out.push(a);

    let mut b = state.clone();
    {
        let mut pool: Vec<_> =
            b.order_deck.iter().chain(b.orders_removed.iter()).copied().collect();
        pool.shuffle(rng);
        let d = b.order_deck.len();
        b.order_deck = pool[..d].to_vec();
        b.orders_removed = pool[d..].to_vec();
    }
    out.push(b);

    let mut c = state.clone();
    c.config.setup_seed = c.config.setup_seed.wrapping_add(12345);
    out.push(c);

    out
}

/// Walk a random game, and at each decision point return (true_state, view,
/// legal) for the ACTING seat.
fn decision_points(
    players: u8,
    seed: u64,
    max_steps: usize,
    rules: &Rules,
) -> Vec<(GameState, PlayerView, Vec<Move>)> {
    let mut state = keep_game(players, seed, rules);
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xBEEF);
    let mut out = Vec::new();
    let mut step = 0;
    while !is_game_over(&state) && step < max_steps {
        let legal = get_legal_moves(&state, rules);
        let view = observe(&state, state.current_player).unwrap();
        out.push((state.clone(), view, legal.clone()));
        let pick = legal[rng.gen_range(0..legal.len())].clone();
        state = apply_move_unchecked(&state, &pick, rules).unwrap();
        step += 1;
    }
    out
}

/// THE decision-invariance sweep. Walk a 2P game and, at every decision point,
/// assert `decide` returns the SAME move under every hidden variant of the true
/// state — the variants observe back to the same view, so a decider that reads
/// only its view cannot move.
///
/// Extracted (Phase 3.2 Task 11) so the sweep covers more than one decider: the
/// canary below is the DETECTOR (deliberately hidden-content-sensitive, so it
/// CAN fail), and the shipped `FamilyWeighted` `PolicyAgent` is the thing being
/// certified. One harness, so neither can drift into checking a weaker property
/// than the other.
///
/// Returns the comparison count for the caller's anti-vacuity floor.
fn run_decision_invariance_sweep(
    label: &str,
    rules: &Rules,
    variant_seed: u64,
    mut decide: impl FnMut(&PlayerView, &[Move], &Rules) -> Move,
) -> usize {
    let mut rng = ChaCha8Rng::seed_from_u64(variant_seed);
    let mut checked = 0usize;
    for (truth, view, legal) in decision_points(2, 0, 150, rules) {
        let base = decide(&view, &legal, rules);
        for v in variants(&truth, &mut rng) {
            let vv = observe(&v, v.current_player).unwrap();
            let vlegal = get_legal_moves(&v, rules);
            assert_eq!(
                decide(&vv, &vlegal, rules),
                base,
                "{label}: a decision moved when only hidden state changed"
            );
            checked += 1;
        }
    }
    checked
}

#[test]
fn t2_the_canarys_decision_never_moves_under_hidden_variation() {
    let rules = Rules::standard();
    let checked = run_decision_invariance_sweep("T2 canary", &rules, 4, |view, legal, rules| {
        canary_pick(view, legal, rules, 42, None)
    });
    assert!(checked >= 300, "anti-vacuity: only {checked} comparisons");
}

/// The agent seed the honest cells below run on. A literal, deliberately: what
/// these tests are about is that the seed's PROVENANCE is a constant rather than
/// a function of the state (spec §3's OS-entropy rule binds bot-facing
/// evaluation runs, not fixtures).
const HONEST_AGENT_SEED: u64 = 0x5EED_0F15_EA7E_D000;

/// The arena's SHIPPED agent, run as a decider. A FRESH [`PolicyAgent`] per
/// call, so its rng starts at a known offset every time and the only thing that
/// can move the pick is the input — the isolation the invariance claim needs.
/// (In a real game the agent is long-lived and its stream advances; that is a
/// different property, pinned by the arena's own determinism tests.)
///
/// The whole honest chain runs — view → `BeliefSampler` → `DecisionHandle` →
/// `RolloutPolicy::pick` — because that chain is what 3.2 ships. A thin
/// re-implementation here could hold while the real one leaked.
fn policy_agent_pick(
    policy: RolloutPolicy,
    agent_seed: u64,
    view: &PlayerView,
    rules: &Rules,
) -> Move {
    let mut agent = PolicyAgent::new(policy, agent_seed);
    agent
        .choose(view, rules)
        .expect("the honest chain resolves a move at a real decision point")
        .mv
}

/// The shipped `FamilyWeighted` agent joins the sweep above (spec §2's honesty
/// thread). The canary proves the harness CAN catch a leak; this proves the
/// agent that will actually play into 3.3's corpus does not have one.
///
/// It is not implied by the canary's result. The canary reads a sampled world's
/// hidden CONTENT directly; the policy reads the FACTORED SURFACE over that same
/// world (family tags and sizes) plus its own rng. Those are different reads of
/// the same sample, and only this cell exercises the second.
///
/// WHAT THIS CELL ACTUALLY PINS (T11 review F1 — the earlier claim here was
/// false): across every comparison the VIEW is identical (measured 0/450
/// differing), and `PolicyAgent::choose(view, rules)` makes hidden state
/// unreachable BY SIGNATURE — so this cell compares f(x) against f(x) and can
/// fail only on internal nondeterminism (a thread_rng, a clock, an
/// iteration-order dependence) or a future signature change that re-opens the
/// channel. BOTH are worth pinning; NEITHER is "a decider that can move
/// declining to". The canary below moves 68 times by varying the SEED — the
/// one input this cell holds constant — so it is evidence about a different
/// channel, not this cell's non-vacuity. The structural guarantee here is the
/// signature; this cell is the tripwire that notices if that guarantee is
/// ever traded away.
#[test]
fn t2_the_family_weighted_agents_decision_never_moves_under_hidden_variation() {
    let rules = Rules::standard();
    let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
    let checked =
        run_decision_invariance_sweep("T2 family-weighted", &rules, 4, |view, _legal, rules| {
            policy_agent_pick(policy, HONEST_AGENT_SEED, view, rules)
        });
    assert!(checked >= 300, "anti-vacuity: only {checked} comparisons");
}

#[test]
fn t2_positive_control_the_canary_does_react_to_a_legitimate_view_change() {
    // If the canary were insensitive to everything, the test above would be
    // worthless. A genuinely different observation must move its pick.
    let rules = Rules::standard();
    let points = decision_points(2, 0, 150, &rules);
    let moved = points.windows(2).any(|w| {
        let a = canary_pick(&w[0].1, &w[0].2, &rules, 42, None);
        let b = canary_pick(&w[1].1, &w[1].2, &rules, 42, None);
        a != b
    });
    assert!(moved, "the canary is insensitive: it cannot detect anything");
}

#[test]
fn t2_mutation_proof_a_cheating_sampler_is_caught() {
    // The detector must actually detect. Same harness, but the "sampler" reads
    // the true state -- exactly the regression T2 exists to catch.
    let rules = Rules::standard();
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    let mut caught = false;
    for (truth, _view, legal) in decision_points(2, 0, 150, &rules) {
        let base = canary_pick(
            &observe(&truth, truth.current_player).unwrap(),
            &legal,
            &rules,
            42,
            Some(&truth),
        );
        for v in variants(&truth, &mut rng) {
            let vlegal = get_legal_moves(&v, &rules);
            let vv = observe(&v, v.current_player).unwrap();
            if canary_pick(&vv, &vlegal, &rules, 42, Some(&v)) != base {
                caught = true;
            }
        }
        if caught {
            break;
        }
    }
    assert!(caught, "the T2 harness failed to notice a sampler reading the true state");
}

// ---------------------------------------------------------------------------
// Phase 3.2 Task 11 (spec §2's honesty thread): the two additions aimed at the
// weighted policy. A policy has two channels the 3.0 suites never had to think
// about — where its rng SEED came from, and what WEIGHTS it exposes — and
// neither is covered by pick invariance alone.
// ---------------------------------------------------------------------------

/// Fold ONLY the boxed pile's CONTENTS into a number — hidden member 3, and
/// nothing else, so a seed derived from this is unambiguously seeded from hidden
/// state (`hidden_fingerprint` above mixes three members; this one names its
/// source).
///
/// Order-sensitive on purpose: the pile a seat cannot see has no canonical
/// meaning to that seat, so any reading of it is a leak.
fn removed_pile_fingerprint(s: &GameState) -> u64 {
    s.orders_removed
        .iter()
        .fold(0xC0FF_EE00_u64, |acc, c| acc.wrapping_mul(31).wrapping_add(u64::from(c.0)))
}

/// SEED-PROVENANCE CANARY (spec §2, M3). Pick invariance certifies what an agent
/// READS. It says nothing about where the agent's rng seed came from — and a
/// seed folded from hidden state is a leak that produces perfectly ordinary
/// moves, every one of them legal, every one of them view-consistent.
///
/// So this test proves the detector detects: the same shipped `PolicyAgent`, the
/// same sweep, the same variants — the only difference is that the leaky cell's
/// `agent_seed` is `HONEST_AGENT_SEED ^ removed_pile_fingerprint(truth)`. The
/// honest cell must be flagged ZERO times and the leaky cell at least once, in
/// the SAME run over the SAME decision points, which is what makes the catch
/// attributable to the seed's provenance and not to sampling noise.
///
/// This is the `t2_mutation_proof_a_cheating_sampler_is_caught` discipline
/// applied to a channel that mutation proof cannot reach: that one corrupts the
/// SAMPLER, and a corrupted sampler shows up in the sampled world; this one
/// leaves the world untouched and corrupts only the stream index.
#[test]
fn a_policy_seeded_from_a_hidden_fingerprint_is_caught() {
    let rules = Rules::standard();
    let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
    let mut rng = ChaCha8Rng::seed_from_u64(4);

    let mut compared = 0usize;
    let mut fingerprint_moved = 0usize;
    let mut caught_leaky = 0usize;
    let mut caught_honest = 0usize;

    // COUPLED to `run_decision_invariance_sweep`'s constants (2P seed 0, 150
    // points, variant rng seed 4) — this loop counts where the sweep asserts,
    // so it re-implements rather than calls it; if the sweep's constants move,
    // move these with them (T11 review F2).
    for (truth, view, _legal) in decision_points(2, 0, 150, &rules) {
        let truth_fp = removed_pile_fingerprint(&truth);
        let leaky_base = policy_agent_pick(policy, HONEST_AGENT_SEED ^ truth_fp, &view, &rules);
        let honest_base = policy_agent_pick(policy, HONEST_AGENT_SEED, &view, &rules);

        for v in variants(&truth, &mut rng) {
            let vv = observe(&v, v.current_player).unwrap();
            let v_fp = removed_pile_fingerprint(&v);
            compared += 1;
            if v_fp != truth_fp {
                fingerprint_moved += 1;
            }
            if policy_agent_pick(policy, HONEST_AGENT_SEED ^ v_fp, &vv, &rules) != leaky_base {
                caught_leaky += 1;
            }
            if policy_agent_pick(policy, HONEST_AGENT_SEED, &vv, &rules) != honest_base {
                caught_honest += 1;
            }
        }
    }

    eprintln!(
        "seed-provenance canary: {compared} comparisons, hidden fingerprint moved in {fingerprint_moved}; \
         flagged leaky {caught_leaky}, flagged honest {caught_honest}"
    );
    assert!(
        fingerprint_moved > 0,
        "no variant changed the boxed pile, so the leaky seed never differed — the cell would be \
         vacuous whatever it reported"
    );
    assert!(
        caught_leaky > 0,
        "the harness did NOT flag a policy whose rng seed folds in the boxed pile's contents \
         ({compared} comparisons, fingerprint moved in {fingerprint_moved}) — it cannot be trusted \
         to flag a real one"
    );
    // STRUCTURALLY zero, not an independent noise control (T11 review F1): the
    // honest pick's inputs are identical across the comparison by
    // construction, so this restates the same determinism the t2 cell pins.
    // The attribution of the 68 catches still holds — the leaky pick differs
    // from the honest one ONLY in seed provenance — it just holds by
    // construction rather than by this counter's evidence.
    assert_eq!(
        caught_honest, 0,
        "the honest agent was flagged {caught_honest} times over the same points — either the \
         shipped chain went nondeterministic or the comparison's inputs are no longer identical"
    );
}

/// The weights `FamilyWeighted` exposes at a Movement decision point: its two
/// knobs, plus the two EXACT family sizes that turn them into a draw.
///
/// This is the whole weight vector — spec §2 says so in terms, and §2's
/// structural note is why it is only two entries long: `families()` returns more
/// than one tag at exactly ONE decision-point shape, so "family weighting" IS a
/// routes-vs-retreat scalar. A future third tag would make this type wrong,
/// which `RolloutPolicy::pick`'s two-tag `debug_assert` is the loud version of.
#[derive(Debug, PartialEq, Eq)]
struct WeightVector {
    retreat_ppm: u32,
    recovery_min_of: u8,
    routes_len: u64,
    retreat_len: u64,
}

/// `policy`'s weight vector at `state`, or `None` when `state` is not a
/// two-family Movement point (or `policy` is not the weighted rung).
fn weight_vector_at(
    policy: RolloutPolicy,
    state: &GameState,
    rules: &Rules,
) -> Option<WeightVector> {
    let RolloutPolicy::FamilyWeighted { retreat_ppm, recovery_min_of } = policy else {
        return None;
    };
    let handle = DecisionHandle::new(state, rules);
    let [routes @ MoveFamily::MovementRoutes, retreat @ MoveFamily::MovementRetreat { .. }] =
        handle.families()
    else {
        return None;
    };
    Some(WeightVector {
        retreat_ppm,
        recovery_min_of,
        routes_len: handle.size(routes).value(),
        retreat_len: handle.size(retreat).value(),
    })
}

/// WEIGHT-VECTOR INVARIANCE (spec §2, M3) — strictly stronger than pick
/// invariance, and the reason it is worth having: two decision points can share
/// a pick by luck (a one-move family picks the same move whatever the weights
/// say), but they cannot share a weight vector by luck. If the retreat-vs-routes
/// weight were ever a function of hidden state, the DISTRIBUTION the arena draws
/// from would be seat-privileged even while every individual move it made looked
/// honest.
///
/// Three sources of hidden variation, all held to the same vector:
///   1. the true state itself (the base),
///   2. every `variants()` hidden variant of it — read DIRECTLY, not through the
///      view, so this does not merely re-derive T1 and then stop,
///   3. several distinct `BeliefSampler` determinizations of the seat's view —
///      the worlds the arena's agent actually builds its handle over, which is
///      the case the property exists for.
///
/// Together with `observation_honesty.rs`'s
/// `families_iter_and_len_are_invariant_across_hidden_variants_and_sampled_worlds`
/// this is the executable evidence for spec §4's per-family-invariance
/// requirement.
///
/// MUTATION-PROVEN (run at authoring, reverted by inverse edit). Folding the
/// boxed pile's first card into the routes weight —
/// `routes_len: handle.size(routes).value() + state.orders_removed.first().map_or(0, |c| u64::from(c.0))`
/// — fails at the first variant comparison:
///
/// > a hidden variant moved the weighted policy's weight vector
/// > left:  Some(WeightVector { retreat_ppm: 166667, recovery_min_of: 5, routes_len: 19, retreat_len: 8 })
/// > right: Some(WeightVector { retreat_ppm: 166667, recovery_min_of: 5, routes_len: 12, retreat_len: 8 })
///
/// DIVISION OF LABOUR (T11 review F3): `retreat_len` rides in this vector for
/// completeness but the FamilyWeighted retreat branch weights over the
/// `on_board` MAP, which is not in this struct — that half is covered by the
/// honesty extension's `Vec<MoveFamily>` equality, whose `MovementRetreat`
/// payload IS the map. The two cells jointly discharge spec §4's per-family
/// invariance; neither alone does.
#[test]
fn family_weighted_weight_vector_is_invariant_under_hidden_variation() {
    let rules = Rules::standard();
    let policy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;
    let mut rng = ChaCha8Rng::seed_from_u64(11);

    let mut movement_points = 0usize;
    let mut variant_comparisons = 0usize;
    let mut effective_variants = 0usize;
    let mut world_comparisons = 0usize;
    let mut differing_worlds = 0usize;

    // COUPLED to `run_decision_invariance_sweep`'s constants (2P seed 0, 150
    // points, variant rng seed 4) — this loop counts where the sweep asserts,
    // so it re-implements rather than calls it; if the sweep's constants move,
    // move these with them (T11 review F2).
    for (truth, view, _legal) in decision_points(2, 0, 150, &rules) {
        let Some(base) = weight_vector_at(policy, &truth, &rules) else {
            continue;
        };
        movement_points += 1;

        for v in variants(&truth, &mut rng) {
            if v != truth {
                effective_variants += 1;
            }
            assert_eq!(
                weight_vector_at(policy, &v, &rules).as_ref(),
                Some(&base),
                "a hidden variant moved the weighted policy's weight vector"
            );
            variant_comparisons += 1;
        }

        for seed in 0..4u64 {
            let mut sampler = BeliefSampler::from_view(&view, &rules, 900 + seed).unwrap();
            let world = sampler.sample();
            if world.order_deck != truth.order_deck {
                differing_worlds += 1;
            }
            assert_eq!(
                weight_vector_at(policy, &world, &rules).as_ref(),
                Some(&base),
                "a sampled world moved the weighted policy's weight vector"
            );
            world_comparisons += 1;
        }
    }

    eprintln!(
        "weight-vector invariance: {movement_points} Movement points, {variant_comparisons} variant \
         comparisons ({effective_variants} effective), {world_comparisons} sampled-world comparisons \
         ({differing_worlds} drew a different deck)"
    );
    assert!(
        movement_points >= 20,
        "only {movement_points} two-family Movement points reached — the weight vector exists \
         nowhere else, so a run that misses them proves nothing"
    );
    assert!(effective_variants >= 40, "only {effective_variants} variants actually differed");
    assert!(differing_worlds >= 40, "only {differing_worlds} sampled worlds drew a different deck");
}

/// T8a (fulfillment mask-lift): a starting order lifts its mask the instant it
/// is fulfilled -- placed face-up (`01-rules-reference.md:457`), never
/// returning to hand -- so it becomes a PUBLIC located card. It must leave the
/// unseen pool, and no sampled world may re-hide it into a deck or box.
#[test]
fn t8a_a_fulfilled_starting_order_lifts_its_mask_permanently() {
    let rules = Rules::standard();
    let mut state = keep_game(2, 7, &rules);
    let card = state.players[1].starting_order.expect("Keep shape records the card");
    // Fulfill it: hand -> completed pile (the engine's own write pair,
    // apply_move.rs:1335), conserving the 36-card count.
    state.players[1].order_cards_in_hand.retain(|&c| c != card);
    state.players[1].completed_orders.push(card);
    assert!(check_invariants(&state, &rules).is_ok(), "a completed starting order is coherent");

    let view = observe(&state, PlayerId(0)).unwrap();
    assert!(
        view.players[1].hand.iter().all(|c| matches!(c, HandCard::Known(_))),
        "no hidden slot survives fulfillment"
    );
    assert_eq!(view.hidden_slot_count(), 0, "2P: seat 1 was the only mask, and it lifted");
    assert!(
        !view.unseen_orders(&rules).contains(&card),
        "a face-up completed order is located, not unseen"
    );

    // The card is public, so EVERY consistent world keeps it face-up.
    let mut s = BeliefSampler::from_view(&view, &rules, 8).unwrap();
    for _ in 0..10 {
        let w = s.sample();
        assert!(
            w.players[1].completed_orders.contains(&card),
            "the sampler keeps the fulfilled card face-up"
        );
        assert!(
            !w.order_deck.contains(&card) && !w.orders_removed.contains(&card),
            "a fulfilled order can never be re-hidden into a deck or box"
        );
    }
}

#[test]
fn t8b_hidden_slots_are_filled_position_exactly_for_the_acting_seat() {
    // Organic incidence of "acting seat has >=2 affordable orders" is ~0.04%,
    // so this corner gets a constructed fixture rather than sampling luck.
    let rules = Rules::standard();
    let mut state = keep_game(2, 7, &rules);
    // Seat 1 acts, holding its (hidden-to-seat-0) starting order plus a second
    // card, both affordable from a generous purse.
    state.current_player = PlayerId(1);
    let extra = state.order_deck.pop().unwrap();
    state.players[1].order_cards_in_hand.push(extra);
    state.players[1].money = 50;
    state.players[1].copper = 9;
    state.players[1].silk = 9;
    state.players[1].tea = 9;
    state.players[1].seafood = 9;
    state.players[1].imported_goods = 9;
    // Moving a card deck->hand preserves the 36-card total, so the fixture needs
    // no `orders_removed` compensation -- pushing one there would double-count
    // the card and BREAK conservation. Prove the fixture is clean before
    // observing it.
    assert!(
        check_invariants(&state, &rules).is_ok(),
        "the deck->hand move conserves 36 cards; the fixture is conservation-clean"
    );

    let view = observe(&state, PlayerId(0)).unwrap();
    let hidden_idx =
        view.players[1].hand.iter().position(|c| *c == HandCard::HiddenStartingOrder).unwrap();
    let mut s = BeliefSampler::from_view(&view, &rules, 9).unwrap();
    for _ in 0..10 {
        let w = s.sample();
        assert_eq!(
            w.players[1].order_cards_in_hand.len(),
            state.players[1].order_cards_in_hand.len(),
            "hand length preserved"
        );
        assert_eq!(
            w.players[1].starting_order,
            Some(w.players[1].order_cards_in_hand[hidden_idx]),
            "the drawn card sits at the masked index, both sites agree"
        );
        for (i, c) in view.players[1].hand.iter().enumerate() {
            if let HandCard::Known(id) = c {
                assert_eq!(w.players[1].order_cards_in_hand[i], *id, "known cards keep their index");
            }
        }
    }
}

#[test]
fn t8c_live_matrix_reconciles_at_every_player_count() {
    let rules = Rules::standard();
    for players in [2u8, 3, 4] {
        let opponents: Vec<u8> = (1..players).collect();
        let configs: Vec<Vec<u8>> = vec![
            vec![],             // no Unknown
            vec![opponents[0]], // exactly one Unknown
            opponents.clone(),  // all opponents Unknown
        ];
        for unknown in configs {
            let state = live_game(players, 7, &unknown, &rules);
            let view = observe(&state, PlayerId(0)).unwrap();

            // Reconciliation identity.
            assert_eq!(
                view.unseen_orders(&rules).len(),
                view.order_deck_len + view.orders_removed_len + view.hidden_slot_count(),
                "{players}P unknown={unknown:?}: |U| must reconcile"
            );
            // Physical box size, independent of tracking shape.
            let expected_box = state.orders_removed.len() - unknown.len();
            assert_eq!(view.orders_removed_len, expected_box);

            // Sample, validate, round-trip.
            let mut s = BeliefSampler::from_view(&view, &rules, 3).unwrap();
            let w = s.sample();
            assert!(check_invariants(&w, &rules).is_ok());
            assert_eq!(observe(&w, PlayerId(0)).unwrap(), view);

            // A live view equals the SIM view of the same physical truth.
            let sim = keep_game(players, 7, &rules);
            assert_eq!(
                observe(&sim, PlayerId(0)).unwrap(),
                view,
                "{players}P unknown={unknown:?}: tracking shape is not seat-visible"
            );
        }

        // Own-seat Unknown is refused at every player count.
        let own_hidden = live_game(players, 7, &[0], &rules);
        assert_eq!(
            observe(&own_hidden, PlayerId(0)),
            Err(ViewError::ViewerDealUnrecorded(PlayerId(0)))
        );

        // Correction (Task-2 carry-forward): the empty-hand cells above cannot
        // distinguish hand POSITION from mere set membership. Pin the index-0
        // synthesis convention against a REAL multi-card hand: a live Unknown
        // seat that also holds a witnessed Port take. The take is witnessed by
        // the table (`01-rules-reference.md:703`) so it must sit AFTER the
        // synthesized hidden slot, and both sampling and observing must preserve
        // that VECTOR order.
        let mut multi = live_game(players, 7, &[1], &rules); // seat 1 Unknown
        let take = multi.order_deck.pop().expect("deck is not empty");
        multi.players[1].order_cards_in_hand.push(take); // deck->hand: conservation-clean
        assert!(
            check_invariants(&multi, &rules).is_ok(),
            "{players}P: a tracked take on a hidden seat conserves 36 cards"
        );
        let mview = observe(&multi, PlayerId(0)).unwrap();
        assert_eq!(
            mview.players[1].hand,
            vec![HandCard::HiddenStartingOrder, HandCard::Known(take)],
            "{players}P: the synthesized slot sits at index 0, ahead of the witnessed take"
        );
        let mut ms = BeliefSampler::from_view(&mview, &rules, 4).unwrap();
        for _ in 0..10 {
            let w = ms.sample();
            assert_eq!(
                w.players[1].order_cards_in_hand.len(),
                2,
                "{players}P: hand length preserved"
            );
            assert_eq!(
                w.players[1].order_cards_in_hand[1], take,
                "{players}P: the witnessed take keeps hand index 1"
            );
            assert_eq!(
                w.players[1].starting_order,
                Some(w.players[1].order_cards_in_hand[0]),
                "{players}P: the synthesized card fills index 0, both storage sites agreeing"
            );
            assert_eq!(
                observe(&w, PlayerId(0)).unwrap().players[1].hand,
                vec![HandCard::HiddenStartingOrder, HandCard::Known(take)],
                "{players}P: the sampled world round-trips with the hidden slot still at index 0"
            );
        }
    }
}

/// T8d (typed-error refusals): the observation boundary's safe failure mode is a
/// TYPED error -- never a panic, never a partial view. All three `observe`
/// refusals, plus the sampler's typed refusal of a view that hides the viewer's
/// OWN slot (a coalition/own-seat leak that public fields permit constructing
/// but `observe` will never itself produce).
#[test]
fn t8d_non_viewable_seats_and_own_seat_leaks_are_typed_refusals() {
    let rules = Rules::standard();
    let state = keep_game(2, 7, &rules);

    // The DUMMY sentinel and out-of-range seats: typed, distinguishable errors.
    assert_eq!(observe(&state, PlayerId::DUMMY), Err(ViewError::DummySeat));
    assert_eq!(observe(&state, PlayerId(2)), Err(ViewError::SeatOutOfRange(PlayerId(2))));
    assert_eq!(observe(&state, PlayerId(9)), Err(ViewError::SeatOutOfRange(PlayerId(9))));

    // A seat cannot be unknown to itself: a live Unknown on the viewer's own
    // seat is a tracking artifact, refused rather than sampled into a guess...
    let own_hidden = live_game(2, 7, &[0], &rules);
    assert_eq!(
        observe(&own_hidden, PlayerId(0)),
        Err(ViewError::ViewerDealUnrecorded(PlayerId(0)))
    );
    // ...while the OTHER seat still views fine (seat 0 is just a hidden opponent
    // from there), so the refusal is targeted, not a whole-state rejection.
    assert!(observe(&own_hidden, PlayerId(1)).is_ok());

    // The sampler's refusals are typed too. A view mutated to hide the viewer's
    // OWN slot still RECONCILES (the unmasked card leaves the located set as the
    // slot count rises), so it reaches -- and is caught by -- the own-seat
    // guard, never sampled into a fabricated card for our own hand.
    let mut leaky = observe(&state, PlayerId(0)).unwrap();
    leaky.players[0].hand[0] = HandCard::HiddenStartingOrder;
    assert_eq!(
        leaky.unseen_orders(&rules).len(),
        leaky.order_deck_len + leaky.orders_removed_len + leaky.hidden_slot_count(),
        "the mutated view must still reconcile, or this corner proves nothing"
    );
    assert!(matches!(
        BeliefSampler::from_view(&leaky, &rules, 1),
        Err(BeliefError::Inconsistent(_))
    ));
}

#[test]
fn t8e_the_unseen_pool_never_grows() {
    // Information only ever flows private -> public in Yokohama: there are no
    // peek effects and nothing returns a card to a deck (spec F12).
    let rules = Rules::standard();
    let mut state = keep_game(3, 5, &rules);
    let mut rng = ChaCha8Rng::seed_from_u64(5);
    let mut previous = observe(&state, PlayerId(0)).unwrap().unseen_orders(&rules).len();
    for _ in 0..400 {
        if is_game_over(&state) {
            break;
        }
        let legal = get_legal_moves(&state, &rules);
        let pick = legal[rng.gen_range(0..legal.len())].clone();
        state = apply_move_unchecked(&state, &pick, &rules).unwrap();
        let now = observe(&state, PlayerId(0)).unwrap().unseen_orders(&rules).len();
        assert!(now <= previous, "the unseen pool grew: {previous} -> {now}");
        previous = now;
    }
}

#[test]
fn t8f_red_team_the_public_board_still_recovers_a_small_setup_seed() {
    // EXECUTABLE RESIDUAL (spec §1-B / L7). `new_game` generates the board
    // from the setup seed BEFORE shuffling the deck, so the board -- which
    // every honest view must carry verbatim -- fingerprints the seed. Masking
    // the FIELD does not close the CHANNEL; seed entropy does. This test
    // exists so that limit can never be quietly forgotten.
    let rules = Rules::standard();
    let victim_seed = 77u64;
    let state = keep_game(2, victim_seed, &rules);
    let view = observe(&state, PlayerId(0)).unwrap();

    let mut recovered = None;
    for candidate in 0..2_000u64 {
        let pending = new_game(GameConfig::for_player_count(2), &rules, candidate).unwrap();
        if pending.state.board_layout == view.board_layout {
            recovered = Some(candidate);
            break;
        }
    }
    assert_eq!(
        recovered,
        Some(victim_seed),
        "if this ever FAILS to recover, the board/seed coupling changed -- re-read spec L7 \
         before assuming the residual is closed"
    );
}

#[test]
fn t8g_two_seats_see_disjoint_hidden_slots() {
    // The coalition channel, documented in code: agents that pool their views
    // reconstruct more than either seat may know.
    let rules = Rules::standard();
    let state = keep_game(2, 7, &rules);
    let v0 = observe(&state, PlayerId(0)).unwrap();
    let v1 = observe(&state, PlayerId(1)).unwrap();
    assert!(matches!(v0.players[1].hand[0], HandCard::HiddenStartingOrder));
    assert!(matches!(v1.players[1].hand[0], HandCard::Known(_)));
    assert!(matches!(v1.players[0].hand[0], HandCard::HiddenStartingOrder));
    assert!(matches!(v0.players[0].hand[0], HandCard::Known(_)));
}
