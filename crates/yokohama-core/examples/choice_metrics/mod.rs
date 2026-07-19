//! Task 16 (design 2026-07-18 §6.1-2/3): choice-effectiveness counters
//! shared by `examples/selfplay.rs` (aggregated over many random games) and
//! `examples/record_stats.rs` (one real recorded game).
//!
//! This file is NOT a Cargo example target itself: Cargo's example
//! auto-discovery only picks up `examples/*.rs` (flat files) and
//! `examples/*/main.rs` (subdirectory examples) -- a `mod.rs` inside a
//! subdirectory matches neither pattern. Both `selfplay.rs` and
//! `record_stats.rs` pull it in the exact same way `selfplay.rs` already
//! pulls in `tests/common/mod.rs` (`#[path = "..."] mod ...;`), so the
//! counting logic exists in exactly one place despite the two examples
//! being separate binary crate targets that cannot depend on each other.
//!
//! Five metric families (design §6.1-2, spelled out in the Task 16 brief):
//!  1. prompted counts/game per `PendingChoice` type (a move applied while
//!     `pending_choices` is non-empty counts as one prompt for the HEAD
//!     variant's type)
//!  2. mean options offered per prompt (`get_legal_moves(...).len()` at
//!     exactly those moves), per type
//!  3. decline rates for the two Decline-bearing types (WarehouseTake,
//!     AssistantMove): share of applied resolutions that were `Decline*`
//!  4. cascade-depth distribution: run lengths of consecutive applied
//!     choice-resolution moves
//!  5. Church/Customs inline-auto share: a Church/Customs area action NOT
//!     immediately followed by a pending space choice means the engine
//!     placed inline (singleton eligible space, per design: church/customs
//!     only ever queue with >=2 eligible spaces)

use std::collections::BTreeMap;

use yokohama_core::{get_legal_moves, AreaAction, GameState, Move, PendingChoice, Rules};

/// Stable per-variant label, ignoring payload -- used as a map key so e.g.
/// every `ChurchSpace { total_faith }` aggregates under one "ChurchSpace"
/// bucket regardless of the faith total.
fn pending_choice_label(choice: &PendingChoice) -> &'static str {
    match choice {
        PendingChoice::AnyGood => "AnyGood",
        PendingChoice::WarehouseTake => "WarehouseTake",
        PendingChoice::AssistantMove => "AssistantMove",
        PendingChoice::ChurchSpace { .. } => "ChurchSpace",
        PendingChoice::CustomsSpace { .. } => "CustomsSpace",
        PendingChoice::RecoveryLeave => "RecoveryLeave",
        PendingChoice::StationPlace => "StationPlace",
    }
}

/// Fixed print order (matches design §5.2's choice-inventory table), not
/// whatever order a `BTreeMap`'s own key iteration would happen to give.
const ALL_LABELS: [&str; 7] = [
    "AnyGood", "WarehouseTake", "AssistantMove", "ChurchSpace", "CustomsSpace", "RecoveryLeave",
    "StationPlace",
];

fn is_church_or_customs(mv: &Move) -> bool {
    matches!(
        mv,
        Move::ExecuteAreaAction { action: AreaAction::Church { .. } }
            | Move::ExecuteAreaAction { action: AreaAction::Customs { .. } }
    )
}

/// Accumulates the design §6.1-2/3 choice-effectiveness metrics across one
/// or more games. Feed it via `observe(state, mv, rules)` immediately
/// before every applied move -- `state` is what `mv` is about to be applied
/// to, the exact shape `tests/common/mod.rs::run_random_game_observed`'s
/// hook already provides -- then call `finish_game` once after each game's
/// play/replay loop ends.
#[derive(Default)]
pub struct ChoiceMetrics {
    games: u64,
    /// Prompted-move count per type, summed across every game fed in.
    prompted: BTreeMap<&'static str, u64>,
    /// Sum/count of `get_legal_moves(...).len()` at exactly those prompted
    /// moves, per type -- `options_sum[t] / options_n[t]` is the mean.
    options_sum: BTreeMap<&'static str, u64>,
    options_n: BTreeMap<&'static str, u64>,
    warehouse_resolutions: u64,
    warehouse_declines: u64,
    assistant_resolutions: u64,
    assistant_declines: u64,
    /// Cascade-depth histogram: run length (consecutive applied
    /// choice-resolution moves) -> occurrence count.
    cascade_histogram: BTreeMap<u64, u64>,
    current_run: u64,
    church_total: u64,
    church_inline: u64,
    customs_total: u64,
    customs_inline: u64,
    /// One-move-lag buffer: a just-applied Church/Customs move, still
    /// awaiting its inline-vs-queued classification. The observer hook only
    /// ever hands us a BEFORE-state, so a Church/Customs move's own result
    /// (did it queue a space choice?) is only visible as the `state`
    /// argument of the NEXT `observe` call -- this buffers exactly the one
    /// move needed to bridge that gap, without needing to change
    /// `tests/common/mod.rs`'s observer signature (out of scope: that file
    /// is a test file, not one of this task's two example paths).
    previous_church_or_customs: Option<Move>,
}

impl ChoiceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, state: &GameState, mv: &Move, rules: &Rules) {
        if let Some(prev_mv) = self.previous_church_or_customs.take() {
            self.classify_inline(&prev_mv, state);
        }

        if let Some(head) = state.pending_choices.first() {
            let label = pending_choice_label(head);
            *self.prompted.entry(label).or_insert(0) += 1;
            let n_options = get_legal_moves(state, rules).len() as u64;
            *self.options_sum.entry(label).or_insert(0) += n_options;
            *self.options_n.entry(label).or_insert(0) += 1;
            self.current_run += 1;

            match mv {
                Move::ChooseWarehouseTake { .. } => self.warehouse_resolutions += 1,
                Move::DeclineWarehouseTake => {
                    self.warehouse_resolutions += 1;
                    self.warehouse_declines += 1;
                }
                Move::ChooseAssistantMove { .. } => self.assistant_resolutions += 1,
                Move::DeclineAssistantMove => {
                    self.assistant_resolutions += 1;
                    self.assistant_declines += 1;
                }
                _ => {}
            }
        } else if self.current_run > 0 {
            *self.cascade_histogram.entry(self.current_run).or_insert(0) += 1;
            self.current_run = 0;
        }

        if is_church_or_customs(mv) {
            self.previous_church_or_customs = Some(mv.clone());
        }
    }

    fn classify_inline(&mut self, mv: &Move, after: &GameState) {
        match mv {
            Move::ExecuteAreaAction { action: AreaAction::Church { .. } } => {
                self.church_total += 1;
                if !matches!(after.pending_choices.first(), Some(PendingChoice::ChurchSpace { .. })) {
                    self.church_inline += 1;
                }
            }
            Move::ExecuteAreaAction { action: AreaAction::Customs { .. } } => {
                self.customs_total += 1;
                if !matches!(after.pending_choices.first(), Some(PendingChoice::CustomsSpace { .. })) {
                    self.customs_inline += 1;
                }
            }
            _ => {}
        }
    }

    fn count_buffered_as_inline(&mut self, mv: &Move) {
        match mv {
            Move::ExecuteAreaAction { action: AreaAction::Church { .. } } => {
                self.church_total += 1;
                self.church_inline += 1;
            }
            Move::ExecuteAreaAction { action: AreaAction::Customs { .. } } => {
                self.customs_total += 1;
                self.customs_inline += 1;
            }
            _ => {}
        }
    }

    /// Call once after a game's play/replay loop ends.
    ///
    /// `final_state`: pass the actual resulting `GameState` when the caller
    /// has one -- `record_stats`, replaying a fixed event log, always does,
    /// whether or not the replayed game actually finished -- for an exact
    /// trailing classification. Pass `None` only when the caller has JUST
    /// observed the game legitimately end and genuinely has no further
    /// `GameState` to hand over: `selfplay`'s driver
    /// (`run_random_game_observed`) returns only summary `GameStats`, not
    /// the final state. In that case a still-buffered Church/Customs move
    /// is counted as inline without needing the state at all, because the
    /// engine only ever reaches game-over (`TurnEnd`, per
    /// `game::end_game::is_game_over`) via `advance_phase`'s
    /// `AdditionalActionSecond -> TurnEnd` transition, which -- since
    /// `get_legal_moves` offers only choice-resolution moves while
    /// `pending_choices` is non-empty (design §5.1's queue-first gate) --
    /// is structurally unreachable with a choice still queued in any game
    /// driven purely through legal moves (both examples' drivers are). A
    /// queued space choice sitting unresolved is therefore impossible at
    /// this point; the buffered move must have placed inline.
    pub fn finish_game(&mut self, final_state: Option<&GameState>) {
        self.games += 1;
        if let Some(prev_mv) = self.previous_church_or_customs.take() {
            match final_state {
                Some(state) => self.classify_inline(&prev_mv, state),
                None => self.count_buffered_as_inline(&prev_mv),
            }
        }
        if self.current_run > 0 {
            *self.cascade_histogram.entry(self.current_run).or_insert(0) += 1;
            self.current_run = 0;
        }
    }

    pub fn print(&self) {
        let games = self.games.max(1);
        println!("choice-effectiveness metrics (design §6.1-2/3), {} game(s):", self.games);

        println!("  prompted/game (mean) and mean options offered per prompt:");
        for label in ALL_LABELS {
            let prompted = *self.prompted.get(label).unwrap_or(&0);
            let opt_sum = *self.options_sum.get(label).unwrap_or(&0);
            let opt_n = (*self.options_n.get(label).unwrap_or(&0)).max(1);
            println!(
                "    {label}: {:.2}/game, {:.2} options/prompt (n={prompted})",
                prompted as f64 / games as f64,
                opt_sum as f64 / opt_n as f64,
            );
        }

        println!("  decline rates:");
        println!(
            "    WarehouseTake: {}/{} declined ({:.1}%)",
            self.warehouse_declines,
            self.warehouse_resolutions,
            100.0 * self.warehouse_declines as f64 / self.warehouse_resolutions.max(1) as f64,
        );
        println!(
            "    AssistantMove: {}/{} declined ({:.1}%)",
            self.assistant_declines,
            self.assistant_resolutions,
            100.0 * self.assistant_declines as f64 / self.assistant_resolutions.max(1) as f64,
        );

        println!("  cascade-depth distribution (consecutive applied choice-resolution moves):");
        if self.cascade_histogram.is_empty() {
            println!("    (no cascades observed)");
        }
        for (depth, count) in &self.cascade_histogram {
            println!("    depth {depth}: {count}");
        }

        println!("  church/customs inline-auto share (singleton eligible space, never queued):");
        println!(
            "    Church: {}/{} inline ({:.1}%)",
            self.church_inline,
            self.church_total,
            100.0 * self.church_inline as f64 / self.church_total.max(1) as f64,
        );
        println!(
            "    Customs: {}/{} inline ({:.1}%)",
            self.customs_inline,
            self.customs_total,
            100.0 * self.customs_inline as f64 / self.customs_total.max(1) as f64,
        );
    }
}
