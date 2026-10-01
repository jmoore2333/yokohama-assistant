import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import {
  BUILD_VALUES,
  checkProjectUrl,
  checkReleaseFeed,
  checkSiteKey,
  checkText,
} from "./scripts/build-values.mjs";

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
// footer, and — cycle F's — the release feed the door and the lobby read and
// the Turnstile site key. Vite hands only `VITE_`-prefixed variables to
// `import.meta.env`, so `YOKOHAMA_PROJECT_URL`, `YOKOHAMA_SITE_BLURB`,
// `YOKOHAMA_SITE_NOTICE`, `YOKOHAMA_RELEASE_FEED` and
// `YOKOHAMA_TURNSTILE_SITEKEY` — the names both hosts document, and
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
const { version } = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as {
  version: string;
};

export default defineConfig({
  plugins: [react()],
  define: {
    __YOKOHAMA_PROJECT_URL__: JSON.stringify(projectUrl),
    __YOKOHAMA_SITE_BLURB__: JSON.stringify(siteBlurb),
    __YOKOHAMA_SITE_NOTICE__: JSON.stringify(siteNotice),
    __YOKOHAMA_RELEASE_FEED__: JSON.stringify(releaseFeed),
    __YOKOHAMA_TURNSTILE_SITEKEY__: JSON.stringify(siteKey),
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
