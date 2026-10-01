# The web lane's straw poll: what the lane leaves at 0.6.0 (the researcher's report)

**What this is.** The read-only inventory an Opus researcher compiled on 2026-09-28 at cycle D's planning session, from every close since M5.5, the M6.5c2 spec's carried lists, the handovers, the board and the AI lane's re-entry pointer. Its 152 rows are the triage's rows: the owner's final disposition per row (fix in D, fix in E, the public door, by design, not observable, the AI lane's, the owner's) is recorded on the private triage page and transcribed into this file by cycle D's Task 2 as two columns (`owner's disposition`, `phase`) beside the researcher's `rec`; the dossier `docs/plans/2026-09-28-web-frontend-m6.5c2d-dossier.md` corrects the premises the tree showed false. The report below is the researcher's, verbatim, except for those two columns, the tally line and the two sections after §A; the transcription is done (cycle D's Task 2, 2026-09-28).

---

Read-only compilation, 2026-09-28, from an export of HEAD `014433df` (code `ea4f174d`). Nothing was run.
Every row cites where it was first recorded and where it was last mentioned. "tree" means read in the export.

**Legend.** "M6.x §n" and "M5.5 §n" are that cycle's close. PR37 = `2026-09-22-web-frontend-m6.5b2-pr37-fixes.md`.
M5.5corr = the 2026-09-06 owner corrections. spec = the M6.5c2 spec. c2-ho = the M6.5c2 entry handover.
A-ho = cycle A's handover into B (`…-m6.5c2b-handover.md`). B-ho = cycle B's handover into C (`…-m6.5c2c-handover.md`).
C-cl = cycle C's close. D-ho = the cycle D handover. umb = the M6.5 hosting umbrella spec. AI-ho = the AI lane
re-entry handover. board = `docs/handover.md`. wREADME = `worker/README.md`. Plan files live in `docs/plans/`.
**Cost:** S under an hour, M a few hours, L a task of its own, "–" nothing to build.

## A. The inventory (152 rows)

### Defects: behaviour wrong today
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 1 | The dealing screen's "waiting on …" never refreshes when another seat keeps (3P+ tables) | defect | M6.1 §6.2 | M6.4 §6.1; tree `web/src/Dealing.tsx:219` | S | do in D | fix | D | one handler line; carried five closes |
| 2 | The Game menu stays open after a game is chosen | defect | M5.5 §7 | M6.1 §6.1 (carried as "all stand" to M6.4) | S | do in D | fix | D | check with one click; not recorded since M6.4 |
| 3 | The sign-out transition shows the raw 401 sentence above the door | defect | M6.1 §6.2 | M6.2 §6.1 | S | do in D | fix | D | B rebuilt the door; one look settles it |
| 4 | The lobby's three lists fail together: one scope's error blanks all (one `Promise.all`) | defect | M6.2 §6.2 | tree `web/src/Lobby.tsx:67` | S | record and leave | fix | D | the next poll heals it |
| 5 | `undo_allowed` answers 403 on a record that will not replay (500 is honest); replays len−1 events per try | defect | M6.1 §6.2 | M6.2 §6.1 | S | record and leave | fix | D | layer frozen; a sound record never reaches it |
| 6 | The waiting caller's set table is built for `first_human_seat` (`deal.frame.seat` reads 0 to blue) | defect | M6.1 §6.2 | M6.2 §6.1 | S | record and leave | fix | D | no card leaks |
| 7 | `create` reads a store I/O error as "already exists" | defect | M6.1 §6.2 | M6.2 §6.1 | S | record and leave | fix | D | an operator's symptom only |
| 8 | Open-door Worker: a refused claim at `POST /api/archive` reads 500, not the NAS's 401 (R-C10) | defect | C-cl §5 | D-ho §6 item 8 | S | record and leave | fix | D | no open-door Worker is deployed |
| 9 | `expire_inner`'s no-creator arm names the wrong reason on a hand-made document (R-C14) | defect | spec §10.8 | D-ho §6 item 8 | S | do in D | fix | D | ruled: state the class in its docs |
| 10 | `api::matches` binds an empty path segment (`/api/games//table` reaches `get_table` with id "") | defect | M6.5a §6.2 | M6.5a §6.2 | S | record and leave | fix | D | no leak, no panic |
| 11 | NAS: a landed game-over put whose stamp failed is archived again under `abandoned/` (IMP-1) | defect | M6.5a §6.2 | M6.5a §6.2 | M | record and leave | fix | D | a duplicate, never a loss |
| 12 | "Back up everything" hangs (no `onBlocked`) while an M6.5b-era tab holds IndexedDB v1 | defect | M6.5b2 §6.2 | M6.5b2 §6.2 | M | record and leave | fix | D | needs a v1 tab, long gone |
| 13 | `backupEverything` walks the store's ids: a quota-refused create shows on the door, not in the backup | defect | PR37 §5.7 | M6.5c §6.1 ("PR #37 residuals") | S | record and leave | fix | D | the per-game Back up covers it |
| 14 | Storage reads outside the export arm still trap (hold acquisition, startup, `adopt`) | defect | PR37 §5.4 (R-F1′) | PR37 §5.7 | M | record and leave | fix | D | a restart, not a lost move |
| 15 | A newer backup envelope version meets serde's unknown-field sentence, not "version N" (M-7) | defect | M6.5b2 §6.2 | M6.5b2 §6.2 | S | record and leave | fix | D | matters only once the envelope bumps |
| 16 | A restore-compatibility row outlives its reason after a `SEMANTICS_REV` bump (`db.ts` `restoreOne`) | defect | M6.5b2 §6.2 | M6.5b2 §6.2 | S | record and leave | fix | D | untestable until a rev 8 exists |
| 17 | M6.2 menu minors: no Abandon for a seatless creator; a failed open evicts; Back mid-open; the table read twice a visit | defect | M6.2 §6.2 | M6.3 §6.1 | M | record and leave | fix | D | screens rebuilt in B; state not recorded |

### Test debt
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 18 | `plays_locally`'s quota cell: the clicked option can open a sub-picker, so nothing applies | test-debt | M6.5c §2 (red once) | D-ho §6 item 1 | S | do in D | fix | D | assert the click applied a move |
| 19 | Two jsdom cells failed once in five runs under load 5.9 | test-debt | C-cl §6.2 | D-ho §6 item 7 | S | do in D | fix | D | name them before the freeze |
| 20 | The browser host's replay lacks the L-5 presence rule (`corpus.node.test.ts:231-235`; R-C20) | test-debt | spec §10.8 | D-ho §6 item 8 | S | record and leave | fix | D | a third statement is dropped-refactor ground |
| 21 | Four of five error strips never provoked in a headed browser (create form, deal, chat, open table) | test-debt | M6.5b2 §6.3 item 1 | M6.5c §7 | M | do in D | fix | E | fold into the 390 px polish look |
| 22 | No CSS-aware instrument sees the refusal strips or the door's layout; C's stylesheet rules have no mutation reading | test-debt | M6.5b2 §7 | C-cl §4 | L | record and leave | by design |  | pictures stand in for a gate |
| 23 | No jsdom cell holds 44 px on the open table and the device card | test-debt | C-cl §6.2 | C-cl §6.2 | S | record and leave | fix | D | the one-off log witnesses it |
| 24 | M6.5b's parked reload clause (the cold-pump arm) may be reachable now that B ships "First player: The bot" | test-debt | M6.5b §6.3 item 5 | M6.5b §7 | S | do in D | fix | D | one read of `web/src/local/`; not recorded since |
| 25 | Six `sw.ts` decisions, `Abandon.tsx:212`'s catch and the Worker's trap/refusal split lack a unit catcher | test-debt | PR37 §5.7 | M6.5c §6.1 | M | record and leave | fix | D | the e2e pins the split |
| 26 | E2 (an uncontrolled tab reloading on activation) has no browser reading | test-debt | PR37 §5.7 | M6.5c §6.1 | M | record and leave | fix | D | unit cells pin it |
| 27 | The blocked-upgrade cell runs on fake-indexeddb, not two real tabs | test-debt | M6.5b2 §7 | M6.5b2 §7 | M | record and leave | not observable |  | no real v1 tab remains |
| 28 | The determinism pin plays the DEBUG module; the release module is not pinned | test-debt | M6.5b §7 | M6.5b §7 | S | record and leave | fix | D | one feature apart |
| 29 | The three `events*` fixtures cannot be replayed by the browser host | test-debt | M6.5b §7 | M6.5b §7 | S | record and leave | by design |  | Task 8's cells cover local signalling |
| 30 | `cancelPicker`, the walk's Cancel branch, had run in no e2e walk | test-debt | A-ho §5 | A-ho §5 | S | record and leave | fix | D | not recorded since |
| 31 | The acting-seat fence has no whole-game witness; the `persist` cells miss the CAS | test-debt | M6.1 §7 | `CLAUDE.md:36` | S | record and leave | fix | D | stated where it matters |
| 32 | No committed fixture deals at a bot table or shows an open table with two open seats | test-debt | M6.1 §6.2 | M6.2 §7 | S | record and leave | fix | D | shape and value cells stand in |
| 33 | The livelock bound (`VS_BOT_CAP` 5,000) rests on one run of 50 seeds | test-debt | A-ho §5 | C-cl §7 | M | record and leave | fix | D | CI bounds the job at 20 min |
| 34 | Four identity states, the single-player controls, the foreign screen and a foreign Remove are photographed, never walked | test-debt | B-ho §5 | B-ho §5 | M | record and leave | fix | D | no local server reaches them |
| 35 | `hasMove` is copied three times across the e2e specs | test-debt | M6.1 §6.2 | M6.3 §6.2 | S | record and leave | fix | D | dropped-refactor kin |
| 36 | R-N7: a computed key held in a variable passes the live-request structural cell | test-debt | M6.5c §6.2 | spec §2.8 | S | record and leave | by design |  | ruled contrived |
| 37 | Battery residues: M6.5c row 4, r17b, M33–M36, the Archive clock pin; M6.5b2 rows 1 and 16 | test-debt | M6.5b2 §7 | M6.5c §7 | – | record and leave | by design |  | each recorded with its reason |
| 38 | M6.2 test minors (scope matchers, name-keyed `freeze_clock`, password needles without partners, OpenTable strings) | test-debt | M6.2 §6.2 | M6.3 §6.1 | M | record and leave | fix | D | screens rebuilt since; some moot |
| 39 | M6.3–M6.4 test minors (`/stream` in no route sweep, R-pre-3 unasserted, no held "wake" cell, 34 rem tied to `--side`) | test-debt | M6.3 §6.2 | M6.4 §6.2 | M | record and leave | fix | D | each LEAVE'd by its review |
| 40 | M6.5a test minors (the NoSignal premise, the clock cell, the oversize-body sentence, the region-marker blind spot) | test-debt | M6.5a §6.2 | M6.5a §6.2 | S | record and leave | fix | D | none a behaviour |
| 41 | M6.5b2 test minors (no client-drawn restore cell, `channelOf`, the precache regex, `newPage`, `CAP` and `REV` restated) | test-debt | M6.5b2 §6.2 | M6.5b2 §6.2 | S | record and leave | fix | D | a rev bump must find `REV` |
| 42 | Cycle B's harness notes (held arm's `connected` unpinned; lobby sweep red on a leftover 4P table; `run` premise rides the seed) | test-debt | B-ho §7 | B-ho §7 | S | record and leave | fix | D | notes, not defects |

### Documentation debt
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 43 | `docs/web-lane.md` from D8's draft, and `CLAUDE.md`'s web section reduced to a pointer | doc-debt | spec §2.11 | D-ho §2 | L | do in D | fix | E | spec §4 row D |
| 44 | The lane document's owed contents: the 27 traps, C's two instruments, the testing strategy, "What a save file says", the version rule, the FULL public-door list | doc-debt | c2-ho §6 | D-ho §2 | M | do in D | fix | E | with item 43; see §D item 2 |
| 45 | The seven screenshots recaptured off the deployed site, and wREADME §1.4's captions | doc-debt | spec §2.11 | D-ho §2 | M | do in D | fix | E | the pictures show the old door and "watching" |
| 46 | The top-level refresh: README's two-README balance, guidingpath's current state, the board's history as pointers | doc-debt | spec §2.11 | D-ho §2 | M | do in D | fix | E | §C and §D list the false lines |
| 47 | Twelve `RUNBOOK §6.x` citations left in code and configuration | doc-debt | C-cl §6.2 | D-ho §6 item 5 | S | do in D | fix | D | one sweep, one commit |
| 48 | Module figures into the root `Cargo.toml` (Worker 1,258,106 B; browser 1,197,069 B) | doc-debt | C-cl §3.1 | D-ho §6 item 4 | S | do in D | fix | D | tree `Cargo.toml:33,50` read older figures |
| 49 | Cycle C's deferred comment and doc minors (about 25 named; four marked drop) | doc-debt | C-cl §6.2 | D-ho §6 item 7 | S | do in D | fix | D | keep the kept, drop the four |
| 50 | 34 rustdoc warnings in `yokohama-web` | doc-debt | C-cl §6.2 | D-ho §6 item 7 | M | do in D | fix | D | the Worker crate already reads 0 |
| 51 | `CLAUDE.md` lines false at HEAD: :118 first visit "reconnecting…", :36 the gate "LIVELOCKS", :44 cap hits "through deadEnd", :121 "activated only on a click" | doc-debt | not recorded (found here) | `CLAUDE.md` | S | do in D | fix | D | fix them in the lane document's copy |
| 52 | Board lines stale at 0.6.0 (106 first half, 110, 204, 497–511, 535, 544) | doc-debt | not recorded (found here) | board | S | do in D | fix | E | the board rewrite covers them |
| 53 | `deploy/RUNBOOK.md` §3.6 says "The live build is …" (lines 532, 564) | doc-debt | M6.1 §6.2 | M6.2 §6.1 | S | record and leave | fix | E | the section is headed "a record, not a plan" |
| 54 | M5.5's 44 archived minors and the corrections wave's nine | doc-debt | M5.5corr §5.7 | M6.1 §6.1 | M | record and leave | by design |  | comments only; in a gitignored tarball |
| 55 | M6.1–M6.4 comment and naming minors (`KeepRequest` unread, dead CSS, `usePolled` named for a poll, doc lines) | doc-debt | M6.1 §6.2 | M6.4 §6.2 | M | record and leave | fix | D | none a behaviour |
| 56 | M6.5b's deferred minors, Tasks 2–8 (about 40 lines) | doc-debt | M6.5b §6.2 | M6.5b §6.2 | M | record and leave | fix | D | triaged LEAVE by its review |
| 57 | The server binary's unexplained 50,904 B shrink at M6.5b2 | doc-debt | M6.5b2 §6.3 item 9 | M6.5b2 §6.3 item 9 | S | record and leave | by design |  | checked to be from the tree |

### Owner-eye polish
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 58 | Polish 5: `a.primary-link`'s box against `button.primary`'s (~1.6 px) | owner-eye polish | B-ho §7 | spec §2.2 | S | do in D | fix | E | assigned to D |
| 59 | Polish 6: state E's `.two-up` at 976 px against mock 06's ~868 | owner-eye polish | B-ho §7 | spec §2.2 | S | do in D | fix | E | assigned to D |
| 60 | Polish 7: the band's crumbs wrapping under the pill at 390 | owner-eye polish | B-ho §7 | C-cl §6.3 item 4 | S | do in D | fix | E | assigned to D |
| 61 | Polish 8: the Players panel's tag order (gold `V4` beside gold `¥4`) | owner-eye polish | B-ho §7 | spec §2.2 | S | do in D | fix | E | assigned to D |
| 62 | Polish 9: the device card's storage line and backup note; the board's bottom three hues | owner-eye polish | B-ho §6 (R-B12) | spec §2.2 | S | do in D | fix | E | assigned to D |
| 63 | The closed Finished fold's cue, quiet at 1× | owner-eye polish | C-cl §4 (B.3) | D-ho §6 item 6 | S | do in D | fix | E | wants the owner's eye |
| 64 | The lobby's density at 390: creator rows ~146 px under the 44 px rule, beside a small Abandon | owner-eye polish | B-ho §5 | C-cl §6.3 item 4 | S | do in D | fix | E | a hit box is the named option |
| 65 | A "connecting" word on a first visit | owner-eye polish | M6.5c §6.3 item 11 | D-ho §6 item 6 | S | do in D | by design |  | already in the tree: one look, then strike (§D item 1) |
| 66 | `1lh` centring on old Safari | owner-eye polish | C-cl §6.2 | C-cl §6.2 | S | record and leave | fix | E | the phones will show it |
| 67 | The open table's chat sits in the main column, unlike its mock | owner-eye polish | B-ho §5 | B-ho §5 | S | the owner's | fix | E | never carried into C's lists |
| 68 | Some screen readers may announce "Copied" twice | owner-eye polish | B-ho §7 | B-ho §7 | S | the owner's | fix | E | one manual check, never recorded done |
| 69 | `why`'s refusal shows its machine tag; local rows name `local@browser`; the take-over screen names no game | owner-eye polish | M6.5b §6.3 item 10 | M6.5b §6.3 item 10 | S | the owner's | the owner's |  | the tag is by design (`Explain.tsx:296`) |
| 70 | Pre-M6 layout items: the 390 px tile popover; R-F13's lit row above 1201; R-C11's clipped label; the 1200/1201 card; the mobile stack | owner-eye polish | M5.5 §7 | M6.1 §6.1 | M | the owner's | the owner's |  | keep or drop as a set |
| 71 | The 1024 px banner in two rows; the "reconnecting…" strip wrapping it at 1280 | owner-eye polish | M6.1 §6.2 | M6.2 §6.1 | S | the owner's | fix | E | not recorded since M6.2 |
| 72 | M6.4's who-strip band (481–560 px) and CC-2's truncated placeholders at ≤500 px | owner-eye polish | M6.4 §6.2 | M6.4 §6.2 | S | do in D | fix | E | B rebuilt both: check, then strike |
| 73 | M6.3's M-2: an arrival by link or reload draws the lobby for one round trip | owner-eye polish | M6.3 §6.2 | M6.4 §6.1 | S | record and leave | fix | E | costs one stream open and three listings |
| 74 | The Finished group's time rule ("after some time") | owner-eye polish | spec §2.2 (R-6.5c2.22) | spec §10.7 | M | the owner's | the owner's |  | needs a finished-at time; "D's if ever" |
| 75 | R-F7 (your own closing event walks as step 1) and R-F8 (the opening run unreachable from the wizard) | owner-eye polish | M5.5 §7 | M6.1 §6.1 | S | record and leave | by design |  | M5.5-era walk semantics |

### Capabilities not built (most are the public door's)
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 76 | The path-scoped Access application: built, proven locally, never deployed; zero-login single player unshown | capability not built | M6.5b2 §7 | D-ho §2 | M | the owner's | public door | E | public-door precondition |
| 77 | The dev door off and `DEV_DOOR_CLIENT_ID` deleted; the guard prints the door, not the dev door's state | capability not built | M6.5c §6.3 item 14 | C-cl §6.3 item 3 | S | the owner's | public door | E | public-door precondition |
| 78 | The card art's publication (ASSETS.md grants nothing; a public repo is fresh-history) | capability not built | M6.5b2 §8.1 item 2 | C-cl §6.3 item 3 | L | the owner's | the owner's |  | Track 2's decision |
| 79 | The project link set (`YOKOHAMA_PROJECT_URL`) | capability not built | spec §2.10 | spec §2.10 | S | the owner's | public door | E | off C's and D's lists (§D item 2) |
| 80 | Requests per move (~13.8): the client refetches on every frame; the lobby reads three lists per house bump | capability not built | M6.4 §6.3 item 5 | C-cl §3.5 | L | the owner's | fix | E | the free plan's first limit; a spent day answers 429 |
| 81 | The untrusted `GameRecord` on `POST /api/archive` reaches the engine's setup path (bounded, not closed) | capability not built | M6.5b §6.3 item 6 | spec §7 | M | the owner's | public door | E | off C's and D's lists (§D item 2) |
| 82 | R-6.5.3's Access-door LEAVEs: every seat's email to any signed-in caller; `join`/`unlock` oracles without a rate limit; no chat moderation or delete | capability not built | M6.3 §6.3 item 1 | umb §1 (R-6.5.3), §6 | M | the owner's | fix | D | on no public-door list (§D item 2) |
| 83 | A seat holder downloads a finished record, seed included, on the hosted path | capability not built | M6.5a §6.3 item 5 | M6.5a §6.3 item 5 | S | the owner's | the owner's |  | "a decision M6.5c inherits"; not carried since |
| 84 | Open tables and Being played show one page with no "more"; the pager pages Your tables only | capability not built | M6.2 §6.2 | tree `web/src/Lobby.tsx:67,250` | S | the owner's | fix | E | the 21st open table is invisible |
| 85 | Push notifications (R-6.5c.6 deferred) | capability not built | wREADME §11 | wREADME §11 | L | record and leave | by design |  | quotas do not bind at the owner's usage |
| 86 | The held-request arm ships DARK with a residual gap; `EventsQuery.after` is inert | capability not built | M6.3 §7 | M6.4 §7 | M | record and leave | fix | D | SSE and sockets carry both hosts |
| 87 | Chat keeps no read state past a session, a name per browser, and no page size | capability not built | M6.4 §7 | M6.4 §7 | M | record and leave | by design |  | by the M6.4 rulings |
| 88 | Deferring the install under Save-Data | capability not built | M6.5b2 §7 | M6.5b2 §7 | M | record and leave | by design |  | the release installs all or nothing |
| 89 | Ruled out, not items: choosing the bot, resuming across a bot change, house password or accounts, NAS TTL and operator, an open-door Worker | capability not built | spec §6 | spec §6 | – | record and leave | by design |  | owner rulings R-6.5c2.2, .3, .5, .10 |

### Recorded limits
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 90 | "Abandon and save" is the near truth: a just-dealt or already-archived table saves nothing | recorded limit | M6.5a §6.3 item 4 | board Track 3 (M6.5a block); tree `Abandon.tsx:241` | M | record and leave | fix | E | a fix moves eight fixtures |
| 91 | A finished local game uploads itself; the first archive found uploads every finished local game | recorded limit | M6.5b2 §6.3 item 7 | M6.5b2 §7 | S | the owner's | the owner's |  | spec-conformant, and a player notices |
| 92 | The update offer renders on the door only; a waiting release activates once every old page closes | recorded limit | M6.5b2 §6.4 | M6.5b2 §7 | S | record and leave | by design |  | two documents overstate it (§C) |
| 93 | Worker: a refused stamp after a landed upload retries hourly until Abandon or expiry | recorded limit | C-cl §7 | D-ho §4 | S | record and leave | by design |  | by design |
| 94 | Overlapping uploads' pair consistency best-effort; production R2's create-if-absent refusal never watched | recorded limit | M6.5c §7 | C-cl §7 | M | record and leave | by design |  | pool-proven only |
| 95 | An R2 outage costs every request one attempt and one failure batch while a save is owed | recorded limit | M6.5c §7 | M6.5c §7 | – | record and leave | fix | D | the ladder climbs per request |
| 96 | An expired unfinished game is lost while the archive is off, with any owed save | recorded limit | M6.5c §7 | wREADME §11 | S | record and leave | by design |  | the marker and a log line say so |
| 97 | A poisoned or `held` table needs the operator; nothing sweeps it; the listing lags | recorded limit | M6.5c §7 | wREADME §11 | M | record and leave | by design |  | the log names it |
| 98 | The house chat document is dropped at its second consecutive load trap | recorded limit | M6.5c §7 | M6.5c §7 | S | record and leave | by design |  | lobby chat is not training data |
| 99 | Local host: any local 5xx shows the broken-save screen (its Abandon deletes unsent); refused creates and deletes are not "unsaved" | recorded limit | M6.5b §7 | M6.5b §7 | M | record and leave | by design |  | M6.5b's own triage |
| 100 | A refusing archive leaves a local foreign game undeletable; an unreadable IndexedDB hides "Will not open here" rows | recorded limit | B-ho §7 | B-ho §7 | S | record and leave | fix | D | every local game shares the exposure |
| 101 | A spectator may read and post chat at an unfinished foreign game (R-A7) | recorded limit | A-ho §5 | A-ho §6 | S | record and leave | by design |  | ruled as designed |
| 102 | Two activation-stamp residuals (a pre-R-F5 waiting release; an unwritable stamp) | recorded limit | PR37 §5.5 | PR37 §5.7 | S | record and leave | not observable |  | the first transition has passed |
| 103 | The no-Web-Locks fallback is invisible; background suspension unmeasured | recorded limit | M6.5b §6.3 item 7 | spec §7 | M | record and leave | by design |  | every target browser has Web Locks |
| 104 | Hot-seat secrecy rests on signing out; seated players see each other's emails; no rate limit on `/deal` | recorded limit | M6.1 §7 | M6.1 §7 | S | record and leave | by design |  | by design |
| 105 | The door instantiates the module on every visit to list this device's games | recorded limit | M6.5b §6.3 item 3 | M6.5b §7 | M | record and leave | by design |  | cached after one visit; not recorded since |
| 106 | The phones: the iPhone exit and the Android handset against both hosts; WebKit and its eviction unmeasured | recorded limit | M6.5b2 §6.3 item 3 | D-ho §2 | M | do in D | fix | E | the simulator in D; the handsets stay the owner's |
| 107 | The NAS's `cf-access` read once, as the owner: no game played; `run_id` and `bot` on no NAS document; update path and `index.html` rewrite unread | recorded limit | C-cl §7 | D-ho §4 | S | do in D | fix | D | one short game over WARP |
| 108 | The clean-browser single-audience reading and R-F6's Access re-login | recorded limit | M6.5b2 §6.3 item 2; PR37 §5.7 | spec §7 | S | do in D | fix | E | C dropped both silently; the simulator is a clean browser |
| 109 | One unexplained watcher socket close in the first live `two_browsers` run | recorded limit | C-cl §3.4 | D-ho §6 item 2 | M | the owner's | fix | D | chase only on a live run, tail read first |
| 110 | `picker-dropped` never observed live | recorded limit | M6.5c §6.3 item 9 | C-cl §7 | M | record and leave | fix | D | needs a deploy that changes engine, CLI or layer |
| 111 | No not-over foreign-bot game on either host; the versioned refusal and its 409 cell-read only | recorded limit | C-cl §7 | D-ho §4 | M | record and leave | not observable |  | needs a production re-point |
| 112 | The refinish rule proven in the pool only; no live conflict | recorded limit | C-cl §7 | C-cl §7 | S | record and leave | not observable |  | two pool race cells |
| 113 | The `hasTables` memo's premise rests on a throwaway pool cell | recorded limit | C-cl §7 | C-cl §6.2 | S | record and leave | by design |  | a wrong premise costs a scan, not an answer |
| 114 | Rows read ~56 a move on ONE harness game; a plain two-person game never measured | recorded limit | M6.5c §7 | C-cl §7 | M | record and leave | fix | E | the harness refetches on three pages |
| 115 | Pool fact 8: `ctx.id.name` in a production `alarm()` unread; no alarm has fired live | recorded limit | M6.5c §6.3 item 7 | spec §7 | S | record and leave | not observable |  | read when one fires |
| 116 | Frame-after-write and alarm atomicity rest on Cloudflare's documentation | recorded limit | M6.5c §7 | wREADME §11 | – | record and leave | by design |  | no cell can observe them |
| 117 | Never timed live: a large untrusted record's strict replay; a wake on a record of thousands of events | recorded limit | M6.5c §7 | wREADME §11 | M | record and leave | fix | D | the exit's wakes read 9–12 ms |
| 118 | No table has expired live; `held`, an operator's expire and a force's 503 read in the pool only | recorded limit | M6.5c §7 | wREADME §11 | M | record and leave | not observable |  | nothing has aged 90 days |
| 119 | The raw-address rule and the operator gate's refusal of an unlisted person never read live | recorded limit | M6.5c §7 | M6.5c §7 | S | record and leave | public door | E | no such identity to try |
| 120 | Chromium's "closed before the connection is established" believed harmless; a red live walk may be the edge | recorded limit | M6.5c §7 | M6.5c §7 | S | record and leave | by design |  | R3's E-4 and E-5 |
| 121 | The per-process chat count and in-process signals break a multi-process host | recorded limit | M6.4 §6.3 item 6 | M6.4 §7 | – | record and leave | by design |  | neither host runs two processes |
| 122 | The owner's address stays in the git history | recorded limit | C-cl §7 | D-ho §4 | L | the owner's | the owner's |  | Track 2's fresh-history question |
| 123 | Docker's base tag floats; AppleDouble sidecars in workstation-cut images; a CI cache hit skips the `wasm-opt` checksum | recorded limit | M6.5b §6.1 | M6.5b2 §6.2 | S | record and leave | by design |  | docker.yml's cold set names the tag |
| 124 | `walk.json` is 419 KB of pretty-printed JSON rewritten on every regeneration | recorded limit | M6.5b §6.3 item 4 | spec §7 | S | record and leave | by design |  | ruled a fixture's format |
| 125 | The Origin check on WebSocket upgrades | recorded limit | M6.5c §6.3 item 12 | spec §7 | S | the owner's | the owner's |  | a ruling the owner can reverse |
| 126 | Rollbacks: across M6.5a a schema rollback, across M6.5c's wave a generation-order one; a code deploy drops open pickers | recorded limit | M6.5c §6.3 items 9–10 | wREADME §9.6 | – | record and leave | by design |  | goes into the lane document |
| 127 | The dropped refactors: the `table.ts` split, read-once threading, the replay split, four script dedups | recorded limit | spec §1 (R-6.5c2.1) | spec §10.7 (R-6.5c2.26) | L | record and leave | fix | D | owner ruling: dropped, in D as well |
| 128 | NAS server minors: a false tombstone while an unlink fails; `locks` never pruned; a poisoned lock is one dead game | recorded limit | M6.2 §6.2 | M6.3 §6.1 | M | record and leave | fix | D | `check_shape` bounds prevent the poison |
| 129 | NAS transport minors: `events_at` reads the table twice; blocking I/O in `poll_next`; one spurious hot-seat 403 | recorded limit | M6.3 §6.2 | M6.4 §6.1 | M | record and leave | fix | D | self-healing, NAS only |
| 130 | M6.4's review LEAVEs (twenty lines; tick hosts pay a delta GET per own post) | recorded limit | M6.4 §6.2 | M6.4 §6.2 | M | record and leave | fix | D | each LEAVE'd by its review |
| 131 | M6.5b2 notes: the e2e no longer clicks "Send to the archive"; the send button dims per page load; a `.txt` partial deploy fails install | recorded limit | M6.5b2 §6.2 | M6.5b2 §7 | S | record and leave | by design |  | the image ships both halves |

### Operational
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 132 | The merge of #31 to #42 with merge commits, each retargeted by hand, then `v0.6.0` and `release.yml` read to green | operational | spec §1 (R-6.5c2.17) | D-ho §2 | M | do in D | public door | E | on the owner's word, one step at a time |
| 133 | CI refocus: path-filter `test.yml`'s three web jobs; `push` to master only; keep dispatch | operational | C-cl §6.3 item 7 | D-ho §6 item 11 | S | do in D | fix | D | the owner's direction of 2026-09-28 |
| 134 | The core `test` job installs `@stable` (`test.yml:49`) while the toolchain file pins 1.97.1 | operational | M6.5a §6.3 item 3 | M6.5a §6.3 item 3 | S | record and leave | fix | D | harmless: the toolchain file wins |
| 135 | The NAS swap as one script (25.3 s of downtime against 8.45 s); the Portainer token's places | operational | C-cl §6.2 | D-ho §6 item 9 | M | record and leave | fix | D | no NAS deploy is planned in D |
| 136 | The `test/` rule's first expiry | operational | C-cl §7 | D-ho §4 | S | record and leave | not observable |  | read after 2026-10-28 |
| 137 | `wrangler tail` drops and samples under a burst; the Observability query API instead | operational | C-cl §3.8 | D-ho §6 item 3 | S | the owner's | fix | D | C-cl §6.3 item 6 asks the owner |
| 138 | Live runs leave PERMANENT lobby chat posts (one per replay, two per exit) | operational | M6.5c §7 | wREADME §10.4 | S | the owner's | fix | D | real people read them; no delete exists |

### Cross-lane
| # | item | class | first recorded | latest mention | cost | rec | owner's disposition | phase | why |
|---|---|---|---|---|---|---|---|---|---|
| 139 | `record.rs:181`'s "never-bumped" sentence | cross-lane | C-cl §6.2 | D-ho §6 item 7 | S | the AI lane's | the AI lane's |  | fenced: never a web commit |
| 140 | The version rule: bump at every release that changes a save's meaning or the bot (V5 included), and tag it | cross-lane | spec §1 (R-6.5c2.19) | spec §2.11 | S | the AI lane's | by design |  | the lane document records it |
| 141 | A re-point regenerates `production_bot.json` through `wire_fixtures`; after item 133 a core-only push runs no web job | cross-lane | D-ho §6 item 11 | D-ho §6 item 11 | S | the AI lane's | the AI lane's |  | run `cargo test -p yokohama-web` per edit |
| 142 | The ingestion script that sorts saves by bot and version (R-6.5c2.21) | cross-lane | B-ho §7 | spec §10.7 | M | the AI lane's | the AI lane's |  | "What a save file says" is its contract |
| 143 | A finished same-name re-point shows the CLI's drift warning (R-C18; a mask in `client.rs`) | cross-lane | spec §10.8 | D-ho §6 item 8 | S | record and leave | fix | D | only a same-name re-point reaches it |
| 144 | A `bga-human` seat reads as a foreign bot | cross-lane | A-ho §7 | B-ho §3 | S | the AI lane's | fix | D | B marked it so |
| 145 | The V5 notes: a sighted Keep; per-good requirement weights | cross-lane | AI-ho §5 item 11 | board, owner rulings item 9 | M | the AI lane's | the AI lane's |  | bot changes owing pre-registration |
| 146 | The draw-order note (the engine deals before filling the port) | cross-lane | AI-ho §5 item 11 | AI-ho §5 item 11 | M | the AI lane's | the AI lane's |  | a `SEMANTICS_REV` bump for no fidelity |
| 147 | Does the CLI wizard show a bot seat's two dealt cards? | cross-lane | AI-ho §5 item 11 | AI-ho §5 item 11 | S | the AI lane's | the AI lane's |  | unverified; the CLI is fenced |
| 148 | The placement reservoir's `usize` draw differs by width (206 native, 281 wasm) | cross-lane | board Track 3 (M6.5 block) | board Track 3 | M | the AI lane's | the AI lane's |  | owed only if 3–4P bots come in scope (O-11) |
| 149 | The bot cannot end a 2P game against a seat that places nothing | cross-lane | M5.5 §7 | M6.3 §6.3 item 2 | M | the AI lane's | the AI lane's |  | "escalated for both lanes"; no AI-side record |
| 150 | The chat's `bot` author arm is produced by no route (the talking bot) | cross-lane | M6.4 §7 | M6.4 §7 | L | the AI lane's | the AI lane's |  | a shape, not a feature |
| 151 | `cargo clippy --target wasm32-unknown-unknown` fails inside the CLI (`session.rs:439`) | cross-lane | M6.5c §6.2 | spec §7 | S | record and leave | the AI lane's |  | a fenced crate; no gate runs it |
| 152 | No bot choice or difficulty on the web (shown, never chosen) | cross-lane | spec §1 (R-6.5c2.10) | board, owner rulings item 7 | L | the AI lane's | the AI lane's |  | the sim-seat option is its home |

**Tally:** fix 88 (D 62, E 26) · public door 6 · by design 32 · not observable 7 · the AI lane's 11 · the owner's 8.

**Checked closed, so no row:** the gate's livelock (A; tree `play_a_whole_game.rs:277`); the cap's own message (tree
`web/e2e/harness.ts:736`); who starts and the colours (B); the single-player "Lobby" controls (B, R-B18); Task 8's six client
items (B; tree `test/fakeSocket.ts`, `signal.test.ts:907`); L-3, L-5 pool, L-6, L-7, L-9 (B, C); the module's invariants, the
memo and one time unit (C); `DEV_DOOR_CLIENT_ID` a secret, `wasm-opt` 133, `docker.yml`, the NAS on `cf-access`, the scrub,
the `test/` rule, the refinish rule (C); a later copy clearing its refusal (C; tree `Lobby.tsx:282`); the house copy's
"no seed" comments (C; tree `house.ts:196`); the delete leaf chosen by status (M6.5b, R-6.5b.13); M6.3's M-1 and M-3 (M6.4);
R3's docs item 24 (tree `test.yml:17-37`); the operator sentence naming the self-host guide (tree `whoami.ts:131`).

**The premises the dossier corrected** (from `docs/plans/2026-09-28-web-frontend-m6.5c2d-dossier.md`, `## Summary`). The researcher's rows above stand as written; where the tree showed a premise false, this is the correction.

Whole rows false at HEAD:

- **Row 11:** false as written; since R-6.5b.13 an over-mirrored abandon lands under `over/` as `Identical`, so a duplicate needs a stale mirror as well.
- **Row 24:** false as written; a bot-first local game writes its first record after the bot's opening run, so the cold-pump arm is not reached.
- **Row 35:** false as written; `hasMove` has one definition and three importers.

Partly false, moot or drifted:

- **Row 3:** unverified; the M6.1 trigger is guarded now.
- **Row 4:** the first read only.
- **Rows 17, 38, 55 and 56:** sub-items moot, marked in the dossier.
- **Row 23:** it names jsdom, which loads no CSS; only Playwright can hold it.
- **Row 25:** `Abandon.tsx:212` drifted.
- **Row 32:** narrowed; `table_bot.json` shows a bot table at `dealing`.
- **Row 39:** a held "wake" cell exists.
- **Row 40:** two sub-items closed.
- **Row 41:** an e2e restores a Worker-drawn game.
- **Row 47:** 11 in code and configuration, and `CLAUDE.md:118`'s citation is correct.
- **Row 49:** three cited lines drifted.
- **Row 100:** the door's Remove deletes without sending.

**Notes from the triage** (every non-empty note on the private page, by row, verbatim). These are the page's pre-filled calls and notes, the orchestrator's of 2026-09-28 made before any owner change (spec §10.9); the owner changed none of them as of that date, and the page's `triage` store was re-listed empty at cycle D's start.

- **Row 2:** verify with one click first
- **Row 4:** one fetch per scope
- **Row 5:** 500 on a record that will not replay; replay once
- **Row 8:** the Worker's archive route answers 401 as the NAS does
- **Row 9:** state the class in the docs
- **Row 11:** archive once; a failed stamp never re-files
- **Row 12:** add onBlocked
- **Row 15:** name the version
- **Row 17:** verify each on B's screens; fix what remains
- **Row 19:** name them, then harden
- **Row 20:** the third L-5 statement, with the refactors reopened
- **Row 21:** provoke the four at 390 in the polish look
- **Row 22:** the browser walk is the CSS instrument; stated once
- **Row 24:** read local/; claim or fix the clause
- **Row 26:** one e2e cell
- **Row 27:** needs a v1 tab that no longer exists
- **Row 28:** pin the release module too
- **Row 29:** local signalling is covered by its own cells
- **Row 31:** a whole-game witness and the CAS cell
- **Row 32:** two fixtures
- **Row 33:** a 500-seed run in the background; record the tail
- **Row 34:** walk what the open door reaches; state the rest
- **Row 35:** one helper
- **Row 36:** ruled contrived
- **Row 37:** each recorded with its reason
- **Row 38:** verify on B's screens
- **Row 39:** the layer's three; the CSS one is E's
- **Row 42:** notes become cells
- **Row 43:** the lane document
- **Row 45:** off E's deployment
- **Row 47:** one commit
- **Row 49:** keep the kept, drop the four
- **Row 51:** four sentences, now
- **Row 52:** the board rewrite
- **Row 54:** comments in a gitignored tarball; not recoverable work
- **Row 57:** checked to be from the tree
- **Row 63:** your eye on the sheet
- **Row 64:** a hit box
- **Row 65:** already shipped in C (signal.ts:98); struck
- **Row 66:** a line-height fallback
- **Row 67:** as the mock
- **Row 68:** one live region
- **Row 69:** the tag is by design; your call
- **Row 70:** keep or drop as a set, off the sheet
- **Row 72:** check, then strike or fix
- **Row 74:** needs a finished-at in the mirror (a layer field); your call
- **Row 75:** M5.5 walk semantics
- **Row 78:** Track 2
- **Row 79:** set it
- **Row 80:** bounded: the client's refetch policy, a measured target
- **Row 81:** the size cap is D7's; the rate limit is E's public door
- **Row 82:** the roster (D6) and chat delete (D7) are code; the rate limits are E's public door
- **Row 83:** a decision
- **Row 85:** deferred by R-6.5c.6
- **Row 86:** remove the dark arm and the inert parameter, or finish them
- **Row 87:** the M6.4 rulings
- **Row 88:** the release installs whole
- **Row 89:** owner rulings
- **Row 90:** make it true or say the truth
- **Row 91:** spec-conformant; your call
- **Row 92:** the docs say it in E
- **Row 94:** pool-proven
- **Row 95:** a short backoff after a failure, or the cost stated; decide once
- **Row 97:** the operator route exists
- **Row 99:** M6.5b's triage
- **Row 101:** ruled
- **Row 102:** the first transition has passed
- **Row 104:** by design
- **Row 105:** cached after one visit
- **Row 106:** the simulators and the handset
- **Row 107:** one short game at D's deploy
- **Row 108:** the simulator
- **Row 109:** read at D's live run, tail first
- **Row 110:** observable at D's deploy: a picker left open across it
- **Row 111:** needs a re-point
- **Row 112:** no live conflict to make
- **Row 114:** measure a plain game at E's live run
- **Row 117:** time one large replay at D's live run
- **Row 118:** nothing has aged 90 days
- **Row 119:** a second identity at E's public-door test
- **Row 122:** Track 2
- **Row 126:** the lane document
- **Row 127:** reopened: the table.ts split, read-once threading, the replay split, the script dedups
- **Row 130:** verify each; fix what stands
- **Row 132:** the merge and the tag close E
- **Row 133:** first task
- **Row 134:** with the CI reshape
- **Row 135:** script it at D's NAS deploy
- **Row 136:** after 2026-10-28
- **Row 137:** the query API at D's live run
- **Row 138:** the delete route (with 82)
- **Row 140:** recorded in the lane document
- **Row 141:** a CLAUDE.md rule
- **Row 143:** a mask in client.rs
- **Row 144:** a bga-human seat is a human seat
- **Row 151:** fenced

## B. Gaps a cheap check could close

- **The NAS under `cf-access` (row 107):** play one short game at `yokohama-nas.moore.nyc` over WARP, then read the table
  document's `turn.bot` and the record's `run_id` through the Portainer archive read; fetch `/` and `/sw.js` and compare with `web/dist`.
- **A second person on the NAS:** the same game with any other address on the Access allow-list; "not recorded" whether one exists.
- **The first-visit word (row 65):** open either host with cleared storage and read the strip before the first
  connection; expect "connecting…" (tree `web/src/signal.ts:98,270`), then strike the owner item.
- **The single-audience reading and R-F6 (row 108):** the Device Hub iOS simulator is a browser no WARP enrollment
  confounds: one login at `/enter`, then each destination; let the session lapse, then log in again.
- **The dealing screen (row 1):** a local 3P table under `open` with three names; keep at two seats and watch the third's "waiting on".
- **The Game menu and the sign-out sentence (rows 2, 3):** one click each in a headed local browser.
- **The four unprovoked error strips (row 21):** provoke the create form's, the deal's, the chat panel's and the open
  table's refusals at 390 px in a headed local browser.
- **M6.4's two layout LEAVEs (row 72):** one look at 500 to 560 px after B's rebuild.
- **The cold-pump arm (row 24):** read `web/src/local/` for whether "First player: The bot" can leave a stored record at
  the bot's turn; if so, the parked M6.5b reload clause is claimable.
- **The rustdoc count (row 50):** `cargo doc -p yokohama-web --no-deps` names the 34 at D's head.
- **The citation sweep (row 47):** a grep of `RUNBOOK §6` finds ten in code and configuration plus `CLAUDE.md:118`
  and the board (lines 16, 106), which matches the handover's twelve only if those two count.
- **"Copied" read twice (row 68):** one VoiceOver pass over the lobby's Copy link.
- **Lobby paging (row 84):** a pool or local cell with 21 open tables shows whether the 21st is reachable.
- **`picker-dropped` (row 110):** only cheap if D deploys a change to the engine, CLI or layer: leave a multi-step
  picker open on a throwaway table across that deploy and read the log. Otherwise not cheap.
- **A not-over foreign-bot game (row 111):** not cheap; it needs a server whose production bot differs.
- **The `two_browsers` close (row 109):** only on D's next live run, with a fresh tail read first.
- **The `test/` expiry (row 136):** after 2026-10-28, list `test/` for any object older than 30 days.

## C. Sentences false at 0.6.0

**`README.md`**
- `:213-215` "fill one row per seat: this is me, *someone@else*, or the bot … the table opens **dealing**": the form
  also offers "Open seat", the 3–4P default, and such a table opens `open` (tree `web/src/NewGame.tsx:45,54,302`).
- `:330-332` "the cleanup cycle after it, M6.5c2, starts from its handover": three cycles have closed; D's handover is the entry.
- `:387` (M6.2 history) "the first screen is a lobby": the first screen is the door since M6.5b2.
- `:399-401` (M6.3 history) "*Games in progress*" and "You are watching": now "Being played" and "You are spectating"
  (R-6.5c2.7). The italic note at `:356-358` names only the door's rename.
- `:415-417` (M6.4 history) "the name you type beside *You are*, the same string the table forms remember": the name
  now lives in the corner's "Playing as", and "Your name" left the forms (R-6.5c2.8).
- `:426-436` the list of each cycle's close stops at M6.5c; cycle C's close is missing.
- Go false at D's own acts: `:9-10` "current release **v0.5.0**" (at the tag); `:339-340` "operating detail … is in
  `CLAUDE.md`" (at the lane-document move).

**`deploy/README.md`**
- `:211-212` "an open tab offers 'Reload to use it', and only a click takes it": the offer renders on the door only
  (`OnThisDevice.tsx:234`, imported by `FrontDoor.tsx` alone), and a waiting release activates by itself once every old
  page closes (M6.5b2 §6.4). Nothing else in the guide read false.

**`worker/README.md`**
- `:1733-1734` the replay's first-send saves under `test/browser/red_at_wire.test/` are "kept for good by the owner's
  direction": the 30-day `test/` rule expires them (the same list's `:1730-1732`; C-cl §3.7).
- `:1826-1828` "The next cycle's entry … the cleanup cycle that reconciles this host with the NAS": a stale pointer.
- `:225-277` the screenshots and captions (the old door; "Your games", "Games in progress"; "You are watching"):
  flagged in place as recaptured in D.
- Goes false at D's move: `:1839-1840` "`CLAUDE.md`: … the lane's standing rules".

**Beyond the three files, found in passing (D's refresh owns them)**
- `CLAUDE.md:118` "A first visit reads 'reconnecting…' on both hosts": the client reads "connecting" (tree `web/src/signal.ts:98,270`).
- `CLAUDE.md:36` "Its vs-bot arm LIVELOCKS … its driving policy places nothing": cycle A gave it the placement probe and a
  5,000-input cap (tree `play_a_whole_game.rs:277`).
- `CLAUDE.md:44` a `CAP = 4000` hit "reports through `deadEnd` exactly as a stuck table would": it reports through
  `capReached` (tree `web/e2e/harness.ts:736`).
- `CLAUDE.md:121` "activated only on a click": as `deploy/README.md:211` above.
- board `:497-511` "No API route serves the raw record" is false since M6.5a's `GET …/record`; its `curl
  https://yokohama.moore.nyc/api/games` with `X-Yokohama-Claimed-User` now reaches the Worker, and the NAS reads no claimed
  header under `cf-access`.
- board `:204` "the Dockerfile's NAS image stays at 132": it pins 133 since C.

## D. Where the sources contradict each other

1. **The "connecting" word.** Open in C-cl §6.3 item 4, D-ho §6 item 6 and board Track 3 item (4). Done in tree
   `web/src/signal.ts:98,270` and `Door.tsx:74`, and wREADME `:1781-1784` says so. `CLAUDE.md:118` still says "reconnecting…".
2. **The public door's preconditions.** spec §2.10 lists six: the path-scoped application, the dev door off, the art,
   the project link, requests under a public load, the archive route's untrusted record. C-cl §6.3 item 3 and D-ho §2
   carry four, without the project link or the untrusted record, and wREADME `:1767` points at C's shorter list. umb
   R-6.5.3 and §6 left three more on the Access door (the email roster, the `join`/`unlock` oracles, chat moderation
   and rate limits), and no list since carries them.
3. **The single-audience reading and R-F6.** spec §7: taken in C's headed walk if the window allows, otherwise carried as
   owner readings. C-cl, its plan and D-ho name neither, so they were neither taken nor carried.
4. **M6.2 ruling #13.** board "Owner rulings outstanding" item 10 (`:535`) says "keep or strike". M6.3 §8 and the board's
   own M6.3 block (`:459-460`) say the owner approved it and it needs nothing.
5. **board `:106`.** Its first half says the Worker is `6da906a5`, the NAS `0.5.0-m6.5b2.1` "untouched", and EIGHT PRs; its
   second half says both hosts are on 0.6.0 with eleven PRs.
6. **board `:110`.** "The fifteen-gate roster last ran at the V4 merge head, 492.17 s". C-cl §2 read 461 s at `ea4f174d`
   and M6.5c §2 read 533 s, both on the Mac.
7. **`guidingpath.md:25`.** One paragraph says M6.5c2 stands at cycle C closed, then that M6 "has run FOUR of its five
   cycles", "four stacked pull requests", and "M6.5b … is next".
8. **The whole-game gate.** `CLAUDE.md:36` says it LIVELOCKS; board `:13` says it no longer does; C-cl §7 says the bound is
   measured over recorded seeds, not proven.
9. **The update offer.** `CLAUDE.md:121` ("activated only on a click") and `deploy/README.md:211` against M6.5b2 §6.4.
10. **The replay's test saves.** wREADME `:1733` "kept for good" against the same section's `:1730-1732` and C-cl §3.7.
11. **The Worker's runbook pointer.** board `:544` sends readers to `deploy/RUNBOOK.md` "§6 the Worker"; C-cl §1 Task 12
    moved §6 to `worker/RUNBOOK.md`.
12. **The `plays_locally` quota cell.** B-ho §1 already names its cause ("a seed whose first option opens a sub-picker");
    C-cl §2.1 presents that as a revision of M6.5c's "under load" reading. The two now agree on the cause.
