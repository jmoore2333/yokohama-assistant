// The door's project link (M6.5c2, R-6.5c2.6; spec §2.3).
//
// **A build value, and never a request**: the door asks the server nothing
// (R-6.5b2.7), and a run-time value would mean rewriting `index.html` at
// start-up, which the precache's digest check refuses. `vite.config.ts` checks
// `YOKOHAMA_PROJECT_URL` (`web/scripts/project-url.mjs`) and defines it; this
// module is its one reader.

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
