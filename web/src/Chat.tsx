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

import { useCallback, useEffect, useRef, useState } from "react";
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
 *  `locale` are parameters so a cell can freeze both. */
export function when(at: number, now: number = Date.now(), locale?: string): string {
  const d = new Date(at * 1000);
  const n = new Date(now);
  const hm = d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  return d.toDateString() === n.toDateString()
    ? hm
    : `${d.toLocaleDateString(locale, { month: "short", day: "numeric" })} ${hm}`;
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

  /** One answer, from wherever it came: a mount read, a delta, a 201. Filters
   *  what is already held, so a message is never drawn twice; keeps the box
   *  at its bottom unless the reader has scrolled up. */
  const take = useCallback((page: ChatPage) => {
    const fresh = page.messages.filter((m) => m.seq > last.current);
    last.current = Math.max(last.current, page.last, ...fresh.map((m) => m.seq));
    if (fresh.length === 0) return;
    const el = box.current;
    const atBottom = !el || el.scrollHeight - el.scrollTop - el.clientHeight < 8;
    setMessages((old) => [...old, ...fresh]);
    if (foldedNow.current) setUnread((n) => n + fresh.length);
    if (atBottom)
      queueMicrotask(() => {
        const b = box.current;
        if (b) b.scrollTop = b.scrollHeight;
      });
  }, []);

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
   */
  const read = useCallback(
    async function read(why: Why): Promise<void> {
      if (reading.current) {
        if (why !== "mount") wanted.current = why;
        return;
      }
      reading.current = true;
      try {
        take(await api.chat(scopeRef.current, last.current));
        setError(null);
      } catch (e) {
        setError(e instanceof ApiError ? e.message : String(e));
      } finally {
        reading.current = false;
      }
      const again = wanted.current;
      wanted.current = null;
      if (again === "tick" || (again === "count" && countWants())) await read(again);
    },
    [take, countWants],
  );

  // The mount read — everything retained — and a reset when the scope changes.
  useEffect(() => {
    last.current = 0;
    setMessages([]);
    setUnread(0);
    setFolded(loadFolded(key));
    void read("mount");
  }, [key, read]);

  // A count above what is held: fetch, unless folded (the header shows the
  // difference instead). At or below: nothing — the cell that catches a panel
  // fetching on every move.
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

  const send = async () => {
    const text = draft.trim();
    if (!text || inFlight.current) return;
    inFlight.current = true;
    setSending(true);
    setError(null);
    try {
      const name = sendName ? displayName()?.trim() : undefined;
      const m = await api.postChat(scopeRef.current, { text, ...(name ? { name } : {}) });
      take({ messages: [m], last: m.seq });
      setDraft("");
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setSending(false);
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
      <div id={`chat-${key}`} className={`messages${folded ? " folded" : ""}`}>
        <ol className="chat-lines" ref={box} role="log" aria-live="polite">
          {messages.map((m) => (
            <li key={m.seq}>
              {label(m)}
              <time dateTime={new Date(m.created_at * 1000).toISOString()}>{when(m.created_at)}</time>
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
