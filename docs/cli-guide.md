# Yokohama CLI — Tester's Guide

*For the Phase-2 game driver (`yokohama-cli`), 2026-07-20. Audience: playtesters tracking or simulating games of Yokohama (Hisashi Hayashi, 2016). You need the physical rules in your head; this guide covers how the tool speaks about them — and **flags every place its language or behavior differs from the rulebook** (marked ⚠).*

---

## 1. What this is

A terminal program backed by the full Yokohama rules engine. Two modes:

- **Simulation** — the program deals everything from a seed and you play moves for every seat. Good for learning the tool.
- **Live tracking** — you play a *physical* game at the table and enter each move; the engine validates it, tracks all state, and asks you to type in cards as they're physically revealed (port refills, tech refills, 5-Power tokens, hidden starting orders).

The engine only ever offers **legal** moves. If the tool won't offer a move you believe is legal at the table, that is exactly the kind of bug this testing round exists to find — save the game and report it (§9).

> **Testing-round note:** this build carries the full choice-encoding + wizard-robustness work — every reward decision the rulebook gives you is a real prompt now (§8.6), the setup wizard re-asks indefinitely on bad input instead of aborting (§3), and the Canal can be crossed (§8.5). Live-tracking a full physical game start to finish is the point of this round — save and report anything that looks wrong (§9).

## 2. Running it

**Prerequisites.** The Rust toolchain (install it via rustup), a clone of this repository, and a shell sitting in the repo root. Plain `cargo` works on the project machine.

```bash
# from the repo root
cargo run -p yokohama-cli              # start the setup wizard
cargo run -p yokohama-cli -- save.json # resume a saved game
```

**First run.** `cargo run -p yokohama-cli` with no arguments opens the setup wizard, which asks — one question at a time — for the mode, player count, per-seat colors, start seat, seed, and save path, then a mode-specific tail; §3 walks through every question. Fastest way to try it: run the command and answer `sim`, `2`, `red`, `blue`, `0`, then press Enter twice to take a random seed and the default save file, keep either dealt card with `0` and `0`, and `confirm` — you are now driving a 2-player simulation from the prompt in §4.

**Resuming.** `cargo run -p yokohama-cli -- <save.json>` replays a saved game deterministically to exactly where you left off. The game autosaves after **every** applied move once a save path is set, and the wizard sets one by default — `yokohama-game-<seed>.json` in the current directory unless you gave a path — so a crash loses at most the move you were typing; just rerun with the save file.

**Terminal, color, and width.** In a real terminal the tool colors what matters — player names in their colors, VP, warnings, the prompt's phase line — and draws the board grid with box-drawing rulers. Piped or redirected output is always plain text (byte-identical to the pre-color tool), and setting the standard `NO_COLOR` environment variable turns color off in a terminal too. The board grid is wide — 86 columns at 2 players, 120 at 3, and 103 at 4 — and there is no terminal-size detection, so the `board` command prints a one-time reminder of the exact width the first time you render a board wider than 100 columns; widen the window if the rows shear.

**If something looks off:**

- **Board rows shear or wrap** — the terminal is narrower than the grid; widen it to at least the width above for your player count (120 columns clears every board).
- **A save won't load** — the tool prints `failed to load <path>: <reason>` and exits; the reason is a typed error. A JSON parse failure means the file is truncated or hand-edited; `replay failed at event N: ...` means a recorded move no longer reconstructs (a stale or tampered save). A mere engine- or rules-**version** mismatch only prints a `warning:` and replays anyway — v1 never refuses on version drift.
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

**Sim tail:** for each seat the two dealt order cards are shown (`O07 [Britain]  needs …  ->  …`); answer `keep 0/1` with `0` or `1`.

**Live tail** (you're describing the physical table):

1. Per seat: `known` (your own seat — then type its two dealt cards and the keep choice) or `hidden` (opponents whose starting order you can't see).
2. **Tiles**: for each board cell, the area tile physically there, by abbreviation: `cm sm tp fg bank pa pb la lb ea ct ch cu canal` (§8.5 maps them). Checked live against the legal per-type tile count for this player count — e.g. a second `cm` at 2 players (which only ever has one Copper Mine) is refused on the spot with `no cm tiles remaining for this board`, not after the rest of the tail.
3. **Building-site cards**: per non-Canal cell, the site card's number `S<n>` or bare `n` (printed index on the card) — checked against the registry and against every other site index already entered.
4. **Achievements**: a numbered menu of all 12 is printed; type the shown name for each of the 3 in play (a duplicate is refused immediately).
5. **Port contents**: the 6 order cards on Port A left→right as `O<n>`; Port B is only asked at 3-4 players. Each card is checked against the order registry and against every order id already entered anywhere — both seats' deals, Port A, and Port B share one duplicate check.
6. **Lab contents**: the 6 tech cards on Laboratory A left→right, by `T<id>` or a **name prefix** (prefer names — see §8.2); Lab B only at 4 players. Checked the same way as orders, with Lab A and Lab B sharing one duplicate check. ⚠ **A name prefix matching more than one card still in play is REJECTED here** (`'<name>' matches more than one tech card -- use T<id>`) — type the exact `T<id>` instead. This is deliberately different from a live-mode reveal prompt (§6), which resolves the same kind of ambiguous prefix automatically to the lowest matching id: the wizard is recording which *physical* card sits where, so a genuine ambiguity has to be settled by you, not guessed.

Every card/tile/site answer above is validated the moment you type it — a bad entry re-asks that same question instead of surfacing at the end of the tail.

**Review before you commit:** once every question is answered, the tool prints a full summary of everything entered so far (`--- Setup review ---`: mode, players, colors, start seat, seed, save path, plus the mode-specific tail) and asks `confirm / edit <section> / abort?`. Only `confirm` starts the game. `edit <section>` re-asks that section's questions from scratch — with the same live validation — then reprints the updated summary; repeat as many times as you like before confirming. Editable sections:

- **live**: `colors`, `seed`, `save`, `deal <seat>`, `tiles`, `sites`, `achievements`, `porta`, `portb` (3-4p only), `laba`, `labb` (4p only)
- **sim**: `colors`, `seed`, `save`, `deal <seat>`

`mode`, `players`, and the start seat aren't editable sections — they decide the shape of everything asked after them (seat count, grid size, whether Port B/Lab B even exist), so changing one of those means aborting and rerunning the wizard from scratch. Editing `seed` in the **sim** tail re-derives the deal preview from the new seed and re-asks every seat's keep choice against it, so a keep answer never silently survives against a preview you no longer see; in the **live** tail, `seed` is only the save file's identity (plus a throwaway check the wizard uses to validate your tile answers, which doesn't actually depend on which seed you pick for a given player count), so editing it there doesn't re-ask anything else.

## 4. Playing: the prompt and entering moves

Every screen ends with a status header, the numbered legal options, and a hint line:

```
Turn 1 | Red | Additional actions (before) | 3 yen 0 VP | c1 s2 t0 f1 i0 | hand 1
[0] End additional-action phase
> enter a number, or a command ('help' for the list)
```

Header anatomy: turn number | whose turn | phase (§8.4) | money and VP | goods `c`opper `s`ilk `t`ea `f`ish(=Seafood) `i`mports | order cards in hand.

The hint line's own middle clause is phase/choice-aware — it only ever names a shorthand that's actually usable right now: `'a'/'b' for placement` appears in `Placement`, `'r' to retreat (or the numbered retreat entry for guided recovery)` in `Movement`, `'m <from> <to>' to resolve (or the numbered Decline)` while an assistant-move choice is queued, and nothing extra (just the plain form above) everywhere else, including every OTHER kind of pending choice.

**Choosing a move:** type the option's number. Long lists show 20 per page — `more` pages forward (wrapping), `list` returns to page 0; whenever there's more than one page, a `page N/M` line appears right above the hint so you always know where you are and how many pages there are in total.

**Movement: destination-first, with sub-prompts.** The raw rules can offer thousands of Movement combinations — every route to every destination, every count of assistants to recover — so the Movement list is *collapsed*. Each reachable destination is ONE numbered entry: if a single route reaches it, that entry is the move itself (cost and who-gets-paid text unchanged); if several routes reach it — say a free walk versus a longer one that pays a pass-through fee — the entry reads `Move President to <Area> (Ann) — <k> route options`. The whole *retreat* family (Movement Option B) collapses to one `Retreat President to hand — choose recovery (<k> combinations; 'r' grammar works too)` entry (a lone bare retreat — no assistants on board — stays a plain numbered entry, no sub-prompt). A captured Movement list with both kinds:

```
Turn 3 | Red | Movement | 1 yen 3 VP | c0 s1 t1 f1 i0 | hand 1
[0] Move President to Fishing Ground (A05) — 2 route options
[1] Move President to Customs (A06) — 2 route options
[2] Retreat President to hand — choose recovery (8 combinations; 'r' grammar works too)
> enter a number, 'r' to retreat (or the numbered retreat entry for guided recovery), or a command ('help' for the list)
```

Selecting a `route options` entry opens a **route sub-prompt** — that destination's routes, each with its full cost/recipient text, so you pick which one:

```
Routes to Fishing Ground (A05):
[0] Move President to Fishing Ground (A05) — free
[1] Move President to Fishing Ground (A05) — pay 1 yen (1 to Blue)
> pick a route number, or 'cancel'
```

Selecting the collapsed retreat entry opens a **guided retreat**: for each area holding your assistants (ascending area id) it asks `recover how many from <Area> [0-n]?`, then shows the assembled move for a final `confirm`:

```
recover how many from Tea Plantation (A02) [0-1]?
> enter a number, or 'cancel'
...
Return President to hand, recovering 2 assistant(s) (A02 x1, A09 x1)
> confirm / cancel
```

Inside either sub-prompt the only accepted inputs are a valid answer or `cancel` (which returns to the full Movement list, applying nothing); an out-of-range number re-asks with its range, and a view/`save`/`undo`/`more` mid-prompt is refused with `! finish or cancel this prompt first` (it would otherwise render or act against a state the half-finished prompt doesn't reflect yet). Whatever a sub-prompt assembles is matched against the real legal list before it applies — exactly like the `a`/`b`/`m`/`r` shorthands — so neither can produce a move the engine didn't offer.

**Choice prompts:** some rewards leave you a decision — which good, which warehouse item, where to move an assistant, which Church/Customs space (full inventory in §8.6). Whenever one or more of these is queued, the phase slot in the header is replaced by a `Choose: ... — N choice(s) pending` banner naming the head choice, and the numbered list below is exactly that choice's options — nothing about entering them differs from any other phase. A captured example, mid-Church-visit:

```
Turn 1 | Red | Choose: Church space at faith 6 — 1 choice pending | 2 yen 0 VP | c0 s0 t1 f1 i0 | hand 1
[0] Place on Church faith-4 space (3 VP)
[1] Place on Church faith-5 space (4 VP)
[2] Place on Church faith-6 space (5 VP)
> enter a number, or a command ('help' for the list)
```

A choice that lets you decline (warehouse take, assistant move) adds `Decline the ...` as one more numbered option; a choice that's purely "which one" (any-good, Church space, Customs space) has none, because the rulebook doesn't give you one there either. Resolving a choice can queue another — picking the Church space above queues its movement-rider(s) next — and the `— N pending` count tracks how many are still stacked up.

**Placement shorthand** (the one phase with too many options to number usefully):

- `a` — place nothing (legal per the rules: 0–3 assistants, 0–4 with Newspaper)
- `a 4 9 12` — Option A: 1 assistant in each listed area (area ids `0`–`17` as shown on the board grid, up to 3 — 4 with Newspaper tech)
- `b 7` — Option B: 2 assistants in area 7
- `b 7 t 8` — Option B plus the **Telegram** tech bonus assistant in adjacent area 8

Order of ids in `a` doesn't matter; the tool matches your entry against the legal list and refuses anything the engine didn't offer.

**Assistant-move shorthand** (for a pending assistant-move choice — the Movement icon's reward, §8.6): `m <from> <to>` instead of hunting the numbered list, where `<from>` is `hand` or the id of an area holding one of your own on-board assistants, and `<to>` is any area id. Continuing the transcript above once it cascades into an assistant-move choice: `m 8 9` moves the assistant sitting in area 8 to area 9; `m hand 9` moves one from hand to area 9. Like the placement shorthand, `m` only ever matches against the real legal list — a destination the reward doesn't actually allow gets the same rejection a wrong option number would, and there's no `m` shorthand for declining (use the numbered `Decline the assistant move` option). Bad input: `m` alone or `m <one token>` → `Usage: m <from> <to>`; a non-numeric area token → `'<tok>' is not a valid area number`; a well-formed area number that isn't actually offered — including one out of the board's 0-17 range (`m 8 99`), or using `m` when no assistant-move choice is pending at all — → `Illegal: that is not one of the legal assistant moves.` Unlike `a`/`b`, `m` doesn't separately range-check its area numbers first; an out-of-range one just falls through to that same generic rejection.

**Retreat shorthand** (Movement Option B: return the President to hand, optionally recovering assistants — the rulebook's other Step-2 option alongside moving him, §8.4): `r` by itself returns the President to hand recovering no assistants; `r <id>` recovers *every* assistant you currently have on board in area `<id>`; `r <id>:<n>` recovers exactly `n` from area `<id>` (`n` must be ≥ 1). Chain several area tokens in one command: `r 4:1 9` recovers 1 assistant from area 4 *and* every assistant you have in area 9. The numbered list still works too, but the whole retreat family now collapses to the single **guided retreat** entry described under Movement grouping above — `r` is the shorthand that *bypasses* that guided flow, assembling and applying the exact retreat in one line instead of stepping through the per-area questions. Like `a`/`b`/`m`, `r` only ever matches against the real legal list — asking for more assistants than are actually there (or any other combination the rules don't offer) gets `That retreat is not in the legal list (check areas/counts).`, the same rejection a wrong option number would get. Bad input: a non-numeric area token → `Bad area token '<tok>'. Usage: r [<area>[:<n>]]...`; a zero, negative, or non-numeric count after `:` → `Bad count in '<tok>'.` An area number that simply isn't on this board (`r 99`) parses fine and falls through to the legal-list rejection above — the legal list, not the parser, is what rules it out.

## 5. Commands

```
<number>        apply the numbered legal option
a [id id id]    Placement Option A: place 1 assistant in each of 0-3 areas (4 with Newspaper)
b <id>          Placement Option B: place 2 assistants in one area
b <id> t <adj>  Placement Option B with the Telegram bonus: +1 assistant in an adjacent area
m <from> <to>   resolve a pending assistant-move choice (<from> is 'hand' or an area id)
r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)
board           show the area grid (legend: * = unclaimed 5-Power token)
players         show each player's resources, hand, tech, tokens, President
ports           show both Port boards and the order-deck size
labs            show both Laboratory boards (each tech's country + effect) and the tech-deck size
tracks          show the Church and Customs tracks with printed VP and rider icons
sites           show each area's building-site printed rewards, build marks, and adjacency
goals           show achievement cards with claim VP, claim state, and per-player progress
hand            show the current player's order-card hand
score           show current VP/money (or the final scores once the game is over)
log             show the tail of the move/event log
more            show the next page of legal options (wraps to page 0)
list            reset to the first page of legal options
undo            undo the last move (and any reveals bound to it)
save [path]     save to path (or to the last-used path if omitted)
reveal <seat> <O..>  live mode: make a hidden opponent starting order public
cancel          abort an in-progress reveal prompt (live mode) or a route/retreat sub-prompt
help            show this text
quit            exit the REPL ('q' also works)
```

After each applied move the tool prints a **delta summary** — what changed (money, VP, goods, hand, tech, President location, phase, deck sizes, end-trigger announcements) — then the next prompt.

## 6. Live tracking: reveals

In live mode, whenever a move would draw hidden cards, the tool **does not apply it yet** — it asks what physically appeared:

- **Port take** → for each refilled slot, right-to-left (the physical refill order): `What card is now in slot N (Zone)? Enter as 'O<id>' or '<id>'`. Type the order card's **corner number**.
- **Lab take** → the one refilled tech slot: enter `T<id>` or a unique **name prefix** (e.g. `postal`).
- **5-Power token** (taken at 5 power) → one letter for the face-down token you flipped: `i` = 1 Imported Good, `c` = 2 Copper, `s` = 2 Silk, `t` = 3 Tea, `f` = 3 Seafood, `y` = 3 Yen.
- `cancel` at any of these aborts the move entirely (nothing is applied or recorded).

While one of these prompts is showing, everything you type is read as the answer to it, not as a command — numbered picks, `m`, and view commands like `board` don't work until the reveal is answered or you `cancel`. (These prompts are unrelated to the choice banner and `m` grammar in §4/§5: reveals are about typing in what physically appeared on a hidden card or token, not about deciding between options the engine already knows.)

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

Board-grid cells also show: `P:` line (Presidents present, by color letter), assistants (`R2 B1` = 2 red, 1 blue), buildings (`sh R-B- | th Y` = one mark per shophouse space in printed order — owner color letter, `-` = empty — then the trading house owner), and `*` = this area's 5-Power token is still unclaimed. The `board` command repeats that `*` mark as a one-line legend under the grid.

Building sites (`sites` view): one line per area holding a site, e.g. `Copper Mine (A03)  S03  [1vp 2wh - 2move]  TH 7vp  sh ---- | th -  adj: A01, A05  canal->A06` — `S<n>`'s number is the **area's id**, not the physical site card's printed index (that number is only used to validate your entry during wizard setup, §3, and isn't kept once the game starts); `[...]` is the site's four shophouse-space printed rewards (top-left, top-right, mid-left, mid-right; `-` = blank; `wh`/`move`/`any` are the icon rewards from §8.3, real choice prompts when taken); `TH` is the trading-house VP; `sh .. | th ..` are the same build/ownership marks the `board` grid shows for that cell, aligned space-for-space with the `[...]` rewards; `adj:` lists the area ids reachable from here (ascending) — the President's movement neighbors — with any canal-crossing partner shown separately as `canal-><id>` (a transit-only edge, see **Canal crossing** below). Building options during the Building phase print this same printed-reward text instead of a bare space number: you get one `Build a Shophouse` option per **empty** space — you choose the space — each named with its space number and printed reward, e.g. `Build a Shophouse (space 2: 2vp+1yen)`, so two blank spaces stay tellable apart; `Build a Trading House (7vp)` shows the site's TH value.

Tracks (`tracks` view): Church spaces by faith value 4–11, each showing its printed VP and movement rider (`3vp 1move`); Customs spaces 1–7 with printed goods-costs `[1,1,2,2,3,3,4]`, each showing its printed VP and warehouse/yen rider (`5vp 2wh`, `4vp 2yen`; spaces 5–7 carry no rider). The occupant color letter sits directly after the space label, before the reward. In 2-player games `X` marks both pre-blocked dummy spaces — Church faith-7 and Customs space 4.

**Canal crossing:** the Canal tile can never hold an assistant, a President, or a building — that part is unchanged — but President movement can now cross straight through it in a single hop, connecting whichever of its (up to) four flanking pairs actually exist on this board's grid: left↔right, up↔down, and both diagonals. A 3-player board's grid is only 2 rows, so it can only ever have the left↔right pair; a 4-player board's 3-row grid can have all four; 2-player games have no Canal tile at all. Each crossing costs 1 Yen to the bank — a separate, unconditional toll from the opponent-president pass-through fee (which instead goes to that president's owner) — and that includes the very first step out of the President's current area, if that step is itself a crossing. "Bent" transits (entering one flank and exiting anywhere but the exact opposite one) are illegal and never offered: the tool only ever builds the straight-through edges, so there's nothing bent to pick from in the first place. Telegram's adjacent-area placement bonus (§4) does not extend across a canal crossing — it still needs a physically adjoining tile.

### 8.6 Choice prompts — how reward decisions surface

Every reward the rulebook lets you decide is a real prompt now (§4's `Choose: ... — N pending` banner), not an automatic pick. What queues, and how each resolves:

1. **Any-good reward** (`any` in the compact reward text, §8.3): exactly 4 options — 1 copper, 1 silk, 1 tea, or 1 fish. No decline; only which good is optional, not the gain itself. Two any-good icons on the same reward queue and resolve separately, so you can pick a different good for each.
2. **Warehouse-icon reward** (`wh`): up to 3 options — an assistant (free), a shophouse (2 yen), or a trading house (the printed 4/5/6/7 take price, whichever is next) — filtered to what's actually in your warehouse and what you can currently afford, plus an always-available `Decline the warehouse take`. The banner prices them live, e.g. `warehouse take (assistant free / shophouse 2¥ / TH 4¥)` once you can afford all three — the start player's own 3 starting yen is one short of that pictured 4¥ TH option. When nothing in your warehouse is available or affordable, the icon auto-declines silently — no prompt, no banner — same as a table player who has nothing to take either.
3. **Movement-icon reward** (`move`): move one of your own assistants from hand or an on-board area to any other (non-Canal) area — resolve via the numbered list or the `m <from> <to>` shorthand (§4/§5) — or `Decline the assistant move`. One prompt per Movement icon; multiple icons on the same reward resolve independently, same as multiple any-good icons. When you have no assistant anywhere the icon could draw from — nothing in hand, nothing on the board — it auto-declines silently the same way: no prompt appears, same as a table player with nothing to move either.
4. **Church and Customs space choice**: after paying, if 2 or more spaces are legally open (Church: at your computed total faith; Customs: at the matching printed-cost pair), you pick which one — numbered by faith/space number and its VP. When only ONE space is open, the tool places you there automatically without a prompt — same as a table player who has no real choice either. A Church placement then queues its space's movement rider(s) as their own assistant-move choice(s), so a single Church visit can cascade through several stacked prompts (the `— N choice(s) pending` count tracks how many are left); a Customs space-1/space-3 placement similarly queues its warehouse-take rider(s).
5. **Telephone's Recovery choice**: owning Telephone with ≥1 of your own assistants in the just-finished action's area pauses Recovery and asks whether to leave exactly 1 assistant there (`Leave 1 assistant behind (Telephone)`) or recover everyone as usual (`Recover all assistants from the action area`). Without Telephone — or with nothing there to leave — Recovery stays fully automatic, same as always.
6. **Station placement** (after taking the Station technology): you choose which area the Station token goes in — every non-Canal area is eligible, so on a full board this is a long numbered list. There is NO decline option: the token is going somewhere. Once placed, `tracks` (§5) shows where it sits, and **every** player — not just its owner — gets +1 Power when acting in that area, so it is worth reconciling against the physical board. If only one area were ever eligible the tool would place it inline without asking; that can't happen on a real board, but it makes Station the one choice whose automatic resolution *does* something rather than nothing.
7. **University and Textile Mill ("Spinning Mill")**: no longer queued choices — they're standing options offered directly in both Additional-Action windows (§8.4), like any other move (`Use University: pay 1 yen, take an assistant from the warehouse`; `Use Textile Mill: discard 1 silk, gain 2 yen`), each usable once per turn.

None of this touches the 5-Power token reveal menu (§6): the physical box check found the 20 tokens are only Import/Copper/Silk/Seafood/Tea/Yen faces — there's no "Movement" token in the real game, so the reveal letters stay exactly `i/c/s/t/f/y`.

## 9. Reporting problems

Every save file is a complete reproduction: seed + setup + every move/reveal. When something looks wrong:

1. `save` (if the session locked itself, `save` still works),
2. note what the table said should happen vs. what the tool did (`log` shows the recent entries),
3. send the JSON file + the seed + your note.

A `CRITICAL: desync` lockdown or an `INVARIANT VIOLATION` message is always report-worthy. So is any legal table move the tool refused, any illegal move it offered, and any place its wording confused you — the display language is part of what's under test.

*Happy testing. The engine has survived 15,000 random self-play games; you are here to be meaner than random.*
