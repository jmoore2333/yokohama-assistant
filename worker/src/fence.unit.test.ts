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
 * fenced without an edit here. Comments are cut before the search, because a
 * comment may name the two calls to say why they are not made.
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

/** Every source file under `src/` that is not a test, relative to it, with
 *  its text less its comments. `//` is a comment only where a `:` does not
 *  precede it, so a URL in a string survives. */
function sources(): Map<string, string> {
  const files = fs
    .readdirSync(HERE, { recursive: true })
    .filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts"))
    .sort();
  return new Map(
    files.map((f) => [
      f,
      fs
        .readFileSync(new URL(f, HERE), "utf8")
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .replace(/(^|[^:])\/\/.*$/gm, "$1"),
    ]),
  );
}

describe("objects are reached by name (pool fact 8)", () => {
  it("no source file calls idFromString or newUniqueId", () => {
    const read = sources();
    // The premise: it read the Worker, where every stub is made, and found a
    // call it CAN find — so it cannot pass by reading nothing.
    expect(read.get("index.ts"), "premise: the Worker's own file was read").toMatch(/\bidFromName\s*\(/);
    const offenders = [...read].filter(([, text]) => /\b(?:idFromString|newUniqueId)\s*\(/.test(text)).map(([f]) => f);
    expect(offenders).toEqual([]);
  });
});
