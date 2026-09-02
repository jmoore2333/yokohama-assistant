#!/usr/bin/env python3
"""Draw the M5 tile overlay rects onto every area art, for a human to look at.

    python3 docs/tools/tilerects.py      # writes docs/temp/web-review/tile-rects.png

Reads `web/src/tile_rects.json` -- the SAME file the client draws from -- and
the thirteen non-Canal area crops, and writes one contact sheet: the printed
regions in red, the site card in magenta, the pieces band in green, the id
badge in cyan. Nothing here asserts anything; the assertion is the owner
looking at it (M5 spec §4.1, gate V0). The output goes under the gitignored
`docs/temp/` because a picture of the art is the art (`ASSETS.md`).
"""
from __future__ import annotations

import json
import pathlib
import sys

try:
    from PIL import Image, ImageDraw
except ImportError:  # pragma: no cover
    sys.exit("this script needs Pillow:  python3 -m pip install pillow")

ROOT = pathlib.Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "yokohama-web" / "assets" / "art"
OUT = ROOT / "docs" / "temp" / "web-review" / "tile-rects.png"
RECTS = json.loads((ROOT / "web" / "src" / "tile_rects.json").read_text())
PRINTED = (255, 64, 64)
OVERLAY = {"site_card": (255, 0, 255), "pieces": (0, 255, 0), "id_badge": (0, 220, 255)}


def box(draw, r, w, h, colour, width):
    draw.rectangle(
        [r["left"] * w, r["top"] * h, (r["left"] + r["width"]) * w, (r["top"] + r["height"]) * h],
        outline=colour,
        width=width,
    )


def main() -> None:
    manifest = json.loads((ART / "manifest.json").read_text())
    faces = [(k, ART / e["file"]) for k, e in sorted(manifest["areas"].items()) if k != "Canal"]
    tiles = []
    for name, path in faces:
        im = Image.open(path).convert("RGB")
        w, h = im.size
        d = ImageDraw.Draw(im)
        for r in RECTS["printed"].values():
            box(d, r, w, h, PRINTED, 2)
        for key, r in RECTS["overlays"].items():
            box(d, r, w, h, OVERLAY[key], 4)
        d.text((8, h - 20), name, fill=(255, 255, 255))
        tiles.append(im)
    w, h = tiles[0].size
    cols, rows = 4, (len(tiles) + 3) // 4
    sheet = Image.new("RGB", (cols * w, rows * h), (20, 20, 20))
    for i, im in enumerate(tiles):
        sheet.paste(im, ((i % cols) * w, (i // cols) * h))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(OUT)
    print(f"wrote {OUT.relative_to(ROOT)} ({len(tiles)} arts, {sheet.size[0]}x{sheet.size[1]})")


if __name__ == "__main__":
    main()
