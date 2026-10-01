// **The restore validator, run through the COMPILED module** (M6.5b2,
// R-6.5b2.10 – .12; spec §6.1, §6.2).
//
// `crates/yokohama-web/src/backup.rs`'s own cells and `crates/yokohama-wasm`'s
// native cell prove the validator NATIVELY. This runs the same committed
// envelope through the module built for `wasm32-unknown-unknown`, in Node, and
// asserts the report is the one committed beside the envelope.
//
// **What that buys is the whole reason the jsdom cells can be honest about
// their fake.** `web/src/test/fakeWorker.ts` answers `restore_read` from the
// committed report, and this is what says that report is what the module really
// gives — across a compilation boundary, a different architecture and a
// different allocator. Without it the fake would be a second statement of a
// rule with nothing holding the two together.
//
// Its own project (`vitest.node.config.ts`, `npm run test:wasm`) so `npm test`
// never asks for a module it did not build.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));
const modulePath = path.resolve(here, "../wasm-node/yokohama_wasm.js");
const envelopePath = path.resolve(here, "wire-script/backup-envelope.json");
const reportPath = path.resolve(here, "wire-script/backup-report.json");

/**
 * The one export this file drives, declared HERE rather than imported from the
 * generated `.d.ts` — `corpus.node.test.ts`'s ruling P-2, for its reason: a
 * `typeof import(…)` would make this file untypecheckable wherever the module
 * has not been built, which is the `web-client` CI job and any fresh clone.
 */
interface WasmHost {
  restore_read(envelope: string): string;
}

const have = fs.existsSync(modulePath);
if (!have && !process.env.YOKOHAMA_WASM_OPTIONAL) {
  throw new Error(
    "the browser host's module is not built, so this command tests nothing — run `npm run " +
      "build:wasm` first (or set YOKOHAMA_WASM_OPTIONAL=1 to skip it, which tests NOTHING)",
  );
}
if (!have) {
  // Straight to the file descriptor, for `corpus.node.test.ts`'s measured
  // reason: vitest attaches a collecting file's `console` output to nothing
  // when its only suite is then skipped, and a silent green is what the opt-out
  // must never be.
  process.stderr.write(
    "\nYOKOHAMA_WASM_OPTIONAL is set and the module is absent: the backup validator ran NO\n" +
      "assertions at all. Nothing in web/src/wire-script/ was checked against the module.\n\n",
  );
}

describe.skipIf(!have)("the browser host's restore validator", () => {
  const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
  const envelope = fs.readFileSync(envelopePath, "utf8");
  const committed: unknown = JSON.parse(fs.readFileSync(reportPath, "utf8"));

  it("gives the committed report for the committed envelope", () => {
    const report: unknown = JSON.parse(wasm.restore_read(envelope));
    expect(report).toEqual(committed);
  });

  // **The four arms, said back** — the anti-vacuity partner for the compare
  // above, which would pass just as well over a report whose every verdict was
  // the same word.
  it("answers all four verdicts", () => {
    const report = JSON.parse(wasm.restore_read(envelope)) as {
      id: string;
      verdict: { kind: string };
    }[];
    expect(report.map((r) => r.verdict.kind)).toEqual([
      "accepted",
      "dealing",
      "unplayable",
      "rejected",
    ]);
  });

  // A file this build cannot read is refused AS A WHOLE, through the export the
  // page really calls — and the refusal reaches JS as a thrown error, which is
  // the shape `client.ts`'s `localRestoreRead` turns into its rejection.
  //
  // **The version-2 file carries a key version 1 lacks** (M6.5c2 cycle D, the
  // mutation battery's row 41), as `backup.rs`'s own
  // `an_envelope_of_another_version_is_refused_whole` makes its file: a newer
  // envelope is newer because it says something this build does not know. A
  // file that bumped only the number parsed whole as version 1, so it answered
  // "version 2" whichever the order, and the version-before-parse order had no
  // witness on this host. With the key, a whole parse first answers serde's
  // "unknown field" and this cell fails.
  it("refuses a file of another version, as a throw", () => {
    const newer = envelope
      .replace('"envelope": 1', '"envelope": 2')
      .replace('{\n  "kind"', '{\n  "added_in_version_2": true,\n  "kind"');
    expect(newer, "the mutation must change the version").toContain('"envelope": 2');
    expect(newer, "and add a key version 1 lacks").toContain('"added_in_version_2": true');
    // Refused BY ITS VERSION (M6.5c2 cycle D, straw poll row 15), in the
    // sentence `backup.rs`'s `restore_read` words, and not by the key.
    expect(() => wasm.restore_read(newer)).toThrow(/version 2 backup/);
    expect(() => wasm.restore_read(newer)).not.toThrow(/added_in_version_2/);
  });
});
