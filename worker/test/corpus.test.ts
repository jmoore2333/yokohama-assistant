import { describe, expect, it } from "vitest";
import { WIRE } from "./corpus.gen";
import { WALK, WALK_HOST, walk } from "./walk";

// **The objects serve the corpus** (spec §5.3's pool half, R-6.5c.8; the
// plan's Task 12 Step 3; battery 7).
//
// `crates/yokohama-web/tests/wire_fixtures.rs` walks the axum router through a
// deterministic two-player game and freezes every payload the client reads.
// This replays THE SAME WALK through the objects — the table objects, the
// house over its scrubbed copies, and the Worker's own `/api/art` — with
// `X-Yokohama-User` set as the Worker sets it and the module's clock frozen at
// the corpus's `1_789_000_000` (`wrangler.test.jsonc`, the plan's ruling
// P-2), and asserts that every named reply is the committed file. It is
// `web/src/local/corpus.node.test.ts`'s shape, one host further: the lobby
// rows come from the house's COPIES, so a scrub that changed what a row reads
// fails here by name.
//
// ONE file, and no other file shares its objects (pool fact 5): the walk's ids
// are fixed, and the house's listing must hold the walk's tables and nothing
// else, as the committed pages do.

/**
 * The fields a HOST is entitled to differ on (spec §4's second class): which
 * header names the caller (`identity`, and `mode` on `/api/me`) and how a
 * change reaches a browser (`signal`). This host reads `cf-access` and `ws`;
 * the axum host the corpus was cut from, `claimed` and `sse`. `multiplayer` is
 * NOT here — both hosts offer it — nor is `archive`: both read `off` in the
 * pool, so exempting it would hide a host that had acquired one.
 */
const HOST_FIELDS: Record<string, string[]> = {
  health: ["identity", "signal"],
  me: ["mode"],
  me_signed_out: ["mode"],
};

/**
 * The third column: every committed file no row of the replay names on this
 * host, SORTED. It is derived below exactly as the node replay derives its
 * own, and stated back here, so a fixture that stopped being routed appears by
 * name and a new one cannot join the corpus without landing in a column.
 *
 * - `site_space_words` is a contract, never a route's payload.
 * - `watching`, `locked` and the four `why`/explanation files are built in
 *   process by the emitter's `constructed()`: a deterministic bot-free walk
 *   cannot reach them.
 * - `house_chat_off_404` and the eight `archive_*` files come from the
 *   emitter's second and third `Api`s — a lobby chat switched off, and a real
 *   archive. The eight are replayed from `archive-walk.json` once this host
 *   serves the archive (the plan's Task 13).
 *
 * The node replay's other four — `art_manifest` and the three `events*` — are
 * `walk-host.json`'s rows, which this host serves.
 */
const THIRD_COLUMN = [
  "archive_400_digest",
  "archive_400_replay",
  "archive_400_rev",
  "archive_400_table",
  "archive_409",
  "archive_post",
  "archive_post_abandoned",
  "archive_post_identical",
  "explanation_terminal",
  "house_chat_off_404",
  "locked",
  "site_space_words",
  "watching",
  "why",
  "why_refusals",
  "why_refused",
];

describe("the objects serve the corpus (spec §5.3)", () => {
  it("replays the walk through the objects and every named reply is its committed file", { timeout: 600_000 }, async () => {
    const named = new Set<string>();
    const sent = await walk({
      host: true,
      onRow: (_index, row, reply) => {
        const label = `${row.method} ${row.path}${row.query ? `?${row.query}` : ""} as ${row.who ?? "no one"}`;
        // EVERY row's status, named or not: a row that answered differently
        // has left the objects in a different state, and the next named
        // comparison would report a drift whose cause was here.
        expect(reply.status, label).toBe(row.status);
        if (row.fixture === null) return;
        named.add(row.fixture);
        const text = WIRE[row.fixture];
        if (text === undefined) throw new Error(`${row.fixture} is named by the walk and not committed`);
        const committed = JSON.parse(text) as Record<string, unknown>;
        const got = (reply.text ? JSON.parse(reply.text) : null) as Record<string, unknown>;
        // An exempted field must really DIFFER between the two hosts: an
        // exemption that differs nowhere is a hole a later host could hide
        // behind (M6.5b's battery).
        for (const field of HOST_FIELDS[row.fixture] ?? []) {
          expect(got[field], `${row.fixture}.${field} is exempted, but both hosts agree on it`).not.toEqual(committed[field]);
        }
        const strip = (v: Record<string, unknown>): Record<string, unknown> => {
          const copy = { ...v };
          for (const field of HOST_FIELDS[row.fixture!] ?? []) delete copy[field];
          return copy;
        };
        expect(strip(got), `${row.fixture} (${label})`).toEqual(strip(committed));
      },
    });
    expect(sent, "every row of both scripts was sent").toBe(WALK.length + WALK_HOST.length);

    // **The three columns, derived**: every committed file is named by a row
    // (and was just compared), or is one this host does not serve.
    const third = Object.keys(WIRE)
      .filter((name) => !named.has(name))
      .sort();
    expect(third, "the files the scripts do not name").toEqual(THIRD_COLUMN);
    // The second column is a column: each exempted file was replayed, so its
    // exemption was spent rather than merely listed.
    for (const name of Object.keys(HOST_FIELDS)) {
      expect(named.has(name), `${name} is replayed with its named fields exempted`).toBe(true);
    }
  });
});
