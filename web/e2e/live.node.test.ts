// **Every live API call goes through `liveRequest`, held by the suite instead
// of by memory** (M6.5c Task 15's re-review, R-1; the whole-branch review's
// L-2).
//
// MEASURED (Playwright 1.62.1; the review's H-1): a THROWN API request's
// message carries a "Call log" listing every header it sent — a live context's
// `CF_Authorization` cookie among them — and Playwright writes a failing test's
// error to the reporter and to `test-results/<test>/error-context.md` whatever
// the trace setting reads. `liveRequest` (`live.ts`) withholds that log, and
// `toBeOK()`'s failure message prints it too. Both rules were written down and
// enforced by nothing: a new unwrapped `page.request.get(…)` in a live spec
// passed every gate, and would have put the session into a retained file on
// its first failure.
//
// So this READS the sources, the shape `every_spec_can_skip.spec.ts` and the
// Rust crate's `platform_calls_stay_behind_the_server_feature` already use —
// and it parses them with the TypeScript compiler rather than matching text,
// so a call split over lines, or a comment that merely NAMES one, is read for
// what it is.
//
// **The one form it accepts** is a call on an API context, written where the
// context is read: `<x>.request.<verb>(…)`, or the fixture's
// `request.<verb>(…)`, directly in the function handed to `liveRequest`. Every
// OTHER read of a context is refused wherever it is — an alias
// (`const api = x.request`), an argument, a destructure under another name,
// a bracketed key — because the calls later made on it carry no name the scan
// knows (R3's R-N2, ruling I-91). Inside liveRequest's function as well: an
// alias made there can be called from a callback nested in it, the escape the
// nested-call rule closes for a direct call. `r.request()` — a browser
// request's own method — reads no context. What it reads:
//
//   - every helper module in this directory — every `.ts` that is not a spec
//     or a test, since a live spec may import any of them;
//   - every spec a live run can play: `LIVE_SPECS` and `EXIT_SPEC`, the lists
//     `playwright.config.ts` matches a live run by.
//
// A local-only spec is not read: it never holds the cookie (`it_explains` asks
// the local server through the `request` fixture, and that is fine there).
//
// Node and not Playwright, beside the live tier's other node cells, so it runs
// in `npm run test:wasm` where CI installs no Chromium.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";
import { EXIT_SPEC, LIVE_SPECS } from "./live";

/** The `APIRequestContext` methods that SEND — and `newContext`, which makes a
 *  context whose own calls would be named after it rather than `request`, so a
 *  live file that needs one has to say why in this list. */
const SENDS = new Set(["get", "post", "put", "patch", "delete", "head", "fetch", "newContext"]);

/** Whether `key` names `request`: as a name, a string, or a bracketed
 *  string. */
function isRequestKey(key: ts.Node): boolean {
  if (ts.isComputedPropertyName(key)) return isRequestKey(key.expression);
  return (ts.isIdentifier(key) || ts.isStringLiteralLike(key)) && key.text === "request";
}

/** Whether `n` is the callee of a call: `r.request()`. */
function isCallee(n: ts.Node): boolean {
  return ts.isCallExpression(n.parent) && n.parent.expression === n;
}

/** Whether an identifier `request` is READ where it stands — rather than
 *  declared, a property's key, an import or export, a type, or the name half
 *  of `x.request` (which its property access reads). A destructure's default
 *  value is a read. */
function isRead(id: ts.Identifier): boolean {
  const p = id.parent;
  if (ts.isPropertyAccessExpression(p)) return p.expression === id;
  if (ts.isShorthandPropertyAssignment(p)) return true;
  if (ts.isBindingElement(p)) return p.initializer === id;
  if ((p as { name?: ts.Node }).name === id) return false;
  return !(
    ts.isImportSpecifier(p) ||
    ts.isImportClause(p) ||
    ts.isNamespaceImport(p) ||
    ts.isExportSpecifier(p) ||
    ts.isTypeReferenceNode(p) ||
    ts.isQualifiedName(p) ||
    ts.isTypeQueryNode(p)
  );
}

/**
 * Whether `n` reads an API context: `<x>.request` or `<x>["request"]` that is
 * not itself called, the fixture's `request` where it is read, a destructure
 * or an import that takes `request` under another name.
 */
function readsContext(n: ts.Node): boolean {
  if (ts.isPropertyAccessExpression(n)) return n.name.text === "request" && !isCallee(n);
  if (ts.isElementAccessExpression(n)) return isRequestKey(n.argumentExpression) && !isCallee(n);
  if (ts.isIdentifier(n)) return n.text === "request" && isRead(n);
  if (ts.isBindingElement(n) || ts.isImportSpecifier(n)) {
    return n.propertyName !== undefined && isRequestKey(n.propertyName) && !(ts.isIdentifier(n.name) && n.name.text === "request");
  }
  return false;
}

/** The call a context read is SENT by — `<read>.<verb>(…)` with a verb in
 *  `SENDS` — or `undefined` when the read goes anywhere else. */
function sentBy(read: ts.Node): ts.CallExpression | undefined {
  const verb = read.parent;
  if (!ts.isPropertyAccessExpression(verb) || verb.expression !== read || !SENDS.has(verb.name.text)) return undefined;
  return isCallee(verb) ? (verb.parent as ts.CallExpression) : undefined;
}

/** Whether `node`'s NEAREST enclosing function is the function handed to
 *  `liveRequest(…)` as its argument. The nearest, and not any: a call inside
 *  a callback nested in that function rejects somewhere `liveRequest` is not
 *  awaiting, and its log would print as it always did. */
function insideLiveRequest(node: ts.Node): boolean {
  for (let n = node.parent; n !== undefined; n = n.parent) {
    if (ts.isFunctionLike(n)) {
      const call = n.parent;
      return (
        call !== undefined &&
        ts.isCallExpression(call) &&
        call.arguments[0] === n &&
        ts.isIdentifier(call.expression) &&
        call.expression.text === "liveRequest"
      );
    }
  }
  return false;
}

/** What one file does with live requests: every API call that is not made
 *  inside `liveRequest`, every other read of an API context, and every
 *  `toBeOK(`, each as `file:line: the code`; and how many calls ARE wrapped —
 *  so a scan that found nothing can say so. */
function scan(file: string, source: string): { refused: string[]; wrapped: number } {
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TS);
  const refused: string[] = [];
  let wrapped = 0;
  const where = (n: ts.Node) =>
    `${file}:${sf.getLineAndCharacterOfPosition(n.getStart(sf)).line + 1}: ${n.getText(sf).split("\n")[0]}`;
  const visit = (n: ts.Node): void => {
    if (readsContext(n)) {
      const call = sentBy(n);
      if (call === undefined) {
        refused.push(
          `${where(n)} — an API context taken where this scan cannot follow it (an alias, an argument, a destructure): ` +
            "call it as <x>.request.<verb>(…) directly inside liveRequest(…)",
        );
      } else if (insideLiveRequest(call)) {
        wrapped += 1;
      } else {
        refused.push(`${where(call)} — a live API call outside liveRequest(…), whose failure would print the context's cookie`);
      }
    }
    if (ts.isCallExpression(n)) {
      const callee = n.expression;
      if (ts.isPropertyAccessExpression(callee) && callee.name.text === "toBeOK") {
        refused.push(`${where(n)} — toBeOK() prints the call log on failure; assert res.ok() instead`);
      }
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return { refused, wrapped };
}

const here = dirname(fileURLToPath(import.meta.url));

describe("scan — the rule, on sources written for it", () => {
  const wrap = (body: string) => `import { liveRequest } from "./live";\n${body}\n`;

  it("accepts a call made inside liveRequest, in either form a caller writes it", () => {
    const { refused, wrapped } = scan(
      "ok.ts",
      wrap(
        [
          "const a = await liveRequest(() => page.request.get(`${BASE_URL}/api/health`));",
          "const b = await liveRequest(async () => {",
          "  const res = await context.request.post(url, { data: {} });",
          "  return res;",
          "});",
        ].join("\n"),
      ),
    );
    expect(refused).toEqual([]);
    expect(wrapped).toBe(2);
  });

  it("refuses a call outside it — on a page's context, on the fixture, and across lines — naming the file and line", () => {
    const { refused, wrapped } = scan(
      "bad.ts",
      wrap(
        [
          "const a = await page.request.get(`${BASE_URL}/api/games/x/events`);",
          "const b = await request.post(\"/api/games\", { data: {} });",
          "const c = await x.context()",
          "  .request.delete(url);",
        ].join("\n"),
      ),
    );
    expect(wrapped).toBe(0);
    expect(refused).toHaveLength(3);
    expect(refused[0]).toMatch(/^bad\.ts:2: page\.request\.get\(/);
    expect(refused[1]).toMatch(/^bad\.ts:3: request\.post\(/);
    expect(refused[2]).toMatch(/^bad\.ts:4: x\.context\(\)/);
  });

  it("refuses a call in a callback NESTED inside liveRequest's function, whose rejection it never sees", () => {
    const { refused } = scan(
      "nested.ts",
      wrap("await liveRequest(async () => { void [1].map(() => page.request.get(url)); return 1; });"),
    );
    expect(refused).toHaveLength(1);
  });

  it("refuses a context of its own, and toBeOK anywhere", () => {
    const { refused } = scan(
      "ctx.ts",
      wrap("const ctx = await playwright.request.newContext({});\nawait expect(res).toBeOK();"),
    );
    expect(refused.map((r) => r.split(" — ")[0])).toEqual([
      "ctx.ts:2: playwright.request.newContext({})",
      "ctx.ts:3: expect(res).toBeOK()",
    ]);
  });

  // R3's R-N2 (ruling I-91): an API context taken anywhere the scan cannot
  // follow — an alias, an argument, a destructure under another name, a
  // bracketed key — is refused where it is taken, since the calls made on it
  // later carry no name the scan knows. Inside liveRequest's function too: an
  // alias made there can be called from a callback nested in it, which is the
  // escape the nested-call cell above closes for a direct call. Before this
  // rule the scan refused none of them.
  it("refuses the API context taken where the scan cannot follow it — an alias, an argument, a destructure — inside liveRequest or out", () => {
    const { refused, wrapped } = scan(
      "alias.ts",
      wrap(
        [
          "const api = x.request;",
          "await api.get(`${BASE_URL}/api/health`);",
          "await send(page.request, url);",
          "const { request: renamed } = page;",
          "const fixture = request;",
          'const viaKey = page["request"];',
          "await liveRequest(async () => { const inner = page.request; return inner.get(url); });",
          "const { fallback = request } = options;",
        ].join("\n"),
      ),
    );
    expect(refused.map((r) => r.split(" — ")[0])).toEqual([
      "alias.ts:2: x.request",
      "alias.ts:4: page.request",
      "alias.ts:5: request: renamed",
      "alias.ts:6: request",
      'alias.ts:7: page["request"]',
      "alias.ts:8: page.request",
      "alias.ts:9: request",
    ]);
    expect(wrapped, "none of them is a call the scan can see").toBe(0);
  });

  // The companion: a destructure that KEEPS the name declares `request`, which
  // is not a read, and every call made on it is then held by that name.
  it("holds a destructure that keeps the name by that name: a call on it outside is refused, inside is wrapped", () => {
    const { refused, wrapped } = scan(
      "shorthand.ts",
      wrap(["const { request } = page;", "await request.get(url);", "await liveRequest(() => request.get(url));"].join("\n")),
    );
    expect(refused.map((r) => r.split(" — ")[0])).toEqual(["shorthand.ts:3: request.get(url)"]);
    expect(wrapped).toBe(1);
  });

  it("reads what is CODE: a comment naming a call, a response's request(), a browser's context, and a key or a declaration named request are not reads", () => {
    const { refused, wrapped } = scan(
      "prose.ts",
      wrap(
        [
          "// never `await page.request.get(url)` here, and never `expect(res).toBeOK()`",
          "const method = r.request().method();",
          "const ctx = await browser.newContext();",
          'const options = { request: "GET" };',
          "type Shape = { request: number };",
          "function named(request: number) { return 1; }",
        ].join("\n"),
      ),
    );
    expect(refused).toEqual([]);
    expect(wrapped).toBe(0);
  });
});

describe("every live API call in web/e2e goes through liveRequest", () => {
  it("holds across every helper module and every spec a live run can play", () => {
    const files = readdirSync(here).filter((f) => f.endsWith(".ts"));
    const helpers = files.filter((f) => !f.endsWith(".spec.ts") && !f.endsWith(".test.ts"));
    const live = [...LIVE_SPECS, EXIT_SPEC];

    // **Anti-vacuity, and it is the whole risk in a check like this**: a scan
    // over an empty or wrong listing passes without reading a line. The
    // module that DEFINES the rule has to be in it, every live spec has to be
    // a file here, and the two calls that are wrapped today have to be found.
    expect(helpers, "read the wrong directory: live.ts is not in it").toContain("live.ts");
    expect(helpers).toContain("harness.ts");
    for (const spec of live) expect(files, `the live spec ${spec} is not in ${here}`).toContain(spec);

    let wrapped = 0;
    const refused: string[] = [];
    for (const f of [...helpers, ...live]) {
      const read = scan(f, readFileSync(join(here, f), "utf8"));
      wrapped += read.wrapped;
      refused.push(...read.refused);
    }
    expect(refused, "live API calls outside liveRequest(…), or a toBeOK()").toEqual([]);
    expect(wrapped, "the wrapped calls the scan must find: the abandon's DELETE and the event count's GET at least").toBeGreaterThanOrEqual(2);
  });
});
