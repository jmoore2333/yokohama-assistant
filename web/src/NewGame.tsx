// Setup (M6.1 §7.3): who sits where, and the tables you already sit at.
//
// **A FORM again, and the deal has left it.** Until this cycle this screen drew
// the set table itself: it asked `POST …/preview` for a throwaway position with
// every seat's two cards on it, sequenced a hot-seat pass between the human
// seats, and sent the whole thing back as a create carrying a `keeps` array. A
// create now OPENS A TABLE and starts nothing (§4.2): the reply is
// `status: "dealing"`, each seat fetches its own two cards from `GET …/deal`,
// and the sequence a person holding two seats walks is the server's answer after
// each keep. The set table moved to `Dealing.tsx` with it, along with
// `DealBacks`, the Keep-score hedge and the `turn`/`passing`/"Next seat" state
// that made the pass.
//
// So what is left here is the question a create actually asks: how many seats,
// who holds each one, and — for a fixture or a diagnostic — which seed.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { api, ApiError, type NewGameRequest, type SeatRequest } from "./api";
import { seatClass } from "./Cards";
import { TableFacts } from "./TableFacts";
import type { PlayerColor, TableView } from "./types";

const COLORS = ["Red", "Blue", "Yellow", "Green"] as const;

/** One row of the seat list, as the form holds it before it is a `SeatRequest`.
 *
 *  `email` keeps its typed text even while the row is switched to `me`, so a
 *  player who flips a row back and forth does not lose what they typed. */
type SeatRow = { kind: "me" | "email" | "bot"; email: string };

/** The default seating for a player count.
 *
 *  **The bot sits at seat 1, and only at two players (R-P10).** The server
 *  refuses a bot at three and four — "the bot is measured at two players only" —
 *  so the form does not offer one there and every seat defaults to the person
 *  filling the form in, which is the hot-seat table this client has always
 *  supported. */
const defaultSeats = (players: number): SeatRow[] =>
  Array.from({ length: players }, (_, i) => ({
    kind: players === 2 && i === 1 ? ("bot" as const) : ("me" as const),
    email: "",
  }));

export function NewGame({
  me,
  onCreated,
  onOpen,
  who,
}: {
  /** Who this browser plays as, from the DOOR.
   *
   *  **Not from storage** (Task 7's review, `NewGame.tsx:179`). `api.ts` holds a
   *  claimed name and attaches it as a header; what that header is WORTH is the
   *  server's answer, and `/api/me` is the only thing that knows it. A form that
   *  read storage would seat a name the server may not answer to — and under
   *  Cloudflare Access or the edge Worker there is nothing in storage at all,
   *  so it would post an empty seat. */
  me: string;
  /** The table was opened, and is dealing. `App` takes it from here.
   *
   *  Both callbacks may return a promise, and both are AWAITED inside `run`:
   *  `App`'s `open` is two requests, and a refusal from either belongs in this
   *  screen's error line rather than in an unhandled rejection. */
  onCreated: (table: TableView) => void | Promise<void>;
  /** A saved table was clicked. The ID only: whether it is still dealing is a
   *  `GET …/table` away, and `App` is what asks. */
  onOpen: (id: string) => void | Promise<void>;
  /** The "You are …" strip and its sign-out button, built by `App` — a node
   *  rather than an email and a callback, so there is one statement of the strip
   *  and one wiring of the button. */
  who?: ReactNode;
}) {
  const [players, setPlayers] = useState(2);
  const [seats, setSeats] = useState<SeatRow[]>(() => defaultSeats(2));
  const [seedText, setSeedText] = useState("");
  const [name, setName] = useState("");
  const [saved, setSaved] = useState<TableView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  /** The saved game whose `delete` has been clicked once — see the row below. */
  const [confirming, setConfirming] = useState<string | null>(null);

  /** The tables this player sits at, scoped to them by the server (M6.1).
   *
   *  **The whole table, not just its id**: a row says what the table is called,
   *  how many seats it has and what it is waiting on, which is what makes a list
   *  of three games something a player can read. */
  const savedTables = async () => (await api.list()).games;

  useEffect(() => {
    savedTables().then(setSaved).catch(() => setSaved([]));
    // `savedTables` is re-created every render and this must run once, so it is
    // deliberately not a dependency.
  }, []);

  // The same guard, and the same reasoning, as `App`'s `inFlight`.
  //
  // `busy` STATE could not hold it: React batches, so two clicks in the same
  // tick both read `busy === false` and both post. Here that is two `POST
  // /api/games`, and the two tables are genuinely different — the server draws a
  // fresh bot seed per table — so the loser's document lands on disk and the
  // client is silently switched onto the other one.
  const inFlight = useRef(false);

  const run = async (fn: () => Promise<void>) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  };

  const colors = COLORS.slice(0, players) as PlayerColor[];

  /** The seat list §4.2 reads: one entry per seat, in seat order, each exactly
   *  one of `user` / `bot: true`.
   *
   *  **Normalized here, because the server normalizes too (R-1.1).** A table
   *  seated under `blue@wire.test` does not answer to `Blue@Wire.test`, and the
   *  person at that address would be told they hold no seat. */
  const seatRequests = (): SeatRequest[] =>
    seats.map((row) =>
      row.kind === "bot"
        ? { bot: true }
        : { user: row.kind === "me" ? me : row.email.trim().toLowerCase() },
    );

  const body = (): NewGameRequest => ({
    players,
    colors: [...colors],
    start_seat: 0,
    seats: seatRequests(),
    // Blank means "the server draws one", which is the right default: the board
    // is an injective fingerprint of the setup seed, so a small or sequential
    // seed hands over the hidden set. The value is a STRING from the keystroke
    // to the wire — a `u64` does not survive a JS number, and a rounded seed
    // deals a different game from move one.
    ...(seedText.trim() ? { seed: seedText.trim() } : {}),
    ...(name.trim() ? { name: name.trim() } : {}),
  });

  const setRow = (i: number, patch: Partial<SeatRow>) =>
    setSeats((rows) => rows.map((r, n) => (n === i ? { ...r, ...patch } : r)));

  return (
    <div className="setup">
      {/* This screen carries no banner, so the strip gets a row of its own —
          the same node the dealing screen puts in its banner and the game screen
          puts in its own, so there is one place a player reads who they are. */}
      {who && <div className="who-strip">{who}</div>}
      <h2>New game</h2>
      {/* **R-P10, said where the choice is made.** The server refuses a bot at
          three and four players by name; a form that only let the player find
          that out as a 400 after filling it in would be telling them about a
          field they never touched. */}
      <p className="lede">
        Seat one bot at two players; at three and four everyone is a person &mdash; the bot is
        measured at two players only.
      </p>

      <div className="field">
        <label htmlFor="players">Players</label>
        <select
          id="players"
          value={players}
          onChange={(e) => {
            const n = Number(e.target.value);
            setPlayers(n);
            // **Rebuilt, not resized.** The bot is legal at two players only, so
            // a seat list carried across a count change would either strand a
            // bot at three players (which the server refuses) or leave two
            // players with no bot and no way to ask for one.
            setSeats(defaultSeats(n));
          }}
        >
          {[2, 3, 4].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </div>

      {seats.map((row, i) => (
        <div className="field seat-row" key={i}>
          <label htmlFor={`seat-${i}`}>Seat {i}</label>
          <select
            id={`seat-${i}`}
            value={row.kind}
            onChange={(e) => setRow(i, { kind: e.target.value as SeatRow["kind"] })}
          >
            <option value="me">me</option>
            <option value="email">another email</option>
            {players === 2 && <option value="bot">bot</option>}
          </select>
          <span className={`meta ${seatClass(colors[i])}`}>{colors[i]}</span>
          {row.kind === "email" && (
            <input
              id={`seat-${i}-email`}
              aria-label={`Seat ${i} email`}
              value={row.email}
              inputMode="email"
              placeholder="someone@example.com"
              onChange={(e) => setRow(i, { email: e.target.value })}
            />
          )}
        </div>
      ))}

      <div className="field">
        <label htmlFor="name">Name</label>
        <input
          id="name"
          value={name}
          placeholder="blank — the table is listed by its id"
          onChange={(e) => setName(e.target.value)}
        />
      </div>

      <div className="field">
        <label htmlFor="seed">Seed</label>
        <input
          id="seed"
          value={seedText}
          inputMode="numeric"
          placeholder="blank — the server draws one"
          onChange={(e) => setSeedText(e.target.value)}
        />
      </div>

      <button
        type="button"
        className="primary"
        disabled={busy}
        onClick={() =>
          run(async () => {
            await onCreated(await api.create(body()));
          })
        }
      >
        Deal
      </button>

      {/* Every move is persisted as its `GameRecord` after it is applied, and a
          game whose handle is gone is rebuilt from that record — so resuming one
          is a `GET`, not a restore. */}
      <section className="saved">
        <h3>Saved games</h3>
        {saved.length === 0 ? (
          <p className="muted">none yet — a game appears here as soon as it starts.</p>
        ) : (
          <ul>
            {saved.map((t) => {
              // The id is what every REQUEST is keyed by, and the name is only
              // ever what is shown. Bound once per row so the two cannot be
              // confused below: `delete ${name}` beside `api.remove(id)` is how
              // a row deletes a game other than the one it names.
              const name = t.name ?? t.id;
              return (
                <li key={t.id}>
                  {/* **The id goes up, not the game.** A table that is still
                      dealing refuses every session route with a 409, so a row
                      that fetched `GET /api/games/{id}` here would show a
                      refusal for exactly the tables this cycle exists to open.
                      `App` asks `GET …/table` first and picks the screen. */}
                  <button
                    type="button"
                    className="ghost"
                    disabled={busy}
                    onClick={() =>
                      run(async () => {
                        await onOpen(t.id);
                      })
                    }
                  >
                    {name}
                  </button>
                  <TableFacts table={t} />
                  {/* **Two clicks, and the first one asks (M5 final review I2).**
                      Deleting a record is not undoable — the game is gone — and
                      M5 put this list one banner menu away from a running game,
                      so a stray click beside the game you meant to resume used
                      to remove a different one.

                      A row that asks, rather than `window.confirm`: a native
                      dialog blocks the browser walk this client is tested with,
                      and the question belongs where the mistake is made. Both
                      buttons NAME the game, so the confirmation cannot be about
                      a row other than the one that opened it. */}
                  {confirming === t.id ? (
                    <>
                      <button
                        type="button"
                        className="ghost danger"
                        disabled={busy}
                        aria-label={`confirm delete ${name}`}
                        onClick={() =>
                          run(async () => {
                            await api.remove(t.id);
                            setConfirming(null);
                            setSaved(await savedTables());
                          })
                        }
                      >
                        confirm delete
                      </button>
                      <button
                        type="button"
                        className="ghost"
                        disabled={busy}
                        aria-label={`cancel deleting ${name}`}
                        onClick={() => setConfirming(null)}
                      >
                        cancel
                      </button>
                    </>
                  ) : (
                    <button
                      type="button"
                      className="ghost danger"
                      disabled={busy}
                      aria-label={`delete ${name}`}
                      onClick={() => setConfirming(t.id)}
                    >
                      delete
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
