// The development door.
//
// It exists under ONE deployment mode. `claimed` is the mode where the server
// believes the `X-Yokohama-Claimed-User` header (spec R-1.2), which is the
// local and LAN-hosted shape; behind Cloudflare Access or the edge Worker the
// identity arrives from in front of the server and this screen is never
// rendered — `App` routes a null user under those modes to a sentence naming
// the header that was supposed to be set.
//
// The name typed here is normalized to the SAME shape the server normalizes to
// (R-1.1), so the seat you are given and the name on your screen are one
// string. A client that stored the raw text would be signed in as
// `Red@Wire.TEST` and seated as `red@wire.test`.

import { useState } from "react";
import { setClaimedUser } from "./api";

/** The server's own R-1.1 shape, restated so the refusal happens here rather
 *  than as a 400 on the first request. Deliberately the same crude rule and not
 *  a stricter one: two different notions of "an email" would refuse names the
 *  server would have accepted. */
const EMAIL = /^[^\s@]+@[^\s@]+$/;

export function SignIn({
  onSignedIn,
  notice,
}: {
  onSignedIn: (email: string) => void;
  /** Why the player is here, in the server's own words, when they did not
   *  choose to be (spec §7.1). A 401 unmounts the screen that caught it, so the
   *  sentence has to arrive with them or it is lost — and a game vanishing into
   *  a bare form is not something anyone can act on. */
  notice?: string;
}) {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const submit = () => {
    const normalized = value.trim().toLowerCase();
    if (!normalized) {
      setError("Type the email you want to play under.");
      return;
    }
    if (!EMAIL.test(normalized)) {
      setError(`${normalized} is not an email`);
      return;
    }
    // **Stored BEFORE the callback, because the callback is what re-reads
    // `/api/me`.** `api.ts` attaches the header from storage on every call, so
    // a refresh that ran first would ask the server who we are while the
    // browser was still holding nobody, and the answer would send us straight
    // back to this screen.
    setClaimedUser(normalized);
    setError(null);
    onSignedIn(normalized);
  };
  return (
    <div className="setup sign-in">
      <h2>Sign in</h2>
      {/* Above the form: the reason you are here is read before the thing it is
          asking you to do. `role="status"` because it appears without the
          player having done anything on this screen. */}
      {notice && (
        <p className="error door-notice" role="status">
          {notice}
        </p>
      )}
      <p className="lede">
        This is a development door: the name you type is the name you play under. Behind Cloudflare
        Access it is still Access that lets you in.
      </p>
      {/* A real form, so Enter in the field signs in. The default is prevented
          because there is nothing to submit TO — the name goes to storage and
          the next request carries it as a header. */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <div className="field">
          <label htmlFor="email">Email</label>
          <input
            id="email"
            type="text"
            autoComplete="email"
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
        </div>
        <button type="submit" className="primary">
          Sign in
        </button>
      </form>
      {error && <div className="error">{error}</div>}
    </div>
  );
}
