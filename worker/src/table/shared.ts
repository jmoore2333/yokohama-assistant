import type { Exports } from "../../gen/factory.js";
import type { Marker } from "../marker";
import { deleteMeta, getMeta } from "../storage";

// What the table object's base and more than one of its parts read (M6.5c2
// cycle D, Task 9): the trap's sentence, the layer's documents and how they
// are read back out of the module, a load trap's reset, and the marker.

/** The trap 500's sentence, host-written (spec §2.4, §4). */
export const TRAP = "the table's engine stopped — try again";

/** The layer's documents a table object stores, in the order the wake loads
 *  them (spec §2.4). The fourth `docs` kind, `picker`, is the host's own. */
export const LAYER_DOCS = ["record", "table", "chat"] as const;
export type LayerDoc = (typeof LAYER_DOCS)[number];

/**
 * The layer's three documents as the module holds them. `read` refuses one of
 * these kinds — it throws its sentence, where a trap throws a `RuntimeError` —
 * only for an id the store will not use: nothing can be stored under such an
 * id, so there is nothing to compare, and the map is empty.
 */
export function readLayerDocs(mod: Exports, id: string): Map<LayerDoc, string | undefined> {
  const docs = new Map<LayerDoc, string | undefined>();
  for (const kind of LAYER_DOCS) {
    try {
      docs.set(kind, mod.read(kind, id));
    } catch (e) {
      if (typeof e !== "string") throw e;
      return new Map();
    }
  }
  return docs;
}

/** A load trap forgotten: the count, and the kind whose load trapped. */
export function clearLoadTraps(sql: SqlStorage): void {
  deleteMeta(sql, "load_traps");
  deleteMeta(sql, "poison");
}

/** `meta.marker` — the shell's own JSON — or `undefined` when none stands. */
export function markerOf(sql: SqlStorage): Marker | undefined {
  const text = getMeta(sql, "marker");
  return text === undefined ? undefined : (JSON.parse(text) as Marker);
}
