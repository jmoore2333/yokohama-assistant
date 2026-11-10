# Contributing

Yokohama Assistant is a Rust rules engine for the board game Yokohama (Hisashi Hayashi, 2016), with a command-line
game driver and live tracker, a browser client with two server hosts, and a computer player built to play to win.
The engine is the one authority on legality: every surface asks it, and none computes a rule of its own.

Three documents are worth reading before a change: [`guidingpath.md`](guidingpath.md), the authority, whose
decision log holds every rules ruling; [`CLAUDE.md`](CLAUDE.md), the standing rules, commands and gates; and
[`docs/handover.md`](docs/handover.md), the board, a pointer to the work in flight that never outranks the other two.

## How a change lands

Work on a branch and open a pull request to `master`. The project's own cycles are developed in a private working
repository and arrive here whole, each as one reviewed pull request, so `master`'s history is the merges of finished
branches; your pull request is opened here, like theirs. Write the failing test first, then run the per-edit suites and
the lint gate, each green with zero warnings:

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

A change that reaches what the web layer compiles (`crates/yokohama-core`, `crates/yokohama-cli`, `SEMANTICS_REV` or
the workspace version) also runs the web lane's suites, which
[`docs/web-lane.md` §2](docs/web-lane.md#2-the-suites-and-the-gates) lists. A clone without the two gitignored Board
Game Arena captures sets `YOKOHAMA_BGA_CORPUS_OPTIONAL=1`, which turns fourteen pins' deliberate panic into a skip.

## The STOPs and the rules sources

`SEMANTICS_REV` (`crates/yokohama-core/src/game/semantics.rs`) is bumped in the same commit as any behavior-affecting
change to legality, application, scoring or setup, and the oracle (`crates/yokohama-oracle`) is authored from rules
sources only, never from the engine's logic. A change that touches either, or the computer player's frozen
identities and pre-registrations, is proposed as an issue first.

The engine answers to the OKAZU Brand English rulebook (16 pages) and the Tasty Minstrel Games English edition (20
pages, in the Tabletop Simulator upload), neither of which is in the tree, to the knowledge base's
[rules reference](analysis-1/yokohama-kb/01-rules-reference.md), and to the rulings in the decision log, which settle
the rule where the editions differ or are silent.

## Dependencies

Dependencies move at two moments: when a cycle opens or a release is cut, and when a security advisory reaches
something that ships. Nothing moves on a schedule, and the repository takes no automated update pull requests.

- **A routine move takes a release at least one week old.** A tampered release is usually caught within days, and
  passing tests would not show it. For the two npm packages that is one flag, in `web/` and in `worker/`:
  `npm audit fix --package-lock-only --before=<the date a week ago>`. For Rust it is
  `cargo update -p <crate> --precise <version>`, one crate at a time and never a blanket `cargo update`, which moves
  crates no one read — `serde_json` among them, which writes the byte-pinned wire corpus and the records.
- **The one exception is a fix that is needed.** An advisory that is reachable in what ships — a Rust crate, or the
  browser client's `react` and `react-dom` — and rated high or critical takes its patched release at once, whatever
  its age.
- **Everything development-only waits for the next of those moments.** GitHub's alerts do the watching in between,
  so no one reads the advisory list at every push: at a batch, read what is marked runtime, and run the commands for
  the rest.
- **`wrangler` and `@cloudflare/vitest-plugin` move as a pair**, by the command at the head of
  [`worker/vitest.config.ts`](worker/vitest.config.ts), which says why.

A lockfile is a change like any other: the per-edit suites above, and the web lane's when `Cargo.lock` or a lockfile
under `web/` or `worker/` moved.

## AI-assisted work

AI-assisted work is welcome, from an issue to a pull request. The one ask is the same as for any contribution: the
person submitting is comfortable with what they send and comfortable discussing it, even if a model is coaching them
through that discussion on the side.

## Issues

There are two forms, **Bug report** and **Rules question**, and a blank issue is fine for anything else. A proposal
that touches a STOP is welcome as an issue first. If a layout helps, most of the project's own issues use this one:
Context · What happens today · The change · Where · Acceptance · Size · Record to update when it lands. A security
vulnerability is reported privately, as [`SECURITY.md`](SECURITY.md) describes.
