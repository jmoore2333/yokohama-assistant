// What TypeScript reads for `mint-session.mjs` (`web/e2e/live.ts` imports it): an `.mjs` import reads no `.d.ts`
// (TS7016), so the declarations sit beside it as `.d.mts`.

export declare const COOKIE_NAME: "__Host-yokohama_session";
export declare function isTestAddress(email: string): boolean;
export declare function fenceSentence(address: string): string;
export declare function cookieLine(address: string, secret: string, nowSeconds: number, days?: number): string;
