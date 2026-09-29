// The SERVER's first screen (spec section 5.2; M6.5c2, R-6.5c2.7, mock 02):
// the band, then three lists as rows (the tables you sit at, the tables
// waiting for someone to sit down, and the games being played) with the lobby
// chat in a rail beside them, and below them on a phone. Under the third list,
// folded, the finished games you do not sit at (cycle C, R-6.5c2.22).
//
// **At `#/lobby` since M6.5b2, and no longer the screen a visitor lands on.**
// `#/` is the front door (R-6.5b2.1), which draws from this browser alone; the
// Access turnstile `/enter` redirects here, which is why the lobby needed an
// address of its own. What is left of "On this device" here is one link back
// to it.
//
// **Three scopes, ONE signal** (R-plan-4). `mine`, `open` and `playing` are
// three questions about the same store and a player reads them together, so
// one `usePolled` asks all three at once and hands over one reading. Three
// hooks would be three connections and a screen where part of the lobby is
// older than the rest. **Each scope's answer is its own** (M6.5c2 cycle D,
// dossier row 4): one refused listing blanked all three while they were one
// `Promise.all`, so the reading settles each apart, a section draws the list
// it last received, and a refusal is said in the section it is about.
//
// **The stream it listens to is the HOUSE channel**, `/api/stream`, and a frame
// off it is two counters and never a table (M6.3 R-3.4, M6.4 R-6.4.7): a
// `watch` channel coalesces, so a frame naming a table would sometimes name the
// wrong one. The lobby re-reads ALL its lists on any bump instead, which is
// what `Source`'s default `frames: "wake"` mode does; `once` is the answer and
// the frame is the wake-up. **Its chat counters are read, for the chat panel
// alone** (M6.5c2 cycle D, row 130): `chat` and `rev` are true of the moment
// the frame was written, so the panel is a COUNT host here as it is beside a
// game, and asks for its delta only when the house chat has moved — not on
// every table's move, and not for its own post.
//
// **The first load is this component's, not the hook's.** The house stream says
// NOTHING on connect (deliberately: the lobby has just fetched its lists to
// render at all, and a frame at connect would make it fetch them again on every
// navigation), so an unaided hook would show nothing until somebody else moved.
// The three headings are static and drawn at once; until the lists land, Your
// tables reads "loading..." over skeleton rows. That is the lobby's own wait
// and not the door's state D, which is over by the time the lists are asked
// for.
//
// **A row is the table's name and age, a chip per seat, and ONE action**
// (R-6.5c2.7; the words are `tables.ts`'s). The name is the row's one LINK
// (R-14) and the action is a button to the same address, so a table is never
// two links (`exit_game.spec.ts` counts one). The creator's own rows keep
// `Abandon` under the action, in a cell of its own (`.row-abandon`), so the
// action stays on the row's line: the lobby is the only place a dealing or
// playing server table can be abandoned (R-13).

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { Abandon } from "./Abandon";
import { api, ApiError, lobbyChat } from "./api";
import { LobbyBand } from "./Band";
import { Chat } from "./Chat";
import { Padlock, PasswordTag, SeatChip } from "./Chips";
import { Unreachable } from "./Door";
import { Refusal } from "./Refusal";
import { hashOf, navigate } from "./route";
import { usePolled } from "./signal";
import { mayAbandon, rowAction, rowMeta, seatChips, type Section } from "./tables";
import type { GamesList, TableView } from "./types";

/** The three listings, one per section. */
type Scope = "mine" | "open" | "playing";
const SCOPES: readonly Scope[] = ["mine", "open", "playing"];

/** One reading of the three listings: each scope's answer or its refusal,
 *  apart (dossier row 4). */
type Reading = Record<Scope, PromiseSettledResult<GamesList>>;

/** What the screen holds: each scope's last list that LANDED, or `null`
 *  before one has. */
type Lists = Record<Scope, GamesList | null>;

/** Each scope's refusal from the last reading, or `null` where it answered. */
type Refusals = Record<Scope, string | null>;

const NOTHING: Lists = { mine: null, open: null, playing: null };
const NO_REFUSALS: Refusals = { mine: null, open: null, playing: null };

/** Outside the component because it closes over nothing: `api` is a module, so
 *  a definition inside would be a new function on every render and the poll's
 *  own doc explains why that matters.
 *
 *  **Three scopes at once, settled APART** (M6.5c2 cycle D, dossier row 4). They
 *  are asked together for the reason two were — a player reads the lobby as
 *  one screen — and each answer stands on its own, so a refusal of one leaves
 *  the other two drawn. **Every scope refused is a failed READ and throws**
 *  (the first refusal): that is what the poll's ladder and the "Can't reach
 *  the server" strip are for, and one sentence says it rather than three. */
const fetchLists = async (): Promise<Reading> => {
  const [mine, open, playing] = await Promise.allSettled([
    api.list("mine"),
    api.list("open"),
    api.list("playing"),
  ]);
  if (mine.status === "rejected" && open.status === "rejected" && playing.status === "rejected") {
    throw mine.reason;
  }
  return { mine, open, playing };
};

/** The house frame's two chat counters (`{seq, chat, rev}` on both hosts), or
 *  `null` for a frame that does not carry a `chat` number — the panel then
 *  keeps the count it had. Read defensively: it is a frame off the wire. */
const houseCounts = (data: string): { chat: number; rev?: number } | null => {
  try {
    const f = JSON.parse(data) as { chat?: unknown; rev?: unknown };
    if (typeof f.chat !== "number") return null;
    return { chat: f.chat, ...(typeof f.rev === "number" ? { rev: f.rev } : {}) };
  } catch {
    return null;
  }
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

/** A table you sit at is YOURS: it is listed under Your tables, and not again
 *  under Open tables, Being played or Finished, whose rows are for a table you
 *  could take a seat at, spectate or view (mock 02). `my_seats` is the server's
 *  answer for this caller, so the rule guesses nothing. */
const notMine = (t: TableView): boolean => t.my_seats.length === 0;

/** A listing that got NO answer, in a player's words: the door's state C says
 *  "Can't reach the server", and the platform's own `TypeError: Failed to
 *  fetch` is a message to a programmer. `api.ts`'s `call` turns every HTTP
 *  status into an `ApiError`, so anything else is `fetch` failing with none
 *  (`whoami.ts`'s `failureOf`, the same rule). True as written: the poll goes
 *  on asking, and the first read that lands clears the strip (`take`). */
const UNREACHED = "Can't reach the server. The lists come back as soon as it answers.";

/** A refusal in the player's words: the server's own sentence, or
 *  [`UNREACHED`] where nothing answered. */
const said = (e: unknown) => (e instanceof ApiError ? e.message : UNREACHED);

export function Lobby({
  me,
  corner,
  notice,
  door,
  serverUp,
  onReached,
  onRetry,
}: {
  /** Who this browser plays as, from the door — `null` when the door has not
   *  opened, which since M6.5b is a screen this component still draws.
   *
   *  **No row control reads the address any more** (M6.5c2 cycle D,
   *  R-6.5c2.35): the destructive one (R-13) is the table's CREATOR's, and the
   *  row says so itself — `TableView.mine`, a fact about this reader — rather
   *  than handing every reader the creator's address to compare. Passed down
   *  rather than read from `/api/me` again, because `App` has already asked
   *  and a second answer is a second thing that can disagree.
   *
   *  **`null` also stops every request this screen makes.** Every
   *  `/api/games` route is a 401 without an identity, so a lobby that listed
   *  anyway would show the player a refusal as their first screen — which is
   *  what the door being a GATE used to prevent and what the door being
   *  CONTENT has to go on preventing. */
  me: string | null;
  /** Who is playing, drawn in the band's corner — `App`'s `IdentityCorner`
   *  once somebody is signed in (M6.5c2, spec §2.1). Absent while nobody is,
   *  when the corner is the way back to this device's games instead. */
  corner?: ReactNode;
  /** A sentence from OUTSIDE the lobby, drawn ABOVE the lobby's lists.
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
   *  (M6.5b, spec §2.5): the door's states and the name step (`Door.tsx`). Built by `App` and
   *  drawn HERE, in the place the four sections would be — where those four
   *  answers used to stand in front of the whole page, the games in this
   *  browser included. */
  door?: ReactNode;
  /** Whether `GET /api/health` landed. `null` until it answers; `false`
   *  replaces the actions and the lists with state C's card, because a screen drawing
   *  "none yet" over an API it cannot reach says something false about the
   *  player's own games. */
  serverUp?: boolean | null;
  /** A listing LANDED, so the server is reachable whatever the health read
   *  said — which is what brings the sections back. One direction only: this
   *  screen reports its own refusals in its error strip, and the line above is
   *  about the health read. */
  onReached?: () => void;
  /** "Try again" on the card that says the server cannot be reached (state C):
   *  `App`'s `retry`, which asks both reads again. */
  onRetry?: () => void;
}) {
  const [lists, setLists] = useState<Lists>(NOTHING);
  /** Each section's own refusal from the last reading (dossier row 4). */
  const [refusals, setRefusals] = useState<Refusals>(NO_REFUSALS);
  /** The lobby's own strip, at the foot: a reading where every scope was
   *  refused, the pager's, a copy the browser refused, and Abandon's. */
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
  /** The house chat's newest message number and revision, off the house frame
   *  itself (`Source.peek`; M6.5c2 cycle D, row 130) — the lobby chat's
   *  `count` and `rev`. The frame names no table (M6.3 R-3.4) and is still only
   *  a wake for the listings, but its counters are the house's own, so the
   *  panel beside them is a count host: it reads its delta when `chat` passes
   *  what it holds, and not on the house wake every table's move sends — nor
   *  for its own post, whose 201 has already moved the cursor. **`null` until
   *  the first frame**: the house stream says nothing on connect, so a panel
   *  holds its mount read alone until the house next moves. **A message
   *  whose frame this browser never got** — sent while the socket was down
   *  (its one reconnect probe re-reads the listings, which carry no chat
   *  count; the tick used to turn that into a chat read too), or while the
   *  stream was reconnecting or the tab was hidden (the house says nothing on
   *  connect, on either host) — **is read at the next house frame**, whatever
   *  moved: every house frame carries the house chat's count. Until then it
   *  waits, for as long as the house is quiet. And if the next thing is the
   *  reader's OWN post, its 201's number shows the gap and the panel reads
   *  the page rather than stepping past it (`Chat`'s gap check; the Task 11
   *  re-review). **The cost that is left, stated** (spec §6.2, §11): a house
   *  message is one more house wake, and every visible lobby refetches its
   *  three listings on it. */
  const [house, setHouse] = useState<{ chat: number; rev?: number } | null>(null);
  /** The row whose link was just copied, for the two seconds its button says
   *  so. */
  const [copied, setCopied] = useState<string | null>(null);
  /** The two-second timer that puts the word back, held so a lobby that goes
   *  away first leaves nothing to fire (the whole-branch review's Minor 11),
   *  and so a second copy restarts the two seconds rather than racing the
   *  first copy's timer. */
  const copiedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (copiedTimer.current !== null) clearTimeout(copiedTimer.current);
    },
    [],
  );
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

  /** One reading, taken scope by scope: a listing that answered replaces its
   *  section's and clears its strip; one that was refused keeps the section's
   *  last list and says why beside it (dossier row 4). At least one answered —
   *  `fetchLists` throws when none did. */
  const take = useCallback((v: Reading) => {
    setLists((old) => {
      const next = { ...old };
      for (const s of SCOPES) {
        const r = v[s];
        if (r.status === "fulfilled") next[s] = r.value;
      }
      return next;
    });
    setRefusals(() => {
      const next = { ...NO_REFUSALS };
      for (const s of SCOPES) {
        const r = v[s];
        if (r.status === "rejected") next[s] = said(r.reason);
      }
      return next;
    });
    setError(null);
    // A listing landed, so the server is reachable — whatever the boot's one
    // health read concluded. This is what brings the four sections back for a
    // player who opened the page while the API was down.
    reached.current?.();
  }, []);

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

  const link = usePolled<Reading>(
    {
      key: "lobby",
      once: fetchLists,
      stream: "/api/stream",
      peek: (data) => {
        const counts = houseCounts(data);
        if (counts) setHouse(counts);
      },
    },
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

  /** What follows an abandon: re-read the listings, so the screen agrees with
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

  /** Put a table's address on the clipboard (mock 02's Copy link). A browser
   *  that refuses (an insecure origin has no clipboard) gets the address in
   *  the error strip instead, which a player can still copy by hand — and a
   *  copy that then lands clears the strip (M6.5c2 cycle C; the Opus
   *  re-review's one Minor), so no refusal stands beside "Copied". */
  const copyLink = async (t: TableView) => {
    const url = new URL(hashOf({ kind: "table", id: t.id }), window.location.href).href;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(t.id);
      setError(null);
      if (copiedTimer.current !== null) clearTimeout(copiedTimer.current);
      copiedTimer.current = setTimeout(() => {
        copiedTimer.current = null;
        setCopied((c) => (c === t.id ? null : c));
      }, 2000);
    } catch {
      setError(`the link could not be copied \u2014 it is ${url}`);
    }
  };

  /** One row: the name (the row's one LINK) with the password tag, the line
   *  under it, a chip per seat, and the one action.
   *
   *  **Abandoning is the CREATOR's, SEATED** (R-13), in a cell of its own
   *  under the action (`.row-abandon`) and quieter than it: in the action's
   *  cell it moved the action off the row's line. The rule is the layer's
   *  (`mayAbandon`, M6.5c2 cycle D): its delete refuses a caller who holds no
   *  seat before it asks who created the table. A table you sit at is listed
   *  under Your tables alone (`notMine`), so the control is only ever drawn
   *  there, and once. The action and Abandon both NAME the table in their
   *  accessible names, so a control can never be about another row. */
  const row = (t: TableView, section: Section) => {
    const action = rowAction(t, section);
    const name = t.name ?? t.id;
    const yours = mayAbandon(t);
    return (
      <li key={t.id} className={section === "mine" ? "lobby-row mine" : "lobby-row"}>
        <div className="row-title">
          <div className="row-name">
            <a className="row-link" href={hashOf({ kind: "table", id: t.id })}>
              {name}
            </a>
            {t.locked && <PasswordTag />}
          </div>
          <div className="row-meta">{rowMeta(t, section)}</div>
        </div>
        <div className="row-seats">
          {seatChips(t).map((chip, i) => (
            <SeatChip key={i} color={chip.color} label={chip.label} open={chip.open} toMove={chip.toMove} />
          ))}
        </div>
        <div className="row-action">
          {action.kind === "go" && (
            <button
              type="button"
              className={action.primary ? "primary yen row-go" : "ghost row-go"}
              aria-label={`${action.label}: ${name}`}
              onClick={() => navigate({ kind: "table", id: t.id })}
            >
              {action.padlock && <Padlock />}
              {action.label}
            </button>
          )}
          {action.kind === "copy" && (
            <button
              type="button"
              className="ghost row-go"
              aria-label={`${copied === t.id ? "Copied" : "Copy link"}: ${name}`}
              // A polite live region, so "Copied" is heard as well as seen.
              aria-live="polite"
              onClick={() => void copyLink(t)}
            >
              {copied === t.id ? "Copied" : "Copy link"}
            </button>
          )}
          {action.kind === "stranded" && (
            <span
              className="stranded"
              title={t.bot ? `played against ${t.bot.label}, which this server does not run` : undefined}
            >
              can&rsquo;t continue here
            </span>
          )}
        </div>
        {yours && (
          <div className="row-abandon">
            <Abandon table={t} onDone={reread} onError={setError} />
          </div>
        )}
      </li>
    );
  };

  const mine = lists.mine ? unique([...lists.mine.games, ...(paged?.games ?? [])]) : [];
  const open = lists.open ? lists.open.games.filter(notMine) : [];
  /** The `playing` scope, split (R-6.5c2.22): it lists finished games too,
   *  because "over" is not a table STATUS (`tables.rs`), so Being played takes
   *  the games still under way and Finished the rest — in the listing's own
   *  order, which is newest first. */
  const playing = lists.playing ? lists.playing.games.filter(notMine).filter((t) => !t.over) : [];
  const finished = lists.playing ? lists.playing.games.filter(notMine).filter((t) => t.over) : [];
  const nextCursor = paged ? paged.next : (lists.mine?.next ?? null);
  /** A section's own refusal, drawn under its heading (dossier row 4). */
  const strip = (s: Scope) => {
    const refused = refusals[s];
    return refused !== null && <Refusal sentence={refused} />;
  };
  /** Whether this deployment answers the house chat (R-6.4.6): the rail, or
   *  one column with no rail at all. */
  const rail = lobbyChat();

  /** The way back to the games on this device (R-6.5b2.1): in the band's corner
   *  while nobody is signed in — the states a player most needs it in are the
   *  ones where nothing else here is pressable — and beside New table once
   *  somebody is. An ANCHOR (R-14); its arrow is decoration,
   *  so its NAME is the destination. */
  const toDevice = (
    <a className="to-door" href={hashOf({ kind: "door" })}>
      <span aria-hidden="true">&larr; </span>Games on this device
    </a>
  );

  return (
    <div className="lobby">
      <LobbyBand crumbs={["Lobby"]} corner={corner ?? (me === null ? toDevice : null)} />
      <div className="lobby-body">
        {notice && <Refusal sentence={notice} />}

        {/* **The part of this screen the SERVER owns** (M6.5b, spec §2.5;
            M6.5c2, spec §2.1). The door's states are `Door`'s and `App` hands
            them in when nobody is signed in; a signed-in lobby whose health read
            failed is state C here; otherwise the actions and the lists. The
            listing keeps trying behind C, and one landed read brings the lists
            back (`onReached`). */}
        {door ? (
          <div className="lobby-door">{door}</div>
        ) : serverUp === false ? (
          <div className="lobby-door">
            <Unreachable onRetry={onRetry ?? (() => {})} />
          </div>
        ) : (
          <>
            <div className="lobby-actions">
              <button type="button" className="primary" onClick={() => navigate({ kind: "new" })}>
                New table
              </button>
              {toDevice}
              {link !== "live" && (
                <span className="meta link-word" role="status">
                  {link}&hellip;
                </span>
              )}
            </div>

            <div className={rail ? "lobby-cols" : "lobby-cols no-rail"}>
              <div className="lobby-lists">
                {/* **`your-games` is a NAME and not decoration**: the browser
                    walks scope their reads by these three classes, and a
                    heading's words may change (M6.5c2 renamed two) while the
                    class does not. */}
                <section className="lobby-section your-games">
                  <header className="section-head">
                    <h3>Your tables</h3>
                  </header>
                  {strip("mine")}
                  {lists.mine === null ? (
                    // The lobby's own wait (the header comment). A read that
                    // REFUSED says so in its strip instead, never
                    // "loading..." over it.
                    error === null && refusals.mine === null && (
                      <>
                        <p className="muted">loading&hellip;</p>
                        <div className="skeleton-rows" aria-hidden="true">
                          <div />
                          <div />
                        </div>
                      </>
                    )
                  ) : mine.length === 0 ? (
                    <p className="muted">none yet &mdash; open a table, or take a seat at one below.</p>
                  ) : (
                    <ul className="lobby-rows">{mine.map((t) => row(t, "mine"))}</ul>
                  )}
                  {nextCursor && (
                    <button type="button" className="ghost more" onClick={() => void page(nextCursor)}>
                      more
                    </button>
                  )}
                </section>

                {/* Every open table on the server that is not already yours
                    (R-2.7): a join link is an address, and this is the other
                    way in. */}
                <section className="lobby-section open-tables">
                  <header className="section-head">
                    <h3>Open tables</h3>
                    <span className="section-hint">take a free seat</span>
                  </header>
                  {strip("open")}
                  {lists.open === null ? null : open.length === 0 ? (
                    <p className="muted">no table is waiting for players.</p>
                  ) : (
                    <ul className="lobby-rows">{open.map((t) => row(t, "open"))}</ul>
                  )}
                </section>

                {/* Every game still being played that is not yours, so a
                    spectator can find one without a link (R-3.9). A row opens
                    `#/t/<id>` as the other two do, and what that lands on is
                    the spectator screen, or at a locked table the face that
                    asks for its password, which the padlock on Spectate is
                    warning about. No Abandon: that is the creator's,
                    seated, under Your tables (`mayAbandon`). */}
                <section className="lobby-section playing-tables">
                  <header className="section-head">
                    <h3>Being played</h3>
                    <span className="section-hint">spectate a game</span>
                  </header>
                  {strip("playing")}
                  {lists.playing === null ? null : playing.length === 0 ? (
                    <p className="muted">nothing is being played right now.</p>
                  ) : (
                    <ul className="lobby-rows">{playing.map((t) => row(t, "playing"))}</ul>
                  )}
                </section>

                {/* **The finished games that are not yours, FOLDED** (R-6.5c2.22,
                    R-C6). Folded, because a table's lifetime is each host's
                    rule and the NAS sets none (R-6.5c2.2): there this list
                    would grow without end, and drawn open it would make every
                    visit to the lobby a longer scroll. Absent rather than
                    empty while there is none, because an empty fold is
                    clutter. A row is the other lists' row with View on it,
                    and opens the address Spectate did, so a locked table's
                    padlock and unlock face are unchanged. Your own finished
                    games stay under Your tables. */}
                {finished.length > 0 && (
                  <section className="lobby-section finished-tables">
                    <details className="fold">
                      <summary>
                        <h3>Finished</h3>
                        <span className="section-hint">
                          {finished.length === 1 ? "1 game" : `${finished.length} games`}
                        </span>
                      </summary>
                      <ul className="lobby-rows">{finished.map((t) => row(t, "finished"))}</ul>
                    </details>
                  </section>
                )}
              </div>
              {/* The house chat (R-6.4.5, R-6.4.6), in the RAIL: folded by
                  default (opening it is entering the lobby chat) and absent
                  when the server says the lobby chat is off, because a panel
                  whose every request is a 404 is worse than no panel. The rail
                  is this screen's and the panel keeps its own root. Its count
                  is the house frame's own (`house`). */}
              {rail && (
                <section className="lobby-rail lobby-chat">
                  <Chat scope={{ kind: "house" }} count={house?.chat} rev={house?.rev} sendName heading="h3" />
                </section>
              )}
            </div>
            </>
        )}

        {/* The lobby's own strip: a reading where EVERY listing was refused
            (one refused listing is said in its own section, `strip`), the
            pager's, a copy the browser refused, and Abandon's. **Not a second "Can't reach the server"
            under state C's card**, which says it already. */}
        {error && !(serverUp === false && error === UNREACHED) && <Refusal sentence={error} />}
      </div>
    </div>
  );
}
