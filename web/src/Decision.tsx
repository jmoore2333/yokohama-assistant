import type { ClientState, Labels, NotOfferedReason } from "./types";

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
  busy,
  onSend,
}: {
  state: ClientState;
  labels: Labels | null;
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
        </div>
      )}

      {prompt.kind === "place_b" && (
        <div className="messages">
          <div>
            Option B &mdash; click one area on the board for two assistants, then confirm.
            {prompt.selected != null && ` Selected A${String(prompt.selected).padStart(2, "0")}.`}
          </div>
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
    default:
      return state.your_turn
        ? `Your turn — ${phase}.`
        : `Seat ${state.to_move} is deciding — ${phase}.`;
  }
}
