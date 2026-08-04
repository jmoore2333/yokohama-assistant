//! **MCTS-v1** — the UCT contestant of the Phase 3.4 architecture experiment
//! (spec §2, §3.1–§3.3). A fresh tree per decision over ONE
//! [`BeliefSampler`]-sampled world, expanded one node per simulation, valued by
//! the frozen [`WeightTable::V1`] evaluator at the leaf and by the official
//! ranking at a terminal.
//!
//! # The honest chain, verbatim (spec §3.1)
//!
//! `observe` → [`BeliefSampler`] `K = 1` → one sampled world → search → the
//! chosen [`Move`] back through the driver's legality boundary. **Sampler
//! failure is a hard error with no fallback arm** — the same rule
//! [`crate::game::greedy`] and [`crate::game::lookahead`] carry, for the same
//! reason: an arena that fell back to the true legal list would emit
//! privileged-play games into the corpus and nothing downstream could tell them
//! from honest ones.
//!
//! `K` exists as a knob and defaults to 1. **`K > 1` is REFUSED at the
//! search**, and the refusal names spec §0.4: aggregating a tree across sampled
//! worlds is information-set search (candidate (c)), which §0.4 keeps OUT of
//! this cycle on measured grounds. The knob is kept so the shape is stated
//! rather than absent; answering a K it cannot honestly aggregate would be
//! worse than saying so.
//!
//! # The node, and why the handle is a scoped temporary (spec §2.5, ledger L10)
//!
//! A [`DecisionHandle`] borrows `&GameState`, so a node cannot own one. A node
//! therefore owns `(GameState, Vec<Move> candidates, stats)` and the handle is a
//! stack temporary built **at most once per node** — structurally, not by
//! discipline: [`Search::expand_point`] is this module's ONLY
//! `DecisionHandle::new` call site (pinned by
//! `the_module_builds_exactly_one_decision_handle_per_node`), it builds exactly
//! one, and every expandable node is created through it.
//!
//! The **forced fast path** is the same cache read: R4 measured 59.9% of 2P
//! decision points offering ≤ 1 candidate, and DISCOVERING that costs a full
//! enumeration, so the answer is kept on the node. At the DECISION level a
//! 0/1-candidate point never builds a tree at all — the root's single
//! enumeration answers it and no simulation runs.
//!
//! A **terminal** node builds no handle: a finished game offers no families, so
//! enumerating it would be an error rather than a cost. Its value comes from
//! [`final_result`] (below), which is why `handles == nodes − terminal_nodes`
//! rather than `handles == nodes`.
//!
//! # Values: the normalization is DECLARED, because UCB1 is meaningless without it
//!
//! R5's treant spike found the foot-gun at source: adding `c · explore` to a
//! RAW `eval::score` mean makes the exploration constant inert (reward scales of
//! 1,000 and 1,000,000 produced identical visit distributions). Spec §2.4
//! therefore requires the mapping to be named. It is:
//!
//! ```text
//! squash(x) = 0.5 · (1 + x / (|x| + scale)),    scale ≥ 1
//! ```
//!
//! — strictly monotone on ℝ into the open `(0, 1)` in exact arithmetic (in
//! `f64` it saturates to a closed `[0, 1]` far outside anything organic; the
//! threshold is stated at [`squash`]), `squash(0) = 0.5`, and `scale` is
//! exactly the relative margin at which the map reads 0.75. `x` is
//! [`eval::score`]'s RELATIVE margin from the ROOT seat's view (the evaluator is
//! already a difference against the opponent extreme, so no second relativization
//! happens here). `scale` is a screening knob (spec §4); the shipped default is
//! [`DEFAULT_SQUASH_SCALE`], whose measured grounds — the candidate-to-candidate
//! score spread at organic 2P decision points — are recorded at that constant,
//! **together with the measured consequence that the shipped `(c, scale)` pair
//! is exploration-dominated at small budgets.** Read that note before reading a
//! screening figure as a statement about UCT rather than about a knob pair.
//!
//! **Terminal states never pass through the squash.** They are valued from
//! [`final_result`] — the official clockwise tie-break, never re-implemented —
//! as `1 − (rank − 1)/(players − 1)`, i.e. 1.0 for a win and 0.0 for last place,
//! which puts them on the same `[0, 1]` axis as the squash without borrowing the
//! evaluator's terminal ladder.
//!
//! # The descent, and the opponent model it implies (spec §3.2)
//!
//! Selection is UCB1 over the normalized values; expansion takes one unexpanded
//! candidate; the leaf value backs up **from the ROOT seat's perspective** at
//! every node on the path, and a node whose acting seat is NOT the root seat
//! SELECTS by that seat's own value — `1 − v` at 2P ([`selection_value`]).
//!
//! **That makes the opponent model a self-play/paranoid descent that reads the
//! opponent's options inside the determinized world — a channel
//! `lookahead-v1` structurally never uses** (a hand-off child is a leaf there).
//! It is stated because §7's world-sensitivity expectation follows from it: a
//! tree that descends past the hand-off reads determinized opponent options as
//! well as determinized refills, so a nonzero measured cross-world divergence is
//! the design working, not a leak. Leakage — which stays asserted at zero — is a
//! different property (spec §7 / ledger L24); §7's contract comment lands with
//! that task.
//!
//! At 3–4P the `1 − v` flip reads as "every non-root seat plays against the
//! root", the paranoid reading. 3–4P strength cells are out of scope (spec §1);
//! the convention is stated so a later reader is not left inferring it.
//!
//! **The root is IN the backprop path**, so `root.visits == simulations`
//! exactly. That is a pin (`the_root_is_in_the_backprop_path`) rather than an
//! implementation detail because R5 found the opposite in a shipped library: a
//! root whose visits never increment freezes any visit-driven schedule at
//! `f(0)` and fails silently.
//!
//! # Keyed streams: every draw is one word, keyed by a coordinate tree shape
//! cannot move
//!
//! [`crate::game::lookahead`]'s discipline, carried over and tightened. A
//! decision draws exactly THREE words off the agent's own play stream — sampler
//! seed, tie key, candidate seed — and nothing inside the search consumes from
//! it. Every selecting point then takes ONE keyed word ([`uniform_index`],
//! Lemire multiply-shift, constant consumption whatever the tie count) from
//! [`McStream`], keyed by:
//!
//! | selecting point | site | coordinate |
//! |---|---|---|
//! | root argmax over visits | [`McSite::RootArgmax`] | — |
//! | UCB selection ties | [`McSite::Ucb`] | the node's path key, its own visit count |
//! | expansion pick | [`McSite::Expansion`] | the node's path key, how many children it has already |
//!
//! The **path key** ([`descend_branch`]) is the fold of the edge indices from
//! the root, so a node's draws are a function of WHERE it is and not of when it
//! was created: growing the tree elsewhere cannot move them. The same key seeds
//! the node's CANDIDATE dice ([`node_candidate_seed`]) — the shipped candidate
//! scheme draws (M retreat samples, a placement reservoir), and threading one
//! continuing stream through an incrementally grown tree would make a node's
//! candidate list depend on the order its siblings happened to be expanded in.
//! The SCHEME is [`crate::game::greedy::candidates`] verbatim (spec §3.3); only
//! its seeding is per-node — scheme-level identity, not draw-level: at a given
//! `candidate_seed` the DRAWN retreat samples and placement reservoirs differ
//! from greedy's own draws, deliberately, and the fidelity tests compare
//! dice-free decision points — and that is the MCTS analogue of the
//! per-decision keying lookahead adopted at its T5 review for the identical
//! reason.
//!
//! lookahead's own `descend_branch` folds into 32 bits and records the
//! resulting collision hazard as a known limitation; this one folds into 64
//! through [`splitmix64`], so two different paths sharing a key is not a hazard
//! this module has to scope.
//!
//! # What this module does NOT claim
//!
//! **Determinism is same-binary.** Values are `f64` and UCB1 uses `ln`/`sqrt`;
//! `sqrt` is IEEE-exact but `ln` is a libm function whose last bit is not
//! standardised across platforms, so "same master ⇒ identical decision" is
//! pinned (and true) for one binary and is not a cross-platform claim. Nothing
//! in the experiment needs one — every cell is a single-machine run whose
//! master is recorded.
//!
//! **The leaf rule shipped here is the DIAGNOSTIC one.** [`LeafRule`] values an
//! unexpanded node by the raw evaluation of the state it stands at, which for
//! ~40% of nodes is MID-TURN — states `WeightTable::V1` was never calibrated on
//! and which `lookahead-v1` by construction never evaluates (R3-4, the measured
//! mechanism behind the prototype's loss). Spec §2.1 makes the hand-off
//! completion leaf the DEFAULT; it arrives with the next task and swaps in at
//! [`Search::leaf_value`]'s single `match`, which is why the rule is an enum
//! rather than an inlined expression.
//!
//! The per-decision scored-successor budget ([`crate::game::budget`]) is
//! METERED here ([`McSearchStats::scored`]) and not yet GATED; the explanation
//! surface (spec §3.5), the arena seam and the provenance version const (spec
//! §3.6) land with their own tasks.
//!
//! # It completes games, and how it completes them is itself a datum
//!
//! Non-termination is a live hazard class this cycle — the capped S1 arm has a
//! MEASURED middle cap band where 2P self-play never ends
//! (`LookaheadAgent::with_cap`) — so the arm was probed before it was handed on.
//! Authoring probe (release, out-of-gate scratch harness, nothing asserted
//! anywhere; 2P MCTS-vs-MCTS self-play at the shipped knobs, masters
//! `0x1EA4_2026_0727_0001 ^ (m · 0x11)` for `m = 0..3`, 20,000-move budget):
//! **3/3 complete**. The turn-count RANGE first recorded here (100–108) is
//! WITHDRAWN (T2 review): the probe's block omitted its agent-stream seeding,
//! Keep resolution and invariants setting — and the quantity is dominated by
//! them. The review reproduced 3/3 completion under three plausible seedings
//! but measured **50–304 turns** across them (N = 3 masters each, contended
//! walls 26–140 ms/game), so what stands is completion under every seeding
//! tried, and the qualitative note that MCTS self-play turns run well above
//! lookahead self-play's under all of them — the slow-development shape R3
//! measured for a naive UCT-eval arm, arriving again here. Recorded, not
//! defended: the leaf rule and the budget wiring the next task brings are the
//! two things expected to move it, and the executable version of this probe is
//! that task's degeneracy pin.
//!
//! Review measurement (T2 review, release, out-of-tree probe, nothing asserted:
//! `Rules::standard()`, frozen V1, 2P, master `0x1EA4_2026_0727_0001`,
//! Greedy-v1 walk, first 120 decision points): at `DEFAULT_SIMULATIONS = 64`,
//! **38 of 46 non-forced points (83%) are settled by the root tie draw** —
//! Placement 19/19, tie-sets of 64 of 186 candidates, `max_depth` 1.0: a STAR
//! TREE in which no value reaches the decision. At sims = 750 (the experiment's
//! B\*) Movement recovers (1/19 tie-decided) but **Placement is still 19/19**
//! (tie-set 6–24). Cause: the expansion pick is a uniform keyed draw over
//! unexpanded slots and the read-out is the mandated argmax over visits, with
//! no priors — spec §2.3's ordering/priors are the NEXT task's job, this
//! measurement is the defect they must visibly fix (re-measure the Placement
//! tie-set after), and spec §4's screening must not read these figures as a
//! statement about UCT-with-priors.

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::DecisionReport;
use crate::game::belief::BeliefSampler;
use crate::game::eval::{self, WeightTable};
use crate::game::greedy::{
    candidates, observed_view, score_starting_order, uniform_index, Candidates,
    DEFAULT_RETREAT_SAMPLES,
};
use crate::game::metrics::{decision_volume, VolumeObs};
use crate::game::rules::Rules;
use crate::game::scoring::final_result;
use crate::game::search_actions::DecisionHandle;
use crate::game::seeds::splitmix64;
use crate::types::{
    GameState, Move, OrderCardId, PlayerId, PlayerView, PresidentLocation, TurnPhase,
};

// ---------------------------------------------------------------------------
// Identity and knobs
// ---------------------------------------------------------------------------

/// The stable controller name — error messages, and (with its own version
/// const, which lands with the arena seam) provenance and the runner grammar.
pub const MCTS_CONTROLLER: &str = "mcts-v1";

/// Simulations per decision. **A placeholder default with no strength claim
/// behind it**: the experiment sizes the search from the shared
/// [`ScoredBudget`](crate::game::budget::ScoredBudget) cap (spec §3.4), which
/// arrives with the leaf/budget task. This value exists so the knob is total and
/// so a caller that never mentions it gets a search that terminates.
pub const DEFAULT_SIMULATIONS: u32 = 64;

/// UCB1's exploration constant `c`, in PARTS PER MILLION (1,414,214 ≈ √2 — the
/// textbook value, and the one R3's prototype ran UNTUNED; spec §4 sweeps it as
/// a screening knob and the research notes flag the untuned figure as such).
///
/// ppm rather than `f64` so the knob is an integer at every boundary that has to
/// carry it — the runner grammar (`c=<ppm>`) and the provenance param map, whose
/// values are `i64`.
pub const DEFAULT_EXPLORATION_PPM: u32 = 1_414_214;

/// The squash's scale, in [`eval::score`] units: the relative margin at which
/// [`squash`] reads 0.75.
///
/// **Measured grounds** (authoring measurement, release, `Rules::standard()`,
/// the frozen `WeightTable::V1`, 2P, a Greedy-v1 walk of the first 300 moves at
/// master `0x1EA4_2026_0727_0001`, the shipped candidate scheme at
/// `M = 8`, 116 decision points offering >1 candidate; an out-of-gate harness
/// run at authoring — nothing below is asserted anywhere). The quantity that
/// matters is how far apart the CANDIDATES at one decision point score, because
/// that is the difference UCB1's exploitation term has to express:
///
/// ```text
/// candidate-to-candidate spread at a point : median 36 · p90 92 · max 167
/// mean abs deviation within a point        : median 9
/// |score change| between successive states : median 0 · p90 250
/// ```
///
/// A scale of 100 puts a median-spread point (36) at a value difference of
/// ≈0.13 and the widest observed point (167) at ≈0.31 — on the steep part of the
/// map rather than in a tail. A scale of 1,000, the first value tried here,
/// would have compressed the median point to ≈0.018, which is the R5 foot-gun in
/// its other form: not an inert `c`, but an inert VALUE.
///
/// **The interaction with [`DEFAULT_EXPLORATION_PPM`] is stated because the
/// same arithmetic reaches it, and it is NOT resolved here.** UCB1's exploration
/// term is identical across children with equal visit counts, so it cancels at
/// the first pass and the value differences above decide it; what it governs is
/// RE-visiting. At `c = √2` and 64 simulations, preferring a once-visited child
/// over an unvisited sibling costs `c·√(ln N)·(1 − 1/√2) ≈ 0.84` of value —
/// several times the widest value difference this evaluator produces at a point.
/// So the shipped pair is exploration-dominated, i.e. near-breadth-first at
/// small budgets. That is exactly what spec §4's screening grid (3 declared `c`
/// values × 2 declared scales, `leaf` on the other axis) exists to resolve, and
/// this constant is the DEFAULT the grid starts from, never a claim about the
/// best one.
pub const DEFAULT_SQUASH_SCALE: i64 = 100;

/// `K`: sampled worlds per decision. **1 ships**, and `K > 1` is refused —
/// see the module doc and spec §0.4.
pub const DEFAULT_WORLDS: u8 = 1;

/// Domain salt for the keyed tie stream, in this crate's `mix` shape.
const SALT_MCTS_STREAM: u64 = 0x3C15_2026_0804_0001;
/// Domain salt for the per-node candidate seed — a DIFFERENT domain from the
/// tie stream, so a node's candidate dice and its tie draws cannot be
/// correlated by construction.
const SALT_MCTS_CANDIDATES: u64 = 0x3C15_2026_0804_0002;
/// Domain salt for the path-key fold.
const SALT_MCTS_BRANCH: u64 = 0x3C15_2026_0804_0003;

/// The root's index in the node arena. Named because "node 0 is the root" is an
/// invariant the descent leans on at every simulation.
const ROOT: usize = 0;

/// How an unexpanded node is valued (spec §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeafRule {
    /// **The DIAGNOSTIC rule**: evaluate the node's own state where it stands,
    /// mid-turn included. Spec §2.1 keeps it as a screening knob precisely
    /// because ~40% of a naive UCT's leaf values are read at states the
    /// evaluator was never calibrated on — measuring that arm is the point of
    /// keeping it.
    ///
    /// The hand-off completion rule (spec §2.1's DEFAULT) swaps in at
    /// [`Search::leaf_value`]'s `match` with the next task.
    #[default]
    MidTurnDiag,
}

// ---------------------------------------------------------------------------
// The keyed stream
// ---------------------------------------------------------------------------

/// WHERE a draw was taken, as a coordinate the tree's growth cannot move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McSite {
    /// The root's argmax over visit counts — the decision itself.
    RootArgmax,
    /// A UCB selection tie at the node whose path key is `branch`, on that
    /// node's `visit`-th descent through it.
    Ucb { branch: u64, visit: u32 },
    /// The expansion pick at the node whose path key is `branch`, filling its
    /// `filled`-th child slot.
    Expansion { branch: u64, filled: u32 },
}

impl McSite {
    /// `(tag, a, b)` — the tag keeps the site families disjoint where their
    /// coordinates coincide.
    fn coords(self) -> [u64; 3] {
        match self {
            Self::RootArgmax => [0, 0, 0],
            Self::Ucb { branch, visit } => [1, branch, u64::from(visit)],
            Self::Expansion { branch, filled } => [2, branch, u64::from(filled)],
        }
    }
}

/// One decision's draw randomness: ONE word off the agent's play stream, folded
/// per site.
///
/// [`crate::game::lookahead::TieStream`]'s two properties, held for the same
/// reasons: **constant consumption** (the search spends zero play-stream words
/// however many draws it takes, so the agent's stream position is a function of
/// the decision COUNT and nothing else) and **site-addressability** (a site's
/// word is a pure function of its coordinates, so the same site in a re-run
/// draws the same word).
///
/// A separate type from lookahead's rather than a shared one: the SITES differ
/// (a tree has no beam cut and no continuation order), and `TieSite` is a closed
/// enum in a module the goldens pin — widening it here would move that module
/// for a reason that has nothing to do with it.
#[derive(Debug, Clone, Copy)]
pub struct McStream {
    key: u64,
}

impl McStream {
    /// Key this decision's draws. Callers take `key` from the agent's own play
    /// stream, once, before the world is sampled.
    pub fn new(key: u64) -> Self {
        Self { key }
    }

    /// The (deterministic) word for `site`.
    pub fn word(&self, site: McSite) -> u64 {
        let mut acc = self.key ^ SALT_MCTS_STREAM;
        for component in site.coords() {
            let mut step = acc ^ component;
            acc = splitmix64(&mut step);
        }
        acc
    }
}

// There is deliberately no `pick(site, len)` convenience here, unlike
// `lookahead::TieStream`: every selecting point in this module is a PURE
// function taking the word ([`ucb_select`], [`expansion_pick`],
// [`root_argmax`]), which is what lets each of them be tested over its full
// tied set without a tree — and a second way to spend a word would be a second
// place for a site's keying to drift.

/// Extend a path key by the edge index taken at this node.
///
/// [`crate::game::lookahead`]'s `descend_branch` in 64 bits: that one folds into
/// a `u32` and records the collision hazard as a known limitation, and there is
/// no reason to inherit the narrower coordinate in a module whose keys are all
/// `u64` anyway. [`splitmix64`] is a bijection on its state, so distinct edge
/// indices at one node always give distinct keys.
fn descend_branch(branch: u64, index: usize) -> u64 {
    let mut x = branch ^ (index as u64).wrapping_add(SALT_MCTS_BRANCH);
    splitmix64(&mut x)
}

/// The candidate-scheme seed for the node at `branch`.
///
/// Per NODE rather than per search: the shipped scheme draws (M retreat samples,
/// a placement reservoir), and a tree grows one node at a time, so a single
/// continuing stream would make a node's candidate list depend on how many
/// draws its siblings happened to take first. Keying on the path makes the list
/// a function of the node's position — which is also what lets the determinism
/// pin be a statement about the decision rather than about an expansion order.
fn node_candidate_seed(candidate_seed: u64, branch: u64) -> u64 {
    let mut x = candidate_seed ^ SALT_MCTS_CANDIDATES ^ branch;
    splitmix64(&mut x)
}

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

/// The DECLARED monotone squash of an [`eval::score`] relative margin — module
/// doc's formula, and the whole of UCB1's value axis.
///
/// **Range, stated exactly.** In exact arithmetic the map is strictly increasing
/// into the OPEN interval `(0, 1)`. In `f64` it SATURATES: once `|x|` exceeds
/// about `2^53 · scale` the quotient rounds to 1.0 and the map returns exactly
/// 0.0 or 1.0, so the honest range of this function is the CLOSED `[0, 1]` and
/// strict monotonicity holds only below that threshold (≈ 9·10^18 at the shipped
/// scale). Nothing organic reaches it — the positional evaluator's totals are
/// four-digit, and a terminal state is valued by [`rank_value`] before it can get
/// here — and no consumer needs the interval open: UCB1 reads a value, not a
/// probability. It is stated because "into (0,1)" would be a claim the tests can
/// falsify at the extremes.
///
/// `scale` is raised to 1: a zero or negative scale would make the map
/// non-monotone (or divide by zero at `x = 0`), and a knob floor stated once
/// beats a defence at each call site.
pub fn squash(value: i64, scale: i64) -> f64 {
    let scale = scale.max(1) as f64;
    let x = value as f64;
    0.5 * (1.0 + x / (x.abs() + scale))
}

/// A finished game's value for a seat placed `rank`-th of `players`, on the same
/// `[0, 1]` axis as [`squash`]: 1.0 for rank 1, 0.0 for last, evenly spaced
/// between.
///
/// The RANK comes from [`final_result`] and the tie-break inside it is never
/// re-implemented here (project law, and the reason this function takes a rank
/// rather than a state).
pub fn rank_value(rank: u8, players: usize) -> f64 {
    if players <= 1 {
        return 1.0;
    }
    let last = (players - 1) as f64;
    1.0 - (f64::from(rank.saturating_sub(1)) / last).clamp(0.0, 1.0)
}

/// [`rank_value`] of the root seat's standing in the finished `state`.
fn terminal_value(state: &GameState, rules: &Rules, seat: PlayerId) -> f64 {
    let result = final_result(state, rules);
    let players = result.standings.len();
    result
        .standings
        .iter()
        .find(|s| s.player == seat)
        .map_or(0.0, |s| rank_value(s.rank, players))
}

/// UCB1: `exploit + c · sqrt(ln(parent_visits) / child_visits)`.
///
/// `child_visits == 0` is `+∞` — an unvisited child outranks every visited one.
/// It cannot arise in the descent (a child exists only once it has been expanded
/// and backed up, so its visit count is ≥ 1), and is defined anyway because the
/// formula is meaningless without it and a caller assembling stats by hand
/// deserves the textbook answer rather than a division result.
pub fn ucb1(exploit: f64, child_visits: u32, parent_visits: u32, c: f64) -> f64 {
    if child_visits == 0 {
        return f64::INFINITY;
    }
    let n = f64::from(parent_visits.max(1));
    let ni = f64::from(child_visits);
    exploit + c * (n.ln() / ni).max(0.0).sqrt()
}

/// The value a node's ACTING seat maximizes, from a mean stored in the ROOT
/// seat's perspective.
///
/// `1 − v` off the root seat (module doc / spec §3.2): exact at 2P, and the
/// paranoid reading at 3–4P.
fn selection_value(node_seat: PlayerId, root_seat: PlayerId, mean: f64) -> f64 {
    if node_seat == root_seat {
        mean
    } else {
        1.0 - mean
    }
}

/// One child's backed-up statistics, as the selection reads them.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChildStat {
    visits: u32,
    /// Summed leaf values in the ROOT seat's perspective.
    value_sum: f64,
}

impl ChildStat {
    fn mean(&self) -> f64 {
        if self.visits == 0 {
            0.5
        } else {
            self.value_sum / f64::from(self.visits)
        }
    }
}

/// UCB1 selection over a fully-expanded node's children, ties drawn uniformly
/// from the keyed word — never first-listed (ledger row 18: at a Placement node
/// the first-listed child is the EMPTY placement, so a first-wins selection is
/// the null-placement degeneracy wearing a search).
fn ucb_select(
    children: &[ChildStat],
    parent_visits: u32,
    parent_seat: PlayerId,
    root_seat: PlayerId,
    c: f64,
    word: u64,
) -> usize {
    debug_assert!(!children.is_empty(), "a selection over an empty child set is undefined");
    let mut best = f64::NEG_INFINITY;
    let mut tied: Vec<usize> = Vec::new();
    for (i, child) in children.iter().enumerate() {
        let exploit = selection_value(parent_seat, root_seat, child.mean());
        let u = ucb1(exploit, child.visits, parent_visits, c);
        if u > best {
            best = u;
            tied.clear();
            tied.push(i);
        } else if u == best {
            tied.push(i);
        }
    }
    tied[uniform_index(word, tied.len().max(1))]
}

/// The expansion pick: a uniform keyed draw over the candidates that have no
/// child yet.
fn expansion_pick(unexpanded: &[usize], word: u64) -> usize {
    debug_assert!(!unexpanded.is_empty(), "an expansion over a full node is undefined");
    unexpanded[uniform_index(word, unexpanded.len().max(1))]
}

/// The decision: argmax over VISITS, ties drawn uniformly from the keyed word.
///
/// Visits rather than mean value — the standard UCT read-out, and the robust one
/// at small budgets, where a child visited once can carry an extreme mean.
fn root_argmax(visits: &[u32], word: u64) -> usize {
    debug_assert!(!visits.is_empty(), "a decision over an empty root is undefined");
    let best = visits.iter().copied().max().unwrap_or(0);
    let tied: Vec<usize> = visits
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (*v == best).then_some(i))
        .collect();
    tied[uniform_index(word, tied.len().max(1))]
}

// ---------------------------------------------------------------------------
// The search
// ---------------------------------------------------------------------------

/// What one search cost and what shape it had. Every field is a COUNT,
/// deterministic in `(world, weights, knobs, keys)`; nothing here is a timing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct McSearchStats {
    /// Root candidates this decision offered.
    pub root_candidates: u64,
    /// Simulations run. 0 at a forced decision point.
    pub simulations: u64,
    /// The ROOT node's own visit count after the search — 0 at a forced point,
    /// otherwise equal to `simulations` BY CONSTRUCTION (the root is on every
    /// backprop path). Surfaced because a backprop that skips the root is
    /// treant's D1 verbatim: [`ucb1`] floors `parent_visits` at 1, `ln 1 = 0`,
    /// and the exploration term goes silently inert while every test that does
    /// not read this field stays green (T2 review MUST-1's mutation).
    pub root_visits: u32,
    /// Nodes created, root included.
    pub nodes: u64,
    /// [`DecisionHandle`]s built. **Equal to `nodes − terminal_nodes`** by
    /// construction — a terminal node enumerates nothing. Counted separately
    /// anyway: the duplicate-handle hazard is precisely the two coming apart.
    pub handles: u64,
    /// Nodes that FORCED the routes memo and therefore paid one President-route
    /// DFS: a Movement decision point, no pending choice, President on board.
    pub route_dfs_nodes: u64,
    /// Nodes offering ≤ 1 candidate — the forced fast path (R4: 59.9% of 2P
    /// decision points, 84.6% of beam-tree nodes).
    pub forced_nodes: u64,
    /// Nodes standing at a finished game: valued by [`final_result`], never
    /// expanded, never handled.
    pub terminal_nodes: u64,
    /// Nodes whose ACTING seat is not the root seat — spec §3.2's opponent
    /// channel, counted so "the tree descends past the hand-off" is observable
    /// rather than asserted in prose.
    pub opponent_nodes: u64,
    /// Successors applied and valued — the currency spec §0.3 prices and
    /// [`crate::game::budget::ScoredBudget`] meters (one apply + observe + eval,
    /// or + [`final_result`] at a terminal). Metered here, GATED with the budget
    /// task.
    pub scored: u64,
    /// Nodes whose placement family exceeded the candidate cap.
    pub placement_cap_binds: u64,
    /// The deepest simulation path, in edges from the root.
    pub max_depth: u32,
}

/// One root candidate as the search left it.
#[derive(Debug, Clone, PartialEq)]
pub struct RootStat {
    pub mv: Move,
    pub visits: u32,
    /// Summed backed-up values, in the ROOT seat's perspective.
    pub value_sum: f64,
}

impl RootStat {
    /// The mean backed-up value, or `None` for a candidate no simulation ever
    /// reached — which is a real state of affairs at a 149-candidate Placement
    /// root under a small budget, and reporting 0.5 for it would invent a
    /// measurement.
    pub fn mean(&self) -> Option<f64> {
        (self.visits > 0).then(|| self.value_sum / f64::from(self.visits))
    }
}

/// One decision's search result.
#[derive(Debug, Clone)]
pub struct McSearch {
    /// Every root candidate, in the order the candidate scheme offered them.
    pub roots: Vec<RootStat>,
    /// The index in [`roots`](Self::roots) the search decided on.
    pub chosen: usize,
    /// L10's bundling: measured at the ROOT handle, the same one the root
    /// candidates came off, so the driver never needs a second enumeration.
    pub volume: VolumeObs,
    pub stats: McSearchStats,
}

/// Everything one search reads that is not the world itself.
#[derive(Debug, Clone, Copy)]
pub struct McCtx<'a> {
    pub rules: &'a Rules,
    pub weights: &'a WeightTable,
    /// `M`, spec §3.3's retreat sample count — the shipped scheme's knob,
    /// inherited verbatim.
    pub retreat_samples: u8,
    pub simulations: u32,
    pub exploration_ppm: u32,
    pub squash_scale: i64,
    pub leaf: LeafRule,
    pub stream: McStream,
    /// The per-decision candidate-dice seed; each node re-keys it by its path
    /// ([`node_candidate_seed`]).
    pub candidate_seed: u64,
}

impl McCtx<'_> {
    /// UCB1's `c`.
    fn exploration(&self) -> f64 {
        f64::from(self.exploration_ppm) / 1e6
    }
}

/// One tree node: spec §2.5's shape `(GameState, Vec<Move>, stats)`.
struct Node {
    state: GameState,
    /// The cache: computed exactly once, at creation, off this node's one
    /// handle. Empty at a terminal node, which has no decision to make.
    candidates: Vec<Move>,
    /// Child node index per candidate; `None` until that edge is expanded.
    children: Vec<Option<usize>>,
    /// How many of `children` are `Some` — the expansion site's coordinate, kept
    /// rather than recounted so the coordinate is O(1).
    expanded: u32,
    /// The seat to move here. Equal to `state.current_player`, hoisted because
    /// the selection reads it per child per descent.
    seat: PlayerId,
    /// This node's path key (module doc).
    branch: u64,
    visits: u32,
    /// Summed backed-up values, ROOT seat's perspective.
    value_sum: f64,
    /// This node's OWN value: the leaf value it was created with, or its
    /// terminal value. Read only when the node is the end of a simulation.
    value: f64,
    terminal: bool,
}

/// The mutable half of one search.
struct Search<'a> {
    ctx: &'a McCtx<'a>,
    /// The ROOT seat — every value in the tree is in this seat's perspective.
    seat: PlayerId,
    nodes: Vec<Node>,
    stats: McSearchStats,
}

/// Whether a node at `state` will pay one President-route DFS.
///
/// Derived from the STATE, not from the handle — [`crate::game::lookahead`]'s
/// `pays_route_dfs` and its reason: counted at the handle site, a duplicate
/// handle would inflate both sides of the counter pin and the pin would pass
/// through the defect it exists to catch.
///
/// The predicate mirrors `search_actions::families`' dispatch: a pending choice
/// short-circuits every phase to `ChoiceResolution`, Movement is the only phase
/// that emits `MovementRoutes`, and that family only SEARCHES with the President
/// on board (its in-hand arm emits `PlacePresident` and walks nothing).
fn pays_route_dfs(state: &GameState) -> bool {
    state.pending_choices.is_empty()
        && state.turn_phase == TurnPhase::Movement
        && matches!(state.current_player().president_location, PresidentLocation::OnBoard(_))
}

impl Search<'_> {
    /// Build the ONE [`DecisionHandle`] this node ever gets and read everything
    /// off it in a single pass.
    ///
    /// **The module's only `DecisionHandle::new` call site.** Volume (when
    /// asked) is taken FIRST, then the candidates, so both read one memo — the
    /// L10 bundling, and the reason a Movement node pays exactly one route DFS.
    fn expand_point(
        &mut self,
        state: &GameState,
        branch: u64,
        want_volume: bool,
    ) -> Result<(Vec<Move>, Option<VolumeObs>), String> {
        let handle = DecisionHandle::new(state, self.ctx.rules);
        self.stats.handles += 1;
        if handle.families().is_empty() {
            return Err(format!(
                "{MCTS_CONTROLLER}: seat {} was asked to move at {:?}, which offers no families \
                 at all",
                self.seat.0,
                handle.turn_phase(),
            ));
        }
        let volume = if want_volume { decision_volume(&handle) } else { None };
        let mut rng =
            ChaCha8Rng::seed_from_u64(node_candidate_seed(self.ctx.candidate_seed, branch));
        let Candidates { moves, placement_cap_bound } =
            candidates(&handle, self.ctx.retreat_samples, &mut rng);
        if placement_cap_bound {
            self.stats.placement_cap_binds += 1;
        }
        if moves.is_empty() {
            return Err(format!(
                "dead end: {MCTS_CONTROLLER} found no candidate at {:?} for seat {}",
                handle.turn_phase(),
                self.seat.0,
            ));
        }
        Ok((moves, volume))
    }

    /// Push an expandable node whose candidates are already in hand.
    fn push_expandable(
        &mut self,
        state: GameState,
        branch: u64,
        candidates: Vec<Move>,
        value: f64,
    ) -> usize {
        self.stats.nodes += 1;
        if pays_route_dfs(&state) {
            self.stats.route_dfs_nodes += 1;
        }
        if candidates.len() <= 1 {
            self.stats.forced_nodes += 1;
        }
        let seat = state.current_player;
        if seat != self.seat {
            self.stats.opponent_nodes += 1;
        }
        let children = vec![None; candidates.len()];
        self.nodes.push(Node {
            state,
            candidates,
            children,
            expanded: 0,
            seat,
            branch,
            visits: 0,
            value_sum: 0.0,
            value,
            terminal: false,
        });
        self.nodes.len() - 1
    }

    /// Create the node reached by an expansion: terminal (valued by
    /// [`final_result`], never handled) or expandable (its one handle, its
    /// cached candidates, its leaf value).
    fn push_child(&mut self, state: GameState, branch: u64) -> Result<usize, String> {
        if state.is_game_over() {
            let value = terminal_value(&state, self.ctx.rules, self.seat);
            self.stats.nodes += 1;
            self.stats.terminal_nodes += 1;
            let seat = state.current_player;
            if seat != self.seat {
                self.stats.opponent_nodes += 1;
            }
            self.nodes.push(Node {
                state,
                candidates: Vec::new(),
                children: Vec::new(),
                expanded: 0,
                seat,
                branch,
                visits: 0,
                value_sum: 0.0,
                value,
                terminal: true,
            });
            return Ok(self.nodes.len() - 1);
        }
        let value = self.leaf_value(&state)?;
        let (candidates, _) = self.expand_point(&state, branch, false)?;
        Ok(self.push_expandable(state, branch, candidates, value))
    }

    /// The value of an unexpanded node, per [`LeafRule`].
    ///
    /// **The seam the next task swaps.** Spec §2.1's default — complete the turn
    /// greedily and evaluate at the hand-off — arrives as a second arm here; the
    /// rest of the module reads a `f64` in `[0, 1]` and never learns which rule
    /// produced it.
    fn leaf_value(&self, state: &GameState) -> Result<f64, String> {
        match self.ctx.leaf {
            LeafRule::MidTurnDiag => {
                let view = observed_view(state, self.seat, MCTS_CONTROLLER)?;
                let raw = eval::score(&view, self.ctx.rules, self.ctx.weights);
                Ok(squash(raw, self.ctx.squash_scale))
            }
        }
    }

    /// Apply `mv` at `state`, hard-erroring the way spec §3.1 requires (the
    /// candidate came off this state's own families, so a rejection is a bug).
    fn apply(&self, state: &GameState, mv: &Move) -> Result<GameState, String> {
        apply_move_unchecked(state, mv, self.ctx.rules)
            .map_err(|e| format!("{MCTS_CONTROLLER}: candidate {mv:?} failed to apply: {e}"))
    }

    /// UCB1 over a fully-expanded node's children.
    fn select(&self, at: usize) -> usize {
        let node = &self.nodes[at];
        let stats: Vec<ChildStat> = node
            .children
            .iter()
            .map(|c| {
                let child = &self.nodes[c.expect("a fully-expanded node has every child")];
                ChildStat { visits: child.visits, value_sum: child.value_sum }
            })
            .collect();
        ucb_select(
            &stats,
            node.visits,
            node.seat,
            self.seat,
            self.ctx.exploration(),
            self.ctx.stream.word(McSite::Ucb { branch: node.branch, visit: node.visits }),
        )
    }

    /// ONE simulation: descend to an expandable node, expand one edge, value the
    /// new node, back the value up the path.
    ///
    /// **Termination needs no guard.** The descent only continues through
    /// FULLY-expanded nodes and every simulation creates exactly one node, so a
    /// path is bounded by the tree's size, which is bounded by the simulation
    /// count. (That is why a determinized world that cycles — the measured
    /// non-terminating band on the capped S1 arm — cannot hang this loop.)
    ///
    /// Both per-step `Vec`s below (the unexpanded slots, and [`select`]'s child
    /// stats) are `O(candidates)` and deliberately unoptimized: the step they
    /// sit beside pays a full candidate ENUMERATION, which the research base
    /// prices two orders of magnitude above a slot scan, and splitting either
    /// into an allocation-free variant would leave the tested pure function
    /// ([`expansion_pick`]) beside a second, untested one.
    fn simulate(&mut self) -> Result<(), String> {
        let mut path: Vec<usize> = vec![ROOT];
        let mut at = ROOT;
        let mut depth: u32 = 0;
        let value = loop {
            if self.nodes[at].terminal {
                break self.nodes[at].value;
            }
            let unexpanded: Vec<usize> = self.nodes[at]
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, c)| c.is_none().then_some(i))
                .collect();
            if !unexpanded.is_empty() {
                let (branch, filled) = (self.nodes[at].branch, self.nodes[at].expanded);
                let word = self.ctx.stream.word(McSite::Expansion { branch, filled });
                let slot = expansion_pick(&unexpanded, word);
                let mv = self.nodes[at].candidates[slot].clone();
                let next = self.apply(&self.nodes[at].state, &mv)?;
                self.stats.scored += 1;
                let child = self.push_child(next, descend_branch(branch, slot))?;
                self.nodes[at].children[slot] = Some(child);
                self.nodes[at].expanded += 1;
                path.push(child);
                depth += 1;
                break self.nodes[child].value;
            }
            let slot = self.select(at);
            at = self.nodes[at].children[slot].expect("a fully-expanded node has every child");
            path.push(at);
            depth += 1;
        };
        self.stats.max_depth = self.stats.max_depth.max(depth);
        for &n in &path {
            let node = &mut self.nodes[n];
            node.visits += 1;
            node.value_sum += value;
        }
        Ok(())
    }
}

/// Search one world: build the tree, run the simulations, decide.
///
/// `seat` must be the world's player to move — the agent is only ever asked at
/// its own decision point, and a tree rooted anywhere else would be searching
/// somebody else's decision.
///
/// **The forced fast path is at the top**: a decision point offering ≤ 1
/// candidate is answered by the root's single enumeration and NO simulation
/// runs. The root node is still materialized, so the counter identities hold
/// uniformly (`handles == nodes − terminal_nodes` whether or not a tree grew);
/// what the fast path skips is the tree, which is the cost.
pub fn search_world(
    world: &GameState,
    seat: PlayerId,
    ctx: &McCtx<'_>,
) -> Result<McSearch, String> {
    if world.current_player != seat {
        return Err(format!(
            "{MCTS_CONTROLLER}: asked to search seat {}'s move in a world where seat {} is to play",
            seat.0, world.current_player.0,
        ));
    }
    if world.is_game_over() {
        return Err(format!(
            "{MCTS_CONTROLLER}: asked to search seat {}'s move in a finished game",
            seat.0,
        ));
    }
    let mut search = Search { ctx, seat, nodes: Vec::new(), stats: McSearchStats::default() };

    // The ROOT's one handle: volume and candidates off the same memo.
    let (candidates, volume) = search.expand_point(world, 0, true)?;
    let volume =
        volume.ok_or_else(|| format!("{MCTS_CONTROLLER}: the root offered no volume"))?;
    search.stats.root_candidates = candidates.len() as u64;
    let forced = candidates.len() <= 1;
    // The root's OWN value is never read: `Node::value` is consulted only where a
    // simulation ENDS, and a simulation ends at the root only if the root is
    // terminal — refused above. Evaluating it anyway would buy nothing and would
    // charge the decision an observe+eval the budget does not price.
    let root = search.push_expandable(world.clone(), 0, candidates, 0.0);
    debug_assert_eq!(root, ROOT, "the root must be node 0");

    if !forced {
        // The knob floor, stated once: a decision over zero simulations is not a
        // search. (`squash` floors its own scale for the same reason.)
        for _ in 0..ctx.simulations.max(1) {
            search.simulate()?;
        }
        search.stats.simulations = u64::from(ctx.simulations.max(1));
    }

    search.stats.root_visits = search.nodes[ROOT].visits;
    let root_node = &search.nodes[ROOT];
    let roots: Vec<RootStat> = root_node
        .candidates
        .iter()
        .enumerate()
        .map(|(i, mv)| {
            let (visits, value_sum) = match root_node.children[i] {
                Some(c) => (search.nodes[c].visits, search.nodes[c].value_sum),
                None => (0, 0.0),
            };
            RootStat { mv: mv.clone(), visits, value_sum }
        })
        .collect();
    let visits: Vec<u32> = roots.iter().map(|r| r.visits).collect();
    let chosen = root_argmax(&visits, ctx.stream.word(McSite::RootArgmax));
    Ok(McSearch { roots, chosen, volume, stats: search.stats })
}

// ---------------------------------------------------------------------------
// The agent
// ---------------------------------------------------------------------------

/// MCTS-v1 as a seat: the honest chain, then the tree.
///
/// The arena's [`ArenaAgent`](crate::game::arena::ArenaAgent) impl, the
/// controller version const and the provenance descriptor land with the seam
/// task; [`choose`](Self::choose) and
/// [`choose_starting_order`](Self::choose_starting_order) already have the
/// signatures that impl will delegate to.
pub struct MctsAgent {
    worlds: u8,
    retreat_samples: u8,
    simulations: u32,
    exploration_ppm: u32,
    squash_scale: i64,
    leaf: LeafRule,
    weights: WeightTable,
    rng: ChaCha8Rng,
    /// Kept beside the stream it seeded because `ChaCha8Rng` will not hand its
    /// seed back and provenance has to record it.
    agent_seed: u64,
    /// Every Order card's Keep score, precomputed at construction —
    /// [`score_starting_order`], SHARED with the evaluator rungs rather than
    /// re-invented, which is what makes a cell against one of them
    /// Keep-homogeneous (spec §3.6's `KeepKind::EvaluatorScored`).
    keep_scores: Vec<(OrderCardId, i64)>,
    last_stats: Option<McSearchStats>,
    last_roots: Vec<RootStat>,
}

impl MctsAgent {
    /// MCTS-v1 at the shipped knobs. `agent_seed` is the PLAY stream — key it
    /// with [`agent_stream_seed`](crate::game::arena::agent_stream_seed).
    pub fn new(rules: &Rules, agent_seed: u64) -> Self {
        Self {
            worlds: DEFAULT_WORLDS,
            retreat_samples: DEFAULT_RETREAT_SAMPLES,
            simulations: DEFAULT_SIMULATIONS,
            exploration_ppm: DEFAULT_EXPLORATION_PPM,
            squash_scale: DEFAULT_SQUASH_SCALE,
            leaf: LeafRule::default(),
            weights: WeightTable::V1,
            rng: ChaCha8Rng::seed_from_u64(agent_seed),
            agent_seed,
            keep_scores: rules
                .orders
                .iter()
                .map(|card| (card.id, score_starting_order(card, rules)))
                .collect(),
            last_stats: None,
            last_roots: Vec::new(),
        }
    }

    /// Simulations per decision (floored at 1 by the search).
    pub fn with_simulations(mut self, simulations: u32) -> Self {
        self.simulations = simulations;
        self
    }

    /// UCB1's `c`, in ppm.
    pub fn with_exploration_ppm(mut self, ppm: u32) -> Self {
        self.exploration_ppm = ppm;
        self
    }

    /// The squash scale, in [`eval::score`] units (floored at 1 by [`squash`]).
    pub fn with_squash_scale(mut self, scale: i64) -> Self {
        self.squash_scale = scale;
        self
    }

    /// `M`, the shipped candidate scheme's retreat sample count.
    pub fn with_retreat_samples(mut self, m: u8) -> Self {
        self.retreat_samples = m;
        self
    }

    /// The leaf rule (spec §2.1).
    pub fn with_leaf(mut self, leaf: LeafRule) -> Self {
        self.leaf = leaf;
        self
    }

    /// `K`, sampled worlds per decision. Accepted here and REFUSED at
    /// [`choose`](Self::choose) for `K > 1` — the knob's legality must not
    /// depend on the order two setters were called in
    /// (`LookaheadAgent::with_cap`'s precedent).
    pub fn with_worlds(mut self, worlds: u8) -> Self {
        self.worlds = worlds;
        self
    }

    /// The weights this agent evaluates with. Spec §0.5 freezes
    /// [`WeightTable::V1`] for the whole experiment; the setter exists for
    /// fixtures and for whatever bumps the controller version later.
    pub fn with_weights(mut self, weights: WeightTable) -> Self {
        self.weights = weights;
        self
    }

    /// This agent's play-stream seed, as provenance has to record it.
    pub fn agent_seed(&self) -> u64 {
        self.agent_seed
    }

    // The knobs, one reader each rather than one tuple: the provenance
    // descriptor (seam task) names them individually in its param map, and a
    // positional tuple of `(u8, u8, u32, u32, i64, …)` is exactly the shape a
    // future edit silently reorders.

    /// `K`, sampled worlds per decision.
    pub fn worlds(&self) -> u8 {
        self.worlds
    }

    /// `M`, the candidate scheme's retreat sample count.
    pub fn retreat_samples(&self) -> u8 {
        self.retreat_samples
    }

    /// Simulations per decision.
    pub fn simulations(&self) -> u32 {
        self.simulations
    }

    /// UCB1's `c`, in ppm.
    pub fn exploration_ppm(&self) -> u32 {
        self.exploration_ppm
    }

    /// The squash scale, in [`eval::score`] units.
    pub fn squash_scale(&self) -> i64 {
        self.squash_scale
    }

    /// The leaf rule this agent values unexpanded nodes with.
    pub fn leaf(&self) -> LeafRule {
        self.leaf
    }

    /// The weights this agent evaluates with.
    pub fn weights(&self) -> &WeightTable {
        &self.weights
    }

    /// The last decision's counters.
    pub fn last_stats(&self) -> Option<&McSearchStats> {
        self.last_stats.as_ref()
    }

    /// EVERY root candidate of the last decision with its visits and backed-up
    /// value, in the order the candidate scheme offered them. The explanation
    /// surface (spec §3.5) renders from here rather than re-deriving.
    pub fn last_roots(&self) -> &[RootStat] {
        &self.last_roots
    }

    /// Argmax of [`score_starting_order`] over the two dealt cards, ties to
    /// index 0 — the SAME Keep the evaluator rungs use, so a cell against one of
    /// them is Keep-homogeneous and hence setup-identical.
    ///
    /// No stream draw: a unit's two seatings must resolve the same deal
    /// identically, which holds exactly when a seat's Keep is a deterministic
    /// function of `(seat, deal)` and not of which contestant sits there.
    pub fn choose_starting_order(&mut self, _seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        let score = |id: OrderCardId| {
            self.keep_scores.iter().find(|(k, _)| *k == id).map_or(0, |(_, v)| *v)
        };
        u8::from(score(deal[1]) > score(deal[0]))
    }

    /// One decision: the honest chain, then the tree.
    ///
    /// **Exactly three words of the agent's own play stream, in this order** —
    /// sampler seed, tie key, candidate seed — drawn BEFORE the world is
    /// sampled and independent of everything the search does inside, so the
    /// stream position after `n` decisions is `3n`
    /// (`LookaheadAgent::choose`'s property, held here for the same reason:
    /// a search whose stream consumption depends on its own shape cannot be
    /// compared across worlds or across weight tables).
    pub fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        let seat = view.viewer;
        if self.worlds > 1 {
            return Err(format!(
                "{MCTS_CONTROLLER}: K = {} is refused — aggregating a tree across sampled worlds \
                 is information-set search (spec §0.4's candidate (c)), which this cycle keeps \
                 out; the knob exists so the shape is stated, not so it can be turned",
                self.worlds,
            ));
        }
        // The sampler seed comes from THIS AGENT'S OWN stream and nothing else —
        // deriving it from anything state-flavoured would correlate the sample
        // with the truth it must be blind to.
        let sampler_seed: u64 = self.rng.gen();
        let mut sampler = BeliefSampler::from_view(view, rules, sampler_seed)
            .map_err(|e| format!("belief sampler refused seat {}'s view: {e}", seat.0))?;
        let tie_key = self.rng.next_u64();
        let candidate_seed = self.rng.next_u64();

        let ctx = McCtx {
            rules,
            weights: &self.weights,
            retreat_samples: self.retreat_samples,
            simulations: self.simulations,
            exploration_ppm: self.exploration_ppm,
            squash_scale: self.squash_scale,
            leaf: self.leaf,
            stream: McStream::new(tie_key),
            candidate_seed,
        };
        let world = sampler.sample();
        let searched = search_world(&world, seat, &ctx)?;

        let mv = searched.roots[searched.chosen].mv.clone();
        self.last_stats = Some(searched.stats);
        self.last_roots = searched.roots;
        Ok(DecisionReport { mv, volume: searched.volume })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::ArenaAgent;
    use crate::game::greedy::GreedyAgent;
    use crate::game::observe::observe;
    use crate::game::route_enum::{route_dfs_count, route_dfs_reset};
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::GameConfig;

    /// The committed diagnostic master for this file's driven fixtures.
    /// Diagnostic BY NAME (spec §3's gate-cell exemption): a per-edit pin needs a
    /// fixed master, and these are pins rather than bot-facing evaluation runs.
    const MASTER: u64 = 0x1EA4_2026_0727_0001;

    fn ctx<'a>(
        rules: &'a Rules,
        weights: &'a WeightTable,
        simulations: u32,
        key: u64,
    ) -> McCtx<'a> {
        McCtx {
            rules,
            weights,
            retreat_samples: DEFAULT_RETREAT_SAMPLES,
            simulations,
            exploration_ppm: DEFAULT_EXPLORATION_PPM,
            squash_scale: DEFAULT_SQUASH_SCALE,
            leaf: LeafRule::MidTurnDiag,
            stream: McStream::new(key),
            candidate_seed: 0xC0FFEE,
        }
    }

    /// T2-review MUST-1: the root absorbs every simulation's backprop. A backprop
    /// that skips the root is treant's D1 verbatim — root visits freeze at 0,
    /// `ln 1 = 0`, and `c` goes silently inert while every other test stays
    /// green. Mutation-proven at the fix wave: `path.iter().skip(1)` in the
    /// backprop loop fails exactly this assertion.
    #[test]
    fn the_root_absorbs_every_simulations_backprop() {
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        let (world, seat) = placement_point(&rules);
        let s = search_world(&world, seat, &ctx(&rules, &weights, 64, 7)).expect("search");
        assert!(s.stats.simulations > 0, "the fixture must actually simulate");
        assert_eq!(
            u64::from(s.stats.root_visits),
            s.stats.simulations,
            "every simulation must pass through the root exactly once"
        );
    }

    /// T2-review MUST-2, half one: every backed-up root mean sits on the
    /// DECLARED unit axis. Backing up raw eval scores instead of the squash —
    /// the R5 foot-gun, spec §2.4's subject — sails through every other test
    /// and fails here (means in the hundreds).
    #[test]
    fn the_backed_up_values_sit_on_the_declared_unit_axis() {
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        let (world, seat) = placement_point(&rules);
        let s = search_world(&world, seat, &ctx(&rules, &weights, 64, 11)).expect("search");
        let mut visited = 0;
        for r in &s.roots {
            if let Some(m) = r.mean() {
                visited += 1;
                assert!(
                    (0.0..=1.0).contains(&m),
                    "a backed-up mean left the declared unit axis: {m}"
                );
            }
        }
        assert!(visited > 0, "the fixture must back up at least one value");
    }

    /// T2-review MUST-2, half two: the exploration constant genuinely moves the
    /// visit distribution. Catches BOTH review mutations — a root frozen at 0
    /// visits and a raw-scale value axis each make `c` inert, and an inert `c`
    /// cannot move the visits.
    #[test]
    fn the_exploration_constant_moves_the_visit_distribution() {
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        // A SMALL-family point, deliberately: while unexpanded slots remain the
        // simulate loop only expansion-picks (uniform, c-blind), so a root with
        // more candidates than simulations could never let UCB engage and this
        // test would be vacuous at any c.
        let (world, seat) = on_board_movement_point(&rules);
        let greedy_c = {
            let mut c = ctx(&rules, &weights, 64, 13);
            c.exploration_ppm = 0;
            search_world(&world, seat, &c).expect("c=0 search")
        };
        assert!(
            greedy_c.stats.root_candidates > 1 && greedy_c.stats.root_candidates < 64,
            "premise: the fixture must offer 2..63 root candidates so UCB engages \
             (got {})",
            greedy_c.stats.root_candidates
        );
        let explore_c = {
            let mut c = ctx(&rules, &weights, 64, 13);
            c.exploration_ppm = 5_000_000;
            search_world(&world, seat, &c).expect("c=5 search")
        };
        let v0: Vec<u32> = greedy_c.roots.iter().map(|r| r.visits).collect();
        let v5: Vec<u32> = explore_c.roots.iter().map(|r| r.visits).collect();
        assert_ne!(v0, v5, "c moved from 0 to 5.0 and the visit distribution did not move");
    }

    /// T2-review SHOULD-3: a site's word is keyed by its OWN coordinates —
    /// distinct visits, slots, branches and site families draw distinct words.
    /// Collapsing the coordinate arms to a shared word sequence (the defect the
    /// module-doc table promises against) cannot reproduce these inequalities.
    #[test]
    fn a_sites_word_is_keyed_by_its_own_coordinates() {
        let s = McStream::new(0xD00D);
        assert_ne!(
            s.word(McSite::Ucb { branch: 9, visit: 2 }),
            s.word(McSite::Ucb { branch: 9, visit: 3 }),
            "the visit coordinate must move the word"
        );
        assert_ne!(
            s.word(McSite::Ucb { branch: 9, visit: 2 }),
            s.word(McSite::Ucb { branch: 8, visit: 2 }),
            "the branch coordinate must move the word"
        );
        assert_ne!(
            s.word(McSite::Expansion { branch: 9, filled: 0 }),
            s.word(McSite::Expansion { branch: 9, filled: 1 }),
            "the filled coordinate must move the word"
        );
        assert_ne!(
            s.word(McSite::Expansion { branch: 9, filled: 2 }),
            s.word(McSite::Ucb { branch: 9, visit: 2 }),
            "the site families must stay disjoint at coinciding coordinates"
        );
        assert_ne!(
            s.word(McSite::RootArgmax),
            s.word(McSite::Ucb { branch: 0, visit: 0 }),
            "the root argmax must not share the zero-coordinate UCB word"
        );
    }

    /// T2-review SHOULD-4: a decision consumes EXACTLY three play-stream words —
    /// sampler seed, tie key, candidate seed, in that order — so decision n is a
    /// pure function of words 3n+1..3n+3. The mirror reconstructs the agent's
    /// SECOND decision from words 4..6 alone; a fourth word anywhere in `choose`
    /// desynchronizes it.
    #[test]
    fn a_decision_is_a_function_of_its_three_stream_words() {
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        let (w1, s1) = placement_point(&rules);
        let (w2, s2) = on_board_movement_point(&rules);
        let v1 = observe(&w1, s1).expect("view 1");
        let v2 = observe(&w2, s2).expect("view 2");

        let seed = 0xFAB1E;
        let mut agent = MctsAgent::new(&rules, seed).with_simulations(16);
        let _ = agent.choose(&v1, &rules).expect("decision 1");
        let second = agent.choose(&v2, &rules).expect("decision 2");

        let mut mirror = ChaCha8Rng::seed_from_u64(seed);
        for _ in 0..3 {
            mirror.next_u64();
        }
        let sampler_seed = mirror.next_u64();
        let tie_key = mirror.next_u64();
        let candidate_seed = mirror.next_u64();
        let mut sampler =
            BeliefSampler::from_view(&v2, &rules, sampler_seed).expect("mirror sampler");
        let world = sampler.sample();
        let mc = McCtx {
            rules: &rules,
            weights: &weights,
            retreat_samples: DEFAULT_RETREAT_SAMPLES,
            simulations: 16,
            exploration_ppm: DEFAULT_EXPLORATION_PPM,
            squash_scale: DEFAULT_SQUASH_SCALE,
            leaf: LeafRule::MidTurnDiag,
            stream: McStream::new(tie_key),
            candidate_seed,
        };
        let mirrored = search_world(&world, s2, &mc).expect("mirror search");
        assert_eq!(
            mirrored.roots[mirrored.chosen].mv, second.mv,
            "decision 2 must be a pure function of stream words 4..6"
        );
    }

    /// Play a fresh 2P game under Greedy-v1 until `want` holds, then hand back a
    /// SAMPLED world at that decision point plus the seat to move — the same
    /// shape the agent itself searches in.
    fn point_where(
        rules: &Rules,
        seed: u64,
        want: impl Fn(&GameState) -> bool,
    ) -> (GameState, PlayerId) {
        let pending = new_game(GameConfig::for_player_count(2), rules, seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut agent = GreedyAgent::new(rules, 0x77);
        for _ in 0..400 {
            if want(&state) {
                break;
            }
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let r = agent.choose(&view, rules).expect("move");
            state = apply_move_unchecked(&state, &r.mv, rules).expect("apply");
        }
        assert!(want(&state), "the walk never reached the wanted decision point");
        let seat = state.current_player;
        let view = observe(&state, seat).expect("observe");
        let world = BeliefSampler::from_view(&view, rules, 1).expect("sampler").sample();
        (world, seat)
    }

    fn placement_point(rules: &Rules) -> (GameState, PlayerId) {
        point_where(rules, MASTER, |s| {
            s.turn_phase == TurnPhase::Placement && s.pending_choices.is_empty()
        })
    }

    /// A Movement point with the President ON BOARD — the only shape that pays a
    /// route DFS, and therefore the only shape the L10 counter pin can be
    /// non-vacuous at.
    fn on_board_movement_point(rules: &Rules) -> (GameState, PlayerId) {
        point_where(rules, MASTER, |s| {
            s.turn_phase == TurnPhase::Movement
                && s.pending_choices.is_empty()
                && matches!(s.current_player().president_location, PresidentLocation::OnBoard(_))
        })
    }

    /// A PLACEMENT point whose President is already on board, so the Movement
    /// node that pays the route DFS sits INSIDE the tree rather than at the root.
    fn on_board_placement_point(rules: &Rules) -> (GameState, PlayerId) {
        point_where(rules, MASTER, |s| {
            s.turn_phase == TurnPhase::Placement
                && s.pending_choices.is_empty()
                && matches!(s.current_player().president_location, PresidentLocation::OnBoard(_))
        })
    }

    /// The first `n` decisions `agent` makes driving BOTH seats of a fresh 2P
    /// game at `world_seed`, as debug strings, plus the counters it paid.
    fn first_decisions(
        agent: &mut MctsAgent,
        world_seed: u64,
        n: usize,
        rules: &Rules,
    ) -> (Vec<String>, Vec<McSearchStats>) {
        let pending =
            new_game(GameConfig::for_player_count(2), rules, world_seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut moves = Vec::new();
        let mut stats = Vec::new();
        while moves.len() < n {
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let report = agent.choose(&view, rules).expect("the honest chain produces a move");
            moves.push(format!("{:?}", report.mv));
            stats.push(*agent.last_stats().expect("a decision records its counters"));
            state = apply_move_unchecked(&state, &report.mv, rules).expect("a chosen move applies");
        }
        (moves, stats)
    }

    // -- values: the declared normalization (spec §2.4) -----------------------

    /// The squash is the map the module doc names, and it is STRICTLY monotone
    /// into the open unit interval — the property UCB1's `c` is meaningless
    /// without (R5's measured foot-gun: a raw-scale mean makes `c` inert).
    #[test]
    fn the_squash_is_the_declared_monotone_map_into_the_unit_interval() {
        let scale = 1_000;
        assert_eq!(squash(0, scale), 0.5, "the map is centred at a zero margin");
        assert_eq!(squash(scale, scale), 0.75, "`scale` IS the 0.75 point, by declaration");
        assert_eq!(squash(-scale, scale), 0.25, "and symmetric about it");

        let mut previous = f64::NEG_INFINITY;
        for x in (-20_000..=20_000).step_by(37) {
            let v = squash(x, scale);
            assert!(v > 0.0 && v < 1.0, "the map must stay inside (0,1): squash({x}) = {v}");
            assert!(v > previous, "the map must be strictly increasing at x = {x}");
            previous = v;
        }
        // Totality at the extremes: the evaluator saturates rather than
        // overflowing, so the squash must answer for what it can be handed —
        // and what it answers there is the CLOSED interval, because `f64`
        // rounds `x/(|x|+scale)` to 1.0 long before `i64` runs out (the
        // function doc states the threshold). Both facts are pinned so neither
        // can be quietly re-stated as the other.
        assert_eq!(squash(i64::MAX, scale), 1.0, "the f64 map saturates rather than nearing 1");
        assert_eq!(squash(i64::MIN, scale), 0.0);
        assert!(squash(1 << 40, scale) < 1.0, "and everything organic stays strictly inside");
        assert!(squash(-(1 << 40), scale) > 0.0);
        // A degenerate scale is FLOORED, not divided by.
        assert_eq!(squash(0, 0), 0.5);
        assert!(squash(1, 0) > 0.5, "a floored scale still gives a monotone map");
    }

    /// A terminal is valued off the official ranking's RANK, on the same axis as
    /// the squash: 1.0 for a win, 0.0 for last.
    #[test]
    fn a_terminal_is_valued_by_its_place_in_the_official_ranking() {
        assert_eq!(rank_value(1, 2), 1.0);
        assert_eq!(rank_value(2, 2), 0.0);
        assert_eq!(rank_value(1, 4), 1.0);
        assert_eq!(rank_value(2, 4), 1.0 - 1.0 / 3.0);
        assert_eq!(rank_value(4, 4), 0.0);
        assert_eq!(rank_value(1, 1), 1.0, "a one-player standing is a win");
        // Ranks are 1..=n and never shared (`final_result`'s contract), so
        // anything outside that is a caller bug — answer inside the axis rather
        // than panicking.
        assert_eq!(rank_value(9, 4), 0.0);
        assert_eq!(rank_value(0, 4), 1.0);
    }

    // -- UCB1 (spec §3.2) -----------------------------------------------------

    /// The textbook formula on CONSTRUCTED stats — no tree, no game.
    #[test]
    fn ucb1_is_the_textbook_formula_on_constructed_stats() {
        // exploit + c·sqrt(ln N / n)
        let got = ucb1(0.5, 1, 4, 1.0);
        let want = 0.5 + (4.0f64.ln() / 1.0).sqrt();
        assert!((got - want).abs() < 1e-12, "{got} vs {want}");

        let got = ucb1(0.25, 9, 100, std::f64::consts::SQRT_2);
        let want = 0.25 + std::f64::consts::SQRT_2 * (100.0f64.ln() / 9.0).sqrt();
        assert!((got - want).abs() < 1e-12, "{got} vs {want}");

        assert_eq!(ucb1(0.7, 0, 10, 1.0), f64::INFINITY, "an unvisited child outranks every other");
        assert_eq!(ucb1(0.7, 3, 1, 1.0), 0.7, "ln(1) = 0: a once-visited parent is pure exploit");
        // c = 0 is pure exploitation, whatever the visit counts.
        assert_eq!(ucb1(0.3, 1, 1_000, 0.0), 0.3);
        // More visits at equal exploit ⇒ less exploration bonus.
        assert!(ucb1(0.5, 2, 100, 1.0) > ucb1(0.5, 20, 100, 1.0));
    }

    /// **THE OPPONENT-BACKUP CONVENTION, on a constructed case.** Values are
    /// stored in the ROOT seat's perspective everywhere; a node whose acting seat
    /// is the opponent must therefore select the child that is WORST for the root
    /// — `1 − v` (spec §3.2).
    #[test]
    fn an_opponent_node_selects_the_child_that_is_worst_for_the_root_seat() {
        let root = PlayerId(0);
        let opponent = PlayerId(1);
        // Equal visits, so the exploration term is identical and only the
        // perspective flip can move the pick.
        let children = [
            ChildStat { visits: 4, value_sum: 4.0 * 0.9 },
            ChildStat { visits: 4, value_sum: 4.0 * 0.1 },
        ];
        assert_eq!(
            ucb_select(&children, 8, root, root, std::f64::consts::SQRT_2, 0),
            0,
            "the root seat takes the child that is best for it",
        );
        assert_eq!(
            ucb_select(&children, 8, opponent, root, std::f64::consts::SQRT_2, 0),
            1,
            "the opponent takes the child that is worst for the root seat",
        );
        // And the convention itself, at the arithmetic level.
        assert_eq!(selection_value(root, root, 0.9), 0.9);
        assert_eq!(selection_value(opponent, root, 0.9), 1.0 - 0.9);
    }

    // -- every selecting point draws over its FULL tied set (row 18) ----------

    /// The UCB tie draw reaches every tied child — a first-listed fallback would
    /// return index 0 for every word.
    #[test]
    fn the_ucb_tie_draw_reaches_the_whole_tied_set() {
        let seat = PlayerId(0);
        let children: Vec<ChildStat> =
            (0..6).map(|_| ChildStat { visits: 3, value_sum: 1.5 }).collect();
        let stream = McStream::new(0xB1A5);
        let mut seen = std::collections::BTreeSet::new();
        for visit in 0..400u32 {
            let word = stream.word(McSite::Ucb { branch: 0x51DE, visit });
            seen.insert(ucb_select(&children, 12, seat, seat, std::f64::consts::SQRT_2, word));
        }
        assert_eq!(seen.len(), 6, "every tied child must be reachable, saw {seen:?}");
    }

    /// The expansion pick reaches every unexpanded slot.
    #[test]
    fn the_expansion_pick_reaches_the_whole_unexpanded_set() {
        let unexpanded = [2usize, 5, 7, 11, 13];
        let stream = McStream::new(0x5EED);
        let mut seen = std::collections::BTreeSet::new();
        for filled in 0..400u32 {
            let word = stream.word(McSite::Expansion { branch: 7, filled });
            seen.insert(expansion_pick(&unexpanded, word));
        }
        assert_eq!(
            seen, unexpanded.iter().copied().collect(),
            "every unexpanded candidate must be reachable, saw {seen:?}",
        );
    }

    /// The root argmax draws over the whole MAX-VISIT set, and never over a
    /// candidate below it.
    #[test]
    fn the_root_argmax_draw_reaches_the_whole_tied_set() {
        let visits = [3u32, 7, 7, 1, 7, 7];
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..400u64 {
            let stream = McStream::new(k.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let i = root_argmax(&visits, stream.word(McSite::RootArgmax));
            assert_eq!(visits[i], 7, "a non-maximiser must never be chosen");
            seen.insert(i);
        }
        assert_eq!(seen.len(), 4, "all four maximisers must be reachable, saw {seen:?}");
    }

    /// And the draw reaches the whole tied set THROUGH the search too: at a real
    /// decision point, varying only the tie key must move the chosen move.
    #[test]
    fn the_tie_draw_reaches_more_than_one_move_through_the_search() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..24u64 {
            let c = ctx(&rules, &WeightTable::V1, 24, k.wrapping_mul(0x2545_F491_4F6C_DD1D));
            let searched = search_world(&world, seat, &c).expect("search");
            seen.insert(format!("{:?}", searched.roots[searched.chosen].mv));
        }
        assert!(
            seen.len() > 1,
            "the keyed draw must be able to reach more than one move, saw {} of them",
            seen.len(),
        );
    }

    // -- the tree's shape (spec §2.5, ledger L10) -----------------------------

    /// **THE L10 PIN.** One [`DecisionHandle`] per node, and exactly one
    /// construction site in the whole module.
    #[test]
    fn the_module_builds_exactly_one_decision_handle_per_node() {
        let src = include_str!("mcts.rs");
        let body = src.split("#[cfg(test)]").next().expect("a non-test half");
        assert_eq!(
            body.matches("DecisionHandle::new(").count(),
            1,
            "the search must have exactly one handle construction site",
        );

        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 64, 0xB1A5);
        let s = search_world(&world, seat, &c).expect("search").stats;
        assert!(s.nodes > 1, "anti-vacuity: the search must have expanded ({} nodes)", s.nodes);
        assert_eq!(
            s.handles,
            s.nodes - s.terminal_nodes,
            "one handle per expandable node, no more — a terminal node enumerates nothing",
        );
        assert_eq!(s.simulations, 64, "the search ran the simulations it was asked for");
        assert_eq!(
            s.scored,
            s.nodes - 1,
            "exactly one scored successor per created node, the root excepted (it was not \
             applied) — the identity the budget meter will be gated on",
        );
        assert!(
            s.nodes - 1 <= s.simulations,
            "a simulation creates at most one node (none when it ends at a terminal)",
        );
    }

    /// **THE ROUTE-DFS COUNTER PIN, harness-scoped.**
    ///
    /// `route DFS count == cache-miss on-board-Movement NODES`, and NOT "nodes
    /// created": every non-Movement node and every IN-HAND Movement node forces
    /// no routes memo and pays 0.
    ///
    /// HARNESS-SCOPED, never armed globally: it measures one `search_world` call
    /// with nothing else on the thread. `route_enum`'s doc forbids arming the
    /// counter against a driver, which legitimately pays more in debug.
    ///
    /// BOTH root shapes, for `lookahead`'s measured reason: a turn passes through
    /// Movement exactly once, so at a Movement root the only DFS-paying node is
    /// the root itself, and a cell with only that reading would pass a duplicate
    /// handle inside the tree untouched.
    #[test]
    fn the_route_dfs_count_matches_the_cache_miss_on_board_movement_nodes() {
        let rules = Rules::standard();
        let rules_ref = &rules;
        let cell = |world: &GameState, seat: PlayerId, label: &str| {
            let c = ctx(rules_ref, &WeightTable::V1, 64, 0xB1A5);
            route_dfs_reset();
            let searched = search_world(world, seat, &c).expect("search");
            let counted = route_dfs_count();
            assert!(
                searched.stats.route_dfs_nodes > 0,
                "anti-vacuity ({label}): the cell must contain on-board Movement nodes",
            );
            assert_eq!(
                counted, searched.stats.route_dfs_nodes,
                "{label}: one President-route DFS per cache-miss on-board-Movement node",
            );
            assert!(
                searched.stats.nodes > searched.stats.route_dfs_nodes,
                "anti-vacuity ({label}): not every node may be a Movement node",
            );
        };
        let (world, seat) = on_board_movement_point(&rules);
        cell(&world, seat, "movement root");
        let (world, seat) = on_board_placement_point(&rules);
        cell(&world, seat, "placement root, DFS inside the tree");
    }

    /// The root is IN the backprop path, so its visit count IS the simulation
    /// count — R5's D1: a root whose visits never increment freezes every
    /// visit-driven schedule at `f(0)` and fails silently.
    #[test]
    fn the_root_is_in_the_backprop_path() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 40, 0x0D1);
        let searched = search_world(&world, seat, &c).expect("search");
        let root_visits: u32 = searched.roots.iter().map(|r| r.visits).sum();
        assert_eq!(
            u64::from(root_visits),
            searched.stats.simulations,
            "every simulation must pass through exactly one root child",
        );
        assert!(
            searched.roots.iter().filter(|r| r.visits > 0).count() > 1,
            "anti-vacuity: more than one root child must have been tried",
        );
    }

    /// A 0/1-candidate decision point never builds a tree: the root's single
    /// enumeration answers it and NO simulation runs.
    #[test]
    fn a_forced_decision_point_never_builds_a_tree() {
        let rules = Rules::standard();
        // A point the SHIPPED candidate scheme answers with exactly one move —
        // asked of the scheme rather than of the family sizes, because the
        // scheme is what the search enumerates.
        let is_forced = |s: &GameState| {
            let handle = DecisionHandle::new(s, &rules);
            !handle.families().is_empty()
                && candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut ChaCha8Rng::seed_from_u64(1))
                    .moves
                    .len()
                    == 1
        };
        let (world, seat) = point_where(&rules, MASTER, is_forced);
        let c = ctx(&rules, &WeightTable::V1, 64, 0xFEED);
        let searched = search_world(&world, seat, &c).expect("search");
        assert_eq!(searched.roots.len(), 1, "the fixture must be a forced point");
        assert_eq!(searched.stats.simulations, 0, "a forced point runs no simulation");
        assert_eq!(searched.stats.nodes, 1, "and grows no tree beyond its root");
        assert_eq!(searched.stats.handles, 1, "one enumeration answered the whole decision");
        assert_eq!(searched.stats.scored, 0, "and no successor was scored");
        assert_eq!(searched.chosen, 0, "the one candidate is the decision");
    }

    // -- determinism (the keyed streams) --------------------------------------

    /// **THE DETERMINISM PIN.** The same master takes the same decision and pays
    /// the same counters — at a fixture, and over a short driven self-play where
    /// the tree's shape, the sampler and the candidate dice all compound.
    #[test]
    fn the_same_master_takes_the_same_decision_and_pays_the_same_counters() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 64, 0xB1A5);
        let a = search_world(&world, seat, &c).expect("search");
        let b = search_world(&world, seat, &c).expect("search");
        assert_eq!(
            format!("{:?}", a.roots[a.chosen].mv),
            format!("{:?}", b.roots[b.chosen].mv),
            "the same key must take the same decision",
        );
        assert_eq!(a.stats, b.stats, "and pay the same counters");
        let visits_a: Vec<u32> = a.roots.iter().map(|r| r.visits).collect();
        let visits_b: Vec<u32> = b.roots.iter().map(|r| r.visits).collect();
        assert_eq!(visits_a, visits_b, "and build the same visit distribution");

        // Driven: one agent for both seats, eight decisions, twice.
        let mut first = MctsAgent::new(&rules, 0xA6E7).with_simulations(24);
        let mut second = MctsAgent::new(&rules, 0xA6E7).with_simulations(24);
        let (moves_a, stats_a) = first_decisions(&mut first, MASTER, 8, &rules);
        let (moves_b, stats_b) = first_decisions(&mut second, MASTER, 8, &rules);
        assert_eq!(moves_a, moves_b, "a driven game must reproduce move for move");
        assert_eq!(stats_a, stats_b, "and counter for counter");
        assert!(
            stats_a.iter().any(|s| s.simulations > 0),
            "anti-vacuity: some decision must have searched",
        );

        // A DIFFERENT agent seed must be able to move something — otherwise the
        // pin above would hold for a search that ignores its keys entirely.
        let mut other = MctsAgent::new(&rules, 0x5E11).with_simulations(24);
        let (moves_c, _) = first_decisions(&mut other, MASTER, 8, &rules);
        assert_ne!(moves_a, moves_c, "anti-vacuity: the agent seed must reach the decisions");
    }

    /// The path key is what makes a node's draws independent of when it was
    /// created: distinct edges give distinct keys, and the fold is a pure
    /// function of the path.
    #[test]
    fn the_path_key_is_a_pure_function_of_the_path() {
        let a = descend_branch(descend_branch(0, 3), 7);
        let b = descend_branch(descend_branch(0, 3), 7);
        assert_eq!(a, b, "the same path must key the same node");
        assert_ne!(a, descend_branch(descend_branch(0, 7), 3), "order must matter");
        let siblings: std::collections::BTreeSet<u64> =
            (0..64).map(|i| descend_branch(0x51DE, i)).collect();
        assert_eq!(siblings.len(), 64, "distinct edges must give distinct keys");
        assert_ne!(
            node_candidate_seed(0xC0FFEE, a),
            node_candidate_seed(0xC0FFEE, b ^ 1),
            "and the candidate dice must follow the key",
        );
    }

    // -- the honest chain (spec §3.1's no-fallback rule) ----------------------

    /// A view the sampler refuses is a typed hard error with no fallback arm,
    /// and it leaves no counters behind.
    #[test]
    fn a_view_the_sampler_refuses_is_a_hard_error_with_no_fallback() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mut doctored = observe(&world, seat).expect("observe");
        doctored.order_deck_len += 1;

        let mut agent = MctsAgent::new(&rules, 0xBADF00D).with_simulations(8);
        let err = agent.choose(&doctored, &rules).expect_err("the sampler must refuse");
        assert!(err.contains("belief sampler refused"), "the refusal must be named: {err}");
        assert!(
            agent.last_stats().is_none() && agent.last_roots().is_empty(),
            "a refused decision must not leave a search behind",
        );
        let clean = observe(&world, seat).expect("observe");
        assert!(agent.choose(&clean, &rules).is_ok(), "the undoctored view must be answerable");
    }

    /// `K > 1` is refused and the refusal names where the question went
    /// (spec §0.4).
    #[test]
    fn more_than_one_sampled_world_is_refused_and_routed_to_the_spec() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");
        let mut agent = MctsAgent::new(&rules, 0x2).with_simulations(8).with_worlds(2);
        let err = agent.choose(&view, &rules).expect_err("K > 1 must be refused");
        assert!(err.contains("K = 2") && err.contains("§0.4"), "the routing must be named: {err}");
        assert!(
            MctsAgent::new(&rules, 0x2)
                .with_simulations(8)
                .with_worlds(1)
                .choose(&view, &rules)
                .is_ok(),
            "K = 1 is the shipped shape and must answer",
        );
    }

    /// A search rooted at the wrong seat is refused rather than answered — the
    /// agent is only ever asked at its own decision point.
    #[test]
    fn a_search_rooted_at_the_wrong_seat_is_refused() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let other = PlayerId(1 - seat.0);
        let c = ctx(&rules, &WeightTable::V1, 8, 0x1);
        let err = search_world(&world, other, &c).expect_err("the wrong seat must be refused");
        assert!(err.contains("is to play"), "the refusal must name the mismatch: {err}");
    }

    /// The tree descends past the hand-off, which is the channel spec §3.2
    /// states and §7's world-sensitivity expectation follows from — an
    /// observation, made observable rather than asserted in prose.
    #[test]
    fn the_tree_descends_past_the_hand_off_into_the_opponents_options() {
        let rules = Rules::standard();
        let (world, seat) = on_board_movement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 96, 0x0FF);
        let searched = search_world(&world, seat, &c).expect("search");
        assert!(
            searched.stats.opponent_nodes > 0,
            "the tree must reach at least one node where the opponent is to move ({} nodes, \
             max depth {})",
            searched.stats.nodes,
            searched.stats.max_depth,
        );
    }
}


