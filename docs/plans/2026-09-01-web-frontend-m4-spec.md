# Web frontend — M4 "it remembers": the RATIFIED spec

**Written 2026-09-01 on branch `web-frontend`, at the M4 handover head (`5b710cd`).**

**This is the ratified spec the handover said did not exist.** It supersedes
`2026-08-31-web-frontend-m4-prep.md` and `2026-09-01-web-frontend-m4-handover.md`
wherever they disagree with it. Both remain readable as the record of how the
ground was surveyed; neither is authority any more.

**Every file:line in this document was opened at `5b710cd` while writing it.**
That sentence is load-bearing: the audit behind this spec found that the prep and
the handover between them carry dozens of citations that are off by one to three
lines, several that name the wrong function, and three that are substantively
false. A spec written by copying those cites forward would have inherited all of
them. §2 lists what was wrong; the rest of this file cites only re-opened lines.

---

## 0. How this spec was produced

Nine read-only investigators over the worktree at this head, each paired with an
adversarial verifier whose instruction was to refute rather than agree. Eighteen
agents, ~2.8M tokens. **The verifiers refuted 83 claims**, including the central
recommendation of the investigation's own highest-stakes report — a proposed CSS
mechanism that would have shipped a visibly broken board, cleared by a
measurement that sampled only the first row of a four-row pyramid.

That is recorded here because it is the method, not a mishap. This lane's
documented failure mode is *a mechanism validated by a metric insensitive to the
way it fails*, and the fan-out reproduced it inside its own investigation before
it could reach the code. The refutation is why §3.1 says what it says.

---

## 1. The rulings

All owner, 2026-09-01, ruled against the measured options in this document's
investigation.

| # | Question | Ruling |
|---|---|---|
| **Q1** | Where does the scrubber live? | **Inside the log panel**, with the board staying full-size on `.stage`. See §3.1 for the one refinement inside this ruling and its justification. |
| **Q2** | How far does the pinned viewer reach? | **The replay route only.** `/at/{k}` pins a viewer; the live path stays hot-seat and is not touched. |
| **Q4** | Does M4 introduce URL routing? | **No.** Deferred past M4. Re-opens at hosting — see §6. |
| **Q5** | What does `/at/{k}` return? | **A narrow `HistoricalFrame`**, not a full `ClientState`. |
| **F1** | Fix the `k == len` panic? | **Yes.** |
| **F2** | Put `cargo test -p yokohama-web` in CI? | **Yes.** |
| **F3** | Make the containment cell derive from the registry? | **Yes.** |
| **F4** | Add graceful shutdown? | **Yes** — early, ahead of the hosting phase that needs it. |

Carried forward from earlier rulings, **not re-opened by this spec**:

- **Pin a fixed viewer; never follow-the-mover** (owner, 2026-08-31).
- **No checkpoint ladder** — settled by measurement, prep §3.2 and Appendix A.
- **v1 is replay BROWSING, not branching.** `Session::undo` truncates with no
  tombstone and no redo.
- **The 5-Power token strip is CANCELLED** (owner, 2026-09-01). Not built, and
  its argument is not re-derived.

---

## 2. What the investigation changed — the false claims, first-class

These are corrections to tracked documents. Each was verified by opening the
cited code at this head.

### 2.1 "`build_decision_for` is called by nothing" — FALSE, in both documents

Prep §3.1 and §4, and handover §4, all state that the observer seam exists and is
uncalled. It is called:

- `crates/yokohama-cli/src/screen_model.rs:2515` — `pub fn build_decision_for`
- called by `build_frame` (`screen_model.rs:2698`)
- and `build_frame`'s production caller is the web server itself,
  `crates/yokohama-web/src/client.rs:332`, on **every** payload.

**Consequence, and it is favourable:** the observer arm already fires in the
shipped server whenever the pump exits with a bot as current player. The pinning
machinery M4 needs is not new code — it is a seat argument. This makes the Q2
fence materially cheaper than the handover priced it.

**A second-order defect found while verifying this, which M4 must not lean on:**
the observer arm's own doc (`screen_model.rs:2506-2510`) says it *drops* the
`not_offered` near-miss rows. It does not — it recomputes them against an empty
`offered` set. The test asserting otherwise is **vacuous**: its fixture is a
freshly finalised game sitting at `AdditionalActionFirst`, which trips both of
`build_not_offered`'s early returns. **M4 does not depend on this either way**,
but it is now on the record and the spec does not cite that doc comment.

### 2.2 "The ~1.2 µs figure has been re-measured by nobody" — FALSE

Handover §6 bullet 1 says the replay-cost figure is un-re-measured and lists
re-measuring it as owed before the spec cites it, adding in §7 step 3 that "it is
one command." Both are false. **Prep §3.2 re-measured it in full** — 1.50 µs per
event by HTTP slope, a 0.19 ms intercept, 0.864 s to sweep 1,091 events in
process — with a stated method, and Appendix A rows A1–A6 carry the readings.
The prep also says explicitly that wave0's 1.2 is *not* reproduced exactly and
declines to restate it.

**What is actually true:** the *quantity* is measured; *wave0's specific figure*
is not reproduced; and the re-measurement is a harness, not one command.

**This spec cites prep A2 — 1.50 µs/event, that Mac, at `513484d`** — with its
machine tag, and does not re-run it. Core is byte-identical to master and the
measured path did not move.

### 2.3 "M3.5 touched `crates/` in one place / the server anchors are unchanged" — FALSE

Handover §3.4's own introductory sentence is contradicted by its own table three
rows later, and by the diff: **+11 lines were inserted into
`crates/yokohama-web/src/lib.rs`**, shifting every handler below line 471 by
exactly 11. The prep's `lib.rs:722` (`async fn log`) and `:589` (`async fn why`)
were correct at `513284d` and are now **`733`** and **`600`** — verified. The
handover's `795`/`797` are the *route registration* lines inside `app()`, a
different referent silently swapped in.

### 2.4 §6.5's reveal-refusal conclusion is WRONG for the web

The prep says a scrubber stopped on a `RecordedEvent` that is not a `Move` gets
the typed `Refusal::NotAMove` → wire kind `"not_a_move"`. Through **this API** it
never will: `why` tests `LiveMode` at `why.rs:399` **before** `NotAMove` at
`why.rs:408`, and every reveal-bearing record is a live record. So a per-event
`why` at a reveal answers **`live_mode`** at every index. The UI must word that,
not `not_a_move`.

### 2.5 The provenance argument's premise is wrong on the create path

Prep §2.1 argues `/at/{k}` needs no provenance check because "a game can only be
resident because it already passed those gates." A game created through this
server goes through `Session::new_sim` (`games.rs:297`), **not**
`Session::from_record` (`games.rs:344`), so it has passed neither gate.

**The conclusion survives and the reason is restated:** `/at/{k}` is a projection
over a record the server already holds, and adding a provenance verdict to a
projection would make it a second, divergent authority on record validity. It
does not check, because checking is `from_record`'s job on the path where a
record enters from disk.

### 2.6 Storage: there is nowhere to put per-game metadata

Measured: the store persists **exactly a bare core-owned `GameRecord` JSON** as
`<id>.json`. `Games`' `live` / `written` / `deleted` maps are documented,
process-local cache. Therefore:

- **Splicing a key into the record JSON is dead.** `GameRecord` has no
  `deny_unknown_fields`, so serde **ignores** rather than preserves an unknown
  key, and `Session::to_record_json` serialises a fresh clone — the next
  `persist` rewrites the file and destroys it.
- **A sidecar is dead for M4.** `FsStore::path` hardcodes `.json`, so it needs a
  new `SessionStore` method; and a sidecar through the existing trait **appears
  as a phantom game in `GET /api/games`**. Measured, it also survives a `DELETE`
  and is inherited by a different game later created under the same name.
- **Deriving it is dead.** A bot-free table persists `run: {"schema":1,"seats":[]}`
  — zero seat rows, nothing to derive from.

**This is why the Q2 ruling (replay-route-only) is the right one and not merely
the cheap one: it needs no persistence at all.** See §3.2.

### 2.7 The containment cell cannot see a board inside a panel

`web/e2e/the_screen_itself.spec.ts:619` filters descendants whose right edge
exceeds the body's. `.board` carries `overflow-x: auto`, and **a clipping scroll
container contributes no scrollable overflow to an ancestor's `scrollWidth`** —
so a 992px board inside a 670px panel body **passes** this cell while being
visibly broken. The prep's original instinct ("a legibility argument, not a gate
violation") was right about the gate and wrong to file it as minor.

It also iterates a **hand-written list of six labels** at `:630`, not the
14-entry `SURFACES` registry — and that escape has already fired: `54f0efd` added
`tracks` and `buildings` without touching `web/e2e/` at all. **F3 closes this.**

### 2.8 Registry count — ~~FOURTEEN, not thirteen~~ WITHDRAWN; the handover was right

**This subsection was itself false, and it is left standing with its correction
because §2 is the list of other documents' false claims and had no business
containing one.**

At `5b710cd` the registry holds **13**. It reads 14 only in a tree where M4 has
already added `{ key: "help" }` — which is where it was counted. The handover
said 13 and was correct.

It propagated twice before the review caught it: a comment in
`the_screen_itself.spec.ts` said "eight were never measured" (seven), and §4
item 12's guard landed as `toBeGreaterThanOrEqual(13)` — a floor one BELOW the
real count, tolerating exactly the disappearance that derivation exists to
catch. Both fixed; the floor is now 14.

---

## 3. The design

### 3.1 The host — the log panel, board on the stage, panel stays MODAL

The ruling is the log panel. The measured problem, and why no board goes in it:

| viewport | panel | body content box | 4P board | horizontal |
|---|---|---|---|---|
| 1440 | 704 | 670 | 992 | scrolls 322px |
| 1280 | 704 | 670 | 992 | scrolls 322px |
| 1024 | 960 | 926 | 992 | scrolls 66px |

*(Independently re-derived twice in this investigation; the handover's table is
correct.)* Shrinking the tile closes the horizontal axis and nothing else:

- A panel-scoped `--tile-w` rule **silently does nothing without `!important`** —
  `Board.tsx:76` writes it as an inline custom property. Measured: the used value
  stays `10rem` and the board still reads 992/670.
- At the fitting size the printed power ladder falls to **~5.22px cap height**
  against the **7.90px** M3.5 itself measured as legible, and the longest area
  name shows **~32%** of itself.
- **The vertical axis cannot be closed at any tile size**: ~205px of scroll at a
  900px viewport, because narrowing the tile makes the caption *grow* (art
  118→78px, caption 97→138px, as `.tile-row`'s `flex-wrap` rewraps).
- A percentage-based auto-fit **tapers the pyramid 26.5% row over row**, because
  `.tile` resolves `%` against `.board-row` while the stagger's `marginLeft`
  resolves against `.board`.

**So: no board enters the panel. The panel carries the scrub track and the log
with the current stop marked. The existing `.stage` board
re-renders at `k` beside it, full size.**

**The panel stays MODAL — `showModal()` is unchanged.** This is a refinement
inside the Q1 ruling and it is justified by M3.5's own design statement at
`web/src/styles.css:556-559`:

> *"`showModal` makes the background inert, which is correct — these are
> read-only reference surfaces and nothing behind them is answerable — but the
> board must stay LEGIBLE beside the drawer, which is the entire reason for
> making it a drawer. A conventional dim would undo that."*

Both halves land in M4's favour. The backdrop is deliberately near-clear
(`.surface::backdrop` at `styles.css:559`), tuned for exactly this. And the
condition M3.5 gives for modality being correct — *nothing behind is answerable* —
is **true by construction of a historical board**: a replay frame passes no
`onTileClick`, so every tile is an inert `<div>`. Inertness is the correct
semantics for a read-only position, obtained free.

**What this buys:** zero change to `Surfaces.tsx`'s open path, and **the
"a panel really is modal" cell at `:666` is untouched.**

**Layout already exists.** `styles.css:566` —
`.app:has(.surface[open]) .deck { padding-right: min(44rem, 55vw) }` — already
insets the deck clear of the drawer above 1201px. And the containment cell is
already named *"every panel fits its drawer, **and the board stays readable
beside it**"*. M3.5 built for this.

**Below 1201px there is no drawer** (`styles.css:562` is inside
`@media (min-width: 1201px)`), the panel is a full-width modal, and the stage has
already gone to a column. **The scrubber's board-beside-panel reading is a
≥1201px affordance and the spec claims it at no other width.** At narrower
widths the panel covers the board and scrubbing is track-and-log only. This is
stated, not discovered later.

### 3.2 The viewer pin — replay route only, no persistence

`GET /api/games/{id}/at/{k}` builds its frame for **one seat that is fixed for
the life of the game** — `replay_seat` (`crates/yokohama-web/src/client.rs`),
the lowest non-bot seat. The live path is **not touched**.

> **CORRECTED 2026-09-01, after the adversarial review.** This first specified
> `viewing_seat` and described it as not following the mover. `viewing_seat`
> **is** follow-the-mover — it returns `current_player` whenever that seat is
> human. Measured against the then-shipped code: one sweep of `0..=total`
> returned seat 1's hands; eight inputs later the same sweep returned seat 0's;
> union `{0, 1}`. The pin was per-REQUEST, so a sweep that merely straddled a
> turn change harvested both seats — no waiting, no second pass.
> `replay_seat` is stable because `SeatFacts::is_bot` is fixed at creation, and
> `the_replay_pin_does_not_move_when_the_turn_does` spans a turn boundary and
> fails if the pin moves.

**What this closes.** The threat prep §3.1 names and measured (A10) is that the
same unauthenticated URL serves seat 0's hand, then seat 1's after a turn change
— so a follow-the-mover `/at/{k}` sweep would read **every human seat's whole
hand history in one pass, at will, from a URL**. Pinning `/at/{k}` makes a sweep
return one seat's hands and no other's. The per-turn live disclosure is
unchanged, and unchanged is the point: it is the hot-seat model the CLI is built
on, and one operator at one screen is the product.

**What this deliberately does NOT claim.** It is not privacy, not authentication
and not a per-person fence. §5 says so in the terms that matter.

**And a limit the first draft did not state, which shrinks the claim further.**
`GET /api/games/{id}/log` is unauthenticated and defaults to the WHOLE log, and
the CLI's event wording names order cards as they enter and leave a hand — so a
game's hand HISTORY was already legible to anyone holding the id, by a route M4
does not touch. The sweep this pin prevents was never the only way to get it.
What the stable pin still buys, honestly: `/at/{k}` cannot be used to read a
**second seat's private frame**, at any `k`, ever.

**Why no persistence:** §2.6. A per-game stored seat has nowhere to live that is
not a phantom game, a destroyed key, or a semantic squat on a core field, and the
harvest is what the ruling set out to stop.

**The mutation-proof cell this owes** (the single most important test in the
milestone): build an all-human 2P game, walk `k = 0..=n`, assert that **exactly
one seat's `Hand` panel ever carries cards across the whole sweep**. Mutation-
proof it by flipping the viewer to follow-the-mover and watching it fail.

**Read `build_dashboard` before writing it** (`screen_model.rs:2675`): a frame
carries exactly one hand panel, so a per-frame reading of that assertion is
vacuous. The assertion is over the **union across the sweep**, which is what the
prep's word "ever" already meant.

### 3.3 The route

```
GET /api/games/{id}/at/{k}  ->  200 HistoricalFrame | 404 | 409 | 500
```

**`k` is range-checked by the handler against `record.events.len()` itself**, and
a `k` outside `0..=len` is a **404 naming the bound**. Every `ReplayError` from
`state_at` is then a **500**.

This is deliberately *not* the prep's advice (compare `ReplayError::event_index`
against the length to disambiguate). The reason is `record.rs:324-326`:
`ReplayError` is a **struct**, not an enum — `pub event_index: usize` and
`pub message: String` — and its own doc says a caller branching on `event_index`
cannot distinguish past-the-end from a last-event failure without parsing the
message. **Range-checking first makes the ambiguity unreachable** instead of
navigating it. One line, and it cannot rot.

- **`state_at` never clamps** — `record.rs:568` says so in its own refusal text —
  and the range check runs before the setup preamble.
- **The domain is `0..=len`, inclusive at both ends.** The track has `len + 1`
  stops and the last one is the present, with no event to name.
- **A locked session SERVES this route.** `/at/{k}` is a projection, and the rule
  is stated in code at `lib.rs:608-617`: *the lockdown gate fences COMPUTATION,
  not projection*. The `surface!` macro (`lib.rs:482`) never reads
  `record().lockdown` at all, so a projection cannot gate unless someone adds a
  check. **Nobody adds one.** A locked game is exactly the one whose history you
  want to read.
- **The handler HOLDS the per-game mutex** across its body. `input` already holds
  the same mutex across a bot search and a store write; a ~2 ms projection hold
  is far below the existing worst case, and `repl.session.record().clone()` is
  not free either. This answers prep §6.7's "measure before choosing" from the
  repo rather than with a new benchmark, and does **not** copy `why`'s
  lock-dropping reflexively.
- **It routes through `Games::handle`**, so `k == len` is guaranteed equal to the
  live frame. The pump-and-write on a cache miss (`games.rs:333`) is inherited
  deliberately: a read-only rebuild that skipped the pump would cache an
  un-pumped handle.

### 3.4 The `k == len` panic — F1

`Session::event_description` (`crates/yokohama-cli/src/session.rs:490`) is
`&self.event_descriptions[i]` — **a bare `Vec` index**. `k` is valid for
`state_at` across `0..=n` but this panics at `k == n`. The web crate has **no
`CatchPanicLayer`** (`lib.rs:783-801` is 15 routes, `.with_state` and
`.fallback` — **zero `.layer(...)`**), and a panic under the per-game mutex
poisons that game's lock until eviction or restart.

M4's own route is what makes this reachable from a URL, so M4 closes it:

1. The handler never asks for a description at `k == len` — there is no event
   there to describe, by §3.3's own domain statement.
2. `event_description` gains a checked accessor returning `Option<&str>`
   (`Session::event_description_at`). **The bare-index method keeps its
   signature** so no existing caller changes.

> **CORRECTED after the adversarial review.** This read "**two independent
> fixes, both, because either alone is one mistake from re-opening**". They are
> not independent. Fix 1 is the whole of the defence; `event_description_at` has
> **zero production callers** — only its own unit test uses it — so it is a safe
> accessor available to a future caller rather than a second line of defence.
> Calling it one overstated the guard, in the section about not overstating
> guards.

### 3.5 The payload — `HistoricalFrame`

```rust
pub struct HistoricalFrame {
    pub index: usize,             // k
    pub total: usize,             // record.events.len()
    pub frame: FrameModel,        // nests the EXISTING type
    pub to_move: u8,              // state_k.current_player
    pub over: bool,
    pub deck_remaining: usize,
    pub end_game_triggered: bool,
    pub end_triggers: String,
    pub scores: Option<...>,
}
```

Every field re-derives from `state_at(record, k, rules)`. The six that would be
present-tense lies if inherited from `ClientState` — `over`, `scores`,
`deck_remaining`, `end_game_triggered`, `end_triggers`, `your_turn` — are the
argument for the narrow type. **`your_turn` is deliberately ABSENT**, and the
audit sharpened why: it is the one field of the six that is *not* a function of
`GameState` alone, because it compares against a seat that lives on the `Repl`.
It is also meaningless for a position nobody is playing.

**Absent by construction, not answered:** `prompt` (built from `Repl::sub`, an
in-memory sub-prompt no record holds) and `autosaved` (an unconditional `true`,
where both values are slightly false about a past position).

**`warnings` is also absent** — a seventh present-tense field the prep's table
omits. It is a load-time session fact, not a property of `state_k`.

**`frame` nests the existing `FrameModel`.** This is what makes the wire cost the
low end of §4's range rather than the high end.

### 3.6 `tile.reachable` — the decision made explicitly

`build_board` (`screen_model.rs:1938`) derives `tile.reachable` from the legal
list, so a historical board built with `legal: &[]` **silently marks every tile
unreachable with no error**.

**Ruling: pay the enumeration.** `/at/{k}` calls
`get_legal_moves(&state_k, rules)` and passes it, so a historical board's reach
marks are the real ones for that position.

The cost is negligible and the reasoning is structural, not a fresh benchmark:
`state_at` already runs one `get_legal_moves` per replayed `Move` event via the
checked `apply_move`, so one more enumeration is strictly cheaper than one more
event — under 1% of the request even at `k = 0`, the worst relative case. The
live path already pays exactly one per payload at `client.rs:337`.

**Known and accepted:** `build_frame` passes the same `legal` to the board
unconditionally, so a pinned-viewer historical frame still shows the **actor's**
reachable set. President destinations are public, so this discloses nothing — but
it is asserted rather than assumed, because **nothing at HEAD pins it in either
direction**: the existing assertion is vacuous (its fixture sits at
`AdditionalActionFirst`, where no President move can be legal) and the wire
corpus has the identical blind spot (`watching.json` is the only observer payload
and it is at the same phase with zero marks). **M4 owes a non-vacuous cell here
whichever way it rules.**

---

## 4. The work, with its guard

No item is done until its guard passes. TDD: failing test first.

| # | Work | Guard |
|---|---|---|
| 1 | `HistoricalFrame` in `client.rs`; `at_k` handler in `lib.rs`; route registered | `k == len` returns a frame whose board and dashboard equal the live frame's |
| 2 | Range check → 404 naming the bound; every `ReplayError` → 500 | `k = len + 1` is a 404 whose body names `len`; a corrupt record at a valid `k` is a 500 |
| 3 | **F1** — the `k == len` description panic, both fixes | A request at `k == len` returns 200; a follow-up request on the same game succeeds (proves the mutex is not poisoned) |
| 4 | The viewer pin | **The sweep cell** (§3.2), mutation-proofed by flipping to follow-the-mover |
| 5 | Locked-session behaviour | One test, both directions: `/at/{k}` serves a locked session AND `why` still refuses at the same index |
| 6 | `tile.reachable` at `k` | Equals `president_destinations` of that `k`'s legal list, on a fixture **where the phase permits a President move** — the existing pin is vacuous (§3.6) |
| 7 | Non-`Move` events | A frame at a reveal index still builds; the per-event account words **`live_mode`**, not `not_a_move` (§2.4) |
| 8 | Wire corpus + `contract.ts` | `UPDATE_WIRE_FIXTURES=1`, then **read the diff** — it is a contract |
| 9 | Client: scrub track in the log panel; `.stage` renders at `k`; "back to live" | Component tests over committed fixtures only — never a hand-authored payload |
| 10 | Debounce | e2e counts network requests across a drag (`page.on("request")`). A cell that only checks the final frame passes a scrubber firing one request per pixel |
| 11 | Board-beside-drawer at ≥1201px | The `:619` containment cell, extended; and an anti-vacuity assertion that the stage board is **present and non-empty** at the scrubbed `k` |
| 12 | **F3** — containment cell derives from `SURFACES` | The cell covers all 14 registry surfaces; verify by adding a throwaway 15th and watching it get picked up |
| 13 | **F2** — CI runs `cargo test -p yokohama-web` | A deliberate server-side field rename goes **red** in CI. Today it ships green |
| 14 | **F4** — graceful shutdown | **NO GUARD SHIPPED** — see the note below |

**F4 shipped with no test, and that is a gap rather than something to gloss.**
Nothing sends SIGTERM, nothing touches `shutdown_requested`, and there is no
integration test of the binary's shutdown path — so the behaviour rests on
axum's documented contract plus a doc comment, in a milestone that opens by
insisting a doc comment is not evidence. Closing it wants a spawned-binary test
that issues a slow `POST /input`, sends SIGTERM mid-flight, and asserts the
record on disk carries the move.

**Wire-contract cost — corrected.** The prep's "eleven sites" is a **ceiling, not
a floor**. Only five are mechanically forced (proven by executing `tsc`), and
three fixtures already in the corpus ship with two, four and four sites. Because
`HistoricalFrame` nests the existing `FrameModel`, the discretionary sites cost
one entry each rather than one per nested shape. Prep sites 1 and 2 also name the
wrong mechanism: a route-served payload joins the corpus by `out.push`, not by
the walk's `capture(...)` deduper, so "the owed list" is not a site.

**Per-edit after every task:** `cargo test -p yokohama-web` · `cargo test -p yokohama-cli` ·
`cd web && npm run typecheck && npm test` · `npm run e2e` ·
`cargo clippy --workspace --all-targets -- -D warnings`.

**Baseline at this head:** `npm test` 222 · `npm run e2e` 21 ·
`cargo test -p yokohama-web` 65 · wire corpus 42 · `cargo test -p yokohama-cli`
849+2 · `cargo test -p yokohama-core` 1577+29, byte-identical to master ·
`SEMANTICS_REV` 7.

**`crates/yokohama-core` stays byte-identical to master.** Nothing in M4 needs a
core change; every mechanism it uses is already `pub`.

---

## 5. What is NOT claimed

- **`/at/{k}` is not privacy.** The pin stops a one-pass harvest. It is not
  authentication, not authorization, and not a fence around a person.
- **There is no identity in this crate.** Verified: zero `.layer(...)` on the
  router, no `HeaderMap`/`HeaderName`/`HeaderValue`/`TypedHeader` anywhere, and
  the only two request extractors are `JsonBody` and `QueryArgs` — the exact two
  sources M2 §7 says identity is "never" read from. **The M2 sentence "the
  identity story in this crate is 'read a configurable header'" describes an
  intention that was never built**, in the present tense. Recorded as a fourth
  false self-claim in this lane.
- **A game id is not a capability.** `GET /api/games` returns every id in the
  store, unauthenticated, so the id space is enumerable in one request rather
  than guessable.
- **No layout claim below 1201px.** The board-beside-the-drawer reading is a
  ≥1201px affordance (§3.1).
- **No wall figure is re-measured.** The 1.50 µs/event slope is prep A2's, on
  that Mac, at `513284d`, and is cited with its machine tag.
- **No branching.** "Play from here" is not in M4. §6 records the shape it would
  take.
- **CI still cannot see a layout regression.** F2 adds the Rust half; the
  Playwright walk stays local by ruling, so a layout claim from this lane is
  still checked by hand on one machine.

---

## 6. What the M4 close doc owes (owner direction, 2026-09-01)

The M4 close is the **hosting lane's entry artifact**, not merely M4's record.
It carries the routing so the hosting work is not invented later:

1. **The target shape** — Cloudflare Worker at `yokohama.moore.nyc` behind the
   existing Access rule, with Modal as the container compute layer.
2. **Blockers already found, on the record now rather than at deploy time:**
   - `main.rs:18` binds **`127.0.0.1` hardcoded**. A container needs `0.0.0.0`.
   - **F4** closes the scale-to-zero SIGTERM hole. `Cargo.toml` already enables
     tokio's `signal` feature, unused.
   - **Access header verification does not exist and is not free.** Cloudflare
     injects `Cf-Access-Jwt-Assertion`, `Cf-Access-Authenticated-User-Email` and
     a `CF_Authorization` cookie, and Cloudflare's own guidance is that
     *validating the header alone is not sufficient — the JWT signature must be
     confirmed to avoid identity spoofing*. A header the origin merely trusts is
     spoofable if the origin is reachable without going through Access.
   - `GET /api/games` enumerates every game to any caller (§5).
3. **The deferred Q4.** URL routing re-opens here, because a *shareable* replay
   position is a different feature from a scrubber and it is the one that needs
   hosting to exist.
4. **"Pick up with a bot from a replay point"** (owner, 2026-09-01). Not M4.
   The cheap shape neither prior document considered: `state_at(k)` plus the
   record's first `k` events is a complete, legal `GameRecord` prefix, so
   "play from here" can be a `POST` writing a **new game id** from that prefix
   with a bot seat assigned — a save-as, needing no tombstone, no redo and no
   branch semantics in `Session`. To be costed, not assumed.

---

## 7. Read order for whoever picks this up

1. `CLAUDE.md` → `guidingpath.md` current state + decision-log tail
2. `2026-09-01-web-frontend-m3.5-spec.md` §2 and §6
3. **This file** — the authority
4. The prep and the handover, for *how the ground was surveyed only*. §2 above is
   the list of what they get wrong; do not carry a citation from either without
   opening it.

---

## 8. The adversarial review, and what it changed (2026-09-01)

Six lenses over the finished M4 diff — vacuity first, then self-claims, route
correctness, the viewer pin, the client, and the tracked documents. Every
finding below was proven by breaking something and watching a test's behaviour,
not by re-reading a sentence. **Six HIGH; the three most expensive were the
milestone's own claims about itself**, which is the same result this lane has
had at every previous milestone and the reason the pass is worth its cost.

### 8.1 The security claim was backwards

§3.2 and four code sites said the replay route "does not follow the mover".
`viewing_seat` **is** follow-the-mover. Corrected in code (`replay_seat`) and in
every doc site, and now guarded by a cell that spans a turn boundary. Full
account in §3.2.

### 8.2 Three new cells could not fail

- **The inert-board cell** (`the_screen_itself.spec.ts`) asserted a replayed
  board offers no pressable tile — but at the moment it measured, the LIVE board
  had none either, because `picking` is false outside a placement picker.
  Deleting `&& !past` from `App.tsx` left it green. It now walks to a real
  picker first and asserts the live count is nonzero before asserting the
  replayed one is zero.
- **The debounce cell** counted requests across eight keypresses that complete
  in ~25 ms, which React's effect cleanup coalesces on its own; setting the
  debounce to `0 ms` left it green. The presses are now spaced past a frame
  each, so an undebounced client issues one per press.
- **The log-jump cell** asserted `toHaveBeenCalledWith(0)` against entry 0,
  which a handler hard-coded to `move(0)` satisfies. It now clicks two
  different non-zero lines.

### 8.3 "e2e 23/23 green" was one run of four

F3 — deriving the containment loop from the nav — correctly added **Final
score**, and `GET …/result` is a **409 by design** on a running game, which
`watchForBreakage` counts as breakage. Three runs in four failed. `watchForBreakage`
now takes a per-call expected-refusal list, opted into by that one cell.

### 8.4 The Help panel overflowed its drawer by 794px

Four 1440px screenshots with **no CSS rule anywhere** inside a 651px panel. The
containment cell could not see it: `loading="lazy"` leaves the images 0×0 at
measure time. Both fixed — a `.shot img` rule, and the cell now settles images
before measuring.

### 8.5 Documents wrong about their own artifacts

- §2.8 "corrected" the handover's registry count and was itself wrong (§2.8).
- The README's alt text for `replay.png` named figures from a **superseded
  generation of that same screenshot** — written, then the shots regenerated,
  then not re-read. Rewritten without volatile numbers so a regeneration cannot
  make it false again.
- `Help.tsx` named a control "Card reference" (it is "Cards") and counted four
  entries behind it (five), and its `placement.png` alt described selections and
  a running fee that are not in the picture.
- `shots.spec.ts` documented a command that runs nothing, in the one file a
  reader would copy it from.

### 8.6 Two guards that were never as strong as written

- **`event_description_at` has zero production callers** — "two independent
  fixes" was one fix plus an unused accessor (§3.4).
- **F4 shipped with no test at all** (§4).

### 8.7 A test helper that quietly capped every route cell

`play` answered `0` to a retreat prompt forever and never sent `confirm`, so it
deadlocked at the first Recovery and every M4 route cell ran on a 17-event
opening regardless of the `moves` it was given. The whole-game gate already had
the right shape; this helper was written without it.

### 8.7b A pre-existing flake, diagnosed and fixed

`the answer bar is pinned to the bottom at every scroll offset` — one of M3.5's
seven CSS-visible cells — failed roughly two runs in three, **on a clean tree as
well as on M4's**, so it was not M4's doing. Diagnosed rather than retried: its
probe hit-tests the FIRST `.option`, but `.options.scrolls` is a scroll
container, so once ordinary play scrolls it, row 0 sits outside its own clip and
`elementFromPoint` returns whatever is painted there instead. It reported "the
first answer row was covered" on a screen where the commentary occupied **six
pixels**. The probe now scrolls the list to its top before hit-testing, which is
the only way the question it asks — *does the commentary squeeze the list until
its first row is unreachable* — is about the commentary at all. Three
consecutive runs green, with the anti-vacuity line still reporting a real
squeeze (commentary peaked 112px, list floored 72px).

### 8.8 Not fixed, recorded

- **The pin's value is smaller than §3.2 first implied**, because `/log`
  already discloses hand history unauthenticated. Stated in §3.2 rather than
  fixed: closing it is a scoping decision about an existing route, not M4's.
- **`Path<(String, usize)>` rejects a non-numeric `k` with a PLAIN-TEXT body**,
  breaking this API's own "every refusal is a JSON sentence" invariant. `/at/{k}`
  is the first numeric path parameter in the crate, so it is the first route to
  meet it. A typed extractor mirroring `JsonBody`/`QueryArgs` is the fix; it is
  a crate-wide convention change and is left for a follow-on rather than smuggled
  into M4.
