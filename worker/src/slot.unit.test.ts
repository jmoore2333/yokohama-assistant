import { describe, expect, it } from "vitest";
import { advances, cmpGen, copyDigest, decide, nextRetry, type Generation, type HouseRow, type Slot } from "./slot";

/** A generation at `at` with a random part written as 16 hex digits. */
const gen = (at: number, rand = "0000000000000000"): Generation => ({ at, rand });

const G1 = gen(1_789_000_000, "00000000000000a1");
const G2 = gen(1_789_000_000, "00000000000000b2");
const G3 = gen(1_789_000_100, "0000000000000001");
const G4 = gen(1_789_000_200, "0000000000000000");

/** A push of `copy` (null: a REMOVAL) at `(g, r)` for the id `t`. */
function push(g: Generation, revision: number, copy: string | null = "{}"): Slot {
  return { id: "t", generation: g, revision, copy, last_active: 1, bytes: 1, attempt: 0 };
}

const row = (current: HouseRow["current"], retired: Generation[] = []): HouseRow => ({ current, retired });

// Spec §2.5: generations are ORDERED by `(created_at, 64 random bits)`.
describe("the generation's order (spec §2.5)", () => {
  // Mutants: the random part compared before the time; either part ignored;
  // the sign flipped.
  it("orders by the time first, then by the random part", () => {
    expect(cmpGen(G1, G2)).toBeLessThan(0);
    expect(cmpGen(G2, G1)).toBeGreaterThan(0);
    expect(cmpGen(G2, G3), "a later time wins over a larger random part").toBeLessThan(0);
    expect(cmpGen(G3, G2)).toBeGreaterThan(0);
    expect(cmpGen(G3, gen(G3.at, G3.rand))).toBe(0);
  });
});

// Spec §2.5, every row of the house's rule: it applies a push when its
// generation is current and its revision higher; or when the id has no
// current generation and the pushed one is not retired and orders after every
// retired one. Anything else is ignored.
describe("decide (spec §2.5)", () => {
  // Battery 5's rule. Mutants: `>=` for `>`; the revision not compared.
  it("a current generation applies a higher revision and ignores an equal or lower one", () => {
    expect(decide(row({ generation: G1, revision: 3 }), push(G1, 4))).toBe("apply");
    expect(decide(row({ generation: G1, revision: 3 }), push(G1, 3))).toBe("ignore");
    expect(decide(row({ generation: G1, revision: 3 }), push(G1, 2))).toBe("ignore");
  });

  // Battery 18: a removed table's late push is ignored in EITHER order —
  // before a re-created table's first push (no current) and after it (a newer
  // current). Mutants: the retired set not consulted; a retired generation
  // applied because its revision is higher than the current one's.
  it("a retired generation is ignored whether or not a newer one is current", () => {
    expect(decide(row(null, [G1]), push(G1, 9)), "before the re-created table's first push").toBe("ignore");
    expect(decide(row({ generation: G3, revision: 1 }, [G1]), push(G1, 9)), "after it").toBe("ignore");
    expect(decide(row(null, [G1]), push(G1, 9, null)), "a retired generation's late REMOVAL").toBe("ignore");
  });

  // Mutants: "after SOME retired one" for "after EVERY"; a generation equal
  // in time and random part to a retired one read as newer.
  it("a new generation with no current applies only if it orders after every retired one", () => {
    expect(decide(row(null), push(G1, 1)), "nothing retired, nothing current").toBe("apply");
    expect(decide(row(null, [G1, G3]), push(G4, 1))).toBe("apply");
    expect(decide(row(null, [G1, G3]), push(G2, 1)), "after one retired generation, before another").toBe("ignore");
    expect(decide(row(null, [G3]), push(G1, 1)), "before the only retired one").toBe("ignore");
  });

  // A current row is replaced only through its own generation: another
  // generation — older or newer — waits for the current one's removal. Mutant:
  // any generation applied over the current one when its revision is higher.
  it("another generation is ignored while one is current", () => {
    expect(decide(row({ generation: G1, revision: 1 }), push(G3, 5)), "a newer generation").toBe("ignore");
    expect(decide(row({ generation: G3, revision: 1 }), push(G1, 5)), "an older generation").toBe("ignore");
  });

  // A removal is ordered by the same rule as a copy. Mutant: a removal
  // applied whatever its generation or revision.
  it("a removal follows the same rule", () => {
    expect(decide(row({ generation: G1, revision: 3 }), push(G1, 4, null))).toBe("apply");
    expect(decide(row({ generation: G1, revision: 3 }), push(G1, 3, null))).toBe("ignore");
    expect(decide(row(null), push(G1, 1, null)), "a table removed before its first copy arrived").toBe("apply");
  });
});

// Spec §2.5: `acked` is recorded only when `(g, r)` is newer than the `acked`
// the table holds.
describe("advances (spec §2.5)", () => {
  // Mutants: `>=` for `>`; the generation not compared; a newer generation
  // with a lower revision refused.
  it("holds for a newer revision and a newer generation, and not for an equal or older one", () => {
    const acked = { generation: G2, revision: 5 };
    expect(advances(null, G1, 1), "nothing acknowledged yet").toBe(true);
    expect(advances(acked, G2, 6), "a newer revision").toBe(true);
    expect(advances(acked, G3, 1), "a newer generation, whatever its revision").toBe(true);
    expect(advances(acked, G2, 5), "equal").toBe(false);
    expect(advances(acked, G2, 4), "an older revision").toBe(false);
    expect(advances(acked, G1, 9), "an older generation, whatever its revision").toBe(false);
  });
});

// Spec §2.4: "Backoff for retries: 30 s, 2 min, 10 min, then hourly".
describe("nextRetry (spec §2.4)", () => {
  // Mutants: the ladder off by one; no hourly ceiling.
  it("answers 30 s, 2 min, 10 min, then hourly", () => {
    expect([0, 1, 2, 3, 4, 50].map(nextRetry)).toEqual([30_000, 120_000, 600_000, 3_600_000, 3_600_000, 3_600_000]);
  });
});

// `acked` keeps the acknowledged copy's digest (spec §2.4): SHA-256 of the
// copy's UTF-8 bytes, in lowercase hex. The vectors are FIPS 180-2's and
// Python's `hashlib`, at every length the padding treats differently (55 and
// 56, 63 to 65, 119 and 120 bytes) and over multi-byte UTF-8. Mutants: a
// UTF-16 digest; the length written in bytes rather than bits; a padding
// block too few.
describe("the acknowledged copy's digest", () => {
  it.each([
    ["", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"],
    ["abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"],
    ["abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"],
    ["x".repeat(55), "d5e285683cd4efc02d021a5c62014694958901005d6f71e89e0989fac77e4072"],
    ["x".repeat(56), "04c26261370ee7541549d16dee320c723e3fd14671e66a099afe0a377c16888e"],
    ["x".repeat(63), "75220b47218278e656f2013bb8f0c455a25eaf01e86c64924e9d48d89776d6f2"],
    ["x".repeat(64), "7ce100971f64e7001e8fe5a51973ecdfe1ced42befe7ee8d5fd6219506b5393c"],
    ["x".repeat(65), "9537c5fdf120482f7d58d25e9ed583f52c02b4e304ea814db1633ad565aed7e9"],
    ["x".repeat(119), "000b48d4edf0fa7bee3c6236ecd2785baa5db4eeb8bb54341b029e0d9fa5fb0c"],
    ["x".repeat(120), "13f05a0b594787f5ecd315edc96141bd3243203d1b7d4f0836f37308b276ba98"],
    ["Café ☕ 東京", "c56d73be848f4f4b239fb43f840597a322117513b049e51750c53027423626dd"],
    ["🀄 Zoë", "de2399f3f62b3f2914de600cb1fde5306f99c811f4c0ea1982c97af222dd8a2e"],
    ["a".repeat(1_000_000), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"],
  ])("digests a copy of %#", (text, hex) => {
    expect(copyDigest(text)).toBe(hex);
  });
});
