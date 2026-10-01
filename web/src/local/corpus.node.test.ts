// **The browser host serves the corpus** (M6.5b, spec §6.3).
//
// `crates/yokohama-web/tests/wire_fixtures.rs` walks the axum router through a
// deterministic two-player game and freezes every payload the client reads.
// This replays THE SAME WALK through the compiled WebAssembly module, in Node,
// and asserts that the module's reply to each named request is the committed
// file. The walk's questions are `web/src/wire-script/walk.json`; the answers
// are `web/src/wire/`.
//
// **What that buys, beyond "the module answers".** The layer is one
// `api::dispatch` and both hosts call it, so this is not two implementations
// agreeing — it is the claim that the LAYER is host-free in fact and not only
// in its dependency graph, across a compilation boundary to a different
// architecture, a different allocator and a different clock. Because the walk
// is bot-free and seeded, it is also a determinism claim across that boundary:
// two thousand moves land in the same places.
//
// **Its own project** (`vitest.node.config.ts`, `npm run test:wasm`) so that
// `npm test` keeps its count and never asks for a module it did not build.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));
const wire = path.resolve(here, "../wire");
const scriptFile = path.resolve(here, "../wire-script/walk.json");
const modulePath = path.resolve(here, "../../wasm-node/yokohama_wasm.js");

/** One row of `walk.json` — `ScriptRow` in `wire_fixtures.rs`. */
interface Row {
  who: string | null;
  method: string;
  path: string;
  query: string;
  body: unknown;
  status: number;
  fixture: string | null;
}

/**
 * The module's shape, declared HERE rather than imported from the generated
 * `.d.ts` (SDD ruling P-2).
 *
 * `typeof import("../../wasm-node/yokohama_wasm")` would make this file
 * untypecheckable wherever the module has not been built — which is the
 * `web-client` CI job, and any clone that has not run `npm run build:wasm`.
 * The cost is that this declaration can drift from the real exports; the
 * benefit is that `npm run typecheck` means the same thing everywhere. The
 * drift is caught on the first run, loudly, because a missing export is
 * `undefined` and calling it throws.
 */
interface WasmHost {
  init(): void;
  set_clock(seconds?: number | null): void;
  dispatch(
    user: string | null | undefined,
    method: string,
    path: string,
    query: string,
    content_type: string | null | undefined,
    body: Uint8Array,
  ): { status: number; body: string; free(): void };
}

const have = fs.existsSync(modulePath);
if (!have && !process.env.YOKOHAMA_WASM_OPTIONAL) {
  throw new Error(
    "the browser host's module is not built, so this command tests nothing — run `npm run " +
      "build:wasm` first (or set YOKOHAMA_WASM_OPTIONAL=1 to skip it, which tests NOTHING)",
  );
}
if (!have) {
  // **Straight to the file descriptor, not through `console`.** vitest buffers
  // a test file's console output and attaches it to the test that produced it;
  // a warning emitted while COLLECTING a file whose only suite is then skipped
  // is attached to nothing and never printed — measured. The opt-out has to say
  // out loud that it tested nothing, or it is a silent green.
  process.stderr.write(
    "\nYOKOHAMA_WASM_OPTIONAL is set and the module is absent: the wasm corpus replay ran NO\n" +
      "assertions at all. Nothing in web/src/wire/ was checked against the browser host.\n\n",
  );
}

/**
 * The fields a HOST is entitled to differ on (spec §5, second column).
 *
 * `identity` is which header names the caller and `signal` is how a change
 * reaches a browser — the axum host reads `open` and `sse`, the browser host
 * `local` and `none`. `mode` on `/api/me` is the same fact from the client's
 * side. `multiplayer` (M6.5b2) is the third: the axum host the corpus is cut
 * from offers it and the browser host cannot, there being nobody else at the
 * table — and it is exactly NOT `serves_games`, which is true on both, since a
 * local game is played through the game routes.
 *
 * **`archive` is deliberately absent from this list.** Both hosts read `"off"`
 * — the corpus's primary `Api` keeps `NoArchive` and so does the module — so
 * exempting it would hide a browser host that had somehow acquired one, which
 * is a real thing to notice rather than a deployment difference to allow.
 */
const HOST_FIELDS: Record<string, string[]> = {
  health: ["identity", "signal", "multiplayer"],
  me: ["mode"],
  me_signed_out: ["mode"],
};

/**
 * Spec §5's third column: every committed file no row of this walk produces on
 * this host.
 *
 * **Not "does not serve", which was wider than the truth** (whole-branch
 * review, vacuity #2). The module DOES serve `/why` — `browser_played_pin`
 * drives it through this very layer — and `watching`, `locked` and the two
 * other `why*` files below are out of this list for a different reason from
 * the transport's: they are shapes a deterministic bot-free WALK cannot reach,
 * not routes the host lacks.
 *
 * It is a DERIVED truth stated back, not a filing decision — the replay asserts
 * this set equals exactly the files `walk.json` does not name, so a fixture
 * that quietly stopped being routed appears here by name and a new one cannot
 * join the corpus without landing in a column. Several reasons, not one:
 *
 * - `art_manifest`, `site_space_words`, `identity_cases` and `seed_cases`
 *   were never route payloads: the first is the host's own art directory,
 *   the other three CONTRACTS the client holds itself to (the terminal's
 *   words, the key grammar's verdicts, a seed's verdict).
 * - `watching`, `locked`, `why`, `why_refused`, `explanation_terminal` and
 *   `why_refusals` are built in process by `constructed()`, because a router
 *   cannot reach them from a deterministic bot-free game. The ROUTES exist on
 *   this host — the `why` ones are what the committed pair is reconstructed
 *   through — and it is the walk that cannot ask for them.
 * - `events`, `events_dealing` and `events_open` are the TRANSPORT route, which
 *   axum serves and `api::dispatch` has no row for. A browser host has no
 *   stream to signal on, so it has nothing to answer here.
 * - `house_chat_off_404` and the eight `archive_*` files come from a second and
 *   a third `Api` — a deployment with the lobby chat off, and one with a real
 *   archive directory. The module is neither.
 * - `production_bot` is a CONTRACT the client imports (M6.5c2), never a
 *   route's payload.
 * - `table_bot`, `foreign_bot_409`, `table_foreign` and `why_foreign` come from
 *   two more `Api`s of the emitter's (M6.5c2) — one holding a vs-bot table, one
 *   holding the committed pair re-seated against another bot — which no script
 *   replays.
 * - `deal_bot_table` and `table_two_open` (cycle D) come from that same vs-bot
 *   table's `Api`, which no script replays.
 */
const NOT_SERVED = [
  "art_manifest",
  "site_space_words",
  "identity_cases",
  "seed_cases",
  "watching",
  "locked",
  "why",
  "why_refused",
  "explanation_terminal",
  "why_refusals",
  "events",
  "events_dealing",
  "events_open",
  "house_chat_off_404",
  "archive_post",
  "archive_post_identical",
  "archive_post_abandoned",
  "archive_409",
  "archive_400_rev",
  "archive_400_digest",
  "archive_400_replay",
  "archive_400_table",
  "production_bot",
  "table_bot",
  "deal_bot_table",
  "table_two_open",
  "foreign_bot_409",
  "table_foreign",
  "why_foreign",
];

/** `FROZEN_CREATED_AT` in `wire_fixtures.rs`, which the walk's own clock reads. */
const FROZEN_CREATED_AT = 1_789_000_000;

/** A reply's body, parsed — `null` for an empty one — and a body that is not
 *  JSON refused in a sentence naming the ROW that got it and quoting the
 *  start of what came back (M6.5c2 cycle D, dossier row 56). */
function replyOf(text: string, row: string): Record<string, unknown> | null {
  if (!text) return null;
  try {
    return JSON.parse(text) as Record<string, unknown>;
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    throw new Error(
      `${row} answered something that is not JSON: ${JSON.stringify(text.slice(0, 80))} (${why})`,
    );
  }
}

// **A reply that is not JSON names the ROW that got it** (M6.5c2 cycle D,
// dossier row 56). The walk is 2,209 requests long, and a bare `SyntaxError`
// from the parse — "Unexpected token '<'" — says nothing about which of them
// answered with something other than this API's shape. Its own suite, with no
// module: it is about the replay's reading of a reply, not about the module.
describe("the replay's reading of a reply", () => {
  it("refuses a reply that is not JSON by naming the row and quoting what came back", () => {
    expect(() => replyOf("<html>oops</html>", "GET /api/games/wire/log")).toThrow(
      /GET \/api\/games\/wire\/log answered something that is not JSON: "<html>oops<\/html>"/,
    );
  });

  it("parses a JSON reply, and reads an empty one as no body", () => {
    expect(replyOf('{"a":1}', "GET /api/me")).toEqual({ a: 1 });
    expect(replyOf("", "DELETE /api/games/wire")).toBeNull();
  });

  // **And the walk USES it** (fix round 1, review Minor 2): the two cells
  // above pin the helper, and a walk that went back to parsing its replies by
  // hand would leave both green. The walk's cell is read from this file and
  // must reach its replies through `replyOf` and parse none itself.
  it("is the only way the walk reads a reply", () => {
    const source = fs.readFileSync(fileURLToPath(import.meta.url), "utf8");
    // Anchored at the START OF A LINE, which this cell's own text never is:
    // the first draft searched for the bare title, found it in this very
    // line, and so read ITSELF — green over a walk that parsed by hand.
    const start = source.indexOf('\n  it("replays the walk through the module');
    expect(start, "the walk's cell is in this file").toBeGreaterThan(-1);
    const walk = source.slice(start, source.indexOf("\n  it(", start + 1));
    expect(walk, "anti-vacuity: the slice is the walk, which dispatches").toContain("wasm.dispatch(");
    expect(walk).toContain("replyOf(text, ");
    expect(walk).not.toMatch(/JSON\.parse\(\s*text\b/);
  });
});

describe.skipIf(!have)("the browser host serves the corpus", () => {
  it("replays the walk through the module and every named reply is its committed file", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
    wasm.init();
    // The same reading the corpus was cut under, so the two hosts stamp the
    // same second and the lobby's `(created_at desc, id asc)` falls to the
    // same tie-break the committed pages show.
    wasm.set_clock(FROZEN_CREATED_AT);

    const rows = JSON.parse(fs.readFileSync(scriptFile, "utf8")) as Row[];
    expect(rows.length, "the script is not empty").toBeGreaterThan(0);

    const encoder = new TextEncoder();
    const named = new Set<string>();

    for (const row of rows) {
      const hasBody = row.body !== null;
      const reply = wasm.dispatch(
        row.who ?? undefined,
        row.method,
        row.path,
        row.query,
        hasBody ? "application/json" : undefined,
        hasBody ? encoder.encode(JSON.stringify(row.body)) : new Uint8Array(),
      );
      const status = reply.status;
      const text = reply.body;
      reply.free();

      // EVERY row's status, named or not. An intermediate that answered
      // differently has left the module in a different state, and the next
      // named comparison would report a drift whose cause was here.
      expect(status, `${row.method} ${row.path}${row.query ? `?${row.query}` : ""}`).toBe(
        row.status,
      );
      // Every row is REPLAYED; only a named one is compared.
      if (row.fixture === null) continue;

      named.add(row.fixture);
      const committed = JSON.parse(
        fs.readFileSync(path.join(wire, `${row.fixture}.json`), "utf8"),
      ) as Record<string, unknown>;
      const got = replyOf(text, `${row.method} ${row.path}${row.query ? `?${row.query}` : ""}`) as Record<
        string,
        unknown
      >;
      // **An exempted field must really DIFFER between the two hosts.** The
      // exemption list is the replay's own accounting, and nothing else checks
      // it: the close's mutation battery added `archive` to `health`'s list and
      // every cell stayed green, because both hosts read `"off"` — an exemption
      // that changes nothing is a hole a later host could hide behind (a local
      // host reporting `"dir"`). So a field is stripped only after this asserts
      // that the module's value and the committed value are NOT equal; a field
      // that agrees does not belong on the list.
      for (const field of HOST_FIELDS[row.fixture!] ?? []) {
        // **Present, THEN different** (R-C20, the pool's presence rule stated
        // for this host): `expect(undefined).not.toEqual(x)` passes, so a
        // field the module stopped sending would be exempted unread. The rule
        // here is "present and differs", not the pool's "sent as a string",
        // because `multiplayer` is a boolean.
        expect(
          got,
          `${row.fixture}.${field} is exempted as host-parameterised, so the module must SEND it`,
        ).toHaveProperty(field);
        expect(
          got[field],
          `${row.fixture}.${field} is exempted as host-parameterised, but both hosts agree on it — take it off HOST_FIELDS`,
        ).not.toEqual(committed[field]);
      }
      const strip = (v: Record<string, unknown>): Record<string, unknown> => {
        const copy = { ...v };
        for (const field of HOST_FIELDS[row.fixture!] ?? []) delete copy[field];
        return copy;
      };
      expect(strip(got), `${row.fixture} (${row.method} ${row.path})`).toEqual(strip(committed));
    }

    // **The three columns, derived.** Every committed file is either named by
    // the script (and was just compared), or is one this host does not serve.
    const all = fs
      .readdirSync(wire)
      .filter((f) => f.endsWith(".json"))
      .map((f) => f.slice(0, -5));
    const third = all.filter((n) => !named.has(n)).sort();
    expect(third, "the files the script does not name").toEqual([...NOT_SERVED].sort());

    // And the second column is a column: each host-parameterized file was
    // actually replayed, so its exemption was spent rather than merely listed.
    for (const name of Object.keys(HOST_FIELDS)) {
      expect(named.has(name), `${name} is replayed with its named fields exempted`).toBe(true);
    }
  });

  it("keeps no archive, so the browser sends a finished game to the server's", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
    wasm.init();
    const reply = wasm.dispatch(
      "local@browser",
      "POST",
      "/api/archive",
      "",
      "application/json",
      new TextEncoder().encode(JSON.stringify({ record: "{}", table: "{}" })),
    );
    const status = reply.status;
    const text = reply.body;
    reply.free();

    expect(status).toBe(404);
    expect((JSON.parse(text) as { error: string }).error).toBe("this server keeps no archive");
  });
});
