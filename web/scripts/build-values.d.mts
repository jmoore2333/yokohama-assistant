/** `web/scripts/build-values.mjs`'s declaration, for the node project that
 *  type-checks `vite.config.ts` (TypeScript resolves `./build-values.mjs` to
 *  this file under `moduleResolution: "bundler"`). */
export declare const PROJECT_URL_VAR: "YOKOHAMA_PROJECT_URL";
export declare const PROJECT_URL_MAX: number;
export declare const RELEASE_FEED_VAR: "YOKOHAMA_RELEASE_FEED";
export declare const RELEASE_FEED_MAX: number;
export declare const SITE_KEY_VAR: "YOKOHAMA_TURNSTILE_SITEKEY";
export declare const SITE_KEY_MAX: number;
export declare const SITE_URL_VAR: "YOKOHAMA_SITE_URL";
export declare const SITE_URL_MAX: number;
/** The six build values and their caps. */
export declare const BUILD_VALUES: {
  YOKOHAMA_PROJECT_URL: number;
  YOKOHAMA_SITE_BLURB: number;
  YOKOHAMA_SITE_NOTICE: number;
  YOKOHAMA_RELEASE_FEED: number;
  YOKOHAMA_TURNSTILE_SITEKEY: number;
  YOKOHAMA_SITE_URL: number;
};
/** `null` for unset or empty; the value itself when it passes; a thrown
 *  `Error` naming `YOKOHAMA_PROJECT_URL` otherwise. */
export declare function checkProjectUrl(raw: string | undefined): string | null;
/** `null` for unset or empty; the value when it passes; a thrown `Error`
 *  naming `name` for whitespace only, a control character or more than `max`
 *  characters. */
export declare function checkText(name: string, raw: string | undefined, max: number): string | null;
/** The release feed a GitHub project link (`https://github.com/<owner>/<repo>`,
 *  a trailing slash tolerated) implies — that repository's latest-release
 *  endpoint — or `null` for any other link, or none. */
export declare function defaultFeedFor(projectUrl: string | null): string | null;
/** The release feed's three states: unset or empty is `defaultFeedFor(projectUrl)`,
 *  the literal `off` is `null`, anything else is that URL (`https://` only, at
 *  most 300 characters) or a thrown `Error` naming `YOKOHAMA_RELEASE_FEED`. */
export declare function checkReleaseFeed(raw: string | undefined, projectUrl: string | null): string | null;
/** `null` for unset or empty; the key when it is plain text of at most 100
 *  characters; a thrown `Error` naming `YOKOHAMA_TURNSTILE_SITEKEY` otherwise. */
export declare function checkSiteKey(raw: string | undefined): string | null;
/** `null` for unset or empty; the origin when the value is an `https://` origin
 *  with no path beyond a trailing `/` (the trailing `/` taken off); a thrown
 *  `Error` naming `YOKOHAMA_SITE_URL` otherwise. */
export declare function checkSiteUrl(raw: string | undefined): string | null;
