// What TypeScript reads for `corpus-walk.mjs` (the pool's `test/walk.ts` and
// `test/corpus.test.ts` import it): an `.mjs` import reads no `.d.ts`
// (TS7016), so the declarations sit beside it as `.d.mts`. The module's own
// comments say what each is.

export declare const HOST_FIELDS: Readonly<Record<string, readonly string[]>>;

/** One row in its place: the script it is from, and its index there. */
export interface Ordered<R> {
  script: "walk.json" | "walk-host.json" | "archive-walk.json";
  index: number;
  row: R;
}

export declare function order<W, H extends { after: number }, A>(
  walk: readonly W[],
  host: readonly H[],
  archive: readonly A[],
): Array<Ordered<W | H | A>>;
