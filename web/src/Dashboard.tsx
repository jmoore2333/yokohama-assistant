import type { AchievementModel, Labels, Panel, PlayerModel, TracksPanel } from "./types";

const good = (n: number, cls: string, label: string) =>
  n > 0 ? (
    <span className={`chip ${cls}`} key={cls}>
      {n} {label}
    </span>
  ) : null;

export function Dashboard({ panels, labels }: { panels: Panel[]; labels: Labels | null }) {
  return (
    <>
      {panels.map((panel, i) => {
        if ("Players" in panel) return <Players key={i} players={panel.Players.players} />;
        if ("Tracks" in panel) return <Tracks key={i} tracks={panel.Tracks} />;
        if ("Achievements" in panel)
          return <Achievements key={i} cards={panel.Achievements.cards} labels={labels} />;
        if ("Hand" in panel) {
          const hand = panel.Hand;
          return (
            <section className="panel" key={i}>
              <h2>Your hand</h2>
              {hand.hidden ? (
                <div className="tile-row muted">{hand.card_count} card(s) — hidden</div>
              ) : hand.cards.length === 0 ? (
                <div className="tile-row muted">empty</div>
              ) : (
                hand.cards.map((c) => (
                  <div className="tile-row" key={c.id}>
                    <span className="chip">O{String(c.id).padStart(2, "0")}</span>
                    <span>{c.printed?.country ?? ""}</span>
                    {c.is_starting_order && <span className="chip">starting</span>}
                  </div>
                ))
              )}
            </section>
          );
        }
        return null;
      })}
    </>
  );
}

function Players({ players }: { players: PlayerModel[] }) {
  return (
    <section className="panel">
      <h2>Players</h2>
      {players.map((p) => (
        <div className={`seat ${p.is_current ? "current" : ""}`} key={p.seat}>
          <div className="seat-head">
            <span className={`seat-name seat-${p.color.toLowerCase()}`}>{p.color}</span>
            {p.is_bot && <span className="tag">bot</span>}
            <span className="tag">seat {p.seat}</span>
            <span className="chip yen">&#165;{p.money}</span>
            <span className="chip vp">{p.victory_points} VP</span>
          </div>
          <div className="stat-row">
            {good(p.goods.copper, "copper", "copper")}
            {good(p.goods.silk, "silk", "silk")}
            {good(p.goods.tea, "tea", "tea")}
            {good(p.goods.seafood, "seafood", "seafood")}
            {good(p.goods.imports, "imports", "imports")}
          </div>
          <div className="stat-row">
            <span className="chip">
              asst {p.assistants.in_hand}h / {p.assistants.on_board}b / {p.assistants.in_warehouse}w
            </span>
            <span className="chip">shop {p.shophouses.built}</span>
            <span className="chip">
              trade {p.trading_houses.built} (next &#165;{p.trading_houses.next_take_cost})
            </span>
            <span className="chip">orders {p.orders_in_hand}</span>
          </div>
        </div>
      ))}
    </section>
  );
}

function Tracks({ tracks }: { tracks: TracksPanel }) {
  return (
    <section className="panel">
      <h2>
        Tracks &mdash; ends at {tracks.trigger_at} of your pawns
      </h2>
      <div className="tile-row muted">Church faith &middot; {tracks.church_filled} taken</div>
      <div className="track">
        {tracks.church.map((s) => (
          <span className={`space ${s.occupant != null ? "taken" : ""}`} key={s.faith}>
            {s.faith}
          </span>
        ))}
      </div>
      <div className="tile-row muted" style={{ marginTop: "0.4rem" }}>
        Customs &middot; {tracks.customs_filled} taken
      </div>
      <div className="track">
        {tracks.customs.map((s) => (
          <span className={`space ${s.occupant != null ? "taken" : ""}`} key={s.space}>
            {s.space}
          </span>
        ))}
      </div>
    </section>
  );
}

function Achievements({ cards, labels }: { cards: AchievementModel[]; labels: Labels | null }) {
  return (
    <section className="panel">
      <h2>Achievements</h2>
      {cards.map((c) => (
        <div className="seat" key={c.achievement_type}>
          <div className="seat-head">
            <span className="tag">{c.category}</span>
            <span>{labels?.achievements[c.achievement_type] ?? c.achievement_type}</span>
          </div>
          <div className="stat-row">
            <span className="chip vp">
              {c.first_place_vp} / {c.subsequent_vp} VP
            </span>
            <span className="chip">
              {c.claimed_by.length === 0 ? "unclaimed" : `claimed ×${c.claimed_by.length}`}
            </span>
          </div>
        </div>
      ))}
    </section>
  );
}
