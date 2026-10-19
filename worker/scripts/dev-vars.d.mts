// What TypeScript reads for `dev-vars.mjs` (`web/e2e/live.ts` imports it): an
// `.mjs` import reads no `.d.ts` (TS7016), so the declarations sit beside it
// as `.d.mts`. The module's own comments say what each is.

export declare const ORIGIN: string;
export declare const DEV_VARS: string;
export declare const KEYS: readonly ["CF-Access-Client-Id", "CF-Access-Client-Secret"];
export declare function readCredential(file?: string): { id: string; secret: string };
export declare const SESSION_COOKIE_ENV: "YOKOHAMA_SESSION_COOKIE";
export declare function readSessionCookie(env?: Record<string, string | undefined>): string | null;
export declare const SESSION_SECRET_ENV: "YOKOHAMA_SESSION_SECRET";
export declare function readSessionSecret(env?: Record<string, string | undefined>): string | null;
export declare function readIdentity(
  env?: Record<string, string | undefined>,
  file?: string,
): { secret: string | null; cookie: string | null; credential: { id: string; secret: string } | null };
export declare const TWO_IDENTITIES: string;
export declare function sessionCookieParts(line: string): { name: string; value: string; address: string; expires: number } | null;
