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
export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true },
  server: {
    proxy: {
      "/api": { target: "http://127.0.0.1:8080", changeOrigin: false },
      "/art": { target: "http://127.0.0.1:8080", changeOrigin: false },
    },
  },
});
