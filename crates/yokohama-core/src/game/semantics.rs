//! Executable-semantics revision stamp + game-state digest (guidingpath
//! Phase-3 MANDATORY handover item (a), 2026-07-20; anchored at the Phase 2.9
//! differential-oracle close).
//!
//! WHY THIS EXISTS. A `GameRecord` already carries two compatibility stamps
//! (`engine_version`, `rules_hash`), but NEITHER identifies the engine's
//! executable semantics: `engine_version` is the workspace `0.1.0` that has
//! never been bumped, and `rules_hash` fingerprints only the static
//! card/site/token/board DATA -- not the code in `get_legal_moves` /
//! `apply_move` / scoring / setup that turns that data into behaviour. So a
//! change that, say, re-rules a scoring edge case moves neither stamp, and
//! `replay_strict` would silently REINTERPRET an old record against the new
//! rules and call it a clean replay. That is exactly the failure mode item (a)
//! flags as blocking before ANY self-play/training corpus is retained across
//! engine revisions.
//!
//! `SEMANTICS_REV` closes that gap: a hand-maintained revision of the engine's
//! executable rules, stamped into every record.
//!
//! ## Bump contract
//!
//! Bump [`SEMANTICS_REV`] in the SAME commit as ANY behaviour-affecting change
//! to:
//!   * legality enumeration (`get_legal_moves` and everything it calls),
//!   * move application (`apply_move` / the apply arms / their scoring effects),
//!   * end-game scoring or the winner / tie-break (`final_result`),
//!   * setup semantics (`new_game` / `finalize_setup` / setup reconciliation).
//!
//! Do NOT bump it for:
//!   * card / site / token / board DATA edits -- those already move `rules_hash`,
//!   * presentation / CLI / doc changes -- those move neither stamp.
//!
//! The litmus test: a change that alters what state some
//! `(seed, config, choices, events)` record replays to MUST bump this
//! constant; a change that cannot is a data or presentation change and MUST
//! NOT. (Ideally a CI check enforces the bump; until then it is convention,
//! same as the TDD discipline the repo already runs on.)
//!
//! ## Rev 1 anchor
//!
//! Rev 1 is dated to the Phase 2.9 close (2026-07-20): the differential-oracle
//! campaign converged -- the engine and an independently authored rules oracle
//! agree on legality, scoring, and route labels everywhere tested -- making
//! today the defensible birthday for "the executable semantics of record".
//! Every retained cross-revision corpus is dated relative to this.

use crate::types::GameState;

/// Revision of the engine's executable semantics (see the module docs for the
/// bump contract). Stamped into every `GameRecord` at creation; `replay`
/// warns and `replay_strict` rejects when a record's stamp differs from the
/// running engine's.
///
/// Rev 2 (2026-07-21, Task 1 — board topology): the board became a staggered
/// pyramid (was a rectangular lattice) and Canal crossings became any-to-any.
/// That changes movement legality and president routes, so any rev-1 record
/// replays to a different state under rev 2 — old records become
/// strict-rejected, correctly.
///
/// Rev 3 (2026-07-21, PR #5 §1.1 — 5-Power token observation): 5-Power tokens
/// are FACE-UP at setup (rulebook p.3 step 5), so `reconcile_setup` now derives
/// their faces from a new `ObservedBoard.tokens` observation instead of relaying
/// the seeded assignment. A live record carrying token observations replays to a
/// board with the observed faces — a setup-reconciliation change per the bump
/// contract. Self-play / sim is unaffected (no token observations ⇒ the seeded
/// relay is retained); older records are strict-rejected as usual.
///
/// Rev 4 (2026-07-25, Canal legality alignment): the two shared movement
/// predicates — `is_possible_action_area_full` and `can_step_onto` — now carry
/// the rule that the Canal is never placeable and never stoppable (rulebook
/// p.14; guidingpath 2026-07-21 Canal re-ruling), so the LEGALITY SET changed:
/// `get_legal_moves` no longer emits a President placement onto, or a walk
/// hopping through / ending on, a Canal-typed area that holds one of the
/// mover's own assistants. Previously the generator emitted those while every
/// apply path rejected them.
///
/// NO REACHABLE-STATE BEHAVIOUR CHANGES, and this is why: both new gates fire
/// only when an own assistant sits on a Canal-typed area, and no reachable
/// state has one. The engine has exactly three writers that ADD to
/// `assistants.on_board` — `apply_placement` and `apply_placement_same_area`
/// (both behind `require_placeable_area`) and `ChooseAssistantMove`'s
/// resolution arm (fed by `assistant_move_options`, which excludes the Canal,
/// and as of this revision guarded by name as well). Setup seats no player
/// assistant on the board. So the affected states are FIXTURE-ONLY, every
/// existing record replays identically, and the bump is the contract letter for
/// a legality-SET change rather than a report of changed play. Its one
/// user-visible consequence is the usual one: rev-3 saves stop resuming.
pub const SEMANTICS_REV: u32 = 4;

/// FNV-1a 64 over `bytes`, seeded with the standard offset basis. Hand-rolled
/// for the same reason `Rules::content_hash` hand-rolls it: `std::hash` is not
/// stable across releases and this digest is written into saved records.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Stable content digest of a `GameState`: FNV-1a 64 (16 hex chars) over the
/// state's canonical `serde_json` serialization. `GameState`'s maps are all
/// `BTreeMap`s (no `HashMap`s), so serde emits their entries in key order and
/// the string is deterministic across runs and machines for a given state --
/// and two `PartialEq`-equal states serialize identically, so their digests
/// match (that is what makes `replay_strict`'s terminal check sound: the
/// replayed state equals the state that produced the record).
///
/// Used for the `GameRecord` terminal-state digest -- defense in depth on top
/// of `replay_strict`'s per-event invariant checks: the record stores the
/// digest of the state its event log folds to, and strict replay recomputes it
/// from the replayed terminal state and rejects a mismatch (catching an
/// events-vs-digest divergence even when every intermediate state is valid).
pub fn state_digest(state: &GameState) -> String {
    // Compact `to_string`, not `to_string_pretty`: whitespace is irrelevant to
    // identity and compact is cheaper. Serializing a `GameState` cannot fail
    // in practice (all-BTreeMap, integer/unit-enum keys; the fuzz suite's
    // failure dump serializes full states the same way) -- an unserializable
    // state could not have produced or been saved as a record to begin with.
    let canon = serde_json::to_string(state).expect("GameState serializes to JSON");
    format!("{:016x}", fnv1a_64(canon.as_bytes()))
}

/// Stable content digest of a `PlayerView` -- the OBSERVATION identity the
/// leakage suites compare on. Same FNV-1a-64-over-canonical-JSON recipe as
/// `state_digest` and sound for the same reason: a `PlayerView`'s maps are
/// all `BTreeMap`s (it copies only `BTreeMap`-bearing state types), so serde
/// emits them in key order.
///
/// `state_digest` cannot serve this purpose: it hashes the hidden members, so
/// two worlds a seat cannot tell apart get different state digests.
pub fn view_digest(view: &crate::types::PlayerView) -> String {
    let canon = serde_json::to_string(view).expect("PlayerView serializes to JSON");
    format!("{:016x}", fnv1a_64(canon.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::Rules;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::GameConfig;

    fn a_state(seed: u64) -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, seed).unwrap();
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap()
    }

    #[test]
    fn state_digest_is_16_hex_and_deterministic() {
        let state = a_state(5);
        let a = state_digest(&state);
        let b = state_digest(&state);
        assert_eq!(a, b, "same state -> same digest");
        assert_eq!(a.len(), 16, "fnv1a-64 hex");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn state_digest_changes_when_state_changes() {
        let rules = Rules::standard();
        let state = a_state(5);
        let before = state_digest(&state);
        let moves = crate::game::legal_moves::get_legal_moves(&state, &rules);
        let next = crate::game::apply_move::apply_move(&state, &moves[0], &rules).unwrap();
        assert_ne!(before, state_digest(&next), "a real move must change the digest");
    }

    #[test]
    fn view_digest_is_16_hex_and_equal_for_equal_views() {
        use crate::game::observe::observe;
        use crate::types::PlayerId;
        let state = a_state(5);
        let v1 = observe(&state, PlayerId(0)).unwrap();
        let v2 = observe(&state.clone(), PlayerId(0)).unwrap();
        let d = view_digest(&v1);
        assert_eq!(d.len(), 16);
        assert!(d.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(d, view_digest(&v2));
    }

    #[test]
    fn view_digest_ignores_hidden_state_that_state_digest_does_not() {
        use crate::game::observe::observe;
        use crate::types::PlayerId;
        let state = a_state(5);
        let mut scrambled = state.clone();
        scrambled.order_deck.reverse();
        assert_ne!(
            state_digest(&state),
            state_digest(&scrambled),
            "the FULL-state digest sees the deck order"
        );
        assert_eq!(
            view_digest(&observe(&state, PlayerId(0)).unwrap()),
            view_digest(&observe(&scrambled, PlayerId(0)).unwrap()),
            "a seat cannot see the deck order, so its view digest must not move"
        );
    }
}
