// Setup (spec §5.3; M6.5c2 spec §2.2, R-6.5c2.8, mock 07): who sits where, who
// plays first, what the table is called, whether it wants a password — and,
// folded away for a fixture or a diagnostic, which seed.
//
// **TWO forms in one file, and they share only what is truly one thing.** The
// table form (`#/new`) asks everything a room asks; the form for a game on this
// device (`#/new/local`) asks only what a game against the bot can be asked —
// its name and who starts. They share the segmented control, the submit's guard
// and the create call.
//
// **"Your name" left both forms** (R-6.5c2.8): the lobby's corner owns it, and
// what a create sends as `display_name` is the name this browser already keeps,
// read when the form is sent. The form is about the table.

import { useRef, useState, type ReactNode } from "react";
import {
  api,
  ApiError,
  displayName,
  LOCAL_USER,
  useIdentityMode,
  type NewGameRequest,
  type SeatRequest,
} from "./api";
import { LobbyBand } from "./Band";
import { seatClass } from "./Cards";
import { VersionTag } from "./Chips";
import { normalizeName } from "./nameRule";
import { Refusal } from "./Refusal";
import { checkSeed, trimSeed } from "./seedRule";
import type { PlayerColor, TableView } from "./types";
import productionBot from "./wire/production_bot.json";

/** The seat colours, in the ENGINE's order. */
const COLORS = ["Red", "Blue", "Yellow", "Green"] as const;

/** The bot this build deals, by its short label (R-6.5c2.10) — read at BUILD
 *  time from the contract file `wire_fixtures` emits and refuses when stale, so
 *  the form asks the server nothing and states no second truth (spec §2.4). */
const BOT_LABEL = productionBot.label;

/** One seat row as the form holds it before it is a `SeatRequest`. `person`
 *  keeps its typed text while the row is switched away, so flipping a row back
 *  and forth loses nothing. */
type SeatRow = { kind: "me" | "person" | "bot" | "open"; person: string };

/** Who plays first: a seat, or a draw made when the form is sent. */
type First = number | "random";

/** The default seating for a count. **The bot sits at seat 1, and only at two
 *  players (R-P10)**; above two the other seats default to OPEN (spec §5.3). */
const defaultSeats = (players: number): SeatRow[] =>
  Array.from({ length: players }, (_, i) => ({
    kind: i === 0 ? ("me" as const) : players === 2 ? ("bot" as const) : ("open" as const),
    person: "",
  }));

/** A seat drawn over `n` from the browser's own generator, at the moment the
 *  form is SENT (R-6.5c2.8). **Rejection sampling**, so every seat is equally
 *  likely: a byte taken modulo three would favour seat 0 by one part in 85, so
 *  a byte at or past the largest multiple of `n` is drawn again. */
export function drawSeat(n: number): number {
  const byte = new Uint8Array(1);
  const limit = 256 - (256 % n);
  for (;;) {
    crypto.getRandomValues(byte);
    if (byte[0]! < limit) return byte[0]! % n;
  }
}

/** A few choices, one of them on: radio buttons drawn as segments (mock 07), so
 *  the arrow keys walk the group and a screen reader says "2 of 3" for free. */
function Segmented<T extends number | string>({
  legend,
  name,
  options,
  value,
  onChange,
}: {
  legend: string;
  name: string;
  options: readonly { value: T; label: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <fieldset className="form-field">
      <legend>{legend}</legend>
      <div className="seg">
        {options.map((o) => (
          <label key={String(o.value)} className={o.value === value ? "on" : undefined}>
            <input
              type="radio"
              name={name}
              checked={o.value === value}
              onChange={() => onChange(o.value)}
            />
            {o.label}
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** The submit's guard, for both forms. **A ref decides whether the request
 *  happens; the state only disables the button** — React batches, so two clicks
 *  in one tick both read a `busy` STATE of false, and two `POST /api/games` are
 *  two different tables. */
function useSubmit() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
  return { busy, error, run };
}

/** The name this browser keeps, trimmed, or nothing — read when a form is
 *  SENT, so a name set in the lobby's corner a moment ago is the one used. */
const keptName = (): Partial<Pick<NewGameRequest, "display_name">> => {
  const shown = displayName()?.trim();
  return shown ? { display_name: shown } : {};
};

export function NewGame({
  me,
  onCreated,
  onCancel,
  who,
  local,
}: {
  /** Who this browser plays as, from the DOOR — never from storage: what a
   *  claimed header is worth is the server's answer at `/api/me`. */
  me: string;
  /** The table was opened; `App` takes it from here. Awaited inside the
   *  submit, so a refusal from `App`'s own two requests lands in this form. */
  onCreated: (table: TableView) => void | Promise<void>;
  /** Back out without opening anything: the lobby for the table form, the door
   *  for the local one (mock 07's Cancel). */
  onCancel: () => void;
  /** The band's corner, built by `App` (the identity corner, Task 4). The
   *  local form draws none — it is a game on this device (mock 07). */
  who?: ReactNode;
  /** Open the table in THIS BROWSER (M6.5b, R-6.5b.8): two seats, you and the
   *  bot, no password and no seed — `draw_seed` inside the module draws it. */
  local?: boolean;
}) {
  return local ? (
    <LocalForm onCreated={onCreated} onCancel={onCancel} />
  ) : (
    <TableForm me={me} onCreated={onCreated} onCancel={onCancel} who={who} />
  );
}

function TableForm({
  me,
  onCreated,
  onCancel,
  who,
}: {
  me: string;
  onCreated: (table: TableView) => void | Promise<void>;
  onCancel: () => void;
  who?: ReactNode;
}) {
  const [players, setPlayers] = useState(2);
  const [seats, setSeats] = useState<SeatRow[]>(() => defaultSeats(2));
  const [first, setFirst] = useState<First>(0);
  const [name, setName] = useState("");
  const [locked, setLocked] = useState(false);
  const [password, setPassword] = useState("");
  const [seedText, setSeedText] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const { busy, error, run } = useSubmit();
  /** **A specific person is a NAME on an open door and an ADDRESS behind
   *  Access** (spec §2.1): the door's own name, from `/api/health`. */
  const byName = useIdentityMode() === "open";
  const colors = COLORS.slice(0, players) as PlayerColor[];

  /** Blank is "the server draws one" and is never sent; anything else is
   *  checked by the server's own rule as it is typed (spec §2.2). */
  const seedVerdict = trimSeed(seedText) === "" ? null : checkSeed(seedText);
  const seedError = seedVerdict !== null && !seedVerdict.ok ? seedVerdict.reason : null;

  /** Why a "specific person" row cannot be sent: `null` when it can, `""` when
   *  it is blank — held without a sentence, because nothing typed is not a
   *  mistake yet — and the key grammar's own reason otherwise (`nameRule.ts`). */
  const personError = (row: SeatRow): string | null => {
    if (row.kind !== "person") return null;
    if (row.person.trim() === "") return "";
    const n = normalizeName(row.person);
    return n.ok ? null : n.reason;
  };
  const held = seedError !== null || seats.some((r) => personError(r) !== null);

  /** **Rebuilt, not resized**: the bot is legal at two players only, so a seat
   *  list carried across a count change would strand a bot at three (R-P10).
   *  A first player the new count still has is kept. */
  const setCount = (n: number) => {
    setPlayers(n);
    setSeats(defaultSeats(n));
    setFirst((f) => (f === "random" || f < n ? f : 0));
  };
  const setRow = (i: number, patch: Partial<SeatRow>) =>
    setSeats((rows) => rows.map((r, n) => (n === i ? { ...r, ...patch } : r)));

  const body = (): NewGameRequest => {
    const seed = trimSeed(seedText);
    return {
      players,
      colors: [...colors],
      start_seat: first === "random" ? drawSeat(players) : first,
      // **Normalized here, because the server keys it so** (R-1.1; spec §2.1):
      // a seat bound under one spelling answers to that key alone.
      seats: seats.map((row): SeatRequest => {
        if (row.kind === "bot") return { bot: true };
        if (row.kind === "open") return { open: true };
        if (row.kind === "me") return { user: me };
        const n = normalizeName(row.person);
        return { user: n.ok ? n.key : row.person.trim() };
      }),
      // The seed is a STRING from the keystroke to the wire (`WireSeed`), and
      // blank is omitted: the board is an injective fingerprint of the seed, so
      // the server's own draw is the safe default.
      ...(seed ? { seed } : {}),
      ...(name.trim() ? { name: name.trim() } : {}),
      ...keptName(),
      // Only when ticked AND typed: a password field left blank under a ticked
      // box would lock a table nobody can join.
      ...(locked && password.trim() ? { password: password.trim() } : {}),
    };
  };

  /** A table with an open seat is not dealt when it is created, so "Deal" there
   *  would promise the set table and hand over a waiting room. */
  const opensSeats = seats.some((r) => r.kind === "open");

  return (
    <div className="band-screen">
      <LobbyBand crumbs={["Lobby", "New table"]} corner={who} />
      <div className="setup">
        {/* **Task 2's raised card**, and the title a level under the band's
            crumb: the band's last crumb is the page's `<h2>` and reads "New
            table" too (mock 07 draws both). */}
        <section className="raised-card form-card" aria-labelledby="new-table-title">
          <div className="card-band vp" aria-hidden="true" />
          <div className="card-in">
            <h3 id="new-table-title">New table</h3>

            {/* **Free text, and never an id** (R-2.4); the placeholder is the
                whole explanation (R-6.5c2.8: no subtext). */}
            <div className="form-field">
              <label htmlFor="name">Name</label>
              <input
                id="name"
                value={name}
                autoComplete="off"
                placeholder="ID shown in the lobby if left blank"
                onChange={(e) => setName(e.target.value)}
              />
            </div>

            <Segmented
              legend="Players"
              name="players"
              options={[2, 3, 4].map((n) => ({ value: n, label: String(n) }))}
              value={players}
              onChange={setCount}
            />

            <fieldset className="form-field">
              <legend>Seats</legend>
              {seats.map((row, i) => {
                const color = colors[i]!;
                const why = personError(row);
                return (
                  <div className="form-seat" key={color}>
                    <span className="color">
                      <span className={`seat-dot ${seatClass(color)}`} aria-hidden="true" />
                      {color}
                    </span>
                    <div className="pick">
                      <select
                        aria-label={`${color} seat`}
                        value={row.kind}
                        onChange={(e) => setRow(i, { kind: e.target.value as SeatRow["kind"] })}
                      >
                        <option value="me">You</option>
                        <option value="open">Open seat</option>
                        <option value="person">A specific person</option>
                        {players === 2 && <option value="bot">The bot</option>}
                      </select>
                      {row.kind === "bot" && <VersionTag label={BOT_LABEL} />}
                    </div>
                    {row.kind === "person" && (
                      <input
                        className="person"
                        aria-label={`${color} player's ${byName ? "name" : "address"}`}
                        type={byName ? "text" : "email"}
                        autoComplete="off"
                        placeholder={byName ? "their name" : "someone@example.com"}
                        value={row.person}
                        aria-invalid={why ? true : undefined}
                        aria-describedby={why ? `seat-${i}-error` : undefined}
                        onChange={(e) => setRow(i, { person: e.target.value })}
                      />
                    )}
                    {why ? (
                      <p className="field-error" id={`seat-${i}-error`}>
                        {why}
                      </p>
                    ) : null}
                  </div>
                );
              })}
              {/* R-P10 said where the choice is made, with the version the bot
                  plays as — shown, never chosen (R-6.5c2.10). */}
              <p className="seat-note">
                The bot <VersionTag label={BOT_LABEL} /> is offered at two players only: that is where
                it has been measured.
              </p>
            </fieldset>

            <Segmented<First>
              legend="First player"
              name="first"
              options={[
                ...colors.map((c, i) => ({ value: i as First, label: c })),
                { value: "random" as First, label: "Random" },
              ]}
              value={first}
              onChange={setFirst}
            />

            {/* **The checkbox carries the row's only label**, and the label
                says what the password guards: joining AND spectating. The box
                it reveals is asked about by its own name, with no placeholder
                (R-6.5c2.8). */}
            <div className="form-field">
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={locked}
                  onChange={(e) => {
                    setLocked(e.target.checked);
                    // Cleared with the box: an unticked form cannot carry a
                    // word the player can no longer see.
                    if (!e.target.checked) setPassword("");
                  }}
                />
                <span className="switch-track" aria-hidden="true" />
                Ask for a password to join or spectate
              </label>
              {locked && (
                <input
                  className="password"
                  type="password"
                  aria-label="Table password"
                  autoComplete="new-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              )}
            </div>

            {/* **Held open while the seed is refused**: a button held for a
                reason the player cannot see is a button that looks broken. */}
            <details
              className="fold"
              open={advanced || seedError !== null}
              onToggle={(e) => setAdvanced(e.currentTarget.open)}
            >
              <summary>Advanced</summary>
              <div className="form-field">
                <label htmlFor="seed">Seed</label>
                <input
                  id="seed"
                  value={seedText}
                  inputMode="numeric"
                  autoComplete="off"
                  spellCheck={false}
                  aria-invalid={seedError !== null ? true : undefined}
                  aria-describedby={seedError !== null ? "seed-error" : undefined}
                  onChange={(e) => setSeedText(e.target.value)}
                />
                {seedError !== null && (
                  <p className="field-error" id="seed-error">
                    {seedError}
                  </p>
                )}
              </div>
            </details>

            <div className="form-foot">
              <button type="button" className="quiet-link" onClick={onCancel}>
                Cancel
              </button>
              <button
                type="button"
                className="primary"
                disabled={busy || held}
                onClick={() =>
                  run(async () => {
                    await onCreated(await api.create(body()));
                  })
                }
              >
                {opensSeats ? "Open table" : "Deal"}
              </button>
            </div>
            {error && <Refusal sentence={error} />}
          </div>
        </section>
      </div>
    </div>
  );
}

function LocalForm({
  onCreated,
  onCancel,
}: {
  onCreated: (table: TableView) => void | Promise<void>;
  onCancel: () => void;
}) {
  const [name, setName] = useState("");
  const [first, setFirst] = useState<First>(0);
  const { busy, error, run } = useSubmit();

  const body = (): NewGameRequest => ({
    players: 2,
    colors: ["Red", "Blue"],
    start_seat: first === "random" ? drawSeat(2) : first,
    // **Seated by the MODULE's own address** (R-6.5b.1): `dispatch` answers as
    // `local@browser` for every local call, so a create naming anyone else
    // would open a table whose own player holds no seat. **No seed, ever**:
    // `draw_seed` inside the module draws it (spec §2.2 of M6.5b).
    seats: [{ user: LOCAL_USER }, { bot: true }],
    ...(name.trim() ? { name: name.trim() } : {}),
    ...keptName(),
  });

  return (
    <div className="band-screen">
      <LobbyBand crumbs={["On this device"]} corner={undefined} />
      <div className="setup">
        <section className="raised-card form-card local" aria-labelledby="new-local-title">
          <div className="card-band yen" aria-hidden="true" />
          <div className="card-in">
            {/* The tag beside the heading and not inside it, so the heading's
                name is the sentence and the version is a fact next to it. */}
            <div className="title-row">
              <h3 id="new-local-title">New game against the bot</h3>
              <VersionTag label={BOT_LABEL} />
            </div>
            <p className="lede">Two players: you and the bot.</p>

            <div className="form-field">
              <label htmlFor="name">Name</label>
              <input
                id="name"
                value={name}
                autoComplete="off"
                placeholder="Optional"
                onChange={(e) => setName(e.target.value)}
              />
            </div>

            {/* The seating is not a choice here, so it is not a control. */}
            <div className="form-field">
              <span className="field-label" id="local-seats">
                Seats
              </span>
              <ul className="form-seats" aria-labelledby="local-seats">
                <li className="form-seat">
                  <span className="color">
                    <span className={`seat-dot ${seatClass("Red")}`} aria-hidden="true" />
                    Red
                  </span>
                  <span>You</span>
                </li>
                <li className="form-seat">
                  <span className="color">
                    <span className={`seat-dot ${seatClass("Blue")}`} aria-hidden="true" />
                    Blue
                  </span>
                  <span>
                    The bot <VersionTag label={BOT_LABEL} />
                  </span>
                </li>
              </ul>
            </div>

            <Segmented<First>
              legend="First player"
              name="first"
              options={[
                { value: 0, label: "You" },
                { value: 1, label: "The bot" },
                { value: "random", label: "Random" },
              ]}
              value={first}
              onChange={setFirst}
            />

            <div className="form-foot">
              <button type="button" className="quiet-link" onClick={onCancel}>
                Cancel
              </button>
              <button
                type="button"
                className="primary yen"
                disabled={busy}
                onClick={() =>
                  run(async () => {
                    // **The host is the caller's INTENT** (R-6.5b.12): a create
                    // carries no id yet, so nothing in the path could route it.
                    await onCreated(await api.create(body(), "local"));
                  })
                }
              >
                Deal
              </button>
            </div>
            {error && <Refusal sentence={error} />}
          </div>
        </section>
      </div>
    </div>
  );
}
