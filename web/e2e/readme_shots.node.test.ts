// **What a failed README run may write, held by the spec's own source** (the
// whole-branch review, W-2 and W-3; `readme_shots.spec.ts`'s header).
//
// The README generator is about to run against a public host. A FAILED run must
// not leave a stranger's text or pixels behind: Playwright 1.62.1 writes an
// unredacted accessibility snapshot into `error-context.md` unless
// `PLAYWRIGHT_NO_COPY_PROMPT` is set, and the config's failure screenshots shoot
// every open page unmasked. The spec carries its own switches so a run cannot
// forget them, and these cells read the spec's source, parsed by the TypeScript
// compiler (`live.node.test.ts`'s manner), so the switches cannot quietly go:
//
//   - `process.env.PLAYWRIGHT_NO_COPY_PROMPT = "1"` and
//     `test.use({ screenshot: "off", video: "off", trace: "off" })` are
//     statements of the module itself, before its first `test(…)`;
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
    const refusal = top.find(
      (s) =>
        ts.isIfStatement(s) &&
        /\bSTRANGER\b/.test(s.expression.getText(sf)) &&
        /\bLIVE\b/.test(s.expression.getText(sf)) &&
        /\bthrow\b/.test(s.thenStatement.getText(sf)),
    );
    expect(refusal, "no top-level if (STRANGER && LIVE) throw").toBeDefined();
    expect(top.indexOf(refusal!), "it comes after the first test").toBeLessThan(firstTest);
  });
});
