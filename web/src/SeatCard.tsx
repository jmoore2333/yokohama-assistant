// One seat at a table that is filling (M6.5c2, spec §2.2; mocks 08 and 09).
//
// The seat's colour is the card's `color` (its seat class), so the band and
// the caption take it from `currentColor` and nothing restates a hue. A card is
// a REGION named by its caption, which is what lets a screen reader — and a
// cell — find "Blue" among four seats.

import type { ReactNode } from "react";
import { seatClass } from "./Cards";
import type { PlayerColor } from "./types";

export function SeatCard({
  color,
  caption,
  who,
  note,
  action,
  open = false,
}: {
  color: PlayerColor;
  /** "Red · first player", "Blue" — shown in small capitals, in the seat's colour. */
  caption: string;
  /** Who is there: a name, "You", "Open seat". */
  who: ReactNode;
  /** One quiet line under it: "seated", "yours if you join". */
  note?: string;
  /** A control for this seat — "Sit here" — or nothing. */
  action?: ReactNode;
  /** Nobody has taken it: drawn dashed, on no ground. */
  open?: boolean;
}) {
  return (
    <section className={`seat-card ${seatClass(color)}${open ? " open" : ""}`} aria-label={caption}>
      <div className="seat-card-band" aria-hidden="true" />
      <div className="seat-card-in">
        <p className="seat-card-caption">{caption}</p>
        <p className="seat-card-who">{who}</p>
        {note && <p className="seat-card-note">{note}</p>}
        {action}
      </div>
    </section>
  );
}
