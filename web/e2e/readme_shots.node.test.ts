// **What a failed README run may write, held by the spec's own source** (the
// whole-branch review, W-2 and W-3; `readme_shots.spec.ts`'s header).
//
// The README generator is about to run against a public host. A FAILED run that
// can meet a stranger must not leave the page behind, as text or as pixels.
// Playwright 1.62.1 writes an unredacted accessibility snapshot into
// `error-context.md` unless `PLAYWRIGHT_NO_COPY_PROMPT` is set; a locator
// assertion that fails unmatched carries a snapshot of the whole page on its
// error, written into the same file whatever that variable says, and its message
// prints what it received; and the config's failure screenshots shoot every open
// page unmasked. The spec carries its own switches so a run cannot forget them,
// and these cells read the spec's source, parsed by the TypeScript compiler
// (`live.node.test.ts`'s manner), so the switches cannot quietly go:
//
//   - `process.env.PLAYWRIGHT_NO_COPY_PROMPT = "1"` and
//     `test.use({ screenshot: "off", video: "off", trace: "off" })` are
//     statements of the module itself, before its first `test(…)`;
//   - the test body catches a failure and, on a run that can meet a stranger
//     (`WITHHOLD`, `LIVE || STRANGER`), throws a fresh `Error` built from a
//     literal and the last picture's name alone — never from the caught error —
//     and otherwise rethrows the caught error unchanged;
//   - `play`'s body is a `try` whose `finally` sends every page to
//     `about:blank`;
//   - `PASSWORD` is drawn at run time: no string literal is ever assigned to it,
//     since the repository is public and the lobby lists the locked table;
//   - the stranger arm throws, at the module's top, when the target is live.
//
// Node and not Playwright, so `npm run test:wasm` runs it where CI installs no
// Chromium.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const SPEC = fileURLToPath(new URL("./readme_shots.spec.ts", import.meta.url));
const sf = ts.createSourceFile("readme_shots.spec.ts", readFileSync(SPEC, "utf8"), ts.ScriptTarget.ES2022, true, ts.ScriptKind.TS);
const top = sf.statements;

/** `s` is `test(…);` — a test declared at the module's top. */
const isTestCall = (s: ts.Statement): boolean =>
  ts.isExpressionStatement(s) && ts.isCallExpression(s.expression) && ts.isIdentifier(s.expression.expression) && s.expression.expression.text === "test";

/** The index of the module's first `test(…)` statement. */
const firstTest = top.findIndex(isTestCall);

/** `s` is `process.env.PLAYWRIGHT_NO_COPY_PROMPT = "<non-empty>";`. */
function setsNoCopyPrompt(s: ts.Statement): boolean {
  if (!ts.isExpressionStatement(s) || !ts.isBinaryExpression(s.expression)) return false;
  const e = s.expression;
  return (
    e.operatorToken.kind === ts.SyntaxKind.EqualsToken &&
    e.left.getText(sf) === "process.env.PLAYWRIGHT_NO_COPY_PROMPT" &&
    ts.isStringLiteralLike(e.right) &&
    e.right.text !== ""
  );
}

/** `s` is `test.use({ screenshot: "off", video: "off", trace: "off" });`,
 *  exactly those three keys, each the string "off". */
function turnsArtifactsOff(s: ts.Statement): boolean {
  if (!ts.isExpressionStatement(s) || !ts.isCallExpression(s.expression)) return false;
  const call = s.expression;
  if (call.expression.getText(sf) !== "test.use" || call.arguments.length !== 1) return false;
  const arg = call.arguments[0]!;
  if (!ts.isObjectLiteralExpression(arg)) return false;
  const pairs = arg.properties.map((p) =>
    ts.isPropertyAssignment(p) && ts.isIdentifier(p.name) && ts.isStringLiteralLike(p.initializer) ? `${p.name.text}=${p.initializer.text}` : "?",
  );
  return [...pairs].sort().join(",") === "screenshot=off,trace=off,video=off";
}

/** Every call `<x>.goto("about:blank")` under `n`. */
function blanks(n: ts.Node): number {
  let count = 0;
  const visit = (m: ts.Node): void => {
    if (
      ts.isCallExpression(m) &&
      ts.isPropertyAccessExpression(m.expression) &&
      m.expression.name.text === "goto" &&
      m.arguments.length >= 1 &&
      ts.isStringLiteralLike(m.arguments[0]!) &&
      (m.arguments[0] as ts.StringLiteralLike).text === "about:blank"
    ) {
      count += 1;
    }
    ts.forEachChild(m, visit);
  };
  visit(n);
  return count;
}

describe("readme_shots.spec.ts writes nothing of a stranger's when it fails", () => {
  it("premise: the module declares its test", () => {
    expect(firstTest, "no test(…) at the module's top").toBeGreaterThan(-1);
  });

  it("sets PLAYWRIGHT_NO_COPY_PROMPT at the module's top, before the first test", () => {
    const at = top.findIndex(setsNoCopyPrompt);
    expect(at, "no top-level process.env.PLAYWRIGHT_NO_COPY_PROMPT = \"…\"").toBeGreaterThan(-1);
    expect(at, "it comes after the first test").toBeLessThan(firstTest);
  });

  it("turns screenshots, video and the trace off for itself, before the first test", () => {
    const at = top.findIndex(turnsArtifactsOff);
    expect(at, 'no top-level test.use({ screenshot: "off", video: "off", trace: "off" })').toBeGreaterThan(-1);
    expect(at, "it comes after the first test").toBeLessThan(firstTest);
  });

  it("withholds a failure's error on a run that can meet a stranger, rethrowing one built from a literal and the stage", () => {
    // WITHHOLD is exactly the two runs whose pages can hold a stranger.
    const withhold = top.some(
      (s) =>
        ts.isVariableStatement(s) &&
        s.declarationList.declarations.some((d) => ts.isIdentifier(d.name) && d.name.text === "WITHHOLD" && d.initializer?.getText(sf) === "LIVE || STRANGER"),
    );
    expect(withhold, "WITHHOLD is not declared at the module's top as LIVE || STRANGER").toBe(true);

    // The test body's try, and its catch clause.
    const body = top.filter(isTestCall).map((s) => ((s as ts.ExpressionStatement).expression as ts.CallExpression).arguments[1])[0];
    expect(body !== undefined && ts.isArrowFunction(body) && ts.isBlock(body.body), "the test's body is not a block").toBe(true);
    const tries = ((body as ts.ArrowFunction).body as ts.Block).statements.filter((s): s is ts.TryStatement => ts.isTryStatement(s));
    expect(tries, "the test's body holds no try, or more than one").toHaveLength(1);
    const clause = tries[0]!.catchClause;
    expect(clause?.variableDeclaration, "the try has no catch (e)").toBeDefined();
    const caught = clause!.variableDeclaration!.name.getText(sf);
    const [first, ...rest] = clause!.block.statements;

    // An ordinary run gets the caught error back unchanged, and first.
    expect(first?.getText(sf), "the catch does not begin by rethrowing the caught error unless WITHHOLD").toBe(`if (!WITHHOLD) throw ${caught};`);

    // Past that line the caught error is never read: not its message, its
    // matcher's snapshot, or anything else.
    const reads: string[] = [];
    const visit = (n: ts.Node): void => {
      if (ts.isIdentifier(n) && n.text === caught) reads.push(n.parent.getText(sf));
      ts.forEachChild(n, visit);
    };
    for (const s of rest) visit(s);
    expect(reads, "the withheld path reads the caught error").toEqual([]);

    // And it throws a fresh Error whose message is a literal, interpolating at
    // most the stage, `where`.
    const thrown = rest.filter((s): s is ts.ThrowStatement => ts.isThrowStatement(s));
    expect(thrown, "the withheld path throws no error, or more than one").toHaveLength(1);
    const made = thrown[0]!.expression;
    expect(ts.isNewExpression(made) && made.expression.getText(sf) === "Error", "the withheld path throws something other than new Error(…)").toBe(true);
    const args = (made as ts.NewExpression).arguments ?? ts.factory.createNodeArray();
    expect(args, "new Error takes one argument").toHaveLength(1);
    const message = args[0]!;
    const literal =
      ts.isStringLiteralLike(message) ||
      (ts.isTemplateExpression(message) && message.templateSpans.every((span) => ts.isIdentifier(span.expression) && span.expression.text === "where"));
    expect(literal, `the withheld message is not a literal and the stage: ${message.getText(sf)}`).toBe(true);
    expect(message.getText(sf), "the withheld message does not say the error is withheld").toContain("withheld");
  });

  it("empties every page in play's finally", () => {
    const play = top.find((s): s is ts.FunctionDeclaration => ts.isFunctionDeclaration(s) && s.name?.text === "play");
    expect(play?.body, "no function play").toBeDefined();
    const tries = play!.body!.statements.filter((s): s is ts.TryStatement => ts.isTryStatement(s));
    expect(tries, "play's body holds no try").toHaveLength(1);
    const fin = tries[0]!.finallyBlock;
    expect(fin, "play's try has no finally").toBeDefined();
    expect(blanks(fin!), 'the finally sends no page to "about:blank"').toBeGreaterThan(0);
    // And every page `play` opens is one the finally reaches: they are opened
    // through `open`, which records each, and `person` is called nowhere else.
    const persons = (n: ts.Node): number =>
      (ts.isCallExpression(n) && ts.isIdentifier(n.expression) && n.expression.text === "person" ? 1 : 0) +
      (n.getChildren(sf).reduce((k, c) => k + persons(c), 0));
    expect(persons(play!.body!), "play opens a page other than through open").toBe(1);
  });

  it("assigns PASSWORD no string literal anywhere", () => {
    const literals: string[] = [];
    const visit = (n: ts.Node): void => {
      const named =
        (ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && n.name.text === "PASSWORD" && n.initializer) ||
        (ts.isBinaryExpression(n) && n.operatorToken.kind === ts.SyntaxKind.EqualsToken && ts.isIdentifier(n.left) && n.left.text === "PASSWORD" && n.right);
      if (named && (ts.isStringLiteralLike(named) || ts.isTemplateExpression(named))) literals.push(n.getText(sf));
      ts.forEachChild(n, visit);
    };
    visit(sf);
    const declared = top.some(
      (s) => ts.isVariableStatement(s) && s.declarationList.declarations.some((d) => ts.isIdentifier(d.name) && d.name.text === "PASSWORD"),
    );
    expect(declared, "premise: PASSWORD is declared at the module's top").toBe(true);
    expect(literals, "PASSWORD is a literal in the public source").toEqual([]);
  });

  it("refuses the stranger arm at the module's top when the target is live", () => {
    // The condition EXACTLY `STRANGER && LIVE` and the branch a lone `throw`:
    // a looser read (both names somewhere in the condition) let `STRANGER &&
    // LIVE && false` pass when this cell was probed.
    const refusal = top.find(
      (s): s is ts.IfStatement =>
        ts.isIfStatement(s) &&
        s.expression.getText(sf) === "STRANGER && LIVE" &&
        s.elseStatement === undefined &&
        ts.isBlock(s.thenStatement) &&
        s.thenStatement.statements.length === 1 &&
        ts.isThrowStatement(s.thenStatement.statements[0]!),
    );
    expect(refusal, "no top-level if (STRANGER && LIVE) { throw … }").toBeDefined();
    // And STRANGER is the variable itself, read where it is declared.
    const declared = top.some(
      (s) =>
        ts.isVariableStatement(s) &&
        s.declarationList.declarations.some(
          (d) => ts.isIdentifier(d.name) && d.name.text === "STRANGER" && d.initializer?.getText(sf) === 'process.env.YOKOHAMA_WEB_README_SHOTS_STRANGER === "1"',
        ),
    );
    expect(declared, 'STRANGER is not process.env.YOKOHAMA_WEB_README_SHOTS_STRANGER === "1"').toBe(true);
    expect(top.indexOf(refusal!), "it comes after the first test").toBeLessThan(firstTest);
  });
});
