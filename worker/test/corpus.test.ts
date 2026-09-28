import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { WIRE } from "./corpus.gen";
import { replaceOnce, settle, tableStub } from "./helpers";
import { ARCHIVE_WALK, WALK, WALK_HOST, walk } from "./walk";

// **The objects serve the corpus** (spec §5.3's pool half, R-6.5c.8; the
// plan's Task 12 Step 3; battery 7).
//
// `crates/yokohama-web/tests/wire_fixtures.rs` walks the axum router through a
// deterministic two-player game and freezes every payload the client reads.
// This replays THE SAME WALK through the objects — the table objects, the
// house over its scrubbed copies, the Worker's own `/api/art`, and a browser's
// saves through its caller's archive object and this file's bucket — with
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
 * header names the caller (`identity`, and `mode` on `/api/me`), how a change
 * reaches a browser (`signal`), and where its archive is kept (`archive`).
 * This host reads `cf-access`, `ws` and `r2`; the axum host the corpus was cut
 * from, `open`, `sse` and `off` — its primary `Api` keeps no archive.
 * `multiplayer` is NOT here: both hosts offer it.
 */
const HOST_FIELDS: Record<string, string[]> = {
  health: ["identity", "signal", "archive"],
  me: ["mode"],
  me_signed_out: ["mode"],
};

/** What one exempted field of a named reply came to: `spent` when this host
 *  sent it as a string that differs from the committed one; `no-string` when
 *  it sent no string for it, absent or of another type; `agrees` when both
 *  hosts sent the same value. */
type HostFieldVerdict = { field: string; verdict: "spent" | "no-string" | "agrees" };

/**
 * Every exempted field of `fixture`'s reply, judged by the PRESENCE rule (R3's
 * L-5): an exemption is spent only on a field this host SENT, as a string,
 * that differs from the committed one. A field this host dropped is no
 * difference between two hosts, and an exemption spent on it — or on a field
 * both hosts agree on — is a hole a later host could hide behind (M6.5b's
 * battery): `expect(undefined).not.toEqual(…)` passes, which is how the old
 * check here let an absent field through.
 *
 * **The rule's twin** is the live replay's, stated in
 * `worker/scripts/live-replay.mjs`'s `judge` (its "An exemption is spent only
 * on a value this host SENT" comment): the same rule over the same fields, one
 * statement per replay (ruling R-C3), each naming the other.
 */
function hostFields(fixture: string, committed: unknown, got: unknown): HostFieldVerdict[] {
  const sent = (typeof got === "object" && got !== null ? got : {}) as Record<string, unknown>;
  const cut = (typeof committed === "object" && committed !== null ? committed : {}) as Record<string, unknown>;
  return (HOST_FIELDS[fixture] ?? []).map((field) => {
    const value = sent[field];
    if (typeof value !== "string") return { field, verdict: "no-string" };
    return { field, verdict: value === cut[field] ? "agrees" : "spent" };
  });
}

/** `value` with `fixture`'s exempted fields taken out — a copy, and only of an
 *  object: anything else is answered as it is. */
function withoutHostFields<T>(fixture: string, value: T): T {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return value;
  const copy: Record<string, unknown> = { ...(value as Record<string, unknown>) };
  for (const field of HOST_FIELDS[fixture] ?? []) delete copy[field];
  return copy as T;
}

/**
 * The third column: every committed file no row of the replay names on this
 * host, SORTED. It is derived below exactly as the node replay derives its
 * own, and stated back here, so a fixture that stopped being routed appears by
 * name and a new one cannot join the corpus without landing in a column.
 *
 * - `site_space_words`, `identity_cases` and `seed_cases` are contracts,
 *   never a route's payload.
 * - `watching`, `locked` and the four `why`/explanation files are built in
 *   process by the emitter's `constructed()`: a deterministic bot-free walk
 *   cannot reach them.
 * - `house_chat_off_404` comes from an `Api` of the emitter's with the lobby
 *   chat switched off, which no script replays. (The eight `archive_*` files
 *   are `archive-walk.json`'s, which this host replays.)
 * - `production_bot` is a CONTRACT the client imports (M6.5c2), never a
 *   route's payload.
 * - `table_bot`, `foreign_bot_409`, `table_foreign` and `why_foreign` come from
 *   two more `Api`s of the emitter's (M6.5c2) — one holding a vs-bot table, one
 *   holding the committed pair re-seated against another bot — which no script
 *   replays.
 *
 * The node replay's other four — `art_manifest` and the three `events*` — are
 * `walk-host.json`'s rows, which this host serves.
 */
const THIRD_COLUMN = [
  "explanation_terminal",
  "foreign_bot_409",
  "house_chat_off_404",
  "identity_cases",
  "locked",
  "production_bot",
  "seed_cases",
  "site_space_words",
  "table_bot",
  "table_foreign",
  "watching",
  "why",
  "why_foreign",
  "why_refusals",
  "why_refused",
];

describe("the objects serve the corpus (spec §5.3)", () => {
  it("replays the walk through the objects and every named reply is its committed file", { timeout: 600_000 }, async () => {
    const named = new Set<string>();
    const sent = await walk({
      host: true,
      archive: true,
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
        const committed: unknown = JSON.parse(text);
        const got: unknown = reply.text ? JSON.parse(reply.text) : null;
        // An exempted field must be SENT by this host, as a string, and differ
        // from the committed one (`hostFields`, the live replay's rule's twin;
        // R3's L-5): an exemption spent on a field this host dropped, or on
        // one both hosts agree on, is a hole a later host could hide behind.
        for (const { field, verdict } of hostFields(row.fixture, committed, got)) {
          expect(verdict, `${row.fixture}.${field} is exempted: this host must send it as a string, and differ`).toBe("spent");
        }
        expect(withoutHostFields(row.fixture, got), `${row.fixture} (${label})`).toEqual(withoutHostFields(row.fixture, committed));
      },
    });
    expect(sent, "every row of the three scripts was sent").toBe(WALK.length + WALK_HOST.length + ARCHIVE_WALK.length);

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

    // **The walk's own game over, archived** (spec §2.6's game-over site):
    // `wire` finished at the row that names `game_over`, and its record went
    // to this file's bucket after that reply, under the `test/` root — the
    // very bytes archive-walk.json's first send carries, which the emitter
    // read off the same walk's store. No reply above can show it: a save is
    // read only from the bucket, and no committed payload carries the stamp,
    // `archived_at`.
    await settle(tableStub("wire"));
    const saved = await env.ARCHIVE!.get("test/rev7/over/wire.json");
    const committed = (ARCHIVE_WALK[0]!.body as { record: string }).record;
    expect(committed, "premise: the emitter is the axum host").toContain('"run_id": "web/server"');
    // The record is one game's on every host, and differs by exactly the host
    // that dealt it (M6.5c2, spec §2.4): the emitter is the axum host, this
    // pool is the Worker, and `run.run_id` says so.
    expect(await saved?.text(), "the walk's game over saved its record, as this host names itself")
      .toBe(replaceOnce(committed, '"run_id": "web/server"', '"run_id": "web/worker"'));
  });
});
