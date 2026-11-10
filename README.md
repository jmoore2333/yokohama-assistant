# Yokohama Assistant

A rules engine, a CLI game tracker and a browser client for the board game Yokohama (Hisashi Hayashi, 2016), with a
computer player built to play to win. It is for people who play Yokohama and want a game checked move by move or a
bot to play against, and for people who build and measure game-playing programs.

[![Latest release](https://img.shields.io/github/v/release/jmoore2333/yokohama-assistant)](https://github.com/jmoore2333/yokohama-assistant/releases) [![Licence](https://img.shields.io/github/license/jmoore2333/yokohama-assistant)](LICENSE) [![Tests](https://github.com/jmoore2333/yokohama-assistant/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/jmoore2333/yokohama-assistant/actions/workflows/test.yml)

Yokohama is a merchant trading game for two to four players, set in Meiji-era Yokohama. This repository holds its
rules as a library, two ways to play it on a screen, and a bot you can ask why it played what it did.

## Try it in a browser

Open [yokohama.moore.nyc](https://yokohama.moore.nyc) and choose **You against the bot**: the bot plays the whole game
in your browser, with no account. **Play with friends** seats two to four people at a table and signs each one in by
a code sent to their e-mail address.

![A two-player game in the browser at the first turn: the area tiles with their site cards on the left and the
end-trigger strip along the top; on the right the phase ladder, the prompt for Red's additional actions with its two
numbered moves and an Undo button, and below them both players' yen, goods and assistants](worker/docs/img/game-desktop-player.webp)

The browser client is a second frontend over the same engine, never a second engine: the server routes every move
through the engine's legal list and computes nothing about the game, so the terminal and the browser, at the same rules
revision, cannot disagree about a rule. It runs at the hosted site, on your own server from one Docker image, or
entirely inside the browser against the bot, offline once installed. At a table you can play friends or the bot, watch a
game, chat, step back through the replay, and ask the bot what it would play and why it played what it did.

The [browser guide](docs/web-guide.md) covers playing at the hosted site, asking the bot, and hosting your own.

## Where to go from here

| If you want to | Start with |
|---|---|
| play right now | the hosted site above: no install, and no account for a game against the bot |
| run it yourself | the [quick start](#quick-start) below: the terminal client from a release archive or a clone, the browser client from one Docker command |
| see how the bot decides, and how it got this far | [the computer player](docs/computer-player.md): what it is allowed to see, how to ask it why it played a move, and each measured step from the first bot to the one that ships |
| use the rules engine in a program of your own | [the engine](docs/engine.md): a complete game as one short program, where legality is decided, and how the engine is checked against an independent referee |
| learn the terminal client | the [CLI guide](docs/cli-guide.md): the wizard, entering moves, every command, and tracking a game on a physical table |
| host a server for your friends | the [browser guide, §4](docs/web-guide.md#4-hosting-your-own), then [`deploy/README.md`](deploy/README.md) for Docker and [`worker/README.md`](worker/README.md) for Cloudflare |
| look up a rule or a ruling | the [rules reference](analysis-1/yokohama-kb/01-rules-reference.md), and the decision log in [`guidingpath.md`](guidingpath.md), which carries every ruling |
| see how the project is run | [how the work is organized](docs/method.md), the owner's task board with the work in flight ([`docs/handover.md`](docs/handover.md)), and the index of every document ([`docs/README.md`](docs/README.md)) |

## Quick start

### The command-line client

On an Apple-silicon Mac, fetch the release archive, unpack it and run the binary:

```bash
curl -fLO https://github.com/jmoore2333/yokohama-assistant/releases/latest/download/yokohama-cli-v0.7.3-macos-apple-silicon.tar.gz \
  && tar xzf yokohama-cli-v0.7.3-macos-apple-silicon.tar.gz \
  && cd yokohama-cli-v0.7.3-macos-apple-silicon && ./yokohama-cli
```

If the fetch answers 404, the release this page names is not the latest one: take the archive from the
[Releases page](https://github.com/jmoore2333/yokohama-assistant/releases), or build from a clone as below.

The Releases page always carries the newest build, for three platforms:

| Platform | The archive's name ends | Run |
|---|---|---|
| macOS, Apple silicon | `-macos-apple-silicon.tar.gz` | `./yokohama-cli` |
| Ubuntu, x86_64 | `-ubuntu-x86_64.tar.gz` | `./yokohama-cli` |
| Windows, x86_64 | `-windows-x86_64.zip` | `.\yokohama-cli.exe`, from PowerShell in the unpacked folder |

Each archive carries the guide as `CLI-GUIDE.html` beside the binary, and the Releases page also lists it on its own
as `yokohama-cli-guide.html`. A browser download of the macOS archive lands quarantined, and macOS then refuses the
binary inside, which is ad-hoc signed rather than notarized: fetch it with `curl` as above, build it with `cargo`, or
clear the mark with `xattr -d com.apple.quarantine yokohama-cli`.

From a clone, with Rust installed through [rustup](https://rustup.rs), which fetches the toolchain the repository pins
in `rust-toolchain.toml` on the first build:

```bash
git clone https://github.com/jmoore2333/yokohama-assistant.git
cd yokohama-assistant
cargo run -p yokohama-cli
```

Either way, the first screen is the setup wizard's first question:

```text
 __   __   ___    _  __   ___    _   _      _      __  __     _
 \ \ / /  / _ \  | |/ /  / _ \  | | | |    / \    |  \/  |   / \
  \ V /  | | | | | ' /  | | | | | |_| |   / _ \   | |\/| |  / _ \
   | |   | |_| | | . \  | |_| | |  _  |  / ___ \  | |  | | / ___ \
   |_|    \___/  |_|\_\  \___/  |_| |_| /_/   \_\ |_|  |_|/_/   \_\

Mode? (sim/live)
```

To play the bot, answer the wizard like this:

| The wizard asks | Answer | What it means |
|---|---|---|
| `Mode?` | `sim` | a game the engine deals; `live` tracks a game on a physical table |
| `Number of players?` | `2` | |
| `Seat 0 color?` then `Seat 1 color?` | `red` then `blue` | |
| `Which seat starts?` | `0` | |
| `Seed?` then `Save path?` | Enter, twice | a random seed and the default save file |
| `Seat 0: keep card 1 or 2?` then seat 1's | `1` then `1` | each seat keeps one of its two dealt order cards |
| `Bot seats?` | `1` | the bot takes seat 1 and you play seat 0 |
| `confirm / edit <section> / abort?` | `confirm` | the game starts |

The game autosaves after every applied move, to `yokohama-game-<seed>.json` in the directory you ran the CLI from, so
the file appears with your first move and not before. `abort` leaves the wizard and `q` leaves a game. The
[CLI guide](docs/cli-guide.md) walks through every question and command.

### The browser client, on your own machine

From a clone, with Docker running and `curl` installed:

```bash
bash deploy/run-local.sh
```

It builds the server's image from the checkout and runs it on this machine. On Apple silicon a first build runs under
emulation and takes minutes, and when the container starts Docker prints a `WARNING` that the image's platform
(linux/amd64) does not match the host's, which is expected. The script ends with these two lines:

```text
run-local: open http://127.0.0.1:8080/
run-local: this is the open door — anyone who can reach this port may use any name; it is bound to 127.0.0.1 (deploy/README.md §2 for a LAN)
```

That address opens on the door's three cards: **You against the bot**, **Play with friends** and **On this device**.
`bash deploy/run-local.sh --stop` stops the server and keeps its games, and `--dry-run` prints every command a run
would execute and runs none.

### The tests

```bash
cargo test -p yokohama-core
YOKOHAMA_BGA_CORPUS_OPTIONAL=1 cargo test -p yokohama-cli
```

The first is the engine's suite. The CLI suite reads Board Game Arena captures that a clone does not carry, and stops
with a message naming them when they are absent; the variable turns that stop into a skip.
[`CONTRIBUTING.md`](CONTRIBUTING.md) lists the suites to run after an edit and the lint gate, and links the web
client's.

## How it is built

- **One engine decides every rule.** The terminal, the browser and the bot all take their moves from the engine's
  legal-move list and compute no rule of their own ([the engine](docs/engine.md)).
- **An independent referee checks the engine.** The oracle is a second implementation written from the rules sources
  alone, never from the engine's code, and a differential harness compares the two on legal moves and final scores
  ([how it is verified](docs/engine.md#7-how-it-is-verified)).
- **The bot sees what a player sees.** It decides from its own seat's view and a guess at the hidden cards, never from
  the engine's private state. `suggest` shows the arithmetic behind the move it would play, and `why` replays the game
  and re-runs a move it made, refusing to explain a move it cannot reproduce
  ([the computer player, §4](docs/computer-player.md#4-suggest-and-why)).
- **A strength claim is registered before it is measured.** Each milestone's claim, its field and the rule that decides
  it are committed before a game is played, and every figure is quoted as its record states it, the results that came
  back null among them ([how the work is organized, §4](docs/method.md#4-pre-registration-and-the-rules-of-evidence)).

## What is inside

| Path | What it is |
|---|---|
| `crates/yokohama-core` | The engine: legal moves, checked application, scoring, seeded setup, saved records and strict replay. The computer player is built on it. |
| `crates/yokohama-cli` | The command-line client: a setup wizard and a prompt for games the engine deals, against the bot or between people, and for tracking a game on a physical table, with save, resume, undo, `suggest` and `why`. |
| `crates/yokohama-oracle`, `crates/yokohama-difftest` | The independent referee and the harness that compares the engine against it. |
| `crates/yokohama-web`, `web/` | The web layer and the browser client. |
| `crates/yokohama-wasm`, `crates/yokohama-worker`, `worker/` | The same layer compiled to WebAssembly, for the game that runs inside the browser and for the Cloudflare Worker host. |
| `deploy/` | The Docker image and the scripts for a server of your own. |
| `analysis-1/yokohama-kb`, `crates/gen-data` | The knowledge base, with the rules reference and the card data as YAML, and the generator that builds the engine's card tables from it. |
| `crates/yokohama-bridge`, `bridge/` | A read-only capture of the project owner's own Board Game Arena sessions, which the CLI's `import` replays through the engine. |

## Status

- **The engine** is complete and differentially verified against the oracle at `SEMANTICS_REV` 7, the rules revision
  every saved game is keyed to. One rules defect is known, tracked as
  [issue #62](https://github.com/jmoore2333/yokohama-assistant/issues/62): the Language School's yen can miss its own
  order's warehouse take, and its fix bumps the revision.
- **The command-line client and the browser client** are complete, and the browser client is live at the hosted site.
  What is open on it is polish, tracked in the [issues](https://github.com/jmoore2333/yokohama-assistant/issues).
- **The bot** that ships is `v4-step9-draft1`, the fourth version (V4), in the terminal and in the browser. In a match
  registered in advance it beat `v2-padded`, which plays as its predecessor V2 does, with the whole interval above
  parity ([the results ladder](docs/computer-player.md#5-the-results-ladder)). Nothing has been tuned on, or measured
  against, a human player. It is production and provisional: the route to the bar, below, runs through a later freeze
  that may name a different bot ([the computer player, §3](docs/computer-player.md#3-v4-production-and-provisional)).
- **The bar.** The milestone bar, set in §1.9 of the [Phase-3 map](docs/plans/2026-07-25-phase3-map.md), asks at two
  players for a constructed scenario suite, statistical superiority over the preserved baseline players, an explanation
  for every move, and the owner's acceptance play through the CLI
  ([the computer player, §6](docs/computer-player.md#6-the-bar-not-claimed)).
  The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.
- **Next:** V4 is vetted in played games, then the next version, V5, opens under a pre-registration of its own
  ([the computer player, §7](docs/computer-player.md#7-what-is-next)).

[`guidingpath.md`](guidingpath.md) is the authority: the mission, the current state, the roadmap and the decision log.
It outranks every other page, this one included.

## Contributing · Security · Licence

- **Contributing:** how a change lands, the suites and the rules sources are in [`CONTRIBUTING.md`](CONTRIBUTING.md).
- **Security:** report a vulnerability privately, as [`SECURITY.md`](SECURITY.md) describes.
- **Licence:** the code is Apache-2.0 ([`LICENSE`](LICENSE), [`NOTICE`](NOTICE)); the game's components are the rights
  holder's, and [`ASSETS.md`](ASSETS.md) names them. This project is not affiliated with or endorsed by the game's
  publishers.
