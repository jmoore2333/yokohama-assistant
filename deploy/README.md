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

**The one rule this door rests on:** the tunnel must be the ONLY path to the container's port. The server
does not verify Access's signed assertion (`Cf-Access-Jwt-Assertion`), so anything that reaches port 8080
without passing Access can send the header and be anyone. Publish the port to nothing but the tunnel.

The third mode, `worker`, reads `X-Yokohama-User`. It is the header the Cloudflare host's own Worker sets
for its objects. An identity-aware proxy of your own can play that part. It must:
- authenticate the person;
- remove any copy of the header the client sent;
- be the only path to the port.

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

**Names:**
- 1–32 characters of ASCII letters, digits, spaces, `.`, `-` and `_`, beginning with a letter or a digit;
- an email address is also a name;
- case and extra spaces do not count: `"  Ben   Castle "` and `ben castle` are one player, keyed
  `ben castle`; each browser shows the name as its player typed it, spaces tidied, so the first reads
  `Ben Castle`;
- `anonymous` is not a name.

The corner of the lobby and the setup screens reads "Playing as" and the name, and its action is
**switch player**, which forgets the name and asks for another.

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
docker build --platform linux/amd64 -f deploy/Dockerfile -t yokohama-web:0.6.0 .
```

It compiles the server and the browser module on the pinned Rust toolchain, then builds the client. The
image is `linux/amd64` by design, the architecture of the owner's NAS and of CI's runner, and the build
fetches an x86_64 `wasm-opt`; on an arm64 machine the `--platform` flag has Docker build it under
emulation. A
first build takes a while; a rebuild after a source edit reuses the toolchain layers. On the owner's
workstation (Apple silicon, the `linux/amd64` build under emulation) one build with only the base images
cached took 230 s, and `docker images` read the image's content as 42.4 MB (151.8 MB once unpacked),
a reading of 2026-09-28 (M6.5c2).
`deploy/Dockerfile.dockerignore` decides what the build sees.

**The build values** are build arguments: the door's three, and the release feed and the Turnstile site key
beside them. Each is optional, and each is checked by the build:

```bash
docker build --platform linux/amd64 -f deploy/Dockerfile \
  --build-arg YOKOHAMA_PROJECT_URL=https://example.org/yokohama \
  --build-arg YOKOHAMA_SITE_BLURB="A few words under the title." \
  --build-arg YOKOHAMA_SITE_NOTICE="Not affiliated with Asmodee" \
  -t yokohama-web:0.6.0 .
```

| Argument | Maximum | What it draws |
|---|---|---|
| `YOKOHAMA_PROJECT_URL` | 200 characters | A link under the title on the first screen, and a quiet link among the lobby's actions. An absolute `https://` or `http://` URL with a host, holding no whitespace or control character. |
| `YOKOHAMA_SITE_BLURB` | 400 characters | A paragraph under the title on the first screen. |
| `YOKOHAMA_SITE_NOTICE` | 200 characters | One line at the foot of every screen. |
| `YOKOHAMA_RELEASE_FEED` | 300 characters | Where the first screen and the lobby look for the latest release, to say when it is newer than the one running. **Three states:** unset or empty derives the feed from `YOKOHAMA_PROJECT_URL` when that names a GitHub repository (`https://github.com/<owner>/<repo>`); a URL is that feed, `https://` only, answering GitHub's release shape; the literal `off` is no feed and no check. |
| `YOKOHAMA_TURNSTILE_SITEKEY` | 100 characters | The public site key of a Turnstile check on the way to the lobby. Plain text, no whitespace or control character. Blank, the default, draws the plain link. |

- **Omit an argument, or leave it empty, for none of that value** — except the release feed, where unset and empty
  are the derived default and only `off` is none.
- **The release feed is read in the visitor's browser**, once per page and, after a read, answered or refused, not
  again in that browser for an hour. Your server never sees the request; the feed's host sees it, from the visitor's
  address.
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
  --restart unless-stopped \
  yokohama-web:0.6.0
```

- **`/data` is the one thing that must outlive the container.** Every applied move is written there as the
  game's record, and a game is rebuilt from it. Use a named volume: the server runs as uid 10001, and a
  named volume takes the image's ownership on first use, where a host bind mount arrives owned by root.
- **`127.0.0.1:8080`** keeps the port on this machine, for a tunnel or a proxy on the same host. On a
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
| `YOKOHAMA_WEB_MULTIPLAYER` | `on`, `off` | `on`. Off, the lobby, the tables and chat are not served at all; single player still works |
| `YOKOHAMA_WEB_ARCHIVE_DIR` | a directory, e.g. `/data/archive` | no archive. Set, every finished and abandoned game is written once under `rev7/over/` and `rev7/abandoned/`, as the record the CLI loads beside its table document |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `on`, `off` | `on` |
| `YOKOHAMA_WEB_CHAT_KEEP` | a positive number of messages, or `all` | 500 a room |
| `YOKOHAMA_WEB_CHAT_TEXT` | a positive number of characters | 500 a message |

A bad value stops the server at start-up, naming the variable. `/api/health` reports the door, the
transport, the archive and the chat limits, so a client sizes itself from the server's rules. No
table expires on this server: it reads no TTL, and `/api/health` reports `table_ttl_days` and
`open_table_ttl_days` as `null` (the Worker's two TTLs are its own, `worker/RUNBOOK.md` §3).

The image sets its own paths, which you do not need to touch: `YOKOHAMA_WEB_HOST=0.0.0.0` and
`YOKOHAMA_WEB_PORT=8080` inside the container; `YOKOHAMA_WEB_DIST`, `YOKOHAMA_WEB_ART` and
`YOKOHAMA_WEB_DIR=/data/games`.

## 5. Check the image before you trust it

```bash
bash deploy/smoke-image.sh yokohama-web:0.6.0          # or: … <image> <port>, default 18080
```

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
  Docker network the two share, with no port published at all, as the owner's NAS does — and add the
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
then goes to the archive if the server keeps one. A new image is a new release: an open tab offers
"Reload to use it" on the door, and a waiting release nobody clicks activates by itself
once every old page has closed.

## 8. Cleaning out old games by hand

**No sweep runs on this server.** It reads no TTL: a table lives until its creator abandons it or you remove
it, and `/api/health` says so (`table_ttl_days: null`). The Cloudflare host is the one with expiry
(`worker/RUNBOOK.md` §3). A sweep here — a periodic job over the games directory that ends idle tables through
the same delete the creator's Abandon uses, so a finished game's archive copy lands exactly once — is a
potential feature if there is a desire for it; it is not built.

**What is on disk.** Under `/data` (the volume §4's run command names, `yokohama-web-data`):

| Path | What it is |
|---|---|
| `/data/games/<id>.json` | the game's record — the save file; every applied move is written here |
| `/data/games/<id>.table.json` | the table: who sits where, the status, the lock, the setup seed |
| `/data/games/<id>.chat.json` | the table's chat |
| `/data/games/_house.chat.json` | the lobby chat — never a game; leave it |
| `/data/archive/rev7/over/`, `/data/archive/rev7/abandoned/` | the archive, when `YOKOHAMA_WEB_ARCHIVE_DIR` is set: one copy of each finished or abandoned game, with its table document |

The table document's modification time is its last write — a keep, a status change, and every applied move
refreshes it — so it is the "last activity" a sweep would read.

**The preferred way** is the creator's **Abandon** in the lobby: it files the archive copy (once, even for a
finished game), leaves a marker, and removes the table. Everything below is for tables nobody will abandon.

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
