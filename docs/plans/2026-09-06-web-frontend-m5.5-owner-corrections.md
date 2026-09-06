# M5.5 — the owner's first-run corrections (2026-09-06): the set table shows only your own deal, and the technologies and goals wait until you have chosen

**Status:** a correction wave on branch `web-frontend` after the M5.5 close (`docs/plans/2026-09-05-web-frontend-m5.5-close.md`, frozen — corrections are recorded FORWARD, here). Owner direction 2026-09-06, verbatim in §0; the rulings in §1; the design in §2; the tasks in §3 (the SDD shape); the gate and the deploy in §4; the record at the close in §5. This doc is spec, plan and record in one because the wave is small and owner-ruled.

## 0. The owner's direction (2026-09-06, on the first run of `0.5.0-m5.5.2`)

> On a first run of the game, I see two issues. The first is that when I'm taking a card in a two player game, I can see the cards offered to the bot. This shouldn't be the case, the cards chosen by another human player, or the bot player, should be hidden. The game record should know but it shouldn't be visible to the player. The second is a break from the plan — the player per page 4 step 12 — steps 9 through 12 show the setup rules, and the player can see the board layout, building cards, and the port/dock but not the technology cards or achievement cards during that setup phase, those are revealed to the player after they choose their card. We ruled that we would not adjust the seed to reflect that dealing since the game engine knows the data this is just about when the human player is made aware of that, and right now the bot vision doesn't extend to this data point — so we would be aligning the human vision to fit the rules — so when they do their setup they see the board and building cards and port/dock but have to hold on the technology & achievement until after that step. Also a minor note on the setting up screen when on a mobile device (not a priority but worth understanding why it's different, the rendering of the cards doesn't resize/responsively react the same way it does on a normal screen).

Also authorized the same day: erasing the saves the checks had created (done: `sim-5c3b4e596bb8cf97`, `sim-7d949b884ebb2adb`, `sim-fa6b6bb5049a3929` deleted through the API; the owner's five stay), and changes to the deployed game over Portainer.

## 1. Rulings (the orchestrator's readings of §0; each with its cost if wrong)

- **R-C1 — another seat's deal never reaches the wire.** `Deal.cards` is EMPTY for a bot seat and `bot_keep` leaves the wire entirely; the server still keeps for every bot seat by the same argmax at `create` (M5.5 spec §3.3.2's mechanism stands; its EXPOSURE clause, R5's "the bot's card is exposed", is superseded by §0). The record knows: the bot's kept card is in the game's state, as before. Cost if wrong: one fixture and one fence test.
- **R-C2 — human seats choose ONE AT A TIME on the set table.** The deck shows the current chooser's two cards face up and every other human seat's as two card backs ("chosen" or "chooses after you"); after a pick, "Next seat" advances (Start on the last). At 2P against the bot the flow is unchanged in feel: your two cards, Start. The payload carries every human seat's cards because one browser serves them all; the secrecy is the sequence. M5.5 spec §5's "hot-seat secrecy is not improved" is superseded by this improvement, and hot-seat remains hot-seat: the earlier chooser's cards were on the screen before it was passed. Cost if wrong: one component's flow and the harness's set-table step.
- **R-C3 — the technologies and the achievements are withheld from the set table.** The preview's `table` carries the laboratory with EMPTY slots (`deck_remaining` kept — public arithmetic) and NO `Achievements` panel; the set table draws those two sections as "revealed once you have chosen your card" (the shape of "available once the game starts"); the game's first screen shows them as today. **The rulebook note — RESOLVED by the owner's edition, which is IN THE REPOSITORY:** `tts-assets/pdf/httpssteamusercontentaakamaihdnetugc75371631842576614037EE3A37AB39CBA5E73B74B7F97C54556D7C7719.PDF` (the Tabletop Simulator upload, 20 pages; its setup steps are on p.5, and it is the edition that carries the D1 NOTE the 2.9 ruling list quotes — `pdftotext` finds "The dummy player's assistants are taken into account during final scoring" in it and not in `analysis-1/00-yokohama-rulebook.pdf`, 16 pages, the file CLAUDE.md and the KB cite as "the rulebook PDF"). In that edition the order deck is built, the port filled, and each player dealt two orders to choose from IN SECRET at **step 9**; the technology cards go face up on the laboratory at **step 10**; the achievements at **step 11**; the 2P blocking pawns at **step 12**. So the choice is made BEFORE the technologies and the achievements exist on the table, and §0 is that edition's order applied to the human's view. `analysis-1/00-yokohama-rulebook.pdf` (p.4) orders them 10 achievements / 11 technologies / 12 orders-and-deal — that file is the OUTLIER on both counts, the setup order and the D1 NOTE; two rulebook editions sit in the repository and the owner plays by the TTS one. The engine's draw order and the seed are untouched (M5.5 spec §1: no `SEMANTICS_REV` change while V4 is played; the deal-vs-port order note in guidingpath stands — the engine deals before it fills the port, which is equivalent in distribution). The bot's keep (`score_starting_order`) reads nothing but the card, so the human's information at that decision matches the bot's. Cost if wrong: nothing against the owner's edition; against the bundled PDF, two sections shown one screen later.
- **R-C4 — what STAYS visible on the set table:** the pyramid with its site cards (the "building cards") and the face-up 5-Power tokens (rulebook step 5), the port (§0 names it), and the tracks (printed board, not cards). The 2P blocking pawns are the owner's edition's step 12, after the choice, but their places are fixed by rule (faith 7, customs 4) and reveal nothing, so the tracks draw with them as the game does. Building sites stay "available once the game starts" as before. Cost if wrong: one section's visibility.
- **R-C5 — every sentence that describes the set table says what is now true:** the deck's line ("…as the rulebook has it at step 12"), `Help.tsx`'s set-table paragraph, README's set-table sentence, `NewGame.tsx`'s module doc, CLAUDE.md's M5.5 bullet (the "bot's argmax marked" clause). Cost if wrong: a false sentence.
- **R-C6 — mobile is DIAGNOSED, not redesigned:** measure the set table and the game screen at 390×844 (headless Chromium, this Mac, DPR as stated), name the difference the owner saw and its cause; fix it in this wave only if it is one CSS line with a cell; otherwise record it for M6. Cost if wrong: a diagnosis the owner re-reads.
- **R-C7 — process:** Opus implementers and per-task Opus reviewers, Fable orchestrates and runs ONE clean final review against the live site after the redeploy (the M5.5 shape); the redeploy is `yokohama-web:0.5.0-m5.5.3` by runbook §3.5's recipe, recorded as a §3.6 addendum; the verification on screen covers BOTH layouts (≥1201 and the stack), which is the lesson of 2026-09-05. Cost if wrong: review seats.

## 2. Design

**Wire.** `Deal { seat, is_bot, cards: Vec<DealCard> }` — `bot_keep` removed from the struct (so nothing can serialize it); a bot seat's `cards` is `[]`. `Preview.table` stays a `HistoricalFrame`, built by `HistoricalFrame::at(&repl, 0)` and then WITHHELD: every lab board's `slots` emptied, the `Achievements` panel removed from `frame.dashboard`. `NewGame.keeps` unchanged (`[n, null]`). The corpus (`preview.json`, `preview_bot.json`) regenerates; `contract.test.ts` keeps the ANTI-VACUITY the corpus already has (`at_k.json`'s labs are NOT empty and DO carry a dashboard `Achievements` panel, so an emptied preview is a property of the preview and not of the type).

**Fences (server, `games.rs` tests):** (a) at a seed with a bot seat, the preview's serialized JSON carries no `bot_keep` key and the bot seat's `cards` is empty, while the human seat's two cards and keep scores are present; (b) the preview's JSON contains none of the six technology TITLES the created game's laboratory shows and none of the three achievement type names its dashboard carries (create the game from the same seed to learn them), while the port's six order ids ARE present (anti-vacuity: the withholding is targeted, not a blank table).

**Client.** `NewGame.tsx`: the bot section shows two `order_back.webp` faces (`backArt(m, "order")`) with the line "keeps one of its two cards — unseen"; no ids, no scores, no mark. Human seats: state `turn` = index into the human seats of the current chooser; the current seat's section carries class `current` and the two `deal-pick` buttons; a seat already chosen carries `chosen` with two backs and the word "chosen"; a seat yet to choose carries `waiting` with two backs and "chooses after you"; the button under the current seat's cards is "Next seat" while more humans follow, and Start appears only when every human has chosen. `TableSections.tsx`: a `withheld(key)` renderer ("revealed once you have chosen your card") used for `labs` and `achievements` when `game === null`. The deck's line: "The table is set: the areas with their site cards and tokens, the port and the tracks. The technologies and the goals are revealed once you have chosen." Help, README, the module doc, CLAUDE.md follow (R-C5).

**Harness.** `startGame` in `web/e2e/harness.ts`: for each `.deal-seat.human` in order — wait for it to carry `current`, click its first `button.deal-pick`, then click "Next seat" if it is visible; finally Start. `gates.spec.ts` and `the_table.spec.ts`'s bot-deal assertions (`.bot-keeps`, the bot's `.card-note`s in viewport) become assertions on the two backs and the line, in viewport at 1710×869 (R-T12-3's height budget stands, and gets easier: no scores under the bot's cards).

## 3. Tasks

### Task 1: Another seat's deal never reaches the wire or the screen (R-C1, R-C5 for the deal sentences)

**Files:** `crates/yokohama-web/src/games.rs` (the `Deal` struct, `preview`, the tests), `crates/yokohama-web/tests/wire_fixtures.rs` (regenerate), `web/src/wire/preview.json` + `preview_bot.json` (regenerated), `web/src/types.ts` (`Deal`), `web/src/wire/contract.test.ts`, `web/src/NewGame.tsx` (the bot section + the module doc), `web/src/NewGame.test.tsx`, `web/src/styles.css` (the backs' size in the side column, if the base `.card-face.sm` rule does not already fit), `web/e2e/gates.spec.ts`, `web/e2e/the_table.spec.ts`, `web/src/Help.tsx`, `README.md`.

- [ ] **Step 1 (RED, server):** in `games.rs`'s tests, `a_bot_seats_deal_never_reaches_the_wire`: `preview` at the existing `ARGMAX_IS_CARD_2` seed (bot at seat 1): `p.deals[1].is_bot && p.deals[1].cards.is_empty()`; `serde_json::to_string(&p)` contains no `"bot_keep"`; `p.deals[0].cards.len() == 2` with both `keep_score`s present. Run it, keep the failing output.
- [ ] **Step 2 (GREEN, server):** remove `bot_keep` from `Deal`; in `preview`, emit `cards: vec![]` for a bot seat (the argmax is still computed only where `create` keeps — `config_of`/`choices_of` untouched). Rewrite the tests that read `p.deals[1].cards` / `bot_keep` (`:891-923`): the argmax cells must keep DISCRIMINATING through `create` (the bot's kept card in the created game's hand at the two seeds — the M5.5 R-T11-2 shape), not through the preview. Update `games.rs`'s doc comments that describe `bot_keep` on the wire. `cargo test -p yokohama-web` green except `wire_fixtures`.
- [ ] **Step 3 (corpus):** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then without the variable; the report names the two files that changed and how (the bot's `cards: []`, no `bot_keep` in either).
- [ ] **Step 4 (RED, client contract):** `web/src/types.ts` drops `bot_keep` from `Deal`; `contract.test.ts`: replace the two `bot_keep` cells (`:194-203`) with: `previewBot`'s bot deal has `cards.length === 0` and `!("bot_keep" in deal)`; `preview`'s two human deals carry two cards each; `previewBot`'s human deal carries two cards. `npm run typecheck` now fails on `NewGame.tsx`'s `bot_keep` reads — that is the RED for Step 5.
- [ ] **Step 5 (GREEN, client):** `NewGame.tsx`: the bot section renders two `<img>` backs (`backArt(art, "order")`, `alt=""`, inside `.card-face.sm.back` or the existing card box) and the line `keeps one of its two cards — unseen`; no `OrderCardView`, no `.bot-keeps`, no note. The module doc's paragraph "A bot seat shows its two cards with the one it keeps marked" is rewritten to what is true. `NewGame.test.tsx`: the bot-mark cells (`:230-260`) become: the bot's section contains NO `.card-face` with an id, no `.card-note`, no `.bot-keeps`, two `img.back`s and the "unseen" line; and an anti-vacuity: the HUMAN section still shows two ids from the fixture. `styles.css` only if the backs need a rule.
- [ ] **Step 6 (e2e):** `gates.spec.ts:339-375` and `the_table.spec.ts:417-449`: the `.bot-keeps` and `.card-note` assertions become "the bot's two backs and its line are in viewport at 1710×869" and "no `.deal-seat.bot .card-face[data-id]` / no Keep score text under the bot"; `the_table.spec.ts:410`'s title says the bot's cards are HIDDEN. `npm run e2e`.
- [ ] **Step 7 (sentences, R-C5):** `Help.tsx:173-180` (the bot's cards "in the side column with the one it keeps marked" → two backs, unseen; the record knows); `README.md:205-207` if it names the bot's cards or the mark (it names the Keep score under YOUR cards — keep that). Gate: `cd web && npm run typecheck && npm test && npm run e2e`; `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] **Commit:** `fix(web): another seat's deal never reaches the wire or the screen — the bot's two cards are backs, its keep stays in the record`

### Task 2: The technologies and the goals are withheld until the choice (R-C3, R-C4, R-C5 for the setup sentences)

**Files:** `crates/yokohama-web/src/games.rs` (`preview` withholds; the fence test), the corpus (regenerated), `web/src/wire/contract.test.ts`, `web/src/TableSections.tsx` (`withheld`), `web/src/NewGame.tsx` (the deck's line; the module doc), `web/src/NewGame.test.tsx`, `web/src/TableSections.test.tsx` (or where the set-table sections are tested), `web/e2e/the_table.spec.ts`, `web/src/Help.tsx`, `README.md`.

- [ ] **Step 1 (RED, server):** `the_preview_withholds_the_laboratory_and_the_achievements`: at the `ARGMAX_IS_CARD_2` seed, `p.table.labs.boards.iter().all(|b| b.slots.is_empty())`, `p.table.labs.deck_remaining` equals the created game's (public), `p.table.frame.dashboard` has no `Achievements` panel; the preview's JSON contains none of the six technology titles the created game's laboratory panel names and none of its three achievement type names, while it DOES contain the six port order ids (anti-vacuity). Keep the failing output.
- [ ] **Step 2 (GREEN, server):** after `HistoricalFrame::at(&repl, 0)` in `preview`, empty every lab board's `slots` and remove the `Achievements` panel from `frame.dashboard`, in a named function `withhold_until_the_choice(&mut HistoricalFrame)` whose doc states R-C3 and the rulebook note in two sentences. Regenerate the corpus (Step 3's shape); the report names the diff (both previews: empty lab slots, no Achievements panel).
- [ ] **Step 3 (client contract):** `contract.test.ts`: both previews' `table.labs.boards[*].slots` are empty and `table.frame.dashboard` carries no `Achievements` key, AND `at_k.json`'s labs carry slots and its dashboard carries `Achievements` (anti-vacuity).
- [ ] **Step 4 (RED, client):** a `NewGame.test.tsx` cell: on the set table the `labs` and `achievements` sections (switch them on through the fixture's sections) carry the line "revealed once you have chosen your card" and NO `.card-face`; the `ports` section carries six `.card-face`s; the `tracks` section renders. Run it RED.
- [ ] **Step 5 (GREEN, client):** `TableSections.tsx`: `withheld(key)` (the `unavailable` shape with the new line), returned for `labs` and `achievements` when `game === null`; the game screen's branches untouched. The deck's line in `NewGame.tsx` becomes: "The table is set: the areas with their site cards and tokens, the port and the tracks. The technologies and the goals are revealed once you have chosen." The module doc's "the same pyramid, port, laboratory and goals … at its step 12" rewritten.
- [ ] **Step 6 (e2e):** in `the_table.spec.ts`'s set-table cell: with labs and achievements switched on, the set table shows the withheld line for both and zero `.table-sec.labs .card-face`; after Start the game shows six lab faces (the existing cell's port assertion stays). `npm run e2e`.
- [ ] **Step 7 (sentences):** `Help.tsx:173-175` ("the port, the laboratory and the goals — exactly what the rulebook has on the table when the two cards are drawn" → the port and the tracks; the technologies and the goals come out once you have chosen), `README.md:205-207` the same. The gate as Task 1's.
- [ ] **Commit:** `fix(web): the set table withholds the technologies and the goals until the card is chosen`

### Task 3: Human seats choose one at a time (R-C2)

**Files:** `web/src/NewGame.tsx`, `web/src/NewGame.test.tsx`, `web/src/styles.css`, `web/e2e/harness.ts`, `web/e2e/the_table.spec.ts` (the 3P shape if a cell exists; otherwise a unit cell carries it), `web/src/Help.tsx`.

- [ ] **Step 1 (RED):** `NewGame.test.tsx`, a 3P preview with two human seats (seats 0 and 1, bot at 2 — build it from `preview.json`'s shape through the existing fixture idiom, never a hand-authored payload): after Deal, seat 0's section carries `current` with two `button.deal-pick`s, seat 1's carries `waiting` with two `img.back`s and no buttons, Start is absent, "Next seat" is absent until a pick; after picking, "Next seat" appears; after clicking it, seat 0 carries `chosen` with two backs and no buttons, seat 1 carries `current` with two buttons, and Start appears only after seat 1 picks; `createKeeps` sends both picks. Run it RED.
- [ ] **Step 2 (GREEN):** the `turn` state and the three section states; the "Next seat" button; Start gated on every human having chosen (it already is); the backs reuse Task 1's markup. CSS for `.deal-seat.human.chosen/.waiting` if needed (the backs' box).
- [ ] **Step 3 (harness):** `harness.ts:117` loop → for each `.deal-seat.human` in order: `await expect(seat).toHaveClass(/current/)`, click its first pick, click "Next seat" if visible; then Start. The bot-free fixture game (two humans) exercises it in every e2e run.
- [ ] **Step 4 (sentences):** Help's set-table paragraph gains one sentence on hot-seat: each human seat chooses in turn; the others' cards are face down until then. Gate as Task 1's.
- [ ] **Commit:** `feat(web): human seats choose their starting order one at a time, the others' cards face down`

### Task 4: The mobile set table, diagnosed (R-C6)

**Files:** the report; `web/src/styles.css` + a cell ONLY under R-C6's one-line clause.

- [ ] **Step 1:** with the release server (`YOKOHAMA_WEB_PORT=<spare> YOKOHAMA_WEB_DIR=<scratch> target/release/yokohama-web` after `npm run build`, as the final review did), a throwaway Playwright script at 390×844 (DPR 2): screenshot the set table after Deal at scroll 0 and scrolled to the deck, and the game screen after Start at scroll 0; read `getBoundingClientRect()` of a deal card, a tile, the deck, and the page's `scrollWidth`; write the pictures to gitignored `docs/temp/web-review/mobile/`.
- [ ] **Step 2:** name the difference (the tiles use the M5 fit rule only ≥1201 and scroll at 557 px inside their column below it — M5 R8; the deal cards are 8rem fixed; the deck is sticky; whatever else the pictures show) and its cause, in the report, with the readings.
- [ ] **Step 3:** if the cause is one CSS line whose fix has no effect ≥1201 and does not touch the stack's mechanism (M5 R8), fix it with a cell; otherwise no code moves. Say which in the report.
- [ ] **Commit (only if code moved):** `fix(web): <the one line>`

### Task 5: The record — this doc's §5, the decision-log row, CLAUDE.md, README (after the redeploy and the final review)

**Files:** this document (§5), `guidingpath.md` (one row appended), `CLAUDE.md` (the M5.5 bullet's set-table clause; counts), `README.md` (re-read).

- [ ] **Step 1:** §5 of this doc: what shipped per task with the cells, the counts at the close beside the M5.5 close's, the deploy figures (`0.5.0-m5.5.3`), the final review's findings and rulings, the mobile diagnosis, the rulings taken in execution with costs.
- [ ] **Step 2:** one guidingpath row: the owner's 2026-09-06 corrections — the hidden deals, the withheld technologies and goals with the EDITION note (R-C3: the owner's edition deals at step 9 before the technologies and achievements; the bundled PDF is the outlier, as for D1), hot-seat in turn — superseding M5.5 spec R5's exposure clause and §5's hot-seat non-claim; Why column: the first run by the owner found what three reviews on screen did not, because none of them asked what the PLAYER should not see.
- [ ] **Step 3:** CLAUDE.md's M5.5 bullet: "with both dealt cards, the CLI's own Keep score and hedge under each, and the bot's argmax marked" → your own two cards with the Keep scores; the bot's two are backs and its keep stays in the record; the technologies and goals withheld until the choice (2026-09-06 corrections, this doc); the counts re-measured. CLAUDE.md's Rules-knowledge section gains one sentence beside the D1 bullet: the owner's edition places the order deal and the secret choice at setup step 9, before the technologies (10) and the achievements (11); the bundled English PDF orders them 10/11/12 — a second edition divergence, and the web set table follows the owner's edition. README re-read.
- [ ] **Commit:** `docs(web): the 2026-09-06 corrections recorded — hidden deals, withheld technologies and goals, hot-seat in turn`

### Task 6: The M6 handover (the entry artifact for the next session)

**Files:** create `docs/plans/2026-09-06-web-frontend-m6-handover.md`.

- [ ] **Step 1:** the handover, in the shape of `docs/plans/2026-09-04-web-frontend-m5.5-handover.md` (read it first): (1) where the lane stands at HEAD — the live tag, the branch and worktree, the counts, the deploy recipe by reference (runbook §3.5/§3.6), the two on-screen layouts to verify after every deploy; (2) what M5.5 and this correction wave shipped, by reference to the close and this doc; (3) M6's charter: spec §6.3 of the M5.5 spec VERBATIM (the walk as the spectator primitive, M4's hosting list by reference, the three questions with their leans) plus what §0 of this doc adds (a hidden deal is a per-seat view, which M6's multi-browser room makes natural — hot-seat's sequence is the interim); (4) the LEAVE list M6 inherits (from the close §7 and this doc §5); (5) the process the owner wants (R-C7 here; the tiered-model rule; the SDD workspace convention; the append-only log); (6) what is NOT claimed. Nothing in it that the record does not support.
- [ ] **Commit:** `docs(web): the M6 handover — the entry for the next session`

## 4. Gate and deploy

Per task: `cd web && npm run typecheck && npm test && npm run e2e`; when `crates/` moves: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`; the wire corpus rule (regenerate, run without the variable, report the diff). Blast radius: `git diff e4f41e4..HEAD -- crates/yokohama-core crates/yokohama-cli crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge` EMPTY; `SEMANTICS_REV` 7. Redeploy after Task 4 (orchestrator): runbook §3.5's recipe as `yokohama-web:0.5.0-m5.5.3` (the musl binary rebuilds — `crates/yokohama-web` moves), a §3.6 addendum; then the clean Fable review against the live site (both layouts), a fix wave if it finds anything, Tasks 5–6.

## 5. The record (filled at the close)

Written at `97b2cb5` (the runbook addendum), the wave's last commit before this one. The wave ran
under the SDD shape in `.superpowers/sdd/2026-09-06-web-frontend-m5.5-owner-corrections/`, whose
`progress.md` is the ledger of record; the four task reports, the seven review documents and the
two extracts sit beside it (Task 1's own re-review verdict is a ledger row and not a file). **LIVE is `yokohama-web:0.5.0-m5.5.4` from `fb6cec7`.**

### 5.1 What shipped, task by task

**Task 1 — another seat's deal never reaches the wire or the screen** (`40b1ed2`, fix round
`a19f8fd`; R-C1, and R-C5 for the deal sentences).

`Deal` loses `bot_keep` **from the struct**, so the field cannot serialize because it does not
exist; a bot seat's `cards` is `[]`. `create` still keeps for every bot seat by the same
`score_starting_order` argmax, and the kept card is in the created game's hand as before. The
cells:

- `a_bot_seats_deal_never_reaches_the_wire` (`games.rs`) — the bot deal is `is_bot` with empty
  `cards`, the serialized preview carries no `bot_keep`, and — the anti-vacuity — the human seat's
  two cards and both `keep_score`s are present, re-derived through `greedy::score_starting_order`
  rather than hard-coded.
- `the_preview_and_the_game_agree_on_the_bots_card` was **renamed**
  `the_created_game_holds_the_bots_own_keep_at_both_seeds`: the old name is false once the preview
  stops answering. It discriminates through `create` at BOTH seeds (`CORPUS_SEED`, argmax card 1;
  `ARGMAX_IS_CARD_2`, argmax card 2), so a keep collapsed to either constant fails one half.
- `lib.rs`'s route cell asserts the absence through the router; `contract.test.ts` asserts it on
  the client half; two `NewGame.test.tsx` cells assert the bot's two backs with no id, no note and
  no mark (paired with the human seat's own two ids) and that the backs' `src` is
  `/art/backs/order_back.webp`; `gates.spec.ts` and `the_table.spec.ts` assert the same in a real
  browser at 1710×869.

The corpus moved in exactly two files: `preview.json` lost two `"bot_keep": null` keys;
`preview_bot.json` lost seat 0's `"bot_keep": null`, seat 1's `"bot_keep": 1`, and seat 1's two
dealt cards (O23 `keep_score` 57, O32 `keep_score` 31) for `"cards": []`.

The fix round closed three review findings, two Important and one folded Minor. `Help.tsx`
rendered **"cards areface down"** — a newline before `<strong>` that JSX drops, on prose no test
had ever rendered — fixed with `{" "}` and pinned by a NEW file `Help.test.tsx` carrying three
cells at that point: the sentence, a general rule walking every `<strong>`/`<em>` in the rendered
panel for a letter glued to a letter across an element boundary, and an anti-vacuity that the
panel rendered at all. (It carries **five** at the close: Tasks 2 and 3 each pinned their own new
sentence in it.) `contract.test.ts`'s root-key fence never inspected a bot deal; one row over
`previewBot.deals[1]` closes it, **proven by a temporary `kept_card_index` on that deal failing
that row and nothing else**. And the `keeps` doc was corrected to separate what the request sends
from what the response means.

Two brief deviations, both named in the report: the backs' manifest key is `order_back` (the
`backs` family is keyed by file stem, not by an engine identifier), and the e2e selector is
`.card-id` rather than `.card-face[data-id]`, because nothing in this client renders `data-id` and
the brief's selector would have asserted an emptiness that was already empty.

**Task 2 — the technologies and the goals are withheld until the choice** (`e35f9fe`, fix round
`5f73491`; R-C3, R-C4, R-C5 for the setup sentences).

`withhold_until_the_choice(&mut HistoricalFrame)` runs immediately after
`HistoricalFrame::at(&repl, 0)` in `preview`: it clears every laboratory board's `slots` and
`retain`s the dashboard without its `Achievements` panel. `deck_remaining` is kept deliberately
(public arithmetic). The doc states the other half out loud: this is a VIEW, not a rule — the
seed, the draw order and the position `new_game` built are untouched, `SEMANTICS_REV` does not
move, and a `create` from the same seed shows everything.

`the_preview_withholds_the_laboratory_and_the_achievements` reads the six technology titles and
the three achievement type names **off the game the same seed creates**, then requires them absent
from the preview's serialized JSON — a hard-coded list would stop discriminating the moment the
deal changed — with the created game's six port order ids required PRESENT as the anti-vacuity.
On the client, `TableSections.tsx` gained `withheld(key)`, returned for `labs` and `achievements`
when `game === null`; the game screen's branches are untouched. The e2e cell asserts the delay as
a delay: the withheld line and zero faces before Start, six technologies and three goals after it,
from the same server one screen later.

The rulebook citation was verified at the source before it was written down: `pdftotext -f 5 -l 5
-layout` over the owner's edition returns **9** (the order deck, the port, the deal and the secret
choice), **10** (the technologies), **11** (the achievements), **12** (the 2P blocking pawns).

The fix round took two Importants and six Minors. Both Importants were sentences: two "step 12"
strings left in `NewGame.test.tsx` (the outlier edition's number for the arrangement this task
removes), and a call-site comment claiming the laboratory rides on the frame and that ONE section
says a line, when three do. The Minors: the achievement assertion now matches the PANEL's absence
rather than an empty name list (**mutation-proven** — a withholding that kept the panel and
cleared its cards failed it); `boards.is_empty()` is asserted first so `all(is_empty)` cannot pass
vacuously; four near-identical section renderers became one `note(key, line, standing)` behind
four named one-line wrappers, `loading` the one that does not take `collapsed`; and
`.table-sec.collapsed { align-self: flex-start }` stops the withheld boxes stretching to their
row, **gated as a property** — each lined box shorter than the row's tall one — with the reading
printed (laboratory 85 px, goals 85 px, tracks 222 px at 1710×869) and the CSS line's removal
failing the cell.

**Task 3 — human seats choose one at a time** (`42c51d6`, fix round `f5cb50e`; R-C2).

`NewGame.tsx` gained `turn`, an index into the HUMAN seats rather than a `PlayerId` (so a bot's
seat number cannot land in it), clamped on read and reset with `chosen` at both handlers. Three
states per human section: `current` (two `deal-pick` buttons), `chosen` and `waiting` (both
Task 1's `DealBacks`, with "· chosen" and "· chooses after you" in the label). "Next seat" is
drawn while more humans follow; Start only when every human has chosen — neither drawn early and
disabled, because a disabled Start under a seat still choosing reads as the wrong thing to press.

**There is no peek path**, which the task reviewer verified rather than took on the report's word:
a waiting seat's cards never enter the DOM, the render sitting behind `state === "current"`, and
the previous seat's buttons unmount on the pass (asserted). The two cells run over the committed
fixtures per R-C8 — `preview.json` (both seats human) for the sequence, `preview_bot.json` for the
single-human shape where the flow is unchanged in feel.

Start used to be always-rendered-and-disabled, and eight sites used its presence as the signal
that the set table had arrived; all eight now wait for **Re-deal**, which is there from the first
frame. The `keeps` half of the brief ("`createKeeps` sends both picks") is unreachable over a
bot-free fixture the FORM cannot request, so the cell asserts `keeps[0] === 2` (the second card,
which no constant and no first-card default produces) and rests both-picks on Start's appearance,
because `ready` and `createKeeps` read the same map.

One CSS rule was needed and its reason is a measurement. Driving a real 4P set table at 1710×869,
the ASKING seat's own label sat **28 px above the fold** on the middle turn. With
`.deal-seat.human:not(.current) .card-face.sm { width: 4rem }` the deck's block drops 931 → 738 px
and that seat's top moves −28 → +25. The fix round then **gated** it: a 4P viewport cell that
fails once the rule is removed and `dist` is rebuilt — and the first probe was a FALSE PASS
because `npx playwright test` does not rebuild `dist`, the M5.5 lesson met again. The round also
moved focus to the next seat's first pick on the pass (firing only on a "Next seat" advance, via a
`passing` ref, not on first render or Re-deal), corrected the harness comment, and anchored two
matchers.

The harness now walks the sequence for every `.deal-seat.human` in order, with an anti-vacuity
guard requiring `players - 1` human seats — `locator(…).all()` returns an empty list when nothing
matches, and a `for` over it asserts and clicks nothing while the walk goes on to press a Start
that is not there. The pass is therefore exercised at 3P and 4P on every e2e run.

**Task 4 — the mobile set table** (no commit; §5.5).

**The fix wave** (`fb6cec7`) — the final review's three fix-now items, §5.4.

### 5.2 The counts at the close, beside the M5.5 close's

Measured at `97b2cb5` rather than copied, **with two stated exceptions**. `npm run typecheck`,
`npm test`, the corpus count, the help and art counts, `SEMANTICS_REV` and the blast radius were
run here. So was the server suite's shape: **the lib's 76 passed and `wire_fixtures`' 5 passed
were both re-run at this HEAD** (`the_committed_wire_fixtures_are_what_the_server_emits` among
them, so the corpus is byte-identical to what this server emits). The two exceptions:

1. **The browser walk (37)** is the figure the final reviewer measured at `f5cb50e` and the fix
   wave's implementer at `fb6cec7`. `git diff --stat f5cb50e..HEAD -- web/e2e/` is empty.
2. **`play_a_whole_game`'s ONE test** was confirmed to be one test in that binary here (`running
   1 test`) but its GREEN is the same two readings, not mine: it is OS-entropy seeded and plays a
   whole 2P game against the shipped bot in a debug build, and the run started for this record was
   stopped at 25 minutes rather than block the close. Its slowness is a known property — the M5.5
   close records the same gate capping twice at 20,000 inputs and then passing 660 consecutive
   times. `git diff --stat fb6cec7..HEAD -- crates/` is empty, so no Rust has moved since the
   green reading.

| reading | M5.5 close (`cc01dab`) | this wave (`97b2cb5`) |
|---|---|---|
| `npm test` | **381** in **23** files | **393** in **24** files |
| `npm run e2e` | **36** in 6 spec files (+2 opt-in) | **37** in 6 spec files (+2 opt-in) |
| `cargo test -p yokohama-web` | **80** (74 lib + 1 gate + 5 `wire_fixtures`) | **82** (76 lib + 1 `play_a_whole_game` + 5 `wire_fixtures`) |
| wire corpus | **44** files | **44** files, **two of them changed** |
| help pictures | **5**, 4,210,470 B | **5**, 4,210,470 B (unmoved) |
| card art | **132** files | **132** files (unmoved) |
| `SEMANTICS_REV` | **7** | **7** |

**The +2 on the server are the two fences this wave exists to hold**: the deal fence
(`a_bot_seats_deal_never_reaches_the_wire`, which after R-C10 asserts the bot deal's exact key
set) and the withholding fence (`the_preview_withholds_the_laboratory_and_the_achievements`).

**The +12 on the client** are `Help.test.tsx` — a NEW file, 5 cells, the first thing in this lane
that renders the help panel's prose — plus `NewGame.test.tsx` 18 → 23 and `contract.test.ts`
83 → 85 assertions. `TableSections.test.tsx` did not move: the withholding is asserted on the
screen through `NewGame.test.tsx`, which is where the set table is mounted.

**The +1 on the browser walk** is the 4P fold cell in `the_table.spec.ts`.

`cd web && npm run typecheck` is clean at `97b2cb5`, and **nothing under `web/` or `crates/` moved
for this record** — the Task 5 commit touches `docs/`, `guidingpath.md`, `CLAUDE.md` and
`README.md` only.

**One correction beyond this wave's own text, found while re-reading README under R-C5.** Its
"Gates" paragraph still carried the M2.5-era browser figures — **231** vitest tests and **23**
Playwright tests, with `shots.spec.ts` described as "a fifth spec FILE" and the only opt-in
generator. Every one of those was false at HEAD and had been false since M3.5; none of the four
milestones since re-read that paragraph. It now reads 393 in 24 files and 37 in six spec files
(36 in a browser, one structural), with BOTH generators named. **The set-table sentences in the
same file were already true** — Tasks 1 and 2 rewrote them — and were left alone.

**Blast radius** (plan §4), at `97b2cb5`: `git diff e4f41e4..HEAD` over
`crates/yokohama-core crates/yokohama-cli crates/yokohama-oracle crates/yokohama-difftest
crates/yokohama-bridge` is **EMPTY (0 bytes)**; `SEMANTICS_REV` is **7**
(`crates/yokohama-core/src/game/semantics.rs:180`); the whole `crates/` diff since `e4f41e4` is the
same six files of `yokohama-web` the M5.5 close named. **Still no gate and no roster place**: the
fifteen are untouched and nothing added is `#[ignore]`d.

### 5.3 The two deployments

The plan §4 anticipated ONE redeploy. There were two, because R-C12 turned out to be product text
and the live site is kept equal to HEAD. Both went out by runbook §3.5's recipe through the
Portainer MCP; the record is `deploy/RUNBOOK.md` §3.6's addendum (`97b2cb5`).

| | `0.5.0-m5.5.3` (from `f5cb50e`) | `0.5.0-m5.5.4` (from `fb6cec7`) |
|---|---|---|
| musl binary | 6,105,960 B static-pie, 5.53 s incremental | 6,105,976 B (**+16 B**, `games.rs`'s shifted panic-location lines; the test module compiles out), 3.89 s |
| client bundle | `index-B-jx3Iuo.js` 284.91 kB · `index-C4YknxHb.css` 30.63 kB | `index-LfoAAWF6.js` 285.11 kB · the same css |
| build context | 13,900,042 B (sha256 `1e271a57…`) | 13,900,069 B (sha256 `acdab372…`) |
| image | 25,996,682 B, built ~15 s, created 13:06:46Z | 25,996,894 B, built ~15 s, created 13:48:06Z |
| container | `54d0c24f` created 13:07:18Z, started 13:07:24Z | `d2dfda65` created 13:48:38Z, started 13:48:43Z |
| address | 172.21.0.4 — **the same**, no `cloudflared` restart | 172.21.0.4 — the same |
| healthy | at 30 s after start, and at the two later probes | at 30 s (09:49:13 local) |
| tunnel serving the new bundle | 20 s after start | 17 s after start |

**Eight tags now sit on the NAS**, each a rollback point (runbook §5): `0.5.0`, `0.5.0-m5`,
`0.5.0-m5.1`, `0.5.0-m5.5`, `0.5.0-m5.5.1`, `0.5.0-m5.5.2`, `0.5.0-m5.5.3`, `0.5.0-m5.5.4`.

Verified through the tunnel at each: `/api/health` 200 (`0.5.0` / rev 7 / `d85445c479cb1ead`), `/`
shipping that build's bundle hash, `/help/screen.png` 200, `GET /api/games` the owner's five. The
**live wire fence** was read from the shell at each, with the client's own request body — at `.3`
a 2P request, whose deals are `[(0, human, 2 cards), (1, bot, 0 cards)]` with no `bot_keep` key
anywhere, laboratory slots `[0]` with `deck_remaining` 24, no `Achievements` panel and the port's
six present; at `.4` a 3P request, whose three deals each carry **exactly** the keys
`cards`/`is_bot`/`seat`, with `ports [6, 6]` and the same withholding.

On screen at 1710 wide, at BOTH builds: the bot's two backs and its line with zero ids and zero
notes, the laboratory and the goals reading "revealed once you have chosen your card" with zero
faces, the port's six, Start absent before the pick. At `.3` a 3P hot-seat pass was additionally
walked end to end (Red `current` → pick → "Next seat" → Red `chosen` with its ids GONE from the
DOM, Yellow `current`, focus on a pick button, Start still absent until Yellow picks). At `.4` the
form's subtitle was read at 2P, 3P, 4P and back to 2P.

**One limit, recorded rather than glossed:** the orchestrator's own browser would not resize —
`resize_window` to 1000×740 reported success while `innerWidth` stayed 1710 (the M5.5 final
reviewer hit the same on this Mac) — so the below-1201 STACK was not read live from that session.
It WAS read live by the final review, whose Chrome reported 1000×563, and by Playwright against
the same live server at 390×844.

Every game the checks created was erased the same day under the owner's authorization in §0
(`sim-1a0630c54df1a5b4` from the `.3` screen check; `sim-f70ee3c0cb034533` and
`sim-698e0769a544bd88` from the final review). **The owner's five were never opened, played or
deleted.**

### 5.4 The final review, its findings, and what was done with them

One clean Fable subagent with no prior context, against the LIVE `0.5.0-m5.5.3` (R-C7). Verdict
**READY WITH FIXES: 0 Critical, 1 Important, 3 Minor.** Record: `final-review.md`.

The three corrections hold live in BOTH layouts (its own Chrome at 1000×563; Playwright at
1440×900 and 390×844) and on the wire from two different callers (the shell and the page's own
patched `fetch`): no bot card anywhere, the technologies and the goals withheld and then shown
after Start, and the earlier seat's cards **absent from the DOM** rather than hidden. Its gate at
`f5cb50e`: typecheck clean · vitest 391/24 · Playwright 37 (3.2 min) · `cargo test -p yokohama-web`
82 · clippy clean. Every sentence the wave wrote about itself was tried, including the rulebook
citation re-verified at the source.

**Seventeen mutations, sixteen caught.** The notable ones: the bot's cards restored (3 cells
failed); an empty `Achievements` panel left behind (caught); the withholding deleting the BOARDS
instead of their slots (caught); a waiting seat's cards rendered but `hidden` (caught); the
harness never pressing "Next seat" (the 3P width cell timed out); the 4rem rule removed (the
label's "viewport ratio 0"); `align-self` removed (the withheld box stretching, 222 vs 222). **M2
— the keep re-added under a different NAME — was caught by the corpus pin and the client's key-set
rows and NOT by the server's own fence**, which is Minor 1 and became R-C10.

**The one green mutation** is Important 1: `setTurn(0)` could be deleted from the Re-deal handler,
the Deal handler, or both, with all 391 cells green.

| ruling | finding | disposition | cost if wrong |
|---|---|---|---|
| **R-C9** | Important 1 — the sequence's restart is pinned by nothing | FIX NOW | one cell |
| **R-C10** | Minor 1 — the server's deal fence is literal-name-based | FOLD | one assertion |
| **R-C11** | Minor 2 — the asking seat's label half-clipped after the pass at 1000×563 | **LEAVE → M6** | a label one wheel-tick up at one window size |
| **R-C12** | Minor 3 — the setup form's subtitle is false above 2P | FOLD, and REDEPLOY | a sentence and a five-minute redeploy |

The fix wave (`fb6cec7`) and its scoped re-review (`final-rereview.md`) closed all three as
ADDRESSED, with no new Critical or Important breakage. Two things the re-review established that
the finding could not:

- **R-C9's letter asked for a discrimination the code makes impossible.** `preview` becomes
  non-null at exactly one site, Re-deal is the only non-unmounting exit from the set table and
  resets first, and Start unmounts the component — so no reachable path presses Deal with
  `turn > 0`, and no cell can be RED with one site alone. The cell that shipped is RED for the
  only defect the UI can reach (both removed): a second deal opening with seat 0 marked "chosen"
  though it chose nothing. Both calls were KEPT, because the `turn` doc states the pair as one
  invariant and a lone reset would leave the other handler a trap.
- **R-C10's fence now stands on its own.** `Deal` is a plain three-field `Serialize` with no
  `flatten` and no `skip_serializing_if`, so any escaped keep field lands in the key vector under
  whatever name it was given; the failure message interpolates the whole object, so it names the
  leak rather than only its arity. Proven against the review's own M2 mutation.

### 5.5 The mobile set table, diagnosed (R-C6) — and the 1200/1201 artifact

**No code moved**, which is R-C6's default and its explicit condition: the cause is not a defect
and its fix is not one CSS line. Method: `npm run build` at `f5cb50e`, the release binary on a
spare port with its own scratch store, driven by a throwaway Playwright script (headless Chromium,
`deviceScaleFactor: 2`); pictures in gitignored `docs/temp/web-review/mobile/`.

| reading | 390×844 | 1440×900 |
|---|---|---|
| deal card, the choosing seat | 128 × 223.4 | **128** × 223.4 |
| board tile | 224 × 168.3 (`--tile-w: 14rem`, a constant) | 251.2 × 188.6 (the M5 fit rule) |
| board scroller | client 358 / content 915 / **overflow 557** | 1024 / 1024 / overflow 0 |
| the deck | 390 × 504.2, `position: sticky` | 384 × 520.7, `position: static` |
| page overflow | **0** | **0** |

A width sweep from 320 to 2560 reads **0 page overflow at every width** and **128 px for the deal
card at every width**.

**The why, in one sentence:** the cards are not sized differently on a phone — `.card-face.sm` is
a flat `8rem` at every viewport (`styles.css:502`; its only override is Task 3's 4 rem for a seat
that is not choosing), and the strip's `--card-w` is `clamp(8rem, 12cqw, 12rem)` whose query
container (`container-type: size` on `.table`) is declared INSIDE `@media (min-width: 1201px)`, so
below the breakpoint `cqw` falls back to the viewport and 12 % of 390 px is far under the floor —
what differs is everything around them: above 1201 px the table scales to the room, below it
nothing scales at all (a `14rem` tile chosen by PLAYER COUNT, the board scrolling 557 px inside
its own column, and a sticky deck taking 504 of 844 px).

Why nothing was changed: nothing is broken (0 page overflow at 320 px, both cards legible with
their ids and Keep scores); the obvious one-liner (`min(8rem, 40vw)` under `max-width: 1200px`)
resolves to 128 px at every phone width because the `min` never binds; and making the cards LARGER
to use the room re-opens the deck's height budget, the pinned-deck rule and the hedge's fit — a
redesign R-C6 rules out for this wave. **Carried to M6 as a layout question.**

**The artifact found on the way, also for M6:** a port/laboratory strip card is **WIDER at 1200 px
(144) than at 1201 px (128)** — a 16 px shrink as the window grows. Same mechanism read from the
other side: below the breakpoint `12cqw` tracks the WINDOW (12 % of 1200 = 144), above it it
tracks `.table` (~817 px at 1201, so 98, back under the 8 rem floor). **No cell sits at that
pixel** — every width cell in the suite is at 390, 1024, 1201, 1440, 1710 or 2560, and the
discontinuity lives between 1200 and 1201.

### 5.6 Every ruling, with its cost

**Taken while planning:** R-C1 … R-C7 in §1 above, each with its cost. All seven held as ruled.
Two changed shape in flight: **R-C3**'s open rulebook note was RESOLVED before Task 2 read it (by
R-C3a and R-C3b below) and the plan was amended IN PLACE, this wave being open; and **R-C7**'s one
redeploy became two (§5.3). **R-C5's last clause is discharged by this commit** — the two stale
CLAUDE.md sentences it named (the corpus's non-null `bot_keep`, and "the bot's argmax marked") are
rewritten here, which is where the plan scheduled them.

**Taken in execution**, in the order they were made (the ledger's rows, and `rulings-extract.md`):

| ruling | what it settled | cost if wrong |
|---|---|---|
| **R-C3a** (owner) | R-C3's rulebook note is RESOLVED: the owner's edition deals and chooses at step 9, before the technologies (10) and the achievements (11); the bundled PDF (10/11/12) is the outlier, as it is for D1. The plan was amended IN PLACE (the wave was open) and committed | a record line |
| **R-C3b** (fact) | that edition is IN the repository — `tts-assets/pdf/httpssteamusercontent…C7719.PDF`, 20 pages, setup on p.5, and it carries the D1 NOTE that `analysis-1/00-yokohama-rulebook.pdf` (16 pages, 10/11/12) lacks. Verified independently by the implementer and again by the final reviewer with `pdftotext` | a record line |
| **R-C8** | Task 3's fixture: the brief's "3P preview with two human seats" is replaced by the committed bot-free 2P `preview.json`, whose two seats are both human — no constructed payload, and the seat list is what varies, so the 3P/4P shapes ride the browser walk | a cell at one player count |
| **R-C9** | the final review's Important — a `NewGame.test.tsx` cell pinning the restart at the first seat | one cell |
| **R-C10** | the server's deal fence asserts the bot deal's EXACT key set, not a literal name | one assertion |
| **R-C11** | the clipped label at 1000×563 is **LEFT to M6**, with the R-F11 lesson attached: the fix, if taken, scrolls the DECK's own scroller by a computed `scrollTop`, **never** `scrollIntoView` | a label one wheel-tick up at one window size |
| **R-C12** | the setup form's count-aware subtitle, and — because it is product text — a second redeploy so the live site equals HEAD | a sentence and a five-minute redeploy |

**Fourteen rulings in all**: seven in the plan, seven in execution.

### 5.7 What is deferred, and what M6 inherits

**Nine deferred minors**, every one a comment or an unpinned statement, none a behaviour:

1. `DealBacks`' no-manifest fallback (`.card-blank` "face down") has no cell (Task 1).
2. The `keeps` doc's "It is NOT inert on the bot's seat" (`web/src/NewGame.tsx:160` at this HEAD)
   is loose about which thing is not inert (Task 1).
3. The un-stretching of the replay-collapsed and buildings boxes is stated in the CSS comment and
   pinned by nothing (Task 2).
4. That comment's "and so do the tracks" attaches a loading fallback the tracks branch lacks
   (Task 2).
5. `styles.css`'s before/after pair for the 4 rem rule mixes two mechanisms — 931 / −28 was taken
   before the focus effect existed, 738 / +25 with the focus call scrolling; both true of their
   states, and the gated property is unaffected (Task 3).
6. The 4P viewport cell is filed above the part-B header (Task 3).
7. That cell's comment omits the focus scroll (Task 3).
8. Comment adjacency in `NewGame.test.tsx`: the set-table paragraph (`:194-201` at this HEAD) now
   sits above the subtitle's `describe` rather than the set table's (fix wave).
9. The fix report's probe table records 22 where the file now runs 23 — the probe predates the
   R-C12 cell in the same commit (fix wave).

**Carried to M6** beyond the M5.5 close's own §7 list: **R-C11** (the clipped label, with its
lesson), **the 1200/1201 strip-card artifact**, and **the mobile diagnosis** — the stack does not
scale, which is a layout question and not a defect.

**Seen and not raised** by the final review: site-card art re-fetches after Start over the tunnel
(a first-paint effect; `/art/*` is served `max-age=3600` with a cache HIT); the set table's tab
order runs through every tile, each opening its popover on focus, before it reaches the pick
buttons (M5's behaviour, unchanged here); the Boards menu's "on" count differs per browser (per-
browser storage, by design); and the opt-in `gates.spec.ts` was not run in that review — its
set-table assertions were rewritten in the range and the implementers ran it green (6) at each
task.

### 5.8 What this wave does NOT claim

- **No engine, CLI or rules change.** The blast radius over the five fenced crates is empty and
  `SEMANTICS_REV` is 7. R-C3 changes **when a human is shown** the technologies and the
  achievements; the engine's draw order, the seed and the dealt position are untouched, and the
  bot's keep reads nothing but the card, so the human's information at that decision now matches
  the bot's.
- **Hot-seat is still hot-seat.** R-C2 improves the SEQUENCE, not the payload: one browser serves
  every human seat, so the preview carries all of their cards, and an earlier chooser's cards were
  on that screen before it was passed. The M5.5 spec §5 non-claim ("hot-seat secrecy is not
  improved") is superseded by that improvement and by nothing more. A hidden deal is properly a
  per-seat VIEW; M6's room is where that becomes natural.
- **No mobile fix.** §5.5 is a diagnosis (R-C6), and the pictures are gitignored.
- **No claim about the below-1201 stack from the deploying session's own browser** (§5.3). The
  live evidence for that layout is the final review's Chrome at 1000×563 and Playwright at
  390×844 against the same server.
- **Still no gate and no roster place** for this lane; the fifteen are untouched.

**HEAD at the close: `97b2cb5`. LIVE: `yokohama-web:0.5.0-m5.5.4`, built from `fb6cec7`.**
