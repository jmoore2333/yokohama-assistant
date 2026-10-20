// What the service worker tells the page when an install fails, and which window it tells to pass the edge's
// challenge (security S3, W-3; design §3.1 steps 1–2; D-1, D-13).
//
// **The writer's and the reader's one module.** `sw.ts` posts the message and the page's `swRegister.ts` reads
// it; the type is stated here so that both can import it, and the shape is stated once. The worker's bundle is
// built apart from the page's (`scripts/precache.mjs`, esbuild, `tsconfig.sw.json`'s WebWorker lib), so this
// module imports only what is as free of the page as it is: `swRoute.ts`'s server paths and `tableId.ts`'s id
// grammar. It touches no global.

import { isServerPath } from "./swRoute";
import { isSafeId } from "./tableId";

/** Why the install failed, in the one word the door acts on: `"challenged"` — the edge asked for a check before
 *  it would answer a precache fetch, and THIS window is the one elected to pass it — or `"failed"`, everything
 *  else, and every window not elected. */
export type InstallFailedKind = "challenged" | "failed";

/** The message a failed install posts to every window (D-1: a `kind` beside the `reason`, not a new message).
 *  `reason` is a sentence naming the path, the same for every window of one install. */
export type InstallFailed = { yokohama: "install-failed"; kind: InstallFailedKind; reason: string };

/** A window as `clients.matchAll({ type: "window" })` lists it, the three fields the election reads. Structural,
 *  so the page's DOM lib and the worker's WebWorker lib both compile it. */
export interface ListedWindow {
  readonly url: string;
  readonly focused: boolean;
  readonly visibilityState: string;
}

/** Whether the address `url` is the DOOR's: `route.ts`'s `parseRoute(hash).kind === "door"`, restated because
 *  the worker's bundle cannot import `route.ts`, which imports React. `swMessages.test.ts` holds the two equal
 *  over a table of hashes, `parseRoute` imported there and nowhere in the worker.
 *
 *  **An address, not a screen.** The worker hands this the address each window was LOADED at, which in Chromium
 *  does not follow a hash change (measured in Playwright's Chromium, the Task 5 review, 2026-10-20), so what it
 *  reads is where a window came in, and what keeps a game screen clear is the page's check of its own route
 *  (`swRegister.ts`, P-26).
 *
 *  The rule, as `parseRoute` has it: `#/lobby`, `#/new`, `#/new/local` and `#/t/<id>` with an id the store's
 *  grammar admits are the other screens, and every other hash is the door — an empty one, `#`, `#/` and anything
 *  unrecognized among them. `WindowClient.url` may carry the fragment or not, so an address with no hash is the
 *  door.
 *
 *  **And two things a hash cannot say.** The app runs wherever the shell draws, which is every path a navigation
 *  can name except the server's (`swRoute.ts`'s rule 2 and `isServerPath`); `clients.matchAll` lists every
 *  window of the origin, so a window at the sign-in (`/enter`), under `/api` or `/cdn-cgi`, or a blank window the
 *  origin opened (`about:blank`) is not the door, nor is an address that does not parse. Elected, such a window
 *  would swallow the challenge where nothing reads the message. */
export function isDoorUrl(url: string): boolean {
  let at: URL;
  try {
    at = new URL(url);
  } catch {
    return false;
  }
  if (at.protocol !== "https:" && at.protocol !== "http:") return false;
  if (isServerPath(at.pathname)) return false;
  const path = at.hash.startsWith("#") ? at.hash.slice(1) : at.hash;
  if (path === "/lobby" || path === "/new" || path === "/new/local") return false;
  const table = /^\/t\/([^/]+)$/.exec(path);
  if (!table) return true;
  // `parseRoute`'s two door arms for a table address: an escape `decodeURIComponent` refuses, and an id the
  // store's grammar refuses.
  try {
    return !isSafeId(decodeURIComponent(table[1]!));
  } catch {
    return true;
  }
}

/** The ONE window to carry `"challenged"` (D-13): among the windows whose address is the door's, the focused
 *  one, else a visible one, else the first; `null` when no window's is, and then every window is told
 *  `"failed"`. Two windows alike elect the first listed, so an election always names at most one.
 *
 *  **It picks one window and does not keep a game screen clear.** The address is the one each window was LOADED
 *  at, which in Chromium does not follow a hash change (measured in Playwright's Chromium, the Task 5 review,
 *  2026-10-20), so a tab that came in at the door and moved into a game can be elected. The page's check of its
 *  own route (`swRegister.ts`, P-26) is the fence that keeps such a tab from drawing an overlay mid-play.
 *
 *  **The reverse corner, accepted:** a tab that arrived on a game link and then moved to the door is listed at
 *  the game's address and is never elected until it reloads, so its "Try again" cannot pass the challenge on that
 *  page — no worse than the `"failed"` it drew before S3. */
export function electDoorWindow<W extends ListedWindow>(windows: readonly W[]): W | null {
  const doors = windows.filter((w) => isDoorUrl(w.url));
  return (
    doors.find((w) => w.focused) ?? doors.find((w) => w.visibilityState === "visible") ?? doors[0] ?? null
  );
}
