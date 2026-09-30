/** `web/scripts/tally-report.mjs`'s declaration, for the node project that
 *  type-checks `e2e/harness.ts` and the report's own cells (TypeScript
 *  resolves `./tally-report.mjs` to this file under `moduleResolution:
 *  "bundler"`). */

/** The rate limit's window, in milliseconds. */
export declare const WINDOW_MS: number;
/** The ONE regex a route is named by: `<id>` after `/api/games/`, `<n>` after
 *  a trailing `/at/` or `/chat/`. Global. */
export declare const ROUTE_IDS: RegExp;
/** A path's route: the query dropped, the ids named by `ROUTE_IDS`. */
export declare function normalizeRoute(path: string): string;
/** `…/join`, `…/unlock` or `/api/archive`, as `normalizeRoute` names them. */
export declare function isOracle(route: string): boolean;
/** The most of `times` inside any half-open window `[t, t + windowMs)`. */
export declare function peakInWindow(times: number[], windowMs: number): number;

export interface TallyLine {
  t: number;
  page: string;
  method: string;
  route: string;
  status: number;
}
export interface Figure {
  count: number;
  perMove: number;
  peak: number;
}
export interface RouteFigure extends Figure {
  method: string;
  route: string;
  refused: number;
}
export interface Report {
  moves: number;
  requests: number;
  spanMs: number;
  pages: Record<string, number>;
  routes: RouteFigure[];
  api: Figure;
  oracle: Figure;
  notApi: Figure;
  all: Figure;
}
/** The tally's lines; a line that is not one throws naming its number. */
export declare function parseTally(text: string): TallyLine[];
export declare function tallyReport(lines: TallyLine[], opts: { moves: number }): Report;
export declare function formatReport(r: Report): string;
/** `<path> --moves <n>`; throws on a missing path or moves that are not a
 *  positive whole number. */
export declare function parseArgs(argv: string[]): { path: string; moves: number };
