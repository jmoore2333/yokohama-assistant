# Web frontend — M3: it explains. The spec.

**Written 2026-08-31, on branch `web-frontend`, at the M2.5 head (`31f19c1`).**
The lane's plan (`docs/temp/webresearch/PLAN.md`) states M3 as four prose
bullets and no route-level detail; the M2.5 handover names one open design
question and two owner decisions. This file settles all of it before code, so
that the four surfaces are one design rather than four dialects.

Reading order: `2026-08-31-web-frontend-m2.md` (M2's decisions) →
`-m2.5.md` (the harness, and **§5, what it does not cover**) → this file.

---

## 0. What M3 adds, and the one sentence that shapes it

`suggest`, `why`, the four card indexes, and the final score breakdown.

**`why` is the signature surface, and it is the only one that is not a
projection.** Every route the crate serves today reads a model the CLI already
built; `suggest` and `why` RUN A SEARCH. That is a real departure from
`lib.rs:9-10`'s "if it ever computes something about the game, that is a bug",
and §3.3 below amends that sentence rather than quietly violating it.

## 1. Owner rulings taken for this milestone (2026-08-31)

| # | Question | Ruling |
|---|---|---|
| **R1** | What should the `why` chart look like? | **A diverging bar chart, one row per feature, sorted by \|product\| descending**, bars growing left (hurts) / right (helps) from a centre zero line, monospace numerals in a right-hand column, **and the total footed at the bottom so the exact-sum identity is visible rather than merely asserted.** |
| **R2** | Request bodies default rather than refuse — fix now or later? | **Now, before M3 widens it.** Every request type in the crate gets `#[serde(deny_unknown_fields)]`, with a test per body proving a renamed field is a **400**. `#[serde(default)]` STAYS where a field is genuinely optional — the defect is silent acceptance of an *unknown key*, not the defaults themselves. |

R2's reasoning, restated because it is easy to lose: `NewGame`'s `start_seat`,
`seed`, `bot_seats` and `name` are `#[serde(default)]` and nothing in the crate
uses `deny_unknown_fields`, so a client that renames a field gets a **201 and a
different game**, not a 400. `suggest` and `why` both take request parameters,
so M3 widens the hole. Fixing it first means the new parameters are born fenced.

## 2. What the recon dissolved

Four things the reconnaissance raised as owner decisions are settled by code
that already exists. Recorded so nobody re-opens them:

- **A wire key for `Feature` needs no core change.** `Feature::name()`
  (`crates/yokohama-core/src/game/eval.rs:479`) is already `pub` and returns the
  stable snake_case token, and its own doc says report columns, explanation
  rendering and the T4 probe all come from there "so none of them is
  hand-typed". That token is the wire key. A `Debug` string was never needed and
  a `Feature::label()` in core is forbidden.
- **Human wording is a web concern, and the token stays on the wire.** The
  snake_case name is the stable cross-reference against `eval_probe`, the
  scenario suite and every recorded figure, so it is what the payload carries.
  Display wording is authored in `yokohama-web` and served from `/api/labels`,
  exactly as every other enum→string mapping already is.
- **`why`'s reusable half was split out FOR this.** `WhyAccount`'s doc
  (`crates/yokohama-cli/src/why.rs:560-570`) says it in as many words: *"A
  consumer that draws its own table wants the first half and none of the second,
  and it must not reimplement the first half: the refusals ARE the honesty
  gate."* The web calls `account_parts`. It does not re-derive a single refusal.
- **A `why` fixture needs no new request field.** The bot seed is OS entropy, so
  a golden over a router-created bot game churns — but `wire_fixtures.rs`'s
  `constructed()` already builds states with a fixed `BOT_SEED`, and `watching`
  is the precedent. `why` fixtures go through `constructed()`.

## 3. The API surface

### 3.1 Four new routes, all GET, all read-only

```
GET /api/games/{id}/suggest                 -> Suggest | typed refusal
GET /api/games/{id}/why[?event=<n>]         -> Why | typed refusal
GET /api/games/{id}/index/{kind}[?filter=]  -> CardIndex          kind = orders|techs|goals|sitecards
GET /api/games/{id}/result                  -> GameOutcome | 409 while unfinished
```

`event` omitted means "the most recent bot move", which is what bare `why` does
in the CLI. `kind` is a path segment because it names a different corpus, not a
filter over one corpus; `filter` is a query parameter because it narrows one.

### 3.2 GET, not POST, and the refusals are 200

Both search routes **apply nothing, record nothing and autosave nothing** —
`suggest`'s CLI contract exactly — so GET is honest about the effect even though
it is dishonest about the cost. They are idempotent in the strong sense the CLI
already guarantees: byte-identical on repeat, keyed by game identity, seat and
move count.

**A typed refusal is a 200 with a refusal body, not a 4xx.** `why` refusing
because the last move was a human's is not a client error; it is the answer. The
twelve `Refusal` variants are *data the browser renders*, and the M2.5 lesson is
that a branch nobody has ever seen is where defects live. Only a malformed
request (bad `kind`, non-numeric `event`, unknown query key) is a 400, and an
absent game is a 404.

### 3.3 The lock, and the amended invariant

**A search must not be run inside `handle.lock()`.** Every handler today holds
the per-game `Mutex<Repl>` across its whole body, which is right for a
projection and wrong for a multi-second search: it would block every other
request for that game, including the poll that draws the board.

So both search handlers **clone what they need out of the lock and drop it
before searching**. `GameState` is history-free by design and CLAUDE.md states
it is cheap to clone for exactly this reason; `Rules` is already an `Arc`; the
`GameRecord` and `BotSeats` clone cheaply enough. This is stated here because
there is no precedent in the crate and the next handler will copy whichever
shape it finds.

`lib.rs`'s module doc gains one clause: the crate computes nothing about the
game **state** — it never advances one — and the two search routes are the named
exception that reads an *opinion* about a state without changing it.

### 3.4 Request-body hardening (R2)

Three request types exist: `NewGame` (`games.rs:69`), `Input` (`lib.rs:227`),
`LogQuery` (`lib.rs:417`). All three get `#[serde(deny_unknown_fields)]`, and
so does every request type M3 adds. One test per body sends a renamed field and
asserts **400**, so the fence is proven rather than declared.

## 4. The DTOs

New module `crates/yokohama-web/src/explain.rs` for the evaluation surface,
`src/index.rs` for the card indexes, `src/outcome.rs` for the score breakdown.
**Every one is a web-side DTO. `yokohama-core` stays byte-identical to master**
— the rule the lane already enforced once by reverting a core change.

### 4.1 The explanation, and the identity it must carry

```
Term        { feature: string (Feature::name), viewer_value, opponent_value, weight, product }   // all i64
Terminal    { rank, seats, viewer_total, best_other_total, value }
Explanation { terms: Term[], terminal: Terminal | null, total: i64, progress_ppm: u32 }
```

**THE EXACT-SUM IDENTITY travels with the payload and the client foots it:**

```
total == sum(terms[].product) + (terminal ? terminal.value : 0)
```

That is `Explanation`'s own documented guarantee (`eval.rs:1267-1281`). Two
consequences the chart must respect, both non-negotiable:

1. **Zero-product rows are not dropped from the payload.** `Explanation`
   guarantees one term per feature always — all 34 — and the identity only holds
   over all of them. The CLI dims the zero rows and prints "N of 34 rows carry a
   nonzero product"; the client collapses them behind a disclosure and prints
   the same count. Dropping them server-side would make the identity
   unverifiable by the reader, which is the one thing this surface exists to
   avoid.
2. **A terminal explanation is a different picture, not an empty one.** On a
   finished game every one of the 34 products is 0 and the whole score sits in
   `terminal.value` — a bar chart over features renders empty against a nonzero
   total. When `terminal` is non-null the client draws the final-standing row
   instead of the bars, mirroring the CLI's own answer.

### 4.2 `suggest`

```
Suggest {
  seat, move: { text },            // worded by `describe_move`; the client never constructs a move
  explanation: Explanation,
  continuation: { text }[],        // root move first
  ranking: { move: {text}, value: i64, margin: i64 }[],   // best first, margin vs the suggestion
  stream_seed: string,             // u64 as a STRING, per the wire's own seed rule
  search_hit_ply_cap: bool,
  tied_at_top: number
}
```

Moves cross the wire **pre-worded**. The client's standing rule is that it never
constructs a move and never prices one (`Decision.tsx:80-82`), and typing `Move`
in `types.ts` would be a large hand-written union mirroring the engine — the
`ts-rs` question, which is still open and which M3 does not force.

`margin` is the runner-up's value minus the suggestion's, so **0 means tied** —
the wording `suggest::RUNNER_UPS_LABEL` already uses.

### 4.3 `why`

```
Why {
  ok: {
    reconstruction: { contestant, controller_version, evaluator_version,
                      index, seat, decision_seed: string, forced: bool,
                      recorded_version: number|null, current_version: number },
    move: { text },
    explanation: Explanation,
    continuation: { text }[],
    counters: { label, value }[]           // the grouped SearchStats block
  } | null,
  refusal: { kind: "<snake_case tag>", message: string, ...fields } | null
}
```

Exactly one of `ok` / `refusal` is non-null. `message` is the CLI's own
`Display` for the variant — **the wording is not re-authored**, because
`guide_sync` pins the guide to those strings and a second wording drifts from
the screen the guide documents.

### 4.4 The card indexes and the outcome

`CardIndex` mirrors `screen_model`'s index models, which M1's serde sweep
already made `Serialize`. The filter is parsed by the CLI's own
`card_prompt::parse_filter` — **there is exactly one card-entry grammar in this
project and the web does not get a second one.**

`GameOutcome` carries the ranked standings with each seat's score components. It
is a **new route, not a change to `ClientState.scores`**: `scores` is already
frozen in `game_over.json`, typed, consumed by `Decision.tsx` and asserted by
three tests, and re-keying it would churn the corpus for no gain. `scores` stays
the compact in-frame view; `/result` is the breakdown.

## 5. The seam: what widens in `yokohama-cli`

**One commit, behaviour-inert, touching nothing else.** Visibility only — no
signature changes, no restructuring, no rendering touched.

| Item | Site | Why |
|---|---|---|
| `pub mod why` | `lib.rs:23` | the module is unreachable today |
| `Refusal`, `Mismatch` (+ fields) | `why.rs:149,167` | the twelve typed rows are the payload |
| `WhyAccount`, `Reconstruction` (+ fields) | `why.rs:548,570` | the honesty gate's output |
| `account_parts` | `why.rs:392` | **the gate itself** |
| `bot::Decision`, `bot::DecisionExplanation` | `bot.rs:518,531` | `WhyAccount::explain`'s type |
| `Repl::index_placements` | `repl.rs:1706` | named by the plan; **widen, do not copy** — its doc defends a leak fence |
| `Repl::bots()` accessor | `repl.rs:67` | `account_parts` needs `&BotSeats`, and re-deriving it from provenance re-runs seed reconciliation and re-emits warnings the `Repl` already consumed |
| `pub mod card_prompt` + `parse_filter`, `IndexFilter` | `lib.rs:5`, `card_prompt.rs:464` | one filter grammar, not two |

The proof that it is inert: the CLI suite is **unmoved at 849 + 2**, the nine
`assert_capture` goldens and 18 `guide_sync` tests are unchanged, and
`yokohama-core` stays byte-identical to master.

## 6. Testing duties — the harness is the point, not an extra

M2.5 exists so that M3 is built on it. Every one of these is mandatory:

1. **Every new payload gets a wire fixture.** Extend `wire_fixtures.rs`'s corpus
   and add the import + `Widen` assignment + root `KeysOf` pin in `contract.ts`.
   `why` fixtures come from `constructed()` with the fixed `BOT_SEED`.
2. **`why`'s twelve `Refusal` variants get the exhaustiveness treatment the
   prompts got.** A real `match` with no `_` arm over
   `yokohama_cli::why::Refusal` in the test file, a load-bearing count, and a
   coverage test with a **named exemption list**. Two variants are structurally
   unreachable from an honest sim record — `Unaccounted` (every production arm
   sets `explain: Some(..)`, so a missing explanation is already a
   `Reconstruction` refusal) and `Reconstruction` itself — and they are named
   exemptions carrying that reason, exactly as `reveal` is.
3. **Component tests over server-emitted payloads only.** No hand-authored
   state; the corpus is the props.
4. **The browser walk learns to open `why`.** A dedicated spec with a bot seat,
   separate from the bot-free whole-game walk so the walk's reproducibility is
   not traded away.
5. **Anything fixed by clicking gets a component test with a mutation proof.**
6. **The identity is asserted, not assumed**: a test that the served
   `Explanation` foots — `sum(product) + terminal == total` — on every fixture.

## 7. What M3 deliberately does not do

- **`ts-rs` stays open.** M3 adds hand-written types under the M2.5 contract, so
  the decision is no more urgent after M3 than before it — and taking it inside
  a feature milestone would mean deciding it under delivery pressure.
- **No visual-regression baseline.** Still a design phase; a screenshot baseline
  now is a test that fails on every intended change.
- **No phone layout.** Measured at 545px of overflow and deliberately not
  asserted; M3 does not widen the claim.
- **No roster row.** The fifteen-gate roster stays untouched and nothing added
  is `#[ignore]`d. Whether the browser walk earns a row is still the owner's.
- **`reveal` stays untested**, because this API still creates no live games.

## 8. Order of work

| Wave | Contents | Gate |
|---|---|---|
| **A** | The `yokohama-cli` seam (§5) | CLI suite unmoved at 849+2; goldens and `guide_sync` unchanged |
| **B** | Request-body hardening (§3.4) | a 400 test per body, red first |
| **C** | `explain.rs` DTOs + `/suggest` + `/why` + fixtures + the refusal exhaustiveness pattern | `yokohama-web` green; corpus regenerated and **read** |
| **D** | `/index/{kind}` + `/result` + fixtures | same |
| **E** | The client: the `why` chart (R1), the suggest panel, the indexes, the outcome | `npm run typecheck` / `test` / `e2e` green; mutation proofs recorded |
| **F** | Adversarial review, then registration (CLAUDE.md, README, guidingpath) and the close record | the full per-edit set, one pass |
