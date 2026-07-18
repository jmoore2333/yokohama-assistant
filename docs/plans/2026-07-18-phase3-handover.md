# Phase-3 handover — prompt for a fresh agent (2026-07-18)

> Paste everything below the line into a new Claude Code session in this repo.

---

You are picking up the **Yokohama assistant** project (Rust engine + CLI tracker + planned AI player for the board game Yokohama, Hisashi Hayashi 2016). Two phases are DONE and merged to master: the rules engine (15,000-game fuzz gate, 390 tests) and the Phase-2 CLI game driver (simulated games playable end-to-end; live-tracking machinery built and integration-tested; 39 CLI tests; final whole-branch review: ready-to-merge, unconditional). Everything below assumes you have read nothing else yet.

**Read in this order before doing anything:**
1. `guidingpath.md` — the authoritative status, roadmap, and decision log (every rules ruling lives there). Pay particular attention to: the *Current state* section, the Phase-2 subsection **"Deferred by the Phase-2 final review — REQUIRED spec inputs for the next plan"**, and the 2026-07-18 decision-log rows.
2. `CLAUDE.md` — commands, architecture, conventions (auto-loaded, but read it deliberately once).
3. `.superpowers/sdd/progress.md` — the execution ledger for both finished phases: per-task commits, review verdicts, every adjudication and riding minor. This is the project's institutional memory.
4. `docs/plans/2026-07-18-phase2-cli-driver-design.md` §5.4/§5.7 and `docs/cli-guide.md` §8 — how live tracking works and how the CLI's language maps to the rulebook.

**A note on phase naming:** Jeremy calls the next block of work "Phase 3". The roadmap's *Phase 3 — computer player ladder* only begins after the **Phase-2 gate** (live-track one full physical game without fighting the tool) has passed — and that gate is deliberately blocked on the **choice-encoding plan**. So your work order is:

## 1. Choice-encoding plan (first — it completes the Phase-2 gate)

Produce a dated design + implementation plan (brainstorm → design doc → plan, per repo convention), consuming guidingpath's "Deferred by the Phase-2 final review" list as the spec:

- **Gate-blocking (items 1–2 of that list):** Move encodings for every v1 auto-resolution — any-good picks, warehouse-icon takes, Church/Customs space choice, University / Textile Mill (rulebook name "Spinning Mill") / Telephone-Recovery, the 5-Power Movement token's real move, the Skip-forfeits-Port-+3VP revisit — plus the wizard-robustness cluster (parse-time card validation, re-ask flows, end-of-wizard review). New Move variants auto-surface in the CLI (numbered options + one `describe_move` arm each; keep `describe_move` wildcard-free so the compiler enforces coverage). **Engine invariant to preserve:** bound reveal events are pushed in the same `Session::apply` commit as their move, or undo grouping breaks.
- **Riding backlog (items 3–8):** live-driver hardening (event-index errors, tech desync assert, fsync, typed desync outcome), the display/dedup sweep, delta_summary extensions, `is_over()` defense-in-depth, dogfooding smalls, and the carried engine items (I4 canal mini-plan, WrongPhase disposition, box check, I5). Schedule these opportunistically — several are "do while touching that file anyway".
- Choice-encoding engine changes **shift move distributions** → rerun the full fuzz gate and expect to re-baseline the 5000×3 sweep, exactly as the C2/C3 fixes did.

## 2. The Phase-2 gate itself

When choice encoding lands: a live physical game with Jeremy at the table, tracked start to finish via `cargo run -p yokohama-cli` (live mode). A testing group also exists — they have `docs/cli-guide.md`/`.html`; their bug reports arrive as save JSONs, which replay deterministically (`cargo run -p yokohama-cli -- <file>`), so every report is a repro.

## 3. Then the AI ladder (roadmap Phase 3)

Per guidingpath's roadmap + evidence base: hand-crafted evaluator → shallow search first (Catanatron evidence), no framework wrapping, bot-vs-bot arena. The Phase-2 carry-over note in the roadmap lists the surfaces you build on (`Session`, `get_legal_moves`, `describe_move`, `GameRecord`; `GameState` stays history-free and cheap to clone). The bar (Jeremy's): *provably purposeful play*, not beats-random.

## How this project works (non-negotiables)

- **Process:** superpowers skills — brainstorming before designing, writing-plans before executing, subagent-driven-development for execution (one implementer at a time, per-task reviewer, ledger in `.superpowers/sdd/progress.md`, review-package/task-brief scripts). Keep the ledger current — it has survived three session-limit outages as the only recovery map.
- **TDD strictly**, suite green + zero warnings after every task; demand literal RED output in implementer reports (a Phase-2 task was caught skipping it).
- **Fuzz gate** (`cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored`) before any merge touching engine behavior.
- **Model tiering:** sonnet for implementation/review, fable only where capability changes outcomes (final whole-branch review, vision). Specify the model on every dispatch.
- **Rules questions:** resolve against `analysis-1/yokohama-kb/` + the rulebook PDF; orchestrator adjudicates and writes a decision-log row; genuinely contested facts go to Jeremy. Three Phase-2 briefs contradicted the real engine — the implementers escalated instead of guessing, and that standard held the branch together. Keep it.
- **Never touch** `.codex/`, `.vscode/`, `AGENTS.md`, `tts-assets/` (Jeremy's). `git add` specific paths, never `-A`. Commit style `feat:/fix:/test:/docs:`.
- **Practical gotcha:** reviewer subagents in this environment routinely go idle without delivering their report — nudge them with a SendMessage asking for the report; it has worked every time.
- Update `guidingpath.md` at phase transitions; never claim a phase complete without its executable gate passing.

Start by reading the four items at the top, then bring Jeremy a brainstormed direction for the choice-encoding design before writing any plan.
