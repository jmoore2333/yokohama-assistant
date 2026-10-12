import { runInDurableObject } from "cloudflare:test";
import type { Archive } from "../src/archive";
import type { Env } from "../src/env";
import { House } from "../src/house";
import type { Slot } from "../src/slot";
import type { Table } from "../src/table";

/**
 * **The pool facts' technique, in one place** (the plan's pool facts 1 and 2):
 * a house push held by patching `House.prototype.apply` — the PROTOTYPE, which
 * the pool's RPC resolver reads through. An instance's own property makes
 * every RPC to it THROW rather than hang, which is how the SECOND battery-25
 * text was vacuous; the first used `evictDurableObject`, which drains the
 * push. A push FAILED is made on the table's side instead (`failPushes`),
 * and so is an archive upload held or failed: the object's own bucket is
 * swapped (`swapBucket`), with its puts held (`heldPuts`) or rejected
 * (`failingPuts`), or its share's listing rejected (`failingList`).
 *
 * Every cell restores its patch in a `finally`, and BEFORE any
 * `runDurableObjectAlarm` that follows an abort: a module-level patch survives
 * `abortAllDurableObjects()` and `evictDurableObject()`, so a gate left in
 * place hangs the alarm's own delivery (pool fact 2).
 */

/** What the house answers a push: the `(generation, revision)` it was handed. */
type Ack = Awaited<ReturnType<House["apply"]>>;

/** A gate: a promise the cell resolves. MADE IN THE CELL'S OWN CONTEXT, when
 *  the hold is installed — a promise made inside the house, while it serves a
 *  push, cannot be resolved from the cell: workerd refuses it ("Cannot
 *  perform I/O on behalf of a different Durable Object"), and the held push
 *  then fails with "Promise will never complete" (measured). */
interface Gate {
  opened: Promise<void>;
  open(): void;
}

function gate(): Gate {
  let open!: () => void;
  const opened = new Promise<void>((resolve) => (open = resolve));
  return { opened, open };
}

/** A push's key: the PAIR, because revisions are per table and a revision
 *  alone would also hold another table's push (re-review m-m). */
const keyOf = (id: string, revision: number) => `${id}\u0000${revision}`;

export interface Hold {
  /** Let held pushes through: `(id, revision)`'s — even before its push has
   *  arrived — or, with no argument, every push, and no push is held after
   *  it. A hold of EVERY push releases only all at once: its pushes share one
   *  gate. */
  release(id?: string, revision?: number): void;
  /** Put the house's own `apply` back, and let through every push still
   *  held: a cell that fails between its hold and its release must not leave
   *  a push waiting forever, which would hang the next cell's eviction of
   *  the house. */
  restore(): void;
  /** Whether `(id, revision)`'s push has reached its gate. */
  reached(id: string, revision: number): boolean;
  /** Whether `(id, revision)`'s held push has run through the house's own
   *  `apply` — the house's half; the table's half is `settle` (helpers.ts). */
  settled(id: string, revision: number): boolean;
}

/**
 * Hold house pushes at a gate the cell opens: every push, or with `only` just
 * those `(id, revision)` pairs. A held push neither fails nor succeeds until
 * its gate opens, then runs through the house's own `apply`.
 */
export function holdApply(only?: Array<[id: string, revision: number]>): Hold {
  const original = House.prototype.apply;
  const shared = only === undefined ? gate() : undefined;
  const gates = new Map((only ?? []).map(([id, revision]) => [keyOf(id, revision), gate()]));
  const reached = new Set<string>();
  const settled = new Set<string>();
  let holding = true;
  House.prototype.apply = async function (this: House, slot: Slot): Promise<Ack> {
    const key = keyOf(slot.id, slot.revision);
    const door = holding ? (shared ?? gates.get(key)) : undefined;
    if (door === undefined) return original.call(this, slot);
    reached.add(key);
    await door.opened;
    try {
      return await original.call(this, slot);
    } finally {
      settled.add(key);
    }
  };
  return {
    release(id, revision) {
      if (id === undefined || revision === undefined) {
        holding = false;
        shared?.open();
        for (const door of gates.values()) door.open();
        return;
      }
      if (shared !== undefined) throw new Error("a hold of every push is released all at once");
      const door = gates.get(keyOf(id, revision));
      if (door === undefined) throw new Error(`no gate holds ${id} at revision ${revision}`);
      door.open();
    },
    restore() {
      House.prototype.apply = original;
      holding = false;
      shared?.open();
      for (const door of gates.values()) door.open();
    },
    reached: (id, revision) => reached.has(keyOf(id, revision)),
    settled: (id, revision) => settled.has(keyOf(id, revision)),
  };
}

/**
 * The next `times` pushes `stub`'s table makes FAIL, on the TABLE's side: its
 * `env` is swapped, inside the object, for a copy whose `HOUSE` answers
 * `apply` with a rejection — what an unreachable house's RPC gives the table
 * (`push`'s catch, then `failed`). Pushes after those reach the real house.
 *
 * Why not the house's side: an `apply` that throws inside workerd's RPC
 * dispatch prints an "uncaught exception" on stderr for every refusal, at the
 * pool's RPC wrapper and beyond the reach of vitest's capture (measured), so
 * the run would not be quiet. The object's `env` is REPLACED by a copy, never
 * written to, so nothing else that holds the original sees the swap; an
 * eviction discards the copy with the instance (pool fact 4), and `restore()`
 * puts the original back.
 */
export async function failPushes(stub: DurableObjectStub<Table>, times: number): Promise<{ restore(): Promise<void> }> {
  let left = times;
  let own: Env | undefined;
  await runInDurableObject(stub, (table: Table) => {
    own = table["env"];
    const house = own.HOUSE;
    const unreachable = {
      idFromName: (name: string) => house.idFromName(name),
      get: (id: DurableObjectId) => {
        const real = house.get(id);
        return {
          apply: (slot: Slot) =>
            left-- > 0 ? Promise.reject(new Error("the house is unreachable (failPushes)")) : real.apply(slot),
        };
      },
    };
    table["env"] = { ...own, HOUSE: unreachable as unknown as Env["HOUSE"] };
  });
  return {
    restore: () =>
      runInDurableObject(stub, (table: Table) => {
        if (own !== undefined) table["env"] = own;
      }),
  };
}

/** What a swapped bucket does with a `put` (`swapBucket`): `real` is the
 *  object's own bucket, whose `put` the answer may call through to. */
export type PutThrough = (real: R2Bucket, key: string, value: string, options?: R2PutOptions) => Promise<R2Object | null>;

/** What a swapped bucket does with a `list` (`swapBucket`): `real` is the
 *  object's own bucket, whose `list` the answer may call through to. */
export type ListThrough = (real: R2Bucket, options: R2ListOptions | undefined) => Promise<R2Objects>;

/**
 * The bucket a LIVE object reaches, replaced — its `env` swapped, inside the
 * object, for a copy whose `ARCHIVE` reads from the object's own bucket and
 * hands every `put` to `put` and every `list` to `list` (by default the real
 * bucket's: an archive object lists its share before every save, security
 * S2b) — as `failPushes` swaps `HOUSE`, and for its
 * reasons: the original is never written to, an eviction discards the copy
 * (pool fact 4), and `restore()` puts the original back. A table object or an
 * archive object alike: each reads `this.env.ARCHIVE` when it uploads.
 */
export async function swapBucket(
  stub: DurableObjectStub<Table> | DurableObjectStub<Archive>,
  put: PutThrough,
  list: ListThrough = (real, options) => real.list(options),
): Promise<{ restore(): Promise<void> }> {
  const target = stub as unknown as DurableObjectStub;
  let own: Env | undefined;
  await runInDurableObject(target, (object: Table | Archive) => {
    own = object["env"];
    const real = own.ARCHIVE;
    if (real === undefined) throw new Error("premise: the pool binds the bucket as ARCHIVE (wrangler.test.jsonc)");
    const swapped = {
      get: (...args: Parameters<R2Bucket["get"]>) => real.get(...args),
      put: (key: string, value: string, options?: R2PutOptions) => put(real, key, value, options),
      list: (options?: R2ListOptions) => list(real, options),
    };
    object["env"] = { ...own, ARCHIVE: swapped as unknown as R2Bucket };
  });
  return {
    restore: () =>
      runInDurableObject(target, (object: Table | Archive) => {
        if (own !== undefined) object["env"] = own;
      }),
  };
}

/** The message a `failingPuts` put (and a `failingList` list) rejects with —
 *  what R2's own failure carries into the outcome's `detail`. */
export const R2_UNREACHABLE = "R2 is unreachable (failingPuts)";

/** The next `times` puts whose key `only` accepts REJECT, as an unreachable
 *  R2 answers; every other put reaches the real bucket. */
export function failingPuts(times: number, only: (key: string) => boolean = () => true): PutThrough {
  let left = times;
  return (real, key, value, options) =>
    only(key) && left-- > 0 ? Promise.reject(new Error(R2_UNREACHABLE)) : real.put(key, value, options);
}

/** A bucket whose `list` REJECTS as an unreachable R2 answers, every other call reaching the real bucket. */
export function failingList(): ListThrough {
  return () => Promise.reject(new Error(R2_UNREACHABLE));
}

/**
 * Every put HELD at a gate the cell opens, then passed to the real bucket —
 * the gate made in the cell's own context, as `holdApply`'s is. `reached()`
 * says whether a put has arrived at it.
 */
export function heldPuts(): { through: PutThrough; reached(): boolean; open(): void } {
  const door = gate();
  let reached = false;
  return {
    through: async (real, key, value, options) => {
      reached = true;
      await door.opened;
      return real.put(key, value, options);
    },
    reached: () => reached,
    open: () => door.open(),
  };
}

/**
 * Two uploads' puts, each held at a gate of its OWN — the M6.5c2 refinish
 * race's (spec §2.5): the FIRST put to arrive waits at `openFirst`'s gate, the
 * SECOND at `openSecond`'s, and every later put goes straight through. An
 * upload's first put is its table's (`executePlan`), so two overlapping
 * uploads each wait at their table's put and are released one at a time: open
 * the first, and its record put follows its table's through; then the second.
 * Gates made in the cell's own context, as `heldPuts`'s are. `arrived()`
 * counts the puts that have reached the bucket.
 */
export function heldInTurn(): { through: PutThrough; arrived(): number; openFirst(): void; openSecond(): void; openAll(): void } {
  const [first, second] = [gate(), gate()];
  let arrived = 0;
  return {
    through: async (real, key, value, options) => {
      const turn = arrived++;
      if (turn === 0) await first.opened;
      else if (turn === 1) await second.opened;
      return real.put(key, value, options);
    },
    arrived: () => arrived,
    openFirst: () => first.open(),
    openSecond: () => second.open(),
    openAll: () => {
      first.open();
      second.open();
    },
  };
}
