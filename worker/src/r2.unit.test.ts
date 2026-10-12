import { describe, expect, it, vi } from "vitest";
import { executePlan, logOutcome, parsePlan, parseStamp, settledArm, stampArm, type Bucket, type Outcome, type Plan } from "./r2";

/**
 * A plan as the module writes one (the plan's names table). The record's seed
 * is above 2^53, so only its TEXT tells it from the record `OTHER_SEED`
 * carries: `JSON.parse` reads both seeds as the same number.
 */
const PLAN: Plan = {
  key: "0".repeat(64),
  record_key: "test/rev7/abandoned/t.json",
  table_key: "test/rev7/abandoned/t.table.json",
  record: '{"seed":18446744073709551615,"events":[]}',
  table: '{\n  "id": "t",\n  "archived_at": 1789000000\n}',
};
const OTHER_SEED = '{"seed":18446744073709551614,"events":[]}';

/**
 * R2's `get` and `put` as `executePlan` calls them, over a map. A put whose
 * `onlyIf` is create-if-absent — `If-None-Match: *`, in a `Headers` or in R2's
 * object form `etagDoesNotMatch: "*"` — answers `null` and writes nothing when
 * the key holds an object, as R2 answers a precondition that fails. Every put
 * is recorded with whether it was conditional.
 *
 * - `racer`: a concurrent writer, whose bytes land at `key` just before a
 *   conditional put of that key arrives;
 * - `fails`: the error a call rejects with, for an operation and a key.
 */
class FakeBucket {
  readonly objects = new Map<string, string>();
  readonly puts: Array<{ key: string; conditional: boolean }> = [];
  racer: { key: string; bytes: string } | undefined;
  fails: ((op: "get" | "put", key: string) => unknown) | undefined;

  async get(key: string): Promise<{ text(): Promise<string> } | null> {
    const failure = this.fails?.("get", key);
    if (failure !== undefined) throw failure;
    const text = this.objects.get(key);
    return text === undefined ? null : { text: async () => text };
  }

  async put(key: string, value: string, options?: R2PutOptions): Promise<{ key: string } | null> {
    const failure = this.fails?.("put", key);
    if (failure !== undefined) throw failure;
    const onlyIf = options?.onlyIf;
    const conditional =
      onlyIf instanceof Headers ? onlyIf.get("If-None-Match") === "*" : (onlyIf as R2Conditional | undefined)?.etagDoesNotMatch === "*";
    this.puts.push({ key, conditional });
    if (conditional && this.racer?.key === key) this.objects.set(key, this.racer.bytes);
    if (conditional && this.objects.has(key)) return null;
    this.objects.set(key, value);
    return { key };
  }

  /** As `executePlan` takes it: the structural type is R2's, whose overloads
   *  a two-method fake cannot spell. */
  get asBucket(): Bucket {
    return this as unknown as Bucket;
  }
}

// Spec §2.6: the shell carries a plan out in `DirArchive::put`'s own order —
// the record key read; the table key put UNCONDITIONALLY; the record key put
// create-if-absent and, if that precondition fails, read again; any other
// failure `failed`, carrying R2's message. Every outcome carries a `detail`,
// which the module requires (Task 6's review, escalation c).
describe("executePlan, in DirArchive::put's order (spec §2.6)", () => {
  // Mutants: the record written before the table (the puts' order); the
  // record put unconditionally; the detail left out.
  it("lands a pair in an empty bucket: the table, then the record created only if absent", async () => {
    const bucket = new FakeBucket();
    expect(await executePlan(bucket.asBucket, PLAN)).toEqual({ outcome: "landed", detail: PLAN.record_key });
    expect(bucket.puts).toEqual([
      { key: PLAN.table_key, conditional: false },
      { key: PLAN.record_key, conditional: true },
    ]);
    expect(Object.fromEntries(bucket.objects)).toEqual({ [PLAN.record_key]: PLAN.record, [PLAN.table_key]: PLAN.table });
  });

  // Mutants: the step-1 read skipped (the pair written again, or the
  // conditional put's null read as a refusal); the outcomes swapped.
  it("answers identical, writing nothing, when the record key holds these very bytes", async () => {
    const bucket = new FakeBucket();
    bucket.objects.set(PLAN.record_key, PLAN.record);
    expect(await executePlan(bucket.asBucket, PLAN)).toEqual({ outcome: "identical", detail: PLAN.record_key });
    expect(bucket.puts).toEqual([]);
  });

  // The record is compared as its TEXT, and a conflict hands the text the key
  // holds on to the module, whose refinish rule reads it (M6.5c2,
  // R-6.5c2.11): as TEXT, since parsed it would lose its seed's last digits.
  // Mutants: a comparison that parses the two records (both seeds read as one
  // number: identical); any write; `held` left off, or re-serialized.
  it("answers conflict, writing nothing, when the record key holds other bytes — a digit past 2^53 among them — and hands those bytes on, unparsed", async () => {
    const bucket = new FakeBucket();
    bucket.objects.set(PLAN.record_key, OTHER_SEED);
    expect(JSON.parse(OTHER_SEED), "premise: parsed, the two records are equal").toEqual(JSON.parse(PLAN.record));
    const outcome = await executePlan(bucket.asBucket, PLAN);
    expect(outcome).toEqual({ outcome: "conflict", detail: PLAN.record_key, held: OTHER_SEED });
    expect(outcome.held, "the held bytes, exactly").toBe(OTHER_SEED);
    expect(JSON.stringify(JSON.parse(OTHER_SEED)), "premise: a parse and a re-serialization lose the digits").not.toBe(OTHER_SEED);
    expect(bucket.puts).toEqual([]);
    expect(bucket.objects.get(PLAN.record_key)).toBe(OTHER_SEED);
  });

  // Battery 20. A put that crashed between its two writes left the table and
  // no record; the next put REPAIRS it — the stale table overwritten, not
  // refused. Mutant: the table put create-if-absent (the R2 order reversed).
  it("lands over a HALF PAIR, the stale table overwritten (battery 20)", async () => {
    const bucket = new FakeBucket();
    bucket.objects.set(PLAN.table_key, '{"archived_at": 1788999999}');
    expect(await executePlan(bucket.asBucket, PLAN)).toEqual({ outcome: "landed", detail: PLAN.record_key });
    expect(Object.fromEntries(bucket.objects)).toEqual({ [PLAN.record_key]: PLAN.record, [PLAN.table_key]: PLAN.table });
  });

  // A concurrent writer's record lands between the read and the create. The
  // create refuses, and the record is read again: a conflict then carries the
  // racer's bytes, as the first read's does. Mutants: the refusal read as
  // landed; the record put unconditionally (the racer's record overwritten);
  // the second read skipped; the re-read's conflict without `held`.
  it.each([
    ["the same bytes", PLAN.record, { outcome: "identical", detail: PLAN.record_key }],
    ["other bytes", OTHER_SEED, { outcome: "conflict", detail: PLAN.record_key, held: OTHER_SEED }],
  ] as const)("reads the record again when its create loses a race to %s", async (_what, bytes, want) => {
    const bucket = new FakeBucket();
    bucket.racer = { key: PLAN.record_key, bytes };
    expect(await executePlan(bucket.asBucket, PLAN)).toEqual(want);
    expect(bucket.objects.get(PLAN.record_key), "the racer's record stands").toBe(bytes);
    expect(bucket.puts.map((p) => p.key)).toEqual([PLAN.table_key, PLAN.record_key]);
  });

  // Mutants: a throw escaping executePlan; a detail other than R2's message;
  // the record put before the table — which a table put that fails would
  // leave standing alone, the one state the order exists to prevent.
  it("answers failed with R2's message when a call throws, and never leaves a record without its table", async () => {
    const failures: Array<[string, (op: "get" | "put", key: string) => unknown, string, string[]]> = [
      ["the table's put", (op, key) => (op === "put" && key === PLAN.table_key ? new Error("put: internal error (10001)") : undefined), "put: internal error (10001)", []],
      ["the record's put", (op, key) => (op === "put" && key === PLAN.record_key ? new Error("put: quota exceeded") : undefined), "put: quota exceeded", [PLAN.table_key]],
      ["the record's read", (op) => (op === "get" ? new Error("get: network lost") : undefined), "get: network lost", []],
      ["a throw that is not an Error", (op) => (op === "get" ? "unreachable" : undefined), "unreachable", []],
    ];
    for (const [what, fails, detail, left] of failures) {
      const bucket = new FakeBucket();
      bucket.fails = fails;
      expect(await executePlan(bucket.asBucket, PLAN), what).toEqual({ outcome: "failed", detail });
      expect([...bucket.objects.keys()], `${what}: what the bucket holds`).toEqual(left);
    }
  });
});

// The plan crosses the module boundary as a string (the orchestrator's ruling
// I-25). The shell reads its five fields, and the record and the table stay
// the exact strings the module wrote.
describe("parsePlan", () => {
  // Mutants: a field not read; the record re-serialized.
  it("reads the module's plan, the record and the table as their exact text", () => {
    const plan = parsePlan(JSON.stringify(PLAN));
    expect(plan).toEqual(PLAN);
    expect(plan.record).toBe(PLAN.record);
  });

  // A plan the shell cannot carry out is refused, naming the field, and never
  // echoing a record. Mutants: a missing or non-string field accepted; the
  // plan's text in the sentence.
  it("refuses a plan that is not the module's five strings, naming the field", () => {
    const { table: _table, ...noTable } = PLAN;
    for (const [bad, field] of [
      [noTable, "table"],
      [{ ...PLAN, key: 7 }, "key"],
      [{ ...PLAN, extra: "x" }, "extra"],
    ] as const) {
      const text = JSON.stringify(bad);
      expect(() => parsePlan(text), field).toThrow(field);
      expect(() => parsePlan(text), `${field}: the record is not echoed`).not.toThrow(PLAN.record);
    }
    expect(() => parsePlan("[]")).toThrow("not an object");
  });
});

// `stamp_archive`'s 200 is `{"stamped": bool, "over": bool}` (the
// orchestrator's ruling I3, refined at Task 6). Its body carries no seed.
describe("the stamp's reply and its arm (the orchestrator's ruling I3)", () => {
  // Mutants: a field not read; a body of another shape accepted.
  it("reads the module's two booleans, and nothing else", () => {
    expect(parseStamp('{"stamped":true,"over":true}')).toEqual({ stamped: true, over: true });
    expect(parseStamp('{"stamped":false,"over":false}')).toEqual({ stamped: false, over: false });
    for (const bad of ['{"stamped":true}', '{"stamped":"yes","over":true}', '{"stamped":true,"over":true,"at":1}', "[true,true]"]) {
      expect(() => parseStamp(bad), bad).toThrow();
    }
  });

  // The three arms. `stamped` completes the save; a game no longer over — an
  // undo at a one-person table after the plan was made (Task 6's escalation
  // b) — is owed nothing; a game still over whose stamp did not land retries.
  // Mutants: the undo case read as a failed stamp (retried for ever), or a
  // failed stamp read as done (the save never stamped).
  it("completes a stamped save, clears a game no longer over, and retries a stamp that did not land", () => {
    expect(stampArm({ stamped: true, over: true })).toBe("stamped");
    expect(stampArm({ stamped: false, over: false })).toBe("no-longer-over");
    expect(stampArm({ stamped: false, over: true })).toBe("retry");
    // A stamp that landed earlier stays landed, whatever the game did since.
    expect(stampArm({ stamped: true, over: false })).toBe("stamped");
  });
});

// R-6.5c2.11 (M6.5c2, spec §2.5): a conflict goes to the stamp, and what the
// upload and the stamp leave TOGETHER is `settledArm`'s. A conflict the stamp
// landed on — the layer found an earlier completion of this same game at the
// key — completes the save, whether or not the game is still over; one it did
// not land on, WHATEVER the stamp answered, is final as every conflict was
// (the M6.5c spec §2.6 arm, kept for every conflict the layer did not stamp):
// never retried. Every other outcome is `stampArm`'s, and a stamp that
// answered no 200 is retried. Mutants: a conflict read through `stampArm`
// (another game's `{stamped: false, over: true}` retried hourly for ever); a
// conflict completed without its stamp; a landed conflict read through `over`
// too (`stamped && over`); a landed upload's refused stamp not retried.
describe("what an upload and its stamp leave together (R-6.5c2.11)", () => {
  const conflict: Outcome = { outcome: "conflict", detail: PLAN.record_key, held: OTHER_SEED };
  it("completes a conflict the stamp landed on, and leaves every other conflict final", () => {
    for (const stamp of [{ stamped: true, over: true }, { stamped: true, over: false }]) {
      expect(settledArm(conflict, stamp), JSON.stringify(stamp)).toBe("stamped");
    }
    for (const stamp of [{ stamped: false, over: true }, { stamped: false, over: false }, undefined]) {
      expect(settledArm(conflict, stamp), JSON.stringify(stamp)).toBe("final");
    }
  });
  it("reads every other outcome as stampArm does, and retries a stamp that answered no 200", () => {
    for (const outcome of [
      { outcome: "landed", detail: PLAN.record_key },
      { outcome: "identical", detail: PLAN.record_key },
      { outcome: "failed", detail: "put: internal error (10001)" },
    ] as const) {
      for (const stamp of [
        { stamped: true, over: true },
        { stamped: true, over: false },
        { stamped: false, over: false },
        { stamped: false, over: true },
      ]) {
        expect(settledArm(outcome, stamp), `${outcome.outcome} ${JSON.stringify(stamp)}`).toBe(stampArm(stamp));
      }
      expect(settledArm(outcome, { stamped: false, over: true }), outcome.outcome).toBe("retry");
      expect(settledArm(outcome, undefined), `${outcome.outcome}: no 200`).toBe("retry");
    }
  });
});

// The held record is a whole game — a seed above 2^53, and every move — and it
// is the MODULE's to read: the shell hands it on and never logs it. Mutant: the
// outcome spread into the line.
describe("logOutcome", () => {
  it("names the outcome's detail, and never the record a conflict holds", () => {
    const spy = vi.spyOn(console, "log").mockImplementation(() => {});
    try {
      logOutcome("t", "game-over", PLAN, { outcome: "conflict", detail: PLAN.record_key, held: OTHER_SEED }, { attempt: 0 });
      expect(spy.mock.calls.map(([line]) => JSON.parse(String(line)) as unknown)).toEqual([
        { event: "archive-conflict", id: "t", site: "game-over", record_key: PLAN.record_key, detail: PLAN.record_key, attempt: 0 },
      ]);
      expect(String(spy.mock.calls[0]?.[0]), "no byte of the held record").not.toContain("18446744073709551614");
    } finally {
      spy.mockRestore();
    }
  });

  // Security S2b, finding D23 (PF-03): a landed, identical or conflicting outcome's detail IS the record's key, and
  // the one failed case's sentence begins with it, so the key is redacted in BOTH fields. A browser save's key
  // carries `browser/<readable>-<12 hex>/`, the readable half spelling the caller's address. Mutants: `redactKey`
  // dropped at `record_key`; `detail` logged unredacted.
  const BROWSER_PLAN: Plan = { ...PLAN, record_key: "test/browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.json" };
  const REDACTED = "test/browser/c91ad2a917f7/rev7/over/wire.json";
  /** The one line `logOutcome` writes for `outcome`, raw and parsed. */
  function lineOf(outcome: Outcome): { raw: string; parsed: Record<string, unknown> } {
    const spy = vi.spyOn(console, "log").mockImplementation(() => {});
    try {
      logOutcome("t", "browser", BROWSER_PLAN, outcome);
      expect(spy.mock.calls.length, "one line").toBe(1);
      const raw = String(spy.mock.calls[0]?.[0]);
      return { raw, parsed: JSON.parse(raw) as Record<string, unknown> };
    } finally {
      spy.mockRestore();
    }
  }

  it("logs a browser save's key redacted in record_key and in the detail a landed outcome carries", async () => {
    const landed = await executePlan(new FakeBucket().asBucket, BROWSER_PLAN);
    expect(landed, "premise: the outcome as executePlan builds it").toEqual({ outcome: "landed", detail: BROWSER_PLAN.record_key });
    const { raw, parsed } = lineOf(landed);
    expect([parsed.record_key, parsed.detail]).toEqual([REDACTED, REDACTED]);
    expect(raw, "no readable half").not.toContain("red_at_wire");
    expect(raw, "no readable half").not.toContain("browser/red");
  });

  it("logs the failed sentence that begins with the key redacted", async () => {
    // A create refused as present, then a read finding nothing: the one failure executePlan writes itself.
    const vanishing = {
      get: async () => null,
      put: async (key: string, _value: string, options?: R2PutOptions) => (options?.onlyIf === undefined ? { key } : null),
    } as unknown as Bucket;
    const failed = await executePlan(vanishing, BROWSER_PLAN);
    expect(failed, "premise: the outcome as executePlan builds it").toEqual({
      outcome: "failed",
      detail: `${BROWSER_PLAN.record_key} refused a create as present, then read as absent`,
    });
    const { raw, parsed } = lineOf(failed);
    expect(parsed.record_key).toBe(REDACTED);
    expect(parsed.detail).toBe(`${REDACTED} refused a create as present, then read as absent`);
    expect(raw, "no readable half").not.toContain("red_at_wire");
  });
});
