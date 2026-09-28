# Cycle D — the code: the dossier

**What this is.** The file:line facts cycle D's plan (`docs/plans/2026-09-28-web-frontend-m6.5c2d-plan.md`) is written from, compiled read-only by an Opus researcher on 2026-09-28 from an export of `web-room-m6.5c2d` at `09095ebb` (code identical to cycle C's reviewed head `ea4f174d`). Every plan task cites its rows here by number ("dossier row N") for WHERE and NOW, and carries the design and the steps itself. Line numbers are readings at that commit: **find by content once a task before yours has moved a file.** A dated artifact, frozen; corrections go forward in the plan's ledger and the close.

Compiled read-only on 2026-09-28 from an export of `web-room-m6.5c2d` at `09095ebb` (the reviewed code head is `ea4f174d`).
Nothing was run: no cargo, npm or test. Every line number is the export's, and every path is relative to the repository root.
Row numbers are the straw poll's. Each entry gives Where · Now · Fix touches (with corpus / `types.ts`-`contract.ts` / pool
fixtures / e2e) · Covered by · Risk/open · Premise. Tasks run 3, then 4 to 13, as the plan numbers them.

## Summary

- **Rows located:** 54 of 54 (53 numbered rows plus the seat-email roster). Sub-items not located: row 19's two cells (no
  tracked document names them; only C's gitignored ledger might); row 130's "three refusal paths" (the close names none);
  four of row 49's minors (the two `deploy-guard` comments, `archive.test.ts`'s closures, two long comment lines, the
  `status` arm), whose lines lived in the gitignored review.
- **Premises false at HEAD (whole rows):** 11 as written (since R-6.5b.13 an over-mirrored abandon lands under `over/` as
  `Identical`; a duplicate needs a stale mirror as well), 24 (a bot-first local game writes its first record after the
  bot's opening run, so the cold-pump arm is not reached), 35 (`hasMove` has one definition and three importers).
- **Partly false, moot or drifted:** 3 (unverified; the M6.1 trigger is guarded now), 4 (the first read only), 17, 38, 55
  and 56 (sub-items moot, marked), 23 (it names jsdom, which loads no CSS; only Playwright can hold it), 25
  (`Abandon.tsx:212` drifted), 32 (narrowed: `table_bot.json` shows a bot table at `dealing`), 39 (a held "wake" cell
  exists), 40 (two sub-items closed), 41 (an e2e restores a Worker-drawn game), 47 (11 in code and configuration, and
  `CLAUDE.md:118`'s citation is correct), 49 (three cited lines drifted), 100 (the door's Remove deletes without sending).
- **The riskiest fix:** the seat-email roster (Task 6). It reshapes every table payload, about twenty corpus files, eight
  client files and the contract, and its scope is open: chat authors still carry every poster's address (the house
  chat's to any signed-in caller), so R-6.5c2.35's pin "no payload built for one caller carries another seat's address"
  fails on `chat.json` unless authors are in scope. Runners-up: row 82's delete, which a count-based delivery cannot
  announce; row 86, where `deny_unknown_fields` turns a dropped `wait` or `after` into a 400 for an older precached
  release; row 128's poison recovery, sound for the `Mutex<()>` table locks and wrong for the per-game `Mutex<Repl>`.

**Where rows meet** (so one task does not undo another): the roster (Task 6) and row 17's seatless creator both key on
`created_by === me`; row 138 rides on row 82's delete; row 42's first item is moot if Task 8 retires the held arm; row 49's
`answerShape` twin is row 127's dedup kin; row 49 carries R-C10, R-C14, R-C18 and R-C20, which are rows 8, 9, 143 and 20;
row 55's `ChatConfig` item is also row 130's; row 56's `CAP = 4000` copies are row 41's; row 56's trap and 5xx items are
rows 14 and 99; row 130's 34 rem item is row 39's; row 2's fix must rewrite `the_table.spec.ts:160-180` in the same change.

## Task 3 — the small debts

### Row 47 — the remaining `RUNBOOK §6.x` citations
- **Where (code and configuration, 11 at HEAD), each with its target from the map `deploy/RUNBOOK.md:2310-2324` (§6.N → `worker/RUNBOOK.md` §N):**
  - `.github/workflows/test.yml:30` "(deploy/RUNBOOK.md §6.7)" → `worker/RUNBOOK.md` §7
  - `.github/workflows/test.yml:40` "through `npm run deploy` (RUNBOOK §6.2)" → §2
  - `deploy/Dockerfile:12` "which runs no image — RUNBOOK §6.)" → `worker/RUNBOOK.md`
  - `worker/wrangler.jsonc:6-7` "`deploy/RUNBOOK.md` // §6 is how it is built, deployed and read" → `worker/RUNBOOK.md` (wrapped across two lines, so a one-line grep misses it)
  - `worker/wrangler.jsonc:61` "RUNBOOK §6 says what each one's absence means" → §3
  - `worker/wrangler.jsonc:86` "RUNBOOK §6.3 says what changes" → §3; `:91` "(RUNBOOK §6.3)" → §3
  - `worker/scripts/read-deploy.mjs:9` "as RUNBOOK §6's recipe writes it" → §4 (the header-file recipe, `worker/RUNBOOK.md:326`)
  - `worker/scripts/read-deploy.test.mjs:53` "The credential's header file (RUNBOOK §6)" → §4
  - `web/e2e/exit_game.spec.ts:48` "the window RUNBOOK §6.7 opens and closes" → §7 ("The exit's window: counting wakes", `:637`)
  - `web/e2e/exit_game.spec.ts:73` "the table's 90-day marker (RUNBOOK §6)" → `worker/RUNBOOK.md`'s opening (the MARKER, `:62`)
- **Documents outside `docs/plans/`** (the dated records are frozen): `worker/README.md:1644` "(RUNBOOK §6)" names no file (right target `worker/RUNBOOK.md` §6, Rollback); `docs/handover.md:16` "each recorded in RUNBOOK §6" → `worker/RUNBOOK.md` §8–§14; `docs/handover.md:106` "`deploy/RUNBOOK.md` §6" → `worker/RUNBOOK.md`; `docs/handover.md:544` "`deploy/RUNBOOK.md` (§0–§3 and §5 the NAS, §6 the Worker)" → `worker/RUNBOOK.md`.
- **Not stale:** `CLAUDE.md:118` cites "`worker/RUNBOOK.md` §6", which is Rollback's current home; `deploy/RUNBOOK.md:12,2039,2300-2326` and `worker/RUNBOOK.md:3,16` are the move's own map.
- **Fix touches:** comments in seven files; no test reads them (only comments in `read-deploy.test.mjs` and `exit_game.spec.ts` name RUNBOOK). corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: comment only
- **Premise:** true, but the count differs: the handover's "twelve … and two more" (D-ho §6 item 5) reads 11 in code and configuration at HEAD (its ten plus `wrangler.jsonc:6-7`). The straw poll's `CLAUDE.md:118` is correct and is not one of them.

### Row 48 — the module figures into the root `Cargo.toml`
- **Where:** `Cargo.toml:30-42` (the browser module's paragraph), `:44-58` (the Worker module's).
- **Now:** browser: "2,843,070 B under plain `release`, 1,783,361 B under this profile, and 1,175,223 B / 456,048 B gzipped (`gzip -9`)" at M6.5b2's fix-wave head (2026-09-22, `wasm-opt` 132), and "a whole bot turn going 28.6 -> 35.2 ms". Worker: "1,256,770 B, and 491,409 B gzipped at level 9 … 18 pages (1,152 KiB), 123,040 B of them the module's data … 170 data segments (the data SECTION is 124,439 B encoded)" at `d431dbc1`.
- **The readings to transcribe** (C close §3.1, `docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md:198-203`; `worker/RUNBOOK.md:1415-1418`): browser 1,814,905 → 1,197,069 B under `wasm-opt` 133, with no gzip reading recorded anywhere; Worker 1,901,547 → 1,258,106 B, gzip 491,934 B, 18 pages, 170 segments, 123,047 B of data, the section 124,446 B.
- **Copies that restate the old browser figure:** `web/scripts/wasm-opt.mjs:5-7` and `web/src/precache.ts:122,144` (dated "re-measured 2026-09-22" readings); `deploy/RUNBOOK.md:127` "1,175,223 B raw / 456,048 B gzip since M6.5b2", present tense and false at 0.6.0. CLAUDE.md:40's rule is "cite it, never copy it".
- **Fix touches:** the `Cargo.toml` comment (and the copies, if in scope). No test pins a figure (`module-figures.test.mjs:6` names the paragraph only). corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Risk/open:** D's Tasks 4–8 move the layer, so D's release build reads different bytes; the plan picks C's `ea4f174d` reading (as the handover says) or D's deploy-time build. The browser gzip needs a `gzip -9` reading. The plain-`release` 2,843,070 B and the 28.6 → 35.2 ms were not re-read at C.
- **Premise:** true at HEAD.

### Row 49 — cycle C's deferred comment and doc minors (C close §6.2, `…-m6.5c2c-close.md:437-446`)
The review file and ledger that carried each minor's line are gitignored, so each is located by its cited line or by search.
- the two `deploy-guard` comments — not locatable from tracked sources.
- `deploy-guard.mjs`'s success line — `worker/scripts/deploy-guard.mjs:430-436`.
- `archive.test.ts`'s three written-out closures (drop) — not located.
- `identity_inner`'s hand-built 401 — `crates/yokohama-worker/src/lib.rs:356-362`: `Err(sentence) => (401, serde_json::json!({ "error": sentence }).to_string())`.
- `receipt.rs`/`picker.rs`'s "one document" class — `crates/yokohama-worker/src/receipt.rs:461-468` ("**One document reaches that 500**"), `picker.rs:1033-1036` ("through one document alone"). This is R-C14 (row 9): state the class.
- `module-figures.mjs`'s LEB128, section bounds and gzip wording — `worker/scripts/module-figures.mjs:28-39` (`u32`, no five-byte bound on the shift), the section walk `:55-58` (`const end = at + size`, never checked against the module's length); `:9` and `:19` say "`gzip -9`" where `:79` runs Node's `gzipSync` at level 9 (`Cargo.toml:50-51` records the 24 B header difference).
- `lib.rs:507`'s "only the two calls" — `crates/yokohama-worker/src/lib.rs:507-509`; `Scope::Table` calls also clear a picker (`:494-496`).
- the `status` arm comparing the layer with itself — not uniquely located; candidates `crates/yokohama-worker/src/lib.rs:1141-1218` (the `direct` comparisons) and `receipt.rs:1276-1289`.
- the scan partners' comment — `crates/yokohama-worker/src/lib.rs:1369-1373` ("Three anti-vacuity partners"); twin `crates/yokohama-wasm/src/lib.rs:401` ("Four").
- `SECOND_PLAN`'s unnamed premise — `crates/yokohama-worker/src/receipt.rs:87-94` (doc and const), refused at `:248`.
- `acknowledged`'s doc — `worker/src/table.ts:368-401`, on `:402`.
- the memo's unstated invariant — `worker/src/storage.ts:52-62` ("Only `deleteAll()` drops the tables") against `worker/src/table.ts:1778-1786`: `transactionSync(() => { ensureTables(sql); writes(sql); })` has no catch, so a `hasTables` answered true inside a batch that then throws leaves `known` naming tables the rollback removed. Fix: `forgetTables(sql)` in a catch, or the sentence.
- two long comment lines (drop) — not located.
- `operator.test.ts`'s optional `DELETE` (drop) — candidates `worker/test/operator.test.ts:418,427`.
- `bots.rs:333`'s "only when" — `crates/yokohama-web/src/bots.rs:333-336`.
- `read-deploy.mjs:178`'s twin — `answerShape` `worker/scripts/read-deploy.mjs:175-180`; the twins `worker/scripts/live-replay.mjs:371` and `web/e2e/live.ts:211` (dedup kin of row 127).
- `live.ts:322`'s omitted reason — `web/e2e/live.ts:318-324` (`waitEnded`: the non-timeout arm prints the error's name, never its message).
- `test.yml:210`'s "to the clock" — `.github/workflows/test.yml:210-211` "the scan that holds its platform calls to the clock"; the wasm scan allows the clock AND the console (CLAUDE.md:40).
- `docker.yml:49`'s warm example — `.github/workflows/docker.yml:49-52`.
- `record.rs:181`'s "never-bumped" — FENCED (core, the AI lane's): out of D.
- `plays_locally.spec.ts:335`'s detached-HEAD read — drifted to `web/e2e/plays_locally.spec.ts:359-361` (`git rev-parse --abbrev-ref HEAD` reads "HEAD" when detached).
- `styles.css:2473`'s shared selector and `:2487`'s grouping — at HEAD the cited lines are a blank line and a comment; the 44 px block is `web/src/styles.css:2475-2526` (selector lists `:2497-2499`, `:2515-2518`).
- `setup_screens.spec.ts:331`'s ~18 px — `web/e2e/setup_screens.spec.ts:330-332` (a figure the cell does not measure).
- `OpenTable.tsx:82`'s error sources — `web/src/OpenTable.tsx:80-85` lists the join, the password and a copy; the opening read's catch (`:134`) also sets `error`.
- Rows of their own in the same list: R-C10 (row 8), R-C14 (row 9, above), R-C18 (row 143), R-C20 (row 20), the rustdoc warnings (row 50), the two jsdom cells (row 19).
- **Fix touches:** comments only, in about fifteen files. corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: comments only
- **Premise:** true; three cited lines (`plays_locally.spec.ts:335`, `styles.css:2473`, `:2487`) no longer land on the named text.

### Row 50 — rustdoc warnings in `yokohama-web` (34 at the C close, `…-m6.5c2c-close.md:447`)
Candidates by reading, since cargo was not run. rustdoc strips private items before it checks links (a library's default), so only documented (public) items count. Paths under `crates/yokohama-web/src/`.
- **Links to non-public items on public docs (`private_intra_doc_links`), 21:** `api.rs:256` `find`; `assets.rs:8` `cache_control` (module doc); `backup.rs:176` `check_pair_with`; `bots.rs:77` `label_of`; `client.rs:260` `drop_hand_for`; `grammar.rs:112` `HEADS`; `store.rs:171` `RESERVED_ID_SUFFIXES`; `table.rs:473` `crate::bots::table_bot`; `tables.rs:340` `config_from`, `:344` `choices_of`; `games.rs:18` and `tables.rs:13` `Games::settle` (module docs; `pub(crate)`, `tables.rs:1179`); `games.rs:146,370` `Games::ticking_clock`; `games.rs:635` `Games::mirror_turn`; `games.rs:918` `Games::archive_if_over`; `tables.rs:341` `Games::finalize`; `tables.rs:363,1668` `Games::read_table`; `tables.rs:1129,1657` `Games::lock_for`.
- **Unresolved (`broken_intra_doc_links`), 2:** `api.rs:796` `[`Games::keep`](games::Games::keep)` and `api.rs:1977` `backup::check_pair_with`; api.rs imports neither `games` nor `backup` (`:21-30`).
- **Unclosed HTML tag (`invalid_html_tags`), 1:** `tables.rs:664` "over · <name> won", on `pub enum Scope`.
- **Only under `--document-private-items`:** `lib.rs:559,560,575,610` (`events_at`, `admitted` on the private `events`/`stream` handlers); `tables.rs:584` (`client::facts_for` on the private `first_human_seat`); `api.rs:2048` (a `tests::` link on `pub(crate) archive_would_accept`).
- **Not accounted for:** about ten. Possible sources: `client.rs:283` `Repl::bots` (the CLI's `Repl` has a `pub(crate)` field and a `pub fn` of that name, `crates/yokohama-cli/src/repl.rs:67,679`), and the `main.rs` bin target, which cargo documents with private items. The task starts by running `cargo doc -p yokohama-web --no-deps` for the real list.
- **Fix touches:** doc comments in the layer only. No gate runs rustdoc for the layer today (the Worker crate reads 0, CLAUDE.md). corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Premise:** true as recorded; the count is not verifiable without cargo.

### Row 51 — four `CLAUDE.md` sentences false at HEAD
- **`CLAUDE.md:36`:** "**Its vs-bot arm LIVELOCKS on some OS-entropy seeds** — its driving policy places nothing and leans on the bot to end the game (M6.3 close §6.3, an owner item)". False since cycle A: `probe_a_placement` and the stuck-picker rule (`crates/yokohama-web/tests/play_a_whole_game.rs:25,160,190-198`), and the measured `VS_BOT_CAP` of 5,000 (`:270-277`). The true residual is row 33's: the cap rests on 50 seeds.
- **`CLAUDE.md:44`:** "its `CAP = 4000` loop iterations can be hit on a long game, which then reports through `deadEnd` exactly as a stuck table would (M6.3 close §6.3)". False: a cap hit reports through `capReached` (`web/e2e/harness.ts:733-742`, "**Never `deadEnd`'s sentence.**").
- **`CLAUDE.md:118`:** "**A first visit reads "reconnecting…"** on both hosts until its first connection". False: it reads "connecting" (`web/src/signal.ts:94-98`, `Link = "connecting" | …`; `web/src/Door.tsx:74-76`).
- **`CLAUDE.md:121`:** "installed all-or-nothing with every body digested, activated only on a click, the previously ACTIVATED generation RETAINED (a tab that did not click is served its own release from it)". False: the offer renders on the door only (`web/src/OnThisDevice.tsx:202-205,234`, imported by `FrontDoor.tsx:29,154` alone), and a waiting worker nobody clicks activates by itself once every old page closes (`OnThisDevice.tsx:203-205`; M6.5b2 close §6.4). `deploy/README.md:211-212` repeats the claim.
- **Fix touches:** `CLAUDE.md` (and `deploy/README.md:211-212`). E moves these passages to `docs/web-lane.md` (R-6.5c2.32), so D fixes them in place or hands E the corrected words. The four lines are 1,441 to 4,190 characters long; each fix is one clause.
- **Premise:** true at HEAD, all four.

## Task 4 — the layer's defects

Paths are under `crates/yokohama-web/` unless named; line numbers are the export's (`09095ebb`).

### Row 5 — `undo_allowed` answers 403 on a replay failure; replays len−1 per try
- **Where:** `src/api.rs:639-692` (`undo_allowed`); the replay `:686-688`; the caller's 403 `:1120-1126`.
- **Now:** `let mover = yokohama_core::state_at(record, len - 1, repl.session.rules()).map_err(|e| e.message)?` — the `ReplayError` becomes a `String`, and the caller maps every `String` to `status: StatusCode::FORBIDDEN`. Every undo attempt at a shared table replays `len - 1` events.
- **Fix touches:** `src/api.rs` only (a typed error: refusal → 403, `ReplayError` → 500; optionally a cheaper mover). corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** `undo_is_admitted_mid_turn_and_refused_once_the_turn_has_passed` (`src/lib.rs:2855`), `undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before` (`:2957`), `a_watcher_cannot_undo_a_finished_game` (`:3267`), `a_locked_game_answers_every_callers_undo_with_the_lockdown_reason` (`:3316`), `the_acting_holder_cannot_undo_a_finished_shared_game` (`:3410`). No cell reaches the `ReplayError` arm.
- **Risk/open:** unreachable through a sound record (a resident `Repl` came through `Session::from_record`), so the 500 cell needs a hand-built record. Adjacent stale doc: `src/grammar.rs:110-111` says the rule "lives in `lib.rs`'s `undo_allowed`"; it is in `api.rs`.
- **Premise:** true at HEAD.

### Row 6 — the waiting caller's set table is built for `first_human_seat`
- **Where:** `src/tables.rs:1307` (`let seat = doc.next_unkept_seat_of(user);`), `:1321` (`seat.unwrap_or_else(|| first_human_seat(&doc.seats))`), `first_human_seat` `:594-598`.
- **Now:** once the caller's seats have all kept, `seat` is `None` and the frame is drawn for the LOWEST human seat, so blue (seat 1) waiting gets `frame.seat` 0. Every human hand is `Unknown` on that projection (`:622-629`): no card leaks.
- **Fix touches:** `src/tables.rs` (`seat.or_else(|| doc.seats_of(user).first().copied())`; `first_human_seat` and its half-cell go or stay as the fallback). corpus: no (`web/src/wire/deal_waiting.json` is red's wait, `frame.seat` 0 either way) · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** `the_set_table_is_drawn_for_a_human_seat_and_never_the_bots` (`src/tables.rs:2112`, asserts `first_human_seat` directly `:2169-2170`), `the_deal_serves_the_callers_own_cards_and_never_the_others` (`:2061`); the walk's `deal_waiting` (`tests/wire_fixtures.rs:939`). No cell has BLUE waiting.
- **Risk/open:** `seats_of(user)[0]` is total only because `deal` refuses a seatless caller (the walk's 403, `tests/wire_fixtures.rs:943-945`); `seats_of` never names a bot, so the doc's bot-unmask worry (`:578-583`) goes away.
- **Premise:** true at HEAD.

### Row 7 — `create` reads a store I/O error as "already exists"
- **Where:** `src/tables.rs:739-747` (the occupied probe); the write arm `:792-794`.
- **Now:** `Ok(found) => found.is_some(), Err(_) => true,` then `GameError::BadRequest(format!("a game named {id:?} already exists"))` — a 400, the store's sentence dropped.
- **Fix touches:** `src/tables.rs` (keep refusing; answer `GameError::Server(e)` or log the sentence). A new `src/log.rs` event (`:66-77`) would also be named in `worker/RUNBOOK.md:383-384` and `worker/README.md:1691-1692`. corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** `create_refuses_a_name_whose_record_is_unreadable` (`src/tables.rs:3227`, asserts "already exists", so it changes); `an_unreadable_entry_is_present_not_absent` (`src/store.rs:925`).
- **Risk/open:** the refusal is load-bearing (the `chmod 000` measurement: a 201 overwrote the record, `:733-735`); only the status and sentence may move.
- **Premise:** true at HEAD.

### Row 10 — `api::matches` binds an empty path segment
- **Where:** `src/api.rs:238-250` (`matches`), the arm `:245`.
- **Now:** `(Some(p), Some(x)) if p.starts_with('{') => bound.push(x),` binds `""`. `/api/games//table` reaches `get_table` (`:775`), then the store's `"\"\" is not a usable game name"` (`src/store.rs:424-426`) as `GameError::Server`: a **500** (`src/lib.rs:208`), where matchit answers 404.
- **Fix touches:** `src/api.rs:245` (`&& !x.is_empty()`). The Worker already sends an empty name to the house (`worker/src/route.ts:51-52`, pinned `worker/src/route.unit.test.ts:50`), whose dispatch then answers the layer's no-route 404. corpus: no · types.ts/contract.ts: no · pool fixtures: no (no pool cell reads such a path) · e2e: no
- **Covered by:** `the_matcher_binds_segments_and_tells_a_wrong_method_from_no_route` (`src/api.rs:2736`), `the_matcher_finds_every_route_in_apps_order` (`:2693`).
- **Premise:** true at HEAD; the answer is a 500, not a refusal.

### Row 143 — a finished same-name re-point shows the CLI's drift warning (R-C18)
- **Where:** `src/client.rs:538` (`warnings: without_version_drift(without_foreign_conversion(...))`), masks `:321-330` and `:554-557`; the CLI's lines `crates/yokohama-cli/src/bot.rs:482-507`, pushed at `:283`.
- **Now:** a same-name re-point parses and is eligible, so the CLI pushes `warning: seat {i} records controller "…" at version {v}, but this build runs version {w} — the seat still plays, under the CURRENT build`. Neither mask's prefix (`warning: seat N was played by controller`, `warning: GameRecord.engine_version=`) matches it.
- **Fix touches:** `src/client.rs` (drop, per `bots::foreign` seat, the `warning: seat {seat} records ` lines; knob-only drift shares that prefix). corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** `a_same_name_re_point_is_told_apart_in_every_refusal_and_keeps_its_label_on_every_screen` (`src/games.rs:1273`; reads `over`, `bots` and the badge at `:1332-1343`, never `warnings`: the fix's assertion goes there); `a_finished_foreign_game_opens_scores_downloads_and_masks_the_conversion` (`src/api.rs:2514`, `warnings == []` at `:2521`); `tests/browser_played_pin.rs:174,219-220`.
- **Risk/open:** spec §10.8 R-C18 kept the CLI-worded warnings apart from §2.6's filter; the fix reverses that choice and should say so.
- **Premise:** true at HEAD (an unfinished re-point is a 409, so only a finished one serves the line).

### Row 144 — a `bga-human` seat reads as a foreign bot
- **Where:** `src/bots.rs:119-133` (`recorded`), filter `:124`; callers `src/api.rs:653`, `src/client.rs:294,325,540`, `src/games.rs:119,571`, `src/explain.rs:472`, `crates/yokohama-worker/src/receipt.rs:1004`.
- **Now:** `.filter(|(_, s)| s.controller != CONTROLLER_HUMAN)` counts `bga-human` (`crates/yokohama-cli/src/bga/mod.rs:46`) as a bot, so `foreign()` names it: unfinished → the 409 "played against bga-human"; finished → undo refused; `ClientState.bots` and the Players badge mark a person. The CLI resumes it as HUMAN (`crates/yokohama-cli/src/bot.rs:293-301`).
- **Fix touches:** `src/bots.rs` (skip `yokohama_cli::bga::CONTROLLER_BGA_HUMAN`), plus `src/client.rs:321-330`, whose mask keys on `foreign()` and would then leave the CLI's "resumes as a HUMAN seat" line showing. corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** `an_unparseable_descriptor_answers_its_controller` (`src/bots.rs:349`, uses "bga-human"), `recorded_names_the_record_s_bot_seats_in_order` (`:359`). No cell loads a bga-human record through a route.
- **Risk/open:** reachable only through a hand-made pair: `check_pair_with` (`src/backup.rs:187-247`) and `agrees_with` (`src/table.rs:341-380`) check no controller. The B handover (`docs/plans/2026-09-28-web-frontend-m6.5c2c-handover.md:88-89`) and the straw poll mark it the AI lane's; the plan should say why D takes it.
- **Premise:** true at HEAD.

## Task 5 — the NAS server's defects and minors

### Row 11 — a landed game-over put whose stamp failed is archived again (IMP-1)
- **Where:** `src/tables.rs:1699-1730` (`delete`'s abandon-and-save), leaf `:1714-1718`; `src/games.rs:847-890` (`archive_if_over`), `:892-914` (`stamp_over`), `mirror_turn` `:952-960` (write result dropped `:959`); `src/archive.rs:306-310` (`DirArchive::put`: same record bytes → `Identical`).
- **Now:** since M6.5b's R-6.5b.13 an unstamped game whose MIRROR says over is put under `over/` at abandon, where the landed copy answers `Identical` and the delete proceeds. A duplicate under `abandoned/` survives only when the mirror is ALSO stale (its write failed or conflicted), the residual named at `src/tables.rs:2632-2638`.
- **Fix touches:** `src/tables.rs` (take the leaf from the record, e.g. `handle(id).is_over`, declined for cost; or probe `over/` first). Two doc sentences are false as written: `src/archive.rs:93-95` ("a DUPLICATE under the other leaf at the next abandon") and `src/games.rs:884-887`. corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** `a_finished_unstamped_game_deleted_lands_under_over` (`src/tables.rs:2641`), `a_game_that_ends_is_archived_once_with_the_stored_bytes` (`:2569`), `a_failing_archive_at_game_over_still_answers_the_move` (`:2600`), `a_stamp_that_does_not_land_is_said` (`src/games.rs:2513`). No cell lands a put, fails the stamp, then abandons.
- **Premise:** FALSE at HEAD as written; a narrower residual (stale mirror plus lost stamp) remains.

### Row 128 — NAS server minors (M6.2 §6.2)
- **False tombstone while an unlink fails** — `src/tables.rs:1744` inserts, `:1745` unlinks, `:1749` removes on failure; a `persist` inside that window refuses "deleted while this move was in flight" and evicts (one refused move). Every test store's `delete` delegates (`src/games.rs:1925,2065,2152,2562,2932`; `src/lib.rs:3947,6100,6863`): no cell. Premise true. Touches `src/tables.rs`.
- **`locks` never pruned** — `src/games.rs:258-273` (the field doc rules it by design: a prune could hand two writers two locks), `lock_for` `:517-520`. Premise true; a by-design line, or a prune under the outer mutex when `Arc::strong_count == 1`.
- **A poisoned lock is one dead game** — `lock.lock().expect("table lock")` at `src/tables.rs:962,1139,1183,1368,1453,1677`. A `Mutex<()>` guards no data, so `unwrap_or_else(PoisonError::into_inner)` is sound there. The per-game `Mutex<Repl>` (`expect("a game")`, 13 sites in `src/api.rs`) is not: evict instead. Covered by `a_malformed_table_document_refuses_typed_and_leaves_the_server_alive` (`src/tables.rs:2408`), `the_last_index_answers_and_leaves_the_game_usable` (`src/lib.rs:5448`). Premise true; `check_shape`'s bounds are today's preventer.
- **`written.remove`/`live.remove` after the unlink** — `src/tables.rs:1752-1753`; shown unreachable by the M6.2 review, carried for the record.

### Row 129 — NAS transport minors (M6.3 §6.2 Tasks 2 and 4)
- **`events_at` reads and settles the table twice** — `src/api.rs:1726` (`table_for_anyone`) and `:1740` → `admitted` `:499`. It runs once per bump per connection on the NAS stream (`src/lib.rs:664`) AND per reading on the Worker (`crates/yokohama-worker/src/events.rs:61` calls the same `api::events_at`), so the fix also saves Worker rows. Fix: an `admitted_with(table, …)` in `src/api.rs`. Covered by `events_answers_in_five_fields_at_every_status_and_fences_its_query` (`src/lib.rs:2756`), `events_answers_an_open_table_with_zero_events_and_status_open` (`:6380`), `events_carries_the_chat_count_at_open_dealing_and_playing` (`:7010`). Premise true.
- **Blocking store I/O in the stream's per-frame closure** — `src/lib.rs:658-679` (`WatchStream::new(rx).map(move |bump| … api::events_at(…))`): synchronous store reads and a game-lock take on the executor, no `spawn_blocking`. Fix: `.then` plus `spawn_blocking`, or a by-design line (bounded by open tabs). Covered by `the_stream_writes_the_current_events_on_connect_then_one_frame_per_bump_and_an_end_on_delete` (`src/lib.rs:5930`), `a_failed_reading_writes_a_comment_and_the_stream_stays_open_for_the_next_bump` (`:6062`). Premise true; NAS only.
- **One spurious hot-seat 403** — `src/api.rs:1089` computes the seat under one lock take (`:503-506`), `:1102-1103` re-locks, and the fence `:1166-1170` compares against the first take's seat. Fix: recompute `client::viewer_seat(&table, user, repl.session.state())` (`src/client.rs:233-239`) under the second lock. Covered by `input_is_the_acting_seats_holders_and_names_whose_turn_it_is` (`src/lib.rs:2815`), `a_hot_seat_creator_is_served_seat_zero_then_seat_one` (`src/tables.rs:2272`); no cell races the two takes. Premise true (it never admits wrongly).

### Row 130 — M6.4's review LEAVEs (M6.4 §6.2); all premises true at HEAD unless marked
- **Server, Task 1:** `HOUSE` passes `is_safe_id` by accident — `src/chat.rs:47`, `MemStore::read` `src/store.rs:423-427`. `check_shape` accepts a whitespace-only text — `src/chat.rs:280`. Normalized `User`/`check_name` results are discarded on read — `src/chat.rs:287-297`. The generic store names `crate::chat::HOUSE` — `src/store.rs:204-209` (R-plan-1's one layering exception).
- **Server, Task 2:** `trim(Some(0))` unguarded (the config refuses 0) — `src/chat.rs:222-229` vs `:192`. `read_chat`'s `id` is dead on the house branch — `src/tables.rs:849-856`. The house key/doc-kind rule is stated twice — `src/tables.rs:854-855` and `:889`. `keep_all_trims_nothing` loops 600 appends, N load-bearing — `src/tables.rs:4488`. "Thin coverage on three refusal paths" — the close names no path: unlocated.
- **Server, Task 3:** the lobby refusal quotes its value lowercased — `src/chat.rs:170-173`, where KEEP and TEXT quote as typed (`:195`, `:206`). A non-UTF-8 value folds to the default — `src/chat.rs:146-150` (`std::env::var(..).ok()`). `ChatConfig`'s two public fields have no doc comments — `src/chat.rs:131-134`.
- **Server, Task 4:** `house_chat_post` repeats `author_of`'s Spectator arm — `src/api.rs:1003-1004` vs `:962-964`. `chat_read` reads the table twice — `src/api.rs:910` (`admitted`), then `chat_page`'s `read_table` `src/tables.rs:995-997`.
- **Corpus, Task 5:** mixed-page authors checked at run time only — `tests/wire_fixtures.rs:1187-1191`. The frozen-clock cell is broader than asked — `every_chat_message_carries_the_frozen_clock` `tests/wire_fixtures.rs:3385`. Two 403 fixtures repeat existing sentences — `chat_403_not_seated` `:818`, `chat_403_locked` `:1218` (dropping them moves the corpus and `web/src/wire/contract.ts`).
- **Client, Tasks 6–7:** the deferred re-ask recurses — `web/src/Chat.tsx:221` (`await read(again)`). `unread` is dead on a count host, by design — `web/src/Chat.tsx:94,321`. Two chat reads at mount on the wake hosts — `Chat.tsx:258-267` skips only the host's FIRST tick, and each host ticks on every `take` (`web/src/Lobby.tsx:218`, `web/src/OpenTable.tsx:119`): plausible still, NOT re-measured. "Six `exact: true` sites, not seven" is a report count with no code.
- **Client, Task 8:** the players' baseline of two stream opens is pinned by no assertion (an e2e note); no code located.
- **Second fix wave:** a TICK host pays one delta GET per own post — `web/src/Chat.tsx:195` parks only `why === "count"`, and `:266` reads on every tick (hosts `Lobby.tsx:531`, `OpenTable.tsx:357`). The other two lines are other rows: 34 rem tied to `--side` is row 39's, and the who-strip's 481–560 px band is row 72's.

## Task 6 — the seat-email roster

### Roster — every seat's address and the creator's on the wire (R-6.5c2.35)
- **Premise:** true at HEAD. `TableDoc::view` clones every seat with its `user`, plus `created_by`, for any admitted caller (`crates/yokohama-web/src/table.rs:388-413`). The type's own doc says so (`table.rs:416-440`).
- **Layer serializers:**
  - `table.rs:39-56` `SeatDoc::Human { user, name }` crosses as stored in `TableView.seats` (`:399`, field `:450`); `created_by` cloned (`:392`, field `:445`).
  - Every `TableView` producer: create `tables.rs:790`; unlock `:1152,:1164`; join `:1426`; keep `:1530-1532`; `DealView.table` `:1325`; list rows `:1636` (`GamesList` `api.rs:719-725`; scopes `open`/`playing` answer any signed-in caller, `tables.rs:652-667`); `get_table` `api.rs:781` (any caller at `open`).
  - Refusal sentences: `api.rs:567-572` `holder_of` feeds `api.rs:1168` `"it is seat {}'s turn ({})"` with the acting seat's address, to a seated non-acting caller (corpus `wrong_seat_403.json`). `tables.rs:1685-1688` `"only {} can delete this table"` names `created_by` to a seated non-creator.
  - Chat authors: `chat.rs:79-96` `Author::Seat { seat, user, name }` and `Author::Watcher { user, name }` serialize the poster's address to every reader. Built by `author_of` `api.rs:945-966`; the house always `Watcher` (`api.rs:996-1006`), readable by any signed-in caller (`api.rs:984-988`).
  - Carry no address: `Events` (`api.rs:1606-1619`); `ClientState` (bot labels only, `client.rs:190`); `outcome.rs:59-65` `Placing`; the record (`finalize` stamps bot provenance only, `tables.rs:1222`).
- **Client readers (non-test):**
  - `web/src/tables.ts:27-36` `seatName` returns `seat.name ?? seat.user`. Used by `seatNames` `:44`, `holder` `:54-56` (the lobby's "waiting on …"/"… won", `:78,:84,:98,:105`), `seatChips` `:161-170`, `Dealing.tsx:298`, `TableFacts.tsx:17`.
  - `Dealing.tsx:365` `seat.kind === "human" && seat.user === me ? "you" : seatName(seat)`: the one reader keyed on the address.
  - `OpenTable.tsx:238-239` finds the opener by `s.user === table.created_by` and shows `opener.name ?? opener.user`; `:283-284` card faces `s.name ?? s.user`; `:347` `Invite mine={table.created_by === me}` (password hint `:435`); `:365` Abandon drawn when `created_by === me`.
  - `Lobby.tsx:315` `yours = section === "mine" && t.created_by === me` draws the row's Abandon (doc `:108-120`).
  - `App.tsx:1474-1475` `mayAbandon = local || (broken.table.created_by === signedIn && broken.table.my_seats.length > 0)`.
  - `Chat.tsx:326,331` author label `a.name ?? a.user`, seat and watcher.
- **`my_seats`:** `table.rs:404` = `seats_of(user)` (`:242-247`, lowest first). Read at `tables.ts:84,98,101,105,162,215-216`, `Lobby.tsx:90` (`notMine`), `OpenTable.tsx:229,282`, `App.tsx:633,1156,1475`. It is already the per-caller key.
- **Worker shell:** reads `created_by` only off the house COPY, as the caller an expiry dispatches (`worker/src/table.ts:2117-2151`, `copyFacts`). The operator listing parses no seat or creator (`worker/src/house.ts:200-225`). A view change does not touch it.
- **Types and contract:** `web/src/types.ts:548-551` human seat `{kind, user, name}`; `:568` `created_by: string`; `:580` `my_seats`; `:1012-1015` seat/watcher `ChatAuthor.user`. `web/src/wire/contract.ts:640-644` `TABLE` key set lists `created_by`; `:696-697` both author key sets list `user`. No key set exists for a human seat, so a `you` key is checked at compile time only.
- **Corpus files with an address** (21 of the 101 JSON files; `why.json`'s `@` is a contestant string). Another seat's address (blue@wire.test beside the reader red): `table.json`, `table_dealing.json`, `table_watchable.json`, `deal.json`, `deal_waiting.json`, `games.json`, `games_next.json`, `games_playing.json`, `wrong_seat_403.json`, and `chat.json` (red, blue and green authors). The caller's own only: `me.json`, `table_open.json`, `table_bot.json`, `games_open.json`, `chat_post.json`, `chat_post_watcher.json`, `chat_after.json`, `house_chat.json`, `house_chat_post.json`. `table_foreign.json` names `local@browser`; `identity_cases.json` is the key grammar's contract.
- **Leakage pins:** `no_committed_payload_carries_the_setup_seed` (`crates/yokohama-web/tests/wire_fixtures.rs:2651`, partners `:2700-2720`); `no_payload_of_this_run_carries_the_password_or_its_hash` (`:2763`); `a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author` (`:2866-2960`; its tree walker `find`/`at_path` is reusable; its one allowance is `author.user` beside `kind: "watcher"`). A new pin needs each payload's READER: `Corpus.payloads` carries none (`:2229-2241`); `ScriptRow.who`/`fixture` give it for walked rows (`:154-165`); `constructed()` rows (`:2044`) have none. Identities: `WALKER` red `:108`, `OTHER` blue `:594`, `GREEN` `:612`, `STRANGER` grey `:622`, `QUIET` `:637`. The partner: the address IS in the stored table document, as the seed pin's is.
- **Covered by:** `table.rs:571` `the_view_carries_no_seed_and_no_keeps_and_marks_a_bot_chosen_from_the_start` (asserts seats and `my_seats`, `:575-582`), `table.rs:549`, `table.rs:856`. jsdom cells keyed on `created_by`: `OpenTable.test.tsx:379,484`, `App.test.tsx:1874-1899`, `Lobby.test.tsx:129-132,327,488,579`.
- **Fix touches:** `table.rs` (`view`, a seat wire type), `api.rs:567-572,1168`, `tables.rs:1685-1688`, `chat.rs` if authors are in scope, the eight client files above. `corpus: yes · types.ts/contract.ts: yes · pool fixtures: no (the corpus replay regenerates; `worker/test/archive.test.ts:77` edits a stored DOCUMENT) · e2e: no spec reads created_by`.
- **Risk/open:** (1) Chat authors: the ruling names seats and `created_by`, but `Author.user` still sends every poster's address, the house's to any signed-in caller. In or out decides whether the watcher pin's allowance inverts. (2) A seat with `name: null` (a holder who kept with no name) has no display once `user` leaves; it needs a fallback word. (3) The creator's display name lives on their seat; a seatless creator (row 17) has none.

## Task 7 — chat delete and moderation, the archive route's size cap

### Row 82 — no chat moderation or delete (the moderation half)
- **Where:** route table `crates/yokohama-web/src/api.rs:188-215` (chat GET/POST only, `:203-204,:210-211`); dispatch arms `:2132-2157`. Append: `tables.rs:954-979` (table: its lock, then `bump_table`), `:981-986` (house: `house_lock`, then `bump_house_chat`), both over `append_locked` `:924-940`. Removal today: the trim only (`chat.rs:222-231`) and a table delete (`store.rs:387,463`, both stores). Worker operator surface: `worker/src/admin.ts:48-56` (`operatorGate`), `:71-79` (`adminRoute`: `GET /api/admin/tables`, `POST /api/admin/tables/{id}/expire[?force]`), wired `worker/src/index.ts:143-151`, `house.ts:103-104`, `table.ts:187-189`; the list is a secret (`worker/src/config.ts:151-160,206-209,238`). NAS: no operator surface, by ruling R-6.5c2.2 (spec `:99,:1168`).
- **Now:** R-6.4.14 "No edit or delete of a message, no moderation, no rate limit on the NAS, no kick" (`2026-09-16-web-frontend-m6.4-chat-spec.md:248-250`). Umbrella R-6.5.3 left it on the Access door: "the house chat's lack of delete (which the house object's SQLite makes one statement, still not built)" (`2026-09-19-web-frontend-m6.5-hosting-spec.md:268-270,729-731`).
- **Author:** `author_of` (`api.rs:945-966`): a seat is `Author::Seat { seat, user: caller, name: the table's }` and a body name is a 400; a watcher is `Author::Watcher { user: caller, name: body }`. The stored document keeps the address, so "delete your own message" is decidable server-side.
- **Fix touches:** `api.rs` (route + dispatch), `tables.rs` (a remover under the same lock-then-bump order), `chat.rs` (tombstone or removal that keeps `last`), Worker `admin.ts`/`index.ts`/`house.ts`/`table.ts` for an operator delete, `Chat.tsx`. `corpus: yes (new payloads and walk rows) · types.ts/contract.ts: yes · pool fixtures: yes (rows through the objects) · e2e: likely`.
- **Covered by:** `tables.rs:4351,4397,4561,4599,4663,4905`; `lib.rs:7010,7112`; `signal.rs:222,297`; `worker/test/operator.test.ts`; `worker/src/admin.unit.test.ts`.
- **Risk/open:** delivery is a COUNT: `Events.chat` is the document's `last` (`api.rs:1611-1618`) and the house frame is `{seq, chat}`. A delete moves neither, so a client that already fetched the message never learns; it needs a second counter or a re-read rule. On the NAS nobody owns the house chat, and only the author or a table's creator could delete.
- **Premise:** true at HEAD.

### Row 138 — live runs leave permanent lobby chat posts
- **Where:** `web/src/wire-script/walk.json:467-479` (red@wire.test posts "anyone up for a second table?", fixture `house_chat_post`), replayed live by `worker/scripts/live-replay.mjs` (`HOUSE_CHAT` `:88`, `normalizeHouseChat` `:189-211`); `web/e2e/exit_game.spec.ts:327-335` (x and y each post; stated `:70-75`). Recorded `worker/README.md:1735-1736`.
- **Now:** "a PERMANENT lobby chat post per replay run … and two per exit run, which real people see until 500 newer messages trim them".
- **Fix touches:** rides on row 82: a delete the runs call after themselves (the replay already forces tables as the operator over WARP, `live-replay.mjs:16-21,326-347`), or the live replay skipping its house post; `exit_game.spec.ts`; `worker/README.md` §10.4. `corpus: yes if the walk gains a delete row · pool: yes · e2e: yes (exit)`.
- **Covered by:** `worker/scripts/live-replay.test.mjs` (the house-chat normalization cells); the pool's corpus replay `worker/test/corpus.test.ts`.
- **Risk/open:** an author delete runs through the credential path, an operator delete through WARP; the two tiers use different ones.
- **Premise:** true at HEAD.

### Row 81 — the untrusted record reaches setup; no route size cap
- **Where:** `crates/yokohama-web/src/backup.rs:187-240` `check_pair_with`: `replay_strict` at `:229` runs BEFORE the table's `check_shape`/`agrees_with` (`:232-236`), which are what bound `players`, `colors` and `start_seat` against the record (`table.rs:341-358`). Route `api.rs:1997` (doc `:1973-1995`). Core bounds `player_count` by `debug_assert!` only (`crates/yokohama-core/src/types/state.rs:27`). Body cap: none per route. NAS `lib.rs:738` `to_bytes(body, 2 << 20)` for every route (doc `:731-734`); Worker `worker/src/index.ts:25-31,124-126,193-215` `BODY_LIMIT` 2 MiB; the Archive object re-reads at `worker/src/archive.ts:79`.
- **Now:** `let state = yokohama_core::replay_strict(&rec, rules)…?;` then `serde_json::from_str(table)`, `check_shape(&doc)`, `doc.agrees_with(&rec)`.
- **Fix touches:** `backup.rs` (a config pre-check, or the table checks moved ahead of the replay), `api.rs` (a route cap before the parse), `worker/src/index.ts` if the Worker caps too. A restore uses the same validator (`backup.rs:337`). `corpus: maybe (archive refusal rows in archive-walk.json if a sentence moves) · types.ts/contract.ts: no · pool fixtures: only if the corpus moves · e2e: no`.
- **Covered by:** `api.rs:2605` `the_archive_route_keeps_its_statuses_sentences_and_refusal_order` (pairwise order: a reorder moves it); `backup.rs:460,470`; the Worker's bound `worker/test/worker.test.ts:198-219`. No NAS cell reads the 2 MiB sentence.
- **Risk/open:** on the Worker a panic in the replay traps the Archive object's module, not only a task. A real finished 2P record is 54,103 B (`crates/yokohama-web/tests/fixtures/browser-played-2p.json`), so a cap far under 2 MiB has room.
- **Premise:** true at HEAD.

## Task 8 — the R2 backoff and the held-request arm

### Row 95 — An R2 outage costs each request one attempt and one failure batch
- **Where:** `worker/src/table.ts:1608` (`archiving` set on EVERY commit while a save is owed, the no-write early return at `:1609` included); `:1244` (`finish` → `waitUntil(deliverArchive())`; same at `:1112`, `:1167`); `:501-517` `deliverArchive` (dedups only an upload already in flight, `this.uploads`); `:600-602` failed → `retryArchive`; `:691-701` `retryArchive` (the failure batch: `attempt + 1`, alarm `retryAt`); `:707-710` `retryAt`; `:721-724` `archiveRung`, re-armed from now by every writing commit (`:1606`); ladder `worker/src/slot.ts:93-95`; `ArchiveOwed` `worker/src/r2.ts:38`; `executePlan` `r2.ts:123-146`.
- **Now:** `const archiving = owes && this.archiveOn();` (table.ts:1608) · `return [30_000, 120_000, 600_000][attempt] ?? 3_600_000;` (slot.ts:94) — nothing stops a request re-attempting before the rung, so the rung climbs per request.
- **Fix touches:** table.ts (a not-before gate on the request path of `deliverArchive`, keeping the game-over first attempt and the alarm path); r2.ts `ArchiveOwed` if a due time is stored (`archiveOwedOf`, table.ts:2093-2096, is an unchecked `JSON.parse … as ArchiveOwed`, so a stored row without it must default); a new pool cell in `worker/test/archive.test.ts`. `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Covered by:** `archive.test.ts:481` "a failing put is retried by the alarm, on the backoff, and lands"; `:949` "a stamp the layer refuses is logged and retried on the backoff…"; `:1134` "an owed save is kept, not retried, while off (battery 27), and resumes at the next request once on"; `slot.unit.test.ts:97-100` (`nextRetry`). No cell drives a REQUEST during an outage: the per-request cost is unpinned.
- **Risk/open:** `:1134`'s "resumes at the next request" premise moves under a gate. The stamp-refused retry (`settledArm` "retry", table.ts:638-646) rides the same owed row. R-6.5c2.30 allows "its cost stated" instead.
- **Premise:** true at HEAD — table.ts:1608, :1244.

### Row 86 — The held arm is dark with a residual gap; `EventsQuery.after` is inert
- **Where:** `crates/yokohama-web/src/api.rs:1621-1653` (`EventsQuery { after, wait }`, "VALIDATED AND NOT ACTED ON"), `:1663` `MAX_WAIT_SECONDS`, `:1667-1674` `check_wait`; NAS handler `lib.rs:580-605` (holds on `rx.changed()`, never reads `q.after`); `SIGNAL_KIND = "sse"` `lib.rs:280`. Worker `crates/yokohama-worker/src/events.rs:7-16,47-65` ("`wait` is validated and never held"), host `"ws"` at `crates/yokohama-worker/src/lib.rs:144`; wasm host `"none"` `crates/yokohama-wasm/src/lib.rs:110`. No host reports `"held"`. Client: `web/src/signal.ts:61-88` (`HELD_REASK_MS`, the residual at `:76-83`, `HELD_WAIT_S`), the arm `:408-430`, `once: () => api.events(game, undefined, HELD_WAIT_S)` `:571` (`after` never sent); `api.ts:998-1012`, `:340-342`; `"held"` in the unions at `types.ts:484`, `local/facts.ts:35,52,147`.
- **Now:** `pub after: Option<usize>,` (api.rs:1638), read nowhere; lib.rs:592-603 holds `wait`, then answers `events_at`.
- **Fix touches:** a decision — (a) retire the arm: api.rs, lib.rs:580-605, events.rs, signal.ts, api.ts, types.ts, facts.ts and their cells; or (b) close the residual: answer at once when `after` differs from the count (lib.rs:592, events.rs:58) and send `after` from signal.ts:571. `corpus: no (the walk's three `…/events` rows carry no query, walk-host.json) · types.ts/contract.ts: yes under (a) · pool fixtures: no · e2e: no`
- **Covered by:** `lib.rs:6464` `events_can_be_held_until_a_bump_and_the_wait_is_bounded`; `lib.rs:2756` `events_answers_in_five_fields_at_every_status_and_fences_its_query` (`after=-1`/`abc` 400s); `api.rs:2328` `a_wait_above_the_cap_is_refused_never_clamped`; Worker `lib.rs:1231` `events_refuses_in_axums_handler_order`; pool `sockets.test.ts:673` "the transport routes answer as axum's router does" (`wait=25`/`26`); `signal.test.ts:643-704` (three held cells).
- **Risk/open:** the socket arm's close PROBE calls the same `once()` (`signal.ts:457`, so `wait=25` reaches the Worker), and `EventsQuery` is `deny_unknown_fields` (api.rs:1635): dropping `wait` (or `after`) turns an open older release's probe into a 400. Keep both keys parsed under (a).
- **Premise:** true at HEAD — api.rs:1621, signal.ts:76-83.

## Task 9 — the Worker refactors under budget

### Row 127 — the table.ts split, four script dedups, the replay split
- **Premise:** true at HEAD; none done. `worker/src/table.ts` is 2,151 lines (the spec read 2,082, `2026-09-26-web-frontend-m6.5c2-spec.md:776`).
- **table.ts shape:** header and types `:1-155`; `class Table` `:156-1860`; free helpers `:1872-2151`.
  - Request path: `fetch` 182-224, `ready` 747-781, `answer` 782-841, `finish` 1240-1275, `wake` 1276-1337, `call` 1338-1380, `commit` 1555-1655, `owed` 1656-1684, `owedWrites` 1685-1744, `copyOf` 1745-1777, `batch` 1778-1792, `arm` 1793-1804, `pinned` 1847-1855, `reading` 1856-1859.
  - Alarms: `alarm` 225-251, `onAlarm` 252-273, `markerAlarm` 274-305, `end` 306-328, the due-times 1805-1846.
  - Slot to house (the spec's `delivery.ts`): `deliverSlot` 329-347, `push` 348-401, `acknowledged` 402-434, `failed` 435-467, `recopy` 468-500; helpers 1938-2019.
  - Archive: `deliverArchive` 501-521, `upload` 522-595, `settleUpload` 596-668, `stamp` 669-690, `retryArchive` 691-706, `retryAt` 707-712, `archiveOn` 713-720, `archiveRung` 721-734; `owing`/`archiveOwedOf` 2077-2098.
  - Delete, expire, operator: `ended` 735-746, `abandon` 842-929, `deleting` 930-990, `redispatch` 991-1029, `expire` 1030-1087, `expiry` 1088-1127, `unexpired` 1128-1159, `operate` 1160-1203, `force` 1204-1239; `copyFacts` 2117-2151.
  - Sockets: `admit` 1381-1431, `sendFrames` 1432-1476, `closeDeleted` 1477-1487, `endSockets` 1488-1501, `webSocketMessage` 1502; `requestedId`/`transportOf`/`isUpgrade` 1872-1897.
  - Guarded-call copies (a pinned reading plus the documents read back): `:671` stamp, `:933` deleting, `:1001` redispatch, `:1341` call. Four at HEAD; the spec counted three.
- **The four dedups** (spec `:82-86`, `:807-815`):
  1. `.dev.vars` parser, two copies: `worker/scripts/live-replay.mjs:70-76,244-262` (`readCredential`) and `web/e2e/live.ts:105-136` (`credential`). Cells `worker/scripts/live-replay.test.mjs:215-238`, `web/e2e/harness.node.test.ts:189`. Target `worker/scripts/dev-vars.mjs` plus `dev-vars.d.mts` (TS7016).
  2. The origin: `live-replay.mjs:60` `ORIGIN`, `web/e2e/live.ts:54` `LIVE_ORIGIN` (route pattern `worker/wrangler.jsonc:18`).
  3. `HOST_FIELDS` `live-replay.mjs:83` vs `worker/test/corpus.test.ts:34-38` (the browser replay's `web/src/local/corpus.node.test.ts:103-107` differs by host); `order` `live-replay.mjs:286-296` vs `worker/test/walk.ts:88`; key names `live-replay.mjs:76` vs `web/e2e/live.ts:107`.
  4. Precache-list parser: `worker/scripts/assemble-assets.mjs:35,43-57` (`PAIR`, `precacheEntriesFrom`) vs `worker/scripts/read-deploy.mjs:50-66` (`ENTRY`, `precacheList`, which also reads `type`).
- **live-replay.mjs (658 lines):** header `:1-57`; constants `:60-107`; pure exemptions `freezeClock` 119 to `firstSendAccepts` 228-243; loading `readCredential` 244-262, `loadScripts` 265, `loadCorpus` 271, `order` 286-296, `urlOf` 297-303, `tableIds` 304; network `send` 310-325, `forceAll` 326-347; reporting `redacted` 352 to `judge` 421-489; driver `replay` 490-619; `finish` 620-640; main 642-658. The five-part cut: credential and config, scripts and order, exemptions, send and force, judge and report, with `replay` their one importer.
- **Fix touches:** `worker/src/table.ts` plus new modules, `worker/scripts/*.mjs`, `web/e2e/live.ts`. `corpus: no · types.ts/contract.ts: no · pool fixtures: no (the fence is the pool's 2,209-row corpus replay, worker/test/corpus.test.ts) · e2e: live.ts only`.
- **Covered by:** pool `worker/test/{table,sockets,archive,expiry,operator,slot,reads,corpus}.test.ts`; `worker/scripts/live-replay.test.mjs`, `read-deploy.test.mjs`, `assemble-assets.test.mjs`; `web/e2e/harness.node.test.ts`, `live.node.test.ts`.
- **Risk/open:** the `.dev.vars` module crosses npm packages (`worker/` and `web/`), which spec review m10 left for the plan to confirm. `web/e2e/live.node.test.ts` reads every e2e helper through the compiler, so moving the reader moves what it scans.

## Task 10 — the layer's test debts and the livelock run

### Row 31 — The acting-seat fence has no whole-game witness; the persist CAS arm is uncovered
- **Where:** fence `crates/yokohama-web/src/api.rs:1165-1171`; gate arm `crates/yokohama-web/tests/play_a_whole_game.rs:549` `two_people_play_to_a_winner_and_each_sees_only_their_own_hand` (posts only as the acting seat); `persist`'s conflict arm `games.rs:801-807`, stated undriven at `games.rs:795-800`.
- **Now:** "**NO CELL DRIVES THE CONFLICT ARM** … the only way to reach it is a SECOND PROCESS" (games.rs:795-798).
- **Fix touches:** play_a_whole_game.rs (each turn the non-acting identity posts once and reads the 403 "it is seat N's turn (…)"); games.rs tests: a store double whose `write_if_version(Doc::Record, …)` returns `WriteError::Conflict` (the `FailingWrites` pattern of `games.rs:2038`, which returns `Io`) to drive the arm and assert the eviction. `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Covered by:** fence — `lib.rs:2815` `input_is_the_acting_seats_holders_and_names_whose_turn_it_is`, `:2855` `undo_is_admitted_mid_turn_and_refused_once_the_turn_has_passed`, `:3178` `a_finished_or_locked_game_refuses_every_caller_the_same_way`; persist — `games.rs:1718` `an_undo_reaches_the_store`, `:1837` `a_stale_handle_cannot_erase_a_move_another_writer_already_stored`, `:2038` `a_write_the_store_refuses_bumps_nothing`; store CAS — `store.rs:549` `a_create_refuses_a_present_document_and_a_stale_version_conflicts`.
- **Risk/open:** the gate probe adds one request per turn to a suite read at 111 s in debug (c-close:133). The `persist` doc's sentence must change with the cell.
- **Premise:** true at HEAD.

### Row 32 — No bot-table deal fixture; no open table with two open seats
- **Where:** `web/src/wire/deal.json`, `deal_waiting.json` (the human table `wire`); `table_bot.json` (a bot table at `dealing`, TABLE view only, `wire_fixtures.rs:1776-1784`); `table_open.json` (2P, one open seat); `games_open.json` (two tables, one open seat each). Emission `wire_fixtures.rs:1098-1113`; `bot_table_body` `:1869`.
- **Now:** "The corpus contains no deal at a bot table" (m6.1-close:738); "`table_open.json` is a 2P fixture" (m6.2-close:596).
- **Fix touches:** wire_fixtures.rs (a `GET …/deal` on the `bots` table; a 3P/4P create with two open seats); `web/src/wire/contract.ts` + `contract.test.ts:175`'s key-set rows; the non-walk exact sets `web/src/local/corpus.node.test.ts:142-171` and `worker/test/corpus.test.ts:99-116`. `corpus: yes · types.ts/contract.ts: contract yes · pool fixtures: yes (the corpus list) · e2e: no`
- **Covered by:** `tables.rs:2061` `the_deal_serves_the_callers_own_cards_and_never_the_others`; `tables.rs:2112` `the_set_table_is_drawn_for_a_human_seat_and_never_the_bots`; `table.rs:814`; `tables.rs:3269` `create_lands_open_when_any_seat_is_open_and_dealing_otherwise`.
- **Risk/open:** a `/deal` before finalize is deterministic (fixed create seed; the bot's stream seed is drawn later). Every leakage pin scans the new files.
- **Premise:** true at HEAD, narrowed: `table_bot.json` now shows a bot table at `dealing`.

### Row 39 — M6.3–M6.4 test minors (bundle; m6.3-close:755-825, m6.4-close:835-875)
- `/stream` in no route sweep — `lib.rs:2301-2323` `session_routes()` lacks `/stream` (and `/unlock`, `/record`); nothing ties it to the router `lib.rs:886-887`; admission covered per cell by `stream_status` (`lib.rs:3831-3841`) in `the_whole_admission_table_is_answered_route_by_route` (`:3679`); the multiplayer-off sweep names it by literal (`lib.rs:5070`) — premise true.
- R-pre-3 unasserted — `tables.rs:1134-1136` "It bumps no signal"; the unlock cells `tables.rs:4974,5059,5091`, `lib.rs:6518,6619` read no receiver — premise true; fix: subscribe, unlock, assert `!rx.has_changed()`.
- no held "wake" cell — `web/src/signal.test.ts:644` "re-asks a WAKE source after the named wait, and never opens the stream" drives `wakeSource` under `setSignalKind("held")` — premise FALSE at HEAD.
- 34 rem tied to `--side` — `web/src/styles.css:1471-1478` `@container (max-width: 34rem)` against `--side: 36rem` at ≥1900 px (`:273`; the comment's "514px at 1900"); nothing pins the coupling; `joins_by_link.spec.ts:194-200` reads text against its row only — premise true.

### Row 40 — M6.5a test minors (bundle; m6.5a-close:441-472)
- NoSignal premise — `games.rs:1073` `every_bump_site_answers_under_no_signal` builds `games()` (`games.rs:2624-2626`, `Games::new`'s default), stated un-assertable at `games.rs:2617-2621`; fix: `Games::with_parts(.., Arc::new(NoSignal))` (`games.rs:410`). Its doc's "the only … the nearest other reaches three" (`games.rs:1063-1066`) is still singular — premise true. The "every cell uses `watched_games`" universal is already scoped to "THIS module" (`games.rs:2621-2623`) — that half FALSE at HEAD.
- The clock cell — `tables.rs:2013` `a_table_and_a_message_are_stamped_by_the_clock_the_games_was_built_with` uses `games().with_clock(Arc::new(|| 1_700_000_000))` (`:2014`), not `with_parts`; `freeze_clock` accepts only `seen > 1_700_000_000` (`wire_fixtures.rs:533,548`) — premise true.
- Oversize-body sentence — the adapter `lib.rs:738-742`: `to_bytes(body, 2 << 20)` → 400 "the body could not be read: {e}", where axum's `JsonBody` read "Failed to buffer the request body: length limit exceeded"; no cell pins either — premise true (the Worker's archive cap is Task 7's).
- Region-marker blind spot (M8) — closed by the M6.5a final wave (IMP-2): `lib.rs:7137-7141` "inside a region every column-0 item must carry its own attribute, or the cell names it"; cell `platform_calls_stay_behind_the_server_feature` `lib.rs:7146`; residual only the top-level test module (`lib.rs:51`) — premise FALSE at HEAD.

### Row 33 — `VS_BOT_CAP` 5,000 rests on one run of 50 seeds
- **Where:** `crates/yokohama-web/tests/play_a_whole_game.rs:270-277` (the measured doc, `const VS_BOT_CAP: usize = 5_000;`), assert `:336`; `bot_stream_seed` `:238-268` (drawn from OS entropy at finalize, read off `record.run.seats`; `YOKOHAMA_WEB_GATE_BOT_SEED` pins by rewriting the record, then `api.restarted()`); `eprintln!` `gate: start bot_seed=` `:315` and `gate: done bot_seed= inputs= events= turns= selections= backed_out=` `:368-371`; CI `.github/workflows/test.yml:275-289` (`timeout-minutes: 20`, `--nocapture`).
- **Now:** "50 runs over OS-entropy bot seeds … inputs min 136, median 386 … max 1,212" (`:270-273`).
- **Fix touches:** no code for the run itself: the A plan's loop (`docs/plans/2026-09-27-web-frontend-m6.5c2a-plan.md:7418-7433`) × 50 batches, filtered to `a_whole_two_player_game_against_the_bot`, unpinned (records each `bot_seed`) or `YOKOHAMA_WEB_GATE_BOT_SEED=$s` over a list; then the `:270-277` doc (and the cap, if the max moves) in one commit. `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Covered by:** the gate itself (`:280`); a stuck-rule mutant reached the cap in 8 s (b-handover:102-105).
- **Risk/open:** the per-seed wall is not in the tree (the A plan sized ten runs per ten-minute batch), so 500 seeds is hours in debug; a release-profile run shortens it. A failed seed is a finding, not a retry.
- **Premise:** true at HEAD.

## Task 11 — the client's defects by screen

### Row 1 — Dealing's "waiting on" line never refreshes at 3P+
- **Where:** web/src/Dealing.tsx:212-224 (the signal handler), :295-298 and :432 (the line), :173-188 (`fetchDeal`, run at mount and after this browser's own keep, :257); the bump is sent at crates/yokohama-web/src/tables.rs:1523-1527
- **Now:** `setChatCount(e.chat); if (e.status !== "dealing") void start();` — a `dealing` frame from another seat's keep is dropped.
- **Fix touches:** Dealing.tsx only (call `fetchDeal()` on a `dealing` frame, skipping frames where only `chat` moved; a chat post also bumps the table at tables.rs:971) · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** Dealing.test.tsx:148 ("opens the game when the signal says playing"), :479 ("names the seats it is waiting on…"); no cell sends a `dealing` frame.
- **Risk/open:** `Events` (web/src/types.ts:390-403) carries no keep count, so `chat` is the only way to tell a keep from a post; each refetch is one more Worker request.
- **Premise:** true at HEAD (M6.3 §6.1 and M6.4 §6.1 re-checked it the same way).

### Row 2 — The Game menu stays open after a switch
- **Where:** web/src/GameMenu.tsx:112-124 (`switchTo`), :127-132 (an uncontrolled `<details>`); App.tsx:1866-1877 (`onSwitch` → `open`)
- **Now:** nothing closes the `<details>` when `onSwitch` resolves; a playing→playing switch keeps the banner mounted. A switch to an open or dealing table unmounts it, so that half closes.
- **Fix touches:** GameMenu.tsx · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: YES
- **Covered by:** GameMenu.test.tsx:22 (its comment :36-43 says jsdom cannot see openness); web/e2e/the_table.spec.ts:160-180.
- **Risk/open:** that e2e cell DEPENDS on the defect: ":169-177 … The menu stays OPEN across a switch … a second summary click would CLOSE it". It must reopen the summary after the fix.
- **Premise:** true at HEAD.

### Row 3 — Sign-out shows the raw 401 sentence above the door
- **Where:** web/src/whoami.ts:103-120 (`leave`, `signOut`, `signedOut`); App.tsx:170-173 (`onUnauthorized(signedOut)`); SignIn.tsx:61-65 (renders `notice`); Door.tsx:131-132
- **Now:** `signOut: useCallback(() => leave(null), [leave])` clears the notice and re-asks `/api/me`, but `me` keeps the old user until that answer; a 401 landing in the window calls `signedOut(sentence)` and the sentence reaches the name step.
- **Fix touches:** whoami.ts (ignore a 401-driven `signedOut` while an asked-for sign-out's re-read is pending) · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** whoami.test.ts:77 ("says nothing when the player signs out on purpose": a PRIOR notice only); GameMenu.test.tsx:161-163 finds the menu's "sign out" and never clicks it.
- **Risk/open:** the M6.1-era trigger (a route-effect `open` after `me` flipped) is now guarded at App.tsx:743, and the lobby stops asking at `me === null` (Lobby.tsx:234), so one headed click decides it.
- **Premise:** unverified at HEAD. The window exists by reading; the observed trigger may be gone.

### Row 4 — One scope's failure blanks all three lobby lists
- **Where:** web/src/Lobby.tsx:66-73 (`fetchLists`), :233-241 (opening read), :244-248 (poll), :440-451 (`lists === null`)
- **Now:** `const [mine, open, playing] = await Promise.all([api.list("mine"), api.list("open"), api.list("playing")]);`
- **Fix touches:** Lobby.tsx (`allSettled`, one error per section) · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** Lobby.test.tsx:506 ("shows a refusal rather than an empty lobby"), :769, :801
- **Premise:** true for the FIRST read only. A later failure keeps the previous `lists` and sets only the strip (`take` at :215-223 is the only writer).

### Row 17 — M6.2 menu minors (four)
- No Abandon for a seatless creator — OpenTable.tsx:365 — now: `{seated && table.created_by === me && (<Abandon …/>)}` — true. The lobby row still offers it; the `created_by === me` compare is also Task 6's.
- A failed `open` from the Game menu — App.tsx:585 (`navigate`) before :595 (`api.table`); GameMenu.tsx:118-119 shows the refusal — the hash then names the refused table while the old game stays drawn, and nothing restores it; a later route-effect run re-`open`s and calls `toLobby` (App.tsx:759-782) — code path intact, runtime unverified.
- Back during an in-flight `open` — App.tsx:595-665 — no check after the awaits (:595, :645) that the route still names `id` before `show(next, opening)` at :665 — true by reading.
- The table read twice a visit — App.tsx:595 then OpenTable.tsx:129-137's opening `api.table(game)`; the doc is not passed down — true.
- Fix touches App.tsx and OpenTable.tsx; corpus, contract and e2e: no. Covered by OpenTable.test.tsx:511, :533 (Abandon strips) and GameMenu.test.tsx:172 (double click).

### Row 38 — M6.2 test minors (four)
- Scope matchers — web/src/test/harness.tsx:50-54 (`isGamesList`) exists, but 20 path-only `/api/games` matchers remain and 6 `split("?")` sit on POST matchers (App.test.tsx:1333, :1675, :2150, :2452; NewGame.test.tsx:36) — true.
- `freeze_clock`'s name-keyed `next` arm — crates/yokohama-web/tests/wire_fixtures.rs:538 `} else if key == "next" {` — true. A shape-keyed rewrite must leave the corpus byte-identical.
- Password needles without partners — wire_fixtures.rs:2758-2761 ("Neither password has such a partner and neither can have one"), needles at :72, :95 — true. A partner exists: both values are in the request script web/src/wire-script/walk.json.
- OpenTable strings — "Waiting for N more" (OpenTable.test.tsx:403), "password required" (:140) and a whitespace name (:191) — MOOT, covered. `join_403_locked` has contract cells only (web/src/wire/contract.test.ts:101, :607) — true.

### Row 55 — M6.1–M6.4 comment and naming minors
- `KeepRequest` unread — web/src/types.ts:630 — declared, referenced nowhere, not in contract.ts — true.
- `.deal-pick.chosen` dead CSS — web/src/styles.css:1026 — Dealing.tsx:404 never adds `chosen` — true.
- `usePolled` named for a poll — web/src/signal.ts:216 — 38 references in 10 files, plus CLAUDE.md — true; a rename edits CLAUDE.md.
- `reread()`'s non-null assertion — signal.ts:296 `await src.current!.once()` — true.
- `ChatConfig`'s two public fields undocumented — crates/yokohama-web/src/chat.rs:131-134 — true.
- `api.health().catch(() => {})` silent — App.tsx:928-930 now releases a latch — MOOT. `Lobby.test.tsx:26`'s comment — rewritten (:21-27) — MOOT. "App.tsx is ~1,010 lines" — now 2,300.

## Task 12 — the local host and backup

### Row 12 — "Back up everything" hangs while an old tab holds IndexedDB v1
- **Where:** web/src/backup.ts:273-280 (`withDb` → `openDb()` with no `onBlocked`), :282; BackupControls.tsx:96-108 (the `busy` ref); outbox.ts:209; local/db.ts:180-207 (`onblocked` only reports)
- **Now:** `const db = await openDb();` — a blocked upgrade never settles, so `busy.current` stays true and the button answers nothing.
- **Fix touches:** backup.ts (an `onBlocked` or a bound), BackupControls.tsx (the sentence), outbox.ts:209 · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** local/db.test.ts:223 ("says what is blocking it, and recovers when that tab goes": the Worker's open); no page-side cell.
- **Risk/open:** only fake-indexeddb can stage a v1 holder (row 27).
- **Premise:** true at HEAD.

### Row 13 — `backupEverything` walks the store's ids, not the module's
- **Where:** web/src/backup.ts:282-308 (`readAll(db)` at :285 → `byId`, the loop at :298); `backupOne` :318-327 asks the Worker through `pairOf`
- **Now:** `const rows = await readAll(db); … for (const [id, stored] of byId) {`
- **Fix touches:** backup.ts (union the module's listing, `api.list("mine", undefined, "local")`, with the stored ids) · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** backup.test.ts:125, :146, :162, :190
- **Premise:** true at HEAD. A quota-refused create is listed from the module (OnThisDevice.tsx:62-63) and missing from the whole-device file.

### Row 14 — Storage reads outside the export arm still trap
- **Where:** web/src/local/worker.ts:40-44 (rule 6, "named here, not fixed"), :138 (`ready`), :151 (`rebuild`), :615-616 (`adopt`)
- **Now:** "a generation or a read that fails during a hold's acquisition (`takeGeneration`, `rebuild`), during the startup `ready` runs, or in `adopt` after a restore is answered `trapped`, and the page restarts the host."
- **Fix touches:** local/worker.ts, local/client.ts (the `hold` and `adopt` replies), local/useLocalHold.ts, backup.ts (restore's adopt) · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: possibly (plays_locally's trap cell)
- **Covered by:** local/exportPair.test.ts:153-180 (the export arm's split, the pattern to copy); local/client.test.ts:243, :352 (restart on a trap)
- **Risk/open:** worker.ts has no unit harness of its own; an "unreadable" answer to `hold` needs a lock state the screens can draw.
- **Premise:** true at HEAD.

### Row 15 — A newer envelope meets serde's unknown-field sentence
- **Where:** crates/yokohama-web/src/backup.rs:355-368 (`restore_read`), :62-68 (`#[serde(deny_unknown_fields)]` on `Envelope`)
- **Now:** the whole `Envelope` is parsed before `if env.envelope != ENVELOPE_VERSION` (:363), so a v2 file that adds a key fails first with "this is not a backup envelope this build reads: unknown field …".
- **Fix touches:** backup.rs (a version-first head parse), then the browser module rebuilt · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** backup.rs:513 `an_envelope_of_another_version_is_refused_whole` (changes the number only), :522-540; the client stubs the sentence (BackupControls.test.tsx:184, backup.test.ts:524)
- **Premise:** true at HEAD.

### Row 16 — A compatibility row outlives its reason after a rev bump
- **Where:** web/src/local/db.ts:412-423 (the note on `restoreOne`, :447); backup.ts:498-501 (`unplayableIds` trusts every row); OnThisDevice.tsx:58, :101-105
- **Now:** "It is deliberately NOT done now — today no rev N+1 record exists to restore, so there is no way to reach the state and no way to test the fix."
- **Fix touches:** local/db.ts (drop the row on an `already-here` whose verdict is not `unplayable`), or backup.ts:498 (re-derive against the module's `/api/health.semantics_rev`) · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** backup.test.ts:328, :359, :385, :401
- **Risk/open:** testable now: a compat row that names the CURRENT rev (7) stages the stale state, so the note's "no way to test" is too strong.
- **Premise:** true at HEAD.

### Row 56 — M6.5b's deferred minors, Tasks 2–8 (by task)
- **Task 2, the archive route:** check order unpinned (by two-defect probes); marker read-then-write not atomic (crates/yokohama-web/src/archive.rs:294); `Failed` carries absolute paths (archive.rs:72, :260, :264); abandon's conflict wording; no seat or ownership check (by design) — true.
- **Task 3, the wasm host:** `doc_of` after init's `expect` (crates/yokohama-wasm/src/lib.rs:206); `set_clock` saturates a negative or NaN to 0 (lib.rs:243-244, `s as u64`) — true. "Nothing in CI builds the self-host image" — MOOT (docker.yml since cycle C). The install layer and the dockerignore wording were not re-located.
- **Task 4, the local host:** acquisition failure posted as a trap (row 14); restart sentence before re-acquisition (local/client.ts:242, by design); `ask`'s `pending` entry leaks if `postMessage` throws (client.ts:255-271); `takeover` ignores `held` and adds unconditionally (client.ts:458-460); lost-handling written twice (client.ts:152, :329); two dead switch arms (local/worker.ts:645-647); `withUnsaved` drops the warning when the reply has no `warnings` array (web/src/api.ts:617-626) — true. Test-only `readGeneration` — MOOT (production use at worker.ts:568). No-Web-Locks fallback is row 103.
- **Task 5, the first screen:** `reached.current` assigned in render (Lobby.tsx:213) — true; the labels bound's timer — MOOT (api.ts:303-319 clears it), but the labels effect still has no cleanup (App.tsx:852-856); the `Lobby`↔`OnThisDevice` cycle — MOOT (Lobby.tsx imports neither); the local form's seating stated twice and the over-claiming comments — B rewrote NewGame.tsx, not re-checked.
- **Task 6, the local game screen:** any local 5xx → the broken-save screen (App.tsx:647-652; row 99); `.decision.locked-out` duplicates `.decision.walking` (styles.css:1672-1694); the hook releases an id it never held (local/useLocalHold.ts:88); `take`'s `setAnswer` has no `live` guard (useLocalHold.ts:92-96); a local game's Game menu lists the SERVER's games in a multiplayer build (App.tsx:1895) — true.
- **Task 7, the corpus replay:** `hosts_own`'s stated reason (wire_fixtures.rs:178, :233); a bare `SyntaxError` on a non-JSON reply (web/src/local/corpus.node.test.ts:222); the drift message names no offset (wire_fixtures.rs:3176-3212) — true. The by-value binding and the double execution are ledger items, not located.
- **Task 8, the e2e and the pin:** the restart gate stays raised if `start()` throws inside `ask` (client.ts:246: `ids.map` throws before `Promise.all`); `CAP = 4000` is now FOUR copies (web/e2e/offline.spec.ts:89, plays_a_whole_game.spec.ts:43, plays_locally.spec.ts:43, two_browsers.spec.ts:84); the `not_a_move` arm (crates/yokohama-web/tests/browser_played_pin.rs:299) — true. The debug module left in `web/dist` is by design.

### Row 100 — A refusing archive; an unreadable IndexedDB
- **Where:** App.tsx:1524-1529 (`local={local && broken.foreign}`); Abandon.tsx:119-142 (`sendFirst`), :171-181; BackupControls.tsx:370-377 → backup.ts:528-538 (`removeForeign`); OnThisDevice.tsx:60-73, :101
- **Now:** under a "send" disposition a non-404 refusal returns false (`if (disposition === "send" && !(await sendFirst())) return;`, :181); "Abandon without saving" is offered only in the `ask` state (:178).
- **Fix touches:** Abandon.tsx (offer the without-saving choice after a refused send), OnThisDevice.tsx · corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no
- **Covered by:** Abandon.test.tsx:322, :335 (keeps the game on a refusal: the fix must keep that first answer), BackupControls.test.tsx:237
- **Premise:** partly true. The game screen's Abandon cannot delete it, but the door's "Remove" deletes without sending when the row is listed. Foreign rows come from the module's listing (OnThisDevice.tsx:62-63), so a store that stops the Worker hides them with every other row.

### Row 24 — Does "First player: The bot" reach the parked cold-pump clause?
- **Where:** web/src/NewGame.tsx:457-470 (`start_seat` 1 is the bot), :525-532; crates/yokohama-web/src/tables.rs:1227 (`bring_alive`) before :1232-1234 (the record-first write); games.rs:587-591 (the cold-pump arm in `live_handle`)
- **Now:** `bring_alive(&mut repl); … let json = repl.session.to_record_json()…; self.store.write_if_version(Doc::Record, &id, &json, None)` — the bot's opening run is played before the first record is written, and every `input` persists after its own pump.
- **Fix touches:** none on a production path. A cell could reach the arm through a RESTORED pair whose record ends at the bot's turn (restore validates by strict replay only).
- **Covered by:** web/e2e/plays_locally.spec.ts:393 (the reload cell, clause parked); the M6.5b close :387-391 and :489 hold the ruling.
- **Premise:** FALSE at HEAD. A bot-first local game still stores its first record at the human's turn.

## Task 13 — the client's test debts

### Row 18 — `plays_locally`'s quota cell: the clicked option may open a sub-picker
- **Where:** `web/e2e/plays_locally.spec.ts:476-545` (the cell); the gap `:501-507`; the sibling's proven loop `:589-596`
- **Now:** `expect(screen.options, "the cell needs a move to make").toBeGreaterThan(0);` / `await page.locator("button.option:not([disabled])").first().click();` / `await expect(page.locator(".deck .warn")).toContainText("APPLIED BUT NOT SAVED");` — nothing between the click (:503) and the expect (:507) asserts a move APPLIED; `playUntilAnswerable` (:216-224) stops at ANY option row, a two-step choice's first step included (C close §2.1: the trace held "Assistant destinations from hand" at turn 1).
- **Fix touches:** `plays_locally.spec.ts` only — replace :501-503 with the sibling's loop (`act` until `unsaved.count() > 0`, 12 tries, :589-595). `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: yes`
- **Covered by:** sibling "a store that will not even be READ still exports…" `:555` (already loops); `playUntilCommitted` `:187-201` ("A click is not a move").
- **Risk/open:** seed-conditional (the local game's seed is OS entropy), so one green run proves nothing about the fix.
- **Premise:** true at HEAD — red at `ea4f174d` (C close :151 sequence row 26, §2.1) and in M6.5c's sequence (M6.5c close :391).

### Row 19 — two jsdom cells failed once in five runs under load 5.9
- **Where:** recorded only at `docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md:447-448`
- **Now:** "two jsdom cells that failed once in five runs under load 5.9 (a flake candidate)."
- **Fix touches:** unknown until named. `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Covered by:** n/a. The method that closed M6.5c's three (`e20c8ff8`: a `Surfaces.test.tsx` budget split, `Lobby.test.tsx` awaiting the listing's link, `App.test.tsx` `resetFactsForTest()` in `beforeEach`; shuffle seeds 22/44, three concurrent suites) is at M6.5c close :1213-1217.
- **Risk/open:** NO tracked document names the two cells (C close, C plan, D handover, spec searched); only C's gitignored ledger might. The task must start with a reading (`npm test` repeated under load).
- **Premise:** true as recorded; the cells are UNNAMED at HEAD.

### Row 20 — the browser host's replay lacks L-5's presence rule (R-C20)
- **Where:** `web/src/local/corpus.node.test.ts:231-235` (list `:103-107`); the pool's rule `worker/test/corpus.test.ts:64-73` (`hostFields`) and `:169-175`; the live twin `worker/scripts/live-replay.mjs:433-449`
- **Now:** `for (const field of HOST_FIELDS[row.fixture!] ?? []) { expect(got[field], …).not.toEqual(committed[field]); }` — `expect(undefined).not.toEqual(x)` passes, so a field the module stopped sending is exempted unread.
- **Fix touches:** `corpus.node.test.ts` (assert present, then differ); the pool's comment `worker/test/corpus.test.ts:55-62` that calls the browser replay "not a third statement". `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Covered by:** pool "hostFields — the presence rule, verdict by verdict (R3's L-5)" `worker/test/corpus.test.ts:125`; the hole's host "replays the walk through the module…" `corpus.node.test.ts:181` (runs under `npm run test:wasm`).
- **Risk/open:** the pool's rule is "SENT as a string"; the browser list exempts `multiplayer`, a BOOLEAN (`web/src/wire/health.json` `"multiplayer": true`), so a verbatim port fails — the browser rule must be "present and differs".
- **Premise:** true at HEAD.

### Row 23 — no cell holds 44 px on the open table and the device card
- **Where:** the rules `web/src/styles.css:2491-2527` (`.open-table-controls button.ghost`, `.invite .row button.ghost`, `.open-table > .chat button.ghost`, `.on-this-device button.ghost:not(.back-up-all)`, `.device-foot button.back-up-all`, `.device-foot .restore-field`); the lobby's sweep `web/e2e/setup_screens.spec.ts:340-362`, helper `height()` `:45-48`
- **Now:** C close :435-436 "no jsdom cell for the 44 px on the open table and the device card (the one-off log witnesses it)."
- **Fix touches:** an e2e height sweep at 390 over `.open-table` and `.on-this-device`, in `setup_screens.spec.ts`'s shape. `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: yes`
- **Covered by:** "Join as Blue" at 390 only (`joins_by_link.spec.ts:352`); the door's cards (`setup_screens.spec.ts:87-94`); the device foot's LAYOUT, not height (`offline.spec.ts:1229-1245`).
- **Risk/open:** the row names an impossible instrument — jsdom loads no `styles.css` (CLAUDE.md), so only Playwright can hold a px rule.
- **Premise:** true at HEAD (the gap is real; "jsdom" mis-states the instrument).

### Row 25 — six `sw.ts` decisions, Abandon's catch and the Worker's trap/refusal split lack a catcher
- The list: PR37 fixes §5.7 :187-197; `web/src/sw.ts:43-53` names them "pinned by NO cell". Per item:
  - whole-install deadline — `sw.ts:122` `setTimeout(() => deadline.abort(), INSTALL_DEADLINE_MS)` — true
  - per-fetch timeout — `sw.ts:181` `setTimeout(() => stop.abort(), FETCH_TIMEOUT_MS)` — true
  - `credentials: "same-origin"` — `sw.ts:192` — true
  - the stamp written first — `sw.ts:137-140` `await writeStamp(cache, {createdAt: …})` before the lanes — true
  - `includeUncontrolled` — `sw.ts:229` `self.clients.matchAll({ includeUncontrolled: true, type: "window" })` — true
  - "took too long" — `sw.ts:216` — true
  - (the abort-and-await before the delete, `sw.ts:164-165`: "aborts the entries still in flight when one fails" `sw.node.test.ts:296` passes by the 15-s timeout, not the abort; PR37 :197 names a fake-timer cell as the instrument)
  - `Abandon.tsx:212` — LINE DRIFT: `:211-212` is now `backUp`'s catch; `abandonAnyway`'s is `:226-227` `onError(e instanceof Error ? e.message : String(e))`. Neither catch has a cell: `Abandon.test.tsx:523` and `:559` are happy paths — true
  - the Worker's trap/refusal split — `web/src/local/worker.ts:233-237` (persist's catch → `saved: "unsaved"`) and `:576` `if (!(err instanceof StoreUnreadable)) throw err;` — e2e-only (`plays_locally.spec.ts:555`, `:641`, `:705`); `client.test.ts:404` drives the fake Worker and is NOT a catcher (PR37 :117) — true
- **Fix touches:** `sw.node.test.ts` (fake timers), `Abandon.test.tsx`, a unit harness for `worker.ts`. `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Risk/open:** `worker.ts` imports the wasm module, so a unit catcher needs it stubbed, or `test:wasm`'s built module.

### Row 26 — E2: an uncontrolled tab's reload on activation has no browser reading
- **Where:** `web/src/swRegister.ts:313-317`
- **Now:** `if (hostRef.current.controlled()) { asked.current = "controllerchange"; } else { asked.current = "activated"; lent.current?.reloadWhenActivated(waiting); }`
- **Fix touches:** `web/e2e/offline.spec.ts` (a cell whose tab is UNCONTROLLED when it clicks "Reload to use it"). `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: yes`
- **Covered by:** unit `swRegister.test.ts:477`, `:511`, `:535`. Every browser click of "Reload to use it" (`offline.spec.ts:506, 529, 581, 761`) is from a controlled tab (`:471` asserts it).
- **Risk/open:** the setup — a first visit (nothing calls `clients.claim()`) whose release is superseded before any reload.
- **Premise:** true at HEAD.

### Row 28 — the determinism pin was played by the DEBUG module
- **Where:** `crates/yokohama-web/tests/fixtures/browser-played-2p.provenance.json` (`"module_build": "debug — npm run build:wasm:debug …"`, cut 2026-09-21 at 0.5.0-m6.5b); producer `web/e2e/plays_locally.spec.ts:352-387`; consumer `crates/yokohama-web/tests/browser_played_pin.rs:40-42`
- **Now:** the sidecar says "the RELEASE module (`npm run build:wasm`) has the same bot and no `debug_trap`".
- **Fix touches:** a node cell in `test:wasm` replaying the pair's bot moves through `web/wasm-node/`, which ONLY `build:wasm` (the release module) writes (`web/package.json:8`); re-cutting the pair instead is blocked by `npm run e2e` always rebuilding debug (`:18`). `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: no`
- **Covered by:** native `the_browser_played_pair_reconstructs_under_the_native_why` `browser_played_pin.rs:242`.
- **Risk/open:** "one feature apart" understates it: debug = `cargo build --release --features debug`, no `wasm-opt` (`package.json:9`); release = `--profile release-wasm` + `wasm-opt -Oz` (`:8`).
- **Premise:** true at HEAD.

### Row 30 — `cancelPicker`, the walk's Cancel branch, had run in no e2e walk
- **Where:** `web/e2e/harness.ts:728-731`; callers `:670` (a picker opened pressed) and `:680` (an Option B that took no area)
- **Now:** `await page.locator(".decision .controls button.ghost", { hasText: "Cancel" }).first().click(); return "cancel";`
- **Fix touches:** a targeted e2e (press a tile, Cancel, assert the picker closed and nothing applied). `corpus: no · types.ts/contract.ts: no · pool fixtures: no · e2e: yes`
- **Covered by:** jsdom `Decision.test.tsx:391` (Cancel present); no spec clicks the picker's Cancel otherwise; walks log `cancel×n` to the console only (`plays_a_whole_game.spec.ts:62-65`) and require option/placement/confirm (`:75-78`).
- **Risk/open:** requiring an occasional affordance is "a flake wearing a coverage badge" (`plays_a_whole_game.spec.ts:79-81`).
- **Premise:** unrecorded at HEAD — no document since A-ho §5 :112 records a cancel count.

### Row 34 — identity states, single-player controls, foreign screen, foreign Remove never walked
- four identity states A–D — jsdom `web/src/Door.test.tsx:35-97`; only the name step is walked (`setup_screens.spec.ts:176`) — true
- the single-player build's four controls — jsdom `App.test.tsx:4194` "in a single-player build" (`vi.stubEnv("VITE_MULTIPLAYER", "false")`); no e2e builds that client — true
- the game route's foreign screen — jsdom `App.test.tsx:1843` "a game this build's bot cannot continue" — true
- a completed foreign Remove — jsdom `OnThisDevice.test.tsx:195`; e2e reads the "Will not open here" heading for a cross-rev pair only (`offline.spec.ts:943`) and clicks no Remove — true
- **Fix touches:** e2e only (a `VITE_MULTIPLAYER=false` build for one spec; a foreign-bot pair seeded into IndexedDB). `e2e: yes`, all else no.
- **Risk/open:** states A (expired Access) and B (a door that named nobody) need a server answering as Access does; the local `open` door cannot (B-ho §5 :113-115).

### Row 35 — `hasMove` copied three times across the e2e specs
- **Where:** `web/e2e/shared_table.ts:55-59` (the ONE definition); imported at `exit_game.spec.ts:100`, `joins_by_link.spec.ts:40`, `two_browsers.spec.ts:48`
- **Now:** `shared_table.ts:5-10` "Lifted out of `two_browsers.spec.ts` … `joins_by_link.spec.ts` reads `hasMove` from here too; it carried the third copy."
- **Fix touches:** none (a move to `harness.ts`, M6.1 close :685's home, is optional). All four: no.
- **Premise:** FALSE at HEAD — deduplicated at M6.5c Task 16; no inline copy of the predicate remains.

### Row 41 — M6.5b2 test minors (M6.5b2 close §6.2 :540-559)
- no client-drawn restore cell — unit cells restore the Rust-emitted `web/src/wire-script/backup-envelope.json` (`backup.test.ts:42-49`) — PARTLY FALSE: e2e `plays_locally.spec.ts:910` restores a Worker-drawn `local-` game into a fresh profile (`:953-964`, PR37); `LOCAL_PREFIX`/`LOCAL_USER` (`web/src/local/ids.ts:19,30`) stay pinned only indirectly
- `channelOf`'s bare `return "multiplayer"` — `web/src/route.ts:81`; no cell names `channelOf` (`route.test.ts` has none) — true
- "leaves the Worker's words alone" — `App.test.tsx:4736-4748` waits 20 ms real time; no partner advances past the 5-s bound with the server answering (the fallback cells are `:4707`, `:4718`) — true (the lost partner's shape is inferred)
- the precache regex — `web/src/precache.ts:412` `body.match(/digest: "/g)` rides esbuild's spacing (fails closed) — true
- the guard misses `browser.newPage()` — `web/e2e/every_spec_declares_its_worker.spec.ts:51` `const OWN_CONTEXT = /browser\.newContext\(/;` (no spec calls it today) — true
- cell 2's "answered nothing from the network" — `offline.spec.ts:431`, no anti-vacuity partner — true
- `CAP = 4000` at `plays_a_whole_game.spec.ts:43`, `plays_locally.spec.ts:43`, `two_browsers.spec.ts:84`, `offline.spec.ts:89`; `REV = "rev7"` at `plays_locally.spec.ts:75`, `offline.spec.ts:125` — true
- **Fix touches:** test files only (`REV` could read `/api/health`'s `semantics_rev`). `e2e: yes`, all else no.

### Row 42 — cycle B's harness notes (B-ho §7 :189-194)
- the held arm's `connected = true` — `web/src/signal.ts:421`; it switches `lost()` (`:270`) between "reconnecting" and "connecting"; the held-arm describe `signal.test.ts:643` asserts no word after a failure (R-N3's cells `:307`, `:325` run other arms) — true
- the lobby sweep — `web/e2e/setup_screens.spec.ts:292` reads EVERY `.lobby li.lobby-row` and refuses a spread over 12 px (`:316-317`); the `finally` (`:393-397`) deletes only this cell's tables — true
- the `run` premise — `web/src/backup.test.ts:624` `expect(JSON.stringify(JSON.parse(RECORD_TEXT), null, 2)).not.toBe(RECORD_TEXT)` is made true by the record's top-level `"seed": 1009902671473821048` (> 2^53) alone — true
- **Fix touches:** `signal.test.ts`; `setup_screens.spec.ts` (filter to `opened`); `backup.test.ts` (reserialize the `RUN` slice alone). `e2e: yes` (lobby cell), all else no.
- **Risk/open:** if Task 8 removes the dark held arm (row 86), the first item is moot.
