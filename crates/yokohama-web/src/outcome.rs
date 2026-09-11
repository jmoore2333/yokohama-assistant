//! The final score breakdown: who won, by how much, and out of what.
//!
//! **A route of its own rather than a change to `ClientState.scores`.** That
//! field is `{seat, total}[]` — the compact in-frame view a finished game's
//! decision zone shows — and it is already frozen in `game_over.json`, typed in
//! `types.ts`, consumed by `Decision.tsx` and asserted by three tests. Widening
//! it would re-key every committed fixture for no gain, so `scores` stays what
//! it is and this is the breakdown beside it.
//!
//! `GameResult`, `Standing` and `ScoreBreakdown` all live in `yokohama-core`
//! and none derives `Serialize`; the lane's rule is that core stays
//! byte-identical to master, so they are mirrored here.

use serde::Serialize;
use yokohama_core::{GameResult, ScoreBreakdown};
use yokohama_core::PlayerId;

/// One seat's score, category by category.
///
/// **The categories are the engine's own field names, not a re-derivation.**
/// Several of them carry decision-log rulings behind them — the church 6/3 and
/// customs 8/4 majorities, the technology majority the 2P dummy competes in at
/// industry value 7.5 — and a browser that re-added a total from parts it chose
/// itself would be a second scoring implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Breakdown {
    /// Victory points at game end, achievements included.
    pub base_vp: u32,
    pub church: u32,
    pub customs: u32,
    pub technology: u32,
    pub country: u32,
    /// Mercenaries, imports, money and goods conversions.
    pub remainder: u32,
    pub total: u32,
}

impl Breakdown {
    fn of(b: &ScoreBreakdown) -> Self {
        Self {
            base_vp: b.base_vp,
            church: b.church,
            customs: b.customs,
            technology: b.technology,
            country: b.country,
            remainder: b.remainder,
            total: b.total,
        }
    }
}

/// One seat's placement.
///
/// `margin` is against the best OTHER seat by rank, which is the same sense the
/// harness and the driven runner use — and **a margin of 0 is not a win**: two
/// seats can tie on total and be separated by the engine's own tie-break, so the
/// rank is the answer and the margin is the distance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Placing {
    pub seat: PlayerId,
    pub rank: u8,
    pub breakdown: Breakdown,
    pub margin: i64,
    pub won: bool,
}

/// `GET /api/games/{id}/result` — the finished game, ranked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    /// Rank order, so `standings[0]` is first place.
    pub standings: Vec<Placing>,
    pub winner: PlayerId,
}

impl Outcome {
    pub fn of(r: &GameResult) -> Self {
        let standings = r
            .standings
            .iter()
            .map(|s| {
                // The best OTHER seat's total — by rank, so a seat that ties on
                // points but loses the tie-break still reads a margin of 0 and
                // `won: false`.
                let best_other = r
                    .standings
                    .iter()
                    .filter(|o| o.player != s.player)
                    .map(|o| i64::from(o.breakdown.total))
                    .max()
                    .unwrap_or(0);
                Placing {
                    seat: s.player,
                    rank: s.rank,
                    breakdown: Breakdown::of(&s.breakdown),
                    margin: i64::from(s.breakdown.total) - best_other,
                    won: s.player == r.winner,
                }
            })
            .collect();
        Self { standings, winner: r.winner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_core::Standing;

    fn standing(seat: u8, total: u32, rank: u8) -> Standing {
        Standing {
            player: PlayerId::new(seat),
            rank,
            breakdown: ScoreBreakdown {
                base_vp: total,
                church: 0,
                customs: 0,
                technology: 0,
                country: 0,
                remainder: 0,
                total,
            },
        }
    }

    /// The margin is against the best OTHER seat, so the winner's is positive
    /// and every loser's is negative or zero — never a seat's distance to
    /// itself.
    #[test]
    fn a_margin_is_measured_against_the_best_other_seat() {
        let r = GameResult {
            standings: vec![standing(0, 82, 1), standing(1, 81, 2), standing(2, 40, 3)],
            winner: PlayerId::new(0),
        };
        let o = Outcome::of(&r);
        assert_eq!(o.standings[0].margin, 1, "82 against the 81 behind it");
        assert_eq!(o.standings[1].margin, -1, "81 against the 82 ahead of it");
        assert_eq!(o.standings[2].margin, -42, "and the tail against the leader");
        assert!(o.standings[0].won);
        assert!(!o.standings[1].won && !o.standings[2].won);
    }

    /// **A margin of 0 is not a win.** Two seats can tie on total and be
    /// separated by the engine's own tie-break; the RANK is the answer, and the
    /// payload says so with `won` rather than leaving a client to infer it from
    /// a zero.
    #[test]
    fn a_tie_on_points_is_ranked_and_only_one_seat_won() {
        let r = GameResult {
            standings: vec![standing(1, 70, 1), standing(0, 70, 2)],
            winner: PlayerId::new(1),
        };
        let o = Outcome::of(&r);
        assert_eq!(o.standings[0].margin, 0);
        assert_eq!(o.standings[1].margin, 0, "both are level on points");
        assert!(o.standings[0].won, "and the engine's tie-break named seat 1");
        assert!(!o.standings[1].won, "a margin of 0 is not a win");
    }

    /// Every category the engine scores reaches the wire. A total with no parts
    /// beside it is a number a player cannot check.
    #[test]
    fn every_scoring_category_is_carried() {
        let b = ScoreBreakdown {
            base_vp: 40,
            church: 6,
            customs: 8,
            technology: 10,
            country: 5,
            remainder: 3,
            total: 72,
        };
        let d = Breakdown::of(&b);
        assert_eq!(
            d.base_vp + d.church + d.customs + d.technology + d.country + d.remainder,
            d.total,
            "the parts add to the total the engine reported"
        );
    }
}
