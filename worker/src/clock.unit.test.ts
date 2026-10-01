import { afterEach, describe, expect, it, vi } from "vitest";
import { nextReading } from "./clock";

// P-2's reading, in SECONDS: what the shell pins the module's clock to. The
// tick cell must be the first to tick in this file — the count is the
// module's own, per isolate, and vitest gives each test file its own module.
describe("the module's reading (the plan's ruling P-2)", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("ticks one second per reading, from the configured one", () => {
    const clock = { kind: "tick", next: 1_789_000_000 } as const;
    expect([nextReading(clock), nextReading(clock), nextReading(clock)]).toEqual([
      1_789_000_000, 1_789_000_001, 1_789_000_002,
    ]);
  });

  it("answers a frozen reading every time", () => {
    const clock = { kind: "frozen", at: 1_789_000_000 } as const;
    expect([nextReading(clock), nextReading(clock)]).toEqual([1_789_000_000, 1_789_000_000]);
  });

  it("reads the wall clock in whole seconds", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_789_000_123_999);
    expect(nextReading({ kind: "wall" })).toBe(1_789_000_123);
  });
});
