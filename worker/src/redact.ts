/**
 * **What the shell says about a caller, and what it never says** (security S2b, finding D23; decisions D-3, D-4,
 * D-6). The shell's own lines persist in Workers Observability, and three of them carried an address or a folder
 * that spells one: the two `operator` lines and every `archive-*` line's `record_key`. A digest names the caller
 * well enough to follow a session through the logs and ties to nothing outside them; the same digest is the
 * sockets' per-caller tag (D-4) and the rate limit's caller half (D-3), so a caller's address reaches no platform
 * store but the identity header the objects read. Pure but for `crypto.subtle`, which is why two of these are async.
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
 *  twelve hex. Matched after `browser/` at the key's start or after a root (`test/`). */
const SEGMENT = /(^|\/)browser\/[^/]*-([0-9a-f]{12})\//;

/** `key` with the segment's readable half dropped — `browser/<12 hex>/…` — so a line still names the folder and
 *  not the address it spells. A key with no such segment (a server game's; a legacy folder) is returned as it is. */
export function redactKey(key: string): string {
  return key.replace(SEGMENT, "$1browser/$2/");
}
