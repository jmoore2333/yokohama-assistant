import { seatClass } from "./Cards";
import { OutcomeView } from "./Index";
import { Tokens, tokenLabel } from "./Token";
import type {
  ArtManifest,
  ChinatownRow,
  ChooserPreview,
  ClientState,
  Outcome,
  Deciding,
  FeeModel,
  Labels,
  NotOfferedReason,
  PendingChoice,
  PlayerColor,
} from "./types";

/** A price, in the CLI's own words (`decision_render::fee_words`).
 *
 *  `null` and a zero total both read "free": `option_fee` returns a zero fee
 *  for every President move whether it costs anything or not, and the terminal
 *  prints `free` rather than nothing because a player comparing destinations is
 *  comparing prices, and an absent price reads as an unknown one. */
function feeWords(fee: FeeModel | null, colors: PlayerColor[]): string {
  if (!fee || fee.total === 0) return "free";
  const parts = fee.to_players.map(([seat, amount]) =>
    `${amount} to ${colors[seat] ?? `seat ${seat}`}`,
  );
  if (fee.canal_tolls > 0) {
    parts.push(`${fee.canal_tolls} canal toll${fee.canal_tolls === 1 ? "" : "s"}`);
  }
  return `pay ¥${fee.total} (${parts.join(", ")})`;
}

/** What a QUEUED CHOICE is asking, in the CLI's own words.
 *
 *  A queued choice OUTRANKS the phase: `Deciding::Choice` is what
 *  `screen_model` reports whenever `pending_choices` is non-empty, and
 *  `decision_render::choice_words` words each variant. The client was switching
 *  only on `prompt.kind` — which is `"options"` for every queued choice — and
 *  falling through to the phase, so a player picking a good was told "Area
 *  Action" and nothing on the screen named the actual question or said how many
 *  more were stacked behind it. */
const CHOICE_WORDS: Record<string, string> = {
  AnyGood: "which good to take — the reward is yours whichever you pick.",
  WarehouseTake:
    "what to take out of your warehouse, or to decline — each option shows its price.",
  AssistantMove: "which assistant to move, and where to — or to decline the move.",
  RecoveryLeave: "whether to leave one assistant behind (Telephone) or recover them all.",
  StationPlace:
    "where the Station token goes — every player reads +1 Power in the area you pick.",
};

function choiceWords(head: PendingChoice): string {
  if (typeof head === "string") return CHOICE_WORDS[head] ?? head;
  if ("ChurchSpace" in head) {
    return `which Church space to take at faith ${head.ChurchSpace.total_faith} — each shows its points and its rider.`;
  }
  const n = head.CustomsSpace.cost;
  return `which Customs space to take for ${n} imported good${n === 1 ? "" : "s"} — each shows its points and its rider.`;
}

/** The `Deciding` arm, when it is a choice rather than a phase. */
function decidingWords(deciding: Deciding): string | null {
  if (typeof deciding === "string" || !("Choice" in deciding)) return null;
  const { head, queued } = deciding.Choice;
  const tail = queued > 1 ? ` (${queued - 1} more to resolve after this one)` : "";
  return `${choiceWords(head)}${tail}`;
}

/** The near-miss rows, in the CLI's own words.
 *
 *  These say why an area you might expect is NOT on the list, and they are the
 *  difference between "the engine forgot" and "you have no assistant there".
 *  The list is capped at three, so the SUPPRESSED count has to be shown: three
 *  rows are indistinguishable from three-of-five otherwise, and a truncated list
 *  nobody is told about reads as the whole story. */
const NOT_OFFERED: Record<NotOfferedReason, string> = {
  StartedThereThisTurn: "you started your turn there",
  CanalIsNeverAStop: "the Canal is a crossing, never a stop",
  NoAssistantThere: "you have no assistant there",
};

/** The decision zone: where the turn stands, what is being asked, and what may
 * be answered.
 *
 * Every answerable thing is a button carrying the number the server expects.
 * The client never constructs a move and never prices one — the words on a row
 * come from the engine's own describer, fees and previews already in them. */
export function Decision({
  state,
  labels,
  colors,
  busy,
  outcome,
  spectating = false,
  art = null,
  onSend,
}: {
  state: ClientState;
  labels: Labels | null;
  colors: PlayerColor[];
  busy: boolean;
  /** The ranked breakdown from `/result`, once the game is over. */
  outcome?: Outcome | null;
  /** The caller holds NO SEAT at this table — a watcher (spec §5.2).
   *
   *  **It is not "it is not your turn", and that is why it is a prop rather
   *  than a read of `prompt.kind`.** A player waiting on somebody else gets a
   *  turn back; a watcher never does, so nothing on this screen is ever theirs
   *  to answer and every control here would be a 403 the server has already
   *  written down (R-4.3). `App` derives it from `state.seat === null`, which
   *  is the server's own statement of it.
   *
   *  **What it suppresses is the ANSWER BAR, not the account of the position.**
   *  The chain, the asking line, the `watching` and `locked` blocks and the
   *  finished game's standing all stay: they are what a watcher came for, and
   *  §5.2 says the result works for them as it does for a player. Suppressing
   *  everything would leave a spectator at a game that had ended reading "The
   *  game is over." over an empty deck.
   *
   *  **A default of `false`, so the fence is opt-in and no existing caller
   *  changes meaning.** */
  spectating?: boolean;
  /** The server's art manifest, for the choosers' tokens. A default of `null`
   *  (the typed marks) so no existing caller changes. */
  art?: ArtManifest | null;
  onSend: (line: string) => void;
}) {
  const d = state.frame.decision;
  const chain = labels?.phase_chain ?? d.phase_chain;
  const prompt = state.prompt;

  return (
    <div className="decision">
      <div className="chain">
        {/* The seat's COLOUR, as the terminal prints it (`TURN 1 · BLUE's
            turn`). A bare seat number is the one thing on this screen a player
            has to translate, and at 3P or 4P one operator drives several seats.
            The banner already colours it; the decision zone did not. */}
        <span>
          Turn {d.turn_number} &middot;{" "}
          <span className={seatClass(colors[d.seat])}>
            {colors[d.seat] ?? `seat ${d.seat}`}
          </span>
        </span>
        {chain.map((name, i) => (
          <span key={name + i} className={i === d.current_phase ? "lit" : ""}>
            {name}
          </span>
        ))}
      </div>

      <div className="asking">{asking(state, labels)}</div>

      {/* A locked session accepts nothing this client can send, so it offers
          nothing. The three lines the REPL would still take under lockdown are
          all refused by the API's grammar fence, which is why there is no
          "try anyway" button here. */}
      {prompt.kind === "watching" && (
        <div className="messages">
          <div>Waiting on seat {prompt.to_move}. Nothing here is yours to answer.</div>
        </div>
      )}

      {prompt.kind === "locked" && (
        <div className="messages">
          <div className="warn">
            This game is LOCKED after a driver or engine fault and cannot be played on.
          </div>
          <div className="warn">{prompt.reason}</div>
          <div>
            The record is intact on the server. Nothing further can be applied to it.
          </div>
        </div>
      )}

      {/* `scrolls` marks THE ONE box the deck is allowed to clip.
          `.options` is overloaded — the `retreat` arm below reuses it as a plain
          wrapper around a message and a row of count buttons — so scrolling
          every `.options` puts those counts inside the clip. Measured as
          `<div class="controls"> intercepts pointer events`, four failing
          browser specs. The answer rows are the only thing that may be
          off-screen inside the deck, because they are the only thing you can
          reach by scrolling to it. */}
      {!spectating && (prompt.kind === "options" || prompt.kind === "pick") && (
        <div className="options scrolls">
          {prompt.rows.length === 0 && <div className="messages">Nothing to answer here.</div>}
          {prompt.rows.map((row) => (
            <button
              type="button"
              className="option"
              key={row.number}
              disabled={busy}
              onClick={() => onSend(String(row.number))}
            >
              <span className="num">{row.number}</span>
              <span>{row.words}</span>
              {/* A price on every row that can carry one, free INCLUDED — the
                  terminal states it either way so that two rows differing only
                  in cost cannot look identical. */}
              {row.fee && (
                <span className={`price ${row.fee.total > 0 ? "costs" : ""}`}>
                  {feeWords(row.fee, colors)}
                </span>
              )}
              {row.opens_picker && <span className="more">opens a choice &rarr;</span>}
            </button>
          ))}
        </div>
      )}

      {!spectating && prompt.kind === "retreat" && (
        <div className="options">
          <div className="messages">
            <div>
              {prompt.remaining === 0 ? (
                <>Every area is answered &mdash; confirm to apply, or cancel.</>
              ) : (
                <>
                  Recover how many assistants from {prompt.area_label}? ({prompt.remaining} area(s)
                  still to answer)
                </>
              )}
            </div>
          </div>
          {/* `area`/`max` are fallback values once every area is answered, so
              the number buttons would be a row of one meaningless zero. The
              only live answer there is confirm. */}
          {prompt.remaining > 0 && (
            <div className="controls">
              {Array.from({ length: prompt.max + 1 }, (_, n) => (
                <button
                  type="button"
                  className="ghost"
                  key={n}
                  disabled={busy}
                  onClick={() => onSend(String(n))}
                >
                  {n}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {!spectating && prompt.kind === "place_a" && (
        <div className="messages">
          <div>
            Option A &mdash; click up to {prompt.max_areas} areas on the board to toggle them, then
            confirm. Selected:{" "}
            {prompt.selected.length === 0
              ? "none"
              : prompt.selected.map((a) => `A${String(a).padStart(2, "0")}`).join(", ")}
          </div>
          {/* The live price of the CURRENT selection. The terminal recomputes
              this on every redraw; without it a player pressed Confirm and paid
              a yen no surface had stated. */}
          {prompt.selected.length > 0 && (
            <div className={prompt.fee && prompt.fee.total > 0 ? "warn" : ""}>
              This placement is {feeWords(prompt.fee, colors)}.
            </div>
          )}
        </div>
      )}

      {!spectating && prompt.kind === "place_b" && (
        <div className="messages">
          <div>
            Option B &mdash; click one area on the board for two assistants, then confirm.
            {prompt.selected != null && ` Selected A${String(prompt.selected).padStart(2, "0")}.`}
          </div>
          {/* Two assistants in one area, so the fee is two yen per president
              owner there rather than one — not something to make a player
              remember mid-choice. */}
          {prompt.selected != null && (
            <div className={prompt.fee && prompt.fee.total > 0 ? "warn" : ""}>
              This placement is {feeWords(prompt.fee, colors)}.
            </div>
          )}
        </div>
      )}

      {/* **The finished game, ranked, with the categories the engine scored.**
          It printed `seat 0` and `46 VP` and nothing else — no colour, no winner
          mark, no breakdown — while the terminal prints `WINNER: <colour>` and a
          per-seat line of base + church + customs + tech + country + remainder.
          `state.scores` cannot supply that: it is `{seat, total}[]` in SEAT
          order with no rank and no `won`, so a winner mark taken from it would
          have to be inferred from the margin, and a margin of 0 is not a win —
          two seats can tie on points and be split by the engine's tie-break.
          `App` fetches `/result`, which carries `Placing.won` and the
          breakdown, and this renders it through the SAME `OutcomeView` the
          Final score surface uses rather than a second table that could
          disagree with it. Until it arrives the compact standing still shows,
          so the block is never empty. */}
      {prompt.kind === "game_over" &&
        (outcome ? (
          <OutcomeView outcome={outcome} colors={colors} />
        ) : (
          state.scores && (
            <div className="options">
              {[...state.scores]
                .sort((a, b) => b.total - a.total)
                .map((s) => (
                  <div className="option" key={s.seat}>
                    <span className="num">{s.seat}</span>
                    <span className={seatClass(colors[s.seat])}>
                      {colors[s.seat] ?? `seat ${s.seat}`}
                    </span>
                    <span className="more">{s.total} VP</span>
                  </div>
                ))}
            </div>
          )
        ))}

      {/* **The Church's chooser** (plan 2a): six kinds of payment, each a toggle.
          Everything on it is the payload's — the number a click sends, the
          words, what the seat has, whether the kind is selected and whether any
          listed move pays it. A kind no listed move pays is `unavailable` by
          the payload and a busy chooser is merely disabled; the walks tell the
          two apart by `data-unavailable`. */}
      {!spectating && prompt.kind === "church" && (
        <div className="chooser church">
          <div className="kinds">
            {prompt.kinds.map((k) => (
              <button
                type="button"
                className="kind"
                key={k.number}
                data-good={k.good}
                aria-pressed={k.selected}
                aria-label={`${k.words}, you have ${k.have}`}
                disabled={busy || !k.offered}
                data-unavailable={k.offered ? undefined : "true"}
                onClick={() => onSend(String(k.number))}
              >
                <Tokens good={k.good} amount={k.amount} art={art} />
                <span className="lbl">
                  <span>{k.words}</span>
                  <span className="have">you have {k.have}</span>
                </span>
              </button>
            ))}
          </div>
          <Verdict preview={prompt.preview} faiths={prompt.offered_faiths} />
        </div>
      )}

      {/* **Chinatown's chooser** (plan 2a): the seat's stock, then nine exchanges,
          four sales under Sell and five purchases under Buy, each with a minus,
          the count chosen and a plus. Everything on it is the payload's — the
          words, the amounts, the count, whether one more can be added and the
          stock; the split is the row's own `sells` flag, never a position.
          **The rows are the deck's scrolling box** (`.rows`, as `.options.scrolls`
          is for a list): the stock and the verdict above and below it do not
          scroll, and Confirm and Cancel are outside the chooser altogether. */}
      {!spectating && prompt.kind === "chinatown" && (
        <div className="chooser chinatown">
          <div className="stock">
            {prompt.stock.map((s) => (
              /* One node a reader hears once (`3 yen`); the small token inside
                 is hidden so it is not read a second time. */
              <span
                className="item"
                key={s.good}
                role="img"
                aria-label={tokenLabel(s.good, s.amount)}
                data-good={s.good}
              >
                <span aria-hidden="true">
                  <Tokens good={s.good} amount={1} art={art} small />
                </span>
                <span className="amt">{s.amount}</span>
              </span>
            ))}
          </div>
          <div className="rows">
            <div className="side-h">Sell</div>
            {prompt.rows
              .filter((r) => r.sells)
              .map((r) => (
                <Exchange key={r.number} row={r} art={art} busy={busy} onSend={onSend} />
              ))}
            <div className="side-h">Buy</div>
            {prompt.rows
              .filter((r) => !r.sells)
              .map((r) => (
                <Exchange key={r.number} row={r} art={art} busy={busy} onSend={onSend} />
              ))}
          </div>
          <Verdict preview={prompt.preview} empty={prompt.chosen === 0} />
        </div>
      )}

      {/* The near-miss rows say why an area is NOT on the list a watcher was
          never offered, so they go with it. */}
      {!spectating && (d.not_offered.length > 0 || d.not_offered_suppressed > 0) && (
        <div className="near-miss">
          {/* The area's NAME, not only its id. `decision_render.rs:352`
              resolves `area_name(board, row.area)` and prints the name; this
              printed `A07` and left the player to find it on the board. The
              board is already in scope, so it is a lookup rather than a
              payload change. */}
          {d.not_offered.map((n) => (
            <div key={n.area}>
              not offered: <b>A{String(n.area).padStart(2, "0")}</b>{" "}
              {areaName(state, labels, n.area)} &mdash;{" "}
              {NOT_OFFERED[n.reason] ?? n.reason}
            </div>
          ))}
          {d.not_offered_suppressed > 0 && (
            <div className="muted">
              &hellip;and {d.not_offered_suppressed} more not shown
            </div>
          )}
        </div>
      )}

      {/* **Confirm is offered only where the engine will take it.**
          `repl::step_pick` has arms for `cancel`, a digit, `more` and `list`,
          and a catch-all at `:2132` answering "! finish or cancel this prompt
          first" — there is NO `confirm` arm, so the button on a pick could only
          ever refuse. `step_retreat` gates it too: `confirming` is
          `collected_len >= areas_len` (`:2334`), so confirming with areas still
          to answer is refused the same way. Cancel is always live. */}
      {!spectating &&
        (prompt.kind === "place_a" ||
          prompt.kind === "place_b" ||
          prompt.kind === "pick" ||
          prompt.kind === "retreat") && (
        <div className="controls">
          {(prompt.kind === "place_a" ||
            prompt.kind === "place_b" ||
            (prompt.kind === "retreat" && prompt.remaining === 0)) && (
            <button
              type="button"
              className="primary"
              disabled={busy}
              onClick={() => onSend("confirm")}
            >
              Confirm
            </button>
          )}
          <button type="button" className="ghost" disabled={busy} onClick={() => onSend("cancel")}>
            Cancel
          </button>
        </div>
      )}

      {!spectating && (prompt.kind === "church" || prompt.kind === "chinatown") && (
        <ChooserControls legal={prompt.preview.legal} busy={busy} onSend={onSend} />
      )}

      {/* Undo lives here, so a watcher loses it with the rest of the bar
          (spec §5.2). The server refuses one from a caller holding no seat
          either way — this is the screen not offering what cannot work. */}
      {!spectating && prompt.kind === "options" && (
        <div className="controls">
          {prompt.rows.length === 1 && (
            <button type="button" className="primary" disabled={busy} onClick={() => onSend("")}>
              Apply the only option
            </button>
          )}
          <button type="button" className="ghost" disabled={busy} onClick={() => onSend("undo")}>
            Undo
          </button>
        </div>
      )}
    </div>
  );
}

/** What stands under a chooser's selection, from the arm's own preview.
 *
 *  Nothing is worked out here: `legal` and `words` are the engine's reading of
 *  the legal list, and `faiths` (the Church's line, when the selection names no
 *  listed move) is the server's own sentence. A `status` region, so the verdict
 *  a click changes is announced as the terminal prints it. */
function Verdict({
  preview,
  faiths,
  empty = false,
}: {
  preview: ChooserPreview;
  faiths?: string | null;
  /** Nothing chosen yet (Chinatown at `chosen` 0): the region stays mounted
   *  and says nothing, so the first selection is announced against it. */
  empty?: boolean;
}) {
  if (empty) return <div className="verdict" role="status" />;
  return (
    <div className={`verdict ${preview.legal ? "ok" : "bad"}`} role="status">
      {preview.legal ? (
        <>
          <span className="tag">legal &mdash; </span>
          {preview.words}
        </>
      ) : (
        <>
          <span className="tag">not offered &mdash; </span>
          adjust the selection
          {faiths && <div className="faiths">{faiths}</div>}
        </>
      )}
    </div>
  );
}

/** One of Chinatown's nine exchanges: the spend, an arrow and the gain as
 *  tokens, the payload's words, and a minus, the count and a plus. The
 *  pictures and the arrow are ONE hidden wrapper, so the words are the row's
 *  single reading. A plus is live exactly where the payload says one more can
 *  be added (`can_add`) and a minus where something is chosen (`count`); each
 *  is `data-unavailable` by what the payload said, and merely disabled while a
 *  request is in flight. A plus sends the row's number, a minus `back` and it. */
function Exchange({
  row,
  art,
  busy,
  onSend,
}: {
  row: ChinatownRow;
  art: ArtManifest | null;
  busy: boolean;
  onSend: (line: string) => void;
}) {
  return (
    <div className={`ex${row.count > 0 ? " on" : ""}`} data-number={row.number}>
      <span className="what">
        <span className="pics" aria-hidden="true">
          <Tokens good={row.spend.good} amount={row.spend.amount} art={art} small />
          <span className="arrow">&rarr;</span>
          <Tokens good={row.gain.good} amount={row.gain.amount} art={art} small />
        </span>
        <span className="words">{row.words}</span>
      </span>
      <span className="step">
        <button
          type="button"
          className="minus"
          aria-label={`one fewer: ${row.words}`}
          disabled={busy || row.count === 0}
          data-unavailable={row.count === 0 ? "true" : undefined}
          onClick={() => onSend(`back ${row.number}`)}
        >
          &minus;
        </button>
        <span className="n">{row.count}</span>
        <button
          type="button"
          className="plus"
          aria-label={`one more: ${row.words}`}
          disabled={busy || !row.can_add}
          data-unavailable={row.can_add ? undefined : "true"}
          onClick={() => onSend(String(row.number))}
        >
          +
        </button>
      </span>
    </div>
  );
}

/** Confirm and Cancel for a chooser. Confirm is live exactly when the preview
 *  is legal; the server still decides, since `confirm` commits only a listed
 *  move. **`data-unavailable` is what the PAYLOAD said** (the preview is not
 *  legal) and not that a request is in flight, so a walk can tell a chooser
 *  that is busy from one that cannot be confirmed; Cancel is never unavailable. */
function ChooserControls({
  legal,
  busy,
  onSend,
}: {
  legal: boolean;
  busy: boolean;
  onSend: (line: string) => void;
}) {
  return (
    <div className="controls">
      <button
        type="button"
        className="primary"
        disabled={busy || !legal}
        data-unavailable={legal ? undefined : "true"}
        onClick={() => onSend("confirm")}
      >
        Confirm
      </button>
      <button type="button" className="ghost" disabled={busy} onClick={() => onSend("cancel")}>
        Cancel
      </button>
    </div>
  );
}

/** An area's printed name, from the board already on the frame. */
function areaName(state: ClientState, labels: Labels | null, area: number): string {
  const tile = state.frame.board.tiles.find((t) => t.area === area);
  if (!tile) return "";
  return labels?.area_types[tile.area_type] ?? tile.area_type;
}

function asking(state: ClientState, labels: Labels | null): string {
  if (state.over) return "The game is over.";
  const d = state.frame.decision;
  const phaseKey = d.phase_chain[d.current_phase];
  const phase = (phaseKey && labels?.phases[phaseKey]) || phaseKey || "";
  // A queued choice outranks the phase, exactly as `deciding_line` has it.
  const choice = decidingWords(d.deciding);
  if (choice && (state.prompt.kind === "options" || state.prompt.kind === "pick")) {
    return `You are deciding — ${choice}`;
  }
  switch (state.prompt.kind) {
    case "pick":
      return state.prompt.title;
    case "retreat":
      return "Guided retreat — how many to recover.";
    case "place_a":
      return "Placement — Option A.";
    case "place_b":
      return "Placement — Option B.";
    case "church":
      return "The Church — what do you pay?";
    case "chinatown": {
      const { max_exchanges: max, chosen } = state.prompt;
      return `Chinatown — up to ${max} exchange${max === 1 ? "" : "s"} · ${chosen} of ${max} chosen`;
    }
    case "reveal":
      return "A card reveal is pending (live mode).";
    case "locked":
      return "This game is locked.";
    case "watching":
      return `Seat ${state.prompt.to_move} is deciding.`;
    default:
      // `your_turn` is now true whenever a human seat is to move, because the
      // payload is built for the acting seat. The distinction that remains is
      // WHICH seat, which matters at 3P and 4P where one operator drives
      // several.
      return state.your_turn
        ? `Seat ${state.seat} — ${phase}.`
        : `Seat ${state.to_move} is deciding — ${phase}.`;
  }
}
