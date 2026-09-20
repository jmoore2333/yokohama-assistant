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
- **The server reads ONE identity header, and which one is a deployment
  decision.** (Since M6.1, 2026-09-15; before it this bullet said "no
  authentication of any kind", which was true then and is not now.)
  `YOKOHAMA_WEB_IDENTITY` picks the mode: `claimed` reads
  `X-Yokohama-Claimed-User`, `cf-access` reads
  `Cf-Access-Authenticated-User-Email`, `worker` reads `X-Yokohama-User`. Every
  mode reads **exactly one** header and ignores the other two, so a forged
  header on a path that does not read it changes nothing. **There is no
  default** — an unset or unknown value refuses to start, naming the three
  accepted values (`main.rs:94-98`). Every route under `/api/games` requires an
  identity and answers **401** without one; a caller with no seat at a table
  gets **403**. Public: `/api/health`, `/api/labels`, `/api/art`, `/art/*`,
  `GET /api/me`, and the client bundle.
- **The bind address is still a real decision, for a reason the header does not
  remove.** `claimed` behind Access authenticates the **door** and not the
  **seat**: anyone past Access can claim any email, so it tells two callers
  apart and verifies neither. `cf-access` and `worker` trust a header that only
  their own edge can set, which is a true statement only while that edge is the
  one path to the port. Publish 8080 and both properties are gone — see §3.
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
  "Env": ["YOKOHAMA_WEB_IDENTITY=claimed"],
  "HostConfig": { "NetworkMode": "cf-internal",
                  "RestartPolicy": { "Name": "unless-stopped" },
                  "Mounts": [{ "Type": "volume", "Source": "yokohama-web-data",
                               "Target": "/data" }] } }
```

**The `Env` line is not optional and is not in the image.** Neither Dockerfile
sets `YOKOHAMA_WEB_IDENTITY`, deliberately: there is no default mode, so an
image started without it **exits 1** naming the three accepted values. A deploy
from this recipe with the line dropped does not come up. The mode belongs to
the container because it is a statement about what is in FRONT of the container,
which the image cannot know.

**A named volume, not a bind mount.** The image runs as uid 10001 and `chown`s
`/data` at build time; a named volume inherits that ownership on first use,
while a host bind mount arrives root-owned and the server cannot write its game
store. Do not publish 8080 to the LAN. `claimed` authenticates the door and not
the seat, and the other two modes trust a header their own edge sets — both
properties hold only while cloudflared is the one path to this port.

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
curl -sS https://yokohama.moore.nyc/api/health     # "identity" names the mode
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/art/areas/Church.webp

# Since M6.1, the two readings that prove the gate is ON rather than merely built:
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/api/games        # 401
curl -sS -H 'X-Yokohama-Claimed-User: you@example.com' https://yokohama.moore.nyc/api/games
```

The 401 is the load-bearing one. A deployment whose mode is wrong, or whose
header name the client and the server disagree about, still answers `/api/health`
perfectly — so a check that reads only the health payload cannot tell a working
identity gate from a broken one.

**This check needs WARP active** (Access authenticates an enrolled device
silently, which is what lets `curl` past it at all): a "Just a moment…"
challenge page back from `curl` means WARP is off, **not** that the deployment
failed — §3.7 is the run where exactly that happened and the in-container read
was the workaround.

Recorded on 2026-09-01, before identity existed: health
`{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead"}`,
art 200 `image/webp`, help screenshots 200 `image/png`, create → list → `/at/0`
→ out-of-range 404, then the smoke game deleted. §3.7 is the current reading.

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

**Addendum, the same day: `yokohama-web:0.5.0-m5.5.1` (17:07Z), then
`0.5.0-m5.5.2` (17:31Z).** The final review's fix wave (`e0de465`…`0689fe7`:
the turn's highlights outlive the walk — the held marks are the union of the
mover's steps, not the empty last step's; the tile keeps its DOM node when the
placement picker opens, so the art no longer remounts and goes dark; the walk
strip is one line; the account scrolls its lit row into view; the seat-row and
strip-slot highlights pinned by unit cells; the ink-edge cell renamed; the
comments made true) was redeployed by the same recipe as **`0.5.0-m5.5.1`**:
the musl binary unchanged (`crates/` did not move), context 13,899,474 B,
image **25,995,030 B** built in ~10 s, the `0.5.0-m5.5` container stopped and
removed, the new one created with the same body, same address (172.21.0.4),
**healthy at 30 s**, the tunnel serving the wave's bundle `index-BdX0BsM6.js`
13 s after start. The on-screen check of THAT build found the account's
scroll-into-view scrolling the PAGE in the below-1201 stack (from the board at
scroll 0 a walk read 1826 → 1851 → 1877 px, the lit row under the sticky
strip) — fixed as `fb92c17` (the account scrolls only its own box, never the
page) and redeployed as **`0.5.0-m5.5.2`**: context 13,899,577 B, image
**25,995,225 B** (~10 s), the same swap, the same address, **healthy at 30 s**
(13:31:50 local), the tunnel serving `index-CVFCAqs1.js` 18 s after start; on
screen at a 1000-px viewport the page held at scroll 0 through a nine-step walk
while the account's box scrolled 0 → 38 px with the lit row inside it. The
checks left three saves on the volume (`sim-5c3b4e596bb8cf97`,
`sim-7d949b884ebb2adb`, `sim-fa6b6bb5049a3929`), the owner's to delete. **Six
tags now sit on the NAS** — `0.5.0`, `0.5.0-m5`, `0.5.0-m5.1`, `0.5.0-m5.5`,
`0.5.0-m5.5.1`, `0.5.0-m5.5.2` — each a rollback point (§5). The live build is
`fb92c17`; the milestone's close commit follows it.

**Addendum, 2026-09-06: the owner's first-run corrections — `yokohama-web:0.5.0-m5.5.3`
(13:07Z), then `0.5.0-m5.5.4` (13:48Z).** The correction wave
(`docs/plans/2026-09-06-web-frontend-m5.5-owner-corrections.md`: another seat's
deal never reaches the wire or the screen — the bot's two cards are backs and its
keep stays in the record; the set table withholds the technologies and the goals
until the card is chosen, the owner's edition's order; human seats choose one at a
time with the others' cards face down; the mobile set table diagnosed, not
changed) went out by the §3.5 recipe as **`0.5.0-m5.5.3`** from `f5cb50e`: the
musl binary REBUILT this time (`crates/yokohama-web` moved — 6,105,960 B,
static-pie, 5.5 s incremental), context 13,900,042 B, image **25,996,682 B**
built in ~15 s, the `0.5.0-m5.5.2` container stopped and removed, the new one
created with the same body, same address (172.21.0.4), **healthy at 30 s**, the
tunnel serving `index-B-jx3Iuo.js` 20 s after start. The clean final review
against that build returned one Important (a test gap — the deal's restart at
the first seat pinned by nothing) and two folded minors, one of them product
text (the setup form's subtitle counted one human seat at every player count),
so the wave's fix commit `fb6cec7` went out as **`0.5.0-m5.5.4`**: binary
6,105,976 B (a test module moved in `games.rs`; the release build differs by
its panic-location lines), context 13,900,069 B, image **25,996,894 B** (~15 s),
the same swap, the same address, **healthy at 30 s** (09:49:13 local), the
tunnel serving `index-LfoAAWF6.js` 17 s after start. Verified through the
tunnel from the shell: the preview payload for a 2P and a 3P request carries a
bot deal of exactly `seat`/`is_bot`/`cards` with `cards: []`, no lab slots and
no achievements, while the port keeps its six per board; on screen at 1710 wide
the subtitle counts the seats at 2P/3P/4P, the bot's seat is two backs, the
laboratory and the goals read their line until the pick. **Eight tags now sit
on the NAS** — `0.5.0`, `0.5.0-m5`, `0.5.0-m5.1`, `0.5.0-m5.5`, `0.5.0-m5.5.1`,
`0.5.0-m5.5.2`, `0.5.0-m5.5.3`, `0.5.0-m5.5.4` — each a rollback point (§5).
The checks' own saves were erased the same day at the owner's word; the owner's
five remain. The live build is `fb6cec7`; the wave's record commit follows it.

### 3.7 M6.1 is live — the fourth deployment, 2026-09-15 (a record, not a plan)

The §3.5 recipe again, through the Portainer Docker proxy (environment 2) from
the workstation. **The one thing that is NEW in the recipe and not just a
number: the create body carries `YOKOHAMA_WEB_IDENTITY=claimed`**, because this
is the first build that reads an identity header at all.

- **Build:** `web-room` at `5be1e54` (the whole-branch review's fix wave; the
  deploy-docs commit is the one after it). The musl binary moved and by more
  than any previous milestone — `ELF 64-bit LSB pie executable, x86-64,
  static-pie linked`, **6,400,224 bytes** (was 6,105,976: identity, the table
  document, the seat gate and the store's version seam); `web/dist` carries the
  M6.1 client, bundle `index-BdYKdXfa.js`. Context tarball **13,990,090 B**
  (sha256 prefix `82ba551a6a1c5f33`), holding `yokohama-web`, `dist/`, `art/`
  and `Dockerfile` (a copy of `deploy/Dockerfile.runtime`), served by
  `python3 -m http.server 8899` on the workstation at **192.168.1.16** — *this
  workstation's LAN address today; §3.6's `.221` was a different day, so read
  it from `ifconfig` every time* — and fetched by the NAS (192.168.1.173) in
  one GET at **06:58:58Z** (02:58:58 local). The file server was stopped after
  the fetch.
- **Image:** `yokohama-web:0.5.0-m6.1`, Id
  `sha256:8a29e8a4f347bebd9bcceb16e44787ecc7957e8261ef8d2156bca7e7c84fe4c2`,
  **26,300,095 B** (was 25,996,894), created **06:59:09Z** — about 11 s after
  the fetch — through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.1&dockerfile=Dockerfile&rm=true`.
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m5.5.4`
  container (`d2dfda65f02c…`, created 2026-09-06) → `DELETE /containers/yokohama-web`
  → `POST /containers/create?name=yokohama-web` with §3's body (Image,
  `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named volume
  `yokohama-web-data` → `/data`, the two labels `maintainer`/`purpose`)
  **plus `"Env": ["YOKOHAMA_WEB_IDENTITY=claimed"]`** → new container Id
  `4e1996e6571e1ef22bbe69cfb604d32b569b187ae45681fe2f7e3d3153fe4a5c`, created
  **06:59:32Z**, started **06:59:36Z**. Same address on `cf-internal`
  (**172.21.0.4**), so the tunnel's ingress needed nothing. The named volume
  rode across untouched. The container log's first line:
  `yokohama-web listening on http://0.0.0.0:8080`. **Healthy on the first
  probe at 07:00:06Z** (03:00:06 local), 30 s after start.
- **Verified TWICE, and the order is the interesting part.** The first read was
  taken from inside the container by `docker exec`, a departure from §3.2, and
  the reason it was taken that way is the lesson §3.2 now carries: WARP was off,
  so the shell's path through the tunnel returned a Cloudflare managed challenge
  page rather than the API. WARP was re-enabled and the §3.2 reading was then
  taken normally. **Both are recorded because the pair is what separates "the
  server is right" from "the whole path to it is right"** — the first reading
  alone cannot see a tunnel or an Access misconfiguration, and the second alone
  cannot tell you where a failure lives.

  **07:00Z, inside the container** (`docker exec`, against `127.0.0.1:8080`):
  - `wget -qO- http://127.0.0.1:8080/api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed"}`
    — the `identity` field is new in this build and is how the mode is read back.
  - `GET /api/games` with no header → **`HTTP/1.1 401 Unauthorized`**.
  - `GET /api/games` with `X-Yokohama-Claimed-User: jamoore@gmail.com` →
    **`{"games":[]}`**.
  - `GET /` serves `index-BdYKdXfa.js`, which exists only in this build.

  **07:02Z, through the tunnel — this is the milestone's stated verification**
  (spec §10, and §3.2's own recipe), from a WARP-enrolled shell, which is what
  Access authenticates silently: the same four readings, identically —
  `https://yokohama.moore.nyc/api/health` returning that same payload with
  `"identity":"claimed"`, `/api/games` **401** bare and `{"games":[]}` with
  `X-Yokohama-Claimed-User: jamoore@gmail.com`, and `/` serving
  `index-BdYKdXfa.js`. So the tunnel, the Access application and the identity
  gate agree end to end. The on-screen check in a browser, at both layouts and
  from two browsers as two people, is the final review's (R-P11) and not this
  record's.
- **The empty list is expected and is not a lost store (R-P7).** `/api/games`
  lists TABLES, and a record with no table document is not a game this API
  serves. Every record already on the volume predates the table document, so
  the list starts empty and fills as tables are created. The records themselves
  are untouched on disk and the Docker-archive pull recipe (`docs/handover.md`
  Track 3) is unaffected.
- **R-1.4, in the two sentences the spec asks for beside the mode.**
  `cf-access` is trustworthy only because port 8080 is unpublished and
  cloudflared is the one path to it (§3: "Do not publish 8080 to the LAN").
  `claimed` behind Access authenticates the **DOOR and not the SEAT** — a
  development convenience, not a security boundary: anyone past Access can
  claim any email, so the mode tells two callers apart and verifies neither.
- **The mode is NOT baked into the image.** Neither Dockerfile sets
  `YOKOHAMA_WEB_IDENTITY`, and an image started without it **exits 1 by
  design** (R-1.3) rather than picking a default. A deploy driven from this
  runbook alone must carry the `Env` line in the create body or the container
  does not come up.
- **Downtime:** the seconds between `stop` and `start`; nobody was playing.
- **Rollback — a WHOLE-IMAGE rollback this time, and that is the difference
  from every previous one.** `yokohama-web:0.5.0-m5.5.4` is still an image on
  the NAS, but its client knows nothing of `/table`, so there is no partial
  step back: recreate the container from that tag with §3's body **without**
  the `Env` line (the old binary ignores the variable). **Nine tags now sit on
  the NAS** — `0.5.0`, `0.5.0-m5`, `0.5.0-m5.1`, `0.5.0-m5.5`, `0.5.0-m5.5.1`,
  `0.5.0-m5.5.2`, `0.5.0-m5.5.3`, `0.5.0-m5.5.4`, `0.5.0-m6.1` — each a
  rollback point (§5).

---

### 3.8 M6.2 is live — the fifth deployment, 2026-09-16 (a record, not a plan)

The §3.5 recipe again, through the Portainer Docker proxy (environment 2) from
the workstation, driven by the orchestrating session under the owner's
standing authorization for this cycle. Nothing in the recipe changed: the
create body is §3.7's, `YOKOHAMA_WEB_IDENTITY=claimed` included.

- **Build:** `web-room-m6.2` at `a214a19` (the whole-branch review's fix wave;
  the server half of that wave is `ad29715`, the commit the binary was built
  at — `a214a19` is client-only). The musl binary: `ELF 64-bit LSB pie
  executable, x86-64, static-pie linked`, **6,496,616 bytes** (was 6,400,224:
  the table lifecycle split out, the open status and seat kind, the lock,
  `join`, the two-scope cursor listing, the per-table locks); `web/dist` carries
  the M6.2 client, bundle `index-C_4DrD7x.js`. Context tarball **14,026,580 B**
  (sha256 prefix `e1b1a16a369ba1b5`), holding `yokohama-web`, `dist/`, `art/`
  and `Dockerfile` (a copy of `deploy/Dockerfile.runtime`), served by
  `python3 -m http.server 8899` on the workstation at **192.168.1.16** (read
  from `ifconfig`, as §3.7 says to) and fetched by the NAS in one GET; the
  tarball was cut at **02:06:41Z** and the image existed at 02:06:59Z. The file
  server was stopped after the fetch.
- **Image:** `yokohama-web:0.5.0-m6.2`, Id
  `sha256:2435328a9967a349df589dbf68d41a746365e1aa7ba69f1428199c42df460fac`,
  **26,407,067 B** (was 26,300,095), created **02:06:59Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.2&dockerfile=Dockerfile&rm=true`.
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.1` container
  (`4e1996e6571e…`, created 2026-09-15 06:59:32Z) → `DELETE /containers/yokohama-web`
  → `POST /containers/create?name=yokohama-web` with §3's body (Image, `Env`
  `YOKOHAMA_WEB_IDENTITY=claimed`, the two labels `maintainer`/`purpose`,
  `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named
  volume `yokohama-web-data` → `/data`) → new container Id
  `faffca0411d469d2f11b27ed72d04063b1c5df2abb90b7a3c12d254569d17e45`, created
  **02:07:23Z**, started **02:07:26Z**. Same address on `cf-internal`
  (**172.21.0.4**) — and the ingress names the container (`yokohama-web:8080`,
  `deploy/cloudflared-config.yml`) — so cloudflared was not restarted. The
  named volume rode across untouched.
- **Verified through the tunnel at 02:07:46Z** (WARP signed in — the owner
  refreshed the Access authorization earlier in the session, after a first
  read had returned "Please authenticate via the warp client"; §3.2's lesson
  again), twenty seconds after the start:
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed"}`.
  - `GET /api/games` with no header → **401**.
  - `GET /api/games?scope=open` and `GET /api/games` with
    `X-Yokohama-Claimed-User: jamoore@gmail.com` → **`{"games":[],"next":null}`**
    — `next` is the M6.2 shape; the owner's list is empty for R-P7's reason.
  - `GET /` serves `index-C_4DrD7x.js`, which exists only in this build.
- **The schema-1 document on the volume read unchanged — measured, not
  assumed (spec §9).** `m6.1-live-review.table.json` was the one table document
  the NAS held (schema 1, by `reviewer-a@m61.test`). Read through the NEW server
  as its creator BEFORE deleting it: `GET /api/games` listed it and
  `GET …/m6.1-live-review/table` opened it, `status: "playing"`, `to_move: 1`,
  both seats `"name": null`, `"locked": false`, `"winner": null` — every M6.2
  field at its default. Then, as the M6.1 close asked, `DELETE
  /api/games/m6.1-live-review` as its creator → **204**; its list is empty and
  the table reads 404. The eight pre-M6 `sim-*.json` records beside it have no
  table document and stay unlisted by design.
- **The milestone's own verification (spec §9), at 02:08:08Z:**
  `POST /api/games` as `deploy-verify-a@m62.test` with one open seat, the label
  "deploy verify m6.2", `display_name` "Verifier A" and the password `sesame`
  → **201**, `status: "open"`, `locked: true`, seat 0 named "Verifier A", seat 1
  `{"kind":"open"}`; `GET /api/games?scope=open` as `deploy-verify-b@m62.test`
  lists it; `POST …/join {"seat":1,"name":"Verifier B","password":"open"}` →
  **403** `{"error":"wrong password for this table"}`; the same with `sesame`
  → **200**, `status: "dealing"`, seat 1 `"name": "Verifier B"`,
  `my_seats: [1]`. The table was then deleted by its creator (**204**) so the
  live review starts from an empty house.
- **Downtime:** the seconds between `stop` and `start`; nobody was playing.
- **Rollback:** `yokohama-web:0.5.0-m6.1` stays on the NAS; recreate the
  container from that tag with §3's body INCLUDING the `Env` line (both builds
  read the mode). A whole-image rollback again: the M6.2 client knows `join`
  and the lobby, the M6.1 server does not, and the table documents this build
  writes are schema 2 — an M6.1 server reads `schema` nowhere, so a schema-2
  document with an `open` seat would reach its `check_shape` and be refused at
  the seat kind, which is the typed refusal and not a crash. **Ten tags now sit
  on the NAS** — the nine of §3.7 plus `0.5.0-m6.2`.

**Point release `0.5.0-m6.2.1`, 02:47Z the same morning.** The clean live review
(the close's §4.2) returned SHIP WITH FIXES on one Important — the seated
open-table screen scrolled sideways 32 px at 390 px because the link row's Copy
button stuck out — and the fix (`8dc6316`: the link input shrinks and the row
wraps, the dealing screen's leave control reads "Lobby", a refused link's
sentence sits above the lobby's lists) is CLIENT-ONLY, so the musl binary is
the same 6,496,616-byte file and only `web/dist` changed (bundle
`index-_UNINWWV.js`). Context tarball **14,026,761 B** (sha256 prefix
`42a0d1b221dbcc02`), cut 02:43:53Z, served from 192.168.1.16:8899 → image
`yokohama-web:0.5.0-m6.2.1`, Id
`sha256:7544c69bfec36a12d72f33a8cb5fa6238a956782cd11fcd1b8f531901c032bb8`,
**26,407,161 B**, created **02:47:10Z**. Swap as above: the `0.5.0-m6.2`
container (`faffca0411d4…`) stopped and deleted → new container
`584b2f96fc22a917b0c34f42f9ad32d3e7b486bcdc497bfe66343606c6c6def0` created
**02:47:39Z**, started **02:47:43Z**, same address 172.21.0.4, no cloudflared
restart. Verified through the tunnel at **02:47:54Z**: `/api/health` the same
payload, bare `/api/games` **401**, `?scope=open` with the owner's header
`{"games":[],"next":null}`, `/` serving `index-_UNINWWV.js`. The 390 px property
itself is a browser matter and is witnessed by the new assertion in
`web/e2e/joins_by_link.spec.ts` (0 px page overflow on the seated open-table
screen), not by a shell read. **Eleven tags now sit on the NAS.** Rollback to
`0.5.0-m6.2` is the same recipe with the earlier tag.

### 3.9 M6.3 is live — the sixth deployment, 2026-09-16 (a record, not a plan)

The §3.5 recipe again, through the Portainer Docker proxy (environment 2) from
the workstation, driven by the orchestrating session under the owner's
in-session authorization ("push the branch, PR against 6.2, and the current
deployment is unused now so go ahead and feel free to redeploy"). Nothing in
the recipe changed: the create body is §3.7's, `YOKOHAMA_WEB_IDENTITY=claimed`
included. **The plan's rc step was FOLDED INTO the release** — HEAD itself was
deployed and the R-3.1 measurement below was taken against the deployed stream,
so there is no `-rc1` image; the only extra image this deployment cut is the
`-rc2` reading B needed, and that one is gone again.

- **Build:** `web-room-m6.3` at `74a0e80` (the doc-pass HEAD; the server half of
  the final fix wave is `9214661` and the client half `9f70aa2`; the three commits
  after them change documentation and source comments only). PR #33 against `web-room-m6.2`,
  https://github.com/jmoore2333/yokohama-assistant/pull/33. The musl
  cross-compile per §1: `Finished release in 10.15s`; the binary is `ELF 64-bit
  LSB pie executable, x86-64, static-pie linked`, **6,637,080 bytes** (M6.2.1's
  was 6,496,616: one watch channel per table plus the house channel, the two
  SSE routes, the admission table, `unlock` and the spectator projection);
  `web/dist` carries the M6.3 client, bundle `index-BhDpWEFw.js` (M6.2.1's was
  `index-_UNINWWV.js`). Context tarball **14,067,814 B** (sha256 prefix
  `e19003d7defbd922`), holding `yokohama-web`, `dist/`, `art/` and `Dockerfile`,
  cut at **19:03:14Z** and served by `python3 -m http.server 8899 --bind
  0.0.0.0` on the workstation at **192.168.1.16** (read from `ifconfig`, as §3.7
  says to); the NAS (192.168.1.173) fetched it in one GET at 15:03:24 local and
  the file server was stopped at 19:03:52Z.
- **Image:** `yokohama-web:0.5.0-m6.3`, Id
  `sha256:2c6d45f02825882f5e9499dd46b53b97d690e4959a35fe279d1e6b3e53e5600f`,
  **26,553,575 B** (was 26,407,161), created **19:03:36Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.3&dockerfile=Dockerfile&rm=true`.
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.2.1` container
  (`584b2f96fc22…`, created 2026-09-16 02:47:39Z) at 19:03:52Z → `DELETE
  /containers/yokohama-web` → `POST /containers/create?name=yokohama-web` with
  §3's body (Image, `Env` `YOKOHAMA_WEB_IDENTITY=claimed`, the two labels
  `maintainer`/`purpose`, `NetworkMode: cf-internal`, `RestartPolicy:
  unless-stopped`, the named volume `yokohama-web-data` → `/data`) → new
  container Id
  `4f37b4bb7099c9211f6647386925893db6350cb1ea6936b481b247a6da29edf3`, created
  **19:04:01Z**, started **19:04:04Z**. Same address on `cf-internal`
  (**172.21.0.4**), so cloudflared was not restarted. The named volume rode
  across untouched. **That container is NOT the one running now** — reading B
  below replaced it twice; see the closing paragraph of the measurement.
- **Verified through the tunnel at 19:04:26Z** (WARP signed in; no re-auth was
  needed this time):
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed","signal":"sse"}`
    — the `signal` field is M6.3's, and it is how a shell reader learns which
    transport this build serves.
  - `GET /api/games` with no header → **401**.
  - `?scope=open`, `?scope=playing` (the M6.3 scope) and `?scope=mine` with the
    owner's header → **`{"games":[],"next":null}`**.
  - `GET /` serves `index-BhDpWEFw.js`, which exists only in this build.
- **The milestone's own verification (spec §10), from 19:05:16Z:** `POST
  /api/games` as `deploy-verify-a@m63.test` with both seats named
  (`deploy-verify-b@m63.test`) and the password `sesame` → **201**
  `sim-f68b6accb83c4073`, `status: "dealing"`. Each seat then `GET …/deal` and
  `POST …/keep` — a first attempt with `"card": 0` was refused
  `"keep 0 — the dealt cards are numbered 1 and 2"`, the CLI's own numbering
  reaching the wire — and after the second keep the table read `status:
  "playing"`, `to_move: 0`; `GET …/events` as A → `{"events":0,"to_move":0,"status":"playing","your_turn":true}`.
  **The stream through the tunnel:** `curl -sN` as B on `…/stream` printed the
  on-connect frame
  `event: signal` / `data: {"events":0,"to_move":null,"status":"dealing","your_turn":false}`
  at +0 s and a heartbeat comment `:` at **+25 s**, over a 35 s hold taken while
  the table was still dealing.

#### The R-3.1 measurement — taken through the tunnel, both ways

The spec held `SIGNAL_KIND` to a reading rather than an argument: SSE ships if a
heartbeated stream survives the edge for ten minutes with a frame arriving in
it, and the dark held-request arm ships if it does not. Both readings were taken
on the live site, on the measurement table above, with WARP on.

**Reading A — with the 25 s heartbeat (19:06:29Z–19:16:29Z, `reading-a.log`).**
B (seat 1, not the acting seat) held `…/stream` for 600 s. The on-connect frame
arrived at 19:06:30.3Z:

```
[19:06:30.3N] event: signal
[19:06:30.3N] data: {"events":0,"to_move":0,"status":"playing","your_turn":false}
```

(the lines are quoted verbatim; the trailing `N` is an artifact of the log's own
timestamp format). Then **23 heartbeat comments at 25 s intervals** — twenty-one
before the move (19:06:55.3, 19:07:20.3, 19:07:45.3 … 19:15:15.3) and two after
it — with A posting `1` (EndAdditionalPhase) at **19:15:29Z, 540 s in**, and the
frame

```
[19:15:30.3N] event: signal
[19:15:30.3N] data: {"events":1,"to_move":0,"status":"playing","your_turn":false}
```

arrived **under one second later**. The next comment came at 19:15:55.3 rather
than 19:15:40.3 — axum resets the keep-alive on a message, so the interval
restarts from the frame (the last comment was 19:16:20.3). The connection was
**still open at 600 s** and was ended by `curl`'s own `--max-time` (exit 0), not
by the edge.

**Reading B — without the heartbeat (19:18:05Z–19:20:10Z, `reading-b.log`).**
The edge's behaviour on an idle stream cannot be reached except through the
tunnel, so `HEARTBEAT` was set to 3600 s in a WORKING COPY only (musl build
`Finished release in 5.39s`, then reverted; `git status` clean), and that image
was deployed for the reading alone: context `ctx-rc2.tar.gz` **14,067,865 B**
(sha256 prefix `b0bbd4a20e08f966`) served 19:16:59Z and fetched 19:17:05Z →
`yokohama-web:0.5.0-m6.3-rc2`, Id
`sha256:da2de00578f6a2d33de1ff1adcce4a0e0c00ffc292434e53110105afd72cae90`,
**26,553,855 B**, created **19:17:24Z**; container
`33021c62493c3319ecf52d21e18a43a123bf60c3d23cc78f07a1a966c2c23bbc` created and
started ~19:17:5xZ, health through the tunnel reading the same payload. B then
held the IDLE stream from **19:18:05Z**: the on-connect frame at +0 s and
nothing after it, and

```
[+125s 19:20:10] 
[+125s 19:20:10] --- curl done: http 200, 125.166883s, exit 92
```

— **the edge reset the connection at +125 s**, `curl` exit 92 being an HTTP/2
stream error, which is the origin-idle close this file's ingress comment expects
near 100 s. Beside reading A's ten minutes, the 25 s comment is what carries the
stream.

**Decision: SSE SHIPS. `SIGNAL_KIND` stays `"sse"`** (spec R-3.1), and the
held-request arm stays built, dark and one constant away.

**The swap back, and the container that is live.** The rc2 container was stopped
and deleted ~19:20:3xZ and `POST /containers/create` ran again with the same
body and the tag `yokohama-web:0.5.0-m6.3` → container
`f0c371c76584156bbfa2aafaeec46c3a085995fd45bb0dc97cb53577c5679b86`, created
**19:20:47Z**, started **19:20:51Z**, same address 172.21.0.4, healthy at
19:21:21 local. Verified through the tunnel at **19:21:05Z**: health with
`"signal":"sse"`, bundle `index-BhDpWEFw.js`, and a 30 s hold showing the
on-connect frame at +0 s and the heartbeat comment at +25 s again. **LIVE =
`yokohama-web:0.5.0-m6.3` in container `f0c371c7…` since 19:20:51Z**; the first
container, `4f37b4bb…`, served 19:04:04Z until the rc2 reading. The rc2 image
was removed from the NAS afterwards and the measurement table was deleted by its
creator, so the house is empty.

**The live review (2026-09-16, the close's §4.5).** A clean Fable subagent drove
three browsers as three people against the live site — two playing, one arriving
by link and watching — in both layouts, and returned **SHIP: 0 Critical, 0
Important, 3 Minor**, with 25 claims tried and found true and zero console errors
on all three screens. The M6.3 exit criterion is shown on the NAS: the watcher
asked `GET …/events` **zero** times and held **one** `…/stream` through two full
turns, through a 444 s game of 1,471 events and through a whole game against the
bot. The three Minors are in the close's §4 and its LEAVE list; none of them
holds a release, and **no point release was cut**.

- **Downtime:** the seconds between `stop` and `start`, twice (the release and
  the swap back); nobody was playing either time.
- **Rollback:** `yokohama-web:0.5.0-m6.2.1` stays on the NAS; recreate the
  container from that tag with §3's body INCLUDING the `Env` line. **Both halves
  roll or neither does:** the M6.3 client opens `…/stream`, the M6.2 server has
  no such route, and the client's ladder would meet a 404 and end the source
  rather than fall back to polling — so a partial rollback leaves a screen that
  never learns anything. The table documents this build writes are schema 2 plus
  an additive `unlocked` list, which an M6.2 server reads at its default.
  **Twelve tags now sit on the NAS** — the eleven of §3.8 plus `0.5.0-m6.3`
  (`-rc2` was removed from the NAS after reading B).

### 3.10 M6.4 is live — the seventh deployment, 2026-09-17 (a record, not a plan)

The §3.5 recipe again, through the Portainer Docker proxy (environment 2) from the
workstation, driven by the orchestrating session under the owner's in-session
authorization given before the task was reached ("just note you've got
authorization to build and deliver"), and — because API usage was about to run
out — taken the moment the whole-branch review's fix wave came back green, BEFORE
the mutation battery, the counts sequence and the close, which the resumed
session owes (the SDD ledger's RESUME MAP carries the order). **The create body
is unchanged and carries NO new variable**: M6.4's three chat variables each have
a default, so an image started without them runs the lobby chat ON with a keep of
500 messages and a cap of 500 characters — `YOKOHAMA_WEB_LOBBY_CHAT=on|off`,
`YOKOHAMA_WEB_CHAT_KEEP=<n>|all`, `YOKOHAMA_WEB_CHAT_TEXT=<n>` are OPTIONAL on the
container and `/api/health` reports what is in force (`lobby_chat`, `chat_keep`
— `null` for `all` — and `chat_text`). Setting `off` makes the two house routes
404 and the client draws no lobby panel; `all` keeps every message.

- **Build:** `web-room-m6.4` at `571c446` (the fix wave's HEAD: sixteen execution
  commits over `f53f6d7`, the branch pushed to origin as a backup with no PR yet).
  The musl cross-compile per §1: `Finished release in 6.75s`; the binary is `ELF
  64-bit LSB pie executable, x86-64, static-pie linked`, **6,796,984 bytes**
  (M6.3's was 6,637,080: the chat documents, the four routes, the fifth `Events`
  field and the two-counter house frame); `web/dist` carries the M6.4 client,
  bundle `index-DbxlTsgc.js` (M6.3's was `index-BhDpWEFw.js`). Context tarball
  **14,112,276 B** (sha256 prefix `74b2c61857fc0a02`), cut at **03:52:59Z** and
  served by `python3 -m http.server 8899 --bind 0.0.0.0` on the workstation at
  **192.168.1.16**; the NAS (192.168.1.173) fetched it in one GET at 23:53:12 local
  and the file server was stopped at 03:53:35Z.
- **Image:** `yokohama-web:0.5.0-m6.4`, Id
  `sha256:51db5d1e0829d55e1afb67e1b934b334f170aae7d64c43b543f3cf81f8b69f27`,
  **26,722,785 B** (was 26,553,575), created **03:53:23Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.4&dockerfile=Dockerfile&rm=true`.
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.3` container
  (`f0c371c7…`, created 2026-09-16 19:20:47Z) → `DELETE /containers/yokohama-web`
  → `POST /containers/create?name=yokohama-web` with §3's body (Image, `Env`
  `YOKOHAMA_WEB_IDENTITY=claimed` alone, the two labels `maintainer`/`purpose`,
  `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named volume
  `yokohama-web-data` → `/data`) → container Id
  `f5db3dfead7a2a5a590adde0a50c220a21a16560552a3b3b51b136489fd9b474`, created
  **03:53:42Z**, started **03:53:45Z**. Same address on `cf-internal`
  (**172.21.0.4**), so cloudflared was not restarted. The named volume rode across
  untouched.
- **Verified through the tunnel at 03:53:56Z** (WARP signed in; no re-auth):
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed","signal":"sse","lobby_chat":true,"chat_keep":500,"chat_text":500}`
    — the three chat keys are M6.4's.
  - `GET /` serves `index-DbxlTsgc.js`, which exists only in this build.
  - `POST /api/chat` as `deploy-verify-a@m64.test` with
    `{"text":"release check 0.5.0-m6.4","name":"Deploy"}` → **201**
    `{"seq":1,"created_at":1789617236,"author":{"kind":"watcher","user":"deploy-verify-a@m64.test","name":"Deploy"},"text":"release check 0.5.0-m6.4"}`;
    `GET /api/chat` as `deploy-verify-b@m64.test` → that one message, `last: 1`;
    `GET /api/chat` with no identity header → **401**. That release-check line
    stays in the house chat — there is no delete (R-6.4.14).
  - **A table's stream carries the count, and a held stream receives a message
    (03:54Z):** table `sim-2a7e5dbb21cef3c2` (2P, both seats named, a password)
    created 201; the on-connect `…/stream` frame at `dealing` read
    `{"events":0,"to_move":null,"status":"dealing","your_turn":false,"chat":0}`;
    the two keeps were refused 400 (a wrong body on the operator's side, not
    re-tried), so the table stayed at `dealing`; seat A then held `…/stream` for
    9 s while seat B posted at +3 s → B's post **201**, and A's held stream
    received the frame with `"chat":1` **three seconds later** (03:54:24 →
    03:54:27); A's `GET …/chat` read the line with `author.kind: "seat", seat: 1`.
    Read at its real size: a HELD stream receiving a chat frame through the
    tunnel at `dealing` (the game screen holds its stream at every status since
    M6.4), not the acting seat during its own playing turn — that variant, and
    every visual claim (the panel folded at the column's foot with its count, the
    chips and names, the watching marker), is the owner's headed pass (spec
    R-6.4.16, amended by the owner to be their own), noted for M6.5's entry. The
    table was deleted by its creator (204).
- **Downtime:** the seconds between `stop` and `start`; nobody was playing.
- **Rollback:** `yokohama-web:0.5.0-m6.3` stays on the NAS; recreate the
  container from that tag with §3's body INCLUDING the `Env` line. **Both halves
  roll or neither does:** the M6.4 client reads `Events.chat` and the `/api/chat`
  routes, which the M6.3 server lacks; the chat documents this build writes
  (`<id>.chat.json`, `_house.chat.json`) are files an M6.3 server never lists and
  never deletes with a game, so a rollback leaves them inert on the volume.
  **Thirteen tags now sit on the NAS** — the twelve of §3.9 plus `0.5.0-m6.4`.

### 3.11 M6.4.1 is live — the eighth deployment, 2026-09-20 (a record, not a plan)

A POINT RELEASE, cut the same evening the M6.4 close's headed live review returned
SHIP WITH FIXES (0 Critical / 1 Important / 4 Minor) — the M6.2 precedent (§3.8's
`.1`) — under the owner's standing authorization for the wrap-up session ("any
changes made as part of the final review process here can be deployed to the NAS
without additional authorization", 2026-09-19). What changed is the browser half
and three server doc comments: at a panel narrower than 34rem a chat line's text
takes its own row (the Important — a long label had left the message a column
eight to eleven characters wide at 390 px, and the same squeeze was photographed in
the DESKTOP side column, which is why the rule is a container query and not a
viewport one); the identity strip wraps its name field onto its own line under
480 px; an own post on an open count-driven panel no longer costs a delta GET; the
setup screens' heading rules no longer leak into the panel; and `index.html`
carries `<link rel="icon" href="data:,">`, which ends the one console error every
HEADED browser had logged and every headless review had been blind to. The create
body is §3.10's, unchanged. The close `docs/plans/2026-09-19-web-frontend-m6.4-close.md`
§4.5 carries the review and its re-check.

- **Build:** `web-room-m6.4` at `b16ccdd` (the second fix wave's head: `6f76b02`
  the client, `b16ccdd` the server doc sentences, over `ebd9911`). The musl
  cross-compile per §1: `Finished release in 13.06s`; the binary is `ELF 64-bit
  LSB pie executable, x86-64, static-pie linked`, **6,796,984 bytes** — the same
  size as §3.10's, the server half of the wave being comments; `web/dist` carries
  the M6.4.1 client, bundle `index-CbONnoLE.js` (§3.10's was `index-DbxlTsgc.js`).
  Context tarball **14,112,926 B** (sha256 prefix `96bb94fce12e4912`), cut at
  **00:37:04Z** and served by `python3 -m http.server 8899 --bind 0.0.0.0` on the
  workstation at **192.168.1.16**; the NAS (192.168.1.173) fetched it in one GET at
  20:37:21 local and the file server was stopped at 00:37:49Z.
- **Image:** `yokohama-web:0.5.0-m6.4.1`, Id
  `sha256:1d23eec39a1cff37edaa4f0f6aa62aecdc0017d71fa5fd95f438b36630996c11`,
  **26,723,904 B** (§3.10's 26,722,785), created **00:37:36Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.4.1&dockerfile=Dockerfile&rm=true`
  (Portainer environment 2).
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.4` container
  (`f5db3dfe…`, created 2026-09-17 03:53:42Z) at ~00:37:50Z → `DELETE
  /containers/yokohama-web` → `POST /containers/create?name=yokohama-web` with §3's
  body (Image, `Env` `YOKOHAMA_WEB_IDENTITY=claimed` alone, the two labels,
  `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named volume
  `yokohama-web-data` → `/data`) → container Id
  `a62b17adfc2daa55aa0732ed61f85c74f0c0a7fefd30b0063130cd01b361182f`, created
  **00:37:58Z**, started **00:38:01Z**. Same address on `cf-internal`
  (**172.21.0.4**), so cloudflared was not restarted; the named volume rode across.
- **Verified through the tunnel at 00:38:28Z** (WARP on — it had been found
  DISCONNECTED at the session's start and was reconnected with `warp-cli connect`
  and `warp-cli debug access-reauth`, which is the door recipe R-6.4.16 assumes and
  this workstation does not keep by itself):
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed","signal":"sse","lobby_chat":true,"chat_keep":500,"chat_text":500}`.
  - `GET /` serves `index-CbONnoLE.js`, which exists only in this build, and
    carries `<link rel="icon" href="data:," />`.
  - **Chat, at a throwaway table rather than in the house (00:40Z):** table
    `sim-d61a6d99e20f73e5` (2P, `deploy-verify-a@m64.test` + the bot) created 201; a
    `POST …/chat` that carried a `name` was refused **400 "a seated caller is named
    by their seat"** — R-6.4.2's fence, live; without it **201**
    `{"seq":1,"author":{"kind":"seat","seat":0,…,"name":null},"text":"release check 0.5.0-m6.4.1"}`;
    a stream held from three seconds before the post received the on-connect frame
    with `"chat":0` and then the frame with `"chat":1` inside the held window;
    `GET …/chat` read the line with `last: 1`; the same read with no identity header
    **401**; `DELETE` **204**, and the table's `table` route answered 404 after. No
    house line was posted for this release; the house holds §3.10's release-check
    line and the live review's three.
  - The headed mobile re-check of the five fixes against this deployment is the
    close's §4.5 (an addendum by the same reviewer, same scaffold).
- **Downtime:** the seconds between `stop` and `start`; nobody was playing (the
  owner's one bot table sat at its turn).
- **Rollback:** `yokohama-web:0.5.0-m6.4` stays on the NAS; recreate from that tag
  with §3's body INCLUDING the `Env` line. Both halves roll together as §3.10
  says; rolling to m6.4 loses only the five review fixes and nothing on the
  volume. **Fourteen tags now sit on the NAS.**

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

**That path runs `YOKOHAMA_WEB_IDENTITY=worker`, not `claimed`**, because the
Worker is what authenticates: it reads the Access identity and sets
`X-Yokohama-User`, stripping any the client sent. `modal_app.py` declares the
variable on the image it builds; the NAS sets it on the container instead
(§3), which is the same decision made at the level that owns it in each case.

Neither is deployed. The NAS is serving the product today, and §3.3's
measurement says it should go on doing so.

---

## 5. Rollback

- **The tunnel:** restore `config.yml.bak-2026-09-01` on the NAS and restart
  `cloudflared-nas`. One file, one restart.
- **The container:** `docker stop yokohama-web`. The tunnel then 502s that
  hostname until the ingress is pointed back.
- **The previous build:** every earlier tag is still an image on the NAS (§3.9
  counts twelve). Stop and remove the running container and recreate it from
  the tag you want with §3's body — the volume is the same, so the records
  carry over.
- **Across M6.1 that is a WHOLE-IMAGE rollback, not a partial one.** Before
  M6.1 a rollback moved only the client and the help pictures. A pre-M6.1
  image has no `/table` route, no identity and no seat gate, so going back
  behind `0.5.0-m6.1` takes all three at once; the table documents on the
  volume are simply not read by the older binary, and the records still
  replay. Drop the `Env` line when recreating from a pre-M6.1 tag — that
  binary ignores `YOKOHAMA_WEB_IDENTITY` and leaving it in is harmless, but
  the line is only meaningful from `0.5.0-m6.1` on.
- **The store is not rolled back by either.** It is the named volume
  `yokohama-web-data`; the records are the whole state, which is why they
  replay.
