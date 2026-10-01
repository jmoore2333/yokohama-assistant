import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { ROUTER_FILES, accessPaths, apiSegments, layerRoutesText } from "./access-paths.mjs";

const repo = fileURLToPath(new URL("../..", import.meta.url));
const script = fileURLToPath(new URL("./access-paths.mjs", import.meta.url));

/** The nine destinations of the Access application the door window's state 2
 *  names (spec R-6.5c2.39): every `/api/` prefix but `health`, and `/enter`. */
const NINE = [
  "yokohama.moore.nyc/api/admin",
  "yokohama.moore.nyc/api/archive",
  "yokohama.moore.nyc/api/art",
  "yokohama.moore.nyc/api/chat",
  "yokohama.moore.nyc/api/games",
  "yokohama.moore.nyc/api/labels",
  "yokohama.moore.nyc/api/me",
  "yokohama.moore.nyc/api/stream",
  "yokohama.moore.nyc/enter",
];

describe("the protected path list", () => {
  // **The catcher for a prefix the router files gain, or one none of them
  // names any more**: NINE is written out by hand, so the printed list cannot
  // move without this cell failing.
  // Mutants: a prefix dropped from the list; `health` listed; `/enter` missing;
  // a new prefix named in a router file; the layer's file read whole (its
  // tests name /api/nope).
  it("is the nine destinations, one per line, sorted, host-prefixed", () => {
    expect(accessPaths(repo)).toEqual(NINE);
    expect(execFileSync("node", [script], { encoding: "utf8" })).toBe(`${NINE.join("\n")}\n`);
  });
  // **What this cell proves is the script's FILTERING** over what the regex
  // finds in all four files: `health` read and then excepted, `/enter` added,
  // the rest sorted and host-prefixed. Both sides are read from the same four
  // files through the script's own `apiSegments` and `layerRoutesText`, so a
  // prefix a file gains or drops moves both sides and passes here; the cell
  // above is the one that fails on it.
  // Mutants: a file left out of ROUTER_FILES (the others still name its
  // prefixes, so the cell above stays green); a prefix hard-coded rather than
  // read.
  it("equals the /api/<segment> prefixes the four router files name, less health", () => {
    expect(ROUTER_FILES).toHaveLength(4);
    const found = new Set();
    for (const file of ROUTER_FILES) {
      const text = readFileSync(`${repo}/${file}`, "utf8");
      const scoped = file.endsWith("api.rs") ? layerRoutesText(text) : text;
      for (const segment of apiSegments(scoped)) found.add(segment);
    }
    expect(found.has("health"), "health is read, then excepted").toBe(true);
    found.delete("health");
    expect(accessPaths(repo)).toEqual([...found].sort().map((s) => `yokohama.moore.nyc/api/${s}`).concat("yokohama.moore.nyc/enter"));
  });
  it("reads a segment out of a new router text, and the layer's table only", () => {
    expect([...apiSegments('const A = "/api/newthing/{id}"; // /api/games')].sort()).toEqual(["games", "newthing"]);
    const rs = 'fn t() { get("/api/nope"); }\npub fn routes(x: bool) -> Vec<Route> {\n    [("GET", "/api/me")]\n}\nfn u() { "/api/nothing" }\n';
    expect([...apiSegments(layerRoutesText(rs))]).toEqual(["me"]);
  });
  it("refuses a layer file whose routes() table it cannot find", () => {
    expect(() => layerRoutesText("fn other() {}")).toThrow(/pub fn routes\(/);
  });
});
