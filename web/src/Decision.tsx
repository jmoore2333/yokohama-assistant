import type {
  ClientState,
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
  onSend,
}: {
  state: ClientState;
  labels: Labels | null;
  colors: PlayerColor[];
  busy: boolean;
  onSend: (line: string) => void;
}) {
  const d = state.frame.decision;
  const chain = labels?.phase_chain ?? d.phase_chain;
  const prompt = state.prompt;

  return (
    <div className="decision">
      <div className="chain">
        <span>
          Turn {d.turn_number} &middot; seat {d.seat}
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

      {(prompt.kind === "options" || prompt.kind === "pick") && (
        <div className="options">
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

      {prompt.kind === "retreat" && (
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

      {prompt.kind === "place_a" && (
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

      {prompt.kind === "place_b" && (
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

      {prompt.kind === "game_over" && state.scores && (
        <div className="options">
          {state.scores.map((s) => (
            <div className="option" key={s.seat}>
              <span className="num">{s.seat}</span>
              <span>seat {s.seat}</span>
              <span className="more">{s.total} VP</span>
            </div>
          ))}
        </div>
      )}

      {(d.not_offered.length > 0 || d.not_offered_suppressed > 0) && (
        <div className="near-miss">
          {d.not_offered.map((n) => (
            <div key={n.area}>
              not offered: <b>A{String(n.area).padStart(2, "0")}</b> &mdash;{" "}
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

      {(prompt.kind === "place_a" ||
        prompt.kind === "place_b" ||
        prompt.kind === "pick" ||
        prompt.kind === "retreat") && (
        <div className="controls">
          <button type="button" className="primary" disabled={busy} onClick={() => onSend("confirm")}>
            Confirm
          </button>
          <button type="button" className="ghost" disabled={busy} onClick={() => onSend("cancel")}>
            Cancel
          </button>
        </div>
      )}

      {prompt.kind === "options" && (
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

      {state.messages.length > 0 && (
        <div className="messages">
          {state.messages.map((m, i) => (
            <div key={i} className={/illegal|refus|cannot|not legal|out of range/i.test(m) ? "warn" : ""}>
              {m}
            </div>
          ))}
        </div>
      )}
    </div>
  );
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
