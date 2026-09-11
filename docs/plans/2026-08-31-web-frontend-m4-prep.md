# Web frontend — M4 prep: "it remembers"

**Written 2026-08-31 on branch `web-frontend`, at the Wave 0 head (`513484d`).**

> **THIS IS NOT AN APPROVED SPEC.** The owner has approved no design for M4.
> Everything below either (a) states a fact about code at this head, with a
> `file:line` a reader can open, or (b) proposes something and says so. §3 lists
> the few things that ARE settled and by whom; §9 lists the forks only the owner
> can close. Nothing here authorises a line of code.

Reading order: `2026-08-31-web-frontend-m2.md` → `-m2.5.md` (§5, what the harness
does NOT cover) → `-m3-spec.md` → `-m3.md` §9 (the six-lens review) →
`-2026-08-31-web-frontend-wave0.md` (the correctness floor this sits on) → here.

**Every load-bearing claim in this file was re-verified against the source tree
at this head, not carried from the reconnaissance.** Where the research and the
code disagree, the code wins and the disagreement is recorded in place, with
`[CORRECTION]`. Where a claim could not be settled from source, it is marked
UNVERIFIED rather than repeated. This is the lane's third document written under
that rule, and the reason is `wave0.md` §0: twelve of the review's nineteen HIGH
findings were sentences this lane wrote about itself.

---

## 1. What M4 is, and where it sits

**M4 is the replay scrubber.** A control that walks the game backwards and
forwards over its own event log and draws the position at each stop, served by a
new route `GET /api/games/{id}/at/{k}` over `record::state_at`.

**The saved-game half is already done.** M2 shipped the list and the load:
`GET /api/games` (`crates/yokohama-web/src/lib.rs:777`), `GET /api/games/{id}`
(`:779`), and the client's saved-games section
(`web/src/NewGame.tsx:196-224`, calling `api.list()` at `:32` and `api.get(name)`
at `:208`). M4 adds no persistence and no new state. It is a projection over a
`GameRecord` the server already holds.

**M4 comes AFTER M3.5 by owner ruling** (`wave0.md` §7): *layout before the
scrubber, because the scrubber's host is the one M4 decision the code cannot
answer without knowing the layout.* That ruling is load-bearing here — §5.1 is
written as a question that M3.5 answers, not as a choice this document makes.

The measured basis for it, at 1440×900, 2P, recorded in `wave0.md` §7: the page
is 1950px / 2.17 screens, `.rail` is 384×1534 against a 992×728 `.board`, and the
decision zone starts at y=1703 — 803px below the fold. I did not re-measure those
figures (no browser run this session); they are the M3.5 record's to defend.

---

## 2. The seam

### 2.1 `state_at`, exactly

```rust
// crates/yokohama-core/src/game/record.rs:561
pub fn state_at(record: &GameRecord, k: usize, rules: &Rules) -> Result<GameState, ReplayError>
```

Re-exported to the crate root by `game/mod.rs:136` (`pub use record::*`) and
`lib.rs:10` (`pub use game::*`), so callers write `yokohama_core::state_at`.
`yokohama-cli`'s `why.rs:418` already calls it that way.

**`k` counts events APPLIED, not an event index.** The function's own doc says so
in as many words (record.rs:524-536) and every property follows from it:

| k | means |
|---|---|
| `0` | post-setup, nothing folded — the position the FIRST event was decided in |
| `n` | the position event `n` was decided in; event `n` is the first NOT applied |
| `events.len()` | the game as it now stands — identical to a full `replay` |

So the domain is `0..=len`, **inclusive at both ends**, and the top position has
no event to name. This matters for the scrubber's right-hand endpoint: a track
over `0..=total` has `total + 1` stops, and the last one is the present.

**It never clamps.** `record.rs:562-571`: `k > available` returns a typed
`ReplayError` with `event_index: available` — the past-the-end boundary, not a
failing event — and a message that says "state_at never clamps" out loud. The
range check runs BEFORE the setup preamble (`:572`), so an out-of-range `k` is
reported as such even on a record whose setup would also fail.

**`ReplayError` is a struct, not an enum** (`record.rs:324-327`: `pub event_index:
usize`, `pub message: String`). A caller gets an index and a string and **cannot
match on a variant**. Worse, the past-the-end convention is shared with
`replay_strict`'s terminal-digest mismatch, and the struct's own doc (`:317-322`)
states the consequence: *"A caller branching on `event_index` alone cannot tell
those from a last-event failure — the message text is the discriminator."*
**A route that wants to answer 404-vs-500 correctly must compare `event_index`
against `record.events.len()` itself**, which it knows, rather than parsing a
message. That is one line and it must be written deliberately.

**What it does NOT verify.** Its doc (`record.rs:551-556`) is explicit: unlike
`replay` it prints no drift warnings, and unlike `replay_strict` it runs no
per-event invariant checks and refuses nothing on `rules_hash` or
`semantics_rev`. *"A caller that needs those verdicts asks for them by calling
the function that renders them."*

For `/at/{k}` that is fine, and the reason is not obvious enough to leave
implied: **a game can only be resident because it already passed those gates.**
`Games::handle` (`games.rs:333`) builds every non-resident game through
`Session::from_record` (`session.rs:218`), which refuses a missing or mismatched
`semantics_rev` (`:238-260`) and a terminal-digest mismatch (`:365-378`), and
returns `engine_version`/`rules_hash` drift as `load_warnings` (`:281-291`).
`/at/{k}` needs no extra provenance check. It should not invent one.

**Cost.** `replay_from` (`record.rs:384-392`) folds `fold_one_event`
(`record.rs:348`), whose `Move` arm is `apply_move` — the checked boundary, a
full `get_legal_moves` enumeration plus a linear `contains` plus a whole-state
clone. Nothing is cached anywhere: `Session.checkpoint` exists (`session.rs:80`)
but is **private** and `None` in all three production constructors (`:144`,
`:173`, `:386`). §3.2 measures what that actually costs, and the answer changes
the milestone.

### 2.2 What a frame needs beyond it

```rust
// crates/yokohama-cli/src/screen_model.rs:2698
pub fn build_frame(state, rules, seats: &[SeatFacts], seat: PlayerId, legal: &[Move], title: TitleModel) -> FrameModel
```

Six arguments. Their provenance at a historical `k`:

| arg | at k | note |
|---|---|---|
| `state` | `state_at(record, k, rules)?` | the only genuinely new call |
| `rules` | `AppState.rules` | time-invariant |
| `legal` | `get_legal_moves(&state_k, rules)` | **one MORE full enumeration on top of the k already paid** — and not optional, see §6.2 |
| `seats` | `repl.seat_facts()` (`repl.rs:693`) | valid at every k, but **has no per-event grain** (§6.6) |
| `seat` | **not derivable — a decision** | §3.1 |
| `title` | `mode` is a record fact; `seed` must stay `None`; `autosaved` has no historical meaning | §5.2 |

**What is well-defined at k with no new decision, verified in source:** the acting
seat (`state_k.current_player` — the same value `why <n>` uses to pick a
contestant, `why.rs:420`); the legal list (and if `events[k]` is a `Move`, it is
*provably* a member, because `apply_move` checked exactly that when the record
folded); the whole `DecisionModel` (`build_decision`, `screen_model.rs:2465-2493`,
reads only `is_game_over`, `pending_choices.first()`, `turn_phase`, `turn_number`,
`current_player`); the `BoardModel`; every dashboard panel.

**What has no historical analogue:** `PromptState` (`client.rs:73-119`) is built
from `Repl::sub`, an in-memory sub-prompt. The record stores only APPLIED moves,
so an intra-move picker state is unrepresentable at any k. Omit it or synthesise
something `Watching`-shaped; there is no third option.

### 2.3 The hand is ONE panel, and `seat` is the whole exposure

This is the fact that makes §3.1 a security decision rather than a UX one, and it
took reading `build_dashboard` to see it:

```rust
// crates/yokohama-cli/src/screen_model.rs:2675-2691
pub fn build_dashboard(state, rules, seats: &[SeatFacts], seat: PlayerId) -> Vec<Panel> {
    let mut panels = vec![Players, Tracks, Achievements];
    if let Some(hand) = build_hand(state, rules, seat, SeatFacts::for_seat(seats, seat).hand_hidden) {
        panels.push(Panel::Hand(hand));
    }
    panels
}
```

**A frame carries exactly one hand: `seat`'s.** `build_hand`
(`screen_model.rs:2315-2339`) empties `cards` when `hidden` and keeps
`card_count`. `SeatFacts::for_seat` (`:147-153`) fails safe to `hand_hidden: true`
for a seat the table does not cover. So the mask is real — and it is
*seat-selected*, not seat-scoped: pass a different `seat` and you get a different
hand.

---

## 3. Decisions already settled — do not re-open these

### 3.1 Viewer seat: PIN A FIXED VIEWER (owner, 2026-08-31)

**Never follow-the-mover.** The mechanism is already in the code and is currently
unused: `build_decision_for` (`screen_model.rs:2515-2523`) branches on
`viewer == state.current_player` and hands an observer the public half of the zone
with `options: []` and `viewer_is_deciding: false` (`screen_model.rs:1000`). Its
own doc, `:2510-2513`: *"This is the seam a multi-seat API needs and the CLI never
did: one operator at one screen is always the acting seat."*

**The three citations the brief asked me to verify, all confirmed at this head:**

1. **Privacy defaults OFF in sim.** `crates/yokohama-cli/src/repl.rs:444` —
   `let privacy = matches!(session.mode, TrackingMode::Live);`. And
   `repl.rs:600-603` folds it: `self.privacy || self.bots.is_bot(seat)`. So in a
   simulation a human seat's `hand_hidden` is `false`.
2. **`bot_seats` may legally be empty.** `crates/yokohama-web/src/games.rs:83-84`
   — `#[serde(default)] pub bot_seats: Vec<u8>`. `config_of` refuses only the
   ALL-bot table (`games.rs:156-160`: *"at least one seat must stay human"*).
   Nothing refuses an all-human one.
3. **`build_decision_for`'s observer arm exists and is not called anywhere in
   this crate.** `ClientState::of` (`client.rs:329-355`) passes
   `viewing_seat(repl)`, and `viewing_seat` (`client.rs:217-229`) returns the
   ACTING seat unless the actor is a bot.

**And a fourth fact, which I measured rather than read, and which makes the ruling
sharper than the reconnaissance did:** *the API has no notion of who is asking.*
There is no auth, no seat parameter, no session cookie. Against the release binary
at this head, on an all-human 2P table created with `bot_seats` omitted:

```
created all-human 2P: seat served = 0   to_move = 0
  hand panel: seat 0  hidden False  cards [33]
after play:  seat served = 1   to_move = 1
  hand panel: seat 1  hidden False  cards [31]
```

The **same unauthenticated `GET /api/games/hotseat`** returned seat 0's card while
seat 0 was to move and seat 1's card afterwards. That is correct for one operator
at one screen — the hot-seat model the CLI is built on — and it means **a
follow-the-mover scrubber would not be introducing a new class of exposure so
much as making the existing one harvestable in one pass**: today you must wait for
a turn to change; a follow-the-mover `/at/{k}` sweep reads every human seat's
whole hand history in a second, at will, from a URL.

**[CORRECTION to the recon §4.1]**, which said *"Today's live path escapes only
because `ClientState::of` builds for one seat at a time."* It does not "escape".
It serves one hand at a time, and that hand is whoever is currently moving. The
recon's conclusion (pin the viewer) is right; its reason understated what is
already true.

**A consequence the ruling does not by itself settle, and which §9.Q1 asks about:**
pinning the viewer is a fence only if the viewer is *not* a client-supplied
parameter. A `?seat=` argument makes the pin decorative — a sweep over `(k, seat)`
returns everything. The three shapes are laid out in §9.Q1.

### 3.2 NO CHECKPOINT LADDER — settled by measurement

This was the reconnaissance's fourth owner question (*"pay the O(n²), or build a
checkpoint ladder now?"*) and its §4.4 hazard. **It is not a question.** A ladder
is a whole subsystem — a second cache beside `Games::live` with its own
invalidation on record-length change and its own memory bound — and M4 does not
need one.

**The lever.** `why?event=n` calls `state_at` BEFORE any of the seat refusals:
`crates/yokohama-cli/src/why.rs:418` is `let state = state_at(record, index,
rules)`, and the `HumanSeat` / `Rung` / `Diagnostic` / `UnknownController`
refusals are at `:423-427`. So timing `why?event=n` on an index the seat refuses
times a real replay of `n` events plus the HTTP round trip, and nothing else.

**I re-ran the measurement, as the brief asked.** Method: `cargo build --release
-p yokohama-web` at this head; a 1,091-event 2P `GameRecord` emitted by
`cargo run --release -p yokohama-core --example arena -- cell m4bench 2
family-weighted family-weighted 2 --emit <dir> --master 12345 --no-invariants`,
dropped into a temp `YOKOHAMA_WEB_DIR`; 40 requests per k over loopback after one
warm-up, median reported. Machine: this Mac (M-series, `darwin 25.6.0`) — **wall
figures are per-machine and this one is not the WSL box or M5.**

| `why?event=k` | median | min | max |
|---|---|---|---|
| 0 | 0.19 ms | 0.17 | 0.29 |
| 100 | 0.32 ms | 0.31 | 0.35 |
| 195 | 0.47 ms | 0.44 | 0.50 |
| 500 | 0.91 ms | 0.88 | 0.98 |
| 1000 | 1.65 ms | 1.58 | 1.73 |
| 1090 | 1.82 ms | 1.76 | 2.08 |

Clean and linear. **The slope is 1.50 µs per event replayed**
((1.82 − 0.19) / 1090), and the intercept — HTTP, the record clone, the whole
setup preamble — is **0.19 ms**.

In-process, against the same records with the HTTP boundary removed:

| record | events | `state_at(0)` | `state_at(n)` | sweep of ALL n+1 positions |
|---|---|---|---|---|
| a0 | 1091 | 0.004 ms | 2.1–3.0 ms | **0.864 s** |
| a0' | 962 | 0.003 ms | 1.8 ms | 0.823 s |
| a1 | 519 | 0.002 ms | 1.3–1.4 ms | 0.245 s |
| a1' | 343 | 0.003 ms | 0.48 ms | 0.086 s |

**Agreement with the wave's figure, and the honest difference.** `wave0.md` §7
records *"~1.2 µs per event"* on a 195-event game and *"~0.7 s"* to sweep the
1,036-event committed fixture. I read **1.50 µs/event** by HTTP slope and
**0.86 s** to sweep 1,091 events in process. Same order, same conclusion,
different machine and a slightly longer record; **the wave's number is not
reproduced exactly and I am not restating it as if it were.** Anyone quoting a
figure should quote one of these two rows with its machine.

**Two corrections to the reconnaissance's cost analysis fall out of this:**

- **[CORRECTION to recon §4.4]** *"every call re-pays the setup preamble
  (`new_game` = board generation + two deck shuffles + `finalize_setup`), which is
  independent of k"* — true, and **it costs 2–4 microseconds**, about two events'
  worth. It is not a term worth engineering around and naming it beside the O(k)
  fold overstates it.
- **[CORRECTION to recon §3/A1]** *"a naive sweep is ~537,000 full legal-move
  enumerations"* — the arithmetic is right and the implication is wrong. 537,000
  checked applies is **under a second of one core**, measured. The quadratic is
  real; at this constant it is not a problem M4 has.

**What follows.** Debounce plus plain `state_at` is honest at every game length
this project produces. The client already has the debounce pattern to copy —
`web/src/Index.tsx:261-279`, a 200 ms `setTimeout` with a `let live = true`
cancellation flag. The ladder is out.

**What this does NOT license.** A drag that fires a request per pixel is still
bad manners toward a shared server, and the route still holds `Games::handle`'s
per-game mutex unless it drops it (§6.3). The claim is "no ladder", not "cost is
free".

### 3.3 The event index space is ONE integer, proved from both ends

`LogEntryModel.index` (`screen_model.rs:551-555`, whose own doc says *"The event's
index in `GameRecord.events`"*) is built by the log route over exactly the record's
own indices:

```rust
// crates/yokohama-web/src/lib.rs:729-732
let total = repl.session.record().events.len();
let first = q.tail.map_or(0, |n| total.saturating_sub(n));
let described: Vec<(usize, String)> =
    (first..total).map(|i| (i, repl.session.event_description(i).to_string())).collect();
```

`why?event=n` uses the same integer to subscript the same vector:

```rust
// crates/yokohama-cli/src/why.rs:402-409
let events = record.events.len();
let index = match arg {
    Some(asked) if asked < events => asked,
    Some(asked) => return Err(Refusal::OutOfRange { asked, events }),
    None => last_explainable_bot_move(record, rules)?,
};
let RecordedEvent::Move(recorded) = &record.events[index] else { ... };
```

and then passes it straight to `state_at` as `k` (`why.rs:418`).

**They are the same integer — with one asymmetry that the scrubber must respect.**
`why` requires `n < len` (it addresses an EVENT). `state_at` accepts `k <= len`
(it counts events APPLIED). So the scrubber's track has **`total + 1` stops**, and
at the last one there is a state but no event, and `why` correctly refuses with
`OutOfRange`. A track that runs `0..total` and calls that "the whole game" is off
by one at the end; a track that runs `0..=total` and calls `why` at every stop
gets one avoidable refusal.

---

## 4. What already exists that M4 must not re-derive

Every row re-verified at this head. Rows the recon carried that Wave 0 moved are
marked.

| Thing | Where (this head) | Note |
|---|---|---|
| Replay to a position | `record.rs:561` `state_at` | plus `pub fn replay_from(&GameState, &[RecordedEvent], &Rules)` at `record.rs:384`, foldable from any base |
| Frame builder over a bare state | `screen_model.rs:2698` `build_frame` | pure; no `Repl`, no `Session`. Already what the head is served from (`client.rs:334`) |
| Observer scoping | `screen_model.rs:2515` `build_decision_for` | the seam §3.1 rules on; called by nothing today |
| Hand masking | `screen_model.rs:132` `SeatFacts` → `:2675` `build_dashboard` → `:2315` `build_hand` | one hand per frame, `for_seat` (`:147`) fails safe to hidden |
| The event index space | `screen_model.rs:554` `LogEntryModel.index` | §3.3 |
| Cached per-event prose | `session.rs:490` `event_description(i)` | a bare `Vec<String>` index; **no seat field** — the seat is a colour prefix inside the text |
| Whole-log route | `lib.rs:722` `GET …/log` | default is the WHOLE log with global indices; `LogPanel { total, entries }` + `omitted()` (`screen_model.rs:568`) |
| `why` at an arbitrary event | `lib.rs:589` route, `why.rs:392` `account_parts` | **`?event=n` is shipped server-side and unreachable from the client** — see below |
| Twelve typed refusals + the web's `locked` | `why.rs:149-163`, `explain.rs:423`, `:481` `kind_of` | `Refusal::NotAMove` (`why.rs:153`) → wire kind `"not_a_move"` (`explain.rs:486`) already answers "you scrubbed to a reveal" |
| Lock-drop-then-compute template | `lib.rs:534-563` (suggest), `:594-612` (why) | clone what you need out of a block, drop the guard, then work |
| Lock-and-project template | `lib.rs:471-482` `macro_rules! surface!` | for a `(&GameState, &Rules) -> Model` route |
| Provenance verdicts, already run | `session.rs:238` (semantics_rev), `:365` (terminal digest) | at `Session::from_record`, i.e. inside `Games::handle` |
| Components that render any `FrameModel` | `web/src/Board.tsx:1-3`, `web/src/Dashboard.tsx:1-15` | neither imports `api` or `ClientState`; **omit `onTileClick` and every tile is an inert `<div>`** (`Board.tsx:197`) |
| Debounced fetch on a control | `web/src/Index.tsx:261-279` | 200 ms `setTimeout` + `let live = true` — the shape a scrub drag needs |
| Modal shell (focus, Escape, `role="dialog"`, stale-payload guard) | `web/src/Surfaces.tsx:105-152`, registry `:55-71` | registering a `SURFACES` row IS registering the nav button |
| A numbered index list with a gutter | `Surfaces.tsx:352-368` | the log surface, unstyled as interactive today |

**`why?event=n` is API-complete and client-unreachable, confirmed.**
`web/src/api.ts:92-96` declares `why: (game, event?) => …` and appends
`?event=${event}` when given one. The single call site is
`web/src/Surfaces.tsx:64` — `load: (g) => api.why(g)` — with no second argument,
and the registry's `load` signature is `(game: string) => Promise<unknown>`
(`Surfaces.tsx:55-59`), which **cannot pass one**. So the per-event `why` half of
the scrubber needs zero server work and one widening of that registry contract,
which the contract's own doc already anticipated for M3 (`Surfaces.tsx:47-54`).

**Rows Wave 0 changed, which the recon's version of this table would now
mis-state:**

- **The width gate is no longer 2P-only.** `web/e2e/the_screen_itself.spec.ts:265`
  now iterates `[390, 1024, 1201, 1440]` across 2/3/4P and its `blame()`
  (`:233-241`) names the widest offending element. The recon reasoned about a
  gate that measured a `documentElement` delta at two widths on 2P.
- **The 545px phone overflow is fixed**, not an accepted limitation:
  `web/src/styles.css:131` is `.board-col { width: 100% }` inside the
  `@media (max-width: 1200px)` block.
- **`Games::persist` now evicts on conflict** (`games.rs:498-501`), which changed
  the undo hazard's shape — and, as of `af706de` (2026-09-01), the guard itself is
  a compare-and-swap and the undo hazard is CLOSED. See §6.4b.

---

## 5. What remains open

### 5.1 Where the scrubber lives — an M3.5 question, deliberately not answered here

The owner ruled layout first for exactly this reason, so this section states the
constraints and stops.

**The candidate hosts, and what each actually costs at this head:**

- **On the log surface.** Make `Surfaces.tsx:361`'s `<li>` a `<button>`, lift the
  selected index, fetch `/at/{k}` and `/why?event=k`. Cheapest diff by a wide
  margin: the gutter already prints the exact integer (`:362`), the modal shell,
  focus handling, Escape and the `loaded.of` staleness guard (`:105-121`) come
  free, and the `?event=n` server half is done.

  **[CORRECTION to recon §3/B1]**, which costed this as blocked by layout: it
  claimed a board inside `.surface` would fight
  `the_screen_itself.spec.ts`'s no-horizontal-overflow assertion. It would not.
  `web/src/styles.css:377` is `.surface-body { padding: 1rem; overflow: auto }`
  and `:136` is `.board { … overflow-x: auto }` — a wide board inside the panel
  scrolls **inside the panel**, and the page-overflow gate measures the page. At
  1440 the panel's content box is 1440 − 2·(2rem) − 2·(1rem) = 1344px against a 4P
  widest row of 6·224 + 5·6.4 = 1376px (`--tile-w: 14rem`, `styles.css:49`), so it
  scrolls by ~32px. That is a legibility argument, not a gate violation, and the
  two deserve different weights.

- **A mode that replaces the stage.** Swap the `.stage` block
  (`App.tsx:126-143`), hide `Decision`, add a scrub track. Full-size board, no
  layout fight, and `colors` re-derives from the HISTORICAL `Players` panel
  exactly as `App.tsx:56-58` derives it from the live one. Costs: the first
  top-level screen fork beside `App.tsx:55` (`if (!state) return <NewGame …>` is
  the app's only screen mechanism today), and `Decision.tsx:83-95` takes
  `state: ClientState` whole with fifteen `state.` reads, so it must be hidden
  rather than reused.

**One fact that bites either way, verified:** the client has **no URL routing and
no client-side persistence.** `grep -rn "pushState\|location.hash\|history\."
web/src` returns nothing; so does `grep -rn "localStorage\|sessionStorage"
web/src`. A reload drops you back to `NewGame` and a scrub position is not
linkable. Whether M4 is the milestone that fixes that is §9.Q4.

### 5.2 What the payload is — the argument each way

**C1: a full `ClientState` at k.** Maximum client reuse: `App.tsx` could render a
historical payload through the same components with no new types, and no fixture
type would need adding to `contract.ts`.

**C2: a narrow `HistoricalFrame`** — `{ index, total, frame, to_move, over,
deck_remaining, end_game_triggered, end_triggers, scores }`, every field re-derived
from `state_k`.

**The code argues for C2, and the argument is the six fields that would be
present-tense lies if inherited.** `ClientState::of` (`client.rs:326-380`) fills
them from the LIVE state:

| field | `client.rs` | what a lie looks like at k |
|---|---|---|
| `over` | `:333` `session.is_over()` | a mid-game board under "GAME OVER" |
| `scores` | `:371-377` `over.then(…final_scores())` | a final tally beside turn 12 |
| `deck_remaining` | `:369` `state.order_deck.len()` | the deck count from the end of the game |
| `end_game_triggered` | `:370` | "FINAL ROUND" at turn 3 |
| `end_triggers` | `:368` `render::end_trigger_line(state)` | the same, as prose |
| `your_turn` | `:365` `!over && current_player == seat` | meaningless for a position nobody is playing |

Every one of those is a pure function of a `GameState` and re-derives correctly at
k. **The point of C2 is not that C1 cannot be made right — it is that C2 makes
their provenance a compile-time fact instead of a review item**, in a lane whose
last three milestones each shipped a false sentence about themselves.

Two more fields have no honest historical value at all: `prompt` (§2.2 — the
sub-prompt is not in the record) and `autosaved` (`client.rs:355` is an
unconditional `true`; both `true` and `false` are slightly false about a past
position). Under C1 they must be answered; under C2 they are absent by
construction.

**Neither choice touches `Board`/`Dashboard`** — both take `frame.board` and
`frame.dashboard` and nothing else (`Board.tsx:12-31`, `Dashboard.tsx:17-27`), so
the component reuse argument does not actually distinguish C1 from C2. The cost
difference is entirely in `contract.ts` (§7.1).

**C3, named and dropped: ship every frame once.** One O(N) forward fold, then all
scrubbing is offline. Killed by payload size. Measured over HTTP against the
release binary at this head — **and this is a correction worth carrying, because
the recon's number came off the pretty-printed golden:**

| payload | over the wire | committed golden on disk |
|---|---|---|
| 2P `ClientState` | **11,977 bytes** | `options.json` 31,659 |
| 4P `ClientState` | **18,925 bytes** | `options_4p.json` 53,504 |
| `/log` at 1,091 events | 71,665 bytes | — |

**[CORRECTION to recon §3/A3]**, which used *"31,705 bytes for one 2P
`ClientState`"*. The server serialises compact; the corpus is pretty-printed for
review. The real 2P figure is 12 KB, so 1,091 frames is ~13 MB at 2P and ~20 MB at
4P rather than the recon's ~33 MB. Still not shippable, and the reasoning is
unchanged — but a document that quotes a golden's file size as a wire size is
quoting the wrong thing, and this lane has a history of exactly that.

---

## 6. Hazards, each re-checked at this head

### 6.1 The present-tense leak — the failure mode unique to a scrubber

Stated in §5.2. It is the reason to prefer C2. **STANDS, unchanged by Wave 0.**

### 6.2 `tile.reachable` is derived from the legal list

```rust
// crates/yokohama-cli/src/screen_model.rs:1938-1950
pub fn build_board(state: &GameState, rules: &Rules, legal: &[Move]) -> BoardModel {
    let reachable = president_destinations(legal);
    …
    tile.reachable = reachable.contains(&area);
```

So **a historical board built with `legal: &[]` silently marks every tile
`reachable: false` and the `◈` mark disappears with no error.** The test
`the_reachable_marks_are_exactly_the_engines_president_destinations`
(`screen_model.rs:3046-3063`) pins both directions, including that an empty list
marks nothing. This is a decision to make explicitly, not a default to fall into:
either pay the extra `get_legal_moves(&state_k, rules)` (roughly one more event's
worth of work — negligible against the k already paid, per §3.2) or declare that
a historical board carries no reach marker and say so on the wire. **STANDS.**

A related asymmetry that pre-exists at the head and that a scrubber reproduces at
every position: `build_frame` passes `legal` to `build_board` unconditionally
(`:2708`) even when `build_decision_for` has just emptied the option list for that
same observer. Low-value information (president destinations are public), but it
means a pinned-viewer historical frame still shows the ACTOR's reachable set.

### 6.3 The locked-session gate — a projection must NOT gate on lockdown

The rule is stated in code, at `crates/yokohama-web/src/lib.rs:596-606`:

> *"**The lockdown gate fences COMPUTATION, not projection**, and this is the rule
> the three route families follow. M2 already settled that a read-only GET serves
> a locked session — `ports`/`labs`/`areas`/`log` do, and `ClientState` reports
> the lockdown as its prompt rather than refusing. But `why` is not a projection:
> it re-runs the same `decide` path against a record the driver has declared
> faulty… `suggest` refuses for the same reason; the card indexes and the final
> score do not, because they read the state rather than an opinion about it."*

Pinned in both directions by `a_locked_session_refuses_the_searches_and_still_serves_the_views`
(`lib.rs:1185`).

**`/at/{k}` is a projection. It must serve a locked session.** A locked game is
exactly the one whose history you want to read — the lockdown means the driver
found a fault, and the scrubber is the instrument for finding where. If M4 also
wires per-event `why`, **that half gates and the frame half does not**, inside one
UI. That asymmetry has to be visible to the user rather than looking like a bug:
the frame draws, the account says "locked". **STANDS.**

### 6.4 The resident handle vs. the durable record — Wave 0 CHANGED this

The recon's §4.6/§4.7 described a world that no longer exists. Re-read from
`games.rs` at this head, and re-measured:

**(a) A read still pumps and writes.** `Games::handle` (`games.rs:333-363`) on a
cache miss rebuilds, runs `bring_alive`, and persists if the pump played:

```rust
// crates/yokohama-web/src/games.rs:350-354
let before = repl.session.record().events.len();
bring_alive(&mut repl);
if repl.session.record().events.len() != before {
    self.persist(id, &repl)?;
}
```

`lib.rs:448-464` retracts an earlier comment that denied this and explains why it
is kept rather than fixed (*"a read-only rebuild that skipped the pump would cache
an un-pumped handle, and the next `input` would apply a human move to a state the
bot had not yet moved in"*). `/at/{k}` routed through `handle` inherits it. The
alternative — reading the `GameRecord` straight from `Games::store()`
(`games.rs:252`), which is `pub` — avoids both the pump and the lock, at the cost
of reading a record that may lag the handle. **STANDS as a genuine fork; §9.Q2.**

**(b) The undo hole is CLOSED** — commit `af706de`, 2026-09-01, after this
document was first written. The paragraphs below are kept in the order the
hazard was actually resolved: what it was, then what closed it, then what M4 may
now assume.

**THE BEFORE STATE — no longer reproducible at this head.** Wave 0 had added
`Games::conflict`, which **evicts** the handle whose write was refused; its own
test doc named the residue: *"Known defect (a) — the undo hole — loses ONE move
this way; this loses the rest of the session"*. Measured then against the real
release binary over HTTP, a 2P game played to 87 stored events, then one `undo`:

```
# PRE-FIX TRANSCRIPT (before af706de) — kept as the BEFORE state, not current behaviour
log total before undo: 87
undo warnings: ['APPLIED BUT NOT SAVED — "undo2" has advanced past this handle
                 (87 stored vs 86 here) — refusing to write backwards']
log total AFTER undo (next request): 87
```

`Session::undo` shortened `events`; `Games::persist` refused the backwards write;
`input` reported the applied state plus an honest warning; `conflict` evicted the
handle; the very next request rebuilt from the store and the undone move was
back. The client was told "Undid your last move" and it un-undid itself one
request later.

**WHAT CLOSED IT.** `persist`'s guard was a monotonicity test — *"does the store
hold MORE events than I do?"* — which is true of a stale handle about to erase
another writer's move (refuse) AND of a live handle that just shortened its own
log (write it), and could not tell them apart. It is now a **compare-and-swap**
against a digest of the exact JSON this process last wrote (or read, on a cold
rebuild), taken under a lock held across the read, the compare and the write.
Same measurement after the fix: **47 events → undo → no warnings → 46 on the next
request**, and the shortened record survives a cold rebuild.

Two things about that fix M4 should not overstate. It is **strictly stronger**
than the length test, not merely more permissive — two divergent lines of the
SAME length differ in content and not in count, which the length test waved
straight through. And it **detects** a cross-process conflict rather than
**preventing** one: nothing serializes read-then-write across processes, and
`FsStore::write` is atomic per file (temp + rename) but the store has no lock.
That limit is unchanged from before.

**[CORRECTION to recon §4.7]** stands as history and is now doubly superseded.
§4.7 said the stale handle *"stayed resident and answered every later request"*
and that the undone move *"silently resurrects"* only on eviction or restart;
post-Wave-0 the resurrection was immediate and every time; post-`af706de` there
is no resurrection. The "still untested" clause is also gone: `undo` now has
`an_undo_reaches_the_store` in `games.rs`, which asserts the store shortened AND
that a cold rebuild replays the undone record.

**What M4 may now assume:** *a shortened record is a legitimate, persisted
state.* A scrubber may treat `events.len()` as free to decrease between requests
without that meaning corruption, a lost write, or a stale handle; the durable
record and the resident handle agree across an undo, so the §9.Q2 fork is no
longer forced by an undo window. What a scrubber must NOT assume is monotonic
growth of the event index — an index captured before an undo can point past the
end of the record afterwards, and that is now an ordinary, expected transition
rather than the symptom of a bug.

### 6.5 `RecordedEvent` variants that are not moves

`record.rs:29-37`: four variants — `Move`, `RevealOrders`, `RevealTechs`,
`RevealStartingOrder`. `state_at(k)` is valid at every one of them; the decision
zone at such a k shows options nobody took next.

The typed answer already exists and reaches the wire: `Refusal::NotAMove { index,
kind }` (`why.rs:153`) → `"not_a_move"` (`explain.rs:486`). In practice the web
creates no live games — `wire_fixtures.rs`'s prompt-coverage test names `reveal`
as its **sole** exemption for exactly that reason — but `Games::handle` will load
any `GameRecord` in `YOKOHAMA_WEB_DIR` (default `~/.yokohama/games`,
`store.rs:209-216`), including a live one written elsewhere. **STANDS.**

### 6.6 `record.run` has no per-event grain

`RunProvenance.seats` is one `SeatProvenance` (`record.rs:106-126`) per seat for
the whole game. A seat downgraded mid-game reads as whatever it is now at every k
— **fail-safe for the hand mask** (a seat that is a bot now was masked then too),
**wrong for any seat badge a historical frame draws.** `why` handles this by
refusing outright (`Refusal::Mismatch`, `why.rs:154`, which names both versions);
a frame has no equivalent gate, so **scrubber seat badges are strictly less
trustworthy than `why`'s verdicts at the same index**. That is worth a sentence in
whatever ships, not a fix. **STANDS.**

Second-order: `session.event_description(i)` (`session.rs:490`) is a bare
`Vec<String>` index with no seat field — the seat is baked into the text as a
colour prefix. A scrubber that wants "who moved at k" reads
`state_at(k).current_player`, not the log line.

### 6.7 Lock discipline

Only `/suggest` and `/why` drop the per-game mutex before working
(`lib.rs:534-563`, `:594-612`); every other handler holds it across its whole
body, which is right for a projection. At §3.2's constants an `/at/{k}` on a
1,000-event game holds it for ~2 ms, which is not obviously worth the clone — and
the clone is not free either: `repl.session.record().clone()` copies up to a
thousand `Move`s. **Measure before choosing; do not copy `why`'s pattern
reflexively.** This is the one hazard in this section where the recon's advice
("follow `why`") is *weaker* than the measurement now supports.

### 6.8 Cache pressure

`MAX_RESIDENT = 64` (`games.rs:238`) with deliberately arbitrary, non-LRU
eviction (`games.rs:531-549`, whose doc explains why: *"the property that matters
here is only that the map is BOUNDED"*). §3.2 removes the reason M4 would have
added a second unbounded map. Nothing to do. **DISSOLVED.**

---

## 7. A test plan

### 7.1 The wire corpus, and the honest cost of one new fixture

**The critic is right: adding a fixture is a five-SITE change, not a five-line
one.** I enumerated the sites at this head. For a NEW type (`HistoricalFrame`)
with one payload:

| # | file | what |
|---|---|---|
| 1 | `crates/yokohama-web/tests/wire_fixtures.rs` | the `capture(…)` call inside the walk |
| 2 | `wire_fixtures.rs:443-457` | an entry in the `owed` list, if the shape must appear every run |
| 3 | `web/src/types.ts` | declare the interface |
| 4 | `web/src/wire/contract.ts:88-128` | `import atK from "./at_k.json";` |
| 5 | `contract.ts:155-197` | `const _atK: Widen<HistoricalFrame> = atK;` |
| 6 | `contract.ts:260-270` | add `_atK` to the `void [...]` list — **`noUnusedLocals` is on** (`web/tsconfig.json:9`) |
| 7 | `contract.ts:305-367` | a root key list `const AT_K = […] as const` |
| 8 | `contract.ts:370-403` | `const _ak: KeysOf<HistoricalFrame, typeof AT_K> = AT_K;` **and** its entry in the second `void [...]` |
| 9 | `contract.ts:408-441` | a `KEYS` entry |
| 10 | `contract.ts:460-…` | an entry in the exported `wire` object, for component tests |
| 11 | `web/src/wire/contract.test.ts:67-101` | a row in the runtime `it.each` key-comparison table |

Eleven sites for a new type; **six** if the payload is a `ClientState`
(1, 4, 5, 6, 10, plus `EVERY_STATE` at `contract.ts:515` and a `PromptOf<…>`
assertion in §1b — so seven). Then `UPDATE_WIRE_FIXTURES=1 …`, which **always
fails by design** (`wire_fixtures.rs:851`), and **read the diff**.

**This is the real cost difference between C1 and C2 in §5.2, and it should be
weighed as a cost, not as an argument.** Sites 7–9 and 11 are the ones that exist
because the client's `contract.ts` refuses to trust either side; they are what
caught six fields that had a compile-time list and no runtime half
(`contract.test.ts:85-90`). Paying them is the lane working correctly.

**Also true, and unchanged:** nothing enumerates the router's routes, so a new
route ships with no coverage by default. `EVERY_STATE` (`contract.ts:515-521`) is
hand-maintained and pinned by nothing.

### 7.2 What belongs where

**In `yokohama-web`'s lib tests** (the 57 that ran green at this head):

- `k == events.len()` returns a frame equal to the live frame's board and
  dashboard. This is the property that proves the historical path and the live
  path agree, and it is cheap.
- `k > events.len()` is a **404 with a message naming the bound**, derived by
  comparing `ReplayError::event_index` against `record.events.len()` rather than
  by reading the message (§2.1).
- **A locked session SERVES `/at/{k}`** — the direction §6.3 requires — and, if
  per-event `why` is in scope, the same test asserts `why` still refuses at the
  same index. One test, both directions, in the shape `lib.rs:1185` already uses.
- **The viewer pin holds under a sweep.** Build an all-human 2P game, walk
  `k = 0..=n`, and assert that **exactly one seat's `Hand` panel ever carries
  cards**. This is the executable form of §3.1 and it is the single most important
  cell in the milestone. Mutation-proof it by flipping the viewer to
  follow-the-mover and watching it fail.
- `tile.reachable` at a historical k matches `president_destinations` of that k's
  legal list — or, under the other ruling, is asserted false everywhere with the
  wire saying so (§6.2). Either way it is asserted, because the failure mode is
  silence.
- A `RecordedEvent` that is not a `Move` still yields a frame (§6.5), constructed
  rather than walked.

**In `web/src/*.test.tsx`** (jsdom, over committed fixtures only — never a
hand-authored payload, `m2.5.md`'s rule):

- `Board` and `Dashboard` render a historical frame with `onTileClick` omitted and
  produce **no `<button>`** (`Board.tsx:197`).
- The track renders `total + 1` stops (§3.3) and the last one is labelled as the
  present, not as an event.

**In `web/e2e/`** (real Chromium — the only place these are visible):

- Scrub to a position and back, and assert the live board is byte-identical to
  where it started. jsdom cannot see this because it does not run the fetch
  sequence against a real server.
- The debounce actually debounces: drag the control and count network requests
  (`page.on("request")`). A test that only checks the final frame passes on a
  scrubber firing one request per pixel.
- Focus: where does focus land when the scrubber opens and when it closes. The
  modal shell's focus handling (`Surfaces.tsx:131-138`) is a per-surface property
  and a new interactive control inside it is untested by construction.
- **And the anti-vacuity discipline `wave0.md` §2 earned**: any cell that opens a
  panel must assert the panel has the content its name claims. C4's lesson — a
  test named "a panel holding two hundred candidates" measured a panel with no
  candidate list in it — applies directly to a scrubber cell that opens a track
  on a two-event game.

### 7.3 CI, which still cannot go red for any of this

`.github/workflows/test.yml:11` is one line: `cargo test -p yokohama-core`, on
`push` and `pull_request`. `git diff --stat master..HEAD -- .github` is **empty**,
and `git diff --stat master..HEAD -- crates/yokohama-core` is **also empty** — so
CI runs, against this branch, a crate this branch has not touched.
`release.yml:47` adds `cargo test -p yokohama-cli` but only on a `v*` tag.
**Nothing in `crates/yokohama-web` or `web/` is reachable from CI at all.**

M4 adds a route, a DTO and a client mode into that hole. This is unchanged since
it was first raised and is `wave0.md` §6's second owed item. §9.Q5.

---

## 8. The M4-adjacent win: the per-seat 5-Power token strip

**Verified at this head, both citations.**

The face is destroyed in *state*:

```rust
// crates/yokohama-core/src/types/player.rs:225
pub five_power_tokens: u8, // Just count, effects already resolved
```

and preserved in *history*:

```rust
// crates/yokohama-core/src/game/apply_move.rs:902-928
fn apply_take_5_power(state: &mut GameState, rules: &Rules) -> Result<(), MoveError> {
    let player = state.current_player;
    …
    let area = resolve_acting_area(state, player).map_err(MoveError::RuleViolation)?;
    let effect = state
        .board_layout
        .five_power_tokens
        .get_mut(&area)
        .and_then(|slot| slot.take())
        .ok_or(MoveError::RuleViolation("No 5-Power token at the acting area"))?;
```

*(The critic cited `apply_move.rs:921`; at this head the `slot.take()` is line 927
and the function opens at 902. Core is byte-identical to master, so that was a
citation drift, not a Wave 0 move.)*

`slot.take()` leaves the board slot `None`. So at the event index `i` where
`record.events[i] == RecordedEvent::Move(Move::Take5PowerToken)`
(`apply_move.rs:121`, `types/moves.rs:99`), the **pre-event** state
`state_at(record, i)` carries the face at the acting area and names the taker as
`current_player` (`apply_move.rs:903`).

**It is a replay-derived projection over the same `GameRecord` the server already
holds — the same machinery M4 builds. No core change, no `PlayerState` field, no
`SEMANTICS_REV` implication.** Two derivations, both available today:

1. **Read the acting area.** `yokohama_core::game::actions::resolve_acting_area` is
   `pub` (`game/actions/tracks.rs:100`, re-exported by `actions/mod.rs:11`) and
   `yokohama-cli` already imports it (`render.rs:11`). It handles the mercenary
   sub-turn correctly, which a hand-written "the current area" would not.
2. **Diff the two states.** `BoardLayout.five_power_tokens` is `pub`
   (`types/board.rs:204`), so comparing `state_at(i)` with `state_at(i+1)` finds
   the one area whose slot went `Some → None`. This restates no rule at all and is
   the safer of the two.

**What it costs once `/at/{k}` exists.** One forward pass over the record —
`state_at(0)` then `replay_from` one event at a time, which is the shape
`why::last_explainable_bot_move` already uses (`why.rs:358-370`) and which its own
doc explains is *"one FORWARD fold rather than a backwards `state_at` walk"*. At
§3.2's constants that is **~2 ms for a 1,000-event game**, once, cached or not.
The wire shape is `Vec<(PlayerId, FivePowerEffect)>` — and `FivePowerEffect` is
already a wire type: `TileModel.token` is `Option<FivePowerEffect>`
(`screen_model.rs:195`) and the client already draws it
(`web/src/art.ts` `fivePowerArt`/`fivePowerWords`, used at `Board.tsx:2`).

So: a small, well-defined follow-on that becomes cheap exactly when M4 lands, and
which turns the owner's request from "buildable only as reinterpreted" into
"buildable as asked". It is **not** part of M4 unless the owner says so.

---

## 9. Open questions for the owner

Genuine forks only. Each states the impact chain before the options, per the
standing preference.

### Q1 — How is the pinned viewer chosen?

**Impact chain:** §3.1 settled *pin a fixed viewer*, but a pin is only a fence if
the caller cannot vary it. The API has no notion of who is asking (§3.1,
measured), so "the viewer" has to come from somewhere. This decides the route's
signature, the DTO, the mutation-proof test in §7.2, and whether a two-device
table is safe.

- **(a) Server-picks: `viewing_seat(repl)` at the LIVE head.** No new parameter,
  no new concept, and the scrubber's exposure is exactly the live payload's. But
  in a hot-seat game that value CHANGES as play advances, so the same historical k
  answers with different hands at different times — reproducible only against a
  fixed head.
- **(b) `?seat=` parameter.** Honest, flexible, and **not a fence**: a sweep over
  `(k, seat)` returns every hand. Acceptable only if the owner's model is "anyone
  with the game id is a trusted player", which is what the deployment already
  assumes (Cloudflare Access in front, per the hosting note — UNVERIFIED here, I
  did not inspect the deployment).
- **(c) A per-game "my seat", chosen at create.** A real fence. Needs a concept
  the API does not have and a migration for every existing record.

**My read, offered as a proposal:** (a), because it adds nothing and leaks nothing
the live route does not already leak, with (c) as the answer if multi-device play
is ever a goal. But (a)'s instability across time is a real product wart and the
owner may hate it.

### Q2 — Does `/at/{k}` read the resident handle, or the durable record?

**Impact chain:** the handle is the truth the player is playing, and reading it
through `Games::handle` can pump a bot and write the store (§6.4a, `games.rs:350-354`).
The store is genuinely read-only and lock-free (`games.rs:252`). It USED to
disagree with the handle for the whole undo window, which is what made this
question urgent; as of `af706de` an undo is written through (§6.4b), so the two
agree across an undo and **the fork is now a design preference, not a forced
choice**. The remaining difference is §6.4a's pump-and-write on a cache miss.

### Q3 — Is the undo hole M4's to fix? — **ANSWERED, and not by M4**

**CLOSED 2026-09-01 by `af706de`**, which is the second of the three options this
question listed: a Wave-0-style follow-on landed BEFORE M4, so M4 inherits a
working undo and owes this nothing. The question is kept for its impact chain,
which still holds — the scrubber is the feature that makes the record visible,
and the first instinct after scrubbing back is to press Undo. What was true when
this was written (undo applied in memory, warned honestly, evicted its own
handle, and reverted on the next request) is the §6.4b BEFORE state and is no
longer reproducible. **A "play from here" branch is not on the table either way** — `Session::undo` (`session.rs:683-727`) truncates
with no tombstone and no redo, and the lane's own plan scopes v1 as *replay
browsing, not branching*.

### Q4 — Does M4 introduce URL routing?

**Impact chain:** no `pushState`, no hash, no `localStorage` anywhere in
`web/src` (verified). A scrub position is not linkable and a reload drops to
`NewGame`, which for a *replay* feature is a bigger wart than for a *play* one
("look at turn 40" is a thing people want to send). But it is the first routing in
the client and it interacts with M3.5's layout. Doing it badly here is worse than
not doing it.

### Q5 — Does CI grow to cover this lane before M4 lands?

**Impact chain:** §7.3 — nothing in `crates/yokohama-web` or `web/` is reachable
from CI, so "green" for this lane means "a human ran the suites". M4 adds a route,
a DTO and a client mode into that hole. Adding `-p yokohama-web`, and deciding
whether the vitest and Playwright halves run in CI at all, changes what "green"
means for every milestone after this one. Unchanged since first asked; the stakes
are now the size of the lane.

---

## Appendix A — measurements taken this session

All on this Mac (M-series, darwin 25.6.0), at `513484d`, release builds.
**Wall figures are per-machine; record a new machine's beside these, never
against them.**

| # | What | Result |
|---|---|---|
| A1 | `why?event=k` over HTTP, 1,091-event record, 40 reps/k, median | 0.19 / 0.32 / 0.47 / 0.91 / 1.65 / 1.82 ms at k = 0 / 100 / 195 / 500 / 1000 / 1090 |
| A2 | slope of A1 | **1.50 µs per event replayed** |
| A3 | intercept of A1 (HTTP + record clone + setup preamble) | **0.19 ms** |
| A4 | `state_at(record, 0)` in process | 0.002–0.011 ms |
| A5 | full in-process sweep of every position, 1,091 events | **0.864 s** |
| A6 | same, 962 / 519 / 343 events | 0.823 / 0.245 / 0.086 s |
| A7 | 2P `ClientState` over the wire | 11,977 bytes |
| A8 | 4P `ClientState` over the wire | 18,925 bytes |
| A9 | `/log` at 1,091 events | 71,665 bytes |
| A10 | all-human 2P table: hand served before/after a turn change | seat 0 card `[33]` → seat 1 card `[31]`, same unauthenticated URL |
| A11 | undo through the API | applied, warned, evicted, **reverted on the next request** (87 → 87 stored events) |
| A12 | `cargo test -p yokohama-web` | 63 (57 lib + 1 gate + 5 `wire_fixtures`), green |
| A13 | `npm test` | 195 in 10 files, green |
| A14 | wire corpus | 41 files |

A12–A14 reproduce `wave0.md` §5 exactly. A1–A11 are new.

**Two rows are SUPERSEDED by `af706de` (2026-09-01)** and are left standing
because this table is a dated reading at `513484d`, not a current one. **A11** is
the §6.4b BEFORE state: undo now persists, and the same measurement reads
47 events → undo → no warnings → 46 on the next request. **A12** is now **65**
(59 lib + 1 gate + 5 `wire_fixtures`), the two added cells being
`an_undo_reaches_the_store` and
`a_divergent_write_of_the_same_length_is_refused`. A13 and A14 were re-measured
at the follow-on head and are unmoved.

Instrument for A4–A6: a throwaway crate in the session scratchpad depending on
`yokohama-core` by path, calling `state_at` directly. Records for A1–A6 emitted by
the documented arena command in §3.2. **Nothing was written into the repository to
take these measurements.**

## Appendix B — what I could not verify

- **The M3.5 layout figures** quoted in §1 (1950px page, y=1703 decision zone,
  69.6% tile-art obscuration). No browser run this session; they are `wave0.md`
  §7's and the M3.5 record's to defend.
- **The deployment's identity gate.** §9.Q1(b) leans on Cloudflare Access sitting
  in front of `yokohama.moore.nyc`. That is recorded in `m2.md:162-165` and I did
  not inspect any deployment. Treat it as owner knowledge, not as a verified
  property of this branch.
- **`wave0.md`'s ~1.2 µs/event.** Not reproduced exactly (I read 1.50 µs on a
  different machine and a longer record). The conclusion it supports — no
  checkpoint ladder — is independently reproduced, with a 3× margin to spare.
