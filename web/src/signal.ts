// The change signal (M6.1 §6, room plan §1), and since M6.3 the TRANSPORT
// itself (spec §5.1).
//
// **One hook, four consumers.** `usePolled` owns the visibility rule (R-P8),
// the connection, the backoff and the three refusals that end a signal rather
// than slowing it down; `useSignal` is the instance whose resource is a game's
// stream, and the dealing screen, the lobby and the open table are the other
// three. That generalization is what made M6.3 one change: the reader under
// this hook was replaced and nothing downstream of a consumer's callback knows
// how the answer arrived.
//
// **There is no interval any more** (R-3.2). A connected reader is a promise
// waiting on a body, so a visible tab holds ONE open request per resource and
// arms NOTHING; the server writes a frame when something moves. The timer that
// is left serves only the two cases where there is no connection to wait on:
// the ladder after a failure, and the held-request arm's re-ask.
//
// **The reader is a `fetch`, not an `EventSource`** (spec §0.1) — `sse.ts` says
// why in one line: `EventSource` cannot carry a header, and the `claimed` door
// is one. It is also the reason a hidden tab can let go at all, since an
// `AbortController` comes with the `fetch` and not with the other.
//
// **The signal is deliberately separate from the fetch** (ruling R-P9). A
// `signal` frame is four fields and no frame built, so a client learns "this
// table now has N events" for a few bytes and goes and gets a board only when
// that moved.
//
// **Nothing is read from a hidden tab** (ruling R-P8). A browser with eight
// backgrounded games is otherwise eight held connections for as long as it is
// open, for boards nobody is looking at. On return the reader is opened again
// and the server's ON-CONNECT frame is the ask — and the hook says "refreshing"
// meanwhile, because the board on screen is as old as the tab was hidden and
// "live" would be a claim it cannot make yet.

import { useEffect, useRef, useState } from "react";
import { api, ApiError, signalKind } from "./api";
import { readStream } from "./sse";
import type { Events } from "./types";

/** The wait after the 1st, 2nd, … consecutive failure, and then forever.
 *
 *  **It ends at a ceiling rather than growing without bound**, because the
 *  thing on the other end is a home server behind Cloudflare Access: a client
 *  that backed off to ten minutes would leave a player staring at a stale board
 *  long after the server came back, and one that never backed off would hammer
 *  it hardest exactly while it was least able to answer. */
export const BACKOFF_MS = [2000, 4000, 8000, 16000, 30000] as const;

/** How long the HELD arm waits before asking again after an answer.
 *
 *  **Not zero, and that is ruling R-pre-5.** The game's `once` is
 *  `GET …/events?wait=25`, which holds on the server and so could be reopened
 *  at once; the lobby's and the open table's are ordinary reads that answer
 *  immediately, and reopening one of those at zero is a hot loop against three
 *  table listings. One value for both arms, at the cadence the poll this
 *  replaces used to run at — it is the fallback transport, and the fallback's
 *  job is to be no worse than what came before. */
export const HELD_REASK_MS = 3000;

/** How long the game's held request asks the server to wait, in seconds. The
 *  server's own ceiling is 25 (spec §3.3); asking for more is refused. */
const HELD_WAIT_S = 25;

/** What the signal can currently say for itself.
 *
 *  `live` is the resting state and shows NOTHING on screen — a permanent green
 *  dot is a thing people stop seeing. The other two are worth a word each. */
export type Link = "live" | "refreshing" | "reconnecting";

/** How long to wait after `failures` consecutive failures. Clamped at both
 *  ends, so a caller can hand it any count including a negative one. */
export function nextDelay(failures: number): number {
  return BACKOFF_MS[Math.min(Math.max(failures, 0), BACKOFF_MS.length - 1)]!;
}

/** The resource a signal watches (spec §5.5, §5.1).
 *
 *  **`key` is the effect's ONLY dependency beside `active`**: a stable string
 *  for the resource — a game id, the literal `"lobby"` — while `once`, `stream`
 *  and every callback ride in refs. A consumer's closure is a new function on
 *  every render, so a `Source` object is rebuilt each time and its identity must
 *  not matter.
 *
 *  **`stream` is what the SSE arm reads and `once` is what the held arm reads**,
 *  and both are declared because a deployment decides which (see `signalKind`).
 *  A `"wake"` source reads `once` under BOTH arms — see `frames`. */
export interface Source<T> {
  key: string;
  once: () => Promise<T>;
  /** The resource's stream URL. */
  stream: string;
  /** What a frame off that stream MEANS, and it is not the same question for
   *  all four consumers (ruling R-7.1).
   *
   *  `"value"` — the frame's `data` IS a `T`, parsed and handed straight to
   *  `onValue`. The game and the dealing screen declare this, and it is what
   *  makes R-3.2 true for them: the whole answer arrives on the stream and
   *  `GET …/events` is never called at all.
   *
   *  `"wake"` (the default) — the frame is a wake-up and `once()` is the
   *  answer. The open table's value is a `TableView` and the lobby's is two
   *  listings, while the frames they receive are an `Events` and a bare
   *  sequence number (R-3.4, and deliberately so: a `watch` channel coalesces,
   *  so a frame naming a table would sometimes name the wrong one). Handing
   *  such a frame over as a `T` would have the lobby render `{seq: 4}`. */
  frames?: "value" | "wake";
}

/** Why a signal STOPPED, as opposed to failing and backing off.
 *
 *  The server said this stream is over (`deleted`), the resource is gone (404)
 *  or it is not ours to read (403). None of the three changes by being asked
 *  again, while the screen goes on saying "reconnecting…" about something that
 *  is not coming back. */
export interface End {
  reason: "deleted" | "gone" | "refused";
  sentence?: string;
}

/**
 * Watch `source` while the tab is VISIBLE and `active`, and hand every answer
 * to `onValue`.
 *
 * `active` is the caller's "there is something to wait for": a screen holding
 * the prompt has nothing to wait on, and a finished game has nothing to wait
 * for at all. A `source` of `null` is the same answer for "no resource on
 * screen".
 *
 * **`source`, `onValue` and `onEnd` are read through refs, so the effect
 * installs ONCE.** Every consumer's callback closes over state and is therefore
 * a new function on every render; an effect that depended on one would abort
 * its reader and open another each time — a connection per render. The refs are
 * written during render, which is the idiom `Dealing`'s own hand-over callback
 * already uses.
 *
 * **There is no 401 handling here and there must not be.** `api.ts` routes
 * every 401 to the door from inside `call` (ruling R-7.1 of M6.1), so a
 * signed-out screen unmounts; a second handler here would be a second thing
 * racing to decide which screen this is. The STREAM's own 401 is not routed
 * there — `readStream` does not go through `call` — and does not need to be: it
 * lands on the ladder, and the next ordinary call takes the player to the door.
 */
export function usePolled<T>(
  source: Source<T> | null,
  active: boolean,
  onValue: (v: T) => void,
  onEnd?: (end: End) => void,
): Link {
  const [link, setLink] = useState<Link>("live");
  /** Consecutive failures, as a REF: it is read by the next attempt and shown
   *  by nothing, so a state variable would only re-render the whole consumer
   *  once per failed open. */
  const failures = useRef(0);
  const src = useRef(source);
  src.current = source;
  const latest = useRef(onValue);
  latest.current = onValue;
  const ended = useRef(onEnd);
  ended.current = onEnd;
  const key = source?.key ?? null;

  useEffect(() => {
    // **The link describes a signal that is RUNNING, so one that stops stops
    // saying anything.** Without this the last word stands after the effect is
    // torn down and nothing is left to clear it: switch games from the menu
    // while the server is unreachable and the new game's banner opens reading
    // "reconnecting…" about a connection nobody opened. A no-op on mount and on
    // every re-run that was already live — React bails on an identical value.
    setLink("live");
    if (!key || !active) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    // **The cleanup's flag, and it is what makes a read in flight harmless.**
    // Aborting stops the reader; it does nothing about a frame already being
    // handed over or a `once` already awaiting its answer, which would
    // otherwise land after the screen is gone and set state on an unmounted
    // tree.
    let stopped = false;
    let controller: AbortController | null = null;
    const visible = () => document.visibilityState === "visible";

    const schedule = (ms: number) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void open(), ms);
    };

    // ---- the `"wake"` arm's re-read ----------------------------------------
    //
    // **Coalesced, because a frame is cheap and the read behind it is not.**
    // The lobby's `once` is three table listings, and a game under way bumps the
    // house on every move; without this a burst of frames would queue a listing
    // each. One read at a time, and a frame that arrives during one is not lost
    // — it sets `again`, and the read is repeated once the first has landed.
    let reading = false;
    let again = false;
    const reread = async () => {
      if (reading) {
        again = true;
        return;
      }
      reading = true;
      try {
        for (;;) {
          again = false;
          const v = await src.current!.once();
          if (stopped) return;
          failures.current = 0;
          setLink("live");
          latest.current(v);
          if (!again) return;
        }
      } catch (e) {
        if (stopped) return;
        if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
          // The resource this stream is about is gone, or is not ours. The
          // reader goes with it: a connection kept open to a table we may not
          // read would deliver frames nothing can act on.
          controller?.abort();
          ended.current?.({
            reason: e.status === 404 ? "gone" : "refused",
            sentence: e.message,
          });
          return;
        }
        // The STREAM is still good and the re-read is what failed, so this is
        // not a trip round the ladder — the next frame tries again. The word on
        // screen is the true one meanwhile.
        setLink("reconnecting");
      } finally {
        reading = false;
      }
    };

    const open = async () => {
      if (stopped || !visible() || !src.current) return;
      // **The controller this attempt owns**, held locally as well as in
      // `controller`: by the time the catch runs, a visibility change may
      // already have replaced the shared one, and "was I the one aborted?" is a
      // question only this attempt's own signal can answer.
      const mine = new AbortController();
      controller = mine;
      try {
        if (signalKind() === "held") {
          // R-plan-5: the same answer, one request at a time. The game's `once`
          // holds on the server for up to 25 s; the others answer at once and
          // are re-asked on the clock.
          //
          // **This one is not aborted by a hidden tab, and does not need to
          // be.** `once` goes through `api.ts`'s `call`, which takes no signal,
          // so a held request in flight runs to its deadline; what R-P8 forbids
          // is ASKING from a hidden tab, and the next ask is the one `open`
          // refuses at its first line. That is what the poll this replaces did
          // with a reply in flight, for the same reason.
          const v = await src.current.once();
          if (stopped) return;
          failures.current = 0;
          setLink("live");
          latest.current(v);
          schedule(HELD_REASK_MS);
          return;
        }
        const frames = src.current.frames ?? "wake";
        const outcome = await readStream(
          src.current.stream,
          mine.signal,
          (f) => {
          if (stopped) return;
          if (f.event === "end") {
            // The server's own reason rides the frame: `deleted` for a table
            // that was abandoned, `gone` for one that stopped being readable
            // under a live connection.
            const { reason } = JSON.parse(f.data) as { reason: "deleted" | "gone" };
            ended.current?.({ reason });
            return;
          }
          if (f.event !== "signal" && f.event !== "house") return;
          if (frames === "value") latest.current(JSON.parse(f.data) as T);
          else void reread();
          },
          // **The CONNECTION, and it is what both of these hang on.**
          //
          // The word first: a `fetch` that has been issued is not a link, so
          // saying "live" above this call said it when the request went out.
          // During an outage that made the word blink off at every rung of the
          // ladder, and against a server that accepts a connection and then
          // hangs it read "live" for ever while nothing was connected. Spec
          // §5.1 ties the word to Connected, and this is that moment.
          //
          // The failure count for the same reason, and the house channel is
          // why it cannot be reset on a frame instead: `/api/stream` writes
          // NOTHING on connect (R-3.4), so a lobby reconnecting to a quiet
          // server would climb 2 s, 4 s, 8 s to the ceiling while every one of
          // those connections was made perfectly well. A 200 with a stream body
          // IS the connection — and a server that accepts and closes at once is
          // still bounded, by the first rung.
          () => {
            if (stopped) return;
            failures.current = 0;
            setLink("live");
          },
        );
        if (stopped || outcome === "ended") return;
        // The body ended with nothing said: a dropped connection, and the
        // ladder is what that is for.
        throw new Error("the stream closed");
      } catch (e) {
        if (stopped || mine.signal.aborted) return;
        // **The two refusals that END this rather than slowing it down.** See
        // `End`; nothing is scheduled after either line.
        if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
          ended.current?.({
            reason: e.status === 404 ? "gone" : "refused",
            sentence: e.message,
          });
          return;
        }
        // **A client that cannot reach the server is not a refusal to show the
        // player.** The board on screen is still the last true thing this
        // client was told; what is wrong is the link, and that is what the word
        // says. The error itself is deliberately not carried up — a sentence in
        // the error strip every few seconds would bury the refusals that are
        // about a move the player actually made.
        setLink("reconnecting");
        const wait = nextDelay(failures.current);
        failures.current += 1;
        schedule(wait);
      } finally {
        // **One reader per hook instance, on every way out of this function.**
        // A throw out of `onFrame` — a consumer's own callback — rejects
        // `readStream` and ABANDONS the body without letting it go, so the
        // ladder would open a second reader while the first still held its
        // connection, once per failure. Aborting here is a no-op on the paths
        // that already ended (the stream said `end`, the tab hid, the effect
        // was torn down) and runs after the catch's own `mine.signal.aborted`
        // test, which is what tells "I was aborted" from "I failed".
        mine.abort();
      }
    };

    const onVisibility = () => {
      if (visible()) {
        setLink("refreshing");
        // At once, and not on a delay: the whole point of letting go while
        // hidden is that coming back is when you want to know.
        schedule(0);
      } else {
        controller?.abort();
        controller = null;
        if (timer) {
          clearTimeout(timer);
          timer = null;
        }
      }
    };

    document.addEventListener("visibilitychange", onVisibility);
    if (visible()) schedule(0);
    return () => {
      stopped = true;
      controller?.abort();
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [key, active]);

  return link;
}

/**
 * The game's change signal — `GET …/{id}/stream` — as ONE INSTANCE of
 * [`usePolled`].
 *
 * Its contract is unchanged and its cells are the same cells: what moved is
 * where the visibility rule, the connection and the backoff live, so the lobby
 * and the open table get them without a second copy.
 *
 * **`frames: "value"`** (R-7.1): a `signal` frame's body IS the `Events` this
 * consumer wants, so the answer arrives whole and `GET …/events` is not called
 * at all. `once` is therefore reached only under the held arm — which is why it
 * asks the server to WAIT, since an immediate read there would be the poll
 * again under another name.
 *
 * `onEnd` is how a deleted game stops being watched: the game screen navigates
 * to the lobby with the sentence rather than reading "reconnecting…" forever.
 */
export function useSignal(
  game: string | null,
  active: boolean,
  onSignal: (e: Events) => void,
  onEnd?: (end: End) => void,
): Link {
  return usePolled<Events>(
    game
      ? {
          key: game,
          once: () => api.events(game, undefined, HELD_WAIT_S),
          stream: `/api/games/${encodeURIComponent(game)}/stream`,
          frames: "value",
        }
      : null,
    active,
    onSignal,
    onEnd,
  );
}
