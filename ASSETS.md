# Third-party game assets — carve-out from the Apache-2.0 license

The code in this repository is licensed under the Apache License, Version 2.0
(see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).

**The paths listed below are NOT.** They are components of a published
commercial board game, they are not the copyright holder's to relicense, and
**no license to them is granted or implied** — not by the Apache-2.0 grant on
the code, not by their presence in this repository, and not by any statement
in this file.

## The excluded paths

| Path | Contents | Scale | Provenance |
|------|----------|-------|------------|
| `tts-assets/` | Source sprite sheets and scans — card fronts and backs, area-board tiles, tokens, the rulebook PDF | 118 tracked files, ~60 MB (84 `.jpg`, 28 `.png`, 2 `.pdf`, plus an extraction summary, a mapping JSON and one extraction script) | Extracted 2025-12-07 from the Tabletop Simulator community mod for Yokohama (Steam Workshop id 1840951801); see `tts-assets/EXTRACTION_SUMMARY.md` |
| `crates/yokohama-web/assets/art/` | Derived crops of the above, cut into per-item images (`orders/`, `techs/`, `sites/`, `areas/`, `tokens/`, `achievements/`, `five-power/`, `mercenaries/`, `backs/`) plus `manifest.json` | 132 tracked files, ~3.0 MB (131 `.webp` + the manifest) | Added 2026-08-31; produced from `tts-assets/` by this repository's own tooling. Being derived does not launder the underlying copyright — these are excluded on exactly the same footing as their source |
| `analysis-1/00-yokohama-rulebook.pdf` | The published English rulebook, complete | 1 file, ~6.5 MiB | Publisher's rulebook, used as the rules source of record for the engine |
| `web/public/help/` | Screenshots of the running browser client, used by its in-app Help panel and by `README.md` | 4 `.png`, ~2 MB | Added 2026-09-01; generated from the real application by `web/e2e/shots.spec.ts`. **The layout, typography and every word in them are this repository's own work — the card and tile images inside them are not**, being the `crates/yokohama-web/assets/art/` crops as the browser draws them. A photograph of excluded material is excluded material, so these sit here on the same footing rather than on the code's |

The extraction script (`tts-assets/parse_tts_mod.py`) and the manifest
(`crates/yokohama-web/assets/art/manifest.json`) are this repository's own
work and are Apache-2.0 like the rest of the code; they are listed above only
because they sit inside excluded directories. Everything else in those four
paths is third-party material.

The screenshot GENERATOR (`web/e2e/shots.spec.ts`) is likewise this
repository's own work and is Apache-2.0; only its output is carved out. And
the output is genuinely reproducible without it: the client renders its full
UI with no art at all when the manifest is absent — a supported state its own
code documents — so a fork that drops these paths can regenerate art-free
screenshots rather than losing the help panel.

## Whose they are

The game is **Yokohama**, designed by **Hisashi Hayashi**, published in 2016 by
**Tasty Minstrel Games** and **Okazu Brand**. All artwork, card text, board
art, iconography, and rulebook text in the paths above are the property of
their respective rights holders. This project is unaffiliated with, and not
endorsed by, the designer or either publisher.

## Why they are here

The repository owner owns physical copies of the game. The assets are present
so that a tracker and study tool for games the owner actually plays can show
the same cards and tiles that are on the table, and so that the engine's rules
implementation can be checked against the rulebook it is implementing. They
are here for that personal use, on a repository that is private and
invite-only.

This is a statement of the situation, not a legal conclusion about it. Nothing
here is an assertion that any particular use is permitted, and nothing here is
legal advice.

## If you fork, copy, or redistribute this repository

You must **remove the paths listed above**, or obtain your own permission from
the rights holders. The Apache-2.0 license on the code gives you nothing with
respect to them. Redistributing the code is fine; redistributing the game's
art and rulebook along with it is not something this repository can authorize.

The code does not require the assets to build or to run: the CLI has no asset
dependency at all, and the web frontend's art layer is a display enhancement
over the same engine state.

## Note for a future reader: the planned public split

A public release of this project is planned as a separate repository, and
**these paths are the reason it has not happened yet.** A public split takes
the code, the docs, and the derived rules data, and leaves `tts-assets/`,
`crates/yokohama-web/assets/art/`, `web/public/help/`, and the rulebook PDF
behind — either dropped entirely, or replaced with placeholder art, a rulebook
link, and screenshots re-cut with the art layer absent. Doing that
cleanly means the split repository is a fresh history, not a re-push of this
one, because these files are tracked from commit 32 onward and a shared history
would carry them into the public tree regardless of what the tip commit
contains.

## Adjacent material, stated plainly

`analysis-1/yokohama-kb/` holds rules references and card-data YAML written for
this project. The prose and the data structures are this repository's own work
and are Apache-2.0 with the rest of the tree; the game rules and card text they
describe are the publisher's. This file records that fact rather than
adjudicating it.
