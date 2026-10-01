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
 * reaches a browser (`signal`), where its archive is kept (`archive`), and how
 * long an idle table, and one still waiting for players, stands
 * (`table_ttl_days`, `open_table_ttl_days`; R-6.5c2.42). This host reads
 * `cf-access`, `ws`, `r2`, 90 and 7; the axum host the corpus was cut from,
 * `open`, `sse`, `off` — its primary `Api` keeps no archive — and `null` twice,
 * since it keeps no TTL. `multiplayer` is NOT here: both hosts offer it.
 *
 * **`saves` is a host field in waiting** (cycle E's final review, Minor 2). A
 * table view's `saves` — a table read or a lobby row — is the abandon rule
 * asked of the view, and the rule reads the host's archive: a playing table
 * with an applied move, not archived at its end, reads `true` here (`r2`) and
 * `false` in the corpus (`off`). Every table payload these scripts replay
 * reads `has_events: false` today, so the two hosts agree; the corpus's one
 * `saves: true`, `table_foreign.json`, comes from an emitter `Api` no script
 * replays. A row added that reads a table AFTER its first applied move exempts
 * `saves` here, under that row's name, or says why the two hosts agree on it.
 */
export const HOST_FIELDS = {
  health: ["identity", "signal", "archive", "table_ttl_days", "open_table_ttl_days"],
  me: ["mode"],
  me_signed_out: ["mode"],
};

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
