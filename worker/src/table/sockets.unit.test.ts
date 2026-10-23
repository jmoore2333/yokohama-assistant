import { describe, expect, it } from "vitest";
import { capVerdict, SEATED_TAG, SOCKETS_PER_CALLER, SOCKETS_PER_TABLE, tableVerdict, WATCHERS_PER_TABLE } from "./sockets";

describe("capVerdict (security S2b, D3; decision D-4)", () => {
  // Mutants: `>` for `>=` on either bound; the own cap read before the total (the total's sentence lost when both
  // are hit); the literals moved.
  it("refuses the ninth of one caller's and the sixty-fifth in all, the total's verdict first, and admits under both", () => {
    const caps = { perCaller: SOCKETS_PER_CALLER, total: SOCKETS_PER_TABLE };
    expect(capVerdict(7, 63, caps)).toBeUndefined();
    expect(capVerdict(8, 63, caps)).toBe("own");
    expect(capVerdict(0, 64, caps)).toBe("total");
    expect(capVerdict(8, 64, caps), "both hit: the table's own sentence, since closing a tab would not help").toBe("total");
    expect([SOCKETS_PER_CALLER, SOCKETS_PER_TABLE]).toEqual([8, 64]);
  });
});

describe("tableVerdict (S4-pre, the spec's Issue 1)", () => {
  // A table's verdict adds the watchers' figure between the two: the total first, then — for a caller with no seat —
  // the watchers' figure, then the caller's own. Each order is pinned by a TWO-defect probe at an adjacent pair (a
  // single-defect probe pins nothing about order). Mutants: the watchers' arm applied to a seated caller; any two
  // arms swapped; `>` for `>=` on the watchers' bound; the literals moved; the lobby switched to this verdict.
  it("judges a table's total first, then a watcher by the watchers' figure, then the caller's own, and leaves a seated caller to the total and their own", () => {
    const caps = { perCaller: SOCKETS_PER_CALLER, total: SOCKETS_PER_TABLE, watchers: WATCHERS_PER_TABLE };
    expect(tableVerdict(7, 63, 31, false, caps)).toBeUndefined();
    expect(tableVerdict(0, 63, 32, false, caps)).toBe("watchers");
    expect(tableVerdict(0, 63, 32, true, caps), "seated: the watchers' figure does not apply").toBeUndefined();
    expect(tableVerdict(8, 63, 32, true, caps), "seated, their own ninth").toBe("own");
    expect(tableVerdict(0, 64, 32, false, caps), "total AND watchers: the total's sentence").toBe("total");
    expect(tableVerdict(8, 63, 32, false, caps), "watchers AND own: the watchers' sentence").toBe("watchers");
    expect(tableVerdict(8, 64, 0, true, caps), "total AND own, seated: the total's sentence").toBe("total");
    expect(capVerdict(8, 64, { perCaller: 8, total: 64 }), "the lobby's verdict is unchanged").toBe("total");
    expect([WATCHERS_PER_TABLE, SEATED_TAG]).toEqual([32, "s:seated"]);
  });
});
