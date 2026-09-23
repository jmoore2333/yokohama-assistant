import type { Clock } from "./config";

/** How many `tick:` readings this isolate has answered. */
let ticked = 0;

/**
 * The reading the shell pins the module's clock to before a call that can
 * reach the layer's clock (the plan's ruling P-2) — in SECONDS, the layer's
 * unit. It feeds ONLY the module: every time the shell keeps for itself, a
 * `written_at` or an alarm's deadline, is the platform's `Date.now()` (the
 * orchestrator's rulings I-43 and I-44).
 *
 * - `wall`: `Date.now()`, in whole seconds;
 * - `frozen:<n>`: `n`, every time — the corpus replay's arrangement;
 * - `tick:<n>`: `n`, then `n + 1`, and so on, one per reading, counted per
 *   isolate — so a re-dispatch that pinned no recorded reading would stamp a
 *   different time than the first.
 *
 * One exported function in a module of its own, so a pool cell can replace it
 * at module scope (the plan's pool fact 4) and every caller in the shell reads
 * the replacement.
 */
export function nextReading(clock: Clock): number {
  switch (clock.kind) {
    case "wall":
      return Math.floor(Date.now() / 1000);
    case "frozen":
      return clock.at;
    case "tick":
      return clock.next + ticked++;
  }
}
