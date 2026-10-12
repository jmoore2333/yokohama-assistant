import { describe, expect, it } from "vitest";
import { capVerdict, SOCKETS_PER_CALLER, SOCKETS_PER_TABLE } from "./sockets";

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
