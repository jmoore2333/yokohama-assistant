# Yokohama Assistant

A rules engine, a CLI game tracker and a browser client for the board game Yokohama (Hisashi Hayashi, 2016), with a
computer player built to play to win. It is for people who play Yokohama and want a game checked move by move or a
bot to play against, and for people who build and measure game-playing programs.

[![Latest release](https://img.shields.io/github/v/release/jmoore2333/yokohama-assistant)](https://github.com/jmoore2333/yokohama-assistant/releases) [![Licence](https://img.shields.io/github/license/jmoore2333/yokohama-assistant)](LICENSE) [![Tests](https://github.com/jmoore2333/yokohama-assistant/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/jmoore2333/yokohama-assistant/actions/workflows/test.yml)

## Try it

Open [yokohama.moore.nyc](https://yokohama.moore.nyc) and choose **You against the bot**: the bot plays the whole game
in your browser, with no account.

## Quick start

### The command-line client

On an Apple-silicon Mac, fetch the release archive, unpack it and run the binary:

```bash
curl -fLO https://github.com/jmoore2333/yokohama-assistant/releases/latest/download/yokohama-cli-v0.7.2-macos-apple-silicon.tar.gz \
  && tar xzf yokohama-cli-v0.7.2-macos-apple-silicon.tar.gz \
  && cd yokohama-cli-v0.7.2-macos-apple-silicon && ./yokohama-cli
```

If the fetch fails, the release may not be published yet, and the `cargo run` path below works from a clone.

The newest build is always the one on the [Releases page](https://github.com/jmoore2333/yokohama-assistant/releases),
beside builds for Ubuntu x86_64 and Windows x86_64. Each archive carries the guide as `CLI-GUIDE.html` beside the
binary, and the Releases page also lists it on its own as `yokohama-cli-guide.html`. A browser download of the macOS
archive lands quarantined, and macOS then refuses the binary inside, which is ad-hoc signed rather than notarized: fetch
it with `curl` as above, or build it with `cargo`.

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

`sim` plays a game the engine deals, and `live` tracks a game on a physical table. To play the bot, answer `sim`, `2`,
`red`, `blue` and `0`, press Enter twice for a random seed and the default save file, keep a dealt card for each seat
with `1` and `1`, seat the bot with `1`, and type `confirm`. The game autosaves after every applied move, to
`yokohama-game-<seed>.json` in the directory you ran the CLI from, so the file appears with your first move and not
before. `abort` leaves the wizard and `q` leaves a game. The [CLI guide](docs/cli-guide.md) walks through every
question and command.

### The browser client, on your own machine

From a clone, with Docker running and `curl` installed:

```bash
bash deploy/run-local.sh
```

It builds the server's image from the checkout and runs it on this machine; on Apple silicon a first build runs under
emulation and takes minutes, and Docker prints a `WARNING` that the image's platform (linux/amd64) does not match the
host's when the container starts, which is expected because the image runs under emulation as it was built. It ends with these two lines:

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
[`CONTRIBUTING.md`](CONTRIBUTING.md) lists the per-edit suites and the lint gate, and links the web lane's.

## The browser client

The browser client is a second frontend over the same engine, never a second engine: the server routes every move
through the engine's legal list and computes nothing about the game, so the terminal and the browser, at the same rules
revision, cannot disagree about a rule. It runs at the hosted site, on your own server from one Docker image, or
entirely inside the browser against the bot, offline once installed. At a table you can play friends or the bot, watch a
game, chat, step back through the replay, and ask the bot what it would play and why it played what it did.

![A two-player game in the browser at the first turn: the area tiles with their site cards on the left and the
end-trigger strip along the top; on the right the phase ladder, the prompt for Red's additional actions with its two
numbered moves and an Undo button, and below them both players' yen, goods and assistants](worker/docs/img/game-desktop-player.webp)

The [browser guide](docs/web-guide.md) covers playing at the hosted site, asking the bot, and hosting your own.

## What is inside

- **The engine**, `crates/yokohama-core`: the rules as pure functions over a resumable game state (legal moves, checked
  application, scoring, seeded setup, saved records and strict replay), and the computer player built on them, which
  decides from its own seat's view and a guess at the hidden cards, never from the engine's private state.
- **The command-line client**, `crates/yokohama-cli`: a setup wizard and a prompt for games the engine deals, against
  the bot or between people, and for tracking a game on a physical table, with save, resume and undo, and `suggest`
  and `why`, the bot's explained advice for your move and its account of any move it made.
- **The oracle and the difftest harness**, `crates/yokohama-oracle` and `crates/yokohama-difftest`: an independent
  referee written from the rules sources alone, never from the engine's code, and the harness that compares the
  engine's legal moves and final scores against it.
- **The web layer**, `crates/yokohama-web`, with its WebAssembly build `crates/yokohama-wasm` and the Cloudflare
  Worker host `crates/yokohama-worker`; the browser client is in `web/` and the Worker's shell in `worker/`.
- **The data generator and the bridge**: `crates/gen-data` builds the card data from the knowledge base's YAML, and
  `crates/yokohama-bridge` is a read-only capture of the owner's own Board Game Arena sessions, translated by the
  CLI's `import`.

## Status, honestly

- The engine is complete and differentially verified against the oracle at `SEMANTICS_REV` 7, the rules revision
  every saved game is keyed to. One rules defect is known, tracked as
  [issue #62](https://github.com/jmoore2333/yokohama-assistant/issues/62): the Language School's yen can miss its own
  order's warehouse take, and its fix bumps the revision.
- The shipped bot is `v4-step9-draft1`, in the terminal and in the browser. It is production and provisional: the
  route to the bar runs through a later freeze that may name a different contestant
  ([the computer player, §3](docs/computer-player.md#3-v4-production-and-provisional)).
- The milestone bar, set in §1.9 of the [Phase-3 map](docs/plans/2026-07-25-phase3-map.md), asks at two players for
  a constructed scenario suite, statistical superiority over the preserved baseline players, an explanation for every
  move, and acceptance play through the CLI ([the computer player, §6](docs/computer-player.md#6-the-bar-not-claimed)).
  The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.
- Next: V4's vetting in played games, then V5 under its own pre-registration
  ([the computer player, §7](docs/computer-player.md#7-what-is-next)).

## The record

- [`guidingpath.md`](guidingpath.md), the authority: the mission, the current state, the roadmap and the decision log,
  which carries every rules ruling.
- [`docs/README.md`](docs/README.md), the index of the documents.
- [`docs/method.md`](docs/method.md), how the work is organized and its rules of evidence; the strength results and
  their records are in [the computer player, §5](docs/computer-player.md#5-the-results-ladder).
- [`docs/handover.md`](docs/handover.md), the board: the work in flight and its next steps.

## Contributing · Security · Licence

- **Contributing:** how a change lands, the suites and the rules sources are in [`CONTRIBUTING.md`](CONTRIBUTING.md).
- **Security:** report a vulnerability privately, as [`SECURITY.md`](SECURITY.md) describes.
- **Licence:** the code is Apache-2.0 ([`LICENSE`](LICENSE), [`NOTICE`](NOTICE)); the game's components are the rights
  holder's, and [`ASSETS.md`](ASSETS.md) names them. This project is not affiliated with or endorsed by the game's
  publishers.
