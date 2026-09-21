// What a local game says when a write did not survive the lock (spec §2.2).
//
// **Its own file for `ids.ts`'s reason, and that reason alone.** The sentence
// is stated by the WORKER — which builds the 409 when a write finds a newer
// ownership generation — and by the PAGE, which builds the same 409 when a
// reply comes back for a game this Worker no longer holds. The two sides of a
// `postMessage` cannot share `client.ts` or `worker.ts` (each would drag the
// other's program in), so a string both must spell is a module both import.
//
// Nothing here touches a global at all, so it compiles under the app project
// and the Worker project alike.

/**
 * The lock went while the call was in flight: the module answered, the answer
 * was thrown away, and the page must not draw it.
 *
 * **One wording for both paths to it** (whole-branch review, Important #1).
 * The Worker notices on the generation — a write whose transaction reads a
 * newer owner — and the page notices on `not-held`, which is what a reply
 * carries when the steal's abort has already taken the hold away. Same fact,
 * same consequence, and a player who met one of them should not be able to
 * tell which.
 *
 * Deliberately NOT `LockScreen`'s `LOST_SENTENCE`, which is a different
 * moment: that one refuses a move the page knew was too late BEFORE sending
 * it. This one is about a move that was already made.
 */
export const TAKEN_OVER_SENTENCE =
  "this game was taken over in another tab — what you just did here was not kept";
