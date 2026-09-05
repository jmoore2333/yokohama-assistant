import { useEffect, useRef, useState } from "react";
import { api, ApiError, type NewGameRequest } from "./api";
import { OrderCardView } from "./Cards";
import type { ArtManifest, ClientState, Preview } from "./types";

const COLORS = ["Red", "Blue", "Yellow", "Green"] as const;

/** Setup: the sim tail of the CLI's wizard, in two steps, plus the saved games.
 *
 * The deal preview is a real request rather than a guess — `new_game` is
 * deterministic from the seed, so the cards shown here ARE the cards dealt.
 * The seed is carried as a STRING throughout: it is a `u64`, a JS number cannot
 * hold one, and this screen round-trips it from preview to create. Rounding it
 * would deal a different game than the one on screen. */
export function NewGame({
  art,
  onStarted,
}: {
  art: ArtManifest | null;
  /** `opening` is true for a game just CREATED and false for one reopened —
   *  the difference between events that have just happened (a bot start seat
   *  has already played by the time `create` replies) and events that are
   *  history. `App` walks the first and not the second. */
  onStarted: (state: ClientState, opening: boolean) => void;
}) {
  const [players, setPlayers] = useState(2);
  const [seedText, setSeedText] = useState("");
  const [botSeat, setBotSeat] = useState(1);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [keeps, setKeeps] = useState<number[]>([]);
  const [saved, setSaved] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  /** The saved game whose `delete` has been clicked once — see the row below. */
  const [confirming, setConfirming] = useState<string | null>(null);

  useEffect(() => {
    api.list().then(setSaved).catch(() => setSaved([]));
  }, []);

  const colors = COLORS.slice(0, players);
  const base = (): NewGameRequest => ({
    players,
    colors: [...colors],
    start_seat: 0,
    ...(seedText.trim() ? { seed: seedText.trim() } : {}),
    keeps: forSeats(),
    bot_seats: [botSeat],
  });

  /** The `keeps` list the wire takes: a number on a human seat, `null` on the
   *  bot's, whose keep is the bot's own. A value on a bot seat is a 400 naming
   *  the seat. `chosen` is this screen's per-seat state, which still holds a
   *  number for every seat; only the bot's entry is dropped on the way out. */
  const forSeats = (chosen?: number[]): (number | null)[] =>
    Array.from({ length: players }, (_, seat) =>
      seat === botSeat ? null : (chosen?.[seat] ?? 1),
    );

  // The same guard, and the same reasoning, as `App`'s `inFlight` — read its
  // comment; this is the second site and it was missing.
  //
  // `busy` STATE could not hold it: React batches, so two clicks in the same
  // tick both read `busy === false` and both post. On `App` that costs a
  // desynced picker. Here it posts `POST /api/games` twice, and the two games
  // are genuinely different — the server draws a fresh bot seed per request, so
  // with a bot on the start seat each plays a different opening. The server
  // now serializes creates and refuses the second by name, but a client that
  // sends a request it knows it should not is the wrong half to leave broken:
  // before that fix the loser's record landed on disk and the client was
  // silently switched onto the other game.
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

  return (
    <div className="setup">
      <h2>New game</h2>
      <p className="lede">
        One human seat against <code>v4-step9-draft1</code>, the shipped bot.
      </p>

      <div className="field">
        <label htmlFor="players">Players</label>
        <select
          id="players"
          value={players}
          onChange={(e) => {
            const n = Number(e.target.value);
            setPlayers(n);
            setBotSeat(Math.min(botSeat, n - 1) || 1);
            setPreview(null);
          }}
        >
          {[2, 3, 4].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </div>

      <div className="field">
        <label htmlFor="seed">Seed</label>
        <input
          id="seed"
          value={seedText}
          inputMode="numeric"
          placeholder="blank — the server draws one"
          onChange={(e) => {
            setSeedText(e.target.value);
            setPreview(null);
          }}
        />
      </div>

      <div className="field">
        <label htmlFor="bot">Bot seat</label>
        <select id="bot" value={botSeat} onChange={(e) => setBotSeat(Number(e.target.value))}>
          {Array.from({ length: players }, (_, i) => i)
            .filter((i) => i !== 0)
            .map((i) => (
              <option key={i} value={i}>
                seat {i} ({colors[i]})
              </option>
            ))}
        </select>
      </div>

      {!preview ? (
        <button
          type="button"
          className="primary"
          disabled={busy}
          onClick={() =>
            run(async () => {
              const p = await api.preview(base());
              setPreview(p);
              setKeeps(Array.from({ length: players }, () => 1));
            })
          }
        >
          Deal
        </button>
      ) : (
        <>
          <p className="lede">
            Seed <code>{preview.seed}</code> &mdash; keep one card per seat.
          </p>
          {preview.deals.map((deal) => (
            <div key={deal.seat}>
              <div className="field">
                <label>
                  Seat {deal.seat} {deal.seat === botSeat ? "(bot)" : "(you)"}
                </label>
              </div>
              <div className="deal">
                {deal.cards.map((dealt, idx) => (
                  <button
                    type="button"
                    className={`deal-pick ${keeps[deal.seat] === idx + 1 ? "chosen" : ""}`}
                    key={dealt.card.id}
                    onClick={() =>
                      setKeeps((k) => k.map((v, i) => (i === deal.seat ? idx + 1 : v)))
                    }
                  >
                    <OrderCardView art={art} card={dealt.card} id={dealt.card.id} />
                  </button>
                ))}
              </div>
            </div>
          ))}
          <div className="controls">
            <button
              type="button"
              className="primary"
              disabled={busy}
              onClick={() =>
                run(async () => {
                  // The seed sent back is the STRING the server gave, verbatim.
                  const req = { ...base(), seed: preview.seed, keeps: forSeats(keeps) };
                  onStarted(await api.create(req), true);
                })
              }
            >
              Start
            </button>
            <button type="button" className="ghost" disabled={busy} onClick={() => setPreview(null)}>
              Re-deal
            </button>
          </div>
        </>
      )}

      {/* Every game is persisted as its `GameRecord` after every applied move,
          and a game whose handle is gone is rebuilt from that record — so
          resuming one is a `GET`, not a restore. */}
      <section className="saved">
        <h3>Saved games</h3>
        {saved.length === 0 ? (
          <p className="muted">none yet — a game appears here as soon as it starts.</p>
        ) : (
          <ul>
            {saved.map((name) => (
              <li key={name}>
                <button
                  type="button"
                  className="ghost"
                  disabled={busy}
                  onClick={() => run(async () => onStarted(await api.get(name), false))}
                >
                  {name}
                </button>
                {/* **Two clicks, and the first one asks (final review I2).**
                    Deleting a record is not undoable — the game is gone — and
                    M5 put this list one banner menu away from a running game,
                    so a stray click beside the game you meant to resume used
                    to remove a different one.

                    A row that asks, rather than `window.confirm`: a native
                    dialog blocks the browser walk this client is tested with,
                    and the question belongs where the mistake is made. Both
                    buttons NAME the game, so the confirmation cannot be about
                    a row other than the one that opened it. */}
                {confirming === name ? (
                  <>
                    <button
                      type="button"
                      className="ghost danger"
                      disabled={busy}
                      aria-label={`confirm delete ${name}`}
                      onClick={() =>
                        run(async () => {
                          await api.remove(name);
                          setConfirming(null);
                          setSaved(await api.list());
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
                    onClick={() => setConfirming(name)}
                  >
                    delete
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
