// The build's values (M6.5c2, R-6.5c2.6 and R-6.5c2.40; spec §2.3; cycle F,
// R-FP2): the door's project link, the blurb under the title and the footer's
// notice, and — cycle F's — the release feed, the Turnstile site key and this
// build's own version.
//
// **A build value, and never a request**: the door asks the server nothing
// (R-6.5b2.7), and a run-time value would mean rewriting `index.html` at
// start-up, which the precache's digest check refuses. `vite.config.ts` checks
// the variables (`web/scripts/build-values.mjs`) and defines them; this module
// is their one reader.

export type ProjectLink = { href: string; text: string };

/** A checked URL as the door draws it: the address, and its text without the
 *  scheme. Pure, so a cell needs no build. */
export function linkFor(url: string | null): ProjectLink | null {
  if (url === null || url === "") return null;
  return { href: url, text: url.replace(/^https?:\/\//i, "") };
}

/** The build's link, or `null`. */
export function projectLink(): ProjectLink | null {
  return linkFor(__YOKOHAMA_PROJECT_URL__);
}

/** The build's blurb, or `null`. */
export function siteBlurb(): string | null {
  return __YOKOHAMA_SITE_BLURB__;
}

/** The build's footer notice, or `null`. */
export function siteNotice(): string | null {
  return __YOKOHAMA_SITE_NOTICE__;
}

/** The release feed this build reads for the latest release, or `null` — `off`,
 *  or no GitHub project link to derive one from. */
export function releaseFeed(): string | null {
  return __YOKOHAMA_RELEASE_FEED__;
}

/** The Turnstile site key, or `null` where the build set none. */
export function turnstileSiteKey(): string | null {
  return __YOKOHAMA_TURNSTILE_SITEKEY__;
}

/** This build's version, `web/package.json`'s — the release notice's "this
 *  site runs" and the number a feed's tag is compared against. */
export function buildVersion(): string {
  return __YOKOHAMA_VERSION__;
}
