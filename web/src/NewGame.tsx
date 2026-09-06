import { useEffect, useRef, useState } from "react";
import { api, ApiError, type NewGameRequest } from "./api";
import { Board } from "./Board";
import { BoardsMenu } from "./BoardsMenu";
import { backArt } from "./art";
import { OrderCardView, seatClass } from "./Cards";
import { TableSections } from "./TableSections";
import type { SectionKey } from "./sections";
import type { ArtManifest, ClientState, Labels, PlayerColor, Preview } from "./types";

const COLORS = ["Red", "Blue", "Yellow", "Green"] as const;

/** Two face-down order cards: a deal this browser was never sent.
 *
 *  **The absence is the server's, not this component's** (R-C1). A bot seat's
 *  `cards` arrives empty and there is no keep field to read, so there is no
 *  filtering here that a bug could skip — the picture is drawn from `is_bot`
 *  alone. The art is the manifest's own order back — the key is `order_back`,
 *  not `order`, because the `backs` family is keyed by FILE stem where the
 *  other families are keyed by an engine identifier; where the manifest has not
 *  arrived the typed alternative is drawn, the rule `art.ts` states for every
 *  family.
 *
 *  Small and reusable on purpose: a human seat that has chosen, or is waiting
 *  its turn, shows the same two backs. */
function DealBacks({ art }: { art: ArtManifest | null }) {
  const src = backArt(art, "order_back");
  return (
    <div className="deal">
      {[1, 2].map((n) => (
        <div className="deal-card" key={n}>
          <figure className="card-face order sm back">
            {src ? (
              <img className="back" src={src} alt="" loading="lazy" />
            ) : (
              <div className="card-blank">face down</div>
            )}
          </figure>
        </div>
      ))}
    </div>
  );
}

/** Setup: the sim tail of the CLI's wizard, in two steps, plus the saved games.
 *
 * The deal preview is a real request rather than a guess — `new_game` is
 * deterministic from the seed, so the cards shown here ARE the cards dealt.
 * The seed is carried as a STRING throughout: it is a `u64`, a JS number cannot
 * hold one, and this screen round-trips it from preview to create. Rounding it
 * would deal a different game than the one on screen.
 *
 * **The second step is the TABLE, not a list of cards (M5.5 §3.3.3).** The
 * preview carries the position after setup as a `HistoricalFrame`, so the deal
 * is drawn over the same pyramid, port, laboratory and goals the M5 table draws
 * during play — through the same `Board` and `TableSections` components, at
 * `game={null}`, which is exactly the arrangement the rulebook has on the table
 * at its step 12 when the two cards are drawn. Nothing is created until Start:
 * the server threw its preview session away.
 *
 * **The choice is per HUMAN seat, and another seat's deal is not yours to see
 * (the 2026-09-06 corrections, R-C1).** A bot seat shows two card BACKS and the
 * line that it keeps one of them: the server sends no cards and no keep on a
 * bot seat, so there is nothing here to filter and nothing to leak. The bot
 * still keeps its own argmax — the record holds the card, the player does not
 * see it — and `keeps` carries `null` on that seat, which the server refuses a
 * value on by name. */
export function NewGame({
  art,
  labels,
  sections,
  onToggleSection,
  onStarted,
}: {
  art: ArtManifest | null;
  labels: Labels | null;
  /** The boards the set table draws, and the toggle behind its Boards menu.
   *
   *  **Owned by `App`, not by this screen (R-T12-4).** `useSections()` seeds
   *  from storage at MOUNT, so a second call here was a second copy: the menu
   *  wrote storage and updated this screen, while `App` — which never remounts,
   *  since this component is its child — went on holding what it read at
   *  start-up and handed THAT to the game's own `TableSections`. A strip
   *  switched off during setup came back the moment the game opened. */
  sections: SectionKey[];
  onToggleSection: (k: SectionKey) => void;
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
  /** The card each HUMAN seat has chosen, `1` or `2`, by seat. Empty after a
   *  deal: Start waits until every human seat is in it. A seat missing from it
   *  is a seat nobody has answered for, which a `number[]` of ones could not
   *  say — the old shape started every seat pre-chosen at card 1. */
  const [chosen, setChosen] = useState<Record<number, 1 | 2>>({});
  const [saved, setSaved] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  /** The saved game whose `delete` has been clicked once — see the row below. */
  const [confirming, setConfirming] = useState<string | null>(null);

  useEffect(() => {
    api.list().then(setSaved).catch(() => setSaved([]));
  }, []);

  const colors = COLORS.slice(0, players) as PlayerColor[];
  const base = (): NewGameRequest => ({
    players,
    colors: [...colors],
    start_seat: 0,
    ...(seedText.trim() ? { seed: seedText.trim() } : {}),
    keeps: previewKeeps(),
    bot_seats: [botSeat],
  });

  /** The `keeps` list the wire takes: a number on a human seat, `null` on the
   *  bot's, whose keep is the bot's own. A value on a bot seat is a 400 naming
   *  the seat, and so is a `null` on a human one.
   *
   *  **Two of them, because the two requests mean different things by it.** The
   *  PREVIEW deals both cards to every seat and keeps neither — it is the deal,
   *  not the choice — so the field is only there to pass the shape check, and
   *  every human seat sends `1`. CREATE is where the choice is made and sends
   *  what the player picked. */
  const previewKeeps = (): (number | null)[] =>
    Array.from({ length: players }, (_, seat) => (seat === botSeat ? null : 1));

  const createKeeps = (): (number | null)[] =>
    Array.from({ length: players }, (_, seat) =>
      seat === botSeat ? null : (chosen[seat] ?? null),
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

  // **The set table.** Everything below the banner is drawn from
  // `preview.table`, which is a real position over a session the server has
  // already thrown away, so this screen creates nothing until Start.
  if (preview) {
    const humans = preview.deals.filter((d) => !d.is_bot);
    const bots = preview.deals.filter((d) => d.is_bot);
    // Start waits for EVERY human seat: at 3P and 4P one operator drives more
    // than one of them, exactly as the terminal does, and a Start that fired on
    // the first choice would send `null` for the rest and be refused by name.
    const ready = humans.every((d) => chosen[d.seat] !== undefined);
    const pad = (id: number) => `O${String(id).padStart(2, "0")}`;
    return (
      <div className="app setup-table">
        <header className="banner">
          <h1>Yokohama</h1>
          <span className="meta">
            setting up &middot; seed <code>{preview.seed}</code>
          </span>
          <span className="meta turn">choose your card</span>
          {/* Spec §3.3.3: the same menu the game banner carries, so a player can
              switch a board on and read a site card at full size before
              choosing. `nav.views` is the banner's own slot for it. */}
          <nav className="views">
            <BoardsMenu enabled={sections} onToggle={onToggleSection} />
          </nav>
        </header>
        <div className="end-strip" role="status" />
        <main className="table">
          <div className="table-body">
            {/* No `onTileClick`: every tile is an inert `<div>`, the same rule a
                replay position takes. There is no game to answer yet. */}
            <Board board={preview.table.frame.board} art={art} labels={labels} colors={colors} />
            {/* `game={null}` and `state={null}`: nothing is fetched, because
                there is nothing to fetch from. The port and the laboratory ride
                on the frame — every `HistoricalFrame` carries them — so they
                draw from the position rather than reading "loading…", and the
                one board that has no position-shaped version says so. */}
            <TableSections
              enabled={sections}
              game={null}
              state={null}
              replaying={false}
              scrubbing={false}
              view={preview.table}
              art={art}
              labels={labels}
              colors={colors}
            />
          </div>
        </main>
        <aside className="side">
          {bots.map((d) => (
            <section className="panel deal-seat bot" data-seat={d.seat} key={d.seat}>
              <h2>
                <span className={seatClass(colors[d.seat])}>{colors[d.seat]}</span> &middot; bot,
                seat {d.seat}
              </h2>
              <DealBacks art={art} />
              <p className="muted deal-unseen">keeps one of its two cards &mdash; unseen</p>
            </section>
          ))}
        </aside>
        <div className="deck">
          <div className="decision">
            <div className="asking">
              Keep one card per seat. The table is set as the rulebook has it at step 12.
            </div>
            {humans.map((d) => (
              <section className="deal-seat human" data-seat={d.seat} key={d.seat}>
                <div className="field">
                  <label>
                    <span className={seatClass(colors[d.seat])}>{colors[d.seat]}</span> &middot;
                    seat {d.seat}
                  </label>
                </div>
                <div className="deal">
                  {d.cards.map((c, i) => (
                    <button
                      type="button"
                      className={`deal-pick ${chosen[d.seat] === i + 1 ? "chosen" : ""}`}
                      aria-pressed={chosen[d.seat] === i + 1}
                      aria-label={`keep ${pad(c.card.id)} for seat ${d.seat}`}
                      key={c.card.id}
                      onClick={() => setChosen((k) => ({ ...k, [d.seat]: (i + 1) as 1 | 2 }))}
                    >
                      <OrderCardView
                        art={art}
                        card={c.card}
                        id={c.card.id}
                        size="sm"
                        chips={false}
                        note={`Keep score ${c.keep_score}`}
                      />
                    </button>
                  ))}
                </div>
              </section>
            ))}
            {/* The CLI's own `KEEP_SCORE_NOTE` (`wizard.rs:1384`), word for
                word: the number is the bot's preference from the card's printed
                economics, and saying so beside it is the difference between a
                factual number and an implied recommendation. */}
            <p className="muted keep-hedge">
              the Keep score is what the built-in bot would prefer, from the card&rsquo;s printed
              economics alone &mdash; hand-set, unmeasured, and no substitute for your own read
            </p>
            <div className="controls">
              <button
                type="button"
                className="primary"
                disabled={busy || !ready}
                onClick={() =>
                  run(async () =>
                    onStarted(
                      // The seed sent back is the STRING the server gave, verbatim.
                      await api.create({ ...base(), seed: preview.seed, keeps: createKeeps() }),
                      true,
                    ),
                  )
                }
              >
                Start
              </button>
              <button
                type="button"
                className="ghost"
                disabled={busy}
                onClick={() => {
                  setPreview(null);
                  setChosen({});
                }}
              >
                Re-deal
              </button>
            </div>
            {error && <div className="error">{error}</div>}
          </div>
        </div>
      </div>
    );
  }

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

      <button
        type="button"
        className="primary"
        disabled={busy}
        onClick={() =>
          run(async () => {
            setPreview(await api.preview(base()));
            setChosen({});
          })
        }
      >
        Deal
      </button>

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
