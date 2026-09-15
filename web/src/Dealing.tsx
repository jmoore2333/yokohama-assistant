// The dealing screen (M6.1 §7.4): the set table, and the two cards THIS person
// is being asked about.
//
// **The secrecy is the server's now, and that is the whole change.** Until this
// cycle one payload carried every seat's deal and the client drew the seat whose
// turn it was — so the only thing keeping seat 1's cards from seat 0 was a
// `turn` index in a component. `GET …/deal` serves the CALLER: `seat` is the
// lowest seat they hold that has not kept, `cards` is that seat's two, and no
// other seat's cards are on the wire at all. A person holding two seats still
// answers for them in turn — the hot-seat table this client has always
// supported — but the sequence is the server's answer after each keep rather
// than something stated here. `NewGame`'s `turn`, `passing` and "Next seat"
// retire with it.
//
// **What is on the table is the owner's edition at its step 9** (2026-09-06,
// R-C3): the areas with their site cards and tokens, the port, and the tracks.
// The technologies reach the Laboratory at step 10 and the achievements the
// score track at step 11, AFTER the secret choice — so the server withholds
// both and `TableSections` draws them as "revealed once you have chosen your
// card". The engine is untouched: the same cards are dealt in the same order
// from the same seed, and the game this screen opens shows them all.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { api, ApiError } from "./api";
import { backArt } from "./art";
import { Board } from "./Board";
import { BoardsMenu } from "./BoardsMenu";
import { OrderCardView, seatClass } from "./Cards";
import { TableSections } from "./TableSections";
import { useSignal } from "./signal";
import { holder, seatName } from "./tables";
import type { SectionKey } from "./sections";
import type { ArtManifest, DealView, Labels, PlayerColor } from "./types";

/** Two face-down order cards: a deal this browser was never sent.
 *
 *  **The absence is the server's, not this component's** (R-C1). A bot seat has
 *  no cards on the wire and no keep field to read, so there is no filtering here
 *  that a bug could skip — the picture is drawn from the seat's KIND alone. The
 *  art is the manifest's own order back; the key is `order_back`, not `order`,
 *  because the `backs` family is keyed by FILE stem where the other families are
 *  keyed by an engine identifier, and where the manifest has not arrived the
 *  typed alternative is drawn. */
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

const pad = (id: number) => `O${String(id).padStart(2, "0")}`;

export function Dealing({
  game,
  me,
  art,
  labels,
  sections,
  onToggleSection,
  onPlaying,
  onLeave,
  who,
}: {
  /** The table being dealt. */
  game: string;
  /** Who this browser plays as, from the door — used to say "you" rather than
   *  your own email back at you on the seats you hold yourself, which at a
   *  hot-seat table is most of them. */
  me: string;
  art: ArtManifest | null;
  labels: Labels | null;
  /** Owned by `App` (R-T12-4), so this screen and the game agree about which
   *  boards are on. */
  sections: SectionKey[];
  onToggleSection: (k: SectionKey) => void;
  /** The table has started. `App` opens the game; this screen is replaced. */
  onPlaying: () => void | Promise<void>;
  /** Back to the games list.
   *
   *  **A screen you cannot leave is a screen you are stuck on.** The M5.5 set
   *  table had "Re-deal" for this; a table is PERSISTED now, so leaving is not
   *  abandoning anything — the row is in the lobby and opens right back here. A
   *  player who opens a table that is waiting on somebody else, and then wants
   *  to play something else, otherwise has only a page reload. */
  onLeave: () => void;
  /** The "You are …" strip, built by `App` and merely placed here. */
  who?: ReactNode;
}) {
  const [deal, setDeal] = useState<DealView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // The same guard, and the same reasoning, as `App`'s `inFlight`: React
  // batches, so two clicks in the same tick both read a `busy` STATE of false
  // and both post — and a second keep on a seat that has already kept is a 409,
  // which a player would read as the screen refusing their own click.
  const inFlight = useRef(false);
  /** The game has been handed on. Everything that can notice the table has
   *  started — the keep's own reply, a refusal from `/deal`, the poll — is
   *  raced against this, so `App` is told exactly once.
   *
   *  **Taken before the attempt and RELEASED if it fails**, which is the whole
   *  of `start` below. Taken before, because the three noticers overlap and two
   *  of them could otherwise open the game twice. Released after a failure,
   *  because by then the game is already running server-side — the last keep
   *  started it — so a 500 or a dropped connection on the read that opens it
   *  would otherwise strand this browser on a dealing screen for a table that is
   *  not dealing, behind a latch nothing could ever clear. The purpose is one
   *  hand-over per SUCCESS, and that is unchanged. */
  const started = useRef(false);

  // `onPlaying` through a ref, so `start` has a stable identity and the poll
  // below is not re-armed by a re-render. `App` re-creates the callback every
  // time, and a timer torn down and restarted on each one is a poll that never
  // reaches its own first tick. (`useSignal` holds its consumer through a ref
  // for exactly the same reason; this one keeps `start` itself stable, which
  // `fetchDeal` also depends on.)
  const playing = useRef(onPlaying);
  playing.current = onPlaying;

  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));

  const start = useCallback(async () => {
    if (started.current) return;
    started.current = true;
    try {
      await playing.current();
    } catch (e) {
      // The hand-over did not happen, so nothing has been handed on: release the
      // latch and let the next poll tick — or the next keep — try again.
      started.current = false;
      setError(said(e));
    }
  }, []);

  /** Ask for the deal this caller is owed.
   *
   *  **A 409 here is an ANSWER, not a failure**: `GET …/deal` refuses a table
   *  that is already playing (§4.3), which is exactly what a table finalized by
   *  somebody else between two of this browser's requests looks like. */
  const fetchDeal = useCallback(async () => {
    try {
      setDeal(await api.deal(game));
      setError(null);
    } catch (e) {
      if (e instanceof ApiError && e.status === 409) {
        void start();
        return;
      }
      setError(said(e));
    }
  }, [game, start]);

  useEffect(() => {
    void fetchDeal();
  }, [fetchDeal]);

  /** How this screen learns that the table started without it: the SAME poll
   *  the game screen uses (M6.1 §6).
   *
   *  **The contract is the one the interval held, and it has not changed**:
   *  when `GET …/events` reports a status other than `dealing`, the game is
   *  running and this screen is done. The only case it serves is a table whose
   *  remaining seats are at OTHER browsers — a keep of this browser's own is
   *  answered by `POST …/keep` itself, which says whether that keep finalized
   *  the table.
   *
   *  What the hook adds is what the interval could not do: nothing is asked
   *  from a tab nobody is looking at (R-P8), the return asks at once, a server
   *  that cannot answer is asked less often rather than more, and the strip
   *  below says which of those is happening.
   *
   *  **`active` is a constant `true`, and the latch is what stops it.** The
   *  three noticers overlap by design — the keep's reply, a 409 from `/deal`,
   *  this poll — so `start` is the one place that decides, and it returns at
   *  once once the hand-over has succeeded. A ref cannot deactivate a hook
   *  without a re-render, and the screen is being replaced anyway: the cost of
   *  a cheap request nothing acts on, for the one or two ticks between the
   *  hand-over and the unmount, is the honest price of keeping ONE decision. */
  const link = useSignal(game, true, (e) => {
    if (e.status !== "dealing") void start();
  });
  /** The poll's own state, said the same way the game banner says it. */
  const linkStrip = link !== "live" && (
    <span className="meta link" role="status">
      {link}&hellip;
    </span>
  );

  const keep = async (card: 1 | 2) => {
    if (!deal || deal.seat === null || inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setError(null);
    try {
      // **The reply decides whether there is anything left to ask.** The LAST
      // human keep finalizes the table server-side, and a client that went back
      // to `GET …/deal` anyway would earn a 409 and sit on a dealing screen
      // until the poll came round.
      const table = await api.keep(game, deal.seat, card);
      if (table.status !== "dealing") {
        await start();
      } else {
        await fetchDeal();
      }
    } catch (e) {
      setError(said(e));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  };

  if (!deal) {
    return (
      <div className="setup dealing-wait">
        {(who || linkStrip) && (
          <div className="who-strip">
            {linkStrip}
            {who}
          </div>
        )}
        <h2>Dealing</h2>
        {error ? <div className="error">{error}</div> : <p className="muted">dealing&hellip;</p>}
      </div>
    );
  }

  const table = deal.table;
  const colors = table.colors as PlayerColor[];
  const mine = deal.seat;
  /** Every seat but the one being asked about, in seat order. A bot shows two
   *  backs; a person shows who they are and whether they have answered —
   *  `chosen` is what crosses the wire instead of their cards, because whether a
   *  seat has answered is public at any table and which card it took is not. */
  const others = table.seats
    .map((seat, i) => ({ seat, i }))
    .filter(({ i }) => i !== mine);
  /** Who the table is still waiting on, once every seat this caller holds has
   *  chosen. Never this browser's own seats: `seat` is null precisely when they
   *  have all kept. */
  const outstanding = table.seats
    .map((seat, i) => ({ seat, i }))
    .filter(({ seat, i }) => seat.kind === "human" && !table.chosen[i])
    .map(({ i }) => holder(table, i));
  const note = deal.keep_note.trim().replace(/\s+/g, " ");

  return (
    <div className="app setup-table dealing">
      <header className="banner">
        <h1>Yokohama</h1>
        <span className="meta">{table.name ?? table.id}</span>
        {linkStrip}
        {who}
        {/* Spec §3.3.3: the same menu the game banner carries, so a player can
            switch a board on and read a site card at full size before
            choosing. */}
        <nav className="views">
          <BoardsMenu enabled={sections} onToggle={onToggleSection} />
          <button type="button" className="ghost" onClick={onLeave}>
            Games
          </button>
        </nav>
      </header>
      <div className="end-strip" role="status" />
      <main className="table">
        <div className="table-body">
          {/* No `onTileClick`: every tile is an inert `<div>`, the rule a replay
              position takes. There is no game to answer yet. */}
          <Board board={deal.frame.frame.board} art={art} labels={labels} colors={colors} />
          {/* `game={null}` and `state={null}`: nothing is fetched, because there
              is nothing to fetch from. The PORT rides on the frame — every
              `HistoricalFrame` carries it — so it draws from the position, and
              so do the tracks. The laboratory and the goals are WITHHELD by the
              server until the card is chosen (R-C3) and say so. */}
          <TableSections
            enabled={sections}
            game={null}
            state={null}
            replaying={false}
            scrubbing={false}
            view={deal.frame}
            art={art}
            labels={labels}
            colors={colors}
          />
        </div>
      </main>
      <aside className="side">
        {others.map(({ seat, i }) =>
          seat.kind === "bot" ? (
            <section className="panel deal-seat bot" data-seat={i} key={i}>
              <h2>
                <span className={seatClass(colors[i])}>{colors[i]}</span> &middot; bot, seat {i}
              </h2>
              <DealBacks art={art} />
              <p className="muted deal-unseen">keeps one of its two cards &mdash; unseen</p>
            </section>
          ) : (
            <section
              className={`panel deal-seat human other ${table.chosen[i] ? "chosen" : "waiting"}`}
              data-seat={i}
              key={i}
            >
              <h2>
                <span className={seatClass(colors[i])}>{colors[i]}</span> &middot; seat {i}
              </h2>
              <p className="muted deal-state">
                {seat.kind === "human" && seat.user === me ? "you" : seatName(seat)} &middot;{" "}
                {table.chosen[i] ? "chosen" : "waiting"}
              </p>
            </section>
          ),
        )}
      </aside>
      <div className="deck">
        <div className="decision">
          {deal.cards && mine !== null ? (
            <>
              <h2 className="asking">Choose your card</h2>
              <p className="muted setup-note">
                The table is set: the areas with their site cards and tokens, the port and the
                tracks. The technologies and the goals are revealed once you have chosen.
              </p>
              <section className="deal-seat human current" data-seat={mine}>
                <div className="field">
                  <label>
                    <span className={seatClass(colors[mine])}>{colors[mine]}</span> &middot; seat{" "}
                    {mine}
                  </label>
                </div>
                <div className="deal">
                  {deal.cards.map((c, n) => (
                    <button
                      type="button"
                      className="deal-pick"
                      disabled={busy}
                      aria-label={`keep ${pad(c.card.id)} for seat ${mine}`}
                      key={c.card.id}
                      onClick={() => void keep((n + 1) as 1 | 2)}
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
              {/* **The terminal's own note, SERVED rather than copied** (§4.3).
                  It is `wizard::KEEP_SCORE_NOTE`, so the hedge beside the Keep
                  score cannot drift from the CLI's; it arrives wrapped for a
                  terminal, and the wrapping is what is dropped here. */}
              <p className="muted keep-hedge">{note}</p>
            </>
          ) : (
            <>
              <h2 className="asking">Waiting for the others</h2>
              <p className="deal-waiting">
                waiting on {outstanding.length > 0 ? outstanding.join(", ") : "the table"}
              </p>
            </>
          )}
          {error && <div className="error">{error}</div>}
        </div>
      </div>
    </div>
  );
}
