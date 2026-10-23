import { describe, expect, it } from "vitest";
import { abandons, isSafeId, ID_MAX, route, safeReturnTo } from "./route";

// Spec §2.3's routing table, row by row. What each target then does is
// `handle`'s, and `test/worker.test.ts` holds it.
describe("route (spec §2.3)", () => {
  // The asset layer hands the Worker only a path it cannot find, or one under
  // `run_worker_first`; every such path outside `/api/` and `/enter` is 404.
  it.each(["/", "/index.html", "/assets/x.js", "/sw2.js", "/t/some-table", "/art/nope.webp", "/api", "/enter/", "/apix"])(
    "%s is missing",
    (path) => {
      expect(route("GET", path)).toEqual({ kind: "missing" });
    },
  );

  it("/enter is the turnstile for GET and HEAD, and not allowed otherwise", () => {
    expect(route("GET", "/enter")).toEqual({ kind: "enter" });
    expect(route("HEAD", "/enter")).toEqual({ kind: "enter" });
    expect(route("POST", "/enter")).toEqual({ kind: "not-allowed" });
    expect(route("DELETE", "/enter")).toEqual({ kind: "not-allowed" });
  });

  it("/api/art is the manifest for GET and HEAD, and not allowed otherwise", () => {
    expect(route("GET", "/api/art")).toEqual({ kind: "art" });
    expect(route("HEAD", "/api/art")).toEqual({ kind: "art" });
    expect(route("POST", "/api/art")).toEqual({ kind: "not-allowed" });
    expect(route("PUT", "/api/art")).toEqual({ kind: "not-allowed" });
  });

  it("POST /api/games is a create; any other method on it is the house's", () => {
    expect(route("POST", "/api/games")).toEqual({ kind: "create" });
    expect(route("GET", "/api/games")).toEqual({ kind: "house" });
    expect(route("PUT", "/api/games")).toEqual({ kind: "house" });
  });

  // The segment exactly as the layer binds it: raw, never percent-decoded.
  it("/api/games/{id} and below go to the table named by the segment", () => {
    expect(route("GET", "/api/games/wire")).toEqual({ kind: "table", name: "wire" });
    expect(route("DELETE", "/api/games/wire")).toEqual({ kind: "table", name: "wire" });
    expect(route("POST", "/api/games/wire/input")).toEqual({ kind: "table", name: "wire" });
    expect(route("GET", "/api/games/wire/events")).toEqual({ kind: "table", name: "wire" });
    expect(route("GET", "/api/games/wire/stream")).toEqual({ kind: "table", name: "wire" });
    expect(route("GET", "/api/games/wire/at/3")).toEqual({ kind: "table", name: "wire" });
    expect(route("GET", "/api/games/caf%C3%A9/table")).toEqual({ kind: "house" });
  });

  // Mutants: an empty name routed to a table object; the bound off by one.
  it("an empty segment, or one over 64 bytes, goes to the house", () => {
    expect(route("GET", "/api/games/")).toEqual({ kind: "house" });
    expect(route("GET", "/api/games//table")).toEqual({ kind: "house" });
    expect(route("GET", `/api/games/${"a".repeat(64)}/table`)).toEqual({ kind: "table", name: "a".repeat(64) });
    expect(route("GET", `/api/games/${"a".repeat(65)}/table`)).toEqual({ kind: "house" });
    expect(route("GET", `/api/games/${"a".repeat(1_100)}`)).toEqual({ kind: "house" });
  });

  // M6.5c2 cycle D (R-D4): a chat message's DELETE lives under the table too,
  // and it is an ordinary request, never the abandon. Mutants: the method
  // read alone (every DELETE the abandon — the chat's delete would take the
  // table down); the path read alone (a GET of the table the abandon).
  it("the abandon is DELETE on the table's own path, and nothing below it", () => {
    expect(abandons("DELETE", "/api/games/wire")).toBe(true);
    expect(abandons("DELETE", "/api/games/caf%C3%A9")).toBe(true);
    expect(abandons("DELETE", "/api/games/wire/chat/3")).toBe(false);
    expect(abandons("DELETE", "/api/games/wire/")).toBe(false);
    expect(abandons("GET", "/api/games/wire")).toBe(false);
    expect(abandons("POST", "/api/games/wire")).toBe(false);
    expect(route("DELETE", "/api/games/wire/chat/3")).toEqual({ kind: "table", name: "wire" });
    expect(route("DELETE", "/api/chat/3")).toEqual({ kind: "house" });
  });

  it("POST /api/archive is the archive's; any other method on it is the house's", () => {
    expect(route("POST", "/api/archive")).toEqual({ kind: "archive" });
    expect(route("GET", "/api/archive")).toEqual({ kind: "house" });
  });

  it("/api/admin and below are the operator's", () => {
    expect(route("GET", "/api/admin/tables")).toEqual({ kind: "admin", rest: "/tables" });
    expect(route("POST", "/api/admin/tables/wire/expire")).toEqual({ kind: "admin", rest: "/tables/wire/expire" });
    expect(route("GET", "/api/admin")).toEqual({ kind: "admin", rest: "" });
    expect(route("GET", "/api/administrator")).toEqual({ kind: "house" });
  });

  // The auth cycle, Task 4: sign-out is `POST /api/leave`, answered by the Worker itself. Mutants: the method not
  // read (a GET signs a player out from a link); a prefix compare (`/api/leaver` taken from the house).
  it("POST /api/leave is the sign-out", () => {
    expect(route("POST", "/api/leave")).toEqual({ kind: "leave" });
  });
  it("GET /api/leave, or any other method on it, is not allowed", () => {
    for (const method of ["GET", "HEAD", "PUT", "DELETE"]) expect(route(method, "/api/leave"), method).toEqual({ kind: "not-allowed" });
  });
  it("POST /api/leaver, or a path below /api/leave, is the house's", () => {
    expect(route("POST", "/api/leaver")).toEqual({ kind: "house" });
    expect(route("POST", "/api/leave/")).toEqual({ kind: "house" });
  });

  it.each([
    ["GET", "/api/health"],
    ["GET", "/api/me"],
    ["GET", "/api/labels"],
    ["GET", "/api/chat"],
    ["POST", "/api/chat"],
    ["GET", "/api/stream"],
    ["POST", "/api/stream"],
    ["GET", "/api/nope"],
    ["GET", "/api/art/"],
  ])("%s %s is the house's", (method, path) => {
    expect(route(method, path)).toEqual({ kind: "house" });
  });
});

// The project's types are workerd's: no node module and no `import.meta.url` (`fence.unit.test.ts` says why), so `node:fs`
// is imported through a variable specifier and typed here; the file runs in node (`npm run test:scripts`).
const FS = "node:fs";
const fs = (await import(/* @vite-ignore */ FS)) as { readFileSync(path: URL, encoding: "utf8"): string };

/** The store's verdicts, emitted by `wire_fixtures` (security S2a, D14): `[{ raw, safe }]`. */
const ID_CASES = JSON.parse(
  fs.readFileSync(new URL("../../web/src/wire/id_cases.json", (import.meta as unknown as { url: string }).url), "utf8"),
) as Array<{ raw: string; safe: boolean }>;

describe("isSafeId (security S2b, C2's name rule; decision D-11)", () => {
  it("agrees with the store's verdict on every committed row, both ways", () => {
    expect(ID_CASES.length).toBeGreaterThanOrEqual(39);
    for (const { raw, safe } of ID_CASES) expect(isSafeId(raw), JSON.stringify(raw)).toBe(safe);
    expect(ID_CASES.some((c) => c.safe) && ID_CASES.some((c) => !c.safe), "both verdicts are represented").toBe(true);
  });

  // Mutants: the leading-character test dropped (`_x` admitted); the suffix test case-sensitive (`x.TABLE`
  // admitted); the bound off by one (65 admitted); the alphabet widened to `%` (a percent-encoded segment admitted).
  it("refuses what the old routableName admitted and the store refuses: a space, a leading underscore, a reserved suffix, a percent escape, a dot segment", () => {
    for (const bad of ["a%20b", "_x", "x.table", "x.TABLE", "x.chat", "%C3%BC", "..", ".", "", "a".repeat(ID_MAX + 1)]) {
      expect(isSafeId(bad), JSON.stringify(bad)).toBe(false);
    }
    for (const good of ["sim-0123456789abcdef", "x", "evening.2", "x.tablet", "a".repeat(ID_MAX)]) expect(isSafeId(good), good).toBe(true);
    expect(ID_MAX).toBe(64);
  });

  it("routes a segment the store refuses to the house, raw and never decoded", () => {
    for (const bad of ["a%20b", "_x", "x.table", "%C3%BC", "..", "a".repeat(65)]) {
      expect(route("GET", `/api/games/${bad}/table`), bad).toEqual({ kind: "house" });
    }
    expect(route("GET", "/api/games/sim-0123456789abcdef/table")).toEqual({ kind: "table", name: "sim-0123456789abcdef" });
  });
});

// The auth cycle, Task 4 (rulings R-23, C-12): where a finished sign-in lands. Exactly the hash routes the client's
// `parseRoute` (`web/src/route.ts`) reads as somewhere other than the door, with a table's segment held RAW to the
// store's grammar; anything else is the lobby, never followed. Mutants: any string starting with `/` accepted (every
// refused row passes through); the segment percent-decoded first (`caf%C3%A9`'s neighbour `sim%2D1` kept); the
// segment held to a narrower alphabet than `isSafeId` (`sim.1` lost); a prefix compare on the fixed routes
// (`/#/lobbyx` kept).
describe("safeReturnTo (the auth cycle, Task 4; rulings R-23, C-12)", () => {
  it("keeps the lobby, the two forms and a table whose raw segment the store accepts", () => {
    for (const kept of ["/#/lobby", "/#/new", "/#/new/local", "/#/t/sim-1", "/#/t/sim.1", "/#/t/sim_1", `/#/t/${"a".repeat(ID_MAX)}`]) {
      expect(safeReturnTo(kept), kept).toBe(kept);
    }
  });
  it("lands anything else on the lobby: another origin, a plain path, an unsafe hash, nothing at all", () => {
    for (const refused of [
      null,
      "",
      "https://evil.test/#/t/x",
      "//evil.test/#/lobby",
      "/api/me",
      "/#/t/../x",
      "#/t/sim-1",
      "/#/t/x/y",
      "/#/t/",
      "/#/t/sim%2D1",
      "/#/t/caf%C3%A9",
      "/#/t/x.table",
      "/#/t/_x",
      `/#/t/${"a".repeat(ID_MAX + 1)}`,
      "/#/lobbyx",
      "/#/lobby?x=1",
      "/#/new/local/",
      "/#/",
      "/#/door",
      " /#/lobby",
    ]) {
      expect(safeReturnTo(refused), JSON.stringify(refused)).toBe("/#/lobby");
    }
  });
});
