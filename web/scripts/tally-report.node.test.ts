// **The requests-per-move report's own cells** (cycle E, Task 7; R-E9, and
// R-6.5c2.43's rate-limit value): what `tally-report.mjs` reads out of the
// browser walk's tally, and the ONE regex both it and `e2e/harness.ts`'s
// `tally` name a route with.
//
// Node and not Playwright: the report is a pure function over lines, and the
// regex a pure function over a path, so no browser and no server is involved —
// which is why `npm run test:wasm` runs it where CI installs no Chromium.

import { describe, expect, it } from "vitest";
import {
  formatReport,
  normalizeRoute,
  parseArgs,
  parseTally,
  peakInWindow,
  tallyReport,
} from "./tally-report.mjs";

/** One tally line as `tally` writes it. */
const line = (t: number, route: string, method = "GET", page = "a", status = 200) =>
  JSON.stringify({ t, page, method, route, status });

describe("normalizeRoute", () => {
  it("names a game's route by <id> and drops the query", () => {
    expect(normalizeRoute("/api/games/sim-abc/join?x=1")).toBe("/api/games/<id>/join");
  });

  it("names the game itself, a position and a message by their placeholders", () => {
    expect(normalizeRoute("/api/games/sim-0123456789abcdef")).toBe("/api/games/<id>");
    expect(normalizeRoute("/api/games/sim-abc/at/17")).toBe("/api/games/<id>/at/<n>");
    expect(normalizeRoute("/api/games/sim-abc/at?from=3&to=9")).toBe("/api/games/<id>/at");
    expect(normalizeRoute("/api/games/sim-abc/chat/4")).toBe("/api/games/<id>/chat/<n>");
    expect(normalizeRoute("/api/chat/12")).toBe("/api/chat/<n>");
  });

  it("leaves a route with no id as it is, the query dropped", () => {
    expect(normalizeRoute("/api/games?scope=mine&before=x")).toBe("/api/games");
    expect(normalizeRoute("/api/archive")).toBe("/api/archive");
    expect(normalizeRoute("/art/orders/O12.webp")).toBe("/art/orders/O12.webp");
  });

  it("is idempotent, so a line already named reads the same", () => {
    expect(normalizeRoute("/api/games/<id>/at/<n>")).toBe("/api/games/<id>/at/<n>");
  });
});

describe("tallyReport", () => {
  // **Six lines over two raw routes — two games' state reads — at 0, 1, 2, 11,
  // 12 and 13 s, interleaved.** Named by the regex they are ONE route, so it
  // reads 2 per move at `--moves 3` and a peak of 3 in a 10-second window.
  // Unnamed they are two routes at 1 per move with a peak of 2 each, so this is
  // the cell a dropped normalization fails.
  const six = [
    line(0, "/api/games/sim-abc/state"),
    line(1_000, "/api/games/sim-def/state"),
    line(2_000, "/api/games/sim-abc/state"),
    line(11_000, "/api/games/sim-def/state"),
    line(12_000, "/api/games/sim-abc/state"),
    line(13_000, "/api/games/sim-def/state"),
  ].join("\n");

  it("reads six lines over two raw routes as one route at 2 per move, a peak of 3, no oracle", () => {
    const r = tallyReport(parseTally(six), { moves: 3 });
    expect(r.routes.map((x) => [x.method, x.route, x.count, x.perMove, x.peak])).toEqual([
      ["GET", "/api/games/<id>/state", 6, 2, 3],
    ]);
    expect(r.api).toEqual({ count: 6, perMove: 2, peak: 3 });
    expect(r.oracle).toEqual({ count: 0, perMove: 0, peak: 0 });
    // And the printed report says it in the same numbers.
    const text = formatReport(r);
    expect(text).toMatch(/^GET \/api\/games\/<id>\/state\s+6\s+2\.00\s+3\s+0$/m);
    expect(text).toMatch(/^oracle paths .*\s+0\s+0\.00\s+0$/m);
  });

  it("counts the three oracle paths and nothing else in the oracle subset", () => {
    const lines = [
      line(0, "/api/games/<id>/join", "POST"),
      line(500, "/api/games/<id>/unlock", "POST"),
      line(9_000, "/api/archive", "POST"),
      line(9_500, "/api/games/<id>/chat", "POST"),
      line(30_000, "/api/games/<id>/join", "POST", "b", 409),
    ].join("\n");
    const r = tallyReport(parseTally(lines), { moves: 2 });
    expect(r.oracle).toEqual({ count: 4, perMove: 2, peak: 3 });
    expect(r.api).toEqual({ count: 5, perMove: 2.5, peak: 4 });
    // The refused join is counted, and counted as refused.
    const join = r.routes.find((x) => x.route === "/api/games/<id>/join");
    expect(join).toMatchObject({ count: 2, refused: 1 });
  });

  it("keeps what is not /api out of the API figures and in the whole-page ones", () => {
    const lines = [
      line(0, "/assets/index-abc.js"),
      line(100, "/art/orders/O12.webp"),
      line(200, "/api/games/<id>"),
    ].join("\n");
    const r = tallyReport(parseTally(lines), { moves: 1 });
    expect(r.api).toEqual({ count: 1, perMove: 1, peak: 1 });
    expect(r.notApi).toEqual({ count: 2, perMove: 2, peak: 2 });
    expect(r.all).toEqual({ count: 3, perMove: 3, peak: 3 });
    expect(r.routes.map((x) => x.route)).toEqual(["/api/games/<id>"]);
  });

  it("refuses a line that is not a tally line, naming its number", () => {
    expect(() => parseTally(`${line(0, "/api/me")}\nnot json`)).toThrow(/line 2/);
    expect(() => parseTally(JSON.stringify({ t: 0, route: "/api/me" }))).toThrow(/line 1/);
  });
});

describe("peakInWindow", () => {
  it("counts a half-open window, so a request exactly 10 s later starts the next one", () => {
    expect(peakInWindow([0, 10_000], 10_000)).toBe(1);
    expect(peakInWindow([0, 9_999], 10_000)).toBe(2);
    expect(peakInWindow([], 10_000)).toBe(0);
  });

  it("reads the times in any order", () => {
    expect(peakInWindow([13_000, 0, 12_000, 2_000, 11_000, 1_000], 10_000)).toBe(3);
  });
});

describe("parseArgs", () => {
  it("takes the path and the moves", () => {
    expect(parseArgs(["t.jsonl", "--moves", "412"])).toEqual({ path: "t.jsonl", moves: 412 });
  });

  it("refuses no moves, or moves that are not a positive whole number", () => {
    expect(() => parseArgs(["t.jsonl"])).toThrow(/--moves/);
    expect(() => parseArgs(["t.jsonl", "--moves", "0"])).toThrow(/--moves/);
    expect(() => parseArgs(["t.jsonl", "--moves", "2.5"])).toThrow(/--moves/);
    expect(() => parseArgs(["--moves", "3"])).toThrow(/path/);
  });
});
