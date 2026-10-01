// Who the SERVER thinks we are — the one read the whole app routes on.
//
// Deliberately not "who did this browser store". `api.ts` holds the stored
// name and attaches it as a header; what that header is WORTH is the server's
// answer, and only the server knows it. Under `cf-access` and `worker` the
// browser holds nothing at all and the identity arrives from an edge it cannot
// see, so a client that decided its own screen from storage would show the
// sign-in form to a player whose Access session is perfectly good — and would
// show the game to one whose session has expired.

import { useCallback, useEffect, useState } from "react";
import { api, ApiError, clearClaimedUser } from "./api";
import type { Channel } from "./route";
import type { Me } from "./types";

/** The door, as the app reads and moves it. */
export interface Door {
  /** `null` until `/api/me` answers — a real third state, not "signed out". */
  me: Me | null;
  /** `/api/me` itself refused. */
  error: string | null;
  /** WHY the player is back at the door, in the server's own words, or `null`
   *  when they asked to be here. It outlives the screen that caught the
   *  refusal, which is the whole reason it is held here: a 401 unmounts that
   *  screen, and a player who arrives at a bare form has been told nothing. */
  notice: string | null;
  /** Ask `/api/me` again. */
  refresh: () => void;
  /** The player asked to sign out. */
  signOut: () => void;
  /** The server refused with a 401: drop the name, carry the sentence, re-ask. */
  signedOut: (sentence: string) => void;
}

/** `/api/me`, on entering the MULTIPLAYER channel and again on demand.
 *
 *  **`me` is `null` until it answers, and that is a THIRD state, not a default
 *  of "signed out".** The app draws nothing at all while it is null: a default
 *  either way flashes the wrong screen at half the players for one round trip.
 *
 *  **It is not a boot read any more** (M6.5b2, R-6.5b2.7, spec §2.2). It ran on
 *  mount whatever screen this was, which made it a FOURTH request on the front
 *  door — the one an earlier draft of that ruling did not count — and `/api/me`
 *  is a PROTECTED path in the coming deployment, so on the door it would be a
 *  401 on every visit. The door needs no identity: it draws the games in this
 *  browser and a link to the turnstile. So the hook takes the channel and does
 *  nothing on the local one; `me` simply stays at its third state until a
 *  screen that needs it is reached, and is KEPT on a trip back to the door
 *  rather than re-read on the way out and in again.
 *
 *  `refresh` is how signing out, and a `no signed-in user` refusal from any
 *  other route, reach the screen — the stored name is dropped and this asks the
 *  server again rather than the client deciding it is now nobody. */
export function useMe(channel: Channel): Door {
  const [me, setMe] = useState<Me | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  useEffect(() => {
    if (channel !== "multiplayer") return;
    let live = true;
    // A refusal on an earlier read is not a fact about this one. Cleared here
    // rather than in `refresh`, so the one place that starts a read is also the
    // one place that forgets the last read's complaint.
    setError(null);
    api
      .me()
      .then((m) => live && setMe(m))
      .catch((e) => live && setError(e instanceof ApiError ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, [tick, channel]);
  const refresh = useCallback(() => setTick((t) => t + 1), []);
  /** Both ways out differ only in what they leave on the door. The stored name
   *  is dropped either way: `/api/me` reads the very header `api.ts` attaches
   *  from storage, so a name kept here comes straight back and the form would
   *  never appear. */
  const leave = useCallback(
    (why: string | null) => {
      clearClaimedUser();
      setNotice(why);
      refresh();
    },
    [refresh],
  );
  return {
    me,
    error,
    notice,
    refresh,
    // Asked for, so the door says nothing — a notice left standing from an
    // earlier refusal would explain a screen the player chose.
    signOut: useCallback(() => leave(null), [leave]),
    signedOut: useCallback((sentence: string) => leave(sentence), [leave]),
  };
}

/** What the screen says when the server names nobody under a door this browser
 *  cannot knock on.
 *
 *  There is no form to offer: both of these identities are set by an edge in
 *  front of the server, so the only useful thing to say is WHICH header was
 *  expected and how the player was supposed to arrive. `open` never reaches
 *  here — it has a screen. */
export function doorSentence(mode: Me["mode"]): string {
  return mode === "cf-access"
    ? "This deployment expects Cloudflare Access to name you (Cf-Access-Authenticated-User-Email), and nothing reached the server. Open the site through its public hostname."
    : "This deployment expects the edge Worker to name you (X-Yokohama-User), and nothing reached the server.";
}
