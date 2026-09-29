// **The ONE reader of a release's precache list** (M6.5c2 cycle D, Task 9; the
// M6.5c2 spec §2.8's "one parser of the precache list
// `web/scripts/precache.mjs` prints"): the list, read out of a service
// worker's text. Two scripts read one — the assembly and the deploy guard a
// BUILT `sw.js` (`precacheEntriesFrom`, `assemble-assets.mjs`), the deploy's
// readings a DEPLOYED one (`precacheList`, `read-deploy.mjs`) — each through
// this, in its own words.

/** One entry as esbuild prints the substituted `__PRECACHE__` into `sw.js`
 *  (`web/scripts/precache.mjs`): unquoted keys, a quoted path with no escape
 *  in it, a SHA-256 in lowercase hex, and the entry's media type. */
const ENTRY = /\{ path: "([^"\\]*)", digest: "([0-9a-f]{64})", type: "([^"\\]*)" \}/g;

/** Each reader's sentences, kept as they were when each had a parser of its
 *  own: a worker with no entry, and one whose digests outnumber the entries
 *  read. */
const SAID = {
  built: {
    none: "the built sw.js carries no precache entries — is it web/scripts/precache.mjs's output?",
    unpaired: (digests, read) =>
      `the built sw.js carries ${digests} digests but paired ${read} of them with a path — ` +
      "the list's printed shape is not the one this reader knows",
  },
  deployed: {
    none: "the deployed sw.js carries no precache entries — is it web/scripts/precache.mjs's output?",
    unpaired: (digests, read) =>
      `the deployed sw.js carries ${digests} digests but read ${read} entries — its list's shape is not the one this reader knows`,
  },
};

/**
 * The precache list out of a service worker's text — every entry's path,
 * digest and type — for the `built` or the `deployed` `sw.js`, which names it
 * in a refusal. Refuses a worker with no entry, and one carrying a
 * `digest: "` this reader could not read as an entry: the count is its own
 * anti-vacuity, since a list whose printed shape drifted would otherwise read
 * as a shorter list.
 */
export function readPrecacheList(swText, which) {
  const said = SAID[which];
  const entries = [...swText.matchAll(ENTRY)].map(([, path, digest, type]) => ({ path, digest, type }));
  if (entries.length === 0) throw new Error(said.none);
  const digests = swText.split('digest: "').length - 1;
  if (digests !== entries.length) throw new Error(said.unpaired(digests, entries.length));
  return entries;
}
