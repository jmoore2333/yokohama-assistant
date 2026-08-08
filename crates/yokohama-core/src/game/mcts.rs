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
//! # THE HONESTY CONTRACT — the five things that break it (spec §7, R3 §5)
//!
//! Everything this module computes is a function of `(view, sampler seed, agent
//! stream position)` and of nothing else. The tree applies moves, refills ports
//! and completes turns INSIDE the sampled world, and every one of those states
//! is the bot's imagination: the world is a determinization of what the seat can
//! see, so reading it as deeply as the search likes is not a leak. That argument
//! survives exactly five edits, and they are named here so a future change is
//! checked against a list rather than against a feeling:
//!
//! 1. **A true-state read.** Any path that reaches a `&GameState` the sampler
//!    did not produce — a `choose(&GameState, …)` signature, a `&Rules` carrying
//!    state, a back-door accessor. The current signature is the guarantee; the
//!    tripwire that notices it being traded away is
//!    `t6_mcts_v1s_decision_never_moves_under_hidden_variation`.
//! 2. **A cache keyed by true-state identity.** A transposition table, a memo,
//!    or any reuse across decisions keyed by something that identifies the REAL
//!    game rather than this decision's world. (Spec §12 keeps a TT arm out of
//!    this cycle; if one ever lands, its key is the honesty question.)
//! 3. **An OS-entropy or wall-clock budget.** [`ScoredBudget`] is a deterministic
//!    count of scored successors precisely so a decision is reproducible from
//!    its master; a time-sliced or entropy-seeded budget makes the search a
//!    function of the machine, which no record can carry and no cell can
//!    reproduce.
//! 4. **`K > 1` aggregation.** Refused at [`MctsAgent::choose`] and NOT an
//!    oversight: aggregating one tree across sampled worlds is information-set
//!    search (spec §0.4's candidate (c)), which is a different honesty argument
//!    from this one and is kept out on measured grounds.
//! 5. **Any sampler fallback.** A [`BeliefSampler`] refusal is a hard error here.
//!    An arena seat that fell back to the true legal list would emit
//!    privileged-play games into the corpus, and nothing downstream could tell
//!    them from honest ones.
//!
//! ## Leakage is NOT world-sensitivity (spec §7, ledger L24)
//!
//! The two get confused because both compare decisions across varied inputs.
//! They are opposite properties, and this module is expected to sit on opposite
//! sides of them:
//!
//! | | **LEAKAGE** | **WORLD-SENSITIVITY** |
//! |---|---|---|
//! | the question | does the decision read what the seat cannot see? | does the decision depend on WHICH world was sampled? |
//! | what varies | the TRUE hidden fields (deck orders, boxed pile, opponent starting order, setup seed) | the sampler seed |
//! | what is held | the view, the sampler seed, the agent stream position | the view, the tie key, the candidate dice |
//! | the honest value | **ZERO**, for every contestant | **NONZERO is expected** for an eval-leaf tree |
//! | status | ASSERTED | RECORDED, never asserted zero |
//! | a nonzero reading means | a defect: the search is reading privileged state, or `observe` stopped masking a member — the shape the cell's premise catches | the search is using the world it was handed |
//! | where it lives | `tests/observation_leakage.rs`, `t6_mcts_v1s_decision_is_invariant_under_the_true_hidden_fields` | same file, `t6_mcts_v1s_world_sensitivity_is_measured_and_recorded` |
//!
//! The leakage row's honest scope note (critic R3-3): the invariance holds AT
//! THE SAME AGENT STREAM POSITION. That is weaker than "under any circumstances"
//! and it is sufficient — a stream position is a count of the agent's own
//! decisions, which is not privileged information.
//!
//! The world-sensitivity row is spec §0.4's ARMED entry instrument for candidate
//! (c), and its SCALE is not calibrated (critic R3-2: part of any divergence is
//! tree-shape dice divergence on unshared prefixes, so the measured rate is an
//! upper bound). No numeric threshold is pre-registered on it.
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
//! different property (spec §7 / ledger L24) — the honesty-contract section
//! above carries the table that separates them, and T6 measured this arm at
//! **0.0857 (3 of 35 non-forced points)** against 0/35 for both rungs, in the
//! regime that cell's doc states.
//!
//! At 3–4P the `1 − v` flip reads as "every non-root seat plays against the
//! root", the paranoid reading. 3–4P strength cells are out of scope (spec §1);
//! the convention is stated so a later reader is not left inferring it.
//!
//! # Priors and ordered expansion (spec §2.3, §3.4)
//!
//! **The MANDATORY ROOT PASS runs before any simulation**: every root candidate
//! is applied, observed and evaluated one ply deep, charged to the meter, and
//! kept as that candidate's PRIOR. Internal nodes with more than
//! [`ORDERING_THRESHOLD`] candidates pay the same pass, lazily, the first time
//! they are descended into; smaller nodes keep T2's uniform keyed expansion pick
//! (the pass would cost more than ordering can buy at a family the search covers
//! anyway — [`ORDERING_THRESHOLD`] states the arithmetic).
//!
//! At an ordered node **selection and expansion are ONE selecting point**: an
//! unexpanded candidate competes as a virtual single-visit child carrying its
//! prior, so expansion happens in descending prior order and a child whose
//! backed-up value beats the best remaining prior is re-visited instead
//! ([`ordered_select`] declares the arithmetic; the alternative — an unvisited
//! child at `+∞` — is T2's, and it is what the measurement at the bottom of this
//! doc is about).
//!
//! Successor states from a pass are NOT retained, so an expansion re-applies and
//! re-charges one successor; [`Search::score_candidates`] states the memory
//! trade that buys.
//!
//! **The budget** ([`ScoredBudget`], spec §3.4) is GATED here, not merely
//! metered: the root pass is the one spend the cap cannot refuse, a simulation
//! STARTS only while `realized < cap` and a started one completes, and a root
//! pass that alone spends the cap starves the decision back to
//! [`best_available_pick`] over its one-ply values — never to a first-listed
//! candidate. [`search_world_budgeted`] is where all three live.
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
//! | UCB ties — and, at an ORDERED node, the expansion pick with them | [`McSite::Ucb`] | the node's path key, its own visit count |
//! | expansion pick at a UNIFORM-plan node | [`McSite::Expansion`] | the node's path key, how many children it has already |
//! | the starved root's fallback over the one-ply values | [`McSite::RootFallback`] | — |
//! | a hand-off completion's greedy cut | [`McSite::Completion`] | the leaf's key line, the completion ply |
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
//! **The completion leaf is a GREEDY line, not a search.** [`LeafRule::HandOff`]
//! completes one turn at beam-1 semantics; the value it returns is the best
//! hand-off the greedy line reaches, and nothing here claims that is the best
//! hand-off REACHABLE. A wider completion is a different (and much more
//! expensive) leaf rule, and it is not what spec §2.1 prices.
//!
//! **The completion cost is not bounded by the simulation count.** A simulation
//! under the default leaf rule pays a whole turn's worth of scored successors,
//! so `simulations` and "scored successors" are different currencies and only
//! the second is the experiment's ([`McSearchStats::scored`]). That is exactly
//! why the cap gates STARTS rather than promising a total.
//!
//! # The explanation, and the two margins that are not the same currency
//!
//! [`McExplanation`] (spec §3.5) is the decision as the search left it: one row
//! per root candidate — visits, mean normalized value, one-ply value, prior —
//! plus the chosen row and its margin over the runner-up. It is a pure
//! projection of an [`McSearch`], so explaining costs no apply, no evaluation
//! and nothing on the meter.
//!
//! **The margin is stated in the currency that DECIDED, and the two are not
//! interchangeable.** A searched decision is read out as the argmax over VISITS,
//! so its margin is a visit lead with the mean-value lead carried beside it; a
//! STARVED decision (spec §3.4) never ran a simulation, so every visit count is
//! 0 and its margin is the lead in the mandatory root pass's one-ply values —
//! reporting visits there would describe a search that did not happen. A forced
//! point has no runner-up at all and says so.
//!
//! **A lead nobody measured is `None`, never zero.** [`RootStat::mean`] already
//! refuses to invent a mean for an unvisited candidate, and at a 186-candidate
//! Placement root under a small budget the runner-up frequently IS unvisited —
//! so the value lead against it is refused while the visit lead (defined for
//! every candidate) is still reported.
//!
//! The count rides [`McTally::explained`] — the SAME handle as the budget
//! meters, one per seat, which that type's doc argues for.
//!
//! # The arena seam (spec §3.6)
//!
//! [`MctsAgent`] is an [`ArenaAgent`], so `Contestant::MctsV1` seats it in a
//! cell; [`controller_provenance`] over [`McKnobs`] is the ONE descriptor both
//! the built agent and the contestant's echo read, and an ABSENT `cap` emits no
//! token at all so nothing that existed before the knob is disturbed by it.
//!
//! **A CAPPED SEAT IS SIZED BY ITS BUDGET AND BY NOTHING ELSE**
//! ([`simulations_for_cap`]): naming a cap raises the simulation count to
//! [`SIMULATION_CEILING`], because a cap beside a simulation count would make
//! whichever bound bit first the real knob and no report would say which. Spec
//! §5's cells are named `mcts-v1@750` — a budget, not a count — and this is
//! what makes that name true.
//!
//! # It completes games, and how it completes them is itself a datum
//!
//! Non-termination is a live hazard class this cycle — the capped S1 arm has a
//! MEASURED middle cap band where 2P self-play never ends
//! (`LookaheadAgent::with_cap`) — so the arm is probed rather than assumed. **T3
//! found one here, and it was the probe that found it**: at the experiment's own
//! configuration (a scored cap with `simulations` at a ceiling) a search whose
//! tree runs out of unfinished game loops forever, because the simulations that
//! spin cost nothing and the meter bounds only spending. See
//! [`MAX_STERILE_SIMULATIONS`], and
//! `a_search_whose_tree_runs_out_of_game_still_terminates` for the executable
//! version.
//!
//! **Completion at the SHIPPED T3 configuration** (authoring probe, release,
//! out-of-gate scratch harness, nothing asserted anywhere; regime: 2P
//! MCTS-vs-MCTS self-play, `Rules::standard()`, frozen V1, setup
//! `Keep(0)/Keep(0)`, NO per-move invariants (a direct agent driver, not
//! `run_arena_game`), one agent PER SEAT keyed by `agent_stream_seed`, leaf
//! `HandOff`, `c = √2`, scale 100, `M = 8`, `cap = 750` with `simulations` left
//! at a ceiling, 20,000-move budget; masters `0x1EA4_2026_0727_0001 ^ (m · 0x11)`
//! for `m = 0..3`): **3/3 complete**, at **611 / 895 / 1,490 moves and 104 / 162
//! / 268 turns**, 0.30–0.70 s/game, **331–354 scored successors per decision**
//! (root share 8–9%, cap-bound in ~41% of decisions, 0 starved, overshoot p90
//! ≈115 max ≈359). MCTS self-play turns still run well above lookahead
//! self-play's — the slow-development shape R3 measured for a naive UCT-eval arm
//! — and the mean spend sits well under the cap because most decision points are
//! small or forced. Recorded, not defended.
//!
//! The T2 authoring probe's turn-count RANGE (100–108) was WITHDRAWN at the T2
//! review for omitting exactly the seeding/Keep/invariants block above; the
//! review's own re-run measured 50–304 turns across three plausible seedings.
//! The figures here supersede neither — they are a different configuration (a
//! different leaf rule and a budget) and say so.
//!
//! # The star-tree measurement, re-taken (T2 review's finding, T3's answer)
//!
//! **The T2 finding, as it stood:** at `DEFAULT_SIMULATIONS = 64`, 38 of 46
//! non-forced points (83%) were settled by the root tie draw — Placement 19/19,
//! tie-sets of 64 of 186 candidates, `max_depth` 1.0: a STAR TREE in which no
//! value reached the decision, because expansion was a uniform keyed draw over
//! unexpanded slots with no priors and the read-out is the mandated argmax over
//! visits.
//!
//! **REGIME for every figure below** (the T2 block's withdrawn range is the
//! cautionary tale — state the seeding or state nothing): release, out-of-tree
//! probe, nothing asserted anywhere; `Rules::standard()`, frozen
//! `WeightTable::V1`, 2P; master `0x1EA4_2026_0727_0001`; setup
//! `Keep(0)/Keep(0)`; NO per-move invariants (there is no driver — each point is
//! searched, then the walk advances by the GREEDY-v1 walker's own move, agent
//! seed `0x77`); belief-sampler seed 1 at every point; tie key `0x1`; candidate
//! seed `0xC0FFEE`; `M = 8`; N = the first 120 decision points, of which 46 are
//! non-forced (19 Placement · 19 Movement · 5 AreaAction · 3 other). The BEFORE
//! column was re-run on the pre-change tree with the SAME probe binary, which
//! computes the one-ply ranking itself rather than reading it off the search, so
//! both columns are the same quantity. Leaf `MidTurnDiag` in both columns —
//! T2's default, so the comparison isolates the priors.
//!
//! | statistic, at sims = 64 | T2 (before) | T3 (after) |
//! |---|---|---|
//! | non-forced points settled by the root tie draw | 38/46 (83%) | 38/46 (83%) |
//! | tie-set size, median over contested points | 16 (max 64) | 16 (max 25) |
//! | Placement: chosen move's ONE-PLY RANK, median | **66 of 186** | **1 of 186** |
//! | Placement: max visits on any root child, median | 1 | 2 |
//!
//! At sims = 750 (the experiment's B\*, as a simulation count): 20/46 → 19/46
//! tie-decided, and the Placement rank median was already 1 before.
//!
//! **Read that honestly. What the priors fixed is WHICH candidates the search
//! spends itself on and what the tie draw is drawn over; they did NOT move the
//! tie RATE at the shipped `(c, scale)`, and nothing here claims they did.** The
//! decision at a 186-candidate Placement root moved from a lottery over a
//! uniformly-drawn 64 of the family (median rank 66) to a draw over the TOP of
//! the one-ply ranking (median rank 1), and visits now concentrate rather than
//! spreading one-per-child — the two properties
//! `expansion_is_eval_ordered_and_the_visits_concentrate` pins.
//!
//! **The residual is exploration dominance AND prior degeneracy, and the first
//! of the two is now REACHABLE, which is the structural part of the fix.** The
//! second is a property of the ROOTS, not of the scheme: at the star-tree
//! fixture the 186 candidates carry only 4 distinct one-ply values (120 of them
//! share the top one), so even a perfectly exploiting descent would hand the
//! argmax a genuine tie there — which is why
//! `expansion_is_eval_ordered_and_the_visits_concentrate` asserts that the
//! priors take ≥ 2 distinct values (anti-vacuity) and deliberately does NOT
//! assert strict floor > ceiling separation. Same probe, same regime, sweeping
//! `c` at sims = 64: `c = 0` → 7/46 (15%, Placement 6/19) · `0.1` → 22/46 ·
//! `0.2` → 26/46 · `0.5` → 31/46 · `√2` (shipped) → 38/46. At sims = 750,
//! `c = 0` → 0/46 (0%).
//! Under T2's scheme NO value of `c` could have moved any of this — while
//! any candidate was unexpanded the descent expanded it, uniformly, whatever `c`
//! was. The shipped pair is exploration-dominated exactly as
//! [`DEFAULT_SQUASH_SCALE`] predicted in arithmetic; resolving it is spec §4's
//! screening grid (leaf × c × scale), and **this task deliberately did not
//! pre-empt that by moving the default**.
//!
//! **At the shipped default leaf** (`HandOff`, same regime): sims = 64 → 36/46
//! (78%), Placement rank median 1; sims = 750 → 19/46 (41%), Placement 17/19
//! tie-decided and rank median **12** — a deeper search departing from the
//! one-ply ranking, which is what a search is for. At the experiment's actual
//! operating point (cap = 750 TOTAL scored successors, simulations bounded only
//! by it): 41/46 tie-decided, median 30 simulations at a Placement root (760
//! scored: 186 root pass + 547 completion), rank median 1 at every phase. A
//! completion leaf buys tens of simulations where a mid-turn leaf buys hundreds
//! (377 median at the same cap) — the mechanism-versus-simulation-count trade
//! spec §2.1 says the screening must weigh JOINTLY, arriving here as a number.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::{ArenaAgent, ControllerProvenance, DecisionReport};
use crate::game::belief::BeliefSampler;
use crate::game::budget::{best_available_pick, summarize_overshoot, OvershootSummary, ScoredBudget};
use crate::game::eval::{self, WeightTable, EVAL_IMPL_VERSION};
use crate::game::greedy::{
    candidates, observed_view, score_starting_order, uniform_index, weight_table_identity,
    Candidates, DEFAULT_RETREAT_SAMPLES,
};
use crate::game::lookahead::MAX_TURN_PLIES;
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

/// The stable controller name — error messages, provenance, and the runner
/// grammar.
pub const MCTS_CONTROLLER: &str = "mcts-v1";

/// THIS controller's impl version (spec §0.5's per-controller rule, as
/// [`LOOKAHEAD_IMPL_VERSION`](crate::game::lookahead::LOOKAHEAD_IMPL_VERSION)
/// and `GREEDY_IMPL_VERSION` carry it).
///
/// Bump it when a change moves what an `mcts-v1` seat PLAYS at fixed knobs; a
/// weight-table change is NOT such a change (it rides `weights=<identity>` in
/// the params map — spec §0.5 / L12), and neither is a change confined to
/// counters or explanations. A bump stops figure comparability across the
/// boundary, which is exactly what it is for.
pub const MCTS_IMPL_VERSION: u32 = 1;

/// Simulations per decision. **A placeholder default with no strength claim
/// behind it**: the experiment sizes the search from the shared
/// [`ScoredBudget`](crate::game::budget::ScoredBudget) cap (spec §3.4). This
/// value exists so the knob is total and so a caller that never mentions it
/// gets a search that terminates.
pub const DEFAULT_SIMULATIONS: u32 = 64;

/// The simulation count a BUDGET-SIZED seat runs — `u32::MAX`, i.e. "let the
/// cap decide".
///
/// A capped contestant is sized by its [`ScoredBudget`] and by nothing else
/// (spec §5's cells are named `mcts-v1@750`, a cap and not a simulation count),
/// so pairing a cap with a simulation count would silently make whichever bound
/// bit first the real knob. Termination under this ceiling rests on TWO guards
/// and neither is optional: the cap stops the loop STARTING simulations, and
/// [`MAX_STERILE_SIMULATIONS`] stops it spinning on simulations that charge the
/// meter nothing.
pub const SIMULATION_CEILING: u32 = u32::MAX;

/// The simulation count a seat at `cap` runs — the ONE site that answers it, so
/// the provenance descriptor and the agent the arena builds cannot disagree
/// about the search a cell ran.
///
/// Uncapped seats keep [`DEFAULT_SIMULATIONS`]: without a budget there is no
/// second bound, and a ceiling there would not terminate.
pub fn simulations_for_cap(cap: Option<u32>) -> u32 {
    match cap {
        Some(_) => SIMULATION_CEILING,
        None => DEFAULT_SIMULATIONS,
    }
}

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

/// Above how many candidates a node pays an ORDERING PASS (spec §2.3): every
/// candidate applied, observed and evaluated one ply deep, charged to the meter,
/// and kept as that node's value priors.
///
/// **The root is exempt in the other direction** — its pass is MANDATORY at any
/// size (spec §3.4), because the root is where the budget is allocated and where
/// the decision is read off.
///
/// The threshold exists because the pass is not free and what it buys shrinks
/// with the family: a node with `n ≤ 16` candidates is FULLY expanded by its
/// 16th visit, so ordering changes only the sequence inside a set the search
/// will cover anyway, while the pass costs `n` scored successors up front — at a
/// node most simulations visit once or twice, that is the whole simulation
/// budget spent on ordering. Above the threshold the opposite holds: a
/// 186-candidate Placement node under any budget this experiment prices expands
/// a small PREFIX, so which prefix it is decides the search. 16 is a declared
/// design constant, not a measured optimum, and it is not a screening axis
/// (spec §4's grid is leaf × c × scale).
pub const ORDERING_THRESHOLD: usize = 16;

/// Domain salt for the keyed tie stream, in this crate's `mix` shape.
const SALT_MCTS_STREAM: u64 = 0x3C15_2026_0804_0001;
/// Domain salt for the per-node candidate seed — a DIFFERENT domain from the
/// tie stream, so a node's candidate dice and its tie draws cannot be
/// correlated by construction.
const SALT_MCTS_CANDIDATES: u64 = 0x3C15_2026_0804_0002;
/// Domain salt for the path-key fold.
const SALT_MCTS_BRANCH: u64 = 0x3C15_2026_0804_0003;
/// Domain salt for a hand-off completion's own key line — a THIRD domain, so a
/// completion's greedy cut draws can never coincide with the tree draws of the
/// node it hangs off.
const SALT_MCTS_COMPLETION: u64 = 0x3C15_2026_0804_0004;

/// The root's index in the node arena. Named because "node 0 is the root" is an
/// invariant the descent leans on at every simulation.
const ROOT: usize = 0;

/// **Totality guard on the simulation loop**: how many STERILE simulations one
/// decision may run before the search stops.
///
/// A simulation is sterile when it charges NOTHING — its descent ended at an
/// already-created terminal node, so it expanded nothing, valued nothing and
/// moved the meter not at all. It still backs a value up, so it is a legal
/// simulation; what it is not is progress the BUDGET can see, and that is the
/// hazard: [`search_world_budgeted`] stops starting simulations when the meter
/// dies, and a sterile one never moves the meter.
///
/// **Measured, not imagined** (authoring, release, out-of-gate probe): at a
/// late-game 2P state (turn 268, Movement, 18 root candidates) a search of 5,000
/// simulations created 156 nodes and spent 499 scored successors — and then
/// nothing further, ever, because every leaf below the root was terminal. Under
/// the experiment's own shape (a scored cap with `simulations` left at a
/// ceiling, which is exactly how a `cap`-sized contestant is configured) that is
/// an infinite loop; it was found by driving a full self-play game, which is why
/// the module's completion probe is a real one and not a formality. The second
/// way it repeats is `c = 0`: a terminal child with the best mean is selected
/// forever whatever remains unexpanded, and a screening grid may legitimately
/// try a small `c`.
///
/// The guard counts sterile simulations in TOTAL rather than consecutively, so
/// the worst-case wasted work is one bounded batch of descents per decision
/// rather than one per productive simulation. When it fires, the decision is
/// taken on the tree as it stands and [`McSearchStats::sterile_simulations`]
/// says so; nothing is discarded, and no value is invented.
pub const MAX_STERILE_SIMULATIONS: u32 = 1_024;

/// How an unexpanded node is valued (spec §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeafRule {
    /// **THE DEFAULT** (spec §2.1): complete the node's CURRENT TURN greedily —
    /// beam-1 semantics — and evaluate `WeightTable::V1` at the
    /// `current_player` transition; a finished game is valued by
    /// [`final_result`] instead. See [`Search::hand_off_value`] for the exact
    /// rule, including whose interest the greedy cut serves at an opponent node.
    ///
    /// The grounds are measured (spec §2.1): ~40% of a naive per-move UCT's leaf
    /// values are read MID-TURN — states V1 was never calibrated on, and states
    /// `lookahead-v1` by construction never evaluates. Completing the turn puts
    /// every leaf value on the axis the evaluator was fitted for.
    #[default]
    HandOff,
    /// **The DIAGNOSTIC rule**: evaluate the node's own state where it stands,
    /// mid-turn included. Spec §2.1 keeps it as a screening knob precisely
    /// because measuring that arm — jointly with the simulation count the shared
    /// cap buys each mode — is how §4 tells the mechanism from its price.
    MidTurnDiag,
    /// **A TEST INSTRUMENT, and never a screening arm**: EVERY value this
    /// search reads is constant — leaf, prior and terminal alike, at 0.5, with
    /// every one-ply value 0 and [`eval::score`] never called — so no value
    /// reaches any decision and the search is driven by its keyed draws alone.
    ///
    /// It is the whole value axis and not just the leaf deliberately: a
    /// constant-leaf arm that kept REAL priors would still rank the root by one
    /// ply, so the pin below would be pinning the one-ply argmax rather than a
    /// value-blind tree — the exact substitution that makes a degeneracy pin
    /// pass while the degeneracy it names is live.
    ///
    /// It exists for the degeneracy pin (T2 review NOTE 7, the capped-S1 arm's
    /// sibling): a search whose values all coincide must STILL place assistants
    /// and finish a game. That is ledger row 18's null-placement degeneracy
    /// asked of a tree — if a value-blind MCTS quietly settles on "place
    /// nothing", the game never ends and the pin says so in seconds. Spec §4's
    /// grid never selects it, and the provenance grammar (T5) carries it as a
    /// diagnostic name.
    ConstantDiag,
}

impl LeafRule {
    /// The runner grammar's spelling (`leaf=<mode>`), and the ONLY place these
    /// names exist — `examples/arena.rs` parses through
    /// [`from_str_name`](Self::from_str_name) rather than matching literals, so
    /// the command line cannot drift from what provenance records.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HandOff => "hand-off",
            Self::MidTurnDiag => "midturn-diag",
            Self::ConstantDiag => "constant-diag",
        }
    }

    /// Parse [`as_str`](Self::as_str); `None` on anything else (the runner turns
    /// that into an error rather than a silent default — an unknown knob VALUE
    /// is as dangerous as an unknown knob NAME).
    pub fn from_str_name(name: &str) -> Option<Self> {
        [Self::HandOff, Self::MidTurnDiag, Self::ConstantDiag]
            .into_iter()
            .find(|rule| rule.as_str() == name)
    }

    /// This rule's WIRE VALUE in the provenance params map, which is
    /// `i64`-valued and cannot carry a name.
    ///
    /// **Stable**: a corpus record's `leaf=` is read back against these numbers,
    /// so they may be extended but never renumbered — the same contract
    /// `weight_table_identity` carries for its digest.
    pub fn code(self) -> i64 {
        match self {
            Self::HandOff => 0,
            Self::MidTurnDiag => 1,
            Self::ConstantDiag => 2,
        }
    }
}

/// The knob set ONE MCTS-v1 seat runs — the arena contestant's payload, the
/// agent's constructor input and the provenance descriptor's input in one type.
///
/// One type rather than three parameter lists because the failure it prevents is
/// specific: a descriptor built from the cell's knobs and an agent built from
/// the same knobs by a different code path are exactly how a report comes to
/// name a search nobody ran. [`MctsAgent::knobs`] reads them back off a built
/// agent, so the round trip is checkable rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McKnobs {
    /// Spec §3.4's TOTAL scored-successor cap, root pass included. `None` is
    /// uncapped — and an ABSENT cap emits NO params token (spec §3.6).
    pub cap: Option<u32>,
    /// UCB1's `c`, in ppm.
    pub exploration_ppm: u32,
    /// The squash scale, in [`eval::score`] units.
    pub squash_scale: i64,
    /// `M`, the shipped candidate scheme's retreat sample count.
    pub retreat_samples: u8,
    /// How an unexpanded node is valued.
    pub leaf: LeafRule,
    /// Simulations per decision. Derived from `cap` by [`simulations_for_cap`]
    /// at every constructor here; carried explicitly so an agent built by hand
    /// at some other count still DESCRIBES itself truthfully.
    pub simulations: u32,
    /// `K`, sampled worlds per decision. 1 ships; above it `choose` refuses.
    pub worlds: u8,
}

impl McKnobs {
    /// The shipped knobs, uncapped.
    pub const DEFAULT: Self = Self {
        cap: None,
        exploration_ppm: DEFAULT_EXPLORATION_PPM,
        squash_scale: DEFAULT_SQUASH_SCALE,
        retreat_samples: DEFAULT_RETREAT_SAMPLES,
        leaf: LeafRule::HandOff,
        simulations: DEFAULT_SIMULATIONS,
        worlds: DEFAULT_WORLDS,
    };

    /// The five knobs a cell names, with the simulation count DERIVED from the
    /// cap (spec §5's `mcts-v1@<cap>` shape — see [`simulations_for_cap`]).
    pub fn sized_by_cap(
        cap: Option<u32>,
        exploration_ppm: u32,
        squash_scale: i64,
        retreat_samples: u8,
        leaf: LeafRule,
    ) -> Self {
        Self {
            cap,
            exploration_ppm,
            squash_scale,
            retreat_samples,
            leaf,
            simulations: simulations_for_cap(cap),
            worlds: DEFAULT_WORLDS,
        }
    }
}

impl Default for McKnobs {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// THE provenance of an MCTS-v1 seat at these knobs — name, version and knob map
/// in one place (spec §3.6).
///
/// A free function for [`crate::game::lookahead::controller_provenance`]'s
/// reason: the arena's [`Contestant`](crate::game::arena::Contestant) must be
/// able to say what a controller calls itself WITHOUT building an agent (a build
/// needs `&Rules` for the Keep table). Both callers route here, so
/// [`MCTS_CONTROLLER`] and [`MCTS_IMPL_VERSION`] are read at exactly one site.
///
/// **An ABSENT cap emits NO token** (spec §3.6, and lookahead's
/// `controller_provenance_capped` verbatim): an uncapped seat's descriptor is
/// the descriptor it would have had if the knob had never been added.
pub fn controller_provenance(
    knobs: McKnobs,
    weights: &WeightTable,
    agent_seed: Option<u64>,
) -> ControllerProvenance {
    let mut params = BTreeMap::from([
        ("c".to_string(), i64::from(knobs.exploration_ppm)),
        ("scale".to_string(), knobs.squash_scale),
        ("m".to_string(), i64::from(knobs.retreat_samples)),
        ("leaf".to_string(), knobs.leaf.code()),
        // `worlds.max(1)` mirrors the search's own floor, exactly as lookahead's
        // descriptor does: a descriptor printing a raw 0 would name a decision
        // over zero sampled worlds, which is not a decision.
        ("k".to_string(), i64::from(knobs.worlds.max(1))),
        ("eval_impl".to_string(), i64::from(EVAL_IMPL_VERSION)),
        ("weights".to_string(), weight_table_identity(weights)),
    ]);
    if let Some(cap) = knobs.cap {
        params.insert("cap".to_string(), i64::from(cap));
    }
    // The absent-optional-knob rule, applied to the DERIVED knob too (T5 review
    // S1): a capped seat's `simulations` is [`SIMULATION_CEILING`] — a loop
    // bound the budget always pre-empts, not a count any run realizes — and a
    // published number must describe what ran. The token is emitted only when
    // it is a real bound.
    if knobs.simulations != SIMULATION_CEILING {
        params.insert("sims".to_string(), i64::from(knobs.simulations));
    }
    ControllerProvenance {
        name: MCTS_CONTROLLER.to_string(),
        version: MCTS_IMPL_VERSION,
        params,
        agent_seed,
    }
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
    /// node's `visit`-th descent through it. At an ORDERED node this is also the
    /// expansion site: selection and expansion are ONE selecting point there
    /// (an unexpanded candidate competes as a virtual single-visit child
    /// carrying its prior), so [`Expansion`](Self::Expansion) is a
    /// uniform-plan-only site.
    Ucb { branch: u64, visit: u32 },
    /// The expansion pick at the UNIFORM-plan node whose path key is `branch`,
    /// filling its `filled`-th child slot.
    Expansion { branch: u64, filled: u32 },
    /// The STARVED root's fallback pick over the mandatory root pass's one-ply
    /// values (spec §3.4's fallback chain). A different site from
    /// [`RootArgmax`](Self::RootArgmax) because it is a different rule over a
    /// different quantity — the visit counts it would otherwise draw over are
    /// all zero, which is exactly the degenerate whole-family draw the chain
    /// exists to replace.
    RootFallback,
    /// The greedy cut inside a hand-off completion: the leaf whose node key is
    /// `branch`, at completion ply `ply`.
    Completion { branch: u64, ply: u32 },
}

impl McSite {
    /// `(tag, a, b)` — the tag keeps the site families disjoint where their
    /// coordinates coincide.
    fn coords(self) -> [u64; 3] {
        match self {
            Self::RootArgmax => [0, 0, 0],
            Self::Ucb { branch, visit } => [1, branch, u64::from(visit)],
            Self::Expansion { branch, filled } => [2, branch, u64::from(filled)],
            Self::RootFallback => [3, 0, 0],
            Self::Completion { branch, ply } => [4, branch, u64::from(ply)],
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

/// The key line a hand-off completion runs on, rooted at the leaf node's own
/// path key and then extended per ply by [`descend_branch`].
///
/// A completion is LEAF-VALUE work, not tree edges (spec §2.1), so its states
/// are never nodes and its keys must not be able to coincide with the keys of
/// the tree nodes that would sit at the same places: its own salt puts the whole
/// line in a disjoint domain.
fn completion_branch(branch: u64) -> u64 {
    let mut x = branch ^ SALT_MCTS_COMPLETION;
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

/// One candidate as an ORDERED node's selection reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Slot {
    /// No child yet: the candidate stands on the PRIOR its ordering pass paid
    /// for (spec §3.2's "used as VALUE and not merely ordering").
    Unexpanded { prior: f64 },
    /// A real child, on its backed-up mean.
    Child(ChildStat),
}

/// **THE PRIOR SEMANTICS, declared** (spec §3.2 / §2.3; the T2 review's
/// star-tree finding is what this function exists to answer).
///
/// At an ordered node every candidate — expanded or not — competes in ONE UCB1
/// comparison:
///
/// ```text
/// exploit = prior            if the candidate has no child yet
///         = backed-up mean   once it has one
/// explore = c · sqrt(ln(parent_visits) / max(child_visits, 1))
/// ```
///
/// **An unexpanded candidate is scored as a VIRTUAL SINGLE-VISIT child carrying
/// its prior** (first-play urgency = prior). Two consequences, both deliberate:
///
/// 1. a prior and a backed-up mean enter at the SAME exploration weight, so the
///    root pass's work is usable as value rather than only as an order — lens 1
///    M7's equal-usability repair, made arithmetic;
/// 2. expansion therefore happens in DESCENDING prior order (ties by the keyed
///    draw), interleaved with re-visits of children whose backed-up value beats
///    the best remaining prior — which is spec §2.3's eval-ordered expansion
///    without a second, separately-tunable schedule beside UCB1.
///
/// The alternative — T2's — is `+∞` for an unvisited child, which orders nothing
/// and expands everything before any value can be read: the measured star tree.
///
/// The perspective flip is [`selection_value`]'s and applies to priors exactly
/// as it does to means (a prior is stored in the ROOT seat's perspective, like
/// every other value in the tree).
fn ordered_select(
    slots: &[Slot],
    parent_visits: u32,
    parent_seat: PlayerId,
    root_seat: PlayerId,
    c: f64,
    word: u64,
) -> usize {
    debug_assert!(!slots.is_empty(), "a selection over an empty candidate set is undefined");
    let mut best = f64::NEG_INFINITY;
    let mut tied: Vec<usize> = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        let (mean, visits) = match slot {
            Slot::Unexpanded { prior } => (*prior, 1),
            Slot::Child(child) => (child.mean(), child.visits.max(1)),
        };
        let u = ucb1(selection_value(parent_seat, root_seat, mean), visits, parent_visits, c);
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

/// `a` beats `b` for a seat maximizing the ROOT value — or minimizing it, which
/// is what every non-root seat does (spec §3.2's convention, in the one shape a
/// completion needs it).
fn better<T: PartialOrd>(a: T, b: T, maximize: bool) -> bool {
    if maximize {
        a > b
    } else {
        a < b
    }
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
    /// Of those, how many were STERILE — a descent that ended at an existing
    /// terminal node and therefore charged nothing (see
    /// [`MAX_STERILE_SIMULATIONS`], which is what stops a budget-bounded loop
    /// from spinning on them). Expected 0 away from the end of a game; recorded
    /// rather than asserted, because how often a tree runs out of unfinished
    /// game to search is a property of the position, not of this module.
    pub sterile_simulations: u64,
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
    /// [`ScoredBudget`] meters (one apply + observe + eval, or + [`final_result`]
    /// at a terminal), and the number the cap gates.
    ///
    /// **The identity every other scored counter is checked against**:
    /// `scored == root_scored + ordering_scored + completion_scored + (nodes −
    /// 1)`, the last term being one expansion per created node with the root
    /// excepted (it was never applied). Pinned, because four contributors summed
    /// into one meter is exactly the shape in which one of them quietly stops
    /// being charged.
    pub scored: u64,
    /// The MANDATORY ROOT PASS's share of [`scored`](Self::scored) — spec
    /// §0.3's "realized root share", and 0 at a forced decision point, which
    /// allocates nothing and therefore ranks nothing.
    pub root_scored: u64,
    /// INTERNAL nodes that paid an ordering pass ([`ORDERING_THRESHOLD`]). The
    /// root's mandatory pass is not counted here — it is unconditional and has
    /// its own counter.
    pub ordering_passes: u64,
    /// Successors scored by those internal ordering passes.
    pub ordering_scored: u64,
    /// Successors scored inside hand-off completions ([`LeafRule::HandOff`]).
    /// **0 under every other leaf rule**, which is what makes the leaf's price
    /// readable rather than inferred from a total.
    pub completion_scored: u64,
    /// Enumerations paid INSIDE completions — one per completion ply beyond the
    /// first (the first re-uses the leaf node's own candidate cache). Counted
    /// because [`handles`](Self::handles) is otherwise no longer equal to
    /// `nodes − terminal_nodes` under the default leaf rule, and an identity
    /// that stops holding without a named term is an identity nobody can check.
    pub completion_points: u64,
    /// Completions [`MAX_TURN_PLIES`] stopped — expected 0 (`lookahead`'s guard,
    /// its constant, and its measured 0 hits). A hit is not a wrong answer but
    /// it is a quietly different one: the leaf is valued where the guard stopped
    /// it, mid-turn, which is the very thing this leaf rule exists to avoid.
    pub completion_cap_hits: u64,
    /// 1 if the cap DENIED work this decision (a simulation could not start, or
    /// the root pass alone spent the budget), else 0. **0 by construction
    /// uncapped.**
    pub budget_binds: u64,
    /// 1 if the MANDATORY root pass alone spent the cap, so no simulation could
    /// start and the decision fell back to the root priors (spec §3.4). A
    /// sub-case of [`budget_binds`](Self::budget_binds).
    pub budget_starved: u64,
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
    /// The MANDATORY root pass's one-ply value, RAW ([`eval::score`] at the
    /// successor, root-seat perspective) — the quantity the starved fallback
    /// ranks on.
    ///
    /// `None` only at a FORCED decision point, where no pass runs because there
    /// is nothing to allocate and nothing to rank (spec §3.4's pass is "before
    /// any deeper work"; a forced point has none).
    pub one_ply: Option<i64>,
    /// The same value on the tree's `[0, 1]` axis — this candidate's PRIOR, as
    /// [`ordered_select`] reads it. Terminal successors are valued by
    /// [`rank_value`] here rather than squashed, so the prior is on the axis the
    /// backed-up child values live on, terminals included.
    ///
    /// **It is the ONE-PLY rule's value, not the leaf rule's**, and the two
    /// coincide only under [`LeafRule::MidTurnDiag`]: a prior evaluates the
    /// successor where it stands, while the shipped [`LeafRule::HandOff`] child
    /// is valued at the end of its turn. Measured at the T3 review over the same
    /// 120-point walk the star-tree table uses: under `MidTurnDiag` the prior
    /// and the first backed-up child value differ at 6.4% of points (median
    /// delta +0.0000), under `HandOff` at 27.5% (median −0.0081). So the prior
    /// is a comparable estimate of the child value, never a promise to equal it.
    pub prior: Option<f64>,
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

impl McSearch {
    /// Spec §3.5's root table, taking the roots rather than copying them: the
    /// agent stores the explanation and serves
    /// [`last_roots`](MctsAgent::last_roots) out of it, so an explained decision
    /// costs no allocation the search had not already paid.
    pub fn into_explanation(self) -> McExplanation {
        McExplanation::new(self.roots, self.chosen, self.stats)
    }
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

/// How a node hands out its next simulation — computed AT MOST ONCE per node,
/// and lazily: a node created as a leaf may never be descended into again, and
/// paying its ordering pass at creation would charge the budget for an ordering
/// nothing ever reads.
#[derive(Debug)]
enum Plan {
    /// Not asked yet.
    Deferred,
    /// `≤ ORDERING_THRESHOLD` candidates (and never the root): unexpanded slots
    /// first, by uniform keyed draw, then UCB1 over the children. T2's scheme,
    /// kept where the ordering pass cannot pay for itself.
    Uniform,
    /// Priors in the ROOT seat's perspective, one per candidate —
    /// [`ordered_select`]'s input.
    Ordered { priors: Vec<f64> },
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
    /// How this node expands (lazily computed).
    plan: Plan,
}

/// The mutable half of one search.
struct Search<'a> {
    ctx: &'a McCtx<'a>,
    /// The ROOT seat — every value in the tree is in this seat's perspective.
    seat: PlayerId,
    nodes: Vec<Node>,
    stats: McSearchStats,
    /// The DECISION's meter, borrowed rather than owned: spec §0.3 prices a
    /// budget per decision, and `K > 1` (refused here, but the shape is stated)
    /// would share one meter across worlds — `lookahead::Run`'s arrangement, for
    /// the same reason.
    budget: &'a mut ScoredBudget,
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
            plan: Plan::Deferred,
        });
        self.nodes.len() - 1
    }

    /// Create the node reached by an expansion: terminal (valued by
    /// [`final_result`], never handled) or expandable (its one handle, its
    /// cached candidates, its leaf value).
    fn push_child(&mut self, state: GameState, branch: u64) -> Result<usize, String> {
        if state.is_game_over() {
            // `unit_value` rather than `terminal_value` directly, so the
            // value-blind diagnostic arm is blind HERE too (see
            // [`LeafRule::ConstantDiag`]); the raw argument is unread at a
            // finished game.
            let value = self.unit_value(&state, 0);
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
                plan: Plan::Deferred,
            });
            return Ok(self.nodes.len() - 1);
        }
        // The ENUMERATION comes first, so the hand-off completion's first ply
        // can run off this node's own candidate cache instead of building a
        // second handle for the very same state (L10, and the reason
        // `completion_points` counts plies BEYOND the first).
        let (candidates, _) = self.expand_point(&state, branch, false)?;
        let value = self.leaf_value(&state, &candidates, branch)?;
        Ok(self.push_expandable(state, branch, candidates, value))
    }

    /// The value of an unexpanded node, per [`LeafRule`] — the one place a leaf
    /// value comes from, so the rest of the module reads an `f64` in `[0, 1]`
    /// and never learns which rule produced it.
    fn leaf_value(
        &mut self,
        state: &GameState,
        candidates: &[Move],
        branch: u64,
    ) -> Result<f64, String> {
        match self.ctx.leaf {
            LeafRule::HandOff => self.hand_off_value(state, candidates, branch),
            LeafRule::MidTurnDiag => Ok(squash(self.raw_value(state)?, self.ctx.squash_scale)),
            LeafRule::ConstantDiag => Ok(0.5),
        }
    }

    /// [`eval::score`] at `state`, RAW, from the ROOT seat's view — the module's
    /// only evaluation site, so "every value in this tree is in the root seat's
    /// perspective" is structural rather than a convention to remember.
    fn raw_value(&self, state: &GameState) -> Result<i64, String> {
        if self.ctx.leaf == LeafRule::ConstantDiag {
            return Ok(0);
        }
        let view = observed_view(state, self.seat, MCTS_CONTROLLER)?;
        Ok(eval::score(&view, self.ctx.rules, self.ctx.weights))
    }

    /// A successor's value on the tree's `[0, 1]` axis: [`rank_value`] when the
    /// game is over, the squash of [`raw_value`](Self::raw_value) otherwise.
    ///
    /// The split is the module doc's: a terminal is valued by the official
    /// ranking and never passes through the squash, so a prior and the child
    /// value that will replace it are the same quantity even at a terminal.
    fn unit_value(&self, state: &GameState, raw: i64) -> f64 {
        if self.ctx.leaf == LeafRule::ConstantDiag {
            return 0.5;
        }
        if state.is_game_over() {
            terminal_value(state, self.ctx.rules, self.seat)
        } else {
            squash(raw, self.ctx.squash_scale)
        }
    }

    /// **THE HAND-OFF COMPLETION LEAF** (spec §2.1, [`LeafRule::HandOff`]).
    ///
    /// Complete the turn that is IN PROGRESS at `start` — beam-1 semantics, one
    /// frontier state per ply, every candidate of it applied+observed+evaluated
    /// and charged — and return the value at the `current_player` transition. A
    /// completion that ends the game is valued by [`final_result`] instead.
    ///
    /// # Four things this rule fixes in writing, because each is a place it
    /// could silently be a different rule
    ///
    /// 1. **Whose turn.** The turn completed is the one the NODE stands in
    ///    (`start.current_player`), not the root seat's. The tree descends past
    ///    the hand-off (spec §3.2), so mid-OPPONENT-turn nodes exist and valuing
    ///    them raw would reintroduce the very miscalibration this leaf exists to
    ///    remove.
    /// 2. **Whose interest the greedy cut serves.** The acting seat's: at a root
    ///    node the cut maximizes the root value, at an opponent node it
    ///    MINIMIZES it — [`selection_value`]'s convention, applied to the same
    ///    numbers, so the leaf agrees with the descent it hangs off rather than
    ///    modelling a second opponent.
    /// 3. **Where the value comes from.** Every state is evaluated from the ROOT
    ///    seat's view ([`raw_value`](Self::raw_value)), so what is returned needs
    ///    no perspective conversion and no zero-sum assumption beyond the one
    ///    §3.2 already states.
    /// 4. **What it is not.** It is NOT
    ///    [`crate::game::lookahead::search_world`] at `beam = 1`. That entry
    ///    point continues EVERY root candidate (its root loop is the beam's
    ///    first ply, uncut), which costs `candidates × turn` per leaf instead of
    ///    `turn`, re-enumerates a state this node has already enumerated, and
    ///    values on the evaluator's raw axis rather than this module's declared
    ///    `[0, 1]` one. `complete_turn` itself is private to that module. What IS
    ///    shared is the guard and its constant ([`MAX_TURN_PLIES`]), the
    ///    candidate scheme, and the evaluator.
    ///
    /// Completion states are LEAF-VALUE work and never tree edges: they are not
    /// nodes, they are not backed up, and nothing selects among them a second
    /// time.
    fn hand_off_value(
        &mut self,
        start: &GameState,
        start_candidates: &[Move],
        branch: u64,
    ) -> Result<f64, String> {
        let actor = start.current_player;
        // The root seat maximizes; any other seat minimizes the root's value.
        let maximize = actor == self.seat;
        let mut best: Option<f64> = None;
        let mut key = completion_branch(branch);

        let (mut carried, mut ply) = (self.complete_ply(start, start_candidates, key, 0, maximize, &mut best)?, 1u32);
        while let Some((state, slot, raw)) = carried {
            if ply >= MAX_TURN_PLIES {
                // Totality guard (lookahead's, and its measured 0 hits). Value
                // the frontier where it stands rather than looping — on the raw
                // score the cut ALREADY computed and charged for this very
                // state, so the guard adds no evaluation the meter cannot see.
                self.stats.completion_cap_hits += 1;
                let stopped = self.unit_value(&state, raw);
                best = Some(match best {
                    Some(b) if !better(stopped, b, maximize) => b,
                    _ => stopped,
                });
                break;
            }
            key = descend_branch(key, slot);
            let (candidates, _) = self.expand_point(&state, key, false)?;
            self.stats.completion_points += 1;
            carried = self.complete_ply(&state, &candidates, key, ply, maximize, &mut best)?;
            ply += 1;
        }

        best.ok_or_else(|| {
            format!(
                "{MCTS_CONTROLLER}: seat {}'s turn produced no hand-off from a completion leaf",
                actor.0,
            )
        })
    }

    /// One ply of the completion: score every candidate, fold the hand-offs into
    /// `best`, and hand back the single continuing successor the greedy cut
    /// keeps — with the candidate index it came from (for the key line) and its
    /// RAW score, which the caller's ply guard reuses instead of re-evaluating a
    /// state this ply already paid for.
    ///
    /// Two comparison spaces, deliberately: hand-offs are compared on the
    /// `[0, 1]` axis because a terminal can only be expressed there, while the
    /// CUT compares RAW [`eval::score`] — a continuing successor is never
    /// terminal, and the squash can round two distinct raw scores onto one
    /// `f64`, which would invent a tie the cut then has to draw over.
    fn complete_ply(
        &mut self,
        state: &GameState,
        candidates: &[Move],
        key: u64,
        ply: u32,
        maximize: bool,
        best: &mut Option<f64>,
    ) -> Result<Option<(GameState, usize, i64)>, String> {
        let actor = state.current_player;
        let mut cut_value: Option<i64> = None;
        let mut cut: Vec<(GameState, usize)> = Vec::new();
        // `candidates` and `state` are the CALLER's borrows, not this search's,
        // so the loop can hold them across the `&mut self` calls inside — which
        // is why the completion pays no per-candidate `Move` clone (the tree's
        // own expansion, indexing into `self.nodes`, has no such option).
        for (i, mv) in candidates.iter().enumerate() {
            let next = self.apply(state, mv)?;
            self.charge();
            self.stats.completion_scored += 1;
            let raw = self.raw_value(&next)?;
            if next.current_player != actor || next.is_game_over() {
                let value = self.unit_value(&next, raw);
                *best = Some(match *best {
                    // STRICTLY better only, so an exact tie leaves the first
                    // one standing — `complete_turn`'s rule, and this is not a
                    // decision tie point: the value is identical either way and
                    // no move is ever played from it.
                    Some(b) if !better(value, b, maximize) => b,
                    _ => value,
                });
            } else if cut_value.is_none_or(|v| better(raw, v, maximize)) {
                cut_value = Some(raw);
                cut.clear();
                cut.push((next, i));
            } else if cut_value == Some(raw) {
                cut.push((next, i));
            }
        }
        if cut.is_empty() {
            return Ok(None);
        }
        // Every survivor of the cut shares `cut_value` by construction — that IS
        // what put them in the same tie set.
        let raw = cut_value.expect("a non-empty cut has a cut value");
        // The cut IS a selecting point (row 18): a first-listed keep would hand
        // the whole completion to whatever the generator emitted first.
        let word = self.ctx.stream.word(McSite::Completion { branch: key, ply });
        let (kept, i) = cut.swap_remove(uniform_index(word, cut.len()));
        Ok(Some((kept, i, raw)))
    }

    /// Charge ONE scored successor to both the cost counter and the decision's
    /// meter. **The single site that increments `scored`**, so the meter cannot
    /// drift from the counter the reports call the realized total.
    ///
    /// **The meter is TOTAL over evaluations**: every
    /// [`raw_value`](Search::raw_value) this module performs belongs to a
    /// successor that was charged here. The one place that came close to an
    /// exception — the completion's [`MAX_TURN_PLIES`] guard, which used to
    /// re-evaluate the frontier state the cut had already scored — now reuses
    /// the raw score [`complete_ply`](Search::complete_ply) hands back, so a
    /// reader can price a decision from `scored` alone.
    fn charge(&mut self) {
        self.stats.scored += 1;
        self.budget.charge();
    }

    /// Apply, charge and one-ply-value every candidate of `at` — the ORDERING
    /// PASS (spec §2.3), and at the root the MANDATORY one (spec §3.4).
    ///
    /// Returns `(raw one-ply values, priors on the [0,1] axis)`.
    ///
    /// **The successor states are deliberately NOT retained.** Keeping them
    /// would save the later re-apply at expansion (`lookahead`'s capped root
    /// pass does exactly that, in `RootPass.wants`), but that arm pays it at ONE
    /// level; here every ordered node in the tree would hold a whole family of
    /// `GameState`s — spec §2.8 prices a node at ≈3.3–3.5 KB, and a
    /// 186-candidate Placement node would hold ≈0.6 MB of them for as long as
    /// the tree lives. The price of dropping them is exactly ONE re-scored
    /// successor per expansion (bounded by the simulation count, and a small
    /// fraction of a completion leaf), charged honestly because the work is
    /// genuinely done twice.
    fn score_candidates(&mut self, at: usize) -> Result<(Vec<i64>, Vec<f64>), String> {
        let n = self.nodes[at].candidates.len();
        let mut raws = Vec::with_capacity(n);
        let mut priors = Vec::with_capacity(n);
        for i in 0..n {
            let mv = self.nodes[at].candidates[i].clone();
            let next = self.apply(&self.nodes[at].state, &mv)?;
            self.charge();
            let raw = self.raw_value(&next)?;
            raws.push(raw);
            priors.push(self.unit_value(&next, raw));
        }
        Ok((raws, priors))
    }

    /// Decide — ONCE per node — how it hands out simulations (see [`Plan`]).
    fn ensure_plan(&mut self, at: usize) -> Result<(), String> {
        if !matches!(self.nodes[at].plan, Plan::Deferred) {
            return Ok(());
        }
        let plan = if self.nodes[at].candidates.len() > ORDERING_THRESHOLD {
            let before = self.stats.scored;
            let (_, priors) = self.score_candidates(at)?;
            self.stats.ordering_passes += 1;
            self.stats.ordering_scored += self.stats.scored - before;
            Plan::Ordered { priors }
        } else {
            Plan::Uniform
        };
        self.nodes[at].plan = plan;
        Ok(())
    }

    /// The candidate index this node hands its next simulation to — expanded or
    /// not. The caller expands it if it has no child yet, and descends into it
    /// otherwise.
    fn choose_slot(&self, at: usize) -> usize {
        let node = &self.nodes[at];
        match &node.plan {
            Plan::Ordered { priors } => {
                let slots: Vec<Slot> = node
                    .children
                    .iter()
                    .zip(priors)
                    .map(|(child, prior)| match child {
                        Some(c) => Slot::Child(ChildStat {
                            visits: self.nodes[*c].visits,
                            value_sum: self.nodes[*c].value_sum,
                        }),
                        None => Slot::Unexpanded { prior: *prior },
                    })
                    .collect();
                ordered_select(
                    &slots,
                    node.visits,
                    node.seat,
                    self.seat,
                    self.ctx.exploration(),
                    self.ctx.stream.word(McSite::Ucb { branch: node.branch, visit: node.visits }),
                )
            }
            Plan::Uniform => {
                let unexpanded: Vec<usize> = node
                    .children
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.is_none().then_some(i))
                    .collect();
                if !unexpanded.is_empty() {
                    let word = self.ctx.stream.word(McSite::Expansion {
                        branch: node.branch,
                        filled: node.expanded,
                    });
                    return expansion_pick(&unexpanded, word);
                }
                self.select(at)
            }
            Plan::Deferred => unreachable!("every descent calls ensure_plan before choose_slot"),
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
    /// **Termination needs no guard.** Every step of the descent moves to a
    /// CHILD of the node it is at — the tree is acyclic by construction — and
    /// every simulation creates exactly one node, so a path is bounded by the
    /// tree's size, which is bounded by the simulation count. (That is why a
    /// determinized world that cycles — the measured non-terminating band on the
    /// capped S1 arm — cannot hang this loop.) Note that an ORDERED node is
    /// descended through while still partially expanded, which T2's
    /// expand-everything-first descent never did; the bound above does not
    /// depend on that difference.
    ///
    /// The per-step `Vec`s in [`choose_slot`](Self::choose_slot) are
    /// `O(candidates)` and deliberately unoptimized: the step they sit beside
    /// pays a full candidate ENUMERATION (and, under the default leaf rule, a
    /// whole turn's completion), which the research base prices orders of
    /// magnitude above a slot scan, and splitting them into allocation-free
    /// variants would leave the tested pure functions ([`expansion_pick`],
    /// [`ordered_select`]) beside second, untested ones.
    fn simulate(&mut self) -> Result<(), String> {
        let mut path: Vec<usize> = vec![ROOT];
        let mut at = ROOT;
        let mut depth: u32 = 0;
        let value = loop {
            if self.nodes[at].terminal {
                break self.nodes[at].value;
            }
            self.ensure_plan(at)?;
            let slot = self.choose_slot(at);
            if self.nodes[at].children[slot].is_none() {
                let branch = self.nodes[at].branch;
                let mv = self.nodes[at].candidates[slot].clone();
                let next = self.apply(&self.nodes[at].state, &mv)?;
                self.charge();
                let child = self.push_child(next, descend_branch(branch, slot))?;
                self.nodes[at].children[slot] = Some(child);
                self.nodes[at].expanded += 1;
                path.push(child);
                depth += 1;
                break self.nodes[child].value;
            }
            at = self.nodes[at].children[slot].expect("the slot was checked to have a child");
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
/// candidate is answered by the root's single enumeration, and NO root pass and
/// NO simulation run. The root node is still materialized, so the counter
/// identities hold uniformly (`handles == nodes − terminal_nodes +
/// completion_points` whether or not a tree grew); what the fast path skips is
/// the tree and the pass, which are the cost.
pub fn search_world(
    world: &GameState,
    seat: PlayerId,
    ctx: &McCtx<'_>,
) -> Result<McSearch, String> {
    search_world_budgeted(world, seat, ctx, &mut ScoredBudget::unlimited())
}

/// [`search_world`] against a DECISION's scored-successor meter (spec §3.4).
///
/// The structure the cap imposes, in the order it happens:
///
/// 1. **The MANDATORY ROOT PASS is the one spend the cap cannot refuse.** Every
///    root candidate is applied, observed and evaluated before any simulation
///    starts, and the pass is NOT gated — gating it would leave candidates with
///    no value at all, which is what makes the fallback below expressible. A
///    root family bigger than the cap therefore OVERSHOOTS, and the overshoot is
///    reported rather than hidden ([`ScoredBudget::overshoot`]).
/// 2. **A simulation starts only while `realized < cap`, and a started one
///    completes.** No caller can price a simulation before running it (the
///    default leaf rule alone can spend a turn's worth of successors), so the
///    start gate is the only honest question — and aborting one mid-flight would
///    back up a value read at a state the search never meant to stop at. The
///    overshoot distribution is the validity line that makes the difference
///    visible.
/// 3. **A root pass that alone spends the cap STARVES the decision**:
///    `budget_starved` is recorded and the decision falls back to
///    [`best_available_pick`] over the pass's one-ply values — the argmax over
///    the WHOLE family with a keyed draw over its ties, **never first-listed**
///    (ledger row 18: at a Placement root the first-listed candidate is the
///    empty placement, so a first-wins fallback is the null-placement degeneracy
///    arriving through the budget).
///
/// A forced decision point (≤ 1 candidate) pays none of this: it has nothing to
/// allocate and nothing to rank, so the root pass would be work whose result is
/// never read — spec §3.4's pass is "before any deeper work", and there is none.
///
/// `budget` is borrowed rather than owned because §0.3's currency is total
/// scored successors **per decision**; at the shipped `K = 1` the per-world and
/// per-decision totals coincide.
pub fn search_world_budgeted(
    world: &GameState,
    seat: PlayerId,
    ctx: &McCtx<'_>,
    budget: &mut ScoredBudget,
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
    let mut search =
        Search { ctx, seat, nodes: Vec::new(), stats: McSearchStats::default(), budget };

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

    let mut one_ply: Vec<Option<i64>> = vec![None; search.nodes[ROOT].candidates.len()];
    let mut priors: Vec<Option<f64>> = vec![None; one_ply.len()];
    let mut starved = false;
    if !forced {
        // THE MANDATORY ROOT PASS (spec §3.4), ungated by construction.
        let (raws, unit) = search.score_candidates(ROOT)?;
        search.stats.root_scored = raws.len() as u64;
        one_ply = raws.into_iter().map(Some).collect();
        priors = unit.iter().copied().map(Some).collect();
        search.nodes[ROOT].plan = Plan::Ordered { priors: unit };

        // The knob floor, stated once: a decision over zero simulations is not a
        // search. (`squash` floors its own scale for the same reason.)
        let want = ctx.simulations.max(1);
        // "The ROOT PASS alone spent the cap" reads the meter, so it is a
        // statement about THIS decision only while the meter starts this
        // decision at zero. At the shipped `K = 1` it does, because `choose`
        // builds one budget per decision; a K > 1 that threaded one partially
        // spent meter across worlds would attribute a previous world's spend to
        // this world's pass. Unreachable rather than guarded: `choose` REFUSES
        // K > 1 outright (spec §0.4).
        starved = !search.budget.may_start();
        let (mut ran, mut sterile) = (0u32, 0u32);
        while ran < want && sterile < MAX_STERILE_SIMULATIONS {
            if !search.budget.may_start() {
                search.stats.budget_binds = 1;
                break;
            }
            let before = search.budget.realized();
            search.simulate()?;
            ran += 1;
            // Sterile: the descent ended at an existing terminal node, charging
            // nothing. See MAX_STERILE_SIMULATIONS — without this the budget
            // cannot bound the loop, because there is nothing for it to bound.
            if search.budget.realized() == before {
                sterile += 1;
            }
        }
        search.stats.simulations = u64::from(ran);
        search.stats.sterile_simulations = u64::from(sterile);
        if starved {
            search.stats.budget_starved = 1;
        }
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
            RootStat { mv: mv.clone(), visits, value_sum, one_ply: one_ply[i], prior: priors[i] }
        })
        .collect();
    // `roots` is NON-EMPTY at both picks below, and the guarantee is structural
    // rather than incidental (T1 review NOTE 4, discharged at this call site
    // rather than by widening `best_available_pick`'s signature for its other
    // caller): `expand_point` REFUSES a decision point whose candidate scheme
    // returns nothing, so a search that reaches this line has at least one root
    // candidate.
    let chosen = if starved {
        // Rung 2 of spec §3.4's chain: the visit counts are all zero here, so an
        // argmax over them would be a uniform draw over the whole family — the
        // one-ply values the mandatory pass already paid for are strictly more
        // information, and they are what the fallback ranks.
        best_available_pick(&one_ply, ctx.stream.word(McSite::RootFallback))
    } else {
        let visits: Vec<u32> = roots.iter().map(|r| r.visits).collect();
        root_argmax(&visits, ctx.stream.word(McSite::RootArgmax))
    };
    Ok(McSearch { roots, chosen, volume, stats: search.stats })
}

// ---------------------------------------------------------------------------
// The explanation (spec §3.5)
// ---------------------------------------------------------------------------

/// The quantity a decision was READ OUT on — which is not always the visits,
/// and an explanation that said "visits" at a starved decision would be
/// describing a search that never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McReadout {
    /// A FORCED decision point: one candidate, no tree, nothing ranked.
    Forced,
    /// The search's read-out: the argmax over VISITS, ties drawn from
    /// [`McSite::RootArgmax`].
    Visits,
    /// The STARVED fallback (spec §3.4): the argmax over the mandatory root
    /// pass's one-ply values, ties drawn from [`McSite::RootFallback`]. Every
    /// visit count is 0 here, so a visit margin would be a statement about
    /// nothing.
    OnePly,
}

impl McReadout {
    /// The word the rendering prints — stable, because a report's bytes are
    /// pinned.
    fn label(self) -> &'static str {
        match self {
            Self::Forced => "forced",
            Self::Visits => "visits",
            Self::OnePly => "one-ply (starved)",
        }
    }
}

/// How far ahead the chosen candidate finished, in the currency that DECIDED —
/// and, where the deciding currency is not the value, the value lead beside it.
///
/// **The rule for a lead nobody can compute is `None`, never zero** (the reason
/// [`RootStat::mean`] is itself an `Option`): a candidate no simulation reached
/// has no mean, and reporting `0.0` — or the axis mid-point — against it would
/// claim a comparison that was never made. The VISIT lead is always defined,
/// because visits are counted for every candidate whether or not any landed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum McMargin {
    /// A forced point: one candidate, no runner-up to be ahead of.
    Forced,
    /// The visit read-out. `visits` is the deciding lead — SIGNED: under the
    /// shipped chain the chosen row IS the visit argmax so it is non-negative,
    /// but [`McExplanation::new`] is public and takes constructed rows, and a
    /// clamped `+0` beside a negative value column would assert a tie rule this
    /// module does not have (T4 review S1). `value` is the same lead in mean
    /// normalized value, `None` when either side has no mean.
    Visits { runner_up: usize, visits: i64, value: Option<f64> },
    /// The starved read-out. The lead is in RAW [`eval::score`] units — the
    /// currency the fallback ranked on — and `None` if either side carries no
    /// one-ply value (unreachable from a starved search, whose pass is
    /// mandatory; expressible because [`McExplanation::new`] is public).
    OnePly { runner_up: usize, one_ply: Option<i64> },
}

/// How many rows [`McExplanation::render`] prints before summarizing the rest.
///
/// A REPORTING cap and nothing else — [`McExplanation::rows`] always carries
/// every candidate, and a 186-candidate Placement root is a table no reader
/// wants in full beside a decision.
pub const RENDERED_ROWS: usize = 8;

/// Spec §3.5's ROOT TABLE: the decision as the search left it, per candidate,
/// plus which candidate was taken and by how much.
///
/// It is a pure PROJECTION of one [`McSearch`] — no re-search, no re-evaluation,
/// no second enumeration — so producing it cannot change what was played and
/// cannot cost the budget anything. [`MctsAgent`] keeps exactly one (the last
/// decision's) and reads [`last_roots`](MctsAgent::last_roots) out of it, which
/// is why nothing here is cloned per decision.
#[derive(Debug, Clone, PartialEq)]
pub struct McExplanation {
    /// Every root candidate, in the order the candidate scheme offered them —
    /// the SAME order the search's `roots` carry, so a row index is a candidate
    /// index everywhere in this module.
    pub rows: Vec<RootStat>,
    /// The index in [`rows`](Self::rows) the decision landed on.
    pub chosen: usize,
    pub readout: McReadout,
    pub margin: McMargin,
    /// What the decision cost — carried so a table is self-contained: a reader
    /// must be able to see that a one-row-per-candidate report came out of 12
    /// simulations rather than 750.
    pub stats: McSearchStats,
}

impl McExplanation {
    /// Build the table from a decision's roots.
    ///
    /// The read-out is DERIVED from the data rather than passed in: a one-row
    /// table is forced, a table whose search starved reads out on one-ply
    /// values, everything else reads out on visits — the same three cases
    /// [`search_world_budgeted`] decides by, so the two cannot disagree about
    /// which rule took the decision.
    pub fn new(rows: Vec<RootStat>, chosen: usize, stats: McSearchStats) -> Self {
        debug_assert!(
            chosen < rows.len(),
            "a decision must land on one of its own candidates ({chosen} of {})",
            rows.len(),
        );
        // Release totality for a public constructor (T4 review N5): an
        // out-of-range `chosen` clamps into the table rather than indexing past
        // it two branches later; the empty table clamps to 0 and reads Forced.
        let chosen = chosen.min(rows.len().saturating_sub(1));
        let readout = if rows.len() <= 1 {
            McReadout::Forced
        } else if stats.budget_starved > 0 {
            McReadout::OnePly
        } else {
            McReadout::Visits
        };
        let margin = match (readout, runner_up_of(&rows, chosen, readout)) {
            (McReadout::Forced, _) | (_, None) => McMargin::Forced,
            (McReadout::Visits, Some(r)) => McMargin::Visits {
                runner_up: r,
                // Signed, not saturating: under this read-out the chosen
                // candidate IS the visit argmax so this is non-negative on the
                // shipped chain — but `new` is public, and a constructed
                // trailing chosen must report its deficit honestly.
                visits: i64::from(rows[chosen].visits) - i64::from(rows[r].visits),
                value: match (rows[chosen].mean(), rows[r].mean()) {
                    (Some(a), Some(b)) => Some(a - b),
                    _ => None,
                },
            },
            (McReadout::OnePly, Some(r)) => McMargin::OnePly {
                runner_up: r,
                one_ply: match (rows[chosen].one_ply, rows[r].one_ply) {
                    (Some(a), Some(b)) => Some(a.saturating_sub(b)),
                    _ => None,
                },
            },
        };
        Self { rows, chosen, readout, margin, stats }
    }

    /// The REPORTING order: the chosen row first, then the rest by the deciding
    /// currency descending, ties by candidate order.
    ///
    /// **Not a selecting point.** It runs after the decision, consumes no
    /// randomness, and no move depends on it — which is why its tie rule is
    /// "lowest candidate index" and not a keyed draw (row 18's discipline
    /// governs picks that move play, and this one cannot). The chosen row leads
    /// unconditionally so a truncated rendering can never omit the row it exists
    /// to explain: under the visit read-out the chosen candidate is an argmax,
    /// but the keyed draw may have settled a 64-wide tie on a late candidate.
    ///
    /// `ranked()[1]` is the runner-up [`margin`](Self::margin) names, by
    /// construction — both read the same key.
    pub fn ranked(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        order.sort_by(|&a, &b| {
            (b == self.chosen)
                .cmp(&(a == self.chosen))
                .then_with(|| match self.readout {
                    // `Option<i64>`'s own order puts `None` LAST under this
                    // reversal, which is where a candidate with no value belongs.
                    McReadout::OnePly => self.rows[b].one_ply.cmp(&self.rows[a].one_ply),
                    _ => self.rows[b].visits.cmp(&self.rows[a].visits),
                })
                .then(a.cmp(&b))
        });
        order
    }

    /// The table as text: one line per candidate in [`ranked`](Self::ranked)
    /// order, capped at [`RENDERED_ROWS`], then the margin.
    ///
    /// **Byte-identical for the same search** — every number is formatted at a
    /// fixed precision and the move is its `Debug` form (the same rendering this
    /// module's determinism pins compare on). Absent values print `-`; nothing
    /// here substitutes a zero for a measurement that was not taken.
    pub fn render(&self) -> String {
        use std::fmt::Write as _;

        let cell_f = |v: Option<f64>| v.map_or_else(|| "-".to_string(), |x| format!("{x:.4}"));
        let cell_i = |v: Option<i64>| v.map_or_else(|| "-".to_string(), |x| x.to_string());

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{MCTS_CONTROLLER} root table: {} candidates, {} simulations, {} scored ({} root), \
             read-out {}",
            self.rows.len(),
            self.stats.simulations,
            self.stats.scored,
            self.stats.root_scored,
            self.readout.label(),
        );
        let _ = writeln!(
            out,
            "  {:>5} {:>7} {:>9} {:>9} {:>8}  move",
            "#", "visits", "mean", "one-ply", "prior",
        );
        let order = self.ranked();
        for &i in order.iter().take(RENDERED_ROWS) {
            let r = &self.rows[i];
            let _ = writeln!(
                out,
                "{} {:>5} {:>7} {:>9} {:>9} {:>8}  {:?}",
                if i == self.chosen { ">" } else { " " },
                i,
                r.visits,
                cell_f(r.mean()),
                cell_i(r.one_ply),
                cell_f(r.prior),
                r.mv,
            );
        }
        if let Some(rest) = order.len().checked_sub(RENDERED_ROWS).filter(|n| *n > 0) {
            let _ = writeln!(out, "  ... and {rest} more candidates");
        }
        let _ = match self.margin {
            McMargin::Forced => writeln!(out, "  forced decision point: no runner-up, no margin"),
            McMargin::Visits { runner_up, visits, value } => writeln!(
                out,
                "  margin over runner-up (candidate {runner_up}): {visits:+} visits, {} value",
                value.map_or_else(|| "unmeasured".to_string(), |v| format!("{v:+.4}")),
            ),
            McMargin::OnePly { runner_up, one_ply } => writeln!(
                out,
                "  margin over runner-up (candidate {runner_up}): {} one-ply",
                one_ply.map_or_else(|| "unmeasured".to_string(), |v| format!("{v:+}")),
            ),
        };
        out
    }
}

/// The best candidate OTHER than `chosen` under `readout`'s currency, ties to
/// the lowest candidate index — a linear scan rather than
/// [`McExplanation::ranked`]'s sort, because this one runs on every decision
/// and the sort runs only when somebody reads the table.
fn runner_up_of(rows: &[RootStat], chosen: usize, readout: McReadout) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, row) in rows.iter().enumerate() {
        if i == chosen {
            continue;
        }
        // Strict `>`, so an equal candidate never displaces an earlier one —
        // the tie rule `ranked` sorts by.
        let ahead = best.is_none_or(|b| match readout {
            McReadout::OnePly => row.one_ply > rows[b].one_ply,
            _ => row.visits > rows[b].visits,
        });
        if ahead {
            best = Some(i);
        }
    }
    best
}

// ---------------------------------------------------------------------------
// The agent
// ---------------------------------------------------------------------------

/// The per-decision counters one [`MctsAgent`] keeps, behind one `Arc`.
#[derive(Debug, Default)]
struct McCounters {
    decisions: AtomicU64,
    explained: AtomicU64,
    scored: AtomicU64,
    root_scored: AtomicU64,
    budget_binds: AtomicU64,
    budget_starved: AtomicU64,
    placement_cap_binds: AtomicU64,
    completion_cap_hits: AtomicU64,
    /// Per-DECISION overshoot samples — the one counter here that is not a
    /// running total, because spec §3.4 reports the overshoot as a p50/p90/max
    /// distribution and a distribution cannot be recovered from a sum. An
    /// UNCAPPED agent pushes nothing, so it pays neither the allocation nor the
    /// lock.
    overshoot: Mutex<Vec<u64>>,
}

/// A cloneable handle on one [`MctsAgent`]'s counters —
/// [`crate::game::lookahead::SearchTally`]'s pattern and its reason: a game's
/// agents are built by the driver's factory and never handed back, so cloning
/// the handle BEFORE the agent moves into the driver is the only way to read
/// them afterwards.
///
/// A separate TYPE from `SearchTally` rather than a shared one, on that type's
/// own grounds: the counter SETS differ (a tree has no continuation length and
/// no beam-ply guard; a beam has no simulations), and a shared type would carry
/// fields one of its owners never writes.
///
/// **It is ALSO the explanation tally, deliberately — one handle, not two**
/// (spec §3.5). `greedy`'s `ExplanationTally` and `lookahead`'s `SearchTally`
/// each carry their own explanation count for the same reason, and the criterion
/// those two types state for splitting is a differing counter SET: there is no
/// second set here, because an MCTS explanation is a pure projection of the
/// search result [`MctsAgent::choose`] already holds (no extra apply, no extra
/// evaluation, no counters of its own). A second type would give the arena two
/// handles to clone per seat where one carries everything T5 surfaces —
/// [`explained`](Self::explained) beside the budget meters.
#[derive(Clone, Default)]
pub struct McTally {
    counters: Arc<McCounters>,
}

impl McTally {
    /// Decisions this agent took.
    pub fn decisions(&self) -> u64 {
        self.counters.decisions.load(Ordering::Relaxed)
    }

    /// Decisions this agent EXPLAINED — spec §3.5's counter, and the one the
    /// gate reads as `x_explanations == Some(x_applied_moves)`.
    ///
    /// **Equal to [`decisions`](Self::decisions) by construction at the shipped
    /// [`choose`](MctsAgent::choose)**, which has no path to a `DecisionReport`
    /// that skips the table — pinned rather than merely stated, because the two
    /// answer different questions (a decision TAKEN and a decision EXPLAINED)
    /// and the pin is what makes a future path that returns one without the
    /// other fail loudly instead of quietly halving the gate's clause.
    ///
    /// A FAILING `choose` increments neither: both are written only on the
    /// success path, so the counters are immune to the staleness
    /// [`MctsAgent::last_explanation`] documents.
    pub fn explained(&self) -> u64 {
        self.counters.explained.load(Ordering::Relaxed)
    }

    /// Successors applied, observed and scored across every decision — **the
    /// budget currency** (spec §0.3), root pass included, so a cell's realized
    /// total is this counter over [`decisions`](Self::decisions).
    pub fn candidates_scored(&self) -> u64 {
        self.counters.scored.load(Ordering::Relaxed)
    }

    /// The ROOT SHARE of [`candidates_scored`](Self::candidates_scored):
    /// successors scored by the mandatory root pass, before any simulation.
    pub fn root_scored(&self) -> u64 {
        self.counters.root_scored.load(Ordering::Relaxed)
    }

    /// Decisions in which the cap DENIED work — at least one simulation could
    /// not start. **0 for an uncapped agent by construction**, which is also the
    /// shape of the experiment's validity condition: a cap that never binds is
    /// not a matched budget.
    pub fn budget_binds(&self) -> u64 {
        self.counters.budget_binds.load(Ordering::Relaxed)
    }

    /// Decisions whose mandatory root pass alone spent the cap, so no simulation
    /// could start and the decision fell back to the root priors (spec §3.4). A
    /// sub-case of [`budget_binds`](Self::budget_binds), and the counter spec
    /// §0.1's S2 surface routes on.
    pub fn budget_starved_decisions(&self) -> u64 {
        self.counters.budget_starved.load(Ordering::Relaxed)
    }

    /// Decisions whose placement family exceeded the candidate cap, summed over
    /// nodes (a tree can bind it several times in one decision).
    pub fn placement_cap_binds(&self) -> u64 {
        self.counters.placement_cap_binds.load(Ordering::Relaxed)
    }

    /// Hand-off completions [`MAX_TURN_PLIES`] stopped, summed over every
    /// decision — **this module's analogue of
    /// [`SearchTally::ply_cap_hits`](crate::game::lookahead::SearchTally::ply_cap_hits),
    /// the same guard and the same constant**, and the counter the arena's
    /// `x_ply_cap_hits` reads for an MCTS seat.
    ///
    /// **Expected 0, and readable so a run can SAY so.** A hit is not a wrong
    /// answer but it is a quietly different one: the leaf is valued where the
    /// guard stopped it, MID-TURN, which is the one thing
    /// [`LeafRule::HandOff`] exists to avoid — so a cell that starts hitting it
    /// is measuring a slightly different leaf rule than the one the figures were
    /// set with. 0 by construction under every other leaf rule, which runs no
    /// completion at all.
    pub fn completion_cap_hits(&self) -> u64 {
        self.counters.completion_cap_hits.load(Ordering::Relaxed)
    }

    /// The per-decision overshoot distribution (p50/p90/max over
    /// `realized − cap`), or `None` if this agent never took a capped decision.
    ///
    /// `None` rather than zeros: an uncapped run has no overshoot distribution
    /// and printing one would claim a measurement nobody took. It is nonzero
    /// exactly where the MANDATORY root pass exceeded the cap, or where the last
    /// started simulation ran past it — the two places spec §3.4 lets the meter
    /// run past its gate.
    pub fn overshoot(&self) -> Option<OvershootSummary> {
        let samples = self.counters.overshoot.lock().expect("overshoot samples");
        summarize_overshoot(&samples)
    }

    /// Note that one decision was explained. Separate from
    /// [`record`](Self::record) because it is a statement about the explanation
    /// surface rather than about what the search paid — and because a merged
    /// call could not tell "the search finished" from "the decision was
    /// explained", which is the whole content of the identity above.
    fn explain_one(&self) {
        self.counters.explained.fetch_add(1, Ordering::Relaxed);
    }

    /// Fold one decision's search into the running counters.
    fn record(&self, stats: &McSearchStats, budget: &ScoredBudget) {
        self.counters.decisions.fetch_add(1, Ordering::Relaxed);
        self.counters.scored.fetch_add(stats.scored, Ordering::Relaxed);
        self.counters.root_scored.fetch_add(stats.root_scored, Ordering::Relaxed);
        self.counters.budget_binds.fetch_add(stats.budget_binds, Ordering::Relaxed);
        self.counters.budget_starved.fetch_add(stats.budget_starved, Ordering::Relaxed);
        self.counters.placement_cap_binds.fetch_add(stats.placement_cap_binds, Ordering::Relaxed);
        self.counters.completion_cap_hits.fetch_add(stats.completion_cap_hits, Ordering::Relaxed);
        if budget.cap().is_some() {
            self.counters.overshoot.lock().expect("overshoot samples").push(budget.overshoot());
        }
    }
}

/// MCTS-v1 as a seat: the honest chain, then the tree.
///
/// The arena's [`ArenaAgent`] impl at the bottom of this section DELEGATES to
/// the inherent [`choose`](Self::choose) /
/// [`choose_starting_order`](Self::choose_starting_order) below — they are the
/// implementation, the trait is the seam, and neither can drift from the other.
pub struct MctsAgent {
    worlds: u8,
    retreat_samples: u8,
    simulations: u32,
    exploration_ppm: u32,
    squash_scale: i64,
    leaf: LeafRule,
    /// Spec §3.4's scored-successor cap. **`None` is the uncapped shape**: an
    /// absent cap threads an unlimited [`ScoredBudget`], which gates nothing,
    /// starves nothing and records no overshoot sample.
    cap: Option<u32>,
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
    counters: McTally,
    /// The last decision's table — and the agent's ONLY per-decision storage:
    /// [`last_stats`](Self::last_stats) and [`last_roots`](Self::last_roots) are
    /// served out of it rather than kept beside it, so the three cannot describe
    /// different decisions and an explained decision costs no second copy of a
    /// root family that can run to 186 candidates.
    last_explanation: Option<McExplanation>,
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
            cap: None,
            weights: WeightTable::V1,
            rng: ChaCha8Rng::seed_from_u64(agent_seed),
            agent_seed,
            keep_scores: rules
                .orders
                .iter()
                .map(|card| (card.id, score_starting_order(card, rules)))
                .collect(),
            counters: McTally::default(),
            last_explanation: None,
        }
    }

    /// MCTS-v1 at an explicit knob set — the arena contestant's constructor
    /// ([`LookaheadAgent::with_knobs`](crate::game::lookahead::LookaheadAgent::with_knobs)'s
    /// shape), so a cell builds its seat and its descriptor from ONE value.
    pub fn with_knobs(rules: &Rules, agent_seed: u64, knobs: McKnobs) -> Self {
        Self::new(rules, agent_seed)
            .with_cap(knobs.cap)
            .with_exploration_ppm(knobs.exploration_ppm)
            .with_squash_scale(knobs.squash_scale)
            .with_retreat_samples(knobs.retreat_samples)
            .with_leaf(knobs.leaf)
            .with_simulations(knobs.simulations)
            .with_worlds(knobs.worlds)
    }

    /// The knob set this agent will actually run — the round trip that makes
    /// "the descriptor names the search" checkable rather than asserted.
    pub fn knobs(&self) -> McKnobs {
        McKnobs {
            cap: self.cap,
            exploration_ppm: self.exploration_ppm,
            squash_scale: self.squash_scale,
            retreat_samples: self.retreat_samples,
            leaf: self.leaf,
            simulations: self.simulations,
            worlds: self.worlds,
        }
    }

    /// Spec §3.4's TOTAL scored-successor cap for one decision, root pass
    /// included. `None` is uncapped.
    pub fn with_cap(mut self, cap: Option<u32>) -> Self {
        self.cap = cap;
        self
    }

    /// This agent's scored-successor cap, if it has one.
    pub fn cap(&self) -> Option<u32> {
        self.cap
    }

    /// A handle on this agent's running counters — clone it BEFORE the agent
    /// moves into a driver.
    pub fn tally(&self) -> McTally {
        self.counters.clone()
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

    /// The last EXPLAINED decision's counters.
    ///
    /// **Stale after a failing `choose`, deliberately** — see
    /// [`last_explanation`](Self::last_explanation) for the rule and its reason.
    pub fn last_stats(&self) -> Option<&McSearchStats> {
        self.last_explanation.as_ref().map(|e| &e.stats)
    }

    /// EVERY root candidate of the last explained decision with its visits and
    /// backed-up value, in the order the candidate scheme offered them — the
    /// explanation's own rows, served rather than duplicated.
    ///
    /// **Stale after a failing `choose`**, on
    /// [`last_explanation`](Self::last_explanation)'s rule.
    pub fn last_roots(&self) -> &[RootStat] {
        self.last_explanation.as_ref().map_or(&[], |e| e.rows.as_slice())
    }

    /// Spec §3.5's ROOT TABLE for the last decision this agent explained.
    ///
    /// **A FAILING [`choose`](Self::choose) leaves the PREVIOUS decision's table
    /// in place rather than clearing it** — `lookahead`'s precedent (T2 review
    /// NOTE 9), and the honest reading is that this accessor answers "the last
    /// decision that succeeded", not "the last call". A caller pairing a table
    /// with a move must therefore read it after an `Ok`, which every in-tree
    /// caller does because a failed decision has no move to pair it with. The
    /// COUNTERS are immune: [`McTally::explained`] is written only on the
    /// success path, so nothing downstream has to reason about staleness to
    /// trust the identity the gate asserts.
    ///
    /// Not on the clone-out [`tally`](Self::tally): a table owns a `Vec` of
    /// every root candidate, and the caller that cannot hold the agent (the
    /// arena driver) wants the COUNT.
    pub fn last_explanation(&self) -> Option<&McExplanation> {
        self.last_explanation.as_ref()
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
        let mut budget = ScoredBudget::from_cap(self.cap);
        let searched = search_world_budgeted(&world, seat, &ctx, &mut budget)?;
        self.counters.record(&searched.stats, &budget);

        // Spec §3.5: the decision is EXPLAINED always, and there is no path to a
        // `DecisionReport` that skips the table (greedy's §0.3(b) rule, and the
        // cheaper half of it — the table is a projection of the search that just
        // ran, so it costs no apply, no evaluation and no budget).
        let mv = searched.roots[searched.chosen].mv.clone();
        let volume = searched.volume;
        self.last_explanation = Some(searched.into_explanation());
        self.counters.explain_one();
        Ok(DecisionReport { mv, volume })
    }
}

/// The arena seam (spec §3.6). Both play methods DELEGATE to the inherent ones
/// above rather than restating them — the T2/T4 doc's stated plan, and the
/// reason the tree's own tests can drive the agent directly without going
/// through a trait object.
impl ArenaAgent for MctsAgent {
    fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        MctsAgent::choose_starting_order(self, seat, deal)
    }

    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        MctsAgent::choose(self, view, rules)
    }

    /// Delegated to [`controller_provenance`] so the agent and the arena's
    /// contestant read [`MCTS_CONTROLLER`] and [`MCTS_IMPL_VERSION`] at one
    /// site, and so a hand-built agent describes ITS OWN knobs (the
    /// [`knobs`](Self::knobs) round trip) rather than the ones a cell would have
    /// derived.
    fn controller(&self) -> ControllerProvenance {
        controller_provenance(self.knobs(), &self.weights, Some(self.agent_seed))
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

    /// The fixture context, on the **`MidTurnDiag`** leaf — a DELIBERATE choice
    /// carried over from T2 rather than an oversight (T3 moved the DEFAULT to
    /// [`LeafRule::HandOff`]).
    ///
    /// The tree pins below are about the tree — handles per node, keyed draws,
    /// counter identities, the descent — and the diagnostic leaf costs one
    /// evaluation where the shipped one costs a whole turn's completion, so this
    /// keeps them at the price T2 measured them at and keeps what they pin
    /// undiluted. The SHIPPED leaf rule is pinned where it belongs: exactly at
    /// [`hand_off_value`](Search::hand_off_value)'s own tests, on the counter
    /// identities, and end to end through the agent (whose default is untouched
    /// by this helper) in the determinism and stream-word pins.
    fn ctx<'a>(
        rules: &'a Rules,
        weights: &'a WeightTable,
        simulations: u32,
        key: u64,
    ) -> McCtx<'a> {
        ctx_leaf(rules, weights, simulations, key, LeafRule::MidTurnDiag)
    }

    fn ctx_leaf<'a>(
        rules: &'a Rules,
        weights: &'a WeightTable,
        simulations: u32,
        key: u64,
        leaf: LeafRule,
    ) -> McCtx<'a> {
        McCtx {
            rules,
            weights,
            retreat_samples: DEFAULT_RETREAT_SAMPLES,
            simulations,
            exploration_ppm: DEFAULT_EXPLORATION_PPM,
            squash_scale: DEFAULT_SQUASH_SCALE,
            leaf,
            stream: McStream::new(key),
            candidate_seed: 0xC0FFEE,
        }
    }

    /// A [`Search`] over an empty tree, for the white-box cells that test one
    /// piece of the machinery (the leaf rule, the ordering pass) without a
    /// decision around it.
    fn search_over<'a>(
        ctx: &'a McCtx<'a>,
        seat: PlayerId,
        budget: &'a mut ScoredBudget,
    ) -> Search<'a> {
        Search { ctx, seat, nodes: Vec::new(), stats: McSearchStats::default(), budget }
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
        // The agent's DEFAULT leaf rule throughout, mirrored below — a mirror on
        // a different leaf would reproduce the move by luck at any point whose
        // two rules agree, and would then pin nothing.
        let mut agent = MctsAgent::new(&rules, seed).with_simulations(16);
        assert_eq!(agent.leaf(), LeafRule::default(), "the mirror must run the shipped rule");
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
            leaf: LeafRule::default(),
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

    /// The decision point `back` moves before the `MASTER` walk's game ENDS —
    /// the one shape a "the tree ran out of game" fixture can be stated as, and
    /// robust in a way a state predicate is not (the game ends by a particular
    /// action, so no state-level test says "this is nearly over").
    fn point_before_the_end(rules: &Rules, back: usize) -> (GameState, PlayerId) {
        let pending = new_game(GameConfig::for_player_count(2), rules, MASTER).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut agent = GreedyAgent::new(rules, 0x77);
        let mut seen: Vec<GameState> = Vec::new();
        while !state.is_game_over() && seen.len() < 2_000 {
            seen.push(state.clone());
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let r = agent.choose(&view, rules).expect("move");
            state = apply_move_unchecked(&state, &r.mv, rules).expect("apply");
        }
        assert!(state.is_game_over(), "the walk must reach the end of the game");
        assert!(seen.len() > back, "the game must be longer than {back} decisions");
        let at = seen.swap_remove(seen.len() - back);
        let seat = at.current_player;
        let view = observe(&at, seat).expect("observe");
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

    /// Play a whole 2P game with `agent` on BOTH seats, returning
    /// `(moves applied, assistants placed)`.
    ///
    /// The assistant count comes off the APPLIED MOVES rather than the final
    /// state — `crate::game::greedy`'s helper and its reason: a final state may
    /// have recovered every assistant it ever placed, so reading the board would
    /// report zero for a game that placed hundreds.
    fn drive_to_the_end(
        agent: &mut MctsAgent,
        world_seed: u64,
        rules: &Rules,
        move_cap: u32,
    ) -> (u32, usize) {
        let pending =
            new_game(GameConfig::for_player_count(2), rules, world_seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let (mut moves, mut placed) = (0u32, 0usize);
        while !state.is_game_over() && moves < move_cap {
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let report = agent.choose(&view, rules).expect("the honest chain produces a move");
            match &report.mv {
                Move::PlaceThreeAreas { placements } => placed += placements.len(),
                Move::PlaceTwoSameArea { adjacent_bonus, .. } => {
                    placed += 2 + usize::from(adjacent_bonus.is_some());
                }
                _ => {}
            }
            state = apply_move_unchecked(&state, &report.mv, rules).expect("a chosen move applies");
            moves += 1;
        }
        (moves, placed)
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

    // -- priors and ordered expansion (spec §2.3, §3.2) -----------------------

    /// **THE PRIOR SEMANTICS, on constructed slots** — no tree, no game.
    ///
    /// An unexpanded candidate competes as a virtual single-visit child carrying
    /// its prior, so a prior and a backed-up mean are comparable AT THE SAME
    /// exploration weight. The second cell is the one T2's scheme could not
    /// express at all: an unvisited child was `+∞` there, so a good child could
    /// never be re-visited while any candidate remained unexpanded — the star
    /// tree, in one assertion.
    #[test]
    fn an_unexpanded_candidate_competes_on_its_prior_at_the_same_exploration_weight() {
        let root = PlayerId(0);
        let opponent = PlayerId(1);
        let c = std::f64::consts::SQRT_2;

        let slots = [
            Slot::Child(ChildStat { visits: 1, value_sum: 0.30 }),
            Slot::Unexpanded { prior: 0.80 },
            Slot::Unexpanded { prior: 0.10 },
        ];
        assert_eq!(
            ordered_select(&slots, 4, root, root, c, 0),
            1,
            "a strong prior must outrank a weak backed-up mean",
        );

        let slots = [
            Slot::Child(ChildStat { visits: 1, value_sum: 0.90 }),
            Slot::Unexpanded { prior: 0.40 },
        ];
        assert_eq!(
            ordered_select(&slots, 4, root, root, c, 0),
            0,
            "and a strong mean must be RE-VISITED rather than a weaker candidate expanded — \
             the concentration T2's `+∞` unvisited child made impossible",
        );
        assert_eq!(
            ordered_select(&slots, 4, opponent, root, c, 0),
            1,
            "the opponent flips priors and means alike",
        );

        // Exploration still discounts a heavily-visited child: the prior wins on
        // the bonus even though the mean is higher.
        let slots =
            [Slot::Child(ChildStat { visits: 50, value_sum: 30.0 }), Slot::Unexpanded { prior: 0.55 }];
        assert_eq!(
            ordered_select(&slots, 100, root, root, c, 0),
            1,
            "UCB1 is still UCB1 — a 50-visit child at 0.60 loses its bonus to an unexplored 0.55",
        );

        // And the tie draw reaches the FULL tied set, priors and children mixed.
        let tied = [
            Slot::Unexpanded { prior: 0.5 },
            Slot::Child(ChildStat { visits: 1, value_sum: 0.5 }),
            Slot::Unexpanded { prior: 0.5 },
            Slot::Child(ChildStat { visits: 1, value_sum: 0.5 }),
        ];
        let stream = McStream::new(0xC0DE);
        let mut seen = std::collections::BTreeSet::new();
        for visit in 0..400u32 {
            let word = stream.word(McSite::Ucb { branch: 0x51DE, visit });
            seen.insert(ordered_select(&tied, 8, root, root, c, word));
        }
        assert_eq!(seen.len(), 4, "every tied slot must be reachable, saw {seen:?}");
    }

    /// The ordering pass fires ABOVE [`ORDERING_THRESHOLD`] and never below it,
    /// costs exactly one scored successor per candidate, and is computed AT MOST
    /// ONCE per node.
    #[test]
    fn the_ordering_pass_fires_only_above_the_threshold_and_only_once() {
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        let (state, seat) = placement_point(&rules);
        let handle = DecisionHandle::new(&state, &rules);
        let all = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut ChaCha8Rng::seed_from_u64(3))
            .moves;
        assert!(all.len() > ORDERING_THRESHOLD + 1, "the fixture must exceed the threshold");

        for n in [ORDERING_THRESHOLD, ORDERING_THRESHOLD + 1] {
            let c = ctx(&rules, &weights, 1, 0x1);
            let mut budget = ScoredBudget::unlimited();
            let mut search = search_over(&c, seat, &mut budget);
            let at = search.push_expandable(state.clone(), 0, all[..n].to_vec(), 0.0);
            search.ensure_plan(at).expect("a plan");
            match &search.nodes[at].plan {
                Plan::Ordered { priors } => {
                    assert!(n > ORDERING_THRESHOLD, "{n} candidates must not have been ordered");
                    assert_eq!(priors.len(), n, "one prior per candidate");
                    assert!(
                        priors.iter().all(|p| (0.0..=1.0).contains(p)),
                        "priors must sit on the declared axis: {priors:?}",
                    );
                    assert!(
                        priors.iter().any(|p| *p != priors[0]),
                        "anti-vacuity: the priors must separate the candidates",
                    );
                    assert_eq!(search.stats.ordering_passes, 1);
                    assert_eq!(search.stats.ordering_scored, n as u64);
                    assert_eq!(search.stats.scored, n as u64, "and the meter saw all of it");
                }
                Plan::Uniform => {
                    assert!(n <= ORDERING_THRESHOLD, "{n} candidates must have been ordered");
                    assert_eq!(search.stats.scored, 0, "a small node pays nothing to plan");
                    assert_eq!(search.stats.ordering_passes, 0);
                }
                Plan::Deferred => panic!("ensure_plan must leave no node deferred"),
            }
            let spent = search.stats.scored;
            search.ensure_plan(at).expect("a plan");
            assert_eq!(search.stats.scored, spent, "a node's plan is computed AT MOST ONCE");
        }
    }

    /// **THE STAR-TREE FIX, made executable** (the T2 review's finding: at a
    /// 186-candidate Placement root every non-forced point was settled by the
    /// root tie draw over a uniformly-drawn 64 of the family, because expansion
    /// was a uniform draw with no priors).
    ///
    /// Two properties, and neither can hold under T2's scheme:
    ///
    /// 1. **Expansion is EVAL-ORDERED**: the expanded set is a PREFIX of the
    ///    descending prior order — every visited candidate's prior is ≥ every
    ///    unvisited one's. (Uniform expansion breaks this on the first draw.)
    /// 2. **Values CONCENTRATE**: some child is visited more than once while
    ///    candidates remain unexpanded. (T2 expanded every slot before any
    ///    re-visit, so its max was exactly 1 here.)
    ///
    /// The module doc records what these two do and do not buy at the SHIPPED
    /// `(c, scale)`: the tie-set shrinks and the decision becomes the top of the
    /// one-ply ranking, while the tie RATE at 64 simulations does not move,
    /// because that pair is exploration-dominated. This cell pins the mechanism,
    /// not the knob.
    ///
    /// BOTH leaf rules, at the T3 review's finding: the priors are the one-ply
    /// rule's either way, but WHICH children get re-visited is the leaf's, so a
    /// cell run only at the diagnostic leaf would leave the star-tree property
    /// unpinned exactly where it ships.
    #[test]
    fn expansion_is_eval_ordered_and_the_visits_concentrate() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        for leaf in [LeafRule::MidTurnDiag, LeafRule::HandOff] {
            let c = ctx_leaf(&rules, &WeightTable::V1, 64, 0xB1A5, leaf);
            let searched = search_world(&world, seat, &c).expect("search");
            assert!(
                searched.roots.len() > 64,
                "{leaf:?}: the fixture must offer more candidates than simulations ({})",
                searched.roots.len(),
            );

            let prior =
                |r: &RootStat| r.prior.expect("the mandatory pass gives every root a prior");
            // ANTI-VACUITY for property 1, and deliberately NOT a strict
            // floor > ceiling separation: at this fixture the 186 candidates
            // carry only 4 distinct one-ply values (120 share the top one), so
            // the prefix property holds here BY EQUALITY and demanding strict
            // separation would assert something the position falsifies. What
            // must not be vacuous is the ORDER: a constant prior vector would
            // make property 1 true of any expansion at all.
            let distinct = searched
                .roots
                .iter()
                .map(|r| prior(r).to_bits())
                .collect::<std::collections::BTreeSet<_>>();
            assert!(
                distinct.len() >= 2,
                "{leaf:?}: anti-vacuity — the priors must order the family, saw {} distinct \
                 value(s)",
                distinct.len(),
            );

            let visited_floor = searched
                .roots
                .iter()
                .filter(|r| r.visits > 0)
                .map(prior)
                .fold(f64::INFINITY, f64::min);
            let unvisited_ceiling = searched
                .roots
                .iter()
                .filter(|r| r.visits == 0)
                .map(prior)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(
                visited_floor >= unvisited_ceiling,
                "{leaf:?}: expansion must be a prefix of the prior order: worst expanded \
                 {visited_floor} < best unexpanded {unvisited_ceiling}",
            );

            let max_visits = searched.roots.iter().map(|r| r.visits).max().expect("roots");
            assert!(
                max_visits > 1,
                "{leaf:?}: values must reach the visit distribution: every expanded child was \
                 visited once, which is the star tree",
            );
            assert!(
                searched.roots.iter().any(|r| r.visits == 0),
                "{leaf:?}: anti-vacuity — the budget must NOT have covered the whole family, or \
                 property 1 would be trivially true",
            );

            // The decision is informed: the move returned is at the top of the
            // one-ply ranking rather than a lottery over the middle of it.
            let chosen = searched.roots[searched.chosen].one_ply.expect("a one-ply value");
            let rank = 1 + searched.roots.iter().filter(|r| r.one_ply > Some(chosen)).count();
            assert!(
                rank <= searched.roots.len() / 10,
                "{leaf:?}: the chosen move ranked {rank} of {} by one-ply value — the T2 lottery's \
                 median was 66 of 186",
                searched.roots.len(),
            );
        }
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
        let weights = WeightTable::V1;
        // TWO fixtures, at the T3 review's finding: at a Placement root the
        // ORDERING term of the four-contributor identity is always ZERO (no node
        // below it exceeds the threshold), so that summand was being checked
        // only against 0. The on-board Movement fixture pays real internal
        // ordering passes, which is what makes the identity's third term
        // non-vacuous.
        let fixtures = [
            ("placement root", placement_point(&rules), false),
            ("on-board movement root", on_board_movement_point(&rules), true),
        ];
        // BOTH leaf rules, because the two identities below differ between them
        // by exactly the completion terms — and a single-leaf cell would let a
        // completion stop being charged, or start being charged twice, with
        // nothing to notice.
        for (label, (world, seat), orders_internally) in &fixtures {
            for leaf in [LeafRule::MidTurnDiag, LeafRule::HandOff] {
                let c = ctx_leaf(&rules, &weights, 64, 0xB1A5, leaf);
                let s = search_world(world, *seat, &c).expect("search").stats;
                assert!(
                    s.nodes > 1,
                    "anti-vacuity ({label}, {leaf:?}): the search must have expanded",
                );
                assert_eq!(
                    s.handles,
                    s.nodes - s.terminal_nodes + s.completion_points,
                    "{label}, {leaf:?}: one handle per expandable node plus one per completion ply \
                     beyond the first — a terminal node enumerates nothing, and a completion's \
                     first ply re-uses its node's cache",
                );
                assert_eq!(
                    s.simulations, 64,
                    "{label}, {leaf:?}: the search ran the simulations it was asked",
                );
                assert_eq!(
                    s.scored,
                    s.root_scored + s.ordering_scored + s.completion_scored + (s.nodes - 1),
                    "{label}, {leaf:?}: the meter's four contributors — mandatory root pass, \
                     ordering passes, completions, and one expansion per created node with the \
                     root excepted (it was never applied)",
                );
                if *orders_internally {
                    assert!(
                        s.ordering_passes > 0 && s.ordering_scored > 0,
                        "anti-vacuity ({label}, {leaf:?}): the identity's ordering term must not \
                         be checked against zero ({} passes, {} scored)",
                        s.ordering_passes,
                        s.ordering_scored,
                    );
                }
                assert!(
                    s.nodes - 1 <= s.simulations,
                    "{label}, {leaf:?}: a simulation creates at most one node (none when it ends \
                     at a terminal)",
                );
                assert_eq!(
                    s.root_scored, s.root_candidates,
                    "{label}, {leaf:?}: the root pass is MANDATORY — every candidate, exactly once",
                );
                match leaf {
                    LeafRule::HandOff => {
                        assert!(
                            s.completion_scored > 0,
                            "{label}: the shipped leaf must complete turns",
                        );
                        assert!(
                            s.completion_points > 0,
                            "{label}: and pay for the plies it enumerates",
                        );
                        assert_eq!(
                            s.completion_cap_hits,
                            0,
                            "{label}: and never hit the ply guard",
                        );
                    }
                    _ => {
                        assert_eq!(s.completion_scored, 0, "{label}, {leaf:?} completes nothing");
                        assert_eq!(s.completion_points, 0);
                    }
                }
            }
        }
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

    // -- the leaf rules (spec §2.1) -------------------------------------------

    /// **THE COMPLETION IS THE GREEDY LINE TO THE HAND-OFF — a MIRROR pin**, the
    /// shape `a_decision_is_a_function_of_its_three_stream_words` uses: the loop
    /// is re-driven here out of [`complete_ply`](Search::complete_ply) (whose
    /// one-ply behaviour the closed-form cell below pins independently) plus the
    /// documented key line, so what this checks is the COMPOSITION — the plies,
    /// their keys, the termination, and that the value returned is the folded
    /// HAND-OFF extremum rather than a reading of wherever the line stopped.
    ///
    /// A closed-form cell over the whole rule is not available at any fixture in
    /// this game: it would need a state whose candidates ALL hand off, and a
    /// scan of the entire 523-move `MASTER` walk found no state with even two
    /// (a turn's last move is the forced tail of a phase, and the tails that
    /// exist do not move the evaluation at all — which is also why a "one move
    /// from the transition" cell could not tell this rule from the mid-turn
    /// one). Recorded rather than worked around.
    #[test]
    fn a_completion_is_the_greedy_line_to_the_hand_off_ply_by_ply() {
        const KEY: u64 = 0x99;
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        let (state, seat) = placement_point(&rules);
        let handle = DecisionHandle::new(&state, &rules);
        let moves = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut ChaCha8Rng::seed_from_u64(9))
            .moves;

        let c = ctx_leaf(&rules, &weights, 1, 0x5EA, LeafRule::HandOff);
        let mut budget = ScoredBudget::unlimited();
        let mut search = search_over(&c, seat, &mut budget);
        let got = search.hand_off_value(&state, &moves, KEY).expect("a completion leaf");
        let stats = search.stats;

        let mut mirror_budget = ScoredBudget::unlimited();
        let mut mirror = search_over(&c, seat, &mut mirror_budget);
        let mut best: Option<f64> = None;
        let mut key = completion_branch(KEY);
        let mut carried =
            mirror.complete_ply(&state, &moves, key, 0, true, &mut best).expect("ply 0");
        let mut plies = 1;
        while let Some((next, slot, _)) = carried {
            key = descend_branch(key, slot);
            let (cands, _) = mirror.expand_point(&next, key, false).expect("a ply's candidates");
            carried =
                mirror.complete_ply(&next, &cands, key, plies, true, &mut best).expect("a ply");
            plies += 1;
        }

        assert!(plies > 1, "anti-vacuity: the fixture must take more than one ply to hand off");
        assert_eq!(Some(got), best, "the leaf's value is the folded hand-off extremum");
        assert_eq!(
            (stats.completion_scored, stats.completion_points),
            (mirror.stats.completion_scored, u64::from(plies - 1)),
            "and it pays exactly the plies the line took",
        );
    }

    /// **THE COMPLETION'S DIRECTION, in closed form.** The greedy cut serves the
    /// ACTING seat: at a root-seat node it continues the successor that
    /// MAXIMIZES the root value, and at an opponent node the one that MINIMIZES
    /// it — [`selection_value`]'s convention, which the leaf inherits rather
    /// than modelling a second opponent. Both extremes are recomputed here from
    /// `apply` + `observe` + [`eval::score`].
    ///
    /// **And the DERIVATION of that direction is pinned too** (T3 review): the
    /// half above passes `maximize` in, so it says nothing about where
    /// [`hand_off_value`](Search::hand_off_value) gets it from — `maximize =
    /// true` there passes every other test in the module. So the whole leaf is
    /// driven at BOTH root seats and checked against the two directions' lines,
    /// computed ply by ply here.
    #[test]
    fn the_completions_greedy_cut_serves_the_acting_seat() {
        let rules = Rules::standard();
        let weights = WeightTable::V1;
        let (state, actor) = placement_point(&rules);
        let handle = DecisionHandle::new(&state, &rules);
        let moves = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut ChaCha8Rng::seed_from_u64(9))
            .moves;
        assert!(moves.len() > 1, "the fixture must offer a choice");

        for root_seat in [actor, PlayerId(1 - actor.0)] {
            let maximize = root_seat == actor;
            let raws: Vec<i64> = moves
                .iter()
                .map(|mv| {
                    let next = apply_move_unchecked(&state, mv, &rules).expect("apply");
                    assert_eq!(next.current_player, actor, "no candidate here may hand off");
                    eval::score(&observe(&next, root_seat).expect("observe"), &rules, &weights)
                })
                .collect();
            let want = raws.iter().copied().fold(
                if maximize { i64::MIN } else { i64::MAX },
                |acc, v| if maximize { acc.max(v) } else { acc.min(v) },
            );
            assert!(
                raws.iter().any(|v| *v != want),
                "anti-vacuity: the successors must not all score the same",
            );

            let c = ctx_leaf(&rules, &weights, 1, 0x5EA, LeafRule::HandOff);
            let mut budget = ScoredBudget::unlimited();
            let mut search = search_over(&c, root_seat, &mut budget);
            let mut best: Option<f64> = None;
            let kept = search
                .complete_ply(&state, &moves, 0, 0, maximize, &mut best)
                .expect("one completion ply")
                .expect("a mid-turn ply must continue");
            assert_eq!(best, None, "no candidate handed off, so nothing may be folded as a leaf");
            assert_eq!(
                raws[kept.1], want,
                "seat {} at a node acted by seat {}: the cut must keep the ACTING seat's extreme",
                root_seat.0, actor.0,
            );

            // The WHOLE leaf now, with the direction DERIVED rather than passed:
            // drive both directions' lines from the same key line and check
            // `hand_off_value` against the one the acting seat implies.
            const KEY: u64 = 0x99;
            let fold = |maximize: bool| -> f64 {
                let mut b = ScoredBudget::unlimited();
                let mut m = search_over(&c, root_seat, &mut b);
                let mut best: Option<f64> = None;
                let mut key = completion_branch(KEY);
                let mut carried =
                    m.complete_ply(&state, &moves, key, 0, maximize, &mut best).expect("ply 0");
                let mut ply = 1u32;
                while let Some((next, slot, _)) = carried {
                    key = descend_branch(key, slot);
                    let (cands, _) = m.expand_point(&next, key, false).expect("a ply's candidates");
                    carried = m
                        .complete_ply(&next, &cands, key, ply, maximize, &mut best)
                        .expect("a ply");
                    ply += 1;
                }
                best.expect("a completion reaches a hand-off")
            };
            let (maximizing, minimizing) = (fold(true), fold(false));
            assert_ne!(
                maximizing, minimizing,
                "anti-vacuity: the two directions must value this fixture differently, or the \
                 derivation below would be unobservable",
            );

            let mut whole_budget = ScoredBudget::unlimited();
            let mut whole = search_over(&c, root_seat, &mut whole_budget);
            let got = whole.hand_off_value(&state, &moves, KEY).expect("a completion leaf");
            assert_eq!(
                got,
                if maximize { maximizing } else { minimizing },
                "root seat {} at a node acted by seat {}: the leaf must derive its direction from \
                 the ACTING seat (maximizing {maximizing}, minimizing {minimizing})",
                root_seat.0,
                actor.0,
            );
        }
    }

    /// A MULTI-ply completion pays for the plies it enumerates, and the three
    /// leaf rules are three different values at the same node — the anti-vacuity
    /// the closed-form cell above cannot give (its fixture is one ply deep).
    #[test]
    fn the_leaf_rules_are_three_distinct_values_and_the_default_is_the_completion() {
        assert_eq!(LeafRule::default(), LeafRule::HandOff, "spec §2.1's DEFAULT");
        let rules = Rules::standard();
        assert_eq!(MctsAgent::new(&rules, 1).leaf(), LeafRule::HandOff, "and the agent's");

        let weights = WeightTable::V1;
        let (state, seat) = placement_point(&rules);
        let handle = DecisionHandle::new(&state, &rules);
        let moves = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut ChaCha8Rng::seed_from_u64(9))
            .moves;
        let value_of = |leaf: LeafRule| -> (f64, McSearchStats) {
            let c = ctx_leaf(&rules, &weights, 1, 0x5EA, leaf);
            let mut budget = ScoredBudget::unlimited();
            let mut search = search_over(&c, seat, &mut budget);
            let v = search.leaf_value(&state, &moves, 0).expect("a leaf value");
            (v, search.stats)
        };
        let (hand_off, s) = value_of(LeafRule::HandOff);
        let (mid_turn, m) = value_of(LeafRule::MidTurnDiag);
        let (constant, k) = value_of(LeafRule::ConstantDiag);

        assert!(s.completion_points > 0, "a Placement node is several plies from its hand-off");
        assert!(
            s.completion_scored > moves.len() as u64,
            "and scores more than the first ply's candidates ({} for {} candidates)",
            s.completion_scored,
            moves.len(),
        );
        assert_eq!(s.handles, s.completion_points, "one handle per ply beyond the first");
        assert_eq!((m.completion_scored, m.handles), (0, 0), "the diagnostic leaf completes none");
        assert_eq!((k.completion_scored, k.handles), (0, 0), "nor does the constant one");
        assert_eq!(constant, 0.5, "the constant leaf is the mid-point of the axis");
        assert_ne!(
            hand_off, mid_turn,
            "anti-vacuity: completing the turn must move the value away from the mid-turn read",
        );
        assert!((0.0..=1.0).contains(&hand_off), "and stay on the declared axis: {hand_off}");
    }

    // -- the scored-successor budget (spec §3.4) ------------------------------

    /// **THE CAP'S SEMANTICS.** A simulation is STARTED only while
    /// `realized < cap`; a started one completes (so the realized total lands at
    /// or past the cap, never short of it while simulations remain); the bind is
    /// recorded; and the whole thing is a deterministic function of the master.
    ///
    /// The uncapped arm beside it is the anti-vacuity AND the no-op claim: an
    /// absent cap runs every simulation asked for and binds nothing.
    ///
    /// **BOTH leaf rules, at the T3 review's finding.** The shipped leaf spends
    /// the overwhelming majority of a decision inside its COMPLETIONS, and this
    /// is the only cell in the module that reads that spend against the BUDGET.
    /// The counter identity elsewhere pins `scored` against its four
    /// contributors, so a completion that stopped counting is caught there — but
    /// a completion that counted and did not CHARGE (`stats.scored += 1` in
    /// place of `charge()`) left the whole module green before this arm existed,
    /// and the cap it silently stopped binding is the experiment's own budget.
    /// Mutation-proven at the T3 fix wave, in exactly that shape.
    #[test]
    fn a_capped_search_starts_a_simulation_only_while_the_budget_admits_one() {
        const CAP: u32 = 400;
        const SIMS: u32 = 64;
        let rules = Rules::standard();
        let (world, seat) = on_board_movement_point(&rules);
        for leaf in [LeafRule::MidTurnDiag, LeafRule::HandOff] {
            let c = ctx_leaf(&rules, &WeightTable::V1, SIMS, 0xB1A5, leaf);

            let mut budget = ScoredBudget::capped(CAP);
            let s = search_world_budgeted(&world, seat, &c, &mut budget).expect("search");
            if leaf == LeafRule::HandOff {
                // ANTI-VACUITY, and it comes FIRST: the equality below is a
                // statement about the completion's spend only while there IS
                // one.
                assert!(
                    s.stats.completion_scored > 0,
                    "the shipped leaf must have completed turns inside this cap",
                );
            }
            assert_eq!(s.stats.scored, budget.realized(), "{leaf:?}: the counter IS the meter");
            assert!(
                s.stats.root_scored < u64::from(CAP),
                "premise ({leaf:?}): the fixture's root pass ({}) must fit the cap",
                s.stats.root_scored,
            );
            assert!(
                s.stats.simulations < u64::from(SIMS),
                "{leaf:?}: the cap must have stopped the search short ({} simulations)",
                s.stats.simulations,
            );
            assert!(s.stats.simulations > 0, "{leaf:?}: but not before it started");
            assert_eq!(s.stats.budget_binds, 1, "{leaf:?}: and the bind is recorded");
            assert_eq!(
                s.stats.budget_starved, 0,
                "{leaf:?}: a fitting root pass is not starvation",
            );
            assert!(
                budget.realized() >= u64::from(CAP),
                "{leaf:?}: a started simulation completes, so the total lands at or past the cap: \
                 {}",
                budget.realized(),
            );

            let mut again = ScoredBudget::capped(CAP);
            let s2 = search_world_budgeted(&world, seat, &c, &mut again).expect("search");
            assert_eq!(s.stats, s2.stats, "{leaf:?}: the same master must truncate identically");
            assert_eq!(again, budget, "{leaf:?}: down to the meter");
            assert_eq!(
                format!("{:?}", s.roots[s.chosen].mv),
                format!("{:?}", s2.roots[s2.chosen].mv),
                "{leaf:?}: and decide identically",
            );

            let mut uncapped = ScoredBudget::unlimited();
            let u = search_world_budgeted(&world, seat, &c, &mut uncapped).expect("search");
            assert_eq!(
                u.stats.simulations,
                u64::from(SIMS),
                "{leaf:?}: an absent cap runs the full count",
            );
            assert_eq!(u.stats.budget_binds, 0, "{leaf:?}: and can never bind");
            assert_eq!(uncapped.overshoot(), 0, "{leaf:?}: nor overshoot");
            assert_eq!(
                u.stats.scored,
                uncapped.realized(),
                "{leaf:?}: the counter IS the meter uncapped too",
            );
            assert!(
                u.stats.scored > s.stats.scored,
                "{leaf:?}: anti-vacuity — the cap must have cost something",
            );
        }
    }

    /// **THE STERILE-SIMULATION GUARD.** The experiment's own shape is a scored
    /// CAP with `simulations` left at a ceiling — and a simulation that ends at
    /// an already-created terminal node charges nothing, so the cap cannot bound
    /// that loop at all. Found by driving a full self-play game, where it was an
    /// infinite loop (see [`MAX_STERILE_SIMULATIONS`] for the measurement); this
    /// is the executable version.
    ///
    /// The fixture is a decision point from which the game can END — the last
    /// few of the `MASTER` walk, where the tree runs out of unfinished game to
    /// search. (A state whose EVERY candidate ends the game would be a tidier
    /// fixture and does not exist in this walk: the game ends by a particular
    /// action while its siblings play on.)
    #[test]
    fn a_search_whose_tree_runs_out_of_game_still_terminates() {
        let rules = Rules::standard();
        // `back` is fixture-derived, not principled: it must land on a point
        // whose bounded search actually reaches finished games. Re-derived 2
        // -> 4 at SEMANTICS_REV 6 (the D1 church-cost ruling moved the seeded
        // greedy walk); probe adjacent values if a future rev drifts it again.
        let (world, seat) = point_before_the_end(&rules, 4);

        // Simulations bounded ONLY by the budget — the contestant shape.
        let mut c = ctx(&rules, &WeightTable::V1, u32::MAX, 0xE7D);
        c.simulations = u32::MAX;
        let mut budget = ScoredBudget::capped(10_000);
        let s = search_world_budgeted(&world, seat, &c, &mut budget).expect("search");

        assert!(s.stats.terminal_nodes > 0, "premise: the tree must reach finished games");
        assert_eq!(
            s.stats.sterile_simulations,
            u64::from(MAX_STERILE_SIMULATIONS),
            "the guard is what stopped it: every simulation past the exhausted tree is sterile",
        );
        assert!(
            budget.realized() < 10_000,
            "and it stopped WITHOUT spending the budget — which is the whole point: the meter \
             could not bound this loop",
        );
        assert_eq!(s.stats.budget_binds, 0, "nothing was denied by the cap here");
        assert!(
            s.roots.iter().any(|r| r.visits > 0),
            "the decision is still taken on the tree that was built",
        );

        // THE SHIPPED LEAF at the same fixture (T3 review). Two properties the
        // diagnostic arm above cannot reach: a completion running THROUGH the end
        // of the game must be recognised as a hand-off — dropping
        // `|| next.is_game_over()` from `complete_ply`'s test leaves a finished
        // state in the cut, and the next ply asks a finished game for candidates,
        // which `expand_point` hard-errors on rather than answering; and the
        // completion must still terminate here, where the game itself is about to.
        // Simulations bounded ONLY by the budget here too — the contestant shape.
        let hc = ctx_leaf(&rules, &WeightTable::V1, u32::MAX, 0xE7D, LeafRule::HandOff);
        let mut hb = ScoredBudget::capped(10_000);
        let h = search_world_budgeted(&world, seat, &hc, &mut hb)
            .expect("the shipped leaf must complete turns that run into the end of the game");
        assert!(
            h.stats.completion_scored > 0,
            "anti-vacuity: the shipped leaf must actually have completed turns here",
        );
        assert!(h.stats.terminal_nodes > 0, "premise: the tree must reach finished games");
        assert!(
            h.roots.iter().any(|r| r.visits > 0),
            "and the decision is still taken on the tree that was built",
        );
    }

    /// **THE STARVED FIXTURE (spec §3.4's fallback chain).** A cap smaller than
    /// the MANDATORY root pass leaves nothing for simulations — so the pass runs
    /// anyway, overshoots, records the starvation, and the decision falls back
    /// to the one-ply values the pass produced.
    ///
    /// That fallback is why the pass is mandatory: no candidate is ever
    /// valueless, so a starved decision is a one-ply argmax over the WHOLE
    /// family rather than a draw over whichever prefix the budget reached — and
    /// never a first-listed pick, which at a Placement root is the empty
    /// placement (ledger row 18 through the budget).
    #[test]
    fn a_starved_root_pass_falls_back_to_the_mandatory_one_ply_values() {
        const CAP: u32 = 8;
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 64, 0xB1A5);

        let mut budget = ScoredBudget::capped(CAP);
        let s = search_world_budgeted(&world, seat, &c, &mut budget).expect("search");
        assert!(
            s.stats.root_candidates > u64::from(CAP),
            "anti-vacuity: the fixture's root family ({}) must exceed the cap",
            s.stats.root_candidates,
        );
        assert_eq!(
            s.stats.root_scored, s.stats.root_candidates,
            "the root pass is MANDATORY: every candidate scored, cap or no cap",
        );
        assert_eq!(s.stats.scored, s.stats.root_scored, "and no deeper work may be paid for");
        assert_eq!(s.stats.simulations, 0, "no simulation could start");
        assert_eq!(s.stats.nodes, 1, "so only the root node exists");
        assert_eq!(s.stats.budget_starved, 1);
        assert_eq!(s.stats.budget_binds, 1, "starvation is a bind");
        assert_eq!(
            budget.overshoot(),
            s.stats.scored - u64::from(CAP),
            "the mandatory pass's overrun is reported, not hidden",
        );

        let best = s.roots.iter().filter_map(|r| r.one_ply).max().expect("one-ply values");
        let worst = s.roots.iter().filter_map(|r| r.one_ply).min().expect("one-ply values");
        assert!(best > worst, "anti-vacuity: the one-ply values must separate the candidates");
        assert_eq!(
            s.roots[s.chosen].one_ply,
            Some(best),
            "a starved decision is the one-ply argmax over the whole family",
        );
        assert!(
            s.roots.iter().all(|r| r.visits == 0),
            "premise: nothing was searched, so the visit argmax would have been a whole-family \
             draw — which is exactly what the fallback replaces",
        );
    }

    /// The agent's tally is the experiment's read-out, so it is wired end to end
    /// rather than merely written: an uncapped seat binds nothing and records no
    /// overshoot DISTRIBUTION at all, a capped one records both.
    #[test]
    fn the_tally_carries_the_budget_meters_and_an_absent_cap_records_no_distribution() {
        const SEED: u64 = 0xB0DE_0000_0001;
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");

        let mut uncapped = MctsAgent::new(&rules, SEED).with_simulations(8);
        assert_eq!(uncapped.cap(), None, "the cap knob is OFF by default");
        uncapped.choose(&view, &rules).expect("a decision");
        let stats = *uncapped.last_stats().expect("stats");
        let t = uncapped.tally();
        assert_eq!(t.decisions(), 1);
        assert_eq!(t.candidates_scored(), stats.scored, "the tally sums what the search paid");
        assert_eq!(t.root_scored(), stats.root_scored);
        assert!(t.root_scored() > 0 && t.root_scored() < t.candidates_scored(), "a real share");
        assert_eq!(t.budget_binds(), 0, "an absent cap can never bind");
        assert_eq!(t.budget_starved_decisions(), 0);
        assert_eq!(t.overshoot(), None, "and records no overshoot distribution");

        let mut capped = MctsAgent::new(&rules, SEED).with_simulations(8).with_cap(Some(8));
        assert_eq!(capped.cap(), Some(8));
        capped.choose(&view, &rules).expect("a starved decision still answers");
        let t = capped.tally();
        assert_eq!(t.budget_starved_decisions(), 1, "the root pass alone spent the cap");
        assert_eq!(t.budget_binds(), 1, "starvation is a bind");
        assert_eq!(
            t.overshoot().map(|o| o.decisions),
            Some(1),
            "and a capped decision always contributes an overshoot sample, zero or not",
        );
        assert!(t.overshoot().expect("a distribution").max > 0, "this one overran");
    }

    /// **THE DEGENERACY PIN (T2 review NOTE 7; the capped-S1 arm's sibling).** A
    /// search in which NO value reaches any decision — every leaf, prior and
    /// terminal constant, every choice a keyed draw — must still put assistants
    /// on the board and END.
    ///
    /// It is the tripwire for the failure this architecture can produce quietly:
    /// a tree that settles on "place nothing" every turn looks, from outside,
    /// exactly like a game that never finishes (the greedy rung's ZERO-weights
    /// pin measured that at 1,501 turns when its tie-break was made
    /// first-listed). Nothing here asserts anything about STRENGTH — a
    /// value-blind search is not supposed to play well, only to play.
    #[test]
    fn a_value_blind_search_still_places_assistants_and_finishes_a_game() {
        const MOVES: u32 = 20_000;
        let rules = Rules::standard();
        let mut agent = MctsAgent::new(&rules, 0xB111_0000_0001)
            .with_simulations(4)
            .with_leaf(LeafRule::ConstantDiag);
        let (moves, placed) = drive_to_the_end(&mut agent, MASTER, &rules, MOVES);
        println!("value-blind self-play: {moves} moves · {placed} assistants placed");
        assert!(moves < MOVES, "the game must END, not run out the move budget");
        assert!(placed > 0, "a value-blind search must still place assistants");
        assert_eq!(agent.tally().decisions(), u64::from(moves), "one decision per applied move");

        // Anti-vacuity: the arm really was blind — the priors it ranked on are
        // constant, so nothing but the keyed draws could have decided anything.
        let (world, seat) = placement_point(&rules);
        let c = ctx_leaf(&rules, &WeightTable::V1, 4, 0x1, LeafRule::ConstantDiag);
        let s = search_world(&world, seat, &c).expect("search");
        assert!(s.roots.len() > 1, "premise: a contested point");
        assert!(
            s.roots.iter().all(|r| r.prior == Some(0.5) && r.one_ply == Some(0)),
            "every value must be constant under the blind rule",
        );
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

    // -- the explanation surface (spec §3.5) ----------------------------------

    /// Two constructed rows, for the cells that pin the MARGIN RULE rather than
    /// the search: the search cannot be made to produce an unvisited runner-up
    /// or a chosen-late tie on demand, and a rule stated in a doc comment and
    /// pinned only where the search happens to exercise it is a rule nobody can
    /// check.
    fn constructed_rows(mv: &Move, visits: &[u32], means: &[f64], one_ply: &[i64]) -> Vec<RootStat> {
        (0..visits.len())
            .map(|i| RootStat {
                mv: mv.clone(),
                visits: visits[i],
                value_sum: means[i] * f64::from(visits[i]),
                one_ply: Some(one_ply[i]),
                prior: Some(squash(one_ply[i], DEFAULT_SQUASH_SCALE)),
            })
            .collect()
    }

    /// **THE ROOT TABLE (spec §3.5).** Every root candidate gets a row carrying
    /// its visits, its mean normalized value, its one-ply value and its prior —
    /// and the mean is `None` for a candidate no simulation reached, which at a
    /// 100+-candidate Placement root under a small budget is most of them.
    ///
    /// The visit read-out's margin is the LEAD IN THE DECIDING CURRENCY (visits),
    /// with the value lead carried beside it and REFUSED where either side has
    /// no mean.
    #[test]
    fn the_root_table_reports_every_candidate_and_refuses_a_mean_it_never_measured() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        // Few simulations against a big family, deliberately: the honest-`None`
        // claim is only checkable where candidates go unvisited.
        let c = ctx(&rules, &WeightTable::V1, 8, 0xE1);
        let searched = search_world(&world, seat, &c).expect("search");
        let (n, chosen_mv) =
            (searched.roots.len(), format!("{:?}", searched.roots[searched.chosen].mv));
        let e = searched.into_explanation();

        assert!(n > RENDERED_ROWS, "premise: the fixture must offer a big root family ({n})");
        assert_eq!(e.rows.len(), n, "every root candidate gets a row");
        assert_eq!(e.readout, McReadout::Visits, "a searched decision reads out on visits");
        assert_eq!(format!("{:?}", e.rows[e.chosen].mv), chosen_mv, "and the chosen row is the move");
        for (i, r) in e.rows.iter().enumerate() {
            assert_eq!(
                r.mean().is_some(),
                r.visits > 0,
                "row {i}: a mean exists exactly where a simulation measured one",
            );
            assert!(
                r.one_ply.is_some() && r.prior.is_some(),
                "row {i}: the MANDATORY root pass values every candidate",
            );
        }
        assert!(
            e.rows.iter().any(|r| r.visits == 0),
            "anti-vacuity: at 8 simulations most of this family must go unvisited",
        );

        let McMargin::Visits { runner_up, visits, value } = e.margin else {
            panic!("a searched decision must report a visit margin, got {:?}", e.margin);
        };
        assert_ne!(runner_up, e.chosen, "the runner-up is somebody else");
        assert_eq!(
            visits,
            i64::from(e.rows[e.chosen].visits) - i64::from(e.rows[runner_up].visits),
            "the margin is the lead in the deciding currency",
        );
        let order = e.ranked();
        assert_eq!(order[0], e.chosen, "the report leads with the chosen row");
        assert_eq!(order[1], runner_up, "and the runner-up is the second row, by the same key");
        match (e.rows[e.chosen].mean(), e.rows[runner_up].mean()) {
            (Some(a), Some(b)) => assert_eq!(value, Some(a - b), "both measured: the lead is real"),
            _ => assert_eq!(value, None, "and a lead over an unmeasured mean is refused"),
        }
    }

    /// **THE MARGIN RULE against an UNVISITED runner-up**, in closed form: the
    /// visit lead is defined (visits are counted for every candidate), the VALUE
    /// lead is not, and `None` is the answer — never 0.0, and never the squash's
    /// mid-point, either of which would report a comparison nobody made.
    #[test]
    fn the_value_margin_is_refused_against_a_runner_up_no_simulation_reached() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mv = search_world(&world, seat, &ctx(&rules, &WeightTable::V1, 4, 0x1))
            .expect("search")
            .roots
            .swap_remove(0)
            .mv;

        let unvisited = constructed_rows(&mv, &[3, 0], &[0.75, 0.0], &[10, 4]);
        let e = McExplanation::new(unvisited, 0, McSearchStats::default());
        assert_eq!(e.readout, McReadout::Visits);
        assert_eq!(
            e.margin,
            McMargin::Visits { runner_up: 1, visits: 3, value: None },
            "an unvisited runner-up has no mean, so it has no value margin",
        );

        let both = constructed_rows(&mv, &[3, 2], &[0.75, 0.5], &[10, 4]);
        let e = McExplanation::new(both, 0, McSearchStats::default());
        assert_eq!(
            e.margin,
            McMargin::Visits { runner_up: 1, visits: 1, value: Some(0.25) },
            "two measured means DO subtract",
        );
    }

    /// The REPORTING order leads with the chosen row even when the decision was
    /// a tie the keyed draw settled on a late candidate — otherwise a truncated
    /// table could omit the very row it exists to explain.
    ///
    /// It is a REPORTING order and nothing else: it consumes no randomness, it
    /// runs after the decision, and no move depends on it.
    #[test]
    fn the_report_leads_with_the_chosen_row_even_when_the_decision_was_a_tie() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mv = search_world(&world, seat, &ctx(&rules, &WeightTable::V1, 4, 0x1))
            .expect("search")
            .roots
            .swap_remove(0)
            .mv;

        let tied = constructed_rows(&mv, &[5, 5, 5], &[0.5, 0.5, 0.5], &[1, 2, 3]);
        let e = McExplanation::new(tied, 2, McSearchStats::default());
        assert_eq!(e.ranked(), vec![2, 0, 1], "the chosen row leads its own tie block");
        assert_eq!(
            e.margin,
            McMargin::Visits { runner_up: 0, visits: 0, value: Some(0.0) },
            "a tie is a zero lead, reported as one",
        );
    }

    /// A STARVED decision explains on the currency it actually decided on: the
    /// mandatory root pass's one-ply values, not on visits that are all zero.
    #[test]
    fn a_starved_decision_explains_on_the_one_ply_values_it_decided_on() {
        const CAP: u32 = 8;
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 64, 0xB1A5);
        let mut budget = ScoredBudget::capped(CAP);
        let s = search_world_budgeted(&world, seat, &c, &mut budget).expect("search");
        assert_eq!(s.stats.budget_starved, 1, "premise: the fixture must starve");
        let e = s.into_explanation();

        assert_eq!(e.readout, McReadout::OnePly, "the read-out names the currency");
        assert!(e.rows.iter().all(|r| r.visits == 0 && r.mean().is_none()), "nothing was searched");
        let runner_best = e
            .rows
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != e.chosen)
            .filter_map(|(_, r)| r.one_ply)
            .max()
            .expect("a starved decision values every candidate");
        let McMargin::OnePly { runner_up, one_ply } = e.margin else {
            panic!("a starved decision must report a one-ply margin, got {:?}", e.margin);
        };
        assert_eq!(e.rows[runner_up].one_ply, Some(runner_best), "the runner-up is the next best");
        assert_eq!(
            one_ply,
            Some(e.rows[e.chosen].one_ply.expect("the chosen was valued") - runner_best),
            "and the margin is the lead in that currency",
        );
        assert!(one_ply.expect("a lead") >= 0, "the chosen candidate is the argmax");
        assert_eq!(e.ranked()[0], e.chosen, "the report still leads with the chosen row");
    }

    /// A FORCED decision point explains a table of one, with no runner-up to be
    /// ahead of — and says so rather than reporting a zero margin over itself.
    #[test]
    fn a_forced_decision_point_explains_a_table_of_one_with_no_margin() {
        let rules = Rules::standard();
        let is_forced = |s: &GameState| {
            let handle = DecisionHandle::new(s, &rules);
            !handle.families().is_empty()
                && candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut ChaCha8Rng::seed_from_u64(1))
                    .moves
                    .len()
                    == 1
        };
        let (world, seat) = point_where(&rules, MASTER, is_forced);
        let s = search_world(&world, seat, &ctx(&rules, &WeightTable::V1, 64, 0xFEED))
            .expect("search");
        let e = s.into_explanation();
        assert_eq!(e.rows.len(), 1, "premise: a forced point");
        assert_eq!(e.readout, McReadout::Forced);
        assert_eq!(e.margin, McMargin::Forced);
        assert_eq!(e.ranked(), vec![0]);
        assert!(
            e.render().contains("forced decision point"),
            "and the rendering says so: {}",
            e.render(),
        );
    }

    /// **THE IDENTITY (spec §3.5, the gate's `x_explanations` clause).** Every
    /// SUCCESSFUL `choose` produces exactly one explanation — and a FAILING one
    /// produces none, while leaving the previous decision's table STALE rather
    /// than clearing it (T2 review NOTE 9's rule, which `last_stats` and
    /// `last_roots` already follow; the tally is immune to it because it is only
    /// ever incremented on the success path).
    #[test]
    fn every_successful_choose_explains_exactly_once_and_a_refusal_neither_counts_nor_clears() {
        const DECISIONS: usize = 7;
        let rules = Rules::standard();
        let mut agent = MctsAgent::new(&rules, 0xE7_0000_0001).with_simulations(6);
        assert_eq!(agent.tally().explained(), 0, "a fresh agent has explained nothing");
        assert!(agent.last_explanation().is_none(), "and holds no table");

        // Cloned BEFORE the decisions, deliberately: the arena factory clones
        // the handle out at agent construction (`ExplainHandle` precedent), so
        // the property under test is the clone's LIVENESS — a `tally()`
        // returning a dead snapshot must fail here (T4 review S2's mutation).
        let t = agent.tally();
        let (moves, _) = first_decisions(&mut agent, MASTER, DECISIONS, &rules);
        assert_eq!(t.explained(), DECISIONS as u64, "one explanation per decision taken");
        assert_eq!(
            t.explained(),
            t.decisions(),
            "the two counters answer different questions and must not come apart",
        );
        let last = agent.last_explanation().expect("the last decision's table").clone();
        assert_eq!(
            format!("{:?}", last.rows[last.chosen].mv),
            moves[DECISIONS - 1],
            "the table explains the move that was actually returned",
        );

        let (world, seat) = placement_point(&rules);
        let mut doctored = observe(&world, seat).expect("observe");
        doctored.order_deck_len += 1;
        agent.choose(&doctored, &rules).expect_err("the sampler must refuse");
        assert_eq!(
            agent.tally().explained(),
            DECISIONS as u64,
            "a failing choose explains nothing",
        );
        assert_eq!(
            agent.last_explanation(),
            Some(&last),
            "and leaves the previous decision's table STALE, not cleared",
        );
    }

    /// **THE RENDERING IS BYTE-IDENTICAL AT THE SAME MASTER** — the same
    /// property the decision itself carries, extended to what a reader is shown,
    /// so a T5 report can print a table without becoming a source of churn.
    #[test]
    fn the_rendered_root_table_is_byte_identical_at_the_same_master() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let c = ctx(&rules, &WeightTable::V1, 12, 0x4AB1E);
        let a = search_world(&world, seat, &c).expect("search").into_explanation();
        let b = search_world(&world, seat, &c).expect("search").into_explanation();
        let rendered = a.render();
        assert_eq!(rendered, b.render(), "the same master must render the same bytes");
        println!("{rendered}");

        assert!(
            rendered.starts_with("mcts-v1 root table: "),
            "the table names its author: {rendered}",
        );
        assert!(rendered.contains("read-out visits"), "and the deciding currency: {rendered}");
        assert!(
            rendered.contains(&format!("{:?}", a.rows[a.chosen].mv)),
            "the chosen move is on the table: {rendered}",
        );
        assert!(rendered.contains("margin over runner-up"), "with its lead: {rendered}");
        assert!(
            a.rows.len() > RENDERED_ROWS,
            "premise: the fixture must overflow the reporting cap ({})",
            a.rows.len(),
        );
        assert!(
            rendered.contains(&format!("... and {} more candidates", a.rows.len() - RENDERED_ROWS)),
            "a truncated table SAYS how much it left out: {rendered}",
        );
        assert_eq!(
            rendered.lines().count(),
            RENDERED_ROWS + 4,
            "header, column titles, the capped rows, the truncation line, the margin: {rendered}",
        );

        // Anti-vacuity: the rendering is a function of the SEARCH, not a fixed
        // shape — a different tie key moves it.
        let other = search_world(&world, seat, &ctx(&rules, &WeightTable::V1, 12, 0x5EED))
            .expect("search")
            .into_explanation();
        assert_ne!(other.render(), rendered, "a different master must render differently");
    }

    // -----------------------------------------------------------------------
    // The arena seam (spec §3.6) — T5
    // -----------------------------------------------------------------------

    /// The `leaf=<mode>` vocabulary is a CLOSED round trip, and its wire codes
    /// are distinct.
    ///
    /// Three separate claims, because three separate defects hide in them: a
    /// name that does not parse back makes a runner knob unreachable; an unknown
    /// name that parses to a DEFAULT puts a published cell under a mechanism it
    /// did not run (which is why the runner errors on it); and two rules sharing
    /// a code makes every corpus record that carries `leaf=` ambiguous forever.
    #[test]
    fn the_leaf_rule_grammar_round_trips_and_its_wire_codes_are_distinct() {
        let all = [LeafRule::HandOff, LeafRule::MidTurnDiag, LeafRule::ConstantDiag];
        for rule in all {
            assert_eq!(
                LeafRule::from_str_name(rule.as_str()),
                Some(rule),
                "{:?} does not parse back from its own name",
                rule,
            );
        }
        let mut codes: Vec<i64> = all.iter().map(|r| r.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len(), "two leaf rules share a wire code");
        // The literals, not `as_str()` on both sides — these strings are the
        // command-line and the doc, and comparing a call against itself pins
        // nothing.
        assert_eq!(LeafRule::HandOff.as_str(), "hand-off");
        assert_eq!(LeafRule::MidTurnDiag.as_str(), "midturn-diag");
        assert_eq!(LeafRule::ConstantDiag.as_str(), "constant-diag");
        assert_eq!(LeafRule::default(), LeafRule::HandOff, "the shipped leaf rule is the default");
        assert_eq!(LeafRule::from_str_name("handoff"), None, "an unknown mode must not default");
        assert_eq!(LeafRule::from_str_name(""), None);
    }

    /// The knob set survives the trip through the agent, and the agent's
    /// descriptor is the free function's — so a cell that builds a seat and a
    /// cell that only ECHOES one cannot describe different searches.
    ///
    /// Includes the SIZING RULE, which is the one derived knob: a capped seat
    /// runs [`SIMULATION_CEILING`] and lets the budget bound it, an uncapped one
    /// runs [`DEFAULT_SIMULATIONS`]. Pairing a cap with a simulation count would
    /// make whichever bound bit first the real knob, and no report would say
    /// which.
    #[test]
    fn the_knob_set_round_trips_through_the_agent_and_its_descriptor() {
        let rules = Rules::standard();
        let knobs = McKnobs::sized_by_cap(Some(750), 900_000, 250, 3, LeafRule::MidTurnDiag);
        assert_eq!(knobs.simulations, SIMULATION_CEILING, "a capped seat is sized by its cap");
        assert_eq!(
            McKnobs::sized_by_cap(None, 1, 1, 1, LeafRule::HandOff).simulations,
            DEFAULT_SIMULATIONS,
            "an uncapped seat has no second bound, so the ceiling would not terminate",
        );
        assert_eq!(McKnobs::DEFAULT.cap, None, "the shipped knob set is uncapped");

        let agent = MctsAgent::with_knobs(&rules, 0xABCD, knobs);
        assert_eq!(agent.knobs(), knobs, "the agent will run a different search than it was given");
        assert_eq!(
            ArenaAgent::controller(&agent),
            controller_provenance(knobs, &WeightTable::V1, Some(0xABCD)),
            "the agent and the free descriptor disagree about what it is",
        );
    }

    /// Spec §3.6's ABSENT-KNOB rule, on this controller: a `None` cap emits NO
    /// `cap=` token, so an uncapped seat's descriptor is the descriptor it would
    /// have had if the knob had never been added. Everything else it names is
    /// present.
    #[test]
    fn an_absent_cap_emits_no_token_and_a_present_one_does() {
        let uncapped = controller_provenance(McKnobs::DEFAULT, &WeightTable::V1, None);
        assert_eq!(uncapped.name, MCTS_CONTROLLER);
        assert_eq!(uncapped.version, MCTS_IMPL_VERSION);
        assert!(!uncapped.params.contains_key("cap"), "an absent cap must emit no token");
        for key in ["c", "scale", "m", "leaf", "sims", "k", "eval_impl", "weights"] {
            assert!(uncapped.params.contains_key(key), "the descriptor lost {key}");
        }
        assert_eq!(uncapped.params["leaf"], LeafRule::HandOff.code());
        assert_eq!(uncapped.params["sims"], i64::from(DEFAULT_SIMULATIONS));

        let capped = controller_provenance(
            McKnobs::sized_by_cap(Some(750), DEFAULT_EXPLORATION_PPM, DEFAULT_SQUASH_SCALE, 8, LeafRule::HandOff),
            &WeightTable::V1,
            None,
        );
        assert_eq!(capped.params["cap"], 750);
        assert!(
            !capped.params.contains_key("sims"),
            "a ceiling-derived sims is a loop bound the budget pre-empts, not a realized knob"
        );
        // Anti-vacuity: the two descriptors differ ONLY where the cap does.
        let mut without_cap = capped.params.clone();
        without_cap.remove("cap");
        without_cap.insert("sims".to_string(), i64::from(DEFAULT_SIMULATIONS));
        assert_eq!(without_cap, uncapped.params, "the cap knob moved something it should not have");
    }

    /// The seam's own counter: hand-off completions stopped by
    /// [`MAX_TURN_PLIES`] are readable off the CLONED tally, which is the only
    /// handle the arena driver ever holds. Expected 0 — what is pinned is that
    /// the number is wired, since a counter written and never read is exactly as
    /// useful as no counter (the 3.3 six-lens finding, restated here rather than
    /// re-learned).
    #[test]
    fn the_tally_carries_the_completion_cap_counter() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");
        let mut agent = MctsAgent::new(&rules, 0xC0DE_0001).with_simulations(8);
        let tally = agent.tally();
        assert_eq!(tally.completion_cap_hits(), 0, "nothing has run yet");
        agent.choose(&view, &rules).expect("a decision");
        let stats = *agent.last_stats().expect("stats");
        assert_eq!(
            tally.completion_cap_hits(),
            stats.completion_cap_hits,
            "the tally must sum the search's own counter",
        );
        assert_eq!(tally.completion_cap_hits(), 0, "the guard is not expected to fire here");
        assert!(stats.completion_scored > 0, "premise: the shipped leaf ran completions at all");
    }
}
