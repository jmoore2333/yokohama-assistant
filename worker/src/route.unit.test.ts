import { describe, expect, it } from "vitest";
import { routableName, route, TABLE_NAME_MAX_BYTES } from "./route";

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
    expect(route("GET", "/api/games/caf%C3%A9/table")).toEqual({ kind: "table", name: "caf%C3%A9" });
  });

  // Mutants: an empty name routed to a table object; the bound off by one.
  it("an empty segment, or one over 64 bytes, goes to the house", () => {
    expect(route("GET", "/api/games/")).toEqual({ kind: "house" });
    expect(route("GET", "/api/games//table")).toEqual({ kind: "house" });
    expect(route("GET", `/api/games/${"a".repeat(64)}/table`)).toEqual({ kind: "table", name: "a".repeat(64) });
    expect(route("GET", `/api/games/${"a".repeat(65)}/table`)).toEqual({ kind: "house" });
    expect(route("GET", `/api/games/${"a".repeat(1_100)}`)).toEqual({ kind: "house" });
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

describe("routableName", () => {
  // Mutant: the bound read in characters rather than bytes — a path segment is
  // percent-encoded ASCII, but a create body's `id` may be anything.
  it("is a non-empty name of at most 64 UTF-8 bytes", () => {
    expect(TABLE_NAME_MAX_BYTES).toBe(64);
    expect(routableName("wire")).toBe(true);
    expect(routableName("a".repeat(64))).toBe(true);
    expect(routableName("")).toBe(false);
    expect(routableName("a".repeat(65))).toBe(false);
    // 33 characters, 66 bytes: the bound is in bytes, as the layer's is.
    expect(routableName("é".repeat(32))).toBe(true);
    expect(routableName("é".repeat(33))).toBe(false);
  });
  // A lone surrogate can arrive in a create body's `id` (`"\ud800"`), and no
  // percent-encoding or UTF-8 can carry it; the layer refuses the body anyway.
  it("refuses a name holding a lone surrogate", () => {
    expect(routableName("a\ud800")).toBe(false);
    expect(routableName("\udc00a")).toBe(false);
    expect(routableName("a😀")).toBe(true);
  });
});
