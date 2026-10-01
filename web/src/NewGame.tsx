// Setup (spec §5.3): who sits where, and what the table is called.
//
// **A FORM, and nothing else now.** The deal left it at M6.1 — a create opens a
// DEALING table and each seat fetches its own two cards from `GET …/deal` — and
// M6.2 took the saved-tables list with it: the lobby is the first screen and
// lists both your games and the open ones, so a second list here would be the
// same tables read two ways.
//
// What is left is the question a create actually asks: how many seats, who holds
// each one, what the table is called, what YOU are called at it, whether it
// wants a password, and — for a fixture or a diagnostic — which seed.

import { useRef, useState, type ReactNode } from "react";
import {
  api,
  ApiError,
  displayName as storedName,
  LOCAL_USER,
  setDisplayName,
  type NewGameRequest,
  type SeatRequest,
} from "./api";
import { seatClass } from "./Cards";
import type { PlayerColor, TableView } from "./types";

const COLORS = ["Red", "Blue", "Yellow", "Green"] as const;

/** One row of the seat list, as the form holds it before it is a `SeatRequest`.
 *
 *  `email` keeps its typed text even while the row is switched to `me`, so a
 *  player who flips a row back and forth does not lose what they typed. */
type SeatRow = { kind: "me" | "email" | "bot" | "open"; email: string };

/** The default seating for a player count.
 *
 *  **The bot sits at seat 1, and only at two players (R-P10).** The server
 *  refuses a bot at three and four — "the bot is measured at two players only" —
 *  so the form does not offer one there.
 *
 *  **Above two players the other seats default to OPEN** (spec §5.3). Until
 *  M6.2 they defaulted to the person filling the form in, which made a hot seat
 *  the only 3P and 4P table this client could open; a room is people, and the
 *  seats they have not arrived at yet are open ones. A player who really is
 *  running a hot seat sets the rows to `me`, which is one control away. */
const defaultSeats = (players: number): SeatRow[] =>
  Array.from({ length: players }, (_, i) => ({
    kind: i === 0 ? ("me" as const) : players === 2 ? ("bot" as const) : ("open" as const),
    email: "",
  }));

export function NewGame({
  me,
  onCreated,
  who,
  local,
}: {
  /** Who this browser plays as, from the DOOR.
   *
   *  **Not from storage** (M6.1 Task 7's review). `api.ts` holds a claimed name
   *  and attaches it as a header; what that header is WORTH is the server's
   *  answer, and `/api/me` is the only thing that knows it. A form that read
   *  storage would seat a name the server may not answer to — and under
   *  Cloudflare Access or the edge Worker there is nothing in storage at all,
   *  so it would post an empty seat.
   *
   *  The DISPLAY name below is read from storage, and that is not the same
   *  thing: it is a label on a seat, not a claim about who is asking. */
  me: string;
  /** The table was opened. `App` takes it from here — the reply's `status` says
   *  whether it is open or dealing, and `open` is what asks.
   *
   *  It may return a promise and is AWAITED inside `run`: `App`'s `open` is two
   *  requests, and a refusal from either belongs in this screen's error line
   *  rather than in an unhandled rejection. */
  onCreated: (table: TableView) => void | Promise<void>;
  /** The "You are …" strip and its sign-out button, built by `App` — a node
   *  rather than an email and a callback, so there is one statement of the strip
   *  and one wiring of the button. */
  who?: ReactNode;
  /** Open the table in THIS BROWSER (M6.5b, R-6.5b.8; spec §2.5).
   *
   *  The same form with everything a local table cannot be asked taken out:
   *  two seats, you and the bot, no password (there is nobody to keep out),
   *  and **no seed — never offered and never sent**, because `draw_seed`
   *  inside the module draws it and the board is an injective fingerprint of
   *  it. The submit goes to the Worker; `me` is not what seats it either, for
   *  the reason `LOCAL_USER` exists: the module answers as one fixed address
   *  whatever door this browser signed in at. */
  local?: boolean;
}) {
  const [players, setPlayers] = useState(2);
  const [seats, setSeats] = useState<SeatRow[]>(() => defaultSeats(2));
  const [seedText, setSeedText] = useState("");
  const [name, setName] = useState("");
  /** What this browser calls itself, prefilled from the last table it opened.
   *
   *  Read ONCE, in the initializer: a value re-read on every render would
   *  overwrite what the player is in the middle of typing. */
  const [myName, setMyName] = useState(() => storedName() ?? "");
  const [locked, setLocked] = useState(false);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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

  /** The seat list §3.1 reads: one entry per seat, in seat order, each exactly
   *  one of `user` / `bot: true` / `open: true`.
   *
   *  **Normalized here, because the server normalizes too (R-1.1).** A table
   *  seated under `blue@wire.test` does not answer to `Blue@Wire.test`, and the
   *  person at that address would be told they hold no seat. */
  const seatRequests = (): SeatRequest[] => {
    // **A local table is seated by the MODULE's own address** (R-6.5b.1), not
    // by whoever this browser signed in at and not by nobody: `dispatch` is
    // handed `local@browser` for every local call, and a create naming any
    // other address would open a table whose own player holds no seat.
    if (local) return [{ user: LOCAL_USER }, { bot: true }];
    return seats.map((row) => {
      if (row.kind === "bot") return { bot: true };
      if (row.kind === "open") return { open: true };
      return { user: row.kind === "me" ? me : row.email.trim().toLowerCase() };
    });
  };

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
    // **A local create never carries one**, whatever is in the field (there is
    // none on that form): `draw_seed` inside the module draws it and it is
    // never typed — spec §2.2.
    ...(!local && seedText.trim() ? { seed: seedText.trim() } : {}),
    ...(name.trim() ? { name: name.trim() } : {}),
    ...(myName.trim() ? { display_name: myName.trim() } : {}),
    // **Only when the box is ticked AND something was typed.** A password field
    // left blank under a ticked box is a table nobody can join rather than a
    // table anybody can — so the absent field is the honest thing to send, and
    // the table opens unlocked.
    // A local table has nobody to keep out, so the control is not drawn and
    // the field cannot ride even if something were left in it.
    ...(!local && locked && password.trim() ? { password: password.trim() } : {}),
  });

  const setRow = (i: number, patch: Partial<SeatRow>) =>
    setSeats((rows) => rows.map((r, n) => (n === i ? { ...r, ...patch } : r)));

  /** What the button says it will do, and the two things differ.
   *
   *  A table with an open seat is not dealt when it is created — it waits for
   *  somebody to sit down — so a button reading "Deal" would promise the set
   *  table and hand over a waiting room. */
  const opensSeats = seats.some((r) => r.kind === "open");

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
          field they never touched.

          **A local table is not asked any of it** (R-6.5b.8): it is you and
          the bot, in this browser, and the sentence says where the game lives
          because that is the one thing about it a player cannot see. */}
      <p className="lede">
        {local
          ? "You and the bot, played in this browser: the game is kept on this device and never reaches the server unless you send it there."
          : "Seat one bot at two players; at three and four everyone is a person — the bot is measured at two players only."}
      </p>

      {local ? (
        // The seating is not a choice here, so it is not a control: two seats,
        // in seat order, with the colours the server's own default gives them.
        seats.map((row, i) => (
          <div className="field seat-row" key={i}>
            <span className="meta">Seat {i}</span>
            <span>{row.kind === "bot" ? "the bot" : "you"}</span>
            <span className={`meta ${seatClass(colors[i])}`}>{colors[i]}</span>
          </div>
        ))
      ) : (
        <>
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
                <option value="open">open seat</option>
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
        </>
      )}

      {/* **Free text, and never an id** (R-2.4). The server draws the storage id
          and a client-chosen one is a name two people can collide on; this is
          only what the lobby prints. */}
      <div className="field">
        <label htmlFor="name">Name</label>
        <input
          id="name"
          value={name}
          placeholder="blank — listed by its id"
          onChange={(e) => setName(e.target.value)}
        />
      </div>

      {/* What YOU are called at this table, written onto every seat you hold and
          onto no other — a name is a claim about the person who gave it. Kept in
          this browser so the next table does not ask again. */}
      <div className="field">
        <label htmlFor="display-name">Your name</label>
        <input
          id="display-name"
          value={myName}
          placeholder="blank — your email is shown"
          onChange={(e) => setMyName(e.target.value)}
        />
      </div>

      {/* **The checkbox carries the row's only label**, so the field is asked
          about by one name rather than two. A "Password" caption beside it would
          be a second label on the same control — the checkbox's accessible name
          would read as both — and the field it reveals is then labelled by
          something it is not.

          **Neither this nor the seed is drawn for a local table**: there is
          nobody to keep out of a game in your own browser, and the seed is the
          module's to draw (spec §2.2, §2.5). */}
      {!local && (
        <div className="field password-field">
          <label className="check" htmlFor="lock">
            <input
              id="lock"
              type="checkbox"
              checked={locked}
              onChange={(e) => {
                setLocked(e.target.checked);
                // Cleared with the box, so an unticked form cannot carry a word
                // the player can no longer see.
                if (!e.target.checked) setPassword("");
              }}
            />{" "}
            Ask for a password to join
          </label>
          {locked && (
            <input
              id="password"
              type="password"
              aria-label="Table password"
              placeholder="the word people join with"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          )}
        </div>
      )}

      {!local && (
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
      )}

      <button
        type="button"
        className="primary"
        disabled={busy}
        onClick={() =>
          run(async () => {
            // Remembered on SUBMIT, not on every keystroke: a name half typed
            // and then abandoned is not what this browser is called.
            setDisplayName(myName.trim());
            // **The host is the caller's INTENT**, which is what `create`'s own
            // `host` argument is for (R-6.5b.12): a create carries no id yet,
            // so nothing in the path could route it.
            await onCreated(await api.create(body(), local ? "local" : undefined));
          })
        }
      >
        {opensSeats ? "Open table" : "Deal"}
      </button>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
