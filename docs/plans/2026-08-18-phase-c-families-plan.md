# Phase C — the four driven families: implementation plan

**Executes `docs/plans/2026-08-18-phase-c-families-design.md` (the AMENDED text — the Fable
DESIGN review's 6 MUST / 9 SHOULD / 6 NOTE are folded in place there; every task below cites
the design section it implements and the review findings it discharges). THIS PLAN was
itself Fable-reviewed (verdict SOUND-WITH-FIXES, 4 MUST / 6 SHOULD / 8 NOTE, ALL folded in
place — the load-bearing four: the `EndGameTrigger`-exhaustive precondition set, the
strided-supply arithmetic with pre-declared extension ordinals, the replay-derived turn-hook
mechanism with the root-pre-candidate first argument, and the candidate-width bound as a
predicate conjunct; three design lines were amended under that review's license and are
current in the design text). Branch
`phase3.5c-families` off master (base: the design commit), worktree
`../yokohama-families-wt`, one PR at close. Six code tasks T1–T6 under the standing cadence
(RED commit → GREEN commit → Fable-max review → adjudication → fix wave), then T7, the
orchestrator-led freeze/run/route protocol. The orchestrator never touches the tree while an
implementer runs.**

## Global constraints (every task inherits these)

- **TDD, RED first**: the failing-test commit precedes the implementation commit; suites
  green + `cargo clippy --workspace --all-targets -- -D warnings` clean after every task.
- **Per-edit gates after every task**: `cargo test -p yokohama-core` (expect 1090+Δ passing,
  29 ignored — the ignored count MUST NOT move; nothing in this phase is release-tier) and
  the other five cargo suites + 4 node suites unmoved.
- **`SEMANTICS_REV` stays 7. No file under `game/` that affects legality/application/scoring
  /setup is touched** — every edit lands in `driven.rs`, `driven_families.rs` (new),
  `game/mod.rs` (the one-line module wiring), `driven_runner.rs`, `examples/driven.rs`, and
  their tests. `eval.rs`, weights, features, preserved rungs, oracle: untouched (the
  design's STOP fence).
- **Corpus-free tests**: no test reads `docs/temp/` or `bridge-captures/`; registry-entry
  tests keep the typed-miss posture. **`git add <path>`, never `-A`** (the symlink lesson).
- **No new `#[ignore]`** anywhere in this phase (the four-doc-site rule must not fire).
- **L12**: no weight parameter on any new surface; contestants/policies only.
- **Masters recorded before use** — T7 step 0 writes the ledger row before any pilot run;
  in-test masters use constructed constants documented as fixtures (existing practice).
- **Dispatch**: implementers Opus max effort; reviews Fable max; orchestrator adjudicates.
- Reference reading for every implementer: the design doc (above), `CLAUDE.md`, the harness
  design `docs/plans/2026-08-17-phase-b-harness-design.md`, and the module docs in
  `driven.rs`. Rust 1.97.1 pinned; run commands from the worktree root.

---

## T1 — Predicates, truth helper, ecology v2, stride (design §1 families 1–2, §2 first two
blocks; discharges MUST-6, SHOULD-8)

**Files**: modify `crates/yokohama-core/src/game/driven.rs` (the `RootPredicate` /
`GeneratingEcology` / scan region, ~lines 240–535); tests in the same file's `mod tests`.

**Produces (exact interfaces later tasks consume):**
```rust
impl RootPredicate {
    pub fn holds(self, state: &GameState, rules: &Rules) -> bool;   // signature CHANGE: gains &Rules
    pub fn ecology(self) -> GeneratingEcology;                      // which ecology scans this predicate
}
// new variants: TriggerRoundNotFinal, EndFirableAheadInTruth, EndFirableBehindInTruth,
//               ExclusiveRaceOneStep, AchievementClaimAvailable
pub fn true_margin_now(state: &GameState, rules: &Rules) -> i64;    // mover total − best-rival-by-rank total, via final_result on the as-if-now state
pub(crate) fn firing_moves(state: &GameState, rules: &Rules) -> Vec<Move>; // legal moves whose apply sets end_game_triggered
pub struct GeneratingEcology { /* existing fields */ pub per_game_cap: u32 } // NEW FIELD — identity, id strings bump
pub fn v2_lookahead() -> GeneratingEcology;                         // 2P, both seats the shipped lookahead echo
```

Content:
1. `holds` gains `&Rules`; the shipped variant ignores it — pin byte-identical behavior on
   the existing fixtures. Call sites (the scan, registry loader if any) updated.
2. New predicate arms, each with the design's exact conjuncts: `TriggerRoundNotFinal` =
   `end_game_triggered && !final_round && !is_game_over`; `EndFirableAheadInTruth` =
   `!end_game_triggered && !firing_moves(..).is_empty() && true_margin_now(..) >= 10 &&
   width_ok`; `EndFirableBehindInTruth` mirrors at `<= -10`; `ExclusiveRaceOneStep` (an
   opponent is one completed action from a declared race step AND the mover holds a move
   altering it, AND `width_ok` — write the exact state arithmetic in-source with a doc
   comment; the design accepts this arm may read `Exhausted` at scan, which is recorded
   evidence, so favor PRECISION over recall); `AchievementClaimAvailable` = a legal
   `Move::FulfillAchievement` exists AND `width_ok`. **`width_ok` is the design §4
   candidate-width bound as a PREDICATE CONJUNCT** (plan review MUST-4 — a recorded
   generator constraint, never a post-hoc filter): `get_legal_moves(state, rules).len() <=
   60`, one shared helper with its own pin (a 61-candidate constructed state fails it).
3. `firing_moves` SHORT-CIRCUITS (SHOULD-8), **with the precondition set DERIVED as an
   exhaustive `match` over `EndGameTrigger`'s FIVE variants** (plan review MUST-1 — the
   prose-derived church/customs/deck list missed both building-completion triggers and
   named a tech-deck clock that does not exist): `ChurchTrackFull` → church track one space
   from full; `CustomsTrackFull` → customs one from full; `OrderDeckEmpty` → order deck
   near-empty per that variant's actual condition; `TradingHousesComplete` → any player at
   3 of 4 trading houses; `ShophousesComplete` → any player at 7 of 8 shophouses (read the
   exact thresholds off the trigger's own firing sites in `apply_move.rs`; a future variant
   is a compile error, not a scan bias). Then apply-probe `get_legal_moves` via
   `apply_move_unchecked` on clones. Soundness pins: a one-space-from-full clock state AND
   a one-building-from-complete state each find their firing move through the
   short-circuit; a mid-game state short-circuits empty AND a full probe (test-side, no
   short-circuit) also finds none — the skip-never-lies property.
4. `true_margin_now` via `final_result(state, rules)` standings (works on any state): mover
   = `state.current_player`, best rival = highest-ranked other seat; margin = mover total −
   rival total as i64. Pins: a hand-computed 2P fixture; the D1/D5 dummy participating; a
   level-total pair (margin 0 while `final_result` still ranks them — margin 0 ≠ win).
5. `GeneratingEcology` gains `per_game_cap: u32` (**serde: NO `#[serde(default)]`** — no
   committed artifact holds a Generated triple, verified by grep and stated in the commit
   message, so absence-fails-loudly is the honest posture — NOTE-4); `v1()` sets cap 2 AND
   **raises `games` 4 → 16 in the same id bump** (`driven-generated-v1` →
   `driven-generated-v1.1`; plan review MUST-2's supply arithmetic: pilot 4 + confirmatory
   6 + escrow 6 = 16 strided ordinals demanded per family-1 predicate, and 4 games × cap 2
   = 8 could strand a frozen confirmatory block on `Exhausted`; 16 games × cap 2 = 32
   covers demand plus dead-root slack at ~1.2s of rung scan); `v2_lookahead()` = 2P, both
   seats `LOOKAHEAD_V1_DEFAULT`'s resolved echo, arena driver line, its own id
   `driven-generated-v2-lookahead`, **games bound 24** (same arithmetic at cap 2 with NO
   supply guarantee — the layer-2 predicates are selective, and `Exhausted` is recorded
   evidence per the design; a games raise at fixed master+cap is PREFIX-STABLE, the
   design-§4 recovery), cap 2, seed-derivation line identical to v1's.
   `RootPredicate::ecology()`: family-1 pair → `v1()`; the three new layer-2 predicates →
   `v2_lookahead()`.
6. The scan (`generate_root`) takes its ecology from `predicate.ecology()` and enforces the
   STRIDE: at most `per_game_cap` hits per generated game enter the ordinal sequence.
   Stride pin (cheap, rung ecology): at a test master where game 0 holds ≥3 final-round
   hits, ordinals 0..cap come from game 0 and ordinal `cap` has `game_index >= 1`
   (read off the descriptor); the exhaustion error still names the bound.

Tests: one RED module (~12–16 fns) covering every numbered item; wall discipline — no
lookahead GAMES in tests (the v2 ecology's field values are pinned without running a scan;
scan-machinery tests run the rung ecology only).

## T2 — `drive_turn` (design §2 third block; discharges SHOULD-6)

**Files**: modify `crates/yokohama-core/src/game/driven.rs` (beside `drive_to_terminal`).

**Produces:**
```rust
pub struct TurnOutcome {
    pub moves: Vec<AppliedMove>,   // the mover's applied moves, in order
    pub handoff: GameState,        // state at the first current_player change, or terminal
    pub game_over: bool,           // true when the stop was the game ending, not a hand-off
    pub counters: SeatCounters,    // the mover's WHOLE-TURN counters
    pub decisions: u64,
}
pub fn drive_turn(root: &GameState, mover: SeatSpec, rules: &Rules, decision_seed: u64,
                  opts: &DrivenOpts) -> Result<TurnOutcome, DrivenError>;
```

Content: the same honest chain and root hygiene as `drive_to_terminal` (eligibility
precheck, root `check_invariants` guard naming the root, per-move invariants per opts,
decision bound as typed refusal). ONE agent constructed at `decision_seed` for the mover's
seat, its stream advancing across the turn's decisions (the arena discipline scoped to a
turn — no third discipline; say so in the doc comment). Stop when `current_player !=
root.current_player` (set `game_over` from `is_game_over` when that is why) — the design's
verified premise: `current_player` has one production writer. First-decision join pin: at
the same `decision_seed`, `drive_turn`'s first applied move equals `root_table`'s winner
move for the same seat spec (the counter-check join). Other pins: hand-off stop exactness
on a rung seat's turn (the hand-off state's `current_player` differs, every prior state's
does not); determinism (same seed → identical outcome digest twice — reuse `state_digest`
on handoff + move list compare); the terminal-root corner (zero-move outcome, `game_over`
true, consistent with `drive_to_terminal`'s documented posture); sampler-failure and
seats-mismatch refusals shared with the terminal driver (extract the shared guard rather
than duplicating). **Prefix-equivalence pin (plan review SHOULD-6 — `drive_turn` is a third
per-decision loop and the first-decision join covers decision 0 only):** at `decision_seed
= agent_stream_seed(world, mover, Slot(mover))`, `drive_turn`'s move list equals the
mover's own prefix of `drive_to_terminal` at `world` up to the first opponent move (rung
seats, cheap — the pin that keeps the turn loop honest against the play half beyond
decision 0). Wall: rung seats only in loop pins; ONE smoke with the shipped lookahead
subject bounded to a single turn (~3s debug budget, stated in-test).

## T3 — `ReplyPolicy::RaceRusher` (design §2 RaceRusher block; discharges MUST-5, SHOULD-7,
NOTE-1, NOTE-2)

**Files**: modify `crates/yokohama-core/src/game/driven.rs` (`ReplyPolicy`, `SeatSpec`
materialization, the agent seam near line 1148).

**Produces:**
```rust
pub enum ReplyPolicy { FamilyWeighted, GreedyAtFinal, LookaheadAtFinal, RaceRusher }
// SeatSpec materialization: RaceRusher has NO Contestant. The existing single
// materialization site grows a second arm returning Box<dyn ArenaAgent> directly;
// ReplyPolicy::contestant() becomes a partial mapping with a typed story for RaceRusher
// (the N-C structural argument restated in the doc comment: still ONE site, two arms).
pub(crate) struct RaceRusherAgent { /* owns an inner mirrored greedy agent */ }
```

Rule, exactly as the design states it, with the REAL family-tag names (plan review NOTE-1):
probe candidates only from families {`AdditionalAction`, `AreaAction`, `ChoiceResolution`};
never {`Placement`, `MovementRoutes`, `MovementRetreat`}; `FivePower`/`Building` excluded
by decision. Classify by apply-probe on the agent's own sampled PROBE WORLD, whose seed is
a **named derivation over (the seat's agent seed, the decision ordinal) that NEVER draws
from the inner greedy's stream** (plan review SHOULD-2 — a probe that consumed the inner
stream would break the fallback byte-identity by construction; document the derivation
beside the salt constants with a cross-pointer, and keep a per-seat decision ordinal
counter in the agent): (1) `FulfillAchievement` → (2) church progress → (3) customs
progress, where (2)/(3) are QUEUE-AWARE: occupancy grew OR the probed clone queues a
church/customs space-placement `PendingChoice` for the mover. First candidate of the
highest nonempty class in legal-list order; otherwise DELEGATE the whole decision to the
inner greedy agent (constructed by mirroring `Contestant::build`'s greedy arm — drift pin —
and owned for the seat's lifetime, so the no-race fallback is byte-identical to a bare
greedy seat at the same seed BY CONSTRUCTION; the probes consume zero randomness from the
inner stream). Echo `reply:race-rusher-v1{fallback=<the greedy echo>}` with the
provisional-until-D1 comment. Argv reachability is T6's item (the seat grammar gains
`race-rusher` — SHOULD-3's decision).

Pins (RED first): class priority on constructed decisions (claim beats church beats
customs); the ≥2-eligible queue-aware case (a Church action with two eligible spaces
classifies class-2 — the MUST-5 pin, on a constructed state whose apply queues the choice);
probe-scope (a decision whose only candidates are movement/placement/retreat probes
NOTHING and delegates — instrument via the probe being structurally unreachable there,
e.g. assert the pick equals the inner greedy's pick and the probe counter, if one is kept
`cfg(test)`, stays 0); **fallback byte-identity DECISION-SCOPED** (plan review SHOULD-2 —
the whole-game "no race candidate ever fires" premise is uncontrollable): constructed
no-race decisions assert pick == the inner greedy's pick, AND a second decision at the
same seat asserts the stream was untouched by the first's probes (the pick a bare greedy
seat makes at that stream position); determinism; echo string pinned verbatim.

## T4 — the read machinery: turn predicates, DEAD classifier, adjudicator hook, admission,
`family_read` (design §2 families-module block, §3, §4; discharges MUST-1, MUST-2, MUST-3's
report shape, MUST-4's arithmetic, SHOULD-1/2's printed forms)

**Files**: create `crates/yokohama-core/src/game/driven_families.rs` (+ `pub mod` wiring in
`game/mod.rs`); modify `driven.rs` for: `CandidatePlayout` gains
`pub turn_verdict: Option<bool>`, and `adjudicate_root_with(source, seats, rules, master,
replications, opts, turn_hook: Option<&dyn Fn(&GameState, &GameState) -> bool>)` — the
existing `adjudicate_root` delegates with `None`; the ~5 `CandidatePlayout` literal
constructions in `driven.rs` tests AND the one in `driven_runner.rs`'s tests gain
`turn_verdict: None` (NOTE-4 — so "existing suite passes" is a real check, not a hope).
**The hook mechanism, stated truthfully (plan review MUST-3a):** the playout runs through
`play_from_state`, which exposes NO per-move observer — so the verdict is DERIVED FROM THE
APPLIED-MOVE LIST, the repo's canonical replay-reconstruction pattern (`state_at`, `why`):
take the playout's seat-attributed `applied` moves, replay root → forced candidate → the
mover's applied prefix up to the first non-mover move (or terminal), and evaluate the hook
at that hand-off state. Cost: ≤ one turn of re-applies per playout, and only when a hook is
supplied. **The hook's FIRST argument is the ADJUDICATED ROOT, pre-candidate — never the
post-candidate state** (MUST-3b): the mover's turn INCLUDES the forced candidate (decision
zero), and evaluating `AchievementClaimed` against the post-candidate state would classify
a claim-first playout predicate-FALSE (its claimed set already grew) and invert family 4's
dominance partition.

**Produces:**
```rust
pub enum ContestedStep { ChurchSpace { faith: u8 },      // the church board's faith-4..11 spaces
                         CustomsSpace { index: u8 },     // the customs board's 7 spaces, 0..=6
                         TechMajorityRankPast { doubled_threshold: u32 } }  // the F6 arithmetic
pub enum TurnPredicate { EndFired, NotEndFired, AchievementClaimed,
                         ContestedStepTaken(ContestedStep), DenialNotTaken(ContestedStep) }
impl TurnPredicate { pub fn eval(&self, root: &GameState, handoff: &GameState) -> bool; }
pub enum PassRule { Turn(TurnPredicate), AdjudicatedBest { epsilon: i64 } }
pub const DEAD_SPREAD_BOUND: i64 = 4;      // the triage §3 measured dead-field spread
pub fn classify_dead(adj: &Adjudication, spread_bound: i64) -> bool;
pub struct AdmissionRead { /* per-policy: group means, delta, both-nonempty, verdict;
                              plus the sole-dissenter flag and exclusion routing */ }
pub fn admission(adjs: &[(ReplyPolicy, Adjudication)], delta: i64, control: bool) -> AdmissionRead;
pub const SALT_DRIVEN_FAMILY_READ: u64 = 0xA3E1_D21E_0000_0008; // next free D21E serial (0006 = SALT_DRIVEN_REPLICATION; 0007 was consumed by T3's SALT_DRIVEN_RACE_PROBE — orchestrator ruling at T3 adjudication: first consumer keeps the serial)
pub struct FamilyReadOutcome { /* per-replication verdicts+picks, miss count, binomial line,
                                  counter check, refusal arm */ }
pub fn family_read(root: &GameState, subject: SeatSpec, rule: &PassRule, r: u32,
                   master: u64, rules: &Rules) -> Result<FamilyReadOutcome, FamilyReadError>;
```

Exact semantics (from the design, normative):
- `classify_dead` = NO candidate wins ANY replication AND (max mean − min mean) ≤ bound.
  PIN AT THE MEASURED SHAPE: a constructed adjudication with 15 distinct means, spread
  3.76, zero wins ⇒ DEAD; the same with one win ⇒ LIVE; spread 5 ⇒ LIVE (MUST-1's fix).
- `admission`: mean margin over predicate-TRUE playouts minus mean over predicate-FALSE
  playouts ≥ δ (control: ≤ −δ), BOTH groups non-empty else `CannotDemonstrate`
  (recorded-excluded); per-policy; the caller assembles the conjunction and the
  sole-dissenter flag (lookahead-only dissent → collected, routed — MUST-4). δ is a
  parameter; T7's freeze supplies the value.
- `family_read` replication seed = `mix(SALT_DRIVEN_FAMILY_READ, [master, root_word, r])`
  where **`root_word` = `u64::from_str_radix(&state_digest(root), 16).unwrap()`** — the
  digest is exactly sixteen hex chars encoding one u64, so the WHOLE value is the word
  (plan review SHOULD-1; the seed-distinctness pin asserts this derivation word for word).
  The new salt lives beside `SALT_DRIVEN_REPLICATION` with a cross-pointer comment at the
  D21E family's home (NOTE-6). Family-1 grain: `root_table` per replication, pick judged by
  the caller against per-policy top sets (this fn returns picks; the three-way
  all/some/none rule lives in the family-run assembly, T5). Turn grain: `drive_turn` per
  replication, `TurnPredicate::eval(root, handoff)`. **The outcome carries, per
  replication, the DERIVED DECISION SEED and `root_word` verbatim** (plan review SHOULD-4 —
  T7's separator step feeds a failing replication's seed straight to `table … 
  --decision-seed`; without it the orchestrator hand-computes the mix, the registry
  recorded-seed lesson in reverse). Counter check: one `root_table` at replication 0's seed
  + every turn replication's `SeatCounters`; cap/bind/starved ⇒ the typed `CounterRefusal`
  arm (branch-0 routing), tie/band ⇒ annotations on the outcome (NOTE-5's split).
- Anti-vacuity/mutation pins (SHOULD-9): every `TurnPredicate` has an inversion control on
  a hand-built (root, handoff) pair; the δ group arithmetic on a hand-built adjudication
  (rows with known verdict mixes — include the 10–10 margin-0 trap); the hook's verdict
  pinned against a hand-walked playout (force a candidate on a near-terminal fixture, walk
  the mover's turn by hand, compare); seed-distinctness (two roots, same master, different
  words); determinism (same inputs → byte-identical outcome twice).

## T5 — builders + the family registry (design §1 families 3–4, §2 `FamilyDecl`; discharges
SHOULD-3/4's structures, NOTE-6)

**Files**: `driven_families.rs` (continued).

**Produces:**
```rust
pub fn build_race_positive(seed: u64, rules: &Rules) -> Result<(GameState, ContestedStep), BuilderError>;
pub fn build_race_no_denial(seed: u64, rules: &Rules) -> Result<(GameState, ContestedStep), BuilderError>;
pub fn build_achievement_proximity(seed: u64, rules: &Rules) -> Result<GameState, BuilderError>;
pub enum FamilyLabel { Confirmatory, ExistenceDemonstration }
pub enum StrategicClass { RulesFinalRound, EndTimingProximity, OpponentReply }
pub struct FrozenBlock { pub master: u64, pub ordinals: std::ops::Range<u32> } // baked at the freeze commit
pub struct FamilyDecl { pub id: &'static str, pub label: FamilyLabel, pub class: StrategicClass,
    pub generator: FamilyGenerator,   // Predicate(RootPredicate) | Builder(fn(u64,&Rules)->…)
    pub rule: PassRule, pub replications: u32, pub per_root_f: u32,
    pub menu: &'static [ReplyPolicy], pub pilot: std::ops::Range<u32>,
    pub confirmatory: Option<FrozenBlock>,   // None until the freeze commit
    pub canon: &'static str }
pub fn families() -> Vec<FamilyDecl>;        // exactly four; ESCROW ordinals deliberately absent
pub struct FamilyBlockReport { /* per-root: admission tables + read outcome + three-way
    judgment (family 1); the family verdict, exclusion rate, per-policy dissent breakdown,
    sole-dissenter collection, and the VOID-below-4-admitted rule applied */ }
pub fn run_family_block(decl: &FamilyDecl, block: BlockKind, master_override: Option<u64>,
                        subject: SeatSpec, rules: &Rules) -> Result<FamilyBlockReport, FamilyRunError>;
pub enum BlockKind { Pilot, Confirmatory }
```

Builders on the `scenario_suite` grain (its `keep_game(2, seed, rules)` then doctor-fields
pattern — cite `tests/scenario_suite.rs:446` in-source): each builder derives its variation
from `seed` (SplitMix over declared parameter ranges — which areas, which track, how much
yen — ranges in the doc comment), ends with `check_invariants` + its own SHAPE SELF-CHECK
(race positive: the opponent is one completed action from the declared step AND the mover
can alter it AND the threat is public — no reliance on hidden deal; no-denial control: the
denial-shaped step exists and taking it forfeits a strictly-better arithmetic line, the
construction arithmetic written out; proximity: the F8 produce-then-claim line is legal
within one turn). Builder pins: two seeds produce distinct digests; self-checks hold across
a seed sweep (0..8); invariants clean; `ExclusiveRaceOneStep::holds` is TRUE on
`build_race_positive` output (the predicate/builder coherence pin). `families()` carries
the four declarations exactly as the design's §1 defines them (menus: family 1 the full
four; families 2–4 the full four with per-policy K left to T7's freeze), pilot ordinals
0..4, `confirmatory: None` — a test pins that NO declaration carries a frozen block yet
(the pre-freeze state) and that escrow appears nowhere in the module (grep-shaped test on
the declarations, not the source text: no third block field exists by construction).

`run_family_block` is the block EXECUTION (lib logic — T6 only parses argv and renders the
report): resolve the block's draws **in ONE scan pass per block** (plan review NOTE-8 —
`generate_root` re-scans from game 0 per ordinal, and a 6-ordinal v2 block would pay the
both-seats-lookahead scan repeatedly; batch-resolve the block's ordinals in a single walk,
descriptors identical to per-ordinal resolution, pinned equal on the rung ecology), run the
admission adjudications under the declared menu (with the turn hook for turn-grain
families; per-policy K from the decl once frozen, a uniform pilot K before), apply
`admission` + `classify_dead` (**DEAD is judged PER POLICY and AND-ed across the declared
menu — dead means dead under EVERY policy, spread bound applied per policy** — NOTE-2), run
`family_read` on admitted/LIVE roots, assemble family 1's three-way all/some/none judgment
(pass / UNRESOLVED-pending-exact / fail) from the per-policy top sets, apply the family
verdict rules (per-root F, family allowance, the VOID rule below 4 admitted,
sole-dissenter collection) — **with the ExistenceDemonstration exemption explicit**
(NOTE-3): family 3's verdict arm SKIPS the failure allowance entirely and emits the
per-root gap block instead, and that arm gets its own smoke. Family 4's report block additionally prints the
recorded RIDER: the root's shipped `achievement_progress_sum` beside the race-proximity
reading — the proximity arithmetic lives in the probe example, so `driven_families` hosts a
thin mirrored equivalent NAMING the probe source in-source (the harness design §5's own
pattern for example-local logic). Pins: a smoke-scale `run_family_block` on a constructed
family (rung subject, tiny R) exercising pass, fail, excluded, and VOID arms; the
three-way judgment on hand-built top-set combinations; the rider's mirror pinned equal to
a hand-computed value on a constructed state.

## T6 — runner surfaces (design §2 runner block; discharges SHOULD-4's refusal, NOTE-3/4)

**Files**: modify `crates/yokohama-core/src/game/driven_runner.rs` (+ the example's usage
line only if the dispatch table needs the new verbs).

Grammar (parse + render lib-tested, the example stays print-only):
- Root spec `gen:<predicate-name>:<master-hex>:<ordinal>` accepted wherever
  `<fixture|path:event>` parses (table/run/adjudicate). Predicate names = the
  `RootPredicate::as_str` values; unknown name → typed rc=2 refusal naming the valid set.
- The seat grammar (`parse_seat`, driven_runner.rs:394–401) gains `race-rusher` and the
  USAGE seat-name list is updated with its pin (plan review SHOULD-3's decision: argv-
  REACHABLE, because T7's pilots want manual `adjudicate --seats race-rusher,…`; the
  literal policy array and USAGE list are both hand-maintained, so the T6 pins assert the
  new name parses and round-trips).
- `family <id> --block pilot --master <hex>` and `family <id> --block confirmatory`: parses
  to a `run_family_block` call (T5's lib fn — the runner owns NO execution logic; pilot: the
  caller's master, MANDATORY; confirmatory: the `FrozenBlock` master, `--master` REFUSED
  with a sentence naming why) and renders `FamilyBlockReport` — label first, counter check
  second, per-policy admission table, per-root replication line, binomial line,
  exclusion/dissent block. Before the freeze commit, `--block confirmatory` on a
  `confirmatory: None` declaration refuses by name (pinned).
- `opening --master <hex> --games <N> [--contestant <spec>]`: N seeded fresh 2P games,
  **setup via the PUBLIC path** (plan review SHOULD-5 — the Phase-B split seamed out only
  `play_from_state`; the setup half is still interior to the arena's private loop):
  `new_game` + each seat's `ArenaAgent::choose_starting_order` + `finalize_setup`, with a
  SETUP-EQUIVALENCE pin — the built opening state equals `state_at(record, 0)` of a
  `run_arena_game` at the same master and streams (rung seats, one cell). Then the subject
  plays the FIRST TURN of the first-moving seat via `drive_turn`; aggregate (placement
  areas, president area, purchase kind) distribution + lab-rush share with a Wilson
  interval; the setup-path line printed in the output (design NOTE-4).
Pins: parse round-trips per verb; refusal arms (unknown family id, confirmatory+--master,
pilot without --master, unknown predicate); render pins on constructed outcomes (the
report's section ORDER pinned — label first, counter check second); USAGE text updated and
pinned; `list` unchanged byte-identical.

## T7 — freeze, pilots, confirmatory runs, D7 routing, close (orchestrator-led; design §4–§5)

Not agent-implemented; the orchestrator runs it with the shipped binaries, recording every
command. Steps:
0. **Masters ledger row FIRST** (before any non-test run): the next free A3E1 2026 0818
   serials by order — one scan master per generated family (family 1's TWO predicates SHARE
   one scan master, a distinct predicate being a distinct ordinal space at the same master
   — legitimate and stated; then 2-positive, 2-control, 4), one seed master per builder
   family (3-positive, 3-control, 4-proximity corner), ONE admission/read master per family
   (sound because the two instruments mix under DIFFERENT salts —
   `SALT_DRIVEN_REPLICATION` …0006 vs `SALT_DRIVEN_FAMILY_READ` …0008 (0007 = T3's
   `SALT_DRIVEN_RACE_PROBE`) — said out loud,
   plan review NOTE-5), plus the opening-probe master. 9999-scratch for any command-shape
   smoke.
1. **Pilots** (`--block pilot`): supply rates (live-root rate for family 1;
   Exhausted-or-not for `ExclusiveRaceOneStep`), per-root world-sensitivity (drives F),
   playout-margin sd (drives δ/K), adjudication-margin sd at family-1 pilots (drives ε),
   wall costs. **Cost expectation carried for scheduling** (plan review NOTE-8): an
   admission adjudication at menu 4 × K 4 × ≤60 candidates with lookahead replies is
   TENS OF MINUTES per root in release — the pilot prices it and the freeze sizes K
   accordingly; budget T7's runs in hours, not minutes. Nothing confirmatory quotes a
   pilot.
2. **The freeze COMMIT (code+doc)**: bake `FrozenBlock`s (masters + strided confirmatory
   ordinals) into `families()`; set F/K/δ/ε from the pilot-derived conservative UCBs;
   author `docs/plans/2026-08-18-phase-c-family-freeze.md` with every §5.2 content item
   (subject echo, generator identities, RaceRusher rule verbatim, escrow ordinals SPLIT
   3+3 by consumer, **the VOID rule's pre-declared EXTENSION ordinals per family** — plan
   review MUST-2, without which the VOID disposition is unexecutable — error arithmetic
   under the declared benign-miss model, strategic-class partition,
   WHAT-THIS-DOES-NOT-MEAN).
3. **Confirmatory runs**, one command per family block, stdout preserved to the scratchpad
   and quoted in `docs/plans/2026-08-18-phase-c-family-runs.md`; the opening probe runs
   beside them, and a deck-clock corner root is RECORDED (not gated) for family 2 if the
   scan supplied one (the design's §1 clause).
4. **D7 routing** per the design's §5.4 (hard-refuse vs annotate; separator on tie-mass
   roots at b=4/8/16 via `table gen:...` + `--contestant 'lookahead-v1:b=8'`; verdicts
   recorded); dispositions ledger rows; the sole-dissenter and exclusion blocks read.
5. **Close**: guidingpath Phase-C decision row; CLAUDE.md updates (runner command line +
   core/cli counts + the families module bullet); execution-ledger rows; the close handover
   `docs/plans/2026-08-18-phase-c-close-handover.md` (read order, owner asks — the STOP
   routings — the D2-major call, next-phase entry); PR body with the full trail; Fable-max
   whole-branch review; merge on a comfortable verdict under the owner's granted authority;
   memory-file update. **The design's stretch task (the `eval_probe` tie-share counters
   line) is CUT from this phase** — recorded here so the design's "cut first" clause reads
   discharged, not forgotten; it stays Phase D's named instrument gap.

## Task sequencing and review cadence

T1 → T2 → T3 → T4 → T5 → T6 strictly sequential (same files/module chain). Each task: one
Opus-max implementer (RED commit, GREEN commit, self-review hardening commit allowed), then
one Fable-max reviewer (verification the strong form: re-derive arithmetic by hand, re-run
suites, mutation probes with escapes named), orchestrator adjudication, fix wave by an
Opus-max agent if findings demand. Suites + clippy green at every task boundary. T7 opens
only after T6's adjudication closes.
