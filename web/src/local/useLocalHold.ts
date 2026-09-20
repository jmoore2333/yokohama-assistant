// Who owns the local game on screen (spec §2.2, §4; R-6.5b.5).
//
// **The hold is taken from the ROUTE's id, before anything is read** (P-5).
// A cold `GET /api/games/{id}` on a local game PUMPS the bot — the module
// plays the trailing run on the way into the screen — so a read taken while
// another tab holds the game changes a record this tab does not own. Ownership
// therefore cannot be derived from a `ClientState` that has already been
// fetched: by then the damage is the fetch.
//
// **Leaving RELEASES.** The page holds the lock for as long as the game screen
// is mounted, which is what the Worker's callback promise is kept open for, so
// navigating to the lobby or to another table frees the game for the other tab
// without closing anything.

import { useCallback, useEffect, useState } from "react";
import { hold, onLost, release, takeover } from "./client";
import { isLocal } from "./ids";

/** What this tab may do with the game the route names.
 *
 *  `"none"` is every screen that is not a local game — and also the instant
 *  before the lock has answered, which is why the page waits for `"held"`
 *  rather than for "not elsewhere". */
export type LockState = "none" | "held" | "elsewhere" | "lost";

export interface Hold {
  state: LockState;
  /** Take it anyway — Take over from the second tab, Take back from the one
   *  that lost it. One message and one state: the lock is STOLEN, and the
   *  Worker rebuilds its store from IndexedDB before it answers a call for the
   *  game, so this tab starts from the last state either tab acknowledged. */
  takeover: () => Promise<void>;
}

export function useLocalHold(id: string | null): Hold {
  const [state, setState] = useState<LockState>("none");

  useEffect(() => {
    if (id === null || !isLocal(id)) {
      setState("none");
      return;
    }
    // **The previous game's answer is not this one's.** Without this a switch
    // from a game this tab holds to one another tab has open would read
    // `"held"` until the new ask came back — and the screen would fetch.
    setState("none");
    let live = true;
    void hold(id)
      .then((ok) => {
        if (live) setState(ok ? "held" : "elsewhere");
      })
      .catch(() => {
        // The local host could not be asked at all — a Worker that will not
        // start, a module that will not load. `client.ts` has already told
        // every fault listener in its own words, and the first screen is what
        // shows them; leaving this at `"none"` is what keeps the page from
        // reading a game it was never given.
        if (live) setState("none");
      });
    const off = onLost((lost) => {
      if (lost === id && live) setState("lost");
    });
    return () => {
      live = false;
      off();
      // **The release is fire-and-forget on purpose**: a screen being torn
      // down has nothing to do with the answer, and an await here would be an
      // unhandled rejection whenever the Worker is already gone.
      void release(id).catch(() => undefined);
    };
  }, [id]);

  const take = useCallback(async () => {
    if (id === null) return;
    await takeover(id);
    setState("held");
  }, [id]);

  return { state, takeover: take };
}
