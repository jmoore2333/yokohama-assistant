/// <reference types="vite/client" />

// The build-time values this client reads, declared so the typecheck knows
// them. `vite/client` types `import.meta.env` with an index signature, so an
// undeclared name is `any` rather than an error — which is exactly how a
// misspelt switch ships silently. Naming them here makes each one a property
// with a type.
//
// **The declaration alone was not enough, which is why `ViteTypeOptions` is
// here.** Vite's own fallback index signature stays in place unless a project
// opts out of it, so `import.meta.env.VITE_MULTIPLAER` still typed as `any` and
// the comment above described a fence that did not exist. `strictImportMetaEnv`
// is Vite's switch for removing it: with it set, a name not declared below is a
// compile error, which is what this file was always meant to say.

interface ViteTypeOptions {
  strictImportMetaEnv: unknown;
}

interface ImportMetaEnv {
  /** Whether this BUILD draws the way on to the server (R-6.5b2.7, spec §2.1).
   *
   *  Absent is ON. The one value that shuts the door is the literal `"false"`;
   *  see `FrontDoor.tsx`'s `multiplayerOffered`, which is the only reader. It is
   *  a build value and never a request, because the door asks the server
   *  nothing — the RUNTIME switch is what enforces. */
  readonly VITE_MULTIPLAYER?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

/** The door's project link (M6.5c2, spec §2.3): the checked
 *  `YOKOHAMA_PROJECT_URL`, or `null` where the build set none — a `define` in
 *  `vite.config.ts`, read by `buildValues.ts` alone. */
declare const __YOKOHAMA_PROJECT_URL__: string | null;
/** The blurb under the door's title: the checked `YOKOHAMA_SITE_BLURB`, or
 *  `null` — read by `buildValues.ts` alone. */
declare const __YOKOHAMA_SITE_BLURB__: string | null;
/** The footer's one-line notice: the checked `YOKOHAMA_SITE_NOTICE`, or
 *  `null` — read by `buildValues.ts` alone. */
declare const __YOKOHAMA_SITE_NOTICE__: string | null;
/** The sentence under every sign-in link (the auth cycle, Task 6; R-A14): the
 *  checked `YOKOHAMA_SIGNIN_NOTICE` — who signs the player in, and what this
 *  site keeps — or `null`, and nothing is drawn. Read by `buildValues.ts`
 *  alone. */
declare const __YOKOHAMA_SIGNIN_NOTICE__: string | null;
/** The release feed the door and the lobby read for the latest release (cycle
 *  F, R-F2): the checked `YOKOHAMA_RELEASE_FEED` — a URL, or one derived from a
 *  GitHub project link — or `null` for `off` and for no feed. Read by
 *  `buildValues.ts` alone. */
declare const __YOKOHAMA_RELEASE_FEED__: string | null;
/** The Turnstile site key (cycle F, R-F6): the checked
 *  `YOKOHAMA_TURNSTILE_SITEKEY`, or `null` — blank draws the plain link. Read
 *  by `buildValues.ts` alone. */
declare const __YOKOHAMA_TURNSTILE_SITEKEY__: string | null;
/** The deployment's canonical address (cycle F, R-F12): the checked
 *  `YOKOHAMA_SITE_URL` — an `https://` origin with no trailing slash — or
 *  `null` where the build set none. The sitemap, `<link rel="canonical">` and
 *  `og:url` are written from it at build; read by `buildValues.ts` alone. */
declare const __YOKOHAMA_SITE_URL__: string | null;
/** This build's own version: `web/package.json`'s `version`, the number the
 *  release notice compares the feed's tag against. Read by `buildValues.ts`
 *  alone. */
declare const __YOKOHAMA_VERSION__: string;
