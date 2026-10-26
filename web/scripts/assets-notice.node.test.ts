// **The notice holds the pictures to the tree** (security S4a, finding C4): the
// third-party artwork is named in `ASSETS.md` by directory, and a picture a
// README or a guide shows lives in a directory. If a document starts showing
// pictures from a directory the notice does not name, the carve-out silently
// stops covering them — this cell is the catcher.
//
// It reads the root README, the Worker's README and the two guides
// (`docs/computer-player.md` carries no picture today; it is read so that its
// first one is held too). From each it collects every markdown reference
// `](<path>)` whose path ends in `.png`, `.webp`, `.jpg`, `.jpeg`, `.gif` or
// `.svg` (any case; an optional `?…` or `#…` suffix and an optional quoted
// title allowed; a multi-line alt text is no obstacle, since the reference
// starts at `](`), and every `<img … src="<path>">` value, quoted either way.
// A path with a URL scheme, or one that starts with `//`, is not in the tree
// and is skipped. A path that starts with `/` resolves from the repository
// root, every other path against the referencing file's own directory; the
// cell takes the directory and asks that `ASSETS.md` spells every one of them
// as a whole backtick token, `` `<dir>/` `` — anywhere in the notice; the cell
// does not check which sentence carries it (the excluded-path list, or the
// sentence that says `docs/images/`'s pictures are text renders carrying no
// art). A directory that occurs only inside a longer token (`docs/img/` inside
// `` `worker/docs/img/` ``) is not named, and a picture in a subdirectory of a
// named directory reads red until the notice spells the subdirectory.
//
// Mutants these cells are written to kill (the plan's P-24): the
// `worker/docs/img/` spelling removed from `ASSETS.md`; a picture reference
// added under a directory `ASSETS.md` does not name; an `<img src>` picture
// moved to such a directory; a picture at the root `docs/img/`, at `docs/` or
// at `web/`, each a substring of a named token and none a token itself; a
// resolution that reads every path as written instead of from the referencing
// file's directory, which turns the Worker README's `docs/img/…` into the
// unnamed root `docs/img/`.

import { readFileSync } from "node:fs";
import { dirname, join, posix } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("../..", import.meta.url)).replace(/\/$/, "");

const DOCUMENTS = ["README.md", "worker/README.md", "docs/web-guide.md", "docs/computer-player.md"];

/** A markdown reference to a picture: `](path)`, `](<path>)`, a `?…`/`#…` suffix, a quoted title. */
const MARKDOWN =
  /\]\(\s*<?([^)\s>]+\.(?:png|webp|jpe?g|gif|svg))(?:[?#][^)\s>]*)?>?(?:\s+(?:"[^"]*"|'[^']*'))?\s*\)/gi;
/** An `<img>` element's `src`, quoted either way; a quoted attribute before it may hold a `>`. */
const IMG = /<img\b(?:[^>"']|"[^"]*"|'[^']*')*?\ssrc\s*=\s*(?:"([^"]*)"|'([^']*)')/gi;
/** A path that is not in the tree: a URL scheme, or a network path. */
const ELSEWHERE = /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i;

interface Pictures {
  dirs: string[];
  imgs: number;
}

/** Every repository-relative directory a picture of `file` lives in, and how many `<img>` pictures were read. */
function pictures(file: string): Pictures {
  const text = readFileSync(join(root, file), "utf8");
  const dirs: string[] = [];
  let imgs = 0;
  const take = (ref: string | undefined, seen: string) => {
    // Each pattern's path group always takes part in a match; a miss is a defect in this reader, not a skip.
    if (ref === undefined) throw new Error(`${file}: a picture reference matched with no path: ${seen}`);
    if (ELSEWHERE.test(ref)) return;
    const path = ref.startsWith("/") ? posix.normalize(ref.slice(1)) : posix.normalize(posix.join(dirname(file), ref));
    dirs.push(posix.dirname(path));
  };
  for (const m of text.matchAll(MARKDOWN)) take(m[1], m[0]);
  for (const m of text.matchAll(IMG)) {
    imgs += 1;
    take(m[1] ?? m[2], m[0]);
  }
  return { dirs, imgs };
}

describe("the notice and the pictures", () => {
  it("ASSETS.md names every directory a README or guide picture lives in", () => {
    const assets = readFileSync(join(root, "ASSETS.md"), "utf8");
    const dirs = new Set<string>();
    let imgs = 0;
    for (const file of DOCUMENTS) {
      const found = pictures(file);
      for (const d of found.dirs) dirs.add(d);
      imgs += found.imgs;
    }

    // Anti-vacuity: a reader that finds nothing would pass for the wrong reason, and so would an `<img>` reader
    // that never matches (the Worker README draws its phone pictures as `<img>`). Two directories are read today:
    // `worker/docs/img/`, which the root README's one picture shares with the Worker README, and the web guide's
    // `web/public/help/`.
    expect([...dirs].sort().length).toBeGreaterThanOrEqual(2);
    expect(imgs, "the <img> reader read no picture").toBeGreaterThanOrEqual(1);

    const unnamed = [...dirs].filter((d) => !assets.includes("`" + d + "/`")).sort();
    expect(unnamed, `ASSETS.md does not spell these picture directories: ${unnamed.join(", ")}`).toEqual([]);
  });
});
