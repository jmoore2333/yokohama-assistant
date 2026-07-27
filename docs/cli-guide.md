# Yokohama CLI — Tester's Guide

*For the game frontend (`yokohama-cli`), 2026-07-21. Audience: playtesters tracking or simulating games of Yokohama (Hisashi Hayashi, 2016). You need the physical rules in your head; this guide covers how the tool speaks about them — and **flags every place its language or behavior differs from the rulebook** (marked ⚠).*

---

## 1. What this is

A terminal program backed by the full Yokohama rules engine. Two modes:

- **Simulation** — the program deals everything from a seed and you play moves for every seat. Good for learning the tool.
- **Live tracking** — you play a *physical* game at the table and enter each move; the engine validates it, tracks all state, and asks you to type in cards as they're physically revealed (port refills, tech refills, hidden starting orders). The face-up 5-Power tokens are entered once at setup (§3), not revealed in play.

The engine only ever offers **legal** moves. If the tool won't offer a move you believe is legal at the table, that is exactly the kind of bug this testing round exists to find — save the game and report it (§9).

> **Testing-round note:** this build is the v1-beta frontend redesign — a composed game screen on a real terminal (title banner, the true staggered-pyramid board, a live players panel), guided pickers for Placement and assistant moves, fee/faith/cost previews on the options themselves, bare-Enter on forced single-option prompts, and a privacy toggle for passing the laptop around a live table (§2, §4). Every reward decision the rulebook gives you is a real prompt (§8.6), the setup wizard re-asks indefinitely on bad input instead of aborting (§3), and the Canal can be crossed (§8.5). Live-tracking a full physical game start to finish is the point of this round — save and report anything that looks wrong (§9).

## 2. Running it

**Prerequisites.** The Rust toolchain (install it via rustup), a clone of this repository, and a shell sitting in the repo root. Plain `cargo` works on the project machine.

```bash
# from the repo root
cargo run -p yokohama-cli              # start the setup wizard
cargo run -p yokohama-cli -- save.json # resume a saved game
```

**First run.** `cargo run -p yokohama-cli` with no arguments opens the setup wizard, which asks — one question at a time — for the mode, player count, per-seat colors, start seat, seed, and save path, then a mode-specific tail; §3 walks through every question. Fastest way to try it: run the command and answer `sim`, `2`, `red`, `blue`, `0`, then press Enter twice to take a random seed and the default save file, keep either dealt card with `0` and `0`, and `confirm` — you are now driving a 2-player simulation from the prompt in §4.

**Resuming.** `cargo run -p yokohama-cli -- <save.json>` replays a saved game deterministically to exactly where you left off. The game autosaves after **every** applied move once a save path is set, and the wizard sets one by default — `yokohama-game-<seed>.json` in the current directory unless you gave a path — so a crash loses at most the move you were typing; just rerun with the save file.

**The composed screen (terminal only).** On a real terminal the tool is a full game screen, redrawn after every input: a title strip, the board (one bordered tile box per area, laid out as the physical staggered pyramid — widest row on top, each row below offset half a tile), a live players panel (current player marked `>`, then each player's seat number — the digit the `reveal <seat>` command uses — yen/VP, goods, assistant zones, built counts, President location) — a block per player beside the board at 2-3 players, and one line per player below it at 4, where vertical room is scarcer and the next trading-house take price gives way (the `players` view always has it), the board legend, a game-info strip naming every view, then the message zone (what the last move changed) and the prompt. `panel` hides/shows the players box; `screen` turns the whole composed mode off and back on (plain rolling text, same content). Piped or redirected output is always plain rolling text with zero escape codes — the composed screen and all color exist only on a terminal.

**Color and width.** The tool colors what matters — player names and their pieces in their colors, tile names by area type, goods letters in resource tones, VP, warnings, the phase line — and setting the standard `NO_COLOR` environment variable turns color off in a terminal too (the composed screen stays; `screen` turns that off). The composed screen is wide, and wider than the board alone: the board grid is 72 columns at 2 players, 90 at 3, and 108 at 4, but the players panel sits beside it at 2-3 players and the game-info strip runs 96 columns on its own, so the whole frame starts at 114 columns at 2 players, 132 at 3, and 108 at 4 — and creeps a couple of columns wider as scores and area names get longer. There is no terminal-size detection, so size the window before you start: **140 columns** clears every layout's width — a test now pins every line the driver *lays out* (board, panel, option lists, prompts and hints, the choice banner, a lockdown notice) at or under 140 columns, so none of them wraps mid-token; the one thing that can still wrap is a message quoting something you typed, such as an autosave warning naming a long save path — and the screen opens 32 rows at 2 players, 33 at 3, and 38 at 4 — so a **140x40** window shows the whole opening screen at any player count, and 50 rows leaves comfortable room under it. What eats the extra rows is the part below the board: a long option list (20 per page — `more` pages them) or a big message zone pushes the title and the top board row off the top. `panel` (hide the players box) and `screen` (plain rolling text) buy those rows back. The `board` command's one-time reminder covers only the board grid's own width (and only when that grid is wider than 100 columns), so it will not warn you at 2 players.

**Privacy at a live table.** The one genuinely hidden thing in Yokohama is a starting order, and it sits in a hand — so in **live** mode hand contents start hidden: `hand` and `players` show counts (`1 card (hidden — 'privacy' to toggle)`) instead of card text, and the players panel never shows hand contents at all. Toggle with `privacy` (sim mode starts open). Everything public at the table — the board, tracks, buildings, port displays, deltas of face-up takes — never masks. **Bot seats mask on their own.** With a bot at the table (§3) that seat's hand stays hidden whatever `privacy` says — a bot's hand holds its unfulfilled starting order, and sim mode starts open — while your own hand keeps following the toggle; the bot's seat is marked `[bot]` in the players panel. That mask is table manners, not a sealed envelope: you set the game up, so you saw every seat's dealt cards in the wizard, and `log` still replays everything that happened.

**If something looks off:**

- **Board rows shear or wrap** — the terminal is narrower than the composed screen; widen it to at least the frame width above for your player count (140 columns clears every layout).
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

**Sim tail:** for each seat the two dealt order cards are shown, numbered `[0]`/`[1]` and each carrying a `(bot Keep score N)` — what the built-in bot (§4) would prefer, read off the card's printed economics alone. It is hand-set and unmeasured, and a note under the pair says so; the choice is yours. Answer `keep 0/1` with `0` or `1`.

**Bot seats (sim only):** the sim tail's last question is `Bot seats?`. **Blank** — just Enter — means none, and the game is exactly the all-human one described everywhere else. Otherwise type the seat numbers a bot should play, space-separated: `1`, or `0 2`. At least one seat must stay human (an all-bot table is refused and the question re-asks), and there is no live-mode equivalent — a bot cannot read a face-down card off your table. Every bot plays the engine's `family-weighted` policy; the wizard draws a full-width stream key for them and echoes it as `bot seed: N`, so write that down beside the game seed if you want the bot's line to be reproducible. The setup review names them (`bots: seat 1 playing family-weighted (seed N)`), `edit bots` re-asks the question, and the answer travels in the save file — a resumed game comes back with the same seats botted.

**Live tail** (you're describing the physical table):

1. Per seat: `known` (your own seat — then type its two dealt cards and the keep choice) or `hidden` (opponents whose starting order you can't see).
2. **Tiles**: for each board cell, the area tile physically there, by abbreviation: `cm sm tp fg bank pa pb la lb ea ct ch cu canal` (§8.5 maps them). Checked live against the legal per-type tile count for this player count — e.g. a second `cm` at 2 players (which only ever has one Copper Mine) is refused on the spot with `no cm tiles remaining for this board`, not after the rest of the tail. **Read the orientation block the wizard prints first — see "Which tile is cell 0?" below.**
3. **Building-site cards**: per non-Canal cell, the site card's number `S<n>` or bare `n` (printed index on the card) — checked against the registry and against every other site index already entered. Same cells in the same order as the tiles, and each question names the tile you entered there (`Cell 3/14 (row 1 of 4, tile 4 of 5) Fishing Ground -- building-site index?`), so you always know which physical tile you are reading a card off.
4. **5-Power tokens**: the face-up token on each non-Canal cell, by letter — `i` = 1 imported good, `c` = 2 copper, `s` = 2 silk, `t` = 3 tea, `f` = 3 seafood, `y` = 3 yen. Rulebook p.3 step 5 places these **face-up**, so you enter what you physically see. Same cells in the same order as the tiles, each question naming the tile you entered there, and Canal cells carry no token and are skipped. Checked live against the 20-token pool (10 import, 2 each of the rest), so a third copper — or any face beyond the pool — is refused on the spot with `no more 2 copper tokens available for this board`.
5. **Achievements**: a numbered menu of all 12 is printed; type the shown name for each of the 3 in play (a duplicate is refused immediately).
6. **Port contents**: the 6 order cards on Port A left→right as `O<n>`; Port B is only asked at 3-4 players. Each card is checked against the order registry and against every order id already entered anywhere — both seats' deals, Port A, and Port B share one duplicate check.
7. **Lab contents**: the 6 tech cards on Laboratory A left→right, by `T<id>` or a **name prefix** (prefer names — see §8.2); Lab B only at 4 players. Checked the same way as orders, with Lab A and Lab B sharing one duplicate check. ⚠ **A name prefix matching more than one card still in play is REJECTED here** (`'<name>' matches more than one tech card -- use T<id>`) — type the exact `T<id>` instead. This is deliberately different from a live-mode reveal prompt (§6), which resolves the same kind of ambiguous prefix automatically to the lowest matching id: the wizard is recording which *physical* card sits where, so a genuine ambiguity has to be settled by you, not guessed.

Every card/tile/site answer above is validated the moment you type it — a bad entry re-asks that same question instead of surfacing at the end of the tail.

**Which tile is cell 0?** This is the one setup answer nothing downstream can catch for you, so the wizard spends real screen on it. Before the first tile question it draws a numbered skeleton of the actual board shape for your player count — the real row lengths, brick-offset, widest row on top, each cell showing the number it will ask for — and states the orientation in words. The rules, in short:

- The board is a staggered pyramid of four rows, **widest row first**: 4/3/2/1 tiles at 2 players, 5/4/3/2 at 3, 6/5/4/3 at 4.
- Turn the assembled board so the **widest row is farthest from you**. **Cell 0 is the leftmost tile of that widest far row**; numbering runs left to right along a row, then down to the next row, the way you read a page. Every prompt repeats where you are — `Cell 7/14 (row 2 of 4, tile 3 of 4)` — and each row is announced as entry reaches it, so the picture scrolling away costs you nothing.
- Read every row in the **same direction**. Which end you start from does not matter — reading all four rows from the other end mirrors the board, and a consistently mirrored board is the same board to the engine — but switching direction partway through is not, and neither is changing the row order.
- ⚠ **The rulebook's p.3 diagram prints this pyramid the other way up**, narrowest row at the top. Do not follow that picture's reading order: enter the widest row first regardless of which way your copy of the diagram faces. The engine models the board's adjacency **graph**, not the printed picture — which is also why the mirror above is harmless.

Why it matters: the engine checks that your tiles are the right *set* for the player count, but it cannot check the *order* — every arrangement is a legal board. Get the order wrong and movement, placement fees, Telegram bonuses and canal tolls all run on a board that isn't the one on your table, silently, for the whole game. Hence the numbered skeleton, the coordinates on every prompt, and the board check below.

**Review before you commit:** once every question is answered, the tool prints a full summary of everything entered so far (`--- Setup review ---`: mode, players, colors, start seat, seed, save path, plus the mode-specific tail) and asks `confirm / edit <section> / abort?`. `edit <section>` re-asks that section's questions from scratch — with the same live validation — then reprints the updated summary; repeat as many times as you like before confirming. In the live tail the tile lines echo the same coordinates the questions used (`cell 7 (row 2 of 4, tile 3 of 4): Port A S6`). Editable sections:

- **live**: `colors`, `seed`, `save`, `deal <seat>`, `tiles`, `sites`, `achievements`, `porta`, `portb` (3-4p only), `laba`, `labb` (4p only)
- **sim**: `colors`, `seed`, `save`, `deal <seat>`, `bots`

In the live tail, `edit tiles` re-asks the **building-site cards** in the same pass: the site cards sit on the tiles, so a re-entered board moves them too, and re-asking the tiles alone would silently re-attach the old site cards to the new tile order.

**The board check (live only):** `confirm` no longer starts the game — it builds the game and draws the finished board back to you, with the same renderer play itself uses, and asks `Does this match your physical board? (yes/no)`. Compare the **shape** first (widest row on top), then the tile in every cell; `A00` is cell 0, the leftmost tile of the widest top row. `yes` starts the game. `no` returns you to the setup review, where `edit tiles` re-enters the board — nothing is lost, and you can go round as many times as you need. This is the last point at which a mis-ordered board can be caught, so give it the ten seconds. (Anything that isn't `yes` or `no` re-asks; `abort` still works here, as at every other question.) The **sim** tail has no board check: nothing was entered, so there is nothing to compare.

**The two-player dummy assistants (live, 2 players only):** the board check signs off with a setup instruction — before the first move, put two neutral assistants on the physical board, one on Church faith-7 and one on Customs space-4, in a player color nobody is using. They are **not** printed board art, and nothing you typed into the wizard describes them: the engine places both itself the moment the game is built, so only your table can be missing them. At two players the board check's own question names them — a `yes` there confirms the pawns are physically placed, not just that the tiles match. Each one blocks its space against both players, counts toward the occupied spaces that end the game on its track (so your **4th** assistant there is the last one), and competes in that track's majority bonus at final scoring. Skip them and all three diverge from what the players see, silently, for the rest of the game — the tool refuses a placement they think is legal, ends the game a pawn sooner than they expect, and scores both majorities against a rival that isn't on their board. Once play starts, `tracks` shows them as `D` (§8.5). At 3 and 4 players there is no dummy and no instruction, and **sim** never shows it either, having no physical board to put anything on.

`mode`, `players`, and the start seat aren't editable sections — they decide the shape of everything asked after them (seat count, grid size, whether Port B/Lab B even exist), so changing one of those means aborting and rerunning the wizard from scratch. Editing `seed` in the **sim** tail re-derives the deal preview from the new seed and re-asks every seat's keep choice against it, so a keep answer never silently survives against a preview you no longer see; in the **live** tail, `seed` is only the save file's identity (plus a throwaway check the wizard uses to validate your tile answers, which doesn't actually depend on which seed you pick for a given player count), so editing it there doesn't re-ask anything else.

## 4. Playing: the prompt and entering moves

Every screen ends with a status header, the numbered legal options, and a hint line:

```
Turn 1 | Red | Additional actions (before) | 3 yen 0 VP | c0 s0 t0 f0 i0 | hand 1
[0] End additional-action phase
> enter a number, or a command ('view' for game info, 'help' for the list)
```

Header anatomy: turn number | whose turn | phase (§8.4) | money and VP | goods `c`opper `s`ilk `t`ea `f`ish(=Seafood) `i`mports | order cards in hand.

The hint line's own middle clause is phase/choice-aware — it only ever names a shorthand that's actually usable right now: `'a'/'b' for placement` appears in `Placement`, `'r' to retreat (the numbered retreat entry is guided)` in `Movement`, `'m <from> <to>' to resolve (or the numbered Decline)` while an assistant-move choice is queued, and nothing extra (just the plain form above) everywhere else, including every OTHER kind of pending choice.

**Choosing a move:** type the option's number. Long lists show 20 per page — `more` pages forward (wrapping), `list` returns to page 0; whenever there's more than one page, a `page N/M` line appears right above the hint so you always know where you are and how many pages there are in total.

**Forced moves take bare Enter.** When exactly one option is displayed, the prompt adds `(only option — press Enter to apply)` and an empty line applies it — recorded in the log and save exactly like typing its number. When that lone entry is a grouped family instead (a Route, the Retreat family, an Option A or B placement), the line reads `press Enter to open` and Enter opens that picker without committing anything. Turn ceremony (the End-additional-actions, Skip-5-Power, Skip-building no-ops of an ordinary turn) becomes four taps of Enter. On a multi-option prompt an empty line still does nothing.

**Previews on the options.** Decisions carry their arithmetic inline, computed by the same engine code that will charge you: placement options show the fee and who receives it (`— fee 2 yen (2 to Blue)`); Employment Agency composites show their total (`— cost 6 yen`, from the 2-yen shophouse price and the printed 4/5/6/7 trading-house progression) or `(free)`; Church payment options show the faith they produce (`-> faith 6 (power 3 + 3 kinds)`); production options show the yield at your current power AND the next threshold (`Produce copper (power 2 -> +0 copper, next +1 at power 3)`), so a zero-yield line reads as "not yet", not as a bug; the Bank previews its yield the same way (`Take money at the Bank (power 3 -> +3 yen)`); and the Customs payment options preview the space(s) they land on with printed VP (`Customs: pay 1 imported good -> space 1 (+5 VP)`).

**Finding information.** Everything the physical game shows you is one menu away: `view` (also `views` or `?`) opens a numbered game-info menu — board, players, port offer, lab offer, tracks, building sites, goals, hand, score, log — and picking a number renders that surface and returns you to the prompt. The composed screen's info strip lists the same names on every frame, every hint line points at `view`, and each surface is also a direct command (§5) — the menu and the commands render through the same code, so they can never disagree.

**Movement: destination-first, with sub-prompts.** The raw rules can offer thousands of Movement combinations — every route to every destination, every count of assistants to recover — so the Movement list is *collapsed*. Each reachable destination is ONE numbered entry: if a single route reaches it, that entry is the move itself (cost and who-gets-paid text unchanged); if several routes reach it — say a free walk versus a longer one that pays a pass-through fee — the entry reads `Move President to <Area> (Ann) — <k> route options`. The whole *retreat* family (Movement Option B) collapses to one `Retreat President to hand — choose recovery (<k> combinations; 'r' grammar works too)` entry (a lone bare retreat — no assistants on board — stays a plain numbered entry, no sub-prompt). A captured Movement list with both kinds:

```
Turn 3 | Red | Movement | 1 yen 3 VP | c0 s1 t1 f1 i0 | hand 1
[0] Move President to Fishing Ground (A05) — 2 route options
[1] Move President to Customs (A06) — 2 route options
[2] Retreat President to hand — choose recovery (8 combinations; 'r' grammar works too)
> enter a number, 'r' to retreat (the numbered retreat entry is guided), or a command ('view' for game info, 'help' for the list)
```

Selecting a `route options` entry opens a **route sub-prompt** — that destination's routes, each with its full cost/recipient text, so you pick which one:

```
Routes to Fishing Ground (A05):
[0] Move President to Fishing Ground (A05) — free
[1] Move President to Fishing Ground (A05) — pay 1 yen (1 to Blue)
> pick a number, or 'cancel'
```

Selecting the collapsed retreat entry opens a **guided retreat**: for each area holding your assistants (ascending area id) it asks `recover how many from <Area> [0-n]?`, then shows the assembled move for a final `confirm`:

```
recover how many from Tea Plantation (A02) [0-1]?
> enter a number, or 'cancel'
...
Return President to hand, recovering 2 assistant(s) (A02 x1, A09 x1)
> confirm / cancel
```

Inside either sub-prompt the accepted inputs are a valid answer or `cancel` (which returns to the full Movement list, applying nothing); `quit`/`q` also exit the program, silently discarding the un-applied sub-prompt (nothing was recorded). An out-of-range number re-asks with its range. The **route** sub-prompt is itself a real paged list, so `more`/`list` page it exactly like the main options; everywhere else in the two sub-prompts a view/`save`/`undo` — and `more` inside the *sequential* guided retreat, which is a question flow rather than a list — is refused with `! finish or cancel this prompt first` (it would otherwise render or act against a state the half-finished prompt doesn't reflect yet). Whatever a sub-prompt assembles is matched against the real legal list before it applies — exactly like the `a`/`b`/`m`/`r` shorthands — so neither can produce a move the engine didn't offer.

**Choice prompts:** some rewards leave you a decision — which good, which warehouse item, where to move an assistant, which Church/Customs space (full inventory in §8.6). Whenever one or more of these is queued, a `Choose: ... — N choice(s) pending` banner naming the head choice appears on its own line directly under the header (the header itself keeps showing the phase), and the numbered list below is exactly that choice's options — nothing about entering them differs from any other phase. A captured example, mid-Church-visit:

```
Turn 1 | Red | Area action | 2 yen 0 VP | c0 s0 t1 f1 i0 | hand 1
Choose: Church space at faith 6 — 1 choice pending
[0] Place on Church faith-4 space (3vp 1move)
[1] Place on Church faith-5 space (4vp 1move)
[2] Place on Church faith-6 space (5vp 2move)
> enter a number, or a command ('view' for game info, 'help' for the list)
```

A choice that lets you decline (warehouse take, assistant move) adds `Decline the ...` as one more numbered option; a choice that's purely "which one" (any-good, Church space, Customs space) has none, because the rulebook doesn't give you one there either. Resolving a choice can queue another — picking the Church space above queues its movement-rider(s) next — and the `— N pending` count tracks how many are still stacked up.

**Placement: pickers, with a grammar bypass.** The raw rules offer every legal combination (186 options on a fresh 2-player board), so the Placement list is collapsed to three entries — `Option A — place 1 assistant each in up to 3 areas (pick areas; 'a' grammar works too)`, `Option B — place 2 assistants in one area (10 eligible; 'b' grammar works too)`, and `Place no assistants`. Selecting **Option A** opens an area-toggle picker: a compact directory of every placeable area (with a `(fee)` mark wherever an opponent President stands — on a terminal the full board is right above it), and typing an area id toggles it in or out of the selection — a mistyped id that is not a real area on this board (`A99`) is refused at that moment (`A99 is not a placement area on this board.`) and never enters the selection. After every toggle the picker echoes the exact move your selection currently assembles — with its fee text — or warns `NOT currently offered (adjust the selection)` (or names an over-limit pick as `too many areas`); `confirm` places, and if the selection is not currently legal it keeps the picker open with your selection intact so you can fix it rather than discarding your work; `cancel` backs out. Selecting **Option B** lists the eligible areas **by id**, each with its fee note — the same grammar Option A uses, and deliberately *unnumbered* so an id can never be read as a list position. **Option B does not commit on the id alone.** Typing an id only *selects* that area and previews the exact move it will place (`-> legal — ...`); **`confirm` is the step that actually places it**, and `cancel` backs out (a Telegram owner gets one more numbered pick over that area's bonus variants at `confirm`). That second step is deliberate: the picker used to place the instant you typed an id, so a mis-typed area entered the game with nothing on screen to catch it — the preview is your chance to see the wrong area *before* it is committed. Read the preview line every time; it is the only mis-entry check this flow has. The grammars do the same thing in one line — and note that a grammar line, unlike the picker, has **no preview and no `confirm` step**, so it commits immediately:

- `a` — place nothing (legal per the rules: 0–3 assistants, 0–4 with Newspaper)
- `a 4 9 12` — Option A: 1 assistant in each listed area (area ids `0`–`17` as shown on the board, up to 3 — 4 with Newspaper tech)
- `b 7` — Option B: 2 assistants in area 7
- `b 7 t 8` — Option B plus the **Telegram** tech bonus assistant in adjacent area 8

Order of ids in `a` doesn't matter; picker and grammar alike match your entry against the legal list and refuse anything the engine didn't offer.

**Assistant-move: grouped by source, with the `m` bypass** (for a pending assistant-move choice — the Movement icon's reward, §8.6). The numbered list groups by SOURCE — one `Move an assistant from <src> — N destinations` entry per source (plus the `Decline` line); picking a source opens the numbered destination list (a single-destination source stays a plain entry). Or type `m <from> <to>` directly, where `<from>` is `hand` or the id of an area holding one of your own on-board assistants, and `<to>` is any area id. Continuing the transcript above once it cascades into an assistant-move choice: `m 8 9` moves the assistant sitting in area 8 to area 9; `m hand 9` moves one from hand to area 9. Like the placement shorthand, `m` only ever matches against the real legal list — a destination the reward doesn't actually allow gets the same rejection a wrong option number would, and there's no `m` shorthand for declining (use the numbered `Decline the assistant move` option). Bad input: `m` alone or `m <one token>` → `Usage: m <from> <to>`; a non-numeric area token → `'<tok>' is not a valid area number`; a well-formed area number that isn't actually offered — including one out of the board's 0-17 range (`m 8 99`), or using `m` when no assistant-move choice is pending at all — → `Illegal: that is not one of the legal assistant moves.` Unlike `a`/`b`, `m` doesn't separately range-check its area numbers first; an out-of-range one just falls through to that same generic rejection.

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
Turn 3 | Red | Additional actions (before) | 3 yen 0 VP | c1 s1 t1 f1 i0 | hand 1
[0] End additional-action phase
(only option — press Enter to apply)
> enter a number, or a command ('view' for game info, 'help' for the list)
```

The first two lines are your own move's summary; the four `Seat 1` lines are the bot's turn; the two `Blue:` lines are that turn's single summary, and the `Turn … -> Turn …` line between them is the hand-over marker the summary brackets. Read the summary as a NET change — an assistant a bot places and then recovers in the same turn nets out of it, which is why the per-move lines above it, and `log`, are the record of what actually happened. The bot decides from the same masked view of the game a seated player gets — its own hand, the public board, and a guess at everything hidden — and never from the engine's private state; that is a property the test suite pins rather than a claim about how well it plays. It is a rollout policy, not a strong opponent: it is here so a whole game can be driven end to end without a second person, and playing to win is a later phase's job.

**Asking the bot for a move (simulation only).** `suggest` runs the built-in evaluator bot on **your own seat's view** and prints what it would play, the arithmetic behind it, and what it nearly played instead. It applies nothing, records nothing, and does not autosave — press it as often as you like, at any decision point, and the game is exactly where you left it. Pressing it twice at one position gives byte-identical output: the bot's randomness is keyed by the game's identity, your seat, and how many moves have been played, so "same position" and "same answer" are the same condition — play a move (or `undo` one) and the answer is free to change. It is deliberately **absent in live mode**, where it prints a refusal instead: a live game's hidden cards are a model reconciled from the physical table rather than the engine's own deck, so the search would be reading a position nobody is playing. A captured suggestion, mid-simulation:

```
─ Suggest ──────────────────────────────
Seat 0 (Red) · lookahead-v1 · beam 4, K=1, M=8, eval v2 · stream 0x9e9a6a9c009baa79
Suggested move:
  Place 2 assistants in Laboratory A (A01)

The line it searched, 7 move(s) to the hand-over (the move above is step 1):
  1. Place 2 assistants in Laboratory A (A01)
  2. Place President from hand in Laboratory A (A01)
  3. Laboratory: acquire University, +2 yen
  4. Skip the 5-Power token
  5. Build a Shophouse (space 1: 1vp)
  6. Use University: pay 1 yen, take an assistant from the warehouse
  7. End additional-action phase

Score at that point: 100   ·   stage 125000/1000000 ppm (0 = opening, 1000000 = end trigger)
  feature                        you   best opp   weight     product
  victory_points                   1          0       13          13
  country_bonus_now                0          0       14           0
  tech_bonus_now                  10          0        7          70
  church_bonus_now                 0          0       -2           0
  customs_bonus_now                0          0        5           0
  assistants_in_hand_core          2          2       29           0
  assistants_in_hand_excess        4          2        4           8
  assistants_in_warehouse         11         12      -10          10
  assistants_on_board              3          4       14         -14
  shophouses_built                 1          0       13          13
  trading_houses_built             0          0       16           0
  five_power_tokens                0          0        8           0
  imported_goods                   0          0        5           0
  achievement_progress_sum         1          1        2           0
  mercenaries_total                0          0        3           0
  hand_size                        1          1       -3           0
  TOTAL                                                          100

Runner-ups (margin against the suggestion, so 0 means tied) — 186 candidate(s) ranked:
         -20  Place 1 assistant in each of: Port A (A00), Tea Plantation (A02), Fishing Groun…
         -20  Place 1 assistant in each of: Port A (A00), Tea Plantation (A02), Silk Mill (A0…
         -20  Place 1 assistant in each of: Port A (A00), Tea Plantation (A02), Bank (A06)
This is the bot's arithmetic, not advice: each row is (your value − the best
opponent's) × the weight for this stage, summed. It read ONE guess at the hidden
cards, and its weights are hand-set rather than tuned.
```

**Reading it.** The header names the search that ran — `lookahead-v1`, a 4-wide beam over ONE sampled guess at the hidden cards, evaluator version `v2` — and the stream key that press used, so two presses that differ can be told apart at a glance. **The line it searched** is the sequence the bot valued the move by: it plays *its own* turn out to the hand-over and scores the position **there**, which is why the score belongs to the end of that line and not to the first move alone. That line lives inside the sampled world, and if a step stops matching your real position the tool says so under the list. **The table** is that score, term by term: your value for a feature, the best opponent's value, the weight that feature carries at this stage (weights slide from an early-game set to a late-game one, and `stage` is how far along the tool thinks the game is), and their product; the products sum to `TOTAL`. **Runner-ups** are the next-best candidates with their margin against the suggestion — a `0` margin is a genuine tie, and a line under them says how many candidates tie at the top. It is the tallest screen this tool prints (roughly 45 rows under the board): on a short terminal the board scrolls off above it, which costs nothing — the suggestion itself stays on screen and the board redraws on your next input — and `panel` or `screen` (§2) buys those rows back.

**It is arithmetic, not advice.** The weights are hand-set rather than tuned, the search is one turn deep in one sampled world, the opponents' hidden cards are guessed, and nothing here has been measured against a human player. What it gives you is a window on what the bot thinks — which is exactly what makes it useful for this testing round: a suggestion that is obviously wrong at the table is worth reporting (§9).

## 5. Commands

```
<number>        apply the numbered legal option
<Enter>         on a single-option prompt: apply that only option, or open it if it is a grouped family
a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas (4 with Newspaper)
b <id>          Placement Option B: place 2 assistants in one area
b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id)
r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)
board           show the area grid and its legend (5-Power token faces, [St] Station, · empty build slot, ~ Canal toll)
players         show each player's resources, hand, tech, tokens, President
ports           show both Port boards and the order-deck size
labs            show both Laboratory boards (each tech's country + effect) and the tech-deck size
tracks          show the Church and Customs tracks with printed VP and rider icons
sites           show each area's building-site printed rewards, build marks, and adjacency
goals           show achievement cards with claim VP, claim state, and per-player progress
hand            show the current player's order-card hand
score           show current VP/money (or the final scores once the game is over)
log             show the tail of the move/event log
view            open the numbered game-info menu over all of the views above ('views'/'?' also work)
suggest         simulation only: what the built-in bot would play from your seat, with its arithmetic
privacy         toggle hiding hand contents in views (default: hidden in live mode)
panel           toggle the players panel inside the composed screen (terminal only)
screen          toggle the composed screen itself (terminal only; pipes are always plain)
more            show the next page of legal options (wraps to page 0)
list            reset to the first page of legal options
undo            undo the last move (and any reveals bound to it)
save [path]     save to path (or to the last-used path if omitted)
reveal <seat> <O..>  live mode: make a hidden opponent starting order public
cancel          back out of any sub-prompt: reveal collection (live), view menu, numbered pick, either Placement picker, guided retreat
help            show this text
quit            exit the REPL ('q' also works)
```

After each applied move the tool prints a **delta summary** — what changed (money, VP, goods, hand, tech, President location, phase, deck sizes, end-trigger announcements) — then the next prompt.

## 6. Live tracking: reveals

In live mode, whenever a move would draw hidden cards, the tool **does not apply it yet** — it asks what physically appeared:

- **Port take** → for each refilled slot, right-to-left (the physical refill order): `What card is now in slot N (Zone)? Enter as 'O<id>' or '<id>'`. Type the order card's **corner number**. Any real, still-hidden order card is accepted — including one boxed face-down at setup (at 2–3 players most of the 36-card deck is never seen): the tool cannot know the true deck order, so it trusts the table and reconciles its hidden model to match. A truthful card is never refused for being one the tool "didn't expect"; only a card that is already face-up elsewhere, or not a real order card, is rejected.
- **Lab take** → the one refilled tech slot: enter `T<id>` or a **name prefix** (e.g. `postal`). A prefix that matches both copies of a title still in the deck resolves automatically to the lowest matching id — the two copies are identical, so tracking is unaffected. (This is the opposite of the *wizard*, which rejects an ambiguous prefix, because setup records which *physical* card sits where — §3.7, §8.2.) Techs are never boxed at setup, so a tech that is not in the deck is a genuine mismatch and is refused.
- `cancel` at any of these aborts the move entirely (nothing is applied or recorded); `quit`/`q` also exit the program from a reveal prompt, discarding the parked (un-applied) move the same way.

There is **no** 5-Power token reveal: the tokens are face-up open information you enter once at setup (§3, step 4) and read off the board (§8.5), so taking one at 5 power just applies — the tool already knows the face.

While one of these prompts is showing, everything you type is read as the answer to it, not as a command — numbered picks, `m`, and view commands like `board` don't work until the reveal is answered, or you `cancel` (or `quit`/`q`, which exits and discards the parked move). (These prompts are unrelated to the choice banner and `m` grammar in §4/§5: reveals are about typing in what physically appeared on a hidden card, not about deciding between options the engine already knows.)

Once all reveals are typed, the move + reveals commit **as one atomic unit** — and one `undo` removes the whole unit.

**Hidden starting orders:** when an opponent's starting order becomes public at the table (they fulfill it), enter `reveal <seat> O<n>` — e.g. `reveal 1 O23` → `Seat 1 revealed O23 as their starting order.`, and the card then shows entering that seat's hand (any autosave failure is reported inline)

**Catching your own typos.** Nothing can detect that you typed the *wrong* real card — enter `O16` for a slot that physically shows `O17` and the tool faithfully records `O16`. Two things let you catch it yourself: every reveal is echoed as it applies, and the `log` view lists the exact card ids recorded for each reveal event, so you can read them back against the table. When you spot a mis-entry, `undo` removes the whole move-and-reveal group so you can re-enter it.

**Desync guard.** After a reveal applies, the tool re-reads the board and confirms the engine placed the revealed card where it said it would. Because the engine places exactly the card you typed, this is an *internal* consistency check — a tripwire for a driver/engine bug, **not** a check that your entry matched the table (that is what the echo and `log` above are for). If it ever fails you'll see:

```
CRITICAL: desync detected -- the revealed card(s) do not match the board after applying the move. Session locked.
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
| `O17 GB  2cop 1fish -> 8vp 1imp` (compact) | The card's country flag, required goods, and rewards. |
| `O17 [Britain]  needs 2 copper + 1 fish  ->  8 VP + 1 import` (long form) | Same, in words. |

### 8.2 Technology cards

| CLI | Physical card / rulebook |
|---|---|
| Card **names** (`Postal System`, `Winery`, …) | Match the printed card titles. |
| ⚠ `Textile Mill` | The printed card title. **The rulebook text calls this card "Spinning Mill"** — same card, one name on cardboard, another in the book. The CLI follows the cardboard. |
| ⚠ `T<id>` input form (e.g. `T12`) | Engine-internal numbering — **not printed anywhere on the physical card**. When entering techs, prefer a name prefix (`postal`, `tex`, `winery`); use `T<id>` only when copying an id the CLI itself displayed. At wizard time an ambiguous prefix is rejected (both copies of a title can be on the boards); mid-game a prefix resolves to the lowest matching id in the deck. |
| 20 titles, 30 physical cards | Ten titles exist as two copies each. The rules allow a player to own only one card *of the same name*; the engine enforces that. |

### 8.3 Goods and resources

| CLI (compact / header letter) | Rulebook term |
|---|---|
| `cop` / `c` | Copper |
| `silk` / `s` | Silk |
| `tea` / `t` | Tea |
| ⚠ `fish` / `f` | **Seafood** (the Fishing Ground good — the CLI abbreviates it "fish") |
| `imp` / `i` | Imported Goods |
| `yen` | Yen (money) — shown spelled out in the status header (`3 yen`); it has **no** single-letter header code (the header goods letters are only `c s t f i`). |
| `vp` | Victory Points |
| ⚠ `wh` | The **Warehouse icon** (take 1 token from your Warehouse tile to hand, paying the Employment Agency cost). A real choice prompt — see §8.6. |
| ⚠ `any` | An "any good of your choice" reward. A real choice prompt — see §8.6. |
| ⚠ `move` | The **Movement icon** (move an assistant). A real choice prompt (`m <from> <to>`, §4/§5) — see §8.6. |

### 8.4 Countries and turn phases

Countries: `GB`=Britain, `US`=America, `FR`=France, `DE`=Germany, `NL`=Netherlands (flags on the order cards).

Turn phases as the CLI names them, against the rulebook's turn structure:

| CLI phase | Rulebook |
|---|---|
| `Additional actions (before)` / `(after)` | ⚠ Not a rulebook step name — the windows in which an Additional Action (fulfilling orders, mercenary use, achievement claims) may be taken before/after the main turn. |
| `Placement` | Step 1 — place assistants (Option A / Option B). |
| `Movement` | Step 2 — move the President (Option A / Option B). |
| `Area action` | Step 3 — carry out the action of the President's area. |
| ⚠ `5-Power bonus` | Steps 4–5 (both optional, both at 5 power): take the area's 5-Power token / claim achievements. |
| `Building` | The optional build (Shophouse or Trading House, 4+ power). |
| `Recovery` | Step 6 — automatic; the CLI performs it without asking. |
| `Turn end` | Bookkeeping between turns. |

⚠ **`Skip` option:** in the six "may"-worded areas (Port, Laboratory, Employment Agency, Chinatown, Church, Customs) the CLI offers an explicit `Skip` move = carry out the area action declining its optional parts. The rulebook has no named "skip" — it's the engine's encoding of "you may". Note v1's conservative reading: taking `Skip` at a 5-power Port forgoes the +3 VP bonus.

### 8.5 Areas, boards, and tracks

The board draws one bordered tile box per area, positioned as the physical staggered pyramid (widest row on top, each row below offset half a tile). Tile display names and wizard entry codes:

| Area | Tile name | Wizard |
|---|---|---|
| Copper Mine | `Copper Mn` | `cm` |
| Silk Mill | `Silk Mill` | `sm` |
| Tea Plantation | `Tea Plant` | `tp` |
| Fishing Ground | `Fishing` | `fg` |
| Bank | `Bank` | `bank` |
| Port A / B | `Port A`/`Port B` | `pa`/`pb` |
| Laboratory A / B | `Lab A`/`Lab B` | `la`/`lb` |
| Employment Agency | `Employ Ag` | `ea` |
| Chinatown | `Chinatown` | `ct` |
| Church | `Church` | `ch` |
| Customs | `Customs` | `cu` |
| Canal | water tile (`~` fill, `toll 1 yen`) | `canal` |

Each tile box shows: a header `A03 Copper Mn c` (area id, name in its type color, then the face-up 5-Power token's good letter — `i/c/s/t/f/y` = import/copper/silk/tea/seafood/yen, blank once the token is taken), a `P:` line (Presidents present by color letter, plus `[St]` when the Station token sits here), assistants (`R2 B1` = 2 red, 1 blue), and buildings (`sh R·B· | th Y` = one mark per shophouse space in printed order — owner color letter, `·` = empty — then the trading house owner). The Canal tile draws as water with its printed toll instead — nothing can ever stand or build there. Under the board a legend documents the token letters, `[St]`, and `·` marks, plus the canal crossing rule whenever this board has a Canal.

Building sites (`sites` view): one line per area holding a site, e.g. `Copper Mine (A03)  S03  [1vp 2wh - 2move]  TH 7vp  sh ···· | th ·  adj: A01, A05  canal->A06` — `S<n>`'s number is the **area's id**, not the physical site card's printed index (that number is only used to validate your entry during wizard setup, §3, and isn't kept once the game starts); `[...]` is the site's four shophouse-space printed rewards (top-left, top-right, mid-left, mid-right; `-` = blank; `wh`/`move`/`any` are the icon rewards from §8.3, real choice prompts when taken); `TH` is the trading-house VP; `sh .. | th ..` are the same build/ownership marks the `board` grid shows for that cell, aligned space-for-space with the `[...]` rewards; `adj:` lists the areas your President can step to from here for free (ascending) — its ordinary movement neighbors, and only those: the Canal tile itself never appears (nothing can stand on it, so it is never a destination and never a Telegram bonus target), and neither does any tile reachable only by crossing the Canal — those are listed separately as `canal-><id>`, a tolled transit-only edge (see **Canal crossing** below). Building options during the Building phase print this same printed-reward text instead of a bare space number: you get one `Build a Shophouse` option per **empty** space — you choose the space — each named with its space number and printed reward, e.g. `Build a Shophouse (space 2: 2vp+1yen)`, so two blank spaces stay tellable apart; `Build a Trading House (7vp)` shows the site's TH value.

Tracks (`tracks` view): Church spaces by faith value 4–11, each showing its printed VP and movement rider (`3vp 1move`); Customs spaces 1–7 with printed goods-costs `[1,1,2,2,3,3,4]`, each showing its printed VP and warehouse/yen rider (`5vp 2wh`, `4vp 2yen`; spaces 5–7 carry no rider). The occupant color letter sits directly after the space label, before the reward. In 2-player games `D` marks the dummy player's assistant on Church faith-7 and Customs space 4 — it occupies the space and competes in that track's majority scoring, and whenever it is on the board the view opens with its own legend line (`D = Dummy — the 2P neutral marker; it holds its space and competes in each track's majority bonus`). Below the two tracks, a **`Station:`** line reports where the Station token sits (`Copper Mine (A03) (all players +1 Power there)`) or `not placed` — it is board-level shared state, so once placed **every** player gets +1 Power when acting in that area (§8.6 item 6). A **`Mercenary supply:`** line follows with the public per-country pile counts (`GB`/`US`/`FR`/`DE`/`NL`, §8.4's codes).

**Canal crossing:** the Canal tile can never hold an assistant, a President, or a building — on the board it draws as water — but President movement can cross straight through it in a single hop, shore to shore: ANY tile touching the Canal connects to ANY other tile touching it. Two shore tiles that already adjoin each other directly keep their ordinary free edge (the free step wins; no toll appears there) — the tolled crossings are exactly the pairs the Canal genuinely bridges, and `sites` lists them per area as `canal-><id>`. Each crossing costs 1 Yen to the bank — a separate, unconditional toll from the opponent-president pass-through fee (which instead goes to that president's owner) — and that includes the very first step out of the President's current area, if that step is itself a crossing. 2-player games have no Canal tile at all. Telegram's adjacent-area placement bonus (§4) does not extend across a canal crossing — it still needs a physically adjoining tile.

### 8.6 Choice prompts — how reward decisions surface

Every reward the rulebook lets you decide is a real prompt now (§4's `Choose: ... — N pending` banner), not an automatic pick. What queues, and how each resolves:

1. **Any-good reward** (`any` in the compact reward text, §8.3): exactly 4 options — 1 copper, 1 silk, 1 tea, or 1 fish. No decline; only which good is optional, not the gain itself. Two any-good icons on the same reward queue and resolve separately, so you can pick a different good for each.
2. **Warehouse-icon reward** (`wh`): up to 3 options — an assistant (free), a shophouse (2 yen), or a trading house (the printed 4/5/6/7 take price, whichever is next) — filtered to what's actually in your warehouse and what you can currently afford, plus an always-available `Decline the warehouse take`. The banner prices them live, e.g. `warehouse take (assistant free / shophouse 2¥ / TH 4¥)` once you can afford all three — the start player's own 3 starting yen is one short of that pictured 4¥ TH option. When nothing in your warehouse is available or affordable, the icon auto-declines silently — no prompt, no banner — same as a table player who has nothing to take either.
3. **Movement-icon reward** (`move`): move one of your own assistants from hand or an on-board area to any other (non-Canal) area — resolve via the numbered list or the `m <from> <to>` shorthand (§4/§5) — or `Decline the assistant move`. One prompt per Movement icon; multiple icons on the same reward resolve independently, same as multiple any-good icons. When you have no assistant anywhere the icon could draw from — nothing in hand, nothing on the board — it auto-declines silently the same way: no prompt appears, same as a table player with nothing to move either.
4. **Church and Customs space choice**: after paying, if 2 or more spaces are legally open (Church: at your computed total faith; Customs: at the matching printed-cost pair), you pick which one — numbered by faith/space number and its printed VP and rider (the same `3vp 1move` / `5vp 2wh` reward text the `tracks` view prints, §8.5). (The Church *payment* options themselves already preview the faith each would produce — §4's previews — so you know what you're buying before you pay.) When only ONE space is open, the tool places you there automatically without a prompt — same as a table player who has no real choice either. A Church placement then queues its space's movement rider(s) as their own assistant-move choice(s), so a single Church visit can cascade through several stacked prompts (the `— N choice(s) pending` count tracks how many are left); a Customs space-1/space-3 placement similarly queues its warehouse-take rider(s).
5. **Telephone's Recovery choice**: owning Telephone with ≥1 of your own assistants in the just-finished action's area pauses Recovery and asks whether to leave exactly 1 assistant there (`Leave 1 assistant behind (Telephone)`) or recover everyone as usual (`Recover all assistants from the action area`). Without Telephone — or with nothing there to leave — Recovery stays fully automatic, same as always.
6. **Station placement** (after taking the Station technology): you choose which area the Station token goes in — every non-Canal area is eligible, so on a full board this is a long numbered list. There is NO decline option: the token is going somewhere. Once placed, `tracks` (§5) shows where it sits, and **every** player — not just its owner — gets +1 Power when acting in that area, so it is worth reconciling against the physical board. If only one area were ever eligible the tool would place it inline without asking; that can't happen on a real board, but it makes Station the one choice whose automatic resolution *does* something rather than nothing.
7. **University and Textile Mill ("Spinning Mill")**: no longer queued choices — they're standing options offered directly in both Additional-Action windows (§8.4), like any other move (`Use University: pay 1 yen, take an assistant from the warehouse`; `Use Textile Mill: discard 1 silk, gain 2 yen`), each usable once per turn.

None of this touches the 5-Power tokens: they are face-up open information you enter once at setup (§3, step 4) and read off the board (§8.5), not a play-time choice or reveal. (The physical box check found the 20 tokens are only Import/Copper/Silk/Seafood/Tea/Yen faces — there's no "Movement" token in the real game, so the board letters stay exactly `i/c/s/t/f/y`.)

## 9. Reporting problems

Every save file is a complete reproduction: seed + setup + every move/reveal. When something looks wrong:

1. `save` (if the session locked itself, `save` still works),
2. note what the table said should happen vs. what the tool did (`log` shows the recent entries),
3. send the JSON file + the seed + your note.

A `CRITICAL: desync` lockdown or an `INVARIANT VIOLATION` message is always report-worthy. So is any legal table move the tool refused, any illegal move it offered, and any place its wording confused you — the display language is part of what's under test.

*Happy testing. The engine has survived 15,000 random self-play games; you are here to be meaner than random.*
