import { describe, expect, it } from "vitest";
import { guardAlarmName } from "./alarm";

// The orchestrator's ruling R-09: the guard is a pure function, pinned here;
// `test/table.test.ts` proves `alarm()` consults it.
describe("the nameless-alarm guard (pool fact 8)", () => {
  it("skips an alarm the platform delivered with no name", () => {
    expect(guardAlarmName(undefined)).toBe("skip");
  });

  it("runs an alarm that carries the object's name", () => {
    expect(guardAlarmName("sim-00000000000000aa")).toBe("run");
  });
});
