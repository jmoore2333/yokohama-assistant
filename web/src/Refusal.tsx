// A refusal on screen, and the one refusal that comes with a way out of it
// (M6.5b2, spec §2.1 and §4).
//
// **Two 401s mean two different things, and only one of them has an answer the
// player can act on.** `api.ts` tells them apart by the header Access sends —
// a 401 carrying `WWW-Authenticate: Cloudflare-Access` is the EDGE saying the
// session in front of the server has expired, and this API's own `{"error": …}`
// 401 is the SERVER saying nobody is signed in. The first is fixed by passing
// the turnstile once; the second by signing in. So the expired-session sentence
// is drawn with a link to `/enter` beside it and every other sentence is drawn
// as it always was.
//
// **A LINK and never a fetch.** Access answers a navigation with a `302` to
// another origin, which a `fetch` follows and then fails opaquely; a navigation
// is what the browser already knows how to do, and passing `/enter` is what
// mints the cookie every later request carries.

import { ACCESS_CHALLENGE } from "./api";

export function Refusal({
  sentence,
  className = "error",
}: {
  /** What refused, in the server's own words — or, for the edge, in
   *  [`ACCESS_CHALLENGE`]'s. */
  sentence: string;
  /** The class the host already drew this strip with, so moving a site to this
   *  component changes no styling. */
  className?: string;
}) {
  return (
    <div className={className}>
      {sentence}
      {sentence === ACCESS_CHALLENGE && (
        <>
          {" "}
          {/* No class: `.enter` is the front door's own big link and is scoped
              to that screen. Here the sentence is a line in an error strip and
              the link reads as one. */}
          <a href="/enter">Sign in again</a>
        </>
      )}
    </div>
  );
}
