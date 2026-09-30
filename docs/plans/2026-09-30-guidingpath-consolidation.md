# 2026-09-30 — `guidingpath.md` by grouping: the relocation map

**What this is.** The third relocation of the project's authoritative status file. The owner ruled it as R-6.5c2.32 in cycle D's planning session (the M6.5c2 spec `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` §10.9, revision 2.8, 2026-09-28). It was executed as Task 13 of cycle E, "the finish" (`docs/plans/2026-09-29-web-frontend-m6.5c2e-plan.md`). The owner's words, as the spec records them:
- *"would it make sense to be archiving it more in depth, so it's starting nearly fresh, or having it just contain pointers and we break out the archives by groupings so future agents can choose what to read and dig into?"* — and the ruling was both.
- *"the 3.5c arc is a key part of the last section of the computer player, but we also don't need the file to be too large … the work prior to 3.5c on the computer player has been nearly fully superceded … Things like the dec 2025 stall and the july 2026 evidence was superceded by the 3.5c v4 near completion - we pivoted to the web lane to make continued vetting and save game gathering easier and testing of future lanes easier than the CLI while still allowing automated play to be straightforward in continued development"*

`guidingpath.md` had grown to 412,353 B in 232 physical lines. Every fresh session reads it right after `CLAUDE.md`. After this pass it is 22,205 B in 84 lines.

**Scope, as ruled.**
- `guidingpath.md` is rebuilt around what it keeps, and two archives are created: `guidingpath-archive-phase3.md` and `guidingpath-archive-web.md`. The engine-era archive `guidingpath-archive.md`, split out on 2026-08-19, is untouched.
- The rest of the ruling belongs to other tasks of cycle E. `CLAUDE.md`'s read-order line names the four files; Task 12 wrote it first, and this pass uses its names verbatim. The board's history is Task 14's, and the README is Task 15's. The decision-log row that records this pass is E's close row (Task 16), so this pass adds no row.
- No code, test, CI or build file moves. The one new executable is the relocation script, and no suite runs it.

**The governing invariant.** The 2026-08-19 record's rule, "every deleted FACT exists somewhere pointed-to", made mechanical: every moved LINE exists byte-identical, exactly once, in a file the live decision log's index names.

**Method.**
- **Assembled, then checked from what was written.** `docs/tools/guidingpath-relocate.py write` built all three files from `guidingpath.md` at `19f3feb1` and a plan. The plan gives each file as an ordered list of source line ranges and new text. Kept and moved lines are pulled by line number, so they are byte-identical by construction. The script then reads every file back from disk and checks what it read; its docstring lists the six checks.
- **The new text lived in session scratch**: the plan and nine text fragments. The committed script holds the plan's shape as its built-in map `2026-09-30`, with each new text replaced by its line count. So the check re-runs from committed files alone:

  ```text
  python3 docs/tools/guidingpath-relocate.py check                 # the files in the tree
  python3 docs/tools/guidingpath-relocate.py check --at <commit>   # once the live file has grown
  ```

- **Test first.** The script's self-test has twelve cases. The first nine were written before its body and ran red, 9 of 9 failing, then green. They include the two the plan names: an empty map, dry-run against the tree, exits 0 and changes nothing; an overlapping map exits 1, naming the line placed twice. The task review added three: a non-identity plan under `--dry-run`, run in a directory of its own, writes nothing; a range past the source's end is a named refusal; a changed new-text line is refused in write mode. Seven mutants of the script were each killed by exactly their own case, the three the review found surviving among them.
- **Line numbers.** "Old" is `guidingpath.md` at `19f3feb1`; "new" is each file after this pass.
- **Nothing archived outranks the live file's current state.** The archived Current state and landed notes are the text as it stood, false sentences included. The live Current state and roadmap table replace them. The archived **rows** remain authoritative where cited, as the index says.

## §1 — What moved where

| Old lines | What it carried | Dates it covers | Now in | New lines |
|---|---|---|---|---|
| 1–17 | The title, the authority line, the mission and priority order | — | `guidingpath.md`, kept | 1–17 |
| 18–20 | A rule between blank lines | — | `guidingpath.md`, kept | 20–22 |
| 21–36 | The "Current state" section: its 2026-08-19 heading and read order, the 2026-09-19 update, the engine, the gate claim, Phase 3 at 3.5c, the 2026-08-20 route to V4, the machines | 2026-08-19 → 2026-09-19 | `guidingpath-archive-phase3.md` §3 | 121–136 |
| 37–38 | A rule | — | `guidingpath.md`, kept | 37–38 |
| 39–62 | "Why the Dec-2025 stall happened" and "Evidence base (July 2026 research)", with their rules | December 2025, July 2026 | `guidingpath-archive-phase3.md` §1 | 18–41 |
| 63–64 | The "Roadmap" heading | — | `guidingpath.md`, kept | 39–40 |
| 65–136 | The roadmap's subsections: Phase 0+1 (65–67), Phase 2 (69–99), Phase 2.9 (101–103), Phase 3 (105–134) and Phase 4 (135–136) | 2026-07-16 → 2026-08-30 | `guidingpath-archive-phase3.md` §2 | 44–115 |
| 137–141 | A rule and the "Decision log" heading | — | `guidingpath.md`, kept | 68–72 |
| 142–143 | The log table's header | — | `guidingpath.md`, kept | 82–83 |
| 144 | The 2026-08-19 pointer row to `guidingpath-archive.md` | 2026-07-16 → 2026-08-08 | `guidingpath-archive-phase3.md` §4, at its head | 143 |
| 145–204 | The log's rows, 60 of them: the 3.5c opening through V4's close | 2026-08-09 → 2026-08-30 | `guidingpath-archive-phase3.md` §4 | 144–203 |
| 205–231 | The log's rows, 27 of them. The V5 notes for the AI lane are old 214, and the personal-data exception is old 230. Old 228–229 stand out of date order (2026-09-26, then 2026-09-23) and keep that order | 2026-08-31 → 2026-09-28 | `guidingpath-archive-web.md` | 17–43 |
| 232 | The log's row for M6.5c2 cycle D's close | 2026-09-29 | `guidingpath.md`, kept | 84 |

**Four judgment calls**, each within the ruling:
1. **The old Current state went to the phase3 archive as §3**, after the origins and the landed notes. The plan says that archive "opens with the origins", so it cannot open with the Current state. The check therefore holds the live file to source order and each archive to the order its map declares. Every range stays in source order within itself.
2. **Every roadmap subsection moved, not only Phase 3's.** R-6.5c2.32 has the phase3 archive "opening with the origins — the Dec-2025 stall, the July-2026 evidence base, the Phase 0–2.9 notes". Phase 4's two lines went with the range, and the table's Phase-4 row restates them.
3. **The pointer row moved with the rows it headed.** Left in the live file it would be false: it says the log below resumes at the 3.5c opening. In the archive it heads the rows it describes. The index replaces it.
4. **No new decision-log row.** E's close row (Task 16) goes after D's and records this pass. The index's last row says a new row goes at the bottom.

## §2 — What was rewritten in the live file

Four insertions, and nothing else. Check 4 proves that no kept line changed, since checks 2 and 3 fix which lines are kept and in what order. Check 6 states the same in the brief's words and can never be the check that fails first.

| New lines | What | Its sources | The old text is at |
|---|---|---|---|
| 18–19 | ONE dated line in the mission: why the web lane exists, in the owner's words | Spec §10.9 R-6.5c2.32, quoted exactly | — (new) |
| 23–36 | A fresh one-screen Current state: the read order; the engine, with the gate claim and its standing clause; the computer player at V4, with its re-entry pointer and its two HOLDs; the web lane finished at E; the STOPs; the machines | `CLAUDE.md`'s "Where things stand" and its web pointer at `19f3feb1`; the board's Track 1 for the diagnostic games; `git diff 35b694d 19f3feb1 -- crates/yokohama-core`, which is empty | `guidingpath-archive-phase3.md` §3 (old 21–36) |
| 41–67 | The roadmap as ONE table of 23 rows, one per phase and sub-phase: status, date, gate, record of record. It runs past V4 to the bar: D3 beside the ladder with the owner items open at leisure, then Phases D, E and F | Each row from the subsection it replaces (old 65–136) or from `CLAUDE.md` at `19f3feb1`. The rows past V4 come from old 133's "What remains of 3.5c after V4", the amendment `docs/plans/2026-08-11-phase3.5c-scoping-amendment.md` §3 with D1, D6, D8 and D9, and the re-entry handover §3 and §5. Every PR number was checked against its merge commit, and every path against the tree | `guidingpath-archive-phase3.md` §2 (old 65–136) |
| 73–81 | The decision log's INDEX: date range, file, and what each of the four files holds. It says the archived rows remain authoritative where cited, and that a citation by date resolves through it | The four files; this record | The old pointer row, now at `guidingpath-archive-phase3.md` §4's head (old 144) |

Three standing rules from the old Current state are carried into the new one, because each changes what the next agent does:
- the Phase-2 gate's standing clause: no production search player is declared usable on arena cells alone (old 29);
- W6's capability condition: it waits until a version has a reasonable chance of playing to win, and is never an automatic arrow (old 33);
- wall figures are per-machine (old 35).

The road after V4 that old 133 named is carried as roadmap rows rather than prose. Phase D holds W6 and D1's identity freeze. Phase E holds W7, the OD-5 pre-registration, and OD-6, whose restatement is RIPE. Phase F holds OD-8, whose inherited "brake behind" rubric line must be re-authored, then OD-11. The D3 row names the CLI branch-4 owner register and the V5 shelf.

What the new Current state does not carry is still verbatim in `guidingpath-archive-phase3.md` §3:
- the 2026-08-20 route to V4 and its next-session entry note, since that route has been run;
- M6's per-cycle story in the 2026-09-19 update, which the web archive's rows and `docs/web-lane.md` §11 tell;
- the old read order's clause on the V3 close handover. Checkpoint §11 superseded its forward half.

## §3 — The byte-check

```text
$ python3 docs/tools/guidingpath-relocate.py --self-test
ok    st_empty_map_dry_run_changes_nothing
ok    st_overlapping_map_exits_1
ok    st_non_identity_dry_run_writes_nothing
ok    st_range_past_the_end_is_a_named_refusal
ok    st_changed_new_text_is_refused
ok    st_round_trip_counts_and_arithmetic
ok    st_gap_is_refused
ok    st_live_out_of_order_is_refused
ok    st_tampered_moved_line_is_named
ok    st_moved_line_left_in_new_text_is_refused
ok    st_extra_line_past_the_plan_is_refused
ok    st_opaque_new_lines_check_like_text
self-test: 12 passed, 0 failed

$ python3 docs/tools/guidingpath-relocate.py check
map 2026-09-30: source guidingpath.md at 19f3feb1; files in the tree
CHECK  200 lines moved, byte-identical; 32 lines kept; 52 lines inserted
  source: 232 lines
  guidingpath.md: 84 lines = 32 kept + 52 new
  guidingpath-archive-phase3.md: 203 lines = 173 moved + 30 new
  guidingpath-archive-web.md: 43 lines = 27 moved + 16 new
```

**A second, independent check: the line arithmetic.** `wc -l` before = (the live file's lines after − the lines inserted) + the lines moved:

```text
232 = (84 − 52) + 200
```

**A third: the slices.** Every one of the twelve ranges was cut with `sed -n` from `git show 19f3feb1:guidingpath.md` and from its new file at the new lines in §1, and compared with `cmp`. All twelve were identical.

**Bytes.** The estimator is bytes ÷ 2.5, as calibrated on 2026-08-19.

| File | Before | After |
|---|---|---|
| `guidingpath.md` | 412,353 B, 232 lines (~165k tok) | **22,205 B, 84 lines** (~8.9k tok), 5.4% of before |
| `guidingpath-archive.md` | 209,378 B, 233 lines | unchanged |
| `guidingpath-archive-phase3.md` | — | 309,551 B, 203 lines, read on demand |
| `guidingpath-archive-web.md` | — | 102,840 B, 43 lines, read on demand |

The bytes balance: the source's 412,353 B are 5,583 B kept plus 406,770 B moved. The new text is 16,622 B in the live file, 3,358 B in the phase3 archive's headers and 2,263 B in the web archive's.

**Links.** Every Markdown link in the four guidingpath files resolves: 16 links, none broken. None of the four uses a double-bracket link. Every backticked path in the new text resolves in the tree, this record included.

**Inbound references into the moved text.** Each still resolves, through the index or a pointer:
- Code comments cite rows by date, for example "guidingpath 2026-08-23" in `crates/yokohama-core/examples/eval_probe.rs`. They resolve through the index, and the fenced crates are not touched.
- `CLAUDE.md`'s arena-runner bullet cites "guidingpath's 3.2 landed note" for the driver's cost figures. That note is now `guidingpath-archive-phase3.md` line 98 (old 119). The read-order line names that archive, "with the origins".
- `crates/yokohama-core/examples/route_cost.rs` cites "guidingpath's 3.1b ledger row (T7)". That note is now `guidingpath-archive-phase3.md` line 95 (old 116).
- `crates/yokohama-core/tests/action_source_worstcase.rs` cites "guidingpath:172b", a line number that was already stale before this pass. The item it means, the mandatory six-figure retreat handover, is `guidingpath-archive-phase3.md` lines 90–92 (old 111–113).
- The header of `guidingpath-archive.md` says the live log resumes at the 3.5c opening in `guidingpath.md`. Those rows now open `guidingpath-archive-phase3.md` §4. That file is append-closed and untouched; the index says where the rows are.

## §4 — The reviews

Not yet reviewed at the commit that adds this record. The task's review fills this section in Task 9's shape: the reviewer, the verdict, each finding's disposition, and the re-review.
