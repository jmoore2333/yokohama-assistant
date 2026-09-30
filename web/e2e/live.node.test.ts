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
// nested-call rule closes for a direct call. And a request context MADE from
// one — `<x>.request.newContext(…)` — is refused wherever it is, inside
// liveRequest's function too: it holds the live cookie, and every call made on
// it later is named after it (R3's R-N7). `r.request()` — a browser request's
// own method — reads no context, and `browser.newContext()` makes a browser's.
// What it reads:
//
//   - every helper module in this directory — every `.ts` that is not a spec
//     or a test, since a live spec may import any of them;
//   - every spec a live run can play: `LIVE_SPECS`, `EXIT_SPEC` and `README_SHOTS_SPEC`, the lists
//     `playwright.config.ts` matches a live run by.
//
// A local-only spec is not read: it never holds the cookie (`it_explains` asks
// the local server through the `request` fixture, and that is fine there).
//
// Node and not Playwright, beside the live tier's other node cells, so it runs
// in `npm run test:wasm` where CI installs no Chromium.
//
// **And a second rule over the same live specs** (R3's L-9), at the end of the
// file: every create is armed through `awaitCreate` right before its click,
// and every `afterEach` abandons through `abandonOpened`.

import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";
import { EXIT_SPEC, LIVE_SPECS, README_SHOTS_SPEC } from "./live";

/** The `APIRequestContext` methods that SEND. `newContext` is not one of
 *  them: it MAKES a context, which is refused wherever it is ([`madeFrom`]). */
const SENDS = new Set(["get", "post", "put", "patch", "delete", "head", "fetch"]);

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

/** Where a request context is MADE from a context read — `<read>.newContext`,
 *  called or not — or `undefined` (R3's R-N7). The made context holds the
 *  live cookie, and every call made on it is named after it rather than after
 *  `request`, so no place it is made is one this scan can follow — inside
 *  `liveRequest`'s function as well. A browser's `browser.newContext()` reads
 *  no API context, and is not this. */
function madeFrom(read: ts.Node): ts.Node | undefined {
  const made = read.parent;
  if (!ts.isPropertyAccessExpression(made) || made.expression !== read || made.name.text !== "newContext") return undefined;
  return isCallee(made) ? made.parent : made;
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
      const made = madeFrom(n);
      const call = sentBy(n);
      if (made !== undefined) {
        refused.push(
          `${where(made)} — a request context made from an API context, inside liveRequest(…) or out: it holds the ` +
            "live cookie, and the calls made on it carry no name this scan can follow",
        );
      } else if (call === undefined) {
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

  // **R3's R-N7: a request context MADE from an API context is refused
  // wherever it is — inside liveRequest's function as well.** It holds the
  // live context's cookie, and every call made on it later is named after
  // it, not after `request`: `liveRequest(() => x.request.newContext())`
  // counted as one wrapped call, and the `ctx.get(url)` after it — whose
  // failure would print the cookie — was a call the scan never saw. A
  // BROWSER's `browser.newContext()` is another receiver, and stays allowed
  // (the "reads what is CODE" cell below).
  it("refuses a request context made from an API context, inside liveRequest or out — so nothing escapes to a name the scan cannot follow", () => {
    const { refused, wrapped } = scan(
      "made.ts",
      wrap(
        [
          "const ctx = await liveRequest(() => x.request.newContext());",
          "await ctx.get(url);",
          "const fixture = await request.newContext({ baseURL });",
          "const later = x.request.newContext;",
        ].join("\n"),
      ),
    );
    expect(refused.map((r) => r.split(" — ")[0])).toEqual([
      "made.ts:2: x.request.newContext()",
      "made.ts:4: request.newContext({ baseURL })",
      "made.ts:5: x.request.newContext",
    ]);
    expect(refused[0]).toContain("a request context made from an API context");
    expect(wrapped, "a context made inside liveRequest is not a wrapped call").toBe(0);
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
    const live = [...LIVE_SPECS, EXIT_SPEC, README_SHOTS_SPEC];

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

// ---- a live table is known from the moment its create is SENT (R3's L-9) ----
//
// A walk that armed its own wait for the create's reply, and set `opened` only
// once it had READ that reply, left a window — the click to the read — in
// which a failure left its table in the shared lobby. `awaitCreate` (live.ts)
// arms the wait before the click and hands back the `Opening` the `afterEach`
// abandons through `abandonOpened`. This holds every live spec to that:
//
//   - every click that CREATES a table — `NewGame.tsx`'s one button, which
//     reads "Open table" or "Deal", as `getByRole("button", { name: … })` — is
//     the statement right after `opened = awaitCreate(<the same page>)`;
//   - `opened` is set from `awaitCreate(…)` or to `null`, and nothing else;
//   - the spec's `test.afterEach` calls `abandonOpened`.
//
//   - no `opened` is declared anywhere but the module's scope: a block's own
//     `opened` would shadow the one the `afterEach` reads, and a create armed
//     into it would be abandoned by nobody.
//
// Its limit, stated: the scan reads a create ONLY as a direct
// `<page>.getByRole("button", { name: <a create label> }).click()` in a live
// spec. Unread: a create button found by any other locator (a regular
// expression, a test id), a chained or stored locator (`.first().click()`,
// `const b = a.getByRole(…); await b.click()`), another action (`press("Enter")`
// in the form, `dblclick`), and a create made from a helper MODULE, whose
// sources this rule does not read for creates. Two premises catch only the
// case where a spec's SOLE create leaves the scan's view: the two labels are
// still `NewGame.tsx`'s, and each live spec makes at least one create click
// the scan reads. A second create in a spec, reached any of those ways, would
// go unread.

/** The labels of the one button that sends `POST /api/games`. */
const CREATE_BUTTONS = new Set(["Open table", "Deal"]);

/** The page a click creates a table on: `<page>.getByRole("button", { name:
 *  <a create label> }).click()`'s `<page>`, as text — or `undefined`. */
function createdOn(n: ts.Node, sf: ts.SourceFile): string | undefined {
  if (!ts.isCallExpression(n) || !ts.isPropertyAccessExpression(n.expression) || n.expression.name.text !== "click") return undefined;
  const locate = n.expression.expression;
  if (!ts.isCallExpression(locate) || !ts.isPropertyAccessExpression(locate.expression) || locate.expression.name.text !== "getByRole") {
    return undefined;
  }
  const [role, options] = locate.arguments;
  if (role === undefined || !ts.isStringLiteralLike(role) || role.text !== "button") return undefined;
  if (options === undefined || !ts.isObjectLiteralExpression(options)) return undefined;
  const named = options.properties.some(
    (p) =>
      ts.isPropertyAssignment(p) &&
      ts.isIdentifier(p.name) &&
      p.name.text === "name" &&
      ts.isStringLiteralLike(p.initializer) &&
      CREATE_BUTTONS.has(p.initializer.text),
  );
  return named ? locate.expression.expression.getText(sf) : undefined;
}

/** The statement `n` stands in: its nearest ancestor that a block lists. */
function statementOf(n: ts.Node): ts.Statement | undefined {
  let at: ts.Node = n;
  while (at.parent !== undefined && !ts.isBlock(at.parent) && !ts.isSourceFile(at.parent)) at = at.parent;
  return at.parent === undefined ? undefined : (at as ts.Statement);
}

/** `opened = awaitCreate(<page>)`'s `<page>` when `s` is that statement. */
function openedOn(s: ts.Statement | undefined, sf: ts.SourceFile): string | undefined {
  if (s === undefined || !ts.isExpressionStatement(s)) return undefined;
  const e = s.expression;
  if (!ts.isBinaryExpression(e) || e.operatorToken.kind !== ts.SyntaxKind.EqualsToken) return undefined;
  if (!ts.isIdentifier(e.left) || e.left.text !== "opened") return undefined;
  const call = e.right;
  if (!ts.isCallExpression(call) || !ts.isIdentifier(call.expression) || call.expression.text !== "awaitCreate") return undefined;
  return call.arguments.length === 1 ? call.arguments[0]!.getText(sf) : undefined;
}

/** What one live spec does with the tables it opens: how many create clicks
 *  it makes, whether its `test.afterEach` calls `abandonOpened`, and every
 *  breach of the rule above as `file:line: the code`. */
function creates(file: string, source: string): { clicks: number; abandons: boolean; refused: string[] } {
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TS);
  const refused: string[] = [];
  let clicks = 0;
  let abandons = false;
  const where = (n: ts.Node) =>
    `${file}:${sf.getLineAndCharacterOfPosition(n.getStart(sf)).line + 1}: ${n.getText(sf).split("\n")[0]}`;
  const calls = (n: ts.Node, name: string): boolean =>
    (ts.isCallExpression(n) && ts.isIdentifier(n.expression) && n.expression.text === name) || ts.forEachChild(n, (c) => calls(c, name) || undefined) === true;
  const visit = (n: ts.Node): void => {
    const page = createdOn(n, sf);
    if (page !== undefined) {
      clicks += 1;
      const statement = statementOf(n);
      const siblings: readonly ts.Statement[] = statement === undefined ? [] : (statement.parent as ts.Block | ts.SourceFile).statements;
      const before = statement === undefined ? undefined : siblings[siblings.indexOf(statement) - 1];
      if (openedOn(before, sf) !== page) {
        refused.push(`${where(n)} — a create not right after opened = awaitCreate(${page}), so its table is not known from the moment it is sent`);
      }
    }
    if (
      (ts.isVariableDeclaration(n) || ts.isParameter(n) || ts.isBindingElement(n) || ts.isFunctionDeclaration(n)) &&
      n.name !== undefined &&
      ts.isIdentifier(n.name) &&
      n.name.text === "opened"
    ) {
      const moduleScope =
        ts.isVariableDeclaration(n) && ts.isVariableDeclarationList(n.parent) && ts.isVariableStatement(n.parent.parent) && ts.isSourceFile(n.parent.parent.parent);
      if (!moduleScope) {
        refused.push(`${where(n)} — an opened declared inside a scope shadows the module's, which is the one the afterEach abandons`);
      }
    }
    if (ts.isBinaryExpression(n) && n.operatorToken.kind === ts.SyntaxKind.EqualsToken && ts.isIdentifier(n.left) && n.left.text === "opened") {
      const right = n.right;
      const fromCreate = ts.isCallExpression(right) && ts.isIdentifier(right.expression) && right.expression.text === "awaitCreate";
      if (!fromCreate && right.kind !== ts.SyntaxKind.NullKeyword) {
        refused.push(`${where(n)} — opened is set from awaitCreate(…) or to null, and nothing else`);
      }
    }
    if (
      ts.isCallExpression(n) &&
      ts.isPropertyAccessExpression(n.expression) &&
      ts.isIdentifier(n.expression.expression) &&
      n.expression.expression.text === "test" &&
      n.expression.name.text === "afterEach" &&
      n.arguments.some((a) => calls(a, "abandonOpened"))
    ) {
      abandons = true;
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return { clicks, abandons, refused };
}

describe("creates — the rule, on sources written for it", () => {
  // Task 7's review, M-1: the afterEach arm is pinned both ways — this cell
  // reads `false` off a hook that abandons BY HAND, the next-but-one `true`.
  // Mutant: `abandons` starting `true`.
  it("refuses the old shape: the walk's own wait before the click, opened set from the reply it read, and a hook abandoning by hand", () => {
    const { clicks, abandons, refused } = creates(
      "old.ts",
      [
        'const created = a.waitForResponse((r) => r.request().method() === "POST");',
        'await a.getByRole("button", { name: "Deal" }).click();',
        "const id = String(((await (await created).json()) as { id: string }).id);",
        "opened = { page: a, id };",
        "test.afterEach(async () => {",
        "  const table = opened;",
        "  opened = null;",
        "  if (LIVE && table) await abandonAsCreator(table.page, table.id);",
        "});",
      ].join("\n"),
    );
    expect(clicks).toBe(1);
    expect(abandons, "a hook that abandons by hand is not abandonOpened").toBe(false);
    expect(refused).toEqual([
      'old.ts:2: a.getByRole("button", { name: "Deal" }).click() — a create not right after opened = awaitCreate(a), so its table is not known from the moment it is sent',
      "old.ts:4: opened = { page: a, id } — opened is set from awaitCreate(…) or to null, and nothing else",
    ]);
  });
  // Task 7's review, M-2: `opened` is matched by NAME, so a block's own
  // `opened` would take the create while the module's stayed null — refused
  // wherever it is declared, but at the module's scope.
  // Mutant: a `let opened` in the test body, before the create.
  it("refuses an opened declared anywhere but the module's scope — a block's, a parameter, a destructure, a loop's", () => {
    const read = creates(
      "shadow.ts",
      [
        "let opened: Opening | null = null;",
        'test("t", async () => {',
        "  let opened: Opening | null = null;",
        "  const f = (opened: number) => opened;",
        "  const { opened: renamed } = row;",
        "  const { opened } = row;",
        "  for (const opened of rows) f(opened);",
        "  opened = awaitCreate(a);",
        '  await a.getByRole("button", { name: "Deal" }).click();',
        "});",
      ].join("\n"),
    );
    // Line 5 declares `renamed`, not `opened`: a property read under another
    // name is no shadow.
    expect(read.refused).toEqual([
      "shadow.ts:3: opened: Opening | null = null — an opened declared inside a scope shadows the module's, which is the one the afterEach abandons",
      "shadow.ts:4: opened: number — an opened declared inside a scope shadows the module's, which is the one the afterEach abandons",
      "shadow.ts:6: opened — an opened declared inside a scope shadows the module's, which is the one the afterEach abandons",
      "shadow.ts:7: opened — an opened declared inside a scope shadows the module's, which is the one the afterEach abandons",
    ]);
  });
  it("refuses awaitCreate armed AFTER the click, on another page, or with a statement between", () => {
    const late = creates("late.ts", ['await a.getByRole("button", { name: "Open table" }).click();', "opened = awaitCreate(a);"].join("\n"));
    const other = creates("other.ts", ["opened = awaitCreate(b);", 'await a.getByRole("button", { name: "Open table" }).click();'].join("\n"));
    const between = creates(
      "between.ts",
      ["opened = awaitCreate(a);", "await a.waitForTimeout(1);", 'await a.getByRole("button", { name: "Open table" }).click();'].join("\n"),
    );
    for (const read of [late, other, between]) {
      expect(read.clicks).toBe(1);
      expect(read.refused).toHaveLength(1);
    }
  });
  it("accepts the new shape inside a test's body, and reads the afterEach's abandon", () => {
    const read = creates(
      "new.ts",
      [
        "test.afterEach(async () => {",
        "  const table = opened;",
        "  opened = null;",
        "  await abandonOpened(table);",
        "});",
        'test("t", async () => {',
        "  opened = awaitCreate(x);",
        '  await x.getByRole("button", { name: "Open table" }).click();',
        "  const id = await opened.id;",
        '  await x.getByRole("button", { name: "Open tables" }).click();',
        "});",
      ].join("\n"),
    );
    expect(read).toEqual({ clicks: 1, abandons: true, refused: [] });
  });
});

describe("every live spec knows its table from the moment its create is sent (R3's L-9)", () => {
  // Mutant: a live spec that arms its own wait for the create's reply and
  // sets `opened` after it — the shape before this rule — or an afterEach
  // that abandons by hand.
  it("each creates through awaitCreate armed right before the click, sets opened from nothing else, and abandons through abandonOpened", () => {
    // The premise: the two labels are still the create button's.
    expect(readFileSync(join(here, "..", "src", "NewGame.tsx"), "utf8"), "NewGame.tsx's create button").toContain('{opensSeats ? "Open table" : "Deal"}');
    const refused: string[] = [];
    for (const spec of [...LIVE_SPECS, EXIT_SPEC, README_SHOTS_SPEC]) {
      const read = creates(spec, readFileSync(join(here, spec), "utf8"));
      expect(read.clicks, `premise: ${spec} creates a table`).toBeGreaterThan(0);
      refused.push(...read.refused);
      if (!read.abandons) refused.push(`${spec}: its test.afterEach does not abandon through abandonOpened`);
    }
    expect(refused, "a create not armed through awaitCreate, opened set otherwise, or an afterEach abandoning by hand").toEqual([]);
  });
});

describe("the README's screenshot generator is collected only when asked for (Task 8, R-6.5c2.44)", () => {
  // It WRITES into `worker/docs/img/`, so a plain run — local or live — must
  // never collect it. Read through Playwright itself (`--list`), not the
  // config's text: the property is what the runner collects. Mutants: the
  // `testIgnore` entry missing; the live `testMatch` not naming it under the
  // switch; the switch read as anything but "1".
  const collected = (env: Record<string, string>): boolean => {
    const out = execFileSync("npx", ["playwright", "test", "--list"], {
      cwd: join(here, ".."),
      env: { ...process.env, YOKOHAMA_WEB_README_SHOTS: "", YOKOHAMA_WEB_E2E_TARGET: "", YOKOHAMA_WEB_E2E_OPTIONAL: "1", ...env },
      encoding: "utf8",
    });
    return out.includes(README_SHOTS_SPEC);
  };
  it("is not collected by a plain run, local or live, and is collected under the switch on either target", () => {
    expect(collected({}), "plain, local").toBe(false);
    expect(collected({ YOKOHAMA_WEB_E2E_TARGET: "live" }), "plain, live").toBe(false);
    expect(collected({ YOKOHAMA_WEB_README_SHOTS: "0" }), "the switch reads only 1").toBe(false);
    expect(collected({ YOKOHAMA_WEB_README_SHOTS: "1" }), "switch, local").toBe(true);
    expect(collected({ YOKOHAMA_WEB_README_SHOTS: "1", YOKOHAMA_WEB_E2E_TARGET: "live" }), "switch, live").toBe(true);
  }, 120_000);
});
