// The edge's Managed Challenge, as a same-origin answer shows it (security S3, W-3; design §3.1 step 1).
//
// **One predicate, two bundles.** The page asks it of every answer from the server through `turnstile.ts`'s
// `isEdgeChallenge`, and the service worker asks it of every precache fetch (`sw.ts`'s `installOne`). The worker
// is built apart from the page — `scripts/precache.mjs` bundles `sw.ts` with esbuild, and `tsconfig.sw.json`
// checks it under the WebWorker lib — so this module imports nothing and touches no global: it is the one copy
// both import, where a restatement in the worker kept equal to the page's by a cell would be two.

/** Whether an answer is the EDGE's Managed Challenge: Cloudflare answers a challenged request `403` with
 *  `cf-mitigated: challenge`, readable by a same-origin fetch. Both, and never one of them: a 403 is also this
 *  API's own refusal of a locked table or a stranger's seat, and a release file the host refuses.
 *
 *  `cfMitigated` is the header as `Headers.get` reads it, which has already folded the name's case and trimmed
 *  the value's surrounding whitespace; the value itself is compared exactly. */
export function isEdgeChallengeResponse(status: number, cfMitigated: string | null): boolean {
  return status === 403 && cfMitigated === "challenge";
}
