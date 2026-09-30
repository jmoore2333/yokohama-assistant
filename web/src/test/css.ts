// Reading the stylesheet's rules, in a test.
//
// jsdom loads no stylesheet, so a claim about a declaration is a claim about
// the file. **One parser, here** (Task 4b's review, Minor 6): `styles.test.ts`
// and `FrontDoor.test.tsx` each carried a copy, and two copies of a parser are
// two places for a rule to be read differently. It lives beside the other test
// helpers rather than in `styles.test.ts`, because importing a test file from
// another registers its cells a second time in the importer's run.

/** The stylesheet's top-level rules, comments stripped, as selector lists and
 *  bodies — what is inside an `@media` (or any `@`) block is skipped whole. */
export function topLevelRules(css: string): { selectors: string[]; body: string }[] {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules: { selectors: string[]; body: string }[] = [];
  let depth = 0;
  let start = 0;
  let head = "";
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "{") {
      if (depth === 0) {
        head = text.slice(start, i).trim();
        start = i + 1;
      }
      depth++;
    } else if (text[i] === "}") {
      depth--;
      if (depth === 0) {
        if (!head.startsWith("@")) {
          rules.push({ selectors: head.split(",").map((s) => s.trim()), body: text.slice(start, i) });
        }
        start = i + 1;
      }
    }
  }
  return rules;
}
