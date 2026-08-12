# Yokohama CLI — Tester's Guide

*For the game frontend (`yokohama-cli`), 2026-08-06. Audience: playtesters tracking or simulating games of Yokohama (Hisashi Hayashi, 2016). You need the physical rules in your head; this guide covers how the tool speaks about them — and **flags every place its language or behavior differs from the rulebook** (marked ⚠).*

---

## 1. What this is

A terminal program backed by the full Yokohama rules engine. Two modes:

- **Simulation** — the program deals everything from a seed and you play moves for every seat. Good for learning the tool.
- **Live tracking** — you play a *physical* game at the table and enter each move; the engine validates it, tracks all state, and asks you to type in cards as they're physically revealed (port refills, tech refills, hidden starting orders). The face-up 5-Power tokens are entered once at setup (§3), not revealed in play.

The engine only ever offers **legal** moves. If the tool won't offer a move you believe is legal at the table, that is exactly the kind of bug this testing round exists to find — save the game and report it (§9).

> **Testing-round note:** this build is the redesigned frontend. On a real terminal it is a composed game screen (title strip, the true staggered-pyramid board, a persistent dashboard), every decision is a numbered menu in full words, every panel has a **full-screen view** one word away (§4), every card is identified off a **card index** rather than by an id you have to know (§4), and a line editor gives you ↑ recall and normal editing (§2). Every reward decision the rulebook gives you is a real prompt (§8.6), the setup wizard re-asks indefinitely on bad input instead of aborting (§3), and the Canal can be crossed (§8.5). Live-tracking a full physical game start to finish is the point of this round — save and report anything that looks wrong (§9).

## 2. Running it

**Prerequisites.** The Rust toolchain (install it via rustup), a clone of this repository, and a shell sitting in the repo root. Plain `cargo` works on the project machine.

```bash
# from the repo root
cargo run -p yokohama-cli              # start the setup wizard
cargo run -p yokohama-cli -- save.json # resume a saved game
cargo run -p yokohama-cli -- import capture.ndjson --answers # BGA bridge: setup answers from a capture
cargo run -p yokohama-cli -- import capture.ndjson --follow  # BGA bridge: live-follow a table being captured
```

**First run.** `cargo run -p yokohama-cli` with no arguments opens the setup wizard, which asks — one question at a time — for the mode, player count, per-seat colors, start seat, seed, and save path, then a mode-specific tail; §3 walks through every question. Fastest way to try it: run the command and answer `sim`, `2`, `red`, `blue`, `0`, then press Enter twice to take a random seed and the default save file, keep either dealt card with `1` and `1`, press Enter once more for no bot seats, and `confirm` — you are now driving a 2-player simulation from the prompt in §4.

**Resuming.** `cargo run -p yokohama-cli -- <save.json>` replays a saved game deterministically to exactly where you left off. The game autosaves after **every** applied move once a save path is set, and the wizard sets one by default — `yokohama-game-<seed>.json` in the current directory unless you gave a path — so a crash loses at most the move you were typing; just rerun with the save file.

**Importing a BGA capture (bridge).** `cargo run -p yokohama-cli -- import <capture.ndjson>` reads a relay capture file from the browser bridge (`docs/bridge-setup.md`) and prints a **setup answer sheet**: every question the live wizard asks, answered where the capture carries evidence — each answer names its provenance — and marked `ABSENT` with the reason wherever it does not, so you know exactly what the wizard will still ask. `--answers` stops there. Without it the tool goes on to **replay the captured game** through the engine's own legality checks: it resolves the setup from the capture's evidence (declared assumptions and replay-decided candidates are listed, never silent), then translates the notification stream move by move, cross-checking the game state against BGA's own counters at every socket checkpoint and against BGA's final-scoring table at game end. `--through <move>` bounds the replay at a BGA move id; `--out <record.json>` saves the replayed game as a normal live-mode record (stamped `bga-import/<table>` — comparison material, never a tuning input); `--report <path>` writes the replay report to a file; `--table <id>` picks one table out of a capture holding several. When a captured game replays cleanly under MORE THAN ONE achievement assignment — the capture's own quantities matching two engine cards and the game never distinguishing them — the tool refuses rather than picking one, and `--achievement <A|B|C>=<Type>` (e.g. `--achievement B=Orders5`) is how you supply the answer from the physical cards. The answer narrows the candidates; it does not skip the replay, so an answer the game contradicts still stops, and the report declares the assignment as an owner answer rather than as evidence. The importer never guesses: an unrecognized card, code, or shape in the capture, an untranslatable notification, or a state divergence is a typed error naming what was seen, and hidden information stays hidden — the opponent's cards and the undrawn decks come through as counts, never identities, exactly as live tracking models them.

**Live-following a table (bridge).** `cargo run -p yokohama-cli -- import <capture.ndjson> --follow` turns the importer into a read-only **spectator**: it tails the capture file as the browser bridge appends to it, and whenever new complete lines arrive it rebuilds the whole replay from move 1 through the same checked legality boundary and redraws the game screen — the same composed frame the driver plays in, with a status line where the prompt would be, and plain rolling text through a pipe. Rebuilding from scratch is what makes a BGA **takeback** honest (the feed collapse retracts the undone packets, so the spectate state rewinds with the table) and what makes the follower **restart-safe**: kill it, run it again on the same file, and it reaches the identical state, byte for byte. Anything the pipeline cannot yet pass — no snapshot taken yet, evidence still to arrive, a documented rules divergence — is a `HOLD` printed **once**: the follower keeps tailing and resumes on its own the moment later input makes progress possible; it never exits on trouble. It ends only when the replay reaches game end (printing the final-score cross-check against BGA's own totals) or when you Ctrl-C it. `--out` re-saves the record on every advance, so a crash loses nothing past the last state change; `--table` still selects a table; `--follow` cannot be combined with `--answers`, `--through`, or `--report`, which describe a run that ends.

**The composed screen (terminal only).** On a real terminal the tool is a full game screen, redrawn after every input: a title strip, the board (one bordered tile box per area, laid out as the physical staggered pyramid — widest row on top, each row below offset half a tile), a live dashboard (a PLAYERS panel — current player marked `▶`, then each seat's yen/VP, goods, assistant/shophouse/trade-house zones with the next trading-house take price, one line per technology owned, mercenaries, President location by area name, orders in hand, and the seat number the `reveal <seat>` command uses — then a TRACKS panel, an ACHIEVEMENTS panel, and YOUR HAND drawn as a card) — beside the board at 2 players, where the board legend follows both. At 3 and 4 the board is too wide to leave room beside it, so the legend comes first, directly under the board it explains, and the dashboard is laid out below in **one column per seat** — the same fields in the mock's shorter wording (`8 hand · 0 board · 12 warehouse`), technologies named rather than explained, and the orders/locked-pawns/next-take figures on a tail line under each seat (`next take ¥4` is the **Employment Agency price of that seat's next trading house** — the printed 4/5/6/7 progression; the beside layout prints it on the Trade houses line itself, where the noun is right there) — with the TRACKS, ACHIEVEMENTS and YOUR HAND panels each compressed onto one titled row (your hand named there rather than drawn; `hand` draws the cards). Then an **end-trigger strip** — one line, on every screen, stating how close the game is to ending on every trigger at once: `end triggers: deck 8 · church 2/4 · customs 0/4 · shophouses 5/8 · trade houses 2/4` reads as orders left in the deck (a count only — never which cards), Church and Customs spaces filled against the count that ends the game (4 at 2 players, 5 at 3, 6 at 4 — **real pawns only**: at 2 players the neutral dummy on faith-7 and space-4 sits *outside* this count, so your 4th real assistant is still the trigger; the deck reading is also necessary-not-sufficient — the rulebook ends the game only when the empty deck leaves a Port slot unfillable), and the best-placed seat's shophouses of 8 and trading houses of 4. Once a trigger fires the line announces `FINAL ROUND` instead. Then a game-info strip naming every view, then the message zone (what the last move changed) and the prompt. `panel` hides/shows the dashboard; `screen` turns the whole composed mode off and back on (plain rolling text, same content). Piped or redirected output is always plain rolling text with zero escape codes — the composed screen and all color exist only on a terminal. (Every captured screen in this guide is that plain form, taken through a pipe: on a real terminal the same screen is coloured, and its standing hint carries one extra clause — `↑ recalls what you typed` — that only appears where the line editor is live.)

**Color and width.** The tool colors what matters — player names and their pieces in their colors, tile names in bold, site-slot tokens and goods letters in resource tones, VP, warnings, the phase line — and setting the standard `NO_COLOR` environment variable turns color off in a terminal too (the composed screen stays; `screen` turns that off). The composed screen is wide, and at most counts wider than the board alone: the board grid is 120 columns at 2 players, 150 at 3, and 180 at 4, and a different part of the screen sets the total at each count — the dashboard joined beside the board at 2, and the dashboard stacked below it at 3 and 4 — so the whole frame is 198 columns at 2 players, 175 at 3, and 200 at 4. At 3 players that width is what it takes to write every legal state out in full words: the widest achievements row a game can show is 171 columns, so the block is held open to 175 rather than to the 163 the board and its legend would have set. It stays there: a test walks whole games at every player count and the frame never grows past those widths, so what you size the window to at the start is what the game needs at the end. The composed frame does no terminal-size detection, so size the window before you start: **200 columns** clears every layout's width — a test now pins every line the driver *lays out* (board, panel, option lists, prompts and hints, the choice banner, a lockdown notice) at or under 200 columns, so none of them wraps mid-token; the one thing that can still wrap is a message quoting something you typed, such as an autosave warning naming a long save path — and the screen opens 49 rows at 2 players, 50 at 3, and 49 at 4 — so a **200x50** window shows the whole opening screen at **every** player count (the 3-player opening fills it exactly). The column layout is what makes the higher counts fit, and it fits them for the whole game rather than only at the start: at 3 and 4 players a seat column is a fixed block of rows whatever that seat owns, so nothing a player collects can make the dashboard taller. What can still add rows is the part below it: the decision zone budgets twelve rows for its option list (`more` pages the rest, packing short options into columns to fit more of them into those twelve), and a big message zone pushes the title and the top board row off the top. In play the screen reaches 61 rows at 3 and 4 players, measured over whole-game walks and held there by a test. **Two players is the one count that keeps growing.** Its dashboard sits beside the board rather than below it, and that shape is ragged: it gains a line for each technology a seat owns, so a long 2-player game gets a slowly taller screen rather than a fixed one — 68 rows on the deepest walk the tests run, and higher on a longer game. That is a property of the side-by-side layout, not a bound. `panel` (hide the dashboard) and `screen` (plain rolling text) buy those rows back. The `board` command's one-time reminder covers only the board grid's own width (and only when that grid is wider than 100 columns), which every player count now is. **The card index is the one surface that does measure the terminal** (§3, §4): it asks how wide the window is and lays its card grid out to fit — five cards to a row at 148 columns rather than the eight it draws at 200 — so a card is never split down the middle, and the wizard says so once, above the first index it draws, when the window is under 200. Piped output measures nothing and is unchanged.

**Typing (terminal only).** On a real terminal — the wizard's questions and the game prompt alike — the line you are typing goes through a line editor instead of being read raw: ← and → move inside the line, Backspace and Delete edit it, Home and End jump to its ends, and **↑ and ↓ walk back through the lines you have already entered this session**, so a long area id or card name you typed three moves ago comes back with one keypress. That history is per-session and lives in memory only; it is **never written to a file**, because in a live game the lines you type name cards that are still hidden, and a history file would leave them on disk after the game ends. **Ctrl-C** throws away the line you are part-way through and gives you a fresh one — it does not quit, and it never applies anything (which is exactly why it is not delivered as a bare Enter, the keystroke that applies a forced single option). **Ctrl-D** on an empty line ends the session, the same way a script running out of lines does; type `quit` instead if you want the `bye` line. There is no single-keypress toggle: a view is its word (`tracks`, `sites`, `hand`, `log`, `?`) or its number in the `view` menu, then Enter. Piped or redirected input skips the editor entirely and is read line by line, byte for byte — which is what keeps a scripted run identical to what it was before the editor existed.

**Privacy at a live table.** The one genuinely hidden thing in Yokohama is a starting order, and it sits in a hand — so in **live** mode hand contents start hidden: `hand` and `players` show counts (`1 card (hidden — 'privacy' to toggle)`) instead of card text, and the dashboard never shows another seat's hand contents at all. Toggle with `privacy` (sim mode starts open). Everything public at the table — the board, tracks, buildings, port displays, deltas of face-up takes — never masks. **Bot seats mask on their own.** With a bot at the table (§3) that seat's hand stays hidden whatever `privacy` says — a bot's hand holds its unfulfilled starting order, and sim mode starts open — while your own hand keeps following the toggle; the bot's seat is marked `[bot]` in the dashboard. That mask is table manners, not a sealed envelope: you set the game up, so you saw every seat's dealt cards in the wizard, and `log` still replays everything that happened.

**If something looks off:**

- **Board rows shear or wrap** — the terminal is narrower than the composed screen; widen it to at least the frame width above for your player count (200 columns clears every layout).
- **The title, or the top of the board, is missing** — the terminal is shorter than the composed screen; give it at least the opening row count above for your player count, and remember a long option list adds rows below the board.
- **A save won't load** — the tool prints `failed to load <path>: <reason>` and exits; the reason is a typed error. A JSON parse failure means the file is truncated or hand-edited; `replay failed at event N: ...` means a recorded move no longer reconstructs (a stale or tampered save). A **semantics-revision** drift is refused outright: a save recorded under a different game-semantics revision — or carrying none at all (a pre-Phase-2.9 record) — cannot be resumed, because the rules or the board have changed since it was written and replaying it would silently rebuild the game under semantics it was never played under. The message names both revisions and tells you to start a new game; that is the only remedy, and it is why saves from earlier test builds of this round do not reopen. A mere engine- or rules-**version** mismatch is *not* refused: it prints a `warning:` line at the top of the first screen and replays anyway.
- **A save loads with a `warning:` line above the prompt** — the save came from a build with a different engine version or different card/board data. It replayed anyway; the warning is there so you can say so in a report (§9).
- **A move is rejected with `Illegal: ...`** — the engine disagrees that the move is legal right now. If it is legal at the table, that is a bug worth reporting: `save` and follow §9. Its more serious cousin, a mid-game `CRITICAL: desync` lockdown, is covered in §6.

The release gates — `cargo test` and the fuzz sweep — are developer-only; a tester never runs them.

## 3. The setup wizard

Questions are asked one at a time. An invalid answer re-asks the SAME question and shows why (`  ! '<answer>' is not ...`) — there's no limit on retries. Typing `abort` (case-insensitive) at any question exits setup right away; that's the only other way out mid-wizard. Answers in `[brackets]` are the literal things you type.

| Question | Answer |
|---|---|
| Mode | `sim` or `live` |
| Players | `2`, `3`, or `4` |
| Color, per seat | `red`, `blue`, `yellow`, `green` (no repeats) |
| Start seat | seat index, `0`-based (e.g. `1` = second seat listed) |
| Seed | any integer, or **blank** for a time-derived one (it echoes `seed: N` — write it down; it is the game's identity) |
| Save path | a path, or **blank** for `yokohama-game-<seed>.json` in the current directory |

**Sim tail:** for each seat the two dealt order cards are shown, numbered `[1]`/`[2]` and each carrying a `(bot Keep score N)` — what the built-in bot (§4) would prefer, read off the card's printed economics alone. It is hand-set and unmeasured, and a note under the pair says so; the choice is yours. The question is `keep card 1 or 2?`; answer `1` or `2` — the numbers printed beside the cards.

**Bot seats (sim only):** the sim tail's last question is `Bot seats?`. **Blank** — just Enter — means none, and so does typing `none`; the game is then exactly the all-human one described everywhere else. Otherwise type the seat numbers a bot should play, space-separated: `1`, or `0 2`. At least one seat must stay human (an all-bot table is refused and the question re-asks), so at a two-player table one bot is the most any legal answer can name, and the prompt's example there offers a single seat rather than a pair. There is no live-mode equivalent — a bot cannot read a face-down card off your table. Every bot plays `lookahead-v1` — the engine's shipped search, a 4-wide beam over one sampled guess at the hidden cards scoring positions with the frozen **v2** weight table, which is the same thing `suggest` runs on your own seat (§4). It is the controller this project intends to ship rather than a sparring dummy, and it is deliberately **provisional**: it stays pinned to that exact identity until the strength work freezes a champion to replace it. There is no difficulty setting and no per-seat choice — the question asks which seats, not which opponent. The wizard draws a full-width stream key for them and echoes it as `bot seed: N`, so write that down beside the game seed if you want the bot's line to be reproducible. The setup review names them in full (`bots: seat 1 playing lookahead-v1@v2{b=4,eval_impl=3,k=1,m=8,weights=-7339724794018772210} (seed N)` — the controller, its implementation version, and every knob and identity value the save records), `edit bots` re-asks the question, and the answer travels in the save file — a resumed game comes back with the same seats botted, playing the search the record names rather than today's default.

**Every card question speaks one grammar** (whether you are naming an order card, a technology, a building-site card or an achievement):

- **a number** — the number printed beside the card, wherever a menu or index is on screen;
- **an id** — `O14`, `T07`, `S12`. Ids are shown as small annotations and are never required; a technology's id is not printed on the card at all, so use its name.
- **a name or any unambiguous start of one** — `telegram`, `telep`, `Copper5`. A start that names two cards is refused and told you what it could have meant, never guessed.
- **`?`** — draws that registry's card index, then asks the same question again underneath it with everything you have already entered still entered. Nothing closes the index in the wizard, because nothing has been covered up: the question simply continues below, and `?` again redraws. Once cards with numbers are on screen, a bare number picks the card by the number printed on it — **and it keeps that meaning for the rest of that run of questions**, because the index is still on the screen above them: open it once at Port A slot 1 and the numbers you read off it are the numbers to type down all six slots. The prompt's own hint says which reading is live every time it asks (`a number from the screen` once an index is up, plain `a number` before it), a filter you type replaces the screen the numbers come from, and the next section of the tail — a different registry, or the same one asked about somewhere else — starts over from ids.
- **`f <term>`** narrows an index, and **what a registry can be filtered by differs**, because the four cards carry different things: **order cards** take a good (`f silk`), a VP floor (`f 8vp`) or a flag (`f france`); **technologies** take a flag (`f france`) — filter them by name instead, since typing a name is how you pick one anyway; **achievements** take a category (`f a`, `f b`, `f c`); **building-site cards** take no filter at all, there being nothing printed on them to sort by. `f` alone clears, everywhere. A term that is real somewhere else is refused rather than applied — `f silk` on the site index says so and names what that index does take — because a filter that quietly matched nothing would draw the same empty screen as one that legitimately found nothing.
- **`abort`** still leaves setup from any of them.

Whatever you enter is **echoed back as the whole card** before the next question, so a card typed one digit wrong is caught on sight rather than at the end of the tail. A card already recorded somewhere else is refused **by that place** — `O14 is Seat 0's deal — a card cannot be in two places.` — and every card question carries a running `already recorded — …` line.

**Live tail** (you're describing the physical table):

1. Per seat: `known` (your own seat — then type its two dealt cards and the keep choice) or `hidden` (opponents whose starting order you can't see).
2. **Tiles**: for each board cell, the area tile physically there. Each question prints **the tiles still to place on this board, numbered, with how many of each is left**, and the list shrinks as you go — but **a tile type's number never moves**: it is that type's place in the board's whole tile set, fixed for the pass, so a row that runs out takes its number with it rather than handing it to the tile below. (Type the number of a type you have used up and you are told so by name, with how many the set has.) — so a 2-player board never offers the Canal, Chinatown, Port B or Laboratory B, which are in the box at that count. Answer with the number, the full name (or any unambiguous start of it), or the old short code — `cm sm tp fg bank pa pb la lb ea ct ch cu canal` still work as aliases (§8.5 maps them). A tile the count does not use is refused by name (`Canal is not part of a 2-player board — that tile stays in the box`), and one whose copies are all placed is refused with the set size (`no more Copper Mine tiles for this board — the 2-player set has 1`) — on the spot, not after the rest of the tail. **Read the orientation block the wizard prints first — see "Which tile is cell 0?" below.**
3. **Building-site cards**: per non-Canal cell, the site card's number `S<n>` or bare `n` — checked against the registry and against every other site card already entered (a repeat is refused by the area it is already on). Same cells in the same order as the tiles, and each question names the tile you entered there (`Cell 3/14 (row 1 of 4, tile 4 of 5) Fishing Ground — building-site card?`), so you always know which physical tile you are reading a card off. Nothing is printed on these cards, so `?` draws all 24 with their four spaces — as a **look-up**, not a pick list: a site card's number *is* its identity, so the index shows one number per card (`SITE 7`) and that is the number you type.
4. **5-Power tokens**: the face-up token on each non-Canal cell. Like the tiles, each question prints **the faces still to place, numbered, with how many of each is left**, counting down as you go — and like the tiles, **a face's number never moves**: it is that face's place in the box's own list, fixed for the pass, so a face you have spent takes its number with it rather than handing it to the one below (typing it then names the face and says there are none left). The letters still work as aliases — `i` = 1 imported good, `c` = 2 copper, `s` = 2 silk, `t` = 3 tea, `f` = 3 seafood, `y` = 3 yen. Rulebook p.3 step 5 places these **face-up**, so you enter what you physically see. Same cells in the same order as the tiles, each question naming the tile you entered there, and Canal cells carry no token and are skipped. Checked live against the 20-token pool (10 import, 2 each of the rest), so a third copper — or any face beyond the pool — is refused on the spot with `no more 2 copper tokens available for this board`.
5. **Achievements**: three cards are in play, **one from each category** (A, B, C), so the wizard asks for them one category at a time — `Which category A card is in play?`, then B, then C. Each question prints that category's four cards with their numbers, their scoring (`8/6 VP` = first claimant / each later one) and the name you can type instead — reprinted every time the question is asked, so a refusal or a `?` never leaves you picking a number off a screen that has scrolled away. `?` draws all twelve, numbered `1`-`12`; those numbers are the same ones the four-card menu shows, and unlike every other index they do **not** renumber when you filter — `f b` leaves category B's cards on `[5]`-`[8]`, so one number always means one card. Naming a card from another category is refused and says so (`Money10 is a category B card — this question is asking for the category A one`), which is why a table with two cards from one category can no longer be entered at all.
6. **Port contents**: the 6 order cards on Port A left→right, by number, id (`O<n>`) or `?`; Port B is only asked at 3-4 players. Each card is checked against the order registry and against every order card already entered anywhere — both seats' deals, Port A, and Port B share one duplicate check, and a repeat is refused by the place it is already in.
7. **Lab contents**: the 6 tech cards on Laboratory A left→right, **by name** — any unambiguous start of one will do (`telep`, `postal`, `tex`), and `?` draws all twenty with their effects. Lab B only at 4 players. Checked the same way as orders, with Lab A and Lab B sharing one duplicate check. Ten of the twenty titles circulate as **two identical copies**: typing the name twice fills two slots with the two copies, and a third time is refused by the slot the first one is in. `T<id>` is accepted but never required — nothing of the sort is printed on the card (§8.2).

Every card/tile/site answer above is validated the moment you type it — a bad entry re-asks that same question instead of surfacing at the end of the tail.

**Which tile is cell 0?** This is the one setup answer nothing downstream can catch for you, so the wizard spends real screen on it. Before the first tile question it draws a numbered skeleton of the actual board shape for your player count — the real row lengths, brick-offset, widest row on top, each cell showing the number it will ask for — and states the orientation in words. The rules, in short:

- The board is a staggered pyramid of four rows, **widest row first**: 4/3/2/1 tiles at 2 players, 5/4/3/2 at 3, 6/5/4/3 at 4.
- Turn the assembled board so the **widest row is farthest from you**. **Cell 0 is the leftmost tile of that widest far row**; numbering runs left to right along a row, then down to the next row, the way you read a page. Every prompt repeats where you are — `Cell 7/14 (row 2 of 4, tile 3 of 4)` — and each row is announced as entry reaches it, so the picture scrolling away costs you nothing.
- Read every row in the **same direction**. Which end you start from does not matter — reading all four rows from the other end mirrors the board, and a consistently mirrored board is the same board to the engine — but switching direction partway through is not, and neither is changing the row order.
- ⚠ **The rulebook's p.3 diagram prints this pyramid the other way up**, narrowest row at the top. Do not follow that picture's reading order: enter the widest row first regardless of which way your copy of the diagram faces. The engine models the board's adjacency **graph**, not the printed picture — which is also why the mirror above is harmless.

Why it matters: the engine checks that your tiles are the right *set* for the player count, but it cannot check the *order* — every arrangement is a legal board. Get the order wrong and movement, placement fees, Telegram bonuses and canal tolls all run on a board that isn't the one on your table, silently, for the whole game. Hence the numbered skeleton, the coordinates on every prompt, and the board check below.

**Review before you commit:** once every question is answered, the tool prints a full summary of everything entered so far (`--- Setup review ---`: mode, players, colors, start seat, seed, save path, plus the mode-specific tail) and asks `confirm / edit <section> / abort?`. `edit <section>` re-asks that section's questions from scratch — with the same live validation — then reprints the updated summary; repeat as many times as you like before confirming. In the live tail the tile lines echo the same coordinates the questions used, in full words (`cell 7 (row 2 of 4, tile 3 of 4): Port A S6 5P:i`); the achievements are listed one per line under their category; and a keep choice is echoed as the number you typed (`keep card 1`), not as the engine's index. Editable sections:

- **live**: `colors`, `seed`, `save`, `deal <seat>`, `tiles`, `sites`, `tokens`, `achievements`, `porta`, `portb` (3-4p only), `laba`, `labb` (4p only)
- **sim**: `colors`, `seed`, `save`, `deal <seat>`, `bots`

In the live tail, `edit tiles` re-asks the **building-site cards** in the same pass: the site cards sit on the tiles, so a re-entered board moves them too, and re-asking the tiles alone would silently re-attach the old site cards to the new tile order.

**The board check (live only):** `confirm` no longer starts the game — it builds the game and draws the finished board back to you, with the same renderer play itself uses, and asks `Does this match your physical board? (yes/no)`. Compare the **shape** first (widest row on top), then the tile in every cell; `A00` is cell 0, the leftmost tile of the widest top row. `yes` starts the game. `no` returns you to the setup review, where `edit tiles` re-enters the board — nothing is lost, and you can go round as many times as you need. This is the last point at which a mis-ordered board can be caught, so give it the ten seconds. (Anything that isn't `yes` or `no` re-asks; `abort` still works here, as at every other question.) The **sim** tail has no board check: nothing was entered, so there is nothing to compare.

**The two-player dummy assistants (live, 2 players only):** the board check signs off with a setup instruction — before the first move, put two neutral assistants on the physical board, one on Church faith-7 and one on Customs space-4, in a player color nobody is using. They are **not** printed board art, and nothing you typed into the wizard describes them: the engine places both itself the moment the game is built, so only your table can be missing them. At two players the board check's own question names them — a `yes` there confirms the pawns are physically placed, not just that the tiles match. Each one blocks its space against both players, counts toward the occupied spaces that end the game on its track (so your **4th** assistant there is the last one), and competes in that track's majority bonus at final scoring. Skip them and all three diverge from what the players see, silently, for the rest of the game — the tool refuses a placement they think is legal, ends the game a pawn sooner than they expect, and scores both majorities against a rival that isn't on their board. One part of the dummy is **not** a pawn at all: at final scoring it also competes in the **Technology majority**, with an industry value of exactly **7.5** — a real industry total of 7 or less loses its place (10 VP for first, 5 VP for second) to the dummy, and a place the dummy takes is awarded to **nobody**. Nothing sits on the table for that one; the engine applies it in every 2-player game. Once play starts, `tracks` shows them as `D` (§8.5). At 3 and 4 players there is no dummy and no instruction, and **sim** never shows it either, having no physical board to put anything on.

`mode`, `players`, and the start seat aren't editable sections — they decide the shape of everything asked after them (seat count, grid size, whether Port B/Lab B even exist), so changing one of those means aborting and rerunning the wizard from scratch. Editing `seed` in the **sim** tail re-derives the deal preview from the new seed and re-asks every seat's keep choice against it, so a keep answer never silently survives against a preview you no longer see; in the **live** tail, `seed` is only the save file's identity (plus a throwaway check the wizard uses to validate your tile answers, which doesn't actually depend on which seed you pick for a given player count), so editing it there doesn't re-ask anything else.

## 4. Playing: the prompt and entering moves

Every screen ends with the **decision zone**: a phase chain, a "YOU ARE DECIDING" line saying what the game is asking, the numbered legal options, any dimmed near-miss rows, and a standing hint above the input caret:

```
TURN 1 · RED's turn    ADDITIONAL  ▸ Placement ▸ Movement ▸ Area Action ▸ 5-Power ▸ Building ▸ Recovery ▸ Additional ▸ End

YOU ARE DECIDING · whether to use an additional action before Placement, or to move on.
   [1] End additional-action phase
(only option — press Enter to apply · nothing is final until you apply it · 'back' undoes the last applied entry · 'view' any surface · '?' the card index · 'help' for commands)
> 
```

Chain anatomy: turn number and whose turn, then all nine phases of a turn (§8.4) with the current one lit — so where you are in the turn is one glance rather than a name you have to place. Whose money, goods and cards are what: the dashboard beside (or below) the board, which the composed screen redraws every turn.

The standing hint's second clause is phase/choice-aware — it only ever names a shorthand that is actually usable right now: `'a'/'b' place directly` in `Placement`, `'r' retreats directly` in `Movement`, `'m <from> <to>' moves one directly` while an assistant-move choice is queued, and nothing extra everywhere else, including every OTHER kind of pending choice. Those shorthands are aliases for people who already know them; every decision on the screen is reachable by its number.

On a real terminal the hint also names **`↑ recalls what you typed`** — the line editor's history (§2). It is printed only where it is true: a piped or redirected run has no editor and no arrow keys, so the clause is absent there rather than promising a key that does nothing. Its other two navigation clauses are live everywhere and name two DIFFERENT screens — `view` opens the game-info menu, `?` opens the card index.

**Players are named by colour; seats are named by number.** The tool holds one rule about this and it is worth knowing which is which. Anything that says *who did this, who has this, who is standing here* uses the **colour**, because the colour is what is on the table: the board, the dashboard, the fee clauses (`— fee 2 yen (2 to Blue)`), the site cards' builders, the log. A **seat number** is used only where the thing being named is a seat rather than a player — the `reveal <seat>` command and its menu, the `(seat 1)` annotation on the dashboard that is that command's key, and the setup wizard's per-seat questions, where a deal belongs to a seat nobody has played from yet (`O14 is Seat 0's deal`).

**Numbers start at 1.** Every numbered list in the tool — the options below the decision zone, the `view` menu, a route sub-prompt, the card index — numbers its first entry `[1]`, so what you read is what you type. Three things on screen are *not* list numbers and do not shift: **seat numbers** (`Seat 0`), **ids** printed on the components or held in your save (`A03`, `O14`, `T07`, site indexes), and **counts** — the guided retreat's `recover how many from … [0-4]` is asking for a quantity, and `0` there means "recover none".

**Choosing a move:** type the option's number. The list is budgeted at **twelve rows** rather than a fixed number of options: a long list of short options packs into two or three columns (numbered down the first column, then on to the top of the next), and whatever still does not fit pages — `more` goes forward (wrapping back to the first page), `list` returns to the first. Whenever there is more than one page, a line under the options says how many are left and which page `more` will land on; on the last page, where nothing is left to count, it says so and names the page `more` wraps back to. Short lists stay one option per line — and an option too long for your frame's width (§2) wraps onto an indented continuation line rather than being cut short, because the tail of an option row is where its price is and the price is usually what you are choosing between.

**Forced moves take bare Enter.** When exactly one option is displayed, the hint opens with `only option — press Enter to apply` and an empty line applies it — recorded in the log and save exactly like typing its number. When that lone entry is a grouped family instead (a Route, the Retreat family, an Option A or B placement), the line reads `press Enter to open` and Enter opens that picker without committing anything. Turn ceremony (the End-additional-actions, Skip-5-Power, Skip-building no-ops of an ordinary turn) becomes four taps of Enter. On a multi-option prompt an empty line still does nothing.

**Previews on the options.** Decisions carry their arithmetic inline, computed by the same engine code that will charge you: placement options show the fee and who receives it (`— fee 2 yen (2 to Blue)`); Employment Agency composites show their total (`— cost 6 yen`, from the 2-yen shophouse price and the printed 4/5/6/7 trading-house progression) or `(free)`; Church payment options show the faith they produce (`-> faith 6 (power 3 + 3 kinds)`); production options show the yield at your current power AND the next threshold (`Produce copper (power 2 -> +0 copper, next +1 at power 3)`), so a zero-yield line reads as "not yet", not as a bug; the Bank previews its yield the same way (`Take money at the Bank (power 3 -> +3 yen)`); and the Customs payment options preview the space(s) they land on with printed VP (`Customs: pay 1 imported good -> space 1 (+5 VP)`).

**Finding information: the view pattern.** Every panel on the game screen is a *summary*. The tracks panel prints eight bracketed faith numbers, which is enough to see that faith 7 is taken and not enough to decide whether to take faith 9. Each of those summaries has a **full surface** behind it, and that surface is one word away: it replaces the whole screen, draws the same facts card by card, and `Enter` puts you back at the prompt you left with nothing entered and nothing there cancelled.

Six surfaces render that way, plus four card indexes:

| Word | Surface |
|---|---|
| `ports` | **Port offer** — all six slots of each Port board as the order card on them, with the action power each zone takes to reach (LOW 2, MID 3, HIGH 4) and the order deck's remaining count. |
| `labs` | **Laboratory offer** — every Laboratory slot's technology, its country, its whole effect text in full words, and the **industry it costs in that slot** (the card's printed cost plus the slot's own surcharge, already added). |
| `tracks` | **Tracks** — every Church faith space and Customs space as a card, with its printed rewards, its occupant, and (at 2 players) the neutral dummy. |
| `sites` | **Building sites** — every area's site card: the four printed space rewards in full words, who has built on each, the trading house's VP, and the area's tolled canal crossings. |
| `goals` | **Achievements** — the three cards in play, what each pays first and later, who has claimed it, and every seat's progress. ⚠ This one is a card *index* wearing a word of its own, so it leaves like an index (below), not like a view. |
| `hand` | **Your hand** — the current player's order cards drawn full size, the kept starting order marked. |
| `log` | **Log** — the move and reveal trail, each line worded as it read at the moment it applied. |
| `orders` `techs` `sitecards` `?` | the **card indexes** (below). |

`view` (also `views`) opens a numbered menu over all of them — itself a full-screen surface, and its own header says which of its two families a row belongs to: most rows open the full surface named above, while **Board, Players and Score redraw the frame** with their block on it rather than replacing the screen. Picking a number opens that row. `Enter` or `cancel` closes the menu with nothing opened; `?` still reaches the card index from it, as it does from every prompt. The composed screen's info strip lists the same words on every frame, every hint line points at `view`, and each surface is also a direct command (§5) — the menu row and the command run the same code, so they can never disagree. Every one of these is read-only: none of them can apply a move, and none of them autosaves.

**Leaving a view — read this before you type a number at one.** `Enter` or `cancel` closes it and *only* closes it. **Any other line closes the view and then runs at the prompt underneath** — so `score` prints the score, `back` undoes your last applied entry, and **a number applies that numbered option**, which is the one to know about, because a view is a screen you opened to read and a stray `2` at one is a move. Every view says all of this on its own last two lines. (The card index behaves differently on purpose: it is an *input* surface whose numbers select cards, so it keeps the screen up and refuses anything that is not a card — and there `back` closes it rather than undoing.)

**`?` is the card index** (also `index`, `orders`, or row 11 of the `view` menu). It is a full-screen picture of all 36 order cards, grouped by flag, each drawn as the card you are holding — country, its goods requirement, what it pays — with its engine id (`O14`) as a small grey tag in the corner. **You never have to know an id.** The index is the bridge: find the card by what you can see, type the number printed on it, done. Three things it does beyond looking things up:

- **It opens from anywhere, including mid-question.** `?` works at the main prompt, inside a Movement sub-prompt, and — where it matters most — in the middle of a live reveal, when the tool is asking which card physically refilled a Port slot. Pressing `?` there shows the index with a header naming the exact slot being recorded; typing a card's number answers the question; `?` again (or Enter, or `cancel`) closes it and puts you back on the same question, with everything you had already typed still collected.
- **It shows what is already recorded.** A card the tool can already see — sitting on a Port display, or in your own hand while it is unmasked — is drawn dimmed with a note saying where it is (`= Port A slot 1`). Type one anyway and the refusal names the same place, so "already entered" is never a mystery.
- **It filters.** `f silk` narrows it to cards needing silk, `f 8vp` to cards paying 8 VP or more, `f france` to one flag; `f` on its own clears the filter, and the keyword ignores case like every other word in the grammar (`F silk`, `filter silk`, `FILTER SILK`). Any prefix long enough to name one thing works — `f si` is silk, `f fr` is France — and one that names two does not: `f s` could be seafood or silk, so it is refused and names both rather than picking one. A term the index does not know (`f 8`) is refused as well, naming the forms that work — it never silently clears the filter you were reading. The remaining cards renumber from 1, so the number on screen is always the number to type.

**Movement: destination-first, with sub-prompts.** The raw rules can offer thousands of Movement combinations — every route to every destination, every count of assistants to recover — so the Movement list is *collapsed*. Each reachable destination is ONE numbered entry: if a single route reaches it, that entry is the move itself — `Move to <AREA> ◈ — <k> steps · <price>`, where the `◈` is the same mark the board tints on that tile and the price is what the engine will charge (a Station or Tram hop has no steps to count, so it reads `— free teleport` and prices itself in that word); if several routes reach it — say a free walk versus a longer one that pays a pass-through fee — the entry reads `Move to <AREA> — <k> route options`. The whole *retreat* family (Movement Option B) collapses to one `Retreat President to hand — choose recovery (<k> combinations; 'r' grammar works too)` entry (a lone bare retreat — no assistants on board — stays a plain numbered entry, no sub-prompt). Under the offered entries the zone may add **dimmed near-miss rows** — `(not offered: <AREA> — <reason>)` — for a tile next to your President that the engine did *not* offer, when the state says plainly why: you have no assistant there to stop on, your President started the turn there (and may not return this turn), or it is the Canal, which is a crossing rather than a stop. When the reason would be something the screen cannot read off the state — an unaffordable toll, an opponent's President in the way — there is **no row at all** rather than a guess, so a row you do see is a fact. At most three rows appear at once; when more neighbours qualify, a dimmed `(…and N more)` line says how many were left out, so the cap never passes for the whole story. Two captured Movement lists follow, each rendered from its own position on the same four-area test board — your President on the Bank, an opponent's parked on the Silk Mill.

```
TURN 1 · RED's turn   Additional ▸ Placement ▸  MOVEMENT  ▸ Area Action ▸ 5-Power ▸ Building ▸ Recovery ▸ Additional ▸ End

YOU ARE DECIDING · where your President moves — destinations need one of your assistants to stop on (◈ on the board).
   [1] Move to COPPER MINE — 2 route options
   [2] Move to TEA PLANTATION ◈ — 1 step · free
   [3] Retreat President to hand — choose recovery (24 combinations; 'r' grammar works too)
(pick a number · 'r' retreats directly · 'back' undoes the last applied entry · 'view' any surface · '?' the card index · 'help' for commands)
> 
```

Both entry kinds are in that list: the Copper Mine is reachable two ways — a direct canal crossing and a longer walk that pays a pass-through fee — so it collapses to a `route options` entry, while the Tea Plantation has exactly one route and stays a plain move. **The Silk Mill gets no row at all**, and that is the rule above working rather than a gap: it is adjacent and the engine did not offer it, but the reason is the opponent's President standing on it, which is not a fact this screen can read off the state. Take the Tea Plantation assistant away and that destination becomes a near miss — same board, same President, one row different:

```
TURN 1 · RED's turn   Additional ▸ Placement ▸  MOVEMENT  ▸ Area Action ▸ 5-Power ▸ Building ▸ Recovery ▸ Additional ▸ End

YOU ARE DECIDING · where your President moves — destinations need one of your assistants to stop on (◈ on the board).
   [1] Move to COPPER MINE — 2 route options
   [2] Retreat President to hand — choose recovery (12 combinations; 'r' grammar works too)
   (not offered: TEA PLANTATION — you have no assistant there to stop on)
(pick a number · 'r' retreats directly · 'back' undoes the last applied entry · 'view' any surface · '?' the card index · 'help' for commands)
> 
```

Selecting a `route options` entry opens a **route sub-prompt** — that destination's routes, each with its full cost/recipient text, so you pick which one:

```
Routes to Fishing Ground (A05):
[1] Move President to Fishing Ground (A05) — free
[2] Move President to Fishing Ground (A05) — pay 1 yen (1 to Blue)
> pick a number, or 'cancel'
```

Selecting the collapsed retreat entry opens a **guided retreat**: for each area holding your assistants, in ascending area id, it asks how many to bring back. Captured from a position with assistants in four areas, the first question reads:

```
recover how many from Bank (A00) [0-2]?
> enter a number, or 'cancel'
```

Answer each in turn — a number in range, or `0` to leave that area alone — and after the last one the assembled move is shown for a final `confirm`:

```
Return President to hand, recovering 5 assistants (A00 x2, A01 x1, A02 x1, A03 x1)
> confirm / cancel
```

Inside either sub-prompt the accepted inputs are a valid answer or `cancel` (which returns to the full Movement list, applying nothing); `quit`/`q` also exit the program, silently discarding the un-applied sub-prompt (nothing was recorded). An out-of-range number re-asks with its range. The **route** sub-prompt is itself a real paged list, and `more`/`list` step it exactly as they step the main options — it pages at twenty entries and words its footer in the decision zone's own vocabulary (`(5 more — 'more' for page 2 of 2, 'list' back to the first)`), so one pager reads the same on every screen that has one; everywhere else in the two sub-prompts a view/`save`/`undo` — and `more` inside the *sequential* guided retreat, which is a question flow rather than a list — is refused with `! finish or cancel this prompt first` (it would otherwise render or act against a state the half-finished prompt doesn't reflect yet). Whatever a sub-prompt assembles is matched against the real legal list before it applies — exactly like the `a`/`b`/`m`/`r` shorthands — so neither can produce a move the engine didn't offer.

**Choice prompts:** some rewards leave you a decision — which good, which warehouse item, where to move an assistant, which Church/Customs space (full inventory in §8.6). Whenever one or more of these is queued, the "YOU ARE DECIDING" line words that choice instead of the phase (the chain above still shows which phase you are in), and says how many more are stacked behind it; the numbered list below is exactly that choice's options — nothing about entering them differs from any other phase. A captured example, mid-cascade:

```
TURN 1 · RED's turn    ADDITIONAL  ▸ Placement ▸ Movement ▸ Area Action ▸ 5-Power ▸ Building ▸ Recovery ▸ Additional ▸ End

YOU ARE DECIDING · which good to take — the reward is yours whichever you pick. (1 more to resolve after this one)
   [1] Take 1 copper (any-good reward)
   [2] Take 1 silk (any-good reward)
   [3] Take 1 tea (any-good reward)
   [4] Take 1 seafood (any-good reward)
(pick a number · 'back' undoes the last applied entry · 'view' any surface · '?' the card index · 'help' for commands)
> 
```

A choice that lets you decline (warehouse take, assistant move) adds `Decline the ...` as one more numbered option; a choice that's purely "which one" (any-good, Church space, Customs space) has none, because the rulebook doesn't give you one there either. Resolving a choice can queue another — picking a Church space queues its movement-rider(s) next — and the `(N more to resolve after this one)` tail tracks how many are still stacked up.

**Placement: pickers, with a grammar bypass.** The raw rules offer every legal combination (186 options on a fresh 2-player board), so the Placement list is collapsed to three entries — `Option A — place 1 assistant each in up to 3 areas (pick areas; 'a' grammar works too)`, `Option B — place 2 assistants in one area (10 eligible; 'b' grammar works too)`, and `Place no assistants`. Selecting **Option A** opens an area-toggle picker: a compact directory of every placeable area (with a `(fee)` mark wherever an opponent President stands — on a terminal the full board is right above it), and typing an area id toggles it in or out of the selection — a mistyped id that is not a real area on this board (`A99`) is refused at that moment (`A99 is not a placement area on this board.`) and never enters the selection. After every toggle the picker echoes the exact move your selection currently assembles — with its fee text — or warns `NOT currently offered (adjust the selection)` (or names an over-limit pick as `too many areas`); `confirm` places, and if the selection is not currently legal it keeps the picker open with your selection intact so you can fix it rather than discarding your work; `cancel` backs out. Selecting **Option B** lists the eligible areas **by id**, each with its fee note — the same grammar Option A uses, and deliberately *unnumbered* so an id can never be read as a list position. **Option B does not commit on the id alone.** Typing an id only *selects* that area and previews the exact move it will place (`-> legal — ...`); **`confirm` is the step that actually places it**, and `cancel` backs out (a Telegram owner gets one more numbered pick over that area's bonus variants at `confirm`). That second step is deliberate: the picker used to place the instant you typed an id, so a mis-typed area entered the game with nothing on screen to catch it — the preview is your chance to see the wrong area *before* it is committed. Read the preview line every time; it is the only mis-entry check this flow has. The grammars do the same thing in one line — and note that a grammar line, unlike the picker, has **no preview and no `confirm` step**, so it commits immediately:

- `a` — place nothing (legal per the rules: 0–3 assistants, 0–4 with Newspaper)
- `a 4 9 12` — Option A: 1 assistant in each listed area (area ids `0`–`17` as shown on the board, up to 3 — 4 with Newspaper tech)
- `b 7` — Option B: 2 assistants in area 7
- `b 7 t 8` — Option B plus the **Telegram** tech bonus assistant in adjacent area 8

Order of ids in `a` doesn't matter; picker and grammar alike match your entry against the legal list and refuse anything the engine didn't offer.

**Assistant-move: grouped by source, with the `m` bypass** (for a pending assistant-move choice — the Movement icon's reward, §8.6). The numbered list groups by SOURCE — one `Move an assistant from <src> — N destinations` entry per source (plus the `Decline` line); picking a source opens the numbered destination list (a single-destination source stays a plain entry). Or type `m <from> <to>` directly, where `<from>` is `hand` or the id of an area holding one of your own on-board assistants, and `<to>` is any area id — or `hand`, which returns an on-board assistant to your hand, where it stays (a ruling adopted for parity with BGA's engine; the printed rule is ambiguous). Continuing the transcript above once it cascades into an assistant-move choice: `m 8 9` moves the assistant sitting in area 8 to area 9; `m hand 9` moves one from hand to area 9; `m 8 hand` brings the area-8 assistant back to your hand. There is no `m hand hand` — a hand-to-hand move would change nothing, so the engine never offers it (decline instead). Like the placement shorthand, `m` only ever matches against the real legal list — a destination the reward doesn't actually allow gets the same rejection a wrong option number would, and there's no `m` shorthand for declining (use the numbered `Decline the assistant move` option). Bad input: `m` alone or `m <one token>` → `Usage: m <from> <to>`; a non-numeric area token → `'<tok>' is not a valid area number`; a well-formed area number that isn't actually offered — including one out of the board's 0-17 range (`m 8 99`), or using `m` when no assistant-move choice is pending at all — → `Illegal: that is not one of the legal assistant moves.` Unlike `a`/`b`, `m` doesn't separately range-check its area numbers first; an out-of-range one just falls through to that same generic rejection.

**Retreat shorthand** (Movement Option B: return the President to hand, optionally recovering assistants — the rulebook's other Step-2 option alongside moving him, §8.4): `r` by itself returns the President to hand recovering no assistants; `r <id>` recovers *every* assistant you currently have on board in area `<id>`; `r <id>:<n>` recovers exactly `n` from area `<id>` (`n` must be ≥ 1). Chain several area tokens in one command: `r 4:1 9` recovers 1 assistant from area 4 *and* every assistant you have in area 9. The numbered list still works too, but the whole retreat family now collapses to the single **guided retreat** entry described under Movement grouping above — `r` is the shorthand that *bypasses* that guided flow, assembling and applying the exact retreat in one line instead of stepping through the per-area questions. Like `a`/`b`/`m`, `r` only ever matches against the real legal list — asking for more assistants than are actually there (or any other combination the rules don't offer) gets `That retreat is not in the legal list (check areas/counts).`, the same rejection a wrong option number would get. Bad input: a non-numeric area token → `Bad area token '<tok>'. Usage: r [<area>[:<n>]]...`; a zero, negative, or non-numeric count after `:` → `Bad count in '<tok>'.` An area number that simply isn't on this board (`r 99`) parses fine and falls through to the legal-list rejection above — the legal list, not the parser, is what rules it out.

**Playing against a bot.** A bot seat plays itself. When your move hands the turn over, the bot's whole turn is already played by the time the next prompt appears — you never wait for it and you never type for it — and a bot sitting on the start seat plays before the very first prompt of the game. Each of its moves is reported on one line prefixed by its seat (at most 10 lines, then `` …N more — type `log` ``), followed by ONE summary of everything that turn changed. A captured hand-over, seat 1 botted:

```
Phase: Additional actions (after) -> Additional actions (before)
Turn 1 Red -> Turn 2 Blue
Seat 1 (Blue) — End additional-action phase
Seat 1 (Blue) — Place 1 assistant in each of: Silk Mill (A02), Laboratory A (A07)
Seat 1 (Blue) — Return President to hand (recover no assistants)
Seat 1 (Blue) — End additional-action phase
Blue: assistant moved from hand to Silk Mill (A02)
Blue: assistant moved from hand to Laboratory A (A07)
Turn 2 Blue -> Turn 3 Red
TURN 3 · RED's turn    ADDITIONAL  ▸ Placement ▸ Movement ▸ Area Action ▸ 5-Power ▸ Building ▸ Recovery ▸ Additional ▸ End

YOU ARE DECIDING · whether to use an additional action before Placement, or to move on.
   [1] End additional-action phase
(only option — press Enter to apply · nothing is final until you apply it · 'back' undoes the last applied entry · 'view' any surface · '?' the card index · 'help' for commands)
> 
```

The first two lines are your own move's summary; the four `Seat 1` lines are the bot's turn; the two `Blue:` lines are that turn's single summary, and the `Turn … -> Turn …` line between them is the hand-over marker the summary brackets. Read the summary as a NET change — an assistant a bot places and then recovers in the same turn nets out of it, which is why the per-move lines above it, and `log`, are the record of what actually happened. The bot decides from the same masked view of the game a seated player gets — its own hand, the public board, and a guess at everything hidden — and never from the engine's private state; that is a property the test suite pins rather than a claim about how well it plays. It is a rollout policy, not a strong opponent: it is here so a whole game can be driven end to end without a second person, and playing to win is a later phase's job.

**The turn review, when a game OPENS on a bot.** Starting or resuming a game where a bot is the one to move is the one case that gets its own screen. On a real terminal the bot's turn is played and then shown on a full-screen review of its own — the same lines described above, headed `WHILE YOU WERE AWAY — the bot's turn` — and **one press of Enter** takes you to your board and your options, clean and complete. Nothing is waiting on you while it is up. It is a pause, not a prompt, and it draws no input caret for that reason: `quit` quits, exactly as it does everywhere else, and anything else you type there is discarded. What `log` keeps is every **move** the bot made — re-described against the state at the time, so it is the record of what happened rather than a copy of this screen (the turn summary and the hand-over line live only here). This exists because those lines are tall: composed into the same screen as the board they pushed its top rows, and the title strip with them, off a 50-row terminal — you could only get back to them by scrolling. **Mid-game is deliberately unchanged**: there the bot's reply rides the answer to your own move, exactly as captured above, and a keypress between every turn would be a toll on the ordinary case. A **piped or redirected** run gets no review and no pause at all — the narrative stays in the stream where it has always been, and the first command you send is still the first command it reads.

**Asking the bot for a move (simulation only).** `suggest` runs the built-in evaluator bot on **your own seat's view** and prints what it would play, the arithmetic behind it, and what it nearly played instead. It applies nothing, records nothing, and does not autosave — press it as often as you like, at any decision point, and the game is exactly where you left it. Pressing it twice at one position gives byte-identical output: the bot's randomness is keyed by the game's identity, your seat, and how many moves have been played, so "same position" and "same answer" are the same condition — play a move (or `undo` one) and the answer is free to change. It is deliberately **absent in live mode**, where it prints a refusal instead: a live game's hidden cards are a model reconciled from the physical table rather than the engine's own deck, so the search would be reading a position nobody is playing. A captured suggestion, mid-simulation:

```
─ Suggest ──────────────────────────────
Seat 0 (Red) · lookahead-v1 · beam 4, K=1, M=8, evaluator impl 3 · stream 0x7199df924be0e320
Suggested move:
  Place 1 assistant in each of: Port A (A00), Laboratory A (A01), Employment Agency (A07)

The line it searched, 6 move(s) to the hand-over (the move above is step 1):
  1. Place 1 assistant in each of: Port A (A00), Laboratory A (A01), Employment Agen…
  2. Place President from hand in Laboratory A (A01)
  3. Laboratory: acquire Postal System, +2 yen
  4. Skip the 5-Power token
  5. Skip the building
  6. End additional-action phase

Score at that point: 72   ·   stage 0/1000000 ppm (0 = opening, 1000000 = end trigger)
  feature                        you   best opp   weight     product
  victory_points                   0          0        1           0
  country_bonus_now                0          0       20           0
  tech_bonus_now                   5          0       14          70
  church_bonus_now                 0          0       -4           0
  customs_bonus_now                0          0        3           0
  assistants_in_hand_core          2          2       30           0
  assistants_in_hand_excess        4          6        5         -10
  assistants_in_warehouse         12         12      -10           0
  assistants_on_board              2          0        6          12
  shophouses_built                 0          0       12           0
  trading_houses_built             0          0       15           0
  five_power_tokens                0          0        8           0
  imported_goods                   0          0        5           0
  achievement_progress_sum         1          1        2           0
  mercenaries_total                0          0        3           0
  hand_size                        1          1       -1           0
  TOTAL                                                           72

Runner-ups (margin against the suggestion, so 0 means tied) — 186 candidate(s) ranked:
           0  Place 1 assistant in each of: Port A (A00), Laboratory A (A01), Tea Plantation …
           0  Place 1 assistant in each of: Port A (A00), Laboratory A (A01), Fishing Ground …
           0  Place 1 assistant in each of: Port A (A00), Laboratory A (A01), Silk Mill (A04)
  36 candidates tie at the top; which one you see is this position's own
  keyed draw, so a second press will not change it.
This is the bot's arithmetic, not advice: each row is (your value − the best
opponent's) × the weight for this stage, summed. It read ONE guess at the hidden
cards, and its weights are hand-set rather than tuned.
```

**Reading it.** The header names the search that ran — `lookahead-v1`, a 4-wide beam over ONE sampled guess at the hidden cards, scoring positions with the frozen **v2** weight table at **evaluator implementation 3** (two different numbers doing two different jobs: `v2` names the thirty-two weights the bot prices positions with — frozen 2026-08-10, after three rounds of measurement, replacing the v1 table shipped before it — while `implementation 3` is the evaluator MODULE's own revision. Neither of them contradicts the `-v1` in `lookahead-v1`, which names the SEARCH and has not changed) — and the stream key that press used, so two presses that differ can be told apart at a glance. **The line it searched** is the sequence the bot valued the move by: it plays *its own* turn out to the hand-over and scores the position **there**, which is why the score belongs to the end of that line and not to the first move alone. That line lives inside the sampled world, and if a step stops matching your real position the tool says so under the list. **The table** is that score, term by term: your value for a feature, the best opponent's value, the weight that feature carries at this stage (weights slide from an early-game set to a late-game one, and `stage` is how far along the tool thinks the game is), and their product; the products sum to `TOTAL`. **Runner-ups** are the next-best candidates with their margin against the suggestion — a `0` margin is a genuine tie, and a line under them says how many candidates tie at the top. **Long move names.** Every line of this screen is sized to the frame width §2 states for your player count, so a move name too long for it is wrapped onto an indented second line when it is the suggestion itself — the answer is never cut — and trimmed with a `…` when it is a list row, where keeping the columns aligned is worth more than the last few words. It is the tallest screen this tool prints (roughly 45 rows under the board): on a short terminal the board scrolls off above it, which costs nothing — the suggestion itself stays on screen and the board redraws on your next input — and `panel` or `screen` (§2) buys those rows back.

**It is arithmetic, not advice.** The weights are hand-set — informed by measurement since the v2 freeze, but not machine-tuned — the search is one turn deep in one sampled world, the opponents' hidden cards are guessed, and nothing here has been measured against a human player. What it gives you is a window on what the bot thinks — which is exactly what makes it useful for this testing round: a suggestion that is obviously wrong at the table is worth reporting (§9).

## 5. Commands

```
<number>        apply the numbered legal option
<Enter>         on a single-option prompt: apply that only option, or open it if it is a grouped family
a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas (4 with Newspaper)
b <id>          Placement Option B: place 2 assistants in one area
b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id; <to> is an area id, or 'hand' to return the pawn)
r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)
board           show the area grid and its legend (▲ president, ● assistants, ◈ a Movement destination, [St] Station, site-slot tokens)
players         show each player's resources, hand, tech, tokens, President
ports           open the PORT OFFER view: every slot's order card, and the power its zone takes
labs            open the LABORATORY OFFER view: every slot's technology, what it does, and what it costs
tracks          open the TRACKS view: every Church and Customs space as a card, with the 2P dummy
sites           open the BUILDING SITES view: every area's site card, its builders, and its canal tolls
hand            open YOUR HAND view: the current player's order cards, drawn in full
orders          open the ORDER CARD INDEX: all 36 cards, grouped by flag, full-screen
techs           open the TECHNOLOGY INDEX: all 20 titles by flag, each with its full purchase text
goals           open the ACHIEVEMENTS index: the cards in play by category, with claim VP and progress
sitecards       open the SITE CARD INDEX: all 24 printed cards, for setup entry
?               open the card index for whatever is being asked ('index' also works) — works mid-reveal and mid-sub-prompt, and returns you to it
score           show current VP/money (or the final scores once the game is over)
log             open the LOG view: the move and reveal trail, each line as it read when it applied
view            open the numbered game-info menu over all of the views and indexes above ('views' also works)
suggest         simulation only: what the built-in bot would play from your seat, with its arithmetic
privacy         toggle hiding hand contents in views (default: hidden in live mode)
panel           toggle the players panel inside the composed screen (terminal only)
screen          toggle the composed screen itself (terminal only; pipes are always plain)
more            show the next page of legal options (wraps to page 1)
list            reset to the first page of legal options
undo            undo the last move (and any reveals bound to it) ('back' also works)
save [path]     save to path (or to the last-used path if omitted)
reveal          live mode: pick the seat whose hidden starting order just became public from a numbered menu, then name the card
reveal <seat> <O..>  the same thing in one line, for anyone who knows both halves
cancel          back out of whatever is open: either reveal (live), the view menu, a card index, a full-screen view, a numbered pick, a Placement picker, guided retreat
help            show this text
quit            exit the REPL ('q' also works)
```

After each applied move the tool prints a **delta summary** — what changed (money, VP, goods, hand, tech, President location, phase, deck sizes, end-trigger announcements) — then the next prompt.

## 6. Live tracking: reveals

In live mode, whenever a move would draw hidden cards, the tool **does not apply it yet** — it parks the move and asks what physically appeared, on its own screen:

```
LIVE TRACKING · Port A refilled 1 slot from the order deck.

YOU ARE RECORDING · Port A slot 6 (reveal 1 of 1) — which order card is it?
(a number, 'O14', or '?' for the card index · 'cancel' drops the move — nothing is applied or recorded · 'tracks', 'sites', 'ports', 'labs', 'hand' or 'log' to look around)
> 
```

The first line says what just happened; the second names the exact slot and what kind of card answers it; the last line is the grammar and the ways out. A multi-slot refill asks once per slot, right-to-left (the physical refill order), and carries an `already recorded in this reveal — O04 · O17` line so you can see what the group holds so far.

- **Port take** → for each refilled slot, type the order card's **corner number** (`14`), its id (`O14`), or press `?` and pick it off the index. Any real, still-hidden order card is accepted — including one boxed face-down at setup (at 2–3 players most of the 36-card deck is never seen): the tool cannot know the true deck order, so it trusts the table and reconciles its hidden model to match. A truthful card is never refused for being one the tool "didn't expect"; only a card that is already face-up elsewhere, or not a real order card, is rejected — and a card that is elsewhere is refused **by saying where** (`O04 is Port A slot 1 — a card cannot be in two places.`).
- **Lab take** → the one refilled tech slot: enter a **name** (any unambiguous start of one, e.g. `postal`), `T<id>`, or `?` for the technology index. A name that matches both copies of a title resolves automatically to the copy nothing has claimed — the two copies are identical, so tracking is unaffected, and the *wizard* reads a name the same way (§3, live tail step 7). Techs are never boxed at setup, so a tech that is not in the deck is a genuine mismatch and is refused.
- **Card entry is the same grammar everywhere** (§4, *The card index*): a bare number, an id, a name where cards have names, `?` for the index, `f <term>` to open it filtered. Nothing here is a grammar you only need at a reveal.
- `cancel` at any of these aborts the move entirely (nothing is applied or recorded); `quit`/`q` also exit the program from a reveal prompt, discarding the parked (un-applied) move the same way. A **blank line** and `back` do neither — they re-ask, so pressing Enter can never throw away the move you have half-answered for.

There is **no** 5-Power token reveal: the tokens are face-up open information you enter once at setup (§3, step 4) and read off the board (§8.5), so taking one at 5 power just applies — the tool already knows the face.

While one of these prompts is showing, everything you type is read as the answer to it, not as a command — numbered picks, `m`, and view commands like `board` don't work until the reveal is answered, or you `cancel` (or `quit`/`q`, which exits and discards the parked move). What DOES work mid-reveal is what the hint line names: `?` and the index commands, and the `sites`/`tracks` views — all of which return you to the exact question with everything already collected still collected. (These prompts are unrelated to the choice banner and `m` grammar in §4/§5: reveals are about typing in what physically appeared on a hidden card, not about deciding between options the engine already knows.)

Once all reveals are typed, the move + reveals commit **as one atomic unit** — and one `undo` removes the whole unit.

**Hidden starting orders.** When an opponent's starting order becomes public at the table (they fulfill it), type `reveal` and pick the seat:

```
Whose starting order just became public?
[1] Seat 1 — Blue
> pick a number, or 'cancel'
```

...and the card question is the same screen every other reveal uses:

```
LIVE TRACKING · Seat 1's starting order is face up at the table.

YOU ARE RECORDING · Seat 1's hidden starting order — which order card is it?
(a number, 'O14', or '?' for the card index · 'cancel' backs out — nothing is recorded · 'tracks', 'sites', 'ports', 'labs', 'hand' or 'log' to look around)
> 
```

The result is `Seat 1 revealed O23 as their starting order.`, and the card then shows entering that seat's hand (any autosave failure is reported inline). Only seats whose starting order the engine still holds as hidden appear on the menu; if none do, the tool says so instead of showing an empty list; and a seat the table does not have (`reveal 3` at two players) is told so rather than answered about. A card that is not still hidden — one already face up on a board, or in a hand — is refused in the same words a Port or Lab reveal uses (`O04 is not an unrevealed order card (not in the deck or the boxed/removed pile).`), and the question stays open underneath, so a typo costs one line rather than the whole entry. `reveal <seat> O<n>` — e.g. `reveal 1 O23` — does the whole thing in one line and records exactly the same event, for anyone who knows both halves.

**Catching your own typos.** Nothing can detect that you typed the *wrong* real card — enter `O16` for a slot that physically shows `O17` and the tool faithfully records `O16`. Two things let you catch it yourself: every reveal is echoed as it applies, and the `log` view lists the exact card ids recorded for each reveal event, so you can read them back against the table. When you spot a mis-entry, `undo` removes the whole move-and-reveal group so you can re-enter it.

**Desync guard.** After a reveal applies, the tool re-reads the board and confirms the engine placed the revealed card where it said it would. Because the engine places exactly the card you typed, this is an *internal* consistency check — a tripwire for a driver/engine bug, **not** a check that your entry matched the table (that is what the echo and `log` above are for). If it ever fails you'll see:

```
CRITICAL: desync detected — the revealed card(s) do not match the board after applying the move. Session locked.
```

A locked session accepts only `save`, `log`, and `quit`, and the lockdown is **persisted**: it survives `save` → quit → reload (the reason is written into the save file and restored on load), so a desynced game can never be reopened as if nothing were wrong. **Save immediately** and report with the file (§9).

## 7. Save, load, undo

- `save` / autosave writes the full game record as JSON — seed, setup, every move and reveal. Saves are atomic (a crash mid-save can't corrupt the previous good file).
- Resume: `cargo run -p yokohama-cli -- <file>`. The game replays deterministically to exactly where you left off — unless the save predates a change to the game's rules or board semantics, which the tool refuses to resume rather than replay onto semantics the game was never played under (§2).
- `undo` works all the way back to the first move, one entry at a time (a live move takes its bound reveals with it). With a bot at the table one `undo` takes back the bot's whole reply AND your own last move, so you land back at your own decision instead of in the middle of the opponent's turn; it says how many entries went (`Undid your last move and the bot's reply (5 entries).`). If the log holds nothing but the bot's opening turn there is nothing of yours to take back, and it says so — that turn replays unchanged from the same seed. Undo still works after a reload, because who played what is worked out by replaying the record rather than remembered in memory. Page position resets after each undo.
- If `save <path>` fails (bad directory), the previous save path is kept — autosave keeps working against the old target.

## 8. LANGUAGE: how the CLI names things vs. the rulebook

The engine's canonical vocabulary differs from the rulebook's in places. **Differences are marked ⚠.**

### 8.1 Order cards

| CLI | Physical card / rulebook |
|---|---|
| `O01`–`O36` | ⚠ The rulebook has no card numbering. The number **is printed in the card's corner** — that corner index is the CLI's card identity. Find the corner number, type it. |
| `BRITAIN          O27` (the card's title line) | Its country flag, and the corner number that is its identity. |
| `2 Seafood 2 Copper` then `★1 VP ¥2` (the card's two interior blocks) | The goods it needs, then what it pays. An order is always drawn as a card — in `hand`, in the `?` index, and on the dashboard. |
| ⚠ `Fulfill O01 [America]  needs 1 tea + 2 fish  ->  1 VP + 1 warehouse, +2 VP (Ball)` | ⚠ The clause after the comma is **not printed on the order card** — it is what your own **Ball** (+2 VP) and **Language School** (+1 Yen) technologies add to *every* order you complete (§8.2). It appears on the fulfil option and in `log`, naming the card that pays, and only for technologies you actually own; own neither and the line ends at the card's printed reward. Read the whole line before you fulfil: with Ball out, a 2 VP order banks 4. |

### 8.2 Technology cards

| CLI | Physical card / rulebook |
|---|---|
| Card **names** (`Postal System`, `Winery`, …) | Match the printed card titles. |
| ⚠ `Textile Mill` | The printed card title. **The rulebook text calls this card "Spinning Mill"** — same card, one name on cardboard, another in the book. The CLI follows the cardboard. |
| ⚠ `T<id>` input form (e.g. `T12`) | Engine-internal numbering — **not printed anywhere on the physical card**, and never required: name a technology by its printed name (`postal`, `tex`, `winery`) or pick it off the `?` index. Use `T<id>` only when copying an id the CLI itself displayed. A name matching both copies of a title resolves to whichever copy is not already recorded — the two are identical cardboard — at setup and mid-game alike. |
| 20 titles, 30 physical cards | Ten titles exist as two copies each. The rules allow a player to own only one card *of the same name*; the engine enforces that. |

### 8.3 Goods and resources

Goods are written three ways, and which one you see depends on the surface. Cards, the dashboard and the full-screen views spell them out in **full words**. The rolling `players` panel (§5) uses **single letters**. Option rows use **compact tokens**, because a whole reward has to fit beside a menu number.

| Rulebook term | Full words (cards, dashboard, views) | `players` panel | Compact token (option rows) |
|---|---|---|---|
| Copper | `1 Copper` | `c1` | `2cop` |
| Silk | `1 Silk` | `s1` | `2silk` |
| Tea | `1 Tea` | `t1` | `2tea` |
| ⚠ Seafood | `1 Seafood` | `f1` | ⚠ `3fish` |
| Imported Goods | `0 Imports` | `imports 0` — spelled out, **no letter** | not abbreviated (no space pays imports) |
| Yen (money) | `¥3` | `3 yen` — spelled out, **no letter** | `1yen` |
| Victory Points | `★4 VP` | `0 VP` | `2vp` |

So the `players` panel's goods letters are exactly `c s t f`: imports and yen are spelled out there, and neither has a letter of its own.

> ⚠ **"fish" vs "Seafood".** The good is **Seafood** on the cardboard, and every surface this redesign draws says so. Two strings still read `fish`, because the engine builds them rather than the display layer: the order line `needs 2 copper + 1 fish` (seen on Fulfil options, in `log`, in `suggest`, and on the wizard's dealt cards) and the compact space reward inside `Build a Shophouse (space 4: 3fish)`. Typing `fish` is still accepted wherever a good is named, and so are `seafood` and `f`.

The three icon rewards are each a real choice prompt — see §8.6:

| Compact token | Full words on a card | What it is |
|---|---|---|
| ⚠ `2wh` | `2 warehouse takes` | The **Warehouse icon**: take 1 token from your Warehouse tile to hand, paying the Employment Agency cost. |
| ⚠ `1any` | `1 any good` | An "any good of your choice" reward. |
| ⚠ `1move` | `1 assistant move` | The **Movement icon**: move an assistant (`m <from> <to>`, §4/§5). |

The board's site-slot strip uses a fourth and tighter set again (`★n ¥n fn tn sn cn *n an ⌂n Tn`); its legend sits under the board and §8.5 keys it.

### 8.4 Countries and turn phases

Countries: `GB`=Britain, `US`=America, `FR`=France, `DE`=Germany, `NL`=Netherlands (flags on the order cards).

Turn phases as the CLI names them, against the rulebook's turn structure:

| CLI phase | Rulebook |
|---|---|
| `Additional actions (before)` / `Additional actions (after)` | ⚠ Not a rulebook step name — the windows in which an Additional Action (fulfilling orders, mercenary use, achievement claims) may be taken before/after the main turn. |
| `Placement` | Step 1 — place assistants (Option A / Option B). |
| `Movement` | Step 2 — move the President (Option A / Option B). |
| `Area action` | Step 3 — carry out the action of the President's area. |
| ⚠ `5-Power bonus` | Steps 4–5 (both optional, both at 5 power): take the area's 5-Power token / claim achievements. |
| `Building` | The optional build (Shophouse or Trading House, 4+ power). |
| `Recovery` | Step 6 — automatic; the CLI performs it without asking. |
| `Turn end` | Bookkeeping between turns. |

> ⚠ **The "skip" option:** in the six "may"-worded areas (Port, Laboratory, Employment Agency, Chinatown, Church, Customs) the CLI offers an explicit option reading **`Take the area action, declining all optional parts`**. The rulebook has no named "skip" — it's the engine's encoding of "you may". Two other options do say the word (`Skip the 5-Power token`, `Skip the building`), so look for the sentence rather than the word on an area list. Note v1's conservative reading: declining at a 5-power Port forgoes the +3 VP bonus.

### 8.5 Areas, boards, and tracks

The board draws one bordered tile box per area, positioned as the physical staggered pyramid (widest row on top, each row below offset half a tile). An ordinary tile and the Canal, captured from a real board, with the legend that sits under them:

```
┌ A03 COPPER MINE ▲ [St] ────┐
│ 5-Power: 1 Import Site 2   │
│ ●●  ●●●●+2                 │
│ sites ★1 ⌂ — t2  │ T7      │
└────────────────────────────┘
┌ A10 CANAL ─────────────────┐
│ transit only — no pawns,   │
│ no tokens, no buildings    │
│ ¥1 to the bank / crossing  │
└────────────────────────────┘
marks: ▲ president (owner color) · ● assistants (4, then +n) · ◈ a Movement destination the engine offers · [St] Station (+1 Power for everyone)
site slots: ⌂ built (owner color) · ★n VP · ¥n yen · f/t/s/cn goods · *n any good · an assistants · ⌂n build · Tn trading-house VP — 'sites' renders the full cards
Canal: the President may cross shore to shore for ¥1 to the bank ('sites' lists the pairs)
```

The **title** carries the area id, the area's FULL name in capitals, and then every mark standing on that tile: `▲` for a President in its owner's colour, `[St]` where the Station token sits, and `◈` during Movement on every tile the engine offers you as a destination (§4). One sweep along the top row answers "where is everybody" — which is what the marks are on the title for rather than on a line of their own.

The **5-Power line** names the face-up token in words and colour (`1 Import`, `3 Seafood`, `2 Copper`, `3 Yen`) and then the building-site card sitting on the area (`Site 2` — the number setup entry takes; nothing is printed on the physical card). Once somebody has collected the token the word reads `taken`. The tokens are face-up open information you enter at setup, never a reveal (§8.6).

The **pawn line** draws each seat's assistants as a run of `●` in that seat's colour, up to four, then `+n` for the rest — so `●●  ●●●●+2` is two of one colour and six of another. An area nobody is standing on says `no pawns` rather than drawing an empty line. On a genuinely crowded tile, where four seats' runs would not fit the box, the line falls back to the compact letter-and-count form (`R12 B3`): a pawn the board cannot draw is a pawn you would not know about, so the tile tells the whole truth in less space instead of clipping.

The **site-slot strip** is the area's building site in miniature: its four Shophouse spaces in printed order, then `│` and the trading house's VP. A *built* space shows `⌂` in its builder's colour; an *empty* one shows what it still pays, in the compact tokens the legend keys (`★n` VP, `¥n` yen, `f/t/s/c` plus a count for seafood/tea/silk/copper, `*n` any good, `an` assistant moves, `⌂n` warehouse takes); a space with nothing printed on it is an em dash. The trading-house `Tn` is tinted in its owner's colour once somebody has built it. This strip is a summary — `sites` (below) draws the same site as a full card with every reward in words.

The **Canal** gets none of that, because nothing can ever stand, sit or be built there: its box says so in words and prints the crossing toll. 2-player games have no Canal tile at all. The legend under the board keys every mark and every site-slot token, and adds the Canal line whenever this board has one.

**Naming a tile to the wizard** (§3) takes the full name, any unambiguous start of it, or the menu number beside it. The old two-letter codes still work as aliases: `cm` Copper Mine, `sm` Silk Mill, `tp` Tea Plantation, `fg` Fishing Ground, `bank`, `pa`/`pb` Port A/B, `la`/`lb` Laboratory A/B, `ea` Employment Agency, `ct` Chinatown, `ch` Church, `cu` Customs, `canal`.

Building sites (`sites` view): a full-screen surface of **one card per area holding a site** — the Movement companion, since what is still unbuilt on an area is often the reason to pay a canal toll to reach it. Each card names its site-card number (`SITE 7`, the number setup entry takes — nothing is printed on the physical card) and the area it sits on, then its four Shophouse spaces in **full words**, one per line, in printed order (top-left, top-right, mid-left, mid-right; `blank` where nothing is printed; the icon rewards from §8.3 read as `1 assistant move` / `2 warehouse takes` / `2 any goods`, and are real choice prompts when taken). Beside each space: `open`, or `⌂` and the **builder's name**. Then the trading house with its VP and its owner, and last the area's own id plus any **canal tolls** from it (`A03 · canal toll to A06`) — which tiles simply touch is legible from the board itself, but a tolled crossing is not, so each card names its own. Building options during the Building phase print this same printed-reward text instead of a bare space number: you get one `Build a Shophouse` option per **empty** space — you choose the space — each named with its space number and printed reward, e.g. `Build a Shophouse (space 2: 2vp+1yen)`, so two blank spaces stay tellable apart; `Build a Trading House (7 VP)` shows the site's TH value.

Tracks (`tracks` view): a full-screen surface with **every space as a card**. Church faith 4–11, each card showing its printed VP and its movement rider (`★3 VP`, `+1 assistant moves`); Customs spaces 1–7, each showing the printed cost you must pay in imported goods (`pay 2 imports`, printed costs `[1,1,2,2,3,3,4]`), its VP, and its warehouse/yen rider (spaces 5–7 carry no rider). Under each card sits its occupant: `empty`, or the player's name in their colour. In 2-player games a neutral **dummy** holds Church faith-7 and Customs space-4, drawn as `D · neutral dummy`; it occupies the space and **competes** in that track's majority scoring, but it is **not** counted towards the end-game trigger — so each section rule states the fill as `n of 4 REAL assistants placed`. (The same dummy also competes in the **Technology majority** at final scoring, at an industry value of exactly **7.5** — that competition has no pawn, so no view shows it; §3's dummy note carries the details.) Below the two tracks a line reports where the **Station** token sits (`Station: A03 — all players read +1 Power there`, or `not placed`) and the public per-country **mercenary supply** (`GB`/`US`/`FR`/`DE`/`NL`, §8.4's codes).

Port offer (`ports` view): both Port boards, **every slot as the order card physically on it**, with a label above each card giving the slot number and its zone. A Port take is gated by your action power — LOW needs power 2, MID needs 3, HIGH needs 4 — so the section rule over each board states that gate, and a card in a zone you cannot reach is not an offer to you this turn. Empty slots draw as holes rather than closing up, because a take slides the remaining cards LEFT and then refills from the RIGHT, so which slot a card sits in is information. The shared order deck's remaining count sits under the boards.

Laboratory offer (`labs` view): both Laboratory boards, every slot's technology as a card carrying its name, its country, its **whole effect text in full words** (never clipped — the qualifiers are what change a purchase decision), and the **industry the card costs in that slot**: the printed card cost plus the slot's own +0/+1/+2 surcharge, already added, with the two components shown beside the total. Industry comes from your action power, and each further point can be bought with 2 yen or 1 imported good — so a card one point out of reach costs two yen more, not a turn. The tech deck's remaining count sits under the boards. This is the **offer**; the `techs` index (§4) is the *identification* surface over all twenty titles.

Your hand (`hand` view): the current player's order cards drawn full size, the kept starting order marked `your starting order`. In live mode, or for a bot seat, it shows the count and says why the contents are hidden rather than showing nothing (§2, *Privacy*).

Log (`log` view): the move and reveal trail with each entry's global index — the number `undo` counts in and a bug report quotes — and each line worded **as it read at the moment it applied**, so an old trading-house take still quotes the price it actually paid and an old movement fee still names the player who actually received it. **A move names the player who made it, by colour** (`Red: End additional-action phase`), which is what tells two identically-worded turns apart; a reveal is a recording rather than somebody's action and carries no such prefix. The view carries the last 40 entries and says how many earlier ones it left out; the whole record is in the save file. (A locked session shows a shorter postmortem block of the last 10 instead — §6.)

**Leaving a view — read this before you type a number at one.** `Enter` or `cancel` closes the view and *only* closes it: you land back at the prompt you left, exactly as you left it, with nothing entered there and nothing there cancelled. **Any other line closes the view and then runs at that prompt underneath** — so `score` prints the score, `back` undoes your last applied entry, and **a number applies that numbered option**, which is the one to know about, because a view is a screen you opened to read and a stray `2` at one is a move. **A view opened over a live reveal says something different, because a different thing is true there** (§6): the question underneath is asking which card is physically in a slot, so a number typed at the view names that CARD and records it — and, on the answer that completes the group, applies the move parked behind it — while `back` undoes nothing until the reveal is answered or dropped. Every view says whichever of the two applies on its own last two lines, so the screen in front of you is the authority. (The `?` card index behaves differently on purpose: it is an input surface whose numbers select cards, so it keeps the screen up and refuses anything that is not a card — and there `back` closes it rather than undoing.)

**Canal crossing:** the Canal tile can never hold an assistant, a President, or a building — on the board it draws as water — but President movement can cross straight through it in a single hop, shore to shore: ANY tile touching the Canal connects to ANY other tile touching it. Two shore tiles that already adjoin each other directly keep their ordinary free edge (the free step wins; no toll appears there) — the tolled crossings are exactly the pairs the Canal genuinely bridges, and the `sites` view names them per area as `canal toll to A06`. Each crossing costs 1 Yen to the bank — a separate, unconditional toll from the opponent-president pass-through fee (which instead goes to that president's owner) — and that includes the very first step out of the President's current area, if that step is itself a crossing. 2-player games have no Canal tile at all. Telegram's adjacent-area placement bonus (§4) does not extend across a canal crossing — it still needs a physically adjoining tile.

### 8.6 Choice prompts — how reward decisions surface

Every reward the rulebook lets you decide is a real prompt now (§4's "YOU ARE DECIDING" line words it, and says how many more are queued behind it), not an automatic pick. What queues, and how each resolves:

1. **Any-good reward** (`any` in the compact reward text, §8.3): exactly 4 options — 1 copper, 1 silk, 1 tea, or 1 seafood. No decline; only which good is optional, not the gain itself. Two any-good icons on the same reward queue and resolve separately, so you can pick a different good for each.
2. **Warehouse-icon reward** (`wh`): up to 3 options — an assistant (free), a shophouse (2 yen), or a trading house (the printed 4/5/6/7 take price, whichever is next) — filtered to what's actually in your warehouse and what you can currently afford, plus an always-available `Decline the warehouse take`. The banner prices them live, e.g. `warehouse take (assistant free / shophouse ¥2 / trading house ¥4)` once you can afford all three — the start player's own 3 starting yen is one short of that pictured 4¥ TH option. When nothing in your warehouse is available or affordable, the icon auto-declines silently — no prompt, no banner — same as a table player who has nothing to take either.
3. **Movement-icon reward** (`move`): move one of your own assistants from hand or an on-board area to any other (non-Canal) area, or bring an on-board one back to your hand (BGA-parity ruling; it simply stays there) — resolve via the numbered list or the `m <from> <to>` shorthand (§4/§5) — or `Decline the assistant move`. One prompt per Movement icon; multiple icons on the same reward resolve independently, same as multiple any-good icons. When you have no assistant anywhere the icon could draw from — nothing in hand, nothing on the board — it auto-declines silently the same way: no prompt appears, same as a table player with nothing to move either.
4. **Church and Customs space choice**: after paying, if 2 or more spaces are legally open (Church: at your computed total faith; Customs: at the matching printed-cost pair), you pick which one — numbered by faith/space number and its printed VP and rider, in the compact option tokens of §8.3 (`3vp 1move`, `5vp 2wh`). The `tracks` view draws those same two engine numbers as a card instead, in full words (`★3 VP`, `+1 assistant moves`, §8.5) — an option row and a card are different budgets, so the renderings differ on purpose while the source does not. (The Church *payment* options themselves already preview the faith each would produce — §4's previews — so you know what you're buying before you pay.) When only ONE space is open, the tool places you there automatically without a prompt — same as a table player who has no real choice either. A Church placement then queues its space's movement rider(s) as their own assistant-move choice(s), so a single Church visit can cascade through several stacked prompts (the deciding line's `(N more to resolve after this one)` tail tracks how many are left); a Customs space-1/space-3 placement similarly queues its warehouse-take rider(s).
5. **Telephone's Recovery choice**: owning Telephone with ≥1 of your own assistants in the just-finished action's area pauses Recovery and asks whether to leave exactly 1 assistant there (`Leave 1 assistant behind (Telephone)`) or recover everyone as usual (`Recover all assistants from the action area`). Without Telephone — or with nothing there to leave — Recovery stays fully automatic, same as always.
6. **Station placement** (after taking the Station technology): you choose which area the Station token goes in — every non-Canal area is eligible, so on a full board this is a long numbered list. There is NO decline option: the token is going somewhere. Once placed, `tracks` (§5) shows where it sits, and **every** player — not just its owner — gets +1 Power when acting in that area, so it is worth reconciling against the physical board. If only one area were ever eligible the tool would place it inline without asking; that can't happen on a real board, but it makes Station the one choice whose automatic resolution *does* something rather than nothing.
7. **University and Textile Mill ("Spinning Mill")**: no longer queued choices — they're standing options offered directly in both Additional-Action windows (§8.4), like any other move (`Use University: pay 1 yen, take an assistant from the warehouse`; `Use Textile Mill: discard 1 silk, gain 2 yen`), each usable once per turn.

None of this touches the 5-Power tokens: they are face-up open information you enter once at setup (§3, step 4) and read off the board (§8.5), not a play-time choice or reveal. (The physical box check found the 20 tokens are only Import/Copper/Silk/Seafood/Tea/Yen faces — there's no "Movement" token in the real game, so the **setup token letters** stay exactly `i/c/s/t/f/y`. Those letters are the wizard's vocabulary, not the board's: a tile spells its face out in words, `5-Power: 1 Import`.)

## 9. Reporting problems

Every save file is a complete reproduction: seed + setup + every move/reveal. When something looks wrong:

1. `save` (if the session locked itself, `save` still works),
2. note what the table said should happen vs. what the tool did (`log` shows the recent entries),
3. send the JSON file + the seed + your note.

A `CRITICAL: desync` lockdown or an `INVARIANT VIOLATION` message is always report-worthy. So is any legal table move the tool refused, any illegal move it offered, and any place its wording confused you — the display language is part of what's under test.

*Happy testing. The engine has survived 15,000 random self-play games; you are here to be meaner than random.*
