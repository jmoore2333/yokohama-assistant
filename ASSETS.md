# Third-party material

The code in this repository is Apache-2.0 (`LICENSE`, `NOTICE`). These paths are not:

- `crates/yokohama-web/assets/art/` — crops of the game's cards, tiles and tokens, and `manifest.json`
- `web/public/help/`, `worker/docs/img/` and the README's pictures of the client where they show that art
  (`docs/images/` holds text renders of the terminal and carries none)

They are the artwork of Yokohama, designed by Hisashi Hayashi and published in 2016 by Okazu Brand
and Tasty Minstrel Games, or Asmodee. This project is independent of the designer and the publishers
and not endorsed by them. These assets are used so that people can more easily work to develop a
computer player for this great board game. Buy a copy: the game was recently reprinted with new art
and is at its best at a table.

A fork carries these paths with the same standing; a crate whose package would bundle them declares
`publish = false`.

Two typefaces ship with the browser client under the SIL Open Font License 1.1, each with its licence
text beside its files: Fraunces (`web/src/fonts/fraunces/`, served copy `web/public/fonts/fraunces/`)
and JetBrains Mono (`web/src/fonts/jetbrains-mono/`, served copy `web/public/fonts/jetbrains-mono/`).

The rules references and card data under `analysis-1/yokohama-kb/` are this repository's own prose
and data structures; the rules and card text they transcribe are the game's.
