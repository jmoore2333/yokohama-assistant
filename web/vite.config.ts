import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

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
// door's "Play with others" link lands back on the front door — the one
// control on that screen whose whole job is to leave it.
export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    proxy: {
      "/api": { target: "http://127.0.0.1:8080", changeOrigin: false },
      "/art": { target: "http://127.0.0.1:8080", changeOrigin: false },
      "/enter": { target: "http://127.0.0.1:8080", changeOrigin: false },
    },
  },
});
