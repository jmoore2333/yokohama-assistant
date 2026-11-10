// **The reader-facing documents hold their links to the tree** (security S4b,
// Task 2; the design §7.2 and §10.15): a repository that is about to be public
// is read by following its links, so a link to a file that moved, or to a
// heading that was renamed, is a broken first impression this cell catches.
//
// It reads the eight documents a newcomer starts from: the root `README.md`,
// the docs index, the method page, the engine page, the two guides,
// `CONTRIBUTING.md` and `SECURITY.md`. From each it collects every markdown
// reference `](<target>)`
// (a `<…>` wrapper and a quoted title allowed; a multi-line link text is no
// obstacle, since the reference starts at `](`) and every
// `<a … href="<target>">` value, quoted either way. Fenced code blocks are not
// read: a link shown as an example is not a link. A file's lines are read alike
// whether they end in LF or CRLF. A target with a URL scheme (`https:`,
// `mailto:` …) or one that starts with `//` is not in the tree and is skipped.
// Every other target is resolved — a leading `/` from the repository root,
// anything else from the referencing file's own directory (a target that is
// only `#anchor` points into the referencing file itself) — and must exist as a
// file or a directory in the tree GitHub serves: every component in its exact
// case, none a symbolic link, and, when there is a `.git` to ask, tracked by
// git (a directory when a tracked file lies under it). A `#anchor` must name a
// heading of the target markdown file by GitHub's slug (below), or an
// `id`/`name` attribute outside its fences, or an `id`/`name` attribute of a
// target HTML file.
//
// The slug is GitHub's: the heading's text with its markdown stripped (a code
// span keeps its text verbatim, its backticks gone; outside one a link keeps
// its text, and an HTML tag and the `*` emphasis mark go), lower-cased, every
// character that is not a letter, a combining mark, a digit, whitespace, `-` or
// `_` removed, each space turned into `-`; a slug that repeats within one file
// takes `-1`, `-2`, … on its later occurrences, skipping a suffix that is
// already some heading's slug (github-slugger's rule). Two divergences are
// known and recorded (Task 2's re-review): an `_` emphasis mark stays, and a
// no-break space or a tab stays, where GitHub drops each. The cell
// `the slug rule …` proves the rule against headings whose anchors the tree
// already links, so a defect in the rule reads there and not as a "broken"
// link.
//
// The anti-vacuity floors: at least 40 links are read in all, so an empty read
// can never pass, and the count is printed in the assertion message; and every
// document yields at least one link, so one whose links drop out of the read
// cannot hide under the global count.
//
// Mutants these cells are written to kill (the plan's Review Focus 3): a link's
// path broken; an anchor off by one character; an `<a href>` broken; a link
// under a directory that exists to a file that does not; the floor raised above
// the count; a broken link after a fence in a CRLF copy of a document.

import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmdirSync,
  statSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join, posix } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("../..", import.meta.url)).replace(/\/$/, "");

const DOCUMENTS = [
  "README.md",
  "docs/README.md",
  "docs/method.md",
  "docs/engine.md",
  "docs/web-guide.md",
  "docs/computer-player.md",
  "CONTRIBUTING.md",
  "SECURITY.md",
];

/** The anti-vacuity floor: links read in all. */
const FLOOR = 40;

/** A markdown reference: `](target)`, `](<target>)`, an optional quoted title. */
const MARKDOWN = /\]\(\s*(?:<([^>]*)>|([^)\s]*))(?:\s+(?:"[^"]*"|'[^']*'))?\s*\)/g;
/** An anchor element's `href`, quoted either way; a quoted attribute before it may hold a `>`. */
const ANCHOR = /<a\b(?:[^>"']|"[^"]*"|'[^']*')*?\shref\s*=\s*(?:"([^"]*)"|'([^']*)')/gi;
/** A target that is not in the tree: a URL scheme, or a network path. */
const ELSEWHERE = /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i;

function read(file: string): string {
  return readFileSync(join(root, file), "utf8");
}

/**
 * The text without its fenced code blocks, its lines rejoined with `\n` whatever ending they had (CRLF or LF), so
 * every reader after this one sees LF alone. An opener is a line whose first non-space characters are three or more
 * backticks (or tildes) with no backtick after them on the line (CommonMark: a line such as ```` ```x``` ```` is inline
 * code); it closes on a line of at least as many of the same character and nothing else. The fence lines go too.
 */
function withoutFences(text: string): string {
  const kept: string[] = [];
  let fence: string | null = null;
  for (const line of text.split(/\r?\n/)) {
    if (fence === null) {
      const open = /^ {0,3}(?:(`{3,})(?!.*`)|(~{3,}))/.exec(line);
      if (open) fence = (open[1] ?? open[2])!;
      else kept.push(line);
    } else {
      const close = /^ {0,3}(`{3,}|~{3,})[ \t]*$/.exec(line);
      if (close && close[1]![0] === fence[0] && close[1]!.length >= fence.length) fence = null;
    }
  }
  return kept.join("\n");
}

/**
 * GitHub's slug of a heading's text. A code span keeps its text verbatim, its backticks gone and no HTML stripped
 * inside it (`Option<u8>` keeps `<u8>`); outside code spans a link keeps its text, an HTML tag goes and emphasis marks
 * go. Then lower-case, remove every character that is not a letter, a digit, a combining mark, whitespace, `-` or `_`,
 * and turn each space into `-`.
 */
function slug(heading: string): string {
  const parts: string[] = [];
  let last = 0;
  for (const m of heading.matchAll(/(`+)(?!`)([\s\S]*?[^`])\1(?!`)/g)) {
    parts.push(outsideCode(heading.slice(last, m.index)), m[2]!);
    last = m.index + m[0].length;
  }
  parts.push(outsideCode(heading.slice(last)));
  return parts
    .join("")
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{Nd}\s_-]/gu, "")
    .replace(/ /g, "-");
}

/** Heading text outside code spans: a link's destination, an HTML tag and the `*` emphasis mark go (an `_` stays); brackets fall to the slug's class. */
function outsideCode(text: string): string {
  return text.replace(/\]\([^)]*\)/g, "]").replace(/<[^>]*>/g, "").replace(/\*/g, "");
}

/** Every anchor a markdown file offers: its headings' slugs and any `id`/`name` attribute outside fenced blocks. */
function markdownAnchors(text: string): Set<string> {
  const anchors = new Set<string>();
  const outside = withoutFences(text);
  for (const line of outside.split("\n")) {
    const h = /^ {0,3}#{1,6}[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$/.exec(line);
    if (!h) continue;
    // github-slugger's collision rule: a suffix that is already some heading's slug is skipped.
    const base = slug(h[1]!);
    let result = base;
    let n = 0;
    while (anchors.has(result)) {
      n += 1;
      result = `${base}-${n}`;
    }
    anchors.add(result);
  }
  for (const m of outside.matchAll(/<[a-z0-9]+\b[^>]*?\s(?:id|name)\s*=\s*(?:"([^"]*)"|'([^']*)')/gi)) {
    anchors.add(m[1] ?? m[2] ?? "");
  }
  return anchors;
}

/** Every `id`/`name` attribute value of an HTML file. */
function htmlAnchors(text: string): Set<string> {
  const anchors = new Set<string>();
  for (const m of text.matchAll(/\s(?:id|name)\s*=\s*(?:"([^"]*)"|'([^']*)')/gi)) anchors.add(m[1] ?? m[2] ?? "");
  return anchors;
}

interface Link {
  file: string;
  raw: string;
}

/** Every reference in `text` (the content of `file`) that points into the tree (or at the file itself), as written. */
function linksOfText(file: string, text: string): Link[] {
  const body = withoutFences(text);
  const out: Link[] = [];
  const take = (raw: string | undefined, seen: string) => {
    // Each pattern's target group always takes part in a match; a miss is a defect in this reader, not a skip.
    if (raw === undefined) throw new Error(`${file}: a link matched with no target: ${seen}`);
    if (raw === "" || ELSEWHERE.test(raw)) return;
    out.push({ file, raw });
  };
  for (const m of body.matchAll(MARKDOWN)) take(m[1] ?? m[2], m[0]);
  for (const m of body.matchAll(ANCHOR)) take(m[1] ?? m[2], m[0]);
  return out;
}

/** The documents that yielded no link at all: one that drops out of the read must not hide under the global floor. */
function filesWithoutLinks(counts: Map<string, number>): string[] {
  return [...counts].filter(([, n]) => n === 0).map(([file]) => file);
}

/** What the resolver asks: a directory, the set of paths git tracks (null when there is no git to ask), and a cache. */
interface Tree {
  dir: string;
  tracked: Set<string> | null;
  trackedDirs: Set<string>;
  anchors: Map<string, Set<string> | null>;
}

function openTree(dir: string, tracked: Set<string> | null): Tree {
  const trackedDirs = new Set<string>();
  for (const f of tracked ?? []) {
    for (let d = posix.dirname(f); d !== "."; d = posix.dirname(d)) trackedDirs.add(d);
  }
  return { dir, tracked, trackedDirs, anchors: new Map() };
}

/** The paths git tracks at the root, or null when the root holds no `.git` (an export): the second half of the check is then skipped. */
function gitTracked(dir: string): Set<string> | null {
  if (!existsSync(join(dir, ".git"))) return null;
  const out = execFileSync("git", ["-C", dir, "ls-files", "-z"], { encoding: "utf8", maxBuffer: 1 << 28 });
  return new Set(out.split("\0").filter((f) => f !== ""));
}

/**
 * The reason a normalized repository path is not in the tree GitHub serves, or null. Each component is looked up in an
 * exact-case directory listing (a case-insensitive file system answers `Readme.md` for `README.md`; GitHub does not),
 * none may be a symbolic link, and when git is asked the path must be tracked: a file by name, a directory when a
 * tracked file lies under it.
 */
function notServed(tree: Tree, target: string): string | null {
  let at = tree.dir;
  const parts = target === "." ? [] : target.split("/");
  for (const [i, part] of parts.entries()) {
    let names: string[];
    try {
      names = readdirSync(at);
    } catch {
      return `no such file or directory: ${target}`;
    }
    if (!names.includes(part)) {
      const other = names.find((n) => n.toLowerCase() === part.toLowerCase());
      return other === undefined ? `no such file or directory: ${target}` : `case differs from ${other} on disk: ${target}`;
    }
    at = join(at, part);
    if (lstatSync(at).isSymbolicLink()) return `a symbolic link (${parts.slice(0, i + 1).join("/")}) is not in the tree: ${target}`;
  }
  if (tree.tracked === null || target === ".") return null;
  const here = statSync(at).isDirectory() ? tree.trackedDirs.has(target) : tree.tracked.has(target);
  return here ? null : `not tracked by git: ${target}`;
}

/** The anchors of a repository path, or null when the file's kind has none this cell can read. */
function anchorsOf(tree: Tree, path: string): Set<string> | null {
  if (!tree.anchors.has(path)) {
    let found: Set<string> | null = null;
    if (path.endsWith(".md")) found = markdownAnchors(readFileSync(join(tree.dir, path), "utf8"));
    else if (path.endsWith(".html")) found = htmlAnchors(readFileSync(join(tree.dir, path), "utf8"));
    tree.anchors.set(path, found);
  }
  return tree.anchors.get(path) ?? null;
}

/** The reason a link is broken, or null when it resolves. */
function problem({ file, raw }: Link, tree: Tree): string | null {
  const hash = raw.indexOf("#");
  const pathPart = hash === -1 ? raw : raw.slice(0, hash);
  const anchor = hash === -1 ? null : raw.slice(hash + 1);
  const decode = (s: string) => {
    try {
      return decodeURIComponent(s);
    } catch {
      return s;
    }
  };
  const clean = decode(pathPart.replace(/\?.*$/, ""));
  let target: string;
  if (clean === "") target = file;
  else if (clean.startsWith("/")) target = posix.normalize(clean.slice(1));
  else target = posix.normalize(posix.join(dirname(file), clean));
  target = target.replace(/\/$/, "") || ".";
  if (target.startsWith("../") || target === "..") return `leaves the repository (${target})`;
  const missing = notServed(tree, target);
  if (missing !== null) return missing;
  if (anchor === null || anchor === "") return null;
  if (!statSync(join(tree.dir, target)).isFile()) return `an anchor into a directory: ${target}`;
  const anchors = anchorsOf(tree, target);
  if (anchors === null) return `an anchor into a file this cell cannot read headings of: ${target}`;
  return anchors.has(decode(anchor)) ? null : `${target} has no heading or id whose anchor is #${anchor}`;
}

describe("the reader-facing documents and the tree", () => {
  it("every relative link and anchor in the reader-facing documents resolves", () => {
    const tree = openTree(root, gitTracked(root));
    if (tree.tracked === null) {
      console.info("doc-links: no .git at the repository root (an export); the git-tracked half of the check is skipped");
    }
    const all: Link[] = [];
    const counts = new Map<string, number>();
    for (const file of DOCUMENTS) {
      // A document absent from the tree is a failure, never a skip.
      expect(existsSync(join(root, file)), `${file} is listed for the link cell and is absent`).toBe(true);
      const found = linksOfText(file, read(file));
      counts.set(file, found.length);
      all.push(...found);
    }

    const broken = all
      .map((l) => ({ l, why: problem(l, tree) }))
      .filter((x) => x.why !== null)
      .map((x) => `${x.l.file}: (${x.l.raw}) ${x.why}`);
    expect(broken, `broken links:\n${broken.join("\n")}`).toEqual([]);

    // Anti-vacuity: a reader that finds nothing would pass for the wrong reason, and a document whose links drop out
    // of the read (a fence taken for the rest of the file) must not hide under the global count.
    const silent = filesWithoutLinks(counts);
    expect(silent, `these documents yielded no links: ${silent.join(", ")}`).toEqual([]);
    expect(all.length, `the cell read ${all.length} links over ${DOCUMENTS.length} files`).toBeGreaterThanOrEqual(FLOOR);
  });

  it("the slug rule reproduces GitHub's anchors", () => {
    // Each pair is a heading of this tree (the θ̂ pair a fragment of one) and the anchor GitHub gives it; the source is named beside it.
    const pairs: [string, string][] = [
      // deploy/README.md §2, linked from SECURITY.md: a colon, a comma, a leading number.
      ["2. The open door: a home network, a LAN or a VPN", "2-the-open-door-a-home-network-a-lan-or-a-vpn"],
      // docs/plans/2026-09-16-web-frontend-m6.4-chat-spec.md: `Option<u8>` inside a code span keeps its `<u8>` text.
      ["2.3 `check_shape(doc, players: Option<u8>)`", "23-check_shapedoc-players-optionu8"],
      // docs/plans/2026-07-26-phase3.3-evaluator-search-spec.md: a section sign, an em dash, a colon.
      ["§5 — Arena: contestant seam, driver extensions, pairing scope", "5--arena-contestant-seam-driver-extensions-pairing-scope"],
      // docs/web-lane.md line 1: an em dash between words leaves two hyphens.
      ["The web lane — its operating record", "the-web-lane--its-operating-record"],
      // docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md §9.9: θ followed by a combining circumflex keeps its mark.
      ["at θ̂ 0.691 [0.667, 0.715]", "at-θ̂-0691-0667-0715"],
    ];
    for (const [heading, anchor] of pairs) expect(slug(heading), heading).toBe(anchor);

    // deploy/RUNBOOK.md carries "#### Rollback" more than once: the later ones take -1, -2, …; and GitHub skips a
    // suffix that is already some heading's slug, so "Notes", "Notes 1", "Notes" give notes, notes-1, notes-2.
    expect([...markdownAnchors("#### Rollback\n\ntext\n\n#### Rollback\n")]).toEqual(["rollback", "rollback-1"]);
    expect([...markdownAnchors("# Notes\n\n# Notes 1\n\n# Notes\n")]).toEqual(["notes", "notes-1", "notes-2"]);
  });

  it("a line opening with inline triple-backtick code is not a fence", () => {
    // CommonMark: a backtick fence's info string holds no backtick, so this line is inline code and the links after it count.
    const text = "```npm test``` runs the suite.\n\nSee [the guide](docs/nope.md).\n";
    expect(linksOfText("X.md", text).map((l) => l.raw)).toEqual(["docs/nope.md"]);
    // A real fence still hides what it holds, and closes only on a line of at least as many of the same character.
    const fenced = "````\n```\n[a](in-fence.md)\n```\n[b](still-in-fence.md)\n````\n[c](after.md)\n~~~\n[d](tilde.md)\n~~~\n";
    expect(linksOfText("X.md", fenced).map((l) => l.raw)).toEqual(["after.md"]);
  });

  it("a CRLF copy of a document reads every link after its fences, and a broken one RED", () => {
    // A checkout with CRLF line endings (Git for Windows' default) must read what an LF one reads: the fence closer
    // once failed on a trailing `\r`, so every link after the first fence dropped out and a broken one read green.
    const lf = `${read("README.md")}\n[a broken link after every fence](no-such-file.md)\n`;
    expect(lf, "the copy holds a fence before the broken link").toMatch(/\n```/);
    const crlf = lf.replace(/\n/g, "\r\n");
    expect(linksOfText("README.md", crlf)).toEqual(linksOfText("README.md", lf));
    // The existence half alone (no git asked): the appended link is the one broken link of the copy.
    const tree = openTree(root, null);
    expect(linksOfText("README.md", crlf).filter((l) => problem(l, tree) !== null).map((l) => l.raw)).toEqual(["no-such-file.md"]);
    expect(markdownAnchors(crlf)).toEqual(markdownAnchors(lf));
  });

  it("the per-file floor names a document that yields no links", () => {
    expect(filesWithoutLinks(new Map([["a.md", 3], ["b.md", 0], ["c.md", 1]]))).toEqual(["b.md"]);
    expect(filesWithoutLinks(new Map([["a.md", 3]]))).toEqual([]);
  });

  describe("the resolver asks the tree GitHub serves", () => {
    // A throwaway tree under the system temp directory: `README.md`, `real/f.md`, `linked` (a symbolic link to `real`)
    // and `ignored/x.md` (a real file that git does not track). It is removed file by file, never recursively.
    const dir = mkdtempSync(join(tmpdir(), "doc-links-"));
    const made = [
      "README.md",
      "real/f.md",
      "ignored/x.md",
    ];
    mkdirSync(join(dir, "real"));
    mkdirSync(join(dir, "ignored"));
    for (const f of made) writeFileSync(join(dir, f), "# Title\n");
    symlinkSync(join(dir, "real"), join(dir, "linked"));
    afterAll(() => {
      unlinkSync(join(dir, "linked"));
      for (const f of made) unlinkSync(join(dir, f));
      rmdirSync(join(dir, "real"));
      rmdirSync(join(dir, "ignored"));
      rmdirSync(dir);
    });
    const tracked = new Set(["README.md", "real/f.md"]);
    const link = (raw: string): Link => ({ file: "README.md", raw });

    it("a case-only change of a path is a broken link, on a case-insensitive file system too", () => {
      expect(problem(link("README.md"), openTree(dir, tracked))).toBeNull();
      expect(problem(link("Readme.md"), openTree(dir, tracked)), "Readme.md for README.md").toMatch(/case/);
      expect(problem(link("Real/f.md"), openTree(dir, tracked)), "Real/ for real/").toMatch(/case/);
    });

    it("a path through a symbolic link is a broken link", () => {
      expect(problem(link("real/f.md"), openTree(dir, tracked))).toBeNull();
      expect(problem(link("linked/f.md"), openTree(dir, tracked))).toMatch(/symbolic link/);
      expect(problem(link("linked/"), openTree(dir, tracked))).toMatch(/symbolic link/);
    });

    it("a path git does not track is a broken link; a directory is tracked when a file under it is", () => {
      expect(problem(link("ignored/x.md"), openTree(dir, tracked))).toMatch(/not tracked/);
      expect(problem(link("ignored/"), openTree(dir, tracked))).toMatch(/not tracked/);
      expect(problem(link("real/"), openTree(dir, tracked))).toBeNull();
      // With no git to ask (an export), the tracked half is skipped and the others still hold.
      expect(problem(link("ignored/x.md"), openTree(dir, null))).toBeNull();
      expect(problem(link("Readme.md"), openTree(dir, null))).toMatch(/case/);
    });
  });

  it("the slug rule reads a link, emphasis, a fenced heading and an underscore the way GitHub does", () => {
    // Headings written for this cell (not the tree's): the pairs above are the tree's own.
    expect(slug("3. The testing strategy: when each tier runs, what it proves, what skipping it costs")).toBe(
      "3-the-testing-strategy-when-each-tier-runs-what-it-proves-what-skipping-it-costs",
    );
    expect(slug("`Contestant` and the [web](x.md) client — a **bold** `v1` note (§4)")).toBe("contestant-and-the-web-client--a-bold-v1-note-4");
    expect(slug("Snake_case and under_scores")).toBe("snake_case-and-under_scores");
    expect(slug("Half ½ and square ²")).toBe("half--and-square-");

    // A fenced heading is no heading, and an id inside a fence is no anchor.
    const anchors = markdownAnchors('# Notes\n\n## Notes\n\n```\n## Notes\n<span id="ghost"></span>\n```\n\n### Notes\n');
    expect([...anchors].sort()).toEqual(["notes", "notes-1", "notes-2"]);
  });
});
