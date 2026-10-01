# Hotfix spec — the grandfathered named identities in the CLI's `eligible` (2026-10-01)

**What this is.** The one CLI line the V5 ship owes (R-F4, cycle F, 2026-10-01; `docs/plans/2026-09-11-ai-lane-reentry-handover.md` §5 item 12), done now, at the web lane's wrap, while the code is in hand. A spec without a plan: one file, four cells, five sentences.

## 1. The problem

`crates/yokohama-cli/src/bot.rs`'s `eligible` admits a NAMED identity by equality with `Contestant::PRODUCTION` alone (line 401) and refuses every other named table as a research identity. The day production re-points to a V5 identity, `v4-step9-draft1` becomes "a named research identity … not available as a CLI bot": every unfinished V4 game resumes its bot seat as a HUMAN in the CLI, and the web refuses to open it (`bots::runs_here` goes through `BotSeats::from_provenance`, which maps `from_parts` through `eligible`). R-F4's rule is the opposite: *a build that reconstructs the exact recorded bot plays on; new games deal only the production bot.*

## 2. The change — one file

In `crates/yokohama-cli/src/bot.rs`:

1. `pub(crate) const GRANDFATHERED_NAMED_IDENTITIES: &[NamedIdentity] = &[NamedIdentity::V4Step9Draft1];` — every named identity that has SHIPPED as `Contestant::PRODUCTION`, in ship order. **A re-point APPENDS the retired identity here and never removes one** (the doc says so; cell 1 enforces it).
2. `eligible` becomes a thin wrapper over `eligible_under(production: Contestant, contestant: Contestant)`, whose named-table arm reads: admitted when `contestant == production` OR its identity is in the list; every other named table refused exactly as today. The match stays exhaustive with no `_` arm.
3. The function's doc gains one paragraph: admission here is for a game that EXISTS — the resume (`from_provenance`) and `why`'s reconstruction — and the wizard deals `PROVISIONAL_DEFAULT_BOT` alone (`wizard.rs` line 667), so a grandfathered identity is never a seat anyone can choose at a new table.

**Today's behaviour is byte-identical:** V4 is both the production identity and the list's only member, so the admitted set does not move; `from_provenance`, `why`, the wizard, the guide and the wire corpus read exactly as before.

## 3. What does NOT change

The wizard (production only, R-F4); the research refusal's sentence for `v2-padded` and `v2-refit-padded`; the drift warnings (`identity_drift_warnings`, R-F4's half (b): a V5 that moves `LOOKAHEAD_IMPL_VERSION` or `EVAL_IMPL_VERSION` still refuses V4 games on the web as DRIFT, and the CLI still warns and plays on); `SEMANTICS_REV` 7; the workspace version (nothing a save means or which bot plays changes — no bump, no tag); the web crate (no code: `runs_here` inherits the admission through `from_provenance`; `bots.rs`'s label match is untouched); `docs/cli-guide` (`guide_sync` untouched).

## 4. The cells (TDD, `bot.rs`'s tests)

1. `grandfathered_named_identities_carry_every_identity_that_shipped_as_production` — `GRANDFATHERED_NAMED_IDENTITIES` contains `Contestant::PRODUCTION`'s identity; the failure message says "append, never remove" (the re-point's reminder: a V5 ship fails this cell until it appends V4 — which is already there — and then V5 itself).
2. `a_grandfathered_identity_is_admitted_under_any_production` — `eligible_under(NamedTable(V2Padded) /* a stand-in for V5 */, NamedTable(V4Step9Draft1))` is `Ok`; `eligible_under(the same stand-in, NamedTable(V2RefitPadded))` is the research refusal; `eligible_under(the stand-in, MctsV1 …)` is still the diagnostic refusal.
3. `from_provenance_resumes_a_recorded_v4_seat_under_a_re_pointed_production` — a `RunProvenance` with one V4 seat (the descriptor `Contestant::PRODUCTION.descriptor()` as this build derives it), resolved under the stand-in production, resumes as the bot with NO warning; the same record with a `v2-refit-padded` seat resumes HUMAN with the research warning. (`from_provenance` is threaded through a private `from_provenance_under`; the public signature is unchanged.)
4. The existing pins stand unchanged and must stay green: `eligible(PROVISIONAL_DEFAULT_BOT) == Ok(…)` (line 1147, 1255), the research identities refused (line 1227), the wizard's default swept (line 1069).

Mutants the cells must catch: the list emptied (cell 1); the `||` dropped (cell 2, 3); the arm reading `contains` on the wrong side (cell 2's refusal half).

## 5. The documents — five sentences, one row

Each of these says today that a V5 build plays V4 games "exactly when the V5 ship (a) admits `v4-step9-draft1` in the CLI's `eligible` AND (b) …"; after this hotfix (a) is in place, and each reads "exactly when it still derives V4's exact descriptor, `LOOKAHEAD_IMPL_VERSION` and `EVAL_IMPL_VERSION` unmoved — a weights-only V5; the admission is in place since 2026-10-01 (`GRANDFATHERED_NAMED_IDENTITIES`), and a re-point appends the retired identity there":
- `CLAUDE.md`, the browser-lane bullet's third fact (line 86);
- `docs/web-lane.md` §7's last bullet and §15's V5 bullet;
- `docs/handover.md`, Track 1's "Owed by the V5 ship" paragraph (now: done; the re-point's one obligation is the append);
- `docs/plans/2026-09-11-ai-lane-reentry-handover.md` §5 item 12 (the pointer the AI lane re-enters from);
- one decision-log row in `guidingpath.md`.

## 6. The suites

`cargo test -p yokohama-cli` (851 + 2 ignored at `18829e9a`), `cargo test -p yokohama-web` (445; the layer compiles the CLI), `-p yokohama-wasm` (9), `-p yokohama-worker` (76), `cargo clippy --workspace --all-targets -- -D warnings`. No release gate (no core change), no deploy, no tag.

## 7. Cost if wrong

A game recorded against a research identity still refuses as today; the only admission this adds is V4's, which this build already plays as production. The risk is a sentence left behind: §5 names every site.

## 8. Execution (a fresh session, from this spec alone)

1. Read `CLAUDE.md`, the board (`docs/handover.md`), the last row of `guidingpath.md`'s decision log, and this spec. The tree is master at or after `4ad19c40`; `git status` clean; no branch is needed beyond what the session chooses.
2. TDD in `crates/yokohama-cli/src/bot.rs`: cells 1–3 of §4 red first, then §2's change, then green; cell 4's pins untouched and green.
3. §5's five sentences and the decision-log row.
4. §6's suites and the lint gate, every one green, counts stated at the head.
5. One or two commits on master, pushed. No tag, no deploy, no workflow enabled.
