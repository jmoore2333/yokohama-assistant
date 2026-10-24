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
const jobsAt = (name) => text.indexOf(`\n  ${name}:\n`);
const nextJob = (from) => { const r = text.slice(from + 1).search(/\n  [a-z][\w-]*:/); return r < 0 ? text.length : from + 1 + r; };

describe("release.yml", () => {
  it("pins every action to a commit SHA with its version beside it", () => {
    // every `uses:` value is `<owner>/<repo>@<40 hex>` followed by a `# <version>` comment
    const uses = text.split("\n").filter((l) => /^\s*-?\s*uses:/.test(l));
    expect(uses.length).toBeGreaterThanOrEqual(6);
    for (const l of uses) expect(l, l).toMatch(/uses:\s+\S+@[0-9a-f]{40}\s+#\s*\S/);
  });

  it("grants contents: read at the top level and contents: write to the publish job alone", () => {
    // the top-level permissions block is `contents: read`; with comments blanked to equal-length spaces (so indices
    // hold), the WORD `write`/`write-all` occurs exactly once in the file and inside the publish job's span — every
    // spelling of a grant GitHub accepts (a flow mapping, a quoted key or value, a trailing comment, an explicit
    // `!!str` tag) carries the word, and anchors, aliases and merge keys, which could carry one by reference, are
    // refused outright (P-10, P-11)
    const top = text.indexOf("\npermissions:\n");
    expect(text.slice(top, top + 40)).toContain("\npermissions:\n  contents: read\n");
    const code = text.replace(/(^|\s)#.*$/gm, (m, p) => p + " ".repeat(m.length - p.length));
    expect(code).not.toMatch(/:\s+[&*][\w-]|<<:/);
    const writes = [...code.matchAll(/\bwrite(-all)?\b/g)];
    const pub = jobsAt("publish");
    expect(writes).toHaveLength(1);
    expect(writes[0].index).toBeGreaterThan(pub);
    expect(writes[0].index).toBeLessThan(nextJob(pub));
  });

  it("runs on v* tags and refuses a tag off master in a first job every build needs", () => {
    // the trigger and the tag rule: the merge-base line sits inside the tag job's OWN span (bounded by the next job
    // head, so an inserted job cannot hold it instead), ends at `origin/master` or at the exact loud clause (so no
    // `exit 0` or `|| true` can follow), and the file carries no `if:` and no `continue-on-error` anywhere (P-10, P-11)
    expect(text).toMatch(/on:\n\s+push:\n\s+tags:\n\s+- "v\*"/);
    const t0 = jobsAt("tag");
    expect(t0).toBeGreaterThan(0);
    const tagSpan = text.slice(t0, nextJob(t0));
    expect(tagSpan).toMatch(/^\s*(run: .*&& )?git merge-base --is-ancestor "\$GITHUB_SHA" origin\/master( \|\| \{ echo "[^"]*"; exit 1; \})?$/m);
    expect(text).not.toMatch(/continue-on-error|^\s*if:/m);
    for (const job of ["ubuntu", "macos", "windows"]) {
      expect(text.slice(jobsAt(job), jobsAt(job) + 120)).toContain("needs: [tag]");
    }
  });
});
