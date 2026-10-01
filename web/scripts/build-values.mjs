// The build's SIX values, checked where the BUILD reads them (M6.5c2,
// R-6.5c2.6 and R-6.5c2.40; spec §2.3; cycle F, R-F2, R-F6 and R-F12).
//
// `vite.config.ts` calls these with `process.env`, and a throw here fails
// `vite build` — the Worker's release build and the Docker image's
// `build:client` alike — naming the variable and the reason. The door's three:
// `YOKOHAMA_PROJECT_URL` (the door's and the lobby's link to the repository:
// unset or empty is no link; otherwise an absolute `https://` or `http://` URL
// with a host, no whitespace or control character, at most 200 characters),
// `YOKOHAMA_SITE_BLURB` (the words under the title, at most 400) and
// `YOKOHAMA_SITE_NOTICE` (the one-line footer on every screen, at most 200);
// the last two through `checkText`. Unset or empty is none, for all three.
//
// **Cycle F adds three.** `YOKOHAMA_RELEASE_FEED` (the URL the door and the lobby
// read for the latest release; THREE states, `checkReleaseFeed`: unset or empty
// derives it from a GitHub project link, a URL is that feed, the literal `off`
// is none), `YOKOHAMA_TURNSTILE_SITEKEY` (the public key a Turnstile
// widget is drawn with; unset or empty is no key, `checkSiteKey`) and
// `YOKOHAMA_SITE_URL` (the deployment's canonical origin, which the sitemap,
// the page's canonical link and `og:url` are written from; unset or empty is
// no address and none of the three, `checkSiteUrl`). One row in
// `BUILD_VALUES` and one checker each: a later value joins in the same shape.
//
// **Build time, never run time**: the release is precached under a digest of
// its own bytes, so a server that rewrote `index.html` at start-up would fail
// the service worker's install check. A plain module with a declaration beside
// it (`build-values.d.mts`), so the node project type-checks the config.

export const PROJECT_URL_VAR = "YOKOHAMA_PROJECT_URL";
export const PROJECT_URL_MAX = 200;

/**
 * The rules every URL build value shares: not over `max`, no whitespace or
 * control character, one of the allowed schemes, parseable, with a host. Every
 * refusal names `name`, so a deployer reads which variable to fix.
 *
 * @param {string} name
 * @param {string} raw a NON-EMPTY value; the callers read unset and empty first
 * @param {number} max
 * @param {boolean} httpsOnly whether plain `http://` is refused as well
 * @returns {string}
 */
function checkUrl(name, raw, max, httpsOnly) {
  /** @param {string} why @returns {never} */
  const refuse = (why) => {
    throw new Error(`${name} ${why}: ${JSON.stringify(raw)}`);
  };
  if (raw.length > max) refuse(`is longer than ${max} characters`);
  if (/[\s\u0000-\u001f\u007f-\u009f]/u.test(raw)) refuse("holds whitespace or a control character");
  if (httpsOnly) {
    if (!/^https:\/\//i.test(raw)) refuse("must begin with https://");
  } else if (!/^https?:\/\//i.test(raw)) {
    refuse("must begin with https:// or http://");
  }
  let url;
  try {
    url = new URL(raw);
  } catch {
    refuse("is not a URL");
  }
  if (url.hostname === "") refuse("names no host");
  return raw;
}

/**
 * @param {string | undefined} raw
 * @returns {string | null}
 */
export function checkProjectUrl(raw) {
  if (raw === undefined || raw === "") return null;
  return checkUrl(PROJECT_URL_VAR, raw, PROJECT_URL_MAX, false);
}

export const RELEASE_FEED_VAR = "YOKOHAMA_RELEASE_FEED";
export const RELEASE_FEED_MAX = 300;
export const SITE_KEY_VAR = "YOKOHAMA_TURNSTILE_SITEKEY";
export const SITE_KEY_MAX = 100;

/**
 * The feed a GitHub project link implies: `https://github.com/<owner>/<repo>`
 * (a trailing slash tolerated) is that repository's latest-release endpoint.
 * Anything else — another host, a subdomain, a path under the repository, plain
 * `http` — is `null`, so a link that only looks like GitHub's never points a
 * build at another host's API. Pure; `checkReleaseFeed` is its caller.
 *
 * @param {string | null} projectUrl the CHECKED project link, or `null`
 * @returns {string | null}
 */
export function defaultFeedFor(projectUrl) {
  if (projectUrl === null) return null;
  const match = /^https:\/\/github\.com\/([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)\/?$/i.exec(projectUrl);
  if (match === null) return null;
  return `https://api.github.com/repos/${match[1]}/${match[2]}/releases/latest`;
}

/**
 * The release feed's THREE states (R-FP5), spelled so "empty" and "unset"
 * cannot be confused with "off": unset or empty derives the feed from the
 * project link (`defaultFeedFor`); the literal `off` is none; anything else is
 * a URL, checked as the project link is but `https://` only (the two hosts are
 * served over TLS, and a plain-http request from such a page is blocked) and at
 * most 300 characters.
 *
 * @param {string | undefined} raw
 * @param {string | null} projectUrl the CHECKED project link, or `null`
 * @returns {string | null}
 */
export function checkReleaseFeed(raw, projectUrl) {
  if (raw === undefined || raw === "") return defaultFeedFor(projectUrl);
  if (raw === "off") return null;
  return checkUrl(RELEASE_FEED_VAR, raw, RELEASE_FEED_MAX, true);
}

/**
 * The Turnstile site key: public, but still typed by a deployer, so plain text
 * with no whitespace or control character (an invisible format character
 * counts) and at most 100 characters. Unset and empty are both no key.
 *
 * @param {string | undefined} raw
 * @returns {string | null}
 */
export function checkSiteKey(raw) {
  if (raw === undefined || raw === "") return null;
  if (/[\s\p{Cc}\p{Cf}\p{Zl}\p{Zp}]/u.test(raw)) {
    throw new Error(`${SITE_KEY_VAR} holds whitespace or a control character: ${JSON.stringify(raw)}`);
  }
  if (raw.length > SITE_KEY_MAX) throw new Error(`${SITE_KEY_VAR} is longer than ${SITE_KEY_MAX} characters`);
  return raw;
}

export const SITE_URL_VAR = "YOKOHAMA_SITE_URL";
export const SITE_URL_MAX = 200;

/**
 * The deployment's canonical address (cycle F, R-F12; spec §2.7): an absolute
 * `https://` ORIGIN and nothing more — the shared URL rules (`checkUrl`, which
 * names the variable in every refusal), then no path beyond one trailing `/`,
 * no query, no fragment and no credentials. The value comes back as the
 * parsed origin (a trailing `/` gone, the scheme and host in lower case), so
 * every consumer writes `${siteUrl}/sitemap.xml` and none can produce a double
 * slash. A credential is refused WITHOUT echoing the value: a build log is no
 * place for a password a deployer pasted by mistake.
 *
 * @param {string | undefined} raw
 * @returns {string | null}
 */
export function checkSiteUrl(raw) {
  if (raw === undefined || raw === "") return null;
  checkUrl(SITE_URL_VAR, raw, SITE_URL_MAX, true);
  const parts = /^https:\/\/([^/?#]*)\/?$/i.exec(raw);
  if (parts === null) {
    throw new Error(
      `${SITE_URL_VAR} must be an origin, with no path, query or fragment beyond a trailing /: ${JSON.stringify(raw)}`,
    );
  }
  const url = new URL(raw);
  if (url.username !== "" || url.password !== "") {
    throw new Error(`${SITE_URL_VAR} holds credentials, which an origin does not carry`);
  }
  return url.origin;
}

/** The six build values and the longest each may be. */
export const BUILD_VALUES = {
  YOKOHAMA_PROJECT_URL: PROJECT_URL_MAX,
  YOKOHAMA_SITE_BLURB: 400,
  YOKOHAMA_SITE_NOTICE: 200,
  YOKOHAMA_RELEASE_FEED: RELEASE_FEED_MAX,
  YOKOHAMA_TURNSTILE_SITEKEY: SITE_KEY_MAX,
  YOKOHAMA_SITE_URL: SITE_URL_MAX,
};

/**
 * @param {string} name
 * @param {string | undefined} raw
 * @param {number} max
 * @returns {string | null}
 */
export function checkText(name, raw, max) {
  if (raw === undefined || raw === "") return null;
  if (raw.trim() === "") throw new Error(`${name} holds no words: ${JSON.stringify(raw)}`);
  if (/[\p{Cc}\p{Zl}\p{Zp}\p{Cf}]/u.test(raw)) {
    throw new Error(`${name} holds a control character: ${JSON.stringify(raw)}`);
  }
  if (raw.length > max) throw new Error(`${name} is longer than ${max} characters`);
  return raw;
}
