import { DurableObject } from "cloudflare:workers";
import type { Exports, Reply } from "../gen/factory.js";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { frame } from "./frames";
import { Instance, readReply, type Replied } from "./module";
import { errorBody, log, respond } from "./respond";
import { deleteDoc, ensureTables, readDocs, upsertDoc } from "./storage";
import { isUpgrade, NOT_AN_UPGRADE, USER_HEADER } from "./table";

/** The house chat's one id — the layer's `chat::HOUSE`, the only id its store
 *  takes for that document. It is also the object's name. */
const HOUSE = "house";

/** The tag every lobby socket is accepted under (spec §2.5). */
export const LOBBY_TAG = "lobby";

/** The house's trap 500, host-written in the table's shape (spec §2.4,
 *  "Traps"). */
const TRAP = "the lobby's engine stopped — try again";

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
 * **The house object, first cut** (spec §2.5; the plan's Task 11 — Task 12
 * brings the table copies, their slot and `seq`'s increments): ONE object,
 * named `house`, whose module is a layer instance holding the house chat.
 *
 * - Every request but the lobby socket is `dispatch`: `/api/health`,
 *   `/api/me`, `/api/labels`, `/api/chat`, `GET /api/games` — which lists an
 *   empty store until Task 12's copies — and every `/api/*` the Worker routes
 *   nowhere else, whose answer is the layer's own 404 or 405.
 * - The house chat is persisted as ONE `docs` row of kind `house`, written by
 *   the request that changed it in one batch, and loaded into every new
 *   instance before its first call — so it survives an eviction.
 * - The lobby socket, `GET /api/stream`, is admitted by the module's
 *   `identity`: its 401 is answered unupgraded; otherwise the socket is
 *   accepted under the tag `lobby` with `{ user }`, and sent NOTHING on
 *   connect, as the NAS's lobby stream writes nothing on connect. A non-`GET`
 *   is 405 with an empty body, and a `GET` that is not an upgrade the table's
 *   426 (the orchestrator's ruling I-48).
 * - A reply carrying `bump_house_chat` sends every lobby socket ONE frame,
 *   `frame("house", house_frame(seq, chat))` — `chat` being the house chat's
 *   `last` — after its batch.
 */
export class House extends DurableObject<Env> {
  /** The Worker's variables, read once. */
  private readonly config: ShellConfig | { error: string };
  /** The module (spec §2.2), built at the first request that needs it. */
  private instance: Instance | undefined;
  /** The lobby frame's `seq` (spec §2.5): the house's own counter, raised by
   *  every copy and removal the house applies — none until Task 12 — and kept
   *  in memory only, as the NAS keeps its own. */
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
   * A NEW module gets the stored house chat before its first call. `init`'s
   * refusal is the 500 that carries its sentence; a trap while building or
   * loading is the house's trap 500, with the module discarded. Answers
   * `undefined` when the module is ready.
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
    const stored = readDocs(this.ctx.storage.sql).get("house");
    if (stored === undefined) return undefined;
    try {
      built.mod.load("house", HOUSE, stored);
    } catch (e) {
      instance.drop();
      if (e instanceof WebAssembly.RuntimeError) {
        log("load-trap", { id: HOUSE, kind: "house", error: String(e) });
        return respond(500, errorBody(TRAP));
      }
      throw e;
    }
    return undefined;
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
    const sql = this.ctx.storage.sql;
    if (house === readDocs(sql).get("house")) return;
    this.ctx.storage.transactionSync(() => {
      ensureTables(sql);
      if (house === undefined) deleteDoc(sql, "house");
      else upsertDoc(sql, "house", house, Date.now());
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
