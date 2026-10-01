/** `web/scripts/build-values.mjs`'s declaration, for the node project that
 *  type-checks `vite.config.ts` (TypeScript resolves `./build-values.mjs` to
 *  this file under `moduleResolution: "bundler"`). */
export declare const PROJECT_URL_VAR: "YOKOHAMA_PROJECT_URL";
export declare const PROJECT_URL_MAX: number;
/** The three build values and their caps. */
export declare const BUILD_VALUES: {
  YOKOHAMA_PROJECT_URL: number;
  YOKOHAMA_SITE_BLURB: number;
  YOKOHAMA_SITE_NOTICE: number;
};
/** `null` for unset or empty; the value itself when it passes; a thrown
 *  `Error` naming `YOKOHAMA_PROJECT_URL` otherwise. */
export declare function checkProjectUrl(raw: string | undefined): string | null;
/** `null` for unset or empty; the value when it passes; a thrown `Error`
 *  naming `name` for whitespace only, a control character or more than `max`
 *  characters. */
export declare function checkText(name: string, raw: string | undefined, max: number): string | null;
