// What a local game says when this browser's storage could not be read
// (M6.5c2 cycle D, dossier row 14).
//
// **The broken-store face.** A game's acquisition reads IndexedDB — its
// ownership generation, then the rebuild's read of every stored pair — and
// when that fails the Worker answers in the storage's own words rather than
// trapping into a restart that would meet the same store. Nothing of the game
// has been read, so there is no board to draw under it: the lock screen's
// shape (the band and one notice card), the storage's sentence, and Reload.
//
// **Its own file, and that is not tidiness.** Its Reload reloads the page on
// purpose — it asks this browser for its storage again from the start — and
// `GameMenu.test.tsx` reads `App.tsx`'s source for `location.reload` to prove
// the Game menu's way out is NOT a reload. A deliberate one in the same file
// would make that check something to weaken rather than a claim that stays
// true.

import { LobbyBand } from "./Band";
import { Refusal } from "./Refusal";

export function BrokenStore({
  sentence,
  onLeave,
  leaveLabel,
}: {
  /** The Worker's sentence, carried whole: "this browser's storage could not
   *  be read — …". */
  sentence: string;
  /** The lock screen's way out — the lobby, or the door in a build that
   *  serves no lobby (`App`'s `localExit`). */
  onLeave: () => void;
  leaveLabel: string;
}) {
  return (
    <div className="band-screen">
      {/* A LOCAL game's screen: the band names the device and draws no
          identity corner, as the lock screen's does. */}
      <LobbyBand crumbs={["On this device"]} corner={undefined} />
      <div className="setup broken-store">
        <div className="notice-card">
          <h3 className="page-title">This game could not be opened</h3>
          <Refusal sentence={sentence} className="error notice-reason" />
          <div className="notice-actions">
            <button type="button" className="primary" onClick={() => window.location.reload()}>
              Reload
            </button>
            <button type="button" className="quiet-link" onClick={onLeave}>
              <span aria-hidden="true">&larr; </span>
              {leaveLabel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
