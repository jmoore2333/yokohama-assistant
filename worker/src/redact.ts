/**
 * **What the shell says about a caller, and what it never says** (security S2b, finding D23; decisions D-3, D-4,
 * D-6). The shell's own lines persist in Workers Observability, and fifteen of them carried an address, a folder
 * that spells one, or a refused claim, as Task 7's inventory found them: the two `operator` lines; the seven that
 * log a record key — `logOutcome`'s `archive-*` line, in its `record_key` and its `detail`, `archive-owed-kept`,
 * `archive-stamped`, `held` and the three `archive-owed-dropped`; and the Archive object's own six — two
 * `init-refused`, three `trap` and `archive-count-failed` — whose `id` was the object's name, `archive:<address>`,
 * one `init-refused` also repeating in its `error` a claim the key grammar refused. A digest names the caller well
 * enough to follow a session through the logs and ties to nothing outside them; the same digest is the sockets'
 * per-caller tag (D-4) and the rate limit's caller half (D-3). Pure but for `crypto.subtle`, which is why two of
 * these are async.
 */

/** How many hex characters of the digest name a caller in a log line: 48 bits, the archive segment's own suffix
 *  (`archive::SEGMENT_DIGEST`, security S1c), so a redacted key's folder half still equals the folder's suffix. */
export const DIGEST_HEX = 12;

async function sha256Hex(text: string): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
  return [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** The tag a caller's sockets carry and the key their writes are counted under: `u:` and the whole digest, 66
 *  characters for any address (a tag is at most 256; an address at most 254, which with its prefix would be the
 *  limit exactly, and would put the address in the platform's tag store). */
export async function callerTag(user: string): Promise<string> {
  return `u:${await sha256Hex(user)}`;
}

/** The twelve hex a log line names a caller by. */
export async function callerDigest(address: string): Promise<string> {
  return (await sha256Hex(address)).slice(0, DIGEST_HEX);
}

/** A browser save's segment, as `archive::caller_segment` writes it since security S1c: the readable half, `-`,
 *  twelve hex — the readable half in `caller_segment`'s own alphabet (`[a-z0-9._-]`), so a match never runs across a
 *  message to a later hex token (the S2 stack review, lens 1 N-1). Matched wherever `browser/` starts the key, follows a root (`test/`), or follows any character no key
 *  segment holds — a quote or a space, where R2's message spells the key inside a sentence — and EVERY such segment
 *  in a message, since one R2 sentence can name two keys (security S2c, the residual S2b's re-review parked). The
 *  digest is followed by the segment's `/` or by no further hex digit — a prefix a message spells without its
 *  slash, or a segment at the message's end (the S2 stack review, lens 1, Minor 1) — and the slash, when there is
 *  one, is kept as it was. The `g` flag is safe here because only `replace` reads this expression (never `test` or
 *  `exec`, which would carry `lastIndex` between calls). */
const SEGMENT = /(^|[^A-Za-z0-9._-])browser\/[a-z0-9._-]*-([0-9a-f]{12})(\/|(?![0-9a-f]))/g;

/** `key` with the segment's readable half dropped — `browser/<12 hex>/…` — so a line still names the folder and
 *  not the address it spells. A key with no such segment (a server game's; a legacy folder) is returned as it is. */
export function redactKey(key: string): string {
  return key.replace(SEGMENT, "$1browser/$2$3");
}
