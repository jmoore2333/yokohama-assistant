import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { BUILD_VALUES, checkProjectUrl, checkText } from "./scripts/build-values.mjs";

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
// **`define` carries THREE build values, one checker** (M6.5c2, R-6.5c2.6 and
// R-6.5c2.40; spec §2.3): the door's project link, the blurb under its title
// and the one-line notice in the footer. Vite hands only `VITE_`-prefixed
// variables to `import.meta.env`, so `YOKOHAMA_PROJECT_URL`,
// `YOKOHAMA_SITE_BLURB` and `YOKOHAMA_SITE_NOTICE` — the names both hosts
// document, and `deploy/Dockerfile`'s `ARG`s — reach the bundle here, CHECKED
// by `scripts/build-values.mjs`: a throw in this file fails `vite build`
// naming the variable. Unset or empty is `null`, and nothing is drawn.
// `vitest.config.ts` merges this file, so the component suite reads the same
// values — as globals there (measured).
const projectUrl = checkProjectUrl(process.env.YOKOHAMA_PROJECT_URL);
const siteBlurb = checkText("YOKOHAMA_SITE_BLURB", process.env.YOKOHAMA_SITE_BLURB, BUILD_VALUES.YOKOHAMA_SITE_BLURB);
const siteNotice = checkText("YOKOHAMA_SITE_NOTICE", process.env.YOKOHAMA_SITE_NOTICE, BUILD_VALUES.YOKOHAMA_SITE_NOTICE);

export default defineConfig({
  plugins: [react()],
  define: {
    __YOKOHAMA_PROJECT_URL__: JSON.stringify(projectUrl),
    __YOKOHAMA_SITE_BLURB__: JSON.stringify(siteBlurb),
    __YOKOHAMA_SITE_NOTICE__: JSON.stringify(siteNotice),
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
