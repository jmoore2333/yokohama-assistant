// **The web presence** (cycle F, R-F12; spec §2.7): `robots.txt`, `llms.txt`,
// the sitemap and the page's metadata — static, built from the checked build
// values, free on both hosts, and fetched, precached and scripted by nothing.
//
// Three layers of cell, because each catches what the others cannot:
//
// 1. **The pure builders over values** — the exact texts, with and without
//    each value. A text read off the builder is the text the build writes; the
//    `Sitemap:` line, the repository section, the canonical link and `og:url`
//    exist ONLY with the value they derive from.
// 2. **`presence.mjs` as a PROCESS over a scratch `dist`** — the way
//    `build:client` runs it: which files land, which do not, what a bad value
//    does (a refusal naming the variable, written nowhere), and that a re-run
//    without the site's address leaves no stale sitemap behind.
// 3. **A REAL client build** (`vite build`, then the generator, then this
//    script — `build:client`'s own sequence, into a temporary directory),
//    twice: every build value set, and none. The shell is read for each tag
//    exactly once and for its JSON-LD parsing; both trees for the files; the
//    release's list for what it must NOT name. This layer needs the generated
//    wasm glue like `src/precache.node.test.ts`'s, and skips out loud under
//    the same opt-out.
//
// Mutants these cells are written to kill (the battery cites them): a
// `robots.txt` that allows `/api/` or loses one `Disallow`; a `Sitemap:` line,
// a sitemap, a canonical link or an `og:url` written with no site address; a
// repository link in `llms.txt` or `codeRepository` in the JSON-LD with no
// project link; a blurb that closes the JSON-LD's script or escapes an
// attribute; a `String.replace` pattern (`$&`) read out of a blurb; a tag
// written twice; an `@` in `llms.txt`; `presence.mjs` dropped from
// `build:client` or ordered before the generator; a presence file in the
// precache list.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, describe, expect, it } from "vitest";
import { readPrecacheList } from "../../worker/scripts/precache-list.mjs";
import {
  DEFAULT_DESCRIPTION,
  injectMetadata,
  llmsTxt,
  metadataTags,
  METADATA_PLACEHOLDER,
  OG_IMAGE_PATH,
  robotsTxt,
  sitemapXml,
  writePresence,
} from "./presence.mjs";

const web = fileURLToPath(new URL("..", import.meta.url)).replace(/\/$/, "");
const presenceScript = join(web, "scripts", "presence.mjs");

const SITE = "https://yokohama.example.org";
const REPO = "https://github.com/example/yokohama";
const NONE = { siteUrl: null, blurb: null, notice: null, projectUrl: null };
const ALL = { siteUrl: SITE, blurb: "A few words, set by the deployer.", notice: "Not affiliated with anyone.", projectUrl: REPO };

const scratch: string[] = [];
afterAll(() => {
  for (const dir of scratch.splice(0)) rmSync(dir, { recursive: true, force: true });
});
function scratchDir(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  scratch.push(dir);
  return dir;
}

/** The five build-value variables a process reads, cleared so a developer's own
 *  shell never leaks into a cell that means to set only some of them. */
const VARS = [
  "YOKOHAMA_PROJECT_URL",
  "YOKOHAMA_SITE_BLURB",
  "YOKOHAMA_SITE_NOTICE",
  "YOKOHAMA_RELEASE_FEED",
  "YOKOHAMA_TURNSTILE_SITEKEY",
  "YOKOHAMA_SITE_URL",
];
function envWith(set: Record<string, string>): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { ...process.env };
  for (const name of VARS) delete env[name];
  return Object.assign(env, set);
}

/** How many times `re` matches in `text`. */
function count(text: string, re: RegExp): number {
  return [...text.matchAll(new RegExp(re.source, re.flags.includes("g") ? re.flags : `${re.flags}g`))].length;
}

/** The five entities `metadataTags` writes, read back. `&amp;` last. */
function unescapeHtml(text: string): string {
  return text.replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
}

/** The `content` (or `href`) of the one tag whose opening is `open`, unescaped. */
function attrOf(html: string, open: string, attr: "content" | "href"): string | null {
  const m = new RegExp(`<${open} ${attr}="([^"]*)" />`).exec(html);
  return m === null ? null : unescapeHtml(m[1]!);
}

/** The JSON-LD blocks of `html`, parsed — a block that does not parse throws. */
function jsonLd(html: string): Array<Record<string, unknown>> {
  return [...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map(
    (m) => JSON.parse(m[1]!) as Record<string, unknown>,
  );
}

const ROBOTS = "User-agent: *\nAllow: /\nDisallow: /api/\nDisallow: /enter\nDisallow: /cdn-cgi/\n";

const SUMMARY =
  "> Yokohama, the board game by Hisashi Hayashi, in your browser: a bot game that runs entirely in the browser, and multiplayer behind a door. An independent open-source project, not affiliated with the game's designer or publisher.";
const VISITOR = [
  "## What a visitor can do here",
  "",
  "- Play the bot with no account: everything runs in the browser and works offline once installed.",
  "- Play with friends behind the door at /enter, which is not for automated clients.",
  "- The API under /api/ serves the game's own client and is not a public API.",
].join("\n");

describe("robotsTxt", () => {
  it("allows everything but the metered paths, and names no sitemap without an address", () => {
    expect(robotsTxt({ siteUrl: null })).toBe(ROBOTS);
  });

  // The catcher for a crawler reaching a metered path: the three `Disallow`
  // lines are the whole point of the file, so they are read as a SET and
  // `/api/` is asserted never allowed, not only present among the disallows.
  it("disallows /api/, /enter and /cdn-cgi/ and allows nothing under /api/", () => {
    for (const siteUrl of [null, SITE]) {
      const lines = robotsTxt({ siteUrl }).split("\n");
      expect(lines.filter((l) => l.startsWith("Disallow:")).sort()).toEqual([
        "Disallow: /api/",
        "Disallow: /cdn-cgi/",
        "Disallow: /enter",
      ]);
      expect(lines.filter((l) => l.startsWith("Allow:"))).toEqual(["Allow: /"]);
      expect(lines.filter((l) => /^User-agent:/i.test(l))).toEqual(["User-agent: *"]);
    }
  });

  it("names the sitemap only with the site's address", () => {
    expect(robotsTxt({ siteUrl: SITE })).toBe(`${ROBOTS}\nSitemap: ${SITE}/sitemap.xml\n`);
    expect(robotsTxt({ siteUrl: null })).not.toMatch(/sitemap/i);
  });
});

describe("llmsTxt", () => {
  it("is the title, the summary and what a visitor can do — and nothing else — with no value set", () => {
    expect(llmsTxt({ blurb: null, notice: null, projectUrl: null })).toBe(`# Yokohama\n\n${SUMMARY}\n\n${VISITOR}\n`);
  });

  it("is the whole text with every value set", () => {
    expect(llmsTxt(ALL)).toBe(
      [
        "# Yokohama",
        "",
        SUMMARY,
        "",
        "- The blurb: A few words, set by the deployer.",
        "- The notice: Not affiliated with anyone.",
        "",
        "## Code",
        "",
        `- [The repository](${REPO}): Apache-2.0; the game's own material carries no grant (ASSETS.md in the repository).`,
        "",
        VISITOR,
        "",
      ].join("\n"),
    );
  });

  // The catcher for the repository named where no link was given: the section,
  // its link and the word "repository" exist only with a project link.
  it("carries the repository section only when the project link is set", () => {
    const without = llmsTxt({ ...ALL, projectUrl: null });
    expect(without).not.toMatch(/## Code/);
    expect(without).not.toMatch(/repository/i);
    expect(without).not.toContain("github.com");
    const withLink = llmsTxt({ ...ALL, projectUrl: REPO });
    expect(count(withLink, /## Code/)).toBe(1);
    expect(withLink).toContain(`[The repository](${REPO})`);
  });

  it("names the blurb and the notice only when each is set", () => {
    const blurbOnly = llmsTxt({ blurb: "Just a blurb.", notice: null, projectUrl: null });
    expect(blurbOnly).toContain("- The blurb: Just a blurb.");
    expect(blurbOnly).not.toContain("The notice");
    const noticeOnly = llmsTxt({ blurb: null, notice: "Just a notice.", projectUrl: null });
    expect(noticeOnly).toContain("- The notice: Just a notice.");
    expect(noticeOnly).not.toContain("The blurb");
  });

  // "No document or file names an address": the file's own words carry no `@`,
  // with every value set and with none.
  it("holds no @ of its own", () => {
    expect(llmsTxt(NONE)).not.toContain("@");
    expect(llmsTxt(ALL)).not.toContain("@");
  });

  it("names the licence's carve-out and the door, and calls the API not public", () => {
    const text = llmsTxt(ALL);
    expect(text).toContain("the game's own material carries no grant (ASSETS.md in the repository)");
    expect(text).toContain("/enter, which is not for automated clients");
    expect(text).toContain("is not a public API");
  });
});

describe("sitemapXml", () => {
  it("is one url: the site's root", () => {
    const xml = sitemapXml({ siteUrl: SITE });
    expect(xml).toBe(
      `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>${SITE}/</loc></url>\n</urlset>\n`,
    );
    expect(count(xml, /<url>/)).toBe(1);
    expect(count(xml, /<loc>/)).toBe(1);
  });

  it("escapes what XML reserves", () => {
    expect(sitemapXml({ siteUrl: "https://a.example.org:8443" })).toContain("<loc>https://a.example.org:8443/</loc>");
    expect(sitemapXml({ siteUrl: "https://a&b.example.org" })).toContain("<loc>https://a&amp;b.example.org/</loc>");
  });
});

describe("the page's metadata", () => {
  it("writes every tag exactly once, with no value set", () => {
    const html = metadataTags(NONE);
    expect(count(html, /<meta name="description" /)).toBe(1);
    expect(count(html, /<meta property="og:title" /)).toBe(1);
    expect(count(html, /<meta property="og:description" /)).toBe(1);
    expect(count(html, /<meta property="og:type" /)).toBe(1);
    expect(count(html, /<meta property="og:image" /)).toBe(1);
    expect(count(html, /<meta name="twitter:card" /)).toBe(1);
    expect(count(html, /<script type="application\/ld\+json">/)).toBe(1);
    expect(attrOf(html, 'meta property="og:title"', "content")).toBe("Yokohama");
    expect(attrOf(html, 'meta property="og:type"', "content")).toBe("website");
    expect(attrOf(html, 'meta name="twitter:card"', "content")).toBe("summary");
  });

  // The catcher for an address the build was never given: the canonical link
  // and `og:url` exist ONLY with the site URL, and the image stays
  // root-relative without one (a relative image URL in a card is a request to
  // the page's own host, which is true wherever it is served).
  it("writes the canonical link and og:url only with the site's address, and the image relative without it", () => {
    const without = metadataTags(NONE);
    expect(without).not.toContain("canonical");
    expect(without).not.toContain("og:url");
    // The only addresses in the tags are the schema's and the licence's: none
    // is the deployment's, because the build was given none.
    expect([...without.matchAll(/https:\/\/[^"\\]+/g)].map((m) => m[0]).sort()).toEqual([
      "https://schema.org",
      "https://www.apache.org/licenses/LICENSE-2.0",
    ]);
    expect(attrOf(without, 'meta property="og:image"', "content")).toBe(OG_IMAGE_PATH);
    expect(jsonLd(without)[0]).not.toHaveProperty("url");

    const withUrl = metadataTags({ ...NONE, siteUrl: SITE });
    expect(count(withUrl, /<link rel="canonical" /)).toBe(1);
    expect(count(withUrl, /<meta property="og:url" /)).toBe(1);
    expect(attrOf(withUrl, 'link rel="canonical"', "href")).toBe(`${SITE}/`);
    expect(attrOf(withUrl, 'meta property="og:url"', "content")).toBe(`${SITE}/`);
    expect(attrOf(withUrl, 'meta property="og:image"', "content")).toBe(`${SITE}${OG_IMAGE_PATH}`);
    expect(jsonLd(withUrl)[0]).toHaveProperty("url", `${SITE}/`);
  });

  // The image is the app's own largest raster, and the cell reads the file so
  // a rename in `public/icons/` fails here and not in a card that shows nothing.
  it("names an image that exists, the app's own 512 px icon", () => {
    expect(OG_IMAGE_PATH).toBe("/icons/icon-512.png");
    expect(existsSync(join(web, "public", OG_IMAGE_PATH))).toBe(true);
  });

  it("describes the page from the blurb, else the default sentence", () => {
    expect(DEFAULT_DESCRIPTION).toBe("Yokohama, the board game, in your browser. Play the bot here, or play with friends.");
    const dflt = metadataTags(NONE);
    expect(attrOf(dflt, 'meta name="description"', "content")).toBe(DEFAULT_DESCRIPTION);
    expect(attrOf(dflt, 'meta property="og:description"', "content")).toBe(DEFAULT_DESCRIPTION);
    expect(jsonLd(dflt)[0]).toHaveProperty("description", DEFAULT_DESCRIPTION);

    const set = metadataTags({ ...NONE, blurb: "The deployer's own words." });
    expect(attrOf(set, 'meta name="description"', "content")).toBe("The deployer's own words.");
    expect(attrOf(set, 'meta property="og:description"', "content")).toBe("The deployer's own words.");
    expect(jsonLd(set)[0]).toHaveProperty("description", "The deployer's own words.");
  });

  it("writes the JSON-LD for a free WebApplication in the games category, under the Apache-2.0 licence", () => {
    const [ld] = jsonLd(metadataTags(NONE));
    expect(ld).toEqual({
      "@context": "https://schema.org",
      "@type": "WebApplication",
      name: "Yokohama",
      applicationCategory: "GameApplication",
      operatingSystem: "Any",
      isAccessibleForFree: true,
      license: "https://www.apache.org/licenses/LICENSE-2.0",
      description: DEFAULT_DESCRIPTION,
    });
  });

  // The catcher for the repository named in structured data where no link was
  // set: `codeRepository` is the project link or it is absent.
  it("names the repository in the JSON-LD only when the project link is set", () => {
    expect(jsonLd(metadataTags(NONE))[0]).not.toHaveProperty("codeRepository");
    expect(metadataTags(NONE)).not.toContain("codeRepository");
    expect(jsonLd(metadataTags({ ...NONE, projectUrl: REPO }))[0]).toHaveProperty("codeRepository", REPO);
  });

  // "No claim of affiliation anywhere": the structured data names no author,
  // publisher, creator, brand or organization, and no tag says "official".
  it("claims no affiliation", () => {
    const html = metadataTags(ALL);
    const [ld] = jsonLd(html);
    for (const key of ["author", "publisher", "creator", "brand", "provider", "copyrightHolder", "sameAs"]) {
      expect(ld, key).not.toHaveProperty(key);
    }
    expect(html).not.toMatch(/official/i);
  });

  // The blurb is the deployer's text, and the checker lets `<`, `&` and both
  // quotes through. It must survive an attribute and a script block intact,
  // and neither may let it out.
  it("carries a hostile blurb through an attribute and the JSON-LD intact", () => {
    const blurb = `"><script>alert(1)</script> & 'quotes' </script> $& $' $1`;
    const html = metadataTags({ ...NONE, blurb });
    expect(attrOf(html, 'meta name="description"', "content")).toBe(blurb);
    expect(attrOf(html, 'meta property="og:description"', "content")).toBe(blurb);
    // Exactly one script element, and its text parses back to the blurb.
    expect(count(html, /<script/)).toBe(1);
    expect(count(html, /<\/script>/)).toBe(1);
    expect(jsonLd(html)).toHaveLength(1);
    expect(jsonLd(html)[0]).toHaveProperty("description", blurb);
    // Nothing the blurb held is live markup in the output.
    expect(html).not.toContain("<script>alert");
  });
});

describe("injecting the metadata into the shell", () => {
  const shell = `<head>\n    <title>Y</title>\n    ${METADATA_PLACEHOLDER}\n  </head>`;

  it("replaces the placeholder with the tags, once", () => {
    const out = injectMetadata(shell, ALL);
    expect(out).not.toContain(METADATA_PLACEHOLDER);
    expect(out).toContain(metadataTags(ALL).split("\n")[0]);
    expect(count(out, /<meta name="description" /)).toBe(1);
  });

  // `String.prototype.replace` reads `$&`, `$'` and `$1` out of a replacement
  // STRING; a blurb is arbitrary text. The cell is the catcher for a build
  // that corrupted the shell from a footnote-shaped blurb.
  it("is not a String.replace pattern: a blurb with $& and $' lands verbatim", () => {
    const blurb = "Costs $& more, $' less.";
    const out = injectMetadata(shell, { ...NONE, blurb });
    expect(attrOf(out, 'meta name="description"', "content")).toBe(blurb);
    expect(out.startsWith("<head>\n    <title>Y</title>\n    <meta")).toBe(true);
  });

  // A shell with no placeholder would silently carry no metadata and still
  // build; a build that stops is the honest answer.
  it("refuses a shell with no placeholder, or with two", () => {
    expect(() => injectMetadata("<head></head>", ALL)).toThrow(/placeholder/);
    expect(() => injectMetadata(`${shell}${METADATA_PLACEHOLDER}`, ALL)).toThrow(/placeholder/);
  });

  it("finds its placeholder once in the real shell, after the manifest and the theme colour", () => {
    const source = readFileSync(join(web, "index.html"), "utf8");
    expect(count(source, new RegExp(METADATA_PLACEHOLDER.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")))).toBe(1);
    expect(source.indexOf(METADATA_PLACEHOLDER)).toBeGreaterThan(source.indexOf('<meta name="theme-color"'));
    expect(source.indexOf(METADATA_PLACEHOLDER)).toBeGreaterThan(source.indexOf('<link rel="manifest"'));
    // None of the tags is written by hand beside the transform's.
    for (const tag of ['name="description"', 'rel="canonical"', 'property="og:', 'name="twitter:', "application/ld+json"]) {
      expect(source, tag).not.toContain(tag);
    }
  });
});

describe("writePresence, and presence.mjs as a process", () => {
  it("writes the two always-on files, and the sitemap only with the site's address", () => {
    const none = scratchDir("yokohama-presence-none-");
    expect(writePresence(none, NONE)).toEqual(["robots.txt", "llms.txt"]);
    expect(readdirSync(none).sort()).toEqual(["llms.txt", "robots.txt"]);
    expect(readFileSync(join(none, "robots.txt"), "utf8")).toBe(robotsTxt(NONE));
    expect(readFileSync(join(none, "llms.txt"), "utf8")).toBe(llmsTxt(NONE));

    const all = scratchDir("yokohama-presence-all-");
    expect(writePresence(all, ALL)).toEqual(["robots.txt", "llms.txt", "sitemap.xml"]);
    expect(readdirSync(all).sort()).toEqual(["llms.txt", "robots.txt", "sitemap.xml"]);
    expect(readFileSync(join(all, "sitemap.xml"), "utf8")).toBe(sitemapXml(ALL));
    expect(readFileSync(join(all, "robots.txt"), "utf8")).toContain(`Sitemap: ${SITE}/sitemap.xml`);
  });

  // A re-run of the script over a tree that holds a sitemap from an earlier
  // run WITH the address must not leave that address published.
  it("removes a stale sitemap when the address is no longer set", () => {
    const dist = scratchDir("yokohama-presence-stale-");
    writePresence(dist, ALL);
    expect(existsSync(join(dist, "sitemap.xml"))).toBe(true);
    writePresence(dist, NONE);
    expect(existsSync(join(dist, "sitemap.xml"))).toBe(false);
    expect(readFileSync(join(dist, "robots.txt"), "utf8")).not.toMatch(/sitemap/i);
  });

  function run(dist: string, set: Record<string, string>): { stdout: string; files: string[] } {
    const stdout = execFileSync(process.execPath, [presenceScript], {
      cwd: web,
      env: { ...envWith(set), YOKOHAMA_WEB_DIST_DIR: dist },
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { stdout, files: readdirSync(dist).sort() };
  }

  /** A scratch `dist` that holds the shell, as a finished `vite build` leaves it. */
  function distWithShell(): string {
    const dist = scratchDir("yokohama-presence-run-");
    writeFileSync(join(dist, "index.html"), "<!doctype html>\n");
    return dist;
  }

  it("reads the checked build values from the environment, as the build does", () => {
    const none = run(distWithShell(), {});
    expect(none.files).toEqual(["index.html", "llms.txt", "robots.txt"]);

    const dist = distWithShell();
    const all = run(dist, {
      YOKOHAMA_SITE_URL: `${SITE}/`,
      YOKOHAMA_PROJECT_URL: REPO,
      YOKOHAMA_SITE_BLURB: ALL.blurb,
      YOKOHAMA_SITE_NOTICE: ALL.notice,
    });
    expect(all.files).toEqual(["index.html", "llms.txt", "robots.txt", "sitemap.xml"]);
    expect(readFileSync(join(dist, "llms.txt"), "utf8")).toBe(llmsTxt(ALL));
    expect(readFileSync(join(dist, "robots.txt"), "utf8")).toBe(robotsTxt(ALL));
    // The checked origin: the trailing slash the deployer typed is gone.
    expect(readFileSync(join(dist, "sitemap.xml"), "utf8")).toBe(sitemapXml(ALL));
    expect(all.stdout).toMatch(/robots\.txt/);
  });

  // The script reads the values through the BUILD's checkers, so a bad one
  // stops it naming the variable, and writes nothing.
  it.each([
    ["YOKOHAMA_SITE_URL", "https://yokohama.example.org/path"],
    ["YOKOHAMA_SITE_URL", "http://yokohama.example.org"],
    ["YOKOHAMA_PROJECT_URL", "not a url"],
    ["YOKOHAMA_SITE_BLURB", "two\nlines"],
  ])("refuses a bad %s, naming it, and writes nothing", (name, value) => {
    const dist = distWithShell();
    let failure: { status: number | null; stderr: string } | undefined;
    try {
      run(dist, { [name]: value });
    } catch (error) {
      failure = error as { status: number | null; stderr: string };
    }
    expect(failure, "the script stops").toBeDefined();
    expect(failure!.status).not.toBe(0);
    expect(String(failure!.stderr)).toContain(name);
    expect(readdirSync(dist)).toEqual(["index.html"]);
  });

  // The script runs only when it is the entry point, which it decides by
  // comparing its own file to the one node was asked to run. A checkout reached
  // through a symlink (macOS's /tmp is one) makes those two spell the path
  // differently, and a guard that compared the strings would skip `main()` and
  // let the build succeed with no presence files at all — the quietest way for
  // this stage to fail.
  it("writes its files when started through a symlink", () => {
    const dist = distWithShell();
    const linked = join(scratchDir("yokohama-presence-link-"), "presence.mjs");
    symlinkSync(presenceScript, linked);
    const stdout = execFileSync(process.execPath, [linked], {
      cwd: web,
      env: { ...envWith({}), YOKOHAMA_WEB_DIST_DIR: dist },
      encoding: "utf8",
    });
    expect(stdout).toMatch(/robots\.txt/);
    expect(readdirSync(dist).sort()).toEqual(["index.html", "llms.txt", "robots.txt"]);
  });

  it("refuses a dist with no built shell, rather than writing into nothing", () => {
    const dist = scratchDir("yokohama-presence-empty-");
    mkdirSync(join(dist, "assets"));
    let failure: { status: number | null; stderr: string } | undefined;
    try {
      execFileSync(process.execPath, [presenceScript], {
        cwd: web,
        env: { ...envWith({}), YOKOHAMA_WEB_DIST_DIR: dist },
        stdio: ["ignore", "pipe", "pipe"],
      });
    } catch (error) {
      failure = error as { status: number | null; stderr: string };
    }
    expect(failure, "the script stops").toBeDefined();
    expect(String(failure!.stderr)).toMatch(/index\.html/);
    expect(readdirSync(dist)).toEqual(["assets"]);
  });
});

describe("the 404 page and the build script", () => {
  // Task 5a's file, READ ONLY: a page a scanner is sent to must not be indexed.
  it("keeps noindex on the 404 page", () => {
    const page = readFileSync(join(web, "public", "404.html"), "utf8");
    expect(count(page, /<meta name="robots" content="noindex" \/>/)).toBe(1);
  });

  // The catcher for presence dropped from the build, or run before the
  // generator: `build:client` ends `precache.mjs && presence.mjs`.
  it("has build:client write the presence files after the precache generator", () => {
    const { scripts } = JSON.parse(readFileSync(join(web, "package.json"), "utf8")) as { scripts: Record<string, string> };
    const stages = scripts["build:client"]!.split("&&").map((s) => s.trim());
    expect(stages).toEqual(["tsc -b", "vite build", "node scripts/precache.mjs", "node scripts/presence.mjs"]);
  });
});

// **A real build's output** (layer 3): `build:client`'s own sequence minus the
// typecheck, into a temporary directory, with every build value set and with
// none. Needs the generated wasm glue; skips out loud without it.
const glue = join(web, "src", "local", "wasm", "yokohama_wasm.js");
if (!existsSync(glue)) {
  process.stdout.write(
    "\npresence: the REAL-BUILD cells were SKIPPED and checked NOTHING — " +
      "src/local/wasm/yokohama_wasm.js is absent.\n  Run `npm run build:wasm` to check them.\n\n",
  );
}

describe.skipIf(!existsSync(glue))("what a real build writes", () => {
  const vite = join(web, "node_modules", "vite", "bin", "vite.js");

  /** `vite build`, the generator, then the presence script, into a fresh
   *  temporary directory, with only `set` in the build-value environment. */
  function build(set: Record<string, string>): string {
    const out = scratchDir("yokohama-presence-build-");
    const env = { ...envWith(set), YOKOHAMA_WEB_DIST_DIR: out };
    const opts = { cwd: web, env, stdio: "pipe" as const };
    execFileSync(process.execPath, [vite, "build", "--outDir", out, "--emptyOutDir", "--logLevel", "error"], opts);
    execFileSync(process.execPath, [join(web, "scripts", "precache.mjs")], opts);
    execFileSync(process.execPath, [presenceScript], opts);
    return out;
  }

  const read = (dir: string, name: string) => readFileSync(join(dir, name), "utf8");
  const TAGS: Array<[string, RegExp]> = [
    ["description", /<meta name="description" /],
    ["og:title", /<meta property="og:title" /],
    ["og:description", /<meta property="og:description" /],
    ["og:type", /<meta property="og:type" /],
    ["og:image", /<meta property="og:image" /],
    ["twitter:card", /<meta name="twitter:card" /],
    ["JSON-LD", /<script type="application\/ld\+json">/],
  ];

  let withValues = "";
  let without = "";

  it("builds the shell with its metadata, every tag exactly once, every value set", () => {
    withValues = build({
      YOKOHAMA_SITE_URL: SITE,
      YOKOHAMA_PROJECT_URL: REPO,
      YOKOHAMA_SITE_BLURB: "A blurb with a \"quote\" & an <angle>.",
      YOKOHAMA_SITE_NOTICE: "A notice.",
      YOKOHAMA_RELEASE_FEED: "off",
    });
    const html = read(withValues, "index.html");
    for (const [what, re] of TAGS) expect(count(html, re), `${what} once`).toBe(1);
    expect(count(html, /<link rel="canonical" /), "canonical once").toBe(1);
    expect(count(html, /<meta property="og:url" /), "og:url once").toBe(1);
    expect(count(html, new RegExp(METADATA_PLACEHOLDER.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))), "no placeholder left").toBe(0);

    expect(attrOf(html, 'link rel="canonical"', "href")).toBe(`${SITE}/`);
    expect(attrOf(html, 'meta property="og:url"', "content")).toBe(`${SITE}/`);
    expect(attrOf(html, 'meta property="og:image"', "content")).toBe(`${SITE}${OG_IMAGE_PATH}`);
    expect(attrOf(html, 'meta name="description"', "content")).toBe('A blurb with a "quote" & an <angle>.');
    const lds = jsonLd(html);
    expect(lds, "the JSON-LD parses").toHaveLength(1);
    expect(lds[0]).toHaveProperty("codeRepository", REPO);
    expect(lds[0]).toHaveProperty("url", `${SITE}/`);
    expect(lds[0]).toHaveProperty("description", 'A blurb with a "quote" & an <angle>.');
    // The shell the worker installs is this file, byte for byte.
    expect(readFileSync(join(withValues, "shell.bin")).equals(readFileSync(join(withValues, "index.html")))).toBe(true);
  }, 180_000);

  it("builds the shell with its metadata and none of the address-bound tags, no value set", () => {
    without = build({ YOKOHAMA_RELEASE_FEED: "off" });
    const html = read(without, "index.html");
    for (const [what, re] of TAGS) expect(count(html, re), `${what} once`).toBe(1);
    // Counted as TAGS: the shell's own comment names the canonical link and
    // `og:url` in words, and a word is not a tag.
    expect(count(html, /<link rel="canonical"/), "no canonical link").toBe(0);
    expect(count(html, /<meta property="og:url"/), "no og:url").toBe(0);
    expect(html).not.toContain("codeRepository");
    expect(attrOf(html, 'meta name="description"', "content")).toBe(DEFAULT_DESCRIPTION);
    // The image is root-relative, and the build left it so.
    expect(attrOf(html, 'meta property="og:image"', "content")).toBe(OG_IMAGE_PATH);
    const lds = jsonLd(html);
    expect(lds, "the JSON-LD parses").toHaveLength(1);
    expect(lds[0]).not.toHaveProperty("url");
    expect(lds[0]).not.toHaveProperty("codeRepository");
  }, 180_000);

  it("writes the files each tree should hold, and the stamp says which address-bound values it carries", () => {
    expect(withValues, "the first build ran").not.toBe("");
    expect(without, "the second build ran").not.toBe("");
    for (const dir of [withValues, without]) {
      expect(existsSync(join(dir, "robots.txt")), "robots.txt").toBe(true);
      expect(existsSync(join(dir, "llms.txt")), "llms.txt").toBe(true);
      expect(existsSync(join(dir, "404.html")), "404.html").toBe(true);
      expect(read(dir, "404.html")).toContain('<meta name="robots" content="noindex" />');
      expect(read(dir, "llms.txt")).not.toContain("@");
    }
    expect(existsSync(join(withValues, "sitemap.xml"))).toBe(true);
    expect(read(withValues, "robots.txt")).toBe(`${ROBOTS}\nSitemap: ${SITE}/sitemap.xml\n`);
    expect(read(withValues, "llms.txt")).toContain(`[The repository](${REPO})`);
    expect(count(read(withValues, "sitemap.xml"), /<url>/)).toBe(1);

    expect(existsSync(join(without, "sitemap.xml"))).toBe(false);
    expect(read(without, "robots.txt")).toBe(ROBOTS);
    expect(read(without, "llms.txt")).not.toMatch(/## Code|repository/i);

    expect(JSON.parse(read(withValues, "build-values.json"))).toHaveProperty("site_url", true);
    expect(JSON.parse(read(without, "build-values.json"))).toHaveProperty("site_url", false);
  });

  // The precache list is the tree less the files a controlled browser never
  // needs: the generator ran BEFORE the presence files existed, and its
  // exclusion would hold them out of a re-run too.
  it("lists none of the presence files in the release", () => {
    for (const dir of [withValues, without]) {
      const listed = readPrecacheList(read(dir, "sw.js"), "built").map((e) => e.path);
      for (const name of ["/robots.txt", "/llms.txt", "/sitemap.xml", "/404.html", "/build-values.json"]) {
        expect(listed, `${name} is not precached`).not.toContain(name);
      }
    }
    // A second run of the generator over the finished tree (what the offline
    // spec's second release does) still leaves them out.
    execFileSync(process.execPath, [join(web, "scripts", "precache.mjs")], {
      cwd: web,
      env: { ...envWith({}), YOKOHAMA_WEB_DIST_DIR: withValues },
      stdio: "pipe",
    });
    const again = readPrecacheList(read(withValues, "sw.js"), "built").map((e) => e.path);
    for (const name of ["/robots.txt", "/llms.txt", "/sitemap.xml"]) expect(again).not.toContain(name);
  }, 180_000);
});
