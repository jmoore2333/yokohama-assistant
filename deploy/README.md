# Self-hosting Yokohama — `deploy/`

This directory builds and runs **the table layer's server and its browser client as one Docker image**.
The image serves:
- the multiplayer lobby and tables;
- the card art;
- the client, which also plays a single-player game entirely in the browser.

The hosted deployment on Cloudflare is a different host, with its own guide:
[`worker/README.md`](../worker/README.md).

What else is in this directory:
- **[`RUNBOOK.md`](RUNBOOK.md)** is this host's operating record: the owner's NAS deployment and its recipe
  (§0–§3), every deployment's readings (§3.5 on), and rollback (§5).
- **`Dockerfile`** builds everything from a checkout. It is the path this guide follows.
- **`Dockerfile.runtime`** assembles an image from artifacts built elsewhere (a static musl binary and a
  built `web/dist`). That is how the NAS is deployed (RUNBOOK §1–§2).
- **`smoke-image.sh`** checks an image under each door before you put it anywhere (§5).
- **`update.sh`** moves a running container to a release tag: fetch, build, smoke, swap (§9).

**The image contains the game's own card art**, which carries no licence grant ([`ASSETS.md`](../ASSETS.md)).
Build it for yourself, and never push it to a public registry.

## 1. Access first: the repository's default

The server reads exactly ONE identity header, and which one is your decision: `YOKOHAMA_WEB_IDENTITY` has
**no default**, and a container started without it exits, naming the accepted values.

**This repository's default, and the door its own NAS runs (since 2026-09-28, `RUNBOOK.md` §3.17), is `cf-access`:**
- the container sits behind a Cloudflare Tunnel whose hostname is a Cloudflare Access self-hosted
  application;
- Access authenticates each person and adds `Cf-Access-Authenticated-User-Email` to every request it
  forwards;
- the server reads that header and no other, and only an address in it. Anything else there reads as no
  one.

A person's seats are keyed by that address, so a player is who Access says they are.

**The one rule this door rests on, and how to make it true:** the tunnel must be the ONLY path to the container's
port. The server does not verify Access's signed assertion (`Cf-Access-Jwt-Assertion`) — by the owner's ruling of
2026-10-13 (security S2c, R-1): a verifier in the binary would add an outbound TLS dependency, a token parser on
every request and the first C build in the musl binary to close a path the recipe below closes for nothing — so
anything that reaches port 8080 without passing Access can send the header and be anyone. The recipe:
1. **A Docker network only the tunnel and this server join.** Create it (`docker network create cf-internal`),
   start `cloudflared` on it, start this server on it with NO port published (`YOKOHAMA_WEB_NETWORK=cf-internal` and
   `YOKOHAMA_WEB_PUBLISH=` empty for `deploy/update.sh`, or `--network cf-internal` and no `-p` at all), and put every
   other container — a terminal, a registry, anything — on a
   different network. cloudflared may join several networks; this server joins one.
2. **cloudflared validates the assertion itself.** In its configuration, each ingress rule for an Access-covered
   hostname carries `originRequest.access` with `required: true`, your `teamName` and the application's `audTag`
   (`deploy/cloudflared-config.yml` is the owner's). cloudflared then refuses, in its own code, any request that
   reaches it without a valid token for that application.
3. **A hardened container:** `--cap-drop ALL --security-opt no-new-privileges:true --memory 512m` on the `docker run`
   (§4's flags, with no `-p` under step 1; `deploy/update.sh` passes the same three; the owner's NAS takes them as
   `CapDrop`, `SecurityOpt` and `Memory` from `deploy/RUNBOOK.md` §3's create body, since S2's window on 2026-10-17).
4. **Read it back:** from a shell in any OTHER container on the host, first a CONTROL that proves the tool works —
   `wget -qO- --timeout=3 http://127.0.0.1:<that container's own port>/` must answer — then `wget -qO-
   --timeout=3 http://yokohama-web:8080/api/health` must FAIL with a resolution, connection or timeout error ("bad
   address", "refused" or "timed out" from busybox `wget`; any HTTP answer, "not found" included, fails the probe;
   GNU `wget -q` prints nothing, and its exit status 4, a network failure, is the pass — exit 0, exit 8 for an HTTP
   error answer, or any other status is the finding), and so must the same address by the server's IP on
   `cf-internal` (`docker inspect`; 172.21.0.4 on the owner's NAS at the recorded recreations through `RUNBOOK.md`
   §3.18, where §3.19–§3.21 record none; read it again after a network change) — a name that does not resolve proves
   nothing about a route.
   List the network's members (`docker network inspect cf-internal`): the tunnel and this server, nothing else. As
   yourself through the hostname, `/api/me` names you. Repeat after any change to the host. What this does not
   close is §4's second bullet: on a shared host, a process that can route to the container's address.

The third mode, `worker`, reads `X-Yokohama-User`. It is the header the Cloudflare host's own Worker sets
for its objects. An identity-aware proxy of your own can play that part. It must:
- authenticate the person;
- remove any copy of the header the client sent;
- be the only path to the port.

**The Cloudflare Worker host has more sources than this server** (the auth cycle; `worker/README.md` §1.5). Its
`worker` door may also run `oidc` against an issuer that publishes a discovery document and accepts
`client_secret_post` or PKCE (an issuer that accepts only `client_secret_basic` cannot be configured this cycle,
R-20), with `https` endpoints (a loopback host is accepted for development) — or `workos`, WorkOS AuthKit's
first-party API — and mints its own session cookie after the sign-in. **This server gains no sign-in arm and no
`/api/leave` route**: under `cf-access` a sign-out falls back to Access's own logout, `/cdn-cgi/access/logout`;
under its other modes the client forgets the name. Its `cf-access` door and the one rule it rests on, above,
are unchanged.

**Verify** as yourself, through the tunnel: `/api/me` must name you — `{"mode":"cf-access","user":"<your
address>"}` — and a caller Access admits without an address, such as a service token, must read
`"user":null` ([`RUNBOOK.md`](RUNBOOK.md) §3.2 is the recipe). `/api/health` answers the same whether the
door works or not, so it proves nothing about the door.

**When the page says nothing arrived.** Under `cf-access`, a browser the server cannot name reads "This
server couldn't tell who you are", and the card's "For the operator" fold says: "This server expects
Cloudflare Access to name each player (Cf-Access-Authenticated-User-Email), and nothing arrived." The
server answered, and no address reached it. Look for one of these:
- the page was reached some way that does not pass Access: a LAN address, a published port, or a
  hostname the Access application does not cover;
- the caller is one Access admits without an address, a service token among them;
- the header held something that is not an address, which this door reads as no one.

The cure is the hostname Access covers, as a person Access knows. A home network with no identity
provider at all is the open door's case, which follows.

## 2. The open door: a home network, a LAN or a VPN

`YOKOHAMA_WEB_IDENTITY=open` is for a deployment with **no identity provider at all**. The first screen
asks for a name, and the name is who you are. You do not need to set up an identity service to play at
home.

**The trust model, in full: anyone who can reach the server may use any name.**
- A name is a label and a key, never a credential: two people who type the same name are the same player
  to this server, and nothing stops one person typing another's.
- Your tables stay with the name you sat down as, so a different name later is a different player, whose
  tables are empty.
- There are no accounts and no house password.

So bind the port only where everyone who can reach it is someone you would hand the table to: your home
network, a LAN, or a VPN. Never put an open door on the public internet.

On a LAN, list the name or address your browsers type in `YOKOHAMA_WEB_HOSTNAMES` (§4), or set `YOKOHAMA_WEB_HOSTNAMES_ENABLED=off` when you do not want the host boundary.

**Names:**
- 1–32 characters of ASCII letters, digits, spaces, `.`, `-` and `_`, beginning with a letter or a digit;
- an address is refused here: under the open door a name is a name, and an `@` always means a verified address on every other door (security S2b, C1);
- case and extra spaces do not count: `"  Ben   Castle "` and `ben castle` are one player, keyed
  `ben castle`; each browser shows the name as its player typed it, spaces tidied, so the first reads
  `Ben Castle`;
- `anonymous` is not a name.

The corner of the lobby and the setup screens reads "Playing as" and the name, and its action is
**switch player**, which forgets the name and asks for another.

**A Turnstile site key goes with this door and no other, on this server.** A client built with
`YOKOHAMA_TURNSTILE_SITEKEY` (§3) holds a nameless visitor's finished game for a name only where the server's
facts say `open`; it no longer reads the key as the door (the auth cycle, Task 6). So build with a key only for
`YOKOHAMA_WEB_IDENTITY=open`: the server reads the build's stamp at start and refuses a keyed build under
`cf-access` or `worker`, naming both. (The Cloudflare Worker's provider-signed door is the one other place a
keyed build is accepted, `worker/README.md` §1.5.)

**Changing one origin's door.** A browser keeps the name it typed as a display name, per origin. So an
operator who flips one origin from `open` to `cf-access` sees that stored display name in the corner
until "change name" replaces it, while every seat is keyed by the Access address. Two hosts at two
hostnames are two origins and share nothing.

**A build that must never run open:** `cargo build --release -p yokohama-web --no-default-features
--features server` refuses `YOKOHAMA_WEB_IDENTITY=open` at start-up, by name. The `Dockerfile` builds with
the default features, so its image includes the open door.

## 3. Build the image

From the repository root:

```bash
docker build --platform linux/amd64 -f deploy/Dockerfile -t yokohama-web:0.6.3-s2 .
```

It compiles the server and the browser module on the pinned Rust toolchain, then builds the client. The
image is `linux/amd64` by design, the architecture of the owner's NAS and of CI's runner, and the build
fetches an x86_64 `wasm-opt`; on an arm64 machine the `--platform` flag has Docker build it under
emulation. A
first build takes a while; a rebuild after a source edit reuses the toolchain layers. On the owner's
workstation (Apple silicon, the `linux/amd64` build under emulation) one build with only the base images
cached took 230 s, and `docker images` read the image's content as 42.4 MB (151.8 MB once unpacked),
a reading of 2026-09-28 (M6.5c2).
The build runs under BuildKit (Docker 23's default), which reads `deploy/Dockerfile.dockerignore` as
this Dockerfile's own ignore file: it decides what the build sees, and keeps the secrets files
`.dev.vars` and `.env`, with their dotted variants, out of the context wherever they sit, and the
pinned-browser instrument's profiles with them (security S2a). The legacy builder reads no such file.

**The build values** are build arguments: the door's three, and the release feed, the Turnstile site key and the
site's own address beside them. Each is optional, and each is checked by the build:

```bash
docker build --platform linux/amd64 -f deploy/Dockerfile \
  --build-arg YOKOHAMA_PROJECT_URL=https://example.org/yokohama \
  --build-arg YOKOHAMA_SITE_BLURB="A few words under the title." \
  --build-arg YOKOHAMA_SITE_NOTICE="Not affiliated with Asmodee" \
  -t yokohama-web:0.6.3-s2 .
```

| Argument | Maximum | What it draws |
|---|---|---|
| `YOKOHAMA_PROJECT_URL` | 200 characters | A link under the title on the first screen, and a quiet link among the lobby's actions. An absolute `https://` or `http://` URL with a host, holding no whitespace or control character. |
| `YOKOHAMA_SITE_BLURB` | 400 characters | A paragraph under the title on the first screen. |
| `YOKOHAMA_SITE_NOTICE` | 200 characters | One line at the foot of every screen. |
| `YOKOHAMA_RELEASE_FEED` | 300 characters | Where the first screen and the lobby look for the latest release, to say when it is newer than the one running. **Three states:** unset or empty derives the feed from `YOKOHAMA_PROJECT_URL` when that names a GitHub repository (`https://github.com/<owner>/<repo>`); a URL is that feed, `https://` only, answering GitHub's release shape; the literal `off` is no feed and no check. |
| `YOKOHAMA_TURNSTILE_SITEKEY` | 100 characters | The public site key of a Turnstile check on the way to the lobby. Plain text, no whitespace or control character. Blank, the default, draws the plain link. It belongs only with `YOKOHAMA_WEB_IDENTITY=open` (§2) on this server, and the server refuses to start a keyed build under any other door, naming both. The browser does not read the key as the door: it reads the server's facts, and holds a visitor with no name of their own for a name only where those facts say `open`; with the facts unread it asks the server's health first and may show the widget once, at a fresh context's first game over (the Cloudflare Worker's provider-signed door is the other place a keyed build is accepted, `worker/README.md` §1.5). **Behind Access, leave it blank:** the widget is the no-Access Worker's, where an edge rule stands in front of the open door. **A keyed image is a manual build for an open-door host:** `deploy/update.sh` (§9) refuses a key, because the image smoke starts every image under `cf-access`, which refuses a keyed build. |
| `YOKOHAMA_SITE_URL` | 200 characters | The site's own address, the one a search engine should know it by: an `https://` origin such as `https://yokohama.example.org`, with no path, query, fragment or credentials (a trailing `/` is dropped). With it the build writes `sitemap.xml`, the page's canonical link and `og:url`, and `robots.txt` names the sitemap; without it none of the three. |

- **Omit an argument, or leave it empty, for none of that value** — except the release feed, where unset and empty
  are the derived default and only `off` is none.
- **The release feed is read in the visitor's browser**, once per page and, after a read, answered or refused, not
  again in that browser for an hour. Your server never sees the request; the feed's host sees it, from the visitor's
  address.
- **The web presence is built from these values** (`web/scripts/presence.mjs`, the last step of `npm run build:client`):
  `robots.txt` (everything allowed but `/api/`, `/enter` and `/cdn-cgi/`), `llms.txt` (what the site is, in the
  `llmstxt.org` shape, with the repository link only where `YOKOHAMA_PROJECT_URL` is set), the page's description, Open
  Graph and Twitter-card tags and one JSON-LD block. They are static files, served without a request to anything else
  and left out of the offline release. Nothing in them names an address but the one you gave as `YOKOHAMA_SITE_URL`.
- The blurb and the notice must hold words and no control character (a line break, a zero-width character or a bidirectional control all count). A value the build
  refuses stops the build with a message naming the variable.

**Why these are fixed at build time and not at run time:** the whole release is precached for offline
play under a name that is a digest of its own bytes, and every browser checks each file against that
digest when it installs. A server that rewrote `index.html` at start-up would fail that check in every
browser. So each value is baked into the release, and changing one is a rebuild.

## 4. Run it

```bash
docker run -d --name yokohama-web \
  -p 127.0.0.1:8080:8080 \
  -v yokohama-web-data:/data \
  -e YOKOHAMA_WEB_IDENTITY=cf-access \
  -e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on \
  -e YOKOHAMA_WEB_HOSTNAMES=<your hostname> \
  --restart unless-stopped \
  --cap-drop ALL --security-opt no-new-privileges:true --memory 512m \
  yokohama-web:0.6.3-s2
```

- **`/data` is the one thing that must outlive the container.** Every applied move is written there as the
  game's record, and a game is rebuilt from it. Use a named volume: the server runs as uid 10001, and a
  named volume takes the image's ownership on first use, where a host bind mount arrives owned by root.
- **`127.0.0.1:8080`** keeps the port on this machine, for a tunnel or a proxy on the same host. With
  `cf-access`, §1's recipe forbids a published port — a private network and `--network cf-internal` in place of
  `-p` — and this example is the case without a tunnel network, cloudflared running on the host, say. On a
  machine others share, loopback is a path too: under `cf-access` any process there can send the header
  and be anyone. A Docker network the tunnel shares, with nothing published, keeps the port off the
  host's published ports and off loopback (§6), but on a Linux host any process that can route to the
  container's address, and any other container on that network, can still send the header. So on a
  shared machine the port is only as trusted as the host's users: no door this server has closes that
  path, since `worker` too trusts a header it does not verify. For the open door on a LAN, bind the
  LAN address you mean — never `0.0.0.0` on a machine with a public interface.

| Variable | Values | Unset means |
|---|---|---|
| `YOKOHAMA_WEB_IDENTITY` | `cf-access`, `open`, `worker` | the server refuses to start |
| `YOKOHAMA_WEB_HOSTNAMES_ENABLED` | `on`, `off` | `on`. On, the server answers only the host names below and refuses any other `Host` with 421. Off, it answers any `Host`, as it did before 0.7.2, and the start line says so. Any other value stops the server at start. Write `on` beside the list, so the switch is visible to whoever reads your deploy file |
| `YOKOHAMA_WEB_HOSTNAMES` | host names separated by commas, e.g. `yokohama-nas.moore.nyc` | `localhost`, `127.0.0.1` and `::1`, plus the bind address when it is a specific one. A list adds to that set and never replaces it. An entry is a name or an IPv6 literal written without brackets, with no space, scheme, path, user or port, and an unlisted `Host` is 421. The port of a request is never compared. The start line prints the names the server answers |
| `YOKOHAMA_WEB_MULTIPLAYER` | `on`, `off` | `on`. Off, the lobby, the tables and chat are not served at all; single player still works |
| `YOKOHAMA_WEB_ARCHIVE_DIR` | a directory, e.g. `/data/archive` | no archive. Set, every finished and abandoned game is written once under `rev7/over/` and `rev7/abandoned/`, as the record the CLI loads beside its table document; a browser's uploads are bounded — 4,096 events and 512 KiB a record, 256 records and 128 MiB a caller under `browser/<segment>/` (security S1b) — the segment is the address made readable, then `-` and a twelve-character digest (security S1c) |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | whole days above zero, e.g. `90` | no table expires and no sweep runs. Set, an hourly sweep ends every table idle this long through its creator's Abandon (§8). It needs `YOKOHAMA_WEB_ARCHIVE_DIR`: set without it, the server refuses to start |
| `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` | whole days above zero | `7`, and inert while `YOKOHAMA_WEB_TABLE_TTL_DAYS` is unset: how long a table with a seat still empty stands idle before the sweep closes it. A table at its deal, every seat taken, waits the first |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `on`, `off` | `on` |
| `YOKOHAMA_WEB_CHAT_KEEP` | a positive number of messages, or `all` | 500 a room |
| `YOKOHAMA_WEB_CHAT_TEXT` | a positive number of characters | 500 a message |

A bad value stops the server at start-up, naming the variable, and so does a table TTL with no archive
directory and a client built with `YOKOHAMA_TURNSTILE_SITEKEY` under any door but `open` (§2). `/api/health` reports the door, the transport, the archive, the chat limits and the two TTLs, so
a client sizes itself from the server's rules; `table_ttl_days` and `open_table_ttl_days` read `null` while
no TTL is set. The two TTLs are the Cloudflare host's names with its meanings (`worker/RUNBOOK.md` §3).

**The security headers come from the client build** (security S2a). The client build writes
`dist/headers.json` from its build values, and the server reads it from the client it serves, once
at start, then sets that header set on every answer: the content security policy, `X-Frame-Options:
DENY`, `X-Content-Type-Options: nosniff`, the referrer policy, a `Permissions-Policy` and
`Cross-Origin-Opener-Policy`, with `Cache-Control: no-store` beside them on every answer under `/api`.
Since 0.6.3 the policy is sent as `Content-Security-Policy`, enforced, with a fixed nonce for the
script Cloudflare injects (`web/scripts/headers.mjs`); 0.6.2 was built with it Report-Only and never
deployed. A client build without the file (older than 0.6.2, or none) serves no header set, and the
server says so once at start, on stderr. A malformed file
stops it at start, naming the file. The smoke (§5) reads `X-Frame-Options` and the policy on `/`, and
`no-store` and `nosniff` on `/api/health`.

**Why the host list exists, and the trap in it** (security S3, D4).
A web page on another site can make a browser send requests to your server under a name the attacker controls, by changing what that name resolves to.
The one thing that name cannot forge is the `Host` the browser sends, so the server answers only the names it is told to.
The image binds `0.0.0.0`, which is no name, so the default set inside a container is loopback alone.
A container reached through a tunnel or a proxy with no list answers every request 421, while a smoke run on loopback passes.
So list the name your browsers use, and read your hostname's `/api/health` before you call a deploy done.
If you do not want the boundary, set `YOKOHAMA_WEB_HOSTNAMES_ENABLED=off`.

The image sets its own paths, which you do not need to touch: `YOKOHAMA_WEB_HOST=0.0.0.0` and
`YOKOHAMA_WEB_PORT=8080` inside the container; `YOKOHAMA_WEB_DIST`, `YOKOHAMA_WEB_ART` and
`YOKOHAMA_WEB_DIR=/data/games`.

## 5. Check the image before you trust it

```bash
bash deploy/smoke-image.sh yokohama-web:0.6.3-s2      # or: … <image> <port> [hostname], port default 18080
```

Give a hostname as the third argument to check the host list too.
The smoke then runs the containers with `YOKOHAMA_WEB_HOSTNAMES_ENABLED=on` and that hostname listed.
It reads `/api/health` under each door as `Host: <hostname>`, which must answer 200, and as `Host: smoke-foreign.example`, which must answer 421.
Without it the smoke says the host probe was not asked for and runs as it always has, on loopback.

It needs `docker`, `curl` and `python3`, and it starts the image three times, publishing only on
`127.0.0.1`:
- with no door, which must refuse to start, exit 1 and name `YOKOHAMA_WEB_IDENTITY`;
- under `cf-access`, which must read only Access's header, and only an address in it;
- under `open`, which must key a typed name as the server keys it.

Under each door it reads `/api/health`, `/api/me` and `/api/games` with and without a caller, and fetches
the client and one art file. It prints each serving container's memory, the image's size as Docker
reports it, and `smoke: PASS` — or `smoke: FAIL` and the first thing that was wrong. Every container it
starts is removed with its volume. CI runs the same script on the image it builds without a project
link, and checks the image it builds with one for the link alone (`.github/workflows/docker.yml`).

## 6. Behind a proxy

- **Cloudflare Tunnel with Access** (`cf-access`): route the hostname to the container in cloudflared's
  ingress — `http://localhost:8080` when cloudflared runs on the same host, or the container's name on a
  Docker network the two share, with no port published at all, as the owner's NAS does (§1's recipe; the
  `http://localhost:8080` form publishes the port that recipe forbids) — and add the
  hostname to a self-hosted Access application. `RUNBOOK.md` §3 and `cloudflared-config.yml` are the
  owner's working example, the DNS step included (§3.1).
- **Any other proxy:** the server serves plain HTTP, including server-sent events on
  `/api/games/<id>/stream` and `/api/stream`. Keep responses unbuffered for those paths; the server sends a
  heartbeat every 25 s. Behind an identity-aware proxy, use `worker` (§1). Behind a plain TLS proxy on a
  private network, use `open` (§2).

## 7. What the browser keeps

The release — the client, the WebAssembly module, the fonts, the help pictures and the card art — is
precached whole on a first visit. The door and a game against the bot then work offline, and the app
installs to a home screen. A single-player game lives in the browser's own storage until it is finished,
then goes to the archive if the server keeps one. A new image is a new release, and a returning browser
finds it by itself: the door installs it in the background and reloads once, with no click, while a
game in play shows "A new version is ready. Reload to use it" and changes nothing until it is clicked.
§9 says what each person sees when you update.

## 8. Cleaning out old games

**The sweep.** Set `YOKOHAMA_WEB_TABLE_TTL_DAYS`, and `YOKOHAMA_WEB_ARCHIVE_DIR` with it, and the server ends
idle tables itself: one pass a minute after start, then one every hour. A table's last activity is the latest
write to any of its three files (the table, the record and the chat, listed below): a seat taken, a keep, an
applied move, a chat message. A pass ends:

- a table still waiting for players, a seat empty, once it has stood idle `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`
  (7 unless set), with the reason `waited`;
- any table once it has stood idle `YOKOHAMA_WEB_TABLE_TTL_DAYS`, with the reason `expired`. A table at its
  deal, every seat taken, waits this one.

**Each pass lists before it ends anything.** It writes one `sweep-listed` line for every table it is about to
end, and only then ends them, one `sweep-ended` line each: a pass that ends forty tables has written forty
lines before the first one goes. Each ending is the creator's own **Abandon**. A finished game is already in
the archive from game over, so it is not filed twice. A game in play is filed under `rev7/abandoned/`.
A table where no move was played files nothing. If the archive cannot be written, the delete
is refused, a `sweep-failed` line says why, and the table stays for the next pass. A table the sweep cannot
read gets a `sweep-unread` line and stays. The ended table's link answers "no such game": this server leaves
no marker in its place, and the log line is the record.

**A sweep never throws a game away.** A TTL with no archive directory stops the server at start-up:

```
YOKOHAMA_WEB_TABLE_TTL_DAYS is set but YOKOHAMA_WEB_ARCHIVE_DIR is not — a sweep without an archive throws games away; set both or neither
```

A value that is not a whole number of days stops it too, naming the variable:

```
YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole number of days above zero — got "x"
YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS must be a whole number of days above zero — got "x"
```

**The lines** are in the container's log (`docker logs <container>`), one JSON object each:

```
{"event":"sweep-listed","id":"<id>","idle_days":"91","reason":"expired","status":"playing"}
{"event":"sweep-ended","id":"<id>","kept":"archive","reason":"expired"}
{"event":"sweep-failed","error":"<the delete's reason>","id":"<id>","reason":"expired"}
{"event":"sweep-unread","error":"<the reason>","id":"<id>"}
```

`kept` is `archive` when the game is in the archive, `nothing` when the table held no game, and `off` for a
game with no archive to keep it, which a server that sweeps never has.

**A move during a sweep.** A move can land in the seconds between a pass's listing and its delete, and the
table is ended all the same, but the move is not lost. The delete lays its tombstone (its mark that the table is
going) before it reads the game it files, under the lock a move's save holds from its check of that mark to its
write. A move whose save began before the tombstone is filed with the game like every move before it. A move
whose save begins after it is refused out loud. Every Abandon works the same way.

**The sweep reads the files' modification times**, so a copy or a restore of the volume that resets them
restarts every table's clock.

**With no TTL set, no sweep runs.** A table lives until its creator abandons it or you remove it, and
`/api/health` says so (`table_ttl_days: null`).

**What is on disk.** Under `/data` (the volume §4's run command names, `yokohama-web-data`):

| Path | What it is |
|---|---|
| `/data/games/<id>.json` | the game's record — the save file; every applied move is written here |
| `/data/games/<id>.table.json` | the table: who sits where, the status, the lock, the setup seed |
| `/data/games/<id>.chat.json` | the table's chat |
| `/data/games/_house.chat.json` | the lobby chat — never a game; leave it |
| `/data/archive/rev7/over/`, `/data/archive/rev7/abandoned/`; a browser's uploads under `/data/archive/browser/<segment>/rev7/{over,abandoned}/` | the archive, when `YOKOHAMA_WEB_ARCHIVE_DIR` is set: one copy of each finished or abandoned game, with its table document; a browser's uploads under `browser/<segment>/` (the segment is the address made readable, then `-` and a twelve-character digest, security S1c) stop at 4,096 events and 512 KiB a record, 256 records and 128 MiB a caller (security S1b) |

**The preferred way** to end one table is the creator's **Abandon** in the lobby: it files the archive copy
(once, even for a finished game) and removes the table, as the sweep does. Everything below is for a server
with no TTL and tables nobody will abandon.

**By hand, with any Docker.** The server keeps a running game's handle in memory and would write its record
again on the next move, and a file removed under it is a cross-process conflict the server detects rather than
prevents — so stop the container first.

```bash
# 1. list the tables idle for more than N days (the container's name from §4; N=90 here)
docker exec <container> find /data/games -name '*.table.json' -mtime +90
# 2. keep what you want: a finished game's record is the CLI's save file
docker cp <container>:/data/games/<id>.json ./<id>.json
# 3. stop, remove the three files of each id, start
docker stop <container>
docker run --rm -v yokohama-web-data:/data alpine sh -c 'rm /data/games/<id>.json /data/games/<id>.table.json /data/games/<id>.chat.json'
docker start <container>
```

A table removed this way leaves the lobby at once and leaves no marker: its link answers "no such game". A game
removed mid-play is NOT archived — only a game over and an Abandon file there — so step 2 is the only copy.
The archive directory is never part of a sweep; it is the keep.

## 9. Updating

`deploy/update.sh` moves a running container to a release tag. From a checkout of this repository:

```bash
YOKOHAMA_WEB_BUILD_ENV=~/yokohama/build.env \
YOKOHAMA_WEB_ENV=~/yokohama/run.env \
  bash deploy/update.sh --dry-run <tag>     # print every command, run none
YOKOHAMA_WEB_BUILD_ENV=~/yokohama/build.env \
YOKOHAMA_WEB_ENV=~/yokohama/run.env \
  bash deploy/update.sh <tag>
```

**What it does, in order**, stopping at the first failure:
1. Refuses a working tree with uncommitted changes to tracked files, because the image is built from the tree and
   an edit would ship under the tag's name.
2. `git fetch --tags`, then `git checkout --detach refs/tags/<tag>`, so a branch of the same name is never built.
3. `docker build --platform linux/amd64 -f deploy/Dockerfile`, with the build values from `YOKOHAMA_WEB_BUILD_ENV`
   as `--build-arg`s (§3), tagged `yokohama-web:<tag>`.
4. `deploy/smoke-image.sh` on that image (§5), **while the old container is still serving**.
5. Only then the swap: `docker stop`, `docker rm`, and `docker run` with `--env-file "$YOKOHAMA_WEB_ENV"`, the
   named volume at `/data`, and §4's other arguments.
6. Reads the start rather than assuming it. `docker run -d` returns before the server has read its environment,
   so the script polls the new container for up to 30 seconds, every 2: until `docker inspect` reads it running,
   not restarting and never restarted, and its log holds the server's "listening on" line. Otherwise it prints the
   container's last log lines, the rollback line naming the previous image, and **exits 1**.

So **a build or a smoke that fails leaves the old container exactly as it was**, and the volume is never touched:
the script removes only the stopped container, never a file or a directory. Every earlier image stays on the
machine for a rollback. A container that does not come up is reported by that read, and the old one is already
gone by then: the run file is the usual cause (a table TTL with no archive directory, a bad chat value), which the
smoke cannot see, since it starts the image with its own environment.

**The two files** are `NAME=value` lines, each value taken literally to the end of its line, with no quotes and no
shell expansion; blank lines and lines starting `#` are skipped.
- **`YOKOHAMA_WEB_BUILD_ENV`** may hold the six build values of §3 and nothing else: a name that is not one of them
  is refused, naming it. An empty file builds with none. **It builds keyless images only:** a non-empty
  `YOKOHAMA_TURNSTILE_SITEKEY` is refused up front, before the fetch, because the smoke starts every image under
  `cf-access` and the server refuses to start a keyed build there (§2). A keyed image is a manual build for an
  open-door host (§3).
- **`YOKOHAMA_WEB_ENV`** is the container's environment, `YOKOHAMA_WEB_IDENTITY` first: the server refuses to start
  without it, and §4's table is the rest.

Both are required; a real run refuses without them before it runs anything. Four more variables are optional and
default to §4's run command: `YOKOHAMA_WEB_CONTAINER` (`yokohama-web`), `YOKOHAMA_WEB_VOLUME`
(`yokohama-web-data`), `YOKOHAMA_WEB_PUBLISH` (`127.0.0.1:8080:8080`; set it empty to publish no port) and
`YOKOHAMA_WEB_NETWORK` (Docker's default network; §6's shared network for a tunnel). The script needs `git`,
`docker`, `curl` and `python3`.

**`--dry-run`** prints each command and runs none: no fetch, no checkout, no build, no container change, not even a
read of Docker or git. Read it once before the first real run, and again after changing either file.

**What people see when the container changes:**
- **The door updates itself.** A browser that comes back finds the new release on its next visit, when a tab returns
  into view or within 15 minutes of an open page. The door installs it in the background and reloads once, with no
  click. The client is the same code on every host, and the update was read live on the Cloudflare host at cycle F's
  revert: a plain reload to "Ready to play offline" on the new release in seven seconds.
- **A game in play shows a line, and nothing reloads.** The lobby and a game screen draw "A new version is ready.
  Reload to use it", and the click is the only switch. It is safe: a game against the bot resumes from the browser's
  storage, and a table game rejoins by its link.
- **The swap itself is short.** On `docker stop`'s SIGTERM the server stops taking connections and lets in-flight
  requests finish, and a move the server has answered is already on `/data` (§4); a server-sent-events stream
  reconnects by itself. The owner's NAS, swapped by its own script through Portainer, read 1.6 seconds from the old
  container's stop to the new one's "listening" line (`RUNBOOK.md` §3.20).
- **A game not yet over keeps its bot or is refused.** The server opens a game exactly when the new release can
  reconstruct the bot recorded in it, and a game it cannot is refused with the reason, never played as another bot
  ([`docs/web-lane.md`](../docs/web-lane.md#6-what-a-save-file-says) §6).

**To roll back**, run the previous image again; the volume is the same:

```bash
docker image ls yokohama-web                     # every earlier tag is still here
docker stop yokohama-web && docker rm yokohama-web
docker run -d --name yokohama-web --network cf-internal -v yokohama-web-data:/data \
  --env-file ~/yokohama/run.env --restart unless-stopped \
  --cap-drop ALL --security-opt no-new-privileges:true --memory 512m yokohama-web:<previous tag>
```

That is §1's form, for a `cf-access` host: no port published. A host with no tunnel network, §4's example and the
script's default, runs it with `-p 127.0.0.1:8080:8080` in place of `--network cf-internal`.

The script names the previous image when it swaps. [`RUNBOOK.md`](RUNBOOK.md) §5 lists what each release changed on
the volume that an older binary cannot read.
