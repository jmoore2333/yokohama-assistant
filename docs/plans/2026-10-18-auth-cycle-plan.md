# The auth cycle — the public door's identity: implementation plan (2026-10-18)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> A dispatched implementer reads its own task, §"Global Constraints", §"Names shared across tasks" and the design's
> §3 rulings; it PROBES every code block and every cited line of its brief before reporting, and a sentence it finds
> false is a `DONE_WITH_CONCERNS` corrected in THIS PLAN first (the S2 runs' rule). Tiers (the owner's R-A13, design §9.4:
> no Fable per task): each task's header names `impl:` sonnet | opus and `review:` opus; the orchestrator (Mythos)
> runs the pre-flight and the after review itself.

**Goal:** the public Worker at `yokohama.moore.nyc` signs players in through WorkOS AuthKit (or any OpenID Connect
issuer, by configuration) instead of Cloudflare Access, keeps them signed in with its own cookie, and archives every
finished browser game — signed in or anonymous — so the site can open to everyone.

**Architecture:** the layer's `worker` door is unchanged; the Worker SHELL gains a second identity source beside
Access (`YOKOHAMA_IDENTITY_SOURCE`), two sign-in arms (`workos`, WorkOS's own first-party API; `oidc`, any discovery
document) that run once at the callback on `/enter`, a stateless HMAC session cookie read on every request, and an
anonymous archive caller formed as a synthetic address the key grammar already accepts. The client learns one new
header and posts a device id. The deploy guard learns a second deployment identity. No Rust crate changes; no new
dependency anywhere.

**Tech Stack:** TypeScript (the Worker shell under wrangler 4.136.3, vitest 4 with `@cloudflare/vitest-plugin` 1.2.3
— "the pool"; the client under Vite, vitest, Playwright), Node 22 for the scripts (vitest `vitest.scripts.config.ts`),
`crypto.subtle` for HMAC and signature checks, `fetch` for the issuer. No npm package is added to `worker/` or `web/`.

**Spec:** `docs/plans/2026-10-18-auth-cycle-design.md` — the design; its §3 rulings R-A1–R-A11 bind every task, its
§4 is the mechanism, its §9 the review record. Behind it `docs/plans/2026-10-08-security-remediation-spec.md` §9.7
(lines 1034–1036) and §6 (the fences), and `docs/web-lane.md` §8 (the public door's preconditions) and §10 (the
window recipes). The plan argues from the design; an implementer reads both.

## Global Constraints

- The design's §8 fences: `SEMANTICS_REV` 7; `crates/yokohama-core`, `-oracle`, `-difftest`, `-bridge`, `gen-data`,
  AND `crates/yokohama-web`, `crates/yokohama-worker`, `crates/yokohama-wasm` byte-identical — this cycle touches no
  Rust. No npm dependency added to `worker/package.json` or `web/package.json`. The review report is never committed.
- One identity source per deployment: `YOKOHAMA_IDENTITY_SOURCE` is `access` | `oidc` | `workos`, required under
  the `worker` door, refused under `open` only if set to anything (ignored absent), no default (design §4.1).
- The session cookie: name `yokohama_session`; `HttpOnly; Secure; SameSite=Lax; Path=/`; 30 days by default
  (`YOKOHAMA_SESSION_DAYS`), renewed when a request finds it past half its life; stateless; HMAC-SHA256 under
  `YOKOHAMA_SESSION_SECRET` (32 bytes or more). The login cookie `yokohama_login`: 10 minutes, same attributes.
- Both halves of the sign-in live on `/enter`; `run_worker_first` stays exactly `["/api/*", "/enter"]` in every
  wrangler file (the guard's check 5). Sign-out is `POST /api/leave`.
- The anonymous caller is the address `anon-<16 lowercase hex>@anon.invalid` (the device) or
  `anon-<game id>@anon.invalid` (no device header; the body's table id, held to `isSafeId`), forwarded as
  `X-Yokohama-User` on the `archive` target only; limited by IP under `CREATE_LIMIT`.
- `mode` on the wire stays `cf-access` under the `worker` door (R2.2-m2); the only wire addition is
  `WWW-Authenticate: Yokohama-Session realm="/enter"` on every 401 under `/api` when the source is not `access`.
- Every address is judged RAW by `forwardable` before any trim, then ASCII-lowercased by `address` (the Kelvin rule,
  `worker/src/identity.ts`); `email_verified` must be exactly `true`.
- Process (the owner's standing rules): implementers write their full report to a transcript file and answer with
  its path and three lines; no `rm -rf`, `rm -r` or bulk `rm -f` in any dispatch; foreground polling only, no waits
  on events, no monitors; a reviewer's mutants run in an EXPORT (`git archive`), never in the implementer's tree;
  fix rounds are new commits; stage with explicit paths, never `git add -A`.
- Per task: `cd worker && npm run typecheck && npm test && npm run test:scripts` and `cd web && npm run typecheck &&
  npm test` must pass before the commit; `npm run build:module:debug` first when a task touches `gen/` (none here).

## Review Focus

Inputs the design implies and no task's happy path exercises; each line's test is added to the task that owns it.

1. A `return_to` that is another origin, a path, or a hash outside the router's grammar must land on `/#/lobby`, never
   be followed (Task 4).
2. An ID token with `email_verified` absent or `false`, or a WorkOS user object with it `false`, must mint no cookie
   and redirect to `/#/lobby?signin=failed` (Tasks 3, 4).
3. A session cookie with one MAC byte changed, a payload with `expires` in the past, or a payload that is not JSON must
   read as no one, in constant time, with no exception reaching `answer` (Task 2).
4. A device header of 15 or 17 hex digits, uppercase hex, or a Kelvin sign must be treated as absent, so the post
   falls to the game id, or to the 401 when the body names no safe id (Task 5).
5. A browser running TODAY's client (no device header, the table document naming `local-<16 hex>`) must land its
   game under `browser/anon-local-<16 hex>_at_anon.invalid-<digest>/` (Task 5); and a renewal `Set-Cookie` never rides
   a 101 or a 401 (Task 4).

## Names shared across tasks

| name | where | value |
|---|---|---|
| `Source` | `worker/src/config.ts` | `"access" \| "oidc" \| "workos"` |
| `ShellConfig.source` | `worker/src/config.ts` | `Source \| null` (`null` under `open`) |
| `ShellConfig.signin` | `worker/src/config.ts` | `{ issuer: string \| null; clientId: string; clientSecret: string \| null } \| null` |
| `ShellConfig.session` | `worker/src/config.ts` | `{ secret: string; days: number } \| null` |
| `SESSION_COOKIE`, `LOGIN_COOKIE` | `worker/src/session.ts` | `"yokohama_session"`, `"yokohama_login"` |
| `Session` | `worker/src/session.ts` | `{ address: string; sid: string \| null; issued: number; expires: number }` |
| `Login` | `worker/src/session.ts` | `{ state: string; nonce: string; verifier: string; returnTo: string; expires: number }` |
| `SignedIn` | `worker/src/signin.ts` | `{ address: string; sid: string \| null }` |
| `SESSION_CHALLENGE` | `worker/src/index.ts` | `'Yokohama-Session realm="/enter"'` |
| `DEVICE_HEADER` | `worker/src/identity.ts` | `"X-Yokohama-Device"` |
| `Identity` kind | `worker/src/identity.ts` | `{ kind: "session"; email: string; sid: string \| null; renew: boolean }` and `{ kind: "anonymous"; device: string }` |
| `ANON_DOMAIN` | `worker/src/identity.ts` | `"anon.invalid"` |
| `SIGN_IN_NEEDED` | `web/src/api.ts` | `"Sign in to play multiplayer."` |
| `DEVICE_KEY` | `web/src/device.ts` | localStorage key `"yokohama.device"` |
| `handle` | `worker/src/index.ts` | `handle(req, env, ctx, keys, fetcher = fetch)` — the fifth parameter is new, optional, the arms' network |

**Plan decision D-1 — ACCEPTED by the owner on 2026-10-18 (R-A12; the design's §4.7 amended in place, §9.4).** The design's §4.7 made the public
Worker a NEW deployment and kept `yokohama-web` private. With R-A6 (`yokohama.moore.nyc` is the public site, the
private one comes later on another domain) the roles invert: `wrangler.jsonc` — the Worker `yokohama-web`, the bucket
`yokohama-archive`, the custom domain — becomes the public deployment by setting `YOKOHAMA_IDENTITY_SOURCE` to
`workos`, and the LATER private Worker is the new file (`wrangler.private.jsonc`, its own name and bucket, source
`access`), written when the owner names its domain. Why: a custom domain binds to one Worker, so a new public Worker
would have to take the domain FROM `yokohama-web` in the window; the existing objects, archive and the owner's
seats stay where they are; and the key space does not change — Access and WorkOS both vouch for verified addresses,
which is the C1 finding's whole concern. The guard's identity table (Task 7) is written for both files, with the
private one's record present and its file optional until it exists.

---

### Task 0: the pre-flight — WorkOS's answer shapes, read live (the orchestrator, with the owner for one sign-in)

**Files:** none in the repository; a scratchpad script; the readings go into this plan's §"Pre-flight record" at
the end, as key SETS, never values.

- [ ] **Step 1:** the owner's WorkOS workspace exists (team `yokohama-assistant`; project `project_01M58WM4WC8XFJJCG7DXR9EJQY`;
  Staging `environment_01M58WM4WYQD0JRC0W78E566ZZ`, client `client_01M58WM57D340A6BXYPBP52F6K`; Production
  `environment_01M58WM6E8GTQH4YVZRCY7SFPV`, client `client_01M58WM6MNRQPY96302487DQF7` — read with the WorkOS MCP's
  `whoami` on 2026-10-18) and the MCP is connected. The orchestrator, through the MCP against STAGING: `query
  redirectUris`, then `mutate setRedirectUris` with the existing list plus `http://localhost:8787/enter` (dry-run
  first); `mutate updateAuthkitSettings` with `isMagicAuthEnabled: true, isGoogleOauthEnabled: true` and nothing
  else changed; `query environmentAppBranding` recorded. The owner pastes the staging API key into the shell
  environment (`WORKOS_API_KEY`; the MCP mints none), never into a file under the repository.
- [ ] **Step 2:** the orchestrator writes `/private/tmp/.../scratchpad/workos-probe.mjs`: prints
  `https://api.workos.com/user_management/authorize?client_id=client_01M58WM57D340A6BXYPBP52F6K&redirect_uri=http://localhost:8787/enter&response_type=code&provider=authkit&state=probe`;
  the owner opens it, signs in with an email code, and pastes back the `code` from the browser's address bar (nothing
  listens on 8787; the navigation fails and the URL keeps the code).
- [ ] **Step 3:** the script POSTs `https://api.workos.com/user_management/authenticate` with
  `{client_id, client_secret: <API key>, grant_type: "authorization_code", code}` as JSON, and prints: the response
  status; the top-level key set; `user`'s key set; whether `user.email_verified` is a boolean and its value; whether
  `access_token` is a three-part JWT and its header's `alg` and `kid` and its payload's key set; whether
  `refresh_token` is present. Then `GET https://api.workos.com/sso/jwks/<client_id>` and prints the key set's `kid`
  list and `alg`s. Then prints the logout URL `https://api.workos.com/user_management/sessions/logout?session_id=<sid>`
  and the owner opens it once to read where it lands.
- [ ] **Step 4:** record every reading in §"Pre-flight record" (key names, booleans, algorithms; no token, no
  address). If `user.email_verified` is absent, or the access token's `sid` is absent, Task 3's `workos` arm is
  corrected IN THIS PLAN before Task 3 is dispatched (design §3 R-A7's fallback stays the SDK at the callback).

---

### Task 1: the identity source and its variables (impl: sonnet · review: opus)

**Files:**
- Modify: `worker/src/env.ts` (the `Env` interface, lines 49–63 region), `worker/src/config.ts` (`ShellConfig`,
  lines 18–33; `shellConfig`, lines 277–300; `openDoorRefusals`, lines 250–264)
- Modify: `worker/wrangler.jsonc`, `worker/wrangler.dev.jsonc`, `worker/wrangler.test.jsonc` (`vars`; the two
  open files are NOT touched)
- Modify: `docs/web-lane.md` §12 (six rows, in the table's own format — line 652's `YOKOHAMA_DEV_DOOR` row is the
  model): `worker/scripts/lane-variables.test.mjs` greps the lane's code for every `YOKOHAMA_*` name and holds §12's
  table to it both ways, so the rows land in THIS task's commit or `npm run test:scripts` goes red
- Test: `worker/src/config.unit.test.ts`; `worker/scripts/lane-variables.test.mjs` (unchanged, must stay green)

**Interfaces:**
- Produces: `export type Source = "access" | "oidc" | "workos"`; `ShellConfig.source: Source | null`;
  `ShellConfig.signin: { issuer: string | null; clientId: string; clientSecret: string | null } | null`;
  `ShellConfig.session: { secret: string; days: number } | null`; `Env.YOKOHAMA_IDENTITY_SOURCE?`,
  `Env.YOKOHAMA_OIDC_ISSUER?`, `Env.YOKOHAMA_OIDC_CLIENT_ID?`, `Env.YOKOHAMA_OIDC_CLIENT_SECRET?`,
  `Env.YOKOHAMA_SESSION_SECRET?`, `Env.YOKOHAMA_SESSION_DAYS?` (all `string | undefined`).

- [ ] **Step 1: Write the failing tests** in `config.unit.test.ts`, with the file's own fixture builder
  `env(vars: Record<string, unknown>, bucket = false): Env` (line 23) — `WORKER_ENV` below stands for the file's valid
  `worker`-door `vars` (the Access pair, the dev door, the archive with `bucket = true`) and `OPEN_ENV` for its valid
  `open`-door `vars`; spell each test with `env({ ...WORKER_ENV, … }, true)`:

```ts
it("the worker door requires a source, with no default", () => {
  expect(shellConfig({ ...WORKER_ENV, YOKOHAMA_IDENTITY_SOURCE: undefined })).toEqual({
    error: "YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos under the worker door — it is not set, and there is no default: who vouches for an address is a deployment decision",
  });
});
it("access keeps today's shape", () => {
  const c = shellConfig({ ...WORKER_ENV, YOKOHAMA_IDENTITY_SOURCE: "access" });
  expect(c).toMatchObject({ source: "access", signin: null, session: null });
});
it("oidc needs the issuer, the client id and the session secret, and ignores the Access pair", () => {
  const env = { ...WORKER_ENV, ACCESS_TEAM_DOMAIN: undefined, ACCESS_AUD: undefined, YOKOHAMA_IDENTITY_SOURCE: "oidc",
    YOKOHAMA_OIDC_ISSUER: "https://issuer.test", YOKOHAMA_OIDC_CLIENT_ID: "client_1", YOKOHAMA_SESSION_SECRET: "s".repeat(32) };
  expect(shellConfig(env)).toMatchObject({ source: "oidc", signin: { issuer: "https://issuer.test", clientId: "client_1", clientSecret: null }, session: { secret: "s".repeat(32), days: 30 } });
  expect(shellConfig({ ...env, YOKOHAMA_OIDC_ISSUER: undefined })).toEqual({ error: "YOKOHAMA_OIDC_ISSUER must be set under YOKOHAMA_IDENTITY_SOURCE=oidc — the issuer whose /.well-known/openid-configuration names the endpoints" });
  expect(shellConfig({ ...env, YOKOHAMA_SESSION_SECRET: "short" })).toEqual({ error: "YOKOHAMA_SESSION_SECRET must be at least 32 bytes — it signs every session cookie (wrangler secret put YOKOHAMA_SESSION_SECRET)" });
  expect(shellConfig({ ...env, YOKOHAMA_SESSION_DAYS: "0" })).toEqual({ error: 'YOKOHAMA_SESSION_DAYS must be a whole number of days above zero — got "0"' });
});
it("workos needs the client id and the API key as the secret, and refuses an issuer", () => {
  const env = { ...WORKER_ENV, ACCESS_TEAM_DOMAIN: undefined, ACCESS_AUD: undefined, YOKOHAMA_IDENTITY_SOURCE: "workos",
    YOKOHAMA_OIDC_CLIENT_ID: "client_1", YOKOHAMA_OIDC_CLIENT_SECRET: "sk_test_x", YOKOHAMA_SESSION_SECRET: "s".repeat(32) };
  expect(shellConfig(env)).toMatchObject({ source: "workos", signin: { issuer: null, clientId: "client_1", clientSecret: "sk_test_x" } });
  expect(shellConfig({ ...env, YOKOHAMA_OIDC_CLIENT_SECRET: undefined })).toEqual({ error: "YOKOHAMA_OIDC_CLIENT_SECRET must be set under YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS API key the code exchange is authenticated with (wrangler secret put YOKOHAMA_OIDC_CLIENT_SECRET)" });
  expect(shellConfig({ ...env, YOKOHAMA_OIDC_ISSUER: "https://x" })).toEqual({ error: "YOKOHAMA_OIDC_ISSUER is set beside YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS arm's issuer is https://api.workos.com and is not configurable" });
});
it("open ignores an absent source and refuses a present one", () => {
  expect(shellConfig({ ...OPEN_ENV, YOKOHAMA_IDENTITY_SOURCE: undefined })).toMatchObject({ source: null, signin: null, session: null });
  expect(shellConfig({ ...OPEN_ENV, YOKOHAMA_IDENTITY_SOURCE: "access" })).toEqual({ error: "YOKOHAMA_IDENTITY_SOURCE is set beside YOKOHAMA_WEB_IDENTITY=open — under the open door a name is a claim and no source vouches for it" });
});
it("a source that is none of the three is refused by value", () => {
  expect(shellConfig({ ...WORKER_ENV, YOKOHAMA_IDENTITY_SOURCE: "Access" })).toEqual({ error: 'YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got "Access"' });
});
```

  Under `access` the Access pair stays required exactly as today (`doorRules` in the guard mirrors it in Task 7); under
  `oidc`/`workos` a SET `ACCESS_TEAM_DOMAIN` or `ACCESS_AUD` is ignored, not refused (a self-hoster may leave them).

- [ ] **Step 2: Run** `cd worker && npx vitest run src/config.unit.test.ts` — Expected: FAIL (`source` undefined; no
  such sentences).
- [ ] **Step 3: Implement.** In `config.ts`: `export type Source = "access" | "oidc" | "workos"`; the three fields on
  `ShellConfig` after `door`; a reader `source(door, raw): Source | null` with the sentences above; `signin(env,
  source)` and `session(env, source)` readers (secret length counted in UTF-8 bytes with `TextEncoder`, as
  `isAddress` counts); `shellConfig` builds them in `ShellConfig`'s field order (the door first, then the source, so
  the first refusal named is the door's); `moduleConfig` is untouched (the module never learns the source). In
  `env.ts` the six keys with one doc line each.
- [ ] **Step 4:** add `"YOKOHAMA_IDENTITY_SOURCE": "access"` to the `vars` of `wrangler.jsonc`, `wrangler.dev.jsonc`
  and `wrangler.test.jsonc`, right after `YOKOHAMA_WEB_IDENTITY` (the two open files do NOT get it — `open` refuses
  a present one). The deploy guard stays green: the dev file still equals production but for the dev door, and the
  guard learns the key's rules in Task 7. Write the six §12 rows (kind `Worker var` for the first two and the days,
  `secret` for the two secrets; the host `the Worker`; the file `worker/src/config.ts`).
- [ ] **Step 5: Run** `cd worker && npm run typecheck && npx vitest run src/config.unit.test.ts && npm test && npm run test:scripts` —
  Expected: all PASS (the pool's `worker` project reads `wrangler.test.jsonc`, now with the source; `lane-variables`
  finds every new name in §12).
- [ ] **Step 6: Commit** `git add worker/src/env.ts worker/src/config.ts worker/src/config.unit.test.ts worker/wrangler.jsonc worker/wrangler.dev.jsonc worker/wrangler.test.jsonc docs/web-lane.md` —
  `feat(worker): YOKOHAMA_IDENTITY_SOURCE and the sign-in and session variables, read by the shell with no default (auth cycle, Task 1)`.

---

### Task 2: the session cookie (impl: opus · review: opus)

**Files:**
- Create: `worker/src/session.ts`
- Test: `worker/src/session.unit.test.ts`

**Interfaces:**
- Produces: `SESSION_COOKIE`, `LOGIN_COOKIE`, `Session`, `Login` (the table above);
  `mintSession(s: Omit<Session, "expires">, days: number, secret: string): Promise<string>` (the cookie VALUE);
  `readSession(cookieHeader: string | null, secret: string, nowSeconds: number): Promise<Session | null>`;
  `renewDue(s: Session, nowSeconds: number, days: number): boolean` (true past half the life);
  `mintLogin(l: Omit<Login, "expires">, nowSeconds: number, secret: string): Promise<string>` (10 minutes);
  `readLogin(cookieHeader: string | null, secret: string, nowSeconds: number): Promise<Login | null>`;
  `setCookie(name: string, value: string, maxAgeSeconds: number): string` → `${name}=${value}; Max-Age=…; Path=/; HttpOnly; Secure; SameSite=Lax`;
  `clearCookie(name: string): string` → the same with `Max-Age=0` and an empty value;
  `cookieValue(cookieHeader: string | null, name: string): string | null`.
- Format: `v1.<base64url(JSON payload)>.<base64url(HMAC-SHA256(secret, "v1." + payload))>`; `issued` and `expires`
  are whole seconds; `address` is already normalized by the caller.

- [ ] **Step 1: Write the failing tests:**

```ts
const SECRET = "0123456789abcdef0123456789abcdef";
const NOW = 1_789_000_000;
it("mints and reads back, with the expiry 30 days out", async () => {
  const v = await mintSession({ address: "red@t.test", sid: "session_01", issued: NOW }, 30, SECRET);
  expect(v.startsWith("v1.")).toBe(true);
  expect(await readSession(`other=1; ${SESSION_COOKIE}=${v}`, SECRET, NOW + 5)).toEqual({ address: "red@t.test", sid: "session_01", issued: NOW, expires: NOW + 30 * 86_400 });
});
it("a changed MAC byte, an expired payload, a non-JSON payload, another secret and a v0 prefix each read as nobody", async () => {
  const v = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, SECRET);
  const [p, body, mac] = v.split(".");
  const flipped = mac[0] === "A" ? "B" : "A";
  for (const bad of [`${p}.${body}.${flipped}${mac.slice(1)}`, `v0.${body}.${mac}`, `${p}.!!!.${mac}`, v.slice(0, -1)]) {
    expect(await readSession(`${SESSION_COOKIE}=${bad}`, SECRET, NOW)).toBeNull();
  }
  expect(await readSession(`${SESSION_COOKIE}=${v}`, "x".repeat(32), NOW)).toBeNull();
  expect(await readSession(`${SESSION_COOKIE}=${v}`, SECRET, NOW + 30 * 86_400 + 1)).toBeNull();
  expect(await readSession(null, SECRET, NOW)).toBeNull();
  expect(await readSession("a=b", SECRET, NOW)).toBeNull();
});
it("renewal is due past half the life and not before", async () => {
  const s = { address: "a@b.test", sid: null, issued: NOW, expires: NOW + 30 * 86_400 };
  expect(renewDue(s, NOW + 15 * 86_400 - 1, 30)).toBe(false);
  expect(renewDue(s, NOW + 15 * 86_400 + 1, 30)).toBe(true);
});
it("the login cookie lives ten minutes and carries the four fields", async () => {
  const v = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: "/#/t/sim-1" }, NOW, SECRET);
  expect(await readLogin(`${LOGIN_COOKIE}=${v}`, SECRET, NOW + 599)).toEqual({ state: "st", nonce: "no", verifier: "ve", returnTo: "/#/t/sim-1", expires: NOW + 600 });
  expect(await readLogin(`${LOGIN_COOKIE}=${v}`, SECRET, NOW + 601)).toBeNull();
});
it("the Set-Cookie lines carry the attributes verbatim", () => {
  expect(setCookie(SESSION_COOKIE, "v", 60)).toBe("yokohama_session=v; Max-Age=60; Path=/; HttpOnly; Secure; SameSite=Lax");
  expect(clearCookie(LOGIN_COOKIE)).toBe("yokohama_login=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax");
});
```

- [ ] **Step 2: Run** `npx vitest run src/session.unit.test.ts` — Expected: FAIL (module missing).
- [ ] **Step 3: Implement `session.ts`.** HMAC via `crypto.subtle.importKey("raw", …, {name:"HMAC", hash:"SHA-256"})`
  and `crypto.subtle.verify` (constant time, so no manual compare); the payload parsed only after the MAC verifies;
  every exception inside `readSession`/`readLogin` is caught and answers `null`; `cookieValue` splits on `;`, trims,
  and takes the first pair whose name matches exactly.
- [ ] **Step 4: Run** the file and `npm run typecheck` — Expected: PASS.
- [ ] **Step 5: Commit** `git add worker/src/session.ts worker/src/session.unit.test.ts` —
  `feat(worker): the Worker's own session and login cookies — HMAC-SHA256, stateless, 30 days renewed past half-life (auth cycle, Task 2)`.

---

### Task 3: the two sign-in arms (impl: opus · review: opus)

**Files:**
- Create: `worker/src/signin.ts`
- Modify: `worker/src/identity.ts` (export the private helpers `bytes` and `json`, lines 39–42, unchanged in body)
- Test: `worker/src/signin.unit.test.ts`

**Interfaces:**
- Consumes: `ShellConfig.signin` (Task 1); `forwardable`, `asciiLower`, `isAddress` from `identity.ts`; a
  `fetcher: typeof fetch`.
- Produces: `SignedIn`; `authorizeUrl(source: Exclude<Source,"access">, signin: NonNullable<ShellConfig["signin"]>, p: { redirectUri: string; state: string; nonce: string; challenge: string | null }, discovery: Discovery | null): string`;
  `exchange(source, signin, p: { code: string; redirectUri: string; verifier: string; nonce: string; ip: string | null; userAgent: string | null }, net: { fetcher: typeof fetch; keys: KeySource; discovery: Discovery | null; nowSeconds: number }): Promise<SignedIn | null>`;
  `discovery(issuer: string, fetcher: typeof fetch): Promise<Discovery>` with `Discovery = { issuer: string; authorization_endpoint: string; token_endpoint: string; jwks_uri: string; end_session_endpoint: string | null }`, cached per isolate exactly as `keySource` caches (one in flight, a failed fetch rejects every joiner and caches nothing);
  `keySourceAt(url: string, fetcher?: typeof fetch, clock?: () => number): KeySource` — `keySource`'s body with the URL a parameter; `keySource(teamDomain, …)` becomes `keySourceAt(`https://${teamDomain}/cdn-cgi/access/certs`, …)` with its tests untouched;
  `verifyIdToken(token: string, keys: JwkSet, c: { iss: string; aud: string; nonce: string; nowSeconds: number }): Promise<SignedIn | null>`;
  `WORKOS_AUTHORIZE = "https://api.workos.com/user_management/authorize"`, `WORKOS_AUTHENTICATE = "https://api.workos.com/user_management/authenticate"`, `WORKOS_JWKS = (clientId) => `https://api.workos.com/sso/jwks/${clientId}``, `WORKOS_LOGOUT = (sid) => `https://api.workos.com/user_management/sessions/logout?session_id=${encodeURIComponent(sid)}``;
  `leaveUrl(source, signedInSid: string | null, discovery: Discovery | null, returnTo: string): string` — `workos`: `WORKOS_LOGOUT(sid)` + `&return_to=` when `sid`; `oidc`: `end_session_endpoint` + `?id_token_hint`-less `post_logout_redirect_uri=` when the document has one; else `returnTo`.

- [ ] **Step 1: Write the failing tests** (the RS256 key pair and signing helper from `identity.unit.test.ts` lines
  18–50, copied into this file — the pool isolates per file and a shared helper module is what the house avoids):

```ts
describe("verifyIdToken", () => {
  it("accepts RS256 and ES256 tokens with iss, aud, exp, nonce and a verified email, lowercased in ASCII", …);
  // sign { iss: "https://issuer.test", aud: "client_1", exp: NOW+60, nonce: "n1", email: "Red@T.test", email_verified: true, sid: "s1" }
  // expect { address: "red@t.test", sid: "s1" }
  it("refuses: wrong iss, aud missing the client, exp past, nonce mismatch, email_verified false or absent, a Kelvin-sign email, alg none, kid unknown, bad signature", …);
  // each a null; the Kelvin case: "Kevin@x.test" → null (forwardable refuses it RAW)
});
describe("authorizeUrl", () => {
  it("workos: the five parameters and provider=authkit, no nonce, no code_challenge", () => {
    const u = new URL(authorizeUrl("workos", { issuer: null, clientId: "c", clientSecret: "k" }, { redirectUri: "https://h/enter", state: "st", nonce: "no", challenge: null }, null));
    expect(u.origin + u.pathname).toBe(WORKOS_AUTHORIZE);
    expect(Object.fromEntries(u.searchParams)).toEqual({ client_id: "c", redirect_uri: "https://h/enter", response_type: "code", provider: "authkit", state: "st" });
  });
  it("oidc: the discovery document's endpoint with scope, nonce, and code_challenge S256 only when given", …);
  // { client_id, redirect_uri, response_type: "code", scope: "openid email", state, nonce, code_challenge, code_challenge_method: "S256" }
});
describe("exchange", () => {
  it("workos: POSTs the JSON body with client_secret and returns the user's lowercased address and the access token's sid", async () => {
    const fetcher = recording(200, { user: { email: "Red@T.test", email_verified: true }, access_token: await sign({ sid: "session_01", exp: NOW + 60 }), refresh_token: "r" });
    expect(await exchange("workos", SIGNIN, { code: "c1", redirectUri: "https://h/enter", verifier: "", nonce: "", ip: "1.2.3.4", userAgent: "ua" }, { fetcher, keys, discovery: null, nowSeconds: NOW })).toEqual({ address: "red@t.test", sid: "session_01" });
    expect(fetcher.calls[0]).toMatchObject({ url: WORKOS_AUTHENTICATE, method: "POST", body: { client_id: "c", client_secret: "k", grant_type: "authorization_code", code: "c1", ip_address: "1.2.3.4", user_agent: "ua" } });
  });
  it("workos: email_verified false, a non-200, a body that is not JSON, or a user with no email each answer null", …);
  it("workos: an access token whose signature fails the JWKS still answers the address, with sid null", …); // belt and braces, never a refusal of a TLS-authenticated answer
  it("oidc: POSTs form-encoded grant_type=authorization_code with client_secret_post, or with code_verifier and no secret, and verifies the id_token", …);
  it("oidc: a token response with no id_token answers null", …);
});
describe("discovery", () => {
  it("fetches /.well-known/openid-configuration once per isolate and rejects every joiner on a non-200", …);
});
```

  `recording(status, body)` is a test fetcher returning `Response.json(body, {status})` and recording
  `{url, method, body}` of each call.

- [ ] **Step 2: Run** `npx vitest run src/signin.unit.test.ts src/identity.unit.test.ts` — Expected: FAIL (module
  missing); identity's own suite PASS.
- [ ] **Step 3: Implement `signin.ts`.** `verifyIdToken` is `resolveIdentity`'s verification steps (lines 112–139)
  restated for a token in hand: `alg` RS256 (`RSASSA-PKCS1-v1_5`) or ES256 (`ECDSA` with `P-256`, `SHA-256`), string
  `kid`, the key found, the signature, `iss` exact, `aud` as string or array containing the client id, `exp > now`,
  `nbf <= now + 60` when present, `nonce` exact, `email_verified === true`, `email` through `forwardable` then
  `asciiLower(trim)`, `isAddress`; `sid` a string or `null`. The `workos` exchange POSTs JSON, reads `user.email` and
  `user.email_verified` the same way, and reads `sid` from the access token's payload only after `verifyJwtSignature`
  (the signature half of the above, no claims) passes against `WORKOS_JWKS(clientId)` — on failure, `sid` is `null`
  and the address stands. The `oidc` exchange POSTs `application/x-www-form-urlencoded` with `grant_type`, `code`,
  `redirect_uri`, `client_id`, and either `client_secret` or `code_verifier`. Every network or parse failure answers
  `null` after `log("signin-failed", { source, step, error })` (no body, no token in the log).
- [ ] **Step 4: Run** both files and `npm run typecheck` — Expected: PASS; `identity.unit.test.ts` unchanged and PASS.
- [ ] **Step 5: Commit** `git add worker/src/signin.ts worker/src/signin.unit.test.ts worker/src/identity.ts` —
  `feat(worker): the workos and oidc sign-in arms — authorize URL, code exchange, ID token verification, discovery and a key source by URL (auth cycle, Task 3)`.

---

### Task 4: `/enter`'s two halves, the session on every request, sign-out and the 401 header (impl: opus · review: opus)

**Files:**
- Modify: `worker/src/route.ts` (the `Target` union, lines 15–35; `route`, line 82), `worker/src/index.ts`
  (`handle`, line 108; `answer`, lines 198–262; `withApiHeaders`, lines 100–105; `identify`, line 286),
  `worker/src/identity.ts` (the `Identity` union, lines 13–22; `callerName`, lines 165–176)
- Test: `worker/src/route.unit.test.ts`, `worker/test/signin.test.ts` (new pool file)

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: `Target` gains `{ kind: "leave" }` for `POST /api/leave` (`GET`/`HEAD` on it: `not-allowed`; any other
  method: `not-allowed`); `Identity` gains `{ kind: "session"; email: string; sid: string | null; renew: boolean }`,
  `callerName` answers its `email`; `handle(req, env, ctx, keys, fetcher = fetch)`;
  `SESSION_CHALLENGE = 'Yokohama-Session realm="/enter"'`; `withApiHeaders(res, challenge: boolean)` adds
  `WWW-Authenticate: <SESSION_CHALLENGE>` when `challenge` and `res.status === 401`.

- [ ] **Step 1: Write the failing tests.** In `route.unit.test.ts`: `POST /api/leave` → `{kind:"leave"}`;
  `GET /api/leave` → `{kind:"not-allowed"}`; `POST /api/leaver` → `{kind:"house"}`. In `test/signin.test.ts` (pool,
  `worker` project, modelled on `test/door.test.ts` line 49: `handle(req, sent.env ?? env, createExecutionContext(),
  keys)`; every `handle` call here passes an env override `{ ...env, YOKOHAMA_IDENTITY_SOURCE: "workos",
  YOKOHAMA_OIDC_CLIENT_ID: "c", YOKOHAMA_OIDC_CLIENT_SECRET: "k", YOKOHAMA_SESSION_SECRET: SECRET,
  ACCESS_TEAM_DOMAIN: undefined, ACCESS_AUD: undefined }` and a recording fetcher as the fifth argument; the pool's
  own `wrangler.test.jsonc` stays `access`, so the file's "under access" cells pass the plain `env`):

```ts
it("GET /enter starts the sign-in: a 302 to WorkOS carrying state, a login cookie, and return_to kept", async () => {
  const res = await handle(get("/enter?return_to=%2F%23%2Ft%2Fsim-1"), ENV, ctx, keys, fetcher);
  expect(res.status).toBe(302);
  const to = new URL(res.headers.get("Location")!);
  expect(to.origin + to.pathname).toBe(WORKOS_AUTHORIZE);
  const login = await readLogin(res.headers.get("Set-Cookie"), SECRET, NOW);
  expect(login).toMatchObject({ state: to.searchParams.get("state"), returnTo: "/#/t/sim-1" });
});
it("a return_to that is another origin, a plain path, or an unsafe hash becomes /#/lobby", …);
// "https://evil.test/#/t/x", "/api/me", "/#/t/../x", "#/t/sim-1" (no slash), "" → returnTo "/#/lobby"
it("GET /enter?code&state completes it: the exchange runs once, the session cookie is set, the login cookie cleared, and the 302 lands on return_to", async () => {
  // mint a login cookie for state "st"; fetcher answers the WorkOS exchange with a verified user
  expect(res.status).toBe(302); expect(res.headers.get("Location")).toBe("/#/t/sim-1");
  const cookies = res.headers.getSetCookie();
  expect(cookies.some((c) => c.startsWith("yokohama_session=v1."))).toBe(true);
  expect(cookies.some((c) => c.startsWith("yokohama_login=; Max-Age=0"))).toBe(true);
  expect(fetcher.calls.filter((c) => c.url === WORKOS_AUTHENTICATE)).toHaveLength(1);
});
it("a state mismatch, a missing login cookie, or an exchange answering nobody lands on /#/lobby?signin=failed with no session cookie and no exchange where the state failed", …);
it("GET /enter under access stays the 302 to /#/lobby, code or no code, with no cookie", …);
it("a session cookie names the caller on /api/me, with no header and no issuer call", async () => {
  const v = await mintSession({ address: "red@t.test", sid: "s1", issued: NOW }, 30, SECRET);
  const res = await handle(get("/api/me", { cookie: `yokohama_session=${v}` }), ENV, ctx, keys, fetcher);
  expect(res.status).toBe(200);
  expect(await res.json()).toEqual({ mode: "cf-access", user: "red@t.test" });
  expect(fetcher.calls).toHaveLength(0);
});
it("no cookie is the layer's 401 with the session challenge header; under access the header is absent", async () => {
  const res = await handle(get("/api/me"), ENV, ctx, keys, fetcher);
  expect(res.status).toBe(401);
  expect(res.headers.get("WWW-Authenticate")).toBe('Yokohama-Session realm="/enter"');
  const under = await handle(get("/api/me"), env /* access, as wrangler.test.jsonc has it */, ctx, keys, fetcher);
  expect(under.status).toBe(401); expect(under.headers.has("WWW-Authenticate")).toBe(false);
});
it("a cookie past half its life is renewed on a 200, never on a 401, and never on a 101", …);
// mint with issued = NOW - 16 days; /api/me → Set-Cookie yokohama_session=v1.… with Max-Age=2592000; a bad-MAC cookie → 401 and no Set-Cookie; a stream upgrade with the cookie → 101 and no Set-Cookie
it("POST /api/leave clears the cookie and answers the leave URL; under access it answers Access's logout and clears nothing", async () => {
  const res = await handle(post("/api/leave", { cookie: `yokohama_session=${v}`, origin: ORIGIN }), ENV, ctx, keys, fetcher);
  expect(res.status).toBe(200);
  expect(await res.json()).toEqual({ leave: "https://api.workos.com/user_management/sessions/logout?session_id=s1&return_to=https%3A%2F%2F…%2F" });
  expect(res.headers.get("Set-Cookie")).toBe("yokohama_session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax");
  const under = await handle(post("/api/leave", { origin: ORIGIN }), env, ctx, keys, fetcher);
  expect(await under.json()).toEqual({ leave: "/cdn-cgi/access/logout" }); expect(under.headers.has("Set-Cookie")).toBe(false);
});
it("POST /api/leave from another site is the cross-site 403 before anything else", …);
```

- [ ] **Step 2: Run** `npx vitest run src/route.unit.test.ts test/signin.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement.** In `answer`, after `shellConfig`: when `target.kind === "enter"` and
  `config.source !== "access"` and `config.source !== null`, branch on `url.searchParams.has("code")`: the START draws
  `state`, `nonce` and `verifier` (32 random bytes each, base64url; `challenge` = base64url(SHA-256(verifier)) under
  `oidc` with no secret, else `null`), computes `returnTo` by `safeReturnTo(url.searchParams.get("return_to"))`
  (accepts exactly `/#/` followed by a route `parseRoute` in `web/src/route.ts` would accept — restated as the
  regex `^/#/(lobby|new|new/local|t/[A-Za-z0-9_-]{1,64})$`; else `/#/lobby`), answers 302 to `authorizeUrl(...)` with
  `Set-Cookie: setCookie(LOGIN_COOKIE, await mintLogin(...), 600)`; the CALLBACK reads `readLogin`, compares `state`
  in constant time (`crypto.subtle.timingSafeEqual` on equal-length buffers, else false), calls `exchange(...)` with
  `ip = req.headers.get("cf-connecting-ip")` and `userAgent = req.headers.get("user-agent")`, and on success answers
  302 `returnTo` with `Set-Cookie` for the session (Max-Age `days*86400`) and the login cookie cleared; on any failure
  302 `/#/lobby?signin=failed` with the login cookie cleared and no session cookie. Under `access`, `enter` stays
  line 203's 302. The identity: when `config.door === "worker"` and `config.source !== "access"`, `identity` is
  `readSession(req.headers.get("cookie"), …)` mapped to `{ kind: "session", email, sid, renew: renewDue(...) }`, else
  `NO_ONE`; `identify` (the Access path) runs only under `access`. `leave`: a target under `/api`, so the cross-site
  check (line 212) runs first; then `{ leave }` JSON by `leaveUrl` with `returnTo = ${url.origin}/`, the session cookie
  cleared when the source is not `access`, the Access logout path under `access`. Renewal: when the identity is a
  `session` with `renew` and the final response's status is 2xx and not 101, append `Set-Cookie` with a fresh mint.
  `withApiHeaders(res, challenge)`: `challenge = config.source !== null && config.source !== "access"`, read in
  `handle` from `shellConfig(env)` (a second call is cheap and pure; or thread it from `answer` by returning the
  pair — the implementer's choice, stated in the report). The 405 for `GET /api/leave` is `not-allowed`'s existing
  empty 405. **The arms' key sources** live beside the Access `keys` closure the default export holds ("ONE key
  source per isolate, made at the first request", `index.ts` line 445): a module-level `Map<string, KeySource>` from
  JWKS URL to `keySourceAt(url, fetcher)`, so a forged `kid` costs one fetch a minute per URL exactly as it does for
  Access; the `workos` arm's URL is `WORKOS_JWKS(clientId)`, the `oidc` arm's the discovery document's `jwks_uri`.
- [ ] **Step 4: Run** `npm run typecheck && npm test` — Expected: PASS, every existing pool cell included (the
  `worker` project's env is `access`; nothing moves for it).
- [ ] **Step 5: Commit** `git add worker/src/route.ts worker/src/index.ts worker/src/identity.ts worker/src/route.unit.test.ts worker/test/signin.test.ts` —
  `feat(worker): /enter starts and completes the sign-in, the session cookie names the caller, POST /api/leave, and the Yokohama-Session challenge on 401s (auth cycle, Task 4)`.

---

### Task 5: the anonymous archive (impl: opus · review: opus)

**Files:**
- Modify: `worker/src/identity.ts` (the `Identity` union; `callerName`), `worker/src/index.ts` (`answer`, the
  identity and the forward, lines 220–231; `objectFor`'s `archive` arm, lines 420–439), `worker/src/limits.ts`
  (`limitKey`, lines 29–35; `limiterFor`, lines 39–42; `limited`, line 50), `worker/src/archive.ts` (`Incoming`,
  lines 20–27; `fetch`, lines 79–100; `save`, lines 146–156), `worker/src/r2.ts` (`executePlan`, lines 138–163)
- Test: `worker/src/identity.unit.test.ts`, `worker/src/limits.unit.test.ts`, `worker/test/archive.test.ts`

**Interfaces:**
- Consumes: `createId` (`index.ts` line 362) generalized to `bodyId(body)` — the same function, exported.
- Produces: `DEVICE_HEADER = "X-Yokohama-Device"`; `ANON_DOMAIN = "anon.invalid"`; `isDeviceId(s: string): boolean`
  (exactly `/^[0-9a-f]{16}$/`); `Identity` gains `{ kind: "anonymous"; device: string }` where `device` is the 16 hex
  or a safe game id; `callerName` answers `anon-${device}@anon.invalid`; `anonymousIdentity(headers: Headers, body:
  Uint8Array | null): Identity` — the header if `isDeviceId`, else the body's id if `isSafeId`, else `NO_ONE`;
  `limitKey` answers the IP key for `anonymous`; `limiterFor(method, target, limits, anonymous: boolean)` sends an
  anonymous `archive` POST to `limits.create` with kind `create`; `executePlan(bucket, plan, meta?: Record<string,
  string>)` puts both keys with `{ customMetadata: meta }` when given; `Incoming.device: string | null`.

- [ ] **Step 1: Write the failing tests.** `identity.unit.test.ts`: `callerName({kind:"anonymous", device:
  "0123456789abcdef"})` → `"anon-0123456789abcdef@anon.invalid"`; `anonymousIdentity` with headers
  `{"X-Yokohama-Device": "0123456789ABCDEF"}` (uppercase), `"0123456789abcde"` (15), `"0123456789abcdef0"` (17),
  `"K123456789abcde"` → falls to the body: body `{"id":"local-aaaaaaaaaaaaaaaa"}` → device `local-aaaaaaaaaaaaaaaa`;
  body `{"id":"../x"}` or no body → `{kind:"none"}`. `limits.unit.test.ts`: `limitKey("worker", anonymous, "9.9.9.9",
  "create")` → `"ip:9.9.9.9"` (never the caller tag); `limiterFor("POST", {kind:"archive"}, limits, true)` →
  `{limiter: limits.create, kind: "create"}`; with `false` → the write limiter as today. `test/archive.test.ts` (the
  pool, the `worker` project, env `access` as it is): the anonymous cells go THROUGH THE SHELL — `handle(req, env,
  createExecutionContext(), keys)` as `test/worker.test.ts` line 79 drives it — because the anonymous identity is
  formed in `answer`; the file's `save(caller, body)` helper (line 197) reaches the archive object directly and
  serves the signed-in variant. `LOCAL_PAIR` is the body the file's browser-save cells post (a `{record, table}` pair
  whose table id is `LOCAL`) and `SAVE_STATUS` the status those cells assert — read them, do not guess:

```ts
it("a caller-less post with a device header lands under the device's anonymous prefix with the metadata", async () => {
  const res = await handle(post("/api/archive", LOCAL_PAIR, { "X-Yokohama-Device": "0123456789abcdef", "content-type": "application/json" }), env, createExecutionContext(), keys);
  expect(res.status).toBe(SAVE_STATUS /* the status the browser-save cells assert */);
  const seg = await segmentOf("anon-0123456789abcdef@anon.invalid");
  const head = await env.ARCHIVE.head(`browser/${seg}/rev7/over/${LOCAL}.json`);
  expect(head?.customMetadata).toEqual({ caller: "anonymous", device: "0123456789abcdef", rev: "7", why: "over", uploaded: expect.stringMatching(/^\d+$/) });
});
it("a caller-less post with NO device header files under the game's own id", async () => {
  // body's table id LOCAL (local-…): the key is browser/<segmentOf(`anon-${LOCAL}@anon.invalid`)>/rev7/over/<LOCAL>.json; metadata.device === "none"
});
it("a signed-in post keeps the caller's prefix and carries the device in the metadata", …);
// dev-door credential + X-Yokohama-Claimed-User red@t.test + X-Yokohama-Device → test/browser/<seg>/…; metadata { caller: "red@t.test", device: "0123456789abcdef", … }
it("a caller-less post whose body names no safe id is the layer's 401", …);
it("an anonymous post spends the create limiter by IP", …);
// a fake create limiter counting keys: two posts from cf-connecting-ip 1.1.1.1 with different device headers count ONE key "ip:1.1.1.1"; the write limiter sees nothing
it("the anonymous caller never reaches a table route", …);
// POST /api/games with X-Yokohama-Device and no caller → the layer's 401, and no X-Yokohama-User forwarded (read the house's answer as today's no-caller cell does)
```

- [ ] **Step 2: Run** `npx vitest run src/identity.unit.test.ts src/limits.unit.test.ts test/archive.test.ts` —
  Expected: FAIL on the new cells.
- [ ] **Step 3: Implement.** In `answer`, after the identity is resolved and only when `target.kind === "archive"`
  and `callerName(identity) === undefined`: `identity = anonymousIdentity(req.headers, body)` (the header read BEFORE
  the strip loop, which drops every `x-yokohama-*`). After the strip: when `target.kind === "archive"` and the raw
  device header `isDeviceId`, `headers.set(DEVICE_HEADER, raw)` so the object reads it. `objectFor` is unchanged:
  `callerName` of the anonymous kind is the synthetic address, within `ADDRESS_MAX_BYTES`. In `archive.ts`,
  `Incoming.device = req.headers.get(DEVICE_HEADER)` when `isDeviceId`, else `null`; `save` passes
  `executePlan(bucket, plan, { caller: callerOf(name) is anonymous (starts with "anon-" and ends with "@anon.invalid") ? "anonymous" : callerOf(name), device: incoming.device ?? "none", rev: <from plan.record_key, the "rev<N>" segment's digits>, why: <the "over"|"abandoned" segment>, uploaded: String(Math.floor(at / 1000)) })`.
  `rev` and `why` parse off `plan.record_key` by splitting on `/` (the layout `<prefix>rev<N>/<why>/<id>.json`,
  `archive::paths`); a key that does not parse answers no metadata and logs `metadata-skipped` rather than failing
  the upload.
- [ ] **Step 4: Run** `npm run typecheck && npm test` — Expected: PASS.
- [ ] **Step 5: Commit** `git add worker/src/identity.ts worker/src/index.ts worker/src/limits.ts worker/src/archive.ts worker/src/r2.ts worker/src/identity.unit.test.ts worker/src/limits.unit.test.ts worker/test/archive.test.ts` —
  `feat(worker): the anonymous archive — a caller-less upload files under anon-<device|game>@anon.invalid, limited by IP, with metadata on the object (auth cycle, Task 5)`.

---

### Task 6: the client — the device header, rule 6b, the sign-in refusal and sign-out (impl: opus · review: opus)

**Files:**
- Create: `web/src/device.ts`
- Modify: `web/src/buildValues.ts` (a sixth build value, `signInNotice(): string | null`, read from
  `VITE_YOKOHAMA_SIGNIN_NOTICE` exactly as `siteNotice()` reads its own, line 32), `web/src/FrontDoor.tsx` (the
  multiplayer card, lines 159–175: the notice under the `/enter` link when set), `docs/web-lane.md` §12 (ONE row,
  `YOKOHAMA_SIGNIN_NOTICE`, kind *build value*: the lane-variables cell requires it in this commit), `web/scripts/`
  (wherever the five build values are stamped into `build-values.json` — read `presence.mjs`/`precache.mjs`; the
  notice joins them as the sixth, so the guard's stamp cell and the README's build-values list move together)
- Modify: `web/src/api.ts` (`isAccessChallenge`/`ACCESS_CHALLENGE`, lines 326–350; `call`'s 401 handling, lines
  129–176; `api.archive`, line 1014; the `api` object), `web/src/outbox.ts` (`namelessUnderKey`, line 512;
  `noteGameOver`, lines 700–728; `attempt`, lines 450–500; `reconsiderAll`, line 863; the head comment, rule 6b),
  `web/src/Refusal.tsx`, `web/src/whoami.ts` (`failureOf`, `signOut`), `web/src/OnThisDevice.tsx` (`Remedy`, lines
  472–500: the link gains `?return_to=`), `web/src/types.ts` (an `ArchiveReply` unchanged; a `LeaveReply = { leave: string }`)
- Test: `web/src/device.test.ts`, `web/src/api.test.ts`, `web/src/outbox.test.ts`, `web/src/Refusal.test.tsx`,
  `web/src/whoami.test.ts` (the existing files, extended)

**Interfaces:**
- Consumes: the Worker's `WWW-Authenticate: Yokohama-Session realm="/enter"` (Task 4), `X-Yokohama-Device` (Task 5),
  `POST /api/leave` → `{ leave }` (Task 4).
- Produces: `deviceId(): string` — 16 lowercase hex from `crypto.getRandomValues(new Uint8Array(8))`, kept under
  `localStorage["yokohama.device"]`, drawn once, re-drawn when the stored value is not 16 hex, and a per-page value
  when storage throws (a private window) — **the plan's correction of the design's §4.4 "the outbox's IndexedDB
  store": the `META` store is keyed by GAME id and swept by `forgetGame` and the backups, so a device row there would
  be swept or exported; localStorage is per origin and survives republishes, which is all the id needs**;
  `isSessionChallenge(res: Response): boolean` (`WWW-Authenticate` starts with `Yokohama-Session`);
  `SIGN_IN_NEEDED = "Sign in to play multiplayer."` (**the plan's wording, replacing the design's §4.3 "This table is
  multiplayer. Sign in to take your seat.": one sentence serves the table link, the lobby and every refusal strip,
  and the strip is where the player reads it**); `isSessionExpired(e)` true for either sentence;
  `api.leave(): Promise<LeaveReply>`; `api.archive` sends `X-Yokohama-Device: deviceId()`;
  `signInHref(): string` → `/enter?return_to=${encodeURIComponent(location.hash || "/#/lobby")}` with the hash
  prefixed `/` (so `#/t/x` → `/#/t/x`); **the transparency notice (R-A14)**: `signInNotice()` drawn as a `<p
  className="muted">` directly under every sign-in link `Refusal` and `Remedy` draw and under the front door's
  multiplayer link, nothing drawn when the value is unset (a self-host with no provider), so a player reads who signs
  them in and what is kept before they click. The public build's text is Task 10's.

- [ ] **Step 1: Write the failing tests.**

```ts
// device.test.ts
it("draws sixteen lowercase hex once and keeps it", …); // two calls equal; matches /^[0-9a-f]{16}$/; localStorage holds it
it("re-draws a stored value that is not sixteen hex, and survives a storage that throws", …);
// api.test.ts
it("a 401 with WWW-Authenticate Yokohama-Session throws ApiError(SIGN_IN_NEEDED, 401); Cloudflare-Access still throws ACCESS_CHALLENGE; a bare 401 throws the body's sentence", …);
it("api.archive sends X-Yokohama-Device with the device id", …);
it("api.leave POSTs /api/leave and returns the body", …);
// outbox.test.ts
it("rule 6b holds a nameless game for a name ONLY when the server facts say open", …);
// keyed build + facts.identity "open" + no name → row "signed-out", no post; keyed build + facts.identity "cf-access" + no name → the post is made (the device header present); keyed build + facts unread → the post is made
it("a 401 carrying the session challenge parks the row as signed-out, whose remedy is the door", …);
// Refusal.test.tsx
it("draws the /enter link beside SIGN_IN_NEEDED, with return_to the current hash", …);
// whoami.test.ts
it("failureOf reads SIGN_IN_NEEDED as a challenge", …);
it("signOut POSTs /api/leave and navigates to the answer's leave", …);
```

- [ ] **Step 2: Run** `cd web && npx vitest run src/device.test.ts src/api.test.ts src/outbox.test.ts src/Refusal.test.tsx src/whoami.test.ts` — Expected: FAIL on the new cells.
- [ ] **Step 3: Implement.** `namelessUnderKey` becomes `heldForAName = () => Boolean(turnstileSiteKey()) &&
  serverFacts().identity === "open" && claimedUser() === null` and the head comment's rule 6b is rewritten to say
  so (and why: a keyed build under `worker` is the public deployment since the auth cycle, design §4.5). `call`
  checks `isSessionChallenge` beside `isAccessChallenge` and throws `new ApiError(SIGN_IN_NEEDED, 401)`. `Refusal`
  draws `<a href={signInHref()}>Sign in</a>` beside `SIGN_IN_NEEDED` and keeps "Sign in again" beside
  `ACCESS_CHALLENGE`; the `Remedy`'s link becomes `signInHref()`. `whoami.signOut` calls `api.leave()` then
  `window.location.assign(reply.leave)`; its failure path keeps today's behaviour (clear the name, re-ask).
- [ ] **Step 4: Run** `cd web && npm run typecheck && npm test` — Expected: PASS (the `Refusal.test.tsx` directory
  scan still finds no stray strip).
- [ ] **Step 5: Commit** the listed files —
  `feat(web): the device id on archive posts, rule 6b held to the open door, the sign-in refusal with its link, and sign-out through /api/leave (auth cycle, Task 6)`.

---

### Task 7: the deploy guard's second identity, the keyed pairing and the deployable files (impl: opus · review: opus)

**Files:**
- Modify: `worker/scripts/deploy-guard.mjs` (`doorRules`, lines 472–515; `SECRETS` (the map `secretsInVars`
  reads); `turnstileConsistency`, lines 637–697; `configFailures`, lines 717–802; `DOORS`/`doorLine`, lines 397–527;
  the identity constants, lines 273–302), `worker/scripts/deploy-guard.test.mjs`, `worker/wrangler.jsonc` (the
  source becomes `workos` ONLY at the window, Task 10 — this task leaves it `access`), new
  `worker/wrangler.private.jsonc` (D-1: the later private Worker, source `access`, name `yokohama-web-private`,
  bucket `yokohama-archive-private`, rate-limit namespaces `7303`/`7304`, NO `routes` yet — the owner names the
  domain later; `workers_dev: false`), `worker/package.json` (`deploy:private` script), `web/src/outbox.ts` (none),
  the client's `check_build_stamp` (none — the self-host does not move)
- Test: `worker/scripts/deploy-guard.test.mjs`

**Interfaces:**
- Produces: `const DEPLOYMENTS = { "wrangler.jsonc": { name: "yokohama-web", bucket: "yokohama-archive", namespaces: ["7301", "7302"], routes: ["yokohama.moore.nyc"] }, "wrangler.private.jsonc": { name: "yokohama-web-private", bucket: "yokohama-archive-private", namespaces: ["7303", "7304"], routes: null } }`;
  `SOURCES = { access: "Cloudflare Access, verified by the shell", oidc: "an OpenID Connect issuer, verified by the shell at the callback", workos: "WorkOS AuthKit, through its first-party API at the callback" }`;
  `sourceOf(value)`; `doorLine` prints `… deploys the worker door, source <source> — <SOURCES[source]>`.

- [ ] **Step 1: Write the failing tests** (the file's fixture style: a temp root with the five files copied and one
  edited):

```js
it("a worker-door file names a source, with no default", …); // remove YOKOHAMA_IDENTITY_SOURCE from wrangler.jsonc → one failure naming it
it("access requires the Access pair; oidc requires the issuer and client id and forbids the pair; workos requires the client id and forbids the issuer", …);
it("the two secrets are refused in any vars block", …); // YOKOHAMA_OIDC_CLIENT_SECRET, YOKOHAMA_SESSION_SECRET in vars → failures naming them as secrets
it("a keyed build is accepted under worker with source oidc or workos, and refused under worker with access, and on an open file as before", …);
// check 12: wrangler.jsonc with YOKOHAMA_TURNSTILE on + source workos + a keyed stamp → no failure; + source access → the existing KEYED_IS_OPEN sentence reworded to name the source
it("wrangler.private.jsonc is held to its own record: name, bucket, namespaces, source access, no route, and production's other blocks", …);
// change its name → failure; give it production's bucket → failure; add a route → failure "no route until the owner names the private domain"
it("the dev file still differs from production in the dev door alone, with the source copied", …);
it("doorLine names the source", …);
```

- [ ] **Step 2: Run** `cd worker && npm run test:scripts` — Expected: FAIL on the new cells.
- [ ] **Step 3: Implement.** `doorRules`: read `sourceOf`; under `worker` require one of the three with Task 1's
  sentence; per source the variable rules (Task 1's table), refusing `ACCESS_*` under `oidc`/`workos` only when a
  value is non-empty (the shell ignores them; the guard, which writes the deploy, refuses the confusion). `SECRETS`
  gains the two names with `"a secret — wrangler secret put <name>; never a var"`. `turnstileConsistency`: `open =
  door === "open" || (door === "worker" && (source === "oidc" || source === "workos"))` for the identity rule, with
  the sentence `KEYED_IS_OPEN` reworded: "a keyed build is the open door or a provider-signed worker door (R-FP4,
  narrowed by the auth cycle's R-A3/R-A4): under Access the edge is the bot defence". `configFailures`: a judged file
  in `DEPLOYMENTS` is held to its record (name, bucket, namespaces, routes) and to production's `assets`,
  `observability`, `durable_objects` and `migrations` blocks; `wrangler.private.jsonc` is optional (absent is not a
  failure) and never a sibling of production. `DEV_DIFFERS` and `OPEN_DIFFERS` unchanged except `OPEN_DIFFERS`
  gains `YOKOHAMA_IDENTITY_SOURCE: undefined`. `package.json`: `"deploy:private": "node scripts/deploy-guard.mjs --config wrangler.private.jsonc && node scripts/secrets-guard.mjs --config wrangler.private.jsonc && wrangler deploy --config wrangler.private.jsonc"`.
- [ ] **Step 4: Run** `npm run test:scripts && node scripts/deploy-guard.mjs --config wrangler.jsonc && node scripts/deploy-guard.mjs --config wrangler.dev.jsonc && node scripts/deploy-guard.mjs --config wrangler.open.jsonc && node scripts/deploy-guard.mjs --config wrangler.private.jsonc` — Expected: PASS; each guard run prints its door line and exits 0 (an assembled tree may be required by checks 2, 9, 11, 12 — run `npm run build` first if so, as the window does).
- [ ] **Step 5: Commit** `git add worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/wrangler.private.jsonc worker/package.json` —
  `feat(worker): the deploy guard reads the identity source, holds two deployment identities and accepts a keyed build under a provider-signed worker door (auth cycle, Task 7)`.

---

### Task 8: the minted test session and the live instruments (impl: sonnet · review: opus)

**Files:**
- Create: `worker/scripts/mint-session.mjs`, `worker/scripts/mint-session.test.mjs`
- Modify: `worker/scripts/dev-vars.mjs` (`KEYS = ["CF-Access-Client-Id", "CF-Access-Client-Secret"]`, line 24: the
  credential's headers, which the replay sends with `X-Yokohama-Claimed-User`, `live-replay.mjs` line 28),
  `worker/scripts/live-replay.mjs`, `web/e2e/live.ts` (line 137 returns the claimed header for the live context)
- Test: `worker/scripts/mint-session.test.mjs` (new), `worker/scripts/live-replay.test.mjs`, `web/e2e/harness.node.test.ts` (extended)

**Interfaces:**
- Produces: `node scripts/mint-session.mjs <address> [--days N]` prints `yokohama_session=v1.…` for `.test`
  addresses ONLY, reading `YOKOHAMA_SESSION_SECRET` from the environment, exit 2 with
  `mint-session: only a .test address may be minted — got <address>` otherwise, exit 2 with
  `mint-session: YOKOHAMA_SESSION_SECRET is not set` when absent; the instruments read `YOKOHAMA_SESSION_COOKIE`
  from the environment and, when set, send it as `Cookie` INSTEAD of the Access credential headers (both set is a
  refusal: `… both YOKOHAMA_SESSION_COOKIE and the service credential are set — one identity per run`).
- The script is `.mjs` run by plain `node`, so it does not import `session.ts`: it re-implements the mint with
  `node:crypto`'s `createHmac("sha256", secret)` over the same `v1.<payload>` bytes, and `mint-session.test.mjs`
  (vitest, which transpiles) imports BOTH and pins them byte-identical on one fixed payload — the cross-check cell,
  so the two can never drift unseen.

- [ ] **Step 1: Write the failing tests:** the `.test` fence; the missing secret; the cross-check against
  `session.ts` (dynamic `import("../src/session.ts")` under vitest, which transpiles); `live-replay` with
  `YOKOHAMA_SESSION_COOKIE` set sends `Cookie` and no `CF-Access-Client-Id`; with both set, refuses.
- [ ] **Step 2: Run** `npm run test:scripts` — Expected: FAIL.
- [ ] **Step 3: Implement** as the interfaces say.
- [ ] **Step 4: Run** `npm run test:scripts` — Expected: PASS.
- [ ] **Step 5: Commit** —
  `feat(worker): a minted .test session for the live instruments on a provider-signed Worker (auth cycle, Task 8)`.

---

### Task 9: the documents (impl: sonnet · review: opus)

**Files:**
- Modify: `docs/web-lane.md` (§12's six rows were Task 1's; here §8: a paragraph under "Still owed before the door
  opens for real" moving the provider items to "met by the auth cycle"; §4: one bullet, "three identity sources
  under the worker door"; §15:
  W-3's bullet gains the sentence that the anonymous upload no longer depends on an update), `worker/RUNBOOK.md`
  (§3's table: the six rows; a new §23 "The public door opens — the switch to WorkOS" RECIPE: the WorkOS production
  environment's three URLs, the methods on and passwords off, the two secrets put, the keyed build, the edge rule
  ENABLED by its id FIRST, `npm run deploy` with `wrangler.jsonc` at `workos`, the hostname OFF the Access application
  LAST, the readings, the rollback = the previous `wrangler.jsonc` redeployed, the hostname back on Access, the rule
  disabled), `worker/README.md` (§1.5 "The two doors" → the two doors and the worker door's three sources, one
  paragraph), `deploy/README.md` (§1: a sentence that the Worker may also run `oidc` against any issuer; the NAS
  stays `cf-access`), `docs/security-ledger.md` (the C-tier rows' cycle column where the auth cycle met them;
  a row `AUTH` noting the anonymous archive's caps), `docs/plans/2026-10-18-auth-cycle-design.md` (§9.4 appended:
  this plan's D-1 and the device-store correction), `CLAUDE.md` (ONE standing-rule sentence in the browser-lane
  paragraph: the three sources, one per deployment, the cookie the Worker's, both halves on `/enter`).
- Test: `cd worker && npm run test:scripts` (the `lane-variables` cell still green; the §3 runbook rows are not
  pinned by a test and are read by the reviewer against `config.ts`).

- [ ] **Step 1:** write the paragraphs and the recipe; the recipe's order is the design's §4.7 and the lane's §10.
- [ ] **Step 2: Run** `cd worker && npm run test:scripts` — Expected: PASS.
- [ ] **Step 3: Commit** —
  `docs: the auth cycle's variables, the switch recipe, the sources and the standing rule (auth cycle, Task 9)`.

---

### Task 10: the window — the public door opens (the orchestrator and the owner; no implementer)

- [ ] **Step 1 — the after review first** (the orchestrator's own, over an export of the branch head): every task's
  cells re-run; the mutation battery (§"Mutants" below) each reverted; the per-edit suites and the lane's rows
  (`docs/web-lane.md` §2); the roster is OWED because the workspace version moves (Step 2).
- [ ] **Step 2 — the version**: `0.7.0` (the lane's §7 rule: the identity model changes); the bump commit; the
  fifteen release gates measured as one sequence and recorded in the close.
- [ ] **Step 3 — WorkOS production** (the orchestrator through the MCP against `environment_01M58WM6E8GTQH4YVZRCY7SFPV`,
  the owner at the keyboard for what the MCP cannot do): `setRedirectUris` → `https://yokohama.moore.nyc/enter`;
  `updateAuthkitExternalLoginUri` → `https://yokohama.moore.nyc/enter`; the Sign-out URI `https://yokohama.moore.nyc/`
  (the dashboard's Redirects page if no operation names it — `list_operations` with "logout"/"sign-out" first);
  `updateAuthkitSettings` → `isMagicAuthEnabled`, `isGoogleOauthEnabled`, `isGithubOauthEnabled`,
  `isAppleOauthEnabled`, `isPasskeyAuthEnabled` true, `isDiscordOauthEnabled` true if the owner wants it (the MCP's
  settings list it, correcting the design's "no Discord"; the provider apps for Google/GitHub/Apple/Discord need the
  owner's credentials in the dashboard first), `isPasswordAuthEnabled` false, `allowSignUp` true, `maxSessionTime`
  and `inactivityTimeout` at their long settings (the Worker's cookie governs ours; these govern the silent re-auth),
  `appHomepageUrl` `https://yokohama.moore.nyc/`; **branding (R-A14)**: `updateAppBranding` with `displayName`
  "Yokohama", `theme` `System`, the light and dark page/button/link colours from the client's palette, `radius`,
  `authkitSignInHeadingText`, `authkitSignInLegalText` (the transparency sentence: the verified address is kept to
  seat you; finished games are archived), `authkitSignInPrivacyPolicyUrl` to the repository's privacy page once it
  exists — and the logos by hand in the dashboard (`Upload` fields are not settable over the MCP). The client id
  into `wrangler.jsonc`'s `YOKOHAMA_OIDC_CLIENT_ID`, the API key `wrangler secret put YOKOHAMA_OIDC_CLIENT_SECRET`,
  `openssl rand -base64 48 | wrangler secret put YOKOHAMA_SESSION_SECRET`.
- [ ] **Step 4 — the build**: `cd web && YOKOHAMA_TURNSTILE_SITEKEY=<key> YOKOHAMA_SITE_URL=https://yokohama.moore.nyc YOKOHAMA_SIGNIN_NOTICE="Sign-in is handled by WorkOS. This site keeps your verified email address to seat you at your tables, and archives finished games for the computer player." … npm run build`
  (the keyed build, as the F window's recipe lists the five values, plus the notice), `cd worker && npm run build`.
- [ ] **Step 5 — `wrangler.jsonc`**: `YOKOHAMA_IDENTITY_SOURCE: "workos"`, `YOKOHAMA_TURNSTILE: "on"`, the Access pair
  REMOVED, the client id set; committed as the window's commit (the guard accepts it: Task 7).
- [ ] **Step 6 — the order** (the lane's §10): the edge rule ENABLED by its id `dc4c5d3e986144649617cbbaea58cd8a`;
  `npm run deploy`; the owner signs in from a fresh profile (the readings: the overlay, the WorkOS page, the
  redirect back, `/api/me` naming them, a table dealt, a WebSocket opened under the cookie, two invocations for the
  sign-in in the logs, and the console clean of any CSP line at every level — the design's §4.3 claim that the header
  set does not move, read live); THEN the hostname `yokohama.moore.nyc` taken off the Access application (the NAS and CLI
  hostnames stay); a second fresh profile reads the whole path with no Access anywhere; a third profile, with
  today's RELEASED client still installed and a finished local game waiting, opens the site and the game lands
  anonymously under the game's id (the first real reading of R-A11); the friend's game, when they visit.
- [ ] **Step 7 — the readings** into `worker/RUNBOOK.md` §23 (the record), the ledger's live column, the lane's §1
  and §8; rollback bullets beside them.
- [ ] **Step 8 — the close**: the design's §9.5 (the window), the spec's §9.9 (this cycle's rulings, decisions and
  corrections verbatim), guidingpath's row and current state, the board's Track 4, the memory.

## Mutants (the after review's battery; each must turn a named cell red, then is reverted)

| mutant | the cell that reads red |
|---|---|
| `readSession` compares the MAC with `===` on strings | Task 2 "a changed MAC byte …" (still red: the flipped byte) — and the review notes constant time is `crypto.subtle.verify`'s, not a cell's |
| `verifyIdToken` accepts `email_verified` absent | Task 3 "refuses … email_verified false or absent" |
| `exchange("workos")` reads `email` without `forwardable` | Task 3 the Kelvin case |
| `safeReturnTo` accepts any string starting with `/` | Task 4 "a return_to that is another origin …" |
| the renewal `Set-Cookie` is appended on every 2xx including 101 | Task 4 "never on a 101" |
| `withApiHeaders` sets the challenge under `access` | Task 4 "under access the header is absent" |
| `anonymousIdentity` is formed on every target | Task 5 "never reaches a table route" |
| `limitKey` keys an anonymous caller by its tag | Task 5 "spends the create limiter by IP" |
| `isDeviceId` accepts uppercase | Task 5 the uppercase case |
| `heldForAName` ignores the facts | Task 6 "rule 6b holds … ONLY when the server facts say open" |
| the guard accepts a keyed build under `access` | Task 7 check-12 cell |
| `mint-session.mjs` mints a non-`.test` address | Task 8 the fence |

## Pre-flight record

- 2026-10-18 (this plan's hand-off): the WorkOS workspace read through the MCP — team `yokohama-assistant`, role
  ADMIN; Staging `environment_01M58WM4WYQD0JRC0W78E566ZZ` / `client_01M58WM57D340A6BXYPBP52F6K`; Production
  `environment_01M58WM6E8GTQH4YVZRCY7SFPV` / `client_01M58WM6MNRQPY96302487DQF7`. `updateAuthkitSettings` lists
  `isDiscordOauthEnabled` (Discord is available; the design's table said otherwise), `maxSessionTime` and
  `inactivityTimeout`; `updateAppBranding` sets colours, theme, radius, headings, legal text and policy URLs over
  the MCP and takes logos only through the dashboard; no operation mints an environment API key.
- 2026-10-18, Task 0 Step 1 (the orchestrator, over the MCP against Staging): `redirectUris` was empty;
  `setRedirectUris` dry-run then real registered `http://localhost:8787/enter` as the default
  (`redir_01M58XC7GKW2EY2SJH5WDXQ6SK`); `updateAuthkitSettings` set `isMagicAuthEnabled` true and `isGoogleOauthEnabled`
  true (Google was already on; magic auth was off; password auth on, email verification required, Apple, GitHub and
  Microsoft on — none touched; `isEmailVerificationRequired` is account-gated and was not passed). `environmentAppBranding`
  reads every field null. The default application is `app_01M58WM5BMEFEDZ4635C479PH1` with one never-used API key,
  `maxSessionTime` 31536000, `inactivityTimeout` 172800, `accessTokenExpiry` 300. The MCP's name for the redirect-URI
  operation is `setRedirectUris` (`setAuthkitApplicationRedirectUris` targets an application by id; the environment form
  targets the default application), and `updateAuthkitSettings` answers as `updateUserlandSettings`.
- 2026-10-18, Task 0 Steps 2–3 (the owner's one sign-in against Staging, the probe run in the owner's own shell; key
  SETS only): `POST /user_management/authenticate` answered 200 `application/json` with the top-level keys
  `access_token`, `authentication_method`, `refresh_token`, `user` (no `organization_id`); `authentication_method` was
  `GoogleOAuth` (the owner signed in with Google rather than the email code — the shape is the same). `user` carries
  `created_at`, `email`, `email_verified`, `external_id`, `first_name`, `id`, `last_name`, `last_sign_in_at`, `locale`,
  `name`, `object`, `profile_picture_url`, `updated_at`; `user.email_verified` is a BOOLEAN and read `true`; `user.email`
  is a string. `access_token` is a three-part JWT, header `alg` `RS256` with a string `kid`, payload keys `auth_time`
  (number), `client_id`, `exp` (number), `iat` (number), `iss`, `jti`, `sid` (string), `sub` — NO `email` claim, so the
  address is `user.email`'s as Task 3 reads it. `refresh_token` present. `GET /sso/jwks/<client_id>` answered 200
  `{ keys: [ { kid, alg: "RS256", kty: "RSA", use: "sig" } ] }` — one key. **Task 0 Step 4's gate: `user.email_verified`
  present and the access token's `sid` present — Task 3's `workos` arm stands unchanged.** The logout URL
  `…/user_management/sessions/logout?session_id=<sid>` opened once by the owner (no `return_to`, the staging
  environment's `appHomepageUrl` null) landed on `https://error.workos.com/user_management/app-homepage-url-not-found`,
  a generic WorkOS error page: so `leaveUrl`'s `return_to` (Task 3) is load-bearing, and Task 10 sets `appHomepageUrl`
  on production as its Step 3 already says. The staging API key was pasted into the orchestrating session's
  transcript rather than the shell alone; the owner retires it at the wrap (Task 10 Step 8).
- 2026-10-18, the pool at Task 1's head (`4a727698`) after `npm run build:module:debug`: 13 files, 252 tests passed
  (the seven cells that failed in the implementer's run were the stale release module, C-5 below).

## Review record

(The plan review's corrections, each task's `DONE_WITH_CONCERNS` and its correction in this plan, the after review.)

- **2026-10-18, the orchestrator's pre-flight scan, correction C-1 (Task 4):** `/api/me` with no caller answers 200
  `{"mode":"cf-access","user":null}` today (`worker/test/worker.test.ts` lines 409–410), never a 401, so Task 4's cells
  "no cookie is the layer's 401 with the session challenge header; under access the header is absent" and the renewal
  cell's "a bad-MAC cookie → 401 and no Set-Cookie" cannot be read on `/api/me`. Those cells use a request that answers
  401 with no caller: `POST /api/archive` with the body `{"record":"","table":""}` (it names no safe id, so it stays
  the house's 401 after Task 5's anonymous arm) or the `/api/stream` upgrade with no cookie (401 unupgraded). `/api/me`
  serves the "names the caller" cell and the renewal-on-200 cell only. The mutant row "`withApiHeaders` sets the
  challenge under `access`" reads red on the same archive post under the plain `access` env.
- **Pre-flight note (Task 4):** `answer` answers `enter`'s 302 before `shellConfig(env)` today (`index.ts` lines 203
  and 210); the new branch reads the config first, so the `enter` arm moves below it — under `access` the 302 is
  unchanged, and a misconfigured deployment answers the config 500 on `/enter` as it does under `/api`.
- **Pre-flight note (Task 6):** the five build values are stamped by `web/scripts/build-values.mjs` (the task text
  names `presence.mjs`/`precache.mjs` as the files to read; they consume it). Line citations elsewhere drift by a few
  lines; every brief's "probe every cited line" covers it.
- **2026-10-18, Task 1's `DONE_WITH_CONCERNS` (commit `4a727698`), four corrections:**
  - **C-2 (Task 1 Step 4 and Task 7 Step 3):** "the deploy guard stays green … the two open files do NOT get it" was
    half true: the guard holds each open file to production on every `vars` key not in `OPEN_DIFFERS`, so production
    gaining the key turned two `deploy-guard.test.mjs` cells red. The line Task 7 Step 3 assigns — `OPEN_DIFFERS` gains
    `YOKOHAMA_IDENTITY_SOURCE: undefined` — landed in Task 1's commit instead (one line in
    `worker/scripts/deploy-guard.mjs`, no guard cell of its own); Task 7 adds the cell and the rest of the rules.
  - **C-3 (every task's `npx vitest run src/<file>.unit.test.ts` line):** the `src/*.unit.test.ts` files run under the
    scripts config, not the pool — the command is `npx vitest run -c vitest.scripts.config.ts src/<file>.unit.test.ts`
    (a bare `npx vitest run src/…` matches nothing). Tasks 2, 3, 4 and 5 read their Step 2/4 lines so.
  - **C-4 (Task 1 Step 1):** `WORKER_ENV`/`OPEN_ENV` exist under no name; the file's `env(vars, bucket)` builder now
    defaults `YOKOHAMA_IDENTITY_SOURCE` to `access` for a worker-door fixture unless `vars` names the key (`undefined`
    included) or the door is `open`, so every older cell keeps its meaning; `marker.unit.test.ts`'s typed `ShellConfig`
    literal and the pool's open-door cell in `test/worker.test.ts` (the pool env now carries `access`) moved with it.
  - **C-5 (the Global Constraints' per-task line):** the pool runs over the DEBUG module in `worker/gen/`; a tree whose
    `gen/` holds a release build (the last window's) fails seven cells in `factory`, `sockets` and `table` with
    "the pool runs over the DEBUG module" at HEAD, before any change. `npm run build:module:debug` (cargo, run by the
    orchestrator once at the cycle's start) is the remedy, after which no task of this cycle touches `gen/`.
- **2026-10-18, Task 2's `DONE_WITH_CONCERNS` (commit `f653a288`) and the harness's security review of it:**
  - **C-6 (the Global Constraints' cookie line, Task 2, Task 4's cell strings, Task 8's printed line, Task 9's docs) —
    the orchestrator's ruling R-14:** both cookies take the `__Host-` prefix — `SESSION_COOKIE = "__Host-yokohama_session"`,
    `LOGIN_COOKIE = "__Host-yokohama_login"` — with the attributes unchanged (`Secure; Path=/`, no `Domain`: exactly what
    the prefix requires), so a browser refuses the cookie when a sibling subdomain sets it (cookie tossing / session
    fixation, flagged by the harness's background security review at the commit; `moore.nyc` carries the NAS and CLI
    hosts, a self-hoster's domain whatever else it runs). A `Secure` cookie is already dropped over plain http, so the
    prefix adds no failure mode; localhost is a secure context. First-match parsing stays: under `__Host-` a duplicate
    name cannot arrive from another origin or path. Every later cell reads the name through the constants.
  - **C-7 (Task 2 Step 1, cell 2):** `const [p, body, mac] = v.split(".")` does not typecheck under the project's
    `noUncheckedIndexedAccess` (TS18048, `mac` possibly undefined); the cell carries the cast `as [string, string,
    string]`. Vitest alone passes it (esbuild does not typecheck); `npm run typecheck` is what catches it.
  - **C-8 (the Mutants table, row 1):** a `readSession` that compares the MAC with `===` on strings still REFUSES a
    flipped byte, so "a changed MAC byte …" stays green under that mutant — the row's parenthetical had it backwards.
    The cell that reads red is the task's added spy cell, "the MAC is `crypto.subtle.verify`'s, and the payload is
    parsed only once it verifies" (it asserts `verify` is called and `sign` never is on the read path, and that no
    `JSON.parse` of the payload happens under a flipped MAC). The after review's battery names that cell.
  - **Pre-flight note (Task 4):** `Headers.get("Set-Cookie")` joins two lines with `", "`, so
    `readLogin(res.headers.get("Set-Cookie"), …)` holds for the START answer (one cookie) only; the callback's cells,
    which read two cookies, use `res.headers.getSetCookie()` (measured by the Task 2 implementer).
  - **Recorded choice (Task 2, ruling R-15):** `renewDue` is "the life LEFT is less than half the CONFIGURED days"
    (`s.expires - now < days * 86400 / 2`; exactly half is not due) — the brief's "past half the life" when the
    cookie's life equals `days`, and after a change of `YOKOHAMA_SESSION_DAYS` every cookie is still due throughout
    the last half of today's days before its expiry (read by the life SPENT instead, a cookie minted under fewer days
    than half of today's would expire before it ever came due — measured). Two cells pin it.
- **2026-10-18, before Task 3 — C-9 (Task 3 Step 1):** "the RS256 key pair and signing helper from `identity.unit.test.ts`
  lines 18–50, copied into this file — the pool isolates per file and a shared helper module is what the house avoids"
  is false on both counts: `identity.unit.test.ts` IMPORTS its signer from the shared `worker/test/tokens.ts` (`signer(kid)`,
  an RS256 pair; `b64url`; `personClaims`; `credentialClaims`), and a shared helper is the house's pattern. Task 3's
  unit file imports from `../test/tokens` and ADDS an ES256 signer there (`P-256`, `ECDSA`/`SHA-256`, the `alg`/`kid`
  header shape the RS256 signer has) for its ES256 cell; nothing is copied. The pre-flight scan's R-2 is withdrawn.
- **2026-10-18, Task 2's review, ruling R-16 (the design's §4.2 sentence, for Task 9):** `mintSession` derives `expires`
  from `issued`, so a renewal re-stamps the time and a stolen cookie lives as long as it is used — only rotating the
  secret ends it. This is R-A2 as the owner accepted it (no session store, renewed on use, rotation the emergency
  revoke), so the behaviour STANDS and the module doc says it plainly ("… and renewal extends it, so only rotating the
  secret ends a stolen session"); Task 9 aligns the design's §4.2 "a stolen one is good until it expires" in place,
  marked and dated, and appends the ruling to §9.4. No absolute session cap this cycle: the payload keeps no sign-in
  time, so a cap would be a `v2` form after Task 8 pins `v1`. Task 2 closed at `5954b8ee` (review clean after one fix
  round; two minors deferred to the after review: the MAC-length check pre-empts the padded/spaced MAC rows, so two
  hand-signed BODY rows are owed to keep the strict decoder's coverage; one doc phrase).
- **2026-10-18, Task 3's `DONE_WITH_CONCERNS` (commit `b72269a4`), rulings and corrections:**
  - **R-18 (C-1; Task 3's `leaveUrl`, Task 4's caller):** the `oidc` leave URL carries `client_id` beside
    `post_logout_redirect_uri` — RP-Initiated Logout 1.0 §3 lets an issuer refuse the redirect when neither
    `id_token_hint` nor `client_id` names the client, and the design chose no `id_token_hint`. `leaveUrl` gains a fifth
    parameter, `clientId: string`, used by the `oidc` shape only: `<end_session_endpoint>?post_logout_redirect_uri=<enc
    returnTo>&client_id=<enc clientId>` (`&` when the endpoint has a query). The `workos` shape is unchanged.
  - **R-19 (C-2; `discovery`):** the document's `issuer` must equal the configured issuer (Discovery 1.0 §4.3), trailing
    slashes ignored; a mismatch rejects with a sentence naming both, caches nothing.
  - **R-20 (C-3):** `client_secret_post` and PKCE are the two client authentications this cycle supports; an issuer that
    accepts only `client_secret_basic` is out of scope, and the self-hoster documents (Task 9, `worker/README.md`,
    `deploy/README.md`) say so in one sentence.
  - **R-21 (C-4; `verifyIdToken`):** when `azp` is present it must be a string equal to the client id, and when `aud` is
    an array of more than one audience `azp` must be present (Core 1.0 §3.1.3.7); one cell each.
  - **R-22 (C-5; every commit):** a `Co-Authored-By` trailer names the model that authored the commit — an
    implementer's harness names its own model, the orchestrator's its own. `b72269a4` (Opus 5.5) stands; Tasks 1–2's
    trailers (Mythos 5.1, as their dispatches said) stand as reported SHAs; later dispatches say "the line your own
    attribution reminder names".
  - **C-10 (the Mutants table, row 3):** "`exchange("workos")` reads `email` without `forwardable` → Task 3 the Kelvin
    case" — the brief's own cells hold the Kelvin case under `verifyIdToken` only; the cell that reads red is the task's
    added `workos: a user.email carrying a non-ASCII character answers null (judged raw by forwardable)`. The after
    review's battery names it.
  - **C-11 (Task 3 Step 3's "restated"):** `verifyIdToken` is STRICTER than `resolveIdentity`, which never applies
    `isAddress` to a person's `email`; the arm's `isAddress` is the right rule under the `worker` door (the layer admits
    addresses only) and stands.
  - **Carried to Task 4 (N-1):** the discovery cache is a module-level map keyed by issuer whose FIRST caller's fetcher
    serves it for the isolate's life, so a pool cell under `oidc` names an issuer no other cell in its file uses (the
    plan's Task 4 cells run `workos`). The implementer's `keySourceAt` error sentences now name the URL ("the key set at
    <url> answered <status>", "the answer at <url> holds no key set"); `identity.unit.test.ts` is untouched and green.
  - **Observed, pre-existing (for the after review):** the pool prints two `uncaught exception … a reply the shell cannot
    read` lines from the deliberate throw at `test/worker.test.ts:660`, measured identical at the task's base.
- **2026-10-18, Task 3's review (at `1642ba7d`), two rulings and one correction for Task 4:**
  - **R-24 (the reviewer's m-2; `loadDiscovery`):** the issuer and the four endpoints of a discovery document must be
    `https:` URLs with no fragment — RFC 6749 §3.1–§3.2 require TLS on the authorization and token endpoints and forbid
    a fragment, and an `http:` issuer would let a network attacker serve a document of its own with its own `jwks_uri`
    — EXCEPT that `http:` is accepted when the host is loopback (`localhost`, `127.0.0.1`, `[::1]`), the development
    issuer. A document failing the rule rejects with a sentence naming the field and nothing is cached. The
    `YOKOHAMA_OIDC_ISSUER` variable itself stays as Task 1 reads it: the document check covers the issuer URL at the
    first fetch, and Task 9's self-hoster documents say "https, or loopback for development".
  - **R-25 (the reviewer's m-3; Task 4):** the PKCE decision has ONE site — `answer`'s start half derives the challenge
    from `config.signin.clientSecret === null` (a challenge with a secret configured would make the issuer refuse every
    sign-in, silently fail-closed) and a Task 4 cell pins both shapes (secret → no `code_challenge`; no secret →
    `code_challenge` + `S256`, and the verifier sent at the exchange).
  - **C-12 (Task 4 Step 3's `safeReturnTo`; ruling R-23):** the regex `^/#/(lobby|new|new/local|t/[A-Za-z0-9_-]{1,64})$`
    misstates the client's `parseRoute`: a table hash is `/t/<one segment, no "/">` whose id passes `isSafeId`
    (`^[A-Za-z0-9][A-Za-z0-9._-]*$`, at most 64, reserved suffixes refused), which the Worker exports from `route.ts`.
    `safeReturnTo` accepts exactly `/#/lobby`, `/#/new`, `/#/new/local` and `/#/t/<seg>` where the RAW segment passes
    `isSafeId` (a percent-encoded segment lands on the lobby: `%` is outside the grammar); anything else → `/#/lobby`.
  - For Task 4's review (the reviewer's m-4b): `keySourceAt`'s doc commits to "ONE closure per isolate and URL"; Task 4's
    module-level map is what makes it true for the sign-in URLs.
- **2026-10-18, Task 3 closed at `98d589ba`** (review clean after one fix round; one nit deferred: a 136-column doc
  line at `signin.ts:361`). **C-13 (Task 4 Step 1, the pool cells' clock):** the shell reads the REAL clock
  (`Math.floor(Date.now() / 1000)`, `index.ts` line 223) and the pool's Access tokens are signed at the real clock
  (`test/worker.test.ts` line 31), while the brief's cells mint session cookies at the fixed `NOW` (1 789 000 000,
  September 2026) — a cookie that is already expired at a real clock, so every "names the caller" cell would read 401.
  Task 4's pool cells mint with `issued: Math.floor(Date.now() / 1000)`, the half-life cell with `issued = now - 16
  days`, and the login cookie with the real `nowSeconds`; the fixed `NOW` stays in the UNIT files, which inject the
  clock.
