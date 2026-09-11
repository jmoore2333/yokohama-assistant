# Web frontend — M4 "it remembers": the handover into the next session

**Written 2026-09-01 on branch `web-frontend`, at the M3.5 close head (`c1fb1c1`).**

**This is a HANDOVER, not a spec and not an approval.** It exists because M4 has
a large, good prep document written *before* M3.5 landed, and M3.5 moved the
ground under parts of it. Read this first, then the prep document, and treat this
file as the correction layer over it.

**Nothing here authorises a line of code.** §5 lists what the owner still has to
rule.

---

## 1. Does a full documentation path exist? — Yes, with one gap

| Document | What it is | Status |
|---|---|---|
| `2026-08-31-web-frontend-m4-prep.md` (966 lines) | The substantive M4 groundwork: the seam, what is settled, the hazards, a test plan, the owner's forks | **NOT an approved spec** — it says so at the top. Same posture M3.5's prep had |
| `2026-09-01-web-frontend-m4-handover.md` | This file | The correction layer M3.5 forced |
| **A ratified M4 spec** | — | **DOES NOT EXIST.** This is the gap |

**The path is therefore: prep + this handover → owner rulings on §5 → a ratified
spec → build.** That is exactly the shape M3.5 followed, and it is the shape that
worked: M3.5's prep was audited before it was built, the audit returned 7 HIGH
and 21 MED **all landing on the prep's own sentences**, and four of those changed
what got built. **Budget for the same audit here.** The prep document is better
than M3.5's was, but it is one milestone older and its client half is now stale
in every citation (§3).

### Read order for a fresh session

1. `CLAUDE.md` → `guidingpath.md` Current state + the decision log's tail (the
   M3.5 row is the last one)
2. `2026-08-31-web-frontend-m2.md` → `-m2.5.md` §5 → `-m3-spec.md` → `-m3.md` §9
   → `-wave0.md`
3. `2026-09-01-web-frontend-m3.5-spec.md` — **especially §2 (what an audit did to
   the previous plan) and §6 (the close, and where the spec was wrong)**
4. `2026-08-31-web-frontend-m4-prep.md` — the substance
5. **This file** — what of it is no longer true

---

## 2. What M4 is, in one paragraph

A replay scrubber: a control that walks the game backwards and forwards over its
own event log and draws the position at each stop, served by a new route
`GET /api/games/{id}/at/{k}` over `record::state_at` (`record.rs:561`). It adds
no persistence and no new state — it is a projection over a `GameRecord` the
server already holds. The saved-game half already shipped in M2.

---

## 3. What M3.5 changed about M4's inputs

**Everything in this section was measured at `c1fb1c1`, not reasoned about.**

### 3.1 The prep's §5.1 host analysis INVERTS — this is the important one

The prep costed "host the scrubber on the log surface" as the cheapest option by
a wide margin, and priced its one drawback like this:

> *"At 1440 the panel's content box is 1344px against a 4P widest row of
> 6·224 + 5·6.4 = 1376px … so it scrolls by ~32px. That is a legibility argument,
> not a gate violation."*

**Both numbers are now wrong, in opposite directions, and the conclusion flips.**
Measured, a 4P board inside the open panel:

| viewport | panel | body content box | 4P board | result |
|---|---|---|---|---|
| 1440 | **704** (drawer) | 670 | 992 | **scrolls 322px** |
| 1280 | **704** (drawer) | 670 | 992 | **scrolls 322px** |
| 1024 | 960 (modal) | 926 | 992 | scrolls 66px |

Two M3.5 changes did it: the panel is a **right-hand drawer above 1201px**
(`min(44rem, 55vw)`), and the tile scales to the board so a 4P board is 992px at
10rem rather than 1376px at 14rem. At the widths the product targets, a third of
the board would be off-screen *inside* the panel.

**And a second, independent argument the prep could not have made:** the panel is
now a real `<dialog>` opened with `showModal()`, so while it is open the entire
page outside it is **inert and behind a backdrop**. A scrubber hosted there makes
the live board un-clickable and dimmed — correct for a read-only reference
surface, wrong for the primary view of a feature.

**A third host now exists that did not when the prep was written: `.deck`.** M3.5
built a pinned container at the bottom of the viewport holding the warnings, the
prompt and any error, with a stated space policy (controls never shrink, the
answer list floors then scrolls, commentary yields first). A scrub track is
exactly the shape that belongs in it, with the existing `.stage` board re-rendered
at `k`. That is the prep's option 2 ("a mode that replaces the stage") with the
expensive half already built.

**No ruling is made here.** The prep's option 1 is now the weakest rather than
the cheapest, option 2 is cheaper than it was, and there is a third. §5.Q1.

### 3.2 Two of the prep's open questions are now closed

- **Q5 — "Does CI grow to cover this lane before M4 lands?"** → **YES, it did.**
  `.github/workflows/test.yml` has a `web-client` job running `npm ci`,
  `npm run typecheck` and `npm test` (owner ruling, M3.5 fork 4). **Read the
  limit, which is written into the job:** it catches the component and
  wire-contract half and **no layout regression at all** — `styles.css` is
  imported by `main.tsx` alone and vitest runs jsdom. The browser walk stays
  local by ruling. So M4's route and DTO will be covered; M4's *layout* will not.
- **Q3 — the undo hole** → already marked answered in the prep (`af706de`), still
  true.

### 3.3 The prep's §8 is MOOT — the owner dropped it

§8 is "the M4-adjacent win: the per-seat 5-Power token strip", a replay
projection over `state_at`. **The owner cancelled it on 2026-09-01**, in these
words: *"in terms of collecting the tokens knowing which ones were taken isn't
something we need to build beyond what the CLI has."* What they wanted was the
token FACE visible on the board, which M3.5 shipped in its printed 5-space.

**Do not build §8, and do not re-derive its argument.** The mechanism it
documents (`apply_move.rs:927`'s `slot.take()`, `resolve_acting_area` and the
mercenary mis-attribution trap at `actions/tracks.rs:95-99`) is correct and is
kept only so a future reader does not re-discover it. See the M3.5 spec §1.2.

### 3.4 Every client-side citation in the prep has moved

The engine and server anchors are **unchanged** — M3.5 touched `crates/` in one
place. The client anchors are all stale. Corrected at `c1fb1c1`:

| What | Prep says | **At HEAD** |
|---|---|---|
| `state_at` | `record.rs:561` | `record.rs:561` ✓ |
| `replay_from` | `record.rs:384` | `record.rs:384` ✓ |
| `build_frame` | `screen_model.rs:2698` | `screen_model.rs:2698` ✓ |
| `build_decision_for` (the observer seam) | `screen_model.rs:2515` | `screen_model.rs:2515` ✓ |
| `event_description(i)` | `session.rs:490` | `session.rs:490` ✓ |
| `account_parts` | `why.rs:392` | `why.rs:392` ✓ |
| `surface!` macro (lock-and-project) | `lib.rs:471-482` | `lib.rs:482` |
| `GET …/log` | `lib.rs:722` | **`lib.rs:795`** |
| `GET …/why` | `lib.rs:589` | **`lib.rs:797`** |
| The panel shell | `Surfaces.tsx:105-152` | **`Surfaces.tsx:195`** (and it is a `<dialog>` now, not a `<div>`) |
| `SURFACES` registry | `Surfaces.tsx:55-71` | **`Surfaces.tsx:68`**, and it holds **13** surfaces, not 11 |
| `why` loader (the un-passable one) | `Surfaces.tsx:64` | **`Surfaces.tsx:85`** |
| The log's `<ol>`/`<li>` | `Surfaces.tsx:352-368` | **`Surfaces.tsx:552`** |
| Inert board tile (`<div>`, no `onTileClick`) | `Board.tsx:197` | **`Board.tsx:271`** |
| Debounced fetch template | `Index.tsx:261-279` | **`Index.tsx:273`** |
| `.board-col { width: 100% }` | `styles.css:131` | **`styles.css:140`** |
| `--tile-w: 14rem` | `styles.css:49` | **`styles.css:63`** — and it is **overridden inline per board** by `Board.tsx`'s ladder, so 14rem is the 2P value only |
| `.surface-body` | `styles.css:377` | **`styles.css:586`**, and it now genuinely scrolls (`min-height: 0`) |
| `.board` | `styles.css:136` | **`styles.css:153`** |
| `.stage` | `App.tsx:126-143` | **`App.tsx:214`**, and it is `<main>` now |
| `if (!state) return <NewGame …>` | `App.tsx:55` | **`App.tsx:100`** |
| `Decision`'s props | `Decision.tsx:83-95` | **`Decision.tsx:86`**, and it takes a new `outcome` prop |
| The width gate's widths | `the_screen_itself.spec.ts:265` | **`:516`** |

### 3.5 One new mechanism M4 should use rather than invent

The prep correctly notes that `why?event=n` is API-complete and
client-unreachable, because the registry's `load` is `(game: string) =>
Promise<unknown>` and cannot carry a second argument (`Surfaces.tsx:71`, still
true).

**M3.5 created the precedent for the fix.** The `tracks` surface has a loader
that resolves a sentinel and renders from a **prop** instead:

```ts
const NOTHING_TO_FETCH = Symbol("already on the frame");   // Surfaces.tsx:66
{ key: "tracks", …, load: () => Promise.resolve(NOTHING_TO_FETCH) },  // :81
```

and `Surface` gained a `tracks?: TracksPanel | null` prop that `App` fills from
the frame. `Decision` gained `outcome?: Outcome | null` the same way in the same
milestone. **A scrub index is the same shape**, and widening the registry
contract is one option among several rather than the only one.

### 3.6 What M4 must not break

M3.5 added **seven browser cells that are the only assertions in this repository
able to see a CSS change** (mutation-tested: the pre-M3.5 gate missed
`.tile{width:20rem}`, a 600px banner, a 1200px push-down, a fixed rail over the
board, 1px tiles and transparent text). They are in
`web/e2e/the_screen_itself.spec.ts`:

- the deck pinned at three scroll offsets, every control hit-tested
- the answer list's floor under a bot's commentary (probed after every move)
- only the answer list scrolls inside the deck
- the two rules the art fix deletes, read as computed values
- the tile's box reservation with the art held at the network
- focus containment by tabbing an open panel
- per-surface containment in the drawer, at 1440 and 900
- the panel's real modality and the one-row banner

A scrub mode that swaps `.stage` interacts with the first three directly.

---

## 4. What is settled and must not be re-opened

From the prep, re-checked and still true:

- **§3.1 Pin a fixed viewer. Never follow-the-mover.** Owner ruling
  (2026-08-31). The observer seam already exists at `screen_model.rs:2515` and
  is called by nothing. The prep's measured point still stands and is sharp: the
  API has no notion of who is asking, so a follow-the-mover `/at/{k}` sweep would
  make an existing per-turn exposure harvestable in one pass.
- **§3.2 NO checkpoint ladder.** Settled by measurement (~1.2 µs/event; a
  195-event game sweeps in ~17 ms). *Carried, not re-measured — see §6.*
- **§3.3 The event index space is ONE integer**, proved from both ends.
- **v1 is replay BROWSING, not branching.** `Session::undo` truncates with no
  tombstone and no redo.

---

## 5. What the owner still has to rule

The prep's §9, re-stated with M3.5's answers folded in and its dead question
removed.

**Q1 — Where does the scrubber live?** *Newly urgent, and the options changed
(§3.1).* Hosting it in the log panel — the prep's cheapest option — now costs
322px of a 4P board off-screen at 1440 and makes the live board inert behind a
modal backdrop. Hosting it as a stage-replacing mode is cheaper than it was,
because `.deck` exists. **This is the one to rule first: everything else in a
spec hangs off it.**

**Q2 — How is the pinned viewer chosen?** Unchanged. (a) server-picks
`viewing_seat` at the live head — adds nothing, leaks nothing new, but the same
`k` answers differently as play advances; (b) `?seat=` — honest and **not a
fence**; (c) a per-game "my seat" chosen at create — a real fence, needs a
concept the API lacks and a migration. The prep proposes (a).

**Q3 — Does `/at/{k}` read the resident handle or the durable record?** Now a
design preference rather than a forced choice (the compare-and-swap closed the
disagreement). The remaining difference is a pump-and-write on a cache miss.

**Q4 — Does M4 introduce URL routing?** Still open, and M3.5 did not touch it:
`grep -rn "pushState\|location.hash\|history\.\|localStorage\|sessionStorage"
web/src` still returns nothing. A scrub position is not linkable and a reload
drops to `NewGame`.

**Q5 — What is the payload?** The prep's §5.2: a full `ClientState` at `k`
(maximum client reuse) versus a narrow `HistoricalFrame`. The prep argues for the
narrow one and names the six fields that make the case.

---

## 6. What this handover could NOT verify

Stated rather than left to be discovered, in the shape this lane now uses:

- **The ~1.2 µs-per-event replay figure** (prep §3.2, the basis for "no
  checkpoint ladder") is **carried from `wave0.md` §7 and has been re-measured by
  nobody** — not by the M3.5 prep, not by M3.5's eight-cluster audit, and not
  here. It is load-bearing for a settled decision. **Re-measure it before the
  spec cites it again**; the prep names the exact lever (`why?event=n` times a
  real `state_at` before any seat refusal, `why.rs:418` vs the refusals at
  `:423-427`).
- **The prep's §2.2, §6 (hazards) and §7 (test plan) were NOT re-verified here.**
  I checked §3, §4, §5.1 and §9. §6.2 (`tile.reachable` is derived from the legal
  list, so a historical frame must not present it as offered) and §6.1 (the
  present-tense leak) are the two I would check first — both are correctness, not
  layout, so M3.5 is unlikely to have moved them, but "unlikely" is not
  "checked".
- **Cloudflare Access in front of the deployment** is assumed by the prep's Q2(b)
  and was marked UNVERIFIED there. Still unverified.
- **Whether the audit shape scales.** M3.5's eight-cluster pass cost ~1.7M tokens
  and returned findings worth far more than that. M4 is a smaller surface. I do
  not know that the same fan-out is the right size for it.

---

## 7. A concrete first session

1. Read the order in §1. Budget the M3.5 spec §2 and §6 properly — they are the
   record of what auditing a prep document is worth on this lane.
2. **Get Q1 ruled** (§5). Render the candidates if that helps — M3.5's forks were
   ruled against pictures and it worked.
3. Re-measure the ~1.2 µs figure (§6). It is one command and it underpins a
   settled decision.
4. Verify the prep's §6 hazards at HEAD, particularly §6.1 and §6.2.
5. Write the ratified spec against the prep + this file + the rulings, in the
   shape `2026-09-01-web-frontend-m3.5-spec.md` uses: rulings, what the audit
   changed, the work with its guard per item, what is not claimed.
6. **Then** build, TDD, per-edit suites after every task:
   `cargo test -p yokohama-web` · `cd web && npm run typecheck && npm test` ·
   `npm run e2e` · `cargo clippy --workspace --all-targets -- -D warnings`.

**Baseline at this head, re-measured at the M3.5 close:** `npm test` **222**,
`npm run e2e` **21**, `cargo test -p yokohama-web` **65**, wire corpus **42**,
`cargo test -p yokohama-cli` **849 + 2** (with `YOKOHAMA_BGA_CORPUS_OPTIONAL=1`;
the two BGA captures are gitignored and absent on a fresh clone),
`cargo test -p yokohama-core` **1577 + 29** and byte-identical to master,
`SEMANTICS_REV` **7**, clippy clean. Still no gate and no roster place.
