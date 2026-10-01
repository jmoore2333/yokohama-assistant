// What the server says about WHO IS ASKING, drawn as the lobby's content (M6.5b,
// spec §2.5; M6.5c2, R-6.5c2.4, spec §2.1; mocks 03 and 05).
//
// **The door's answers are STATES, each a card with one thing to do** — they
// were sentences until M6.5c2 (R-6.5c2.4). What decides each, in the order it
// is tested, and the order is the rule:
//
//   A. `/api/me` met the EDGE's challenge: Cloudflare Access saying the session
//      in front of the server has expired (`WWW-Authenticate:
//      Cloudflare-Access`, `ACCESS_CHALLENGE`). FIRST, because the Access
//      application covers the whole hostname, `/api/health` included — an
//      expired session fails both reads, and the test below would otherwise
//      call a door the player can walk through "can't reach".
//   C. Nothing answered: `/api/me` failed with no HTTP answer at all, or the
//      health read failed while `/api/me` has not answered (the `unreachable`
//      rule `App.tsx` held until M6.5c2, moved here whole).
//   —  The server REFUSED `/api/me` in words of its own: those words, through
//      `Refusal`, inside a card.
//   D. Nothing has answered yet: "Connecting to the server…" over skeleton rows.
//      **The DOOR's alone**: a signed-in lobby waiting for its lists has passed
//      `/api/me`, and "connecting" would be a false sentence about a server that
//      has already answered.
//   —  The OPEN door, and no name yet: the name step (`SignIn`).
//   B. The server answered, and no identity reached it, under a door set in
//      front of it (`cf-access`, `worker`) — a non-email header included, which
//      the mode rule reads as absent (spec §2.1).
//
// State E — multiplayer off — is the front door's (`FrontDoor.tsx`).
//
// **Every card titles itself `<h3>`**: the door is drawn under the lobby's band,
// whose last crumb is the page's `<h2>` (`Band.tsx`).

import type { ReactNode } from "react";
import { Refusal } from "./Refusal";
import { SignIn } from "./SignIn";
import { doorSentence, type Door as DoorState } from "./whoami";

/** A state's card: its coloured band, its title, and what to do. */
function StateCard({ band, title, children }: { band: "vp" | "warn" | "plain"; title: string; children: ReactNode }) {
  return (
    <section className="raised-card state-card">
      <div className={band === "plain" ? "card-band" : `card-band ${band}`} aria-hidden="true" />
      <div className="card-in">
        <h3>{title}</h3>
        {children}
      </div>
    </section>
  );
}

/** State C: nothing answered. Exported because the SIGNED-IN lobby draws it
 *  too, when its own health read fails (`Lobby.tsx`). */
export function Unreachable({ onRetry }: { onRetry: () => void }) {
  return (
    <StateCard band="plain" title="Can't reach the server">
      <p className="lede">
        The lobby needs a connection. Your games on this device still work, including new games against
        the bot.
      </p>
      <div className="state-actions">
        <button type="button" className="ghost" onClick={onRetry}>
          Try again
        </button>
        <a href="#/">Games on this device</a>
      </div>
    </StateCard>
  );
}

/** State D: nothing has answered yet — the one true sentence, over skeleton
 *  rows where the lists will be. */
function Connecting() {
  return (
    <div className="connecting-state">
      <p className="connecting-line" role="status">
        <span className="connecting-dots" aria-hidden="true">
          <i />
          <i />
          <i />
        </span>
        Connecting to the server&hellip;
      </p>
      <div className="skeleton-rows" aria-hidden="true">
        <div />
        <div />
        <div />
      </div>
    </div>
  );
}

export function Door({
  door,
  serverUp,
  onSignedIn,
  onRetry,
}: {
  door: DoorState;
  /** Whether `GET /api/health` landed — `null` until it answers (App's own). */
  serverUp: boolean | null;
  /** The name was typed and stored: ask `/api/me` again rather than deciding
   *  here that this browser is now that person. */
  onSignedIn: () => void;
  /** "Try again": both reads, again (App's `retry`). */
  onRetry: () => void;
}) {
  if (door.failure === "challenge") {
    return (
      <StateCard band="vp" title="Your sign-in has run out">
        <p className="lede">Sign in again to get back to your tables. Games on this device are unaffected.</p>
        {/* A LINK and never a fetch (`Refusal.tsx`'s reason): passing `/enter`
            is what mints the cookie every later request carries. */}
        <a className="primary-link" href="/enter">
          Sign in again
        </a>
      </StateCard>
    );
  }
  if (door.failure === "network" || (serverUp === false && door.me === null)) {
    return <Unreachable onRetry={onRetry} />;
  }
  if (door.failure === "refused" && door.error !== null) {
    return (
      <StateCard band="warn" title="The server refused">
        <Refusal sentence={door.error} className="error door-error" />
      </StateCard>
    );
  }
  if (door.me === null) return <Connecting />;
  if (door.me.user !== null) return null;
  if (door.me.mode === "open") {
    return <SignIn onSignedIn={onSignedIn} notice={door.notice ?? undefined} />;
  }
  return (
    <StateCard band="warn" title="This server couldn't tell who you are">
      <p className="lede">Try signing in again. If this keeps happening, tell whoever runs the server.</p>
      <a className="primary-link" href="/enter">
        Sign in again
      </a>
      <details className="operator-note">
        <summary>For the operator</summary>
        <p>{doorSentence(door.me.mode)}</p>
      </details>
    </StateCard>
  );
}
