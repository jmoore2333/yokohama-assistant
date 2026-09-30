import { closeReason, END_DELETED, END_GONE, endAtAdmission, frame, sentenceOf } from "../frames";
import type { Bump, Replied } from "../module";
import { log, respond } from "../respond";
import type { TableParts } from "../table";

/** The close reason of a deleted table's sockets: short and fixed, because
 *  the `end` frame before the close carries the meaning. */
const DELETED_REASON = "this table was deleted";

/** The two transport routes this object answers itself rather than through
 *  `dispatch`, whose route table has no row for either (spec §2.3). */
export type Transport = "events" | "stream";

/** What a socket carries across hibernation: the caller it was admitted as,
 *  exactly as its door's header named them (`forwardedCaller`) — the viewer
 *  every frame it is sent is computed for. */
interface Attachment {
  user: string | undefined;
}

/**
 * **The sockets** (spec §2.4 "Sockets", §2.7), mixed into the table object
 * over its base (`TableBase`, `src/table.ts`; M6.5c2 cycle D, Task 9): an
 * upgrade's admission (`admit`), the frames a request owes its sockets
 * (`sendFrames`), a deleted table's close, and the message handler the
 * platform calls.
 */
export function withSockets<T extends TableParts>(Base: T) {
  abstract class Sockets extends Base {
    /**
     * An upgrade's answer, from its admission — the request's own
     * `events(id, user, "")`, answered after the request's commit and its
     * frames (spec §2.4, "Sockets"):
     *
     * - **200**: the socket is accepted, hibernatable, with the caller as its
     *   attachment, and sent that reading at once — M6.3's first frame;
     * - **403 or 404**: the socket is ACCEPTED, sent an `end` that carries the
     *   layer's sentence (the objects spec's §2.4, `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md`), and
     *   closed 4403 or 4404 with that sentence as the reason — because a browser
     *   reports a refused upgrade as an anonymous failure it cannot tell from
     *   the network;
     * - **anything else** — the 401, an id the store will not use, a 500 — is
     *   answered unupgraded, exactly as the layer answered it (a browser cannot
     *   read a 401; spec §2.7's probe does).
     *
     * The socket is accepted after the request's frames were sent, so it is
     * sent only its first frame, which already reads whatever those frames
     * announced. Every socket accepted here is tagged with the table's id (spec
     * §2.4); nothing in this object reads the tag, since `sendFrames` frames
     * every socket the object holds.
     */
    protected admit(id: string, user: string | undefined, reply: Replied): Response {
      const { status, body } = reply;
      if (status !== 200 && status !== 403 && status !== 404) return respond(status, body);
      const pair = new WebSocketPair();
      const [client, server] = [pair[0], pair[1]];
      this.ctx.acceptWebSocket(server, [id]);
      if (status === 200) {
        server.serializeAttachment({ user } satisfies Attachment);
        server.send(frame("signal", body));
      } else {
        const sentence = sentenceOf(body);
        server.send(frame("end", endAtAdmission(sentence)));
        server.close(status === 403 ? 4403 : 4404, closeReason(sentence ?? ""));
      }
      return new Response(null, { status: 101, webSocket: client });
    }

    /**
     * The frames a request owes its sockets (spec §2.4, "The framing"), sent
     * right after its commit. `bumps` is every bump the request's calls made,
     * the wake's and the call's, and the whole list is ONE frame per socket,
     * however many bumps it holds: a finalizing keep carries `bump_table` then
     * `bump` (the plan's Task 2), and two identical frames would each wake a
     * screen.
     *
     * - An `end` naming this table wins over every other bump: each socket is
     *   sent `end {"reason":"deleted"}` and closed 4404.
     * - Else, a `bump` or a `bump_table` naming it: each socket's viewer — its
     *   attachment's `user` — is read with `events(id, user, "")`, pinned to
     *   P-2's reading like every call that can reach the layer's clock. A 200 is
     *   sent as a `signal`; a 403 or a 404 — an admission lost under a live
     *   socket, such as a stranger's at an open table whose last seat has just
     *   filled — is the SSE's own `end {"reason":"gone"}` and a 4403 or 4404
     *   close with the layer's sentence as the reason; any other answer sends
     *   NOTHING, the SSE's comment arm, and the socket stays open.
     * - Anything else — no bump, or only the house's chat — sends nothing.
     *
     * A socket that is not open (one this object has closed, whose close has not
     * completed) is passed over.
     *
     * **It never throws**: whatever the request wrote is already committed, so
     * nothing here may cost the request its reply. A trap is a side channel's (spec §2.4
     * "Traps"): logged, the module discarded, and no socket framed after it —
     * `guarded` would answer the next socket from an EMPTY module, whose reading
     * of this table is a 404 that would end a live socket. Any other throw is
     * logged `frame-failed` and ends the framing the same way, and the module is
     * dropped for `answer`'s reason: it may hold what storage does not. Either
     * way the next request builds a new module and reloads it. A framing reply
     * that itself carries bumps or an archive plan is logged
     * `frame-carried-bumps`.
     */
    sendFrames(id: string, bumps: Bump[]): void {
      const ended = bumps.some((b) => "end" in b && b.end === id);
      const bumped = bumps.some((b) => ("bump" in b && b.bump === id) || ("bump_table" in b && b.bump_table === id));
      // Bumps come only from a call into this object's module, so the instance
      // is set whenever there are any; the check narrows the type.
      const instance = this.instance;
      if ((!ended && !bumped) || instance === undefined) return;
      try {
        if (ended) {
          this.closeDeleted();
          return;
        }
        for (const ws of this.ctx.getWebSockets()) {
          if (ws.readyState !== WebSocket.OPEN) continue;
          const user = (ws.deserializeAttachment() as Attachment | null)?.user;
          const done = instance.guarded((mod) => this.pinned(mod, (m) => m.events(id, user, "")));
          if ("trapped" in done) {
            log("trap", { id, call: "frame", error: done.trapped });
            return;
          }
          const { status, body, bumps: carried, archive } = done.ok.reply;
          // A framing call only reads, and no known case pumps here: the wake
          // that loaded this module built the handle — `restore_picker` builds
          // it first wherever there is a game — and a finalizing keep leaves it
          // resident; at an open or dealing table there is no game to pump.
          // Should one ever carry bumps or an archive plan, this pass neither
          // frames nor owes them, and the line records that it happened.
          if (carried.length > 0 || archive !== undefined) {
            log("frame-carried-bumps", { id, bumps: carried, archive: archive !== undefined });
          }
          if (status === 200) {
            ws.send(frame("signal", body));
          } else if (status === 403 || status === 404) {
            ws.send(frame("end", END_GONE));
            ws.close(status === 403 ? 4403 : 4404, closeReason(sentenceOf(body) ?? ""));
          }
        }
      } catch (e) {
        instance.drop();
        log("frame-failed", { id, error: String(e) });
      }
    }

    /** Every open socket ended as a deleted table's: `end {"reason":"deleted"}`,
     *  then a 4404 close with this host's short reason. */
    private closeDeleted(): void {
      for (const ws of this.ctx.getWebSockets()) {
        if (ws.readyState !== WebSocket.OPEN) continue;
        ws.send(frame("end", END_DELETED));
        ws.close(4404, closeReason(DELETED_REASON));
      }
    }

    /** A force's end of every socket (`closeDeleted`), which never throws: the
     *  storage is already gone, or going, so nothing here may cost the force its
     *  answer. A throw is logged `frame-failed`. */
    protected endSockets(id: string): void {
      try {
        this.closeDeleted();
      } catch (e) {
        log("frame-failed", { id, error: String(e) });
      }
    }

    /**
     * A socket carries no move (spec §2.4): whatever a client sends is ignored,
     * so nothing a socket says reaches `dispatch` — a spectator's least of all.
     * The keep-alive's `ping` does not arrive here: the auto-response the
     * constructor sets answers it.
     */
    webSocketMessage(): void {}
  }
  return Sockets;
}

/** The transport route `path` names — `/api/games/{id}/events` or
 *  `/api/games/{id}/stream`, with `{id}` one non-empty segment — or
 *  `undefined` for every other path, which the layer's `dispatch` answers. */
export function transportOf(path: string): Transport | undefined {
  const [root, api, games, id, route, ...rest] = path.split("/");
  if (root !== "" || api !== "api" || games !== "games" || !id || rest.length > 0) return undefined;
  return route === "events" || route === "stream" ? route : undefined;
}

/** Whether a request asks to become a WebSocket: its `Upgrade` token,
 *  compared case-insensitively, as RFC 6455 §4.2.1 reads it. */
export function isUpgrade(headers: Headers): boolean {
  return headers.get("Upgrade")?.toLowerCase() === "websocket";
}
