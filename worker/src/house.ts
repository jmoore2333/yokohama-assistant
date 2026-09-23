import { DurableObject } from "cloudflare:workers";
import type { Exports, Reply } from "../gen/factory.js";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { frame } from "./frames";
import { Instance, readReply, type Replied } from "./module";
import { errorBody, log, respond } from "./respond";
import { decide, type Ack, type HouseRow, type Slot } from "./slot";
import { deleteDoc, deleteMeta, ensureTables, getMeta, readDocs, setMeta, upsertDoc } from "./storage";
import { isUpgrade, NOT_AN_UPGRADE, USER_HEADER } from "./table";

/** The house chat's one id — the layer's `chat::HOUSE`, the only id its store
 *  takes for that document. It is also the object's name (`HOUSE_NAME`). */
const HOUSE = "house";

/** The tag every lobby socket is accepted under (spec §2.5). */
export const LOBBY_TAG = "lobby";

/** The house's trap 500, host-written in the table's shape (spec §2.4,
 *  "Traps"). */
const TRAP = "the lobby's engine stopped — try again";

/** How long the house keeps a retired generation: 90 days (spec §2.5). */
const RETIRED_MS = 90 * 24 * 60 * 60 * 1000;

/** What a lobby socket carries across hibernation: the caller it was admitted
 *  as, exactly as `X-Yokohama-User` named them. */
interface Attachment {
  user: string | undefined;
}

/** One request, as `fetch` read it — its body included. */
interface Incoming {
  user: string | undefined;
  /** `GET /api/stream`, an upgrade: the lobby socket. */
  stream: boolean;
  method: string;
  path: string;
  query: string;
  contentType: string | undefined;
  body: Uint8Array;
}

/**
 * **The house object** (spec §2.5): ONE object, named `house`, whose module is
 * a second layer instance holding the house chat and a SCRUBBED COPY of each
 * table's document, as that table last pushed it — so `GET /api/games` is the
 * layer's own listing over those copies, byte-identical to the NAS's by
 * construction.
 *
 * - Every request but the lobby socket is `dispatch`: `/api/health`,
 *   `/api/me`, `/api/labels`, `/api/chat`, `GET /api/games`, and every
 *   `/api/*` the Worker routes nowhere else, whose answer is the layer's own
 *   404 or 405.
 * - **The copies** arrive only through `apply`, each table's push of its
 *   slot, and are kept in two tables of the house's own: `copies` (the copy
 *   the house lists for an id, its `(generation, revision)`, and its last
 *   activity and size) and `retired` (every generation of an id the house has
 *   removed, pruned after 90 days) — the state `decide` orders every push by.
 * - **The house chat** is ONE `docs` row of kind `house`, written by the
 *   request that changed it, in one batch.
 * - **Every new module** is handed the stored chat, then every copy, before
 *   its first call (`wake`), so the listing survives an eviction.
 * - The lobby socket, `GET /api/stream`, is admitted by the module's
 *   `identity`: its 401 is answered unupgraded; otherwise the socket is
 *   accepted under the tag `lobby` with `{ user }`, and sent NOTHING on
 *   connect, as the NAS's lobby stream writes nothing on connect. A non-`GET`
 *   is 405 with an empty body, and a `GET` that is not an upgrade the table's
 *   426 (the orchestrator's ruling I-48).
 * - Every applied copy or removal, and every reply carrying
 *   `bump_house_chat`, sends every lobby socket ONE frame,
 *   `frame("house", house_frame(seq, chat))` — `chat` being the house chat's
 *   `last` — after its batch.
 */
export class House extends DurableObject<Env> {
  /** The Worker's variables, read once. */
  private readonly config: ShellConfig | { error: string };
  /** The module (spec §2.2), built at the first request that needs it. */
  private instance: Instance | undefined;
  /** The lobby frame's `seq` (spec §2.5): the house's own counter, raised by
   *  every copy and removal the house applies, and kept in memory only, as
   *  the NAS keeps its own: a frame is a wake-up, and the client refetches
   *  the lobby on any frame whatever it carries. */
  private seq = 0;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.config = shellConfig(env);
    // The client's keep-alive (spec §2.7), answered by the platform without
    // waking the house, as the table's is (the orchestrator's ruling I-36).
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const stream = url.pathname === "/api/stream";
    if (stream && req.method !== "GET") return respond(405, "");
    if (stream && !isUpgrade(req.headers)) return respond(426, errorBody(NOT_AN_UPGRADE));
    const incoming: Incoming = {
      user: req.headers.get(USER_HEADER) ?? undefined,
      stream,
      method: req.method,
      path: url.pathname,
      query: url.search.slice(1),
      contentType: req.headers.get("content-type") ?? undefined,
      // The body, read once: the LAST await before the reply, so the wake, the
      // call and the batch run in one synchronous turn, as the table's do.
      body: new Uint8Array(await req.arrayBuffer()),
    };
    return this.answer(incoming);
  }

  /**
   * **A table's push** (spec §2.5) — an RPC method, and the only way a copy
   * reaches the house. It is ordered by `decide` against this id's current
   * `(generation, revision)` and its retired generations:
   *
   * - **ignored**: logged `slot-ignored`, nothing written;
   * - **applied**: ONE batch — a copy replaces the id's row; a REMOVAL deletes
   *   it and retires its generation; retired generations older than 90 days
   *   are pruned — then `seq` rises, and the module and every lobby socket
   *   are brought in line (`mirror`).
   *
   * It ACKNOWLEDGES every push, applied or ignored, with the pushed pair, so
   * a straggler is cleared too. Its body never awaits, so no other event
   * interleaves between the decision and its batch.
   */
  async apply(slot: Slot): Promise<Ack> {
    const ack: Ack = { generation: slot.generation, revision: slot.revision };
    const sql = this.ctx.storage.sql;
    const row = houseRow(sql, slot.id);
    if (decide(row, slot) === "ignore") {
      log("slot-ignored", { id: slot.id, generation: slot.generation, revision: slot.revision, current: row.current });
      return ack;
    }
    const now = Date.now();
    this.ctx.storage.transactionSync(() => {
      ensureCopies(sql);
      if (slot.copy === null) {
        sql.exec("DELETE FROM copies WHERE id = ?", slot.id);
        sql.exec(
          "INSERT OR REPLACE INTO retired (id, gen_at, gen_rand, at) VALUES (?, ?, ?, ?)",
          slot.id,
          slot.generation.at,
          slot.generation.rand,
          now,
        );
      } else {
        sql.exec(
          "INSERT INTO copies (id, json, gen_at, gen_rand, revision, last_active, bytes) VALUES (?, ?, ?, ?, ?, ?, ?) " +
            "ON CONFLICT (id) DO UPDATE SET json = excluded.json, gen_at = excluded.gen_at, gen_rand = excluded.gen_rand, " +
            "revision = excluded.revision, last_active = excluded.last_active, bytes = excluded.bytes",
          slot.id,
          slot.copy,
          slot.generation.at,
          slot.generation.rand,
          slot.revision,
          slot.last_active,
          slot.bytes,
        );
      }
      sql.exec("DELETE FROM retired WHERE at < ?", now - RETIRED_MS);
    });
    this.seq += 1;
    this.mirror(slot);
    return ack;
  }

  /** The request path: the wake, the call, the batch, the frames, the reply. */
  private answer(incoming: Incoming): Response {
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id: HOUSE, error: config.error });
      return respond(500, errorBody(config.error));
    }
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, null)));
    const refused = this.wake(instance);
    if (refused !== undefined) return refused;
    if (incoming.stream) return this.admit(instance, incoming.user);
    let reply: Replied;
    let house: string | undefined;
    try {
      const done = instance.guarded((mod) => ({
        reply: this.pinned(mod, (m) =>
          m.dispatch(incoming.user, incoming.method, incoming.path, incoming.query, incoming.contentType, incoming.body),
        ),
        house: mod.read("house", HOUSE),
      }));
      if ("trapped" in done) {
        log("trap", { id: HOUSE, call: "dispatch", error: done.trapped });
        return respond(500, errorBody(TRAP));
      }
      ({ reply, house } = done.ok);
      this.commit(house);
    } catch (e) {
      // As the table's request path: the module may hold what storage does
      // not, so it is rebuilt from what IS stored.
      instance.drop();
      throw e;
    }
    if (reply.bumps.some((b) => "bump_house_chat" in b)) this.frameLobby(instance, house);
    return respond(reply.status, reply.body);
  }

  /**
   * A NEW module gets the house's stored documents before its first call:
   * the house chat, then every copy. Answers `undefined` when the module is
   * ready, or the answer that ends the request.
   *
   * - `init`'s refusal is the 500 that carries its sentence; a trap while
   *   building is the house's trap 500.
   * - **A trap loading the house chat** is counted in `load_traps`, and
   *   answered with the trap 500 — until the SECOND consecutive one, which
   *   DELETES the chat row (logged `house-chat-dropped`) and serves without
   *   it: the house has no poison, and the lobby never stays 500 (the
   *   orchestrator's ruling on Task 11's review). A load that succeeds resets
   *   the count.
   * - A trap handing it a copy is the trap 500, the module discarded and
   *   nothing counted: `put_copy` stores the text it is handed without
   *   reading it, so no copy's content can be what traps.
   */
  private wake(instance: Instance): Response | undefined {
    let built: { mod: Exports; fresh: boolean };
    try {
      built = instance.get();
    } catch (e) {
      if (typeof e === "string") {
        log("init-refused", { id: HOUSE, error: e });
        return respond(500, errorBody(e));
      }
      if (e instanceof WebAssembly.RuntimeError) {
        log("trap", { id: HOUSE, call: "init", error: String(e) });
        return respond(500, errorBody(TRAP));
      }
      throw e;
    }
    if (!built.fresh) return undefined;
    const sql = this.ctx.storage.sql;
    const chat = readDocs(sql).get("house");
    const traps = Number(getMeta(sql, "load_traps") ?? "0");
    try {
      if (chat !== undefined) built.mod.load("house", HOUSE, chat);
    } catch (e) {
      instance.drop();
      if (!(e instanceof WebAssembly.RuntimeError)) throw e;
      const count = traps + 1;
      if (count >= 2) {
        this.batch((sql) => {
          deleteDoc(sql, "house");
          deleteMeta(sql, "load_traps");
        });
        log("house-chat-dropped", { id: HOUSE, traps: count, error: String(e) });
        // Once more, with no chat stored: this wake cannot reach here again.
        return this.wake(instance);
      }
      this.batch((sql) => setMeta(sql, "load_traps", String(count)));
      log("load-trap", { id: HOUSE, kind: "house", traps: count, error: String(e) });
      return respond(500, errorBody(TRAP));
    }
    try {
      // ONE id (Task 5's escalation 3): each copy goes in under the key it
      // was pushed with, its table's name, which its document names as `id`.
      for (const { id, json } of storedCopies(sql)) built.mod.put_copy(id, json);
    } catch (e) {
      instance.drop();
      if (!(e instanceof WebAssembly.RuntimeError)) throw e;
      log("load-trap", { id: HOUSE, kind: "copy", error: String(e) });
      return respond(500, errorBody(TRAP));
    }
    if (traps > 0) this.batch((sql) => deleteMeta(sql, "load_traps"));
    return undefined;
  }

  /**
   * An applied push, after its batch, in the module and on the lobby: the
   * module is woken if it is new — which loads what the batch just wrote — and
   * handed the copy (`put_copy`, replace-or-create) or its removal
   * (`remove_copy`, absent-ok), then every lobby socket is framed. The push
   * is already applied, so its acknowledgement stands whatever happens here:
   * a wake that fails has logged why, and a trap or any other throw is logged
   * and the module discarded, for the next request to rebuild from what is
   * stored.
   *
   * ONE id through `house_copy`, `put_copy` and `remove_copy` (Task 5's
   * escalation 3): the slot's `id` is the table object's name — the key the
   * copy is stored and removed under here — and the listing reads each row's
   * id from the DOCUMENT, which the layer wrote with that same name.
   */
  private mirror(slot: Slot): void {
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id: HOUSE, error: config.error });
      return;
    }
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, null)));
    try {
      if (this.wake(instance) !== undefined) return;
      const done = instance.guarded((mod) => {
        if (slot.copy === null) mod.remove_copy(slot.id);
        else mod.put_copy(slot.id, slot.copy);
      });
      if ("trapped" in done) {
        log("trap", { id: HOUSE, call: slot.copy === null ? "remove_copy" : "put_copy", error: done.trapped });
        return;
      }
    } catch (e) {
      instance.drop();
      log("frame-failed", { id: HOUSE, error: String(e) });
      return;
    }
    this.frameLobby(instance, readDocs(this.ctx.storage.sql).get("house"));
  }

  /** The lobby socket's admission (spec §2.5): the module's `identity`. */
  private admit(instance: Instance, user: string | undefined): Response {
    const done = instance.guarded((mod) => readReply(mod.identity(user)));
    if ("trapped" in done) {
      log("trap", { id: HOUSE, call: "identity", error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    const { status, body } = done.ok;
    if (status !== 200) return respond(status, body);
    const pair = new WebSocketPair();
    const [client, server] = [pair[0], pair[1]];
    this.ctx.acceptWebSocket(server, [LOBBY_TAG]);
    server.serializeAttachment({ user } satisfies Attachment);
    return new Response(null, { status: 101, webSocket: client });
  }

  /** The house chat as the module now holds it, written in ONE batch when it
   *  differs from the stored row — so a request that changed nothing writes
   *  nothing. */
  private commit(house: string | undefined): void {
    if (house === readDocs(this.ctx.storage.sql).get("house")) return;
    this.batch((sql) => {
      if (house === undefined) deleteDoc(sql, "house");
      else upsertDoc(sql, "house", house, Date.now());
    });
  }

  /** A write batch over the house's `docs` and `meta`: ONE
   *  `transactionSync`, which first creates the two tables. */
  private batch(writes: (sql: SqlStorage) => void): void {
    const sql = this.ctx.storage.sql;
    this.ctx.storage.transactionSync(() => {
      ensureTables(sql);
      writes(sql);
    });
  }

  /**
   * One frame to every open lobby socket, after the batch: the output gate
   * holds it until the batch is durable (spec §0.4). It never throws — the
   * write is committed, so nothing here may cost the request its reply: a
   * trap, or any other throw, is logged and the module discarded.
   */
  private frameLobby(instance: Instance, house: string | undefined): void {
    try {
      const done = instance.guarded((mod) => mod.house_frame(this.seq, lastOf(house)));
      if ("trapped" in done) {
        log("trap", { id: HOUSE, call: "house_frame", error: done.trapped });
        return;
      }
      const message = frame("house", done.ok);
      for (const ws of this.ctx.getWebSockets(LOBBY_TAG)) {
        if (ws.readyState === WebSocket.OPEN) ws.send(message);
      }
    } catch (e) {
      instance.drop();
      log("frame-failed", { id: HOUSE, error: String(e) });
    }
  }

  /** A lobby socket carries nothing to the house: whatever a client sends is
   *  ignored. The keep-alive's `ping` does not arrive here: the auto-response
   *  the constructor sets answers it. */
  webSocketMessage(): void {}

  /** One module call pinned to P-2's reading, answered as plain data. */
  private pinned(mod: Exports, f: (mod: Exports) => Reply): Replied {
    if ("error" in this.config) throw new Error(this.config.error);
    mod.set_clock(nextReading(this.config.clock));
    return readReply(f(mod));
  }
}

/** The house chat's `last` — the newest message number ever given — out of
 *  its document, which carries no seed and so may be parsed (spec §2.5); `0`
 *  with no document. */
function lastOf(house: string | undefined): number {
  if (house === undefined) return 0;
  const last = (JSON.parse(house) as { last?: unknown }).last;
  return typeof last === "number" ? last : 0;
}

/**
 * The house's two tables of copies, created by the first `apply` that applies
 * a push and never by a read: `copies`, one row per listed id — its copy as
 * the exact text the table pushed, never parsed here, with its
 * `(generation, revision)`, last activity and size; and `retired`, one row
 * per removed generation, with when the house retired it.
 */
function ensureCopies(sql: SqlStorage): void {
  sql.exec(
    "CREATE TABLE IF NOT EXISTS copies (id TEXT PRIMARY KEY, json TEXT NOT NULL, gen_at INTEGER NOT NULL, " +
      "gen_rand TEXT NOT NULL, revision INTEGER NOT NULL, last_active INTEGER NOT NULL, bytes INTEGER NOT NULL)",
  );
  sql.exec(
    "CREATE TABLE IF NOT EXISTS retired (id TEXT NOT NULL, gen_at INTEGER NOT NULL, gen_rand TEXT NOT NULL, " +
      "at INTEGER NOT NULL, PRIMARY KEY (id, gen_at, gen_rand))",
  );
}

/** Whether the first applied push has created the two tables. */
function hasCopies(sql: SqlStorage): boolean {
  const { n } = sql
    .exec<{ n: number }>("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name IN ('copies', 'retired')")
    .one();
  return n === 2;
}

/** What the house knows of `id` (`decide`'s `HouseRow`): the pair of the copy
 *  it lists, and every generation it has retired. */
function houseRow(sql: SqlStorage, id: string): HouseRow {
  if (!hasCopies(sql)) return { current: null, retired: [] };
  const row = sql
    .exec<{ gen_at: number; gen_rand: string; revision: number }>("SELECT gen_at, gen_rand, revision FROM copies WHERE id = ?", id)
    .toArray()[0];
  const retired = sql
    .exec<{ gen_at: number; gen_rand: string }>("SELECT gen_at, gen_rand FROM retired WHERE id = ?", id)
    .toArray()
    .map((r) => ({ at: r.gen_at, rand: r.gen_rand }));
  return {
    current: row === undefined ? null : { generation: { at: row.gen_at, rand: row.gen_rand }, revision: row.revision },
    retired,
  };
}

/** Every stored copy, by id: what a new module is handed. */
function storedCopies(sql: SqlStorage): Array<{ id: string; json: string }> {
  if (!hasCopies(sql)) return [];
  return sql.exec<{ id: string; json: string }>("SELECT id, json FROM copies ORDER BY id").toArray();
}
