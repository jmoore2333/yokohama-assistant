// The open table (spec §5.4): the screen a join link lands on, and the screen
// the person who opened the table waits on.
//
// **Two faces of one table, and `my_seats` picks between them.** A caller with
// no seat here is a stranger — the one route on this API widened to somebody
// who is not seated (R-2.7), because a join link is an address and the person
// following one has to see what they are joining — and what they are offered is
// a seat: ONE "Join as <colour>" when one seat is free, a "Sit here" in each
// open seat's card when more are (R-6.5c2.9), and at a locked table the
// password box above the seats. The name the seat takes is the one this browser
// keeps (R-6.5c2.8), read at the click; the screen asks for none. A caller who
// already holds a seat is WAITING, and what they are offered is the link to
// send to whoever is still missing, plus — if they opened the table — the way
// to abandon it and a Reopen beside each seat somebody else took (security S3,
// D9).
//
// **Arriving seats nobody.** One click, never zero: a link preview or a curious
// click must not take a seat, and a locked table needs its password typed
// anyway. Nothing on this screen posts a join but a button.
//
// **The table is handed in, and fetched here only when it is not** (M6.5c2
// cycle D, dossier row 17). `App`'s `open` has just read `GET …/table` to know
// this is an open table at all, so it passes that document down and a visit
// costs one read rather than two; without one — a cell, or a host that has
// none — the opening read is what keeps the screen from saying "loading…"
// until somebody else acts, the lobby's own shape. The watch is `usePolled` in
// its default `"wake"` mode (M6.3 R-7.1): a frame off `…/{id}/stream` is an
// `Events` and this screen's value is a `TableView`, so the frame is the
// wake-up and `GET …/table` is the answer. The stream's ON-CONNECT frame is
// therefore a read of the table at connect, which is one request and buys the
// reconnect its compare. The frame's own chat counters go to the chat panel
// (`peek`), which is a count host here: **the connect-time resync frame reads
// the chat only if `chat` or `chat_rev` has moved** past what the panel holds
// (M6.5c2 cycle D, row 130).
//
// **Leaving this screen is a re-OPEN, not a jump to the dealing screen.** The
// three ways a table stops being open — the signal, the join's own reply, and a
// 409 that carries a status — all say only "not open any more", and which of
// the two remaining screens that is depends on whether the last seat has kept.
// So the hand-over asks `App` to open the table again by its status rather than
// naming a screen it cannot know (spec §5.4).

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Abandon } from "./Abandon";
import { LobbyBand } from "./Band";
import { Chat } from "./Chat";
import { PasswordTag } from "./Chips";
import { api, ApiError, displayName } from "./api";
import { hashOf } from "./route";
import { SeatCard } from "./SeatCard";
import { mayAbandon, seatName } from "./tables";
import { usePolled, type End } from "./signal";
import type { Events, PlayerColor, SeatView, TableView } from "./types";
import { Refusal } from "./Refusal";

export function OpenTable({
  game,
  table: handed,
  onStarted,
  onLeave,
  who,
}: {
  /** The table, by id. */
  game: string;
  /** The table's document, where the host has just read it (`App`'s `open`):
   *  drawn at once, and not read again at mount. Absent, the screen reads it
   *  itself. */
  table?: TableView;
  /** The table is no longer open. `App` re-opens it and its STATUS picks the
   *  screen: a table still dealing draws the deal, one already playing draws
   *  the game. Deliberately not `onDealing`, which would be a name that is
   *  wrong exactly when the table filled and both seats kept while this screen
   *  was up. */
  onStarted: () => void;
  /** Back to the lobby, carrying a sentence when there is one to carry: a
   *  table deleted under this screen is a notice the lobby shows, not a silent
   *  return to a list. The words are `signal.ts`'s `endSentence` when the
   *  abandon arrives as an `end` frame, which carries a reason and none of its
   *  own (final review I-2), and the server's own message when a refusal is
   *  what ended the signal. */
  onLeave: (sentence?: string) => void;
  /** The band's corner, built by `App` (the identity corner, `serverCorner`)
   *  and merely placed here beside the signal's own word. */
  who?: ReactNode;
}) {
  const [table, setTable] = useState<TableView | null>(handed ?? null);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  /** **Abandon's refusal, in a strip of its own** (M6.5c2 cycle C; the task
   *  review's I-1, ruled R-C24): one state per strip, because each strip sits
   *  beside its own control. `error` is the join block's — the join's, the
   *  password's, a refused copy's and the opening read's catch; this is the one the controls' Abandon
   *  raises, drawn directly under the controls. One shared state would put an
   *  Abandon refusal a screen-height above its button on a seated face. */
  const [abandonError, setAbandonError] = useState<string | null>(null);
  /** The table's chat count and revision, off the stream's own frame
   *  (`Source.peek`; M6.5c2 cycle D, row 130) — the panel's `count` and `rev`.
   *  The frame is an `Events` and still only a wake for the table read, but
   *  its `chat`/`chat_rev` are the table's own, so the panel is a COUNT host
   *  here as it is on the deal and the game: the connection's resync frame
   *  reads the chat only if `chat` or `chat_rev` moved past what the mount
   *  read holds, and an own post's frame asks nothing once its 201 has moved
   *  the cursor. */
  const [chat, setChat] = useState<{ count: number; rev: number } | null>(null);
  // The guard against a SECOND join going out while the first is in flight —
  // a ref and not the state, for the reason `App`'s own `inFlight` is one:
  // React batches, so two clicks in one tick both read a state flag as false
  // and both post, and the second earns a "seat taken" about the seat this
  // browser just took.
  const inFlight = useRef(false);
  // The two hand-overs through refs, so the poll's callbacks are stable and the
  // effect inside `usePolled` is not torn down on every render. The idiom is
  // `Dealing`'s.
  const started = useRef(onStarted);
  started.current = onStarted;
  const leave = useRef(onLeave);
  leave.current = onLeave;

  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));

  /** One answer about this table, from wherever it came — the opening read, a
   *  frame's re-read, a join's reply, a re-read after a refusal. A table that is no
   *  longer open is handed back rather than drawn: there is nothing on this
   *  screen to say about it. */
  const take = useCallback((t: TableView) => {
    if (t.status !== "open") {
      started.current();
      return;
    }
    setTable(t);
  }, []);

  // The opening read — only when nothing was handed in. See the note at the
  // top of the file for why this is not the hook's job. The handed document
  // is read through a ref, so a host re-rendering with a new object for the
  // same table does not re-run this.
  const wasHanded = useRef(handed !== undefined);
  useEffect(() => {
    if (wasHanded.current) return;
    let live = true;
    api
      .table(game)
      .then((t) => live && take(t))
      .catch((e) => live && setError(said(e)));
    return () => {
      live = false;
    };
  }, [game, take]);

  const link = usePolled<TableView>(
    {
      key: game,
      once: () => api.table(game),
      stream: `/api/games/${encodeURIComponent(game)}/stream`,
      peek: (data) => {
        const e = eventsOf(data);
        if (e) setChat({ count: e.chat, rev: e.chat_rev });
      },
    },
    true,
    take,
    // A 404 is a table somebody abandoned while this screen was up, and a 403
    // is one this caller may no longer read. Neither changes by being asked
    // again, so the screen goes rather than sitting on "reconnecting…".
    useCallback((end: End) => leave.current(end.sentence), []),
  );

  const join = async (seat: number) => {
    if (inFlight.current || !table) return;
    inFlight.current = true;
    setError(null);
    try {
      // **The name this browser keeps, read at the click** (M6.5c2, R-6.5c2.8):
      // the form asks for none — the lobby's corner owns it — so a name set
      // there a moment ago is the one this seat takes. **The keys are omitted
      // rather than sent undefined.** Every request body on this API is
      // `deny_unknown_fields`, and a name this browser does not have is not a
      // name to claim a seat under.
      const shown = displayName()?.trim();
      take(
        await api.join(game, {
          seat,
          ...(shown ? { name: shown } : {}),
          ...(table.locked && password.trim() ? { password: password.trim() } : {}),
        }),
      );
    } catch (e) {
      // **A 409 CARRYING a status is an answer, not a refusal to show.** The
      // table filled and started between this screen being drawn and the click;
      // the useful response is the next screen.
      if (e instanceof ApiError && e.status === 409 && hasStatus(e.body)) {
        started.current();
        return;
      }
      setError(said(e));
      // The other 409 is "that seat is taken", which is a statement about the
      // table having moved — so the table is re-read ONCE. A 403 is about the
      // caller (a wrong password), and the fields it is about stay where they
      // are.
      if (e instanceof ApiError && e.status === 409) {
        try {
          take(await api.table(game));
        } catch {
          // The re-read failed too: the refusal above is the more useful of the
          // two, and a second sentence would replace it with one about this
          // recovery.
        }
      }
    } finally {
      inFlight.current = false;
    }
  };

  /** **The creator's Reopen** (security S3, D9): a joiner's seat back to open,
   *  at an open table only. The answer IS the table, so it is drawn at once; a
   *  refusal is the server's sentence in the join block's strip — the strip
   *  beside the seats it is about — and the screen stays. One post at a time,
   *  on the join's own guard: a second click on the same seat would earn "that
   *  seat holds no joiner" about the seat this click just freed. */
  const reopen = async (seat: number) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setError(null);
    try {
      take(await api.reopen(game, seat));
    } catch (e) {
      setError(said(e));
    } finally {
      inFlight.current = false;
    }
  };

  /** The band's corner: what the signal can say for itself, and whatever `App`
   *  put there (the identity corner). `live` says nothing, as on every screen. */
  const corner = (
    <>
      {link !== "live" && (
        <span className="meta link" role="status">
          {link}&hellip;
        </span>
      )}
      {who}
    </>
  );

  if (!table) {
    return (
      <div className="band-screen">
        {/* "Table" as the page until the table has a name to give it — never
            "Lobby" as the LAST crumb, which is the lobby's own heading (the
            band draws the last crumb as the page's `<h2>`), so a cell waiting
            for the lobby would find it on this screen. */}
        <LobbyBand crumbs={["Lobby", "Table"]} corner={corner} />
        <div className="setup open-table">
          {error ? <Refusal sentence={error} /> : <p className="muted">loading&hellip;</p>}
        </div>
      </div>
    );
  }

  const title = table.name ?? table.id;
  const seated = table.my_seats.length > 0;
  const colors = table.colors as PlayerColor[];
  const url = `${window.location.origin}/${hashOf({ kind: "table", id: game })}`;
  const free = table.seats.flatMap((s, i) => (s.kind === "open" ? [i] : []));
  /** **One free seat, one button** (R-6.5c2.9): nothing to choose, so the page
   *  offers the seat that is there. Two or more, and each open card has its
   *  own. Nobody is seated by arriving — one click, never zero. */
  const onlySeat = !seated && free.length === 1 ? free[0]! : null;
  /** Whose table this is, in the words its creator's seat shows (§5.6): the
   *  seat's NAME, which the server sends as `creator`, and never an address
   *  (M6.5c2 cycle D, R-6.5c2.35). */
  const owner = table.creator ?? "the creator";

  /** One seat, as its card says it (mocks 08, 09). A taken seat is its holder's
   *  NAME with "seated" — not mock 08's "seated by you when you opened the
   *  table", which nothing here can know: a seat bound at create and one joined
   *  later are the same `SeatView`. */
  const face = (s: SeatView, i: number) => {
    const color = colors[i]!;
    const caption = i === table.start_seat ? `${color} \u00b7 first player` : color;
    switch (s.kind) {
      case "open":
        if (seated) {
          return {
            color,
            caption,
            open: true,
            who: "Open seat",
            note: table.locked ? "anyone in the lobby, with the password" : "anyone in the lobby",
          };
        }
        if (onlySeat !== null) return { color, caption, open: true, who: "Open seat", note: "yours if you join" };
        return {
          color,
          caption,
          open: true,
          who: "Open",
          action: (
            <button
              type="button"
              className="primary sit"
              aria-label={`Sit here as ${color}`}
              onClick={() => void join(i)}
            >
              Sit here
            </button>
          ),
        };
      // Unreachable at an OPEN table — a two-seat table with the bot has no
      // seat left open, so it is dealing from its create — and drawn anyway, so
      // a fourth kind of seat is a compile error here and not a blank card.
      case "bot":
        return { color, caption, who: "The bot" };
      // The holder's NAME, and no address (M6.5c2 cycle D, R-6.5c2.35): your
      // own seat notes the name you gave, and another's reads `seatName` — its
      // name, the address you typed if you created the table and they have
      // given none, or "invited".
      //
      // **The creator alone is offered Reopen, beside a seat that is not theirs**
      // (security S3, D9): the layer refuses their own seats (D-15), and a bot's
      // or an open seat has nobody in it, so the control stands only where a
      // press can land. "Not theirs" is this arm's own condition: it is reached
      // only for a seat outside `my_seats`, which the server reads off the same
      // comparison as each seat's `you`.
      case "human":
        return table.my_seats.includes(i)
          ? { color, caption, who: "You", note: s.name ?? undefined }
          : {
              color,
              caption,
              who: seatName(s),
              note: "seated",
              action:
                table.mine ? (
                  <button
                    type="button"
                    className="ghost reopen"
                    aria-label={`Reopen the ${color} seat`}
                    onClick={() => void reopen(i)}
                  >
                    Reopen
                  </button>
                ) : undefined,
            };
    }
  };

  const left =
    free.length === 1 ? "one seat left" : free.length === 2 ? "two seats left" : `${free.length} seats left`;
  const waiting = `Waiting for ${free.length} more ${free.length === 1 ? "player" : "players"}`;

  return (
    <div className="band-screen">
      <LobbyBand crumbs={["Lobby", title]} corner={corner} />
      {/* **Three parts, so the chat can take mock 08's rail** (cycle E, straw
          poll row 67): the table (its title, seats, join and invite), the
          chat, and the way out. From 976px a seated caller's page is two
          columns — the table and the way out on the left, the chat beside
          them on the right, as the lobby's chat is — and below it the three
          stack in this order, which is the page as it was. `with-rail` is the
          seated face; a stranger has no chat, and keeps the one column. */}
      <div className={seated ? "setup open-table with-rail" : "setup open-table"}>
        <div className="open-table-main">
          {/* The page's own title, a level under the band's crumb — the band's
              rule for a card under it; mocks 08 and 09 draw both. */}
          <h3 className="page-title">{title}</h3>
          <p className="table-status">
            <span>
              {seated ? `${waiting} \u00b7 ${table.players} players` : `${owner}'s table \u00b7 ${table.players} players \u00b7 ${left}`}
            </span>
            {table.locked && <PasswordTag />}
          </p>

          {/* **Above the seats** (spec §2.2): typed once, then the choice — the
              one button under the seats, or a seat's own "Sit here". Only at a
              LOCKED table: a password box over a table that needs none is a
              question with no right answer. */}
          {!seated && table.locked && free.length > 0 && (
            <div className="form-field join-password">
              <label htmlFor="join-password">Table password</label>
              <input
                id="join-password"
                type="password"
                autoComplete="off"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>
          )}

          <ol className="seat-cards" aria-label="Seats">
            {table.seats.map((s, i) => (
              <li key={i}>
                <SeatCard {...face(s, i)} />
              </li>
            ))}
          </ol>

          {onlySeat !== null && (
            <button type="button" className="primary join" onClick={() => void join(onlySeat)}>
              Join as {colors[onlySeat]}
            </button>
          )}

          {/* **The join block's refusals sit by the join** (M6.5c2 cycle C,
              polish 4): directly under the password, the seats and "Join as",
              which is what a join's and a wrong password's refusals are about —
              not at the page's foot, under the controls (and, for a seated
              caller, the chat). A refused copy lands here too, directly above the
              Invite panel whose button raised it. Abandon's refusal does NOT: it
              has its own strip under the controls (`abandonError`, below). */}
          {error && <Refusal sentence={error} />}

          {seated && (
            <Invite url={url} locked={table.locked} mine={table.mine} onRefused={setError} />
          )}
        </div>

        {/* ONLY for a seated caller (spec §6.2). The table read is the one route
            widened to somebody who is not seated (R-2.7); the chat is NOT — a
            stranger here is 403 at it — so showing them a panel would be
            showing them a box that refuses everything typed into it. Under the
            invite below 976px; from 976px in mock 08's rail beside the table
            (cycle E, row 67 — it was the main column's at every width). */}
        {seated && (
          <Chat
            scope={{ kind: "table", id: game }}
            count={chat?.count}
            rev={chat?.rev}
            canModerate={table.mine}
            colors={colors}
            sendName={false}
          />
        )}

        <div className="open-table-foot">
          <div className="open-table-controls">
            {/* The shared control (R-13), so this screen and the lobby's rows
                cannot drift apart again. `onDone` is the leave: the table is
                gone, and there is nothing here to go back to. Outlined in the
                warning colour, the quieter danger (mock 08). */}
            {/* **The creator's, SEATED** — the layer's rule, `mayAbandon`: its
                delete refuses a caller who holds no seat ("you are not seated
                at this table") before it asks who created the table, so a
                seatless creator is offered nothing a press would refuse. */}
            {mayAbandon(table) && (
              <Abandon table={table} onDone={() => leave.current()} onError={setAbandonError} />
            )}
            <button type="button" className="quiet-link" onClick={() => leave.current()}>
              <span aria-hidden="true">&larr; </span>Lobby
            </button>
          </div>
          {/* Abandon's refusal, directly under the button that raised it (R-C24). */}
          {abandonError && <Refusal sentence={abandonError} className="error abandon-refusal" />}
        </div>
      </div>
    </div>
  );
}

/** "Invite people" (mock 08): the link, Copy link, and — at a locked table —
 *  the reminder that the password travels separately. Worded for the person
 *  who SET it and for anyone else seated, who did not. */
function Invite({
  url,
  locked,
  mine,
  onRefused,
}: {
  url: string;
  locked: boolean;
  mine: boolean;
  /** Where a refused copy is said: the screen's own refusal strip — and
   *  `null` once a later copy lands, which takes the refusal away. */
  onRefused: (sentence: string | null) => void;
}) {
  const [copied, setCopied] = useState(false);
  /** **"Copied" only once the clipboard has taken the link** (the whole-branch
   *  review's Minor 8). The clipboard is refused in any browser that has not
   *  granted it and in any page not served over TLS, where `navigator.clipboard`
   *  is absent and the call throws; either way the refusal is said in the
   *  lobby's own sentence, which carries the link, and the button keeps
   *  offering the copy. Nothing rejects out of here: an unhandled rejection is
   *  a `pageerror` the browser gate counts as breakage.
   *
   *  **A copy that lands clears the strip** (M6.5c2 cycle C; the Opus
   *  re-review's one Minor): a clipboard refused and then granted left the
   *  refusal standing under a button that says "Copied". */
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      onRefused(null);
    } catch {
      setCopied(false);
      onRefused(`the link could not be copied \u2014 it is ${url}`);
    }
  };
  return (
    <section className="invite" aria-labelledby="invite-caption">
      <h3 id="invite-caption" className="caption">
        Invite people
      </h3>
      <div className="row">
        {/* Read-only rather than a sentence, so it can be selected and copied
            by hand where the clipboard is refused — which it is in every
            browser that has not been granted it, and in a page served over
            plain HTTP. */}
        <input aria-label="Invite link" readOnly value={url} onFocus={(e) => e.currentTarget.select()} />
        {/* **"Copied" is said ONCE** (cycle E, straw poll row 68): by the
            status beside the button, drawn from the first render so its change
            is announced, and never by the button, whose name stays "Copy link"
            — it was the live region, and its words were its name. One flex
            item with the button, so an empty status adds no gap to the row. */}
        <span className="copy">
          <button type="button" className="ghost" onClick={() => void copy()}>
            Copy link
          </button>
          <span className="copy-state" role="status">
            {copied ? "Copied" : ""}
          </span>
        </span>
      </div>
      {locked && (
        <p className="note">
          {mine ? "They'll need the password you set. Send it separately." : "They'll need the table's password."}
        </p>
      )}
    </section>
  );
}

/** The chat counters of one frame off the table's stream — an `Events` — or
 *  `null` for a frame that carries none. Read defensively: it is a frame off
 *  the wire, and a bad one should cost the panel its count, not the screen. */
function eventsOf(data: string): Pick<Events, "chat" | "chat_rev"> | null {
  try {
    const e = JSON.parse(data) as Partial<Events>;
    return typeof e.chat === "number" && typeof e.chat_rev === "number" ? { chat: e.chat, chat_rev: e.chat_rev } : null;
  } catch {
    return null;
  }
}

/** Does this refusal body carry the table's STATUS?
 *
 *  `DealingErrorBody` is the ordinary error shape plus one field, and it is the
 *  field that distinguishes "the table moved on" from "that seat is taken" —
 *  two 409s whose right answers are a different screen and a re-read. Written
 *  as a guard rather than inline so the narrowing is one statement. */
function hasStatus(body: unknown): boolean {
  return typeof body === "object" && body !== null && "status" in body;
}
