#!/usr/bin/env python3
"""Crop the TTS sprite sheets into per-card art for the web client.

    python3 docs/tools/mkcardart.py              # from the repo root
    python3 docs/tools/mkcardart.py --check      # verify, write nothing

Reads the sheets under `tts-assets/organized/` and writes one WebP per card,
plus `manifest.json`, into `crates/yokohama-web/assets/art/`. **It never writes
into `tts-assets/`** -- repo convention treats that tree as the owner's files.

WHY A SCRIPT AND NOT A BUILD STEP. The output is committed. A container image
that had to crop at build time would need Python, Pillow and the 20 MB of
sheets; a committed 1.5 MB of WebP needs none of them, and it is the same
reasoning `.gitignore`'s arena-corpus row gives in reverse -- the corpus
regenerates from a command and so does this, but only this one is needed at
RUNTIME, so only this one is committed. Re-run it when the sheets or the KB
YAMLs change.

THE JOIN KEYS ARE THE WIRE'S OWN IDENTIFIERS, and each was chosen rather than
defaulted:

  orders        `OrderCardId` -- which IS the number printed in the card's
                bottom-right corner (1-36), adjudicated 2026-07-17 because
                country+requirements does NOT identify a card uniquely.
  technologies  the printed TITLE (`TechCard.name`), NOT the registry's `codes`
                slug and NOT the physical card id. Ruled 2026-08-30 after an
                independent review: both copies of a doubled title print the
                same face, and `name` rides inside `Rules::content_hash` while
                the `codes` map is hashed NOWHERE -- keying on the slug would
                put an un-hashed field on a compatibility surface, where it
                could change without moving `rules_hash` and break the art join
                silently. (A `TechRegistry::code()` accessor was written for
                this and reverted. Do not re-add it.)
  sites         the 0-based index `screen_model::printed_site_index` returns,
                which is the position in `Rules::building_sites` and therefore
                in `09-building-sites.yaml`'s file order. It is also the number
                the CLI's tile prints as `Site n`, so the two surfaces name a
                site the same way.
  achievements  the `AchievementType` serde name.
  area tiles    the `AreaType` serde name.

CROP GEOMETRY. Every card sheet shares one 530x799 cell -- 4240/8, 3196/4 and
2650/5 are exact. The two 5-row sheets are the exception worth naming: 3996/5 =
799.2, so a row's top edge is `round(r * H / rows)` rather than `r * 799`, and
the last row would otherwise drift four pixels. The area sheets divide as
709x531 with the same rounding treatment on the odd axis.

The `position: "row,col"` fields in `analysis-1/yokohama-kb/0{7,8,9}-*.yaml` are
the coordinates, 1-based from the top-left. `gen-data` drops them as
"traceability only", so they are reachable from nowhere in Rust -- this script
reads the YAML directly, which is why it exists at all.

REQUIREMENTS: Pillow and PyYAML. Both are used only here; nothing in the
workspace's Rust build or test path touches either.

WHAT ENFORCES THE OUTPUT is not this script. `crates/yokohama-web/src/art.rs`
carries the tests: every engine identifier resolves to a manifest entry, every
manifest entry resolves to a file on disk, and no entry names an identifier the
engine does not have. Running this and not running those proves nothing.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys

try:
    from PIL import Image
except ImportError:  # pragma: no cover - a tooling problem, not a code path
    sys.exit("this script needs Pillow:  python3 -m pip install pillow")
try:
    import yaml
except ImportError:  # pragma: no cover
    sys.exit("this script needs PyYAML:  python3 -m pip install pyyaml")

ROOT = pathlib.Path(__file__).resolve().parents[2]
SHEETS = ROOT / "tts-assets" / "organized"
KB = ROOT / "analysis-1" / "yokohama-kb"
OUT = ROOT / "crates" / "yokohama-web" / "assets" / "art"

# Card art is displayed at most a few hundred CSS pixels wide; 360 is a 2x
# raster for a 180px card and reads cleanly at 1x on a phone. The height is
# COMPUTED from the source cell rather than typed, so the 530:799 aspect cannot
# drift by a typo.
CARD_W = 360
TILE_W = 440
QUALITY = 82

# --- the sheets, and how they divide -------------------------------------
#     name                                    cols rows
CARD_SHEETS = {
    "orders": ("cards/order_cards_8x5.jpg", 8, 5),
    "techs": ("cards/tech_cards_8x4.jpg", 8, 4),
    "sites": ("cards/building_site_cards_5x5.jpg", 5, 5),
    "achievement_A": ("cards/achievement_A_3x2.jpg", 3, 2),
    "achievement_B": ("cards/achievement_B_3x2.jpg", 3, 2),
    "achievement_C": ("cards/achievement_C_3x2.jpg", 3, 2),
}
AREA_SHEETS = {
    "base": ("boards/area_boards_base_3x4.jpg", 3, 4),
    "3plus": ("boards/area_boards_3plus_2x3.jpg", 2, 3),
    "4p": ("boards/area_boards_4p_2x3.jpg", 2, 3),
}
BACKS = {
    "order_back": "cards/order_cards_back.jpg",
    "tech_back": "cards/tech_cards_back.jpg",
    "site_back": "cards/building_site_cards_back.jpg",
    "area_back": "boards/area_boards_back.jpg",
}

# --- achievements: read off the sheets, cross-checked against the engine ---
#
# The three sheets are the three CATEGORIES: sheet A's cards all carry 8/6 VP,
# B's 9/7, C's 10/8, which is exactly what `AchievementType::category()` and the
# two `*_vp()` methods say. Each 3x2 sheet holds four faces and two backs, so
# the two right-hand cells of row 2 are skipped.
#
# The faces were identified by reading the printed condition on each cell: a
# number and the icon it counts.
#
# CATEGORY C IS THE ONE THAT CAN BE GOT BACKWARDS, and the first reading of it
# here WAS backwards. Those four cards name production and commercial only by
# two coloured badges, and the intuitive guess -- yellow for the production
# areas -- is wrong. The badges were resolved against the AREA TILE art
# instead, where the same two icons appear in the top-left corner of every
# tile, and they partition the engine's own predicates exactly:
#
#   YELLOW pen-nib     Port, Dock, Laboratory, Research Center, Bank, Church,
#                      Customs, Employment Agency, Chinatown  == is_commercial()
#   RED post           Silk Mill, Tea Plantation, Copper Mine, Fishing Ground
#                                                             == is_production()
#   neither            Canal
#
# So yellow is COMMERCIAL. The two mixed cards print commercial on the left and
# production on the right -- the reverse of the engine's own (production,
# commercial) tuple order, which is exactly why the mistake is easy to make.
ACHIEVEMENTS = {
    # sheet          (row, col)  AchievementType serde name
    "achievement_A": {(1, 1): "Tea7", (1, 2): "Copper5", (1, 3): "Silk6", (2, 1): "Seafood7"},
    "achievement_B": {
        (1, 1): "Technologies4",
        (1, 2): "Tokens5",
        (1, 3): "Orders5",
        (2, 1): "Money10",
    },
    "achievement_C": {
        (1, 1): "Prod2Comm3",  # 3 over yellow (commercial), 2 over red
        (1, 2): "Commercial4",  # 4 over yellow only
        (1, 3): "Prod3Comm2",  # 2 over yellow, 3 over red (production)
        (2, 1): "Production4",  # 4 over red only
    },
}

# --- area tiles: the mod's own deck listing, verified against the art ------
#
# `tts-assets/1840951801.json` name-tags every tile: TTS's `CardID` is
# `deck_id * 100 + row-major 0-based cell index`, so deck 9 lays out the base
# sheet, deck 10 the 3+ sheet and deck 11 the 4P sheet, each in reading order.
#
# TWO OF THE MOD'S NAMES ARE NOT THE ENGINE'S. The physical game prints a
# second Port and a second Laboratory under different names -- "Dock" and
# "Research Center" -- which the engine models as `PortB` and `LaboratoryB`.
# The count settles it: the engine's `full_tile_set()` is eighteen tiles and so
# is the mod's three decks, and they agree type-for-type only under this
# reading.
#
# The four PRODUCTION types print twice (once on the base sheet, once on the 3+
# or 4P sheet). Only one image per `AreaType` is emitted, since the two faces
# are the same area; `base` wins where a type appears on more than one sheet,
# and the losing cells are listed in `duplicates` in the manifest so the choice
# is visible rather than implicit.
AREA_TILES = {
    "base": {
        (1, 1): "PortA",
        (1, 2): "SilkMill",
        (1, 3): "TeaPlantation",
        (2, 1): "LaboratoryA",
        (2, 2): "Bank",
        (2, 3): "Church",
        (3, 1): "CopperMine",
        (3, 2): "Customs",
        (3, 3): "EmploymentAgency",
        (4, 1): "FishingGround",
    },
    "3plus": {(1, 1): "Canal", (1, 2): "Chinatown", (2, 1): "PortB", (2, 2): "FishingGround"},
    "4p": {
        (1, 1): "CopperMine",
        (1, 2): "LaboratoryB",
        (2, 1): "SilkMill",
        (2, 2): "TeaPlantation",
    },
}
AREA_SHEET_ORDER = ["base", "3plus", "4p"]

TOKENS = {
    "good_copper": "tokens/copper.jpg",
    "good_silk": "tokens/silk.jpg",
    "good_tea": "tokens/tea.jpg",
    "good_seafood": "tokens/fish.jpg",
    "good_imports": "tokens/imports.jpg",
    "station": "tokens/station.jpg",
    "yen_1": "tokens/1_yen.jpg",
    "yen_3": "tokens/3_yen.jpg",
}


def slug(title: str) -> str:
    """A filename for a printed title. The MANIFEST keys on the title itself --
    this only has to be stable and readable, never parsed back."""
    return re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")


def cell_box(size: tuple[int, int], cols: int, rows: int, r: int, c: int):
    """The pixel box of 1-based cell (r, c).

    Uniform floor-sized cells at floor origins — `(c-1) * (w // cols)`. Three
    sheets do not divide evenly (3996/5 = 799.2, 2128/3 = 709.33, 1594/3 =
    531.33), and the tempting reading is that the cells are fractional. THEY
    ARE NOT: the extra pixel is a black padding stripe on the far edge, and the
    cells are exactly 530x799 and 709x531 throughout.

    MEASURED, not assumed. A mean-absolute-difference scan along each axis puts
    the seam gradient at the floor index every time and one pixel away from the
    rounded one wherever the two differ:

        order 8x5   row seam 3   floor idx 2396 -> 3.9   round idx 2397 -> 1.0
        order 8x5   row seam 4   floor idx 3195 -> 45.8  round idx 3196 -> 4.2
        base 3x4    col seam 2   floor idx 1417 -> 67.3  round idx 1418 -> 11.1
        3plus 2x3   row seam 2   floor idx 1061 -> 38.0  round idx 1062 -> 8.9

    with the far edge reading 164-238 in every case — the step into the black
    stripe. Rounded origins therefore shift the last row or column one pixel
    late AND pull the stripe into the crop. TTS's own `NumWidth`/`NumHeight`
    and the 709x531 deck-back file corroborate the cell size independently.
    """
    w, h = size
    cw, ch = w // cols, h // rows
    return ((c - 1) * cw, (r - 1) * ch, c * cw, r * ch)


class Emitter:
    def __init__(self, check_only: bool) -> None:
        self.check_only = check_only
        self.written = 0
        self.bytes = 0

    def emit(self, image: Image.Image, target_w: int, rel: str) -> dict:
        """Scale to `target_w`, preserving the source aspect exactly, and write."""
        w, h = image.size
        target_h = round(h * target_w / w)
        out = image.resize((target_w, target_h), Image.LANCZOS)
        path = OUT / rel
        if not self.check_only:
            path.parent.mkdir(parents=True, exist_ok=True)
            out.save(path, "WEBP", quality=QUALITY, method=6)
            self.written += 1
            self.bytes += path.stat().st_size
        return {"file": rel, "w": target_w, "h": target_h}


def load_sheet(rel: str) -> Image.Image:
    path = SHEETS / rel
    if not path.exists():
        sys.exit(f"missing sheet: {path}")
    return Image.open(path).convert("RGB")


def main() -> None:
    check_only = "--check" in sys.argv[1:]
    em = Emitter(check_only)
    manifest: dict = {
        "provenance": {
            "sheets": "tts-assets/organized/",
            "positions": "analysis-1/yokohama-kb/0{7,8,9}-*.yaml",
            "generator": "docs/tools/mkcardart.py",
            "card_width": CARD_W,
            "tile_width": TILE_W,
            "webp_quality": QUALITY,
        }
    }

    # --- orders -----------------------------------------------------------
    sheet = load_sheet(CARD_SHEETS["orders"][0])
    cols, rows = CARD_SHEETS["orders"][1:]
    orders = {}
    for card in yaml.safe_load((KB / "07-order-cards.yaml").read_text())["order_cards"]:
        r, c = (int(x) for x in card["position"].split(","))
        crop = sheet.crop(cell_box(sheet.size, cols, rows, r, c))
        orders[str(card["id"])] = em.emit(crop, CARD_W, f"orders/O{card['id']:02d}.webp")
    manifest["orders"] = orders

    # --- technologies -----------------------------------------------------
    #
    # `positions` lists one cell per PHYSICAL copy and the copies are identical
    # faces, so the first is the art for the title. Taking the first rather
    # than "any" keeps a re-run byte-identical.
    sheet = load_sheet(CARD_SHEETS["techs"][0])
    cols, rows = CARD_SHEETS["techs"][1:]
    techs = {}
    for card in yaml.safe_load((KB / "08-tech-cards.yaml").read_text())["tech_cards"]:
        r, c = (int(x) for x in card["positions"][0].split(","))
        crop = sheet.crop(cell_box(sheet.size, cols, rows, r, c))
        techs[card["name"]] = em.emit(crop, CARD_W, f"techs/{slug(card['name'])}.webp")
    manifest["techs"] = techs

    # --- building sites ---------------------------------------------------
    sheet = load_sheet(CARD_SHEETS["sites"][0])
    cols, rows = CARD_SHEETS["sites"][1:]
    sites = {}
    for i, card in enumerate(yaml.safe_load((KB / "09-building-sites.yaml").read_text())["building_sites"]):
        r, c = (int(x) for x in card["position"].split(","))
        crop = sheet.crop(cell_box(sheet.size, cols, rows, r, c))
        sites[str(i)] = em.emit(crop, CARD_W, f"sites/S{i:02d}.webp")
    manifest["sites"] = sites

    # --- achievements -----------------------------------------------------
    achievements = {}
    for name, cells in ACHIEVEMENTS.items():
        sheet = load_sheet(CARD_SHEETS[name][0])
        cols, rows = CARD_SHEETS[name][1:]
        for (r, c), ty in cells.items():
            crop = sheet.crop(cell_box(sheet.size, cols, rows, r, c))
            achievements[ty] = em.emit(crop, CARD_W, f"achievements/{ty}.webp")
    manifest["achievements"] = achievements

    # --- area tiles -------------------------------------------------------
    areas: dict = {}
    duplicates: list = []
    for name in AREA_SHEET_ORDER:
        sheet = load_sheet(AREA_SHEETS[name][0])
        cols, rows = AREA_SHEETS[name][1:]
        for (r, c), ty in AREA_TILES[name].items():
            if ty in areas:
                duplicates.append({"area_type": ty, "sheet": name, "row": r, "col": c})
                continue
            crop = sheet.crop(cell_box(sheet.size, cols, rows, r, c))
            areas[ty] = em.emit(crop, TILE_W, f"areas/{ty}.webp")
    manifest["areas"] = areas
    manifest["area_duplicates"] = duplicates

    # --- backs and loose tokens -------------------------------------------
    backs = {}
    for key, rel in BACKS.items():
        width = TILE_W if key == "area_back" else CARD_W
        backs[key] = em.emit(load_sheet(rel), width, f"backs/{key}.webp")
    manifest["backs"] = backs

    tokens = {}
    for key, rel in TOKENS.items():
        tokens[key] = em.emit(load_sheet(rel), 128, f"tokens/{key}.webp")
    manifest["tokens"] = tokens

    if not check_only:
        (OUT / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")

    verb = "would write" if check_only else "wrote"
    print(
        f"{verb} {em.written or sum(len(v) for k, v in manifest.items() if isinstance(v, dict) and k != 'provenance')} "
        f"images ({em.bytes / 1024:.0f} KiB) to {OUT.relative_to(ROOT)}"
    )
    print(
        f"  orders {len(manifest['orders'])} · techs {len(manifest['techs'])} · "
        f"sites {len(manifest['sites'])} · achievements {len(manifest['achievements'])} · "
        f"areas {len(manifest['areas'])} (+{len(duplicates)} duplicate cells skipped) · "
        f"backs {len(manifest['backs'])} · tokens {len(manifest['tokens'])}"
    )


if __name__ == "__main__":
    main()
