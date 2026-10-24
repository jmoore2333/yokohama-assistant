// **The notice holds the pictures to the tree** (security S4a, finding C4): the
// third-party artwork is named in `ASSETS.md` by directory, and a picture a
// README or a guide shows lives in a directory. If a document starts showing
// pictures from a directory the notice does not name, the carve-out silently
// stops covering them — this cell is the catcher.
//
// It reads every `](<path>.png)` and `](<path>.webp)` reference from the four
// documents that carry pictures (a multi-line alt text is no obstacle: the
// reference ends at `](`), resolves each path against the referencing file's
// own directory to a repository-relative path, takes the directory, and asks
// that `ASSETS.md` spells every one of them — as an excluded path, or in the
// sentence that says `docs/images/`'s pictures are text renders carrying no art.
//
// Mutants these cells are written to kill (the battery cites them): the
// `worker/docs/img/` spelling removed from `ASSETS.md`; a picture reference
// added under a directory `ASSETS.md` does not name; a resolution that reads a
// guide's `../web/public/help/…` or the Worker README's `docs/img/…` as written
// instead of from the referencing file's directory.

import { readFileSync } from "node:fs";
import { dirname, join, posix } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("../..", import.meta.url)).replace(/\/$/, "");

const DOCUMENTS = ["README.md", "worker/README.md", "docs/web-guide.md", "docs/computer-player.md"];

/** Every repository-relative directory a picture of `file` lives in. */
function pictureDirectories(file: string): string[] {
  const text = readFileSync(join(root, file), "utf8");
  const dirs: string[] = [];
  for (const m of text.matchAll(/\]\(([^)\s]+\.(?:png|webp))\)/g)) {
    dirs.push(posix.dirname(posix.normalize(posix.join(dirname(file), m[1]))));
  }
  return dirs;
}

describe("the notice and the pictures", () => {
  it("ASSETS.md names every directory a README or guide picture lives in", () => {
    const assets = readFileSync(join(root, "ASSETS.md"), "utf8");
    const dirs = new Set<string>();
    for (const file of DOCUMENTS) for (const d of pictureDirectories(file)) dirs.add(d);

    // Anti-vacuity: a reader that finds nothing would pass for the wrong reason.
    expect([...dirs].sort().length).toBeGreaterThanOrEqual(3);

    const unnamed = [...dirs].filter((d) => !assets.includes(`${d}/`)).sort();
    expect(unnamed, `ASSETS.md does not spell these picture directories: ${unnamed.join(", ")}`).toEqual([]);
  });
});
