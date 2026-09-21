// The SERVER's first screen (spec §5.2): the tables you sit at, and the tables
// waiting for someone to sit down.
//
// **At `#/lobby` since M6.5b2, and no longer the screen a visitor lands on.**
// `#/` is the front door (R-6.5b2.1), which draws from this browser alone; the
// Access turnstile `/enter` redirects here, which is why the lobby needed an
// address of its own. "On this device" went with the door — it is a channel
// now, not a section above these lists — and what is left here is one link
// back to it.
//
// **Two scopes, ONE signal** (R-plan-4). `mine` and `open` are two questions
// about the same store and a player reads them together, so one `usePolled`
// asks both in one `Promise.all` and hands over one object. Two hooks would be
// two connections, two "reconnecting" words for one link, and — the half that
// shows — a screen where half the lobby is older than the other half.
//
// **The stream it listens to is the HOUSE channel**, `/api/stream`, and a frame
// off it is two counters and never a table (M6.3 R-3.4, M6.4 R-6.4.7): a
// `watch` channel coalesces, so two bumps naming two different tables inside
// one wake would lose one, and a frame naming a table would therefore sometimes
// name the wrong one. The lobby re-reads ALL its lists on any bump instead,
// which is what `Source`'s default `frames: "wake"` mode does — `once` is the
// answer and the frame is only the wake-up, so neither counter is READ here.
//
// **The first load is this component's, not the hook's.** The house stream says
// NOTHING on connect (deliberately: the lobby has just fetched its lists to
// render at all, and a frame at connect would make it fetch them again on every
// navigation), so an unaided hook would show "loading…" until somebody else
// moved.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Abandon } from "./Abandon";
import { api, ApiError, clearDisplayName, displayName, lobbyChat, setDisplayName } from "./api";
import { Chat } from "./Chat";
import { hashOf, navigate } from "./route";
import { usePolled } from "./signal";
import { TableFacts } from "./TableFacts";
import type { GamesList, TableView } from "./types";

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
 *  `onClick` calling `navigate` is the same jump with none of it.
 *
 *  **Exported, and a `function` rather than a `const`, since M6.5b.** "On this
 *  device" draws the same row for a game in this browser (R-6.5b.8), and one
 *  wording is the point of sharing it. The two modules import each other — the
 *  lobby renders that section and that section draws these rows — so the
 *  declaration is hoisted, which is what keeps the cycle safe whichever side
 *  the bundler evaluates first.
 *
 *  `abandon` is the control the CREATOR's own listing draws beside a row, and
 *  nothing else passes one: Open tables and Games in progress withhold it
 *  because a table of yours that is playing is listed twice (M6.2 ruling #13),
 *  and a LOCAL row withholds it because a local game is abandoned on its own
 *  screen, where the tab holds it (P-6). It is a node and not a flag: the
 *  control's `onDone` and `onError` are the lobby's own listing and error
 *  strip, which a row builder knows nothing about. */
export function lobbyRow(t: TableView, abandon?: ReactNode): ReactNode {
  return (
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
      {abandon}
    </li>
  );
}

/** All three lists, as one answer. */
type Lists = { mine: GamesList; open: GamesList; playing: GamesList };

/** Outside the component because it closes over nothing: `api` is a module, so
 *  a definition inside would be a new function on every render and the poll's
 *  own doc explains why that matters.
 *
 *  **Three scopes in ONE `Promise.all`**, for the reason two were: a player
 *  reads the lobby as one screen, and three hooks would be three connections
 *  and a screen where one third of it is older than the rest. */
const fetchLists = async (): Promise<Lists> => {
  const [mine, open, playing] = await Promise.all([
    api.list("mine"),
    api.list("open"),
    api.list("playing"),
  ]);
  return { mine, open, playing };
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
  door,
  serverUp,
  onReached,
}: {
  /** Who this browser plays as, from the door — `null` when the door has not
   *  opened, which since M6.5b is a screen this component still draws.
   *
   *  **Only one row control needs the address, and it is the destructive one**
   *  (R-13): a table is abandoned by the person who OPENED it, so the control
   *  is drawn from `created_by === me` rather than from "you sit here". Passed
   *  down rather than read from `/api/me` again, because `App` has already
   *  asked and a second answer is a second thing that can disagree.
   *
   *  **`null` also stops every request this screen makes.** Every
   *  `/api/games` route is a 401 without an identity, so a lobby that listed
   *  anyway would show the player a refusal as their first screen — which is
   *  what the door being a GATE used to prevent and what the door being
   *  CONTENT has to go on preventing. */
  me: string | null;
  who?: ReactNode;
  /** A sentence from OUTSIDE the lobby, drawn ABOVE the lobby's two lists.
   *
   *  **The lobby is where a refused SERVER table lands** (spec §5.1): a
   *  `#/t/<id>` the caller cannot see is a 403, and the screen it goes back to
   *  has to carry the sentence — a player returned to a bare lobby learns only
   *  that their link did nothing. The game screen's error strip is inside the
   *  game, so it could not have said this. A refused `local-` table lands on
   *  the DOOR instead (M6.5b2, R-6.5b2.1), where its siblings are listed, and
   *  `FrontDoor` carries the same sentence there.
   *
   *  **Above the lists, and not under them** (live review M-6). The spec's word
   *  is "shows the sentence above the lobby", and this was rendered last: a
   *  player who followed a dead link on a phone landed on a lobby whose
   *  explanation sat a scroll below two lists, so the screen read as a link
   *  that had simply done nothing. It goes under the heading, which is where
   *  `SignIn` puts the door's own notice — the lobby's OWN refusals stay at the
   *  foot, beside the lists they are about. */
  notice?: string | null;
  /** What the SERVER's part of this screen says when there is no open door
   *  (M6.5b, spec §2.5): the door's error, its silence, the sign-in form or
   *  the sentence naming the header that was expected. Built by `App` and
   *  drawn HERE, in the place the four sections would be — where those four
   *  answers used to stand in front of the whole page, the games in this
   *  browser included. */
  door?: ReactNode;
  /** Whether `GET /api/health` landed. `null` until it answers; `false`
   *  replaces the four sections with one line, because a screen drawing
   *  "none yet" over an API it cannot reach says something false about the
   *  player's own games. */
  serverUp?: boolean | null;
  /** A listing LANDED, so the server is reachable whatever the health read
   *  said — which is what brings the sections back. One direction only: this
   *  screen reports its own refusals in its error strip, and the line above is
   *  about the health read. */
  onReached?: () => void;
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
  /** "Something on the server may have changed" (spec §6.2), counted.
   *
   *  The house frame carries two counters and names no table (M6.3 R-3.4, M6.4
   *  R-6.4.7), and this screen is a `"wake"` consumer, so neither number
   *  reaches here — the frame is dropped and the wake is the whole of it. The
   *  panel is told to ask for its delta off the same wake the three listings
   *  are refetched on, which is the only source it has. **The cost, stated**
   *  (spec §6.2, §11): a house message is one more house wake, and every
   *  visible lobby refetches its three listings on it. The second counter
   *  exists so that a later `"value"` consumer could tell the two apart; this
   *  one does not. */
  const [tick, setTick] = useState(0);
  /** The name this browser plays under, on ONE key with the door, the New table
   *  form and the seat picker (R-6.4.3). Written as TYPED rather than on a
   *  submit, because this screen has none — and an emptied field REMOVES the
   *  key, so "leave it blank and your address is used" is true of the browser
   *  as well as of the help. */
  const [name, setName] = useState(() => displayName() ?? "");

  /** The host's "a read landed" callback, held so that `take` does not depend
   *  on it.
   *
   *  **A dependency here would be a fetch per render of the host.** `App`
   *  builds this inline, as hosts do, so its identity changes every time `App`
   *  re-renders — and `take` is what the OPENING READ's effect depends on.
   *  Measured before it was written this way: three re-renders of an unchanged
   *  screen made four listing reads. A ref costs nothing and asks nothing of
   *  the caller. */
  const reached = useRef(onReached);
  reached.current = onReached;

  const take = useCallback((v: Lists) => {
    setLists(v);
    setError(null);
    setTick((n) => n + 1);
    // A listing landed, so the server is reachable — whatever the boot's one
    // health read concluded. This is what brings the four sections back for a
    // player who opened the page while the API was down.
    reached.current?.();
  }, []);
  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));

  // The opening read. The poll takes it from here — see the note at the top of
  // the file for why this is not the hook's job.
  //
  // **Nothing is asked for with no open door** (M6.5b): every `/api/games`
  // route is a 401 without an identity, so a screen that listed anyway would
  // put a refusal in front of a player who has not signed in yet — which is
  // what the door being a gate used to prevent.
  useEffect(() => {
    if (me === null) return;
    let live = true;
    fetchLists()
      .then((v) => live && take(v))
      .catch((e) => live && setError(said(e)));
    return () => {
      live = false;
    };
  }, [take, me]);

  const link = usePolled<Lists>(
    { key: "lobby", once: fetchLists, stream: "/api/stream" },
    me !== null,
    take,
  );

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

  /** One of this screen's rows: `lobbyRow`'s wording, with the abandon control
   *  wired to THIS listing.
   *
   *  **Abandoning is the CREATOR's, on the YOUR-GAMES rows** (R-13). It was
   *  the setup screen's saved-list control until M6.2; the list moved here, so
   *  the control did, and it reaches the dealing and playing tables the open
   *  table's own screen cannot.
   *
   *  `yours` and not `created_by === me` alone: a table that is both yours and
   *  open is listed in BOTH sections, and drawing the control in each puts two
   *  buttons with the same accessible name on one screen. The ruling names
   *  Your games, so that is where it is. */
  const row = (t: TableView, yours: boolean) =>
    lobbyRow(
      t,
      yours && t.created_by === me ? <Abandon table={t} onDone={reread} onError={setError} /> : null,
    );

  const mine = lists ? unique([...lists.mine.games, ...(paged?.games ?? [])]) : [];
  const nextCursor = paged ? paged.next : (lists?.mine.next ?? null);

  return (
    <div className="setup lobby">
      {who && (
        <div className="who-strip">
          {who}
          <label className="your-name">
            Your name
            <input
              value={name}
              placeholder="optional"
              onChange={(e) => {
                setName(e.target.value);
                const trimmed = e.target.value.trim();
                if (trimmed) setDisplayName(trimmed);
                else clearDisplayName();
              }}
            />
          </label>
          {link !== "live" && (
            <span className="meta link" role="status">
              {link}&hellip;
            </span>
          )}
        </div>
      )}
      <h2>Lobby</h2>
      {/* **The one way back to the door** (R-6.5b2.1). "On this device" stood
          above these lists until this cycle; single player is a channel of its
          own now, at `#/`, and this screen is no longer the first one. It is an
          ANCHOR for `lobbyRow`'s reason — the control's whole job is to go to an
          address — and it is drawn OUTSIDE the three states below, so a player
          who is not signed in, or whose API cannot be reached, can still reach
          the games in their own browser from here. */}
      <a className="to-door" href={hashOf({ kind: "door" })}>
        Play on this device
      </a>
      {notice && <div className="error">{notice}</div>}

      {/* **The part of this screen the SERVER owns, and its three states**
          (M6.5b, spec §2.5). The door's four answers used to be four `return`s
          in front of the whole page; they are drawn here now, so everything
          above — the frame and the games in this browser — is on screen in
          every one of them.

          **The DOOR wins when there is one**, and that is `App`'s decision
          rather than this screen's: a door that answered is a server this
          browser can reach, so its words are the useful ones. `App` withholds
          them only where nothing answered at all, which is the one line below.
          The listing behind it keeps trying, and one landed read brings the
          sections back. */}
      {door ? (
        door
      ) : serverUp === false ? (
        <p className="muted">not connected</p>
      ) : (
        <>
          <button type="button" className="primary" onClick={() => navigate({ kind: "new" })}>
            New table
          </button>

          {/* **`your-games` is a NAME and not decoration.** Until M6.3 this was the
              one section that was not `.open-tables`, and a browser cell scoped its
              reads that way — which a third section silently made ambiguous
              (measured: the join-by-link walk resolved two rows for one table).
              Every section on this screen now says which one it is. */}
          <section className="saved your-games">
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

          {/* **Every game still being played, so a watcher can find one without a
              link** (R-3.9). A row here opens `#/t/<id>` exactly as the other two
              do, and what that lands on is the spectator screen — or, at a locked
              table, the face that asks for its password, which is what the lock
              marker on the row is warning about.

              **No Abandon control, and that is the section rather than the test**
              (M6.2 ruling #13): abandoning is the creator's, on the Your-games
              rows, and a table of yours that is playing is listed in BOTH — two
              buttons with one accessible name on one screen. `row(t, false)` is how
              Open tables withholds it too. */}
          <section className="saved playing-tables">
            <h3>Games in progress</h3>
            {lists === null ? null : lists.playing.games.length === 0 ? (
              <p className="muted">nothing is being played right now.</p>
            ) : (
              <ul>{lists.playing.games.map((t) => row(t, false))}</ul>
            )}
          </section>

          {/* The house chat (R-6.4.5, R-6.4.6): a fourth section, folded by default —
              opening it is entering the lobby chat — and absent when the server says
              the lobby chat is off, because a panel whose every request is a 404 is
              worse than no panel. The SECTION is this screen's and the panel keeps
              its own root, so the lobby styles a component it did not write. Its
              tick is this screen's own wake. */}
          {lobbyChat() && (
            <section className="saved lobby-chat">
              <Chat scope={{ kind: "house" }} tick={tick} sendName heading="h3" />
            </section>
          )}
        </>
      )}

      {error && <div className="error">{error}</div>}
    </div>
  );
}
