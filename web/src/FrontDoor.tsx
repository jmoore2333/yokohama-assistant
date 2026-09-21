// The front door at `#/` (R-6.5b2.1): two ways on, and everyone sees it.
//
// **Single player stops being a section above the lobby and becomes a channel
// of its own.** Until this cycle the first screen was the lobby with the games
// in this browser stacked on top of it, which meant the first screen belonged
// to the server: signing in was the way past it, and the whole hostname sat
// behind Cloudflare Access, so the very first byte of the shell needed a login.
// A door that draws from this browser alone is what lets the public paths be
// public and the protected ones be protected (spec §2.1).
//
// **This screen asks the server NOTHING, and that is a ruling rather than a
// tidiness** (R-6.5b2.7): a single-player visitor is meant to cost zero Worker
// invocations, so whether the multiplayer entry is drawn is a BUILD value and
// never a request. A fresh profile has nothing remembered, so there is nothing
// to read either. The runtime switch (§3.1) is what ENFORCES; this only informs
// the door, and where a deployment builds the client one way and runs the server
// the other, the link lands on `/enter`'s own 404, whose body says so.
//
// **"Play with others" is an `<a>` and never a `fetch`.** Access answers a
// navigation with a `302` to another origin, which a `fetch` follows and then
// fails opaquely; a navigation is what the browser already knows how to do, and
// passing `/enter` is what mints the token every later request carries.

import { OnThisDevice } from "./OnThisDevice";

/** Whether this BUILD offers the other channel.
 *
 *  Absent is ON, so a deployment that sets nothing serves both — and the one
 *  string that shuts the door is the literal `"false"`, not any falsy value: a
 *  `VITE_MULTIPLAYER=0` is a deployment that has said nothing this client can
 *  act on, and guessing would draw a door for a server that serves one.
 *
 *  A function and not a module constant, so a cell can stub the environment and
 *  read it back; it is called once per render of one screen and costs nothing. */
export function multiplayerOffered(): boolean {
  return import.meta.env.VITE_MULTIPLAYER !== "false";
}

export function FrontDoor({
  multiplayer,
  notice,
}: {
  /** Whether to draw the way on to the server. `multiplayerOffered()`'s answer,
   *  passed as a prop so this component is a screen and not a reader of the
   *  build — which is also what lets both arms be rendered in a cell. */
  multiplayer: boolean;
  /** A sentence from OUTSIDE the door, drawn above both entries.
   *
   *  **A local game that ends, is abandoned or refuses to open lands HERE**,
   *  because this is the screen its siblings are listed on — and a player
   *  returned to a bare door would learn only that their link did nothing. The
   *  lobby carries the same sentence for a table of the server's. */
  notice?: string | null;
}) {
  return (
    <div className="setup front-door">
      {notice && <div className="error">{notice}</div>}
      {/* **The local channel, unchanged from M6.5b except for where it
          stands** (spec §2.1): the games in this browser, read from the module
          in this tab's own Worker, and the button that starts one. It is first
          because it is the entry that needs nothing — a door whose first
          offer required a login would be the screen this cycle exists to
          replace. */}
      <OnThisDevice />
      {/* **The turnstile, or the sentence that says there is none.** Not a
          control that 404s and not a hidden one: under R-6.5b2.4 the
          multiplayer routes do not EXIST, so there is nothing to probe and the
          honest thing on screen is one line. */}
      <section className="saved with-others">
        {multiplayer ? (
          <a className="enter" href="/enter">
            Play with others
          </a>
        ) : (
          <p className="muted">This deployment plays single player only.</p>
        )}
      </section>
    </div>
  );
}
