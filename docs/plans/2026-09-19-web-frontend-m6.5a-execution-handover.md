# M6.5a "the seam" — the execution handover

**Written 2026-09-19 on branch `web-room-m6.5` at `43ec58e`, by the orchestrating session that
re-chartered M6.5, wrote its umbrella spec, the M6.5a spec and the M6.5a plan — for the fresh
session that EXECUTES M6.5a.** Nothing of the plan has been executed: every commit on the branch
is a document. The owner ruled: a fresh session, subagent-driven development from the resume
point below.

## 0. Read order (the session's first ten minutes)

1. `CLAUDE.md` — the Web (server) and Web (browser half) bullets, the `crates/yokohama-web`
   architecture bullet, and the M6.5 sentence in "Where things stand".
2. `docs/handover.md` Track 3.
3. `docs/plans/2026-09-19-web-frontend-m6.5a-plan.md` — the whole thing (eleven tasks, six
   plan-level rulings, the close checklist, the self-review).
4. `docs/plans/2026-09-19-web-frontend-m6.5a-seam-spec.md` — §1 (R-6.5a.1–16) and §6.3 (the
   fifteen mutations the close records).
5. `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` — §1 (R-6.5.1–13), §2.1, §6.1.
6. This document's §2 and §3.

## 1. Where the branch stands

`web-room-m6.5` is stacked on `web-room-m6.4` at `faf1a88`; ten commits, all documents:
`2f5381a` umbrella spec · `4051b18` pointer pass · `49d90a7` first fix wave · `1b4c0e0` ·
`a4d238c` second wave · `776ca7e` M6.5a spec · `4dc25a1` five flags · `6999af6` the plan ·
`43ec58e` the board. **Every source file is byte-identical to `faf1a88`**, so every line
citation in the plan holds at this HEAD; the plan's own rule stands — re-read each cited range
at the HEAD a task opens, because Tasks 1–11 move them.

Not merged: PRs #31 → #34 stack `web-room` … `web-room-m6.4`; merge order is the owner's; M6.5's
PR will target `web-room-m6.4` in the same shape.

## 2. The process (R-P11 as refined through M6.4; the plan's R-plan-1–6)

- **Subagent-driven development**: a fresh **Opus** implementer per task, a per-task **Opus**
  reviewer, Fable orchestrating; the whole-branch Fable review BEFORE the release (R-order-1),
  one fix wave, one scoped re-review. Opus is the floor for both roles (owner, 2026-09-02).
- **The SDD workspace** is `.superpowers/sdd/2026-09-19-web-frontend-m6.5a-plan/` (gitignored):
  every brief, report, review and the ledger go there. **Every agent writes its full report to a
  transcript file in it and replies with the path plus three lines** — the notification channel
  truncates at about 4k.
- **Dispatch says run in the FOREGROUND or poll the output file**; a subagent that backgrounds a
  long run and "waits for events" idles forever. When a status stalls, `ls -lt` its workspace and
  wake it with the facts.
- **Never follow up an implementer without `git log -1` and `git status` first; never rewrite a
  reviewed SHA** — a correction lands as a new commit on top.
- **The corpus is regenerated at Tasks 5, 6, 7 and 9 and its diff read every time** (R-3-1); the
  expected diffs are stated in the plan's R-plan-2: none, none, `health.json` +1 key,
  `record_409.json` new. Any other diff is a defect in the task.
- **Per-edit gates after every task**: `cargo test -p yokohama-web`, `cargo clippy --workspace
  --all-targets -- -D warnings`, and from Task 4 on `cargo check -p yokohama-web
  --no-default-features --target wasm32-unknown-unknown`; `cd web && npm run typecheck && npm test`
  when `web/` moves. The whole-game gate's vs-bot arm may livelock on an OS-entropy seed
  (R-plan-6): re-run it alone once before believing a red.
- **The fenced crates and the CLI are not touched**: `git diff faf1a88 -- crates/yokohama-core
  crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
  crates/yokohama-cli` stays empty at every commit.

## 3. Pre-flight facts the executing session should not re-derive

- `wasm32-unknown-unknown` and `wasm32-wasip1` are installed on the pinned toolchain
  (`rustup target add … --toolchain 1.97.1`) on the Mac that wrote this; a new machine adds them.
- The throwaway spikes that measured the wasm claims lived in the writing session's scratchpad
  (`wasm-spike/`, `eprintln-probe/`) and are NOT tracked; their findings are in the umbrella's
  §0.3 and the M6.5a spec's §0.4. Nothing in the plan depends on them existing.
- **Task 5 and Task 6 depend on axum 0.8.9's exact rejection wording**: read
  `~/.cargo/registry/src/*/axum-0.8.9/src/extract/rejection.rs`, `src/json.rs` and
  `src/extract/path/mod.rs` before writing the layer's parsers; the pins against axum's own
  `body_text()` decide, and the plan's strings are corrected in the commit that lands them
  (R-plan-3).
- `Games`' public method signatures are frozen (R-6.5a.1, R-plan-4); `with_parts` grows from four
  parameters (Task 1) to six (Task 7) and `with_limits` fills the last three with `NoSignal`,
  `|| 0`, `NoArchive`.
- The scanning cell (Task 4) enforces a source convention: a `server`-only region inside a mixed
  file begins with `#[cfg(feature = "server")]` and ends with a line `// ---- end server ----`
  at column 0; whole `server` files begin with `#![cfg(feature = "server")]`. Write it into
  `lib.rs`'s module doc in the same task.
- Task 10's client cells name helpers by shape (`renderGameAt`, `wire.gameOver`); the existing
  test files carry the real names — reuse them, never add a second helper.

## 4. The resume point

1. Create the SDD workspace directory and its ledger.
2. Dispatch Task 1 (the `Signal` trait) to an Opus implementer with the plan's Task 1 text, the
   global constraints, and §2's process rules; review; commit; continue in order.
3. At the close: the plan's close checklist — the whole-branch review, the fifteen-mutation
   battery, the counts in one sequence, the NAS deploy as `yokohama-web:0.5.0-m6.5a` with
   `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`, the close document, and the M6.5b handover (the
   M6.5b spec is written at ITS start from the close, per R-6.5.7).
