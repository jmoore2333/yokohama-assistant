// **`release.yml` reads by default, writes from the publish job alone, pins
// every action to a commit SHA and refuses a tag off master** (security S4a
// Task 3; the ledger's D17).
//
// The workflow runs on a tag push and nothing runs it here, so the shape is
// held at the text level: no YAML library, the file read as the reader reads
// it. The first public run is S4c's `0.7.3` tag.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const text = readFileSync(new URL("../../.github/workflows/release.yml", import.meta.url), "utf8");
// (P-12, P-13) STRUCTURAL pins. Module level: `code` is the text with comments blanked to equal-length spaces
// (indices hold); `body` blanks the block-scalar bodies (`run: |`) the same way on top of it, so a shell glob
// cannot trip the anchor refusal and an anchor or alias token anywhere else is visible; every lookup is bounded
// under `jobs:` (a decoy `  publish:` line before it moves nothing); the job heads under `jobs:` are EXACTLY the
// five, bare block keys in order, and every job body is block style (its first line a bare four-space key), so a
// job of any other spelling, an inserted job, or a flow-style body hiding a key behind a quoted `#` is red
const code = text.replace(/(^|\s)#.*$/gm, (m, p) => p + " ".repeat(m.length - p.length));
const body = (() => {
  const lines = code.split("\n");
  let indent = -1;
  for (let i = 0; i < lines.length; i++) {
    if (indent >= 0) {
      if (lines[i].trim() === "" || lines[i].search(/\S/) > indent) { lines[i] = " ".repeat(lines[i].length); continue; }
      indent = -1;
    }
    const m = /^(\s*)[^\s#][^:]*:\s*[|>][-+]?\s*$/.exec(lines[i]);
    if (m) indent = m[1].length;
  }
  return lines.join("\n");
})();
const jobsStart = code.indexOf("\njobs:\n");
const HEADS = ["tag", "ubuntu", "macos", "windows", "publish"];
const jobsAt = (name) => code.indexOf(`\n  ${name}:\n`, jobsStart);
const span = (name, next) => code.slice(jobsAt(name), next ? jobsAt(next) : code.length);
const expectTheFiveJobHeads = () => {
  expect(jobsStart).toBeGreaterThan(0);
  expect(code.slice(jobsStart).match(/^  \S.*$/gm)).toEqual(HEADS.map((h) => `  ${h}:`));
  for (const h of HEADS) expect(code.slice(jobsAt(h) + 1)).toMatch(/^  [a-z]+:\n    [a-z-]+:/);
};

describe("release.yml", () => {
  it("pins every action to a commit SHA with its version beside it", () => {
    // every `uses:` value is `<owner>/<repo>@<40 hex>` followed by a `# <version>` comment
    const uses = text.split("\n").filter((l) => /^\s*-?\s*uses:/.test(l));
    expect(uses.length).toBeGreaterThanOrEqual(6);
    for (const l of uses) expect(l, l).toMatch(/uses:\s+\S+@[0-9a-f]{40}\s+#\s*\S/);
  });

  it("grants contents: read at the top level and contents: write to the publish job alone", () => {
    expectTheFiveJobHeads();
    // --- in the permissions cell, after expectTheFiveJobHeads(): the token `permissions` occurs exactly twice in the
    // file, case-blind — the top-level block, pinned as `contents: read` and nothing more, and publish's block,
    // pinned as `contents: write` and nothing more — so a grant under any other key spelling, in any job, is red;
    // the word write/write-all occurs once, in publish (a belt). Not pinned, recorded: a YAML escape that spells a
    // key or a value without its letters
    expect(code.match(/permissions/gi)).toHaveLength(2);
    expect(code).toMatch(/\npermissions:\n  contents: read\n(?! )/);
    expect(span("publish")).toMatch(/\n    permissions:\n      contents: write\n(?!      )/);
    expect(code.match(/\bwrite(-all)?\b/gi)).toHaveLength(1);
    expect(code.search(/\bwrite(-all)?\b/i)).toBeGreaterThan(jobsAt("publish"));
    // nothing conditional, nothing tolerant, nothing by reference — the key `if` in every spelling (quoted, a space
    // before the colon, the explicit `? if` form), `continue-on-error`, and any `&` anchor or `*` alias token at a
    // YAML node start — after the indentation at a line's start, or after `:`, `?`, `,`, `[`, `{` or `-` and optional
    // blanks — or a `<<` merge key, read on `body` so a shell glob or `&&` in a run script does not count (P-14: a
    // plain one-line `run: … && …` is neither a block body nor quoted, so a whitespace-bound refusal read red on the
    // shipped file; an alias cannot carry properties and always sits at a node start, where this one sees it)
    expect(code).not.toMatch(/(^|[\s{,?-])["']?if["']?\s*:/m);
    expect(code).not.toMatch(/continue-on-error/i);
    expect(body).not.toMatch(/(^[ \t]*|[:?,\[{-][ \t]*)[&*]/m);
    expect(body).not.toMatch(/<<\s*:/);
  });

  it("runs on v* tags and refuses a tag off master in a first job every build needs", () => {
    expectTheFiveJobHeads();
    // --- in the tag cell, after expectTheFiveJobHeads(): the trigger; the tag job pinned WHOLE — its name, runner,
    // the checkout at a 40-hex pin with `fetch-depth: 0`, and the one step — so no extra key (`defaults`, `env`,
    // `container`), no pre-step, no wrap, no extra line and no `exit 0` can neuter it; the builds' `needs: [tag]`;
    // publish's `needs`
    expect(text).toMatch(/on:\n\s+push:\n\s+tags:\n\s+- "v\*"/);
    expect(span("tag", "ubuntu")).toMatch(/^\n  tag:\n    name: Tag is on master\n    runs-on: ubuntu-24\.04\n    steps:\n      - uses: actions\/checkout@[0-9a-f]{40} *\n        with:\n          fetch-depth: 0\n      - name: The tag must sit on master\n        run: \|\n          git fetch --no-tags origin master\n          git merge-base --is-ancestor "\$GITHUB_SHA" origin\/master \|\| \{ echo "[^"\n]*"; exit 1; \}\n\s*$/);
    for (const job of ["ubuntu", "macos", "windows"]) expect(span(job).slice(0, 120)).toContain("needs: [tag]");
    expect(span("publish")).toContain("needs: [ubuntu, macos, windows]");
  });
});
