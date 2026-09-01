# Web frontend — Wave 0: the correctness floor

**2026-08-31. Branch `web-frontend`.** Not a milestone: a fix wave between M3 and
M3.5, opened by an eight-lens adversarial review of M1–M3 plus a completeness
critic. Owner ruling: *its own wave, before M3.5* — the `Games` fixes interlock
and wanted their own green suite, and without the harness fixes M3.5's claims
about M3.5 would have become findings 20 and 21.

## 0. Why this wave exists

The review returned ~110 raw items; after de-duplication and evidence triage,
**7 HIGH defects, 12 HIGH false claims, 9 tests that cannot fail, 31 MEDIUM/LOW**.

The number that shaped the wave: **twelve of the nineteen HIGHs were sentences
this lane wrote about itself** — the same signature the M2.5 and M3 records
already name, for the third consecutive milestone. The mechanism is always the
same, and making a `match` exhaustive does not close it: the compiler forces the
new arm, the developer lands in the right file, and the hand-written list beside
it stays stale.

## 1. What landed

Seven commits, TDD throughout, **every fix mutation-proven with the mutation
executed rather than described**.

| # | Commit | Closes |
|---|---|---|
| 1 | `fix(web): stamp the digest on the way out` | A1 |
| 2 | `fix(web): three interlocking races in Games` | A3, A4, A5 |
| 3 | `docs: Apache-2.0, and the asset carve-out` | licence |
| 4 | `fix(web): stop serving the setup seed` | A6 |
| 5 | `fix(web): the phone overflow was one line` | 545px, C7, A2, C3, C4 |
| 6 | `fix(web): why's unplayed candidates stop naming cards` | A7 |
| 7 | `test(web): cover DELETE` | C8 |

### A1 — the digest was never stamped, and it cut both ways

`Games::persist` serialized `session.record()` directly, but the terminal digest
is stamped at WRITE time, not held on the record. So every record this server
wrote carried none, and `Session::from_record` guards on
`if let Some(expected_digest)` — the provenance refusal was not failing, it was
**never running**. Its own doc states the cost: editing `seed` alone "replays
with zero errors, passes `check_invariants` … and resumes the game on tiles that
are not the ones on the table."

The other half was destructive. Every CLI writer stamps, and `from_record` keeps
the parsed record, so a cold-loaded save held a digest for state N in memory and
the next `persist` wrote N+1 events still claiming it. The rebuild after that
refused: **a permanent 404 on an intact record the server had just written.**

One writer now — `Session::to_record_json`, which `save` also calls.

### A3 / A4 / A5 — three races in one region

- **A name is not a game.** The delete tombstone's own doc names the bound that
  breaks it ("bounded by `create`, which clears the id"); `create` cleared it on
  *attempt*. `persist` now compares the seed — identity before length.
- **A refused write means the handle is wrong.** `live` is never invalidated, so
  a refused handle stayed resident and answered every later request: each move
  applied, each returned 200 with a warning, each write refused. One lost move
  became a lost session. A conflict now evicts; an **I/O failure does not**,
  because there the handle is right and the disk is not, and `input`'s measured
  200-plus-warning answer for that case is deliberate.
- **`create` was check-then-write across a slow middle.** Two requests both read
  absent, both built, both wrote — and the games differ, since `config_of` draws
  a fresh bot seed per request. Serialized by its own lock, not by `live`, so two
  creates queue while a create and a read do not. The realistic trigger was one
  user and one browser: `NewGame`'s submit gated on `busy` STATE, which `App`'s
  own `inFlight` comment already records as insufficient.

### A6 — the setup seed was the whole hidden set

`game/observe.rs` names `config.setup_seed` **first** in the hidden set: it
regenerates both decks, the boxed pile, the board and every deal. It was on 20 of
41 committed payloads, twice over, and rendered by one line of one title strip.
The default id `sim-{seed:016x}` spelled it too, so `GET /api/games` handed out
every game's hidden set to anyone who could list.

`TitleModel::seed` became `Option<u64>` rather than being skipped: the CLI
genuinely renders it, and `skip_serializing` alone would have broken
`every_wire_surface_round_trips_through_json_unchanged` — a field that cannot
round-trip is not a wire field. The terminal passes `Some` and prints exactly what
it printed before; **the nine `assert_capture` goldens and 18 `guide_sync` tests
are unmoved**, which is how that is known rather than believed.

Two fences, deliberately different, because one mechanism is one mistake away:
the server searches every committed payload for the **value** in both JSON
spellings, the client asserts the **shape** it may see. `preview` is the single
exemption and asserts it *still carries* the seed, since the flow round-trips it
into `create`.

### A7 — and a correction to the finding that prompted it

The mechanism is real: `Move::FulfillOrder`'s only generator reads
`p.order_cards_in_hand`, and `describe_move` words it as the printed card, so a
ranking served whole reads out a hand the same server masks in `build_hand`.
Owner ruling: **redact the wording, keep the rows** — the value and margin are
the shape of the bot's preference and name no card, and capping at the terminal's
three would have closed the leak by deleting the "show all candidates" surface M3
built deliberately. `HandVisibility` splits by *whose* hand, not by "is it a bot";
the move that was **played** is never masked, because it happened.

**The review's evidence for this one did not hold.** It reported the leak as
"measured on the committed fixture: `why.json` `ranking` is 222 rows". The count
is right; all 222 rows are `Place …`, and neither committed ranking holds a
single `Fulfill`. The corpus fence written for it failed its own anti-vacuity
assertion and was removed rather than weakened. The property is unit-tested
against constructed candidates, and the test's doc records why.

### The 545px that was not a limitation

Three documents record 545px of horizontal overflow at 390px as a measured,
accepted limitation. It is a one-line bug: the `@media (max-width: 1200px)` block
flips `.stage` to column and never resets `align-items: flex-start`, which in a
column container governs the **width**. `min-width: 0` caps the automatic minimum,
not fit-content; `.board`'s `overflow-x: auto` reduces nothing intrinsic. The same
block already states `width: 100%` for `.rail` — the identical symptom, fixed for
the other child and not for the board.

Measured, fixed, re-measured: 4P at 390px gave `.board-col` at 1376px in a 390px
window and **1006px** of document overflow; 2P gave exactly the recorded **545**.
Both are 0.

**The gate could not see it, twice over**: it measured a `documentElement` delta,
so `.board`'s own overflow never reached it, and it asserted at 1440 and 1024 on a
2P game — 1024 is *below* the breakpoint, and 2P clears it with headroom. It now
returns the widest overflowing element's selector at 2P/3P/4P across 390, 1024,
1201 and 1440.

## 2. Tests that could not fail, and the one that found the others

Fixed: C3 (accepted a refusal in a cell whose setup exists to make one wrong),
C4, C7, C8, plus the e2e browser-absence guard (A2).

**C4 is worth the paragraph.** Fixing it exposed that it had *never run its
click*: at the state it opened the panel, `suggest`'s ranking held one candidate
(the forced opening) and `RunnerUps` returns `null` at `≤ 1`, so a test named
"a panel holding two hundred candidates" was measuring a panel with no candidate
list in it. Three vacuities in one file, the third found only by fixing the second.

**The e2e opt-out was failing the run.** `it_explains.spec.ts` never imported
`haveBrowser`, so with no browser the config drops its `webServer` and four tests
failed instead of skipping — the exact defect M2.5 recorded as fixed,
reintroduced by the milestone that cites M2.5's review as its lesson. It is now
held by `every_spec_can_skip.spec.ts`, which reads the spec directory **at run
time**; it declares no browser fixture, so it is the one spec that runs under the
opt-out it protects, and its own exemption is computed from `import.meta.url`
rather than written down.

## 3. Not claimed

Recorded in-source rather than implied, because a comment that reads like a
covered invariant is how this lane has three times shipped a fence nothing held:

- **`create`'s `entry().or_insert()` is covered by no cell.** The create-race test
  is mutation-proven against removing the lock and passes with this reverted to
  `insert()`. It defends the create-versus-rebuild window, which no test drives —
  reaching it needs a store that can be parked mid-`persist`.
- The `why` redaction is proven on constructed candidates, not on the corpus,
  for the reason in §1.

## 4. Licence

Owner ruling: **Apache-2.0**, verified safe against both trees before proposing
it — 78 crates and 155 npm packages, zero copyleft, zero undeclared; the 21
MIT-only crates are one-way compatible into an Apache-2.0 work. `LICENSE` is
byte-identical to apache.org's canonical text except the appendix copyright line
(diffed, not assumed).

`ASSETS.md` carves out what the grant cannot cover, and the audit found a third
path nobody had counted: `tts-assets/` (118 files, ~60 MB), the derived crops
(132 files), and **`analysis-1/00-yokohama-rulebook.pdf`, the complete published
rulebook at 6.53 MiB**. It states the situation and explicitly declines to draw
the legal conclusion.

**The public split stays its own project**, at the v5 boundary, per the owner.
Recorded so the reason is not re-derived: it must be a **fresh history**, not a
re-push. `tts-assets/` entered at commit 32 of 1,289 (2025-12-07), so removing it
rewrites **1,257 commits** — and this project cites commit hashes as governing
references (`ba7a808`, `35b694d`, `246d4f0`, `f9ca6d4`), every one of which a
rewrite invalidates. Un-committing the derived crops separately buys nothing,
since one `filter-repo` pass removes both.

## 5. Counts at the wave head

| | before | after |
|---|---|---|
| `cargo test -p yokohama-web` | 55 | **63** (57 lib + 1 gate + 5 `wire_fixtures`) |
| `npm test` | 194 | **195** |
| `npm run e2e` | 11 | **14** (13 browser + 1 structural) |
| `cargo test -p yokohama-cli` | 849 + 2 | **849 + 2** — unmoved |
| `yokohama-core` | 1577 + 29 | **unmoved, byte-identical to master** |
| wire corpus | 41 | **41** |

`cargo clippy --workspace --all-targets -D warnings` clean; `npm run typecheck`
clean over both TS projects. The fifteen-gate roster is untouched and nothing
here is `#[ignore]`d.

## 6. Owed, and explicitly not done

- **Cache headers and graceful shutdown.** `art.rs` sets `max-age=3600` on
  stable-named files while `assets.rs` sets nothing on the content-hashed bundle
  — exactly backwards. `tokio`'s `signal` feature is enabled and no
  `with_graceful_shutdown` exists, so a SIGTERM between `Repl::step` and
  `store.write` loses the move; both hosted shapes SIGTERM on scale-to-zero.
- **CI still runs only `cargo test -p yokohama-core`**, so nothing in this lane
  can make it red — ~78,800 added lines unreachable from CI. Unchanged since it
  was first asked, with the stakes now the size of the lane.
- **The MEDIUM/LOW tail** of the review (31 items), and the ranked usability work,
  which is M3.5.

## 7. What M3.5 inherits

Owner ruling: **layout before the scrubber**, because the scrubber's host is the
one M4 decision the code cannot answer without knowing the layout, and today's
answer — a modal at `inset: 3rem 2rem` — cannot fit a 4P board.

Measured at 1440×900, 2P: the page is **1950px, 2.17 screens**; `.board` is
992×728 and `.rail` is 384×**1534**, so the rail is 2.1× the board's height and
the decision zone — the options and Undo — starts at y=**1703**, **803px below
the fold**. At 4P it is 2.48 screens and the board overflows its column by 384px.

Tile art is **69.6% obscured** (white-vs-black difference in real Chromium):
`--art-veil: 0.82` as a whole-image opacity plus a full-bleed six-stop scrim,
mean transmittance 0.304, **52.7% of tile height below 0.25** — and the worst band
is exactly where the printed power ladder and the 5-space live.

**A correction the critic made to the review, which changes M4's menu**: a
per-seat 5-Power token strip *is* buildable with no core change. The face is
destroyed in state (`player.rs:225` is a bare count) but preserved in history —
`apply_move.rs:921` does `slot.take()`, so at the event where a seat takes its
bonus the pre-event state carries the face and the acting seat. It is a
replay-derived projection over the `GameRecord`, i.e. the same `state_at`
machinery M4 builds, and cheap once M4 lands.

**And one M4 decision is already settled by measurement.** `why?event=n` calls
`state_at` *before* it refuses, so it times a real replay: on a 195-event game
against the release binary, **~1.2 µs per event**, sub-millisecond to any
position, and a sweep of every position ≈ **17 ms** of server CPU. Even the
1,036-event committed fixture sweeps in ~0.7 s. **No checkpoint ladder is
needed** — debounce plus plain `state_at` is honest, and that is a whole
subsystem out of M4.
