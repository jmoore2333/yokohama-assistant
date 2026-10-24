# Contributing

Yokohama Assistant is a Rust rules engine for the board game Yokohama (Hisashi Hayashi, 2016), with a command-line
game driver and live tracker, a browser client with two server hosts, and a computer player built to play to win.
The engine is the one authority on legality: every surface asks it, and none computes a rule of its own.

Three documents to read before a change:
- [`guidingpath.md`](guidingpath.md) is the authority: the mission, the current state, the roadmap and the decision
  log, which holds every rules ruling.
- [`docs/handover.md`](docs/handover.md) is the board, a pointer to the tracks in flight and their next steps. It
  never outranks the other two.
- [`CLAUDE.md`](CLAUDE.md) holds the standing rules every session and every contributor reads: the commands, the
  gates, the STOPs and the conventions.

## How a change lands

1. Work on a branch, and open a pull request to `master`.
2. Write the failing test first. The history is strictly test-first, and a change keeps it that way.
3. Run the per-edit suites and the lint gate after every change. Each passes with zero warnings:

   ```bash
   cargo test -p yokohama-core
   cargo test -p yokohama-cli
   cargo test -p yokohama-oracle
   cargo test -p yokohama-difftest
   cargo test -p yokohama-bridge
   cargo test -p yokohama-core --example eval_probe
   cargo test -p yokohama-core --example solver
   node bridge/tests/outbox_race.js && node bridge/tests/wrapper_transparency.js && node bridge/tests/safe_table_parity.js && node bridge/tests/connected_indicator.js
   cargo clippy --workspace --all-targets -- -D warnings
   ```

   The two example targets run under no other command, and the four node suites cover the Board Game Arena
   bridge's browser half, which no cargo suite reaches. A clone without the two captures sets the variable in
   [The capture corpus](#the-capture-corpus) below first.
4. Run the web lane's suites too whenever a change reaches what the layer compiles: `crates/yokohama-core`,
   `crates/yokohama-cli`, `SEMANTICS_REV` or the workspace version.
   [`docs/web-lane.md` §2](docs/web-lane.md#2-the-suites-and-the-gates) lists them with their order rules.
5. A whole-branch review reads the pull request before it merges.

A change to the CLI's behaviour or its text updates both [`docs/cli-guide.md`](docs/cli-guide.md) and
[`docs/cli-guide.html`](docs/cli-guide.html) in the same commit, word for word; the `guide_sync` test holds part of
that. A new design decision is a row appended to the decision log, whose rows are never edited.

The toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml). `rustfmt` is deliberately not adopted, so
never run `cargo fmt`: it would reformat the whole repository.

## The STOPs

Three kinds of change do not land through a pull request alone. A change that touches one is proposed as an issue
first, and it lands with its row in the decision log.

- **`SEMANTICS_REV`** in `crates/yokohama-core/src/game/semantics.rs`. Any behavior-affecting change to legality,
  application, scoring or setup bumps it in the same commit. Records and saves are keyed to it, so a save stops
  resuming across a bump, by design.
- **The oracle.** `crates/yokohama-oracle` is an independent reference, authored from rules sources only (the
  English rulebook, the knowledge base's rules reference and the decision log) and never from the engine's logic.
  The oracle independence rule in `CLAUDE.md` says what that means, down to the imports the crate may hold.
- **The computer player's freezes and pre-registrations.** A frozen identity is never edited, and a different
  weight table is a different named identity. The map
  [`docs/plans/2026-07-25-phase3-map.md`](docs/plans/2026-07-25-phase3-map.md) governs that work.

## The capture corpus

Fourteen pins read two Board Game Arena captures that are gitignored, so a fresh clone does not have them: all nine
tests in `crates/yokohama-cli/tests/bga_import.rs` and five in `crates/yokohama-bridge/tests/feed_preprocess.rs`.
Without the captures those pins panic with a message naming what is missing, deliberately. A clone without them sets
`YOKOHAMA_BGA_CORPUS_OPTIONAL=1` in the run's environment, which turns the absence into a quiet skip. With it set,
those pins pass without asserting anything.

## The rules sources

The engine answers to these, and so does a rules report:
- the 2017 Tasty Minstrel English rulebook, 16 pages, whose page numbers are the ones the record cites;
- the 20-page edition in the Tabletop Simulator upload, whose setup steps are on its p.5;
- the knowledge base's rules reference,
  [`analysis-1/yokohama-kb/01-rules-reference.md`](analysis-1/yokohama-kb/01-rules-reference.md);
- the rulings in the decision log, kept in `guidingpath.md` and its three archives, whose rows remain authoritative.

Neither rulebook is in the tree; a contributor obtains their own copy. Where the editions differ or a source is
silent, a ruling in the decision log is the rules answer. When the engine disagrees with the table, the **Rules
question** issue form is the report this project most wants.

## AI-assisted work

AI-assisted work is welcome, from an issue to a pull request. The one ask is the same as for any contribution: the
person submitting is comfortable with what they send and comfortable discussing it, even if a model is coaching them
through that discussion on the side.

## Issues and pull requests

Open an issue with one of the two forms, **Bug report** or **Rules question**, or a blank issue for anything else.
A security vulnerability is reported privately, never in an issue: [`SECURITY.md`](SECURITY.md) says how. A pull
request opens on the template's checklist: what changes, the suites run, the documents updated, and the ask above.

A blank issue that proposes a change — a STOP's proposal among them — is written in the project's layout: Context ·
What happens today · The change · Where · Acceptance · Size · Record to update when it lands.
