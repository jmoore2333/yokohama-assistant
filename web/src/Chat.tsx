// The chat panel (M6.4 spec §6.1): ONE component, four hosts. It owns its
// list, its fetch and its post; a host tells it only WHERE it is (a table or
// the house) and WHEN to look — a `count` where the host's frame carries the
// server's number, a `tick` where the host's hook drops the frame.
//
// **Folded by default everywhere (R-6.4.10).** The panel is a bonus and play is
// the main attraction, so it sits LAST in the side column and opens only when
// somebody opens it; the fold is remembered per browser per scope. Folded, a
// count-driven host fetches nothing and the header's number is the count
// minus what is held; a wake-driven host still asks the delta on each tick,
// because the delta is the only source it has (spec §6.1 says so).
//
// **The label is the lobby's rule** (R-6.4.2): a seat's colour chip and its name
// or email; a watcher's name or email under a "watching" marker at a table
// (not in the house, where everyone is one); a bot's chip and its name — an
// arm no route produces yet, drawn defensively (R-6.4.15).

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { api, ApiError, chatText, displayName } from "./api";
import { seatClass } from "./Cards";
import { loadFolded, saveFolded } from "./chatFold";
import type { ChatMessage, ChatPage, ChatScope, PlayerColor } from "./types";

export interface ChatProps {
  scope: ChatScope;
  /** The server's newest message number, where the host holds it (the game
   *  and the dealing screen read it off every frame). */
  count?: number;
  /** "Something may have changed", where the host's hook runs in wake mode
   *  and drops the frame (the open table, the lobby). */
  tick?: number;
  /** The table's colours; absent at the house. */
  colors?: PlayerColor[];
  /** Send the browser's remembered name with a post: a watcher, and anyone at
   *  the house. A seated caller sends none (R-6.4.2). */
  sendName: boolean;
  /** The heading level: `h2` in the side column (the panels' own), `h3` in the
   *  lobby, whose three sibling sections are `h3` under the screen's one `h2`. */
  heading?: "h2" | "h3";
}

/** `HH:MM` today; the date before it otherwise (R-P3's night away). `now` and
 *  `locale` are parameters so a cell can freeze both.
 *
 *  **A stored clock is DATA and this render never throws on it** (final review,
 *  Important 1). `created_at` comes off a document, and a value out of `Date`'s
 *  range makes `toISOString()` throw `RangeError` — out of React's render,
 *  taking the whole game screen down for everyone at the table, since there is
 *  no error boundary. The server refuses such a value on read and on write now
 *  (`chat::MAX_CREATED_AT`), and this is the other end of the same rule: a
 *  number that is not a date is DRAWN AS THE NUMBER rather than allowed to
 *  crash the panel. Two guards and not one, because either half alone leaves
 *  the other end of the pipe able to take the screen out. */
export function when(at: number, now: number = Date.now(), locale?: string): string {
  const d = new Date(at * 1000);
  if (!Number.isFinite(d.getTime())) return String(at);
  const n = new Date(now);
  const hm = d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  return d.toDateString() === n.toDateString()
    ? hm
    : `${d.toLocaleDateString(locale, { month: "short", day: "numeric" })} ${hm}`;
}

/** The machine-readable stamp for `<time datetime>`, or `undefined` when the
 *  stored clock is not a date — see [`when`]. An absent attribute is what the
 *  HTML spec asks for when there is no parseable value; `toISOString()` on the
 *  same input throws. */
export function isoStamp(at: number): string | undefined {
  const d = new Date(at * 1000);
  return Number.isFinite(d.getTime()) ? d.toISOString() : undefined;
}

const scopeKey = (scope: ChatScope): string => (scope.kind === "house" ? "house" : scope.id);

/** Why a read was asked for — and, where one was asked for while another was
 *  still in flight, what to re-check when it lands (see `read`). */
type Why = "mount" | "count" | "tick";

export function Chat({ scope, count, tick, colors, sendName, heading = "h2" }: ChatProps) {
  const key = scopeKey(scope);
  const Heading = heading;
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  /** The highest number held — a ref, read by effects that must not re-run
   *  when it moves. */
  const last = useRef(0);
  const [folded, setFolded] = useState(() => loadFolded(key));
  const foldedNow = useRef(folded);
  foldedNow.current = folded;
  const countNow = useRef(count);
  countNow.current = count;
  const [unread, setUnread] = useState(0);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
  const box = useRef<HTMLOListElement | null>(null);
  /** A read is out; and what a trigger that arrived while it was out asked for. */
  const reading = useRef(false);
  const wanted = useRef<Exclude<Why, "mount"> | null>(null);
  /** The scope these reads are for. An answer from an older one is DROPPED —
   *  see `read`. */
  const generation = useRef(0);
  /** The reader was at the foot of the box when an answer landed, so the rows
   *  it appends should keep them there (see the layout effect below). */
  const follow = useRef(false);

  /** One answer, from wherever it came: a mount read, a delta, a 201. Filters
   *  what is already held, so a message is never drawn twice; and records
   *  whether the reader was at the foot of the box, which only a read taken
   *  BEFORE the append can tell.
   *
   *  **`counts` is false for the MOUNT read, and that is a rule about what
   *  "new" means** (final review, Important 3). Spec §6.1 defines the badge as
   *  what arrived WHILE FOLDED; the retained history did not arrive, it was
   *  already there. On a count-driven host the badge is `count - last` and
   *  history has always read 0 — so a wake host that counted its mount read
   *  made the same folded panel say "N new" on every visit, up to the whole
   *  retention, and the two kinds of host disagreed about the first screen. */
  const take = useCallback((page: ChatPage, counts = true) => {
    const fresh = page.messages.filter((m) => m.seq > last.current);
    last.current = Math.max(last.current, page.last, ...fresh.map((m) => m.seq));
    if (fresh.length === 0) return;
    const el = box.current;
    follow.current = !el || el.scrollHeight - el.scrollTop - el.clientHeight < 8;
    setMessages((old) => [...old, ...fresh]);
    if (counts && foldedNow.current) setUnread((n) => n + fresh.length);
  }, []);

  /**
   * Keep the newest message in view — **read before the append, applied after
   * the commit** (ruling R-6-1).
   *
   * The two halves cannot share an instant. "Was the reader at the bottom?" is
   * only true of the box BEFORE the rows arrive, and `scrollTop =
   * scrollHeight` is only the bottom AFTER they are in the DOM — so `take`
   * records the answer and this applies it. A `queueMicrotask` from inside
   * `take` reads the pre-append height and sets a value the box already has: a
   * default-lane update commits on the scheduler's own macrotask, which the
   * microtask beats.
   *
   * One `scrollTop` and nothing else, which is the turn account's rule
   * (R-F11): `scrollIntoView` scrolls every scrollable ancestor, and the page
   * is the player's to scroll. A reader who has scrolled up is left where they
   * are — the flag is simply false — and `useLayoutEffect` is what keeps the
   * corrected position from being painted after the wrong one.
   */
  useLayoutEffect(() => {
    if (!follow.current) return;
    follow.current = false;
    const b = box.current;
    if (b) b.scrollTop = b.scrollHeight;
  }, [messages]);

  /** Does the COUNT rule want a read at this instant? Refs, because it is asked
   *  again AFTER an await, when the effect's own closure is stale. */
  const countWants = useCallback(
    () => countNow.current !== undefined && countNow.current > last.current && !foldedNow.current,
    [],
  );

  /**
   * Ask for the delta above what is held — and **at most ONE request at a
   * time** (ruling R-pre-4).
   *
   * A mount is one fetch whatever the fold state and whatever `count` arrives
   * with: the mount read and the count rule fire in the SAME first commit of a
   * remembered-open panel, and both are true of a panel holding nothing. So a
   * trigger arriving mid-flight is not dropped and not sent — it is REMEMBERED,
   * and re-asked when the answer lands, which for the mount case it no longer
   * is. A tick's re-ask is unconditional (the delta is that host's only
   * source); a count's is the count rule read again against the cursor the
   * answer just moved.
   *
   * **A MOUNT read is never coalesced away, and an answer for a scope this
   * panel has left is DROPPED.** Those are one finding from two sides: the
   * mount effect runs when the scope CHANGES as well as at the start, so
   * deferring its read behind a read that is already out would leave the new
   * table's panel empty until something else happened — and letting the old
   * table's answer land would fill the list it had just cleared with another
   * room's messages, and move the cursor to another room's numbers. So the
   * generation is captured when a read starts and compared when it resolves;
   * a stale answer touches nothing, not the list, not the error line, not the
   * in-flight flag (the newer read owns it), and asks for nothing more.
   */
  const read = useCallback(
    async function read(why: Why): Promise<void> {
      const mine = generation.current;
      // **A post in flight PARKS a count trigger** (live review, M-2): the
      // 201 about to land is the answer to that very count, and asking for the
      // delta first spends a request to be told what the reply already holds.
      // Re-checked in `send`'s `finally`, where the cursor has moved.
      if (why === "count" && inFlight.current) {
        wanted.current = "count";
        return;
      }
      // A mount read falls through a read that is already out, and the two
      // answers are told apart by the generation rather than by arrival order.
      if (reading.current && why !== "mount") {
        wanted.current = why;
        return;
      }
      reading.current = true;
      let page: ChatPage | null = null;
      let refused: string | null = null;
      try {
        page = await api.chat(scopeRef.current, last.current);
      } catch (e) {
        refused = e instanceof ApiError ? e.message : String(e);
      }
      if (mine !== generation.current) return;
      reading.current = false;
      if (page) {
        take(page, why !== "mount");
        setError(null);
      } else setError(refused);
      const again = wanted.current;
      wanted.current = null;
      if (again === "tick" || (again === "count" && countWants())) await read(again);
    },
    [take, countWants],
  );

  // The mount read — everything retained — and a reset when the scope changes.
  // The generation moves FIRST, so a read still out for the scope just left has
  // already been disowned by the time its answer arrives.
  useEffect(() => {
    generation.current += 1;
    wanted.current = null;
    last.current = 0;
    setMessages([]);
    setUnread(0);
    setFolded(loadFolded(key));
    void read("mount");
  }, [key, read]);

  // A count above what is held: fetch, unless folded (the header shows the
  // difference instead). At or below: nothing — the cell that catches a panel
  // fetching on every move. And nothing while THIS panel's own post is in
  // flight, which `read` parks and `send` re-checks.
  //
  // **What spec §6.1 gets wrong, corrected here** (live review, M-2): it says
  // the frame an own post causes is "a no-op compare, because the 201's
  // message is appended locally and `last` advanced". That is true only if the
  // 201 lands FIRST. On the live tunnel the table's bump reaches the poster's
  // own stream inside their round trip — the overlap M6.3 measured — so the
  // count rule read a cursor the reply had not moved yet, and every post cost
  // one delta GET asking from one below the message it had just written. The
  // dated spec stands as written; this is the true sentence.
  useEffect(() => {
    if (count === undefined) return;
    if (count > last.current && !folded) void read("count");
  }, [count, folded, read]);

  // A tick: the delta, folded or not. The FIRST one is the mount's own frame,
  // which the mount read has already covered.
  const firstTick = useRef(true);
  useEffect(() => {
    if (tick === undefined) return;
    if (firstTick.current) {
      firstTick.current = false;
      return;
    }
    void read("tick");
  }, [tick, read]);

  const toggle = () => {
    const next = !folded;
    setFolded(next);
    saveFolded(key, next);
    if (!next) setUnread(0);
  };

  /** Post, and **drop the reply if the panel has moved rooms while it was in
   *  flight** (final review, Important 4).
   *
   *  The 201 was `read`'s one sibling resolution outside the generation guard,
   *  and it is the one that moves the cursor: `take` would draw the old room's
   *  message in the new room and set `last` to the old room's number — after
   *  which every new-room message numbered below it is filtered out, silently,
   *  until that room's numbering passes it. Narrow (the two unkeyed hosts,
   *  inside one round trip) and a real loss of messages when it happens.
   *
   *  The DRAFT is deliberately left alone on that path: the text did go to the
   *  other room, and clearing the box in this one would take the words away
   *  from someone who may want to say them here too. `sending` is cleared by
   *  the `finally`, which runs on the early return as well. */
  const send = async () => {
    const text = draft.trim();
    if (!text || inFlight.current) return;
    const mine = generation.current;
    inFlight.current = true;
    setSending(true);
    setError(null);
    try {
      const name = sendName ? displayName()?.trim() : undefined;
      const m = await api.postChat(scopeRef.current, { text, ...(name ? { name } : {}) });
      if (mine !== generation.current) return;
      take({ messages: [m], last: m.seq });
      setDraft("");
    } catch (e) {
      if (mine !== generation.current) return;
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setSending(false);
      // The parked count, re-asked against the cursor the 201 has just moved —
      // so an own post asks for nothing and a message that arrived from
      // somebody else while this one was out is still fetched. **Only when no
      // read is out**: a count parked behind a READ is that read's tail's to
      // consume, and clearing it here would drop the re-ask.
      if (wanted.current === "count" && !reading.current) {
        wanted.current = null;
        if (countWants()) void read("count");
      }
    }
  };

  const badge = count !== undefined ? Math.max(0, count - last.current) : unread;
  const title = scope.kind === "house" ? "Lobby chat" : "Chat";
  const label = (m: ChatMessage) => {
    const a = m.author;
    if (a.kind === "seat")
      return <span className={`chat-who ${seatClass(colors?.[a.seat])}`}>{a.name ?? a.user}</span>;
    if (a.kind === "bot")
      return <span className={`chat-who ${seatClass(colors?.[a.seat])}`}>{a.name ?? "bot"}</span>;
    return (
      <span className={`chat-who${scope.kind === "table" ? " watching" : ""}`}>
        {a.name ?? a.user}
        {scope.kind === "table" && <span className="meta"> · watching</span>}
      </span>
    );
  };

  return (
    <section className={`panel chat${folded ? " folded" : ""}`} data-scope={key}>
      <Heading>
        {title}
        {folded && badge > 0 && (
          <span className="chat-new" aria-label={`${badge} new`}>
            {badge}
          </span>
        )}
        <button
          type="button"
          className="ghost fold"
          aria-expanded={!folded}
          aria-controls={`chat-${key}`}
          onClick={toggle}
        >
          {folded ? "show" : "hide"}
        </button>
      </Heading>
      {/* The live region is the WRAPPER's and not the list's: `role="log"` on
          an `<ol>` replaces the implicit `list` role, so the messages stopped
          being a list to anything reading the page by structure (final review,
          minor 18). Announcing is what the region is for; being a list is what
          the element is for. */}
      <div
        id={`chat-${key}`}
        className={`messages${folded ? " folded" : ""}`}
        role="log"
        aria-live="polite"
      >
        <ol className="chat-lines" ref={box}>
          {messages.map((m) => (
            <li key={m.seq}>
              {label(m)}
              <time dateTime={isoStamp(m.created_at)}>{when(m.created_at)}</time>
              <span className="chat-text">{m.text}</span>
            </li>
          ))}
        </ol>
        {error && <div className="error">{error}</div>}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <input
            aria-label="Say something"
            value={draft}
            maxLength={chatText()}
            disabled={sending}
            tabIndex={folded ? -1 : 0}
            onChange={(e) => setDraft(e.target.value)}
          />
          <button type="submit" className="ghost" disabled={sending || !draft.trim()} tabIndex={folded ? -1 : 0}>
            send
          </button>
        </form>
      </div>
    </section>
  );
}
