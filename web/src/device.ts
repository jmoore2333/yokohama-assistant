// This browser's device id (the auth cycle, Task 6; design §4.4, as the plan
// corrected it): sixteen lowercase hex digits, drawn once and sent as
// `X-Yokohama-Device` on every archive post (`api.archive`), so a game finished
// with no one signed in files under one anonymous prefix per browser —
// `anon-<device>@anon.invalid` — rather than one per game.
//
// **Kept in `localStorage`, and deliberately NOT the outbox's IndexedDB `META`
// store** the design first named: that store is keyed by GAME id and swept by
// `forgetGame` and the backups, so a device row there would be swept or
// exported with a game. `localStorage` is per origin and survives a release,
// which is all the id needs. A browser that clears its storage is a new
// device, and that is fine: the id is a self-declared LABEL, never a proof
// (ruling R-33). The Worker reads it on the archive target alone: it names the
// anonymous prefix where no verified caller is present, and is noted on the
// archived object's metadata, signed in or not (design §4.4).
//
// **Exactly the Worker's shape** (`isDeviceId`, Task 5): sixteen LOWERCASE hex
// digits. Anything else there reads as no header at all, so a stored value of
// any other shape is drawn again rather than sent.

/** Where the id is kept. */
export const DEVICE_KEY = "yokohama.device";

const DEVICE = /^[0-9a-f]{16}$/;

/** The id this page drew because the browser would not keep one — a private
 *  window, a blocked site — so it is at least one id for the page's life. */
let thisPage: string | null = null;

/** Sixteen lowercase hex digits from the platform's random source. */
function draw(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  let hex = "";
  for (const b of bytes) hex += b.toString(16).padStart(2, "0");
  return hex;
}

/**
 * This browser's device id: the kept one, or one drawn now and kept.
 *
 * **Every storage read and write is inside the `try`**: a storage that throws
 * is a browser that cannot remember, never an error to report — the id is then
 * this page's own, drawn once.
 */
export function deviceId(): string {
  try {
    const kept = localStorage.getItem(DEVICE_KEY);
    if (kept !== null && DEVICE.test(kept)) return kept;
    const drawn = draw();
    localStorage.setItem(DEVICE_KEY, drawn);
    return drawn;
  } catch {
    thisPage ??= draw();
    return thisPage;
  }
}

/** Forget this page's own id, as a fresh page load has none — for the CELLS. */
export function resetDeviceForTests(): void {
  thisPage = null;
}
