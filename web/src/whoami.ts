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
import { api, ApiError } from "./api";
import type { Me } from "./types";

/** `/api/me`, once at mount and again on demand.
 *
 *  **`me` is `null` until it answers, and that is a THIRD state, not a default
 *  of "signed out".** The app draws nothing at all while it is null: a default
 *  either way flashes the wrong screen at half the players for one round trip.
 *
 *  `refresh` is how signing out, and a `no signed-in user` refusal from any
 *  other route, reach the screen — the stored name is dropped and this asks the
 *  server again rather than the client deciding it is now nobody. */
export function useMe(): { me: Me | null; error: string | null; refresh: () => void } {
  const [me, setMe] = useState<Me | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  useEffect(() => {
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
  }, [tick]);
  return { me, error, refresh: useCallback(() => setTick((t) => t + 1), []) };
}

/** What the screen says when the server names nobody under a door this browser
 *  cannot knock on.
 *
 *  There is no form to offer: both of these identities are set by an edge in
 *  front of the server, so the only useful thing to say is WHICH header was
 *  expected and how the player was supposed to arrive. `claimed` never reaches
 *  here — it has a screen. */
export function doorSentence(mode: Me["mode"]): string {
  return mode === "cf-access"
    ? "This deployment expects Cloudflare Access to name you (Cf-Access-Authenticated-User-Email), and nothing reached the server. Open the site through its public hostname."
    : "This deployment expects the edge Worker to name you (X-Yokohama-User), and nothing reached the server.";
}
