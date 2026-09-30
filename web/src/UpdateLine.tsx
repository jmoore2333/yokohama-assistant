// The one line that offers a new release (M6.5c2 cycle F, R-F3, R-FP14; spec
// §2.1).
//
// **One component, drawn in three places**: the door's device foot (inside
// `Readiness`, `OnThisDevice.tsx`), the lobby's actions and a game's banner
// (R-FP9). Off the door it is the ONLY way a release is taken — nothing
// reloads by itself while someone plays — and on the door it is what a player
// sees in the moment before the door takes the release itself
// (`useDoorAutoApply`), or when a restore or a send is holding that.
//
// **Two states draw it, and the click means a different thing in each.** A
// release WAITING behind the running one is taken: `applyUpdate` posts
// skip-waiting and reloads this tab once the new worker is in charge. A STALE
// tab — one another tab's apply already handed to the new worker, with
// nothing waiting any more — has nothing left to take, and its click is a
// plain reload. The sentence is the same because the fact is: this tab is
// behind, and the click brings it up to date. Where both hold (a newer release
// waits behind the one a stale tab was handed to), the click takes the newer
// one, which reloads the tab as well.

import type { ReadinessStatus } from "./swRegister";

export function UpdateLine({ status }: { status: ReadinessStatus }): React.ReactNode {
  if (!status.updateWaiting && !status.staleTab) return null;
  return (
    <p className="muted update-line">
      A new version is ready.{" "}
      <button
        type="button"
        className="linkish"
        onClick={status.updateWaiting ? status.applyUpdate : status.reload}
      >
        Reload to use it
      </button>
    </p>
  );
}
