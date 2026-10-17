// The chat panel (M6.4 spec §6.1): ONE component, four hosts. It owns its
// list, its fetch and its post; a host tells it only WHERE it is (a table or
// the house) and WHEN to look — a `count`, the server's newest message number
// off the host's own frame. **Every host is a count host since M6.5c2 cycle D**
// (row 130): the lobby and the open table dropped their frames and passed a
// `tick`, so the panel asked for a delta on every wake — every move of every
// game, at the house — and one for its own post; both now read the counters
// off the frame (`Source.peek`), and the tick is gone.
//
// **Folded by default everywhere (R-6.4.10).** The panel is a bonus and play is
// the main attraction, so it sits LAST in the side column and opens only when
// somebody opens it; the fold is remembered per browser per scope. Folded, the
// panel fetches nothing and the header's number is the count minus what is
// held.
//
// **The label is the lobby's rule** (R-6.4.2): a seat's colour chip and its name
// or email; a watcher's name or email under a "spectating" marker at a table
// (not in the house, where everyone is one); a bot's chip and its name — an
// arm no route produces yet, drawn defensively (R-6.4.15).
//
// **A message can be deleted** (M6.5c2 cycle D, R-D4): by its author, and at a
// table by the table's creator (`canModerate`, the host's `table.mine`). The
// server keeps it as a TOMBSTONE — its number, clock and author, `text: ""`,
// `deleted: true` — drawn here as "message removed" in its author's row. The
// Delete reply changes nothing on screen by itself: every page carries the
// document's `rev` beside its `last`, and **`rev - last` is constant under
// appends and grows by one per delete** — so when a page (or, on a count host,
// the frame's `chat_rev` against its `chat`) shows it moved, the panel re-reads
// everything it holds from 0 and replaces its list. Every reader who held the
// message, the one who deleted it included, learns it the same way.

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { api, ApiError, chatText, displayName } from "./api";
import { seatClass } from "./Cards";
import { loadFolded, saveFolded } from "./chatFold";
import type { ChatMessage, ChatPage, ChatScope, PlayerColor } from "./types";
import { Refusal } from "./Refusal";

export interface ChatProps {
  scope: ChatScope;
  /** The server's newest message number, where the host holds it (the game
   *  and the dealing screen read it off every frame). */
  count?: number;
  /** The table's chat REVISION off the same frame (`Events.chat_rev`), where
   *  the host holds `count`: moved further than `count`, a message was
   *  deleted, and the panel re-reads what it holds (M6.5c2 cycle D, R-D4). */
  rev?: number;
  /** The reader created this table (`TableView.mine`), so every message has a
   *  Delete control, not only their own. Never at the house, which nobody
   *  created. */
  canModerate?: boolean;
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

/** The most reads one call of `read` makes — its own, and the re-asks that
 *  triggers arriving mid-read owe (see `read`). */
export const CHAT_READ_BOUND = 8;

/** Why a read was asked for — and, where one was asked for while another was
 *  still in flight, what to re-check when it lands (see `read`). `rev` is the
 *  re-read of everything held, after a delete (M6.5c2 cycle D). */
type Why = "mount" | "count" | "rev";

/** Two triggers parked behind one read, as one: a re-read of everything held
 *  answers a delta too, so it wins. */
const merged = (a: Exclude<Why, "mount"> | null, b: Exclude<Why, "mount">): Exclude<Why, "mount"> =>
  a === "rev" || b === "rev" ? "rev" : b;

export function Chat({ scope, count, rev, canModerate = false, colors, sendName, heading = "h2" }: ChatProps) {
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
  const revNow = useRef(rev);
  revNow.current = rev;
  /** The held `rev - last`, from the last page taken, or `null` before the
   *  first one — see the file's note on the delete. */
  const shape = useRef<number | null>(null);
  /** The messages whose delete is out, or answered and waiting for the
   *  frame that shows the tombstone: their controls stay disabled. */
  const [removing, setRemoving] = useState<ReadonlySet<number>>(() => new Set());
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
   *  BEFORE the append can tell. **It counts nothing**: the folded header's
   *  number is `count - last` (the badge below), so history read at mount has
   *  always read 0 new — the rule a wake host's own tally once broke (final
   *  review, Important 3), gone with the wake hosts. */
  const take = useCallback((page: Pick<ChatPage, "messages" | "last">) => {
    const fresh = page.messages.filter((m) => m.seq > last.current);
    last.current = Math.max(last.current, page.last, ...fresh.map((m) => m.seq));
    if (fresh.length === 0) return;
    const el = box.current;
    follow.current = !el || el.scrollHeight - el.scrollTop - el.clientHeight < 8;
    setMessages((old) => [...old, ...fresh]);
  }, []);

  /** A re-read of everything held (M6.5c2 cycle D, R-D4): the page REPLACES
   *  the list, so a message deleted since it was drawn is drawn as its
   *  tombstone.
   *
   *  **A held message numbered above the page's `last` is KEPT**: it landed
   *  after the server built this page — the reader's own post, whose 201
   *  came back while the re-read was out — and a list replaced without it
   *  would lose it for good, since the count its frame brings is one the
   *  cursor has already passed. */
  const replace = useCallback((page: ChatPage) => {
    const fresh = page.messages.filter((m) => m.seq > last.current);
    last.current = Math.max(last.current, page.last, ...fresh.map((m) => m.seq));
    const el = box.current;
    follow.current = fresh.length > 0 && (!el || el.scrollHeight - el.scrollTop - el.clientHeight < 8);
    setMessages((old) => [...page.messages, ...old.filter((m) => m.seq > page.last)]);
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

  /** Does the REVISION rule want a re-read — a count host's frame whose
   *  `chat_rev - chat` is not the `rev - last` the panel holds? Only once a
   *  page has been taken, only from a frame at least as new as what is held
   *  (a host's reset count of 0 is not news), and not while folded, as the
   *  count rule waits. */
  const revWants = useCallback(() => {
    const c = countNow.current;
    const r = revNow.current;
    return (
      c !== undefined && r !== undefined && shape.current !== null && c >= last.current && r - c !== shape.current && !foldedNow.current
    );
  }, []);

  /**
   * Ask for the delta above what is held — and **at most ONE request at a
   * time** (ruling R-pre-4).
   *
   * A mount is one fetch whatever the fold state and whatever `count` arrives
   * with: the mount read and the count rule fire in the SAME first commit of a
   * remembered-open panel, and both are true of a panel holding nothing. So a
   * trigger arriving mid-flight is not dropped and not sent — it is REMEMBERED,
   * and re-asked when the answer lands, which for the mount case it no longer
   * is. A count's re-ask is the count rule read again against the cursor the
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
    async (first: Why): Promise<void> => {
      // **A LOOP, with a bound** (M6.5c2 cycle D; row 130, M6.4's Task 6
      // LEAVE). The re-ask a trigger arriving mid-read owes was `await
      // read(again)` from inside this function — one nested call per read for
      // as long as a room kept a trigger landing during every read. Each turn
      // here is one read; after `CHAT_READ_BOUND` of them the call stops and
      // leaves what is still owed PARKED in `wanted`, which the next trigger's
      // read takes up when it lands (and `send` re-checks a parked count). A
      // room busier than that sends its next frame soon enough.
      let why: Why = first;
      for (let reads = 1; ; reads += 1) {
        const mine = generation.current;
        // **A post in flight PARKS a count trigger** (live review, M-2): the
        // 201 about to land is the answer to that very count, and asking for
        // the delta first spends a request to be told what the reply already
        // holds. Re-checked in `send`'s `finally`, where the cursor has moved.
        if (why === "count" && inFlight.current) {
          wanted.current = merged(wanted.current, "count");
          return;
        }
        // A mount read falls through a read that is already out, and the two
        // answers are told apart by the generation rather than by arrival order.
        if (reading.current && why !== "mount") {
          wanted.current = merged(wanted.current, why);
          return;
        }
        reading.current = true;
        let page: ChatPage | null = null;
        let refused: string | null = null;
        try {
          // A re-read asks from the start; everything else from the cursor
          // (which a mount's is, at 0).
          page = await api.chat(scopeRef.current, why === "rev" ? 0 : last.current);
        } catch (e) {
          refused = e instanceof ApiError ? e.message : String(e);
        }
        if (mine !== generation.current) return;
        reading.current = false;
        // **The revision rule** (M6.5c2 cycle D): a delta whose `rev - last` is
        // not the held one says a message the panel holds was deleted, and a
        // re-read from 0 is owed. A mount or a re-read IS that read.
        let moved = false;
        if (page) {
          if (why === "rev") replace(page);
          else take(page);
          const held = page.rev - page.last;
          moved = why !== "mount" && why !== "rev" && shape.current !== null && held !== shape.current;
          shape.current = held;
          setError(null);
        } else setError(refused);
        const again = moved ? merged(wanted.current, "rev") : wanted.current;
        wanted.current = null;
        // The frame's half is asked again only after a read that was not itself
        // the re-read: the server writes before it signals, so a re-read is
        // never behind the frame that asked for it, and one that somehow were
        // would otherwise ask again at once, for as long as it stayed behind.
        let next: Exclude<Why, "mount"> | null = null;
        if (again === "rev" || (why !== "rev" && revWants())) next = "rev";
        else if (again === "count" && countWants()) next = again;
        if (next === null) return;
        if (reads >= CHAT_READ_BOUND) {
          wanted.current = merged(wanted.current, next);
          return;
        }
        why = next;
      }
    },
    [take, replace, countWants, revWants],
  );

  // The mount read — everything retained — and a reset when the scope changes.
  // The generation moves FIRST, so a read still out for the scope just left has
  // already been disowned by the time its answer arrives.
  useEffect(() => {
    generation.current += 1;
    wanted.current = null;
    last.current = 0;
    shape.current = null;
    setMessages([]);
    setRemoving(new Set());
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

  // The revision rule's frame half (M6.5c2 cycle D): on a count host, a
  // `rev` that has moved further than `count` since the page the panel holds
  // is a delete, and the panel re-reads what it holds — unless folded, when it
  // waits for the unfold exactly as the count rule does.
  useEffect(() => {
    if (revWants()) void read("rev");
  }, [rev, count, folded, read, revWants]);

  const toggle = () => {
    const next = !folded;
    setFolded(next);
    saveFolded(key, next);
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
    /** The 201's number showed messages this panel never read (below). */
    let gap = false;
    try {
      const name = sendName ? displayName()?.trim() : undefined;
      const m = await api.postChat(scopeRef.current, { text, ...(name ? { name } : {}) });
      if (mine !== generation.current) return;
      // **A GAP CHECK on the 201's number** (cycle D's close `2026-09-29-web-frontend-m6.5c2d-close.md` §5, ruling 35: the
      // lost-message finding). Numbers are contiguous — a trim drops the
      // oldest end and never reuses one, and a delete keeps its tombstone — so
      // a post numbered past `last + 1` says the messages between were never
      // read: their frame was lost (a socket's reconnect probe reads the
      // lobby's listings, which carry no chat count; neither a stream
      // reconnect nor a tab's return brings a house frame). Taking the 201
      // would move the cursor past them for good — the post's own frame then
      // counts nothing new — so the cursor stays where it is and the page is
      // READ instead, from 0 (the revision rule's re-read), which draws the
      // missed messages and this post in order. **The cursor never advances
      // past an unread message**: a re-read that fails leaves it here, and
      // the next count asks again from it.
      if (m.seq > last.current + 1) gap = true;
      // A post moves `rev` and `last` together, so the held shape stands.
      else take({ messages: [m], last: m.seq });
      setDraft("");
    } catch (e) {
      if (mine !== generation.current) return;
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setSending(false);
      if (gap) {
        // Behind a read already out it is parked as `rev`, which outranks
        // whatever else is owed, and that read's tail asks it.
        void read("rev");
      } else if (wanted.current === "count" && !reading.current) {
        // The parked count, re-asked against the cursor the 201 has just
        // moved — so an own post asks for nothing and a message that arrived
        // from somebody else while this one was out is still fetched. **Only
        // when no read is out**: a count parked behind a READ is that read's
        // tail's to consume, and clearing it here would drop the re-ask.
        wanted.current = null;
        if (countWants()) void read("count");
      }
    }
  };

  /** Take message `seq` down (M6.5c2 cycle D, R-D4). **The reply changes
   *  nothing on screen**: the server bumps the table (or the house) after its
   *  write, and the frame's revision — or the next page's — is what re-reads
   *  the list and draws the tombstone. The control stays disabled meanwhile;
   *  a refusal puts the server's sentence up and the control back. */
  const remove = async (seq: number) => {
    const mine = generation.current;
    setRemoving((old) => new Set(old).add(seq));
    setError(null);
    try {
      const at = scopeRef.current;
      if (at.kind === "house") await api.deleteHouseChat(seq);
      else await api.deleteChat(at.id, seq);
    } catch (e) {
      if (mine !== generation.current) return;
      setError(e instanceof ApiError ? e.message : String(e));
      setRemoving((old) => {
        const next = new Set(old);
        next.delete(seq);
        return next;
      });
    }
  };

  /** The server's rule, drawn: the reader's own message, or any at a table the
   *  reader created — never a tombstone, and never a bot's for anyone else. */
  const mayDelete = (m: ChatMessage) => !m.deleted && (canModerate || (m.author.kind !== "bot" && m.author.you));

  const badge = count !== undefined ? Math.max(0, count - last.current) : 0;
  const title = scope.kind === "house" ? "Lobby chat" : "Chat";
  const label = (m: ChatMessage) => {
    const a = m.author;
    // A name, else "you" for the reader's own, else the seat or "someone":
    // no author carries an address (M6.5c2 cycle D, R-6.5c2.35).
    if (a.kind === "seat")
      return (
        <span className={`chat-who ${seatClass(colors?.[a.seat])}`}>{a.name ?? (a.you ? "you" : `seat ${a.seat}`)}</span>
      );
    if (a.kind === "bot")
      return <span className={`chat-who ${seatClass(colors?.[a.seat])}`}>{a.name ?? "bot"}</span>;
    return (
      <span className={`chat-who${scope.kind === "table" ? " watching" : ""}`}>
        {a.name ?? (a.you ? "you" : "someone")}
        {/* **The tag** (security S2c, D8): the server's four hex for this poster in this room, drawn where a
            name's provenance would be — "you · a1b2" tells the reader their own tag. `null` before the room's
            first message since the tag landed (an old document), or absent from a server rolled back past it:
            nothing drawn, nothing claimed. */}
        {typeof a.tag === "string" && <span className="meta"> &middot; {a.tag}</span>}
        {scope.kind === "table" && <span className="meta"> &middot; spectating</span>}
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
            <li key={m.seq} className={m.deleted ? "removed" : undefined}>
              {label(m)}
              <time dateTime={isoStamp(m.created_at)}>{when(m.created_at)}</time>
              {m.deleted ? (
                <span className="chat-text removed">message removed</span>
              ) : (
                <span className="chat-text">{m.text}</span>
              )}
              {mayDelete(m) && (
                <button
                  type="button"
                  className="ghost chat-delete"
                  aria-label="Delete this message"
                  disabled={removing.has(m.seq)}
                  tabIndex={folded ? -1 : 0}
                  onClick={() => void remove(m.seq)}
                >
                  delete
                </button>
              )}
            </li>
          ))}
        </ol>
        {error && <Refusal sentence={error} />}
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
