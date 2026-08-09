//! The candidate-vs-incumbent WEIGHT SCREEN — Phase 3.5c, the evaluator-v2
//! protocol's §4.2 **L4** instrument
//! (`docs/plans/2026-08-09-phase3.5c-evaluator-v2-protocol.md`).
//!
//! One cell: `<rung>@candidate-weights` against `<rung>@incumbent-weights`, at
//! 2P, over swapped-pair design units, through the honest driver, tabulated with
//! the arena's own bootstrap. Two rungs are runnable and a round runs BOTH —
//! Greedy-v1 (the preserved one-ply rung) and Lookahead-v1 (the SHIPPED S1
//! rung), per §4.2's ladder.
//!
//! # WHY THIS LIVES OUTSIDE THE RUNNER'S CONTESTANT GRAMMAR (L12)
//!
//! [`Contestant`](crate::game::arena::Contestant) deliberately carries NO weight
//! knob, and its own doc states the rule: *"they run the frozen
//! [`WeightTable::V1`] the shipped agents run, and a weight change is a version
//! bump (spec §0.5 / L12), not a cell parameter."* Adding `weights=` to the
//! `<x>`/`<y>` spec grammar would contradict that rule at the exact moment v2 is
//! being iterated toward — it would let an unversioned table be published under a
//! versioned controller name, which is the confusion the composite comparability
//! key (T16) exists to prevent.
//!
//! But an ITERATION loop must be able to put two tables on one board before
//! either is versioned. So the comparison lives HERE, over the public weighted
//! constructors [`GreedyAgent::with_weights`] and [`LookaheadAgent::with_knobs`],
//! called from tests and round scripts — never from argv, never from the CLI,
//! never from a gate. The precedents are `greedy.rs`'s own re-pricing test and
//! `tests/evaluator_gate.rs`'s helper layer: in-crate cells assembled from the
//! public agent surface, outside the grammar. **How versioned `@V1`/`@V2`
//! identities are exposed to the runner AFTER the freeze is the five-rung
//! pre-registration's question, not this module's.**
//!
//! # WHAT THE SCREEN IS FOR — A REGRESSION NET, NOT THE POSITIVE EVIDENCE
//!
//! Protocol §4.2 is explicit and this module inherits it verbatim: the positive
//! case for a re-pricing is L1–L3's job (a regime-correct signal in the R0
//! marginal table, a constrained held-out fit, scenario counterfactuals). **A
//! near-null screen does not block a corpus-justified re-pricing.** What the
//! screen buys is the one ON-POLICY check inside a round — L1/L2 are measured on
//! INCUMBENT-play corpora and are therefore off-policy for the candidate — and a
//! tripwire against a change that plays visibly worse than it fits.
//!
//! The pre-committed adoption rule it feeds (§4.2 step 4) reads on the LOOKAHEAD
//! rung's screen: adopt iff the interval is **not entirely below the null**, all
//! scenarios are green, and the greedy screen is reported (a greedy interval
//! entirely below 0.40 pauses the round for recorded reconsideration — a
//! hand-set trigger with no measured basis, and a pause rather than a block).
//! [`ScreenReport::entirely_below_null`] is that clause made callable;
//! [`ScreenReport::direction_above`] is the stricter §7 criterion-1 shape.
//!
//! # THE REFUSAL CLASSIFICATION
//!
//! A degenerate unit vector produces
//! [`IntervalOutcome::RefusedZeroWidth`] rather than a printed ±0.000, and §4.2
//! fixes how it classifies against the null, per the ADR's standing definition:
//! **all-won is ABOVE, all-lost is BELOW.** A constant INTERIOR vector (every 2P
//! pair split 1–1) is neither and classifies as [`NullPosition::Spans`].
//!
//! Two things are kept apart on purpose, because conflating them is how a small-N
//! screen would over-claim:
//!
//! * [`ScreenReport::position`] is DIRECTIONAL. An all-won refusal is `Above`
//!   however few units were played.
//! * [`ScreenReport::direction_above`] additionally requires the refusal's
//!   rule-of-three ONE-SIDED BOUND to clear the null — which is what §7's freeze
//!   criterion 1 states (`1 − 3/200 = 0.985 > 0.5`), and which an all-won vector
//!   at n = 3 (bound 0.0) does NOT satisfy.
//!
//! # SCREENS ON ROTATING MASTERS ARE MUTUALLY INCOMPARABLE FIELDS
//!
//! Protocol §4.3: each round's screen draws a FRESH OS-entropy master at round
//! open, recorded BEFORE the screen runs and **never redrawn after seeing the
//! result** (master-shopping is the failure mode that pre-commitment blocks). The
//! price of rotation is stated rather than hidden: **round-to-round screens do
//! not compose** — they are different fields, and "the direction accumulated over
//! four rounds" is not a quantity any of them measured. Accumulation is cashed
//! out exactly once, by §7's single fresh cell at the freeze. Diagnostic masters
//! remain for reproducing one specific screen, and [`SeedScheduleKind`]'s own
//! naming rule carries the waiver.
//!
//! # WHAT IS HELD FIXED, AND WHY
//!
//! * **The RUNG's knobs.** [`ScreenRung`] has no knob fields: each variant runs
//!   its controller's SHIPPED knobs (Greedy `m=8`; Lookahead `b=4, k=1, m=8`,
//!   **UNCAPPED** — the shipped rung, not the 3.4 capped arm). The screen's
//!   subject is the WEIGHTS; a knob that moved in the same cell would confound
//!   the only thing it measures, and knob tuning is W6's business, after the
//!   freeze, against V2.
//! * **The KEEP function.** Both sides run `greedy::score_starting_order`, which
//!   both evaluator agents share and which takes NO weight table — so a screen is
//!   **Keep-HOMOGENEOUS and setup-IDENTICAL by construction**, at any pair of
//!   tables. Its two seatings start from a bit-identical post-`finalize_setup`
//!   state and differ only in which table sits where. `setup-identical=yes` in
//!   the echo is therefore structural here rather than a per-cell observation
//!   (contrast `arena::stats`, where a mixed-Keep cell prints `no`).
//! * **The STATISTICS.** Nothing is re-implemented: design units come from
//!   [`unit_for`], play streams from [`agent_stream_seed`], the master from
//!   [`SeedSchedule`], and the interval from [`interval_for_units`] at
//!   [`BOOTSTRAP_REPLICATES`] — the same percentile bootstrap over the same unit
//!   shares, with the same typed refusal. Only the SEAT-FILLING differs, which is
//!   the whole point.
//!
//! # DETERMINISM
//!
//! [`run_screen`] is single-threaded and every stream descends from the one
//! master, so on a diagnostic master the whole [`ScreenReport::render`] is
//! byte-identical across runs. The resample seed is derived from the DESIGN
//! (`master`, `units`, `players`) and never from the outcomes — `arena::stats`'s
//! rule: a resample stream that read the unit vector would make the interval a
//! function of the data twice over.
//!
//! # NOT A GATE
//!
//! Nothing here is `#[ignore]`d into the release roster and nothing here asserts
//! a direction. A roster cell asserting that some candidate beats V1 would freeze
//! an adjudication the loop has not made (`arena_gate`'s discipline, and the
//! reason `architecture_gate` asserts no direction either). The per-edit tests at
//! the foot of this file check the INSTRUMENT: identities, refusal handling,
//! determinism, provenance.

use std::collections::BTreeMap;

use crate::game::arena::stats::{mean as stats_mean, sample_sd as stats_sample_sd};
use crate::game::arena::{
    agent_stream_seed, interval_for_units, mix, require_supported_player_count, run_arena_game,
    unit_for, ArenaAgent, ArenaError, ControllerProvenance, ExplainHandle, IntervalOutcome,
    SeedScheduleKind, Slot, BOOTSTRAP_REPLICATES,
};
use crate::game::eval::{Feature, WeightTable, EVAL_IMPL_VERSION};
use crate::game::greedy::{self, weight_table_identity, GreedyAgent, DEFAULT_RETREAT_SAMPLES};
use crate::game::lookahead::{
    self, LookaheadAgent, SearchScheme, DEFAULT_BEAM, DEFAULT_WORLDS,
};
use crate::game::rules::Rules;
use crate::game::seeds::SeedSchedule;
use crate::game::semantics::SEMANTICS_REV;
use crate::types::PlayerId;

/// Domain salt for THIS module's bootstrap resample stream — the arena salt
/// family's next free serial (`0001` play, `0002` setup, `0003` raw, `0004`
/// `arena::stats`'s bootstrap, `0005` here).
///
/// A separate domain from `stats`'s deliberately: two instruments resampling the
/// same-shaped design must not silently share a stream just because they were
/// handed the same master.
const SALT_SCREEN_BOOTSTRAP: u64 = 0xA3E1_5C2E_0000_0005;

/// The CANDIDATE's slot, stable across every seating of a unit —
/// `arena::stats::X_SLOT`'s role, under this module's own name because the
/// contestant under test here is a TABLE, not a controller.
const CANDIDATE_SLOT: Slot = Slot(0);

// ---------------------------------------------------------------------------
// The rung
// ---------------------------------------------------------------------------

/// Which evaluator rung a screen runs at — **at that controller's SHIPPED
/// knobs**, always.
///
/// Knob-free by design: see the module doc's "what is held fixed". A round runs
/// both variants and reports both (§4.2's ladder rung L4), because greedy is a
/// preserved rung whose regression the owner is entitled to see even though
/// lookahead is the agent that ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenRung {
    /// Greedy-v1: one-ply argmax of the evaluator, `m` =
    /// [`DEFAULT_RETREAT_SAMPLES`].
    Greedy,
    /// Lookahead-v1: the S1 turn-boundary rung at `b` = [`DEFAULT_BEAM`], `k` =
    /// [`DEFAULT_WORLDS`], `m` = [`DEFAULT_RETREAT_SAMPLES`], **UNCAPPED**.
    ///
    /// Uncapped because that is what SHIPS, and because an absent cap is
    /// byte-identical to the pre-3.4 rung out to the descriptor — a capped screen
    /// would be measuring a hybrid arm (see [`LookaheadAgent::with_cap`]'s
    /// measured 32–128 non-terminating band) rather than the agent v2 is for.
    Lookahead,
}

impl ScreenRung {
    /// The controller's stable name — read from its own const, never typed here.
    pub fn controller(self) -> &'static str {
        match self {
            Self::Greedy => greedy::GREEDY_CONTROLLER,
            Self::Lookahead => lookahead::LOOKAHEAD_CONTROLLER,
        }
    }

    /// What this rung calls itself under `weights`, routed to the controller's
    /// OWN provenance function — so the version const, the knob names and the
    /// weight identity are read at exactly one site each (spec §5.2 / §0.5), and
    /// the screen cannot echo knobs it did not run.
    ///
    /// `agent_seed` is `None`: a screen's provenance describes the CELL, and a
    /// per-seat stream is a per-game fact the arena's own record already stamps.
    pub fn controller_provenance(self, weights: &WeightTable) -> ControllerProvenance {
        match self {
            Self::Greedy => {
                greedy::controller_provenance(weights, DEFAULT_RETREAT_SAMPLES, None)
            }
            Self::Lookahead => lookahead::controller_provenance(
                SearchScheme::TurnBoundary { beam: DEFAULT_BEAM },
                DEFAULT_WORLDS,
                DEFAULT_RETREAT_SAMPLES,
                weights,
                None,
            ),
        }
    }

    /// Build this rung's agent on `weights` and the play stream `agent_seed`,
    /// plus its explanation read-out.
    ///
    /// **`agent_seed` must come from [`agent_stream_seed`]** — the CRN rule is
    /// inherited unchanged, and the slot component is what keeps the two games of
    /// a pair from collapsing into one game (E2's measured trap). The handle is
    /// cloned out HERE, before the agent is boxed, because the driver's factory
    /// never hands an agent back.
    fn build(
        self,
        rules: &Rules,
        weights: WeightTable,
        agent_seed: u64,
    ) -> (Box<dyn ArenaAgent>, ExplainHandle) {
        match self {
            Self::Greedy => {
                let agent =
                    GreedyAgent::with_weights(rules, weights, DEFAULT_RETREAT_SAMPLES, agent_seed);
                let handle = ExplainHandle::Greedy(agent.tally());
                (Box::new(agent), handle)
            }
            Self::Lookahead => {
                // No `.with_cap(..)`: the shipped rung is UNCAPPED, and an absent
                // cap is byte-identical to the pre-3.4 agent out to the echo.
                let agent = LookaheadAgent::with_knobs(
                    rules,
                    SearchScheme::TurnBoundary { beam: DEFAULT_BEAM },
                    DEFAULT_WORLDS,
                    DEFAULT_RETREAT_SAMPLES,
                    weights,
                    agent_seed,
                );
                let handle = ExplainHandle::Search(agent.tally());
                (Box::new(agent), handle)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The spec
// ---------------------------------------------------------------------------

/// One screen: a candidate weight table against an incumbent one, at a rung.
///
/// `units` is the DESIGN unit count, not a game count — the cell plays
/// `units × players` games, each unit under every rotation so each table
/// occupies each seat exactly once. Size it BEFORE the run (protocol §4.2 fixes
/// N = 200 units for a round's screen; `pairs_needed_for_mme` is the sizing
/// function, and feeding it a pilot point estimate is the measured hazard
/// `arena::stats`'s doc names).
#[derive(Debug, Clone)]
pub struct ScreenSpec {
    /// Names the screen for the echo — a round's ledger row quotes it.
    pub name: String,
    pub rung: ScreenRung,
    /// The table under test. Sits in [`CANDIDATE_SLOT`].
    pub candidate: WeightTable,
    /// The table it must not lose to — [`WeightTable::V1`] until a freeze moves
    /// it.
    pub incumbent: WeightTable,
    /// 2 for every screen the protocol registers. 3–4P runs, but nothing has
    /// measured what a multi-table field means there, and E2's 3P seat-symmetry
    /// caveat applies as it does everywhere else.
    pub players: u8,
    pub units: u32,
    pub master: SeedScheduleKind,
}

impl ScreenSpec {
    /// The provenance this spec WILL carry — **`None` for an OS-entropy spec**,
    /// whose master does not exist until the run draws it.
    ///
    /// That `None` is the honest shape rather than an inconvenience: provenance
    /// names the master that RAN, and a placeholder in the one field a rotating
    /// screen is identified by is exactly the thing §4.3's no-redraw rule is
    /// protecting. A diagnostic spec knows its master before any game, so it
    /// answers.
    pub fn provenance(&self) -> Option<ScreenProvenance> {
        match self.master {
            SeedScheduleKind::Diagnostic(master) => Some(self.provenance_at(master)),
            SeedScheduleKind::OsEntropy => None,
        }
    }

    /// The ONE provenance builder — [`provenance`](Self::provenance) and
    /// [`run_screen_with`] both route here so the two cannot drift.
    fn provenance_at(&self, master: u64) -> ScreenProvenance {
        let cand = self.rung.controller_provenance(&self.candidate);
        let inc = self.rung.controller_provenance(&self.incumbent);
        ScreenProvenance {
            screen: self.name.clone(),
            rung: self.rung,
            controller: cand.name.clone(),
            controller_version: cand.version,
            candidate_identity: weight_table_identity(&self.candidate),
            incumbent_identity: weight_table_identity(&self.incumbent),
            candidate_params: cand.params,
            incumbent_params: inc.params,
            semantics_rev: SEMANTICS_REV,
            eval_impl_version: EVAL_IMPL_VERSION,
            players: self.players,
            units: self.units,
            games: u32::from(self.players) * self.units,
            master,
            master_kind: match self.master {
                SeedScheduleKind::Diagnostic(_) => "diagnostic",
                SeedScheduleKind::OsEntropy => "os-entropy (drawn)",
            },
            resample_seed: resample_seed_for(master, self.units, self.players),
            // STRUCTURAL, not observed: both rungs resolve the Keep with
            // `greedy::score_starting_order`, which takes no weight table.
            setup_identical: true,
        }
    }
}

/// The resample stream's seed — derived from the DESIGN, never from the
/// outcomes (`arena::stats`'s rule, and its component list).
fn resample_seed_for(master: u64, units: u32, players: u8) -> u64 {
    mix(SALT_SCREEN_BOOTSTRAP, &[master, u64::from(units), u64::from(players)])
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// **The T16 composite key made unavoidable.** No single constant identifies a
/// comparable measurement, so every screen carries all of them: the rules
/// revision, the evaluator implementation version, the CONTROLLER's version, and
/// **both weight-table identity digests** — plus the design (N, players, master,
/// resample stream) that fixes the field.
///
/// A figure quoted without this block cannot be pooled with any other figure,
/// which is exactly the mistake §2's "every rev-5 figure is a stale comparator"
/// warning is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenProvenance {
    pub screen: String,
    pub rung: ScreenRung,
    /// The controller both sides run — a screen never mixes rungs.
    pub controller: String,
    pub controller_version: u32,
    /// The candidate side's full knob map, from its own descriptor (carries its
    /// `weights=` digest and `eval_impl=`).
    pub candidate_params: BTreeMap<String, i64>,
    /// The incumbent side's, same source.
    pub incumbent_params: BTreeMap<String, i64>,
    pub candidate_identity: i64,
    pub incumbent_identity: i64,
    pub semantics_rev: u32,
    pub eval_impl_version: u32,
    pub players: u8,
    pub units: u32,
    pub games: u32,
    /// The master this screen ran on: the committed one for a diagnostic spec,
    /// the DRAWN one for an OS-entropy spec.
    pub master: u64,
    pub master_kind: &'static str,
    pub resample_seed: u64,
    /// Always `true` here, and structurally so — see the module doc.
    pub setup_identical: bool,
}

impl ScreenProvenance {
    /// One line, `key=value` throughout, in `arena::stats`'s `spec_echo`
    /// spelling — so a figure scraped out of a round's log carries its
    /// measurement policy with it.
    pub fn render(&self) -> String {
        let unit_shape = if self.players == 2 {
            "pair (2 seatings)".to_string()
        } else {
            format!("rotation block ({} seatings)", self.players)
        };
        format!(
            "screen={} rung={} players={} candidate={} incumbent={} units={} unit={} games={} \
             master={}:{:#018x} bootstrap_B={BOOTSTRAP_REPLICATES} resample_seed={:#018x} \
             single-threaded=yes fixed-N=yes setup-identical={} semantics_rev={} \
             eval_impl={} controller_version={}",
            self.screen,
            self.rung.controller(),
            self.players,
            side_echo(&self.controller, self.controller_version, &self.candidate_params),
            side_echo(&self.controller, self.controller_version, &self.incumbent_params),
            self.units,
            unit_shape,
            self.games,
            self.master_kind,
            self.master,
            self.resample_seed,
            if self.setup_identical { "yes" } else { "no" },
            self.semantics_rev,
            self.eval_impl_version,
            self.controller_version,
        )
    }
}

/// `name@vN{k=v,…}` — `arena::stats::contestant_echo`'s spelling, fed by the
/// controller's own descriptor so no knob name is hand-typed.
fn side_echo(name: &str, version: u32, params: &BTreeMap<String, i64>) -> String {
    if params.is_empty() {
        return format!("{name}@v{version}");
    }
    let knobs: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{name}@v{version}{{{}}}", knobs.join(","))
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// Where the screen's interval sits relative to the cell's null (`1/players`).
///
/// **DIRECTIONAL, and deliberately not a strength statement.** A refusal
/// classifies per §4.2's rule — all-won `Above`, all-lost `Below`, constant
/// interior `Spans` — regardless of how few units were played. The quantitative
/// question ("does the bound clear the null?") is
/// [`ScreenReport::direction_above`]'s, and the two answers differ at small N by
/// design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullPosition {
    /// A bootstrap interval entirely above the null, or the all-won refusal.
    Above,
    /// A bootstrap interval entirely below the null, or the all-lost refusal.
    Below,
    /// The interval covers the null, or the data cannot say (constant interior).
    Spans,
}

impl NullPosition {
    fn label(self) -> &'static str {
        match self {
            Self::Above => "above",
            Self::Below => "below",
            Self::Spans => "spans",
        }
    }
}

/// One screened cell — everything a round's ledger row is allowed to quote, plus
/// the provenance that makes it re-runnable.
#[derive(Debug, Clone)]
pub struct ScreenReport {
    pub provenance: ScreenProvenance,
    /// `units × players`.
    pub games: u32,
    /// `1/players` — 0.5 at 2P. Carried rather than assumed, because the
    /// adoption rule and the freeze criterion are both written against it.
    pub null: f64,
    /// The candidate's win share within each unit, in play order — the
    /// interval's input.
    pub candidate_unit_shares: Vec<f64>,
    pub theta_hat: f64,
    pub interval: IntervalOutcome,
    pub position: NullPosition,
    /// Sample sd of the unit shares (n − 1), `NaN` under two units.
    pub unit_sd: f64,
    /// `unit_sd / sqrt(n)`, `NaN` under two units.
    ///
    /// HONESTY (W3 harness review SHOULD-3): a zero here is NOT a collapse
    /// detector for a weight screen. A healthy 2P V1-vs-V1 null SPLITS many
    /// units 1–1 (measured: 22 of 40 at N=40), driving sd DOWN — a broken swap
    /// drives it UP — so the arena/architecture gates' `sd > 0` CRN check
    /// points the WRONG WAY here and is deliberately not borrowed. The routing
    /// pin (`candidate_games_by_seat` + `routing_matched`) is this instrument's
    /// integrity check; at smoke N a refusal with zero sd is the EXPECTED
    /// healthy shape, evidence of nothing.
    pub unit_se: f64,
    /// Moves applied by the candidate's seat, summed over the cell — the seat
    /// changes with the arrangement, so this follows the TABLE.
    pub candidate_applied_moves: u64,
    /// Decisions the candidate side EXPLAINED. Equal to
    /// [`candidate_applied_moves`](Self::candidate_applied_moves) on a completed
    /// cell — spec §0.3(b)'s identity, which rides BOTH sides here because both
    /// sides are evaluator agents (a rung-vs-evaluator cell can only state it
    /// once).
    pub candidate_explanations: u64,
    pub incumbent_applied_moves: u64,
    pub incumbent_explanations: u64,
    /// Games won by seat 0. Under the balanced rotation this sits near
    /// `games/players`; a drift is the seat effect the rotation removes.
    pub seat0_wins: u32,
    /// Mean over games of (candidate's VP total − the mean of the incumbent
    /// seats' totals) — the screening/triage statistic, never the gate one.
    pub vp_margin_mean: f64,
    /// Does the candidate satisfy L14/T1's `core > on_board > excess` in BOTH
    /// vectors? **RECORDED, never enforced here** — the hard constraint is the
    /// ladder's L2 to assert; a screen that refused would be claiming an
    /// adoption authority it does not have. Printed so a violating candidate
    /// cannot be screened without the violation appearing beside the figure.
    pub candidate_hand_split_ok: bool,
    pub incumbent_hand_split_ok: bool,
    /// Games in which the candidate's TABLE sat at each seat, tallied from the
    /// SEATING in the unit loop (arena-tested `unit_for` territory). Under the
    /// balanced design every seat holds the candidate equally often — the
    /// routing pin's first clause (W3 harness review MUST-1: a broken swap
    /// collapses this to one seat and nothing else in the report says so).
    pub candidate_games_by_seat: Vec<u32>,
    /// Did every agent get built with the table its seat was OWED? Cross-checked
    /// per game between the loop's seating-derived candidate seat and the digest
    /// the factory recorded from the table it actually built with — the routing
    /// pin's second clause (an inverted table→seat selection flips a screen's
    /// verdict while every other figure stays plausible). On a V1-vs-V1 null the
    /// digests are equal, so this clause discriminates only when the tables
    /// differ — which is every screen that matters.
    pub routing_matched: bool,
    /// The two SEARCH guards per side, `None` on a rung with no such quantity
    /// (the greedy one-ply argmax truncates nothing). READ from the handles
    /// rather than cloned-and-dropped — the evaluator_gate defect the 3.4
    /// six-lens wave fixed, kept fixed here (W3 harness review SHOULD-2). An
    /// UNCAPPED lookahead's only truncation guard is `MAX_TURN_PLIES`; a capped
    /// line is valued mid-turn, a slightly different measurement.
    pub candidate_ply_cap_hits: Option<u64>,
    pub candidate_free_cap_hits: Option<u64>,
    pub incumbent_ply_cap_hits: Option<u64>,
    pub incumbent_free_cap_hits: Option<u64>,
    /// Whether per-move `check_invariants` ran — carried into `render()` beside
    /// the build profile so an invariants-off screen cannot impersonate an
    /// invariants-on one in an artifact (protocol §2 lists profile among the
    /// provenance a figure must carry).
    pub invariants: bool,
}

impl ScreenReport {
    /// The whole report as one deterministic block: provenance first, then the
    /// estimate, then the counters.
    ///
    /// This is the byte-identity surface — `render()` at one diagnostic master
    /// reproduces exactly. Nothing load-sensitive (no wall clock) may enter it,
    /// for `arena::stats::CellWall`'s stated reason.
    pub fn render(&self) -> String {
        // Inline the full share vector only at smoke scale: at the protocol's
        // N=200 it is a ~1,800-character line inside the byte-identity surface
        // a ledger row quotes (W3 harness review NOTE). The struct always
        // carries the full vector; elision here loses nothing reproducible.
        let shares_repr = if self.candidate_unit_shares.len() <= 24 {
            let shares: Vec<String> =
                self.candidate_unit_shares.iter().map(|s| format!("{s:.6}")).collect();
            format!("[{}]", shares.join(", "))
        } else {
            format!(
                "[{} shares — elided from render; `candidate_unit_shares` carries them]",
                self.candidate_unit_shares.len(),
            )
        };
        let counter = |v: Option<u64>| match v {
            Some(n) => n.to_string(),
            None => "n/a".to_string(),
        };
        format!(
            "{}\n  \
             profile={} invariants={}\n  \
             theta_hat={:.6}  null={:.6}  position={}  interval={}\n  \
             unit sd={:.6}  SE={:.6}  n={} units  shares={}\n  \
             candidate seat rotation={:?}  routing={}\n  \
             candidate: applied={} explained={}   incumbent: applied={} explained={}\n  \
             ply cap hits: candidate={} incumbent={}   free cap hits: candidate={} incumbent={}\n  \
             seat0_wins={}/{}  vp_margin_mean={:+.4}\n  \
             hand-split ordering (L14 core>on_board>excess, both vectors): \
             candidate={} incumbent={}",
            self.provenance.render(),
            if cfg!(debug_assertions) { "debug" } else { "release" },
            if self.invariants { "on" } else { "OFF" },
            self.theta_hat,
            self.null,
            self.position.label(),
            describe_interval(&self.interval),
            self.unit_sd,
            self.unit_se,
            self.candidate_unit_shares.len(),
            shares_repr,
            self.candidate_games_by_seat,
            if self.routing_matched { "ok" } else { "MISMATCH — every figure above is suspect" },
            self.candidate_applied_moves,
            self.candidate_explanations,
            self.incumbent_applied_moves,
            self.incumbent_explanations,
            counter(self.candidate_ply_cap_hits),
            counter(self.incumbent_ply_cap_hits),
            counter(self.candidate_free_cap_hits),
            counter(self.incumbent_free_cap_hits),
            self.seat0_wins,
            self.games,
            self.vp_margin_mean,
            if self.candidate_hand_split_ok { "ok" } else { "VIOLATED" },
            if self.incumbent_hand_split_ok { "ok" } else { "VIOLATED" },
        )
    }

    /// **Freeze criterion 1's shape** (protocol §7): the bootstrap interval sits
    /// entirely above the null, OR the all-won zero-width refusal's
    /// rule-of-three one-sided bound EXCEEDS the null.
    ///
    /// Strictly stronger than `position == Above`: an all-won vector at n = 3
    /// classifies `Above` (directionally, it swept) but its bound is 0.0 and this
    /// returns `false`. That asymmetry is the point — a direction claim quotes a
    /// bound, a classification does not.
    pub fn direction_above(&self) -> bool {
        match self.interval {
            IntervalOutcome::Bootstrap { lo, .. } => lo > self.null,
            IntervalOutcome::RefusedZeroWidth { one_sided_lower, .. } => {
                self.all_units_won() && one_sided_lower > self.null
            }
        }
    }

    /// **Adoption rule (a)'s clause** (protocol §4.2 step 4): is the interval
    /// entirely BELOW the null? Adoption requires this to be `false` at the
    /// lookahead rung.
    ///
    /// Identical to `position == Below` and named separately so a call site reads
    /// as the rule it is applying rather than as a classification lookup.
    pub fn entirely_below_null(&self) -> bool {
        self.position == NullPosition::Below
    }

    fn all_units_won(&self) -> bool {
        !self.candidate_unit_shares.is_empty()
            && self.candidate_unit_shares.iter().all(|&u| u >= 1.0)
    }
}

fn describe_interval(interval: &IntervalOutcome) -> String {
    match interval {
        IntervalOutcome::Bootstrap { lo, hi, b, resample_seed } => {
            format!("bootstrap[{lo:.6}, {hi:.6}] B={b} resample_seed={resample_seed:#018x}")
        }
        IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
            format!("REFUSED one_sided_lower={one_sided_lower:.6} ({method})")
        }
    }
}

/// Classify an outcome against `null`, per the module doc's refusal rule.
///
/// The refusal arms read the UNIT VECTOR rather than the refusal's `method`
/// string: `interval_for_units` chooses its branch on exactly these two
/// predicates, so classifying from the same data cannot drift from it the way
/// matching on a private message would.
fn classify(interval: &IntervalOutcome, units: &[f64], null: f64) -> NullPosition {
    match interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => {
            if *lo > null {
                NullPosition::Above
            } else if *hi < null {
                NullPosition::Below
            } else {
                NullPosition::Spans
            }
        }
        IntervalOutcome::RefusedZeroWidth { .. } => {
            if units.is_empty() {
                NullPosition::Spans
            } else if units.iter().all(|&u| u >= 1.0) {
                NullPosition::Above
            } else if units.iter().all(|&u| u <= 0.0) {
                NullPosition::Below
            } else {
                NullPosition::Spans
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Hard constraints (recorded, not enforced)
// ---------------------------------------------------------------------------

/// L14/T1's one-ply non-degeneracy inequality — `core > on_board > excess` — in
/// BOTH weight vectors.
///
/// It is a NON-DEGENERACY CONDITION, not a prediction: a single linear in-hand
/// term cannot make a placement and a recovery both pay at one ply, which is why
/// the hand split exists at all ([`WeightTable`]'s doc carries the arithmetic).
/// Protocol §4.2 forbids relaxing it at any ladder rung; this function is how a
/// round checks a proposed table cheaply, and how [`ScreenReport`] records the
/// answer beside the figure.
pub fn hand_split_ordering_holds(weights: &WeightTable) -> bool {
    let core = Feature::AssistantsInHandCore.index();
    let board = Feature::AssistantsOnBoard.index();
    let excess = Feature::AssistantsInHandExcess.index();
    let holds = |v: &[i64; Feature::COUNT]| v[core] > v[board] && v[board] > v[excess];
    holds(&weights.early) && holds(&weights.late)
}

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------

/// Run one screen with per-move invariant checking ON — the default every driver
/// in the repo contracts for, and what a round's screen wants.
///
/// Single-threaded by construction: see the module doc's determinism note.
pub fn run_screen(spec: &ScreenSpec, rules: &Rules) -> Result<ScreenReport, ArenaError> {
    run_screen_with(spec, rules, true)
}

/// [`run_screen`] with per-move `check_invariants` as a parameter — the bulk
/// off-switch, on `run_cell_with`'s split and for its reason.
pub fn run_screen_with(
    spec: &ScreenSpec,
    rules: &Rules,
    invariants: bool,
) -> Result<ScreenReport, ArenaError> {
    // Both guards run BEFORE any game, because both failure modes are silent
    // otherwise: `unit_for(0, ..)` yields an empty seating list, so a bad player
    // count or a zero-unit design would tabulate nothing into a report full of
    // NaNs instead of erroring. The player-count check is the ARENA's own, so
    // there is one authority for what the engine plays.
    require_supported_player_count(spec.players)?;
    if spec.units == 0 {
        return Err(ArenaError::Driver(format!(
            "screen {:?} asked for 0 units: N is fixed before the run, and 0 is not a design",
            spec.name,
        )));
    }

    let (mut schedule, master) = match spec.master {
        SeedScheduleKind::Diagnostic(m) => (SeedSchedule::diagnostic(m), m),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };

    let players = spec.players;
    let mut shares: Vec<f64> = Vec::with_capacity(spec.units as usize);
    let mut games = 0u32;
    let mut seat0_wins = 0u32;
    let mut vp_margin_sum = 0.0f64;
    let mut cand_applied = 0u64;
    let mut cand_explained = 0u64;
    let mut inc_applied = 0u64;
    let mut inc_explained = 0u64;
    // MUST-1 (W3 harness review): the routing pin's two independent derivations.
    // `candidate_games_by_seat` is tallied from the SEATING in this loop —
    // arena-tested `unit_for` territory — while the digests below are recorded
    // by the FACTORY from the table it actually built with, so a broken swap
    // fails the first and an inverted table→seat selection fails the second.
    let cand_digest = weight_table_identity(&spec.candidate);
    let inc_digest = weight_table_identity(&spec.incumbent);
    let mut candidate_games_by_seat = vec![0u32; players as usize];
    let mut routing_matched = true;
    // SHOULD-2: the search guards, cloned out per side and READ rather than
    // dropped (the exact evaluator_gate defect the 3.4 six-lens wave fixed —
    // an uncapped lookahead's only truncation guard is MAX_TURN_PLIES, and a
    // capped line is valued mid-turn). `None` = the side has no such quantity.
    let mut cand_ply: Option<u64> = None;
    let mut cand_free: Option<u64> = None;
    let mut inc_ply: Option<u64> = None;
    let mut inc_free: Option<u64> = None;

    for _ in 0..spec.units {
        let seed = schedule.next_world();
        let unit = unit_for(players, seed);
        let mut unit_wins = 0u32;

        for seating in &unit.seatings {
            let cand_seat = seating
                .iter()
                .position(|&slot| slot == CANDIDATE_SLOT)
                .map(|i| PlayerId(i as u8))
                .ok_or_else(|| {
                    ArenaError::Driver(format!(
                        "seating {seating:?} has no slot {} to seat the candidate table in",
                        CANDIDATE_SLOT.0,
                    ))
                })?;

            candidate_games_by_seat[cand_seat.0 as usize] += 1;

            let seating = seating.clone();
            let (rung, candidate, incumbent) = (spec.rung, spec.candidate, spec.incumbent);
            let mut cand_handles: Vec<ExplainHandle> = Vec::new();
            let mut inc_handles: Vec<ExplainHandle> = Vec::new();
            let mut built: Vec<(u8, i64)> = Vec::new();
            let stats = {
                let (ch, ih, bt) = (&mut cand_handles, &mut inc_handles, &mut built);
                let mut make_agent = |seat: PlayerId, _players: u8| -> Box<dyn ArenaAgent> {
                    let slot = seating[seat.0 as usize];
                    let is_candidate = slot == CANDIDATE_SLOT;
                    let weights = if is_candidate { candidate } else { incumbent };
                    // Recorded from the LOCAL the agent is built with, post-
                    // selection — an inverted selection records the wrong digest
                    // and the cross-check below catches it.
                    bt.push((seat.0, weight_table_identity(&weights)));
                    let (agent, handle) =
                        rung.build(rules, weights, agent_stream_seed(seed, seat, slot));
                    if is_candidate {
                        ch.push(handle);
                    } else {
                        ih.push(handle);
                    }
                    agent
                };
                run_arena_game(seed, players, rules, &mut make_agent, invariants)?
            };

            routing_matched &= built.len() == players as usize;
            for (seat, digest) in &built {
                let expected = if *seat == cand_seat.0 { cand_digest } else { inc_digest };
                routing_matched &= *digest == expected;
            }

            games += 1;
            cand_applied += stats.moves_by_seat[cand_seat.0 as usize];
            inc_applied += stats.moves_by_seat.iter().sum::<u64>()
                - stats.moves_by_seat[cand_seat.0 as usize];
            for handle in &cand_handles {
                cand_explained += handle.explained();
                if let Some(h) = handle.ply_cap_hits() {
                    *cand_ply.get_or_insert(0) += h;
                }
                if let Some(h) = handle.free_cap_hits() {
                    *cand_free.get_or_insert(0) += h;
                }
            }
            for handle in &inc_handles {
                inc_explained += handle.explained();
                if let Some(h) = handle.ply_cap_hits() {
                    *inc_ply.get_or_insert(0) += h;
                }
                if let Some(h) = handle.free_cap_hits() {
                    *inc_free.get_or_insert(0) += h;
                }
            }

            if stats.result.winner == cand_seat {
                unit_wins += 1;
            }
            if stats.result.winner == PlayerId(0) {
                seat0_wins += 1;
            }

            let (mut cand_vp, mut inc_vp) = (0.0f64, 0.0f64);
            for standing in &stats.result.standings {
                if standing.player == cand_seat {
                    cand_vp = f64::from(standing.breakdown.total);
                } else {
                    inc_vp += f64::from(standing.breakdown.total);
                }
            }
            vp_margin_sum += cand_vp - inc_vp / f64::from(players - 1);
        }

        shares.push(f64::from(unit_wins) / unit.seatings.len() as f64);
    }

    let theta_hat = stats_mean(&shares);
    let resample_seed = resample_seed_for(master, spec.units, players);
    let interval = interval_for_units(&shares, BOOTSTRAP_REPLICATES, resample_seed);
    let null = 1.0 / f64::from(players);
    let position = classify(&interval, &shares, null);
    let unit_sd = stats_sample_sd(&shares);
    let unit_se = unit_sd / (shares.len() as f64).sqrt();

    Ok(ScreenReport {
        provenance: spec.provenance_at(master),
        games,
        null,
        candidate_unit_shares: shares,
        theta_hat,
        interval,
        position,
        unit_sd,
        unit_se,
        candidate_applied_moves: cand_applied,
        candidate_explanations: cand_explained,
        incumbent_applied_moves: inc_applied,
        incumbent_explanations: inc_explained,
        seat0_wins,
        vp_margin_mean: vp_margin_sum / f64::from(games),
        candidate_hand_split_ok: hand_split_ordering_holds(&spec.candidate),
        incumbent_hand_split_ok: hand_split_ordering_holds(&spec.incumbent),
        candidate_games_by_seat,
        routing_matched,
        candidate_ply_cap_hits: cand_ply,
        candidate_free_cap_hits: cand_free,
        incumbent_ply_cap_hits: inc_ply,
        incumbent_free_cap_hits: inc_free,
        invariants,
    })
}



#[cfg(test)]
mod tests {
    use super::*;

    /// This module's committed diagnostic master: the arena `A3E1` family, date
    /// field `2026_0809`, serial `0001` — the first free serial in that date
    /// field at authoring (grep-verified: no `0xA3E1_2026_0809_*` literal
    /// existed in the crate).
    const MASTER: u64 = 0xA3E1_2026_0809_0001;

    /// The deliberately WEAKENED candidate: [`WeightTable::V1`] with the
    /// warehouse term's SIGN inverted (`−10 → +10` early, `−4 → +4` late), so
    /// stranding assistants in the warehouse reads as an asset instead of the
    /// liability R1-4 measured it to be (0.3932 early, and the largest
    /// standardized weight in every multivariate fit).
    ///
    /// Chosen against three requirements, all asserted below rather than
    /// claimed here:
    ///
    /// 1. **NON-DEGENERATE** (map §8 row 18 — a constant-valued table degenerates
    ///    play into the all-empty-placement game). Exactly one feature moves;
    ///    fifteen are untouched; the table is nowhere near constant and
    ///    [`hand_split_ordering_holds`] still holds, since the warehouse term is
    ///    not in that triple.
    /// 2. **A PLAUSIBLE RE-PRICING SHAPE.** A sign on a measured feature is what
    ///    a round argues about; it is not an arbitrary perturbation.
    /// 3. **IT ACTUALLY MOVES PLAY.** Pinned in
    ///    [`b_a_weakened_candidate_screens_against_v1_with_distinct_identities`],
    ///    and that pin is load-bearing — see its doc.
    ///
    /// **A MEASURED OBSERVATION WORTH CARRYING INTO THE ROUNDS, recorded and NOT
    /// pinned** (scratch probe at authoring, Greedy rung, this master, N = 2
    /// units / 4 games, so it is four games on one master and never a ceiling):
    /// the FIRST candidate tried here was V1 with the whole EARLY vector HALVED,
    /// and it played **byte-identically to V1** — same applied-move counts (1,071
    /// / 1,021), same unit shares, same VP margin — while carrying a different
    /// identity digest. The arithmetic says why: `score = (1−p)·early + p·late`,
    /// so halving the early vector is an exact uniform rescale at `p = 0` (same
    /// argmax, same ties) and a no-op at `p = 1`, leaving only the intermediate
    /// band to disagree in. **The implication for §5 T4** (early-vector mixing,
    /// the protocol's primary re-pricing target) is that the early vector's
    /// overall SCALE is close to inert under this rung and what a round must
    /// re-price is the MIX — relative magnitudes within the vector, and the
    /// early-vs-late balance per feature. Two other probes moved play as
    /// expected (`WeightTable::ZERO`, and VP re-priced 10/40 → 1/4), so the
    /// inertness is a property of that mutation, not of the harness.
    fn warehouse_inverted(base: WeightTable) -> WeightTable {
        let mut out = base;
        let (early, late) = base.get(Feature::AssistantsInWarehouse);
        out.set(Feature::AssistantsInWarehouse, -early, -late);
        out
    }

    fn spec(name: &str, rung: ScreenRung, candidate: WeightTable, units: u32) -> ScreenSpec {
        ScreenSpec {
            name: name.to_string(),
            rung,
            candidate,
            incumbent: WeightTable::V1,
            players: 2,
            units,
            master: SeedScheduleKind::Diagnostic(MASTER),
        }
    }

    /// Everything about a screen that came from PLAY rather than from the spec —
    /// the tuple two screens must differ in before either one is evidence about
    /// anything.
    fn play_fingerprint(r: &ScreenReport) -> (u64, u64, u64, Vec<u64>) {
        (
            r.candidate_applied_moves,
            r.incumbent_applied_moves,
            r.vp_margin_mean.to_bits(),
            r.candidate_unit_shares.iter().map(|s| s.to_bits()).collect(),
        )
    }

    // -----------------------------------------------------------------------
    // (a) the NULL cell — the CRN-collapse detector shape
    // -----------------------------------------------------------------------

    /// V1 against itself: the two sides must be INDISTINGUISHABLE in provenance
    /// (same digest) and the instrument must still produce a usable estimate —
    /// either a real spread or the typed refusal, never a crash and never a
    /// printed ±0.000.
    #[test]
    fn a_null_screen_reports_equal_identities_and_a_usable_estimate() {
        let rules = Rules::standard();
        let s = spec("null-greedy", ScreenRung::Greedy, WeightTable::V1, 2);
        let report = run_screen(&s, &rules).expect("null screen runs");

        assert_eq!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "V1 vs V1 must report the SAME weight identity on both sides",
        );
        assert_eq!(report.games, 4, "2 units at 2P is 4 games");
        assert_eq!(report.candidate_unit_shares.len(), 2, "per-unit values must exist");
        assert_eq!(report.null, 0.5, "2P null");
        assert!(report.candidate_applied_moves > 0, "anti-vacuity: the cell played");
        assert_eq!(report.candidate_explanations, report.candidate_applied_moves);
        assert_eq!(report.incumbent_explanations, report.incumbent_applied_moves);

        match report.interval {
            IntervalOutcome::Bootstrap { lo, hi, .. } => {
                assert!(report.unit_se > 0.0, "a bootstrap interval needs a nonzero SE");
                assert!(lo <= hi, "interval endpoints ordered");
            }
            // At 2 units a healthy null REFUSING is the EXPECTED shape, not a
            // collapse signature (see `unit_se`'s doc — the sd>0 CRN check
            // points the wrong way for a weight screen); this branch only pins
            // that the refusal's arithmetic is coherent with a constant vector.
            IntervalOutcome::RefusedZeroWidth { .. } => {
                assert_eq!(report.unit_sd, 0.0, "a refusal means the unit vector is constant");
                assert_eq!(report.unit_se, 0.0);
            }
        }

        // The greedy rung has NO search guards — `None`, never `Some(0)` (the
        // Option discipline that keeps the lookahead assertion non-vacuous).
        assert_eq!(report.candidate_ply_cap_hits, None);
        assert_eq!(report.candidate_free_cap_hits, None);
        assert_eq!(report.incumbent_ply_cap_hits, None);
        assert_eq!(report.incumbent_free_cap_hits, None);
        // Routing fields exist on a null too; with equal digests the digest
        // clause is trivially satisfied and the SEAT clause still bites.
        assert_eq!(report.candidate_games_by_seat, vec![2, 2], "2 units rotate 2 seats twice");
        assert!(report.routing_matched);
        assert!(report.invariants);
        println!("{}", report.render());
    }

    /// MUST-1 (W3 harness review): the pin that the candidate TABLE provably
    /// reaches the candidate's rotating seat. Two independent clauses over one
    /// 1-unit cell with DISTINCT tables: (1) the seat tally, derived from the
    /// seating in the unit loop — a broken swap reads [2, 0]; (2) the digest
    /// cross-check, recorded by the factory from the table it actually built
    /// with — an inverted table→seat selection records the incumbent's digest
    /// at the candidate's seat and `routing_matched` goes false. Both survived
    /// mutations were verified caught by exactly these clauses before commit.
    #[test]
    fn the_candidate_table_provably_reaches_its_rotating_seat() {
        let rules = Rules::standard();
        let weakened = warehouse_inverted(WeightTable::V1);
        let s = ScreenSpec {
            name: "routing-pin".to_string(),
            rung: ScreenRung::Greedy,
            candidate: weakened,
            incumbent: WeightTable::V1,
            players: 2,
            units: 1,
            master: SeedScheduleKind::Diagnostic(MASTER),
        };
        let report = run_screen(&s, &rules).expect("routing pin cell runs");
        assert_eq!(
            report.candidate_games_by_seat,
            vec![1, 1],
            "one unit at 2P seats the candidate at each seat exactly once",
        );
        assert!(
            report.routing_matched,
            "every agent must be built with the table its seat was owed",
        );
        assert_ne!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "anti-vacuity: the digest clause discriminates only when the tables differ",
        );
    }

    /// The SHIPPED rung's plumbing, at the smallest design that exists. One unit
    /// is not a statistical statement about anything and nothing here treats it
    /// as one — what it buys is that `ScreenRung::Lookahead` actually builds,
    /// plays, explains and tabulates, which a greedy-only file cannot show. It is
    /// also the wall this file spends: ~6 s of its ~7 s debug total.
    #[test]
    fn the_lookahead_rung_runs_the_same_null_at_smoke_scale() {
        let rules = Rules::standard();
        let s = spec("null-lookahead", ScreenRung::Lookahead, WeightTable::V1, 1);
        let report = run_screen(&s, &rules).expect("lookahead null screen runs");

        assert_eq!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "V1 vs V1 must report the SAME weight identity on both sides",
        );
        assert_eq!(report.games, 2);
        assert_eq!(report.provenance.controller, "lookahead-v1");
        // Spec §0.3(b)'s identity, on BOTH sides — both agents explain here,
        // which is a screen's own property (a rung-vs-evaluator cell has one
        // explaining side and can only state it once).
        assert_eq!(report.candidate_explanations, report.candidate_applied_moves);
        assert_eq!(report.incumbent_explanations, report.incumbent_applied_moves);
        assert!(report.candidate_applied_moves > 0, "anti-vacuity: the cell played");
        // SHOULD-2: the search guards are READ, not cloned-and-dropped. An
        // uncapped S1 screen expects `Some(0)` on all four — `Some` proves the
        // plumbing (the greedy null pins `None`, so this cannot pass vacuously),
        // and 0 is the only value at which every line was valued at a true turn
        // boundary rather than mid-turn.
        assert_eq!(report.candidate_ply_cap_hits, Some(0));
        assert_eq!(report.candidate_free_cap_hits, Some(0));
        assert_eq!(report.incumbent_ply_cap_hits, Some(0));
        assert_eq!(report.incumbent_free_cap_hits, Some(0));
        println!("{}", report.render());
    }

    // -----------------------------------------------------------------------
    // (b) the DISCRIMINATING smoke
    // -----------------------------------------------------------------------

    /// A weakened candidate must screen as a different table **in provenance AND
    /// in play**.
    ///
    /// The play half is the load-bearing one and it was added because the first
    /// candidate tried here failed it silently (see [`warehouse_inverted`]'s
    /// recorded observation): differing digests prove only that two *labels*
    /// differ. Without a play separation, an instrument that never routed the
    /// candidate table to the candidate's seat — comparing V1 against V1 under
    /// two names — would pass every other test in this file, and every round
    /// would then read a null screen as evidence.
    ///
    /// **NO DIRECTION IS ASSERTED.** At two units the cell has no power worth the
    /// name, and the protocol's positive evidence is L1–L3's job anyway. The
    /// authoring probe makes the hazard concrete: a candidate with VP re-priced
    /// 10/40 → 1/4 — a clear weakening — SWEPT this master at N = 2 (both units
    /// to the candidate). A direction assertion here would have pinned that.
    #[test]
    fn b_a_weakened_candidate_screens_against_v1_with_distinct_identities() {
        let rules = Rules::standard();
        let weakened = warehouse_inverted(WeightTable::V1);

        // Non-degeneracy, asserted rather than asserted-in-prose (map §8 row 18).
        assert!(hand_split_ordering_holds(&weakened));
        assert!(hand_split_ordering_holds(&WeightTable::V1));
        let moved: Vec<Feature> = Feature::ALL
            .into_iter()
            .filter(|&f| weakened.get(f) != WeightTable::V1.get(f))
            .collect();
        assert_eq!(moved, vec![Feature::AssistantsInWarehouse], "exactly one feature moves");
        assert_eq!(weakened.get(Feature::AssistantsInWarehouse), (10, 4));
        assert_ne!(weight_table_identity(&weakened), weight_table_identity(&WeightTable::V1));

        let null = run_screen(&spec("null-contrast", ScreenRung::Greedy, WeightTable::V1, 2), &rules)
            .expect("contrast null runs");
        let s = spec("weakened-warehouse-greedy", ScreenRung::Greedy, weakened, 2);
        let report = run_screen(&s, &rules).expect("weakened screen runs");

        assert_ne!(
            report.provenance.candidate_identity, report.provenance.incumbent_identity,
            "a weakened candidate must report a DIFFERENT identity from the incumbent",
        );
        assert_ne!(
            play_fingerprint(&report),
            play_fingerprint(&null),
            "the candidate table must reach the CANDIDATE'S SEAT — a screen whose play \
             matches the V1-vs-V1 null on the same master is measuring nothing",
        );
        assert!(report.candidate_hand_split_ok && report.incumbent_hand_split_ok);
        assert_eq!(report.candidate_explanations, report.candidate_applied_moves);
        assert_eq!(report.incumbent_explanations, report.incumbent_applied_moves);
        println!("{}", report.render());
    }

    // -----------------------------------------------------------------------
    // (c) determinism
    // -----------------------------------------------------------------------

    #[test]
    fn c_the_same_master_reproduces_the_report_byte_identically() {
        let rules = Rules::standard();
        let s = spec("determinism", ScreenRung::Greedy, warehouse_inverted(WeightTable::V1), 1);
        let first = run_screen(&s, &rules).expect("first run");
        let second = run_screen(&s, &rules).expect("second run");
        assert_eq!(first.render(), second.render(), "same master, same report");

        // Anti-vacuity, and PLAY-derived rather than string-derived: the echo
        // carries the master, so a whole-string `assert_ne!` alone would pass on
        // a report whose games never moved (`architecture_gate`'s determinism
        // cell states the same hazard about `spec_echo`).
        let mut flipped = s.clone();
        flipped.master = SeedScheduleKind::Diagnostic(MASTER ^ 0x11);
        let other = run_screen(&flipped, &rules).expect("flipped run");
        assert_ne!(first.render(), other.render(), "a flipped master must change the report");
        assert_ne!(
            play_fingerprint(&first),
            play_fingerprint(&other),
            "the flipped master must move the PLAY, not merely the echoed master",
        );
    }

    // -----------------------------------------------------------------------
    // (d) provenance — the T16 composite key
    // -----------------------------------------------------------------------

    #[test]
    fn d_provenance_names_both_identities_the_rev_and_the_rung_knobs() {
        let weakened = warehouse_inverted(WeightTable::V1);
        for rung in [ScreenRung::Greedy, ScreenRung::Lookahead] {
            let s = spec("prov", rung, weakened, 4);
            let p = s.provenance().expect("a diagnostic spec knows its master");
            let rendered = p.render();

            assert_eq!(p.semantics_rev, SEMANTICS_REV);
            assert_eq!(p.eval_impl_version, EVAL_IMPL_VERSION);
            assert_eq!(p.candidate_identity, weight_table_identity(&weakened));
            assert_eq!(p.incumbent_identity, weight_table_identity(&WeightTable::V1));
            assert_ne!(p.candidate_identity, p.incumbent_identity);
            assert_eq!(p.games, 8, "4 units at 2P");
            assert!(rendered.contains(&format!("semantics_rev={SEMANTICS_REV}")));
            assert!(rendered.contains(&format!("eval_impl={EVAL_IMPL_VERSION}")));
            assert!(rendered.contains(&format!("weights={}", p.candidate_identity)));
            assert!(rendered.contains(&format!("weights={}", p.incumbent_identity)));
            assert!(rendered.contains("units=4"));
            assert!(rendered.contains("games=8"));
            assert!(rendered.contains(&format!("master=diagnostic:{MASTER:#018x}")));
            assert!(rendered.contains("setup-identical=yes"));
            assert!(rendered.contains(&format!("controller_version={}", p.controller_version)));

            match rung {
                ScreenRung::Greedy => {
                    assert_eq!(p.controller, "greedy-v1");
                    assert_eq!(p.candidate_params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
                    assert!(rendered.contains("m=8"));
                }
                ScreenRung::Lookahead => {
                    assert_eq!(p.controller, "lookahead-v1");
                    assert_eq!(p.candidate_params["b"], i64::from(DEFAULT_BEAM));
                    assert_eq!(p.candidate_params["k"], i64::from(DEFAULT_WORLDS));
                    assert_eq!(p.candidate_params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
                    assert!(rendered.contains("b=4"));
                    assert!(rendered.contains("k=1"));
                    assert!(rendered.contains("m=8"));
                    // The SHIPPED rung is uncapped, and an absent cap emits no
                    // token at all — the property every 3.3 golden depends on.
                    assert!(!rendered.contains("cap="));
                }
            }
        }

        // The provenance a RUN fills in is the same builder's output, so the two
        // paths cannot drift.
        let rules = Rules::standard();
        let s = spec("prov-run", ScreenRung::Greedy, weakened, 1);
        let ran = run_screen(&s, &rules).expect("runs");
        assert_eq!(ran.provenance.render(), s.provenance().expect("diagnostic").render());

        // An OS-entropy spec has no master until the run draws one, and says so.
        let mut rotating = spec("rotating", ScreenRung::Greedy, WeightTable::V1, 4);
        rotating.master = SeedScheduleKind::OsEntropy;
        assert!(rotating.provenance().is_none());
    }

    // -----------------------------------------------------------------------
    // Refusal classification and the two rules it feeds — pure, no games.
    // -----------------------------------------------------------------------

    #[test]
    fn the_refusal_classification_follows_the_protocols_above_below_rule() {
        let refused = |lower: f64| IntervalOutcome::RefusedZeroWidth {
            one_sided_lower: lower,
            method: "test fixture",
        };
        assert_eq!(classify(&refused(0.985), &[1.0, 1.0], 0.5), NullPosition::Above);
        assert_eq!(classify(&refused(0.0), &[0.0, 0.0], 0.5), NullPosition::Below);
        assert_eq!(classify(&refused(0.0), &[0.5, 0.5], 0.5), NullPosition::Spans);

        let boot =
            |lo: f64, hi: f64| IntervalOutcome::Bootstrap { lo, hi, b: 10, resample_seed: 1 };
        assert_eq!(classify(&boot(0.60, 0.70), &[0.5, 1.0], 0.5), NullPosition::Above);
        assert_eq!(classify(&boot(0.20, 0.40), &[0.0, 0.5], 0.5), NullPosition::Below);
        assert_eq!(classify(&boot(0.40, 0.60), &[0.0, 1.0], 0.5), NullPosition::Spans);
        // The 3-4P null is 1/players, so classification must read the null it is
        // given rather than assume 0.5.
        assert_eq!(classify(&boot(0.30, 0.45), &[0.0, 1.0], 0.25), NullPosition::Above);
    }

    /// `direction_above` is STRICTLY stronger than `position == Above`: the
    /// all-won refusal must clear the null with its BOUND, which n = 3 does not.
    #[test]
    fn an_all_won_refusal_classifies_above_but_only_clears_the_null_at_a_real_n() {
        let report = |n: usize| {
            let units = vec![1.0f64; n];
            let lower = (1.0 - 3.0 / n as f64).max(0.0);
            let interval =
                IntervalOutcome::RefusedZeroWidth { one_sided_lower: lower, method: "fixture" };
            let position = classify(&interval, &units, 0.5);
            ScreenReport {
                provenance: spec("f", ScreenRung::Greedy, WeightTable::V1, 1)
                    .provenance()
                    .expect("diagnostic"),
                games: n as u32 * 2,
                null: 0.5,
                candidate_unit_shares: units,
                theta_hat: 1.0,
                interval,
                position,
                unit_sd: 0.0,
                unit_se: 0.0,
                candidate_applied_moves: 0,
                candidate_explanations: 0,
                incumbent_applied_moves: 0,
                incumbent_explanations: 0,
                seat0_wins: 0,
                vp_margin_mean: 0.0,
                candidate_hand_split_ok: true,
                incumbent_hand_split_ok: true,
                candidate_games_by_seat: vec![n as u32, n as u32],
                routing_matched: true,
                candidate_ply_cap_hits: None,
                candidate_free_cap_hits: None,
                incumbent_ply_cap_hits: None,
                incumbent_free_cap_hits: None,
                invariants: true,
            }
        };
        let small = report(3);
        assert_eq!(small.position, NullPosition::Above);
        assert!(!small.direction_above(), "bound 0.0 does not clear the 0.5 null");
        assert!(!small.entirely_below_null());

        let sized = report(200);
        assert_eq!(sized.position, NullPosition::Above);
        assert!(sized.direction_above(), "1 - 3/200 = 0.985 clears the null");
    }

    #[test]
    fn the_hand_split_check_catches_a_violation_in_either_vector() {
        assert!(hand_split_ordering_holds(&WeightTable::V1));
        assert!(!hand_split_ordering_holds(&WeightTable::ZERO), "a flat table has no ordering");

        let mut early_broken = WeightTable::V1;
        let (_, late) = early_broken.get(Feature::AssistantsInHandCore);
        early_broken.set(Feature::AssistantsInHandCore, 0, late);
        assert!(!hand_split_ordering_holds(&early_broken));

        let mut late_broken = WeightTable::V1;
        let (early, _) = late_broken.get(Feature::AssistantsInHandExcess);
        late_broken.set(Feature::AssistantsInHandExcess, early, 99);
        assert!(!hand_split_ordering_holds(&late_broken));
    }

    #[test]
    fn a_zero_unit_or_unsupported_screen_is_a_typed_error_not_a_nan_report() {
        let rules = Rules::standard();
        let mut zero = spec("zero", ScreenRung::Greedy, WeightTable::V1, 0);
        assert!(matches!(run_screen(&zero, &rules), Err(ArenaError::Driver(_))));
        zero.units = 1;
        zero.players = 5;
        assert!(matches!(run_screen(&zero, &rules), Err(ArenaError::Driver(_))));
    }

    #[test]
    fn the_screens_resample_stream_is_its_own_domain() {
        // Same components, different salt from `arena::stats`'s bootstrap — two
        // instruments handed the same master must not silently share a stream.
        let screen = resample_seed_for(MASTER, 200, 2);
        let stats = mix(crate::game::arena::stats::SALT_BOOTSTRAP, &[MASTER, 200, 2]);
        assert_ne!(screen, stats);
        // …and it is a function of the DESIGN alone, so it reproduces.
        assert_eq!(screen, resample_seed_for(MASTER, 200, 2));
        assert_ne!(screen, resample_seed_for(MASTER, 201, 2));
    }
}
