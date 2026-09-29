import { DurableObject } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import { Table } from "./table";

/**
 * **Every method of the table object is a method on its PROTOTYPE chain**
 * (M6.5c2 cycle D, Task 9; battery row 23). The pool patches prototypes, not
 * instances — `vi.spyOn(Table.prototype, "fetch")` (`worker.test.ts`,
 * `operator.test.ts`, `open/door.test.ts`), `"deliverSlot"` (`slot.test.ts`),
 * `"sendFrames"` and `"abandon"` (`sockets.test.ts`) — the plan's pool fact 1
 * ("patch an RPC target's PROTOTYPE"). A method that stops being one — made an
 * arrow-function field, which lives on each instance, or a free function —
 * leaves the prototype, and a spy set there then patches nothing the object
 * calls: its cell can pass while watching nothing. Since the split into a base
 * and four mixins (`withDelivery`, `withArchiving`, `withLifecycle`,
 * `withSockets`) the methods sit on five prototypes, and `vi.spyOn` finds one
 * up the chain (it shadows it on `Table.prototype`, and deletes the shadow to
 * restore).
 *
 * `METHODS` is the class's method map as it stood before the split (`src/table.ts`
 * at `ff6bdcb1`, read by TypeScript's parser), in that order. The chain must
 * carry EXACTLY these — a method added later is added here, which is what keeps
 * this list the whole map — each ONCE, since a mixin that re-declared a name
 * would silently shadow the one below it.
 *
 * Node runs this file (`npm run test:scripts`), where neither the Workers
 * runtime's `cloudflare:workers` nor the compiled module under `gen/` exists:
 * both are replaced by stand-ins. The class's shape is built by `table.ts` and
 * its mixins alone, the same code the pool loads; only the base the chain ends
 * at, and the module's exports, are the stand-ins'.
 */

vi.mock("cloudflare:workers", () => ({ DurableObject: class DurableObject {} }));
vi.mock("./module", () => ({ Instance: class Instance {}, pinnedTo: () => undefined, readReply: () => undefined }));

const METHODS = [
  "fetch",
  "alarm",
  "onAlarm",
  "markerAlarm",
  "end",
  "deliverSlot",
  "push",
  "acknowledged",
  "failed",
  "recopy",
  "deliverArchive",
  "upload",
  "settleUpload",
  "stamp",
  "retryArchive",
  "retryAt",
  "archiveOn",
  "archiveRung",
  "ended",
  "ready",
  "answer",
  "abandon",
  "deleting",
  "redispatch",
  "expire",
  "expiry",
  "unexpired",
  "operate",
  "force",
  "finish",
  "wake",
  "call",
  "admit",
  "sendFrames",
  "closeDeleted",
  "endSockets",
  "webSocketMessage",
  "commit",
  "owed",
  "owedWrites",
  "copyOf",
  "batch",
  "arm",
  "deadlines",
  "expiryDue",
  "dueBesidesArchive",
  "nextDue",
  "pinned",
  "reading",
] as const;

/** Each prototype from `Table.prototype` up to — not including —
 *  `DurableObject.prototype`, with the names it defines itself. */
function chain(): Array<{ proto: object; names: string[] }> {
  const found: Array<{ proto: object; names: string[] }> = [];
  let proto: object | null = Table.prototype;
  while (proto !== null && proto !== DurableObject.prototype) {
    found.push({ proto, names: Object.getOwnPropertyNames(proto).filter((n) => n !== "constructor") });
    proto = Object.getPrototypeOf(proto) as object | null;
  }
  // The premise: the walk ended at the base the class extends, not at
  // `Object.prototype` — so it read the whole chain and nothing past it.
  expect(proto, "premise: the chain ends at DurableObject.prototype").toBe(DurableObject.prototype);
  return found;
}

describe("the table object's methods (battery row 23)", () => {
  it("every method in the map is a function on Table.prototype's chain", () => {
    const table = Table.prototype as unknown as Record<string, unknown>;
    const off = METHODS.filter((name) => typeof table[name] !== "function");
    expect(off, "methods missing from the prototype chain").toEqual([]);
  });

  it("the chain carries exactly the map, each name once", () => {
    const names = chain().flatMap(({ names }) => names);
    const twice = names.filter((n, i) => names.indexOf(n) !== i);
    expect(twice, "names defined on two prototypes").toEqual([]);
    expect([...names].sort()).toEqual([...METHODS].sort());
  });
});
