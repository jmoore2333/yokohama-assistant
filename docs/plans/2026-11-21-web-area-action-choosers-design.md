# Design — the area-action choosers (issue #77, widened) (2026-11-21)

**Status: ACCEPTED 2026-11-21**, after an independent builder check (§12). The scope and the mockups were accepted the same day; this document writes them down with what the code allows. It is the first commit of cycle 2's first branch, `web-choosers-goods` (§10).

**What this is.** The second of Track 6's three cycles. Cycle 1 (`docs/plans/2026-11-20-web-claims-and-flags-design.md`) made claims and flags readable; this one replaces the long flat lists of four area actions with choosers. It is one design and two plans, run in sequence on the lab (§10). Nothing reaches origin before cycle 3, the wrap.

## 1. Intent

People playing on the browser client report that some area actions answer with a wall of rows: 43 at the Church in the reported position, where the rows differ only in which goods are paid. The terminal shows the same rows. The engine lists one move per distinct payment or combination, which is right for the engine and wrong for a reader. A chooser lets the player say what they pay, and names the one listed move that says so.

## 2. Goals, non-goals, anti-goals

**Goals.**
- G1. The Church, Chinatown and the Port each read as one entry and the decline, and the entry opens a chooser: kinds switched on and off at the Church, exchanges counted up and down at Chinatown, one or two cards picked at the Port.
- G2. The Laboratory reads as one row per technology; a technology with more than one listed row opens a numbered pick of them.
- G3. The chooser is the REPL's, so the terminal and the browser both gain it. The browser draws it with the goods and yen tokens.
- G4. The terminal's players panel shows the per-country flag counts the model has carried since cycle 1.

**Non-goals** (each is a later, additive step; none is closed off).
- The Employment Agency and Customs. Their lists stay flat: the Employment Agency's ran past 20 rows in about one list in a thousand in random play and in none of the played samples, and Customs never read more than 5 rows (§11).
- Folding these families in the frame's own list. `frame.decision.options` still carries every listed move up to `OPTIONS_CAP`, as it does today; only the prompt's rows shrink.
- A token-drawn Laboratory payment in the browser. The browser draws G2 with the rows and the pick it already has.
- Lighting the Port's cards on the port strip. The strip can be switched off, so the answer bar carries the cards (§5.4).
- A one-line expert grammar for area actions, of the kind `a`, `b`, `m` and `r` are.
- The words of an exchange for a Mining owner. `describe` words `2 yen -> 1 copper` whatever the bonus; the chooser shows the same words.

**Anti-goals.**
- A fold in `web/` alone. It would be a second reading of the legal list.
- A chooser that assembles a move and applies it. A chooser names a listed move or commits nothing.
- A rate, a cost or a count worked out in `web/`.
- Any byte of the five fenced crates moved, or `SEMANTICS_REV` moved.
- A new word admitted through the web's line fence (`crates/yokohama-web/src/grammar.rs`).

## 3. What binds

- **The five fenced crates and the engine's authority** (`docs/web-lane.md` §1, line 34): "the five fenced crates (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`, `gen-data`) byte-identical, the CLI touched only by behaviour-preserving seam changes, `SEMANTICS_REV` 7, no bot behaviour moved". That sentence goes on to a fifth clause, the door unmetered since cycle F. This cycle holds every clause but the second. **It does not hold the second**: the REPL's behaviour and text move on purpose, by the decision recorded in the cycle-1 design's §11 ("chosen in the REPL, as Placement Option A is, so the terminal gains it too"). What replaces that clause's proof is in §6 and §7: the capture goldens and `guide_sync` move deliberately, each diff read, and `docs/web-lane.md` §1 records the exception.
- **The layer computes nothing about the game** (`docs/web-lane.md` §4, line 146): "every model is `screen_model`'s, every word is `describe`'s, every move goes through `Session`'s legal-list gate". The chooser's state, its preview and every fact the browser draws are the REPL's, carried on the prompt.
- **The legal list is the only legality authority.** A chooser reads the list. Whether a selection can be confirmed, whether one more exchange can be added, and which kinds are offered are all answered by looking for a listed move, never by a rule restated in the CLI.
- **The web's line fence is an allowlist and stays as it is.** `grammar.rs` admits ten head words (`confirm`, `cancel`, `more`, `list`, `undo`, `back`, `a`, `b`, `m`, `r`) or a head that is a number or an area token, followed by alphanumeric arguments. Every line the client composes for a chooser passes it unchanged (§5.2 to §5.4). The terminal also takes one form the fence would refuse, a Port card as it is displayed (`O11`); the client never sends it.
- **An open picker on the Worker is rebuilt by re-feeding its lines** (`crates/yokohama-worker/src/picker.rs`): at most 64 lines, only at the record and under the code they were recorded at. A chooser is a sub-prompt like the others, so the mechanism covers it unchanged. Its code digest moves with any CLI source, so a picker left open across a deploy of this change is dropped and reopened by the player, as cycle 1's §10 says.
- **The wire corpus is a contract.** `wire_fixtures` refuses a `PromptState` variant the corpus does not hold, and it counts a file only when it can read its `prompt.kind`, so a prompt's fixture is a whole payload from a real `Repl`. Chinatown is not on the two-player board. The corpus's walk already routes a second table at four players (`wire4`), so the Chinatown arm's fixture is routed there, and no public seam is added to the CLI to build one in process. That moves the walk's scripts and the Worker's corpus replay with it.
- **No step depends on a model reading a picture.** Layout cells measure boxes; picture work goes through the generators' `.aria.txt` snapshots; the look with the art in place is the owner's reading.

## 4. Approaches

### 4.1 Which lists are folded, and how

Decided by the owner on 2026-11-21, from the measurement in §11: choosers for the Church, Chinatown and the Port; the lighter fold for the Laboratory; nothing for the Employment Agency and Customs. The chooser is by ingredients (the cycle-1 design's §11), so the issue's "faith first" alternative is not taken: one row per faith still leaves up to 20 rows behind each.

### 4.2 One plan or two

Decided by the owner on 2026-11-21: two plans, in sequence. Considered:
- **One plan.** Four families across two front ends, each moving the guide pair, the capture goldens and the wire corpus. By estimate about twice cycle 1's eight tasks. Declined: it is the size at which this project splits a spec.
- **Two plans split by front end** (the REPL first, the browser second). Declined: a new prompt kind on the wire needs its browser drawing in the same plan, or the browser cannot answer the prompt between the two.
- **Two plans split by kind of chooser** (taken). Plan 2a is the goods choosers, the Church and Chinatown, and builds the shared pieces once. Plan 2b is the card choosers, the Port and the Laboratory, with the terminal's flag counts, written against 2a's head.

### 4.3 How a chooser's lines pass the fence (the designer's call)

The mockups wrote `-<number>` to take a Chinatown exchange back and `yen` / `import` to set the Port's second-card payment. None of the three passes `grammar.rs`: a head must be a listed word, a number or an area token. Considered:
- **Widen the fence** with a head per new word. Workable, and each new head is harmless outside its chooser, but it moves a security fence for a spelling.
- **Stay inside the fence** (taken). Chinatown takes one back with `back <number>`: `back` is a listed head and the number is an ordinary argument. The Port asks for the payment after `confirm`, with the numbered pick the REPL already opens for Option B's Telegram variants, so it needs no word at all.

The cost is one departure from the Port mockup: the payment is chosen after Confirm, in a two-row pick, and only when both payments can be afforded. `cancel` in that pick returns to the main list, as it does from Option B's Telegram pick, and the two cards are picked again.

### 4.4 When a family folds (the designer's call)

- **Above a row count** (say eight). Short lists would stay flat. Declined: the same area would ask in two different ways depending on a count the player cannot see in advance.
- **Whenever the family has more than one listed move** (taken). A family with exactly one move stays a direct row, which is the rule the route, retreat and assistant-move folds already follow, and keeps "Enter applies the only option" true.

## 5. The design

### 5.1 The seam

`Repl::group_options` gains an area-action arm beside Movement's and Placement's, for the AreaAction phase with no choice queued (a mercenary's sub-turn included). It folds by the family the listed moves belong to:

| Family | More than one listed move | Exactly one |
|---|---|---|
| Church (`AreaAction::Church`) | one chooser entry | a direct row |
| Chinatown (`AreaAction::Chinatown`) | one chooser entry | a direct row |
| Port (`AreaAction::Port`), plan 2b | one chooser entry | a direct row |
| Laboratory (`AreaAction::Laboratory`), plan 2b | per technology, by title: an entry opening a pick when it has more than one row | a direct row |

The decline (`AreaAction::Skip`) stays a direct row, last, as today. Every other area action passes through untouched. The entry's words are the mockups': `Church — choose what to pay (42 payment sets)`, `Chinatown — choose exchanges (up to 3; 102 combinations)`, `Port — take 1 or 2 order cards (6 in reach)`, `Laboratory: acquire Station — 3 ways to pay`.

Each chooser is a `SubPrompt` variant, an entry helper like the placement pickers: it holds a selection, shows a live preview, and commits only through an exact match against `session.legal()`. The preview is Option A's:

```
Selected: silk, tea, 2 yen
  -> legal — Church: pay silk, tea, 2 yen -> faith 8 (power 5 + 3 kinds)
```

or `-> NOT currently offered (adjust the selection)`. The words after `legal —` are `describe`'s for the matched listed move. A `confirm` that matches nothing is refused and keeps the chooser open with its selection, as Option A does. The move committed is the listed one, in the engine's own form, never one the chooser built.

### 5.2 The Church (plan 2a)

- Six rows in a fixed order: copper, silk, tea, seafood, 2 yen, imported good, each with how many the player has.
- **A number 1 to 6 switches its kind on or off.** A kind that no listed row pays is refused at the toggle and the selection is kept.
- The match is equality on the six flags.
- A refused selection carries a second line read off the listed rows, saying which faiths the list offers: `the list offers faith 8 to 11 here`. The faith of a listed row comes from the engine's own public arithmetic, as `describe`'s faith preview does.
- The queued church-space choice that follows a payment reaching two or more empty spaces is untouched.

### 5.3 Chinatown (plan 2a)

- Nine rows in the engine's order, four sales then five purchases, each in `describe`'s words (`1 copper -> 2 yen`) with the count chosen.
- **A number 1 to 9 adds one of that exchange.** It is taken only when the list holds the selection with that exchange added; otherwise it is refused and the selection is kept. So selling first is what makes a purchase the player cannot yet afford addable.
- **`back <number>` takes one of that exchange back.** It can leave a selection the list does not hold (the sale that paid for a purchase is gone); the preview says so and `confirm` refuses. Bare `back` is refused inside a chooser, as the route pick, the guided retreat and the two placement pickers refuse it. The reveal flow and the card index close on it; the choosers follow the pickers.
- The heading's "up to 3" is the longest listed row, read from the list.
- The match is by multiset: the same count of each exchange, whatever the order. The engine's listed order is what is committed.
- The amounts the browser draws as tokens come from the engine's public rate table (`chinatown_rate`), not from the words.

### 5.4 The Port (plan 2b)

- The cards in reach, by zone, each named by its printed corner number with the one-line words the terminal already gives an order card (the engine's `order_prompt_line`, which `describe` uses for a fulfil row).
- **A card's number switches it on or off**: the bare form (`11`), which is what the client sends, and in the terminal the displayed one (`O11`) too. Switching a card on is taken only when the list holds a row with the selection and that card; otherwise it is refused and the selection is kept. That one rule covers reach, hand room, the two-card limit and an unaffordable second card.
- `confirm` looks for listed rows taking exactly the selected cards. None: refused. One: committed. Two (the second card payable either way): the numbered pick opens over those two rows, each in `describe`'s words. `cancel` there returns to the main list and the selection is not kept (§4.3).
- An empty selection is not the decline. The decline is its own row on the main list.
- In the browser the cards are in the answer bar, drawn with the client's own card faces. The port strip is a section the reader can switch off, so it cannot be the only place to click.

### 5.5 The Laboratory (plan 2b)

One row per technology, by title, in the list's order. A technology with one listed row stays that direct row. One with more opens the existing numbered pick over its rows, titled with the technology. No new sub-prompt and no new prompt kind: the browser draws it with the rows and the "opens a choice" pick it has today.

The fold is by title and not by card, because ten titles exist as two physical cards and both copies can be on a board at once, in slots with different surcharges. A fold by card would show two entries worded alike. By title, both copies' rows sit in one pick, and each row keeps today's words, its slot fee included, so the copies are told apart by price exactly as they are today.

### 5.6 The wire

Three new `PromptState` arms, typed like `place_a` and `place_b`: `church` and `chinatown` in plan 2a, `port` in plan 2b. They ride the acting seat's payload only, by the scoping `prompt_of_seat` already applies. Each carries everything the browser draws, so the client reads no rule:

- **`church`**: per kind, the number to send, the kind, its cost as a good and an amount, how many the player has, whether it is selected, and whether any listed row pays it; the preview (legal or not, and its words); the offered-faiths line when there is one.
- **`chinatown`**: the longest listed row and the count chosen; per exchange, the number to send, what is spent and gained as goods and amounts, its words, the count chosen, whether one more can be added, and whether any listed row uses it; the preview.
- **`port`**: per card in reach, the number to send (its bare id), its id, its zone, whether it is selected and whether it can be switched on; the preview.

A good is named by the model's own type, which the client maps to the token art it already holds. The Church's yen kind costs 2 yen. The engine holds that as a literal and `describe` already words it as `2 yen`. The amount moves to one place in the CLI that both `describe`'s words and the arm read, and a scratch-apply cell pins it against the engine, by the `placement_fee` precedent.

**A chooser's terminal screen does not ride the browser's messages.** `api.rs` strips the main prompt from a reply's text and keeps an open sub-prompt's block, because the placement pickers' verdict has no structured form on the wire; so `place_a`'s replies carry the terminal's directory and its `> type an area id…` line. The chooser arms carry their preview structured. For the three chooser sub-prompts the block is stripped by identity, as the main prompt is, and a refusal line before it is kept. The placement pickers' and the pick's messages are left as they are.

### 5.7 The browser

`Decision.tsx` gains the three arms, drawn as the mockups: a grid of six token buttons at the Church; sell and buy rows with a minus, a count and a plus at Chinatown; the cards by zone at the Port. Confirm and Cancel serve the new kinds; Confirm is live when the preview is legal, and the server still decides. A kind no row pays, a plus that cannot add and a card that cannot be switched on are disabled from the arm's own flags.

What the client sends is the arm's number for a kind, an exchange's plus or a card; `back <number>` for a minus; `confirm`; `cancel`.

A token is one small component over `tokenArt`, with a one-node accessible reading (`role="img"`) and a plain mark where the art is absent.

### 5.8 The terminal's flag counts (plan 2b)

The terminal's players panel draws the five per-country counts of face-up flags for every seat (`screen_model::face_up_flags`) and, for an open hand, the flags of the orders in hand apart from them (`HandPanel::flags_in_hand`), marked as not yet counting. No new count: the model is cycle 1's and only the terminal's words are new.

### 5.9 What does not move

The engine's legal list and its order. Records and saves: a chooser commits the same `Move` its old row did. The bots, `suggest` and `why`, which read the engine's list and not the REPL's entries. `grammar.rs`. The Worker's picker mechanism. The frame's own option list.

What does move beyond the choosers: the numbers of the main list at these four areas, and with them everything that answers an area action by row number. Found so far: the wire corpus's walk scripts; the piped-dogfood driver; the browser walks; the web layer's whole-game gate (`crates/yokohama-web/tests/play_a_whole_game.rs`, whose `choose` panics on a prompt kind it does not know); and the CLI's own unit tests that commit a Port, Laboratory or Church row by its position in the legal list (in `repl.rs` and `reveal.rs`, the shared fixture `drive_to_double_assistant_move_cascade` among them). The plan lists them from a search of the tree, not from this sentence.

## 6. The cells

- **Every listed move, through the chooser.** For each chooser, over the reported Church position (all 42 sets), a Chinatown position of ordinary size, a Port position with pairs payable both ways, and a per-edit sweep of self-play positions at 2P, 3P and 4P: for every listed move of the family, the lines that name it commit exactly that move, compared as a `Move`. This is the cell that proves every row stays reachable; the design does not rest on how the generator builds its list.
- **Nothing but a listed move.** A selection one step from a listed one cannot be confirmed and applies nothing: a Church selection one kind short of an empty space, a Chinatown set no order makes payable, a second Port card with room in hand for one. Each case's data tells the right answer from the nearest wrong one.
- **The flags tell the truth.** For every selection the sweep reaches, "one more can be added" holds exactly when the list holds the selection with it, and likewise the Church's "offered" and the Port's "can be switched on".
- **The preview is `describe`'s.** The words after `legal —` equal `describe_move` of the matched move.
- **The fence, unmoved.** Every line the client composes for a chooser passes `is_gameplay_line` as it stands, `grammar.rs` has no diff, and `back <number>` outside a chooser applies nothing.
- **The Worker's restore.** A chooser left open is rebuilt by re-feeding its lines on a new instance, with the same selection.
- **The wire.** The corpus holds each new arm, Chinatown's routed on the walk's four-player table; the regeneration's diff is read; the leakage pins stay green.
- **The messages.** A chooser reply's `messages` carries no line of the terminal's chooser screen, and a refused toggle's line is carried.
- **Two copies of a title.** A Laboratory board holding both copies of a technology, in slots with different surcharges, reads as one entry whose pick lists every row of both.
- **The client.** Component cells per arm, a layout cell that measures the chooser's boxes at the phone and desktop widths, and browser walks that answer each chooser, one of them on a 3P or 4P table for Chinatown.
- **The whole game.** The piped-dogfood driver, the web layer's whole-game gate and the browser's whole-game walk learn the choosers and still reach a winner.
- **The mirror.** The Church's yen cost, pinned against a scratch apply.

Mutants, by the standing order: the implementer's probe, the task review on an export, and the whole-branch review.

## 7. The documents

In the commits that move the behaviour:
- **The CLI guide pair** (`docs/cli-guide.md` and `docs/cli-guide.html`), word-identical, with `guide_sync` and the capture goldens moved on purpose.
- **The browser guide** (`docs/web-guide.md`) and the in-app help's words (`web/src/Help.tsx`). The help's pictures are the wrap's: it re-cuts every one on the final tree.
- **`docs/web-lane.md`**: §1's fence sentence, with this cycle recorded as the one that moved the REPL on purpose; §2's counts; the corpus count; the prompt kinds where it lists them.

At the cycle's close: `CLAUDE.md`'s operating facts (the grammars line), the board, and the close document. The decision-log rows are owed at the wrap.

## 8. The suites and the gates

Every per-edit suite with the lint gate after every task, and the web lane's suites (`docs/web-lane.md` §2), since the change reaches `crates/yokohama-cli`. The fifteen release gates as one sequence before each plan's review closes; no fenced crate's bytes move, so fourteen are expected to read as at cycle 1's head and the piped-dogfood gate is the one this cycle changes. The private test Worker carries each plan's head for the owner's reading (`worker/RUNBOOK.md` §26), deployed by the orchestrator on the owner's word.

## 9. Left to the builder

Names of the variants, types and fields. The exact wording of each refusal, in the voice of the placement pickers' refusals. The layout of the Port's card directory in the terminal, and whether `?` opens the card index from inside it, following what the sub-prompts do today. The chooser screens' paging, if any is needed (none of the three lists more than nine rows). The sweep's seeds and game counts, sized to stay per-edit. The cost of a chooser prompt at the longest Chinatown list read (1,237 rows), measured and recorded, not bounded. Where a public engine helper is not reachable from the CLI crate, the rule is a `screen_model` mirror with a drift pin, never a widened engine item.

## 10. The two plans, and what stands before them

- **Plan 2a, the goods choosers:** §5.1's seam, §5.2, §5.3, the `church` and `chinatown` arms, the token component, their cells and documents.
- **Plan 2b, the card choosers:** §5.4, §5.5, the `port` arm, §5.8, their cells and documents. It is written against 2a's reviewed head, and what 2a's build teaches is recorded forward in it.

Each plan runs by subagent-driven development on its own lab branch, 2b's cut from 2a's head, with its task reviews, its whole-branch review and its fix wave there. Whether cycle 2 reaches origin as one pull request or two is the wrap's.

Before plan 2a's branch was cut, by the board's order: the seven files of cycle 1's Task 8 were committed on the owner's reading (`2ae2af07`), cycle 1's lab close was written (`21f11c5c`), and cycle 1's head was read on the private Worker (`worker/RUNBOOK.md` §26's record of 2026-11-21). Plan 2a's branch is `web-choosers-goods`, cut from `5abce3e3`, and this document is its first commit.

## 11. The measurement

Re-measured at `9625b34a` (core unchanged since `e600f5f4`) by a throwaway harness outside the tree, so these are readings and not a committed instrument. `get_legal_moves` at every position in the AreaAction phase with no choice queued, classified by the acting area. The table counts lists that offer something besides the decline: rows at p50 / p90 / max, then the share of lists past 20 rows, one terminal page.

| Family | Owner's games, 2P (7 saves) | Bot play, 3P (60 games) | Bot play, 4P (37 games) | Random play |
|---|---|---|---|---|
| Church | 5 / 27 / 64 · 21% | 2 / 12 / 33 · 3.5% | 2 / 8 / 27 · 0.6% | 6 / 27 / 58 · 18% |
| Chinatown | not on the board | 31 / 136 / 541 · 64% | 24 / 96 / 503 · 57% | 31 / 77 / 1,237 · 76% |
| Port | 5 / 22 / 37 · 20% | 7 / 22 / 37 · 21% | 5 / 22 / 37 · 15% | 4 / 5 / 37 · 1% |
| Laboratory | 5 / 10 / 13 · 0% | 4 / 7 / 31 · 0.4% | 4 / 7 / 25 · 0.1% | 4 / 7 / 25 · 0.1% |
| Employment Agency | 3 / 4 / 4 · 0% | 4 / 8 / 20 · 0% | 4 / 8 / 20 · 0% | 3 / 5 / 31 · 0.1% |
| Customs | 3 / 5 / 5 | 2 / 2 / 2 | 2 / 2 / 2 | 2 / 2 / 3 |

- **Random play** is issue #77's method: uniform-random self-play, 300 games at each player count, seeds 1000 to 1299. Its column is 2P, except Chinatown, which is 3P.
- **The owner's games** are seven saved two-player records, replayed position by position.
- **Bot play** is new: `v4-step9-draft1` at every seat through `examples/arena`, diagnostic master 20261120, 60 games at 3P and 37 of a planned 60 at 4P, stopped for time. It is the first reading of Chinatown under purposeful play.

What it says:
- **The issue's figures reproduce exactly, on one condition the issue does not state:** its percentiles leave out the lists that offer only the decline. With every list counted, seven Church visits in ten in random play offer the decline alone.
- **The Port is a wall as often as the Church.** Between one decision in seven and one in five runs past a page in all three played samples. Random play hid it at 1%.
- **Chinatown holds under purposeful play:** about two lists in three run past 12 rows, and at power 3 the median list is 100 to 134 rows.
- **The Church's wall is real in the owner's games and rare in bot play.**
- **The Laboratory's longest list is 31, above the issue's 25,** and under 0.5% of its lists pass a page. Some card has more than one payment row in 14% to 21% of the played samples' lists, and in 7% of random play's.

## 12. The builder check

An independent read of the first draft, as a builder who has to plan from it and cannot ask, with every factual sentence checked against the source. It returned four questions, seven false or overstated sentences and one defect. All are settled in this text:

- **The chooser's terminal screen in the browser's messages** (the most important): not carried, §5.6.
- **How the Chinatown arm's fixture is reached**: routed on the walk's four-player table, §3. The first draft said the walk was two-player and named a model-fragment fixture as its precedent; both were wrong.
- **Two copies of a title on one Laboratory board** (the defect): the fold is by title, §5.5. A fold by card would have shown two entries worded alike.
- **`cancel` in the Port's payment pick**: returns to the main list, as Option B's does, §4.3.
- **The corrected sentences**: the fence claim narrowed to the lines the client composes (the terminal's `O11` does not pass it); bare `back` is refused by the pickers, not by every sub-prompt; the list of what answers by row number, which missed the web layer's whole-game gate and the CLI's own unit tests, §5.9; `order_prompt_line` is the engine's; the fence sentence's fifth clause; the Church's yen cost, which `describe` already states.

The check also confirmed, by walking the code: every line the client sends passes the fence; `back <number>` at the main prompt applies nothing; a mercenary's sub-turn reaches the area-action arm; the Worker's restore needs no change for a new sub-prompt; and every listed Chinatown row can be built one exchange at a time through listed rows.
