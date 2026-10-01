// What the server says about WHO IS ASKING, drawn where the server's own part
// of the screen would be (M6.5b, spec §2.5).
//
// **These four answers were four gates until this cycle** (`App.tsx:922-926`):
// each of them returned from `App` above every screen, so a browser whose API
// could not be reached drew nothing at all — not the games in its own
// IndexedDB, not the button that starts one. R-6.5b.8 is that the first screen
// renders from the browser FIRST, so the four bodies moved here and the page
// renders them as CONTENT, inside the area the server owns.
//
// **The bodies themselves did not change, deliberately.** They are the same
// four nodes, in the same order, for the same reasons — the order is the rule
// and it is restated below — because a door that said something new while it
// was being moved would be two changes with one test.

import { Refusal } from "./Refusal";
import { SignIn } from "./SignIn";
import { doorSentence, type Door as DoorState } from "./whoami";

export function Door({
  door,
  onSignedIn,
}: {
  door: DoorState;
  /** The name was typed and stored: ask `/api/me` again rather than deciding
   *  here that this browser is now that person. */
  onSignedIn: () => void;
}) {
  // 1. `/api/me` refused. Say so and offer nothing — a sign-in form over a
  //    server that cannot answer would take a name and lose it.
  //
  //    **Through `Refusal` since M6.5b2**: this route is a protected path now,
  //    so the one refusal a player CAN act on — the edge saying their Access
  //    session expired — arrives here, and it is drawn with the turnstile's
  //    link beside it. Every other sentence reads exactly as it did.
  if (door.error) return <Refusal sentence={door.error} className="setup door-error" />;
  // 2. It has not answered yet. **A line rather than nothing**, which is the
  //    one thing that did change with the move: as a gate this arm drew the
  //    whole page blank, and a default either way flashed the wrong screen at
  //    half the players for one round trip. As content beside "On this
  //    device", the honest word is that the server has not answered yet.
  if (!door.me) return <p className="muted">connecting&hellip;</p>;
  // 3. Nobody is signed in under `claimed` — the one door this browser can
  //    knock on. The form.
  if (door.me.mode === "claimed" && !door.me.user)
    return <SignIn onSignedIn={onSignedIn} notice={door.notice ?? undefined} />;
  // 4. Nobody is signed in under Cloudflare Access or the edge Worker. There
  //    is no form to offer: the identity comes from in front of the server, so
  //    the only useful thing to say is which header was expected.
  if (!door.me.user) return <div className="setup door-error">{doorSentence(door.me.mode)}</div>;
  // The door is open, and the screen behind it is the caller's to draw.
  return null;
}
