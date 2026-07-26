//! `WorldSeed` + `SeedSchedule`: full-width per-game seeds for the Phase-3.2
//! arena (spec §3), enforced at the type level rather than by prose.
//!
//! THE RULE LIVES IN THE TWO CONSTRUCTORS. `from_os_entropy` is for bot-facing
//! EVALUATION runs -- anything whose figures get quoted as a strength claim.
//! It draws its master from the OS and hands that master back so the caller
//! RECORDS it (run provenance + the report); re-running the recorded master
//! through `diagnostic` is the intended reproduction path. `diagnostic` is for
//! reproduction, fixtures, and every asserting gate cell, and its NAME is the
//! waiver (M2's naming rule): a fixed master needs no further justification at
//! a call site that says `diagnostic`, and cannot be mistaken for entropy at
//! one that does not. Gate cells are exempt BY DESIGN, not by oversight -- a
//! regression test wants reproducibility, and an OS-entropy null cell would
//! fail at its nominal rate by construction.
//!
//! Why entropy and not merely width: a compile-time master collapses the
//! seed-oracle search space to the schedule's INDEX range, so an adversary
//! brute-forces `i` instead of a u64. Width without entropy is
//! reproducibility, which is a different property with a different purpose.
//!
//! NO VALUE-BASED SEED CHECKS ANYWHERE (spec §3, quoted here so it can be
//! cited): "any u64 is a legitimate draw; the enforced property is the
//! DERIVATION, not the value. A reviewer adding `assert!(seed > 2^32)` is
//! adding false comfort -- this sentence exists to be cited against that."
//! The tests below pin the derivation (SplitMix64 vectors), the stream's
//! distinctness and bit width across many draws, and reproduction from a
//! recorded master. None of them judges a single seed's value. The rule of
//! thumb for telling an ENSEMBLE property (allowed) from a value check
//! (forbidden): if any one draw could be replaced by an arbitrary u64 and the
//! test could still pass, it is an ensemble property and the prohibition does
//! not reach it (T1 review ruling).
//!
//! SEED-ORACLE RESIDUAL (map §8 row 8): `new_game` builds `board_layout` from
//! the setup seed BEFORE the deck operations, so the public board fingerprints
//! the seed -- masking the FIELD does not close the CHANNEL. This module is
//! the OPERATIONAL closure (full-u64 recorded seeds); the residual itself is
//! PERMANENT while `GameRecord` replay requires seed determinism, and
//! `t8f_red_team_the_public_board_still_recovers_a_small_setup_seed`
//! (`tests/observation_leakage.rs`) is the executable reminder -- it stays
//! green and untouched.
//!
//! The four pre-existing `base_seed + i` sweep sites (`examples/selfplay.rs`
//! and difftest's three sweeps) are QUARANTINED by comment rather than
//! rerouted through this module: rerouting would silently change the seed sets
//! behind already-published gate figures. They are diagnostic sweeps, never
//! bot-facing evaluation.

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// One game's full-width seed. The arena's public surface takes this, never a
/// bare `u64`, so a caller cannot pass a hand-typed small seed by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSeed(pub u64);

/// A SplitMix64 stream of `WorldSeed`s over one master.
///
/// The schedule itself counts `next_world()` CALLS (see `index`). Paired
/// designs are a CALLER OBLIGATION on top of that: draw ONE `WorldSeed` per
/// pair/block and reuse it across every seating of that unit, so a call maps
/// to a UNIT, never a game — the arena's `UnitSpec` (one seed, a Vec of
/// seatings) is where that discipline is enforced structurally. A consumer
/// that drew a fresh world per SEATING would silently stop being a paired
/// design while its intervals still computed cleanly.
///
/// Deliberately NOT `Clone`: a cloned schedule would replay the same worlds
/// from the same index — a silent stream fork. If a future consumer needs a
/// spare, it should derive one via a distinct diagnostic master, or add
/// `Clone` here together with a doc line owning the fork semantics.
#[derive(Debug)]
pub struct SeedSchedule {
    master: u64,
    state: u64,
    index: u64,
}

/// SplitMix64's increment (the 64-bit golden-gamma constant).
const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// One SplitMix64 step: advances `state` and returns the mixed output.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl SeedSchedule {
    /// OS-entropy master for bot-facing evaluation runs; returns the schedule
    /// and the master it drew -- RECORD the master, it is the only way back to
    /// the run.
    ///
    /// Panics if the OS has no entropy to give. There is deliberately no
    /// fallback: a clock- or address-derived master would silently collapse
    /// the search space this constructor exists to open, which is exactly the
    /// false comfort the module doc forbids.
    pub fn from_os_entropy() -> (Self, u64) {
        let mut bytes = [0u8; 8];
        OsRng.try_fill_bytes(&mut bytes).expect(
            "the OS refused to supply entropy; an evaluation run cannot draw a master seed \
             (use SeedSchedule::diagnostic for a reproduction run)",
        );
        let master = u64::from_le_bytes(bytes);
        (Self::diagnostic(master), master)
    }

    /// Fixed master for reproduction, fixtures, and gate cells. The NAME is
    /// the waiver (spec §3) -- a call site reading `diagnostic` has declared
    /// itself.
    pub fn diagnostic(master: u64) -> Self {
        Self { master, state: master, index: 0 }
    }

    /// The master this schedule streams from (record it with the run).
    pub fn master(&self) -> u64 {
        self.master
    }

    /// Worlds drawn so far.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// Draw the next world seed (one SplitMix64 step over the master).
    pub fn next_world(&mut self) -> WorldSeed {
        let world = splitmix64(&mut self.state);
        self.index += 1;
        WorldSeed(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn next_world_is_splitmix64_of_the_master() {
        // The DERIVATION pin. These are Vigna's published splitmix64 vectors
        // for x = 0 (splitmix64.c), recomputed independently from the
        // algorithm before being pinned here. Master 1's first output pins
        // that the master seeds the state, not merely that some SplitMix64
        // stream runs.
        let mut zero = SeedSchedule::diagnostic(0);
        assert_eq!(zero.next_world(), WorldSeed(0xE220_A839_7B1D_CDAF));
        assert_eq!(zero.next_world(), WorldSeed(0x6E78_9E6A_A1B9_65F4));
        assert_eq!(zero.next_world(), WorldSeed(0x06C4_5D18_8009_454F));
        assert_eq!(zero.index(), 3, "index counts the worlds drawn");

        let mut one = SeedSchedule::diagnostic(1);
        assert_eq!(one.next_world(), WorldSeed(0x910A_2DEC_8902_5CC1));
        assert_eq!(one.master(), 1, "the master is retained for the record");
    }

    #[test]
    fn ten_thousand_draws_are_distinct_and_cover_the_high_bits() {
        // A WIDTH-and-distinctness pin on the DERIVATION, deliberately NOT a
        // judgement of any single seed (spec §3's prohibition): it asks
        // whether the stream ranges over the full u64, not whether a given
        // draw is "big enough".
        const DRAWS: usize = 10_000;
        let mut schedule = SeedSchedule::diagnostic(0x5EED_2026_0726_0001);

        let mut seen = HashSet::with_capacity(DRAWS);
        let mut any_set = 0u64;
        let mut any_clear = 0u64;
        for _ in 0..DRAWS {
            let world = schedule.next_world();
            seen.insert(world.0);
            any_set |= world.0;
            any_clear |= !world.0;
        }

        assert_eq!(seen.len(), DRAWS, "the stream repeated a world seed");
        for bit in 32..64 {
            let mask = 1u64 << bit;
            assert!(any_set & mask != 0, "high bit {bit} was never set across {DRAWS} draws");
            assert!(any_clear & mask != 0, "high bit {bit} was never clear across {DRAWS} draws");
        }
    }

    #[test]
    fn diagnostic_reproduces_a_recorded_run() {
        // The reproduction path the OS-entropy contract depends on: a recorded
        // master replays its run exactly.
        const MASTER: u64 = 0xA3E1_2026_0726_0001;
        let mut first = SeedSchedule::diagnostic(MASTER);
        let mut second = SeedSchedule::diagnostic(MASTER);

        let a: Vec<WorldSeed> = (0..100).map(|_| first.next_world()).collect();
        let b: Vec<WorldSeed> = (0..100).map(|_| second.next_world()).collect();

        assert_eq!(a, b, "the same master must reproduce the same worlds");
        assert_eq!(first.index(), 100);
        assert_eq!(second.index(), 100);
    }

    #[test]
    fn from_os_entropy_two_calls_differ() {
        // Two independent draws from the OS differ. This rules out a FIXED
        // master -- it cannot pin the source itself: a monotonic counter
        // would also pass (T1 review, mutation-proven), and no unit test can
        // verify OsRng is really underneath short of injection. It says
        // nothing about how large either master is. Collision probability is
        // 2^-64 for a working OS source; a repeat means the draw is not
        // reaching a varying source at all.
        let (first, first_master) = SeedSchedule::from_os_entropy();
        let (second, second_master) = SeedSchedule::from_os_entropy();

        assert_ne!(first_master, second_master, "two OS draws returned the same master");
        assert_eq!(first.master(), first_master, "the returned master must be the one in use");
        assert_eq!(second.master(), second_master);
    }
}
