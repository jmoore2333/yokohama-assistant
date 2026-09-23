import { House } from "../src/house";
import type { Slot } from "../src/slot";

/**
 * **The pool facts' technique, in one place** (the plan's pool facts 1 and 2):
 * a house push held, or failed once, by patching `House.prototype.apply` — the
 * PROTOTYPE, which the pool's RPC resolver reads through. An instance's own
 * property makes every RPC to it THROW rather than hang, which is how the
 * first two battery-25 cells were vacuous.
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

/** The house refuses the NEXT push — it throws, as an unreachable house's RPC
 *  does — and puts its own `apply` back as it does. The table receives the
 *  rejection and logs `slot-failed`; workerd ALSO prints each refusal on
 *  stderr as an "uncaught exception", raised at the pool's RPC wrapper
 *  (`@cloudflare/vitest-plugin`'s `test-internal.mjs`) — measured the same
 *  for a synchronous throw — so each refusal a cell arranges costs one such
 *  line. */
export function failApplyOnce(): { restore(): void } {
  const original = House.prototype.apply;
  House.prototype.apply = async function (this: House): Promise<Ack> {
    House.prototype.apply = original;
    throw new Error("the house refused this push (failApplyOnce)");
  };
  return {
    restore() {
      House.prototype.apply = original;
    },
  };
}
