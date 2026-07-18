# Yokohama CLI — Tester's Guide

*For the Phase-2 game driver (`yokohama-cli`), 2026-07-18. Audience: playtesters tracking or simulating games of Yokohama (Hisashi Hayashi, 2016). You need the physical rules in your head; this guide covers how the tool speaks about them — and **flags every place its language or behavior differs from the rulebook**.*

---

## 1. What this is

A terminal program backed by the full Yokohama rules engine. Two modes:

- **Simulation** — the program deals everything from a seed and you play moves for every seat. Good for learning the tool.
- **Live tracking** — you play a *physical* game at the table and enter each move; the engine validates it, tracks all state, and asks you to type in cards as they're physically revealed (port refills, tech refills, 5-Power tokens, hidden starting orders).

The engine only ever offers **legal** moves. If the tool won't offer a move you believe is legal at the table, that is exactly the kind of bug this testing round exists to find — save the game and report it (§9).

> **Testing-round caveat:** a handful of rules choices are auto-resolved in v1 (§8.6). Live-tracking a full physical game is expected to hit those spots; the follow-up "choice encoding" work removes them.

## 2. Running it

```bash
# from the repo root
cargo run -p yokohama-cli              # start the setup wizard
cargo run -p yokohama-cli -- save.json # resume a saved game
```

Rust toolchain required (plain `cargo` works on the project machine). The game autosaves after **every** applied move once a save path is set (the wizard sets one by default), so a crash loses at most the move you were typing — rerun with the save file to resume.

## 3. The setup wizard

Questions are asked one at a time. Each question allows **3 tries**; a third invalid answer aborts setup (rerun to start over). Answers in `[brackets]` are the literal things you type.

| Question | Answer |
|---|---|
| Mode | `sim` or `live` |
| Players | `2`, `3`, or `4` |
| Color, per seat | `red`, `blue`, `yellow`, `green` (no repeats) |
| Start seat | seat index, `0`-based (e.g. `1` = second seat listed) |
| Seed | any integer, or **blank** for a time-derived one (it echoes `seed: N` — write it down; it is the game's identity) |
| Save path | a path, or **blank** for `yokohama-game-<seed>.json` in the current directory |

**Sim tail:** for each seat the two dealt order cards are shown (`O07 [Britain]  needs …  ->  …`); answer `keep 0/1` with `0` or `1`.

**Live tail** (you're describing the physical table):

1. Per seat: `known` (your own seat — then type its two dealt cards and the keep choice) or `hidden` (opponents whose starting order you can't see).
2. **Tiles**: for each board cell, the area tile physically there, by abbreviation: `cm sm tp fg bank pa pb la lb ea ct ch cu canal` (§8.5 maps them).
3. **Building-site cards**: per non-Canal cell, the site card's number `S<n>` or bare `n` (printed index on the card).
4. **Achievements**: a numbered menu of all 12 is printed; type the shown name for each of the 3 in play (no duplicates).
5. **Port contents**: the 6 order cards on Port A left→right as `O<n>`; Port B is only asked at 3-4 players.
6. **Lab contents**: the 6 tech cards on Laboratory A left→right, by `T<id>` or a **name prefix** (prefer names — see §8.2); Lab B only at 4 players.

> **Quirk (known):** some typos (an order id that isn't in the deck, a duplicated card) are only caught at the *end* of the live tail, which aborts setup after all ~30 questions. Double-check card entries as you go. Fixing this is top of the follow-up backlog.

## 4. Playing: the prompt and entering moves

Every screen ends with a status header, the numbered legal options, and a hint line:

```
Turn 1 | Red | Additional actions (before) | 3 yen 0 VP | c1 s2 t0 f1 i0 | hand 1
[0] End additional-action phase
> enter a number, 'a'/'b' for placement, or a command ('help' for the list)
```

Header anatomy: turn number | whose turn | phase (§8.4) | money and VP | goods `c`opper `s`ilk `t`ea `f`ish(=Seafood) `i`mports | order cards in hand.

**Choosing a move:** type the option's number. Long lists show 20 per page — `more` pages forward (wrapping), `list` returns to page 0.

**Placement shorthand** (the one phase with too many options to number usefully):

- `a` — place nothing (legal per the rules: 0–3 assistants)
- `a 4 9 12` — Option A: 1 assistant in each listed area (area ids `0`–`17` as shown on the board grid, up to 3)
- `b 7` — Option B: 2 assistants in area 7
- `b 7 t 8` — Option B plus the **Telegram** tech bonus assistant in adjacent area 8

Order of ids in `a` doesn't matter; the tool matches your entry against the legal list and refuses anything the engine didn't offer.

## 5. Commands

```
<number>        apply the numbered legal option
a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas
b <id>          Placement Option B: place 2 assistants in one area
b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
board           show the area grid
players         show each player's resources, hand, tech, tokens, President
ports           show both Port boards and the order-deck size
labs            show both Laboratory boards and the tech-deck size
tracks          show the Church and Customs tracks
hand            show the current player's order-card hand
score           show current VP/money (or the final scores once the game is over)
log             show the tail of the move/event log
more            show the next page of legal options (wraps to page 0)
list            reset to the first page of legal options
undo            undo the last move (and any reveals bound to it)
save [path]     save to path (or to the last-used path if omitted)
reveal <seat> <O..>  live mode: make a hidden opponent starting order public
cancel          live mode: abort an in-progress card/token reveal prompt
help            show this text
quit            exit the REPL
```

After each applied move the tool prints a **delta summary** — what changed (money, VP, goods, hand, tech, President location, phase, deck sizes, end-trigger announcements) — then the next prompt.

## 6. Live tracking: reveals

In live mode, whenever a move would draw hidden cards, the tool **does not apply it yet** — it asks what physically appeared:

- **Port take** → for each refilled slot, right-to-left (the physical refill order): `What card is now in slot N (Zone)? Enter as 'O<id>' or '<id>'`. Type the order card's **corner number**.
- **Lab take** → the one refilled tech slot: enter `T<id>` or a unique **name prefix** (e.g. `postal`).
- **5-Power token** (taken at 5 power) → one letter for the face-down token you flipped: `i` = 1 Imported Good, `c` = 2 Copper, `s` = 2 Silk, `t` = 3 Tea, `f` = 3 Seafood, `y` = 3 Yen.
- `cancel` at any of these aborts the move entirely (nothing is applied or recorded).

Once all reveals are typed, the move + reveals commit **as one atomic unit** — and one `undo` removes the whole unit.

**Hidden starting orders:** when an opponent's starting order becomes public at the table (they fulfill it), enter `reveal <seat> O<n>` — e.g. `reveal 1 O23` → `Seat 1 revealed O23 as their starting order.`

**Desync guard:** after applying, the tool verifies the typed cards are really where the board says. If not, you'll see:

```
CRITICAL: desync detected -- the revealed card(s) do not match the board after applying the move. Session locked.
```

A locked session accepts only `save`, `log`, and `quit`. This means tracked state and reality have diverged (mis-entry somewhere upstream, or a tool bug). **Save immediately** and report with the file (§9).

## 7. Save, load, undo

- `save` / autosave writes the full game record as JSON — seed, setup, every move and reveal. Saves are atomic (a crash mid-save can't corrupt the previous good file).
- Resume: `cargo run -p yokohama-cli -- <file>`. The game replays deterministically to exactly where you left off.
- `undo` works all the way back to the first move, one entry at a time (a live move takes its bound reveals with it). Page position resets after each undo.
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
| `yen` / `y` | Yen (money) |
| `vp` | Victory Points |
| ⚠ `wh` | The **Warehouse icon** (take 1 token from your Warehouse tile to hand, paying the Employment Agency cost). See §8.6 — v1 auto-resolves it. |
| ⚠ `any` | An "any good of your choice" reward. See §8.6 — v1 auto-picks Copper. |
| ⚠ `move` | The **Movement icon** (move an assistant). See §8.6 — v1 treats it as no effect. |

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

Area names with their grid abbreviations (`board` view) and wizard entry codes:

| Area | Grid | Wizard |
|---|---|---|
| Copper Mine | `CuM` | `cm` |
| Silk Mill | `SiM` | `sm` |
| Tea Plantation | `TeaP` | `tp` |
| Fishing Ground | `FishG` | `fg` |
| Bank | `Bank` | `bank` |
| Port A / B | `PortA`/`PortB` | `pa`/`pb` |
| Laboratory A / B | `LabA`/`LabB` | `la`/`lb` |
| Employment Agency | `EmpAg` | `ea` |
| Chinatown | `Chna` | `ct` |
| Church | `Chrch` | `ch` |
| Customs | `Cstms` | `cu` |
| Canal | `Canal` | `canal` |

Board-grid cells also show: `P:` line (Presidents present, by color letter), assistants (`R2 B1` = 2 red, 1 blue), buildings (`sh RB | th Y` = shophouses by owner color, trading house owner), and `*` = this area's 5-Power token is still unclaimed.

Building sites: `S03  [1vp 2vp+1yen - 2tea]  TH 8vp` = site card 3, its four shophouse spaces' printed rewards (top-left, top-right, mid-left, mid-right; `-` = blank), and the trading-house VP.

Tracks (`tracks` view): Church spaces by faith value 4–11; Customs spaces 1–7 with printed goods-costs `[1,1,2,2,3,3,4]`. In 2-player games `X` marks the pre-blocked dummy space (Customs space 4). ⚠ **Known display gap:** the 2-player Church dummy block (faith-7 space) is *enforced by the engine but not marked in the `tracks` view* — it renders `-` (open). Don't be surprised when placement there isn't offered at 2p.

### 8.6 ⚠ v1 behavior differences — the CLI will not offer some choices the rules give you

These are deliberate, logged engine limitations ("choice encoding" removes them next). **In a live game these are the spots where tracked state can drift from your table** — if one bites, note it and keep going if the auto-resolution happens to match what was played, or stop and report if it doesn't:

1. **"Any good" rewards auto-pick Copper.** The rules let you choose.
2. **Warehouse-icon rewards auto-take 1 assistant** (free). The rules let you take an assistant, shophouse, or trading house at the Employment Agency cost.
3. **Movement-icon rewards do nothing.** The rules move an assistant per icon.
4. **Church and Customs placements auto-pick the space** (highest eligible faith space / higher-VP space of the cost pair). The rules let you choose any eligible space.
5. **University, Textile Mill ("Spinning Mill"), and Telephone's Recovery choice are never offered** — their once-per-turn/optional effects have no move encoding yet. (Their passive cousins — Winery, Brickyard, Gaslight, Electrical Light, Stagecoach, Station, Tram, Postal System, etc. — all work.)
6. The **5-Power Movement token** applies its existing recover-to-hand behavior rather than offering the full assistant-move choice.

## 9. Reporting problems

Every save file is a complete reproduction: seed + setup + every move/reveal. When something looks wrong:

1. `save` (if the session locked itself, `save` still works),
2. note what the table said should happen vs. what the tool did (`log` shows the recent entries),
3. send the JSON file + the seed + your note.

A `CRITICAL: desync` lockdown or an `INVARIANT VIOLATION` message is always report-worthy. So is any legal table move the tool refused, any illegal move it offered, and any place its wording confused you — the display language is part of what's under test.

*Happy testing. The engine has survived 15,000 random self-play games; you are here to be meaner than random.*
