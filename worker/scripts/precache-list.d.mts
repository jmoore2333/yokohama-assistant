// What TypeScript reads for `precache-list.mjs` (`web/src/precache.node.test.ts`
// imports it): an `.mjs` import reads no `.d.ts` (TS7016), so the declaration
// sits beside it as `.d.mts`. The module's own comments say what it is.

export declare function readPrecacheList(
  swText: string,
  which: "built" | "deployed",
): { path: string; digest: string; type: string }[];
