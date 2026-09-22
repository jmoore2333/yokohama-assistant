// **The install's verification and the activate sweep's retention rule**
// (M6.5b2, spec §2.3), as pure functions for `sw.ts` to call.
//
// Same reason as `swRoute.ts`: neither decision can be read off a running
// browser. An install that accepted a body it did not digest, and an activate
// that deleted the generation an open tab is still running from, both look like
// a healthy release right up until they do not.

import { CACHE_PREFIX } from "./precache";

/** How long ONE entry may take before the install gives up on it.
 *
 *  **"Bounded" is a number** (spec §2.3). A request that never settles neither
 *  rejects nor resolves, so without this the door says "Preparing to play
 *  offline" for ever — which is the one outcome worse than a failure, because
 *  there is nothing on screen to act on. */
export const FETCH_TIMEOUT_MS = 15_000;

/** …and how long the WHOLE install may take.
 *
 *  A per-request timeout alone bounds nothing: ~150 entries each just inside
 *  the limit is forty minutes of "installing". Two minutes is generous for a
 *  ~10 MB release on a slow connection and short enough that a failure is
 *  visible while the player is still there. */
export const INSTALL_DEADLINE_MS = 120_000;

/** How many entries are fetched at once.
 *
 *  Sequential would be ~150 round trips; unbounded would open ~150 sockets at
 *  once on a phone. Six is the browsers' own per-host default and needs no
 *  justification beyond matching what the network is going to do anyway. */
export const INSTALL_CONCURRENCY = 6;

/**
 * A digest as lowercase hex, which is the form [`checkDigest`] compares and the
 * form the build writes.
 *
 * **The zero padding is the whole of it.** `toString(16)` on a byte under 16
 * gives one character, so a digest missing its padding is shorter than 64
 * characters and — far worse — two different byte strings can render the same.
 */
export function hex(buf: ArrayBuffer): string {
  let out = "";
  for (const b of new Uint8Array(buf)) out += b.toString(16).padStart(2, "0");
  return out;
}

/** The media type's essence: what is left once the parameters are dropped. */
function essence(contentType: string | null): string {
  return (contentType ?? "").split(";")[0]!.trim().toLowerCase();
}

/**
 * Whether a response's HEAD is the one this release asked for, or the sentence
 * saying why not.
 *
 * **Exactly 200, and never `res.ok`.** A 206 is a partial body that would cache
 * as a whole one; a 304 has no body at all; and a redirect that ended at a
 * sign-in page is a 200 of the wrong type, which is what the type check is for.
 * `redirect: "error"` at the fetch is the other half of that pair.
 */
export function checkHead(
  status: number,
  contentType: string | null,
  expected: string,
): string | null {
  if (status !== 200) return `answered ${status}`;
  const got = essence(contentType);
  if (got !== expected) return `answered ${got || "no content type"}, not ${expected}`;
  return null;
}

/**
 * Whether the body is the one this release was built from.
 *
 * **Not redundant with `cache: "reload"`, and each catches what the other
 * misses** (spec §2.3). `reload` is for the art, whose URLs are stable rather
 * than hashed and which the server serves `max-age=3600`: without it an install
 * minutes after a re-cut freezes the hour-old picture under the new release's
 * name. This is for a deployment that changes between the first fetch and the
 * last, where every response is a valid 200 of the right type and they come
 * from two different releases.
 */
export function checkDigest(actual: string, expected: string): string | null {
  return actual === expected ? null : `is ${actual}, not the release's ${expected}`;
}

/** One Cache Storage generation, as the sweep reads it. */
export interface Generation {
  readonly name: string;
  /** When it was installed, from the stamp it wrote into itself. `0` for a
   *  generation written before that stamp existed — which reads as the oldest,
   *  which is what it is. */
  readonly createdAt: number;
}

/**
 * Which generations `activate` deletes: everything of OURS older than the
 * previous one.
 *
 * **The previous generation is RETAINED** (spec §2.3), because an open tab
 * still running the old bundle asks for its own hashed module by name and
 * deleting it out from under that tab is the multi-tab failure the whole update
 * policy exists to avoid. Such a tab really does exist until it is closed — and
 * it is controlled by the NEW worker, not the old one: `skipWaiting()` is
 * registration-wide, so the click in one tab hands every tab the old worker
 * controlled to the new one, `clients.claim()` or not. It is served its old
 * names out of this generation by `sw.ts`'s `fromRetained`, reading the caches
 * [`retainedGenerations`] names (PR #37 F2, R-F2). Before that fix nothing read
 * it, and the retention kept bytes no request could reach.
 *
 * It names only caches under [`CACHE_PREFIX`]. The Cache Storage of an origin
 * is shared, so a sweep written as "delete everything that is not mine" would
 * take out anything else the origin ever put there.
 */
export function generationsToDelete(mine: string, generations: readonly Generation[]): string[] {
  const others = generations
    .filter((g) => g.name.startsWith(CACHE_PREFIX) && g.name !== mine)
    // Newest first, and by NAME on a tie so two runs of the sweep agree about
    // which one survived.
    .sort((a, b) => b.createdAt - a.createdAt || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  return others.slice(1).map((g) => g.name);
}

/**
 * Which caches the fetch handler may answer a path THIS release does not list
 * from: every generation of OURS other than this release's own, in the order
 * the platform listed them (PR #37 F2, R-F2).
 *
 * **The reader of the generation [`generationsToDelete`] retains.** A tab that
 * did not click the update offer is handed to the new worker still running the
 * old bundle, and asks for the old release's hashed names; this release does
 * not list them, and the previous generation holds them.
 *
 * **The prefix again, for the sweep's reason read the other way round.** The
 * origin's Cache Storage is shared, so a read of "every cache that is not mine"
 * would serve whatever else the origin ever put there as if a release of ours
 * had fetched it and checked its digest.
 */
export function retainedGenerations(mine: string, names: readonly string[]): string[] {
  return names.filter((name) => name.startsWith(CACHE_PREFIX) && name !== mine);
}
