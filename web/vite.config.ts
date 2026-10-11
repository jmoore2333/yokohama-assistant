import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import {
  BUILD_VALUES,
  checkProjectUrl,
  checkReleaseFeed,
  checkSiteKey,
  checkSiteUrl,
  checkText,
} from "./scripts/build-values.mjs";
import { headerSet, headersJson, styleHashesOf } from "./scripts/headers.mjs";
import { injectMetadata } from "./scripts/presence.mjs";

// The client is served from the SAME origin as the API in production (the Rust
// binary serves `dist/`), so there is no CORS story and no base URL to
// configure. In development Vite proxies `/api` to the running server, which
// keeps the fetch paths in `api.ts` identical in both modes — a client that
// needs a different base URL per environment is a client that will eventually
// ship pointing at the wrong one.
//
// `/art` is proxied for the same reason and is NOT a Vite `public/` directory:
// the cropped card art is the SERVER's asset, committed under
// `crates/yokohama-web/assets/art/`, and copying it into the client build would
// make two copies that drift. Both paths therefore resolve identically whether
// the client is served by Vite or by the Rust binary.
//
// **`/enter` is the third, and it is a real ROUTE rather than a screen**
// (M6.5b2, spec §2.1): the Access turnstile, which answers a 302 to `#/lobby`.
// Without it here, `npm run dev` falls through to `index.html` and the front
// door's "Go to the lobby" link lands back on the front door — the one
// control on that screen whose whole job is to leave it.
//
// **`define` carries the build's values, each through its own checker**
// (M6.5c2, R-6.5c2.6 and R-6.5c2.40; spec §2.3; cycle F, R-FP2): the door's
// project link, the blurb under its title and the one-line notice in the
// footer, and — cycle F's — the release feed the door and the lobby read, the
// Turnstile site key and the site's own address (R-F12). Vite hands only
// `VITE_`-prefixed variables to `import.meta.env`, so `YOKOHAMA_PROJECT_URL`,
// `YOKOHAMA_SITE_BLURB`, `YOKOHAMA_SITE_NOTICE`, `YOKOHAMA_RELEASE_FEED`,
// `YOKOHAMA_TURNSTILE_SITEKEY` and `YOKOHAMA_SITE_URL` — the names both hosts
// document, and
// `deploy/Dockerfile`'s `ARG`s — reach the bundle here, CHECKED by
// `scripts/build-values.mjs`: a throw in this file fails `vite build` naming
// the variable. Unset or empty is `null` (the feed's derived default aside,
// `checkReleaseFeed`), and nothing is drawn. The feed is checked AFTER the
// project link, whose GitHub repository is what an unset feed derives from.
// `__YOKOHAMA_VERSION__` is no variable: it is `package.json`'s `version`,
// which the workspace cell `the_two_npm_packages_carry_the_workspace_version`
// holds to the crates'. `vitest.config.ts` merges this file, so the component
// suite reads the same values — as globals there (measured).
const projectUrl = checkProjectUrl(process.env.YOKOHAMA_PROJECT_URL);
const siteBlurb = checkText("YOKOHAMA_SITE_BLURB", process.env.YOKOHAMA_SITE_BLURB, BUILD_VALUES.YOKOHAMA_SITE_BLURB);
const siteNotice = checkText("YOKOHAMA_SITE_NOTICE", process.env.YOKOHAMA_SITE_NOTICE, BUILD_VALUES.YOKOHAMA_SITE_NOTICE);
const releaseFeed = checkReleaseFeed(process.env.YOKOHAMA_RELEASE_FEED, projectUrl);
const siteKey = checkSiteKey(process.env.YOKOHAMA_TURNSTILE_SITEKEY);
const siteUrl = checkSiteUrl(process.env.YOKOHAMA_SITE_URL);
const { version } = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as {
  version: string;
};

// **The build-values stamp** (cycle F, R-FP3; spec §2.6): `dist/build-values.json`,
// six booleans naming which build values this build carries — never a value —
// read off the SAME checked locals the `define` block below reads, so the stamp
// cannot say one thing while the bundle says another. The Worker's deploy guard
// holds its `YOKOHAMA_TURNSTILE` declaration to `turnstile_sitekey` (its check
// 12): one client build per door state (R-FP4). Written after the bundle, into
// the output directory, so it lands in the Worker's assets tree with the rest
// of `dist`; `scripts/precache.mjs` leaves it out of the release's list. A
// plain file both hosts serve: it names which values were set, which the
// bundle shows anyway. `site_url` is true exactly when the build carries the
// site's address (R-F12), which is what the sitemap, the canonical link and
// `og:url` are written from. The same plugin writes `dist/headers.json` beside
// it (security S2a; below), and `scripts/precache.mjs` leaves that out too.
const stamp = {
  project_url: projectUrl !== null,
  blurb: siteBlurb !== null,
  notice: siteNotice !== null,
  release_feed: releaseFeed !== null,
  site_url: siteUrl !== null,
  turnstile_sitekey: siteKey !== null,
};
// **The header set** (security S2a, decision D-1): the policy and its five companions, from the SAME checked
// locals the `define` block reads — the site's socket host and the feed's origin in `connect-src`, the 404
// page's inline style hashed into `style-src` — written to `dist/headers.json` beside the stamp. The Worker's
// assembly appends it to `_headers`; the NAS reads it at start. Neither host spells a directive of its own.
const headers = headerSet({
  siteUrl,
  releaseFeed,
  styleHashes: styleHashesOf(readFileSync(new URL("./public/404.html", import.meta.url), "utf8")),
});
// The one plugin writes both files after the bundle: the stamp, then the header set.
function stampBuildValues(): Plugin {
  let outDir = "";
  return {
    name: "yokohama-build-values-stamp-and-headers",
    apply: "build",
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    writeBundle() {
      writeFileSync(join(outDir, "build-values.json"), `${JSON.stringify(stamp, null, 2)}\n`);
      writeFileSync(join(outDir, "headers.json"), headersJson(headers));
    },
  };
}

// **The page's metadata** (cycle F, R-F12; spec §2.7): the description, Open
// Graph, the Twitter card and the JSON-LD block, written into the shell at
// build by replacing `index.html`'s one marker with
// `scripts/presence.mjs`'s `metadataTags` — from the SAME checked locals the
// `define` block reads, so the shell cannot say one thing while the bundle
// says another. The canonical link and `og:url` exist only with the site's
// address and the repository only with the project link. `order: "pre"` so the
// marker is read before Vite's own HTML passes touch the file, and a shell with
// no marker (or two) stops the build. Build only: the dev server's page has no
// canonical address to claim.
function presenceMetadata(): Plugin {
  return {
    name: "yokohama-presence-metadata",
    apply: "build",
    transformIndexHtml: {
      order: "pre",
      handler: (html) => injectMetadata(html, { siteUrl, blurb: siteBlurb, projectUrl }),
    },
  };
}

export default defineConfig({
  plugins: [react(), stampBuildValues(), presenceMetadata()],
  define: {
    __YOKOHAMA_PROJECT_URL__: JSON.stringify(projectUrl),
    __YOKOHAMA_SITE_BLURB__: JSON.stringify(siteBlurb),
    __YOKOHAMA_SITE_NOTICE__: JSON.stringify(siteNotice),
    __YOKOHAMA_RELEASE_FEED__: JSON.stringify(releaseFeed),
    __YOKOHAMA_TURNSTILE_SITEKEY__: JSON.stringify(siteKey),
    __YOKOHAMA_SITE_URL__: JSON.stringify(siteUrl),
    __YOKOHAMA_VERSION__: JSON.stringify(version),
  },
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    proxy: {
      // **`ws: true`** (M6.5c2): the Worker's transport is a WebSocket on the
      // same `/api` paths, and a proxy without it never forwards the upgrade.
      "/api": { target: "http://127.0.0.1:8080", changeOrigin: false, ws: true },
      "/art": { target: "http://127.0.0.1:8080", changeOrigin: false },
      "/enter": { target: "http://127.0.0.1:8080", changeOrigin: false },
    },
  },
});
