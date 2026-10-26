# How the work is organized

How Yokohama Assistant is built and what it allows itself to claim: the workflow, how a cycle of work lands, the
decision log and the contracts that guard the rules, and the rules of evidence behind every strength figure. The
standing rules are in [`CLAUDE.md`](../CLAUDE.md), and [`guidingpath.md`](../guidingpath.md) outranks every page.

## 1. The workflow

Work moves brainstorm → spec → plan → execute. Specs and plans are dated files in [`plans/`](plans/), named
`YYYY-MM-DD-topic.md`; once its phase closes a dated artifact is frozen, and a correction is recorded forward in a later
document, never edited in, except where a cycle names an exception and records it. The computer player's ladder of sub-phases is built the same way: each one below the bar, the
[Phase-3 map](plans/2026-07-25-phase3-map.md)'s §1.9 milestone bar, is one spec, one plan and one test-driven pass, with
a gate, executable tests the next one waits on (the map, §3 and §5). No phase is called complete until its gate passes.

Execution is test-driven: the failing test comes first, and the commit history keeps that order. After every task the
per-edit suites, which [`CONTRIBUTING.md`](../CONTRIBUTING.md) lists, and the lint gate,
`cargo clippy --workspace --all-targets -- -D warnings`, run green with zero warnings. A change that reaches what the
web layer compiles also runs the web lane's suites ([`web-lane.md` §2](web-lane.md#2-the-suites-and-the-gates)).

Before a merge or a phase close, the fifteen-gate release roster runs (§5 lists its commands). It is measured as one
uninterrupted sequence with the release binaries built first, single-threaded exactly where a command says so. Every
wall figure it yields is tagged with the machine that measured it, and a new machine's figures are recorded beside the
old, never against them. An ignored test that no documented gate command runs is a test that never runs, so every
release-tier test binary has a gate command of its own.

## 2. How a cycle lands

A cycle runs on its own branch from a written plan. Each task carries a brief with the exact values to use, and the
plan's Review record carries every correction the run meets, at its site; a correction to the plan's code is made in
the plan first, so the plan stays the single source for what the run builds.

The tasks run one at a time, with one implementer at a time in the checkout. The implementer probes the brief's code
before building on it, works test-first, runs the suites, stages by explicit path, commits and reports. A fresh
reviewer then reads the task on an export of the head it reviews (`git archive`, with its own `target/`), never in a
tree an implementer holds, and runs mutants to show that each new test fails when the code it guards is broken. A
finding goes back as a fix round, landed as a new commit.

The branch is then read whole before it merges: a whole-branch review, one fix wave that answers its findings, and a
scoped re-review of that wave. A pull request carries the branch to `master`, and the merge waits on the re-review.

The close is the record of record: a dated document carrying the run, its measurements, every ruling with its cost,
the review findings and the lessons. At the close the board ([`handover.md`](handover.md)) is updated, and a genuinely
new standing rule goes into `CLAUDE.md` in a sentence or two. The security S4a
[plan](plans/2026-10-23-security-s4a-plan.md) and [close](plans/2026-10-23-security-s4a-close.md) are one whole cycle.

## 3. The decision log and the `SEMANTICS_REV` contract

The decision log in [`guidingpath.md`](../guidingpath.md) carries every rules ruling and the project's design
decisions. It is append-only: a row is never edited, and a correction is a later row. Older rows live verbatim in three
archives, each closed to new rows, and an archived row remains authoritative wherever it is cited:
[the engine era](../guidingpath-archive.md), [the computer player's road to V4](../guidingpath-archive-phase3.md) and
[the web lane](../guidingpath-archive-web.md).

`SEMANTICS_REV`, in `crates/yokohama-core/src/game/semantics.rs`, is the engine's rules revision. It is bumped in the
same commit as any behaviour-affecting change to legality, application, scoring or setup, and each revision's scope is
a row in the decision log. Saves and records are keyed to it, not to the crate version, so a save stops resuming
across a bump, by design.

The oracle, `crates/yokohama-oracle`, is an independent referee authored from the rules sources alone: the OKAZU Brand
English rulebook, the knowledge base's [rules reference](../analysis-1/yokohama-kb/01-rules-reference.md) and the
decision log's rulings. It is never written from the engine's logic, its imports stay types-only, and an agent that
knows the engine's implementation never authors its rules logic. The differential gates compare the engine against it,
and that independence is what gives a passing comparison its meaning. A change that touches `SEMANTICS_REV` or the
oracle, or the computer player's freezes and pre-registrations, stops for the owner's ruling; from outside the project
it is proposed as an issue first.

## 4. Pre-registration and the rules of evidence

A strength claim is registered before its field runs. The registration names the claim, the cells that test it, their
size, their masters or how each is drawn, and the rule that decides the outcome. It is committed before any of its cells
runs and never edited once one has ([the 3.4 pre-registration](plans/2026-08-04-phase3.4-preregistration.md) is one): a
run that departs from it is a new, separately declared attempt, and a change is an amendment, recorded forward.

- **Thresholds come from measured variance,** never a preset sample size or an arbitrary percentage (the map, §1.4).
- **Masters are recorded before use.** Each is committed in advance and labelled diagnostic, the form every gate cell,
  fixture and reproduction uses, or drawn fresh from OS entropy. Each milestone with a figure on the ladder rests on
  at least one cell run on a drawn master.
- **An identity never re-points.** A figure is recorded against a contestant's full identity, its name with its
  versions, knobs and weight table, and a different table is a different identity, never a knob or an edit.
- **Nothing is tuned to turn a miss green.** A re-attempt is a counted attempt on a fresh master.
- **Every figure is quoted as recorded, with its record named.**
- **A one-sided bound is called one-sided,** never presented as a two-sided interval.
- **An observed maximum is "max observed under harness X",** never a bound: the space behind it was not searched.
- **No direction is asserted where none was measured.** One cell in the repository asserts a direction, the evaluator
  gate's promotion cell, which reads the 3.3 claim that `lookahead-v1` beats the `family-weighted` rung.

The milestone bar is defined in §1.9 of the [Phase-3 map](plans/2026-07-25-phase3-map.md) and quoted on
[the computer player page, §6](computer-player.md#6-the-bar-not-claimed); the bar is the owner's to declare.
The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.

## 5. Results

The results ladder is [the computer player page, §5](computer-player.md#5-the-results-ladder): one entry per strength
milestone, oldest first, each with the claim as registered before its field ran, the figure as measured and the record
that holds both. This page states no figure of its own. The end result as it stands: the shipped bot plays at the
hosted site, [yokohama.moore.nyc](https://yokohama.moore.nyc), in the command-line client, and in the browser client on
your own machine. The suites are the ones §1 names, and the release roster is these commands, run as one sequence:

```bash
cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored
cargo test --release -p yokohama-core --test choice_encoding -- --ignored
cargo test --release -p yokohama-core --test observation_honesty -- --ignored
cargo test --release -p yokohama-core --test action_source_equivalence -- --ignored
cargo test --release -p yokohama-core --test action_source_worstcase -- --ignored
cargo test --release -p yokohama-core --test arena_gate -- --ignored --test-threads=1
cargo test --release -p yokohama-core --test policy_diversity -- --ignored
cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1
cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1
cargo test --release -p yokohama-core --lib -- --ignored
cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored
cargo test --release -p yokohama-difftest --test differential -- --ignored
cargo test --release -p yokohama-difftest --test routes3_differential -- --ignored
cargo test --release -p yokohama-difftest --test scoring_differential -- --ignored
cargo test --release -p yokohama-difftest --test routes3_worstcase -- --ignored
```

## 6. Reviews across models

Reviews run across models where a cycle's record says so.
