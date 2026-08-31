# Web frontend — M2: the card art, and the CLI parity gap

**Written 2026-08-31, on branch `web-frontend`, at the M2 head.** This is the
repo-side record of a lane that is deliberately OFF the Phase-3 map: an
additive HTTP API and browser client over the existing engine and CLI library,
started 2026-08-30 at the owner's direction as a way to widen the testing pool.
It is not a phase, it claims no gate, and it moves no roadmap. `guidingpath.md`
is untouched — see §6.

The working documents (the plan, the seven research files, the audited parity
gap) live in the untracked `docs/temp/webresearch/`, matching the repo's
convention for material that syncs off-repo. This file exists so that the
decisions taken are in history, because two of them are the kind a future
reader would otherwise re-derive or re-break.

---

## 1. What M1 had already built (2026-08-30)

A `crates/yokohama-web` axum server and a `web/` Vite+React client, playable
end to end against the shipped bot. Its own record is the six commits'
messages. The finding it rests on: `screen_model.rs`'s module doc calls itself
*"the seam a later API or GUI layer can consume without a terminal"*, and it
meant it — the port is serde derives, a handful of `pub(crate)` widenings, and
a client in place of the eight `*_render.rs` modules.

## 2. What M2 added

| | |
|---|---|
| Asset pipeline | `docs/tools/mkcardart.py` crops the TTS sprite sheets into 131 per-card WebP under `crates/yokohama-web/assets/art/`, with a manifest keyed on engine identifiers. Committed, 2.7 MiB. |
| Art routes | `GET /api/art` (the manifest) and `/art/**` (the files). Absent art is a supported state: a 404 and the client draws its typed fallback. |
| The input fence | `crates/yokohama-web/src/grammar.rs` — an allowlist on the one mutation endpoint. See §3. |
| The seed | `WireSeed` — a `u64` crosses the wire as a string. See §4. |
| The end-trigger strip | `ClientState.end_triggers`, worded by `render::end_trigger_line`, plus `deck_remaining` and `end_game_triggered`, which no frame model carries. |
| Four read-only surfaces | `GET …/ports`, `…/labs`, `…/areas`, `…/log` over `screen_model` builders that were already `pub` and reachable from nowhere. |
| The client | Card art throughout, and every Tier-A row of the audited parity gap. |

## 3. The security finding, and the ruling it forced

`POST /api/games/{id}/input` handed a client-supplied `line` straight to
`Repl::step`. That remains the right design — `step` IS the state machine — but
`step` is the TERMINAL's front door and answers to more than the game:

- **`save <path>` is an arbitrary filesystem write.** `Session::save`
  (`session.rs:407`) creates `<path>.tmp`, writes, fsyncs and renames onto
  `<path>`, anywhere the process can write. Being behind an authenticating
  proxy does not make a client entitled to name a path on the server's disk.
- **The overlay words silently change what a NUMBER means.** `?`, `index`,
  `orders`, `techs`, `goals`, `sitecards` and `view` open a sticky overlay in
  which a digit selects a card. `ClientState` has no field reporting one is
  open, because `Repl::overlay` is `pub(crate)`. Measured on a fresh 2P game:
  `step("orders")` then `step("1")` applies nothing and says nothing;
  `step("cancel")` then `step("1")` applies the move.

**Ruled: an allowlist on `/input`, plus read-only GETs for the surfaces.** The
alternative — keeping `step` as the single door and exposing an `overlay` field
on `ClientState` — needs `repl::Overlay` and `repl::ViewSurface` widened AND
given `Serialize`, which they do not derive, and would put a terminal state
machine on a JSON wire. The allowlist is `store::is_safe_id`'s shape and
reasoning: a denylist over a grammar is a list of the tricks someone has thought
of, and a word added to the REPL tomorrow should be refused by default.

The fence is two rules, not one, and the split is load-bearing: a HEAD may only
be a listed word or a number/area, because `save` and `view` are ordinary
alphanumeric words that a single permissive rule lets through — the first draft
did exactly that and its own tests caught it.

## 4. The seed was lossy, and it was not a display bug

`TitleModel.seed` is a `u64` serialized as a JSON number. `draw_seed()` draws
over the full range (which `game/agent.rs:21-29` requires: the public board is
an injective fingerprint of the setup seed), so a real seed is ~1.8e19 and
`JSON.parse` rounds it. The browser was displaying `5856483359458926000`, whose
last four digits are not in the seed.

The setup flow ROUND-TRIPS the value — preview draws a seed and deals from it,
the client sends it back to create the game — so a rounded seed started a
DIFFERENT GAME than the one the player chose cards from. `WireSeed` accepts a
string or a number and always emits a string. `frame.title.seed` stays a number
because `screen_model` is the CLI's model and this port does not move it;
`ClientState.seed` is the field to read, and its doc says so.

## 5. Two mapping errors the tests could not have caught

Both were found by looking at the pictures, and both are recorded because the
class of error will recur on the next regeneration.

1. **The crop origin.** Rounded origins looked right because three sheets do not
   divide evenly; the extra pixel is a black padding stripe on the far edge, not
   a fractional cell. A seam scan puts the gradient at the FLOOR index every
   time. Rounded crops were one pixel late on the last row or column and pulled
   the stripe in.
2. **Yellow is COMMERCIAL.** The four category-C achievement cards name
   production and commercial only by two coloured badges, and the intuitive
   guess is backwards, so all four shipped mapped to the wrong types. The badges
   were re-resolved against the AREA TILE art, where the same icons partition
   the engine's `is_production()`/`is_commercial()` exactly. The mixed cards
   print commercial LEFT, production RIGHT — the reverse of the engine's own
   `(production, commercial)` tuple order, which is what makes it easy to get
   wrong.

**`art.rs`'s tests pass either way.** They prove the join is TOTAL, not TRUE: a
wrong-but-complete mapping is the exact shape they cannot see. That is a known
and accepted limit of the instrument, stated here rather than discovered again.

## 6. What is deliberately NOT done

- **`guidingpath.md` is untouched.** This lane is a side event during the
  owner's V4 gate; a roadmap edit is the owner's call, and so is whether it gets
  a decision-log row.
- **The `ts-rs` question is still open.** `web/src/types.ts` is hand-written and
  says so in its own header. The plan's ruling rested on a premise that is
  wrong: `#[derive(TS)]` needs every field type to impl `TS`, and `screen_model`
  is full of engine types, so it reaches into `yokohama-core`.
- **M3's surfaces** — `suggest`, `why`, the four card indexes, the final score
  breakdown — need the five engine types that have no `Serialize`
  (`Explanation`, `RootScore`, `SearchStats`, `GameResult`, `ScoreBreakdown`).
  A `yokohama-web` DTO avoids touching core; that is the intended route.
- **Live tracking and BGA import** are out of v1 by owner decision.
- **`Repl::index_placements`** (`repl.rs:1706`, `pub(crate)`) is the one
  widening a card-index surface would genuinely need. Widen it rather than
  copying it — its doc defends a leak fence.

## 7. Hosting — recorded now, acted on later (owner, 2026-08-31)

**Not a task.** The owner's words: *"plenty of work before worrying about the
hosting to get it at parity and polished locally, but just so it's
documented."* Nothing in this milestone deploys anything, and the constraints
below are the only part that binds today.

**The custom domain is `yokohama.moore.nyc`.** Published through Cloudflare
Workers on that hostname, the deployment **picks up the existing Access rule by
itself** — there is no separate authentication step to build, and none should be
built. That is the whole reason the identity story in this crate is "read a
configurable header, never a body or a query parameter": Access injects it, and
the server's job is to trust the header the proxy sets and nothing else.

Two things that follow, and that the code already honours:

- **Nothing may exist only in memory.** Both hosted shapes under consideration
  (a Cloudflare Container behind the Worker, or a Modal `@modal.web_server`)
  sleep when idle and lose their disk. Every game is already persisted as its
  `GameRecord` after every applied move and rebuilt via `Session::from_record`,
  so a sleeping container loses nothing.
- **Storage is behind a trait.** `store::SessionStore` is byte-shaped rather
  than path-shaped, so a Modal Volume is a mount path and an object store is a
  second implementation rather than a refactor.

Note for whoever does this: `yokohama.moore.nyc` currently fronts the ttyd
terminal deployment, which lives deliberately outside this repo. Whether the
browser client takes that hostname, a path under it, or a sibling is a decision
for that day — this file records the domain and the Access behaviour, not the
routing.

## 8. What did not move

`crates/yokohama-core` is byte-identical to master and `SEMANTICS_REV` is 7.
The CLI's nine `assert_capture` goldens and 18 `guide_sync` tests are unchanged.
No `*_render.rs`, `screen.rs`, `style.rs` SGR table, `HELP_TEXT`, `describe`
constant arm or `cli-guide.{md,html}` byte moved — the port stayed additive, and
those tests are how that is known rather than believed.
