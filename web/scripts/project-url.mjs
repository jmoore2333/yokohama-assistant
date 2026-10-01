// The door's project link, checked where the BUILD reads it (M6.5c2,
// R-6.5c2.6; spec §2.3).
//
// `vite.config.ts` calls this with `process.env.YOKOHAMA_PROJECT_URL`, and a
// throw here fails `vite build` — the Worker's release build and the Docker
// image's `build:client` alike — naming the variable and the reason. The rule:
// unset or empty is no link; otherwise an absolute `https://` or `http://` URL
// with a host, no whitespace or control character, at most 200 characters.
//
// **Build time, never run time**: the release is precached under a digest of
// its own bytes, so a server that rewrote `index.html` at start-up would fail
// the service worker's install check. A plain module with a declaration beside
// it (`project-url.d.mts`), so the node project type-checks the config.

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
