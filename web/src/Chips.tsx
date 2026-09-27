// The small marks the setup screens share (M6.5c2, spec §2.2): a seat, the
// password a table asks for, and the bot's version.
//
// **Colour means one thing each, as it does everywhere in this client**: a
// seat's dot is the seat's colour, gold is "to move", orange is the password,
// the yen gold outline is the bot. The hues are the stylesheet's.

import { seatClass } from "./Cards";
import type { PlayerColor } from "./types";

/** The padlock, DRAWN (spec §2.2: "not an emoji", which renders differently on
 *  every system and announces as nothing useful). Decoration wherever it is
 *  used — the tag and the action buttons each carry their own words. */
export function Padlock({ size = 11 }: { size?: number }) {
  return (
    <svg
      className="padlock"
      width={size}
      height={size}
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
    >
      <rect x="3" y="7" width="10" height="7.5" rx="1.5" />
      <path d="M5.2 7V5.2a2.8 2.8 0 0 1 5.6 0V7" />
    </svg>
  );
}

/** One seat at a table: a dot in its colour and who is there.
 *
 *  The colour is the dot's accessible NAME (`role="img"`), so a screen reader
 *  hears "Blue, Alice" and the chip's text is the label alone — which is what
 *  a row's words, and the cells that read them, are made of. An open seat has
 *  no dot: nobody's colour sits there yet. */
export function SeatChip({
  color,
  label,
  open = false,
  toMove = false,
}: {
  color: PlayerColor;
  label: string;
  open?: boolean;
  toMove?: boolean;
}) {
  const cls = ["seat-chip", open ? "open" : "", toMove ? "to-move" : ""].filter(Boolean).join(" ");
  return (
    <span className={cls}>
      {!open && <i className={`seat-dot ${seatClass(color)}`} role="img" aria-label={color} />}
      {label}
    </span>
  );
}

/** The table asks for a password (R-6.5c2.7). Named for what it MEANS: the
 *  word "Password" alone reads as a field label. */
export function PasswordTag() {
  return (
    <span className="password-tag" role="img" aria-label="password required">
      <Padlock />
      Password
    </span>
  );
}

/** The bot's version (R-6.5c2.10): a label, never a choice. */
export function VersionTag({ label }: { label: string }) {
  return (
    <span className="version-tag" title="the bot's version">
      {label}
    </span>
  );
}
