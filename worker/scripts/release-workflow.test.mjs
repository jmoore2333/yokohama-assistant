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
    // the top-level permissions block is `contents: read`; the one write grant, in any spelling, sits in the publish job's span
    const top = text.indexOf("\npermissions:\n");
    expect(top).toBeGreaterThan(0);
    expect(text.slice(top, top + 40)).toContain("\npermissions:\n  contents: read\n");
    const writes = [...text.matchAll(/^\s*[a-z-]+:\s*["']?write(-all)?["']?\s*$/gm)];
    const pub = jobsAt("publish");
    expect(pub).toBeGreaterThan(0);
    const rest = text.slice(pub + 1).search(/\n  [a-z][\w-]*:\n/);
    const end = rest < 0 ? text.length : pub + 1 + rest;
    expect(writes).toHaveLength(1);
    expect(writes[0].index).toBeGreaterThan(pub);
    expect(writes[0].index).toBeLessThan(end);
  });

  it("runs on v* tags and refuses a tag off master in a first job every build needs", () => {
    // the trigger and the tag rule
    expect(text).toMatch(/on:\n\s+push:\n\s+tags:\n\s+- "v\*"/);
    expect(jobsAt("tag")).toBeGreaterThan(0);
    // the check sits uncommented inside the tag job's span, with no way to pass around it
    const tagSpan = text.slice(jobsAt("tag"), jobsAt("ubuntu"));
    expect(tagSpan).toMatch(/^\s*(run: .*&& )?git merge-base --is-ancestor "\$GITHUB_SHA" origin\/master/m);
    expect(tagSpan).not.toMatch(/continue-on-error|^\s*if:/m);
    for (const job of ["ubuntu", "macos", "windows"]) {
      expect(text.slice(jobsAt(job), jobsAt(job) + 120)).toContain("needs: [tag]");
    }
  });
});
