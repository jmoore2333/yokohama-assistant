// The first screen after the door (spec §5.2): the tables you sit at, and the
// tables waiting for someone to sit down.
//
// **Two scopes, ONE poll** (R-plan-4). `mine` and `open` are two questions
// about the same store and a player reads them together, so one `usePolled`
// asks both in one `Promise.all` and hands over one object. Two hooks would be
// two timers drifting apart, two "reconnecting" words for one link, and — the
// half that shows — a screen where half the lobby is three seconds older than
// the other half.
//
// **The first load is this component's, not the hook's.** `usePolled` asks one
// interval after it arms, which is right for every other consumer: each already
// holds the thing it is watching. The lobby holds nothing, so an unaided poll
// would show "loading…" for three seconds on every visit.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Abandon } from "./Abandon";
import { api, ApiError } from "./api";
import { hashOf, navigate } from "./route";
import { usePolled } from "./signal";
import { TableFacts } from "./TableFacts";
import type { GamesList, TableView } from "./types";

/** Both lists, as one answer. */
type Lists = { mine: GamesList; open: GamesList };

/** Outside the component because it closes over nothing: `api` is a module, so
 *  a definition inside would be a new function on every render and the poll's
 *  own doc explains why that matters. */
const fetchLists = async (): Promise<Lists> => {
  const [mine, open] = await Promise.all([api.list("mine"), api.list("open")]);
  return { mine, open };
};

/** The rows, with a table listed once however many pages it arrived on.
 *
 *  The poll refreshes PAGE ONE under a pager that has already fetched page two,
 *  so a table that moved between them — somebody played a move, and the listing
 *  is newest-first — is legitimately in both answers. Listed twice it would look
 *  like two tables with one name. */
function unique(tables: TableView[]): TableView[] {
  const seen = new Set<string>();
  return tables.filter((t) => (seen.has(t.id) ? false : (seen.add(t.id), true)));
}

export function Lobby({
  me,
  who,
  notice,
}: {
  /** Who this browser plays as, from the door.
   *
   *  **Only one row control needs it, and it is the destructive one** (R-13):
   *  a table is abandoned by the person who OPENED it, so the control is drawn
   *  from `created_by === me` rather than from "you sit here". Passed down
   *  rather than read from `/api/me` again, because `App` has already asked and
   *  a second answer is a second thing that can disagree. */
  me: string;
  who?: ReactNode;
  /** A sentence from OUTSIDE the lobby, drawn ABOVE the lobby's two lists.
   *
   *  **The lobby is where a refused table lands** (spec §5.1): a `#/t/<id>` the
   *  caller cannot see is a 403, and the screen it goes back to has to carry the
   *  sentence — a player returned to a bare lobby learns only that their link
   *  did nothing. The game screen's error strip is inside the game, so it could
   *  not have said this.
   *
   *  **Above the lists, and not under them** (live review M-6). The spec's word
   *  is "shows the sentence above the lobby", and this was rendered last: a
   *  player who followed a dead link on a phone landed on a lobby whose
   *  explanation sat a scroll below two lists, so the screen read as a link
   *  that had simply done nothing. It goes under the heading, which is where
   *  `SignIn` puts the door's own notice — the lobby's OWN refusals stay at the
   *  foot, beside the lists they are about. */
  notice?: string | null;
}) {
  const [lists, setLists] = useState<Lists | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** The pages after the first, and the cursor after THEM.
   *
   *  **Held apart from `lists`, because the poll replaces `lists`.** A pager
   *  that wrote its cursor back into the polled answer would have it reset three
   *  seconds later, and the next click on "more" would re-fetch the page it
   *  already has. `null` means the pager has not been used, and the cursor is
   *  then the polled list's own. */
  const [paged, setPaged] = useState<{ games: TableView[]; next: string | null } | null>(null);
  const pageInFlight = useRef(false);

  const take = useCallback((v: Lists) => {
    setLists(v);
    setError(null);
  }, []);
  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));

  // The opening read. The poll takes it from here — see the note at the top of
  // the file for why this is not the hook's job.
  useEffect(() => {
    let live = true;
    fetchLists()
      .then((v) => live && take(v))
      .catch((e) => live && setError(said(e)));
    return () => {
      live = false;
    };
  }, [take]);

  const link = usePolled<Lists>({ key: "lobby", once: fetchLists, stream: "/api/stream" }, true, take);

  const page = async (cursor: string) => {
    // The same guard, and the same reasoning, as `App`'s `inFlight`: React
    // batches, so two clicks in one tick both read a state flag as false and
    // both fetch — and here that appends the same page twice.
    if (pageInFlight.current) return;
    pageInFlight.current = true;
    try {
      const p = await api.list("mine", cursor);
      setPaged((old) => ({ games: [...(old?.games ?? []), ...p.games], next: p.next }));
    } catch (e) {
      setError(said(e));
    } finally {
      pageInFlight.current = false;
    }
  };

  /** What follows an abandon: re-read both listings, so the screen agrees with
   *  the store.
   *
   *  **The pager is dropped too.** Its pages were fetched around a table that no
   *  longer exists, and a cursor is `<created_at>,<id>` — the id of a row that
   *  has just gone. The DELETE, its confirmation and its in-flight guard are
   *  `Abandon`'s; this is only the part that is the lobby's.
   *
   *  It can THROW, and is meant to: `Abandon` awaits it inside its own attempt,
   *  so a listing that will not read reports in the strip below rather than as
   *  an unhandled rejection. */
  const reread = async () => {
    setPaged(null);
    take(await fetchLists());
  };

  /** One row: what the table is CALLED, then what it is. The words are
   *  `tables.ts`'s and the facts are `TableFacts`' — the same line the Game menu
   *  draws, so one table cannot read two ways depending on which list you are
   *  looking at.
   *
   *  **The name is a LINK and not a button** (spec §5.2, R-14). The row's whole
   *  job is to go to `#/t/<id>`, which is an address — so it is an anchor with
   *  that address in its `href`, and the browser does the navigating. That is
   *  what makes the row openable in a new tab, copyable from the context menu
   *  and readable as a destination by a screen reader; a button with an
   *  `onClick` calling `navigate` is the same jump with none of it. */
  const row = (t: TableView, yours: boolean) => (
    <li key={t.id}>
      <a className="ghost row-link" href={hashOf({ kind: "table", id: t.id })}>
        {t.name ?? t.id}
      </a>
      <TableFacts table={t} />
      {/* A table you need a password for, said BEFORE you click it. The mark is
          labelled rather than left as an emoji: a lock glyph announces as
          nothing useful, and this is the one fact on the row that changes what
          happens next. */}
      {t.locked && (
        <span className="meta lock" aria-label="password required">
          🔒
        </span>
      )}
      {/* **Abandoning is the CREATOR's, on the YOUR-GAMES rows** (R-13). It
          was the setup screen's saved-list control until this cycle; the list
          moved here, so the control did, and it reaches the dealing and playing
          tables the open table's own screen cannot.

          `yours` and not `created_by === me` alone: a table that is both yours
          and open is listed in BOTH sections, and drawing the control in each
          puts two buttons with the same accessible name on one screen. The
          ruling names Your games, so that is where it is. */}
      {yours && t.created_by === me && (
        <Abandon table={t} onDone={reread} onError={setError} />
      )}
    </li>
  );

  const mine = lists ? unique([...lists.mine.games, ...(paged?.games ?? [])]) : [];
  const nextCursor = paged ? paged.next : (lists?.mine.next ?? null);

  return (
    <div className="setup lobby">
      {who && (
        <div className="who-strip">
          {who}
          {link !== "live" && (
            <span className="meta link" role="status">
              {link}&hellip;
            </span>
          )}
        </div>
      )}
      <h2>Lobby</h2>
      {notice && <div className="error">{notice}</div>}
      <button type="button" className="primary" onClick={() => navigate({ kind: "new" })}>
        New table
      </button>

      <section className="saved">
        <h3>Your games</h3>
        {lists === null ? (
          <p className="muted">loading&hellip;</p>
        ) : mine.length === 0 ? (
          <p className="muted">none yet — open a table, or take a seat at one below.</p>
        ) : (
          <ul>{mine.map((t) => row(t, true))}</ul>
        )}
        {nextCursor && (
          <button type="button" className="ghost" onClick={() => void page(nextCursor)}>
            more
          </button>
        )}
      </section>

      {/* **Every open table on the server, not only the ones you were told
          about** (R-2.7). A join link is an address, and this is the other way
          in: a table filling its seats is a public thing until it starts. */}
      <section className="saved open-tables">
        <h3>Open tables</h3>
        {lists === null ? null : lists.open.games.length === 0 ? (
          <p className="muted">no table is waiting for players.</p>
        ) : (
          <ul>{lists.open.games.map((t) => row(t, false))}</ul>
        )}
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
