export declare const CSP_MODE: "report-only" | "enforce";
export declare const CSP_HEADER: Readonly<Record<"report-only" | "enforce", string>>;
export declare const CLOUDFLARE: Readonly<{ script: readonly string[]; frame: readonly string[]; connect: readonly string[] }>;
export interface HeaderValues {
  siteUrl: string | null;
  releaseFeed: string | null;
  styleHashes: readonly string[];
}
export declare function styleHashesOf(html: string): string[];
export declare function cspOf(values: HeaderValues): string;
export declare function headerSet(values: HeaderValues & { mode?: "report-only" | "enforce" }): [string, string][];
export declare function headersBlock(set: readonly (readonly [string, string])[]): string;
export declare function headersJson(set: readonly (readonly [string, string])[]): string;
export declare function parseHeadersJson(text: string): [string, string][];
