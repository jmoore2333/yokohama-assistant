// What TypeScript reads for `dev-vars.mjs` (`web/e2e/live.ts` imports it): an
// `.mjs` import reads no `.d.ts` (TS7016), so the declarations sit beside it
// as `.d.mts`. The module's own comments say what each is.

export declare const ORIGIN: string;
export declare const DEV_VARS: string;
export declare const KEYS: readonly ["CF-Access-Client-Id", "CF-Access-Client-Secret"];
export declare function readCredential(file?: string): { id: string; secret: string };
