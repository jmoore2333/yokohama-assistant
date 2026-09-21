/// <reference types="vite/client" />

// The build-time values this client reads, declared so the typecheck knows
// them. `vite/client` types `import.meta.env` with an index signature, so an
// undeclared name is `any` rather than an error — which is exactly how a
// misspelt switch ships silently. Naming them here makes each one a property
// with a type.

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
