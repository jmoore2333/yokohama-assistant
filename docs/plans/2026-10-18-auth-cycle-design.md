# The auth cycle — the public door's identity: design (2026-10-18)

> The brainstorm's record and the design a planner builds from. Cut on `security-auth` from master `73f9ba8e`,
> stacked under nothing (the owner's order of 2026-10-12: the auth cycle, then S3 stacked on it, then S4). It binds
> beside the specification; nothing here re-opens a ruling the specification carries. The plan that follows it is
> written against this document and transcribes its rulings; the plan review probes every mechanism named here.

## 0. Read order and sources

1. `docs/plans/2026-10-08-security-remediation-spec.md` — §9.7's two closing paragraphs (lines 1034–1036),
   "S4's identity input, revised on the owner's words" and "The order from here, and the auth cycle's place": the
   provider model, one model per deployment, Access kept as an operating provider, the shell-side work list, and
   the cycle's slot. §6 (lines 124–130), the fences. §1, R-SEC4–R-SEC6, R-SEC8, R-SEC10.
2. `docs/handover.md` — Track 4's "Next session: the AUTH CYCLE's brainstorm" sentence (line 119).
3. `docs/web-lane.md` — §8 "The public door's preconditions" (lines 347–417): the two vetted windows, the
   precondition table, "Still owed before the door opens for real"; §12 every variable; §15 what is by design.
4. The code this design changes or leans on: `worker/src/identity.ts` (the Access verifier, `resolveIdentity`,
   lines 112–146; `keySource`, lines 218–236), `worker/src/index.ts` (`answer`, lines 198–262: route, origin and
   cross-site checks, identity, the limiter, the forward; the archive arm of `objectFor`, lines 420–439;
   `withApiHeaders`, lines 100–105), `worker/src/limits.ts` (`limitKey` and `limiterFor`, lines 30–42),
   `worker/src/config.ts` (`YOKOHAMA_WEB_IDENTITY` `worker` | `open`, lines 113–129; `YOKOHAMA_WEB_ARCHIVE`
   `r2` | absent, lines 139–140), `crates/yokohama-worker/src/config.rs` (`wire_name`, lines 105–115: the wire
   says `cf-access` under the `worker` door, R2.2-m2), `worker/scripts/deploy-guard.mjs` (its fourteen checks;
   check 12, the keyed-build pairing, lines 619–678), `web/src/swRoute.ts` (`isServerPath`, lines 48–56: the
   service worker bypasses exactly `/enter`, `/api`, `/api/…`, `/cdn-cgi`, `/cdn-cgi/…`), `web/src/outbox.ts`
   (rules 1–6c at the head; `attempt`, lines 450–500; `reconsiderAll`, lines 863–912), `web/src/api.ts`
   (`isAccessChallenge` and `ACCESS_CHALLENGE`, lines 326–350; `useIdentityMode`, lines 604–612),
   `web/src/OnThisDevice.tsx` (`Remedy`, lines 472–500), `web/src/FrontDoor.tsx` (the three exits and `/enter`,
   lines 141–257), `web/src/local/db.ts` (the `META` store, line 198), `crates/yokohama-web/src/identity.rs`
   (the three modes and the mode rule, lines 1–60; `User::parse`, the ONE key grammar, lines 140–176),
   `crates/yokohama-web/src/archive.rs` (`paths`, line 183; `caller_segment`, line 234).
5. The brainstorm itself: this session's transcript; the owner's words are quoted in §3 where they were given.

## 1. Intent and the why

The repository goes public within the week, and the game goes online for everyone with it. Today every host sits
behind Cloudflare Access, whose identity is a verified address the Worker reads from an RS256 assertion. Access is
not a consumer identity provider: its free plan seats 50 users and a one-time PIN goes only to addresses a policy
already lists. The open door the project built for self-hosters and LAN play takes a name as a claim, and a public
site where "anyone can type in any email" is, in the owner's words, "not tenable for very long".

What identity protects on the public site is one thing: continuity in multiplayer. The owner's description of the
stranger's first two minutes: "They arrive on the page, just as I would today and they see the home page. They have
the opportunity to play locally, against the computer player — or they have the option to enter into multiplayer (or
they click a link to the GitHub to look around). If they play in solo mode, that runs locally in their browser, no
need for identity, it just loads and they play and that's it. If they want to play multiplayer, this is where they may
not play nonstop, they may come and go, they may want to invite others or play with others, use a password, get
someone from the lobby — and they would expect when they come back, they would be picking up their own game and that
others aren't playing it for them." The layer's `worker` door already keys a seat to a verified address; this cycle
changes who vouches for the address, and nothing about what the address buys.

Two things the brainstorm found that the record did not know:

- **There is no anonymous upload.** The owner believed a finished solo game reached R2 whether or not the player was
  signed in. It does not: the Worker's archive arm sends a caller-less post to the house, whose dispatch answers the
  layer's 401 (`worker/src/index.ts` lines 420–439), and the client's outbox parks a nameless game as "not signed in"
  under every door (`web/src/outbox.ts`, rules 6b and the open-door rule, lines 476–496). The bucket confirms it: 72
  objects, and the only personal prefixes are the owner's own and a test reader's. A friend's finished solo game
  (`local-03bf781aa067cf0a`), played signed in to Access, is not there — the Access session is 24 hours since S2, a
  solo game makes no server call to keep it alive, and the post at the game's end met Access's 401. The owner: "the
  anonymous upload path is important, this must have been punted, there isn't a reason to gate that, and we want to
  gather save game data". This cycle builds it.
- **The session length is the Worker's, not the provider's.** The Worker verifies the provider's token once at the
  callback and mints its own first-party cookie; Clerk's fixed 7 days, Auth0's 3-day idle cap and the rest govern
  only how often a silent redirect to the provider succeeds without a click.

## 2. Goals, non-goals, anti-goals

**Goals**

- A stranger reaches multiplayer through the Turnstile and a consumer sign-in with the least friction the owner
  accepts: an email code, Google, GitHub or Apple, passkeys where the browser offers them, no passwords.
- A returning player stays signed in for weeks on the same browser; a shared table link lands on a page that says
  multiplayer needs a sign-in and carries them back to the table afterwards; a player already signed in just continues.
- The Worker is invoked by the sign-in exactly twice (the redirect and the callback) and never per request after; the
  solo page stays on static assets; a finished solo game costs one invocation, its upload, signed in or not.
- The shell's identity source is a configuration a self-hoster can point at any OpenID Connect issuer by three
  variables, and Access stays an operating source on the private Worker and the NAS unchanged.
- Every finished browser game lands in the archive under the same strict replay, with the verified address when
  there was one and anonymously when there was not, each under a share with a ceiling.

**Non-goals**

- No change to the layer's doors (`open`, `cf-access`, `worker`) beyond admitting the anonymous key class (§4.4);
  no change to the seat rule, the chat, the lobby, the operator surface or the NAS deployment.
- No accounts of our own: no profile, no display-name registry, no password, no email of ours. The provider holds the
  user; the Worker holds an address and a session.
- No per-request call to the provider, no provider SDK in the Worker or the client, no embedded login widget.
- The public split's other work (the fresh history, the document curation, the art) is S4's, not this cycle's.

**Anti-goals** (a failure even if it works)

- A sign-in that adds an invocation per page view, or a provider script on the page: the front door's static shape is
  the cost model (the lane's §8, "requests per move under a public load").
- A session the provider can revoke but the Worker ignores is acceptable; a session the Worker mints from an
  unverified address is not: `email_verified` is required, so two providers naming one address are one player and a
  provider that cannot vouch for an address names no one.
- An anonymous upload with no ceiling, or one that skips the strict replay: a stranger's record reaching the engine is
  the named risk (the lane's §8, the archive row), and S1b's bounds are the floor, not a ceiling to remove.
- A second identity model on one deployment: one source per Worker, by variable, refused at start otherwise.

## 3. The owner's rulings from the brainstorm (2026-10-18), and what each binds

- **R-A1 — the provider model is a stock OpenID Connect relying party; WorkOS AuthKit is the launch issuer.** The
  owner's question: "which of these four is the most plug and play to our current access approach and for others to
  change things, e.g. the stock oauth flow". The answer taken: the Access verifier already checks an RS256 JWT
  against a published key set with an `email` claim, and an OIDC ID token is exactly that at the callback; so the
  second arm is the Access arm with another key source, issuer and audience plus `email_verified` and a nonce, and the
  issuer is three variables. Of the four reviewed, Auth0 and WorkOS publish standard discovery documents (WorkOS:
  `/.well-known/openid-configuration`, code flow, `openid email profile` scopes, PKCE or a client secret); Clerk is
  SDK-first and its OIDC-provider feature is for third-party sign-in, Stytch is API-first with its own session shape.
  WorkOS for its ceiling (1,000,000 monthly active users free), every method free (email code, Google, GitHub, Apple,
  passkeys, MFA), bot defence on its page, and sessions configurable long; the owner: "For WorkOS I tend to agree".
  Auth0 is the drop-in alternative (textbook OIDC; the custom domain free with a card on file; Discord; a 25,000 cliff
  and a 3-day idle cap at the provider). **One tension, recorded:** WorkOS's own integration guidance (the `workos`
  skill the owner installed, `references/workos-authkit-setup.md`) expects its SDK and a sealed session cookie and
  says "never … build a custom OAuth flow"; this design runs the STANDARD OpenID Connect code flow WorkOS publishes in
  its discovery document, with no SDK, because the issuer must stay a configuration a self-hoster can change. The
  pre-flight proves the standard token endpoint accepts a dashboard-created confidential application with its secret
  (the metadata lists `client_secret_post` and `client_secret_basic`); if it does not, the fallback is the
  `@workos-inc/node` SDK's code exchange at the callback alone, one npm dependency the owner would have to admit
  against §8, and never a per-request call. Binds §4.1–§4.2.
- **R-A2 — the session is the Worker's own cookie, long and stateless.** "The session length makes sense since we
  can adjust the game cookie, I do think that having the ability to revoke a cookie can have some benefit but the
  real world scenario seems unlikely — so we would want to go with what the average tells us that users would find
  useful in the open source deployment." Taken as: a signed cookie, 30 days, renewed on use, sign-out clears it, key
  rotation is the emergency revoke, no session store. Binds §4.2.
- **R-A3 — the sign-in page is the provider's; ours is the landing.** "For the sign in page, agreed — but remember we
  have a goal of keeping worker invocations to a minimum, this is one of the reasons our front door is structured
  with static assets as it is, and it's not until the multiplayer button is clicked and the turnstile is passed the
  worker is even invoked — and we also have url rewriting to help protect those links from causing run ups on our
  metered use." The context ("this game needs a sign-in") lives in the client, served static; the provider's page
  stays stock; a sign-in costs two invocations and nothing after. Binds §4.3, §4.6.
- **R-A4 — the anonymous upload is built in this cycle, and the solo page never invokes the Worker.** "the anonymous
  upload path is important … there isn't a reason to gate that, and we want to gather save game data — we also need
  to confirm that r2 upload process is still something that in the build or deploy process can be flagged off, but it
  will need to be included — we should fold that into this phase for validation with or without — or just all
  anonymous. On the single player page, we don't want to get beyond the static assets so we don't bring up
  invocations, so login isn't material, if the user is logged in for multiplayer and the single player detects that
  outside the home page then that's ok to include that in the r2 metadata or save data". Taken as: the finished
  game's post is same-origin, so the browser attaches the session cookie if one exists and the Worker files under the
  verified caller, else anonymously; the client detects nothing; the archive switch stays the per-deployment
  variable `YOKOHAMA_WEB_ARCHIVE` (`r2` | absent; `worker/src/config.ts` lines 139–140), no build flag. The layout
  (§4.4) keeps the per-caller prefix for signed-in uploads and adds an anonymous one; "all anonymous" is the
  recorded alternative, not taken (it discards the per-caller share S1b built). Binds §4.4–§4.5.
- **R-A5 — one model per deployment; Access stays; the public Worker is its own deployment** (restating the
  specification's §9.7 on the owner's words of 2026-10-12, not a new ruling): the private Worker keeps Access for the
  owner and development, the public Worker runs the provider, never both on one Worker; the NAS is untouched.
  Binds §4.7.

**The owner's answers to §5 (2026-10-18, the second round), transcribed:**

- **R-A6 — `yokohama.moore.nyc` is the public site.** "we'll be using yokohama.moore.nyc for the public site — we'll
  setup a new private one later on another domain but this one is the one we'll be deploying against and getting
  this live on." So every browser's stored games and outbox rows, the friend's waiting game among them, meet the
  public deployment at the first visit; the edge rule's existing expression and slot serve it; the private Worker's
  Access destinations move to the new domain when it exists, and until then the private deployment is the one that
  moves off this hostname. Binds §4.5, §4.7.
- **R-A7 — WorkOS, through its own first-party flow, with the generic OIDC arm beside it.** The owner's question:
  "workos says to not use the custom oauth flow, if that's the case, should we be looking at auth0 instead? … If we
  are confident we should proceed anyway with workos then we can do that." The reading that answers it: WorkOS's
  first-party sign-in is its own documented API — `GET /user_management/authorize` with `provider=authkit`, a code,
  then `POST /user_management/authenticate` with `grant_type=authorization_code`, the client id, the client secret
  and the code, answering the `user` object (its `email` and `email_verified`) and the session tokens; its standard
  OIDC endpoints (`/oauth2/authorize`, the discovery document) belong to WorkOS Connect, which is for third-party and
  MCP clients and shows a consent screen. The skill's "never build a custom OAuth flow" is advice to use the SDK rather
  than hand-roll against WorkOS; the SDK's `authenticateWithCode` IS that POST. So the design runs WorkOS's own
  first-party API with two `fetch` calls (the `workos` arm, §4.1), not WorkOS Connect and not a hand-rolled OIDC
  against it, and keeps the generic `oidc` arm for any issuer with a discovery document, Auth0 included. Auth0 is not
  taken for the public site: free to 25,000 monthly active users and then priced by tier (its own page: $2,100 a
  month at 30,000), against "tens of thousands of free users". Confidence rests on the pre-flight: the `workos` arm is
  exercised live against an unclaimed environment the CLI provisions before any plan task leans on it; the SDK at the
  callback stays the recorded fallback. Binds §4.1, §4.3.
- **R-A8 — anonymous uploads never expire.** "Anonymous uploads shouldn't expire". No lifecycle rule; the bucket is
  synced out as the archive module's header already assumes. Binds §4.4.
- **R-A9 — the anonymous share keeps S1b's figures.** "share figures — path of less resistance is fine, the goal is
  to get save files in for the computer player, and more data around it is useful." The per-caller records and bytes
  apply to an anonymous key unchanged. Binds §4.4.
- **R-A10 — the calls stand.** "the other notes are in line with what I expected": the methods (email code, Google,
  GitHub, Apple, passkeys, passwords off), the WorkOS-hosted sign-in domain at launch, the stateless 30-day cookie,
  both halves on `/enter`, `POST /api/leave`, the keyed-pairing rule, the `.test` minted session.
- **R-A11 — Bot Fight Mode does not gate the upload, and the design no longer depends on an update.** The owner:
  "bot fight mode, why would this stop a user from uploading a file — if they are not active in the browser, it
  wouldn't trigger at all, so in the scenario they go back to the site, and that brings up the task for it to upload,
  why would bot fight stop it?" Correct: it does not. W-3 (the lane's §8 table, §15) is about a returning browser
  INSTALLING a new release — Bot Fight Mode challenges a clearance-less browser's precache fetches, `/art/manifest.json`
  among them, so the install fails — and the design had made the anonymous upload depend on the new client, which
  alone sends the device header. The upload post itself meets the edge rule's Turnstile, which the shipped client
  already carries through its "waiting for a human check" overlay (cycle F). So the dependency is removed: a post
  with no device header files under the game's own id (§4.4), and a browser running today's client lands its waiting
  games the first time it comes back, with no update. W-3 leaves this cycle and stays the lane's owed decision about
  updates in general. Binds §4.4, §4.5.

## 4. The design

### 4.1 The identity source, shell-side

The layer's door stays `worker` (`YOKOHAMA_WEB_IDENTITY`); what changes is where the shell gets the address. A new
variable names the source, with no default and refused at start otherwise, as the door is:

| variable | values | read by |
|---|---|---|
| `YOKOHAMA_IDENTITY_SOURCE` | `access` \| `oidc` \| `workos` | `shellConfig`; `open` ignores it; `worker` requires it |
| `YOKOHAMA_OIDC_ISSUER` | the issuer URL whose `/.well-known/openid-configuration` is fetched | the `oidc` arm; refused under `workos`, whose issuer is `https://api.workos.com` |
| `YOKOHAMA_OIDC_CLIENT_ID` | the client id | the `oidc` and `workos` arms |
| `YOKOHAMA_OIDC_CLIENT_SECRET` | a SECRET; under `oidc` absent means a public client with PKCE alone (`none`); under `workos` the API key, required | the code exchange |
| `YOKOHAMA_SESSION_SECRET` | a SECRET, 32 bytes or more, the cookie's HMAC key | `mint` and `read` |
| `YOKOHAMA_SESSION_DAYS` | whole days above zero, absent = 30 | `mint` |

Under `access` the Worker runs exactly today's `resolveIdentity` and the dev door, byte for byte; `ACCESS_TEAM_DOMAIN`
and `ACCESS_AUD` are required under it and ignored under the other two, where the client id and secret are required
and the Access ones ignored. The deploy guard reads the source as it reads the door and refuses a file naming one
source's variables under another (D-7's second identity, below). The public deployment runs `workos` (R-A7); a
self-hoster with any other issuer runs `oidc`.

**The `oidc` arm** verifies an ID token the way the Access arm verifies an assertion — the header's `alg` (RS256; ES256
accepted too, since issuers vary), a string `kid` matched in the issuer's key set, the signature, `iss` equal to the
discovery document's issuer, `aud` containing the client id, `exp` and `nbf` against the clock, and then what Access
never needed: `nonce` equal to the one this login started with, and `email_verified` `true` beside a string `email`
judged raw by `forwardable`, trimmed and ASCII-lowercased by `address` (the Kelvin rule, unchanged). Anything short
of that names no one. The key set is `keySource`'s closure with the discovery document's `jwks_uri` in place of the
Access certs URL, the same once-a-minute refresh on an unknown `kid`. The discovery document is fetched once per
isolate and cached as the keys are. **The arm runs at the callback only** (§4.2); no request after it touches the
issuer.

**The `workos` arm** (R-A7) runs WorkOS's documented first-party flow with two `fetch` calls and no SDK: the
redirect goes to `https://api.workos.com/user_management/authorize` with `client_id`, `redirect_uri`,
`response_type=code`, `provider=authkit` and `state`; the callback POSTs `https://api.workos.com/user_management/authenticate`
as JSON with `client_id`, `client_secret`, `grant_type=authorization_code`, `code`, and the caller's `ip_address` and
`user_agent` (WorkOS's own sample sends them; Radar reads them). The answer's `user.email` is the address, judged by
`forwardable` and `address` exactly as an Access claim is, and `user.email_verified` must be `true`; the answer's
`access_token` is a JWT whose `sid` the session cookie keeps for sign-out (§4.2) and whose signature the arm checks
against `https://api.workos.com/sso/jwks/<client_id>` as belt and braces (the answer is already authenticated by TLS
and the secret). Nothing else in the response is kept; no refresh token is stored, since the session is ours. The
arm is the `oidc` arm less the discovery document, the ID token and the nonce, plus one JSON body; the pre-flight
runs it live against an unclaimed WorkOS environment (§9.2) before any plan task leans on it.

**The wire does not learn the source.** `/api/me`'s `mode` and `/api/health`'s `identity` are the layer's, written
from the module's `wire_name`, which says `cf-access` under the `worker` door by a recorded ruling (R2.2-m2;
`crates/yokohama-worker/src/config.rs` lines 105–115), and nothing here moves that: under `oidc` the client reads
`cf-access` exactly as it does under Access, and every remedy it draws for that mode ("Sign in at the door", one
step) is already the right one. The one wire addition is a header the shell sets on the way out (§4.3's 401).

### 4.2 The session cookie

The callback mints `yokohama_session`: `HttpOnly; Secure; SameSite=Lax; Path=/`, value `v1.<payload>.<mac>` where the
payload is the address, the issue time and the expiry (base64url of a small JSON object) and the MAC is HMAC-SHA256 under
`YOKOHAMA_SESSION_SECRET` over the version and payload, compared in constant time. Reading it is a MAC and a clock
check, microseconds, inside the request that was going to run anyway. The cookie is renewed when a request finds it
past half its life (a fresh `Set-Cookie` on that answer, never on a 101, which carries none). Sign-out is
`POST /api/leave` from the page (so the cross-site refusal B5 built covers it and no navigation can log a player out
from another site): under `oidc` the shell clears the cookie and answers `{"leave":"/"}`; under `access` it answers
`{"leave":"/cdn-cgi/access/logout"}` and clears nothing; the client navigates to the answer's `leave`. A shell-written
JSON answer is the shape `errorBody` already has. Because
`SameSite=Lax` cookies ride same-site requests, an `EventSource` and a WebSocket upgrade from the page carry it with
nothing added to the client, which is the whole reason it exists (the specification's "an `EventSource` and a
WebSocket upgrade carry no header"). A stolen cookie is valid until it expires; rotating the secret ends every session
at once; both are the recorded trade (R-A2).

Under `access` no cookie is minted and none is read: Access's own cookie and assertion do that job, as today.

### 4.3 The flow, and the client

- **Both halves of the sign-in live on `/enter`, the one path every shipped service worker already bypasses.**
  `isServerPath` (`web/src/swRoute.ts` lines 48–56) matches the pathname `/enter` exactly and ignores the query, and
  rule 2 answers every OTHER navigation from the cached shell — so a callback on a new path would be swallowed by
  every browser holding today's worker until it installed a new release, which under the public door a lapsed
  clearance can prevent (W-3; the lane's §8 table). On `/enter` the callback works for returning browsers on day one
  with no service-worker change. `run_worker_first` and `route` stay as they are; `/enter` keeps refusing every
  method but GET and HEAD.
- `GET /enter` with no `code` under `oidc`: the Worker draws `state`, `nonce` and a PKCE verifier, sets a short-lived
  signed `yokohama_login` cookie holding them and the `return_to` hash route (from `?return_to=`, only ever `/#/…`),
  and answers 302 to the issuer's `authorization_endpoint` with `response_type=code`, `scope=openid email`, the client
  id, the redirect URI `https://<host>/enter`, `state`, `nonce` and, when no client secret is set, `code_challenge`
  (WorkOS accepts PKCE on a public application and may refuse it on a confidential one; the pre-flight reads which).
  One invocation. (Under `access`, `/enter` stays the 302 to `/#/lobby` it is today, `worker/src/index.ts` line 203;
  the Access edge does the rest.)
- `GET /enter?code=…&state=…` under `oidc`: the Worker checks `state` against the login cookie, exchanges the code at
  the `token_endpoint` (one subrequest, with the secret or the verifier), verifies the ID token (§4.1), mints the
  session cookie (§4.2), clears the login cookie and answers 302 to `return_to` (anything that is not a hash route on
  this origin is `/#/lobby`). One invocation. A failed exchange, a bad `state` or a token naming no one answers 302
  to `/#/lobby?signin=failed` and sets nothing. Under `access` a `code` on `/enter` is ignored.
- **Under `workos`** the same two steps run against WorkOS's first-party endpoints (§4.1): the redirect carries
  `state` and `provider=authkit` and no nonce or PKCE (the exchange is authenticated by the secret), the callback
  exchanges the code at `/user_management/authenticate` and reads the `user` object; the `state` cookie, the
  `return_to` rule, the session cookie and every refusal are the `oidc` arm's, shared. Sign-out's `leave` under
  `workos` is `https://api.workos.com/user_management/sessions/logout?session_id=<sid>&return_to=https://<host>/`,
  the Sign-out URI the application registers.
- **What the issuer registers** (first-party: the skill's `workos-authkit-setup.md` and `workos-terms.md`): the
  Redirect URI `https://<host>/enter`, matched exactly, trailing slash included; the Sign-out URI `https://<host>/`,
  where a player lands after the issuer's session ends, and every `return_to` the sign-out may name; and the
  Initiate login URI, the app route that STARTS a sign-in when one begins outside the app (a bookmarked hosted page,
  a password-reset or invitation email), which is `https://<host>/enter` as well — WorkOS's guidance says the initiate
  URI is "not the callback URL" because an SDK callback handler fails without a code, and ours does not: `/enter`
  with no `code` starts the flow, so one path serves both. The session cookie's payload also carries the ID token's
  `sid` when the issuer sends one, so `POST /api/leave` can answer the issuer's end-session URL with it and the
  issuer's session ends with ours; without a `sid`, the answer is `/`.
- **`/api/me`** answers the layer's body as today (`mode` `cf-access`, `user` the address) with a session, and the
  layer's 401 without one; under `oidc` the shell adds `WWW-Authenticate: Yokohama-Session realm="/enter"` to every
  401 under `/api` on the way out (`withApiHeaders`, the one place every API answer passes), so the client's one
  predicate that draws a sign-in link beside a refusal (`isAccessChallenge`, `web/src/api.ts` lines 326–350) gains a
  sibling reading this header and the same sentence and link. `OnThisDevice.tsx`'s `Remedy` (lines 472–500) draws
  one step under `cf-access`, which is what `oidc` reports, so it does not move.
- **The client**: the front door's multiplayer card keeps its plain navigation to `/enter` after the Turnstile
  pre-clearance (`FrontDoor.tsx` lines 230–257), unchanged. A table link (`/#/t/<id>`) opened by a browser whose
  `/api/me` answers 401 renders the landing: "This table is multiplayer. Sign in to take your seat." with one button
  that navigates to `/enter?return_to=<the hash route>`; a browser with a session continues as today. Sign-out is a
  link to `/leave` on the lobby and the device page. No sentence names the provider.
- **The header set is unchanged.** The sign-in is a navigation and the callback a redirect; neither is governed by
  `form-action 'self'` or `connect-src`, and no script of the provider's is loaded. The harness that forgives no CSP
  line stays as it is; the pre-flight reads the flow under the enforced policy to prove it.

### 4.4 The anonymous archive

- **The anonymous caller is a synthetic ADDRESS, and the layer does not change.** The key grammar `User::parse` is
  one grammar for every host and every key site (`crates/yokohama-web/src/identity.rs` lines 140–176), and a third
  shape in it would reach seats, watchers, chat, backups and open-door claims; so no shape is added. The shell forms
  the address `anon-<16 hex>@anon.invalid` (`.invalid` is reserved by RFC 6761, so no real player can hold it), which
  the grammar already accepts and the `worker` mode rule already admits as an address. `caller_segment` renders it
  `anon-<16 hex>_at_anon.invalid-<12-hex digest>` under the `browser/` root, never the `test/` one, so `paths`'
  layout, the per-caller share, the receipt and the strict replay run byte for byte as they do for any address:
  `browser/anon-<device>_at_anon.invalid-<digest>/rev7/over/<id>.json` and its `.table.json`. Under `open` a claim
  shaped like an address is refused (`NamesOnly`), so no visitor can claim it; under `cf-access` the NAS never forms
  one and a forged header cannot reach the port (S2c's R-1); on a self-host under `worker` a proxy header spelling it
  is trusted exactly as any address the proxy spells, which is the trusting-origin model already recorded.
- **The device.** The client draws a device id once (16 hex from `crypto.getRandomValues`), keeps it in the outbox's
  own IndexedDB store beside the rows, and sends it as `X-Yokohama-Device` on every archive post. A browser that
  clears its storage is a new device; that is fine.
- **The Worker's archive arm.** The shell's `Identity` gains a kind, `{ kind: "anonymous"; device }`, formed in
  `answer` only when the target is `archive`, the request carries no verified caller, and `X-Yokohama-Device` is
  exactly 16 lowercase hex digits (read before the strip, like the claimed header). `callerName` renders it as the
  synthetic address, so `objectFor`'s archive case forwards it as `X-Yokohama-User` and names the object
  `ARCHIVE_PREFIX + <address>` with no new arm; on any other target the kind is never formed, so the twenty-three
  game routes never see it. **With no caller and no device header** (R-A11: a browser running today's client, whose
  queue posts without one), the device is the game's own id, read from the body's table document as `createId`
  reads a create's `id` (`worker/src/index.ts` lines 370–373) and held to `is_safe_id`: the address is then
  `anon-<game id>@anon.invalid`, one prefix per game, its share trivially met, the IP limiter bounding the rate, and
  the metadata's `device` reads `none`. A body naming no safe id with no caller and no device header is the house's
  401 as today. Under `open` nothing
  changes (a claim is the caller). Under `access` the arm exists too, but a caller-less post never reaches the private
  Worker (the edge answers first), so it is live only where the edge lets one through: the public deployment.
- **The metadata.** Every browser upload's R2 object carries custom metadata the Archive object sets: `caller` (the
  address, or `anonymous`), `device` (the header's value when sent, signed in or not), `rev` (`SEMANTICS_REV`), `why`
  (`over` | `abandoned`) and `uploaded` (the time) — `rev` and `why` read off the plan's key, so `r2.ts`'s five-field
  plan does not change. A review lists the bucket and reads who sent what without opening a file; the record's
  `run.run_id` stays `web/browser`.
- **The ceilings.** (1) The per-caller share applies to an anonymous key exactly as to an address (records and bytes
  under its prefix, counted live by one listing, the 507 when full) — the builder keeps S1b's figures unless the owner
  sets anonymous ones lower — R-A9: S1b's figures, unchanged. (2) `POST /api/archive` from an anonymous caller runs under `CREATE_LIMIT` by connecting
  IP (10 per 60 s): `limitKey` reads the `anonymous` kind as no caller (an IP key, never a per-device key a device
  could rotate) and `limiterFor` sends the anonymous archive post to the create limiter instead of the write one
  (`worker/src/limits.ts` lines 30–42), so one machine cannot mint devices faster than that. (3) No expiry (R-A8):
  anonymous objects stay, and the bucket is synced out as the archive module's header assumes. No global live count
  across every anonymous prefix: a listing of tens of thousands of objects is pages of subrequests inside one
  request, and the IP limiter bounds the rate without it. If the owner wants a hard global ceiling later, it is a
  counter in the house object under its lock, a separate decision.
- **Under the edge rule** a caller-less post from a browser with no clearance meets the Turnstile's challenge, and
  the overlay that passes it is the OUTBOX's, not the multiplayer door's: the front door's button and the outbox are
  two callers of one seam (`passChallenge`, `web/src/turnstile.ts` line 186), so a solo player who never went near
  multiplayer meets the widget on the solo page itself. `noteGameOver` (`web/src/outbox.ts` lines 700–728, fired by
  `App.tsx` line 1144 when a local game reaches `over`) runs interactively whenever the tab is visible — "a person is
  at the game when it ends" — so the health read and the post pass the challenge through the overlay there, usually
  with no click in `managed` mode; a hidden tab or the `online` event marks the row "challenged" (rule 6a), and the
  device page's "Check now" or the next visible launch shows the widget and sends (`drainVisible`). What makes this
  true under the public deployment is §4.5's rewrite of rule 6b: today a keyed build holds a nameless visitor's game
  for a name and asks the server nothing, "a fresh context's first game over included", which under `worker` would
  park every solo-only player's game for ever.

### 4.5 The outbox under the new door

- Rule 6b's "a keyed build is the open door" (`namelessUnderKey`) was true when a Turnstile site key meant the
  open door; the public deployment is a keyed build under `worker` + `oidc`. The rule becomes: a keyed build whose
  server facts say `open` holds for a name; a keyed build whose facts say `worker` posts, and a browser that has not
  read the facts posts too (the post is the cheapest way to learn the answer, and the device header rides it).
- The reasons: `signed-out` stays the row's state for a 401 that names a door (`WWW-Authenticate` present) and the
  remedy is the door's link; a 401 without one under `worker` is the house refusing the anonymous shape, which cannot
  happen with a well-formed device header and is a defect to surface, not to park.
- Games already waiting in a browser: a `signed-out` row is re-evaluated at every trigger (the head of `outbox.ts`,
  rule 2: "Reasons are re-evaluated, not frozen"); on the first visit to the public deployment at the same origin the
  drain posts it with the device header and it lands anonymously, or under the caller if the browser has since
  signed in. Browser storage is per origin: a game held in a browser against `yokohama.moore.nyc` reaches the public
  deployment at `yokohama.moore.nyc` (R-A6), which is where every such game is. The drain runs from the SHIPPED
  client too, since its post is the same post: the old client sends no device header, so the Worker files its game
  under the game's own id (§4.4, R-A11), and the post's Turnstile challenge is carried through the overlay the
  shipped client already has. No update is needed for a waiting game to land; W-3 is not this cycle's.
- The device id lives in the outbox's `META` store (`web/src/local/db.ts` line 198), which exists; no IndexedDB
  version moves.

### 4.6 Cost

| event | Worker invocations | subrequests |
|---|---|---|
| the home page, the solo page, a table link opened | 0 | 0 |
| a sign-in | 2 (`/enter`, `/enter/callback`) | 1 token exchange; the discovery document and the keys once per isolate |
| any later request | as today | 0 for identity |
| a finished solo game | 1 (its upload), as the F window measured | the share's listing, the R2 put, as today |

### 4.7 The second deployment, and the guard

The public Worker is its own deployment: its own name, Durable Object namespaces, R2 bucket and rate-limit
namespaces, deployed from `wrangler.public.jsonc` (`worker`, `oidc`, the archive `r2`, the Turnstile `on`,
`workers_dev: false`, the dev door `off`, no `DEV_DOOR_CLIENT_ID`), under the edge rule the lane's §10 recipe
enables by its id before the hostname leaves Access. The private Worker keeps `wrangler.jsonc` with `access`, on a
hostname the Access application keeps. The deploy guard, which "holds this repository's Worker by name and bucket
until S4" (S2b's D-7), learns a second identity: each deployable file names which of the two it is, and the guard
refuses a file whose name, bucket, source and door do not match that identity's record. The `cf-access` NAS is not
touched. The operator secret `YOKOHAMA_WEB_ADMINS` is set on both.

**The keyed-build pairing (R-FP4, "one client build per door state", narrowed by R-A3 and R-A4).** Today the guard's
check 12 refuses `YOKOHAMA_TURNSTILE=on` or a keyed stamp under any door but `open` (`deploy-guard.mjs` lines
619–678), the self-host refuses the same pairing at start, and the outbox reads a key as the open door. The rule
becomes: a keyed build is accepted under `open`, and under `worker` exactly when `YOKOHAMA_IDENTITY_SOURCE` is
`oidc`; it stays refused under `worker` with `access` (the Access edge is the bot defence there) and on the self-host,
whose `check_build_stamp` has no `oidc` arm because the self-host has no `oidc`. `wrangler.public.jsonc` is its own
reference file with its own object, bucket and rate-limit blocks, held to its own record by the guard's identity
table rather than to production by the sibling rule. Each deployment is its own `npm run build` with its own
`YOKOHAMA_SITE_URL` and key, then its own assembly and deploy, sequentially, as a client build per host has been since
cycle F; check 9 holds each tree to the `web/dist` of its own build.

**The test identity on the public Worker.** The dev door is the Access assertion's `common_name` plus a claimed
`.test` address, and the public Worker has no Access assertion, so its successor there is a session minted from
`YOKOHAMA_SESSION_SECRET` by an orchestrator tool for a `.test` address and nothing else (the tool refuses any other
suffix; the receipt archive's `test/` root keeps those games apart as it does today). The secret's holder can mint any
session by construction, so this adds no trust; it adds a tool, a runbook recipe and a fence the guard and the after
review read. The live replay, the Playwright live tier, the smoke's identity reading and the pre-flight's figures run
through it on the public Worker, and through the dev door on the private one as today.

### 4.8 What a self-hoster does

Keeps `open` for a LAN; or sets `worker` with `access` behind their own Access application as the private Worker
does; or sets `worker` with `oidc` and three variables against any issuer with a discovery document — Auth0, WorkOS,
Keycloak, Authentik, Google directly — plus the two secrets. The README's hosting section gains that paragraph; no
code names a provider.

## 5. Questions only the owner can answer

**All five were answered on 2026-10-18 and are transcribed as R-A6–R-A11 in §3**; the questions stay as asked, for the
record of what the builder check could not settle.

1. **Which hostname is the public deployment's?** `yokohama.moore.nyc` carries every browser's stored games and
   outbox rows today, and the friend's waiting game among them; the public Worker at that hostname inherits them on
   the first visit, and the private Worker moves to a new one. The other way round, the browser-held games stay with
   the private hostname. Two more things move with the answer: the edge rule is per host (`edge-rule.mjs`'s
   expression, `http.host eq`) and the zone's custom-rules phase is full at five on Free, so a NEW public hostname
   needs a slot or a two-host expression; and `access-paths.mjs` hard-codes the private Worker's hostname for its
   Access destinations, which follow whichever hostname stays behind Access.
2. **W-3, the returning browser's update under the no-Access door** (the lane's §8 table and §15: OWED, "the wrap's
   decision"). The public deployment is the first operational no-Access deployment with returning users, and a
   browser whose clearance lapsed cannot install a new release while Bot Fight Mode challenges the precache's
   fetches. The candidates the lane records: Bot Fight Mode off under the public door (F's bursts showed the rule
   alone refuses scanners at zero invocations), a clearance-refreshing read, or the precache's extensions. The
   sign-in itself is immune (§4.3, `/enter`), but the anonymous upload from a stale client is not (§4.5).
3. **Do anonymous uploads expire?** A lifecycle rule in days, or none (keep and sync out).
4. **The anonymous share's figures**: S1b's per-caller records and bytes, or lower ones for a device.
5. **The sign-in methods at launch**: email code, Google, GitHub, Apple, passkeys, passwords off (my call, §2), or a
   different set; and whether the WorkOS-hosted sign-in domain is acceptable at launch or the $99 custom domain is
   wanted from day one.

## 6. What is left to the builder

The cookie's exact payload and the login cookie's lifetime (minutes); the `workos` arm's request and response
shapes as the pre-flight reads them live; the
client's landing component and where the sign-out link sits; the discovery and key caches' shapes; the pool cells
(a fixture issuer with a test key set, as the Access battery has: alg, kid, signature, aud, iss, exp, nonce,
`email_verified` false, a Kelvin address); the device store's migration; the guard's identity records; the metadata
field names' spelling; the runbook recipes for the switch (Access off the public hostname LAST, the rule on FIRST,
as §10 says) and the rollback; the figures the pre-flight measures (the sign-in's invocations and the cookie read's
CPU, the anonymous post under the limiter, the policy read with no CSP line, and whether the WorkOS application takes
`code_challenge` as created); the `anonymous` identity kind's reading in `limitKey` and `limiterFor`; the device
header read before the strip and re-set after; the session-minting tool's shape and its `.test` fence; the guard's
identity table and the public file's own pool file; the third-party licence line for nothing, since no dependency
is added.

## 7. Acceptance

Per edit: the lane's suites (`docs/web-lane.md` §2) and the pool. Per cycle: the plan review on exports, the SDD run
with implementers probing their briefs, the after review, the fifteen-gate roster NOT owed unless core or the version
moves (the version moves: the release that changes the door bumps it by the lane's §7). Live: a window on the public
deployment, empty, in the §10 order; the readings into `worker/RUNBOOK.md`; the friend's game observed landing
anonymously, or the reason it did not, as the first real reading of §4.5.

## 8. The fences

The specification's §6 holds: `SEMANTICS_REV` 7, the oracle and the five fenced crates untouched, no bot behaviour
moves, the review report never committed. This cycle touches the Worker shell, the client, the deploy guard and the
documents; the layer (`crates/yokohama-web`) and the module (`crates/yokohama-worker`) are NOT touched (§4.4's
synthetic address is the reason); nothing enters the Rust dependency tree; the Worker gains no npm dependency (the
OIDC flow is `fetch`, `crypto.subtle` and the JWT code already in `identity.ts`).

## 9. Review record

### 9.1 — 2026-10-18, the builder check (a fresh agent, read-only, over the design and every file it cites)

Five questions where a wrong guess would change the plan; each sorted and answered in place:

1. *Is `anon:<16 hex>` a third shape of the one key grammar, or an archive-only caller beside `User`?* Neither:
   `admit` takes a `User` and `User::parse` refuses the shape, and both answers were a layer change with a security
   reading (a grammar shape reaches every key site on every host; a second type threads four call sites). The design
   now forms a synthetic address the grammar already accepts (§4.4), so the layer and the module are untouched and
   the fence in §8 is tightened accordingly. My call.
2. *The service worker answers every navigation from the cached shell, so `/enter/callback` and `/leave` would be
   swallowed by browsers holding today's worker.* True (`isServerPath` bypasses `/enter` exactly). The callback
   moved onto `/enter` itself and sign-out became `POST /api/leave` (§4.2, §4.3). My call. The two consequences the
   builder found for the hostname answer are added to §5 question 1, and W-3 is raised to the owner as §5 question 2.
3. *What signs a test identity in on the public Worker, with no dev door there?* A session minted from the secret
   for a `.test` address only (§4.7). My call, flagged for the owner's eye.
4. *`mode` and `source` on the wire are the module's, and the Worker says `cf-access` under `worker`.* True
   (R2.2-m2). The design no longer puts `source` on the wire; `mode` stays `cf-access`; the one addition is the 401's
   header (§4.1, §4.3). My call; it removes work.
5. *Which keyed pairing does the guard accept, and does the self-host's stamp check move?* Keyed under `open`, or
   under `worker` with `oidc`; the self-host unchanged; the public file its own reference; one build per deployment
   (§4.7). Recorded as a consequence of R-A3 and R-A4 rather than a new ruling; the owner may overrule.

Checked by the builder and found consistent, kept as pre-flight items: WorkOS's discovery document, `email_verified`
in the `email` scope, `nonce` required, RS256 only, the three token-endpoint auth methods, the PKCE condition; the
R2 plan's five fields; the `META` store; the cookie on the upgrade and the 101 carrying no `Set-Cookie`;
`run_worker_first` pinned exactly in five files by check 5 (now untouched, since no path is added).

### 9.2 — 2026-10-18, the WorkOS first-party references (the `workos` skill the owner installed under
`.agents/skills/workos/references/`, read after the builder check; each file says "if this conflicts with fetched docs,
follow the docs")

- Magic Auth, invitation and password-reset emails are sent by WorkOS; a custom sender domain is optional, needs real
  `welcome@` and `access@` inboxes, and uses WorkOS's CNAME-based authentication (`workos-email.md`). No email of
  ours, no Resend, no Cloudflare Email Sending.
- Radar, the bot and fraud detection, "is built into AuthKit natively" — the hosted page's bot defence costs nothing
  to wire; its standalone API is for custom flows and is in preview (`workos-radar.md`).
- Custom domains are production-environment only; staging always uses `workos.dev`; an AuthKit custom domain means
  updating every registered callback URL (`workos-custom-domains.md`). The launch runs on WorkOS's hosted domain
  unless the owner answers §5 question 5 the other way.
- The three URLs an AuthKit application registers, and the exact-match rule for the Redirect URI, are in §4.3.
- Development can start with no WorkOS account: `WORKOS_MODE=agent npx workos@latest install` provisions an unclaimed
  environment and writes its keys to a gitignored env file; the environment is claimed later from the same machine,
  and "never ship an unclaimed environment's keys" (`SKILL.md`). The pre-flight uses such an environment for the live
  readings of the flow; production credentials come from the owner's account.
- The JWKS endpoint is `https://api.workos.com/sso/jwks/<client_id>` and the access token's claims are `sub`, `sid`,
  `iss`, `exp`, `iat` and organization fields (`workos-terms.md`, `workos-api-authkit.md`); the address rides the ID
  token's `email` claim under the `email` scope (the builder check's reading of the discovery document, §9.1), which
  is why the design verifies the ID token and not the access token.

### 9.3 — 2026-10-18, the owner's answers to §5 and one correction (the second round)

The owner answered all five questions in one message (transcribed as R-A6–R-A11, §3). Two things changed the design:

- **The Auth0 question.** The owner read the WorkOS skill's "never build a custom OAuth flow" and asked whether that
  points at Auth0. The reading taken (R-A7): WorkOS's first-party sign-in is its own documented API, not its Connect
  OIDC endpoints (those serve third-party and MCP clients, with a consent screen), so the design gains a `workos` arm
  that runs that API with two `fetch` calls, keeps the generic `oidc` arm for every other issuer, and does not take
  Auth0 for the public site (its pricing past 25,000 users). The two arms share the state cookie, the session cookie,
  `/enter`'s two halves and every refusal; the difference is one request body and where the address is read.
- **The Bot Fight Mode correction.** The owner asked why Bot Fight Mode would stop an upload at all. It does not; the
  design had let the anonymous upload depend on the new client's device header, and a returning browser can fail to
  install a new client (W-3). Now a post with no device header files under the game's own id, so a browser running
  today's client lands its waiting games the first time it comes back, through the Turnstile overlay it already has.
  W-3 is no longer this cycle's.

The other answers set the hostname (`yokohama.moore.nyc` is the public site; the private one moves to a new domain
later), no expiry on anonymous uploads, S1b's share figures unchanged, and accepted the calls listed in §9.1 and the
methods and hosted domain. With them the document has no open question, and the plan is written against it next.
