/** `web/scripts/presence.mjs`'s declaration, for the node project that
 *  type-checks `vite.config.ts` and the presence cells (TypeScript resolves
 *  `./presence.mjs` to this file under `moduleResolution: "bundler"`). */

/** The checked build values the presence is written from; `null` is unset. */
export interface PresenceValues {
  siteUrl: string | null;
  blurb: string | null;
  notice: string | null;
  projectUrl: string | null;
}
/** The description when no blurb is set. */
export declare const DEFAULT_DESCRIPTION: string;
/** The picture a link card shows: the app's own 512 px icon, root-relative. */
export declare const OG_IMAGE_PATH: "/icons/icon-512.png";
/** The comment in `web/index.html` the transform replaces with the tags. */
export declare const METADATA_PLACEHOLDER: string;
/** `robots.txt`: allowed but `/api/`, `/enter` and `/cdn-cgi/`; the sitemap's
 *  address only with the site's. */
export declare function robotsTxt(values: Pick<PresenceValues, "siteUrl">): string;
/** `llms.txt`: the repository section only with the project link. */
export declare function llmsTxt(values: Pick<PresenceValues, "blurb" | "notice" | "projectUrl">): string;
/** `sitemap.xml`: the one page, `<siteUrl>/`. */
export declare function sitemapXml(values: { siteUrl: string }): string;
/** The head markup, one tag per line: the canonical link and `og:url` only
 *  with the site's address, `codeRepository` only with the project link. */
export declare function metadataTags(values: Pick<PresenceValues, "siteUrl" | "blurb" | "projectUrl">): string;
/** The shell with its metadata in place of its one placeholder comment; a
 *  shell with none, or two, is a thrown `Error`. */
export declare function injectMetadata(
  html: string,
  values: Pick<PresenceValues, "siteUrl" | "blurb" | "projectUrl">,
): string;
/** Writes `robots.txt` and `llms.txt` into `dist`, and `sitemap.xml` only with
 *  the site's address (taking a stale one away otherwise); the names written. */
export declare function writePresence(dist: string, values: PresenceValues): string[];
/** The four values, read from an environment through the build's checkers. */
export declare function readValues(env: Record<string, string | undefined>): PresenceValues;
