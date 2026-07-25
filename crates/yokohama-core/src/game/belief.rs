//! `BeliefSampler`: draw full `GameState`s consistent with one seat's
//! `PlayerView` (Phase 3.0 determinization).
//!
//! v1 contract: CONSERVATION + CONSISTENCY, uniform. Every unseen order card
//! is equally likely to be anywhere it could be; the tech deck is a uniform
//! permutation of an exactly-known remainder. Posterior FIDELITY (which of
//! two dealt cards an opponent chose to keep is a PREFERENCE, not chance --
//! rulebook p.4 step 12) is a measured v2 refinement, deliberately deferred:
//! an unfulfilled order carries zero terminal-score weight (game/scoring.rs
//! reads `completed_orders` only), so mis-sampling one costs nothing at
//! evaluation, only in opponent-action prediction.
//!
//! CARRY-FORWARD LIMIT (spec L3): a sampled world is NOT safe to roll forward
//! across an observed Port/Lab refill -- a refill publicly reveals a deck card
//! (game/cards.rs:230, :266), so a search must re-sample or condition after
//! each one.
//!
//! Like `game::observe`, world construction here uses FULL struct literals
//! (neither `GameState` nor `PlayerState` implements `Default`), so a new
//! state field is a compile error here too. Never build a world by cloning a
//! true `GameState` and patching it -- that reintroduces the leak this whole
//! module exists to prevent.

use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::game::rules::Rules;
use crate::types::{
    GameConfig, GameState, HandCard, OrderCardId, PlayerId, PlayerState, PlayerView, TechCardId,
    VisiblePlayer,
};

/// Why a view cannot be sampled from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeliefError {
    /// The view's published sizes do not reconcile with its unseen pool.
    Inconsistent(String),
    /// Materializing this seat's hidden starting order would exceed the
    /// 3-card hand limit.
    HandOverflow(PlayerId),
    /// A card in the view is not in the registry (or the registry is not the
    /// standard 36/30).
    RegistryMismatch(String),
}

impl std::fmt::Display for BeliefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeliefError::Inconsistent(m) => write!(f, "view does not reconcile: {m}"),
            BeliefError::HandOverflow(p) => {
                write!(f, "seat {}'s hidden card would exceed the 3-card hand limit", p.0)
            }
            BeliefError::RegistryMismatch(m) => write!(f, "registry mismatch: {m}"),
        }
    }
}

impl std::error::Error for BeliefError {}

/// Samples worlds consistent with one seat's observation.
pub struct BeliefSampler {
    view: PlayerView,
    unseen_orders: Vec<OrderCardId>,
    tech_remainder: Vec<TechCardId>,
    /// (player index, hand index) of each masked slot, ascending.
    hidden_slots: Vec<(usize, usize)>,
    rng: ChaCha8Rng,
}

impl BeliefSampler {
    pub fn from_view(view: &PlayerView, rules: &Rules, seed: u64) -> Result<Self, BeliefError> {
        let unseen_orders = view.unseen_orders(rules);
        let tech_remainder = view.tech_deck_remainder(rules);

        // A masked slot is materializable only if it is SOMEONE ELSE'S single
        // unfulfilled deal. Both refusals below are unreachable through
        // `observe` -- exactly like the two size guards -- and are here for the
        // same reason: `PlayerView` has public fields, and the worlds these
        // shapes would produce conserve and pass `check_invariants` while
        // silently failing to observe back to the view that produced them.
        let mut hidden_slots: Vec<(usize, usize)> = Vec::new();
        for (pi, p) in view.players.iter().enumerate() {
            let mut slots_this_seat = 0usize;
            for (hi, c) in p.hand.iter().enumerate() {
                if *c == HandCard::HiddenStartingOrder {
                    // The hand-size cap is an engine invariant
                    // (game/invariants.rs:27), so a hand this deep cannot be
                    // materialized into any legal world -- refuse rather than
                    // emit one that fails `check_invariants`.
                    if p.hand.len() > 3 {
                        return Err(BeliefError::HandOverflow(p.id));
                    }
                    // A seat cannot be unknown to itself. Sampling one would
                    // invent a card for our OWN hand and clobber the real
                    // `starting_order` carried through by `materialize_player`.
                    if p.id == view.viewer {
                        return Err(BeliefError::Inconsistent(format!(
                            "seat {} is the viewer and cannot hold a hidden starting order",
                            p.id.0
                        )));
                    }
                    slots_this_seat += 1;
                    // One deal, one hidden card. `starting_order` holds a
                    // single id, so a second slot would leave a hand position
                    // holding a card no record points at -- and `observe`
                    // would re-mask only one of the two.
                    if slots_this_seat > 1 {
                        return Err(BeliefError::Inconsistent(format!(
                            "seat {} carries more than one hidden starting-order slot; \
                             a seat is dealt exactly one",
                            p.id.0
                        )));
                    }
                    hidden_slots.push((pi, hi));
                }
            }
        }
        // Single-sources the census against the reconciliation identity Task 3
        // pinned with `hidden_slot_count()`: if the two ever disagree, `need`
        // below is computed from a different set than `sample` writes.
        debug_assert_eq!(hidden_slots.len(), view.hidden_slot_count());

        if tech_remainder.len() != view.tech_deck_len {
            return Err(BeliefError::RegistryMismatch(format!(
                "tech remainder {} != published deck size {}",
                tech_remainder.len(),
                view.tech_deck_len
            )));
        }
        let need = view.order_deck_len + view.orders_removed_len + hidden_slots.len();
        if unseen_orders.len() != need {
            return Err(BeliefError::Inconsistent(format!(
                "unseen pool {} != deck {} + box {} + hidden slots {}",
                unseen_orders.len(),
                view.order_deck_len,
                view.orders_removed_len,
                hidden_slots.len()
            )));
        }

        Ok(Self {
            view: view.clone(),
            unseen_orders,
            tech_remainder,
            hidden_slots,
            rng: ChaCha8Rng::seed_from_u64(seed),
        })
    }

    /// One world consistent with the observation. Cheap enough to call per
    /// search node (a clone plus two shuffles).
    pub fn sample(&mut self) -> GameState {
        let mut pool = self.unseen_orders.clone();
        pool.shuffle(&mut self.rng);
        let deck_len = self.view.order_deck_len;
        let order_deck: Vec<OrderCardId> = pool[..deck_len].to_vec();
        let hidden: Vec<OrderCardId> = pool[deck_len..deck_len + self.hidden_slots.len()].to_vec();
        let mut orders_removed: Vec<OrderCardId> =
            pool[deck_len + self.hidden_slots.len()..].to_vec();
        // Rules-inert contents: canonicalize so two behaviourally identical
        // worlds are the same state. (`OrderCardId` has no `Ord`.)
        orders_removed.sort_by_key(|c| c.0);

        let mut tech_deck = self.tech_remainder.clone();
        tech_deck.shuffle(&mut self.rng);

        let mut players: Vec<PlayerState> =
            self.view.players.iter().map(materialize_player).collect();
        for ((pi, hi), card) in self.hidden_slots.iter().zip(hidden) {
            players[*pi].order_cards_in_hand[*hi] = card;
            players[*pi].starting_order = Some(card);
        }

        let v = &self.view;
        GameState {
            config: GameConfig {
                player_count: v.config.player_count,
                player_colors: v.config.player_colors.clone(),
                start_player: v.config.start_player,
                has_port_b: v.config.has_port_b,
                has_lab_b: v.config.has_lab_b,
                // A sampled world is a FICTION. 0 means "not seed-generatable":
                // never hand one to GameRecord/replay as if it were a real game.
                setup_seed: 0,
            },
            board_layout: v.board_layout.clone(),
            players,
            current_player: v.current_player,
            turn_phase: v.turn_phase,
            turn_number: v.turn_number,
            additional_action_state: v.additional_action_state.clone(),
            current_action_power: v.current_action_power,
            is_mercenary_action: v.is_mercenary_action,
            mercenary_target_area: v.mercenary_target_area,
            mercenary_return_phase: v.mercenary_return_phase,
            president_start_area: v.president_start_area,
            port_state: v.port_state.clone(),
            lab_state: v.lab_state.clone(),
            order_deck,
            tech_deck,
            orders_removed,
            church_track: v.church_track.clone(),
            customs_track: v.customs_track.clone(),
            achievements: v.achievements.clone(),
            mercenary_supply: v.mercenary_supply.clone(),
            station_location: v.station_location,
            end_game_triggered: v.end_game_triggered,
            trigger_reason: v.trigger_reason,
            final_round: v.final_round,
            trigger_player: v.trigger_player,
            pending_choices: v.pending_choices.clone(),
        }
    }
}

/// A `VisiblePlayer` back to a `PlayerState`. Masked hand slots are filled
/// with a placeholder that `sample` immediately overwrites -- keeping hand
/// LENGTH and every other index exactly as observed.
fn materialize_player(p: &VisiblePlayer) -> PlayerState {
    let order_cards_in_hand: Vec<OrderCardId> = p
        .hand
        .iter()
        .map(|c| match c {
            HandCard::Known(id) => *id,
            HandCard::HiddenStartingOrder => OrderCardId(0), // overwritten by `sample`
        })
        .collect();
    PlayerState {
        id: p.id,
        color: p.color,
        victory_points: p.victory_points,
        president_location: p.president_location,
        assistants: p.assistants.clone(),
        shophouses: p.shophouses.clone(),
        trading_houses: p.trading_houses.clone(),
        money: p.money,
        copper: p.copper,
        silk: p.silk,
        tea: p.tea,
        seafood: p.seafood,
        imported_goods: p.imported_goods,
        order_cards_in_hand,
        starting_order: p.starting_order,
        // Samples are always Keep-shaped: the card is materialized, so nothing
        // about this world is "unrecorded".
        starting_order_hidden: false,
        completed_orders: p.completed_orders.clone(),
        technologies: p.technologies.clone(),
        five_power_tokens: p.five_power_tokens,
        mercenaries: p.mercenaries.clone(),
        church_positions: p.church_positions.clone(),
        customs_positions: p.customs_positions.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::invariants::check_invariants;
    use crate::game::observe::observe;
    use crate::game::rules::Rules;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::{GameConfig, HandCard, PlayerId};

    fn keep_game(players: u8, seed: u64) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, seed).unwrap();
        let choices: Vec<_> = (0..players).map(|_| StartingOrderChoice::Keep(0)).collect();
        (finalize_setup(pending, &choices).unwrap(), rules)
    }

    /// Viewer keeps; every OTHER seat is a live `Unknown` (the shape a
    /// physically-tracked game produces).
    fn live_game(players: u8, seed: u64) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, seed).unwrap();
        let mut choices = vec![StartingOrderChoice::Keep(0)];
        choices.extend((1..players).map(|_| StartingOrderChoice::Unknown));
        (finalize_setup(pending, &choices).unwrap(), rules)
    }

    #[test]
    fn a_sample_conserves_every_card_exactly_once() {
        let (state, rules) = keep_game(2, 7);
        let view = observe(&state, PlayerId(0)).unwrap();
        let mut s = BeliefSampler::from_view(&view, &rules, 99).unwrap();
        for _ in 0..20 {
            let w = s.sample();
            // 36 order ids, each in exactly ONE zone. `check_invariants` counts
            // but does not identify, so this multiset check is the real gate.
            let mut seen: Vec<u8> = Vec::new();
            seen.extend(w.order_deck.iter().map(|c| c.0));
            seen.extend(w.orders_removed.iter().map(|c| c.0));
            for slot in w.port_state.port_a.slots.iter().flatten() {
                seen.push(slot.0);
            }
            if let Some(b) = &w.port_state.port_b {
                for slot in b.slots.iter().flatten() {
                    seen.push(slot.0);
                }
            }
            for p in &w.players {
                seen.extend(p.order_cards_in_hand.iter().map(|c| c.0));
                seen.extend(p.completed_orders.iter().map(|c| c.0));
            }
            seen.sort_unstable();
            let expected: Vec<u8> = {
                let mut v: Vec<u8> =
                    rules.orders.iter_sorted_by_id().iter().map(|c| c.id.0).collect();
                v.sort_unstable();
                v
            };
            assert_eq!(seen, expected, "every order id exactly once");
            assert!(check_invariants(&w, &rules).is_ok(), "sampled world is a valid state");
        }
    }

    #[test]
    fn a_sample_is_deterministic_from_view_and_seed() {
        let (state, rules) = keep_game(3, 12);
        let view = observe(&state, PlayerId(0)).unwrap();
        let a: Vec<_> = {
            let mut s = BeliefSampler::from_view(&view, &rules, 5).unwrap();
            (0..5).map(|_| s.sample()).collect()
        };
        let b: Vec<_> = {
            let mut s = BeliefSampler::from_view(&view, &rules, 5).unwrap();
            (0..5).map(|_| s.sample()).collect()
        };
        assert_eq!(a, b, "same (view, seed) => same sequence");
        let c: Vec<_> = {
            let mut s = BeliefSampler::from_view(&view, &rules, 6).unwrap();
            (0..5).map(|_| s.sample()).collect()
        };
        assert_ne!(a, c, "a different sampler seed explores a different world");
    }

    #[test]
    fn a_sample_round_trips_back_to_the_same_view() {
        for players in [2u8, 3, 4] {
            let (state, rules) = keep_game(players, 7);
            let view = observe(&state, PlayerId(0)).unwrap();
            let mut s = BeliefSampler::from_view(&view, &rules, 3).unwrap();
            for _ in 0..5 {
                let w = s.sample();
                assert_eq!(
                    observe(&w, PlayerId(0)).unwrap(),
                    view,
                    "{players}P: a sampled world must be indistinguishable from truth to this seat"
                );
            }
        }
    }

    /// Beyond the brief's six: the LIVE tracking shape is this project's
    /// first-priority use case, and it is where sampling actually earns its
    /// keep -- the engine holds NOTHING for an `Unknown` seat (both dealt
    /// cards sit in `orders_removed`), so the sampler is what turns a tracked
    /// physical game into a world a search can roll forward. The round trip is
    /// non-obvious here in a way the Keep shape is not: the sampled world is
    /// Keep-shaped, so `observe` must derive the same view from a DIFFERENT
    /// underlying representation than the one that produced it.
    #[test]
    fn a_sample_from_a_live_tracked_view_round_trips_and_is_valid() {
        for players in [2u8, 3, 4] {
            let (state, rules) = live_game(players, 7);
            let view = observe(&state, PlayerId(0)).unwrap();
            let mut s = BeliefSampler::from_view(&view, &rules, 3).unwrap();
            for _ in 0..5 {
                let w = s.sample();
                assert!(
                    w.players.iter().all(|p| !p.starting_order_hidden),
                    "{players}P: sampling materializes the unrecorded deal"
                );
                assert!(check_invariants(&w, &rules).is_ok(), "{players}P: valid world");
                assert_eq!(
                    observe(&w, PlayerId(0)).unwrap(),
                    view,
                    "{players}P: a world sampled from a live-tracked view must \
                     observe back to it, despite being Keep-shaped underneath"
                );
            }
        }
    }

    #[test]
    fn hidden_slots_are_filled_position_exactly_and_removed_is_canonical() {
        let (state, rules) = keep_game(2, 7);
        let view = observe(&state, PlayerId(0)).unwrap();
        let hidden_idx =
            view.players[1].hand.iter().position(|c| *c == HandCard::HiddenStartingOrder).unwrap();
        let mut s = BeliefSampler::from_view(&view, &rules, 1).unwrap();
        let w = s.sample();

        // The drawn card sits at the SAME hand index the view masked, and is
        // written to BOTH storage sites (the Keep shape).
        let drawn = w.players[1].order_cards_in_hand[hidden_idx];
        assert_eq!(w.players[1].starting_order, Some(drawn));
        assert!(!w.players[1].starting_order_hidden, "samples are always Keep-shaped");

        // The boxed pile is rules-inert, so its order is canonicalized -- two
        // behaviourally identical samples must not be spuriously distinct.
        let mut sorted = w.orders_removed.clone();
        sorted.sort_by_key(|c| c.0);
        assert_eq!(w.orders_removed, sorted);
    }

    #[test]
    fn a_sampled_world_carries_no_seed() {
        let (state, rules) = keep_game(2, 4242);
        let view = observe(&state, PlayerId(0)).unwrap();
        let mut s = BeliefSampler::from_view(&view, &rules, 1).unwrap();
        assert_eq!(s.sample().config.setup_seed, 0, "a sampled world is a fiction, not a seeded game");
    }

    /// Beyond the brief's six: the two reconciliation guards are the only
    /// thing standing between a mis-tracked view and a `sample()` that panics
    /// on an out-of-range slice, so neither may be a dead arm. `PlayerView`
    /// has public fields, so a caller really can hand us a view whose
    /// published sizes disagree with its unseen pool.
    #[test]
    fn a_view_whose_sizes_do_not_reconcile_is_refused() {
        let (state, rules) = keep_game(2, 7);
        let mut view = observe(&state, PlayerId(0)).unwrap();
        view.order_deck_len += 1; // claims one more card than exists unseen
        assert!(matches!(
            BeliefSampler::from_view(&view, &rules, 1),
            Err(BeliefError::Inconsistent(_))
        ));
    }

    #[test]
    fn a_view_whose_tech_deck_size_is_underivable_is_refused() {
        // The tech remainder is EXACT (nothing is boxed at setup), so a
        // published size it cannot reproduce means the view and the registry
        // disagree about which cards exist.
        let (state, rules) = keep_game(2, 7);
        let mut view = observe(&state, PlayerId(0)).unwrap();
        view.tech_deck_len += 1;
        assert!(matches!(
            BeliefSampler::from_view(&view, &rules, 1),
            Err(BeliefError::RegistryMismatch(_))
        ));
    }

    /// A seat cannot be unknown to ITSELF. `observe` can never build such a
    /// view (the viewer's hand is rendered all-`Known`, and an unrecorded own
    /// deal is refused outright), so this is hand-constructed -- but without a
    /// guard the sampler would invent a card for our own hand, overwrite the
    /// real `starting_order` with it, and hand back a world that conserves and
    /// passes `check_invariants` yet silently does NOT observe back to the
    /// view it came from.
    #[test]
    fn a_hidden_slot_on_the_viewers_own_seat_is_refused() {
        let (state, rules) = keep_game(2, 7);
        let mut view = observe(&state, PlayerId(0)).unwrap();
        view.players[0].hand[0] = HandCard::HiddenStartingOrder;

        // The mutated view still RECONCILES (the unmasked card leaves the
        // located set exactly as the slot count rises), so it is this guard
        // under test and not the size guard behind it.
        assert_eq!(
            view.unseen_orders(&rules).len(),
            view.order_deck_len + view.orders_removed_len + view.hidden_slot_count(),
            "the offending view must still reconcile, or this test proves nothing"
        );

        // `.err()` because `BeliefSampler` is deliberately not `Debug` -- the
        // sampler holds a whole `PlayerView`, and a Debug impl on it would be
        // a public surface this task has no reason to add.
        match BeliefSampler::from_view(&view, &rules, 1).err() {
            Some(BeliefError::Inconsistent(m)) => {
                assert!(m.contains("seat 0"), "the refusal must name the seat; got: {m}")
            }
            other => panic!("expected Inconsistent, got {other:?}"),
        }
    }

    /// One deal, one hidden card: two masked slots in a single hand cannot be
    /// materialized coherently. `starting_order` can only hold the last one
    /// written, so the other hand slot would carry a card the seat does not
    /// own a record of -- and `observe` would then re-mask only one of the two
    /// positions.
    #[test]
    fn two_hidden_slots_in_one_hand_are_refused() {
        let (mut state, rules) = keep_game(2, 7);
        // Seat 1 needs a second card before two slots can even be expressed.
        let take = state.order_deck.pop().expect("deck is not empty");
        state.players[1].order_cards_in_hand.push(take);
        let mut view = observe(&state, PlayerId(0)).unwrap();
        assert_eq!(view.players[1].hand.len(), 2);
        view.players[1].hand[1] = HandCard::HiddenStartingOrder;

        assert_eq!(
            view.unseen_orders(&rules).len(),
            view.order_deck_len + view.orders_removed_len + view.hidden_slot_count(),
            "the offending view must still reconcile, or this test proves nothing"
        );

        match BeliefSampler::from_view(&view, &rules, 1).err() {
            Some(BeliefError::Inconsistent(m)) => {
                assert!(m.contains("seat 1"), "the refusal must name the seat; got: {m}")
            }
            other => panic!("expected Inconsistent, got {other:?}"),
        }
    }

    #[test]
    fn a_hand_that_cannot_hold_its_hidden_card_is_refused() {
        // Physically impossible in a faithfully tracked game, but a tracking
        // artifact must be a typed refusal, never an invariant-violating world.
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown])
                .unwrap();
        // Give the hidden seat a full tracked hand: 3 known + 1 synthesized = 4.
        let extra: Vec<_> = state.order_deck.drain(state.order_deck.len() - 3..).collect();
        state.players[1].order_cards_in_hand.extend(extra);

        let view = observe(&state, PlayerId(0)).unwrap();
        assert!(matches!(
            BeliefSampler::from_view(&view, &rules, 1),
            Err(BeliefError::HandOverflow(PlayerId(1)))
        ));
    }
}
