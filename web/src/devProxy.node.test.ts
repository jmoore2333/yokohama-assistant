// The dev server's proxy (M6.5c2 spec §2.8; the M6.5c Task 8 review's
// escalation 4): `/api` must carry the Worker's WebSocket UPGRADES as well as
// its fetches, or a dev loop pointed at a local Worker (`wrangler dev`) sits
// on "reconnecting…" for ever. Harmless against the NAS, which says "sse" and
// opens no socket.
//
// A node cell, because it reads the BUILD config, which the jsdom project
// merges and never inspects; `npm run test:wasm` runs it, and it needs no
// module.

import { describe, expect, it } from "vitest";
import type { UserConfig, UserConfigExport } from "vite";
import config from "../vite.config";

describe("the dev server's proxy", () => {
  it("proxies /api's WebSocket upgrades as well as its requests", async () => {
    const exported = config as UserConfigExport;
    const resolved: UserConfig =
      typeof exported === "function"
        ? await exported({ command: "serve", mode: "development" })
        : await exported;
    expect(resolved.server?.proxy?.["/api"]).toMatchObject({
      target: "http://127.0.0.1:8080",
      ws: true,
    });
  });
});
