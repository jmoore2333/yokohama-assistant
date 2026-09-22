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

/** What a screen asks about the source it is talking to. */
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
 *  — which is a type and not a comment.
 *
 *  **Two fields the local host has no answer to** (M6.5b2): which door this
 *  deployment reads, and whether it serves multiplayer at all. Both are
 *  questions about a SERVER, so they live here rather than on `DeploymentFacts`
 *  — a local game is played by this browser and has no door and no second
 *  seat. */
export interface ServerFacts extends DeploymentFacts {
  signal: "sse" | "held";
  /** `Health.identity` — the door's name. Empty until a health read has landed,
   *  which is a THIRD state and not a mode: nothing has been asked. */
  identity: string;
  /** `Health.multiplayer` — whether the game, chat and stream routes exist at
   *  all (R-6.5b2.4). The BUILD value is what the front door draws from; this
   *  is what the deployment actually serves. */
  multiplayer: boolean;
  /** `Health.semantics_rev`, `Health.rules_hash` and `Health.server_version` —
   *  the three EXPORTER-PROVENANCE fields a backup envelope carries (M6.5b2,
   *  spec §2.5).
   *
   *  **Here rather than asked for at export time, and that is the point.** They
   *  are facts only Rust knows, and the one control that must work with the
   *  module dead is the backup — so a browser writes down what it was last
   *  told and quotes that, instead of asking a Worker that may not answer.
   *  `0` and `""` are the THIRD state `identity: ""` already is: nothing has
   *  ever said. Nothing reads them for a decision; a restore re-derives every
   *  verdict from the games themselves. */
  semanticsRev: number;
  rulesHash: string;
  appVersion: string;
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
 *  Every default is the one each value had before this file existed, and each
 *  is the safe direction: the stream is what the server prefers, so a wrong
 *  guess costs one failed open; the archive off costs a label that says less
 *  than the server does rather than one promising a save it will not make; the
 *  house chat on costs one refusal; the cap matches the server's own so a
 *  client that has not reached health yet has a box. The door's name is EMPTY
 *  rather than a guess at one of three modes, and multiplayer is on for the
 *  reason the build value's absence is: a deployment that has said nothing
 *  serves both.
 *
 *  **"Off" is a DEFAULT and not an answer, and since M6.5b2 the difference is
 *  readable**: `takenAt` below is `null` until a read has landed, which is what
 *  lets `ensureArchiveFact` say `"unknown"` rather than repeat this guess. */
const DEFAULTS: ServerFacts = {
  signal: "sse",
  archive: "off",
  lobbyChat: true,
  chatText: 500,
  identity: "",
  multiplayer: true,
  semanticsRev: 0,
  rulesHash: "",
  appVersion: "",
};

/** Where the last health read is kept, so a reload does not have to ask again
 *  (spec §2.2). One key, in the shape `chatFold.ts` uses. */
const FACTS_KEY = "yokohama.server_facts";

/** What is written under it: the facts, and WHEN they were taken.
 *
 *  **The timestamp is the whole point of remembering at all.** A record with no
 *  age can never be revalidated, so a browser that once read `archive: "off"`
 *  would authorize Abandon's plain delete for ever after the deployment turned
 *  its archive on — with no mechanism that could correct it. */
interface FactsRecord {
  takenAt: number;
  facts: ServerFacts;
}

/** The archive kinds, as VALUES — so a record read back from storage can be
 *  checked against them rather than trusted (review M-8). */
const ARCHIVE_KINDS: readonly string[] = ["off", "dir", "r2"];

/** One stored record, with every field it carries checked against the type it
 *  is declared as (review M-8).
 *
 *  **A missing field falls back to its default and a WRONG one does too**,
 *  which is the half the first draft left open: a hand-edited record could put
 *  a non-union `archive` straight into a render condition, and `"dir "` with a
 *  trailing space would have read as an archive that keeps games. What is
 *  written under this key is only ever this client's own last health read, so
 *  this costs nothing in the ordinary case and closes the one that is not
 *  ordinary. */
function sane(parsed: Partial<ServerFacts> | undefined): ServerFacts {
  const p = parsed ?? {};
  const str = (v: unknown, d: string) => (typeof v === "string" ? v : d);
  const num = (v: unknown, d: number) => (typeof v === "number" && Number.isFinite(v) ? v : d);
  return {
    signal: p.signal === "held" || p.signal === "sse" ? p.signal : DEFAULTS.signal,
    archive: ARCHIVE_KINDS.includes(p.archive as string)
      ? (p.archive as ServerFacts["archive"])
      : DEFAULTS.archive,
    lobbyChat: typeof p.lobbyChat === "boolean" ? p.lobbyChat : DEFAULTS.lobbyChat,
    chatText: num(p.chatText, DEFAULTS.chatText),
    identity: str(p.identity, DEFAULTS.identity),
    multiplayer: typeof p.multiplayer === "boolean" ? p.multiplayer : DEFAULTS.multiplayer,
    semanticsRev: num(p.semanticsRev, DEFAULTS.semanticsRev),
    rulesHash: str(p.rulesHash, DEFAULTS.rulesHash),
    appVersion: str(p.appVersion, DEFAULTS.appVersion),
  };
}

/** The record in storage, or `null`. A storage that throws — a private window,
 *  a blocked site — reads as nothing remembered and never as an error, the
 *  `claimedUser` idiom; so does a value that is not this record's shape.
 *
 *  **Module-private since M6.5b2** (review M-8): the record's TIMESTAMP is
 *  this module's own business, and [`recallServerFacts`] is what a reader
 *  outside asks. */
function recallServerRecord(): FactsRecord | null {
  try {
    if (typeof localStorage === "undefined") return null;
    const raw = localStorage.getItem(FACTS_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<FactsRecord>;
    if (typeof parsed?.takenAt !== "number" || !parsed.facts) return null;
    // **A missing field falls back to its default, and so does a wrong one**
    // (review M-8): a record written by an older release is missing whatever
    // that release did not know about, and one edited by hand can carry
    // anything at all.
    return { takenAt: parsed.takenAt, facts: sane(parsed.facts) };
  } catch {
    return null;
  }
}

/** The remembered facts alone — what a screen asks for when it wants to know
 *  whether this browser has ever been told anything. `null` means it has not,
 *  which is NOT the same as the defaults above. */
export const recallServerFacts = (): ServerFacts | null => recallServerRecord()?.facts ?? null;

/** What this browser last read, taken from storage at module load so a reload
 *  starts where the last visit left off rather than at the guesses. */
const remembered = recallServerRecord();
let server: ServerFacts = remembered?.facts ?? DEFAULTS;
let takenAt: number | null = remembered?.takenAt ?? null;

/** What this source is like. One object per source, read on every ask — a
 *  caller holding the result across a health read would hold the defaults. */
export const facts = (source: Host): DeploymentFacts => (source === "local" ? LOCAL : server);

/** The server's, narrowly typed — what `signalKind()` reads. Same object as
 *  `facts("server")`; a second accessor rather than a cast, because the
 *  narrowing is real and belongs in the type. */
export const serverFacts = (): ServerFacts => server;

/** When the record this browser holds was read from the server, or `null` where
 *  nothing has ever answered. `ensureArchiveFact`'s freshness rule is the one
 *  reader. */
export const serverFactsTakenAt = (): number | null => takenAt;

/**
 * What this browser KNOWS about the deployment's archive (M6.5b2, spec §2.6).
 *
 * **Three answers, and the third is the whole of R-6.5b2.13.** `"off"` is a
 * claim about the deployment; a browser that has never been told anything has
 * no grounds for it, and the difference is exactly `takenAt`. Before this
 * existed, the default `"off"` and a real `"off"` were the same value — which
 * is how a first-visit Abandon deleted a local game without ever trying to
 * save it (spec §0.3).
 *
 * **The ONE synchronous reader** (review M-9): `App`'s screen and `Abandon`'s
 * label both go through the hook over this, so the fact has one accessor
 * rather than two kept in step by hand.
 */
export const archiveAnswer = (): ArchiveKind | "unknown" =>
  takenAt === null ? "unknown" : server.archive;

/** Whoever is drawing something off these facts.
 *
 *  **A listener and not a poll**, because the fact is read at three moments a
 *  render has already happened at (spec §2.2): a module value that changes
 *  triggers no re-render, and the screens that care — the send control, the
 *  Abandon label — would otherwise go on drawing the answer they mounted
 *  with. */
const listeners = new Set<() => void>();

export const onServerFacts = (f: () => void): (() => void) => {
  listeners.add(f);
  return () => {
    listeners.delete(f);
  };
};

const announce = (): void => {
  for (const f of [...listeners]) f();
};

/** Everything ONE health read says, in one statement — held in memory AND
 *  written down with the time it was taken (M6.5b2, spec §2.2).
 *
 *  **One function and not two**, because a setter that recorded the facts
 *  without stamping them would leave a record that can never go stale, which is
 *  exactly the failure the stamp exists to prevent. `takenAt` is a parameter so
 *  a cell can age a record without waiting an hour. */
export const rememberServerFacts = (h: Health, at: number = Date.now()): void => {
  server = {
    signal: h.signal,
    archive: h.archive,
    lobbyChat: h.lobby_chat,
    chatText: h.chat_text,
    identity: h.identity,
    multiplayer: h.multiplayer,
    semanticsRev: h.semantics_rev,
    rulesHash: h.rules_hash,
    appVersion: h.server_version,
  };
  takenAt = at;
  try {
    localStorage.setItem(FACTS_KEY, JSON.stringify({ takenAt: at, facts: server }));
  } catch {
    /* storage refused: this page keeps the facts, the next one asks again */
  }
  announce();
};

/** One of them, for the named setters `api.ts` keeps exporting and for the
 *  cells that move a single fact.
 *
 *  **It does not stamp and must not**: a patch is a test's or a caller's edit
 *  of one value, not evidence that the server said anything. */
export const patchServerFacts = (p: Partial<ServerFacts>): void => {
  server = { ...server, ...p };
  announce();
};

/** Put this module back where a fresh page load finds it, WITHOUT clearing
 *  storage — which is the distinction the cells are about: a reload keeps the
 *  written record and loses the module's state, and `recallServerFacts` is
 *  what brings the record back. */
export const resetFactsForTest = (): void => {
  server = DEFAULTS;
  takenAt = null;
  announce();
};
