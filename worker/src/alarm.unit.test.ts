import { describe, expect, it } from "vitest";
import { earliest, guardAlarmName } from "./alarm";

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

// Spec §2.4: ONE alarm per object, "set to the earliest of" what is due. A
// site's own ask meets another thing owed (an owed save's retry, Task 13).
// Mutants: the later of the two kept; a deletion that loses the other's retry;
// a site that leaves the alarm made to set it.
describe("the one alarm, at the earliest thing due (spec §2.4)", () => {
  it("brings a time forward, and never back", () => {
    expect(earliest(1_000, 500)).toBe(500);
    expect(earliest(500, 1_000)).toBe(500);
  });

  it("re-arms a deletion to what is still due, and deletes only when nothing is", () => {
    expect(earliest(null, 500)).toBe(500);
    expect(earliest(null, undefined)).toBeNull();
  });

  it("leaves an alarm a site leaves, and takes a site's ask when nothing else is due", () => {
    expect(earliest(undefined, 500)).toBeUndefined();
    expect(earliest(1_000, undefined)).toBe(1_000);
  });
});
