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
 *  `vite.config.ts`, read by `projectLink.ts` alone. */
declare const __YOKOHAMA_PROJECT_URL__: string | null;
