// **The app icons, rasterized from the SVG sources beside them** (M6.5b2,
// R-6.5b2.9; spec §2.4).
//
// `web/public/icons/icon.svg` and `icon-maskable.svg` are the sources and the
// PNGs are COMMITTED, for `docs/tools/mkcardart.py`'s reason read one step
// further: a browser needs the rasters at install time, a container should need
// neither a toolchain nor a headless browser to produce them, and the manifest
// names them by path.
//
// Playwright's Chromium rather than a rasterizer library, because this tree
// already installs one for the browser walk and adding an image dependency to
// ship four files would be the larger cost. It is NOT run by any build or gate:
// run it by hand when the mark changes, and commit what it writes.
//
//     cd web && node scripts/mkicons.mjs
//
// **Look at what it produced.** The cycle before this one recorded the lesson
// twice: `art.rs`'s tests prove the art join is TOTAL and not TRUE, and two
// wrong-but-complete mappings were caught only by looking at the pictures.

import { chromium } from "@playwright/test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const icons = fileURLToPath(new URL("../public/icons/", import.meta.url));

/** Which raster comes from which source, and at what size. */
const WANTED = [
  { from: "icon.svg", to: "icon-192.png", size: 192 },
  { from: "icon.svg", to: "icon-512.png", size: 512 },
  { from: "icon.svg", to: "apple-touch-icon-180.png", size: 180 },
  { from: "icon-maskable.svg", to: "icon-maskable-512.png", size: 512 },
];

const browser = await chromium.launch();
try {
  for (const { from, to, size } of WANTED) {
    const svg = readFileSync(join(icons, from), "utf8");
    const page = await browser.newPage({ viewport: { width: size, height: size } });
    // The SVG inlined at the exact pixel size, on a page with no margin: the
    // screenshot is then the icon and not a crop of a document.
    await page.setContent(
      `<!doctype html><meta charset="utf-8">` +
        `<style>html,body{margin:0;padding:0;background:#0d1013}` +
        `svg{display:block;width:${size}px;height:${size}px}</style>` +
        svg.replace(/<\?xml[^>]*\?>/, "").replace(/<!--[\s\S]*?-->/g, ""),
    );
    const png = await page.screenshot({ type: "png" });
    writeFileSync(join(icons, to), png);
    await page.close();
    // eslint-disable-next-line no-console
    console.log(`${to}: ${size}x${size}, ${png.length} B, from ${from}`);
  }
} finally {
  await browser.close();
}
