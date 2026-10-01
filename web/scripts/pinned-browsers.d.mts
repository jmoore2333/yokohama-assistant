/** `web/scripts/pinned-browsers.mjs`'s declaration, for the node project that
 *  type-checks its cells (`pinned-browsers.node.test.ts`; an `.mjs` import
 *  reads no `.d.ts`, so it sits beside it as `.d.mts`). The module's own
 *  comments say what each is. */

export declare const USAGE: string;

export declare const SENTENCES: Readonly<{
  installing: string;
  ready: string;
  failed: RegExp;
  update: string;
}>;

export declare function pinnedArgs(argv: string[]): { label: string; origin: string; stateDir: string; secondTab: boolean };

export declare function doorReadiness(lines: string[]): "none" | "installing" | "ready" | "failed" | "unknown";

/** One check: `ok` true held, false FAILED, null NOT RUN (with why in `saw`). */
export interface Check {
  check: string;
  ok: boolean | null;
  saw: unknown;
}

/** The click's four checks, by name. */
export declare const CLICK_CHECKS: readonly string[];

/** The run's one sentence: `ok` none failed, `complete` every one ran. */
export declare function verdictOf(checks: Check[]): { ok: boolean; complete: boolean; summary: string };

/** A game tab, as a snapshot reads it. */
export interface TabReading {
  lines: string[];
  loads: number;
  url: string;
  events: number;
}

export interface AfterReload {
  /** Whether the run wanted the second tab (no `--no-second-tab`). */
  secondTab: boolean;
  pinnedRelease: string | null;
  /** The door's caches in `before-go.json`: the pinned generation alone, or
   *  the release moved before the reload. */
  cachesBeforeGo: string[];
  release: string | null;
  caches: string[];
  waiting: string | null;
  active: string | null;
  doorLines: string[];
  doorLoads: number;
  serviceWorkerPaths: string[];
  tab: TabReading | null;
  tabBefore: { url: string; events: number } | null;
}

export declare function afterReloadChecks(reading: AfterReload): Check[];

export interface AfterClick {
  before: { url: string; events: number };
  tab: TabReading;
  release: string | null;
  eventsAfterAMove: number | null;
}

export declare function afterClickChecks(reading: AfterClick): Check[];
