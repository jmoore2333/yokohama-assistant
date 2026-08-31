import { useState } from "react";
import { api, ApiError, type NewGameRequest } from "./api";
import type { ClientState, Preview } from "./types";

const COLORS = ["Red", "Blue", "Yellow", "Green"] as const;

/** Setup: the sim tail of the CLI's wizard, in two steps.
 *
 * The deal preview is a real request rather than a guess — `new_game` is
 * deterministic from the seed, so the cards shown here ARE the cards dealt. */
export function NewGame({ onStarted }: { onStarted: (state: ClientState) => void }) {
  const [players, setPlayers] = useState(2);
  const [seedText, setSeedText] = useState("");
  const [botSeat, setBotSeat] = useState(1);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [keeps, setKeeps] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const colors = COLORS.slice(0, players);
  const base = (): NewGameRequest => ({
    players,
    colors: [...colors],
    start_seat: 0,
    ...(seedText.trim() ? { seed: Number(seedText.trim()) } : {}),
    keeps: Array.from({ length: players }, () => 1),
    bot_seats: [botSeat],
  });

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
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
                {deal.cards.map((card, idx) => (
                  <button
                    type="button"
                    className={`card ${keeps[deal.seat] === idx + 1 ? "chosen" : ""}`}
                    key={card.id}
                    onClick={() =>
                      setKeeps((k) => k.map((v, i) => (i === deal.seat ? idx + 1 : v)))
                    }
                  >
                    <div className="country">{card.country}</div>
                    <div className="tile-row muted">
                      O{String(card.id).padStart(2, "0")} &middot; needs{" "}
                      {Object.entries(card.requirements)
                        .filter(([, v]) => (v ?? 0) > 0)
                        .map(([k, v]) => `${v} ${k}`)
                        .join(" + ") || "nothing"}
                    </div>
                    <div className="stat-row">
                      {Object.entries(card.rewards)
                        .filter(([, v]) => v > 0)
                        .map(([k, v]) => (
                          <span className="chip" key={k}>
                            {v} {k}
                          </span>
                        ))}
                    </div>
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
                  const req = { ...base(), seed: preview.seed, keeps };
                  onStarted(await api.create(req));
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

      {error && <div className="error">{error}</div>}
    </div>
  );
}
