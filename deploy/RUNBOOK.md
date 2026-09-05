# Deploying the Yokohama browser client

**This is the record of a deployment that happened, not a plan.** The client went
live at `https://yokohama.moore.nyc` on 2026-09-01, served by a container on the
Synology NAS behind the existing cloudflared tunnel. Where an earlier draft of
this file guessed wrong, the guess is kept beside the correction — the wrong
guesses are the useful part.

---

## 0. What is true before you start

- **`yokohama.moore.nyc` is behind Cloudflare Access.** Verified, not assumed:
  it answers 200 and issues a `CF_Authorization` token from
  `bdvc.cloudflareaccess.com`. **The client inherits that policy by taking the
  hostname and needs no Access application of its own.** (Both M4 documents
  carried this as UNVERIFIED; it is now settled.)
- **The server has no authentication of any kind.** No session, no header read,
  and `GET /api/games` returns every game id to any caller. Everything that
  makes it safe to expose is the Access application in front of it. Remove that
  and the game store is public.
- **The tunnel is LOCALLY managed.** `/volume1/docker/cloudflared/config/config.yml`
  on the NAS is the source of truth for ingress; the Zero Trust dashboard shows
  its routes read-only. A copy of what is deployed lives here as
  `cloudflared-config.yml` — that copy is documentation, and the file on the NAS
  is the thing that runs.
- **The NAS is an Intel Atom C2538: 4 cores, 16.8 GB, DSM 7.4, kernel 3.10.108,
  Docker 24.0.2, x86_64.**
- **The image contains carved-out third-party art** (`ASSETS.md`). Private
  registries only; never a public one.

---

## 1. Build — a musl cross-compile on the workstation

**Correction to the first draft, which said to use `docker buildx --platform
linux/amd64`.** That works and is slow (emulated Rust release build on Apple
silicon). It is also unnecessary: this workspace has **no C dependencies** — the
server's tree is axum, tokio and serde — so `rustc` can cross-compile and link a
fully static binary on macOS using the toolchain's own `rust-lld`, in seconds.

```bash
rustup target add x86_64-unknown-linux-musl   # onto the PINNED toolchain
LLD="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin/rust-lld"
RUSTFLAGS="-C linker=$LLD -C link-self-contained=yes" \
  cargo build --release --target x86_64-unknown-linux-musl -p yokohama-web

cd web && npm run build && cd ..     # dist/ is architecture-independent
```

**Two things that will bite without this shape.** Plain
`cargo build --target x86_64-unknown-linux-musl` on macOS **compiles fine and
fails at LINK**, because rustc reaches for the host `cc`, and Apple's `ld` does
not understand `--as-needed`, `-Bstatic`, `--gc-sections` and friends. And
`rustup target add` must land on the toolchain `rust-toolchain.toml` pins — the
release workflow already carries that lesson (commit `246d4f0`).

**Why musl and not Debian.** The NAS runs **kernel 3.10.108**. A modern-glibc
runtime is a real risk there, not a theoretical one. A static musl binary
carries its own libc and asks the kernel for very little, which is also how this
repo already ships CLI releases. Result: `ELF 64-bit LSB pie executable,
x86-64, static-pie linked`, 5.8 MB, and it started first try.

### 1.1 ARM is the same command with a different target — MEASURED

The NAS is x86_64, so that is what is deployed. But most always-free cloud
compute is ARM (Oracle's Ampere A1, AWS Graviton), and it would be easy to
assume that is a porting question. It is not:

```bash
rustup target add aarch64-unknown-linux-musl
RUSTFLAGS="-C linker=$LLD -C link-self-contained=yes" \
  cargo build --release --target aarch64-unknown-linux-musl -p yokohama-web
```

**16.3 seconds, clean, `ELF 64-bit LSB executable, ARM aarch64, statically
linked`, 5.8 MB** — the same size as the x86_64 build. Nothing else changed.
This works for the same reason the musl cross-compile works at all: the server's
dependency tree is axum, tokio and serde with **no C dependencies**, so there is
no cross-toolchain to source, only a linker to name.

Recorded here because the negative is the useful part: **architecture is not a
constraint on where this can run**, and a hosting decision should never be made
on it.

---

## 2. Deliver — no registry, no image tarball upload

There is no registry between the workstation and the NAS, and the Docker API's
`POST /images/load` needs a binary body that the Portainer MCP proxy cannot
carry. **`POST /build?remote=<url>` can, because the NAS fetches the context
itself.** So the expensive work stays on the workstation and the NAS receives a
few megabytes it assembles in seconds.

```bash
# a context of exactly what the runtime image needs
mkdir ctx && cp target/x86_64-unknown-linux-musl/release/yokohama-web ctx/
cp -R web/dist ctx/dist && cp -R crates/yokohama-web/assets/art ctx/art
cp deploy/Dockerfile.runtime ctx/Dockerfile
tar czf ctx.tar.gz -C ctx .              # ~6.7 MB

python3 -m http.server 8899 --bind 0.0.0.0   # on the LAN, e.g. 192.168.1.16
```

Then, through Portainer's Docker proxy:

```
POST /build?remote=http://<workstation-lan-ip>:8899/ctx.tar.gz&t=yokohama-web:0.5.0&dockerfile=Dockerfile&rm=true
```

Resulting image: **19 MB**. Stop the file server afterwards.

`deploy/Dockerfile` is the OTHER path — a self-contained multi-stage build that
compiles everything from a repo checkout. It is what Modal builds from (§4) and
what to use where a registry exists. `Dockerfile.runtime` is the two-second
assembly step for artifacts built elsewhere.

---

## 3. Run it, and take the hostname

**The container joins `cf-internal`** — the network `cloudflared-nas` shares
with the services it serves. (The config file's recipe said `cf-edge`; the
deployment never matched it, and the file has been corrected.)

```
POST /containers/create?name=yokohama-web
{ "Image": "yokohama-web:0.5.0",
  "HostConfig": { "NetworkMode": "cf-internal",
                  "RestartPolicy": { "Name": "unless-stopped" },
                  "Mounts": [{ "Type": "volume", "Source": "yokohama-web-data",
                               "Target": "/data" }] } }
```

**A named volume, not a bind mount.** The image runs as uid 10001 and `chown`s
`/data` at build time; a named volume inherits that ownership on first use,
while a host bind mount arrives root-owned and the server cannot write its game
store. Do not publish 8080 to the LAN — nothing on the LAN should reach an API
with no authentication.

Then edit the tunnel config (this repo's `cloudflared-config.yml` is the
deployed copy), **back up the old one first**, and restart:

```
POST /containers/cloudflared-nas/restart
```

### 3.1 DNS is NOT automatic — the step that bit

Adding an ingress block routes nothing by itself. Measured immediately after the
restart: `yokohama.moore.nyc` served the client, and `yokohama-cli.moore.nyc`
**did not resolve at all**, leaving ttyd unreachable until a record existed. The
config file's own warning — *"Steps 3 and 4 are not optional. Ingress alone
routes nothing (no DNS) and protects nothing (no Access)"* — is correct and
should be believed.

Each new hostname needs a **proxied CNAME**:

```
<name>.moore.nyc  →  6a6c468e-5e1e-4a7c-88a6-4e2fe07da1fc.cfargotunnel.com
```

Taking a hostname that already exists needs no DNS change, which is why the
client went live on `yokohama.moore.nyc` with only a config edit.

### 3.2 Verify, through the tunnel and not from inside

```bash
curl -sS https://yokohama.moore.nyc/api/health
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/art/areas/Church.webp
curl -sS "https://yokohama.moore.nyc/api/games/<id>/at/999"   # 404 naming the bound
```

Recorded on the day: health `{"server_version":"0.5.0","semantics_rev":7,
"rules_hash":"d85445c479cb1ead"}`, art 200 `image/webp`, help screenshots 200
`image/png`, create → list → `/at/0` → out-of-range 404, then the smoke game
deleted.

### 3.3 What this costs on the hardware — MEASURED

Taken 2026-09-01 against the running container on the NAS (Atom C2538, 4 cores,
kernel 3.10.108). **Wall figures are per-machine; this machine is the NAS, not
M5 and not the WSL box.**

| | Reading |
|---|---|
| Resident memory, idle | **1.0 MB** (`rss` 1,069,056) |
| Resident memory, after two bot searches | **1.2 MB** (`rss` 1,191,936) |
| Total container usage incl. page cache | 9.8 MB idle, 12.0 MB peak |
| Container image | 19 MB |
| **`suggest` — a full search, 186 ranked candidates** | **0.45 s / 0.57 s** over two runs, through the tunnel |
| CPU with nobody playing | ~0 |

**This settles the Modal question, and the answer is no.** The expectation going
in was that `suggest` and `why` would be where a 2014 Atom showed; half a second
for a 186-candidate search is comfortable, and the milestone's own e2e walk
plays whole games against this same binary. **The NAS is sufficient. Modal is a
solution to a problem this workload does not have** — §4 stays as a documented
upgrade path and should not be acted on without a reason that is not
performance.

The memory figure is worth stating plainly because it is easy to disbelieve:
**a little over one megabyte resident.** A static musl binary with no runtime,
no garbage collector, no interpreter and no database, whose largest in-memory
object is one `GameState`. The 12 MB total is mostly page cache for the files it
serves. Any hosting decision that turns on RAM is not a decision about this
program.

### 3.4 What it costs on the wire — MEASURED

Every metered host bills egress, so the payload is worth stating too. The three
first-paint rows are read off `web/dist/assets/` after `npm run build`, at the
head that ships; they were last re-read at the M5 fix wave, and the figures they
replaced were two waves old.

**MB is 10⁶ bytes and MiB is 2²⁰ throughout this table** — the distinction is
not pedantry here, because `du -sk` reads KiB and the rows below came from both
tools:

| | Bytes |
|---|---|
| `index.html` | 719 |
| `assets/index-*.js` | 269,033 (`index-Ba9C5aPH.js`, the bundle the `0.5.0-m5.1` image ships) |
| `assets/index-*.css` | 27,368 |
| **First paint, total** | **297,120 (0.30 MB / 290 KiB)** |
| Card art at `/art` | 7,552 KiB across 132 files = 7.7 MB (7.4 MiB), fetched per area actually rendered |
| Help screenshots | 4,199,746 B across 5 files = 4.2 MB (4.0 MiB) (M5 W7: re-shot at 1710×869, a fifth picture added), and only if the panel is opened |

Re-measured at M5 (W0, then W0b): the crops are now cut at source width (709px
tiles, 530px cards, 799px landscape achievements, and the 24 site cards turned
landscape at W0b), which took the art from 3.0 MB to 7.7 MB across the same 132
files; the per-session figures above scale accordingly. W0b itself is worth
only +28 KiB — the same pixels, re-compressed after the quarter turn — so the
step from the 7.3 MB this section first printed is mostly a UNIT correction:
`du -sk` reads KiB, and 7,552 KiB is 7.4 MiB but 7.7 MB.

So a session that opens help and renders every area in the product pulls about
**12.2 MB** (7.7 art + 4.2 help + 0.3 first paint; it was about 10 before W7
re-shot the help at the owner's viewport and added a fifth picture, and about 5
before W0). The help row is where this sum was previously wrong: 4,199,746 B is
4.2 MB and 4.0 MiB, and the table printed the MiB figure under an MB heading —
`ASSETS.md` had it right. An ordinary session's art is now fetched at ~57
KiB per file instead of ~23 — every file **~2.5x** heavier than before this
re-cut — so "well under 1 MB" is no longer a safe claim; the true total
depends on how much of the board and hand a given game actually renders.

**Correction — this section first said egress "is not a constraint on where this
can run," and that was too broad.** It holds against Oracle's Always Free 10 TB
and Azure's perpetual 100 GB, where the ceiling is tens of thousands of sessions
and the objection really is dead. It does **not** hold on Google: Compute
Engine's Always Free e2-micro and Cloud Run each allow **1 GB/month** of North
American egress, which is roughly 3,700 first paints, or **~200 sessions** if
each one pulls the whole art set. That is enough for a handful of players and
nowhere near "not a constraint." Cloudflare in front, caching the static
assets, is what would make it one again.

Recorded with the correction attached because the generalization is the kind of
thing that gets repeated: **check the number per provider, and do not carry
Oracle's 10 TB over to Google's 1 GB.**

---

### 3.5 M5 is live — the second deployment, 2026-09-02 (a record, not a plan)

The same shape as §1–§3, run through the Portainer Docker proxy from the
workstation, with the numbers that changed:

- **Build:** `web-frontend` at `7d6a503` (Task 14 of the M5 plan; the deploy
  commit is the one after it). The musl binary is unchanged in kind
  (`static-pie linked`, 6,089,656 bytes — the server crate did not move in M5
  beyond `assets/art/**`); `web/dist` carries the M5 client and the five help
  pictures. Context tarball **13.9 MB** (was 6.7 MB — the source-width art and
  the re-shot help are the whole difference), fetched by the NAS
  (192.168.1.173) from `http://192.168.1.16:8899/ctx.tar.gz` in one GET.
- **Image:** `yokohama-web:0.5.0-m5`, **25.9 MB** (was 19 MB, same reason),
  built in ~10 s.
- **Swap:** `POST /containers/{old}/stop` → `DELETE /containers/{old}` →
  `POST /containers/create?name=yokohama-web` with §3's body plus the two
  labels the first container carried (`maintainer`, `purpose`) → `start`. The
  named volume rode across untouched: the setup page listed the saved game
  that was there before, and opening it read-only showed turn 3 exactly as
  left. Same address on `cf-internal` (172.21.0.4), so the tunnel's ingress
  needed nothing. **Healthy on the first probe**, 30 s after start.
- **Verified through the tunnel, per §3.2:** `https://yokohama.moore.nyc/`
  serves the M5 table at the owner's 1710×869 (the pyramid of art tiles, the
  Boards menu, the answer bar pinned at the top of the side column), and
  `/help/strips.png` — the picture that exists only in this build — answers
  as a 1710×869 image. The old image `yokohama-web:0.5.0` is still on the
  NAS, which is the rollback (§5).
- **Downtime:** the seconds between `stop` and `start`; nobody was playing.

**Addendum, the same day (19:30Z): `yokohama-web:0.5.0-m5.1`.** The final
whole-branch review's fix wave (`1e85404`…`ca982ea`: a stale reply for a game
you have switched away from is dropped, the Game menu waits while a move is in
flight, deleting a saved game takes two clicks, and the review's minors) was
redeployed by the same recipe so the live site carries the two safety fixes:
context 13,879,136 B, image **25,950,515 B** built in ~10 s, the `0.5.0-m5`
container stopped and removed, the new one created with the same body, same
address (172.21.0.4), **healthy at 46 s**, and the tunnel serving the wave's
bundle `index-Ba9C5aPH.js` (which exists only in this build). Three tags now
sit on the NAS — `0.5.0`, `0.5.0-m5`, `0.5.0-m5.1` — each a rollback point
(§5). §3.4's first-paint rows above are measured from exactly this build.

### 3.6 M5.5 is live — the third deployment, 2026-09-05 (a record, not a plan)

The §3.5 recipe again, through the Portainer Docker proxy (environment 2)
from the workstation, with the numbers that changed:

- **Build:** `web-frontend` at `6d86989` (Task 12's second fix round of the
  M5.5 plan; the deploy commit is the one after it). The musl binary moved
  this time — `static-pie linked`, **6,105,720 bytes** (was 6,089,656: the
  `events` count on the live payload, the offers on the replay frame, and the
  preview-as-a-position with the bot's argmax keep are the whole difference);
  `web/dist` carries the M5.5 client (bundle `index-D9enc4gM.js`, 282,553 B)
  and the five re-shot help pictures. Context tarball **13,899,025 B** (was
  13,879,136), fetched by the NAS (192.168.1.173) from
  `http://192.168.1.221:8899/ctx.tar.gz` in one GET — **the workstation's LAN
  address is no longer the 192.168.1.16 §1 names**; read it from `ifconfig`
  before serving the context.
- **Image:** `yokohama-web:0.5.0-m5.5`, **25,993,976 B** (was 25,950,515),
  built in ~11 s (the NAS fetched the context at 04:25:30 local and the image
  is stamped 04:25:41).
- **Swap:** `POST /containers/yokohama-web/stop` → `DELETE` → `POST
  /containers/create?name=yokohama-web` with §3's body plus the two labels
  (`maintainer`, `purpose`) → `start` at 08:26:17Z. Same address on
  `cf-internal` (172.21.0.4), so the tunnel's ingress needed nothing. The named
  volume rode across untouched. **Healthy on the first probe**, 30 s after
  start (04:26:47 local).
- **Verified through the tunnel:** from a bare shell, `curl` answers the Access
  redirect (302, "Please authenticate via the warp client") — the tunnel up and
  the origin behind it. From a WARP-enrolled shell (Access authenticates the
  device silently, so the cookie is no longer the browser's alone) the §3.2
  trio passes: `/api/health` 200
  `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead"}`,
  `/` shipping `assets/index-D9enc4gM.js` (which exists only in this build),
  `/help/screen.png` 200, and `/api/games` listing every save. The container's
  own log reads `yokohama-web listening on http://0.0.0.0:8080`. On screen
  (Claude-in-Chrome at 1710×960): Deal → the set table (the pyramid, the port
  strip's six cards, the bot's two cards with one marked "bot keeps this") →
  choose → Start → the bot's turn walked as nine steps with the table outlined
  and the changed tiles lit → `replay` restarted it at step 1 and `skip` ended
  it. The owner's four saved games were listed before and after;
  `sim-d7d5918bb4f74a2a` opened read-only and was not played on. The check left
  one new save (`sim-5c3b4e596bb8cf97`) on the volume — the owner's to delete.
- **Rollback tags now on the NAS:** `0.5.0`, `0.5.0-m5`, `0.5.0-m5.1`,
  `0.5.0-m5.5` (§5).
- **Downtime:** the seconds between `stop` and `start`; nobody was playing.

---

## 4. Modal — the upgrade path, not yet deployed

`deploy/modal_app.py` runs the same image with a Volume for the store,
`max_containers=1` (Modal's Volume rules say a file must have a single writer,
and this store is one JSON file per game), and a commit loop — because on Modal
a drained request is **not yet a durable one**.

`deploy/worker/` is the edge proxy for that path, and the reason it exists is
that a Modal endpoint is world-reachable by default: without
`requires_proxy_auth=True` plus the Worker holding `Modal-Key`/`Modal-Secret`,
an Access policy in front would protect nothing, since anyone with the
`*.modal.run` URL could skip it.

Neither is deployed. The NAS is serving the product today, and §3.3's
measurement says it should go on doing so.

---

## 5. Rollback

- **The tunnel:** restore `config.yml.bak-2026-09-01` on the NAS and restart
  `cloudflared-nas`. One file, one restart.
- **The container:** `docker stop yokohama-web`. The tunnel then 502s that
  hostname until the ingress is pointed back.
- **The previous build (since M5, 2026-09-02):** `yokohama-web:0.5.0` is still
  an image on the NAS. Stop and remove the running container and recreate it
  from that tag with §3's body — the volume is the same, so the games carry
  over; only the client and help go back.
- **The store is not rolled back by either.** It is the named volume
  `yokohama-web-data`; the records are the whole state, which is why they
  replay.
