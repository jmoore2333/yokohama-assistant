// Whether a newer release than this build exists (M6.5c2 cycle F, R-F2, R-FP5;
// spec §2.2): the version comparison and the hook that asks the feed.
//
// **One cross-origin request, and the door's "asks the server nothing" keeps its
// meaning.** The feed is another host's (GitHub's `releases/latest`, unless a
// build points elsewhere): the service worker leaves a cross-origin request
// alone (`sw.ts`'s same-origin rule), and the Worker never sees it. The read
// goes through an injectable `fetch`, so a cell answers it with a fixture and
// the door's own cell asserts that no SAME-origin call was made.
//
// **At most once per page life, and at most once an hour per browser.** The
// page's answer is the MODULE's — the door and the lobby each draw the notice,
// and a player who goes from one to the other is in the same page, so a hook-
// local memory would ask again at every remount. After the page's first check
// the browser's own memory takes over: `{checkedAt, tag, url}` under
// `yokohama.release.checked`, every access wrapped (`browserStorage.ts`, the
// `chatFold.ts` idiom), which keeps a busy device inside the feed host's
// anonymous limit (60 requests an hour). **The ANSWER is kept, not only the
// time** (F-13): a reload inside the hour draws the notice again with no
// request, and the kept tag is compared with the build at READ time, so a site
// that has taken the release since stops saying so. A broken storage means
// "check". **A failure is not an answer**: any non-2xx, network error, body
// that is not JSON or body with no tag draws nothing, keeps nothing and
// retries nothing within the page life.
//
// **The feed is a third party's, so its words are read, never trusted.** The
// tag is compared only after it parses as `vX.Y.Z` (a tag that does not draws
// nothing, never a false alarm), and its `html_url` becomes an `href` only as
// an absolute `http(s)` URL — never `javascript:`.

import { useEffect, useSyncExternalStore } from "react";
import { readItem, writeItem } from "./browserStorage";
import { buildVersion, projectLink, releaseFeed } from "./buildValues";

/** The browser's memory of the feed's last answer. */
export const CHECKED_KEY = "yokohama.release.checked";
/** How long a kept answer is drawn from before the feed is asked again. */
const FRESH_MS = 60 * 60 * 1000;
/** The longest address taken from the feed as a link. */
const URL_MAX = 300;

/** The newest release the feed names: its tag, and where to read about it —
 *  `""` when neither the feed nor the build's project link gave an address, in
 *  which case the notice draws the tag without a link. */
export type ReleaseAnswer = { tag: string; url: string };

/** The feed this build reads, or `null`: `off`, or no GitHub project link to
 *  derive one from (`build-values.mjs`'s `checkReleaseFeed`). */
export function releaseFeedUrl(): string | null {
  return releaseFeed();
}

/** `vX.Y.Z` or `X.Y.Z` as three numbers; anything else — a fourth number, a
 *  suffix like `-rc1`, a component too long to compare exactly — is `null`. */
export function parseVersion(s: string): [number, number, number] | null {
  const m = /^v?(\d{1,9})\.(\d{1,9})\.(\d{1,9})$/.exec(s);
  return m === null ? null : [Number(m[1]), Number(m[2]), Number(m[3])];
}

/** Whether `tag` is AHEAD of `own`, compared place by place as numbers. Equal,
 *  older and unparseable are all `false`: the notice is for a release this
 *  build is behind, and a tag it cannot read is never a false alarm. */
export function isNewer(tag: string, own: string): boolean {
  const a = parseVersion(tag);
  const b = parseVersion(own);
  if (a === null || b === null) return false;
  for (let i = 0; i < 3; i++) {
    const x = a[i] ?? 0;
    const y = b[i] ?? 0;
    if (x !== y) return x > y;
  }
  return false;
}

/** An absolute `http(s)` address with a host, no whitespace or control
 *  character, of a sane length — or `null`. */
function httpUrl(value: unknown): string | null {
  if (typeof value !== "string" || value.length > URL_MAX) return null;
  if (/[\s\u0000-\u001f\u007f-\u009f]/u.test(value)) return null;
  if (!/^https?:\/\//i.test(value)) return null;
  try {
    return new URL(value).hostname === "" ? null : value;
  } catch {
    return null;
  }
}

/** The project link's releases page, or `""` where the build has no link. */
function releasesPage(): string {
  const project = projectLink();
  return project === null ? "" : `${project.href.replace(/\/+$/, "")}/releases`;
}

// ---- the page's answer ------------------------------------------------------

/** Whether this page has started its one check (or drawn from the kept one). */
let started = false;
/** What this page life learned, before it is compared with the build. */
let found: ReleaseAnswer | null = null;
/** Bumped by the cells' reset, so an answer still in flight from a cell that has
 *  ended cannot settle into the next one. */
let generation = 0;
const wakes = new Set<() => void>();

function subscribe(wake: () => void): () => void {
  wakes.add(wake);
  return () => {
    wakes.delete(wake);
  };
}

function settle(answer: ReleaseAnswer): void {
  found = answer;
  for (const wake of [...wakes]) wake();
}

/** The kept answer, when it is under an hour old and well-formed; `null` means
 *  "check" — absent, unreadable, malformed, stale, or stamped in the future
 *  (a clock set back must not silence the check for longer than an hour). */
function keptAnswer(now: number): ReleaseAnswer | null {
  const raw = readItem(CHECKED_KEY);
  if (raw === null) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (typeof parsed !== "object" || parsed === null) return null;
  const { checkedAt, tag, url } = parsed as Record<string, unknown>;
  if (typeof checkedAt !== "number" || !Number.isFinite(checkedAt)) return null;
  if (typeof tag !== "string" || typeof url !== "string") return null;
  const age = now - checkedAt;
  if (age < 0 || age >= FRESH_MS) return null;
  return { tag, url: httpUrl(url) ?? "" };
}

/** One request to the feed, and `null` for every way it can fail. */
async function ask(fetchImpl: typeof fetch, feed: string): Promise<ReleaseAnswer | null> {
  try {
    const res = await fetchImpl(feed, { cache: "no-store", headers: { accept: "application/vnd.github+json" } });
    if (!res.ok) return null;
    const body: unknown = await res.json();
    if (typeof body !== "object" || body === null) return null;
    const { tag_name: tag, html_url: page } = body as Record<string, unknown>;
    if (typeof tag !== "string") return null;
    return { tag, url: httpUrl(page) ?? releasesPage() };
  } catch {
    return null;
  }
}

/** The page's one check. A no-op after the first call in a page life, and a
 *  no-op where the build has no feed. */
async function start(fetchImpl: typeof fetch): Promise<void> {
  if (started) return;
  const feed = releaseFeedUrl();
  if (feed === null) return;
  started = true;
  const mine = generation;
  const kept = keptAnswer(Date.now());
  if (kept !== null) {
    settle(kept);
    return;
  }
  const answer = await ask(fetchImpl, feed);
  if (answer === null || mine !== generation) return;
  writeItem(CHECKED_KEY, JSON.stringify({ checkedAt: Date.now(), tag: answer.tag, url: answer.url }));
  settle(answer);
}

/**
 * The newer release the feed names, or `null` — nothing before the answer, and
 * nothing for an answer that is equal, older, unparseable or absent. The first
 * hook to mount in a page life starts the check (from the kept answer if it is
 * under an hour old, else from the feed); every later mount, on any screen,
 * reads what that check found.
 *
 * `fetchImpl` is the seam a cell answers through; the first mount's is used.
 */
export function useReleaseNotice(fetchImpl: typeof fetch = fetch): ReleaseAnswer | null {
  const answer = useSyncExternalStore(
    subscribe,
    () => found,
    () => null,
  );
  useEffect(() => {
    void start(fetchImpl);
    // The check is once per page life: a different `fetchImpl` on a later
    // render does not ask again.
  }, []);
  return answer !== null && isNewer(answer.tag, buildVersion()) ? answer : null;
}

/** A fresh page life for the cells: the page's answer and its once-flag cleared,
 *  and any answer still in flight from before dropped. Both are the MODULE's, so
 *  a cell that set either would leak it into the next. */
export function resetReleaseNoticeForTests(): void {
  started = false;
  found = null;
  generation++;
}
