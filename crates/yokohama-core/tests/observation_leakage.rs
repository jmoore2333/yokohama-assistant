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
//!
//! PHASE 3.3 (Task 8, spec §0.4) adds the evaluator bots, and the ONE thing to
//! read before reading those cells is that they are not all the same KIND of
//! test. v1 of the spec proposed the decision-invariance sweep as the
//! evaluator's leakage evidence and both clean-eyes lenses independently
//! refuted it as structurally vacuous — the sweep's variants observe back to an
//! IDENTICAL view, so any decider with a `&PlayerView` signature compares f(x)
//! against f(x). The repaired reading is three parts, and each cell below says
//! which part it is:
//!   (i)   SIGNATURE TRIPWIRES — Greedy-v1 and Lookahead-v1 join the sweep.
//!         Content: no internal nondeterminism, and the signature guarantee has
//!         not been traded away. Labelled as tripwires, not measurements.
//!   (ii)  THE FALSIFIABLE CELL — determinization invariance. Its inputs
//!         GENUINELY DIFFER (sampled worlds ⇒ differing successor views) and it
//!         IS spec §4.3's K-revisit trigger: red here means an evaluator term
//!         has begun reading determinized content, and that K > 1 is live.
//!   (iii) THE RECORDED MEASUREMENT — Lookahead's cross-world divergence rate,
//!         printed with its provenance.
//! Ledger L11 is the one-line version: the sweep cell is a tripwire and is
//! labelled one; the leakage clause's real content is (ii) plus (iii).
//!
//! COST (debug, measured at the T8 close, three runs of the whole binary):
//! **9.04 / 9.01 / 9.06 s**, against 0.13 s before T8. The tests run
//! CONCURRENTLY, so that wall is the longest single cell (the divergence
//! instrument, ~8.5 s) and not the ~17 s of CPU the four new cells spend
//! between them — per-cell: divergence ~8.5 s, Lookahead tripwire ~6.5 s,
//! determinization ~1.6 s, Greedy tripwire ~0.5 s. Two of the four are searches,
//! which is the whole of the cost; the Lookahead knobs are reduced for exactly
//! this reason and the reduction is pre-declared at [`TRIPWIRE_BEAM`], per spec
//! §10.1's debug-cost hazard. All four are per-edit and none is `#[ignore]`d, so
//! no new gate command (and no four-site registration) is owed.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::game::eval::{self, Explanation, WeightTable};
use yokohama_core::game::greedy::{candidates, Candidates, GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use yokohama_core::game::lookahead::{LookaheadAgent, SearchScheme};
use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game,
    observe, AreaAction, ArenaAgent, BeliefError, BeliefSampler, DecisionHandle, GameConfig,
    GameState, HandCard, Move, MoveFamily, PlayerId, PlayerView, PolicyAgent, PortState,
    RolloutPolicy, Rules, StartingOrderChoice, ViewError,
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

// ---------------------------------------------------------------------------
// Phase 3.3 Task 8 — spec §0.4's THREE-PART reading of "leakage invariants
// green against the new bot". The three cells below are deliberately NOT three
// of a kind, and reading them as one family is the mis-read §0.4 exists to
// prevent:
//
//   (i)  the two SIGNATURE TRIPWIRES (Greedy-v1, Lookahead-v1) join the sweep
//        above. Like the FamilyWeighted cell they compare f(x) against f(x) —
//        the variants observe back to an IDENTICAL view and `choose(view,
//        rules)` cannot reach hidden state BY SIGNATURE. They are tripwires,
//        not leakage measurements, and they say so.
//   (ii) the DETERMINIZATION-INVARIANCE cell is the falsifiable one: the
//        inputs it compares GENUINELY DIFFER (different sampled worlds ⇒
//        different successor views), and the equality it asserts is a real
//        constraint on what the evaluator reads. It is spec §4.3's K-revisit
//        trigger made executable.
//   (iii) the LOOKAHEAD DIVERGENCE instrument is a MEASUREMENT, reported with
//        its provenance.
// ---------------------------------------------------------------------------

/// The knobs the two Lookahead cells run at, and the one reason they are not
/// the shipped `B = 4` / `M = 8`.
///
/// COST, pre-declared rather than discovered (spec §10.1's debug-cost hazard).
/// Measured in this tree, debug, 40 organic decision points: the shipped
/// `B = 4, M = 8` costs **48.2 ms/decision**, which over the sweep's 600
/// decisions is ~29 s in ONE per-edit test; `B = 1, M = 2` costs **10.4 ms**
/// (K = 1) and **29.9 ms** (K = 3).
///
/// WHY THE KNOBS ARE NOT LOAD-BEARING HERE. A tripwire's content is "no
/// thread-rng, no clock, no iteration-order dependence, and the `&PlayerView`
/// signature has not been traded away" — none of which is a function of `B` or
/// `M`. `M = 2` rather than `M = 0` deliberately: the retreat sampler keeps
/// DRAWING, so a nondeterministic dice source inside the candidate scheme is
/// still in the tripwire's reach. For the divergence instrument the knobs ARE
/// a stated scope on the measurement — see that test.
const TRIPWIRE_BEAM: u16 = 1;
/// `M` for the Lookahead cells; see [`TRIPWIRE_BEAM`].
const TRIPWIRE_RETREAT_SAMPLES: u8 = 2;

/// Greedy-v1 as a decider, on the shipped [`WeightTable::V1`] and `M`.
fn greedy_agent_pick(rules: &Rules, agent_seed: u64, view: &PlayerView) -> Move {
    let mut agent = GreedyAgent::new(rules, agent_seed);
    agent.choose(view, rules).expect("Greedy-v1 resolves a move at a real decision point").mv
}

/// Lookahead-v1 as a decider at [`TRIPWIRE_BEAM`] / [`TRIPWIRE_RETREAT_SAMPLES`].
fn lookahead_agent_pick(rules: &Rules, agent_seed: u64, worlds: u8, view: &PlayerView) -> Move {
    let mut agent = LookaheadAgent::with_knobs(
        rules,
        SearchScheme::TurnBoundary { beam: TRIPWIRE_BEAM },
        worlds,
        TRIPWIRE_RETREAT_SAMPLES,
        WeightTable::V1,
        agent_seed,
    );
    agent.choose(view, rules).expect("Lookahead-v1 resolves a move at a real decision point").mv
}

/// **SIGNATURE TRIPWIRE — Greedy-v1** (spec §0.4(i), ledger L11).
///
/// THIS CELL IS NOT A LEAKAGE MEASUREMENT, and the spec says so in terms: v1 of
/// §0.4 proposed exactly this as the evaluator's leakage evidence and both clean
/// -eyes lenses independently refuted it as structurally vacuous. Across every
/// comparison the VIEW is identical (the FamilyWeighted cell above measured
/// 0/450 differing, and `variants()` is unchanged), and
/// `GreedyAgent::choose(view, rules)` makes hidden state unreachable BY
/// SIGNATURE — so this compares f(x) against f(x). What it CAN catch is worth
/// catching and is the whole of its content:
///
///  - internal nondeterminism — a `thread_rng`, a clock read, a `HashMap`
///    iteration-order dependence anywhere down the chain
///    (`observe` → `BeliefSampler` → `DecisionHandle` → candidates → `score`);
///  - a future signature change that re-opens the channel (`choose(&GameState,
///    …)`, a `&Rules` carrying state, a back-door accessor).
///
/// The T11 note on the FamilyWeighted cell above is the precedent this doc
/// follows deliberately, one rung up: the structural guarantee is the SIGNATURE,
/// and the cell is the tripwire that notices if that guarantee is ever traded
/// away. §0.4(ii)'s determinization cell below is the falsifiable half; cite
/// THAT one for "the evaluator does not read what it must not".
///
/// **FRESH AGENT PER CALL** (the FamilyWeighted cell's rule, and it bites
/// harder here). A `GreedyAgent` draws a sampler seed, `M` retreat samples and a
/// tie-break word from its own stream at every decision, so a LONG-LIVED agent
/// arrives at each comparison at a different stream offset and the pick moves
/// for a reason that is not a leak. Written that way first and RUN, so the
/// distinction is evidence rather than assertion:
///
/// > T8 greedy-v1: a decision moved when only hidden state changed
/// >   left: PlaceThreeAreas { placements: [AreaId(5), AreaId(7), AreaId(8)] }
/// >  right: PlaceThreeAreas { placements: [AreaId(3), AreaId(4), AreaId(5)] }
#[test]
fn t8_greedy_v1s_decision_never_moves_under_hidden_variation() {
    let rules = Rules::standard();
    let checked = run_decision_invariance_sweep("T8 greedy-v1", &rules, 4, |view, _legal, rules| {
        greedy_agent_pick(rules, HONEST_AGENT_SEED, view)
    });
    assert!(checked >= 300, "anti-vacuity: only {checked} comparisons");
}

/// **SIGNATURE TRIPWIRE — Lookahead-v1** (spec §0.4(i), ledger L11).
///
/// Everything the Greedy cell's doc says applies verbatim, and the reason to run
/// BOTH is that the search rung's chain is strictly longer: it samples `K`
/// worlds, keys a [`TieStream`], keys a candidate stream, and then applies moves
/// INSIDE the sampled world for a whole turn. That is more surface for a clock
/// or an iteration-order dependence to hide in, and none of it changes the fact
/// that `choose(&PlayerView, &Rules)` is the whole input.
///
/// Same fresh-agent rule, same recorded red (long-lived agent, run before the
/// fix):
///
/// > T8 lookahead-v1: a decision moved when only hidden state changed
/// >   left: PlaceThreeAreas { placements: [AreaId(1), AreaId(2), AreaId(5)] }
/// >  right: PlaceThreeAreas { placements: [AreaId(2), AreaId(4), AreaId(7)] }
///
/// [`TieStream`]: yokohama_core::game::lookahead::TieStream
#[test]
fn t8_lookahead_v1s_decision_never_moves_under_hidden_variation() {
    let rules = Rules::standard();
    let checked =
        run_decision_invariance_sweep("T8 lookahead-v1", &rules, 4, |view, _legal, rules| {
            lookahead_agent_pick(rules, HONEST_AGENT_SEED, 1, view)
        });
    assert!(checked >= 300, "anti-vacuity: only {checked} comparisons");
}

/// Worlds drawn from ONE view for the determinization cell. Three, not two, so
/// a coincidental agreement between a pair does not carry the cell.
const DETERMINIZATION_WORLDS: usize = 3;
/// The 2P walks the cell reads its organic decision points from. FOUR games
/// rather than one, and the reason is the positive control rather than
/// coverage-for-its-own-sake: a Port take is a ~1-in-37 decision point, so a
/// single 150-point walk yielded only 4 Port-take CANDIDATES in the whole sweep
/// (and 8 differing port states, both comparisons of each) — live, but one
/// unlucky trajectory away from silently degenerating. Four walks measured 19
/// and 38, cost ~1.6 s in debug, and are what the floors below are set from.
const DETERMINIZATION_WALKS: [u64; 4] = [0, 1, 2, 3];
/// Base sampler seed; world `i` draws at `+ i`, so the worlds are independent
/// determinizations of the SAME view.
const DETERMINIZATION_SAMPLER_BASE: u64 = 0x0DE7_0000_0000_0001;
/// The candidate scheme's dice, held WORLD-COMMON (Lookahead's `candidate_rng`
/// discipline, applied here for the same reason): a per-world stream would make
/// the candidate LISTS differ for a reason that is not determinization, and the
/// cell would then be measuring its own harness.
const DETERMINIZATION_DICE: u64 = 0xD1CE_0000_0000_0001;

/// The order cards face-up in the port, folded. The REFILL channel's output:
/// public in the successor view, and drawn off the DETERMINIZED deck.
///
/// This is the POISON the determinization cell's red step used, kept in the tree
/// and RUN as a permanent control — the
/// `t2_mutation_proof_a_cheating_sampler_is_caught` discipline, one rung up. A
/// hypothetical seventeenth evaluator term that folded this into its value is
/// exactly the future edit §4.3's K-revisit trigger is watching for, and the
/// cell asserts that such a term WOULD be caught at every comparison where the
/// channel actually moved rather than merely quoting a failure from authoring
/// time.
/// The lab analog of [`port_fingerprint`] — the T8 review's should-fix: the
/// LAB channel (26 of the 64 genuinely-differing comparisons; refills off the
/// determinized TECH deck, the hidden member the spec's §0.4(ii) prose did not
/// name) was asserted live but had no permanent control vouching that a
/// tech-reading term would be caught. Now it has the same poison discipline as
/// the port.
fn lab_fingerprint(lab: &yokohama_core::LabState) -> u64 {
    let fold = |acc: u64, slots: &[Option<yokohama_core::TechCardId>; 6]| {
        slots.iter().fold(acc, |a, s| {
            a.wrapping_mul(31).wrapping_add(u64::from(s.map_or(0, |c| c.0)) + 1)
        })
    };
    let a = fold(0x1AB_0000_u64, &lab.lab_a.slots);
    lab.lab_b.as_ref().map_or(a, |b| fold(a, &b.slots))
}

fn port_fingerprint(port: &PortState) -> u64 {
    let fold = |acc: u64, slots: &[Option<yokohama_core::OrderCardId>; 6]| {
        slots.iter().fold(acc, |a, s| {
            a.wrapping_mul(31).wrapping_add(u64::from(s.map_or(0, |c| c.0)) + 1)
        })
    };
    let a = fold(0xF0_0000_u64, &port.port_a.slots);
    port.port_b.as_ref().map_or(a, |b| fold(a, &b.slots))
}

/// CONFIG PIN (T8 review): this cell's pipeline deliberately consumes greedy's
/// OWN constants — `DEFAULT_RETREAT_SAMPLES` and `WeightTable::V1`, the same
/// ones `GreedyAgent::new` consumes — so the re-implementation cannot drift on
/// its knobs. What it re-states by hand is the pipeline SHAPE (handle →
/// candidates → apply → observe → score, verified against `GreedyAgent::choose`
/// at the T8 review); if the agent's chain ever changes shape, move this cell
/// with it.
/// **THE FALSIFIABLE CELL — determinization invariance, and spec §4.3's
/// K-REVISIT TRIGGER made executable** (spec §0.4(ii), ledger L11).
///
/// From ONE view it draws [`DETERMINIZATION_WORLDS`] independent
/// `BeliefSampler` worlds, applies the SAME candidate in each, `observe`s the
/// successors, and requires Greedy-v1's whole candidate SCORE VECTOR, its
/// argmax, and the chosen move's `Explanation` to be EQUAL across worlds.
///
/// **THIS ONE CAN GENUINELY FAIL, and that is the entire reason it exists.**
/// The two tripwires above compare f(x) against f(x) — identical views by
/// construction. Here the inputs DIFFER: a Port take's refill draws off the
/// DETERMINIZED deck, so the successor's `port_state` carries content that is a
/// property of the sampled world and not of the view. The run found a SECOND
/// channel of the same shape that the spec does not name: a Laboratory take
/// refills the lab board off the determinized TECH deck (`observe.rs`'s hidden
/// member 3, "tech_deck order — size only survives"), and at authoring the two
/// account for the differing successor views exactly (64 = 38 port + 26 lab).
/// The positive-control counters below assert that both are LIVE in the run —
/// without them a green here would be indistinguishable from a green on a
/// vacuous cell, which is precisely the defect §0.4 was rewritten to fix — and
/// the POISON control goes one further: it RUNS the hypothetical
/// determinized-content-reading term and asserts it would have been caught at
/// every comparison where the channel moved.
///
/// **RED, RUN BEFORE THE GREEN** (the poison was `score + port_fingerprint`, one
/// term reading determinized content — i.e. exactly the future edit this cell
/// exists to catch):
///
/// > a determinization moved Greedy-v1's score for
/// > `ExecuteAreaAction { action: Port { cards_taken: [OrderCardId(8)], second_card_payment: None } }`
/// >   left: 13959226634306026
/// >  right: 13959226634306007
///
/// **WHAT A RED HERE MEANS, FOR WHOEVER SEES IT NEXT.** It is not (necessarily)
/// a bug. It means an evaluator term has started reading a quantity that depends
/// on which world was determinized — and per spec §4.3 that is the signal that
/// **K > 1 has become live**: at K = 1 such a term makes the bot's decision a
/// function of one arbitrary draw. Route it to the ledger as the K-decision
/// re-opening input; do not silence it by dropping the term from the comparison.
///
/// SCOPE, stated: the cell is about the ROOT channel (§4.3's structural half).
/// The TURN-COMPLETION channel — a continuation applying moves inside the
/// sampled world — is not structural and is not asserted anywhere; it is
/// MEASURED by the divergence instrument below.
///
/// TWO HARNESS CHOICES that keep the cell measuring the evaluator and not
/// itself. The candidate dice are held world-common (see
/// [`DETERMINIZATION_DICE`]), and only the INTERSECTION of the three worlds'
/// candidate sets is scored — so a candidate list that legitimately differs
/// between worlds cannot masquerade as a score difference. The argmax is
/// consequently over the COMMON list rather than over each world's full
/// candidate set: that is deliberately not "the move Greedy-v1 would play in
/// world w", because comparing those would conflate a differing candidate SET
/// with a differing SCORE. How often the intersection actually bites is
/// reported (`candidate sets differed`) rather than assumed, and the answer at
/// authoring is **0 of 600 points**: at 2P the acting seat's legality is
/// view-measurable and the dice are world-common, so the three worlds offered
/// IDENTICAL candidate lists everywhere and the cell in practice compares the
/// full list. The intersection stays as a correctness property of the harness
/// (a 3–4P or constructed state need not be so tidy), not as something this
/// run's green depends on.
#[test]
fn t8_greedy_v1s_scores_and_explanations_are_invariant_across_determinizations() {
    let rules = Rules::standard();
    let weights = WeightTable::V1;

    let mut checked_points = 0usize;
    let mut compared = 0usize;
    let mut explanations_compared = 0usize;
    let mut port_take_candidates = 0usize;
    let mut differing_successor_views = 0usize;
    let mut differing_port_states = 0usize;
    let mut differing_lab_states = 0usize;
    let mut poison_caught = 0usize;
    let mut poison_lab_caught = 0usize;
    let mut candidate_sets_differed = 0usize;

    let points: Vec<_> =
        DETERMINIZATION_WALKS.iter().flat_map(|s| decision_points(2, *s, 150, &rules)).collect();
    for (_truth, view, _legal) in points {
        let seat = view.viewer;
        let worlds: Vec<GameState> = (0..DETERMINIZATION_WORLDS)
            .map(|i| {
                let mut s = BeliefSampler::from_view(
                    &view,
                    &rules,
                    DETERMINIZATION_SAMPLER_BASE.wrapping_add(i as u64),
                )
                .expect("the sampler accepts an honest view");
                s.sample()
            })
            .collect();

        let per_world: Vec<Vec<Move>> = worlds
            .iter()
            .map(|w| {
                let handle = DecisionHandle::new(w, &rules);
                let Candidates { moves, .. } = candidates(
                    &handle,
                    DEFAULT_RETREAT_SAMPLES,
                    &mut ChaCha8Rng::seed_from_u64(DETERMINIZATION_DICE),
                );
                moves
            })
            .collect();

        if per_world[1..].iter().any(|w| *w != per_world[0]) {
            candidate_sets_differed += 1;
        }
        let common: Vec<Move> = per_world[0]
            .iter()
            .filter(|m| per_world[1..].iter().all(|w| w.contains(m)))
            .cloned()
            .collect();
        if common.is_empty() {
            continue;
        }
        checked_points += 1;

        let mut successors: Vec<Vec<PlayerView>> = Vec::with_capacity(worlds.len());
        let mut vectors: Vec<Vec<i64>> = Vec::with_capacity(worlds.len());
        for w in &worlds {
            let mut views = Vec::with_capacity(common.len());
            let mut scores = Vec::with_capacity(common.len());
            for mv in &common {
                let next = apply_move_unchecked(w, mv, &rules)
                    .expect("a candidate common to every world applies in every world");
                let sview = observe(&next, seat).expect("the acting seat can observe its successor");
                scores.push(eval::score(&sview, &rules, &weights));
                views.push(sview);
            }
            successors.push(views);
            vectors.push(scores);
        }

        for (ci, mv) in common.iter().enumerate() {
            if matches!(
                mv,
                Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } }
                    if !cards_taken.is_empty()
            ) {
                port_take_candidates += 1;
            }
            for w in 1..worlds.len() {
                if successors[w][ci] != successors[0][ci] {
                    differing_successor_views += 1;
                }
                if successors[w][ci].port_state != successors[0][ci].port_state {
                    differing_port_states += 1;
                    // THE POISON, RUN (see `port_fingerprint`): the score a
                    // hypothetical determinized-content-reading term would
                    // have produced must MOVE here — otherwise the equality
                    // asserted above is being checked on a quantity nothing
                    // could disturb.
                    let poisoned = |v: &PlayerView| {
                        eval::score(v, &rules, &weights)
                            .wrapping_add(port_fingerprint(&v.port_state) as i64)
                    };
                    if poisoned(&successors[w][ci]) != poisoned(&successors[0][ci]) {
                        poison_caught += 1;
                    }
                }
                if successors[w][ci].lab_state != successors[0][ci].lab_state {
                    differing_lab_states += 1;
                    // The lab channel's own poison (T8 review): a tech-reading
                    // term must move wherever the tech-deck refill differed.
                    let poisoned = |v: &PlayerView| {
                        eval::score(v, &rules, &weights)
                            .wrapping_add(lab_fingerprint(&v.lab_state) as i64)
                    };
                    if poisoned(&successors[w][ci]) != poisoned(&successors[0][ci]) {
                        poison_lab_caught += 1;
                    }
                }
                assert_eq!(
                    vectors[w][ci], vectors[0][ci],
                    "a determinization moved Greedy-v1's score for {mv:?}"
                );
                compared += 1;
            }
        }

        // The ARGMAX and the chosen move's EXPLANATION, not just the vector:
        // equal score vectors already imply an equal argmax, so this is a
        // cross-check on the comparison itself — and the explanation is a
        // SEPARATE path (§0.3(a)), so it can move where `score` does not.
        let argmax = |v: &[i64]| {
            let mut best = 0usize;
            for (i, s) in v.iter().enumerate() {
                if *s > v[best] {
                    best = i;
                }
            }
            best
        };
        let base_arg = argmax(&vectors[0]);
        let base_explanation: Explanation =
            eval::explain(&successors[0][base_arg], &rules, &weights);
        for w in 1..worlds.len() {
            let arg = argmax(&vectors[w]);
            assert_eq!(
                arg, base_arg,
                "a determinization moved Greedy-v1's argmax ({:?} vs {:?})",
                common[arg], common[base_arg]
            );
            assert_eq!(
                eval::explain(&successors[w][arg], &rules, &weights),
                base_explanation,
                "a determinization moved the chosen move's explanation ({:?})",
                common[arg]
            );
            explanations_compared += 1;
        }
    }

    eprintln!(
        "determinization invariance: {checked_points} points, {compared} score comparisons, \
         {explanations_compared} argmax+explanation comparisons, {port_take_candidates} port-take \
         candidates, candidate sets differed at {candidate_sets_differed} points; POSITIVE \
         CONTROLS: {differing_successor_views} successor views differed ({differing_port_states} \
         by port refill, {differing_lab_states} by lab refill), poison caught {poison_caught}"
    );

    // Coverage floors, MEASURED FIRST AND THEN SET (§0.2's authoring rule; the
    // run above prints every figure). Measured at authoring on the four walks:
    // 600 points, 49,604 score comparisons, 1,200 argmax+explanation
    // comparisons. Set at ~half, rounded down.
    assert!(checked_points >= 400, "only {checked_points} decision points");
    assert!(compared >= 20_000, "only {compared} score comparisons");
    assert!(explanations_compared >= 800, "only {explanations_compared} explanation comparisons");

    // THE NON-VACUITY ASSERTIONS — what makes this cell §0.4(ii) and not
    // another §0.4(i). Floors MEASURED THEN SET at ~half the authoring run's
    // values (19 / 64 / 38 / 26 → 9 / 32 / 19 / 13) — the T8 review corrected
    // the first draft's `> 0` here, which had numeric floors on the mostly-
    // vacuous counters while the load-bearing ones got the weakest possible
    // check; the four-walk deviation exists precisely because one walk nearly
    // degenerated (4 Port takes), and half-of-measured is the guard against
    // that silent degeneration, same as the counters above.
    assert!(
        port_take_candidates >= 9,
        "no Port-take candidate was scored, so the refill channel this cell exists to watch was \
         never exercised"
    );
    assert!(
        differing_successor_views >= 32,
        "every world produced an IDENTICAL successor view, so this cell degenerated into f(x) vs \
         f(x) — the same vacuity spec §0.4 rejected for the sweep cells"
    );
    assert!(
        differing_port_states >= 19,
        "no successor's port_state differed across worlds: the determinized ORDER-deck refill \
         channel was not live in this run, so the equality above proves nothing about it"
    );
    assert!(
        differing_lab_states >= 13,
        "no successor's lab_state differed across worlds: the determinized TECH-deck refill \
         channel was not live in this run"
    );
    // THE DETECTOR DETECTS, permanently rather than by a quoted authoring run:
    // every comparison where the refill channel moved is one a
    // determinized-content-reading term would have been caught at. Equality
    // rather than `> 0` because both sides are deterministic here (fixed walks,
    // fixed sampler seeds) and a shortfall would mean the fingerprint collided
    // — worth hearing about.
    assert_eq!(
        poison_caught, differing_port_states,
        "the poisoned score failed to move at {} of {differing_port_states} comparisons where the \
         port refill genuinely differed — the control cannot vouch for the assertion above",
        differing_port_states - poison_caught
    );
    assert_eq!(
        poison_lab_caught, differing_lab_states,
        "the lab-poisoned score failed to move at {} of {differing_lab_states} comparisons where \
         the tech-deck refill genuinely differed (T8-review control: the lab channel needs its \
         own vouching, not the port's)",
        differing_lab_states - poison_lab_caught
    );
}

/// The walks the divergence instrument reads its organic decision points from,
/// and how many points it takes from each. Two 2P games rather than one so the
/// rate is not a statement about a single trajectory.
const DIVERGENCE_WALKS: [u64; 2] = [0, 1];
/// Points per walk. `2 × 100 = 200`, the floor spec §0.4(iii)'s instrument is
/// sized at.
const DIVERGENCE_POINTS_PER_WALK: usize = 100;
/// `K` for the measured arm. Two would do; three costs one more world search
/// per point and makes a "worlds 1 and 2 agreed by luck" reading unavailable.
const DIVERGENCE_WORLDS: u8 = 3;
/// How many points the stream-perturbation sensitivity control runs on. A
/// subset, because it costs one more whole search per point and its claim
/// ("the comparison CAN fail") needs a handful of witnesses, not 200.
const DIVERGENCE_SENSITIVITY_POINTS: usize = 40;

/// **THE LOOKAHEAD DIVERGENCE INSTRUMENT — spec §0.4(iii) / §4.3, RECORDED.**
///
/// WHAT IT MEASURES. At each organic decision point, two FRESH `LookaheadAgent`s
/// with the SAME agent seed and the SAME knobs decide: one at `K = 1`, one at
/// `K = DIVERGENCE_WORLDS`. Divergence = the two chose different moves.
///
/// WHY THAT IS "aggregate vs world 0" and not two unrelated runs. A fresh agent
/// spends exactly three play-stream words per decision, drawn BEFORE the world
/// loop (`game/lookahead.rs:1498-1518`): the sampler seed, the [`TieStream`] key,
/// and the candidate-stream seed. Both agents therefore draw the same three
/// words, sample the same FIRST world, and search it with the same tie draws and
/// the same candidate dice — so the `K = 1` answer IS world 0's argmax inside
/// the `K = 3` run, and the only thing that can move the `K = 3` answer is the
/// aggregation over worlds 1 and 2. That keying is T5's fix wave
/// (`game/lookahead.rs:44-60`, `:1470-1490`): lens 2 measured a NAIVE instrument
/// reading 7.66% false divergence at `K = 1` from tie draws alone, and the
/// candidate dice were an open second channel until the same wave closed it.
/// Residual divergence here is determinization-driven and nothing else.
///
/// SCOPE OF THE FIGURE, stated so it is not over-quoted. It is measured at
/// `B = 1` / `M = 2` (see [`TRIPWIRE_BEAM`]) — reduced knobs, cost-driven, and a
/// real narrowing: at `B = 1` the continuation is a single greedy line, so there
/// are fewer states per root candidate for a determinized refill to reach than
/// at the shipped `B = 4`. The instrument PROPERTY (world-common dice; a
/// divergence that means determinization) is what the knobs do not touch. The
/// figure is a statement about this cell, at these knobs, on these two walks.
///
/// WHAT IS ASSERTED, and why the zero is a legitimate pin here rather than an
/// assumption. §4.3 names the turn-completion channel as NOT structural: an S1
/// continuation applies moves inside the sampled world, so a mid-turn Port take
/// can put determinized refills into the hand-off state and per-world hand-offs
/// CAN genuinely differ. Spec §0.4(iii) therefore says record, do not assert
/// zero — and lens 2's MUST-4 is exactly that instruction. THE MEASUREMENT WAS
/// TAKEN FIRST AND THE ASSERTION WRITTEN AFTER (measured-then-set):
///
/// > 200 points (mean 37.77 root candidates over ALL points), 94 contested,
/// > **0 diverged = 0.0000**; ply_cap_hits 0; the K worlds drew a different
/// > deck at 200 points; a perturbed stream moved the pick at 9 of 40 probed
/// > points
///
/// The mechanism is visible once measured: no v1 feature reads a
/// refill-dependent quantity, so two hand-off states that differ ONLY in
/// refilled port/lab content score IDENTICALLY, and a determinization has to
/// change something the sixteen features can see before it can move an argmax.
/// The zero is therefore pinned as a REGRESSION NET on that reading — NOT as a
/// structural claim — and the assert's message says what to do if it ever
/// fires: **do not relax it.** A nonzero rate is §4.3's K-decision re-opening
/// input, with the divergence RATE (not a win rate) as the acceptance
/// statistic; investigate the turn-completion channel, record the mechanism,
/// and route it to the ledger.
///
/// TWO CONTROLS, because a zero is the easiest number in the world to produce
/// vacuously and this file's whole doctrine is that a detector must be shown to
/// detect. (1) the `K` worlds GENUINELY DIFFER — reproduced from the agent's own
/// first stream word, 200/200 points drew a different deck; without this a zero
/// would be arithmetic. (2) the comparison CAN FIRE — a `K = 1` agent on a
/// perturbed stream picks differently at 9 of 40 probed points, so
/// `aggregate != world0` is not comparing two quantities that are equal by
/// construction. The 51.9%-forced figure `game/lookahead.rs`'s module doc
/// records is corroborated in passing: 106 of the 200 points offered ≤ 1 root
/// candidate, which is why the contested count is the denominator worth reading.
///
/// [`TieStream`]: yokohama_core::game::lookahead::TieStream
#[test]
fn t8_lookahead_v1s_cross_world_divergence_is_measured_and_recorded() {
    let rules = Rules::standard();

    let mut points = 0usize;
    let mut contested = 0usize;
    let mut diverged = 0usize;
    let mut diverged_contested = 0usize;
    let mut ply_cap_hits = 0u64;
    let mut root_candidates_total = 0u64;
    let mut worlds_differed = 0usize;
    let mut sensitivity_probed = 0usize;
    let mut sensitivity_moved = 0usize;

    for walk in DIVERGENCE_WALKS {
        for (_truth, view, _legal) in decision_points(2, walk, DIVERGENCE_POINTS_PER_WALK, &rules)
        {
            let mut one = LookaheadAgent::with_knobs(
                &rules,
                SearchScheme::TurnBoundary { beam: TRIPWIRE_BEAM },
                1,
                TRIPWIRE_RETREAT_SAMPLES,
                WeightTable::V1,
                HONEST_AGENT_SEED,
            );
            let world0 = one.choose(&view, &rules).expect("Lookahead-v1 resolves a move").mv;
            let stats = *one.last_stats().expect("a decision records its stats");

            let mut many = LookaheadAgent::with_knobs(
                &rules,
                SearchScheme::TurnBoundary { beam: TRIPWIRE_BEAM },
                DIVERGENCE_WORLDS,
                TRIPWIRE_RETREAT_SAMPLES,
                WeightTable::V1,
                HONEST_AGENT_SEED,
            );
            let aggregate = many.choose(&view, &rules).expect("Lookahead-v1 resolves a move").mv;

            // POSITIVE CONTROL, and the instrument is vacuous without it: if
            // the `K` worlds were the SAME world, a zero divergence rate would
            // be arithmetic rather than evidence. The agent's own worlds are
            // REPRODUCED here, not guessed — a fresh agent's FIRST play-stream
            // draw is its sampler seed (`game/lookahead.rs:1498`), so the same
            // `ChaCha8Rng` at the same agent seed hands back the same `u64`,
            // and successive `sample()` calls are the same successive worlds
            // the K loop searched.
            let sampler_seed: u64 = ChaCha8Rng::seed_from_u64(HONEST_AGENT_SEED).gen();
            let mut sampler = BeliefSampler::from_view(&view, &rules, sampler_seed)
                .expect("the sampler accepts an honest view");
            let drawn: Vec<GameState> =
                (0..DIVERGENCE_WORLDS).map(|_| sampler.sample()).collect();
            if drawn[1..].iter().any(|w| w.order_deck != drawn[0].order_deck) {
                worlds_differed += 1;
            }

            // SECOND CONTROL — the comparison operator is not trivially true.
            // Run on the first [`DIVERGENCE_SENSITIVITY_POINTS`] points only
            // (it costs a whole extra search each): a `K = 1` agent on a
            // PERTURBED stream must sometimes pick a different move, or the
            // `aggregate != world0` test could be reading two things that
            // cannot differ whatever the worlds do. This is the file's canary
            // discipline — the detector must be shown to detect.
            if points < DIVERGENCE_SENSITIVITY_POINTS {
                let mut other = LookaheadAgent::with_knobs(
                    &rules,
                    SearchScheme::TurnBoundary { beam: TRIPWIRE_BEAM },
                    1,
                    TRIPWIRE_RETREAT_SAMPLES,
                    WeightTable::V1,
                    HONEST_AGENT_SEED ^ 0xA5A5_A5A5_A5A5_A5A5,
                );
                let pick = other.choose(&view, &rules).expect("Lookahead-v1 resolves a move").mv;
                sensitivity_probed += 1;
                if pick != world0 {
                    sensitivity_moved += 1;
                }
            }

            points += 1;
            root_candidates_total += stats.root_candidates;
            ply_cap_hits += stats.ply_cap_hits
                + many.last_stats().map_or(0, |s| s.ply_cap_hits);
            let is_contested = stats.root_candidates >= 2;
            if is_contested {
                contested += 1;
            }
            if aggregate != world0 {
                diverged += 1;
                if is_contested {
                    diverged_contested += 1;
                }
            }
        }
    }

    let rate = diverged as f64 / points as f64;
    let contested_rate = diverged_contested as f64 / contested.max(1) as f64;
    eprintln!(
        "lookahead divergence (RECORDED, spec §0.4(iii)): K={DIVERGENCE_WORLDS} vs K=1, \
         scheme=TurnBoundary{{beam:{TRIPWIRE_BEAM}}}, M={TRIPWIRE_RETREAT_SAMPLES}, \
         weights=V1, agent_seed={HONEST_AGENT_SEED:#018x}, walks={DIVERGENCE_WALKS:?} \
         (2P, {DIVERGENCE_POINTS_PER_WALK} points each); {points} points (mean {:.2} root \
         candidates over ALL points), {contested} contested, {diverged} diverged = {:.4} \
         ({diverged_contested} of the \
         contested = {contested_rate:.4}); ply_cap_hits {ply_cap_hits}; POSITIVE CONTROL: the K \
         worlds drew a different deck at {worlds_differed} points; SENSITIVITY: a perturbed \
         stream moved the pick at {sensitivity_moved} of {sensitivity_probed} probed points",
        root_candidates_total as f64 / points as f64,
        rate,
    );

    // COMPLETENESS is asserted unconditionally: a rate over too few points is
    // not a measurement.
    assert!(points >= 200, "instrument incompleteness: only {points} decision points");
    assert!(
        contested >= 50,
        "only {contested} points offered >= 2 root candidates, so divergence was structurally \
         impossible at the rest and the rate is about the wrong denominator"
    );
    assert!(
        sensitivity_moved > 0,
        "a perturbed stream never moved the pick over {sensitivity_probed} points, so the \
         `aggregate != world0` comparison has not been shown capable of firing at all"
    );
    assert!(
        worlds_differed >= 100,
        "the K worlds drew the same deck at all but {worlds_differed} points — a zero divergence \
         rate over identical worlds is arithmetic, not evidence"
    );
    assert_eq!(
        ply_cap_hits, 0,
        "a continuation hit MAX_TURN_PLIES, so some hand-off was read MID-TURN and the \
         divergence figure is about a different quantity than it claims"
    );
    assert_eq!(
        diverged, 0,
        "cross-world divergence is NO LONGER ZERO ({diverged}/{points}). DO NOT relax this \
         assertion. TWO causes, and they are not the same finding. (a) A world-common keying \
         regression in `LookaheadAgent::choose`'s WORLD LOOP (a tie draw or candidate draw \
         re-keyed per world — T5's fix wave undone): CHECK THIS FIRST, and know that THIS CELL \
         IS THE SOLE DETECTOR of it — the T8 review measured both canonical regressions and \
         `the_candidate_dice_are_world_common_within_a_decision` and \
         `a_tie_draw_is_one_keyed_word_and_is_world_common` STAY GREEN under both (they drive \
         the search internals, not the agent's world loop; per-world tie re-keying read \
         31/200 here with every other test in the crate green, per-world candidate re-keying \
         7/200 — the lens-2 shape). Rule (a) out by inspecting the world loop's seed \
         derivations directly. (b) With the keying verified intact, this is spec §4.3's \
         K-decision re-opening input — the turn-completion channel has started moving a root \
         argmax, the divergence RATE is the acceptance statistic, and the mechanism belongs \
         in the ledger before K stays at 1"
    );
}
