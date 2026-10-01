/** `web/scripts/project-url.mjs`'s declaration, for the node project that
 *  type-checks `vite.config.ts` — TypeScript resolves `./project-url.mjs` to
 *  this file under `moduleResolution: "bundler"` (measured, TypeScript 5.9.3). */
export declare const PROJECT_URL_VAR: "YOKOHAMA_PROJECT_URL";
export declare const PROJECT_URL_MAX: number;
/** `null` for unset or empty; the value itself when it passes; a thrown
 *  `Error` naming `YOKOHAMA_PROJECT_URL` otherwise. */
export declare function checkProjectUrl(raw: string | undefined): string | null;
