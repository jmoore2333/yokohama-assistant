// **The walk both WORKER replays run** (M6.5c2 cycle D, Task 9; the M6.5c2
// spec §2.8's "`HOST_FIELDS`, `order()` … each stated once"): the pool's
// corpus replay (`test/walk.ts`, `test/corpus.test.ts`) and the live replay
// (`scripts/live-replay.mjs`) send the same three request scripts in the same
// order, and exempt the same fields as a host's. It is PURE — it imports
// nothing — because the pool runs it inside workerd, where no node module is
// on offer; `corpus-walk.d.mts` beside it is what the pool's TypeScript reads.
// The browser host's node replay (`web/src/local/corpus.node.test.ts`) keeps
// its own list: it is another host, entitled to other fields.

/**
 * The fields a HOST is entitled to differ on (spec §4's second class): which
 * header names the caller (`identity`, and `mode` on `/api/me`), how a change
 * reaches a browser (`signal`), and where its archive is kept (`archive`).
 * This host reads `cf-access`, `ws` and `r2`; the axum host the corpus was cut
 * from, `open`, `sse` and `off` — its primary `Api` keeps no archive.
 * `multiplayer` is NOT here: both hosts offer it.
 */
export const HOST_FIELDS = { health: ["identity", "signal", "archive"], me: ["mode"], me_signed_out: ["mode"] };

/**
 * The rows in the order they run, each with its script's name and its index
 * there: `walk-host.json`'s rows after the walk row their `after` names (`-1`:
 * before the first), then `archive-walk.json`'s. A script a replay does not
 * send is passed as `[]`.
 */
export function order(walk, host, archive) {
  const hosted = host.map((row, index) => ({ script: "walk-host.json", index, row }));
  const at = (after) => hosted.filter(({ row }) => row.after === after);
  const rows = [...at(-1)];
  walk.forEach((row, index) => rows.push({ script: "walk.json", index, row }, ...at(index)));
  if (rows.length !== walk.length + host.length) throw new Error("a host row names no walk row to follow");
  archive.forEach((row, index) => rows.push({ script: "archive-walk.json", index, row }));
  return rows;
}
