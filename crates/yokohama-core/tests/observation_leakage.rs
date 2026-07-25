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

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game,
    observe, BeliefError, BeliefSampler, GameConfig, GameState, HandCard, Move, PlayerId,
    PlayerView, Rules, StartingOrderChoice, ViewError,
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

#[test]
fn t2_the_canarys_decision_never_moves_under_hidden_variation() {
    let rules = Rules::standard();
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    let mut checked = 0usize;
    for (truth, view, legal) in decision_points(2, 0, 150, &rules) {
        let base = canary_pick(&view, &legal, &rules, 42, None);
        for v in variants(&truth, &mut rng) {
            let vv = observe(&v, v.current_player).unwrap();
            let vlegal = get_legal_moves(&v, &rules);
            assert_eq!(
                canary_pick(&vv, &vlegal, &rules, 42, None),
                base,
                "T2: an agent's decision moved when only hidden state changed"
            );
            checked += 1;
        }
    }
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
