// The name step: the open door's one question (M6.5c2, R-6.5c2.3; spec §2.1;
// mock 03).
//
// **It exists under ONE door.** `open` is the mode where the server believes
// the `X-Yokohama-Claimed-User` header — a self-host with no identity provider,
// and this project's own browser walk — and behind Cloudflare Access or the
// edge Worker the identity arrives from in front of the server and this screen
// is never drawn (`Door.tsx`).
//
// **The rule is the server's, and so is the key.** `normalizeName` is pinned
// to `User::parse` over one committed table (`nameRule.ts`), so the header this
// browser sends is exactly the key the server writes seats under. The name as
// TYPED is stored too, as the display name the lobby corner and every seat show.
//
// **Titled `<h3>`**: it is drawn under the lobby's band, whose last crumb is the
// page's `<h2>` (`Band.tsx`).

import { useId, useState } from "react";
import { setClaimedUser, setDisplayName } from "./api";
import { normalizeName } from "./nameRule";
import { Refusal } from "./Refusal";

export function SignIn({
  onSignedIn,
  notice,
}: {
  onSignedIn: (key: string) => void;
  /** Why the player is here, in the server's own words, when they did not
   *  choose to be (spec §7.1). A 401 unmounts the screen that caught it, so the
   *  sentence has to arrive with them or it is lost. */
  notice?: string;
}) {
  const field = useId();
  const rule = useId();
  const refused = useId();
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const submit = () => {
    const name = normalizeName(value);
    if (!name.ok) {
      setError(name.reason);
      return;
    }
    // **Stored BEFORE the callback, because the callback is what re-reads
    // `/api/me`.** `api.ts` attaches the header from storage on every call, so a
    // refresh that ran first would ask the server who we are while the browser
    // was still holding nobody, and the answer would send us straight back here.
    setClaimedUser(name.key);
    setDisplayName(name.display);
    setError(null);
    onSignedIn(name.key);
  };
  return (
    <section className="raised-card state-card name-step">
      <div className="card-band vp" aria-hidden="true" />
      <div className="card-in">
        <h3>Choose a name</h3>
        {/* Above the form: the reason you are here is read before the thing it
            asks you to do. `role="status"` because it appears without the player
            having done anything on this screen. */}
        {notice && (
          <p className="error door-notice" role="status">
            {notice}
          </p>
        )}
        <p className="lede">Other players see it at your tables and in chat.</p>
        {/* A real form, so Enter in the field continues. The default is
            prevented because there is nothing to submit TO — the name goes to
            storage and the next request carries it as a header. */}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <label className="sr-only" htmlFor={field}>
            Name
          </label>
          <input
            id={field}
            type="text"
            autoComplete="nickname"
            aria-describedby={error ? `${rule} ${refused}` : rule}
            aria-invalid={error ? true : undefined}
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
          <p className="hint" id={rule}>
            Letters, numbers, spaces, dots, dashes and underscores. Up to 32 characters.
          </p>
          <button type="submit" className="primary">
            Continue
          </button>
        </form>
        {/* **Announced, and tied to the field** (review M7): `role="alert"` so a
            screen reader says why the name was refused the moment it is, and
            the field is marked invalid and described by the reason as well as
            by the rule. */}
        {error && (
          <div id={refused} role="alert">
            <Refusal sentence={error} />
          </div>
        )}
        <p className="fine">
          This server doesn&rsquo;t check who anyone is, so anyone who can reach it can use any name.
          Play here with people you know. Your tables stay with the name you sat down as, so
          switching names later leaves them behind.
        </p>
      </div>
    </section>
  );
}
