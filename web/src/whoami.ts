// Who the SERVER thinks we are — the one read the whole app routes on.
//
// Deliberately not "who did this browser store". `api.ts` holds the stored
// name and attaches it as a header; what that header is WORTH is the server's
// answer, and only the server knows it. Under `cf-access` and `worker` the
// browser holds nothing at all and the identity arrives from an edge it cannot
// see, so a client that decided its own screen from storage would show the
// sign-in form to a player whose Access session is perfectly good — and would
// show the game to one whose session has expired.

import { useCallback, useEffect, useRef, useState } from "react";
import { api, ApiError, clearClaimedUser, clearDisplayName, isSessionExpired } from "./api";
import type { Channel } from "./route";
import type { Me } from "./types";

/** How the door's own read failed (M6.5c2, spec §2.1's discriminators):
 *  `challenge` — a DOOR refused with its own challenge: Access's expired
 *  session, or a provider-signed Worker's "sign in" (the auth cycle's
 *  `SIGN_IN_NEEDED`) — state A;
 *  `network` — nothing answered at all, a network error or offline (state C);
 *  `refused` — the server answered with a refusal of its own. */
export type DoorFailure = "challenge" | "network" | "refused";

/** What a failed `/api/me` was. An `ApiError` is an answer — the server's or
 *  the edge's — and anything else is `fetch` failing with none: `api.ts`'s
 *  `call` turns every HTTP status into an `ApiError`. */
export function failureOf(e: unknown): DoorFailure {
  if (!(e instanceof ApiError)) return "network";
  return isSessionExpired(e) ? "challenge" : "refused";
}

/** Access's own logout (the auth cycle, Task 6; ruling R-41): where a sign-out
 *  goes when a `cf-access` door answers `POST /api/leave` 404 — the NAS, whose
 *  layer has no such route this cycle and which sits behind Access (B10's one
 *  rule). Access's edge serves the path; under Access the Worker's own
 *  `/api/leave` answer names it too (`worker/src/index.ts`'s `ACCESS_LOGOUT`). */
export const ACCESS_LOGOUT = "/cdn-cgi/access/logout";

/** The door, as the app reads and moves it. */
export interface Door {
  /** `null` until `/api/me` answers — a real third state, not "signed out". */
  me: Me | null;
  /** `/api/me` itself refused. */
  error: string | null;
  /** HOW `/api/me` failed, beside `error` — what tells state A from C from a
   *  plain refusal (spec §2.1). `null` whenever `error` is. */
  failure: DoorFailure | null;
  /** WHY the player is back at the door, in the server's own words, or `null`
   *  when they asked to be here. It outlives the screen that caught the
   *  refusal, which is the whole reason it is held here: a 401 unmounts that
   *  screen, and a player who arrives at a bare form has been told nothing. */
  notice: string | null;
  /** Ask `/api/me` again. */
  refresh: () => void;
  /** The player asked to sign out: the display name is dropped at once — it
   *  labels whoever signs in next — and the server is asked where to go to end
   *  the session (`POST /api/leave`) and the browser goes there; a `cf-access`
   *  door that answers 404 — the NAS — goes to Access's own logout
   *  ([`ACCESS_LOGOUT`]); anywhere else it answers nowhere — the open door's
   *  404 — the stored name is dropped and `/api/me` asked again, the sign-out
   *  the open door always had. */
  signOut: () => void;
  /** The server refused with a 401: drop the name, carry the sentence, re-ask. */
  signedOut: (sentence: string) => void;
}

/** `/api/me`, on entering the MULTIPLAYER channel and again on demand.
 *
 *  **`me` is `null` until it answers, and that is a THIRD state, not a default
 *  of "signed out".** The door says only "Connecting to the server…" while it
 *  is null (state D, `Door.tsx`): a default either way flashes the wrong screen
 *  at half the players for one round trip.
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
  const [failure, setFailure] = useState<DoorFailure | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  /** **The player asked to sign out, and the server has not named anybody
   *  since** (M6.5c2 cycle D, dossier row 3). `signOut` drops the stored name
   *  and re-asks, but `me` goes on naming the old player until that answer
   *  lands — so a request a screen sends in that window goes out with no name
   *  and is refused 401, and `App` hands the sentence to `signedOut`. That
   *  refusal is the sign-out itself, not news: while this is set `signedOut`
   *  does nothing. Released by the first read that names somebody, after
   *  which a 401 is a real one again. A ref, because nothing is drawn from it. */
  const leaving = useRef(false);
  useEffect(() => {
    if (channel !== "multiplayer") return;
    let live = true;
    // A refusal on an earlier read is not a fact about this one. Cleared here
    // rather than in `refresh`, so the one place that starts a read is also the
    // one place that forgets the last read's complaint.
    setError(null);
    setFailure(null);
    api
      .me()
      .then((m) => {
        if (!live) return;
        if (m.user !== null) leaving.current = false;
        setMe(m);
      })
      .catch((e) => {
        if (!live) return;
        setError(e instanceof ApiError ? e.message : String(e));
        setFailure(failureOf(e));
      });
    return () => {
      live = false;
    };
  }, [tick, channel]);
  const refresh = useCallback(() => setTick((t) => t + 1), []);
  /** The door the server last named — what `signOut`'s 404 is read against. */
  const mode = me?.mode ?? null;
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
    failure,
    notice,
    refresh,
    // Asked for, so the door says nothing — a notice left standing from an
    // earlier refusal would explain a screen the player chose.
    //
    // **The SERVER ends the session it holds** (the auth cycle, Task 6; ruling
    // R-37): `POST /api/leave` answers where to go — the issuer's logout under
    // a sign-in arm, its answer clearing the session cookie; Access's own
    // `/cdn-cgi/access/logout` under Access — and the navigation IS the
    // sign-out, so nothing is asked again. **Its failure path is the open
    // door's sign-out exactly**: there is no server session there, the route is
    // the layer's 404, and the stored name is dropped and `/api/me` re-read. A
    // reply naming nowhere is that path too. `leaving` is set at the click, so
    // a 401 that lands while the leave is out is the sign-out, not news.
    // **The one exception is the NAS** (ruling R-41): its layer answers the
    // route 404 too, but it sits behind Access, so re-reading `/api/me` would
    // name the same player again — a sign-out that visibly did nothing. A 404
    // under a `cf-access` door goes to Access's own logout instead; any other
    // failure there (nothing answered, a 5xx) keeps the open door's path.
    //
    // **The display name goes at the click** (the auth cycle, Task 6 fix round
    // 1, review M-3): it labels whoever signs in next on this browser, and
    // behind an edge door the sign-out is a navigation, so nothing after the
    // answer runs. Dropped HERE, in the one wiring every sign-out control
    // calls — the game menu's button and the lobby corner's alike.
    signOut: useCallback(() => {
      leaving.current = true;
      clearDisplayName();
      void api.leave().then(
        (reply) => {
          if (typeof reply?.leave === "string" && reply.leave !== "") window.location.assign(reply.leave);
          else leave(null);
        },
        (e: unknown) => {
          if (e instanceof ApiError && e.status === 404 && mode === "cf-access") window.location.assign(ACCESS_LOGOUT);
          else leave(null);
        },
      );
    }, [leave, mode]),
    // Ignored while a sign-out that was asked for is still being read back:
    // the name is already dropped and the re-read already out (`leaving`).
    signedOut: useCallback(
      (sentence: string) => {
        if (leaving.current) return;
        leave(sentence);
      },
      [leave],
    ),
  };
}

/** What the OPERATOR reads when the server names nobody under a door set in
 *  front of it (state B, spec §2.1): which header was expected, and how a
 *  player was meant to arrive. It sits in the card's "For the operator" fold,
 *  so a player reads the sentence meant for them first. The open door never
 *  reaches here: it has the name step. */
export function doorSentence(mode: Exclude<Me["mode"], "open">): string {
  return mode === "cf-access"
    ? "This server expects Cloudflare Access to name each player (Cf-Access-Authenticated-User-Email), and nothing arrived. Open it through its public hostname, or see the self-host guide on identity."
    : "This server expects the edge Worker to name each player (X-Yokohama-User), and nothing arrived. Open it through its public hostname.";
}
