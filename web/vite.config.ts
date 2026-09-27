import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { checkProjectUrl } from "./scripts/project-url.mjs";

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
// **`define` carries ONE build value, the door's project link** (M6.5c2,
// R-6.5c2.6; spec §2.3). Vite hands only `VITE_`-prefixed variables to
// `import.meta.env`, so `YOKOHAMA_PROJECT_URL` — the name both hosts will
// document, and `deploy/Dockerfile`'s `ARG` today — reaches the bundle here,
// CHECKED: `checkProjectUrl` throws on a bad value, and a throw in this file
// fails `vite build` naming the variable. Unset or empty is `null`, and the
// door draws no link. `vitest.config.ts` merges this file, so the component
// suite reads the same value — as a global there (measured).
const projectUrl = checkProjectUrl(process.env.YOKOHAMA_PROJECT_URL);

export default defineConfig({
  plugins: [react()],
  define: { __YOKOHAMA_PROJECT_URL__: JSON.stringify(projectUrl) },
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    proxy: {
      "/api": { target: "http://127.0.0.1:8080", changeOrigin: false },
      "/art": { target: "http://127.0.0.1:8080", changeOrigin: false },
      "/enter": { target: "http://127.0.0.1:8080", changeOrigin: false },
    },
  },
});
