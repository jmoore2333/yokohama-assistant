// **The web presence: `robots.txt`, `llms.txt`, the sitemap and the page's
// metadata** (cycle F, R-F12; spec §2.7).
//
// Static files and page metadata for search engines and language models, with
// defaults that hold for any public deployment, written from the build's
// CHECKED values (`build-values.mjs`) so their words agree with the door's.
// Nothing here is fetched by the client, precached or scripted, and nothing
// names an address but the one the deployer gave as `YOKOHAMA_SITE_URL`.
//
// Two halves, one module:
//
// - **The pure builders** (`robotsTxt`, `llmsTxt`, `sitemapXml`, `metadataTags`,
//   `injectMetadata`) take values and return text, so a cell reads the exact
//   words without a build. `vite.config.ts` imports `injectMetadata` for its
//   `transformIndexHtml` hook: the shell's tags are written at `vite build`
//   from the same locals the `define` block reads.
// - **The script** (`node scripts/presence.mjs`, the last stage of
//   `npm run build:client`, AFTER `precache.mjs`) reads the same variables
//   through the same checkers and writes the three files into `dist`. It runs
//   after the generator, and `precache.mjs`'s walk leaves all three out of the
//   release's list regardless, so the offline count never moves and a
//   controlled browser never downloads them.
//
// **Each value earns what it names and nothing else.** The `Sitemap:` line,
// `sitemap.xml`, `<link rel="canonical">` and `og:url` exist only with the
// site's address; the repository link appears in `llms.txt` and the JSON-LD
// only with the project link; the blurb and the notice each appear only when
// set. A crawler is told to stay off the three paths a request to which is a
// metered invocation (`/api/`, `/enter`, `/cdn-cgi/`). A plain module with a
// declaration beside it (`presence.d.mts`), as `build-values.mjs` is, so the
// node project type-checks the config that imports it.

import { existsSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { checkProjectUrl, checkSiteUrl, checkText, BUILD_VALUES } from "./build-values.mjs";

/** The description when no blurb is set. */
export const DEFAULT_DESCRIPTION = "Yokohama, the board game, in your browser. Play the bot here, or play with friends.";

/** The picture a link card shows: the app's own 512 px icon, which is original
 *  to this repository (`web/public/icons/`; `ASSETS.md` carves out the card art,
 *  never this mark). Root-relative; absolute when the site's address is known. */
export const OG_IMAGE_PATH = "/icons/icon-512.png";

/** The comment in `web/index.html` the transform replaces with the tags. */
export const METADATA_PLACEHOLDER = "<!-- yokohama:presence-metadata -->";

const LICENSE_URL = "https://www.apache.org/licenses/LICENSE-2.0";

/** Text for an HTML attribute value (double-quoted) or an XML text node. */
function escapeMarkup(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** JSON for the inside of a `<script>` element: `<`, `>` and `&` as their
 *  `\u` escapes, so no text a blurb holds can close the element or open
 *  another. The result is still one JSON document with the same value. */
function jsonForScript(value) {
  return JSON.stringify(value).replace(/[<>&]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

/**
 * `robots.txt`: everything allowed but the three paths a crawler must never
 * reach (the API, the door's turnstile and Cloudflare's own `/cdn-cgi/`), and
 * the sitemap's address when the site's is known.
 *
 * @param {{ siteUrl: string | null }} values
 * @returns {string}
 */
export function robotsTxt({ siteUrl }) {
  const body = ["User-agent: *", "Allow: /", "Disallow: /api/", "Disallow: /enter", "Disallow: /cdn-cgi/"].join("\n");
  return siteUrl === null ? `${body}\n` : `${body}\n\nSitemap: ${siteUrl}/sitemap.xml\n`;
}

/**
 * `llms.txt`, in the `llmstxt.org` shape: a title, a summary blockquote and
 * sections of links. The "Code" section is the repository link and the licence
 * with its carve-out, and exists only when the project link is set.
 *
 * @param {{ blurb: string | null, notice: string | null, projectUrl: string | null }} values
 * @returns {string}
 */
export function llmsTxt({ blurb, notice, projectUrl }) {
  /** @type {string[]} */
  const sections = [
    "# Yokohama",
    "> Yokohama, the board game by Hisashi Hayashi, in your browser: a bot game that runs entirely in the browser, and multiplayer behind a door. An independent open-source project, not affiliated with the game's designer or publisher.",
  ];
  const words = [];
  if (blurb !== null) words.push(`- The blurb: ${blurb}`);
  if (notice !== null) words.push(`- The notice: ${notice}`);
  if (words.length > 0) sections.push(words.join("\n"));
  if (projectUrl !== null) {
    sections.push(
      `## Code\n\n- [The repository](${projectUrl}): Apache-2.0; the game's own material carries no grant (ASSETS.md in the repository).`,
    );
  }
  sections.push(
    [
      "## What a visitor can do here",
      "",
      "- Play the bot with no account: everything runs in the browser and works offline once installed.",
      "- Play with friends behind the door at /enter, which is not for automated clients.",
      "- The API under /api/ serves the game's own client and is not a public API.",
    ].join("\n"),
  );
  return `${sections.join("\n\n")}\n`;
}

/**
 * `sitemap.xml`: the one page the site has. Written only with the site's
 * address, which the caller checks.
 *
 * @param {{ siteUrl: string }} values
 * @returns {string}
 */
export function sitemapXml({ siteUrl }) {
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    `  <url><loc>${escapeMarkup(siteUrl)}/</loc></url>`,
    "</urlset>",
    "",
  ].join("\n");
}

/**
 * The page's metadata, as head markup, one tag per line: the description, the
 * canonical link (with the site's address only), Open Graph, the Twitter
 * summary card and one JSON-LD block for a free `WebApplication` in the games
 * category under Apache-2.0 — `url` with the site's address, the repository
 * with the project link, and no claim of affiliation anywhere.
 *
 * **The repository is `isBasedOn`, a `SoftwareSourceCode` node, never a
 * top-level `codeRepository`**: that property belongs to `SoftwareSourceCode`,
 * not to `WebApplication`, so a validator flags it on the page's own node and
 * a consumer drops it. The node exists only with the project link.
 *
 * @param {{ siteUrl: string | null, blurb: string | null, projectUrl: string | null }} values
 * @returns {string}
 */
export function metadataTags({ siteUrl, blurb, projectUrl }) {
  const description = blurb ?? DEFAULT_DESCRIPTION;
  const page = siteUrl === null ? null : `${siteUrl}/`;
  const image = siteUrl === null ? OG_IMAGE_PATH : `${siteUrl}${OG_IMAGE_PATH}`;
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "WebApplication",
    name: "Yokohama",
    applicationCategory: "GameApplication",
    operatingSystem: "Any",
    isAccessibleForFree: true,
    license: LICENSE_URL,
    description,
    ...(page === null ? {} : { url: page }),
    ...(projectUrl === null ? {} : { isBasedOn: { "@type": "SoftwareSourceCode", codeRepository: projectUrl } }),
  };
  return [
    `<meta name="description" content="${escapeMarkup(description)}" />`,
    ...(page === null ? [] : [`<link rel="canonical" href="${escapeMarkup(page)}" />`]),
    `<meta property="og:title" content="Yokohama" />`,
    `<meta property="og:description" content="${escapeMarkup(description)}" />`,
    `<meta property="og:type" content="website" />`,
    ...(page === null ? [] : [`<meta property="og:url" content="${escapeMarkup(page)}" />`]),
    `<meta property="og:image" content="${escapeMarkup(image)}" />`,
    `<meta name="twitter:card" content="summary" />`,
    `<script type="application/ld+json">${jsonForScript(jsonLd)}</script>`,
  ].join("\n    ");
}

/**
 * The shell with its metadata in place of the placeholder comment. A shell
 * with no placeholder, or two, is refused: the alternative is a build that
 * ships with no metadata and says nothing. The replacement is a FUNCTION, so
 * `$&` and `$'` in a blurb are text and not `String.replace` patterns.
 *
 * @param {string} html
 * @param {{ siteUrl: string | null, blurb: string | null, projectUrl: string | null }} values
 * @returns {string}
 */
export function injectMetadata(html, values) {
  const found = html.split(METADATA_PLACEHOLDER).length - 1;
  if (found !== 1) {
    throw new Error(
      `index.html carries ${found} copies of the presence placeholder ${METADATA_PLACEHOLDER}; it needs exactly one`,
    );
  }
  const tags = metadataTags(values);
  return html.replace(METADATA_PLACEHOLDER, () => tags);
}

/**
 * The files for `values`, written into `dist`: `robots.txt` and `llms.txt`
 * always, `sitemap.xml` only with the site's address — and a sitemap an earlier
 * run left behind is taken away when there is none, so a re-run without the
 * address never leaves one published. Returns the names written, in order.
 *
 * @param {string} dist
 * @param {{ siteUrl: string | null, blurb: string | null, notice: string | null, projectUrl: string | null }} values
 * @returns {string[]}
 */
export function writePresence(dist, values) {
  writeFileSync(join(dist, "robots.txt"), robotsTxt(values));
  writeFileSync(join(dist, "llms.txt"), llmsTxt(values));
  const written = ["robots.txt", "llms.txt"];
  if (values.siteUrl === null) {
    rmSync(join(dist, "sitemap.xml"), { force: true });
  } else {
    writeFileSync(join(dist, "sitemap.xml"), sitemapXml({ siteUrl: values.siteUrl }));
    written.push("sitemap.xml");
  }
  return written;
}

/**
 * The four values the files are written from, read from an environment through
 * the build's own checkers — a bad one throws naming its variable.
 *
 * @param {Record<string, string | undefined>} env
 */
export function readValues(env) {
  return {
    siteUrl: checkSiteUrl(env.YOKOHAMA_SITE_URL),
    projectUrl: checkProjectUrl(env.YOKOHAMA_PROJECT_URL),
    blurb: checkText("YOKOHAMA_SITE_BLURB", env.YOKOHAMA_SITE_BLURB, BUILD_VALUES.YOKOHAMA_SITE_BLURB),
    notice: checkText("YOKOHAMA_SITE_NOTICE", env.YOKOHAMA_SITE_NOTICE, BUILD_VALUES.YOKOHAMA_SITE_NOTICE),
  };
}

function main() {
  const web = fileURLToPath(new URL("..", import.meta.url));
  const dist = process.env.YOKOHAMA_WEB_DIST_DIR ?? join(web, "dist");
  try {
    const values = readValues(process.env);
    if (!existsSync(join(dist, "index.html"))) {
      throw new Error(`no built client at ${dist}: index.html is absent — run \`vite build\` first`);
    }
    const written = writePresence(dist, values);
    console.log(`presence: wrote ${written.join(", ")} into ${dist}`);
  } catch (error) {
    console.error(`presence: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}

/** Whether this file is the entry point node was asked to run. Both sides are
 *  REAL paths: node resolves a symlinked entry for `import.meta.url` and not
 *  for `argv[1]`, and a guard comparing the two spellings would skip `main()`
 *  and let `build:client` succeed with no presence files. */
function isEntryPoint() {
  const entry = process.argv[1];
  if (entry === undefined) return false;
  try {
    return realpathSync(entry) === realpathSync(fileURLToPath(import.meta.url));
  } catch {
    return false;
  }
}

// Run as a script, never on import: `vite.config.ts` imports the builders.
if (isEntryPoint()) main();
