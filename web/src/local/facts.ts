// What each SOURCE this client talks to is like (spec §2.3, R-6.5b.12).
//
// **Per source, because there are now two.** Until M6.5b there was one server
// behind the page and four module-level values in `api.ts` said what it was
// like: which change transport it serves, whether it keeps an archive, whether
// the house chat answers, how long a message may be. A local game is answered
// by a module in this browser's own Worker, whose answers to those four
// questions are CONSTANTS — and a screen that read the server's values for a
// local game would offer the wrong controls the moment the two deployments
// disagreed.
//
// **The local record is a constant and not a read.** The Worker could answer
// `GET /api/health` — the corpus replay proves it does, byte-equal to
// `health.json` — but R-6.5b.12 says `/api/health` and `/api/me` have no local
// caller in the page: a readiness probe through the local health route would be
// one, and the facts it would report are these four literals. So they are
// written here once.

import type { Health } from "../types";

/** Which of the two this is about. A local GAME is `isLocal(id)`; everything
 *  else on this API is the server. */
export type Host = "local" | "server";

/** Which archive a deployment writes finished and abandoned games to: none, a
 *  directory beside the store, or (M6.5c) a bucket. */
export type ArchiveKind = "off" | "dir" | "r2";

/** The four things a screen asks about the source it is talking to. */
export interface DeploymentFacts {
  /** How a change ARRIVES: the stream, the held request, or — locally —
   *  nothing at all, because the bot's trailing run returns inside the reply
   *  and there is nobody else at the table to wait for. */
  signal: "sse" | "held" | "none";
  archive: ArchiveKind;
  lobbyChat: boolean;
  chatText: number;
}

/** The SERVER's record, whose signal is NARROWER: a server always serves one
 *  of the two transports, and `signalKind()`'s union is what `usePolled`'s
 *  held arm branches on. `"none"` is a local fact and is never reachable here
 *  — which is a type and not a comment. */
export interface ServerFacts extends DeploymentFacts {
  signal: "sse" | "held";
}

/** The module in this browser, and there is nothing to read it from.
 *
 *  `chatText` and `lobbyChat` are the layer's own defaults — the local host
 *  builds `ChatConfig::default()`, so its `/api/health` would report exactly
 *  these — and NOTHING in the client reads them for a local game: the lobby's
 *  chat panel is the SERVER's house, and a local table has no panel. They are
 *  written down rather than omitted so the record has one shape. */
const LOCAL: DeploymentFacts = { signal: "none", archive: "off", lobbyChat: true, chatText: 500 };

/** The server's, until `GET /api/health` answers.
 *
 *  Every default is the one the four values had before this file existed, and
 *  each is the safe direction: the stream is what the server prefers, so a
 *  wrong guess costs one failed open; the archive off costs a label that says
 *  less than the server does rather than one promising a save it will not
 *  make; the house chat on costs one refusal; the cap matches the server's own
 *  so a client that has not reached health yet has a box. */
let server: ServerFacts = { signal: "sse", archive: "off", lobbyChat: true, chatText: 500 };

/** What this source is like. One object per source, read on every ask — a
 *  caller holding the result across a health read would hold the defaults. */
export const facts = (source: Host): DeploymentFacts => (source === "local" ? LOCAL : server);

/** The server's, narrowly typed — what `signalKind()` reads. Same object as
 *  `facts("server")`; a second accessor rather than a cast, because the
 *  narrowing is real and belongs in the type. */
export const serverFacts = (): ServerFacts => server;

/** Everything the boot's ONE health read says, in one statement (M6.5b).
 *  Before this the four values were set by four calls at one call site. */
export const setServerFacts = (h: Health): void => {
  server = { signal: h.signal, archive: h.archive, lobbyChat: h.lobby_chat, chatText: h.chat_text };
};

/** One of them, for the four named setters `api.ts` keeps exporting and for
 *  the cells that move a single fact. */
export const patchServerFacts = (p: Partial<ServerFacts>): void => {
  server = { ...server, ...p };
};
