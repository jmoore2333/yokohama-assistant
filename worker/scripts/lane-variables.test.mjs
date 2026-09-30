// **The lane document's variables table is the names the code reads** (M6.5c2
// cycle E, R-6.5c2.45).
//
// `docs/web-lane.md` §12, "Every variable", is one table of every variable
// the web lane reads: its name, kind, default, meaning, host and file. The
// owner asked that the table be checked against the code "where a reader for
// them exists", so this cell greps the lane's code for every name that has
// the lane's shape — `YOKOHAMA_*`, `VITE_*` and `DEV_DOOR_CLIENT_ID` — and
// holds the table to it both ways:
// - every such name in the code is a row, so a variable added later fails
//   here until the table defines it;
// - every row's name is in the code, so a variable the code stops reading
//   fails here until its row goes (names of other shapes — `ACCESS_AUD`,
//   `CI`, `UPDATE_WIRE_FIXTURES` — are held to this half only).
//
// "The code" is the directories and files the cycle E dossier's item 26 read
// (`docs/plans/2026-09-29-web-frontend-m6.5c2e-dossier.md`, "The variable
// inventory"), listed in SCANNED below, tracked files only, code only — no
// Markdown and no JSON, whose corpus and fixtures are data. A mention in a
// comment counts: a name the code talks about is one a reader looks up.
//
// What it cannot see: a variable read under a name of another shape
// (`process.env.TZ` in one cell is the operating system's, not the lane's),
// and a row whose OTHER columns have drifted — the default, the meaning, the
// host. Those are read by eye at a close.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const repo = fileURLToPath(new URL("../..", import.meta.url)).replace(/\/$/, "");

/** This file names the patterns and the exemption; it is not scanned. */
const SELF = "worker/scripts/lane-variables.test.mjs";

/** The web lane's code, as the dossier's item 26 read it: the three crates,
 *  the client and the shell, the self-host image and the two web workflows.
 *  `release.yml` and `test.yml` are the CLI's and the engine's. */
const SCANNED = [
  "crates/yokohama-web",
  "crates/yokohama-wasm",
  "crates/yokohama-worker",
  "web",
  "worker",
  "deploy",
  ".github/workflows/web.yml",
  ".github/workflows/docker.yml",
];

/** Code, by name: sources, configurations, scripts and the image's files. */
const CODE = /(\.(rs|toml|ts|tsx|mts|mjs|js|jsonc|yml|yaml|sh)|\/Dockerfile[^/]*|\/package\.json)$/;

/** The lane's shape: a whole name, never a prefix a comment spells with a
 *  trailing `_` or `*`. */
const SHAPE = /\b(?:(?:YOKOHAMA|VITE)_[A-Z0-9]+(?:_[A-Z0-9]+)*|DEV_DOOR_CLIENT_ID)\b/g;

/** Names of the lane's shape that are not variables, each with its reason.
 *  A stale entry — a name the code no longer spells, or one the table
 *  defines — fails its own cell below. */
const NOT_VARIABLES = new Map([
  [
    "VITE_MULTIPLAER",
    "a deliberate misspelling in web/src/vite-env.d.ts's comment: the typo strictImportMetaEnv turns into a compile error",
  ],
]);

/** The section's heading, exactly as the document writes it. */
const SECTION = "## 12. Every variable";

/** Every tracked code file under SCANNED but this one, with its text. */
function codeFiles() {
  const listed = execFileSync("git", ["ls-files", "-z", "--", ...SCANNED], { cwd: repo, encoding: "utf8" });
  return listed
    .split("\0")
    .filter((path) => path !== "" && path !== SELF && CODE.test(`/${path}`))
    .map((path) => ({ path, text: readFileSync(`${repo}/${path}`, "utf8") }));
}

/** Every name of the lane's shape in `text`. */
function namesIn(text) {
  return new Set(text.match(SHAPE) ?? []);
}

/**
 * Every row of §12's table whose first cell is a name: one code span,
 * `` `NAME` ``. A first cell that OPENS with a backtick and does not parse is
 * refused, never skipped. The header and the separator open with no backtick.
 */
function tableNames(doc) {
  const lines = doc.split("\n");
  const start = lines.findIndex((line) => line.trimEnd() === SECTION);
  if (start === -1) throw new Error(`docs/web-lane.md has no line ${JSON.stringify(SECTION)}`);
  const names = [];
  for (const line of lines.slice(start + 1)) {
    if (line.startsWith("## ")) break;
    const trimmed = line.trim();
    if (!trimmed.startsWith("|")) continue;
    const first = trimmed.slice(1).split("|")[0].trim();
    if (!first.startsWith("`")) continue;
    const m = /^`([A-Z][A-Z0-9_]*)`$/.exec(first);
    if (!m) throw new Error(`a variable's cell is one code span holding one name: ${JSON.stringify(line)}`);
    names.push(m[1]);
  }
  return names;
}

function lanesDocument() {
  return readFileSync(`${repo}/docs/web-lane.md`, "utf8");
}

describe("the lane document's variables table", () => {
  // **The catcher for a variable the code gains** (a new `YOKOHAMA_*` read
  // anywhere in the lane) and for one the table drops.
  // Mutants: a row deleted from §12; a new `process.env.YOKOHAMA_WEB_X` in a
  // web script; a SCANNED entry dropped (the names only it holds go missing
  // from the code side, and their rows then fail the next cell).
  it("names every YOKOHAMA_, VITE_ and DEV_DOOR_CLIENT_ID name the lane's code spells", () => {
    const files = codeFiles();
    const found = new Set();
    for (const { text } of files) for (const name of namesIn(text)) found.add(name);
    for (const name of NOT_VARIABLES.keys()) found.delete(name);
    // Anti-vacuity: the scan read the lane, not an empty list.
    expect(files.length, "premise: the scan reads the lane's code").toBeGreaterThan(100);
    expect(found.has("YOKOHAMA_WEB_IDENTITY") && found.has("VITE_MULTIPLAYER") && found.has("DEV_DOOR_CLIENT_ID")).toBe(true);
    const table = new Set(tableNames(lanesDocument()));
    const missing = [...found].filter((name) => !table.has(name)).sort();
    expect(missing, "in the code, not in docs/web-lane.md §12").toEqual([]);
  });

  // **The catcher for a row nothing reads**: a variable renamed or retired in
  // the code while its row stays, and a row whose name is misspelt.
  // Mutants: a row `YOKOHAMA_WEB_NOTHING`; a row's name misspelt; a name of
  // another shape (`ACCESS_AUD`) misspelt in its row.
  it("defines nothing the lane's code does not spell", () => {
    const code = codeFiles()
      .map(({ text }) => text)
      .join("\n");
    const names = tableNames(lanesDocument());
    expect(names.length, "premise: §12 has rows").toBeGreaterThan(30);
    const unread = names.filter((name) => !new RegExp(`\\b${name}\\b`).test(code)).sort();
    expect(unread, "in docs/web-lane.md §12, spelled nowhere in the lane's code").toEqual([]);
    const twice = names.filter((name, i) => names.indexOf(name) !== i);
    expect(twice, "a name with two rows").toEqual([]);
  });

  it("exempts only names the code spells and the table does not define", () => {
    const found = new Set();
    for (const { text } of codeFiles()) for (const name of namesIn(text)) found.add(name);
    const table = new Set(tableNames(lanesDocument()));
    for (const [name, reason] of NOT_VARIABLES) {
      expect(found.has(name), `${name} is exempted (${reason}) and spelled nowhere: drop the exemption`).toBe(true);
      expect(table.has(name), `${name} is exempted and also defined in §12`).toBe(false);
    }
  });

  // The parser's and the pattern's own guard, over text, so the cells above
  // cannot pass by reading nothing.
  it("reads a name row, refuses a malformed one, stops at the next section, and takes no prefix", () => {
    const doc = "# t\n## 12. Every variable\n\n| Name | Kind |\n|---|---|\n| `YOKOHAMA_WEB_PORT` | launch |\n| `CI` | test-only |\n## 13. Next\n| `NOT_THIS` | x |\n";
    expect(tableNames(doc)).toEqual(["YOKOHAMA_WEB_PORT", "CI"]);
    for (const bad of ["| `A`, `B` | x |", "| `yokohama_web` | x |", "| `YOKOHAMA_WEB_PORT | x |"]) {
      expect(() => tableNames(`## 12. Every variable\n${bad}\n`), bad).toThrow(/one code span/);
    }
    expect(() => tableNames("## 11. Other\n")).toThrow(/no line/);
    expect([...namesIn("YOKOHAMA_WEB_* and `YOKOHAMA_WEB_` and yokohama_web and X_YOKOHAMA_WEB_DIR, VITE_A_B.")]).toEqual(["VITE_A_B"]);
  });
});
