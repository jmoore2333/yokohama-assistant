/**
 * **The durable slot's rules** (spec §2.5, R-6.5c.8): what a table object
 * pushes to the house, and the orderings that keep a late or repeated push
 * from undoing a newer one. Pure — no Workers API — so its cells run in node
 * (`slot.unit.test.ts`); `table.ts` and `house.ts` apply them.
 *
 * - A table holds a **generation**, drawn once, and a **revision**, raised by
 *   every slot it writes. The house orders every push by the pair.
 * - The house applies a push by `decide`, and acknowledges EVERY push —
 *   applied or ignored — with its `(generation, revision)`.
 * - The table records that acknowledgement as `acked` only when it
 *   `advances`, so an acknowledgement that arrives late never makes a newer
 *   one look undelivered, nor an older copy look delivered.
 */

/** The house object's one name: the Worker reaches it by this name
 *  (`index.ts`), and every table pushes its slot to it. */
export const HOUSE_NAME = "house";

/**
 * A table's generation: `at` is the shell's `Date.now()` when the table's
 * first batch drew it, in MILLISECONDS (the orchestrator's rulings I-43/I-44
 * and F-2) — not the table's `created_at`, the layer's seconds — and `rand`
 * is 16 lowercase hex digits, 64 random bits. Ordered by `cmpGen`. A table
 * an earlier deployment created keeps the generation it drew then, whose `at`
 * is a P-2 reading in seconds; every generation drawn since orders after it.
 */
export type Generation = { at: number; rand: string };

/**
 * What a table pushes to the house: its scrubbed copy — or `null`, a REMOVAL —
 * at `(generation, revision)`, with the table's last activity (milliseconds:
 * the newest `written_at` of its record, table and chat, or for a REMOVAL,
 * which removed them, the time it was written) and its SQLite size in bytes as
 * of this slot, which the operator's listing reads (spec §2.9). `attempt`
 * counts the pushes of THIS slot that failed, and sets its next retry
 * (`nextRetry`).
 */
export type Slot = {
  id: string;
  generation: Generation;
  revision: number;
  copy: string | null;
  last_active: number;
  bytes: number;
  attempt: number;
};

/** The house's knowledge of one id: the `(generation, revision)` of the copy
 *  it lists, if any, and every generation of that id it has retired. */
export type HouseRow = { current: { generation: Generation; revision: number } | null; retired: Generation[] };

/** A push's `(generation, revision)`: what the house acknowledges. */
export type Ack = { generation: Generation; revision: number };

/** Orders two generations: by `at`, then by `rand` — 16 hex digits each, so
 *  a string compare is a numeric one. Negative, zero or positive. */
export function cmpGen(a: Generation, b: Generation): number {
  if (a.at !== b.at) return a.at < b.at ? -1 : 1;
  return a.rand === b.rand ? 0 : a.rand < b.rand ? -1 : 1;
}

/**
 * The house's rule (spec §2.5): a push APPLIES when its generation is the
 * current one and its revision is higher; or when the id has no current
 * generation, and the pushed one is not retired and orders after every
 * retired one. Anything else is ignored — so a late push of a removed table is
 * ignored in either order, before or after a re-created table's first push,
 * and two interleaved pushes cannot regress a row. A removal (`copy: null`)
 * is ordered like a copy.
 */
export function decide(row: HouseRow, slot: Slot): "apply" | "ignore" {
  const g = slot.generation;
  if (row.retired.some((retired) => cmpGen(retired, g) === 0)) return "ignore";
  if (row.current !== null) {
    return cmpGen(row.current.generation, g) === 0 && slot.revision > row.current.revision ? "apply" : "ignore";
  }
  return row.retired.every((retired) => cmpGen(g, retired) > 0) ? "apply" : "ignore";
}

/** Whether an acknowledgement of `(g, r)` is newer than `acked`: a newer
 *  generation, whatever its revision, or a higher revision of the same one. */
export function advances(acked: Ack | null, g: Generation, r: number): boolean {
  if (acked === null) return true;
  const order = cmpGen(g, acked.generation);
  return order > 0 || (order === 0 && r > acked.revision);
}

/** The retries' backoff (spec §2.4), in milliseconds: the retry after the
 *  (`attempt` + 1)-th failure in a row — 30 s after the first, 2 min after
 *  the second, 10 min after the third, then hourly. A slot's failed pushes
 *  and an owed copy's traps both climb it. */
export function nextRetry(attempt: number): number {
  return [30_000, 120_000, 600_000][attempt] ?? 3_600_000;
}

/** SHA-256's round constants: the first 32 bits of the fractional parts of
 *  the cube roots of the first 64 primes (FIPS 180-4 §4.2.2). */
const K = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01,
  0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
  0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
  0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08,
  0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
  0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/** SHA-256's initial hash value (FIPS 180-4 §5.3.3). */
const H0 = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];

const rotr = (x: number, n: number) => (x >>> n) | (x << (32 - n));

/**
 * The digest `acked` keeps of the copy it acknowledges (spec §2.4): SHA-256 of
 * the copy's UTF-8 bytes, in lowercase hex. Written here rather than taken
 * from `crypto.subtle`, whose digest is asynchronous: the comparison it serves
 * runs inside a request's one synchronous turn, where nothing may be awaited
 * (spec §2.4).
 */
export function copyDigest(copy: string): string {
  const bytes = new TextEncoder().encode(copy);
  // The message, a 1 bit, zeros, and its length in BITS as 64 bits big-endian,
  // to a multiple of 64 bytes (FIPS 180-4 §5.1.1).
  const padded = new Uint8Array(Math.ceil((bytes.length + 9) / 64) * 64);
  padded.set(bytes);
  padded[bytes.length] = 0x80;
  const view = new DataView(padded.buffer);
  const bits = bytes.length * 8;
  view.setUint32(padded.length - 8, Math.floor(bits / 2 ** 32));
  view.setUint32(padded.length - 4, bits >>> 0);
  const h = [...H0];
  const w = new Array<number>(64).fill(0);
  for (let block = 0; block < padded.length; block += 64) {
    for (let t = 0; t < 16; t++) w[t] = view.getUint32(block + 4 * t);
    for (let t = 16; t < 64; t++) {
      const x = w[t - 15]!;
      const y = w[t - 2]!;
      const s0 = rotr(x, 7) ^ rotr(x, 18) ^ (x >>> 3);
      const s1 = rotr(y, 17) ^ rotr(y, 19) ^ (y >>> 10);
      w[t] = (w[t - 16]! + s0 + w[t - 7]! + s1) >>> 0;
    }
    let [a, b, c, d, e, f, g, k] = h as [number, number, number, number, number, number, number, number];
    for (let t = 0; t < 64; t++) {
      const t1 = (k + (rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25)) + ((e & f) ^ (~e & g)) + K[t]! + w[t]!) >>> 0;
      const t2 = ((rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22)) + ((a & b) ^ (a & c) ^ (b & c))) >>> 0;
      k = g;
      g = f;
      f = e;
      e = (d + t1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (t1 + t2) >>> 0;
    }
    [a, b, c, d, e, f, g, k].forEach((v, i) => (h[i] = (h[i]! + v) >>> 0));
  }
  return h.map((v) => v.toString(16).padStart(8, "0")).join("");
}
