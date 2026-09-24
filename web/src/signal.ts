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
// the ladder after a failure, and the held-request arm's re-ask. **The socket
// arm (M6.5c) is one connection per resource too, and holds ONE timer while it
// is connected — a keep-alive that asks nothing**: a `ping` every 30 s
// (`ws.ts`), answered without waking the server's object, which keeps the edge
// from cutting a quiet connection.
//
// **The stream's reader is a `fetch`, not an `EventSource`** (spec §0.1) —
// `sse.ts` says why in one line: `EventSource` cannot carry a header, and the
// `claimed` door is one. It is also the reason a hidden tab can let go at all,
// since an `AbortController` comes with the `fetch` and not with the other.
//
// **On the Worker the reader is a `WebSocket`** (M6.5c, R-6.5c.6; spec §2.7):
// the same frames in a JSON envelope, handed to the same handler, and let go
// the same way — the abort closes it. A socket carries no header either, and a
// browser hides why an upgrade was refused, so a close nobody explained is
// followed by ONE plain fetch that can say (`open` below).
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
import { api, ApiError, isLocal, isSessionExpired, signalKind, transportWait } from "./api";
import { readStream, type Frame } from "./sse";
import { readSocket } from "./ws";
import type { Events } from "./types";

/** The wait after the 1st, 2nd, … consecutive failure, and then forever.
 *
 *  **It ends at a ceiling rather than growing without bound**, because the
 *  thing on the other end is a home server behind Cloudflare Access: a client
 *  that backed off to ten minutes would leave a player staring at a stale board
 *  long after the server came back, and one that never backed off would hammer
 *  it hardest exactly while it was least able to answer. */
export const BACKOFF_MS = [2000, 4000, 8000, 16000, 30000] as const;

/** How long the HELD arm waits before asking again after an answer — for a
 *  `"wake"` source ONLY.
 *
 *  **It is not one value for both arms, and the correction is the final
 *  review's I-3.** R-pre-5 set a single 3 s because reopening at zero is a hot
 *  loop against three table listings; that is true of the lobby and the open
 *  table, whose `once` answers immediately, and false of the game's and the
 *  dealing screen's, which is `GET …/events?wait=25` and HOLDS on the server.
 *  Worse than merely wasteful: `?wait` subscribes and waits for a NEW bump and
 *  nothing compares the caller's count (R-3.14), so a bump landing inside the
 *  3 s gap was not seen until the NEXT request's 25 s deadline — a 27–28 s
 *  latency where the poll this replaces had 3 s, under a hook whose whole claim
 *  is to be faster. A source whose `once` holds is therefore re-asked at 0 and
 *  only a `"wake"` source waits this long.
 *
 *  **The residual, which re-asking at zero does not close.** A bump that lands
 *  inside one request's own round trip — after the server answered and before
 *  the next subscribe — still waits for the bump after it, or for the 25 s
 *  deadline. Closing that needs the server to answer at once when `after`
 *  differs from the current count, which is a route change and not a reader
 *  one. Recorded here because R-plan-5's "the fallback is a one-line change in
 *  the client's reader" is the sentence this limit belongs to; the arm is
 *  dormant while `signalKind()` is `"sse"`. */
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
 *  The socket arm (M6.5c) opens `stream` as a socket, and asks `once` as its one
 *  probe after a close nobody explained. A `"wake"` source reads `once` under
 *  EVERY arm — see `frames`. */
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
  /** `"local"` for a game the module in this browser's own Worker answers
   *  (M6.5b, R-6.5b.12): there is NOTHING to wait for, because the bot's
   *  trailing run returns inside the reply that caused it and nobody else sits
   *  at the table.
   *
   *  **On the SOURCE and not a third global** (spec §2.3, amending umbrella
   *  §2.5): `open()` holds a `Source` and knows no id and no host, so the
   *  source is the only place the host can be said — the shape M6.3's
   *  `frames?` already used. The lobby's and the open table's sources never
   *  set it, so the server's streams stay open while a local game is on
   *  screen. */
  host?: "local";
}

/** What the LOBBY is told about a signal that ended on a FRAME.
 *
 *  **The frame carries a reason and no words** — `{"reason":"deleted"}` is the
 *  whole of what `Signals::end` writes — and each consumer hands `end.sentence`
 *  to its own leave. Only the game screen had a fallback, so a real abandon
 *  took the dealing screen and the open table to the lobby in silence, and so
 *  did an open table filling its last seat without the stranger watching it
 *  (a 403 read as `gone`). The two refusals that END a signal already carry the
 *  server's own message and are not routed through here; these two are the
 *  frames, and the words are the client's because the frame has none. */
export const endSentence = (reason: "deleted" | "gone"): string =>
  reason === "deleted" ? "this table was abandoned" : "this table is no longer available";

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
 * there — `readStream` does not go through `call` — and does not need to be:
 * the next ordinary call takes the player to the door. The socket arm's probe
 * DOES go through `call`, so a 401 on it reaches the door the ordinary way.
 *
 * **The EDGE's 401 is the exception, and it is not a second door handler**
 * (final review I-2): an expired Access session is not this API saying it does
 * not know you, it is the turnstile in front of it, and nothing this hook can
 * do reopens it. So it ENDS the reader with the sentence instead of climbing
 * the ladder — the consumer draws it through `Refusal`, which is what puts
 * `/enter` beside it.
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
    // **A LOCAL game has no signal at all** (M6.5b, R-6.5b.12; spec §2.3) —
    // no stream, no held request and NO TIMER. The word stays "live", which
    // is true: the module answering this game is in this tab.
    //
    // **Here and not inside `open()`**, which is where a first draft put it:
    // the effect arms a zero-delay timer to make the first open, so a guard
    // one call later would still have scheduled one. The cell that says so
    // reads `vi.getTimerCount()` across sixty fake seconds.
    //
    // Read through the ref like every other field of the source, and safe to
    // read once: `host` is derived from the game id, which IS `key`, so a
    // source cannot change host without re-running this effect.
    if (src.current?.host === "local") return;
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
        if (e instanceof ApiError && (e.status === 404 || e.status === 403 || isSessionExpired(e))) {
          // The resource this stream is about is gone, is not ours, or the
          // EDGE's session in front of it has expired. The reader goes with it:
          // a connection kept open to a table we may not read would deliver
          // frames nothing can act on, and an expired session is fixed by
          // walking through `/enter` rather than by asking again.
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
      const frames = src.current.frames ?? "wake";
      // **One frame, off EITHER transport** (M6.5c): the stream's reader and
      // the socket's hand this the same `Frame`, so what a frame MEANS is
      // decided here once.
      const onFrame = (f: Frame) => {
        if (stopped) return;
        if (f.event === "end") {
          // The server's own reason rides the frame: `deleted` for a table
          // that was abandoned, `gone` for one that stopped being readable
          // under a live connection.
          // **And it is given WORDS here, once, for all four consumers**
          // (final review I-2). A server-supplied sentence is kept if one
          // ever arrives; today none does, and defaulting in the consumers
          // instead would be three copies of the same two strings with one
          // of them already missing.
          const { reason, sentence } = JSON.parse(f.data) as {
            reason: "deleted" | "gone";
            sentence?: string;
          };
          ended.current?.({ reason, sentence: sentence ?? endSentence(reason) });
          return;
        }
        if (f.event !== "signal" && f.event !== "house") return;
        if (frames === "value") latest.current(JSON.parse(f.data) as T);
        else void reread();
      };
      // **The CONNECTION, and it is what both of these hang on.**
      //
      // The word first: a `fetch` that has been issued is not a link, and
      // this hook once said "live" when the request went out. During an
      // outage that made the word blink off at every rung of the ladder, and
      // against a server that accepts a connection and then hangs it read
      // "live" for ever while nothing was connected. Spec §5.1 ties the word
      // to Connected, and this is that moment.
      //
      // The failure count for the same reason, and the house channel is
      // why it cannot be reset on a frame instead: `/api/stream` writes
      // NOTHING on connect (R-3.4), so a lobby reconnecting to a quiet
      // server would climb 2 s, 4 s, 8 s to the ceiling while every one of
      // those connections was made perfectly well. A 200 with a stream body
      // IS the connection — and a server that accepts and closes at once is
      // still bounded, by the first rung. Over the socket the same moment is
      // its `open` event (`ws.ts`).
      const onConnected = () => {
        if (stopped) return;
        failures.current = 0;
        setLink("live");
      };
      try {
        // **Nothing opens before the transport is KNOWN** (M6.5c Task 15, the
        // live tier's finding 1; ruling I-81): the facts' default is the
        // stream, and the Worker answers a stream's GET with 426.
        // `transportWait` says when this waits — only while this page has no
        // health answer — and a read that FAILED rejects here, into the
        // ladder below, rather than letting the default be guessed. A caller
        // that has already failed once asks the read again itself.
        const pending = transportWait(failures.current > 0);
        if (pending !== null) {
          await pending;
          if (stopped || mine.signal.aborted) return;
        }
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
          // **At ONCE for a source whose `once` holds** — see
          // [`HELD_REASK_MS`], which is the `"wake"` sources' wait and the
          // reason the two cannot share a number.
          schedule(frames === "value" ? 0 : HELD_REASK_MS);
          return;
        }
        if (signalKind() === "ws") {
          // **The socket arm** (M6.5c, spec §2.7): the same frames to the same
          // handler, and "live" at the same moment — the socket's `open`.
          const outcome = await readSocket(src.current.stream, mine.signal, onFrame, onConnected);
          if (stopped || outcome === "ended") return;
          // A close this arm made ITSELF — a hidden tab (R-P8), a teardown —
          // asks nothing (Review Focus 2): exactly as the stream reader returns
          // silently on its own abort.
          if (mine.signal.aborted) return;
          // **Nothing is connected now, so the word stops saying "live"**
          // (spec §5.1) — at the close, not once the probe has answered: a
          // probe that hangs would otherwise leave "live" on screen over no
          // connection at all.
          setLink("reconnecting");
          // THE PROBE: one plain fetch through `call`, so an expired Access
          // session is classified exactly as the stream arm classifies it (the
          // catch below ends the source with the `/enter` remedy); an answer is
          // a free resync, delivered.
          const v = await src.current.once();
          if (stopped) return;
          latest.current(v);
          throw new Error("the socket closed"); // then the existing ladder
        }
        const outcome = await readStream(src.current.stream, mine.signal, onFrame, onConnected);
        if (stopped || outcome === "ended") return;
        // The body ended with nothing said: a dropped connection, and the
        // ladder is what that is for.
        throw new Error("the stream closed");
      } catch (e) {
        if (stopped || mine.signal.aborted) return;
        // **The THREE refusals that END this rather than slowing it down.** See
        // `End`; nothing is scheduled after any of these lines.
        //
        // **The third is the EDGE's, and it was the review's Important 2**
        // (final review, false sentence 1): `readStream` has classified
        // Access's 401 since this cycle, but every status that was not 404 or
        // 403 fell through to the ladder below — so a player holding a stream
        // when their Access session expired read "reconnecting…" about a door
        // they could have walked through. The consumers' `onEnd` puts the
        // sentence on a screen that draws it through `Refusal`, which is where
        // the link to `/enter` is.
        if (e instanceof ApiError && (e.status === 404 || e.status === 403 || isSessionExpired(e))) {
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
        // test, which is what tells "I was aborted" from "I failed". On the
        // socket arm it is a no-op on every path: `readSocket` resolves only
        // once its socket has closed, and hangs the socket up itself when a
        // handler throws.
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
 * consumer wants, so the answer arrives whole and no frame costs a
 * `GET …/events`. `once` is therefore reached only under the held arm and by the
 * socket arm's one probe — and it asks the server to WAIT for the held arm's
 * sake, since an immediate read there would be the poll again under another
 * name. The Worker, the one host that serves the socket, answers `wait` at once
 * (spec §2.3), which is what a probe needs.
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
          // **The game screen is the one consumer that can be local**
          // (spec §2.3): the lobby and the open table are the server's
          // whatever is on screen beside them.
          host: isLocal(game) ? "local" : undefined,
        }
      : null,
    active,
    onSignal,
    onEnd,
  );
}
