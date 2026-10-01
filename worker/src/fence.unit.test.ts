import ts from "typescript";
import { describe, expect, it } from "vitest";

/**
 * **Every object is reached by its NAME** (the plan's pool fact 8): an object
 * made by `newUniqueId`, or reached only through `idFromString`, has no
 * `ctx.id.name` — and `meta` stores no id, so such an object could never know
 * which table it serves, and an alarm it armed would find no name (the pool
 * cannot even show that case; production can). So no source file under
 * `worker/src` calls either, and every stub is made with `idFromName`.
 *
 * The directory is read AT RUN TIME, so every object a later task adds is
 * fenced without an edit here. The calls are found by TypeScript's own parser
 * (Task 11's review, I2): a comment or a string that names them is never a
 * call, and no comment can hide code from the search — a text search that cut
 * block comments first read the `/*` in a line comment's `/api/*` as an opener,
 * and was blind to everything up to the next `*\/`.
 */

/** The two node calls this cell makes. The project's types are workerd's,
 *  which declare no node module, so `node:fs` is imported through a variable
 *  specifier and typed here; the file runs in node (`npm run test:scripts`). */
interface Fs {
  readdirSync(path: URL, options: { recursive: true }): string[];
  readFileSync(path: URL, encoding: "utf8"): string;
}
const FS = "node:fs";
const fs = (await import(/* @vite-ignore */ FS)) as Fs;

/** `import.meta.url`, which the Workers types do not declare. */
const HERE = new URL(".", (import.meta as unknown as { url: string }).url);

/** The name a call is made through: `x.name(…)`, `x?.name(…)`, `x["name"](…)`
 *  or a bare `name(…)`. */
function calleeName(callee: ts.Expression): string | undefined {
  if (ts.isPropertyAccessExpression(callee)) return callee.name.text;
  if (ts.isIdentifier(callee)) return callee.text;
  if (ts.isElementAccessExpression(callee) && ts.isStringLiteralLike(callee.argumentExpression)) {
    return callee.argumentExpression.text;
  }
  return undefined;
}

/** Every call in `text` made through one of `names`, as `name:line`. */
function calls(text: string, names: readonly string[]): string[] {
  const source = ts.createSourceFile("source.ts", text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const found: string[] = [];
  const visit = (node: ts.Node): void => {
    if (ts.isCallExpression(node)) {
      const name = calleeName(node.expression);
      if (name !== undefined && names.includes(name)) {
        found.push(`${name}:${source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1}`);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return found;
}

const FORBIDDEN = ["idFromString", "newUniqueId"] as const;

/** Every source file under `src/` that is not a test, relative to it. */
function sources(): Map<string, string> {
  const files = fs
    .readdirSync(HERE, { recursive: true })
    .filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts"))
    .sort();
  return new Map(files.map((f) => [f, fs.readFileSync(new URL(f, HERE), "utf8")]));
}

describe("the search the fence runs", () => {
  // I2's own mutant, in miniature: a `/*` inside a line comment opens no
  // comment, so the call after it is found.
  it("finds a call after a line comment that holds `/*`", () => {
    const text = '// every `/api/*` path\nconst a = 1;\nenv.TABLE.idFromString(x);\n/* a real comment */\n';
    expect(calls(text, FORBIDDEN)).toEqual(["idFromString:3"]);
  });
  it("never counts a name in a comment or a string", () => {
    const text = '/** made by `newUniqueId()`, or reached only through `idFromString()` */\nconst s = "idFromString(x)";\n// newUniqueId()\n';
    expect(calls(text, FORBIDDEN)).toEqual([]);
  });
  it("finds a call made through an optional chain, an element access or a bare name", () => {
    const text = 'a?.newUniqueId();\nb["idFromString"](y);\nidFromString(z);\n';
    expect(calls(text, FORBIDDEN)).toEqual(["newUniqueId:1", "idFromString:2", "idFromString:3"]);
  });
});

describe("objects are reached by name (pool fact 8)", () => {
  it("no source file calls idFromString or newUniqueId", () => {
    const read = sources();
    // The premise: it read the Worker, where every stub is made, and found a
    // call it CAN find — so it cannot pass by reading nothing.
    expect(calls(read.get("index.ts") ?? "", ["idFromName"]).length, "premise: idFromName calls in index.ts").toBeGreaterThan(0);
    const offenders = [...read].flatMap(([file, text]) => calls(text, FORBIDDEN).map((at) => `${file} ${at}`));
    expect(offenders).toEqual([]);
  });
});
