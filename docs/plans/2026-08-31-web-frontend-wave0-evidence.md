# Web frontend — Wave 0 evidence: the M4 reconnaissance and the M1–M3 review

**2026-08-31. Branch `web-frontend`. A frozen evidence record**, in the shape of
this repo's other tracked evidence docs (`2026-08-09-phase3.5c-round{1,2,3}-evidence.md`,
`2026-08-19-phase3.5c-v3-*-evidence.md`). The wave it fed is
[`2026-08-31-web-frontend-wave0.md`](2026-08-31-web-frontend-wave0.md); the two
handover documents it feeds next are `-m3.5-prep.md` and `-m4-prep.md`.

**It is tracked deliberately.** The review's own finding [B5] is that a tracked
document asserted the closure of an audit living only in gitignored `docs/temp/`
— "a tracked doc asserts the closure of a document a repo reader cannot open".
Three tracked documents now cite this material, so it is here rather than there.

## What this is

Three instruments, in the order they ran:

1. **§I — the M4 reconnaissance** (7 parallel readers → 1 synthesis). Mapped the
   `record::state_at` seam, the server and client architecture, the test harness,
   the CLI's own history model, and the lane's alignment, before any M4 design.
2. **§II — the M1–M3 adversarial review** (8 lenses → 1 synthesis). Vacuity first,
   one wave, no verify fan-out — the method this lane's own M3 §9 prescribes.
   ~110 raw items → **7 HIGH defects, 12 HIGH false claims, 9 tests that cannot
   fail, 31 MEDIUM/LOW**. Twelve of the nineteen HIGHs were sentences the lane
   wrote about itself, for the third consecutive milestone.
3. **§III — the completeness critic.** What the eight lenses did not read, what
   class they did not hunt, and which of their own conclusions do not hold.

The eight raw lens reports are working material and are NOT tracked; they live at
`docs/temp/webresearch/2026-08-31-wave0-research/30-lens-*.md` in the worktree
that produced them, and are superseded by §II.

## READ THIS BEFORE TRUSTING ANY CLAIM BELOW

This is frozen evidence, and parts of it were **disproved by the work it caused**.
Corrections are recorded forward, never edited in — so the list is here:

- **§II [A7]'s measurement does not support its finding.** It reports the `/why`
  hand leak as "measured on the committed fixture: `why.json` `ranking` is 222
  rows". The row count is right and the mechanism is real, but **all 222 rows are
  `Place …`** — neither committed ranking holds a single `Fulfill`, so the corpus
  cannot demonstrate this leak at all. The fix shipped; its test is constructed,
  not corpus-anchored, and says why.
- **§II §E.6 wrongly exonerates the 545px overflow** ("either the number is
  measuring something the comment does not name, or it is stale"). §III §4(a)
  overturns that from the stylesheet, and it was then confirmed live: it is a
  one-line bug (`align-items: flex-start` survives the column flip and governs
  width), measured at 1006px on a 4P phone viewport and fixed to 0.
- **§III §3 overturns §II §F.3's central conclusion.** §II says a per-seat
  5-Power token strip "is not implementable without a core change". It is: the
  token face is destroyed in state (`player.rs` keeps a bare count) but preserved
  in history (`apply_move.rs` does `slot.take()`), so the strip is a
  replay-derived projection over the `GameRecord`. No core change, no
  `SEMANTICS_REV` implication.
- **§II's count claims were re-summed and one was already wrong before this
  lane existed**: `yokohama-core` is **1576 + 29** across 21 binaries with the
  doc-test making cargo's aggregate 1577 — the documented 1577+doc/1578 was one
  high.
- **Known-defect (a) — §I §4.7's "undo through the web is never durable" — was
  TRUE when written and is now CLOSED**, by `af706de` (2026-09-01), which
  replaced `Games::persist`'s monotonicity guard with a compare-and-swap against
  a digest of what the process last left in the store. The finding stands as a
  correct one; what no longer holds is its present tense. So §II's references to
  it — the §II synthesis item 2 ("a live correctness bug in the surface M4 sits
  on"), the open question at §II's [2], the eviction generalization at [A6]'s
  *"(a) loses one undo, this loses the rest of the game"*, and the `autosaved`
  finding's list of reachable false states — all describe the **pre-fix** state.
  The fix's own record is `2026-08-31-web-frontend-wave0.md` §"Follow-on
  (2026-09-01)"; the hazard's post-fix reading is `-m4-prep.md` §6.4b.
- Route, test and fixture counts elsewhere in §II are as of the M3 head and moved
  during the wave. The wave record's table is the current reading.
- **Every `file:line` below is as of the M3 head and many have DRIFTED.** The wave
  and its follow-on added lines to the files they fixed — `crates/yokohama-web/src/games.rs`
  alone moved by a few hundred — so roughly three dozen citations here point at
  the right file and the wrong line. The FILE and the SYMBOL are still good;
  open the symbol, not the number. The M3.5 and M4 prep documents carry
  re-verified citations and are the ones to trust for position.

---


# §I — The M4 reconnaissance

# 1. The seam

**What `state_at` gives.** One function, `pub fn state_at(record: &GameRecord, k: usize, rules: &Rules) -> Result<GameState, ReplayError>` (`crates/yokohama-core/src/game/record.rs:561`), re-exported to the crate root through `pub use record::*` (`game/mod.rs:136`) → `pub use game::*` (`lib.rs:10`), so it is `yokohama_core::state_at`. Properties, all verified in source:

- **`k` counts events APPLIED, not an event index** (doc, record.rs:524-536). `state_at(r, 0)` is post-setup with nothing folded; `state_at(r, n)` is the position event `n` was decided *in*; `state_at(r, r.events.len())` equals a full `replay`. Domain is `0..=len`, inclusive — the top position ("the game as it now stands") has no event to name.
- **Never clamps.** `k > len` is a typed `ReplayError` whose `event_index` is the past-the-end boundary (record.rs:562-571). `ReplayError` is a struct, not an enum (record.rs:324) — a caller gets an index and a `String`, and cannot match on a variant.
- **Total and deterministic** over a record that loaded: `rebuild_initial_state` (record.rs:400) = `new_game` (board gen + deck shuffles) → optional `reconcile_setup` → `finalize_setup`, then `replay_from(&initial, &record.events[..k], rules)`.
- **Cost is not free.** `replay_from` folds through `fold_one_event` (record.rs:348), whose `Move` arm is `apply_move` — **the checked boundary** (`game/apply_move.rs:222`: a full `get_legal_moves` enumeration + linear `contains` + a whole-`GameState` clone). So one call = one full setup preamble + `k` full legal-move enumerations. Nothing is cached: `Session.checkpoint` exists (`session.rs:80`) but is private and `None` on every production constructor (`:144`, `:173`, `:386`) — set only by a test-only constructor at `:787`.

**What a frame at k needs beyond it.** `build_frame(state, rules, seats, seat, legal, title)` (`screen_model.rs:2684`) takes six arguments. Their provenance at a historical `k`:

| arg | at k |
|---|---|
| `state` | `state_at(record, k, rules)?` — the only genuinely new call |
| `rules` | time-invariant; `AppState.rules` (`lib.rs:153`) |
| `legal` | `get_legal_moves(&state_k, rules)` — one *more* full enumeration on top of the k already paid |
| `seats: &[SeatFacts]` | record-scoped (`repl.seat_facts()`, `repl.rs:693`, folding `privacy \|\| bots.is_bot`, `repl.rs:602`); valid at every k, **but has no per-event grain** — `record.run` is one `SeatProvenance` row per seat for the whole game |
| `seat` | **not derivable — a design decision** (§6.1) |
| `title` | `mode`/`seed` are record facts; `autosaved: true` (`client.rs:352`) is a *session* fact with no historical meaning |

**Is a historical frame well-defined? Mostly yes.** Well-defined without any new decision: the acting seat (`state_k.current_player` — the same value `why <k>` uses to pick a contestant, `why.rs:419`); the legal list (and if `events[k]` is a `Move`, it is *provably* a member, because `apply_move` checked exactly that when the record was folded); the whole `DecisionModel` (`build_decision` reads only `is_game_over`, `pending_choices.first()`, `turn_phase`, `turn_number`, `current_player` — `screen_model.rs:2451-2479`); `BoardModel`; every dashboard panel.

**Open questions the code cannot answer:**

1. **Whose seat is the frame built for?** `build_decision_for` (`screen_model.rs:2501`) branches on `viewer == state.current_player`; follow-the-mover and fixed-viewer are both honest and are *different products* (options at every k vs. an empty decision zone for the other seat's moves). This is also the hidden-information decision (§4.1).
2. **What does the strip say?** `ClientState` (`client.rs:130-183`) carries six fields read off the **live** session — `over`, `scores`, `deck_remaining`, `end_game_triggered`, `end_triggers`, `your_turn` — plus `prompt` and `warnings`. Every one is a pure function of a `GameState` and *could* be re-derived at k; whether a historical payload carries them at all is a decision.
3. **What is `prompt` at k?** There is no historical analogue. `PromptState` (`client.rs:280-327`) is built from `Repl::sub`, an in-memory sub-prompt (Pick/Retreat/PlaceA/PlaceB/Reveal). The record stores only *applied* moves, so an intra-move picker state is unrepresentable. Omit, or synthesise a `Watching`-shaped value.
4. **What does `autosaved` mean at k?** Both `true` and `false` are slightly false.
5. **Is the frame built from the resident handle or from the durable record?** They can disagree (§4.6).
6. **Does the scrubber offer "play from here"?** The record cannot branch — `Session::undo` (`session.rs:683`) truncates with no tombstone and no redo, so an undone line is gone forever.

---

# 2. What already exists that M4 must not re-derive

| Thing | Where | Note |
|---|---|---|
| Replay-to-position | `record.rs:561` `state_at` | plus `pub fn replay_from(&GameState, &[RecordedEvent], &Rules)` at `record.rs:384` — the primitive a checkpoint ladder needs, foldable from *any* base state |
| Frame builder over a bare state | `screen_model.rs:2684` `build_frame` | pure; no `Repl`, no `Session`. Already what the head is served from (`client.rs:339`) |
| Observer scoping | `screen_model.rs:2501` `build_decision_for` | its own doc: *"This is the seam a multi-seat API needs and the CLI never did"* |
| Hand masking | `screen_model.rs:132` `SeatFacts` → `:2661` `build_dashboard` → `:2301` `build_hand` | `cards: Vec::new()` when hidden; `for_seat` (`:147`) fails safe to `hand_hidden: true` for an uncovered seat |
| The event index space | `screen_model.rs:551-556` `LogEntryModel.index` | *the same integer* as `state_at`'s `k` and `why <n>`'s `n`. Proven: `lib.rs:731` builds `(i, …)` over `0..record.events.len()`; `why.rs:406` subscripts `record.events[index]` with the same value |
| Cached per-event prose | `session.rs:474` `event_description(i)` | a bare `Vec<String>` index, filled at apply time and rebuilt by folding at load (`session.rs:330-333`); the seat is baked into the text as a colour prefix by `describe.rs:1054` — **there is no seat field** |
| Whole-log route | `lib.rs:722-734` `GET /log` | default is the **whole** log with global indices; `LogPanel { total, entries }` + `omitted()` (`screen_model.rs:560-571`) |
| `why` at an arbitrary event | `lib.rs:589` route + `why.rs:392` `account_parts` | `?event=n` is **already shipped server-side** and unreachable from the client — `api.ts:92` has the parameter, `Surfaces.tsx:64` never passes it |
| The twelve typed refusals + the web's thirteenth | `why.rs:149`, `explain.rs:425-440` | `not_a_move` already answers "you scrubbed to a reveal" |
| Lock-drop-then-compute template | `lib.rs:534-563` (suggest), `:595-608` (why) | clone what you need out of a block, drop the guard, then work |
| Lock-and-project template | `lib.rs:471-484` `macro_rules! surface!` | for a `(&GameState, &Rules) -> Model` route |
| Provenance verdicts | `session.rs:239` (`semantics_rev` refusal), `:365-378` (terminal-digest refusal) | run at `Session::from_record`, i.e. at `Games::handle` |
| Client-side components that already render any `FrameModel` | `web/src/Board.tsx:12-32`, `web/src/Dashboard.tsx:17-27` | neither imports `api` or `ClientState`; omit `onTileClick` and `Board` renders inert `<div>`s (`Board.tsx:197`) |
| Debounced fetch-on-a-control | `web/src/Index.tsx:261-279` | 200 ms `setTimeout` + `let live = true` cancellation — the exact shape a scrub-drag needs |
| Modal shell (focus, Escape, `role="dialog"`, stale-payload guard) | `web/src/Surfaces.tsx:79-186`, registry `:53-70` | registering a `SURFACES` row *is* registering the nav button |
| A numbered index list with a right-aligned gutter | `Surfaces.tsx:352-368` + `styles.css:376-378` | the log surface, unstyled as interactive today |

---

# 3. The design space for the scrubber

## Axis A — where the replay happens

**A1. Server computes the frame at k (`GET /api/games/{id}/at/{k}`).**
*Buys:* the whole client half becomes "fetch and render `frame` through the components you already have". No engine in the browser. `Board`/`Dashboard` need **zero** changes.
*Costs:* one full `state_at` per scrub position — setup preamble + `k` checked applies (`record.rs:572-573` → `apply_move.rs:222`). A left-to-right sweep of an N-event game is N(N+1)/2 checked applies. The committed fixture `web/src/wire/log.json` holds **1,036 events**; a naive sweep is ~537,000 full legal-move enumerations. `why.rs:348-352` names this trap explicitly and `why.rs:1334-1341` measures the same shape ("Quadratic in the event count BY CONSTRUCTION… `O(events²)` checked applies"); CLAUDE.md's CLI row prices that one test at ~22.5 s debug.
*Easy because:* `/why` already does exactly this shape off the lock (`lib.rs:595-610`), so the pattern is proven in this crate.

**A2. Client replays.** Requires `yokohama-core` compiled to WASM. Not viable: no wasm target exists anywhere in the tree, and it would put the engine in the browser — a boundary this lane has kept (core byte-identical to master). Name it and drop it.

**A3. Server ships every frame once.** One O(N) forward fold, then all scrubbing is instant and offline. Killed by payload: `web/src/wire/options.json` is **31,705 bytes** for one 2P `ClientState`, `options_4p.json` **53,550**. 1,036 × 32 KB ≈ 33 MB. Viable only in a windowed form (`?from=&to=`), which is really A4.

**A4. A server-side replay cursor / checkpoint ladder.** Keep, per `(game id, record length)`, either a monotone forward cursor or checkpoints every M events; then any k costs ≤ M applies and a forward drag is amortised O(1). This is the shape `Session::load` and `last_explainable_bot_move` already use for their own folds (`session.rs:330-333`, `why.rs:358-370`). Nothing in core needs to change: `replay_from` (`record.rs:384`) is `pub` and folds from any base. `Session.checkpoint` is **not** reusable (private, `None` in production, `session.rs:80/144/173/386/787`), so the ladder is new code in `yokohama-web`.
*Costs:* a second cache beside `Games::live` with its own invalidation (a record whose length changed — or an `undo` that *shortened* it — must drop the ladder), plus memory: a `GameState` is not tiny and 1,036/M of them is a real number.

## Axis B — where the scrubber lives in the client

**B1. On the log surface.** Make `Surfaces.tsx:352-368`'s `<li>` a `<button>`, lift the selected index, fetch `/at/{k}` and `/why?event=k`, render the account and (optionally) a small board inside `.surface-body`.
*Buys:* the cheapest possible diff; the index gutter already carries the exact integer; `/why?event=n` is already wired to the server; inherits the modal shell, focus, Escape, `loaded.of` staleness guard for free.
*Costs:* `.surface { position: fixed; inset: 3rem 2rem }` (`styles.css:350-355`) and a `--tile-w: 14rem` board that is ~84 rem across at 4P — a full board inside the panel scrolls, and `web/e2e/the_screen_itself.spec.ts:107` asserts the *page* never scrolls sideways at 1440/1024. Also `SURFACES`'s loader type is `load: (game: string) => Promise<unknown>` (`Surfaces.tsx:52-56`) — a per-surface parameter is the **second** widening of that contract (the first, for `why`, is documented at `:44-50`).

**B2. As a mode that replaces the stage.** Swap the `.stage` block (`App.tsx:126-143`) for the historical `Board` + `Dashboard`, hide `Decision`, add a scrub track.
*Buys:* full-size board with no layout fight; `Board`/`Dashboard` take the historical frame verbatim; `colors` can be re-derived from the *historical* `Players` panel exactly as `App.tsx:56-58` derives them from the live one.
*Costs:* a new top-level state fork beside `App.tsx:55` (`if (!state) return <NewGame …>`) — the app's only screen mechanism today is that null check. `Decision.tsx:83-95` takes `state: ClientState` whole (14 reads of `state.prompt`) and **cannot** render a historical frame; you must hide it, and then the reusable pieces are only the pure helpers `feeWords`/`choiceWords`/`decidingWords` (`Decision.tsx:17/47/57`). Also: no URL routing exists anywhere in the client (no `pushState`, no hash), so a scrub position is not linkable and a reload drops you back to `NewGame`.

**B3. As an overlay on the live board.** Keep the live stage, dim it, overlay a historical frame.
*Buys:* nothing B2 doesn't, and it invites the worst failure mode — a page showing a past board beside a present-day end-trigger strip (§4.3).

## Axis C — what the payload is

**C1. A full `ClientState` at k.** Maximum reuse client-side; maximum risk of shipping present-tense facts into a past position, and it forces answers for `prompt`, `your_turn`, `autosaved`.
**C2. A narrow `HistoricalFrame { index, total, frame, to_move, deck_remaining, end_game_triggered, end_triggers, over, scores }`** — every field re-derived from `state_k`, no `prompt`, no `messages`, no `your_turn`.
*The second is the one the code argues for*: it makes the six live fields' provenance a compile-time fact rather than a review item, and it keeps `Board`/`Dashboard` unchanged either way (they take `frame.board` and `frame.dashboard`, nothing else).

**Can `Board`/`Dashboard` render a historical frame unchanged? Yes — with one caveat.** `Board.tsx:12-32` imports only types, art helpers and `Cards.tsx`; `Dashboard.tsx:30-47` is a four-arm `in` check over `Panel[]`. The caveat is `TileModel.reachable`: `build_board(state, rules, legal)` sets it from `president_destinations(legal)` (`screen_model.rs:1924-1934`, `:1950`), so a server that builds a historical board **without** a legal list makes every tile silently `reachable: false` and the `◈` mark quietly disappears. Decide it explicitly.

---

# 4. Hazards

**4.1 Hidden information — the `seat` argument is the whole security decision.** `FrameModel`'s only hidden-info surface is `Panel::Hand`; `build_hand` (`screen_model.rs:2301`) empties `cards` when hidden and the mask is asserted through serialization (`screen_model.rs:3822`). But the mask is `SeatFacts.hand_hidden = privacy || is_bot` (`repl.rs:602`), and **`privacy` defaults to `false` in sim** (`repl.rs:444`: `let privacy = matches!(session.mode, TrackingMode::Live)`). The API creates only sim games — and it *will* create multi-human tables: `config_of` (`games.rs:117-134`) accepts 2–4 players and `bot_seats` is `#[serde(default)]` (`games.rs:83`), so `bot_seats: []` is a legal request. A follow-the-mover scrubber therefore hands back every human seat's hand at every position it was to move, and `DecisionModel.options` compounds it (a `FulfillOrder` names a card from that hand). Today's live path escapes only because `ClientState::of` builds for *one* seat at a time. **The mitigation is already in the code and unused: pin the viewer and let `build_decision_for`'s observer arm do its job.**

**4.2 `tile.reachable` is not viewer-scoped.** `build_frame` passes `legal` to `build_board` unconditionally (`screen_model.rs:2695`) even when `build_decision_for` has just emptied the option list for that same observer. Low-value information, pre-existing at the head — but a scrubber reproduces it at every position.

**4.3 The present-tense leak — the failure mode unique to a scrubber.** `ClientState::of` (`client.rs:333-380`) fills `over`, `scores`, `deck_remaining`, `end_game_triggered`, `end_triggers`, `your_turn` from the **live** state and `autosaved: true` unconditionally. Reuse it for a historical frame and the board is at turn 12 while the strip says "FINAL ROUND · GAME OVER · Red 82". Every one of the six is a pure function of a `GameState` (`render::end_trigger_line`, `state.order_deck.len()`, `state.end_game_triggered`, `is_game_over`, `final_scores`) — re-derive, don't inherit.

**4.4 Cost — yes, a naive drag is O(n²) *checked* applies.** Quantified in §3/A1. Two things make it worse than the raw complexity: every step is `apply_move`'s full `get_legal_moves` + `contains` + clone (`apply_move.rs:222`), and **every call re-pays the setup preamble** (`new_game` = board generation + two deck shuffles + `finalize_setup`, `record.rs:400-408`), which is independent of k. Whatever shape you pick, the client must debounce (`Index.tsx:261-279` is the in-repo pattern) *and* the server should not be the only defence.

**4.5 Lock discipline.** Only `/suggest` and `/why` drop the per-game mutex before working (`lib.rs:534-563`, `:595-608`); every other handler holds it across its whole body. An `/at/{k}` that holds the lock across an O(k) replay blocks the board fetch for that game. Follow `why`: clone `(record, mode, bots)` out of a `{ }` block and drop the guard. Note the clone is not free either — `repl.session.record().clone()` copies up to ~1,000 `Move`s per request.

**4.6 A "read-only" GET here is not side-effect free, and the record and the handle can disagree.**
- `Games::handle` on a cache miss rebuilds, runs `bring_alive` (= `pump_bots`) and **persists if the pump played** (`games.rs:290-303`, `:388`). `/log` already has this property and `lib.rs:440-464` retracts an earlier comment that denied it. `/at/{k}` routed through `handle` inherits it: a historical read can advance a bot's turn and write the store.
- **The alternative is a genuine fork with its own hazard.** `Games::store()` is `pub` (`games.rs:200`); reading the `GameRecord` straight from the store avoids both the pump and the lock — but it reads the *durable* record, and after a web `undo` that is **not** the in-memory head (next bullet).

**4.7 Undo through the web is never durable — verified defect, zero coverage.** `Session::undo` (`session.rs:683-727`) strictly decreases `events.len()`. `Games::persist` (`games.rs:335-352`) refuses any write whose count is below the stored one: *"has advanced past this handle … refusing to write backwards"*. `input` (`lib.rs:397-401`) catches that and pushes `"APPLIED BUT NOT SAVED — {e}"` into `warnings`. So after a web undo the browser says "Undid your last move and the bot's reply", the resident `Repl` is unwound, and the store still holds the undone move — until later play re-crosses the old length, at which point a *different* line is written at the same length. An eviction (`MAX_RESIDENT = 64`, arbitrary victim, `games.rs:211`/`:373`) or a restart inside that window silently resurrects the undone move. Grep evidence: `"APPLIED BUT NOT SAVED"` appears only at `lib.rs:400`; the string `undo` appears in no test under `crates/yokohama-web/tests/`, `web/src/*.test.tsx`, or `web/e2e/`. The one adjacent test, `games.rs:447`, pins the two-process case and never exercises undo. **This matters to M4 directly**: a scrubber makes the record visible, and the first thing a user does after scrubbing back is press Undo.

**4.8 Cache pressure.** `MAX_RESIDENT = 64` with arbitrary (not LRU) eviction (`games.rs:373`), ~27.6 KB retained per resident game because each `Repl` owns a full `Rules` clone (`games.rs:203-208`, `games.rs:263`/`:294` do `(*self.rules).clone()`). A checkpoint ladder (A4) adds a *second* unbounded map unless it is bounded and invalidated on record-length change in the same place.

**4.9 The locked-session gate.** The rule, stated in code at `lib.rs:597-606`: **"the lockdown gate fences COMPUTATION, not projection."** `/suggest` and `/why` refuse a locked session as a 200 + refusal body; `ports`/`labs`/`areas`/`log`/`index`/`result` serve normally; `ClientState` reports it as `PromptState::Locked`. Pinned both ways by `a_locked_session_refuses_the_searches_and_still_serves_the_views` (`lib.rs:1142`). **`/at/{k}` is a projection: it must NOT gate on lockdown.** A locked game is exactly the one whose history you want to read. If M4 also adds a per-event `why` fetch, that half gates (it already does).

**4.10 A scrubbed index may not be a move.** `RecordedEvent` (`record.rs:30`) has four variants; three are live-tracking reveals. `state_at(k)` is still valid there, but the decision zone then shows options nobody took next. The typed answer exists (`Refusal::NotAMove`, `why.rs:159`, wire kind `not_a_move`). In practice the web creates no live games — `wire_fixtures.rs:872` records this as the sole prompt-variant exemption — but `Games::handle` will load any `GameRecord` found in `YOKOHAMA_WEB_DIR` (default `$HOME/.yokohama/games`, `store.rs:207-213`), including a live one.

**4.11 `record.run` has no per-event grain.** One `SeatProvenance` per seat for the whole game. A seat downgraded mid-game reads as a bot at every k — fail-safe for the hand mask, wrong for any seat badge the scrubber draws. `why` handles this by refusing (`Refusal::Mismatch`, `why.rs:167`); a frame has no equivalent gate, so scrubber seat badges are strictly less trustworthy than `why`'s verdicts at the same index.

**4.12 Test-surface cost is not five lines.** The handover calls adding a fixture "a five-line change" (`HANDOVER.md:174`). It is a five-*site* change: the capture in `wire_fixtures.rs`, the owed-list entry (`:443`), the import in `contract.ts` (`:88-128`), the `Widen<T>` assertion + `void [...]` entry (`noUnusedLocals` is on) + a `PromptOf<"kind">` assertion if it is a `ClientState`, and the `wire.x` / `EVERY_STATE` rows — then `UPDATE_WIRE_FIXTURES=1`, which **always fails by design** (`wire_fixtures.rs:781`). Also: nothing enumerates the router's routes, so a new route ships with no coverage by default, and `EVERY_STATE` (`contract.ts:509-515`) is hand-maintained and pinned by nothing.

---

# 5. Alignment findings

Ranked by what actually matters.

**1. CI cannot go red for anything this lane does — and it is the open question that grew.** `git diff --stat master..HEAD -- .github` is empty. `.github/workflows/test.yml` runs exactly one command, `cargo test -p yokohama-core`, against a crate this branch left **byte-identical to master**. ~78,800 added lines and 38 new `Cargo.lock` packages are unreachable from CI. This is `06-decisions-and-open-questions.md` §4.3 verbatim, unchanged since asked, with the stakes now the size of the lane. M4 adds a route, a DTO and a client mode into that hole.

**2. The undo/persist defect (§4.7) is a live correctness bug in the surface M4 sits on.** Not a claim problem — a code problem, verified, untested.

**3. "The CLI parity gap CLOSED" is overstated by one row.** The privacy toggle: `grammar.rs:56-64` allowlists `"privacy"` and `grammar.rs:152`'s test asserts it is *"a gameplay line the client can produce"* — and `/usr/bin/grep -rn "privacy" web/src web/e2e` returns **nothing**. The client cannot produce it. `guidingpath.md:201` ("the audited CLI parity gap closed") and `m2.md:37` are the overstatements; `00-README.md`'s careful phrasing ("all of Tier A + the end-trigger strip + **Tier B's four surfaces**") is the honest one. This matters to M4 because privacy is the *other* half of the hidden-information story a scrubber makes urgent (§4.1): with no toggle, a fixed-viewer scrubber is the only fence.

**4. The `docs/temp/webresearch/00-README.md` ledger is stale** — still "M3 ⬜ **NEXT**", 118 vitest / 7 e2e / 33 web / 30 corpus; the true post-M3 figures are 194 / 11 / 55 / 41. It self-describes as archaeology but its ledger section claims currency. Every *tracked* doc (CLAUDE.md, README.md, guidingpath.md, the dated records) is correct. Given that the M3 record's own §9 lesson is about false self-claims, this is cheap to fix and thematically expensive to leave.

**5. The premise "CLAUDE.md doesn't mention the web lane" is false.** `CLAUDE.md:71` carries the crate list with the branch-scoped clause; registration is at eleven sites (`:30`, `:32-35`, `:66`, `:71`, `:81`, `:85-87`, `:90`). `README.md:405-419` names the seventh crate. Nothing stale there.

**6. Reader contradiction — route count.** The `cli-history` reader says *"there are 14 routes and none is `/undo`"*. **False on the count**: `app()` (`lib.rs:772-791`) registers **15** `.route()` calls / 17 method+path pairs (`/api/games` and `/api/games/{id}` each carry two methods). The "none is `/undo`" half is correct — undo reaches the server as a gameplay line through `POST /input`, allowlisted at `grammar.rs:59-60`.

**7. Reader refinement — provenance.** The `state_at` reader's *"a scrubber gets zero provenance verdicts for free"* is true of the function in isolation and **misleading for the web route**: a game can only be resident because `Games::handle` → `Session::from_record` already refused a `semantics_rev` mismatch or `None` (`session.rs:239`) and a terminal-digest mismatch (`session.rs:365-378`), with `engine_version`/`rules_hash` drift surfacing as `load_warnings`. `/at/{k}` needs no extra provenance check.

**8. The plan's route table is not the shipped API, and M4's row is not the only unbuilt one.** `PLAN.md:110-122` lists `POST /option`, `/subprompt`, `/command`, `/undo`, `GET /record`, `POST /import` — all collapsed into the single `/input` mutation or simply not built, alongside `GET /api/games/:id/at/:k`. Treat the table as intent, not spec. Note also that the plan and `06-…md:10` scope v1 as *"save/load/**replay browsing**"* — browsing, not branching.

**9. Cosmetic, but it reads wrong in source.** `grammar.rs:57-63`'s trailing comments are shifted one line: `"more", "list", // the two spellings of the same undo` and `"privacy", // the four move grammars…`. Each comment describes the *next* line's entries.

**Claims I checked and found TRUE:** core byte-identical to master (empty `git diff --stat -- crates/yokohama-core`); `SEMANTICS_REV` unmoved at 7; zero `#[ignore]` in `crates/yokohama-web`; the fifteen-gate roster untouched; `deny_unknown_fields` on all five deserialized request types (`games.rs:70`, `lib.rs:307/498/679/708`); the refusal-is-a-200 rule (`lib.rs:512-516`, `explain.rs:317-322`); the lockdown fences-computation rule (`lib.rs:597-606`); no `ts-rs` anywhere; no `Dockerfile`/`wrangler`/Modal artifact.

---

# 6. The decisions the owner must make

1. **Viewer seat: follow-the-mover, or pin a fixed viewer?** Follow-the-mover mirrors the terminal (`client.rs:238` `viewing_seat`, whose doc records the 3P/4P parity bug behind it) and shows options at every position. Fixed-viewer uses `build_decision_for`'s observer arm, shows an empty decision zone for the other seat's moves, and is the only thing standing between a left-to-right sweep and every human hand in a hot-seat game (`repl.rs:444`: privacy is OFF in sim; `games.rs:83`: `bot_seats` may be empty). Two reasonable people differ; the choice changes the DTO, the tests, and what the product *is*.

2. **`/at/{k}` reads the resident handle, or the durable record?** The handle is the truth the player is playing; reading it means a historical GET can pump a bot and write the store (`games.rs:290-303`). The store is genuinely read-only and lock-free (`games.rs:200` `store()`), but disagrees with the handle for the whole undo window (§4.7). Neither is obviously right, and the answer decides whether M4 must fix the undo defect first.

3. **Full `ClientState` at k, or a narrow `HistoricalFrame`?** The first maximises client reuse and forces answers for `prompt`, `your_turn`, `autosaved` and the six live fields; the second makes their absence structural. This changes the wire contract, the fixture set, and `contract.ts`.

4. **Pay the O(n²), or build a checkpoint ladder now?** Debounce-plus-`state_at` is ~10 lines and is honest at 300-event games; at the shipped 1,036-event fixture a sweep is ~537,000 checked applies. A ladder over `replay_from` (`record.rs:384`) is a new bounded cache in `yokohama-web` with its own invalidation on record-length change. Deferring is defensible; discovering it after the UI ships is not.

5. **Scrubber host: the log surface, or a mode that replaces the stage?** The log surface is a small diff and reuses the modal shell, but a 4P board does not fit `.surface`'s `inset: 3rem 2rem` against the e2e no-horizontal-overflow assertion (`the_screen_itself.spec.ts:107`). A mode is the first top-level screen fork beside `App.tsx:55` and the first thing that would want URL routing (which this client has none of).

6. **Does the scrubber offer `why` per event?** The server half is done and unused (`?event=n`, `lib.rs:589`; `api.ts:92`); wiring it means widening `SURFACES`' `load` contract a second time (`Surfaces.tsx:52-56`) and inheriting `why`'s lockdown gate inside a surface that otherwise must not gate. Cheap and high-value, but it is a scope call.

7. **Does CI grow to cover this lane before M4 lands?** (§5.1.) Adding `-p yokohama-web` — and whether the vitest/Playwright halves run in CI at all — changes what "green" means for every milestone after this one.

---

# §II — The M1–M3 adversarial review

# Web lane (M1–M3) — consolidated adversarial synthesis

Worktree `/Users/jmoore/Documents/Github/yokohama-assistant/.claude/worktrees/cli-web-api-port-936d56`, branch `web-frontend`. Every finding below carries evidence I either re-derived myself or traced to a lens's own reproduction; anything I could not stand behind was dropped. Where I contradict a lens I say so.

Eight lenses returned ~110 raw items. After de-duplication and evidence triage: **7 HIGH defects, 12 HIGH false claims, 9 tests that cannot fail, 31 MEDIUM/LOW.** Twelve of the nineteen HIGHs are sentences this lane wrote about itself — the same signature the M2.5 and M3 records already name, for the third consecutive milestone.

---

# A. HIGH findings — defects

## [A1] `Games::persist` never stamps `terminal_digest` — the engine's anti-tamper refusal is dead on every web record, and a CLI-written save is destroyed by the first web move
`crates/yokohama-web/src/games.rs:341-343`

```rust
let json =
    serde_json::to_string_pretty(repl.session.record()).map_err(|e| e.to_string())?;
self.store.write(id, &json)
```

`Session::save` is the only writer that stamps the field (`crates/yokohama-cli/src/session.rs:414`, `record.stamp_terminal_digest(&self.state);`). I re-ran the grep: the only two production callers in the workspace are `session.rs:414` and `crates/yokohama-core/src/game/arena.rs:1262`. **`yokohama-web` is not one of them.**

**Break (a) — the refusal is disabled.** `Session::from_record` guards on `if let Some(expected_digest) = &record.terminal_digest` (`session.rs:366`). Web records always carry `None`, so the check never runs. Its own doc (`session.rs:349-364`) states what it is for: *"editing `seed` alone in an otherwise clean save replays with zero errors, passes `check_invariants` … and resumes the game on tiles that are not the ones on the table. The digest is the only field that can see that."* The whole setup half of every hosted record is unprotected. `store.rs:110-113` claims the write path is *"the same discipline `Session::save` uses"*; it is that discipline minus the stamping step.

**Break (b) — a stamped record becomes permanently unloadable.** The store is a plain directory of `<id>.json` `GameRecord`s (`store.rs:46-61`, default `~/.yokohama/games`). Every CLI writer stamps: interactive `save` (`repl.rs:3269`), autosave (`session.rs:749`), `import --out` (`bga/import.rs:606`), `--follow --out` (`bga/follow.rs:313`).

Reproduction: drop a CLI save into the store → `GET /api/games/<id>` loads (digest verifies at N events) → `POST …/input` → `persist` writes the N+1-event record **still carrying the digest of state N** → next cold rebuild → `session.rs:369` fails → `handle()` propagates → `ApiError::not_found` → **404 forever on an intact record the server calls corrupt.**

**Fix:** clone the record and `stamp_terminal_digest(repl.session.state())` before serializing — or better, give `Session` one `to_record_json()` that both `save` and `persist` call, so a future field cannot be stamped by one writer and not the other. **Cost: S.**

## [A2] The documented e2e opt-out fails the run — `it_explains.spec.ts` has no browser-absence guard
`web/e2e/it_explains.spec.ts` (whole file)

```
web/e2e/the_screen_itself.spec.ts:14:  test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");
web/e2e/plays_a_whole_game.spec.ts:19: test.skip(!haveBrowser, …);
web/e2e/it_explains.spec.ts:          (no such line; `haveBrowser` is never imported)
```
I re-ran the grep; the file imports only `watchForBreakage` from `./harness` (`it_explains.spec.ts:17-18`).

Reproduced by the claims lens with a real run (`PLAYWRIGHT_BROWSERS_PATH=/tmp/…-no-browsers YOKOHAMA_WEB_E2E_OPTIONAL=1 npx playwright test`): **4 failed, 7 skipped**, non-zero exit. Mechanism: with `haveBrowser` false the config drops the `webServer` block (`playwright.config.ts:107`), so the four unguarded tests fail on browser launch rather than skipping.

This is **exactly the defect M2.5 §9.4 recorded as fixed** ("the silent skip the whole design was avoiding"), reintroduced by the milestone that cites M2.5's review as what it learned from. Three documents assert the fixed behaviour (see [B6], [B7]).

**Fix:** add `import { haveBrowser } from "../playwright.config";` + `test.skip(!haveBrowser, …)` to `it_explains.spec.ts`. Then make it structural: a test in the config, or a lint, that asserts every file in `e2e/*.spec.ts` contains the guard — a per-file convention that only holds by memory is how this recurred. **Cost: S.**

## [A3] A stale resident `Repl` plays an entire phantom game that can never be written back — and the only signal is a warning string
`crates/yokohama-web/src/games.rs:280-305`, `368-378`; `crates/yokohama-web/src/lib.rs:397-402`

`live` is never invalidated against the store (games.rs:315-318 says so), and `evict_down_to` drops arbitrary handles a request may still be holding:
```rust
fn evict_down_to(live: &mut HashMap<GameId, Arc<Mutex<Repl>>>, keep: &str) {
    while live.len() > MAX_RESIDENT {
        let victim = live.keys().find(|k| k.as_str() != keep).cloned();
```
Sequence: (1) `POST …/g1/input` holds H1 and runs `Repl::step` — measured at **0.16–0.30 s** by the repo's own test doc (games.rs:414-416). (2) Traffic pushes `live` past 64; `g1` is evicted. (3) A `GET` rebuilds H2 from the disk record at N. (4) Step 1 finishes and persists N+1. (5) Every later request gets **H2**, applies to a stale state, returns **200 with a divergent game**, and `persist` refuses forever (`on_disk.events.len() > events` → `"…refusing to write backwards"`), surfaced only as `warnings: ["APPLIED BUT NOT SAVED — …"]`.

This is known-defect (a) generalized: (a) loses one undo, this loses **the rest of the game**. The same shape is reachable in one step through [A4]'s clobber and [A5]'s delete/recreate, so eviction is not the only door.

**Fix:** carry the last-published event count on the `live` entry; when `persist` refuses backwards, **evict the handle and force a rebuild** rather than returning 200. Convert the refusal into a **409** at `input` — it is a conflict, not an unlucky save. **Cost: M.**

## [A4] `create` has a check-then-write race and clobbers `live` — two concurrent creates of one name both answer 201, and one client is silently switched onto the other's game
`crates/yokohama-web/src/games.rs:255-276`

```rust
if self.store.exists(&id)? {                       // (1) CHECK — no lock held
    return Err(format!("a game named {id:?} already exists"));
}
self.deleted.lock().expect("games lock").remove(&id);
let mut session = Session::new_sim(...)?;          // slow
…
{ let mut r = handle.lock()…; bring_alive(&mut r); self.persist(&id, &r)?; }   // (2) WRITE
{ let mut live = self.live.lock()…; live.insert(id.clone(), Arc::clone(&handle));  // (3) CLOBBER
```
Nothing serializes (1)→(3). The two games are genuinely different: `config_of` draws a fresh bot seed per request (`games.rs:162`), so with a bot at the start seat `bring_alive` plays a different opening. `persist`'s guard is read-then-write and length-only, so `A.read(absent) → B.read(absent) → A.write(12) → B.write(9)` leaves B on disk and A's 12-event game nowhere.

**The realistic reproduction is one user, one browser.** `NewGame.tsx:45-55`'s `run()` guards on `busy` **state**, not a ref — the exact pattern `App.tsx:17-27` documents as insufficient (*"React batches, so two clicks in the same tick both read `busy === false` and both post"*). A double-click on Start is two `POST /api/games` in one tick.

Note the asymmetry forty lines below: `handle()` deliberately uses keep-first with a comment explaining why (`games.rs:299-302`, *"so one game is never two handles applying moves to two divergent records"*), while `create` — the one place a genuinely **different** game can collide — uses clobber-last.

**Fix:** make the id reservation the serialization point (take `self.live`, refuse if occupied, insert a reservation, then build); use `entry().or_insert()` in `create` too; and change `NewGame.tsx` to the `useRef` guard `App.tsx` already uses. **Cost: S–M.**

## [A5] The delete tombstone defeats itself — delete-then-recreate inside an in-flight move resurrects the deleted game over the new one
`crates/yokohama-web/src/games.rs:259`, `327-340`, `346-357`

The tombstone's own doc states the bound that breaks it (games.rs:348-351): *"It is bounded by `create`, which clears the id."* `create` clears it on **attempt**, at `games.rs:259`, before the new record exists.

Sequence, entirely through the public API: (1) `POST …/g/input` takes the lock and runs `Repl::step` (0.16–0.30 s window, repo-measured). (2) `DELETE /api/games/g` → 204, file gone, tombstone `{g}`. (3) `POST /api/games` `{"name":"g"}` → `exists` false → **tombstone cleared** → new game persisted as `g` (say 6 events), `live[g] = H2`. (4) Step 1 finishes at 15 events → `persist`: deleted-check passes (cleared), monotonicity check `6 > 15` is false → **the write goes through**. Disk holds the *deleted* game; `live` holds the *new* one; the browser that created `g` keeps playing H2 until the next cold rebuild, when its game vanishes and the deleted one returns.

Two guards, each with its own test, and their interaction is the hole.

**Fix:** clear the tombstone only on a create that has actually taken the id; and make `persist` compare **record identity** (`seed`/config/choices), not just a name plus a length. **Cost: S.**

## [A6] The setup seed — hidden-set member #1 — ships on the wire in every payload, twice
`crates/yokohama-web/src/client.rs:352` and `:365`; `crates/yokohama-web/src/games.rs:99`

```rust
seed: crate::games::WireSeed(session.record().seed),   // client.rs:365
seed: session.record().seed,                            // client.rs:352 → frame.title.seed
```
`observe.rs:14` names it first in the hidden set: *"`config.setup_seed` — regenerates deck, box, board AND every deal."* Measured on the committed corpus: **21 of 41** files in `web/src/wire/` carry it, both as `seed` and as `frame.title.seed`. Served by `POST /api/games`, `GET /api/games/{id}`, and **every** `POST …/input`. Rendered by exactly one line — `web/src/App.tsx:76`, a title strip.

The crate's own `draw_seed` doc (`games.rs:105-111`) states the threat model: *"the public board is an injective fingerprint of the seed: small sequential seeds are brute-forceable in milliseconds, **recovering the entire hidden set**."* `lib.rs:31-33` restates it as property #3. Publishing the value removes the need for the brute force: `new_game(config, rules, seed)` reproduces the full `order_deck`/`tech_deck` order and every deal, narrowing an opponent's kept starting order from 36 cards to 2 — and `player_count`, `player_colors` and `start_player` are in the same response.

**Bound, stated honestly:** today this is one operator against a bot, which is parity with the CLI's title strip. It becomes a total break of the hidden-information model on the *first frame* the moment there is a second human or a spectator — the stated future in `docs/temp/webresearch/06-decisions-and-open-questions.md:12`. Aggravating: default ids are `sim-{seed:016x}` (`games.rs:249`), so unauthenticated `GET /api/games` hands out every game's seed.

**Fix:** separate setup from play. The preview round-trip legitimately needs the seed (`games.rs:32-38`); the play payloads do not. Drop `seed` and `title.seed` from `ClientState`, or gate them behind an explicit `?debug=` the deployment can refuse. **Cost: S** (plus a wire-corpus regeneration — the golden will show 21 diffs, which is the contract working). **Do this before any second seat.**

## [A7] `/why` serves the bot's entire root-candidate list, which names cards in the bot's masked hand — and unlike the terminal it does not cap it
`crates/yokohama-web/src/explain.rs:176-195` (`rank_all`), `:354`

I traced every link:
- `Move::FulfillOrder` has one generator, and it reads the mover's hand — `crates/yokohama-core/src/game/legal_moves.rs:76-81`, `for &order_id in &p.order_cards_in_hand`.
- The bot decides over its own view (`crates/yokohama-cli/src/bot.rs:604`, `observe(state, seat)`), so at an AdditionalAction node its root candidates include its own hand's fulfillable orders.
- `rank_all` words **every** root with `describe_move` and filters/truncates nothing (`explain.rs:184`: `text: yokohama_cli::describe::describe_move(&r.mv, state, rules)`).
- `describe.rs:832-833` words a `FulfillOrder` as the printed card: `format!("Fulfill {}{}", order_prompt_line(card), …)` — country, requirements, rewards.
- Measured on the committed fixture: `web/src/wire/why.json` `ranking` is **222 rows**.

Meanwhile the same server never serves a bot's hand at all: `build_dashboard` builds a hand panel only for the viewing seat (`screen_model.rs:2661-2676`), and `viewing_seat` falls back to the first human whenever the actor is a bot (`client.rs:226-238`). One route hides the bot's hand by construction; a sibling route names its contents.

**The lane-new half:** the terminal shows at most three runner-ups — `crates/yokohama-cli/src/suggest.rs:90`, `const RUNNER_UPS_SHOWN: usize = 3;`, applied at `suggest.rs:823`. The web ships all 222. This surface is strictly wider than the terminal it claims parity with.

**Honest caveat:** I could not capture a live instance (the release build holds the cargo lock), so this rests on the code path, not a captured payload. The path is single-sourced and five links long and I verified each.

**Fix:** truncate `ranking` to the CLI's `RUNNER_UPS_SHOWN` + the chosen row + a count, **or** filter root candidates whose move reads a hidden hand when the viewer is not the acting seat. The first is one line and restores parity; the second is the correct fence. **Cost: S** (truncate) / **M** (fence). This fires in the app's primary local use case.

---

# B. HIGH findings — false or overstated claims

Ranked by how badly a reader is misled.

**[B1] `CLAUDE.md:30` — "a thirteenth fails to COMPILE."**
> *"…refusing when it does not cover every `why` **`Refusal`** variant, which needs NO exemption list at all: all twelve are constructed and worded by the CLI's own `Display`, and `explain::kind_of` is a `match` with no `_` arm, so a thirteenth fails to COMPILE."*

**Truth:** a 13th variant plus the one `kind_of` arm the compile error forces leaves the corpus test **green**. `refusal_kinds()` (`explain.rs:507-509`) derives from `refusal_samples()`, a hand-written `vec![…]` of twelve (`explain.rs:472-501`), and `every_why_refusal_the_client_can_be_shown_has_a_payload` compares against *that*. The source was corrected and names the hole in as many words (`explain.rs:462-467`: *"**The residual hole, named:** a variant added to `Refusal` AND to `kind_of` but NOT to the list below still reaches the wire untagged by any fixture. Plain Rust cannot close it…"*). CLAUDE.md was written from the pre-review claim and never revised with the source.
**Edit:** replace the clause with the corrected text — the compile error forces a reader into `explain.rs`; the corpus covers every *sampled* variant, and the residual is named there.

**[B2] `CLAUDE.md:86` and `crates/yokohama-web/src/lib.rs:963` — "it holds only over ALL 34 terms."**
> lib.rs:963: *"`total == sum(terms.product) + terminal.value`, and **it holds only over ALL 34 terms** — which is why the payload carries the zero-product ones rather than shrinking itself."*

**Truth, measured on the committed fixture** (`web/src/wire/suggest.json`): 34 terms, **8 nonzero**, Σ over the nonzero terms alone = **78** = `total`. A zero term contributes zero. `web/e2e/it_explains.spec.ts:118-121` independently demonstrates it by summing only the *visible* rows.
**Compounding:** `docs/plans/2026-08-31-web-frontend-m3.md:306` says *"Corrected at all four sites"* — that sentence is itself false; two live sites still carry it (and `-m3-spec.md:143`, a frozen record, is correctly left alone).
**Edit:** at both live sites, state the real reason — auditability: the reader can see that nothing was dropped. Correct M3 §306 to "corrected at two of four; the remaining two are CLAUDE.md and lib.rs's test doc."

**[B3] `CLAUDE.md:84` and `docs/plans/2026-08-31-web-frontend-m2.md:171-175` — "a container that sleeps loses nothing."**
Refuted by a measurement in this same crate, `crates/yokohama-web/src/lib.rs:313-324`:
> *"In-flight sub-prompt state lives only in the resident `Repl` — `SubPrompt` is not in the `GameRecord` … Measured across two processes over one store: a client holding `{"kind":"place_a","selected":[3,7]}` clicked area 3, and the rebuilt `Repl` applied option 3 — "Place no assistants" — spending the turn's placement on nothing the player chose."*

Three things are lost that the record does not hold, and `input` persists none of them (it writes only when `events.len()` or the lock changes, `lib.rs:398`): `Repl::sub` (the whole multi-step picker), `Repl::privacy`, and any tail after a failed `persist` ([A3]).
**Edit:** "a sleeping container loses every applied move it acknowledged and **nothing else** — an open picker, the privacy toggle, and any move whose write failed do not survive."

**[B4] `crates/yokohama-web/src/games.rs:216` — "An eviction is invisible to a client."**
Same contradiction, same crate, 100 lines apart. It is *visible*: the screen keeps showing "Selected: A03, A07" while the server holds nothing, and the next click is a refusal. The `expect` guard downgrades it from a wrong move to a named refusal — the right fix — but not to invisibility. Practically bounded (`evict_down_to` never drops `keep`, `MAX_RESIDENT = 64`).
**Edit:** "An eviction costs a replay and the open sub-prompt; `Input.expect` turns that into a named refusal rather than a wrong move."

**[B5] `guidingpath.md:201` — "the audited CLI parity gap closed."**
False when written and false at HEAD. Two Tier-B rows of `docs/temp/webresearch/08-cli-parity-gap.md` remain open: L55 (the privacy toggle — known item (b)) and L53 (the per-area *Building sites* view with builders and canal tolls — no such surface; `SURFACES` at `web/src/Surfaces.tsx:53-70` has eleven entries and none is it). Its Tier C (suggest/why/indexes/final score) landed in **M3**, which the same file records downstream — so the M2 row broadened M2's own scope. `docs/plans/2026-08-31-web-frontend-m2.md:37` scopes it correctly ("every **Tier-A** row").
**Aggravating:** the audit lives in **untracked** `docs/temp/webresearch/` — a tracked doc asserts the closure of a document a repo reader cannot open.
**Edit:** "every **Tier-A** row of the parity audit closed; Tier B's privacy toggle and Building-sites view remain open." And move the audit into `docs/plans/` if guidingpath is going to cite it.

**[B6] `web/playwright.config.ts:73-75` — "Every spec now carries `test.skip(!haveBrowser, …)` … the reporter prints seven SKIPPED tests … and the run exits 0."**
False on both counts: `it_explains.spec.ts` carries no such line ([A2]), and there are 11 tests, not 7.
**Edit:** fix the code first, then the count.

**[B7] `CLAUDE.md:34` and `README.md:419` — "`YOKOHAMA_WEB_E2E_OPTIONAL=1` turns absence back into a skip that says out loud it tested nothing."**
Reproduced false: 4 failed / 7 skipped, non-zero exit. Same root as [A2]; fix the code and both sentences become true again.

**[B8] `CLAUDE.md:86` — "every refusal is `why::account_parts`' — **the gate is reached, never reimplemented**."**
True of `why`; false of `suggest`. `SuggestRefusal::message` (`explain.rs:556-595`) says so itself: *"`BotSeat` — **AUTHORED.** … `Locked` — **AUTHORED.** … **These four are the one place M3 re-words at all**."* `explain.rs:12-15`'s module doc carries the unqualified version too.
**Edit:** "every `why` refusal is `why::account_parts`'; `suggest`'s four are authored here and say so."

**[B9] `crates/yokohama-web/src/explain.rs:404-405` — "`RefusalKind` below is what makes it impossible to add a variant and forget one."**
`RefusalKind` **does not exist** anywhere in the repo (verified: `REFUSAL_KINDS`/`RefusalKind` return exactly three hits, all comments). And the claim is the one `explain.rs:459-467` retracts 58 lines below. The review rewrote the function doc and left the struct doc above it.

**[B10] `web/src/wire/contract.test.ts:313-319` — "The server side pins this against `explain::REFUSAL_KINDS`, which a `match` with no `_` arm pins to the enum."**
`explain::REFUSAL_KINDS` does not exist (the real item is `pub fn refusal_kinds()`, `explain.rs:507-509`), and nothing pins the corpus to the enum. This is M3's own first HIGH finding, un-corrected on the client side. The test body is fine; the sentence above it tells a reader the coverage is mechanical when it is not.

**[B11] `crates/yokohama-web/tests/wire_fixtures.rs:848` — "**`9` is load-bearing and moves with the enum.**"**
It does not. See [C1] — a three-line mutation that the compiler *forces* leaves every check green. This is the third repetition of this lane's signature defect and the sentence is what makes it dangerous.

**[B12] `crates/yokohama-web/tests/play_a_whole_game.rs:171-173` — "the handle is dropped and the game is rebuilt from its `GameRecord` alone."**
It is not. See [C2]. The whole test runs against one `Api { router: app(Arc::new(AppState::in_memory())) }` (`play_a_whole_game.rs:49`); `Games::create` inserts the handle into `live` (`games.rs:270`); `Games::handle` returns the resident `Arc` (`games.rs:281-283`); `MAX_RESIDENT` is 64 with one game. `Session::from_record` is never called. The comment names the deployment shape the lane exists for ("a container that sleeps loses its memory") over a check that does not test it.

**[B13] `crates/yokohama-web/src/client.rs:353-355` — "This server persists after every applied move, so the answer is unconditionally yes."**
It attempts to, and has three named ways to fail, each of which reaches the client as a warning **in the same payload that says `autosaved: true`**. See [G-M1].

**[B14] `crates/yokohama-web/src/client.rs:22-25` — "`words` is `describe::describe_move`'s, and for a President move that is a bare 'Place President from hand in Copper Mine (A00)' with no price on it."**
True for `PlacePresident`. **False for `MovePresident` and both placements**, which `describe_move` prices inline — `describe.rs:895-896` (`"Move President to {} — pay 3 yen (1 to Red, 2 canal tolls)"`), `describe.rs:862-868` (`"— free teleport"`), `describe.rs:847`/`:859` (`placement_fee_note` → `" — fee 2 yen (2 to Red)"`). This is also a rendering defect — see [G-M2].

**[B15] `web/src/Decision.tsx:64` — "The near-miss rows, in the CLI's own words."**
False for two of three. `decision_render.rs:345-351` says *"your President started the turn there **and may not return this turn**"* and *"you have no assistant there **to stop on**"*; the browser ships `"you started your turn there"` and `"you have no assistant there"`. Only the Canal row matches. Nothing pins any of the three strings.

**[B16] `web/src/styles.css:56-60` — "the contrast the overlay needs comes from a scrim under the TEXT rather than from dimming the picture."**
Line 145 is `opacity: var(--art-veil)` on the image — a whole-image dim (both verified in place: `--art-veil: 0.82` at `:60`, the `opacity` at `:145`). `:147-148`'s *"near-absent across the middle"* is measured (real Chromium, 1440×900, white-vs-black difference) at a 12.6 % band sitting **above** the midline; at the actual midline transmittance is 0.287. `:10` says "a tile at 15rem"; `--tile-w` is `14rem` (`:49`). `:9` says the art *"sits UNDER the numbers"*, which contradicts `Cards.tsx:6-8` for the same milestone.

*Graded HIGH here rather than Lens 8's MEDIUM because the brief's rubric counts a false claim in a tracked file, and `styles.css` is tracked; the practical consequence is F1 in §F.*

**[B17] `web/src/NewGame.test.tsx:72-79` (tracked) — "the server marks `start_seat`, `seed`, `bot_seats` and `name` `#[serde(default)]` with no `deny_unknown_fields` — so a renamed REQUEST field is not a 400 … Nothing here can close that."**
Closed by M3's owner ruling R2. `games.rs:70` carries `#[serde(deny_unknown_fields)]`, and four more request types do (`lib.rs:307, 498, 679, 708`), all behind `JsonBody`/`QueryArgs`; `lib.rs:851` asserts a 400 naming `startSeat` in both directions. The next reader is told to distrust a fence that exists. (`docs/temp/webresearch/HANDOVER.md:205-211` says the same and calls it *"the cheapest remaining hole and the one M3 will widen"* — LOW only because that file is untracked, per `.gitignore:20`.)

---

# C. Tests that cannot fail

| # | test (file:line) | mutation that leaves it green | realistic? | fix |
|---|---|---|---|---|
| C1 | `the_variant_list_is_every_variant_exactly_once` — `crates/yokohama-web/tests/wire_fixtures.rs:838-855` | Add `Foo` to `PromptState` (`client.rs:54-120`); the compiler **forces** an arm in `PromptState::kind` (`client.rs:191`) and in `prompt_ordinal` (`wire_fixtures.rs:823`). Leave `every_prompt_variant` (`:800`) alone. `all.len()` is still 9; `ordinals == (0..9)`. All four `wire_fixtures` tests, the golden and the client typecheck stay green — and `every_prompt_the_client_can_be_shown_has_a_fixture` derives its tags from `every_prompt_variant()`, so the new kind is never looked for and needs no `EXEMPT` entry. | **Yes — documented history, twice.** M2.5's review added a 10th `PromptState` and all three tests passed (`wire_fixtures.rs:791-798`); M3's added a 13th `Refusal` and all 54 passed. Both were "fixed" by making a match exhaustive so the developer *lands in the file*. This mutation lands in the file and still passes. | Make the list derive rather than be counted: build `every_prompt_variant` **from** `prompt_ordinal`'s arms via a macro that emits both, so one edit produces both. Failing that, assert the fixture set covers every `kind()` string the crate can emit, which is at least reachable from the enum. |
| C2 | the storage round-trip in `play_a_whole_game.rs:171-177` | Break the rebuild branch of `games.rs:284-289` outright (e.g. replace `Session::from_record(...)` with a freshly-seeded session). The test never executes it. | Yes — it is the headline gate, and its comment names the hosted failure mode. | Read through a **second** `AppState` over the same store, the way `a_locked_session_refuses_the_searches_and_still_serves_the_views` (`lib.rs:1142-1171`) already does, and compare `scores` and `frame` — not just `prompt.kind`. |
| C3 | `"suggest answers for the human seat…"` — `web/e2e/it_explains.spec.ts:147-153` | Make `suggest` return `SuggestRefusal::BotSeat` unconditionally (invert the `is_bot` check in `lib.rs:536-548`). `.refusal` renders, `drew` is true, the test passes. | Yes. The setup deliberately puts the bot on seat 0 **and** makes it the start seat so the human is to move — the whole setup exists to make a refusal wrong here, and the assertion then accepts one. | Do what the sibling does: `it_explains.spec.ts:63-68` throws if `.refusal code` has any count. Assert `.bar-row` count > 0. |
| C4 | `"a panel holding two hundred candidates…"` — `it_explains.spec.ts:163-179` | (a) Delete the disclosure button (`Explain.tsx:256-260`) — `more.count()` is 0, the click is skipped, the check runs over 8 collapsed rows. (b) Any refusal from `/suggest` — zero candidate rows. | Yes. | The lane already fixed this shape once in a sibling and wrote down why (`Surfaces.test.tsx:150-157`: *"The first draft of this test wrapped that assertion in `if (actionless.length > 0)`, which is a clause that silently never ran."*). Assert `await more.count()` is 1, then assert `.runner-ups li` count after the click. |
| C5 | the goals blurb cell in `Index.test.tsx:192-211` | Rewrite the goals blurb (`Surfaces.tsx:67`) to anything — `"ordered by gear cost, and how close nobody is"` — and all four parameterized cells pass. Both assertions are behind `if`s; goals groups by `Category` (no `"Country"` tag) and its blurb contains no digit, so **neither fires**. | Yes, and pointedly: this is the one blurb the milestone records as having shipped a false claim (M3 §186-191, *"goals promised 'how close every seat is' over a panel that showed no progress"*). | Assert up front that each blurb triggers at least one check (`expect(tags.has("Country") || stated).toBe(true)` per case), or give goals an explicit cell. The *progress* half is genuinely covered (`Index.test.tsx:48-78`) — that cell is clean. |
| C6 | `"does not offer to show all candidates until there are more than it shows"` — `web/src/Explain.test.tsx:132-141` | Replace `{rest.length > shown.length && …}` (`Explain.tsx:256`) with an unconditional render. The body only asserts the positive branch on a 186-row fixture. | Moderate. No short-ranking fixture exists, so the negative branch has no payload behind it. | Add a short-ranking fixture to the walk, or assert the button's absence on one of the existing small-`ranking` payloads. |
| C7 | the horizontal-overflow cells in `web/e2e/the_screen_itself.spec.ts:107-148` | Set `--tile-w: 100rem`. `.board` is `overflow-x: auto` (`styles.css:122`, verified present since the file's first commit `a979bdf:web/src/styles.css:92`), so its overflow never reaches `documentElement`. **Doubly** insensitive: the asserted game is 2P and the 2P board (915.2 px) fits at both 1440 and 1024 anyway. | Yes — and 3P/4P are never measured; the only 4P test (`:150-172`) runs at the 1280 default where **544 px is clipped, unasserted**. | Measure the widest **element**, not the document delta: `page.evaluate` returning the max `scrollWidth - clientWidth` over `.board`, `.rail`, `.surface-body`, plus the element's selector. Parameterize by player count. |
| C8 | **absent entirely**: `DELETE /api/games/{id}` (`lib.rs:297-303`, routed at `:779`) and the client's confirmation-free delete button (`NewGame.tsx:195-208`) | Make `delete_game` return `NO_CONTENT` without calling `state.games.delete`; or repoint `api.remove` at `/input`; or drop the `setSaved(await api.list())` refresh. Every suite stays green. **Verified:** no test in the repo issues an HTTP `DELETE` — the only occurrences outside `games.rs` doc comments are `web/src/api.ts:60`. | Yes. Given known-item (d) (CI runs only `yokohama-core`), **nothing anywhere exercises the path that permanently destroys a game.** | One route test (create → delete → 404 → not in `list`) and one `NewGame.test.tsx` cell. `Games::delete`'s store-level behaviour *is* covered (`games.rs:412-437`) and that test is good. |
| C9 | **absent entirely**: nothing pins the client's registration of the wire corpus | Add a 21st `ClientState` fixture to `walk()`, regenerate, add the `import` and the `wire` entry so it type-checks, and omit it from `EVERY_STATE` (`contract.ts:515-521`) — it drops out of the root key-set check *and* the seed check. Stronger: omit the `import` line entirely and it is checked by **nothing on the client side**. Counts today: 41 JSON files, 42 `import` lines, 20 `EVERY_STATE` entries — agreeing by hand. | Yes. `wire_fixtures.rs:426-441` lists six shapes as "owed" and the choice-head captures as opportunistic; a grown corpus with an unregistered file is the default outcome. | The **server** side is pinned in both directions (`wire_fixtures.rs:672-685` rejects a committed file nothing emits). Give the client the mirror: `import.meta.glob("./*.json")` compared against `Object.keys(wire)` and against `EVERY_STATE`. |

**Recorded, no action:** `explain.rs:715-717`'s `"one wording per feature, no duplicates by token"` half is structurally unfalsifiable (`feature_labels()` is a `BTreeMap` collect over `Feature::ALL`); the real fence is the `panic!` at `explain.rs:681` and the `== 34`, which the test's own doc identifies correctly. And `wire_fixtures.rs:930-932` claims an empty exemption list *"is asserted to STAY empty"* — nothing asserts it; the list is empty by absence.

**Clean bill, and it matters:** *is there any test that would fail if the server dropped a field the client reads?* **Yes, three independent mechanisms, each traced.** (1) The byte golden (`wire_fixtures.rs:640-687`). (2) The type half — `Widen<T>` assignment at `contract.ts:155-218`, working at every depth because `web/src/types.ts` contains **zero** optional (`?:`) fields. (3) The runtime half — `Object.keys(...).sort()` against `KeysOf`-pinned lists at 31 payload roots (`contract.test.ts:22-103`), the only one that also catches a field the server *adds*. Two stated limits hold: a `| null` field is not descended into when the fixture is null (fixed for `Terminal` and the envelopes by dedicated fixtures), and all three mechanisms only see fixtures **registered in `contract.ts`** — which is C9.

Also verified sound and mutation-resistant, having tried to break them: `grammar.rs`'s allowlist in both directions; the 15-counter fence (`driven_runner.rs:1319-1335` destructures `SearchStats` with no `..`, so a 16th field is a compile error in core); `the_screen_itself.spec.ts:166-169`'s dummy clause; `App.test.tsx:25-48`'s double-click guard; `lib.rs`'s `assert_ne!(SEED, SEED as f64 as u64, …)` premise check; `art.rs:170-266`'s identifier↔manifest join (derived from `Rules::standard()`, never from counts); `playwright.config.ts:44-63`'s throw-by-default. And **M3's own count claims are accurate** — re-counted at HEAD: 55 web tests, 41 wire JSON, 15 routes, 11 e2e, `yokohama-cli` 849+2, zero `#[ignore]` added, `git diff master..HEAD -- crates/yokohama-core` empty, and the CLI change is one visibility-only commit (`fc8ce4c`).

---

# D. CLI/web data parity

Consolidated. `WIRE?` = the committed fixtures already carry the datum, so the fix is client-only.

## D.1 Still missing, and the payload already has it (client-only work)

| Datum | CLI authority | Web today | Cost |
|---|---|---|---|
| **Tolled canal-crossing pairs** | `view_render.rs:520-535` (`sites`) + `:1172-1185` (`areas`), whose own comment says *"The crossings are here because they are on no other surface … A canal crossing is not geometric"* | **zero references to `canal_crossings` in all of `web/src/*.tsx`**; `areas_4p.json` carries all six areas × 3 crossings | ~10 lines |
| Areas surface: 5-Power face / "already taken" / "the Canal carries no token" | `view_render.rs:1078-1094` | absent; `areas.json` carries `tile.token` | ~6 lines |
| Areas surface: site spaces in full words + builder; trading-house VP + owner | `view_render.rs:1096-1140` | absent; `areas.json` carries `tile.site` | ~20 lines |
| Areas surface: board-row grouping | `AreasPanel.rows` | flattened (`Surfaces.tsx:312` `.flat()`) | 3 lines |
| Per-seat `hand_hidden` marker | `panel_render` `(hidden)` | `p.hand_hidden`: 0 refs | 1 line |
| Tracks: per-space VP + rider as **visible text** | `t12-tracks-view` cards | `title=` tooltip only (`Dashboard.tsx:286, 303`) — invisible on touch, invisible to a screen reader, invisible to any assertion | a `tracks` surface |
| Site-index: TL/TR/ML/MR labels and per-space grouping | `t12-site-index`: `TL 1 VP   TR 3 VP + 1 yen` | four spaces flatten into one `.chips` flex row (`Index.tsx:189-191`), so `[{vp:1},{vp:2,yen:1},{},{vp:1,yen:1}]` renders as six peer chips. 6 of 24 sites have a multi-reward space | ~8 lines |
| Near-miss **area name** (`TEA PLANTATION`) | `decision_render.rs:352` | `A07` only (`Decision.tsx:252`) | 3 lines |
| Turn player's **colour** | `TURN 1 · BLUE's turn` | `Turn 1 · seat 0` (`Decision.tsx:104`), with `colors` one prop away | 1 line |
| Game-over standing: colour + winner mark + breakdown | `repl.rs:3334-3355` | `seat 0 / 46 VP` (`Decision.tsx:236-246`); breakdown is a modal away; winner is a **CSS `::after`** only (`styles.css:576`) — not in `textContent`, not in every AT's tree | 5 lines |
| Board legend's Canal line; order-index per-group count; port refill rule; lab pricing rules | goldens / `board_render.rs:493` | absent | 1–2 lines each |

## D.2 Missing whole surfaces

| CLI surface | Web |
|---|---|
| `tracks` (menu [3]) | **none.** `SURFACES` (`Surfaces.tsx:53-70`) has 11 entries and no `tracks` key |
| `sites` — every **area's** site card, builders, canal tolls (parity audit L53, Tier B) | **none.** The web's `sites` button is `api.index(g,"sites")` — the 24-card index, a *different* CLI surface (menu [13]) |
| `players` full view (other seats' hand identities) | absent by design; needs a server widening |

**Consequence, and a live bug:** `Surfaces.tsx:343-345` renders *"detail lives on the {companion} surface"*. `areas.json` carries **2** `"Tracks"` companions and `areas_4p.json` carries **2** — verified by grep on the committed fixtures. Opening Areas on any board points the player at a surface that does not exist.

## D.3 Genuinely not on the wire (server work)

Area-action prose gloss (`view_render.rs:1051`, a CLI-local fn); the nine Chinatown exchange rates (`ChinatownOffer` is CLI-side); the church 6/3 and customs 8/4 majority awards (a CLI literal at `view_render.rs:202-242` — copy the pattern); other seats' order-card identities.

## D.4 Where the web is *ahead*
Every physical copy of a tech title with its own location (`Index.tsx:98-124`; the CLI collapses to `= owned by Red +1 more`); a margin column on the final score; achievement progress as a bar; locked-assistant zone split on the main screen.

## D.5 Parity clean bills
Option wording is the engine's, not a second implementation (`client.rs:244-257` carries `render::DisplayRow.words` and `screen_model::option_fee` — the deepest parity property in the lane, and it holds). All seven `PendingChoice` variants worded verbatim (`Decision.tsx:37-54` vs `decision_render.rs:282-309`). All eight `CardLocation` arms verbatim with a `never` exhaustiveness check (`Index.tsx:61-81` vs `describe.rs:80-101`). `fee_words`, `vp_range` (including the em-dash/en-dash distinction, hexdump-confirmed) and the nine-part site-slot strip in the CLI's own glyph alphabet all match. The end-trigger strip is the CLI's own `render::end_trigger_line`, not a restatement. The delta window survives because `lib.rs:421-425` strips the prompt block **by identity** (`strip_suffix(&repl.prompt())`), not by pattern. **No hidden-hand leak on the frame**: verified on server-emitted fixtures — every payload carries exactly one `Hand` panel and its seat equals the payload's seat; `build_decision_for` hands an observer an *empty* option list, not a redacted one; no `PendingChoice` variant carries a card id.

## Verdict on "the parity gap is closed"

**It is not a true sentence today, and it was not one when `guidingpath.md:201` was written.** What is true is narrower and worth saying precisely: *every Tier-A row of the audit is genuinely rendered, and the frame — board, dashboard, decision zone — is at real parity.* The gap that remains is entirely in the full-screen surfaces: the web has 11 of the CLI's 14, two whole CLI surfaces are absent (`tracks`, the per-area `sites`), and the third (`areas`) renders 3 of the 8 things the terminal's area card carries. **Six distinct data items are already in the committed payloads and drawn nowhere**, so the majority of the remaining gap is client-only. Four items are genuinely off the wire. The audit document itself (`08-cli-parity-gap.md`) re-checked accurate on all ten rows except L34, where it did not distinguish "rendered" from "rendered visibly" and the closure was claimed against a tooltip.

---

# E. Usability and information architecture

**The owner's implicit question — does the game fit on a screen? No.** At 1440×900 a 2P game is ~2.7 viewports tall and the buttons you answer with sit at **y ≈ 1700 px**. Nothing on the page is `position: sticky` (verified: the only `position` values in `styles.css` are `relative`, `absolute`, `fixed`).

## E.1 Layout map (1440×900; 1rem = 16px — neither `index.html` nor `styles.css` sets `html { font-size }`, and `body { font-size: 13px }` at `:77` does not change `rem`)

| Region | CSS | Height |
|---|---|---|
| `header.banner` | `:89-99` | ~56 px, **~107 px when the nav wraps** |
| `.end-strip` | `:102-109` | ~28 px |
| `.stage` = `.board-col` ‖ `.rail` | `:112-114` | **~1610 px** |
| `.decision` — the answer buttons | `:312` | starts at **~1700 px** |

Horizontal (`.stage { gap: 1.5rem; padding: 1.25rem }`, `.rail { flex: 0 0 24rem }` — both verified):
`.board-col = 1440 − 40 − 24 − 384 = 992 px`. Board intrinsic width at `--tile-w: 14rem` (verified `:49`) with `.board-row { gap: 0.4rem }`:

| | widest row | width | vs 992 px |
|---|---|---|---|
| 2P | 4×224 + 3×6.4 | 915.2 | fits |
| 3P | 5×224 + 4×6.4 | 1145.6 | **overflows by 153.6** |
| 4P | 6×224 + 5×6.4 | 1376 | **overflows by 384** — A04 partly, A05 wholly off-screen |

The stacking breakpoint is `@media (max-width: 1200px)` (`:116`), so **1201–1363 px is a dead band** where even a 2P board is clipped while the rail still takes 384 px. That is where a laptop lands.

Vertical: `.tile.has-art { aspect-ratio: 709/531 }` (`:142`) at 224 px wide → **167.75 px tall**, of which tile content needs ~95–115 px. **53–73 px per row is art padding; 212–292 px of board height carries nothing readable.** The rail is worse: `.panel .card-strip { flex-direction: column }` stacks card faces, achievements at `width: 5.2rem` × 530/799 = 125.4 px each × 3 = 376 px, and a hand card at `11rem` = 265 px of art. **With one card in hand the rail spends ~642 px on card art alone — 71 % of a 900 px viewport — before a single number.**

**The inversion, in one line:** the CLI's tested budget is 200×50 (`screen.rs:300` `FRAME_WIDTHS = [198,175,200]`, `screen.rs:368` `FRAME_HEIGHTS = [51,50,49]`) and fits the *entire game* — board, dashboard beside it at 2P, legend, decision zone. At 13 px that frame is ≈1560×975 px, about one 1440×900 screen. **The web spends ~2.7 screens on the same information**, and the difference is almost exactly the ~870 px of printed-card art carrying no fact.

## E.2 What requires leaving the screen

| Surface | Needed *during* a decision? |
|---|---|
| **Areas** — "what every area's action pays at power 1–5" | **Yes, constantly.** This is the fact a Movement destination and an Area Action are chosen on, and there is **no power ladder anywhere on a tile** (`Board.tsx:144-185`). The worst gap. |
| Port / Lab offer | Partly — which zone is reachable at my power is modal-only |
| Log | Sometimes |
| **Final score** | Yes, and a real hole — the inline block prints `seat 0 / 46 VP` with no colour, no winner mark, no breakdown |
| Site cards, Orders, Techs, Goals, Suggest, Why | No — correctly inline or by nature a detour |

**Clean bill, and it is structural rather than careful:** opening a surface does not disturb `Repl::sub`. `lib.rs:471-484`'s `surface!` macro is a pure projection over `repl.session.state()`; the client keeps `surface` in separate state (`App.tsx:14`) and never re-fetches while a panel is open. **Consulting a surface mid-placement is safe** — the same contract `repl.rs:71-79` states for the terminal.

## E.3 What scrolls

Complete list: (1) `document` — vertical, ~2.7 viewports, the only main-stage scroller; (2) `.board` — `overflow-x: auto` (`:122`), which per CSS Overflow 3 makes it a scroll container in **both** axes; (3) `.surface-body` (`:363`). `.stage`, `.board-col`, `.rail` and `.app` declare no `overflow` at all.

**Consequence:** on macOS, overlay scrollbars mean a 3P/4P board is cut off at the right edge **with no visible affordance**. `.board` sets no `scrollbar-color`, no mask, no shadow. A `◈` Movement destination in the last column is invisible and undiscoverable. Test C7 cannot see it.

## E.4 The modal shell

`.surface { position: fixed; inset: 3rem 2rem; z-index: 20 }` (`:351`) is **1376×804 px at 1440×900 — 96 % of the width, 89 % of the height**, with no scrim element. Every surface blocks the board and the decision zone completely, and the surface's top edge at 48 px means **a nav button on a second banner row is underneath the panel** — you cannot switch from Areas to Site cards without pressing Escape first. (The nav's eleven labels total ~1010 px intrinsic; M3 §9.4 already records the height at 1024 px as "a note and not fixed".)

No focus trap, nothing `inert`: `Surfaces.tsx:131-138` does `panel.current?.focus()` plus an Escape listener and nothing else. `aria-modal="true"` (`:144`) asserts a containment the DOM does not have — **Tab from the dialog walks into enabled board tiles and option buttons underneath**, and during a placement prompt a keyboard user can toggle an area they cannot see. Focus is not restored on close (it falls to `<body>` — precisely the state the effect's own comment says it exists to prevent). And `onClose` is `App.tsx:152`'s inline arrow, a new identity every parent render, so **every `setState` in `App` re-runs `panel.current?.focus()`** and yanks focus back mid-interaction.

`grep -rn "aria-live\|role=\"alert\"\|role=\"status\"" web/src` returns **nothing** — the end-trigger strip, `warnings`, the error strip and every applied-move delta and engine refusal are announced to nobody. There is no `<main>` landmark.

## E.5 Ranked proposals

**P1 — Pin the decision zone.** `position: sticky; bottom: 0` on the play `.decision`, `max-height: 45vh; overflow-y: auto`. It already has `background: var(--bg-sunken)` and `border-top` (`:312-315`), so no new visual vocabulary. Caveat the harness already names (`harness.ts:222-224`: *"`.decision` is `.first()` because the client renders the class more than once"*) — give the real one a second class at `Decision.tsx:101` and scope the rule. **This alone converts "scroll 1700 px to answer" into "the answer is always in view." Highest value, near-zero cost.**

**P2 — Delete `.tile.has-art { aspect-ratio }` (`:142`), replace with `min-height: 6.5rem`.** `.tile-art` is already `position: absolute; inset: 0; object-fit: cover`, so the art fills whatever box exists. **Recovers 212–292 px of board height.** One line.

**P3 — Shrink the rail's card art.** `.panel .card-face.achievement img` 5.2rem → 3.4rem (`:257`); pass `size="sm"` at `Dashboard.tsx:385` (`.card-face.sm { width: 8rem }` at `:251` is an existing pattern already used at `Surfaces.tsx:228` and `Index.tsx:170`); drop `flex-direction: column` (`:260`) for the hand so two cards sit per row in a 354.8 px rail. **~260 px saved.** P2+P3 take the stage from ~1610 px to ~1150 px. Two edits.

**P4 — Put the power ladder on the tile.** `AreaCard.ladder = action_ladder(tile.area_type)` (`screen_model.rs:2420`) is a **pure function of the area type**, so it is static for the whole game: fetch `/areas` once beside `api.labels()`/`api.art()`, keep the `area_type → ladder` map, render the power-3 rung as one `.tile-row.muted` and the full ladder into the tile's existing `title=`. ~30 lines, no server change. **Closes the biggest modal-only gap.**

**P5 — Make the surface a right-hand drawer.** `.surface { inset: 3rem 2rem 3rem auto; width: min(44rem, 55vw) }`. **Zero component changes needed** — `.area-grid` and `.index-cards` are both `auto-fill`, `table.outcome` is `width: 100%`, `.bar-row`'s 372.8 px minimum fits a 672 px body. The board stays visible while you read the ladder or the port offer. Pair with `inert` on `.stage` to close the focus-trap finding. **One rule.**

**P6 — Make the board's clipping visible, then unnecessary.** Cheap: `.board { scrollbar-width: thin; scrollbar-color: var(--line-bright) transparent }` — defeats macOS overlay scrollbars. Real: scale `--tile-w` by tile count from `Board.tsx` (same inline-custom-property idiom `Board.tsx:49` already uses). 4P fits 992 px at exactly **10rem** (6×160 + 5×6.4 = 992); 3P at **12rem**. Giving space back from the rail cannot work — 3P would need the rail down to 14.4rem.

**P7 — Collapse the nav.** Keep Areas / Log / Suggest / Why inline; move the four indexes + Final score behind one `Cards ▾` using the `<details>` pattern already at `.legend` (`:209-212`). Recovers a banner row and un-buries the nav from under an open surface.

**P8 — Finish the game-over block, add a live region, add `<main>`.** Colour + winner mark + breakdown inline (`Decision.tsx:236-246`); `role="status"` on the four message channels. The browser's parity claim currently breaks exactly at the moment a game ends.

## E.6 A recorded measurement I cannot reconcile — contradicting the record

`the_screen_itself.spec.ts:128-130`, `m2.5.md:240-241` and `m3.md:187` all record *"MEASURED at 390 × 844 …: 545px of horizontal overflow."* 545 is **arithmetically exactly** `915.2 (the unclipped 2P board) + 20 (.stage padding-left) − 390`. But `.board { overflow-x: auto }` has been on that rule since the file's first commit — I verified `git show a979bdf:web/src/styles.css` line 92 is byte-identical to HEAD line 122 — so the board's overflow cannot reach `documentElement`, and nothing else in the stylesheet is 935 px wide at a 390 px viewport (every wide container declares `flex-wrap: wrap`; the widest fixed widths are 176 px and 224 px). **Either the number is measuring something the comment does not name, or it is stale.** Cheap to settle: one `page.evaluate` that returns the widest element's selector rather than a delta — which is also the fix for C7.

---

# F. Art and asset treatment

All measurements below were taken in real Chromium at 1440×900 DPR 1 against `styles.css` at HEAD, and in Pillow against the committed WebP. I re-verified the CSS constants they rest on (`--tile-w: 14rem` at `:49`, `--art-veil: 0.82` at `:60`, the `opacity` at `:145`, the gradient at `:149-163`).

## F.1 "the CSS over the card images blocks out most of the art"

**Confirmed — and it is the AREA TILES, not the card faces.** Two rules stack: `--art-veil: 0.82` as a whole-image `opacity`, plus `.tile-body`'s six-stop `linear-gradient` scrim. Method: render the tile twice with a pure-white and a pure-black `tile-art` and difference the screenshots row by row; `(white − black)/255` at a row is the fraction of the art's dynamic range reaching the viewer.

| | measured |
|---|---|
| mean art transmittance over the tile | **0.304 → 69.6 % obscured** |
| **maximum** at any row | **0.715** (ceiling is `0.82 × (1−0.12) = 0.7216` — the veil caps it) |
| height passing ≥70 % | 13.2 % |
| height passing **≤25 %** | **52.7 %** |

Row profile: `0:0.00 · 32:0.31 · 64:0.72 · 96:0.38 · 128:0.08 · 160:0.06`.

**The worst band is the part of the art that carries rules information.** The printed power ladder 1–5 with its per-rung yield icons runs across the bottom of every tile (y ≈ 0.61–0.97 → **0.036–0.24 transmittance**), and the printed 5-space — bbox'd by red-pixel detection across seven clean tiles at x ∈ [0.677, 0.893], y ∈ [0.648, 0.712] — sits at **0.10–0.24**.

**Proposal (primary): stop overlaying the tile; caption below the art** — the pattern `Cards.tsx:6-8` already chose and defends, showing 100 % of the art *and* 100 % of the facts.
1. Delete `--art-veil` (`:56-60`) and the `opacity` at `:145`.
2. Delete `.tile.has-art { aspect-ratio }` (`:142`); give the image the aspect: `.tile-art { position: static; display: block; width: 100%; aspect-ratio: var(--tile-aspect); object-fit: cover; }`.
3. Delete the `linear-gradient` and `height: 100%` from `.tile-body` (`:149-163`).
4. Bump `--tile-w` 14rem → 16rem so the un-scrimmed ladder is legible.

Cost: the tile grows 168 → ~263 px tall — **which is why this must be sequenced with P2/P3/P6, not against them.** Benefit: the printed power ladder becomes readable on the board, which today only the Areas modal shows (and see P4 — with the printed ladder legible, P4 becomes redundant, saving that 30 lines).
**Minimal-diff fallback if the overlay must stay:** drop the `opacity` and anchor two scrims to the two measured text bands (head 4.4–15.3 %, pawns/site-strip 60.7–95.6 %). That leaves 30 % of the tile at **100 %** instead of 13 % at 72 % — but still buries the ladder, which is why it is the fallback. **Cost: S either way** (the second is one rule; the first is one rule plus a layout re-check).

**Clean bills, checked specifically:** `.card-face` has **no overlay of any kind** — the owner's complaint does not apply to order/tech/site/achievement cards; `object-fit: cover` crops 0.05 % (cards) and 0.14 % (tiles), so `emitted_art_keeps_the_sheet_aspect` (`art.rs:397-417`) is doing its job; no `filter`/`mask`/`mix-blend-mode`/`backdrop-filter`/pseudo-element paints over any art (the only `filter:` is the correct `grayscale(1)` on a spent mercenary at `:416`); no tile content is clipped by `overflow: hidden` (measured `scrollHeight == clientHeight == 166` on a dense 4P tile).
**Second dimming site (LOW):** `.index-card.placed { opacity: 0.55 }` (`:562`) dims card art in the index surface for every card whose whereabouts is known — deliberate ("reads as SPOKEN FOR") but it compounds F.1 wherever the owner is looking at art.

## F.2 Asset naming

**The manifest is the only join, and no client code constructs a filename.** `web/src/art.ts:16-17` is the whole of it (`BASE + entry.file`); the only `/art/` literal outside the wire fixtures is `art.ts:14`. So a rename costs **three files and zero client changes**: `docs/tools/mkcardart.py`'s emit f-strings, `art.rs:360-368`'s name closures, and a regeneration (`python3 docs/tools/mkcardart.py && … --check`). What pins filenames is deliberate: `art.rs:328-369` (`every_key_names_the_file_it_derives_and_no_file_is_named_twice`) is **the only test that catches a permutation**, so a rename must move it in the same commit.

| dir | n | scheme | self-explanatory? |
|---|---|---|---|
| `orders/` | 36 | `O01..O36` = the number printed in the card's corner | **yes** |
| `techs/` | 20 | slug of the printed title | **yes** |
| `sites/` | 24 | `S00..S23` | **no** |
| achievements / areas / five-power / mercenaries / backs / tokens | 51 | serde names or literals | yes |

**The one genuinely opaque name is `sites/S00..S23`, and it is worse than opaque: the index it encodes is not printed on the card and is not stored.** There is no printed index on a building-site card (the small white mark is artwork). `screen_model::printed_site_index` (`screen_model.rs:2108-2124`) recovers it by **content match** against `Rules::building_sites`, i.e. `09-building-sites.yaml`'s file order. **A YAML reorder silently re-points all 24 files and every test stays green** — key set unchanged, filenames unchanged, pictures moved.

**Proposal:** keep the index prefix (three surfaces print `Site n`) and append the printed content, which both sides can derive: `sites/S13_vp1-build3-blank-build2_T5.webp`. `mkcardart.py` has the YAML dict in hand at line 371; `art.rs`'s test already builds `Rules::standard()`. A YAML reorder then shows up as **24 renames in `git diff`** instead of as nothing. **Cost: S.** Everything else should stay as-is.
**LOW sibling:** `techs/station.webp` and `tokens/station.webp` are two different pictures with the same basename. `art.rs:371-388` counts full relative paths so it correctly does not fire — but a human moving files, or tooling that flattens the tree, will. Suggest `tokens/token_station.webp` (the other seven `tokens/` entries are already prefixed).

**Dead assets (MEDIUM):** 14 of 131 committed files (152 KiB) have no client call site — `backs/*` (6) and `tokens/*` (8) — and their two exported helpers `backArt` (`art.ts:46`) and `tokenArt` (`art.ts:48`) have **zero callers and zero tests**. This also makes `m2.md:31`'s *"131 per-card WebP"* loose. The eight tokens are exactly what the good chips (`Cards.tsx:37-41`) and the yen/station chips want, and `backs/*` is what a face-down deck wants — **wire them up or delete both files and exports.**
**Clean bill:** the manifest ↔ disk join is an exact **131 ↔ 131 bijection**, independently computed. And six site crops were verified true against `09-building-sites.yaml` by eye (S00 ★1/★3/blank/fish3 T7 · S01 · S02 · S07 · S13 · S23 — all exact), which is the check `art.rs`'s own docstring says its tests cannot do (*"They prove the join is TOTAL, not TRUE"*).

## F.3 Five-power crops on player mats — **the owner's model and the engine's model differ, and this must be said before anything is built**

The request: *"the five power cards should be cropped and overlaid on the player mats so that we can see where the shops or trading houses are built and what rewards are there."* Three separate things are crossed, each with a citation:

1. **The six `five-power/*.webp` are not cards and carry no building information.** They are 128×128 round token faces — a numeral and a goods icon (`Copper_2`, `ImportedGoods_1`, `Money_3`, `Seafood_3`, `Silk_2`, `Tea_3`). `analysis-1/yokohama-kb/01-rules-reference.md:678-690`: a 5-Power token says *what you gain when you take it at 5 Power*. It says nothing about shophouses.
2. **What does say "where the shops or trading houses are built and what rewards are there" is the BUILDING SITE card** — `sites/S00..S23.webp`, **one per area, not per player** (`01-rules-reference.md:707-711`; `crates/yokohama-core/src/types/board.rs:126-135`, `BuildingSite { spaces: [SpaceReward; 4] /* TL,TR,ML,MR */, trading_house_reward, shophouses_built, trading_house_owner }`; `board.rs:201` keys them `BTreeMap<AreaId, BuildingSite>`).
3. **"Player mats" is the wrong home for either**, and the one thing a player *does* keep, the engine deliberately does not remember: `crates/yokohama-core/src/types/player.rs:225` — `pub five_power_tokens: u8, // Just count, effects already resolved`. **So "show each player's collected 5-Power token faces on their mat" is not implementable without a core change** (a `Vec<FivePowerEffect>` on `PlayerState` is a serde/save-shape change — STOP territory). The only honest per-player display today is the count, which `Dashboard.tsx:136-138` already renders.

**What is almost certainly wanted, restated in the engine's terms — and buildable today, from data already on the wire, with no core change and no `mkcardart.py` change:** *put the building-site card's occupancy on the site card, and put the 5-Power token art on the tile's printed 5-space.*

Nothing needs cropping. Measured gutters on S00/S13/S23 (360×543, uniform across all three): vertical at x = 0.489–0.497 w; horizontal at y = 0.346–0.355 h and 0.646–0.650 h.

| slot | `spaces[]` | normalized rect on `sites/S{nn}.webp` |
|---|---|---|
| TL | 0 | x 0.03–0.49, y 0.06–0.35 |
| TR | 1 | x 0.51–0.97, y 0.06–0.35 |
| ML | 2 | x 0.03–0.49, y 0.36–0.645 |
| MR | 3 | x 0.51–0.97, y 0.36–0.645 |
| trading house | — | x 0.03–0.97, y 0.66–0.94 |

The TL/TR/ML/MR ↔ index-0..3 mapping is verified against six cards, matching `board.rs:127-128`. Widen `SiteCardView` (`Cards.tsx:224-244`) to take the whole `SiteModel`, wrap the `<img>` in a `position: relative` box, and absolutely position one `⌂` (`&#8962;` — the glyph both `board_render.rs:345` and `Board.tsx:105` already use) per built slot in the builder's seat colour. Place it in the card index's site cell and in a hover/click popover on the board tile, keeping today's compact `.site-strip` as the always-on read. **Cost: M** (one component widened, five CSS rects, one popover).

**The 5-Power token on the tile:** composite `five-power/<key>.webp` (already emitted; key from `art.ts:56-59`) onto the printed 5-space at x 0.66–0.91, y 0.63–0.76 of the tile art, which reproduces the physical board exactly and lets `Board.tsx:169-179`'s text row shrink. **This only works if F.1 is fixed first** — that rect currently sits at 9–22 % transmittance, so the token would be composited into the darkest band of the tile. **Cost: S, gated on F.1.**

**Trap for whoever implements it (MEDIUM):** `web/src/types.ts:631-638` exposes `BuildingSiteCard.shophouses_built` and `.trading_house_owner` with no caveat, but `build_site_index` clones the **pristine registry** card (`screen_model.rs:1775-1784` over `rules.building_sites`), so they are structurally always empty on that endpoint — verified on the committed payload `web/src/wire/index_sites.json`: 24 faces, `shophouses_built` ∈ {(None,None,None,None)}, `trading_house_owner` ∈ {None}. Live occupancy lives on `SiteModel` in the board frame (`screen_model.rs:2045-2053`). An implementer will reach for the index fields first and ship a wrong answer. **Fix: document the emptiness on `types.ts:631`, or split the index face into a `PrintedSiteCard` carrying only `spaces` + `trading_house_reward`.**

---

# G. MEDIUM and LOW findings

**MEDIUM**

- `client.rs:352-355` — `autosaved: true` hardcoded; `App.tsx:77` prints "· saved" in the banner three lines above `App.tsx:116-124`'s `APPLIED BUT NOT SAVED` warning. False in three reachable states (`persist` fails; the known undo defect; every `input` that changes neither `events.len()` nor the lock — i.e. the whole sub-prompt state machine plus `privacy`). **Fix:** `autosaved: warnings.is_empty()`, threaded from `input`'s persist result, or drop the field.
- `Decision.tsx:148-157` — priced rows state the price **twice, in two formats**: `row.words` already embeds it for `MovePresident` (`describe.rs:895`), teleports (`:862`) and both placements (`:847`, `:859`), and the component then appends `feeWords(row.fee)`. A teleport renders `Move President directly to Bank (A05) — free teleport` **`free`**. The CLI does not do this because `decision_render::move_row` re-words exactly these rows and its comment names this precise failure (`decision_render.rs:812-813`: *"the fee clause could only ever have said 'free' a second time (`— free teleport · free`)"*). **No committed fixture exercises a priced `MovePresident`** — I checked all 41: the only priced row in the corpus is a `PlacePresident` with `total: 0` — which is why no test catches it. **Fix:** call the CLI's `move_row` path, or suppress `fee` when `words` already carries it.
- `App.tsx:37-53` + `lib.rs:362-372` — a 409 prompt-mismatch **permanently strands the game**. The server's own message says *"Re-read the game and answer what it asks"*; the client has no re-read. `expect` is always `state.prompt.kind`; the catch does not re-fetch; `api.get` is called from exactly one site (`NewGame.tsx:191`); `App.tsx:55` is unreachable once a game starts. Every subsequent click re-sends the same stale `expect`. **The only escape is a full page reload into the setup screen.** **Fix:** on any `ApiError` from `input`, fall back to `api.get(state.id)` and re-seat the state.
- `web/src/api.ts:19-35` — the body is parsed **before** `res.ok` is checked and `JSON.parse` is unguarded. Behind Cloudflare Access (the documented deployment), an expired session returns an HTML interstitial and the player sees `SyntaxError: Unexpected token '<', "<!doctype "…`; the status code — the only actionable fact — is discarded. This is exactly the failure `lib.rs:734-741` says it built the JSON-404 fallback to prevent, re-opened for every response the server does not generate. **Fix:** check `res.ok` first; wrap the parse.
- `games.rs:292-296` → all ten handlers — a store failure becomes **404 "no such game"**. A read-only disk, a full disk, an EMFILE, or an [A3]/[A5] monotonicity refusal answers `404 {"error":"Permission denied (os error 13)"}` for a game `NewGame`'s list still shows. `input` goes to great length (`lib.rs:378-386`) to say a failed save is not a failed move; `handle` on the same store contradicts it. **Fix:** `NotFound` → 404, everything else → 503 with the store's sentence.
- `repl.rs:345` `PUMP_MOVE_CAP: usize = 200` + `games.rs:292-296` — **a GET is not idempotent on a cap-halted game.** `lib.rs:462-463` claims *"the same GET twice is idempotent"*; `moves` is local to each `pump_bots_with` call, so a rebuild after a cap halt plays 200 more moves and **persists them**, and the next GET plays 200 more. The other two halts (`DeadEnd`, lockdown) genuinely are idempotent. **Fix:** refuse to persist a pump that ended in a `BOT HALT`, and surface it as a warning; or state the exception.
- `games.rs:331-340` — the monotonicity guard has **no notion of which game it is guarding**. An unparseable stored record falls through both `if let`s and is silently overwritten — exactly the artefact `store.rs:264-275` measured and that `create` refuses to replace. And `seed`/`config`/`choices` are never compared, so a handle for a *different* game may publish over this id whenever its event count is ≥ the stored one. `games.rs:322-325` concedes the guard is "not a compare-and-swap" but frames the residual as two writers moving forwards; overwriting a different game is a strictly larger claim, stated nowhere. **Fix:** compare identity; treat an unparseable record as occupied.
- `Dashboard.tsx:30-47` — the panel dispatch ends `return null`. A fifth `screen_model::Panel` variant, or a rename of one of the four, makes **a hand, money, goods or track panel vanish with no message, no placeholder and no console output**. This is the M2.5 silent-fallback hole in its most consequential location. The right pattern is three files over: `Surfaces.tsx:287-299`'s `yieldWords` ends `return JSON.stringify(y);` *"so a new yield shape renders as its own JSON instead of silently as nothing."* Two siblings are worse (fatal rather than silent, currently unreachable, no error boundary in the tree): `Decision.tsx:47-54` `choiceWords` narrows by elimination into `head.CustomsSpace.cost`; `Index.tsx:168-194` falls through to `face.Site.index`.
- `Decision.tsx:291-299` — the "is this a refusal" heuristic is `/illegal|refus|cannot|not legal|out of range/i` and **misses the REPL's own `!` marker**. Seventeen refusal sites in `repl.rs` (302, 1636, 1643, 2113, 2132, 2170, 2174, 2189, 2210, 2225, 2250, 2286, 2294, 2304, 2344, 2359, 2362) match none of it — `! finish or cancel this prompt first`, `! pick a number 1-5`, `! A05 is not one of the eligible areas`. Every one is reachable from the browser's own buttons. **Fix:** test for a leading `!` first.
- `Decision.tsx:264-276` — **Confirm is rendered on the `pick` prompt, where it is never a valid answer.** `repl.rs:2087-2133` (`step_pick`) has no `confirm` arm; the `_` arm answers `! finish or cancel this prompt first`. Reachable in ordinary play (a `place_b` Telegram bonus area, every `opens_picker` row). Same shape on retreat: `step_retreat` accepts `confirm` only when `collected_len >= areas_len`, but Confirm renders unconditionally while `prompt.remaining > 0` — the component already has that guard and uses it on the number buttons at `:181`. **No test presses either** (`harness.ts:169-172` short-circuits on `if (screen.options > 0)` before reaching the Confirm branch).
- `lib.rs:421-425` — the sub-prompt exemption is over-broad, re-introducing the defect `lib.rs:405-419` says it removed. Every branch of `Repl::sub_prompt_text` (`repl.rs:2424-2540`) ends with a caret line the browser renders as a message — `> type an area id to toggle, 'confirm' to place, or 'cancel'` — instructing the player to *type* a grammar with no text input; and on `pick` the numbered `[n]` list is drawn **twice**, once as buttons and once as dead text. The `-> legal —` verdict and the fee note *are* the justification and should stay. **Fix:** strip the trailing `> …` line and the Pick `[n]` block, keep the verdict.
- `NewGame.tsx:31-33` — `api.list().then(setSaved).catch(() => setSaved([]))`. A directory-read failure renders as **"none yet — a game appears here as soon as it starts"**: a player whose games exist is told they do not, and the offered recovery (start a new one) then fails with `a game named … already exists` and no explanation. `App.tsx:29-35`'s sibling swallow for `art` **is** correctly justified (a 404 is a supported state); the `labels` swallow beside it is not — a failed `/api/labels` silently degrades every worded surface to raw Rust identifiers (`CopperMine`, `AdditionalActionFirst`, `own_money` for all 34 features) with no indication anything is wrong.
- No `spawn_blocking` or `block_in_place` anywhere in `crates/yokohama-web/src` — `suggest` (`lib.rs:528`), `why` (`lib.rs:589`) and `Games::handle`'s file I/O + full replay all run on the tokio worker pool. N concurrent `GET …/suggest` parks every worker. Reachable cross-origin (a simple GET needs no preflight); the attacker cannot read the answer but can spend the CPU and, on `GET /api/games/{id}`, advance a bot turn and write the store. Worse hosted, where the documented targets bill per running millisecond.
- No `Host`/`Origin` check. `main.rs:18` binds `127.0.0.1` (correct, and worth stating), but nothing validates `Host`, so a DNS-rebinding page reaches the local server as same-origin and reads every payload — seed included ([A6]). **Absent CORS is the right default** and needs nothing added: cross-origin reads are blocked, every mutation is a JSON `POST`/`DELETE` needing a preflight that never gets an answer, and `axum::Json` rejects a `text/plain` form-post before a handler runs. It is the `Host` check that is missing.
- No identity, no authorization, no per-user partition. `viewing_seat` (`client.rs:226-238`) is a pure function of game state — there is no seat parameter anywhere on the API, so a client cannot ask for another seat's view *or its own*. `list_games` (`lib.rs:275`) returns every stored id in one global namespace. In a 2-human table the payload follows the **actor**, so as the turn passes the same URL hands each browser the other player's hand — the CLI's hotseat model on a network socket. `PLAN.md`'s Named limitation 1 states this honestly, so this is a blocker to record, not a false claim; but `PLAN.md:375` and `m2.md:163-167` describe "identity from a configurable header" as *this crate's* story and **none of it is built**.
- `Index.tsx:132-144` renders `SpaceReward` in a private vocabulary (`1 movement`, `1 warehouse tokens`) that `Board.tsx:91-98` argues against in its own comment (*"The CLI's alphabet, not one of our own … A browser inventing `m` and `w` gives a player cross-reading the two surfaces an unknown token"*). And `SpaceReward.movement` is the *assistant*-move icon (`board.rs:109-111`), so "1 movement" reads as President movement. The generated class names (`chip movement`, `chip warehouse_tokens`) have no CSS rule.

**LOW**

- `grammar.rs:56-63` — `more` and `list` are admitted and are pure no-ops on this wire. `Repl::option_rows` (`repl.rs:721-726`) and `pick_option` (`:1144-1150`) are both unpaged; `self.page` is read only by `prompt()`, which `lib.rs:421-423` strips. Widening the door for a feature the client cannot have. **Fix:** drop both from `HEADS`.
- `store.rs:151` — `FsStore::write` fsyncs the temp file but not the **directory** after the rename, so a power loss can lose the most recent move even though the write returned `Ok`. Bounded (rename is atomic; the old record survives), but `store.rs:110-113` claims the discipline closes the crash window and it closes it one step short.
- `store.rs:192-198` — `MemStore` validates ids only on `write`; `read`/`exists`/`delete` do not, while `FsStore` validates all four via `path()`. Since every `AppState::in_memory()` test runs on `MemStore`, a regression dropping `is_safe_id` from `FsStore::read` would be caught by no in-memory test; the parity test at `store.rs:344-355` checks only the write half.
- `games.rs:200` — `Games::deleted` is unbounded (one id per DELETE, cleared only by a same-name create). Trivial next to the 27.6 KB/game `live` map, but it is the second unbounded map in the struct with no bound documented.
- `games.rs:331-332` — `persist` reads and fully deserializes the stored record on **every applied move** purely to compare a length. At end-game that is a multi-hundred-event `GameRecord` parsed per click, on top of the pretty-print.
- `grammar.rs:69-75` — token *size* is bounded (16 chars) but token *count* is not; the only bound is axum's 2 MB body limit, so a ~700k-token `line` reaches `Repl::step`. Not a leak and not a write, just unbounded parse work per request.
- `Explain.tsx:315-317` and `:358-360` — `SuggestView`/`WhyView` render `null` when both `ok` and `refusal` are null, after `Surfaces.tsx:159` has stopped showing `loading…`: a titled panel with an empty body. Unreachable today; same silent-arm shape as the `Dashboard` finding.
- `Surfaces.tsx:111-112` — an unregistered surface name leaves `loading…` on screen forever. Unreachable today (`SurfaceName` is a closed union).
- `Decision.tsx:323-324` — `prompt.kind === "reveal"` renders no answerable control at all. Unreachable (the web creates no live games) but it is a declared prompt kind with a dead-end UI.
- `types.ts:643` narrows `CardFaceModel::Order.printed` from `Option<OrderCard>` to non-nullable — harmless because `build_order_index` always writes `Some`, but it is the one nullability narrowing resting on a producer's behaviour rather than a type.
- `Index.tsx:41` handles an `IndexGroupKey` variant (`"Goods"`) the server has never had (`screen_model.rs:1463-1471` is `Country | Category | All`), and would look it up in the wrong map.
- `outcome.rs:54` and `:81-83` say the margin is *"against the best OTHER seat by rank"*; the code takes `max` over other seats' totals (`:84-90`). These coincide only because `scoring.rs:59-62` sorts by `Reverse(b.total)` first — a coupling nothing in the web crate asserts.
- `ClientState.deck_remaining` is on the wire and rendered by nothing (the two `.tsx` hits read the *panel* fields). Redundant with the end-trigger string, not a gap.
- `.tile { overflow: hidden }` + a fixed `aspect-ratio` means a tile whose content exceeded 167.75 px would silently lose its bottom row. Not reachable today (content maxes ~115 px); a clip with no failure mode.
- `CLAUDE.md:30` — *"all three carry `#[serde(deny_unknown_fields)]`"*; **five** do. The property holds; the count is stale by two.
- `m3-spec.md:113-116` — *"One test per body"*; `IndexQuery` has none (`lib.rs:1227` tests an unparseable *value*, not an unknown key). Four of five.
- `m3.md` §7's bundle re-verification uses three probes, one of which (`index_filtered`) occurs only as an import path in `contract.ts` and never in any fixture's JSON *content*, so it cannot detect corpus bytes in `dist/`. The other two are real content probes and the claim holds — all four probe strings re-verified at zero hits in `web/dist/`.
- `docs/temp/webresearch/HANDOVER.md:219` — *"guidingpath has TWO rows"*; there are **three** 2026-08-31 rows. (Untracked file, hence LOW; its tracked twin is [B17].)

**Security clean bills, checked and fine:** path traversal is closed structurally in all three places — `store.rs:39-44`'s `is_safe_id` allowlist enforced inside `FsStore::path` so all four methods funnel through it (`%2e%2e%2f` decodes to a literal `../etc/passwd` and is refused by absence, not enumeration); `assets.rs:35-46` refuses any component that is not `Component::Normal`; `art.rs:113-119` reuses that rather than restating it. Request-DTO validation is complete: five bodies fenced behind `JsonBody`/`QueryArgs` converting axum's 422/plain-text rejection into this API's 400/JSON, plus `config_of`'s eight checks and `WireSeed`'s typed parse — I found no missing check. **`save <path>` cannot reach `Repl::step`**: I enumerated every word `is_gameplay_line` admits and traced each into the dispatch; no admitted line reaches the filesystem, network or process (the residual `grammar.rs:14-17` already names — three call sites are safe only because `save_path` is `None` on both construction paths, and nothing in `yokohama-web` asserts it; a one-line `debug_assert!` would pin it). **`more` cannot divide by zero** (`page_bounds` always pushes at least one pair) — which matters, because a panic there poisons the per-game `Mutex<Repl>` and bricks the game for the process. **The deck never reaches the wire as identities** — one hit for `order_deck` in the whole crate and it is `.len()`. **`suggest` and `why` both run on the seat's own `PlayerView`** (`suggest.rs:391`, `bot.rs:604`), and `ensure_offered_by_the_true_state` re-checks against the real legal list. **The card index does not open a second hand hole** (`repl.rs:1735` gates `OwnHand` behind `hides_hand`). The atomic write, unique temp and inert leftovers in `store.rs:132-157` are correct.

---

# H. The milestone split

## H.0 The one thing to fix before either milestone: the review harness itself

Three of the nineteen HIGHs are the *same* defect class recurring for the third milestone in a row ([B11]/[C1], [B12]/[C2], [A2]/[B6]), and the lane's own recorded lesson predicts it. Before M4 starts, spend **half a day** on:

1. `it_explains.spec.ts`'s missing `haveBrowser` guard, **plus a check that every `e2e/*.spec.ts` carries it** — a per-file convention that only holds by memory is how this recurred. ([A2])
2. The `EVERY_STATE`/import-registration mirror on the client (`import.meta.glob` vs `Object.keys(wire)` vs `EVERY_STATE`) — the server side is already pinned in both directions; the client is pinned in neither. ([C9])
3. Make `every_prompt_variant` derive from `prompt_ordinal` rather than be counted beside it. ([C1])
4. Fix `play_a_whole_game`'s round-trip to read through a second `AppState`. ([C2])

Without these four, M4's own claims about M4 will be the fifth and sixth items on this list. **Cost: S.**

## H.1 Defect fixes that must precede both (one wave, before M4 opens)

These are not milestone work; they are the lane's correctness floor. Grouped because they share files and one test session.

| Item | Why now | Cost |
|---|---|---|
| **[A1] stamp the digest in `persist`** | Silent data destruction of any CLI save dropped in the store, and an engine safety refusal that is dead on every hosted record. One line + one test. | S |
| **[A3] + [A4] + [A5] the three `Games` concurrency holes** | They interlock — the same three-line fix region (`create`'s reservation, `persist`'s identity compare, `handle`'s staleness). Fixing one and not the others leaves the same class reachable by a different door. Includes the `NewGame` double-click ref guard, which is the *realistic* trigger for [A4]. | M |
| **[A6] drop the seed from the play payloads** | The single design gate on ever adding a second seat, and it gets more expensive with every fixture regeneration. Do it while the corpus is 41 files. | S |
| **[A7] cap `/why`'s ranking at the CLI's `RUNNER_UPS_SHOWN`** | One line restores parity and closes the hand leak in the app's primary use case. The proper information fence can wait. | S |
| **[G] `autosaved`, the 409 dead end, `api.ts`'s parse-before-ok, `Games::handle`'s 404** | Four small fixes that together decide whether a player can *tell* the above went wrong. The 409 fix (`send` falls back to `api.get`) is also the mechanism M4 needs. | S |
| **[C8] one route test + one component cell for DELETE** | The only destructive path in the product and it has zero coverage at any level. | S |
| **[B1]–[B17] the doc corrections** | Cheap, and each one currently tells the next reader something false about a fence. Batch as one commit. | S |

**Total: S+M — call it one focused wave, roughly the size of M2.5.** Do not fold this into M4; the `Games` fixes are the kind of thing that wants its own green suite.

## H.2 M4 — the replay scrubber

I have no evidence about the scrubber's own design, so I will not price it. What I *can* say from the evidence: **M4 is not short, and the honest reason is that the scrubber lands on top of the two weakest parts of this lane.**

- A scrubber reads history. `GameRecord.events` is on the server; `ClientState` carries none of it beyond `log`, and `Session::state_at` is the reconstruction path `why` already uses (`record::state_at` + the same `decide` key). So M4 needs a *new route shape* — a state at index N — and that route lands squarely in the concurrency region [A3]/[A5] governs, because a scrub is a read that must not be confused with a play.
- A scrubber is the first thing in the client that **re-reads the game**. Today `api.get` is called from exactly one site (`NewGame.tsx:191`) and there is no refresh path at all ([G] 409). M4 has to build that machinery — which is why the 409 fix belongs *before* M4 and not inside it: M4 gets it for free.
- A scrubber is the first client surface with a **timeline** to lay out, on a page that is already 2.7 viewports tall with nothing sticky. Dropping a scrubber into that page without P1 gives it a control the player must scroll past the board to reach.

**So M4 = the scrubber + the state-at-N route + the client's first re-read path.** My honest read is that this is a full milestone, comparable to M3, and calling it "short" would be convenient rather than true.

## H.3 M5 — the screen

The owner's framing was "usability/art could be tasked as an M5 but if M4 is short then we could fit this in the same wave." **M4 is not short. But two items should still ride with M4, because M4 is worse without them:**

**Ride with M4 (small, and M4 depends on them):**
- **P1, pin the decision zone** — one CSS rule + one class rename. Without it the scrubber's controls compete with the answer buttons for a screen position neither has. **S.**
- **P2 + P3, delete the tile `aspect-ratio` and shrink the rail art** — three edits, takes the stage from ~1610 px to ~1150 px, and gives the scrubber somewhere to live. **S.**

**M5 — "the screen": usability, parity closure, art.** This is a real milestone and should be scoped as one, because the three strands share files:

1. **The board fits.** P6 (scale `--tile-w` by tile count; the arithmetic is exact — 4P fits at 10rem, 3P at 12rem) + the scrollbar affordance + fixing test C7 to measure the widest *element*. **S–M.**
2. **The art reads.** F.1's primary fix (drop `--art-veil` and the `.tile-body` gradient; caption below the art) — which **subsumes P4**, because with the printed power ladder legible the biggest modal-only gap closes for free. Plus [B16]'s stylesheet doc corrections. **M.**
3. **The surface stops covering the board.** P5, the right-hand drawer — one CSS rule, zero component changes (verified: every panel's grid is `auto-fill` or `width: 100%`) — plus `inert` on `.stage` and the focus-trap/restore fix, plus P7's nav collapse. **S–M.**
4. **Parity closure, the free half.** The six items in §D.1 that are already on the wire: canal crossings, the Areas surface's three missing blocks + row grouping, `hand_hidden`, near-miss area names, seat colours in the game-over block and the turn line, the site-index space grouping. Plus the `Tracks` companion pointer, which currently points at a surface that does not exist ([D.2]). **M** — it is a dozen small edits, not one.
5. **The two missing surfaces.** A `tracks` surface (which also converts §D.1's tooltip-only per-space rewards into visible text) and the per-area `sites` surface (parity audit L53, still open). **M.**
6. **The site-card occupancy overlay** (F.3): `SiteCardView` widened to take `SiteModel`, five measured rects, `⌂` in the builder's seat colour, on the index cell and a tile popover. Plus the 5-Power token composited onto the printed 5-space — **gated on item 2**, because the target rect is currently at 9–22 % transmittance. **M.**
7. **Accessibility floor:** a live region on the four message channels, a `<main>` landmark, the refusal-marker fix, Confirm hidden where it is not answerable. **S.**

**Sizing honestly: M5 as scoped above is larger than M3.** If it has to be cut, the order that preserves the most value per unit of work is: **1 → 3 → 4 → 2 → 7 → 6 → 5.** Item 2 (the art) is what the owner asked about and is genuinely small in diff, but it is sequenced fourth because it changes tile height and therefore wants items 1 and 3 already landed.

## H.4 Deferrable, explicitly

- **The four §D.3 items genuinely not on the wire** (area-action gloss, Chinatown rates, church/customs majority awards, other seats' hands). Three need a server change and the fourth needs a masking decision. Not before multi-seat.
- **Identity / authorization / per-seat views.** Recorded as a blocker ([G]), correctly named in `PLAN.md` as "recorded now, acted on later". But note the dependency: **[A6] must land before this, not with it** — publishing the seed makes every per-seat masking decision moot.
- **`spawn_blocking` on the two search routes.** Real, but a single-operator server does not feel it. It becomes urgent the same day hosting does.
- **The `sites/S00..S23` rename** (F.2). Correct and cheap (three files, no client change), but its value is preventing a future YAML reorder from silently re-pointing 24 pictures — do it opportunistically, in whatever wave next touches `mkcardart.py`.
- **`backs/` and `tokens/`** (F.2): 14 unreferenced files, two dead exports. Wire up the eight tokens as part of M5 item 4 (the good chips want them) or delete both; do not leave them.
- **The 545 px phone measurement** (§E.6). Settle it with one `page.evaluate` when C7 is fixed; do not spend a session on it standalone.

---

# §III — The completeness critic

## 1. What no lens read

Checked every file under `crates/yokohama-web/` and `web/` against the report's citations. **Ten source files are cited by nothing:**

| uncited | read now? |
|---|---|
| `crates/yokohama-web/Cargo.toml` | yes — finding §2d |
| `web/package.json` | yes — §2b |
| `web/vite.config.ts` | yes — clean bill below |
| `web/vitest.config.ts`, `web/tsconfig.json`, `web/tsconfig.node.json` | yes — clean |
| `web/src/main.tsx` | yes — 12 lines, nothing |
| `web/src/test/{setup.ts,panels.ts,harness.tsx}` | yes — clean |
| `web/src/art.test.ts` | yes — clean |
| `web/src/Board.test.tsx`, `Dashboard.test.tsx`, `Decision.test.tsx` | yes — **one finding, §4c** |
| `crates/yokohama-web/assets/art/manifest.json` (as data) | yes — §2a |

Also uncited: `index.html` is referenced once (for `html{font-size}`) but its `<head>` was never read — §2e.

**Clean bills from the uncited set, each checked:**
- `vite.config.ts:21` proxies `/api` to `127.0.0.1:8080`; `main.rs:15` defaults to `8080`. They agree. `/art` proxied for the same reason with a written justification. Correct.
- `tsconfig.node.json:34` really does include all three config files plus `e2e` — CLAUDE.md:32's "which nothing checked before" is true and the fix is real.
- `src/test/setup.ts:23-33` removes `fetch` rather than defaulting it, and `harness.tsx:112-125` asks the caller's handler *first*. Both are the non-vacuous shape.
- **CLAUDE.md:33's "194 component tests over jsdom in 10 files" is EXACTLY right.** Counted: 114 plain `it(`/`test(` + `it.each` expansions of 2 (Board:66) + 3 (Decision:249) + 12 (Explain:191, `whyRefusals` has 12 entries) + 4 + 4 (Index:27, :192) + 20 (contract:23, `EVERY_STATE` = 20) + 26 (contract:68) + 3×3 (contract:245/250/276) = **194**. The report's "M3's own count claims are accurate" holds for the one count it did not itself re-derive.

---

## 2. Classes no lens hunted

### (a) HIGH — Licence and attribution: not considered anywhere, by anyone, and the metadata makes an affirmative claim

There is **no LICENSE file in the repository** (`git ls-files | grep -i licen` returns nothing), while `Cargo.toml:16` declares `license = "MIT"` and `crates/yokohama-web/Cargo.toml:5` inherits it with `license.workspace = true`. That crate's `assets/art/` is **131 committed WebP crops of a published board game's printed components** — orders, techs, building sites, area tiles, achievement cards, 5-Power tokens, mercenary flags.

Their provenance is recorded, and recorded *only* as a pipeline:

```json
// crates/yokohama-web/assets/art/manifest.json, "provenance"
{"card_width":360,"generator":"docs/tools/mkcardart.py","positions":"analysis-1/yokohama-kb/0{7,8,9}-*.yaml",
 "sheets":"tts-assets/organized/","tile_width":440,"webp_quality":82}
```

Every document that discusses the art treats it as a *join* problem: `docs/tools/mkcardart.py:1-70` (crop geometry, key selection, "regenerate, then look"), `CLAUDE.md:66`, `art.rs:328-369`. Not one sentence anywhere names who owns the pictures. `tts-assets/` — the source sheets — is itself tracked (118 files, present on `master` too), so this predates the lane; **what the lane added is 131 crops cut from the sheets and an MIT declaration over the crate that ships them**, plus a hosting plan.

And the hosting plan considers exactly two constraints — `docs/plans/2026-08-31-web-frontend-m2.md:167-179`: "Nothing may exist only in memory" and "Storage is behind a trait." The art is not one of them. The remote is `https://github.com/jmoore2333/yokohama-assistant.git`.

**Mitigation the owner should get credit for, and which the report never states either:** m2.md:162-165 records that `yokohama.moore.nyc` "picks up the existing Access rule by itself," so the *site* is identity-gated. The exposure is the *repository*, not the deployment.

**Fix, S:** add a `LICENSE` covering the code, plus a short `ASSETS.md` or a `NOTICE` stating that `tts-assets/` and `crates/yokohama-web/assets/art/` are third-party game components and are **not** under the workspace's MIT grant — and set `license = "MIT AND LicenseRef-game-assets"` or exclude `assets/` from the package. Whatever the owner decides, the current state declares the MIT grant over the crate that ships those components.

### (b) MEDIUM/LOW — build config

- `package.json:9` `"preview": "vite preview"` is **dead and broken**: `vite preview` reads `preview.proxy`, not `server.proxy`, and `vite.config.ts:19-23` sets only `server.proxy`. A `npm run preview` build has no `/api` and no `/art`. It is referenced by no doc and no test (`git grep "npm run preview"` → the package.json line only). Delete it or give it a `preview` proxy block.
- No `lint` script and no ESLint config, yet three files carry `// eslint-disable-next-line no-console` (`playwright.config.ts:58`, `:141`... ). Directives for a linter that does not exist.

### (c) MEDIUM — caching: the art has a header, the bundle does not

`art.rs:113` sets `(header::CACHE_CONTROL, "public, max-age=3600")` with a written reason (filenames are stable, not content-hashed, so not `immutable`). `assets.rs:79` sets **no cache header at all**:

```rust
return ([(header::CONTENT_TYPE, content_type(&path))], Body::from(bytes)).into_response();
```

That is exactly backwards from the two files' properties. Vite emits `assets/index-<hash>.js|css` — content-hashed, the canonical `immutable` case — and it is served uncached, so every reload re-downloads the whole bundle. `assets.rs:5-7` says "If asset serving ever needs range requests, compression negotiation or **caching headers**, that is the moment to take the dependency — not before," while its sibling already wrote one by hand with no dependency. One line: `immutable` for `/assets/*`, `no-cache` for `index.html`.

### (d) MEDIUM — no graceful shutdown, and the `signal` feature is enabled to prove it was thought about

`crates/yokohama-web/Cargo.toml:64` enables `tokio = { …, features = [… "signal"] }`. `grep -rn "signal\|graceful" crates/yokohama-web/src/` returns **nothing**. `main.rs:26` is a bare `axum::serve(listener, app(state)).await`.

This is a **deeper instance of the report's [B3]**, from a different file and biting at exactly the moment m2.md:171-175 is describing. Both hosted shapes it names (a Cloudflare Container, a Modal `@modal.web_server`) SIGTERM the process on scale-to-zero. Today that kills an in-flight `POST …/input` between `Repl::step` (which has already mutated the resident `Repl`) and `store.write` — the client gets no response and the move is gone, with no `with_graceful_shutdown` to drain. The report's [B3] rewrite ("a sleeping container loses every applied move it acknowledged and nothing else") is still too generous: it also loses the move it was in the middle of *not yet* acknowledging.

### (e) LOW — a third-party runtime dependency in a client that advertises having none

`web/index.html:7-12` preconnects to `fonts.googleapis.com`/`fonts.gstatic.com` and loads Fraunces + JetBrains Mono from Google's CDN. `styles.css:46-47` gives both real fallback stacks, so a block degrades rather than breaks — but `vite.config.ts:4-6` states the client "is served from the SAME origin as the API in production … so there is no CORS story," and this is the one cross-origin fetch in the product. There is no CSP. For a local-first board-game tracker, self-host the two faces (`woff2` is already in `assets.rs:58`'s content-type table, for nothing). Also: no favicon, so every load 404s `/favicon.ico` through `assets.rs:81-83`.

### (f) LOW — deployment path defaults are relative

`assets.rs:24` defaults to `web/dist` and `art.rs:92` to `crates/yokohama-web/assets/art`, both repo-root-relative. Correct for a checkout, and both name their env override. `assets.rs:88-95` gives a good error for the first; `art.rs`'s miss is a bare 404 "no such art" per file, so a container with the wrong WORKDIR renders a complete, wordless, art-free board rather than saying why. One startup check that `art_dir()/manifest.json` exists would fix it.

---

## 3. The owner question answered weakest: F.3, the 5-Power crops

The report answers the owner's most concrete *feature* request by re-defining it, and its central load-bearing claim is:

> "**So 'show each player's collected 5-Power token faces on their mat' is not implementable without a core change** (a `Vec<FivePowerEffect>` on `PlayerState` is a serde/save-shape change — STOP territory)."

The citation is exact — `crates/yokohama-core/src/types/player.rs:225` is `pub five_power_tokens: u8, // Just count, effects already resolved`. **But the conclusion does not follow, and the missing evidence is one function away.** The face is destroyed in *state* and preserved in *history*:

```rust
// crates/yokohama-core/src/game/apply_move.rs:921-925
let effect = state.board_layout.five_power_tokens
    .get_mut(&area).and_then(|slot| slot.take())
    .ok_or(MoveError::RuleViolation("No 5-Power token at the acting area"))?;
```

`slot.take()` leaves the board slot `None` — so at the event index where a seat takes its bonus, the *pre-event* state carries the face at the acting area and the acting seat is known. `record::state_at` plus a scan is exactly the machinery `why` already uses (CLAUDE.md's `why` paragraph; the report's own H.2 names it). **A per-seat 5-Power token strip is a replay-derived projection over the `GameRecord` the server already holds, with no `SEMANTICS_REV` implication and no `PlayerState` field.** It is more expensive than the report's substitute proposal and it is not STOP territory — which changes the owner's menu, because his request becomes buildable-as-asked rather than only buildable-as-reinterpreted.

The rest of F.3 (the site card is the thing that shows built shophouses; the five measured rects; the `⌂` glyph) is sound and I did not re-verify the rect measurements.

---

## 4. Claims I doubted, checked

### (a) HIGH — §E.6 is not stale. I reconciled the 545 px from the stylesheet, and it is a live layout bug the report exonerated.

The report concludes:

> "`.board { overflow-x: auto }` has been on that rule since the file's first commit … so the board's overflow cannot reach `documentElement` … **Either the number is measuring something the comment does not name, or it is stale.**"

The overflow reaches `documentElement` because at 390 px **the board column is never constrained to the viewport in the first place.** Two rules, four lines apart:

```css
/* web/src/styles.css:112-119 */
.stage { display: flex; gap: 1.5rem; padding: 1.25rem; align-items: flex-start; flex: 1; }
.board-col { flex: 1 1 auto; min-width: 0; }
.rail { flex: 0 0 24rem; display: flex; flex-direction: column; gap: 1rem; }

@media (max-width: 1200px) {
  .stage { flex-direction: column; }
  .rail { flex: 1 1 auto; width: 100%; }
}
```

The media query flips the direction and **does not reset `align-items: flex-start`**. In a *column* flex container `align-items` governs the cross axis, i.e. the width: `flex-start` means fit-content, not stretch. `min-width: 0` no longer helps — it caps the *automatic* minimum, and fit-content is `max(min-content, min(max-content, stretch-fit))`. `.board-row` is a nowrap flex row of `.tile { flex: 0 0 var(--tile-w) }`, so its min-content is the full row (4×224 + 3×6.4 = **915.2 px** at 2P), and `overflow-x: auto` does not reduce an element's intrinsic sizes. So:

```
fit-content = max(915.2, min(915.2, 390 − 40)) = 915.2
overflow = 915.2 + 20 (.stage padding-left) − 390 = 545.2  →  545
```

Exactly the recorded number, from the CSS alone. Corroboration that someone already hit this half-way: the same media block sets `.rail { width: 100% }` — the identical symptom, fixed for the other child of `.stage` and not for the board.

**Three consequences the report does not draw:**

1. The 545 px is a **one-line bug**, not an unexplained measurement: add `.board-col { width: 100% }` to the `@media (max-width: 1200px)` block (or `align-items: stretch`) and `.board`'s existing `overflow-x: auto` engages, taking the phone overflow to 0. It is the cheapest item in the whole review.
2. **C7 is worse than "insensitive" — the gated cell would go RED at 3P/4P.** `the_screen_itself.spec.ts:130-141` asserts `overflow <= 0` at **1024**, which is *below* the 1200 px breakpoint, so the column layout is what it measures. 2P passes with 89 px of headroom (935.2 < 1024); 3P is 1165.6 and 4P is 1396. So the repo's only overflow gate is scoped to the one player count that clears it, and the fix in (1) is a prerequisite for widening it.
3. The report's own `.rail`-vs-`.board-col` arithmetic for the **row** layout is correct — I re-derived `1440 − 40 − 24 − 384 = 992` from lines 112-114, and the 1201–1363 px dead band checks out (a 2P board needs a 1363.2 px viewport to fit uncut, and 1201 px is the first width that does not stack). Clean bill on §E.1.

I derived this by reading; a browser settles it in one `page.evaluate` — first item on the shot list.

### (b) Clean bill — the "222 vs 186" apparent contradiction in [A7] resolves

I doubted [A7]'s "`web/src/wire/why.json` `ranking` is **222** rows" against [C6]/CLAUDE.md:86's "**186**". Measured both:

```
why.json      ok.ranking → 222
suggest.json  ok.ranking → 186
```

Two different fixtures at two different decision points. [A7]'s 222, [C6]'s 186 and CLAUDE.md:86's "row 1 of 186" are all correct and all about the payload they name. The `/why` leak finding stands at 222.

**Two more of the report's fixture-derived counts, re-measured:** §D.2's "`areas.json` carries **2** `Tracks` companions and `areas_4p.json` carries **2**" — exact (`['Tracks','Ports','Labs','Tracks']` and `['Tracks','Labs','Labs','Tracks','Ports','Ports']`). §F.2's 131-entry manifest ↔ 131 files bijection — exact (12+14+6+6+5+36+24+20+8). `backArt`/`tokenArt` zero callers — confirmed, the only hits are their own definitions at `art.ts:46` and `:48`.

**One precision correction to [A6]:** "21 of 41 files … carry it, **both** as `seed` and as `frame.title.seed`." Measured: 21 carry root `seed`, **20** carry `frame.title.seed` — `preview.json` has the root field and no frame. Does not move the finding; the golden will show 21 root diffs and 20 title diffs.

### (c) MEDIUM/HIGH — a test whose *name* claims the coverage it lacks, in a file no lens read

`web/src/Dashboard.test.tsx:158-170`:

```tsx
it("say where each president is, in both arms", () => {
  show(wire.options);
  for (const p of playersOf(wire.options)) {
    if (p.president === "InHand") {
      expect(screen.getAllByText("President in hand").length).toBeGreaterThan(0);
    } else {
      expect(screen.getAllByText(`President at A${String(p.president.OnBoard).padStart(2,"0")}`).length)
        .toBeGreaterThan(0);
    }
  }
});
```

`wire.options`' two seats are **both `InHand`** (measured on the committed file: `[(0,'InHand'), (1,'InHand')]`). The `else` branch never executes. `Dashboard.tsx:70-71` is the only producer of either string:

```tsx
if (p.president === "InHand") return <>President in hand</>;
return <>President at A{String(p.president.OnBoard).padStart(2, "0")}</>;
```

and `grep -rn "President at" web/src web/e2e` finds it in exactly two places — that line and this test's dead branch. **Line 71 is executed by no test in the repo, and the test that would catch that is named "in both arms."** Same signature as [C5] and [C3], in the tenth file.

The fix is one word: **seven committed fixtures already carry an `OnBoard` president** — `pick.json`, `retreat.json`, `retreat_answered.json`, `fee_nonzero.json`, and all three `choice_*.json`. `show(wire.pick)` in a second cell closes it. That the payload is already sitting there makes this an oversight rather than a limitation, which is the useful part.

*Lower-grade sibling in the same uncited set (LOW):* `Board.test.tsx:113-119`'s busy-guard loop asserts only inside `if (/^A\d\d /.test(b.getAttribute("aria-label") ?? ""))`, so a change to the tile's aria-label format empties the loop. Bounded — the neighbouring cell at `:101-108` pins the same label format via `getByRole({name: /^A02 /})`, so the two would fail together. Recording it, not proposing work.

---

## 5. The shot list — what only a browser can answer

The report's §E and §F are almost entirely *derived* from CSS constants, and §4(a) is the proof that derivation missed a live bug. Every item below is a measurement, not a look.

**Layout / overflow (settles E.6, C7, and my §4a):**
1. `page.evaluate` at 390×844, 1024×768, 1200, 1201, 1280, 1363, 1440 — return **the widest overflowing element's selector plus its `scrollWidth`/`clientWidth`**, not a `documentElement` delta. Repeat at 2P / 3P / 4P. This is one function and it settles the 545 px, the dead band, C7's fix and the 4P clipping in a single pass.
2. Screenshot 4P at 1280 (the config default the 4P test actually runs at) with a red outline on `.board` — the report predicts 544 px clipped and unasserted, behind macOS overlay scrollbars.
3. Before/after screenshot of the one-line fix (`.board-col { width: 100% }` inside the ≤1200 media block) at 390 and at 1024/4P.

**The stage height (settles E.1, P1–P3):**
4. `getBoundingClientRect()` on `header.banner`, `.end-strip`, `.stage`, `.board-col`, `.rail`, `.decision` at 1440×900, 2P with **one card in hand** and again with **four** — the report's ~1610 px stage and "y ≈ 1700 px answer buttons" are computed, and the rail's card-art total is the term that moves with hand size.
5. Full-page screenshot at 1440×900 showing where the fold falls relative to the answer buttons. This is the owner's actual question and no image of it exists.
6. The same rect set after P2 (`aspect-ratio` deleted) and P3 (rail art shrunk) — the report claims ~1610 → ~1150.

**The art (audits F.1's numbers, which are the only ones already measured):**
7. Re-run the white-vs-black difference at 1440×900 on a **dense** tile (token + site strip + assistants) and on an **empty** one; the report's 0.304 mean / 0.715 max / 52.7 %-below-0.25 profile does not say which it measured.
8. Crop the printed power-ladder band (y 0.61–0.97) at 1× and at 2× DPR and read whether the rungs are legible — the claim that drives F.1's primary fix and P4's redundancy.
9. Screenshot the F.1 fix (veil + gradient deleted, caption below the art) at `--tile-w` 14rem and 16rem, at 2P and 4P, so the height cost is visible beside the legibility gain.

**Accessibility and the modal (E.4 — every item is behavioural, none is readable from CSS):**
10. Tab from an open `.surface` and screenshot where focus lands — the report predicts it walks into board tiles under the panel despite `aria-modal="true"` (`Surfaces.tsx:144`).
11. Trigger a `setState` in `App` while a surface is open and record whether focus is yanked back (`App.tsx:152`'s inline `onClose` identity).
12. Escape from a surface and report `document.activeElement`.
13. VoiceOver/axe pass on the four message channels — the report found zero `aria-live`/`role="status"` and no `<main>`; a screenshot of the a11y tree is the artefact.
14. At 1024, screenshot whether the nav wraps to a second banner row and whether that row sits **under** the `inset: 3rem` panel (`styles.css:351`).

**Two things a browser is the only way to falsify:**
15. `/api/games/{id}/why` on a real 2P game against the bot: capture the payload and confirm the 222-row `ranking` names cards in the bot's hand. The report flags its own caveat — "I could not capture a live instance." One `curl` behind the e2e server closes [A7]'s only gap.
16. Network panel on a cold load and a reload: count requests and bytes for `/assets/*` versus `/art/*`. Settles §2c's claim that the content-hashed bundle re-downloads while the stable-named art does not.