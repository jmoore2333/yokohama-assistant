# The documents

One line per document a reader may need, grouped by what you came for. The [root README](../README.md) says what the
project is. A test in the web client checks that every relative link on this page leads to a file or folder in the
tree, and every anchor to a heading in the file it points at.

## The guides

- [`cli-guide.md`](cli-guide.md), with its word-identical [`cli-guide.html`](cli-guide.html) twin: the command-line
  game driver and live tracker, the authoritative feature documentation.
- [`web-guide.md`](web-guide.md): the browser client, for players, hosts and readers.
- [`computer-player.md`](computer-player.md): the bot, the names it plays under and how strong it has been measured.
- [`bridge-setup.md`](bridge-setup.md): the Board Game Arena bridge, a read-only capture of the owner's own Board Game
  Arena sessions, translated by the CLI's `import`.
- [`forge-cli-testing.md`](forge-cli-testing.md): visual verification of the CLI in a real terminal session.
- [`suggest-walkthrough-2026-07-27.html`](suggest-walkthrough-2026-07-27.html): a dated walkthrough of the CLI's
  `suggest` command as of v0.3.3, kept as it was written.

## The authority

- [`guidingpath.md`](../guidingpath.md): the mission, the current state, the roadmap and the decision log, which
  carries every rules ruling. It outranks every other page.
- The log is append-only, and its older rows live verbatim in three archives that are closed to appending. An archived
  row stays authoritative wherever it is cited.
  - [`guidingpath-archive.md`](../guidingpath-archive.md): the engine era, 2026-07-16 to 2026-08-08.
  - [`guidingpath-archive-phase3.md`](../guidingpath-archive-phase3.md): the computer player's road to its fourth
    version, 2026-08-09 to 2026-08-30.
  - [`guidingpath-archive-web.md`](../guidingpath-archive-web.md): the web lane, 2026-08-31 to 2026-09-28.

## The board

- [`handover.md`](handover.md): the owner's task board, the tracks in flight and their next steps. It is a pointer that
  is rewritten at every session close and never outranks the authority.

## The method

- [`method.md`](method.md): the workflow, how a cycle lands, the decision log and the `SEMANTICS_REV` contract,
  pre-registration and the rules of evidence.

## The lane records

- [`web-lane.md`](web-lane.md): the web lane's operating record, its suites and gates, standing rules and traps.
- [`security-ledger.md`](security-ledger.md): one row per finding of the web-lane security review, with its status at
  every close.

## The hosts

- [`../deploy/README.md`](../deploy/README.md) and [`../deploy/RUNBOOK.md`](../deploy/RUNBOOK.md): the self-hosted
  binary and Docker image.
- [`../worker/README.md`](../worker/README.md) and [`../worker/RUNBOOK.md`](../worker/RUNBOOK.md): the Cloudflare host.

## The plans

[`plans/`](plans/) holds a dated file for every spec, plan and close, named `YYYY-MM-DD-topic.md`. Each is frozen when
its phase closes, and a correction is recorded forward in a later file. To read a cycle, start with its spec, then its
plan with the Review record that carries every correction, then its close, which is the record of record. The Phase-3
map, [`plans/2026-07-25-phase3-map.md`](plans/2026-07-25-phase3-map.md), governs the computer player's ladder, and
[`plans/2026-07-20-phase-2.9-ruling-list.md`](plans/2026-07-20-phase-2.9-ruling-list.md) is the register of the rules'
ambiguities, every one of them ruled.

## The knowledge base

- [`../analysis-1/yokohama-kb/00-README.md`](../analysis-1/yokohama-kb/00-README.md): the rules reference and the card
  data the engine was built from.

## The tools

Scripts that regenerate or check the tree's derived files, in [`tools/`](tools/):
- [`guidingpath-relocate.py`](tools/guidingpath-relocate.py): moves line ranges of the guiding path into archives
  verbatim and proves it byte for byte.
- [`mkguide.py`](tools/mkguide.py): regenerates the CLI guide's HTML from its markdown.
- [`mkcardart.py`](tools/mkcardart.py): crops the Tabletop Simulator sprite sheets into per-card art.
- [`mkhelpart.py`](tools/mkhelpart.py): re-cuts the help panel's screenshots to WebP.
- [`tilerects.py`](tools/tilerects.py): draws the web client's tile overlay rectangles onto the area art for review.
- [`tts-extraction/`](tools/tts-extraction/): the record and map of the Tabletop Simulator extraction.

## The community files

- [`CONTRIBUTING.md`](../CONTRIBUTING.md): how to build, test and propose a change.
- [`SECURITY.md`](../SECURITY.md): how to report a vulnerability.
- [`ASSETS.md`](../ASSETS.md): the third-party paths outside the Apache-2.0 grant.
