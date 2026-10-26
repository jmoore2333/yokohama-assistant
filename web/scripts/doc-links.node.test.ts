// **The reader-facing documents hold their links to the tree** (security S4b,
// Task 2; the design §7.2 and §10.15): a repository that is about to be public
// is read by following its links, so a link to a file that moved, or to a
// heading that was renamed, is a broken first impression this cell catches.
//
// It reads the seven documents a newcomer starts from: the root `README.md`,
// the docs index, the method page, the two guides, `CONTRIBUTING.md` and
// `SECURITY.md`. From each it collects every markdown reference
// `](<target>)` (a `<…>` wrapper and a quoted title allowed; a multi-line link
// text is no obstacle, since the reference starts at `](`) and every
// `<a … href="<target>">` value, quoted either way. Fenced code blocks are not
// read: a link shown as an example is not a link. A target with a URL scheme
// (`https:`, `mailto:` …) or one that starts with `//` is not in the tree and is
// skipped. Every other target is resolved — a leading `/` from the repository
// root, anything else from the referencing file's own directory (a target that
// is only `#anchor` points into the referencing file itself) — and must exist
// as a file or a directory. A `#anchor` must name a heading of the target
// markdown file by GitHub's slug (below), or an `id`/`name` attribute of a
// target HTML file.
//
// The slug is GitHub's: the heading's text with its markdown stripped (a link
// keeps its text; backticks and emphasis marks go), lower-cased, every
// character that is not a letter, a digit, a space, `-` or `_` removed, each
// space turned into `-`; a slug that repeats within one file takes `-1`, `-2`,
// … on its later occurrences. The cell `the slug rule …` proves the rule against
// headings whose anchors the tree already links, so a defect in the rule reads
// there and not as a "broken" link.
//
// The anti-vacuity floor: at least 40 links are read in all, so an empty read
// can never pass, and the count is printed in the assertion message.
//
// Mutants these cells are written to kill (the plan's Review Focus 3): a link's
// path broken; an anchor off by one character; an `<a href>` broken; a link
// under a directory that exists to a file that does not; the floor raised above
// the count.

import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join, posix } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("../..", import.meta.url)).replace(/\/$/, "");

const DOCUMENTS = [
  "README.md",
  "docs/README.md",
  "docs/method.md",
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

/** The text without its fenced code blocks (``` or ~~~ fences; the fence lines themselves go too). */
function withoutFences(text: string): string {
  const kept: string[] = [];
  let fence: string | null = null;
  for (const line of text.split("\n")) {
    const m = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fence === null) {
      if (m) fence = m[1]![0]!.repeat(m[1]!.length);
      else kept.push(line);
    } else if (m && m[1]!.startsWith(fence) && /^ {0,3}[`~]+\s*$/.test(line)) {
      fence = null;
    }
  }
  return kept.join("\n");
}

/** GitHub's slug of a heading's text. */
function slug(heading: string): string {
  const stripped = heading
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1") // a link or image keeps its text
    .replace(/<[^>]*>/g, "") // inline HTML
    .replace(/[`*]/g, "") // code spans and emphasis marks
    .trim()
    .toLowerCase();
  return stripped.replace(/[^\p{L}\p{N} _-]/gu, "").replace(/ /g, "-");
}

/** Every anchor a markdown file offers: its headings' slugs (repeats suffixed) and any `id`/`name` attribute. */
function markdownAnchors(text: string): Set<string> {
  const anchors = new Set<string>();
  const seen = new Map<string, number>();
  for (const line of withoutFences(text).split("\n")) {
    const h = /^ {0,3}#{1,6}[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$/.exec(line);
    if (!h) continue;
    const base = slug(h[1]!);
    const n = seen.get(base) ?? 0;
    seen.set(base, n + 1);
    anchors.add(n === 0 ? base : `${base}-${n}`);
  }
  for (const m of text.matchAll(/<[a-z0-9]+\b[^>]*?\s(?:id|name)\s*=\s*(?:"([^"]*)"|'([^']*)')/gi)) {
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

/** Every reference of `file` that points into the tree (or at the file itself), as written. */
function links(file: string): Link[] {
  const text = withoutFences(read(file));
  const out: Link[] = [];
  const take = (raw: string | undefined, seen: string) => {
    // Each pattern's target group always takes part in a match; a miss is a defect in this reader, not a skip.
    if (raw === undefined) throw new Error(`${file}: a link matched with no target: ${seen}`);
    if (raw === "" || ELSEWHERE.test(raw)) return;
    out.push({ file, raw });
  };
  for (const m of text.matchAll(MARKDOWN)) take(m[1] ?? m[2], m[0]);
  for (const m of text.matchAll(ANCHOR)) take(m[1] ?? m[2], m[0]);
  return out;
}

const anchorCache = new Map<string, Set<string> | null>();

/** The anchors of a repository path, or null when the file's kind has none this cell can read. */
function anchorsOf(path: string): Set<string> | null {
  if (!anchorCache.has(path)) {
    let found: Set<string> | null = null;
    if (path.endsWith(".md")) found = markdownAnchors(read(path));
    else if (path.endsWith(".html")) found = htmlAnchors(read(path));
    anchorCache.set(path, found);
  }
  return anchorCache.get(path) ?? null;
}

/** The reason a link is broken, or null when it resolves. */
function problem({ file, raw }: Link): string | null {
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
  const onDisk = join(root, target);
  if (target.startsWith("../") || target === "..") return `leaves the repository (${target})`;
  if (!existsSync(onDisk)) return `no such file or directory: ${target}`;
  if (anchor === null || anchor === "") return null;
  if (!statSync(onDisk).isFile()) return `an anchor into a directory: ${target}`;
  const anchors = anchorsOf(target);
  if (anchors === null) return `an anchor into a file this cell cannot read headings of: ${target}`;
  return anchors.has(decode(anchor)) ? null : `${target} has no heading or id whose anchor is #${anchor}`;
}

describe("the reader-facing documents and the tree", () => {
  it("every relative link and anchor in the reader-facing documents resolves", () => {
    const all: Link[] = [];
    for (const file of DOCUMENTS) {
      // A document absent from the tree is a failure, never a skip.
      expect(existsSync(join(root, file)), `${file} is listed for the link cell and is absent`).toBe(true);
      all.push(...links(file));
    }

    const broken = all
      .map((l) => ({ l, why: problem(l) }))
      .filter((x) => x.why !== null)
      .map((x) => `${x.l.file}: (${x.l.raw}) ${x.why}`);
    expect(broken, `broken links:\n${broken.join("\n")}`).toEqual([]);

    // Anti-vacuity: a reader that finds nothing would pass for the wrong reason.
    expect(all.length, `the cell read ${all.length} links over ${DOCUMENTS.length} files`).toBeGreaterThanOrEqual(FLOOR);
  });

  it("the slug rule gives the anchors the tree already links", () => {
    // Headings whose anchors a document links today: punctuation and a leading number removed, a link's text kept.
    expect(slug("2. The open door: a home network, a LAN or a VPN")).toBe("2-the-open-door-a-home-network-a-lan-or-a-vpn");
    expect(slug("3. The testing strategy: when each tier runs, what it proves, what skipping it costs")).toBe(
      "3-the-testing-strategy-when-each-tier-runs-what-it-proves-what-skipping-it-costs",
    );
    expect(slug("`Contestant` and the [web](x.md) client — a `v1` note (§4)")).toBe("contestant-and-the-web-client--a-v1-note-4");
    expect(slug("Snake_case and under_scores")).toBe("snake_case-and-under_scores");

    // A repeated slug within one file takes -1, -2, … on its later occurrences.
    const anchors = markdownAnchors("# Notes\n\n## Notes\n\n```\n## Notes\n```\n\n### Notes\n");
    expect([...anchors].sort()).toEqual(["notes", "notes-1", "notes-2"]);
  });
});
