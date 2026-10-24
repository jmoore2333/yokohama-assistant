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

describe("release.yml", () => {
  it("pins every action to a commit SHA with its version beside it", () => {
    // every `uses:` value is `<owner>/<repo>@<40 hex>` followed by a `# <version>` comment
    const uses = text.split("\n").filter((l) => /^\s*-?\s*uses:/.test(l));
    expect(uses.length).toBeGreaterThanOrEqual(6);
    for (const l of uses) expect(l, l).toMatch(/uses:\s+\S+@[0-9a-f]{40}\s+#\s*\S/);
  });

  it("grants contents: read at the top level and contents: write to the publish job alone", () => {
    // the top-level permissions block is `contents: read`; `contents: write` appears once, inside the publish job
    const top = text.indexOf("\npermissions:\n");
    expect(top).toBeGreaterThan(0);
    expect(text.slice(top, top + 40)).toContain("\npermissions:\n  contents: read\n");
    expect(text.match(/contents: write/g)).toHaveLength(1);
    expect(jobsAt("publish")).toBeGreaterThan(0);
    expect(text.indexOf("contents: write")).toBeGreaterThan(jobsAt("publish"));
  });

  it("runs on v* tags and refuses a tag off master in a first job every build needs", () => {
    // the trigger and the tag rule
    expect(text).toMatch(/on:\n\s+push:\n\s+tags:\n\s+- "v\*"/);
    expect(jobsAt("tag")).toBeGreaterThan(0);
    expect(text).toContain('git merge-base --is-ancestor "$GITHUB_SHA" origin/master');
    for (const job of ["ubuntu", "macos", "windows"]) {
      expect(text.slice(jobsAt(job), jobsAt(job) + 120)).toContain("needs: [tag]");
    }
  });
});
