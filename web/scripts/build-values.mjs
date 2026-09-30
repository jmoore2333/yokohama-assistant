// The door's THREE build values, checked where the BUILD reads them (M6.5c2,
// R-6.5c2.6 and R-6.5c2.40; spec §2.3).
//
// `vite.config.ts` calls these with `process.env`, and a throw here fails
// `vite build` — the Worker's release build and the Docker image's
// `build:client` alike — naming the variable and the reason. The three:
// `YOKOHAMA_PROJECT_URL` (the door's and the lobby's link to the repository:
// unset or empty is no link; otherwise an absolute `https://` or `http://` URL
// with a host, no whitespace or control character, at most 200 characters),
// `YOKOHAMA_SITE_BLURB` (the words under the title, at most 400) and
// `YOKOHAMA_SITE_NOTICE` (the one-line footer on every screen, at most 200);
// the last two through `checkText`. Unset or empty is none, for all three.
//
// **Build time, never run time**: the release is precached under a digest of
// its own bytes, so a server that rewrote `index.html` at start-up would fail
// the service worker's install check. A plain module with a declaration beside
// it (`build-values.d.mts`), so the node project type-checks the config.

export const PROJECT_URL_VAR = "YOKOHAMA_PROJECT_URL";
export const PROJECT_URL_MAX = 200;

/**
 * @param {string | undefined} raw
 * @returns {string | null}
 */
export function checkProjectUrl(raw) {
  if (raw === undefined || raw === "") return null;
  /** @param {string} why @returns {never} */
  const refuse = (why) => {
    throw new Error(`${PROJECT_URL_VAR} ${why}: ${JSON.stringify(raw)}`);
  };
  if (raw.length > PROJECT_URL_MAX) refuse(`is longer than ${PROJECT_URL_MAX} characters`);
  if (/[\s\u0000-\u001f\u007f-\u009f]/u.test(raw)) refuse("holds whitespace or a control character");
  if (!/^https?:\/\//i.test(raw)) refuse("must begin with https:// or http://");
  let url;
  try {
    url = new URL(raw);
  } catch {
    refuse("is not a URL");
  }
  if (url.hostname === "") refuse("names no host");
  return raw;
}

/** The three build values and the longest each may be. */
export const BUILD_VALUES = {
  YOKOHAMA_PROJECT_URL: PROJECT_URL_MAX,
  YOKOHAMA_SITE_BLURB: 400,
  YOKOHAMA_SITE_NOTICE: 200,
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
  if (/[\u0000-\u001f\u007f-\u009f]/u.test(raw)) {
    throw new Error(`${name} holds a control character: ${JSON.stringify(raw)}`);
  }
  if (raw.length > max) throw new Error(`${name} is longer than ${max} characters`);
  return raw;
}
