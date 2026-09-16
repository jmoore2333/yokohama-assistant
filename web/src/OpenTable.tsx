// The open table (spec §5.4): the screen a join link lands on, and the screen
// the person who opened the table waits on.
//
// **Two faces of one table, and `my_seats` picks between them.** A caller with
// no seat here is a stranger — the one route on this API widened to somebody
// who is not seated (R-2.7), because a join link is an address and the person
// following one has to see what they are joining — and what they are offered is
// a seat, a name to take it under and, at a locked table, its password. A
// caller who already holds a seat is WAITING, and what they are offered is the
// link to send to whoever is still missing, plus — if they opened the table —
// the way to abandon it.
//
// **The table is fetched here as well as watched**, which is the lobby's own
// shape and for the lobby's own reason: this screen holds an id and nothing
// else, and the opening read is what keeps it from saying "loading…" until
// somebody else acts. The watch is `usePolled` in its default `"wake"` mode
// (M6.3 R-7.1): a frame off `…/{id}/stream` is an `Events` and this screen's
// value is a `TableView`, so the frame is the wake-up and `GET …/table` is the
// answer. The stream's ON-CONNECT frame is therefore a second read of the table
// at connect, which is one request and buys the reconnect its compare.
//
// **Leaving this screen is a re-OPEN, not a jump to the dealing screen.** The
// three ways a table stops being open — the signal, the join's own reply, and a
// 409 that carries a status — all say only "not open any more", and which of
// the two remaining screens that is depends on whether the last seat has kept.
// So the hand-over asks `App` to open the table again by its status rather than
// naming a screen it cannot know (spec §5.4).

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Abandon } from "./Abandon";
import { api, ApiError, displayName, setDisplayName } from "./api";
import { seatClass } from "./Cards";
import { hashOf } from "./route";
import { usePolled, type End } from "./signal";
import { seatName } from "./tables";
import type { PlayerColor, TableView } from "./types";

export function OpenTable({
  game,
  me,
  onStarted,
  onLeave,
  who,
}: {
  /** The table, by id. */
  game: string;
  /** Who this browser plays as, from the door — compared against the table's
   *  `created_by` to decide whether the abandon control is this person's. */
  me: string;
  /** The table is no longer open. `App` re-opens it and its STATUS picks the
   *  screen: a table still dealing draws the deal, one already playing draws
   *  the game. Deliberately not `onDealing`, which would be a name that is
   *  wrong exactly when the table filled and both seats kept while this screen
   *  was up. */
  onStarted: () => void;
  /** Back to the lobby, carrying a sentence when there is one to carry: a
   *  table deleted under this screen is a 404 the lobby shows, not a silent
   *  return to a list. */
  onLeave: (sentence?: string) => void;
  /** The "You are …" strip, built by `App` and merely placed here. */
  who?: ReactNode;
}) {
  const [table, setTable] = useState<TableView | null>(null);
  const [name, setName] = useState(() => displayName() ?? "");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
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
   *  poll tick, a join's reply, a re-read after a refusal. A table that is no
   *  longer open is handed back rather than drawn: there is nothing on this
   *  screen to say about it. */
  const take = useCallback((t: TableView) => {
    if (t.status !== "open") {
      started.current();
      return;
    }
    setTable(t);
  }, []);

  // The opening read. See the note at the top of the file for why this is not
  // the hook's job.
  useEffect(() => {
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
      const trimmed = name.trim();
      if (trimmed) setDisplayName(trimmed);
      // **The keys are omitted rather than sent undefined.** Every request body
      // on this API is `deny_unknown_fields`, and a name this browser does not
      // have is not a name to claim a seat under.
      take(
        await api.join(game, {
          seat,
          ...(trimmed ? { name: trimmed } : {}),
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

  const strip = (who || link !== "live") && (
    <div className="who-strip">
      {link !== "live" && (
        <span className="meta link" role="status">
          {link}&hellip;
        </span>
      )}
      {who}
    </div>
  );

  if (!table) {
    return (
      <div className="setup open-table">
        {strip}
        <h2>Table</h2>
        {error ? <div className="error">{error}</div> : <p className="muted">loading&hellip;</p>}
      </div>
    );
  }

  const seated = table.my_seats.length > 0;
  const colors = table.colors as PlayerColor[];
  const url = `${window.location.origin}/${hashOf({ kind: "table", id: game })}`;
  const waitingOn = table.seats.filter((s) => s.kind === "open").length;

  return (
    <div className="setup open-table">
      {strip}
      <h2>{table.name ?? table.id}</h2>
      <p className="lede">
        {seated
          ? `waiting for ${waitingOn} more ${waitingOn === 1 ? "player" : "players"}`
          : "take a seat"}
        {table.locked && " · password required"}
      </p>
      {!seated && (
        <>
          <div className="field">
            <label htmlFor="your-name">Your name</label>
            <input id="your-name" value={name} onChange={(e) => setName(e.target.value)} />
          </div>
          {/* Only at a LOCKED table: a password box over a table that needs
              none is a question with no right answer. */}
          {table.locked && (
            <div className="field">
              <label htmlFor="join-password">Password</label>
              <input
                id="join-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>
          )}
        </>
      )}
      <ul className="seats">
        {table.seats.map((s, i) => (
          <li key={i} className={`field seat-row ${seatClass(colors[i])}`}>
            <span className="meta">
              Seat {i} &middot; {colors[i]}
            </span>
            {s.kind === "open" && !seated ? (
              <button type="button" className="primary" onClick={() => void join(i)}>
                sit here
              </button>
            ) : (
              <span>{seatName(s)}</span>
            )}
          </li>
        ))}
      </ul>
      {seated && (
        <div className="field link-field">
          <label htmlFor="join-link">Link</label>
          {/* Read-only rather than a sentence, so it can be selected and copied
              by hand where the clipboard is refused — which it is in every
              browser that has not been granted it, and in a page served over
              plain HTTP. */}
          <input id="join-link" readOnly value={url} />
          <button
            type="button"
            className="ghost"
            onClick={() => {
              // **The rejection is swallowed, and that is deliberate.** The
              // clipboard is refused in any browser that has not granted it and
              // in any page not served over TLS, and an unhandled rejection is
              // a `pageerror` the browser gate counts as breakage. The input
              // beside this holds the link either way, which is why the failure
              // is not worth a sentence.
              navigator.clipboard?.writeText(url).catch(() => {});
              setCopied(true);
            }}
          >
            {copied ? "copied" : "copy"}
          </button>
        </div>
      )}
      <div className="field open-table-controls">
        {/* The shared control (R-13), so this screen and the lobby's rows cannot
            drift apart again — they already had, over the in-flight guard this
            copy did not carry. `onDone` is the leave: the table is gone, and
            there is nothing here to go back to. */}
        {seated && table.created_by === me && (
          <Abandon table={table} onDone={() => leave.current()} onError={setError} />
        )}
        <button type="button" className="ghost" onClick={() => leave.current()}>
          Lobby
        </button>
      </div>
      {error && <div className="error">{error}</div>}
    </div>
  );
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
