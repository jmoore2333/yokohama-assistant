// The hash router's id rule, pinned to the store's (security S2a, finding D14; decision D-6). The table is the
// LAYER's verdicts — `store::is_safe_id` over every row, emitted by `wire_fixtures` into
// `web/src/wire/id_cases.json` and refused when it is stale — so this cell holds the client's copy to the
// server's both ways: an id the server would store under, the client opens; one the server refuses, the client
// sends to the door before any request is made.
import { describe, expect, it } from "vitest";
import { ID_MAX, isSafeId } from "./tableId";
import { wire } from "./wire/contract";

const cases = wire.idCases;

const shown = (s: string): string =>
  `"${[...s].map((c) => { const cp = c.codePointAt(0)!; return cp >= 0x20 && cp <= 0x7e ? c : `\\u${cp.toString(16).padStart(4, "0")}`; }).join("")}"`;

/** The committed table's `raw` column, row for row — the two length rows spelled from `ID_MAX` rather than
 *  copied — so the table must carry every one or the pin proves nothing about them. */
const REQUIRED = [
  "sim-0123456789abcdef", "local-1e205cc2c8b9f836", "x", "7", "evening-2", "evening_2", "evening.2", "x.tablet",
  "a".repeat(ID_MAX), "a".repeat(ID_MAX + 1),
  "x.table", "X.TABLE", "x.chat", "x.Chat",
  "_house", "house", "-x", ".x", "_x", "x-", "x.", "x_",
  ".", "..", "../x", "x/..", "a/b", "x/", "/x",
  // The Kelvin sign and the zero-width space are `\u` escapes, as the Rust half spells them: an invisible code
  // point in code is an escape (the repository's rule), and an NFKC editor would fold a raw Kelvin sign into `K`.
  "a b", " x", "x ", "", "a@b", "a%20b", "Bén", "\u212aelvin", "a\u200bb", "x\n",
];

describe("the id rule", () => {
  it("gives the store's own verdict on every input the store judged", () => {
    for (const c of cases) expect(isSafeId(c.raw), shown(c.raw)).toBe(c.safe);
  });
  it("is pinned on the inputs the rule turns on, so the cell above is not vacuous", () => {
    const raws = new Set(cases.map((c) => c.raw));
    for (const raw of REQUIRED) expect(raws.has(raw), `id_cases.json lacks ${shown(raw)}`).toBe(true);
    expect(cases.some((c) => c.safe), "an accepted id").toBe(true);
    expect(cases.some((c) => !c.safe), "a refusal").toBe(true);
  });
});
