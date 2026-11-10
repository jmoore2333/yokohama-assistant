# The browser client: a guide for players, hosts and readers

Yokohama in a browser: how to play at the hosted site, how to ask the bot about a move, how to run a server of your
own, and which of the client's behaviours are deliberate. The client's own Help panel teaches the screen itself; this
page is the map around it, and §6 names the documents that go deeper.

## 1. What it is, and what it can do

The browser client is the same engine, the same words and the same legal-move gate as the terminal
([`docs/cli-guide.md`](cli-guide.md)), served over HTTP and drawn in a browser. It is a second frontend over one
engine, never a second engine: the server routes, scopes and serializes, and computes nothing about the game, so the
terminal and the browser, at the same rules revision, cannot disagree about a rule.

It runs three ways:
- **The hosted site**, `https://yokohama.moore.nyc`: tables for two to four players, with friends or against the
  bot (§2).
- **Your own server**: one Docker image serves the lobby, the tables, the card art and the client (§4).
- **The bot in the browser, with no server**: the door's bot card plays the whole game inside the browser, keeps it
  in the browser's own storage, and works offline once the release is installed. Any host that serves the release
  will do.

At a table you can play, spectate, talk in the table's chat, walk back through the game on the replay track, ask the
bot what it would play and why it played what it did (§3), and save a finished game as the same file the terminal
loads.

## 2. Playing at the hosted site

Open `https://yokohama.moore.nyc`. Until the switch that opens the site to the public, Cloudflare Access also stands
in front of the hostname and asks for its own login first; past it, every step below is the same with or without it.

### The door

The first screen is the **door**, and it asks the server for nothing. It has three cards:
- **You against the bot**, at two players. It plays entirely on this device, needs no account, and works offline.
  **New game** starts one, and the card names the version of the bot it plays.
- **Play with friends**, at two to four players. **Go to the lobby** leads through the sign-in to the lobby.
- **On this device**: the games this browser holds, which are the bot games you started here. **Back up** writes
  every one of them to a file and **Restore** reads one back, and the card reads "Ready to play offline" once the
  release is installed.

### Signing in

**Go to the lobby** starts the sign-in at `/enter`, which takes you to WorkOS's page, where you give your email
address and type the code sent to it. The sentence under the link says who signs you in, WorkOS, and what the site
keeps: your verified address, which is what seats you. The lobby's top corner reads **Playing as** and your name.
**change name** sets the name your seats and your messages are labelled with (left blank, your address is used), and
**sign out** is beside it.

### The lobby and a table

The lobby has three lists: **Your tables**, the tables you sit at; **Open tables**, every other table still waiting
for players; and **Being played**, the games you can spectate, with other people's finished games folded under
**Finished**. The lobby has a chat of its own beside them.

**New table** is the form, one row per seat. Each seat is you, a friend named by their email address, an open seat
for whoever has the table's link, or the bot. The bot is offered at two players only, because that is where it has
been measured. You choose who plays first or leave it to a draw, and you can give the table a name and a
**password** that anyone joining or spectating has to type. Fill every seat and the table deals at once; leave one
open and it waits.

A table with an open seat has a **link** of its own, its address ending `#/t/<id>`, which **Invite people** on the
table's screen copies. Whoever opens it signs in, types the password if the table asks for one, and takes the seat;
anyone can also find the table under Open tables. The deal begins by itself when the last seat is taken.

Each person keeps one of two dealt cards **from their own view**. You are shown your own two cards over the set
table, each with the bot's **Keep score** under it: its preference from the card's printed economics alone,
hand-set and unmeasured, so a number rather than a recommendation. Nobody else's cards reach your browser. The other
seats read *chosen* or *waiting*, and a bot seat's two cards are face down. The technologies and the goals are
revealed once you have chosen, since the Tasty Minstrel Games English edition (20 pages, in the Tabletop Simulator
upload) puts them on the table after the step where the two cards are dealt and chosen in secret. The game starts
when the last person has kept.

### The game screen

![The client at two players on a desktop screen: the staggered pyramid of ten area tiles fills the table on the
left, each tile the printed area art with its building-site card, its pieces and an id badge, and the side column on
the right carries the answer bar at its top, with the phase ladder, the seat to move and the numbered moves it
offers, above the last move and both seats' goods, money and pieces.](../web/public/help/screen.webp)

The **table** fills the left: the area tiles laid out as the staggered pyramid the real board makes, each the
printed tile with any building-site card and pieces on it, and under the pyramid whichever boards you have switched
on. The **side column** on the right holds the **answer bar** pinned at its top, so whatever the game is asking you
is always on screen. Under it come the account of the last turn, every seat's goods, money and pieces, and your own
hand. When an opponent moves, the turn plays out across your table one step at a time. On a narrow window the same
parts stack, with the answer bar pinned along the bottom edge.

The answer bar lists the moves the engine offers, numbered: press one. While a placement picker is open the table
itself is the input, and the cost of your selection is printed before you commit to it.

- **Boards**, along the top, is one menu with a switch for each board that can sit under the pyramid: the port, the
  laboratory, the tracks, the goals and the building sites.
- **Cards** opens the four full indexes: all 36 orders, all 20 technologies, the achievements in play and the 24
  printed sites, each saying where every copy is.
- **Log** opens the move trail with the **replay track** above it. Drag the track, step with the arrows, or press a
  line to jump to the position that move was decided in, and **Back to live** returns you to now. Nothing you do
  there changes the game.
- **Undo** against the bot works as it does in the terminal. At a table with more than one person you can take back
  your own move only while your turn has not passed.

### Saves, the archive and offline play

- **A table saves itself.** Every move is written to the server as it is applied, and a table you left is under
  **Your tables**, where it resumes exactly where you left it.
- **A finished game is filed to the site's archive**, and a game with moves in it is filed there before its creator
  abandons it.
- **Your own finished game downloads with its seed.** **Save game file** on its screen writes the whole record, the
  same file the terminal loads, and the seed is what lets it replay.
- **A bot game lives in this browser.** It is stored only here, so clearing the site's data takes it with it.
  **Save game file** works at any point, and when the game finishes it goes to the site's archive with no prompt.
- **The release installs for offline play.** Once the door's device card reads "Ready to play offline", the bot card
  plays with no connection.
- **Updates wait for you during a game.** The door takes a waiting release by itself. On the lobby and a game screen
  the line "A new version is ready. Reload to use it" appears instead and waits for your click, so nothing reloads
  while someone plays.

## 3. Asking the bot

Two buttons along the game screen's top ask the bot, `v4-step9-draft1`, the same identity the terminal plays.

- **Suggest** runs the production bot on *your* seat and shows what it would play, the line it looked at, and the
  arithmetic behind the score: term by term, with the numbers on screen adding up to the total printed under them.
  It applies nothing. A spectator is not offered it, because it runs on a seat.
- **Why** does the same for a move a bot already played: the game is replayed to that point and the bot's own
  decision is re-run, so the account is a reconstruction the engine can vouch for rather than a note someone stored.
  If the reconstruction does not match what was played, it refuses and says so.

**It is the bot's arithmetic, not advice.** Each answer read one guess at the hidden cards: the bot sees what a
seated player sees, its own hand and the public board, and the line it searched lives inside that guess, so a
suggestion can rest on a card the real game does not hold. How well the bot plays is measured in
[`docs/computer-player.md`](computer-player.md), not claimed here.

## 4. Hosting your own

**The one command**, from a checkout of this repository, with Docker and `curl` installed:

```bash
bash deploy/run-local.sh
```

It builds the server's image from your checkout and runs it on this machine under the open door, on
`127.0.0.1:8080` (if that port is taken, `YOKOHAMA_WEB_PUBLISH=127.0.0.1:8081:8080` picks another), with the games in
a named volume, `yokohama-web-local-data`. It reads the server's health before it prints the address to open,
`http://127.0.0.1:8080/`. The image is `linux/amd64` by design, so on Apple silicon a first build runs under
emulation and takes minutes; a rebuild reuses the toolchain layers. `--dry-run` prints every command it would run and
runs none. `bash deploy/run-local.sh --stop` stops and removes the container and leaves the volume, so the next run
finds the games where they were.

**The trust model is the open door's: anyone who can reach the port may use any name.** On the way to the lobby you
choose a name once, this browser remembers it, and the name is who you are. The script binds the port to 127.0.0.1,
so only this machine reaches it.

**Beyond this machine**, the host guides take over:
- [`deploy/README.md`](../deploy/README.md), the self-host guide, for a LAN, a tunnel or Cloudflare Access. Its §1 is
  the Access door, the repository's default, with the one rule it rests on; §2 is the open door on a home network,
  a LAN or a VPN; §3 the build values; §4 the run, with the host list; §9 moving a running container to a release
  with `deploy/update.sh`.
- [`worker/README.md`](../worker/README.md), the Cloudflare host: a Worker, Durable Objects and an R2 bucket, its
  doors and identity sources (§1.5), and its build and deploy (§9). **A first deploy of your own Worker records it in
  the deploy guard's `DEPLOYMENTS` table before the guard runs**: in your own copy of the repository you add a row to
  that table, in [`worker/scripts/deploy-guard.mjs`](../worker/scripts/deploy-guard.mjs), keyed by your configuration
  file and naming its Worker, its archive bucket, its two rate-limit namespaces and its routes. The guard holds each
  file in that table to its own record, and holds any other configuration to this repository's production file, so a
  Worker it has no record of is refused. The first deploy then runs the deploy guard and `wrangler deploy`
  by hand, skipping the secrets guard alone, since a Worker that does not exist yet holds no secret; from the second
  deploy on, the `npm run deploy` scripts run both guards (§9.3).

## 5. What is by design

These can read like gaps and are not; each is a property the client keeps, with its reason.

- **Each browser is served its own seat.** Every answer, live or historical, the card indexes and the explanations
  included, is built for the seat the caller holds: your hand is open, every other hand is a count, and a spectator
  sees only counts. The bot is held to the same mask: it decides from its own seat's view and a guess at the hidden
  cards, never from the engine's private state. The reason: nobody at the table, the bot included, reads a card the
  physical table would not show them, and walking every position of the replay returns your own hands and no
  other's.
- **The bot's seat is the production identity, the same one the terminal plays.** A game that is not over, saved at this
  build's rules revision, opens exactly when this build reconstructs the bot recorded in it: an update that still runs a
  game's bot keeps the game playable, and one that no longer does refuses it with the reason, never playing it as
  another bot. The reason: a game is one game only if the bot that started it plays all of it. A finished game opens,
  whichever bot played it, since no bot moves in it again; a game saved at another rules revision, finished or not, does
  not open, by design.
- **Every move goes through the engine's legal list.** The server routes, scopes and serializes and computes nothing
  about the game: a move the engine did not list is refused, and every list the client shows came from the engine.
  The reason: one engine decides, so the terminal and the browser, at the same rules revision, cannot disagree about
  a rule.
- **Your own server answers only the host names it is told.** An unlisted `Host` is answered 421. The reason: a page
  on another site can make a browser send requests to your server under a name the attacker controls, by changing
  what that name resolves to, and the `Host` the browser sends is the one thing that name cannot forge. Loopback is
  always answered; list the name your browsers use for a LAN or a proxy, or switch the check off
  ([`deploy/README.md`](../deploy/README.md) §4).
- **Under the open door, a name is a label and a key.** On a server you run with no identity provider, a seat is
  acted in by whoever types its name: two people who type one name are one player, and nothing stops one person
  typing another's. So bind the port only where everyone who can reach it is someone you would hand the table to:
  your own machine, a home network, a LAN or a VPN ([`deploy/README.md`](../deploy/README.md) §2). The hosted site's
  door is the other kind: there a seat is acted in only by the verified address it names.

## 6. Where to read next

- **The Help panel** inside the client, the **Help** button on a game screen, says which part of the screen is which;
  the door's **How to play** carries its words without the pictures.
- [`docs/web-lane.md`](web-lane.md) is the browser lane's operating record: its architecture, every variable, the API
  map and the longer list of what is by design.
- [`deploy/README.md`](../deploy/README.md) and [`deploy/RUNBOOK.md`](../deploy/RUNBOOK.md) are the self-host guide
  and its operating record.
- [`worker/README.md`](../worker/README.md) and [`worker/RUNBOOK.md`](../worker/RUNBOOK.md) are the Cloudflare host's
  guide and its operating record.
- [`docs/cli-guide.md`](cli-guide.md) is the terminal client over the same engine, and the authority on its features.
- [`docs/computer-player.md`](computer-player.md) is the computer player: what it is, and how it was measured.
