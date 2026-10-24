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
// (P-12) STRUCTURAL pins, on the text with comments blanked to equal-length spaces: the job heads under `jobs:`
// are EXACTLY the five, bare block keys in order, so a job of any other spelling (`Gate`, `_gate`, `"gate"`,
// `gate :`, a flow-style job) is red and every span below is exact
const code = text.replace(/(^|\s)#.*$/gm, (m, p) => p + " ".repeat(m.length - p.length));
const jobsAt = (name) => code.indexOf(`\n  ${name}:\n`);
const span = (name, next) => code.slice(jobsAt(name), next ? jobsAt(next) : code.length);
const expectTheFiveJobHeads = () => {
  const jobsStart = code.indexOf("\njobs:\n");
  expect(jobsStart).toBeGreaterThan(0);
  expect(code.slice(jobsStart).match(/^  \S.*$/gm)).toEqual(["  tag:", "  ubuntu:", "  macos:", "  windows:", "  publish:"]);
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
    // the permissions: the token `permissions` occurs exactly twice in the file, case-blind — the top-level block,
    // pinned as `contents: read` and nothing more, and publish's block, pinned as `contents: write` and nothing
    // more — so a grant under any other key spelling, in any job, is red; the word write/write-all occurs once, in
    // publish (a belt). Not pinned, recorded: a YAML escape that spells a key or a value without its letters
    expect(code.match(/permissions/gi)).toHaveLength(2);
    expect(code).toMatch(/\npermissions:\n  contents: read\n(?! )/);
    expect(span("publish")).toMatch(/\n    permissions:\n      contents: write\n(?!      )/);
    expect(code.match(/\bwrite(-all)?\b/gi)).toHaveLength(1);
    expect(code.search(/\bwrite(-all)?\b/i)).toBeGreaterThan(jobsAt("publish"));
    // nothing conditional, nothing tolerant, nothing by reference — the key `if` in every spelling (quoted, a space
    // before the colon, the explicit `? if` form), `continue-on-error`, anchors, aliases and merge keys
    expect(code).not.toMatch(/(^|[\s{,?-])["']?if["']?\s*:/m);
    expect(code).not.toMatch(/continue-on-error/i);
    expect(code).not.toMatch(/:\s*[&*][\w-]|<<\s*:/);
  });

  it("runs on v* tags and refuses a tag off master in a first job every build needs", () => {
    expectTheFiveJobHeads();
    // the trigger; the tag job's step pinned WHOLE as the last lines of its span (so no wrap, no extra line, no
    // `exit 0`, no `|| true`, nothing moved); the builds' `needs: [tag]`; publish's `needs`
    expect(text).toMatch(/on:\n\s+push:\n\s+tags:\n\s+- "v\*"/);
    expect(span("tag", "ubuntu")).toMatch(/\n      - name: The tag must sit on master\n        run: \|\n          git fetch --no-tags origin master\n          git merge-base --is-ancestor "\$GITHUB_SHA" origin\/master \|\| \{ echo "[^"\n]*"; exit 1; \}\n\s*$/);
    for (const job of ["ubuntu", "macos", "windows"]) expect(span(job).slice(0, 120)).toContain("needs: [tag]");
    expect(span("publish")).toContain("needs: [ubuntu, macos, windows]");
  });
});
