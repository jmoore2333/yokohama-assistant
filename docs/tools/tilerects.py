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


def is_pale(h: int, s: int, v: int) -> bool:
    """The pale-ground test the site-card rect was measured with (M5 spec §2)."""
    return v > 0.784 * 255 and s < 0.18 * 255


def icon_ink_right(im, band) -> float | None:
    """The rightmost printed-icon pixel inside the band's rows, as a fraction of
    the width, or None when the rows hold no icon.

    Only INK ON THE PALE PANEL counts: a non-pale pixel counts when a pale run
    of at least 3% of the width lies immediately to its right (inside x < 0.5).
    That is what separates an icon printed on the icon panel from the
    illustration, which is not pale and carries no data — the owner's
    criterion for the band (M5.5 spec R6: the rect may be entered, the ink may
    not be covered).

    THE NUMBER IS A CANDIDATE THE EYE MUST CONFIRM, never an answer: a pale
    illustration ground defeats the run test — on the thirteen area arts nine
    read illustration as ink this way (PortA's band rows are 55/66 pale sky) —
    so look at the band on every art before setting a value, and do not try to
    automate the eye (ruled during execution, 2026-09-04, recorded in the M5.5
    close; the reading is the eye's by M5.5 spec §3.1.2)."""
    w, h = im.size
    px = im.convert("HSV").load()
    y0, y1 = round(band["top"] * h), round((band["top"] + band["height"]) * h)
    needed = int(0.03 * w)
    right = None
    for y in range(y0, y1):
        run = 0
        for x in range(int(0.5 * w) - 1, -1, -1):
            if is_pale(*px[x, y]):
                run += 1
                continue
            if run >= needed:
                right = x / w if right is None else max(right, x / w)
                break
            run = 0
    return right


def main() -> None:
    ink_mode = "--ink" in sys.argv
    candidates = []
    for arg in sys.argv[1:]:
        if arg.startswith("--candidates="):
            candidates = [float(c) for c in arg.split("=", 1)[1].split(",")]
    band = RECTS["overlays"]["pieces"]
    manifest = json.loads((ART / "manifest.json").read_text())
    faces = [(k, ART / e["file"]) for k, e in sorted(manifest["areas"].items()) if k != "Canal"]
    tiles = []
    for name, path in faces:
        im = Image.open(path).convert("RGB")
        w, h = im.size
        d = ImageDraw.Draw(im)
        if ink_mode:
            edge = icon_ink_right(im, band)
            print(f"{name:18s} icon ink right edge in the band's rows: {edge if edge is None else f'{edge:.3f}'}")
        for c in candidates:
            for yy in range(0, h, 6):
                d.line([(c * w, yy), (c * w, yy + 3)], fill=(255, 255, 0), width=2)
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
