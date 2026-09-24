# Deploying the Yokohama browser client

**This is the record of a deployment that happened, not a plan.** The client went
live at `https://yokohama.moore.nyc` on 2026-09-01, served by a container on the
Synology NAS behind the existing cloudflared tunnel. Where an earlier draft of
this file guessed wrong, the guess is kept beside the correction — the wrong
guesses are the useful part.

**Two hosts since M6.5c.** The NAS (§0–§3, §5) serves `yokohama-nas.moore.nyc`; the
Cloudflare Worker (§6) serves hosted multiplayer at `yokohama.moore.nyc`, since the fourteenth
deployment on 2026-09-23 (§6.8).

---

## 0. What is true before you start

- **The client's hostname is `yokohama-nas.moore.nyc` since 2026-09-21, and it
  is behind Cloudflare Access.** It served `yokohama.moore.nyc` from 2026-09-01
  until M6.5b2 moved it and freed that name for the Worker (§3.14). Verified,
  not assumed: with no session every path answers **302** to
  `bdvc.cloudflareaccess.com`, and an enrolled caller is issued a
  `CF_Authorization` from it. **The client needs no Access application of its
  own** — the owner's existing `yokohama` application carries all three
  hostnames as destinations under ONE audience, and the policy is
  HOSTNAME-WIDE. (Both M4 documents carried this as UNVERIFIED; it is now
  settled, and §3.14 records how it was read rather than believed.)
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

**Since M6.5b the client build BUILDS THE WASM MODULE first** (`npm run build` =
`build:wasm && build:client`), so the workstation needs the target and the
generator on the PINNED toolchain, once:

```bash
rustup target add wasm32-unknown-unknown                   # onto the PINNED toolchain
cargo install wasm-bindgen-cli --version 0.2.128 --locked   # the lock's version; ~2 min from source
brew install binaryen                                       # M6.5b2: wasm-opt, a PREREQUISITE
```

**`binaryen` is not optional since M6.5b2** (R-6.5b2.15). `build:wasm` ends with
`wasm-opt -Oz --strip-debug --strip-producers` over both emitted modules, and
`web/scripts/wasm-opt.mjs` **refuses the build** when `wasm-opt` is not on
`PATH`, naming `brew install binaryen` and `apt-get install binaryen`.
`YOKOHAMA_WASM_OPT_OPTIONAL=1` turns that into a build that prints, out loud,
that it produced an UNOPTIMIZED module — **do not cut a deploy from one**: it is
1.78 MB instead of 1.17 MB.

**Which version, and where it matters.** The workstation reading is `wasm-opt
version 132`, and every byte figure in this document was taken with it; CI and
`deploy/Dockerfile` install that exact binary from the upstream release tarball,
pinned by SHA-256. `apt-get install binaryen` on Debian or Ubuntu gives **120**,
which emits a different module (1,174,709 B against 132's 1,173,055 B on the
same input). That is fine for developing on a Linux box and it is NOT the build
to read a recorded figure or a release budget off. The script prints the version
it used on every run.

`build:wasm` compiles `crates/yokohama-wasm` with **`--profile release-wasm`**
(M6.5b2: `opt-level = "z"`, fat LTO, one codegen unit — the server binary stays
on plain `--release`) for `wasm32-unknown-unknown`, reading it from
`target/wasm32-unknown-unknown/release-wasm/`, and runs `wasm-bindgen` twice:
the web glue INTO `web/src/local/wasm/` (gitignored; Vite bundles it and emits
the module as a hashed asset, `web/dist/assets/yokohama_wasm_bg-<hash>.wasm`,
**1,175,223 B raw / 456,048 B gzip since M6.5b2** — it was ~2.8 MB raw / ~760 KB
gzip — served `immutable` since the M6.5b fix wave; the figures are measured in
ONE place, the `release-wasm` comment in the root `Cargo.toml`, and re-measured
at each release) and the Node glue into `web/wasm-node/` for
`npm run test:wasm`. **Two things that bite:** (1) `npm run e2e` builds the DEBUG
module (`build:wasm:debug`, which carries the `debug_trap` export the trap cell
needs) and leaves it in `web/src/local/wasm/` AND `web/dist/` — so a deploy cut
after an e2e run ships a debug module unless `npm run build` is run again; the
preflight is `grep -c debug_trap web/src/local/wasm/yokohama_wasm.js`, which must
print `0` before the context is cut. (2) The `.wasm` rides in the context tarball
under `dist/assets/`, not under a `wasm/` directory — the runtime image needs no
change for it, and `assets.rs` serves it as `application/wasm`.

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
COPYFILE_DISABLE=1 tar czf ctx.tar.gz -C ctx .   # ~11.3 MB since M6.5b2; no ._ sidecars (§3.15)

python3 -m http.server 8899 --bind 0.0.0.0   # on the LAN, e.g. 192.168.1.16
```

Then, through Portainer's Docker proxy:

```
POST /build?remote=http://<workstation-lan-ip>:8899/ctx.tar.gz&t=yokohama-web:0.5.0&dockerfile=Dockerfile&rm=true
```

Resulting image: **19 MB**. Stop the file server afterwards.

**`COPYFILE_DISABLE=1` is on the `tar` line since §3.16.** Without it, macOS
`tar` writes an AppleDouble `._name` entry beside every file that carries an
extended attribute, and every image cut from this workstation from §3.5 through
§3.15 carried them (§3.15 counted 169 in the running container). Nothing served
them, but they were in the image. Check the context before you serve it:
`tar tzf ctx.tar.gz | grep -c '/\._'` must print `0`.

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
  "Env": ["YOKOHAMA_WEB_IDENTITY=claimed", "YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive"],
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

**The second `Env` entry is the archive (M6.5a, umbrella R-6.5.4), and it is
on the SAME named volume as the store**: a finished game is written under
`/data/archive/rev7/over/` and an abandoned one under `…/abandoned/`, each as
the record `<id>.json` beside its stripped table document `<id>.table.json`, once
per game id — so the NAS collects saves during development, and `/api/health`
answers `"archive": "dir"`. Unset, the archive is off and health says `"off"`;
the image sets no default, for the reason it sets no identity.

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

### 3.12 M6.5a is live — the ninth deployment, 2026-09-20 (a record, not a plan)

The first release with the SECOND `Env` entry: `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`
(umbrella R-6.5.4, M6.5a R-6.5a.9), on the store's own named volume. Deployed on the
owner's in-session authorization ("if we should then let's do so", after a fresh
whole-branch read of the close and the deploy-facing surfaces at HEAD — nothing found
that would block it; the two things it did find are the rollback caveat below and a
LEAVE item on the M6.5b handover). What changed for a player is one button and one
label: **Save game file** on a finished game's header, and "Abandon and save" on a
playing table. What changed underneath is the whole cycle (the M6.5a close
`docs/plans/2026-09-20-web-frontend-m6.5a-close.md` §1): the table layer out from
under axum, the archive of finished and abandoned games, the record route. Every
committed wire payload but two is byte-identical to `0.5.0-m6.4.1`'s.

- **Build:** `web-room-m6.5` at `8a6b391` (the close's docs commit over the close
  HEAD `42832f8`; PR #35 against `web-room-m6.4`, CI green on all three jobs). The
  musl cross-compile per §1: `Finished release in 7.20s` (a warm dependency cache —
  only the crate itself recompiled); the binary is `ELF 64-bit LSB pie executable,
  x86-64, static-pie linked`, **6,525,776 bytes** — 271 KB SMALLER than §3.11's
  6,796,984, the reason not measured (twenty handlers left axum's extractor generics
  this cycle, which is the obvious candidate and not a finding). `web/dist` carries
  the M6.5a client, bundle `index-Du-Mt0Ao.js` (§3.11's was `index-CbONnoLE.js`).
  Context tarball **14,066,259 B** (sha256 prefix `099534dc04ccdfa6`), cut at
  **14:02:59Z** and served by `python3 -m http.server 8899 --bind 0.0.0.0` on the
  workstation at **192.168.1.16**; the NAS (192.168.1.173) fetched it in one GET at
  14:03:49Z and the file server was stopped at 14:04:16Z. Before the build, the
  per-edit gates at HEAD read: web **259** (247 lib + 2 gate + 10 wire), clippy clean,
  the wasm check line clean, typecheck clean, vitest **640** in 38.
- **Image:** `yokohama-web:0.5.0-m6.5a`, Id
  `sha256:ee64642fc70738fb0eab2b24399fb0dec9bc5c42b9214e975c1d70d842a868e5`,
  **26,453,918 B** (§3.11's 26,723,904), created **14:04:00Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.5a&dockerfile=Dockerfile&rm=true`
  (Portainer environment 2).
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.4.1` container
  (`a62b17ad…`, created 2026-09-20 00:37:58Z) at ~14:04:18Z → `DELETE
  /containers/yokohama-web` → `POST /containers/create?name=yokohama-web` with §3's
  body — **both `Env` entries**, `YOKOHAMA_WEB_IDENTITY=claimed` and
  `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`, the two labels, `NetworkMode:
  cf-internal`, `RestartPolicy: unless-stopped`, the named volume `yokohama-web-data`
  → `/data` — → container Id
  `579cd1ffb1e744ecf63a980a6b7f58f7c37c212afa2356d7363e6b7369aa0580`, created
  **14:04:31Z**, started **14:04:35Z**. Same address on `cf-internal`
  (**172.21.0.4**), so cloudflared was not restarted; the named volume rode across.
  **The server created `/data/archive` itself at start** (`DirArchive::from_env`'s
  `create_dir_all`), owned by uid 10001 `yokohama:nogroup` — the named volume's
  ownership rule in §3 holds for the archive as it does for the store.
- **Verified through the tunnel at 14:04:50Z** (WARP connected):
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed","signal":"sse","archive":"dir","lobby_chat":true,"chat_keep":500,"chat_text":500}`
    — the `"archive": "dir"` this section's placeholder named as the check.
  - `GET /` serves `index-Du-Mt0Ao.js`, which exists only in this build.
  - The owner's one table (`sim-54b139c22c136cb0`, a SCHEMA-2 document written under
    M6.4, playing, at its turn) is listed under `scope=mine` and reads with every new
    field at its default — the schema-3 read of a schema-2 file, live. A bare
    `GET /api/games` is **401**; `GET …/record` on that running game is **409**
    "the record is available once the game is over".
  - **The abandon site (14:05:46Z):** a throwaway table `sim-8ee721d4abc0b774` (2P,
    `deploy-verify-a@m65.test` + the bot) created 201, its keep made, ONE move
    (`EndAdditionalPhase`) applied, then `DELETE` **204** and `…/table` **404**
    after. Off the volume: `/data/archive/rev7/abandoned/sim-8ee721d4abc0b774.json`
    (974 B, one event, `terminal_digest` `43fe3db5b3751098`) and `.table.json`
    (614 B, `schema: 3`, `lock: null`, `unlocked: []`, `archived_at: 1789913146`);
    the store's copy gone.
  - **The game-over site (14:05:49Z → 14:15:29Z):** a throwaway table
    `sim-445fb0ef81e7b844` (same seats) played to the END through the JSON API by a
    scripted random walk — the e2e harness's click policy at the wire (options and
    picks by number, one to three areas then `confirm`, a retreat count and
    `confirm` at its last step, `cancel` on a refused placement); the script lives
    in the session's scratchpad, not the repo. **338 events, the Church track
    filled, the bot won 97 to 12.** Off the volume:
    `/data/archive/rev7/over/sim-445fb0ef81e7b844.json` (32,894 B) is
    **byte-identical** to `/data/games/sim-445fb0ef81e7b844.json` (`cmp`; both
    sha256 `99f8e7dcfbbd8ce7…`) AND to the body `GET …/record` served to the seat
    holder (the same sha256, computed on the workstation) — the three copies are one
    string; `.table.json` (609 B, `schema: 3`, `over: true`, `winner: 1`, no lock,
    no roster, `archived_at: 1789913729`), and the store's table document carries
    the same stamp. **The served record loads in the terminal**: `cargo run -p
    yokohama-cli -- <file>` opened it at GAME OVER — the Church track filled — and
    `why` reconstructed the bot's last move (event 337, `v4-step9-draft1`) with no
    refusal. The throwaway was then deleted (**204**; already stamped, so `delete`
    put nothing — the archive still holds exactly the two pairs above) and the
    volume is back to the owner's one table. The record is kept gitignored at
    `docs/temp/owner-games/web-2026-09-20-deploy-check-sim-445fb0ef81e7b844.json`.
  - The container's stderr after both games is the one `listening on` line — no
    `archive of … failed` line, which is the only sign this host has (close §7).
- **A gotcha for scripted checks through the tunnel:** Cloudflare's browser-integrity
  check answers **403 `error code: 1010`** to `Python-urllib`'s default user agent
  (curl's passes). It is the edge, not the server, and the fix is any other
  `User-Agent` string. The first two driver runs failed on it.
- **Downtime:** the ~17 s between `stop` and `start`; nobody was playing (the owner's
  bot table sat at its turn).
- **Rollback:** `yokohama-web:0.5.0-m6.4.1` stays on the NAS; recreate from that tag
  with §3's body (the archive entry is harmless to an older binary, which ignores
  it). **NEW since this release, and it cuts the other way from every rollback before
  it: `TABLE_SCHEMA` is 3, and a table CREATED under `0.5.0-m6.5a` is written at
  schema 3, which the `0.5.0-m6.4.1` binary REFUSES to read** ("schema 3 — this
  server reads up to 2; a newer server wrote it"): its listing SKIPS such a table
  with a line on stderr and every direct read of it fails. Tables created before
  this release stay at schema 2 — only `create` stamps the schema; every other write
  keeps the document's own — so the owner's table is safe either way, and a
  schema-3 table comes back untouched when the newer binary is restored. Rolling to
  m6.4.1 loses the record route, the button and the archive sites, and nothing on
  the volume. **Fifteen tags now sit on the NAS.**

---

### 3.13 M6.5b is live — the tenth deployment, 2026-09-21 (a record, not a plan)

The first release whose client carries the WASM MODULE: single-player runs in the browser
(M6.5b, the close `docs/plans/2026-09-21-web-frontend-m6.5b-close.md`). Deployed on the
owner's authorization of 2026-09-20 21:58 ("the deploy authorization is present, so is a push
and PR at the wrap"), after the Fable whole-branch review's one fix wave, its scoped re-review
and the §6.6 battery at the fixed head. What changed for a player: the first screen opens on
**"On this device"** with **Play the bot here**; a local game plays with no request to this
server; **Save game file** at any point on it; **Abandon and save** and **Send to the archive**
post the record and the local table document to the NEW `POST /api/archive`, which files them
under `/data/archive/browser/<caller>/rev7/{over,abandoned}/`. What changed for the server:
two routes (`GET …/at?from=&to=` and `POST /api/archive`), `Cache-Control: immutable` on the
hashed assets, `.wasm` served as `application/wasm`. Every committed wire payload but the eleven
new ones is byte-identical to `0.5.0-m6.5a`'s.

- **Build:** `web-room-m6.5b` at `8ed985c` (the close's docs commit over the close HEAD
  `e15e369`; PR against `web-room-m6.5` opened at the wrap). The musl cross-compile per §1:
  `Finished release in 7.90s` (warm cache); the binary is `ELF 64-bit LSB pie executable,
  x86-64, static-pie linked`, **6,633,736 bytes** (§3.12's 6,525,776 + 108 KB: the two routes
  and the archive's protocol). The client build was `npm run build` — the RELEASE module, run
  AFTER the close's e2e had left the debug one in place (§1's preflight `grep -c debug_trap
  web/src/local/wasm/yokohama_wasm.js` read **0**): `web/dist/assets/` carries
  `index-nGNJHvmy.js` 329,045 B, `index-CEXGGp1D.css` 34,955 B, `worker-C542ORIG.js` 13,174 B
  (the Worker's own chunk) and **`yokohama_wasm_bg-BJZwkgIr.wasm` 2,842,659 B (758.15 kB
  gzip), sha256 `3ca08c7b4943c19e…`**. Context tarball **14,860,814 B** (§3.12's 14,066,259 +
  the module's compressed size; sha256 prefix `b0fd0318e62150de`), cut at **04:55:56Z**, served
  by `python3 -m http.server 8899 --bind 0.0.0.0` on the workstation at 192.168.1.16; the NAS
  (192.168.1.173) fetched it in one GET at 04:57:02Z; the file server was stopped at 04:58:02Z.
  Before the build the counts at `e15e369` read (the close §2): web 274, wasm 4, CLI 851 + 2,
  clippy and the wasm check clean, typecheck, vitest 746/42, test:wasm 2, e2e 46/9 in 4.0 min.
- **Image:** `yokohama-web:0.5.0-m6.5b`, Id
  `sha256:d033b1b10d49017506906a79794e9a4d429d3a5c8865cc7876e0c37ee0793d54`, **29,429,776 B**
  (§3.12's 26,453,918 + 2.98 MB, the module), created **04:57:15Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.5b&dockerfile=Dockerfile&rm=true`
  (Portainer environment 2).
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.5a` container (`579cd1ff…`,
  created 2026-09-20 14:04:31Z) at ~04:58:00Z → `DELETE /containers/yokohama-web` →
  `POST /containers/create?name=yokohama-web` with §3's body — **both `Env` entries**, the two
  labels, `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named volume
  `yokohama-web-data` → `/data` — → container Id
  `1a14939b7c8fc1b6e47ac535c45f03d7b9f6e4556afd5bb35e29716157f0bf1d`, created **04:58:50Z**,
  started **04:59:06Z**. Same address on `cf-internal` (**172.21.0.4**), so cloudflared was not
  restarted; the named volume rode across (the owner's table and the archive intact).
- **Verified through the tunnel at 04:59:33Z** (WARP connected; `User-Agent` set — §3.12's
  gotcha):
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed","signal":"sse","archive":"dir","lobby_chat":true,"chat_keep":500,"chat_text":500}`.
  - `GET /` serves `index-nGNJHvmy.js`, which exists only in this build.
  - `GET /api/games` **401**.
  - **`GET /assets/yokohama_wasm_bg-BJZwkgIr.wasm` → 200, `content-type: application/wasm`,
    `content-length: 2842659`, `cache-control: public, max-age=31536000, immutable`** — the two
    R-6.5b.14 and fix-wave claims, live; `worker-C542ORIG.js` the same header; `GET /` carries
    NO `cache-control`, as the wave's cell requires.
  - `GET /art/areas/Church.webp` 200 `image/webp`.
  - The HEADED live review of the deployed site is `.superpowers/sdd/2026-09-20-web-frontend-m6.5b-plan/live-review.md`
    and the close's §4.6: **SHIP WITH FIXES** — zero `/api/` requests across a whole local
    session, both takeover directions, the abandon landed under `browser/<caller>/rev7/abandoned/`,
    "not connected" at 5.1 s on a hung API; one Important (a chat panel on a LOCAL game's DEAL
    screen) fixed as the point release below.
- **Downtime:** the ~66 s between `stop` and `start` (the four swap calls were made one per
  tool turn this time); nobody was playing.
- **Rollback:** `yokohama-web:0.5.0-m6.5a` stays on the NAS; recreate from that tag with §3's
  body. **Nothing in this release changes a stored document's schema** (`TABLE_SCHEMA` stays 3,
  the record format is fenced), so unlike §3.12 a rollback across it is NOT a schema rollback:
  every table this release created reads under `0.5.0-m6.5a`. What a rollback loses: the two
  routes (a client at m6.5a never calls them), the module's `immutable` header, and — for a
  browser that already holds the m6.5b client — nothing, since a local game never touches the
  server; a browser game's "Send to the archive" would answer 404 until the newer binary is
  back. **Sixteen tags now sit on the NAS.**
- **The point release `0.5.0-m6.5b.1`, 06:40Z the same night** — the headed live review's one
  Important (a Chat panel on a LOCAL game's DEAL screen, accepting messages from `local@browser`)
  fixed at `9bd1d4b` (the RED cell `253387d` first). A client-only change: the musl binary is the
  same 6,633,736 bytes from the same Rust; `npm run build` re-cut the bundle as `index-BEX0dlqy.js`
  with the module and the Worker chunk unchanged by hash; context tarball 14,860,110 B, fetched by
  the NAS at 06:39:33Z; image `yokohama-web:0.5.0-m6.5b.1` Id `sha256:a13d983096ed…` 29,429,784 B
  created 06:39:45Z; the swap (stop ~06:39:59Z → create 06:40:14Z → start **06:40:19Z**) to
  container `0c3e49f6b21726edc1dff9e8b4388c13a07df562bac5dbaf589cbeca8e3249dc`, same address
  172.21.0.4, both `Env` entries, the volume across; verified through the tunnel at 06:40:33Z
  (health `archive: dir`; `/` serves `index-BEX0dlqy.js`; `/api/games` 401; the module
  `application/wasm` + `immutable`). Not re-walked headed and not re-run through the e2e (the
  close §7 says so). **Seventeen tags now sit on the NAS.**

---

### 3.14 M6.5b2's door is live on the NEW hostname — the eleventh deployment, 2026-09-21 (a record, not a plan)

A **MID-CYCLE** build: the door, not the cycle's release. M6.5b2 Tasks 1–3 are in — the server's
three switches (`YOKOHAMA_WEB_MULTIPLAYER`, the `dev-identity` cargo feature, the archive
directory), the `/enter` turnstile, the front door at `#/` with "On this device" and "Play with
others", and the lobby moved to `#/lobby` — and the NAS took **`yokohama-nas.moore.nyc`**, freeing
`yokohama.moore.nyc` for the Worker. The cycle's remaining work (Tasks 5–9: the module's size, the service
worker, installability, the backup envelope and the outbox) is not in this build, so this is a
development deploy and not a release.

- **Build:** `web-room-m6.5b2` at `553e3ee`, tree clean. The musl cross-compile per §1: `Finished
  release in 5.82s` (warm cache); the binary is `ELF 64-bit LSB pie executable, x86-64, static-pie
  linked`, **6,642,280 bytes** (§3.13's 6,633,736 + 8,544: the three switches and the turnstile).
  The client build was `npm run build` — the RELEASE module; §1's preflight `grep -c debug_trap
  web/src/local/wasm/yokohama_wasm.js` read **0**, and vite's own
  `"debug_trap" is not exported by "src/local/wasm/yokohama_wasm.js"` warning is the same fact from
  the other side, since only the debug module carries that export. `web/dist/assets/` carries
  `index-COGqoKfc.js` 332,072 B, `index-CcfDU4W0.css` 35,524 B, `worker-CqmNdmwq.js` 13,174 B and
  **`yokohama_wasm_bg-CZUeFC4o.wasm` 2,846,763 B (758.03 kB gzip)** — the plain `release` profile
  still, as M6.5b left it; Task 5 is what shrinks it. Context tarball **14,866,798 B**, sha256
  `a46d2d2528ba3707b57b9c644f492680047c018488935c49e25acdfde1030eb1`, cut at **22:18:58Z**, served
  by `python3 -m http.server 8899 --bind 0.0.0.0` on the workstation at 192.168.1.16; the NAS
  (192.168.1.173) fetched it in one GET at **22:19:06Z**; the file server was stopped at
  **22:19:25Z**.
- **Image:** `yokohama-web:0.5.0-m6.5b2-rc1`, Id
  `sha256:2ee5e7bbfc6742fe30d36e00ae91fff3ce9b92428346e7acb08c3e2f99cd90c0`, **29,446,020 B**,
  created **22:19:17Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.5b2-rc1&dockerfile=Dockerfile&rm=true`
  (Portainer environment 2).
- **Swap:** the four calls ran between **22:19:25Z** and **22:19:42.87Z** — `stop` on the
  `0.5.0-m6.5b.1` container (`0c3e49f6…`, created 2026-09-21 06:40:14Z) → `DELETE` →
  `POST /containers/create?name=yokohama-web` with §3's body — **both `Env` entries** and
  **`YOKOHAMA_WEB_MULTIPLAYER` deliberately UNSET, so it defaults to `on`** (R-6.5b2.3 keeps
  `claimed`), the two labels, `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named
  volume `yokohama-web-data` → `/data` — → container Id
  `08e036107e74d3da9cd17237a537056d31b21210a6e7b658cfb8e9771ba32695`, created **22:19:40.24Z**,
  started **22:19:42.87Z**. Same address on `cf-internal` (**172.21.0.4**), so nothing about the
  container needed cloudflared. Downtime under 18 s; nobody was playing.
- **Verified through the tunnel at 22:19:49Z** (on `yokohama.moore.nyc`, which was still the live
  name at that minute):
  - `GET /api/health` →
    `{"server_version":"0.5.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"claimed","signal":"sse","archive":"dir","lobby_chat":true,"chat_keep":500,"chat_text":500,"multiplayer":true}`
    — **`multiplayer: true`** and **`archive: dir`**, the two readings this deploy owed.
  - `GET /` serves `index-COGqoKfc.js`, which exists only in this build.
  - `GET /api/games` **401**; `GET /enter` **302 → `/#/lobby`**, which is Task 1's turnstile answering
    from the deployed binary.
  - `GET /assets/yokohama_wasm_bg-CZUeFC4o.wasm` 200, `application/wasm`, `content-length: 2846763`,
    `cache-control: public, max-age=31536000, immutable`; `GET /art/areas/Church.webp` 200 `image/webp`.

#### The hostname move, in the order it was performed

**The Access application was NOT created or changed here.** The owner ruled on 2026-09-21 that the
hostname-wide policy stays through development, and had already added the new name to the existing
application. Read from the API rather than assumed: application **`yokohama`**, id
`11cbcf68-96cd-4310-baee-f2ade812155d`, audience
`5fa7a989f4f4970771769d68ff273497c125f0374253c0ccf817952dc9c3e8c7`, `session_duration` 730h, with
**three destinations** — `yokohama.moore.nyc`, `yokohama-cli.moore.nyc`, `yokohama-nas.moore.nyc` —
and two **allow** policies, `sbaitso-friendly` (4 emails) and `yokohama-players` (3 emails). No IP
rule, no bypass. So "protect first, then route" was satisfied before step 1 below, and verified at
step 3 before the name was left routed.

1. **DNS.** Proxied CNAME `yokohama-nas.moore.nyc` → `6a6c468e-5e1e-4a7c-88a6-4e2fe07da1fc.cfargotunnel.com`,
   record id `622c274ecda97fe11c65a34bc7e2b2a9`, created **22:20:13.81Z** (§3.1: DNS is not automatic).
2. **Ingress, transitional.** `/volume1/docker/cloudflared/config/config.yml` backed up as
   **`config.yml.bak-2026-09-21`** (2,770 B, sha256 `00ecb105f3043cbdaa3dbc4f72c63fe7919fcedd42026dd30c225e36696176a0`)
   and rewritten with the new block ABOVE the catch-all and the old block KEPT — the writing
   container was created at **22:22:17Z** — then `POST /containers/cloudflared-nas/restart`, whose
   time was never read off a log. **All that is known is the interval**: the restart was issued
   after the config write at **22:22:17Z** and before a reading taken at **22:22:41Z**. An earlier
   draft split that interval into an estimate of ≈22:22:29Z and a twelve-second gap after it, which
   is the same unmeasured quantity stated twice as if one of them had been read.
3. **The protection reading, taken before leaving it routed.** From a vantage with **no session**,
   `https://yokohama-nas.moore.nyc/` read **302 → `https://bdvc.cloudflareaccess.com/cdn-cgi/access/login/yokohama-nas.moore.nyc?kid=5fa7a989…`**
   — not 200. **One earlier reading, and the sequence is worth getting right.** At **22:23:16Z** an
   unenrolled probe could not connect to the new name at all — no A record, the DNS note below — so
   it read nothing about Access. The first unenrolled request that did reach the edge, at
   **≈22:23:45Z with the address pinned, about seventy-five seconds after the restart**, read **403**
   rather than 302 while the destination propagated. A refusal either way, and it did not recur.
   An earlier draft of this record called that 403 "seconds after the restart" and placed it at the
   22:23:16Z probe; both were wrong. The times above are the container timestamps, and the 403 sits
   AFTER the ingress restart, not before it.
4. **Ingress, final.** The `yokohama.moore.nyc` block retired, `config.yml` rewritten (sha256
   `b6f8ecafb71764fa36ff308959f5b03bf63eeba91cf3c7c6117c467e9966df4a`), `cloudflared-nas` restarted
   at **22:28:49Z**; four tunnel connections registered by 22:28:53Z with no ingress error.

**`yokohama.moore.nyc` is now HELD FOR THE WORKER.** Its DNS record still points at this tunnel, so
a request reaches cloudflared and falls through to the catch-all: every path reads **404** from an
enrolled shell, and from an unenrolled one the 404 is not even observable, because Access answers
302 first. **One exception, and it is the edge and not the origin:** `/assets/index-COGqoKfc.js`
still read **200** on the old name with `cf-cache-status: HIT` and `age: 292` — the immutable hashed
asset is in Cloudflare's cache and will age out. Nothing else on that hostname answers.
**Local single-player games did not travel**, because browser storage is per ORIGIN; nothing of
consequence was stored (owner, R-6.5b2.5).

**A drift the move surfaced, worth its own line.** The NAS's `config.yml` had not been touched since
2026-09-02 and still carried the **pre-M6.1** sentence *"this origin has NO authentication of any
kind"* — the repo's copy had been corrected and the deployed file had not. The file installed here is
byte-identical to `deploy/cloudflared-config.yml` **as it stood at the deploy** (sha256
`b6f8ecaf…`), so the two agreed again at that moment. **They no longer match byte-for-byte**: the
fix round that followed this record's review reworded two COMMENTS in the repo copy (this file's
sha256 is now `e487c6aa…`) and the NAS was deliberately not touched to re-sync them,
since a comment is not ingress. The ingress blocks are identical and that is the part that runs;
whoever next restarts `cloudflared-nas` should copy this file over and close the gap.

#### THE MEASUREMENT VANTAGE — read this before trusting any reading below

§3.2 already says the deploy check **needs WARP active**, because Access authenticates an enrolled
device silently. The corollary this deployment had to discover is the other half of that sentence:
**with WARP on, the workstation IS an authenticated caller, so a "no session" reading cannot be taken
from it at all** — every `curl` from it is the owner, and the app's own answers are what come back.
The unenrolled vantage used for every 302/401 below was a **throwaway `alpine` container on the NAS
itself** (`NetworkMode: bridge`, egress the house IP, no WARP and no cookie), driven through
Portainer's Docker proxy. **Nine such containers were created across this deployment** — five
probes, one DNS diagnostic, one config read and two config writes — **and all nine were deleted.**
Both vantages are recorded because they answer different questions: the unenrolled one says whether
the door is shut, the enrolled one says what the application does once you are through it.

**A second wrinkle, recorded because it will look like a failure to the next reader, and because
it is a cost of taking a NEW hostname rather than an existing one.** Two caching resolvers on this
network — the workstation's (WARP's) and the house router's at 192.168.1.1 — held a **NODATA answer
for the A record** of the new name for the whole session, from lookups made ~20 s after the record
was created. Diagnosed rather than guessed: the same resolvers return the **AAAA** records for that
name, and BOTH families for the old one, so it is a negative cache of one rrset and not a Gateway
policy suppressing the domain. (The account does carry a Gateway DNS rule `SecurityRisks` blocking
eleven security categories; a block there would have taken both families, and Cloudflare's intel API
reports no security category on either hostname. Worth an owner glance anyway if the name is still
unreachable tomorrow.) `moore.nyc`'s SOA minimum is **1800**, so the entries expire about thirty
minutes after they were taken. The zone's own nameservers, 1.1.1.1 and 8.8.8.8 all answered
correctly throughout, and every reading in this record was taken with
`--resolve yokohama-nas.moore.nyc:443:104.21.92.62` — which pins the address and leaves SNI, Host and
therefore both Access and the tunnel exactly as a browser would find them.

**The lesson for the next hostname move:** take the CNAME and then do not look at it for a minute.
A lookup taken in the seconds after the record is created can cache a miss for half an hour on every
resolver that hears it, including the owner's own laptop — and it is indistinguishable, from the
shell, from a deployment that did not work.

#### The two curl loops, both hostnames, both vantages

**UNENROLLED, no session (from the NAS), final state.** Identical on both names — which is the
comparison that matters, since the old name is meant to be no more reachable than the new one is
without a session:

**The asset row names a file this vantage never saw.** With no session the page is not served, so
there is nothing to scrape a hashed name out of; `index-COGqoKfc.js` is supplied from the build
(§1's `dist/assets/` listing) and requested by name. That is deliberate — §3.2 wants a REAL hashed
asset rather than the `/assets/` directory, which the SPA fallback would answer either way — but it
means this row tests the door and not the file map.

```
                                    nav   xhr   www-authenticate
/                                   302    —
/assets/index-COGqoKfc.js           302    —
/art/manifest.json                  302    —
/api/health                         302    —
/api/labels                         302    —
/api/art                            302    —
/api/games                          302   401   Cloudflare-Access
/api/games/x/input                  302   401   Cloudflare-Access
/api/chat                           302   401   Cloudflare-Access
/api/stream                         302   401   Cloudflare-Access
/api/games/x/stream                 302   401   Cloudflare-Access
/api/archive                        302   401   Cloudflare-Access
/api/me                             302   401   Cloudflare-Access
/enter                              302   401   Cloudflare-Access
```

**ENROLLED (WARP, the owner's identity), `yokohama-nas.moore.nyc`.** These are the APPLICATION's
answers, past the door:

```
/                                   200      /api/games            401
/assets/index-COGqoKfc.js           200      /api/games/x/input    405   (GET on a POST route)
/art/manifest.json                  200      /api/chat             401
/api/health                         200      /api/stream           401
/api/labels                         200      /api/games/x/stream   401
/api/art                            200      /api/archive          405   (GET on a POST route)
                                             /api/me               200
                                             /enter                302 → /#/lobby
```

The same loop on `yokohama.moore.nyc` after the retirement reads **404 everywhere** but the cached
asset noted above.

**The spec §3.2 public-path 200 readings are DEFERRED BY OWNER DECISION** and are not evidence either
way about the path-scoped design. §3.2 expects `/`, a hashed asset, `/art/manifest.json`,
`/api/health`, `/api/labels` and `/api/art` to answer **200 with no session** so that a browser
playing single player never meets the door. Under the hostname-wide application they answer **302**,
as the table above shows. That is the deployed policy behaving correctly, not a defect, and the
reading §3.2 asks for cannot be taken until the application is path-scoped. (A small thing to know
before comparing: **spec §3.2's step 3 is headed "FIVE DESTINATIONS" and then enumerates six** —
`/api/games`, `/api/chat`, `/api/stream`, `/api/archive`, `/api/me` and `/enter` — and its next
paragraph says "five applications would each need their own". This record follows the
ENUMERATION and says six.)

#### Step 4b — the positive reading, performed in part

On ONE session on the new hostname: `GET /enter` **302 → `/#/lobby`** issuing a single
`CF_Authorization` whose claims name `aud=5fa7a989…`, `email=jamoore@gmail.com`,
`host=yokohama-nas.moore.nyc`; then, with that jar and no further navigation —

```
GET  /api/me                 200   {"mode":"claimed","user":"jamoore@gmail.com"}
GET  /api/games              200   the owner's games
GET  /api/stream             200   text/event-stream; both house bumps arrived on it —
                                   {"seq":3,"chat":0} from a create, {"seq":4,"chat":0} from a delete
POST /api/games              201   a smoke table, deleted below
GET  /api/games/<id>/chat    200   {"messages":[],"last":0}
POST /api/games/<id>/chat    201   author {"kind":"seat","seat":0,…}
GET  /api/games/<id>/stream  200   event: signal / data: {"events":0,"to_move":null,"status":"dealing",…}
POST /api/archive {}         400   "missing field" — the body is wrong, which is the point: not a 401
DELETE /api/games/<id>       204
```

and the jar held **exactly one** `CF_Authorization` at the end. Two smoke tables were created and
both were deleted.

**Read that at its real size.** On an enrolled device WARP re-authenticates every request on its own,
so a single-session pass does not by itself prove one token would have sufficed. The attempt to
remove that confound — replaying the same cookie from the unenrolled NAS — was **refused, 302 on
every route**, which is consistent with a WARP-issued token being bound to the device that obtained
it. **The refusal is what was measured; the mechanism is not** — the token's own decoded claims,
printed during the reading, carried `aud`, `email`, `warp_as_auth` and the hostname, and no
`device_id` was read out of it. The load-bearing conclusion needs only the refusal: the cookie-only
reading cannot be taken from a second machine. What IS settled, and is the
stronger fact, is structural: there is **one** application on this hostname carrying **one** audience,
so the many-audiences failure §2.1 guards against cannot arise here. The reading that would test the
path-scoped design is owed with it.

#### Step 5 — OWNER-OWED

The headed walk of the deployed site (the door draws at `#/`; "Play on this device" starts a local
game with no sign-in; "Play with others" goes through `/enter` to the lobby; a signed-in hosted game
still plays) was **not performed** and is not claimed. No authenticated browser was available to the
session: **the Claude-in-Chrome extension was not connected**, which is the blocker on its own. The
workstation's resolver would also not return the new name's **A** record for the reasons recorded
above, under THE MEASUREMENT VANTAGE — a
`curl` from this machine read `Could not resolve host` throughout — but that is a weaker statement
than the one an earlier draft made: **no browser was tried here**, and the same resolvers did return
the **AAAA** records, so whether a browser on an IPv6-capable path would have loaded the site is
untested. M6.4's §4.5 is why a headless substitute was not offered in place of the walk.

#### What this deployment does NOT claim

- Nothing about the **path-scoped** Access design. It is unbuilt and unmeasured (spec §3.2).
- Nothing about Tasks 5–9. The module is still the plain `release` profile at 2,846,763 B, there is
  no service worker, no backup envelope and no outbox in this build.
- **A known interim state, accepted for this development deploy** (Task 2's review recorded it): on a
  first visit, **Abandon on a RUNNING local game deletes it without a send** — the "abandon and save"
  path does not yet reach the archive from that entry point. Task 9 closes it.

#### Rollback

`yokohama-web:0.5.0-m6.5b.1` stays on the NAS; recreate from that tag with §3's body. **Nothing in
this release changes a stored document's schema** (`TABLE_SCHEMA` stays 3, the record format is
fenced), so — unlike §3.12 — **a rollback across it is NOT a schema rollback**: every table this
release created reads under `0.5.0-m6.5b.1`. What a rollback loses is the three switches and the
`/enter` turnstile, so `#/` would fall back to the M6.5b first screen. **The hostname is a separate
rollback**: restore `config.yml.bak-2026-09-21` on the NAS and restart `cloudflared-nas` to put
`yokohama.moore.nyc` back (the DNS record for it was never removed). **Eighteen tags now sit on the
NAS.**

---

### 3.15 M6.5b2 is live — the twelfth deployment, 2026-09-22 (a record, not a plan)

The cycle's RELEASE, where §3.14 was the mid-cycle door. M6.5b2's remaining work is in:
the module is on `release-wasm` + `wasm-opt -Oz`, the site installs and runs offline behind a
service worker, the help pictures are WebP, the two text fonts are served from `/assets/` with
their OFL grants beside them, a game can be backed up and restored as a file, and a local game
that could not reach the archive queues in an outbox and sends itself later. Built at `43d54a5`
(the fix-wave head, whose mutation battery read 22 of 22 caught), tree clean. What changed for a
player is above; what changed for the operator is that the client is now a PRECACHED release —
`dist/sw.js` names one content-keyed cache generation, and a deploy that half-lands is a deploy
that fails its own install.

- **Build:** `web-room-m6.5b2` at `43d54a5`. The musl cross-compile per §1: `Finished release in
  5.71s` (warm cache); the binary is `ELF 64-bit LSB pie executable, x86-64, static-pie linked`,
  **6,591,376 bytes** — **50,904 bytes SMALLER than §3.14's 6,642,280**, which is the direction a
  reader will not expect from a cycle that added `backup.rs` and two routes, and is recorded
  because it was checked rather than assumed: the binary carries this release's strings
  (`application/manifest+json`, `text/plain; charset=utf-8`, `YOKOHAMA_WEB_MULTIPLAYER` all
  present), and `[profile.release]` did not move this cycle (the new `release-wasm` section is
  its own profile and nothing selects it without `--profile release-wasm`).
- **The client build was `npm run build`** — the RELEASE module. §1's preflight `grep -c
  debug_trap web/src/local/wasm/yokohama_wasm.js` read **0**, and vite's own `"debug_trap" is not
  exported by "src/local/wasm/yokohama_wasm.js"` warning is the same fact from the other side.
  `wasm-opt version 132` reported **1,783,361 → 1,175,223 B (−34.1%)** over both emitted modules
  (the web glue and the Node glue), which is the figure the root `Cargo.toml`'s `release-wasm`
  comment carries and this build reproduces. `web/dist/assets/` carries `index-BEMfPcF1.js`
  **358,444 B**, `index-CaXbZR8U.css` **37,738 B**, `worker-CI3Vv9s2.js` **14,809 B**,
  `fraunces-latin-BOz5zopO.woff2` **67,388 B**, `jetbrains-mono-latin-Db4Uuiha.woff2` **31,340
  B**, and **`yokohama_wasm_bg-Cf0j0Dq2.wasm` 1,175,223 B (456,054 B under `gzip -9`; vite
  printed 457.96 kB, the hashed filename in its own gzip header being the difference), sha256
  `277aa0c4e184a9b0313b7018b7957eb281d302fdcf85ff52e89a5ea49a09b9dc`**; outside `assets/`,
  `index.html` 2,387 B, `manifest.webmanifest` 696 B, `sw.js` 27,220 B and the two OFL texts
  under `dist/fonts/` at 4,391 and 4,399 B. **The raw figure is byte-for-byte the one the
  root `Cargo.toml`'s `release-wasm` comment carries; the gzip figure is 6 bytes over its
  456,048**, which is ordinary `gzip -9` variance and not a change in the module — the two
  readings are of the same 1,175,223 bytes, and they are recorded together here so a later reader
  does not take them for a contradiction.
- **The precache generator's own four lines, verbatim**, because they are the release budget and
  the cache identity in one place:

  ```
  precache: 153 entries, 9,935,340 B (release total ceiling 11,900,000 B)
  precache: module 1,175,223 B at /assets/yokohama_wasm_bg-Cf0j0Dq2.wasm (release module ceiling 1,400,000 B; release 1,400,000, debug 3,200,000)
  precache: module 1,175,223 · bundle 410,991 · fonts 98,728 · shell 11,873 · icons 19,456 · help 764,742 · art 7,454,327
  precache: yokohama-release-19e8532f977bcba2 -> dist/sw.js (27,220 B), 131 art files
  ```

  **The RELEASE ceilings applied, both of them** — 11,900,000 B total and 1,400,000 B for the
  module, not the debug 3,200,000 — which is what proves the release module was the one walked.
  The cache name **`yokohama-release-19e8532f977bcba2`** is the generation this deployment
  installs, and it is read back off the wire below rather than trusted from here.
- **Delivery:** context tarball **11,274,574 B**, sha256
  `d0b0b30e189377f65d38aa59cc45e26d6f47b2d71367f4391adad4c757eed8ff`, cut at **19:07:35Z** — a
  **3.59 MB drop** from §3.14's 14,866,798 B, which is the module's 1.67 MB and the help
  pictures' PNG→WebP (4.21 MB → 764,742 B) arriving together. Served by `python3 -m http.server
  8899 --bind 0.0.0.0` on the workstation at 192.168.1.16 from **19:07:43Z**; the NAS
  (192.168.1.173) fetched it in **one GET at 19:07:52Z**; the file server was stopped at
  **19:08:12Z** and a probe after it read connection refused.
- **Image:** `yokohama-web:0.5.0-m6.5b2`, Id
  `sha256:c802a036c2cb51a2360a7cddf30c795ce31c5823b66943dc0572a3b5657bc8b9`, **24,466,536 B**
  (§3.14's 29,446,020 − 4.98 MB, the same two savings), created **19:08:03.31Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.5b2&dockerfile=Dockerfile&rm=true`
  (Portainer environment 2). **This is the first release image SMALLER than the one before it.**
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.5b2-rc1` container (`08e036107e74…`,
  created 2026-09-21T22:19:40.24Z) → `DELETE /containers/yokohama-web` →
  `POST /containers/create?name=yokohama-web` with §3's body — **both `Env` entries**
  (`YOKOHAMA_WEB_IDENTITY=claimed`, `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`) and
  **`YOKOHAMA_WEB_MULTIPLAYER` deliberately UNSET, so it defaults to `on`**, the two labels,
  `NetworkMode: cf-internal`, `RestartPolicy: unless-stopped`, the named volume
  `yokohama-web-data` → `/data` — → container Id
  `58ac45ba478a57185c1753a55e9f7d800defac1648d7a9fc31c88b5b3fa09f37`, created **19:08:24.59Z**,
  started **19:08:27.10Z**, `healthy` on the probe eight minutes later. Same address on
  `cf-internal` (**172.21.0.4**), so the container itself needed nothing from cloudflared; the
  named volume rode across. **Downtime is a BOUND and not a reading**: the stop was issued after
  the file server stopped at 19:08:12Z and the new process was up at 19:08:27.10Z, so **under
  15.1 s**, and the stop's own timestamp was never read off a log. Nobody was playing.

#### The tunnel config re-sync — the owner item §3.14 left, now closed

§3.14 recorded that the repo's `deploy/cloudflared-config.yml` and the NAS's live `config.yml`
had drifted apart by reworded COMMENTS after that record's fix round, and that whoever next
restarted `cloudflared-nas` should close the gap. The owner authorized a restart at any time
(2026-09-21, mid-session), so it rode this deploy.

1. **Both files hashed first, and both matched §3.14's recorded values**: the NAS's
   `/volume1/docker/cloudflared/config/config.yml` 5,770 B sha256
   `b6f8ecafb71764fa36ff308959f5b03bf63eeba91cf3c7c6117c467e9966df4a`, the repo's 5,936 B sha256
   `e487c6aaaabebbf0411580b11fac1543c7f62cb1d0485322d740af93a9a0946f`. So the NAS file had not
   been touched since the move.
2. **The diff was taken ON THE NAS, under a guard that would have refused a non-comment change.**
   The repo copy was fetched by a throwaway container over the same LAN file server the context
   used, its sha256 checked against `e487c6aa…` before anything was compared, and the write was
   made conditional on every added-or-removed line matching `^[[:space:]]*(#.*)?$`. It printed
   `guard passed: every changed line is a comment or blank`. **The diff is exactly TWO hunks**,
   both comment prose: the recipe's step 4 now says the `yokohama` application carries three
   names of which only two are routed here, and the path-scoped paragraph now notes that spec
   §3.2's own heading says FIVE and then lists six. **The ingress blocks are byte-identical**,
   which is what §3.14 claimed and this is the check of it.
3. **Backup then write:** the old file is `config.yml.bak-2026-09-22` (5,770 B, `b6f8ecaf…`) and
   `config.yml` is now 5,936 B, `e487c6aa…`. **One incidental change worth naming:** the file's
   owner moved from `1026:users` to `root:root`, the mode staying `rwxr-xr-x`; cloudflared runs as
   65532 and reads it world-readable, which the tunnel coming back is the proof of. The three
   earlier backups on the NAS are root-owned for the same reason.
4. **Restart** issued **19:16:09Z**; the tunnel server stopped at **19:16:11Z**, `Starting tunnel`
   at **19:16:13Z** on cloudflared 2026.3.0, and **four connections registered by 19:16:15.75Z**
   (connIndex 0–3, ewr13 / ewr15 / ewr14 / ewr05, protocol quic) with **no ingress error**.
5. **Re-hashed after the restart: `config.yml` reads `e487c6aa…`, equal to the repo's.** The two
   agree byte for byte again, and this time the agreement was verified by a diff rather than by a
   pair of hashes that happened to match a record.

#### Verified through the tunnel — and the vantage, which bit before it helped

**Every reading below was taken from the WARP-enrolled workstation, which means each one is the
APPLICATION's answer to an authenticated caller** (§3.14's MEASUREMENT VANTAGE: with WARP on,
this machine IS the owner and cannot take a "no session" reading at all). Two passes, **19:11:27Z**
before the tunnel restart and **19:16:25Z** after it, identical in every row; the User-Agent was
set (§3.12's gotcha).

**§3.2's "this check needs WARP active" has a second half this deployment had to learn.** At
**19:08:37Z**, minutes after the swap, every path on `yokohama-nas.moore.nyc` read **302** to
`bdvc.cloudflareaccess.com` with the body `Please authenticate via the warp client` — and
`warp-cli status` read `Connected`, `Network: healthy`, registration intact. The challenge's own
meta JWT decoded to `is_warp: true, is_gateway: true, auth_status: NONE`: WARP was up and its
**Access session had lapsed**, which is not something `warp-cli` can renew — the GUI is the only
way back, and the owner re-authenticated. **An enrolled machine with a lapsed Access session reads
exactly like an unenrolled one**, so "curl got a 302" is not by itself evidence about the
deployment, the door, or WARP being off. While it was down the application's own answers were read
from INSIDE the container (`docker exec` to `127.0.0.1:8080`); when the tunnel readings arrived
they agreed with that pass field for field, including every `Cache-Control` and every media type,
which is worth knowing next time: the origin read is a faithful stand-in for everything except
whether the edge is routing.

```
GET /api/health          200  {"server_version":"0.5.0","semantics_rev":7,
                               "rules_hash":"d85445c479cb1ead","identity":"claimed",
                               "signal":"sse","archive":"dir","lobby_chat":true,
                               "chat_keep":500,"chat_text":500,"multiplayer":true}
GET /sw.js               200  text/javascript; charset=utf-8 · cache-control: no-store
                               · content-length: 27220 · cf-cache-status: BYPASS
                               · cache name off the wire: yokohama-release-19e8532f977bcba2
GET /manifest.webmanifest 200 application/manifest+json · content-length: 696
GET /assets/yokohama_wasm_bg-Cf0j0Dq2.wasm
                         200  application/wasm · public, max-age=31536000, immutable
                               · content-length: 1175223
GET /assets/fraunces-latin-BOz5zopO.woff2
                         200  font/woff2 · public, max-age=31536000, immutable
                               · content-length: 67388 · cf-cache-status: HIT
GET /fonts/fraunces/OFL.txt
                         200  text/plain; charset=utf-8 · content-length: 4391
GET /art/areas/Church.webp
                         200  image/webp · public, max-age=3600 · content-length: 51118
GET /help/strips.webp    200  image/webp · content-length: 133332
GET /enter               302  location: /#/lobby
GET /api/games           401
GET /api/me              200  {"mode":"claimed","user":null}
GET /                    200  serving index-BEMfPcF1.js and index-CaXbZR8U.css,
                               which exist only in this build
```

**The three rows that are this cycle's and could not have read this way a day ago**: `/sw.js`
answering `no-store` with the generation name the build printed, `manifest.webmanifest` answering
`application/manifest+json` (the installer checks it), and `/fonts/<family>/OFL.txt` answering
`text/plain` rather than `application/octet-stream`. **The last one is why the server and `dist`
ship together and always will**: the service worker's install verifies content types, the `.txt`
arm lives in the binary and the OFL files live in `dist`, so a split deploy fails the install on
exactly those two files. They are one image, which is what makes the coupling safe rather than
merely true.

**The other two hostnames, same pass:** `yokohama-cli.moore.nyc` reads **200** (the second ingress
block survived the rewrite), and `yokohama.moore.nyc/api/health` reads **404** from the catch-all —
still HELD FOR THE WORKER, unchanged by this deploy.

#### A finding about §2's recipe, pre-existing and cosmetic

**The image contains AppleDouble sidecars**, because macOS `tar czf` writes a `._name` entry for
every file carrying an extended attribute and §2's recipe is a plain `tar czf` on a Mac. The
running container holds **141** of them under `/srv/assets/art` and **28** under `/srv/web/dist`,
beside the **132** real art files and **22** real dist files. **Nothing is broken and nothing is
served wrongly**: the precache list is generated from the workstation's `web/dist`, where these
files do not exist on disk at all (hence `153 entries` and `131 art files`), and the server serves
by name, so no client ever asks for one. The cost is a few kilobytes of image. This has been true
of **every deployment cut from this workstation since §3.5** and is recorded now only because it
was looked at; the fix, whenever §2 is next edited, is `COPYFILE_DISABLE=1 tar czf …`.

#### Rollback — and this one IS a schema rollback, on the BROWSER side

`yokohama-web:0.5.0-m6.5b2-rc1` and `yokohama-web:0.5.0-m6.5b.1` both stay on the NAS; recreate
from either with §3's body. **The server-side store schema did NOT move this cycle** —
`TABLE_SCHEMA` stays 3, the record format is fenced — so on the volume this is an ordinary
rollback and every table reads under the older binary.

**But rolling back across this release is a BROWSER-SIDE SCHEMA ROLLBACK, and nothing on the
server will tell you so.** `web/src/local/db.ts` moves the IndexedDB database `yokohama-local` to
**version 2** in this release, for the outbox store. A browser that has loaded `0.5.0-m6.5b2` has
a version-2 database; served `0.5.0-m6.5b.1` or earlier afterwards, that client opens it at
version 1, IndexedDB answers **`VersionError`**, the Worker's module-scope startup promise rejects
and is cached for the page's life, and **every local game in that browser reads "the games on this
device could not be read" until this release is served again**. Nothing is deleted — the rows are
all still there and the next newer client reads them — but the games are unreachable meanwhile and
a player cannot fix it from their end. **It cannot be rolled back at all, only rolled forward**,
because it is the browser's store and no image or backup here touches it, and its blast radius is
every browser that has loaded this release rather than one table. **§5 carries this as its own
bullet; read it there before rolling anything back**, and read §3.14's "NOT a schema rollback"
as the true statement it is — that was `553e3ee`, the mid-cycle build, which carried no version
bump at all. The bump arrived with the backup and outbox work after it.

A rollback also loses the service worker, and that is its own kind of sticky: a browser that
installed the `yokohama-release-19e8532f977bcba2` generation keeps serving this release's shell
from Cache Storage until an older deployment's own `sw.js` activates over it. **The hostname is a
separate rollback** and is unchanged by this deploy (§3.14 step 4). **Nineteen tags now sit on the
NAS.**

#### What this deployment does NOT claim

- **Nothing about the public paths, and nothing about the path-scoped Access design.** Under the
  hostname-wide application every path is behind the door, so spec §3.2's "public paths answer 200
  with no session" readings **stay DEFERRED BY OWNER DECISION** (2026-09-21) exactly as §3.14
  recorded them. The zero-login single-player claim is not shown live this cycle.
- **No unenrolled vantage was used here at all.** §3.14's 302/401 loops from a throwaway container
  on the NAS were not re-run, because nothing about the door changed: no DNS record, no Access
  application, no ingress block moved. The claim that the door is shut rests on §3.14's readings
  and on the ingress being byte-identical across the re-sync, which the guard above proved.
- **The headed walk of the deployed site was not performed**, and neither was §3.14's clean-browser
  single-audience reading. Both remain **OWNER-OWED from §3.14**; the blocker is the same one, the
  Claude-in-Chrome extension reporting no connected browser (`list_connected_browsers` returned an
  empty list). M6.4's §4.5 is why no headless substitute is offered in its place — four headless
  reviews called a page clean that a headed one found asking for a favicon.
- **Nothing about a phone.** The iPhone exit (install to the home screen, airplane mode, play,
  close the browser entirely, reopen, resume, finish, export; then an update and a restore on a
  fresh profile) is the owner's by R-6.5b2.17, and the Android reading is the owner's own handset
  by his ruling of 2026-09-22. **Both are recorded as OPENING ITEMS for M6.5c**, not as gates of
  this cycle — so "the site installs and runs offline" is a claim this release's tests make and
  this deployment does not.
- **Nothing was played on the deployed build.** No smoke table was created, no local game was
  opened, no archive round trip was exercised through this container. The readings above are
  statuses, media types, cache headers and one health body.

---

### 3.16 M6.5b2.1 is live — the thirteenth deployment, 2026-09-22 (a record, not a plan)

A POINT RELEASE carrying the PR #37 fix wave (`docs/plans/2026-09-22-web-frontend-m6.5b2-pr37-fixes.md`,
its §5 the record). It is **client-side only**: no Rust, no stored-document schema, no wire shape
and no IndexedDB version moved. `git diff --stat 43d54a5..c612797 -- crates Cargo.toml Cargo.lock
web/scripts` is empty. Three defects the live `0.5.0-m6.5b2` carried are fixed:

- **F1.** A backup, Save game file, Send to the archive, Abandon and save, or the outbox, run in a
  tab that does not own the game, could export a stale copy. The Worker now answers from its memory
  only when it owns the game at the current generation, and otherwise from IndexedDB, taking one
  whole pair.
- **F2.** A tab left open across an update could not fetch its own release's files. The service
  worker now answers a path its own release does not list from the RETAINED previous generation
  before the network (R-F2). The sweep keeps the previously ACTIVATED generation (R-F5). And
  `/cdn-cgi/*` is never intercepted (R-F6), so Access's login hop is not answered with the cached
  shell.
- **F3.** A successful "Try again" after a failed install was never adopted. It now is.

**How a player gets it.** A browser running `0.5.0-m6.5b2`'s worker checks `/sw.js` on its next
navigation. `/sw.js` is served `no-store`, and the bytes are new, so the browser installs the
`yokohama-release-d62d6763649a749f` generation beside the live one. The door then offers "Reload
to use it", and only a click takes it. Until then, or until every tab on the old release has
closed, that browser keeps running `0.5.0-m6.5b2` with all three defects. **The first activation
over the live generation KEEPS it** (fix-wave §5.5). The `0.5.0-m6.5b2` generation was stamped
before R-F5's `activatedAt` existed, so it carries an install time and no activation time. A
generation with no activation time is still the one the sweep retains when it is the only other
(`byRetention` in `swInstall.ts`). The generation this release replaces is **`19e8532f977bcba2`**,
read off the wire at 23:46:34Z, before the swap.

- **Build:** `web-room-m6.5b2` at `c612797` (the fix wave's code head `13dc65f` plus two docs
  commits), tree clean. The musl cross-compile per §1 read **`Finished release in 0.59s`**: cargo
  found nothing to rebuild. The binary is the file §3.15 built, mtime 19:06:33Z,
  **6,591,376 bytes**, equal to §3.15's figure, sha256
  `0b30e90833a4e9caf914454f832616c9aacdee9797a1d1825a7633a2606658ab`. **Docker says the same
  thing independently**: the NAS's build reused the binary layer `sha256:f4102fba8e24…` from
  §3.15's image, because the `COPY` checksum `file:e03176a1312608df…` matched. **So there is no
  size difference to explain, and the server is byte-identical to the one it replaces.**
- **The client build was `npm run build`**, the RELEASE module. §1's preflight `grep -c debug_trap
  web/src/local/wasm/yokohama_wasm.js` read **0**. Vite printed its `"debug_trap" is not exported`
  warning, which is the same fact seen from the other side. `wasm-opt version 132` read
  **1,783,361 → 1,175,223 B (−34.1%)** over both modules. **The module is byte-identical to
  §3.15's**: `yokohama_wasm_bg-Cf0j0Dq2.wasm` has the same name and the same sha256
  `277aa0c4…`, and `gzip -9` reads 456,054 B. What moved, against §3.15:

  | File | §3.15 | Now |
  |---|---|---|
  | bundle | `index-BEMfPcF1.js` 358,444 B | `index-DEXKLtYz.js` **359,218 B** (+774) |
  | local Worker | `worker-CI3Vv9s2.js` 14,809 B | `worker-uoUYQefZ.js` **15,667 B** (+858) |
  | `sw.js` | 27,220 B | **29,045 B** (+1,825) |

  Unchanged: `index-CaXbZR8U.css` 37,738 B, both fonts, both OFL texts and
  `manifest.webmanifest` 696 B. `index.html` is the same 2,387 B, but its bytes now name the new
  bundle.
- **The precache generator's four lines, verbatim:**

  ```
  precache: 153 entries, 9,936,972 B (release total ceiling 11,900,000 B)
  precache: module 1,175,223 B at /assets/yokohama_wasm_bg-Cf0j0Dq2.wasm (release module ceiling 1,400,000 B; release 1,400,000, debug 3,200,000)
  precache: module 1,175,223 · bundle 412,623 · fonts 98,728 · shell 11,873 · icons 19,456 · help 764,742 · art 7,454,327
  precache: yokohama-release-d62d6763649a749f -> dist/sw.js (29,045 B), 131 art files
  ```

  The +1,632 B over §3.15's 9,935,340 is the bundle's +774 plus the Worker chunk's +858, to the
  byte. It reproduces the fix wave's own release reading (§5.2: 153 entries, 9,936,972 B). The
  RELEASE ceilings applied.
- **Delivery:** the context tarball was cut with **`COPYFILE_DISABLE=1`** (§3.15's AppleDouble
  finding; §2's recipe line now carries it): **11,260,524 B**, sha256
  `f045b61ce25a590e344a77cd3330ff2fe44c319199f7ee00bd3dbf0df5a29a89`, 174 entries,
  **0 `._` entries**, cut at **23:46:24Z**. It was served by `python3 -m http.server 8899 --bind
  0.0.0.0` on the workstation at 192.168.1.16 from 23:46:41Z. The server's log reads two GETs: the
  workstation's own reachability probe at 23:46:41Z, and the NAS (192.168.1.173) at **23:46:44Z**.
  The file server was stopped before the swap. A probe at **23:47:08Z** read connection refused,
  with no listener left on 8899.
- **Image:** `yokohama-web:0.5.0-m6.5b2.1`, Id
  `sha256:fba5779df002f207ed3c436a8b9ecd07e5edda1bf825a31f2cd4ce17c8c4f987`, **24,442,446 B**,
  created **23:46:54.30Z** through
  `POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m6.5b2.1&dockerfile=Dockerfile&rm=true`
  (Portainer environment 2). It is **24,090 B smaller than §3.15's 24,466,536**, and the layers say
  where:
  - The binary layer is shared.
  - The art layer is **7,454,327 B**, down from 7,477,310. That is exactly the 132 files on disk
    and exactly the precache's `art` figure, so the 22,983 B were the sidecars.
  - The dist layer is **2,511,690 B**, down from 2,512,797. That is exactly the 22 files of
    `web/dist`: the larger bundle, Worker chunk and `sw.js`, net of the sidecars.

  Read from inside the container: **zero `._*` entries under `/srv`**.
- **Swap:** `POST /containers/yokohama-web/stop` on the `0.5.0-m6.5b2` container (`58ac45ba478a…`,
  created 19:08:24.59Z) was issued after 23:47:11.06Z. The old process exited at
  **23:47:11.53Z**, exit code 0: tini forwarded SIGTERM and the graceful shutdown ran. Then
  `DELETE /containers/yokohama-web`, then `POST /containers/create?name=yokohama-web` with §3's
  body:
  - **both `Env` entries** (`YOKOHAMA_WEB_IDENTITY=claimed`, `YOKOHAMA_WEB_ARCHIVE_DIR=/data/archive`);
  - **`YOKOHAMA_WEB_MULTIPLAYER` deliberately UNSET**, so it defaults to `on`;
  - the two labels (`maintainer`, `purpose`), `NetworkMode: cf-internal`,
    `RestartPolicy: unless-stopped`, and the named volume `yokohama-web-data` → `/data`.

  That made container Id `d2d66681d449bb7dd17e013d571b9bca6373917cac71ee6413645f356efea571`,
  created **23:47:17.10Z**, started **23:47:19.89Z**. Its log reads `yokohama-web listening on
  http://0.0.0.0:8080` at **23:47:19.98Z**, and it was `healthy` on the probe. It has the same
  address on `cf-internal` (**172.21.0.4**), and cloudflared, DNS and Access were not touched.
  **Downtime is a READING this time, not only a bound**: from the old process's exit to the new
  one's listening line, both read off Docker, **8.45 s**. Whether anyone was mid-game was not
  checked.

#### Verified through the tunnel

Taken from the WARP-enrolled workstation at **23:47:37Z** with a browser User-Agent, so each row is
the APPLICATION's answer to an authenticated caller (§3.14's vantage). The Access session was
live: the pre-swap read at 23:46:34Z answered 200, and no row below read Access's 302 (the one 302
is `/enter`'s own, to `/#/lobby`), so the in-container fallback §3.15 needed was not needed here.

```
GET /api/health          200  {"server_version":"0.5.0","semantics_rev":7,
                               "rules_hash":"d85445c479cb1ead","identity":"claimed",
                               "signal":"sse","archive":"dir","lobby_chat":true,
                               "chat_keep":500,"chat_text":500,"multiplayer":true}
GET /sw.js               200  text/javascript; charset=utf-8 · cache-control: no-store
                               · content-length: 29045 · cf-cache-status: BYPASS
                               · cache name off the wire: yokohama-release-d62d6763649a749f
GET /manifest.webmanifest 200 application/manifest+json · content-length: 696
GET /assets/yokohama_wasm_bg-Cf0j0Dq2.wasm
                         200  application/wasm · public, max-age=31536000, immutable
                               · content-length: 1175223 · sha256 277aa0c4…
GET /assets/fraunces-latin-BOz5zopO.woff2
                         200  font/woff2 · public, max-age=31536000, immutable
                               · content-length: 67388 · cf-cache-status: HIT
GET /fonts/fraunces/OFL.txt
                         200  text/plain; charset=utf-8 · content-length: 4391
GET /art/areas/Church.webp
                         200  image/webp · public, max-age=3600 · content-length: 51118
GET /help/strips.webp    200  image/webp · content-length: 133332
GET /enter               302  location: /#/lobby
GET /api/games           401
GET /api/me              200  {"mode":"claimed","user":null}
GET /                    200  serving index-DEXKLtYz.js and index-CaXbZR8U.css
GET /assets/index-DEXKLtYz.js
                         200  public, max-age=31536000, immutable · content-length: 359218
GET /assets/worker-uoUYQefZ.js
                         200  public, max-age=31536000, immutable · content-length: 15667
```

**The two readings this release owes:**

- **The served `/sw.js` carries the fix.** This was checked on the body fetched through the tunnel,
  not on the file on disk. It is 29,045 B, sha256 `67a8fefd53cb17a8…`, equal to `web/dist/sw.js`.
  It carries `pathname === "/cdn-cgi" || pathname.startsWith("/cdn-cgi/")` in its server-path
  rule (R-F6). It carries the retained-generation lookup: `fromRetained` and
  `retainedGenerations` twice each. And it carries the activation-ordered sweep: `byRetention`
  twice and `activatedAt` fifteen times. It names ONE generation, `d62d6763649a749f`.
- **The served bundle is this build's.** `/` names `index-DEXKLtYz.js`, which exists only in this
  build. Its served bytes, the Worker chunk's and the stylesheet's all hash equal to `web/dist`'s.
  **The OLD bundle `index-BEMfPcF1.js` still reads 200 through the tunnel, and it is the EDGE
  answering, not this container.** It came back `cf-cache-status: HIT`, `age: 12411`, while the
  origin answers it **404** (read inside the container). The immutable hashed asset sits in
  Cloudflare's cache and will age out, as §3.14 recorded for the old hostname. That matters to
  F2 only as a delay: an unreloaded tab's old asset is served from the edge while it lasts, and
  from the retained generation after.

**The other two hostnames, same pass:** `yokohama-cli.moore.nyc` reads **200**, and
`yokohama.moore.nyc/api/health` reads **404** from the catch-all. It is still HELD FOR THE WORKER.

#### A finding: the edge REWRITES `index.html`, and the install survives it anyway, for a reason not yet identified

Found while checking that `/` names this build's bundle. It is recorded in full because it bears
on whether "installs and runs offline" can be true through this edge at all.

- **Through the tunnel, every `curl` GET of `/` or `/index.html` comes back REWRITTEN.** An extra
  `<script>` before `</body>` injects Cloudflare's JavaScript-detections snippet. It loads
  `/cdn-cgi/challenge-platform/scripts/jsd/main.js`. The body is **3,325 B** instead of 2,387 B,
  with a DIFFERENT sha256 on every request, because the snippet carries a ray id and a timestamp.
  This held across eight request shapes: curl's defaults, with and without a browser
  User-Agent; `Accept: */*` with `Sec-Fetch-Mode: cors`, `Sec-Fetch-Dest: empty`,
  `Sec-Fetch-Site: same-origin` and `no-cache` (close to what a worker's `cache: "reload"` fetch
  sends); the same with a `Referer`; the same with `sec-ch-ua` hints; a navigation's `text/html`,
  `navigate` and `document`; HTTP/1.1; and a compressed transfer.
  **The origin does not do this.** Read inside the container, `/` and `/index.html` are 2,387 B,
  sha256 `a65c90f753dfd072…`, exactly the build's.
- **Why it could matter.** The worker precaches `/index.html` and checks every body's SHA-256
  against the build's digest (`installOne` in `sw.ts`). A rewritten body would fail the install,
  and it would fail LOUDLY, naming `/index.html: its body …` on the door.
- **What was measured instead: the install SUCCEEDED.** The browser was Chromium 153, driven
  through the Playwright MCP, from this WARP-enrolled workstation, through the tunnel, in a profile
  that held no earlier generation for this origin. At 23:48:50Z:
  - The door read **"Ready to play offline."**
  - The registration was `activated`, with ONE cache, `yokohama-release-d62d6763649a749f`,
    holding **154 entries** (153 plus the stamp).
  - The cached `/index.html` was **2,387 B, sha256 `a65c90f7…`, with no snippet in it**.
  - Meanwhile the page's OWN first navigation DID carry the snippet. That page made four
    `/cdn-cgi` requests: two GETs for the challenge-platform script (a 302 and its target), a
    `jsd/oneshot` POST, and a `/cdn-cgi/rum` POST.
  - It made **no `/api/` request**.
  - `beforeinstallprompt` fired, so Chromium judged the deployed site installable. The console
    held no error and no warning. Its one line is Chromium noting the app's own
    `preventDefault()` on that event.

  **A second navigation at 23:49:51Z was CONTROLLED by the worker** (`controller.scriptURL` =
  `/sw.js`). It was answered with the cached shell, with no snippet on the page, and it made zero
  `/cdn-cgi` and zero `/api/` requests.
- **What makes the edge leave the worker's fetch alone is NOT identified.** None of curl's header
  shapes reproduced it. A cookie set by the snippet on the first navigation, or a client
  fingerprint, are guesses and are recorded as guesses. **So the offline install through this edge
  rests on a mechanism nobody has named, and it has been read in ONE browser at ONE vantage.** This
  is not the headed walk, and not a phone.
- **When this began is not known.** §3.15 did not read `index.html`'s body, so this record does not
  say whether the snippet was there at the twelfth deployment. It is a zone-level Cloudflare
  setting, and no zone setting was read or changed here.
- **Carried to the owner and to M6.5c.** It bears on the iPhone exit, whose install is the reading
  that would expose it on Safari. It also bears on the Worker hostname, since the same zone serves
  it. The remedies are the owner's, and none was taken: turn off JavaScript detections for these
  hostnames, or accept the edge's behaviour once a phone has read it.

#### Rollback

`yokohama-web:0.5.0-m6.5b2` stays on the NAS. Recreate from it with §3's body; `0.5.0-m6.5b2-rc1`
and `0.5.0-m6.5b.1` are there too, and §3.15's rollback paragraph governs going back past
`0.5.0-m6.5b2`.

**Rolling back to `0.5.0-m6.5b2` is NOT a schema rollback on either side.**

- The server is byte-identical: the same binary layer, `TABLE_SCHEMA` 3, the same record format.
  So on the volume it changes nothing at all.
- The browser store did not move. `grep -n "VERSION" web/src/local/db.ts` reads
  `118:const VERSION = 2;`, which is §3.15's version 2, and the only change to `db.ts` since
  `43d54a5` is `readGeneration`'s doc comment. So a rolled-back client opens the same version-2
  database, and there is no `VersionError`.

**What a rollback does to a browser that installed `d62d6763649a749f`.**

- On its next navigation that browser's update check finds `0.5.0-m6.5b2`'s `sw.js`. It installs
  `19e8532f977bcba2` again. The generation is retained, so it re-fills an existing cache, and the
  pre-fix install re-stamps it with a fresh `createdAt`.
- The door offers the reload. Until someone clicks, the browser keeps serving THIS release's shell
  and client from Cache Storage. That is safe against the older server, because the server is the
  same binary.
- After the click the pre-fix worker activates. Its sweep, which ranks by install time, keeps
  `d62d…` as the other generation, so nothing is deleted. **All three defects come back for that
  browser**, including the `/cdn-cgi` login hop answered with the shell.

**Twenty tags now sit on the NAS.** They run from `yokohama-web:0.5.0` through
`yokohama-web:0.5.0-m6.5b2.1`: 0.5.0, m5, m5.1, m5.5, m5.5.1–4, m6.1, m6.2, m6.2.1, m6.3, m6.4,
m6.4.1, m6.5a, m6.5b, m6.5b.1, m6.5b2-rc1, m6.5b2, m6.5b2.1.

#### What this deployment does NOT claim

- **No headed walk.** The Playwright MCP's Chromium above is a mechanism reading, meaning the
  install succeeded and the second load was controlled. It is not the owner's walk, which stays
  OWNER-OWED from §3.14. M6.4's §4.5 is why the one does not stand in for the other.
- **No phone.** The iPhone exit and the Android reading remain M6.5c opening items. The edge
  finding above makes the iPhone install a sharper reading than it was.
- **No unenrolled vantage.** No door, DNS record, Access application or ingress block changed, so
  the claim that the door is shut still rests on §3.14's readings.
- **Nothing was played.** No local game was opened, no table was created, no backup or restore was
  run, and no archive round trip went through this container.
- **The update path was not exercised live.** The Chromium profile was fresh, so no browser was
  read taking `19e8532f… → d62d…` through the door's offer, keeping the old generation, or letting
  an unreloaded tab fetch its retained files. Those are the fix wave's e2e cells (§5.2), not
  readings of this site.
- **The `/cdn-cgi` login hop (R-F6) was not exercised live.** No session expired in a controlled
  browser. The `/cdn-cgi` requests above are the edge's own subresources on an uncontrolled first
  load; they are not the Access navigation R-F6 exists for. It stays carried as the fix wave's
  §5.7 says.

---

## 4. Modal and its edge proxy — RETIRED at M6.5c (a record, not a plan)

**This section described an upgrade path. It is now the record of why that path was deleted**, by
umbrella R-6.5.13 (`docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md`), at M6.5c's Task 11
(2026-09-23).

**What the two files were.** `deploy/modal_app.py` ran the NAS's image on Modal: a Volume for the
store, `max_containers=1` (a Volume file wants a single writer, and this store is one JSON file per
game), and a commit loop, because on Modal a drained request is not yet a durable one.
`deploy/worker/` was the Cloudflare Worker in front of it. Access authenticated at that Worker, which
forwarded to the `*.modal.run` endpoint holding a Modal proxy token — without the token the endpoint
was world-reachable and Access would have guarded nothing — and the server ran
`YOKOHAMA_WEB_IDENTITY=worker`, trusting the `X-Yokohama-User` the Worker set. **Neither was ever
deployed**, so nothing had to be torn down.

**Why they went — two reasons.**

- **The path is superseded.** The hosted design is the Durable Objects Worker in `worker/` (§6), and
  two hosted designs on disk is one too many.
- **Its strip rule had a defect.** `deploy/worker/src/index.ts:71`'s `CLIENT_MUST_NOT_SET` removed a
  client's `x-yokohama-user`, and did NOT remove `x-yokohama-claimed-user`. Under the `claimed` door
  that second header IS the caller, so a forwarded one would have impersonated any seat at any table
  (umbrella R-6.5.11). The rule carried out of the deleted file is WIDENED: `worker/`'s `handle`
  removes EVERY client header whose name starts `x-yokohama-` before it forwards anything, and its
  battery-2 cell (`worker/test/worker.test.ts`) sends all four names the objects trust —
  `x-yokohama-user`, `x-yokohama-claimed-user`, `x-yokohama-operator`, `x-yokohama-table-id` — beside
  a real person's assertion, and reads what each object was handed.

The files' last versions are in history: `git show 87d30a14:deploy/worker/src/index.ts` and
`git show 87d30a14:deploy/modal_app.py`. §0–§3 and §5 stay, because the NAS stays (spec §3.4).

## 5. Rollback

- **The tunnel:** restore a backup beside it on the NAS and restart
  `cloudflared-nas`. One file, one restart. **`config.yml.bak-2026-09-21` is the
  one to reach for** — it is the file that was running until §3.14's move, so it
  puts `yokohama.moore.nyc` back. **Both backups carry PRE-M6.1 comment text**,
  not just the older one: the 2026-09-21 copy is the NAS's own untouched
  2026-09-02 file, which still says this origin has "NO authentication of any
  kind" (§3.14's drift note). So either restore re-routes correctly and
  re-introduces a comment that has been false since M6.1; the accurate text is
  `deploy/cloudflared-config.yml` in this repo. `config.yml.bak-2026-09-01` is
  older still (its size was never read; the figure that stood here was not one
  any record carries).
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
- **Across M6.5a it is a SCHEMA rollback as well.** A table created under
  `0.5.0-m6.5a` or later is a schema-3 document, and every binary before it refuses
  one (§3.12): the lobby drops the row and its reads fail until the newer binary is
  back. Tables from before that release keep their schema and are unaffected.
- **Across `0.5.0-m6.5b2` the rollback is a BROWSER-SIDE schema rollback, and
  nothing on the server says so.** `web/src/local/db.ts` moves the IndexedDB
  database `yokohama-local` to **version 2** in that release (the outbox store).
  A rolled-back `0.5.0-m6.5b.1` client opens it at version 1 against a version-2
  database, IndexedDB answers `VersionError`, the M6.5b Worker's module-scope
  startup promise rejects and is cached for the page's life, and **every game in
  that browser reads "the games on this device could not be read" until the
  newer client is redeployed**. Nothing is deleted and nothing is lost: the rows
  are all still there and the next newer client reads them. But the games are
  unreachable meanwhile, and a player cannot fix it from their end.
  - It is the browser's store and not this volume, so no image and no backup
    here changes it, and it cannot be rolled back at all — only rolled forward.
  - Its blast radius is every browser that has loaded the newer client, not one
    table. A player who never opened the release is unaffected.
  - §3.14's "a rollback across it is NOT a schema rollback" is TRUE as written:
    it is about the SERVER's stored documents (`TABLE_SCHEMA` stays 3), and it
    was recorded for `553e3ee`, the mid-cycle release, which carried no version
    bump at all. The bump arrived with the backup and outbox work after it. **§3.15
    is the deployment record that shipped the bump**, and says this in its own
    rollback paragraph.
  - Told here because §3.12's `TABLE_SCHEMA` sentence is the shape a reader
    looks for, and a rollback that takes every local game off the screen has to
    be as easy to find as one that drops a lobby row.
- **The store is not rolled back by either.** It is the named volume
  `yokohama-web-data`; the records are the whole state, which is why they
  replay.

---

## 6. The Worker — hosted multiplayer on Cloudflare (M6.5c)

**`yokohama-web`, a Cloudflare Worker with three kinds of Durable Object, at
`yokohama.moore.nyc`**, behind the same `yokohama` Access application as the NAS. The authority is
`docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` (§2 the mechanism, §3 the deployment);
the code is `worker/` — a TypeScript shell — over `crates/yokohama-worker`, the table layer compiled
to WebAssembly. The NAS (§0–§3) is unchanged and keeps `yokohama-nas.moore.nyc`.

What answers what:

- **The static release** — `web/dist` and the card art — is served by Cloudflare's asset layer and
  invokes the Worker ZERO times. The Worker sees only `/api/*`, `/enter`, and a path that has no file,
  which it answers 404 with an empty body (spec §2.3).
- **The Worker** is stateless. It verifies the Access assertion, removes every client header named
  `x-yokohama-…`, and forwards to one of three SQLite-backed classes: `Table`, one per table;
  `House`, ONE, for the lobby and the house chat; and `Archive`, one per caller, for a browser's saves.
- **A few answers are this host's own words** (spec §4's host-written class, and this cycle's
  additions). Everything else a client reads is the layer's.
  - a path with no file: **404** with an empty body — never `index.html`;
  - any method but GET or HEAD on `/enter` or `/api/art`: **405**, empty; `/enter`: **302** to
    `/#/lobby`;
  - a body over 2 MiB: **400** `the body could not be read: length limit exceeded` — the NAS's own
    sentence, read off it on 2026-09-23;
  - a bad variable: **500** naming it (§6.3);
  - **a WebSocket upgrade from another site's page — its `Origin` present and not
    `https://yokohama.moore.nyc`: 403 `this socket is for the site's own pages`**, unupgraded. A
    WebSocket is outside CORS, so without it a hostile page in a signed-in player's browser could
    read a table's turn and timing frames (Task 11's review, escalation 1). A request with no
    `Origin` — curl, the deploy readings — passes;
  - any method but GET on `…/events`, `…/stream` or `/api/stream`: **405**, empty; a GET of
    `…/stream` or `/api/stream` that is not an upgrade: **426** `this host streams over
    WebSocket`;
  - the create's three: **400** `a game must be created from a JSON object`, **400** `a game id,
    when given, must be a string`, **500** `the create's id does not name the table it was routed
    to`;
  - a create naming a table whose REMOVAL the lobby has not yet acknowledged: **409** `this name
    was used recently — try another` (spec §2.4; Task 12), so a new table's first copy can never
    overwrite a removal the house has not received;
  - a trapped engine: **500** `the table's engine stopped — try again`, `the lobby's engine stopped
    — try again` or `the archive's engine stopped — try again`; a table whose stored document traps
    twice: **500** `this table's stored <kind> cannot be loaded`;
  - a deleted table's sockets: `end {"reason":"deleted"}`, then a **4404** close whose reason is
    `this table was deleted`.

### 6.1 Build

Prerequisites: the pinned toolchain with its `wasm32-unknown-unknown` target, `wasm-bindgen-cli
0.2.128` and binaryen's `wasm-opt` (the same three as §1's client build), and `npm ci` once in both
`web/` and `worker/`.

```sh
cd web && npm run build          # the RELEASE browser module, and web/dist
cd ../worker && npm run build    # build:module → build:assets → smoke:release
```

`worker/`'s `npm run build` is three steps, each refusing by name:

1. **`build:module`** — `crates/yokohama-worker` on `--profile release-wasm`, wasm-bindgen, `wasm-opt`,
   and the factory transform that gives every object its own instance (spec §2.2). A build whose
   optimizer changed nothing leaves `gen/.unoptimized`, which the deploy guard refuses.
2. **`build:assets`** — `worker/assets/` from `web/dist` and `crates/yokohama-web/assets/art/`,
   verified entry by entry against the release's own precache list. A debug browser module is
   refused.
3. **`smoke:release`** — the release module instantiated once, in Node: `init` with the archive off,
   then `GET /api/health` as `smoke@x.test`. It refuses unless the answer is 200, and refuses a debug
   module. Nothing else runs the release module before a deploy.

**The order matters twice.** `web/`'s `npm run e2e` leaves a DEBUG browser module in `web/dist`, so
`web/`'s `npm run build` comes first, every time. And `worker/`'s own tests run the DEBUG module:
after a release build, `npm run build:module:debug` comes before `npm test` in `worker/`.

### 6.2 Deploy

```sh
cd worker && npx wrangler whoami   # the owner's login, account BDVC
npm run deploy                     # node scripts/deploy-guard.mjs && wrangler deploy
```

**The deploy guard** refuses, naming every failure: a missing, DEBUG or unoptimized module; an
assets tree that is not the release its `sw.js` lists, or that holds a debug browser module;
`workers_dev` or `preview_urls` not `false`; the test clock named anywhere in `wrangler.jsonc`; an
`assets` block that is not spec §2.3's; and the two configurations' `durable_objects`, `migrations`
or `r2_buckets` blocks differing, or either file lacking one. `npx wrangler deploy --dry-run`
bundles and validates without uploading.

- **A deploy restarts every object and closes every socket with 1006** (spec §5.5). The client
  probes once and reconnects, an open picker is re-fed, and no acknowledged move is lost — but it is
  still a reconnect for everyone at every table, so deploy between games.
- **The migration `v1` is FROZEN from the first deploy on** (the orchestrator's ruling I-30): a
  class added later takes a new tag. Cloudflare's own docs now prefer a declarative `exports` field
  for NEW Workers, and keep the `migrations` array this Worker uses fully supported; the two may not
  be mixed.

### 6.3 The variables — every one, and what its absence means

`wrangler deploy` REPLACES the Worker's variables with `wrangler.jsonc`'s `vars`, so that file is
the whole list; it leaves secrets alone.

| Variable | At the fourteenth deployment | Absent means |
|---|---|---|
| `ACCESS_TEAM_DOMAIN` | `bdvc.cloudflareaccess.com` | no assertion can be verified: every caller is no one, and no key is fetched |
| `ACCESS_AUD` | the `yokohama` application's audience | the same |
| `DEV_DOOR_CLIENT_ID` | the `yokohama-worker` credential's client id, its public half | the dev door is shut, whatever `YOKOHAMA_DEV_DOOR` says |
| `YOKOHAMA_DEV_DOOR` | `on`, for this cycle's live tests | off: an assertion naming only the credential is no one |
| `YOKOHAMA_WEB_ARCHIVE` | `off` | off (`r2` needs the bucket bound as `ARCHIVE`) |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | unset | no table ever expires |
| `YOKOHAMA_WEB_ADMINS` | unset; a SECRET when set (`wrangler secret put`, the plan's ruling P-1) | the operator routes are the layer's 404 |
| `YOKOHAMA_WEB_COUNT_WAKES` | unset | off: no wake is counted |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `on` | on |
| `YOKOHAMA_WEB_CHAT_KEEP` | `500` | 500 messages |
| `YOKOHAMA_WEB_CHAT_TEXT` | `500` | 500 characters |
| `YOKOHAMA_WEB_TEST_CLOCK` | **never set in production** — `wrangler.test.jsonc` only, and the deploy guard refuses `wrangler.jsonc` if it names it | the wall clock |

- **A bad value answers 500 naming its variable** (spec §2.6), the Worker's form of the NAS refusing
  to start: a switch other than `on` or `off`, an archive other than `r2` or `off`, `r2` with no
  bucket bound, a TTL that is not a whole number of days, an operator entry that is not an address,
  or any value written without quotes. The static release is unaffected, and so are a few `/api/*`
  answers, which is where the rule is not "every `/api/*`":
  - two answers come before any configuration is read: 405 for any method but GET or HEAD on
    `/api/art`, and 403 for an upgrade from another site's page (the list at the top of §6);
  - a bad value of any variable but the three chat variables is the Worker's own refusal: 500 on
    every other `/api/*` request;
  - a bad chat variable is read only by the objects' module (`init`). So the Worker's own
    `GET /api/art` still answers 200, and so do the objects' answers that come before a module is
    built: the 405 for any method but GET on `…/events`, `…/stream` or `/api/stream`, and the 426
    for a GET of either stream route that is not an upgrade. Every other `/api/*` is the object's
    500 naming the variable.
- **An address outside visible ASCII is no caller** (Task 2's review, note E1; Task 11's review,
  I1). The Worker judges the RAW `email` claim — and the raw claimed header — before it trims or
  lowercases anything, and then lowercases ASCII only, as the layer's `User::parse` does. So a
  person whose Access `email` holds any character but a tab, a space or visible ASCII — a no-break
  space, an ideographic space, or a Kelvin sign (U+212A), which JavaScript's `toLowerCase()` would
  turn into another user's `k` — meets the layer's 401, and the Worker forwards no identity header
  for them at all. The layer's own `resolve` reads such a header value as absent, as the NAS
  does.
- **To deploy with no archive** (R-6.5c.14): set `"YOKOHAMA_WEB_ARCHIVE": "off"` in `wrangler.jsonc`
  and deploy (§6.2). The `ARCHIVE` binding stays as it is — the deploy guard holds it equal in the
  two files — and nothing is removed from the bucket. With the switch off:
  - `/api/health` reads `archive: "off"`;
  - game over saves nothing and arms no alarm for a save;
  - a creator's Abandon deletes without a save, taking with it any save the table still owes, which
    it logs as `archive-owed-dropped`;
  - `POST /api/archive` answers the layer's 404 `this server keeps no archive`, which the client
    reads as the archive being off;
  - a game-over save still owed from when the archive was on is KEPT and not retried: no request
    uploads it, no alarm is re-armed for it, and an alarm that runs for other work logs
    `archive-owed-kept`. Set back to `r2`, the table's next request or alarm resumes it.
- **To keep every table** (R-6.5c.15): leave `YOKOHAMA_WEB_TABLE_TTL_DAYS` unset, as the fourteenth
  deployment does.

### 6.4 The credential

**`yokohama-worker`** is an Access Service Auth credential, admitted by the `yokohama` application's
policy "yokohama ST bypass" (`non_identity`, precedence 3). It was created 2026-09-23T01:01Z and
expires in 2031 (spec §0.5).

- **Where it lives.** The repository root's `.dev.vars`, gitignored since `952a68c`, holds two
  lines, `CF-Access-Client-Id=…` and `CF-Access-Client-Secret=…`, named as the headers Access
  expects. The client id is public, and `wrangler.jsonc` carries it as `DEV_DOOR_CLIENT_ID`. The
  secret is in no other file, and is never printed, committed or pasted.
- **What it does.** A request presenting both headers gets an assertion whose `common_name` is the
  client id and which names no `email`, and Service Auth outranks a WARP identity (spec §0.3 row 1).
  With `YOKOHAMA_DEV_DOOR` on, such a request names its caller in `X-Yokohama-Claimed-User`; without
  that header it is no one. Access also sets a `CF_Authorization` cookie, which carries the
  credential on later requests from the same browser.
- **The header-file recipe** — the secret never on argv, never on screen. From the repository root:

  ```sh
  H="$SCRATCH/access.headers"    # SCRATCH: a directory of your own, e.g. the session scratchpad
  node -e '
  const fs = require("fs");
  const v = Object.fromEntries(fs.readFileSync(".dev.vars", "utf8").split(/\r?\n/)
    .filter((l) => l.includes("=")).map((l) => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1).trim()]));
  const names = ["CF-Access-Client-Id", "CF-Access-Client-Secret"];
  if (!names.every((n) => v[n])) { console.error(".dev.vars lacks one of " + names.join(", ")); process.exit(1); }
  fs.writeFileSync(process.argv[1], names.map((n) => n + ": " + v[n]).join("\n") + "\n", { mode: 0o600 });
  ' "$H" && chmod 600 "$H"
  curl -s -H @"$H" https://yokohama.moore.nyc/api/health
  rm -f "$H"
  ```

- **In a browser.** `node scripts/read-deploy.mjs install|console <base-url> --headers "$H"` sends
  the two headers to the Worker's own origin ONLY. Sent to every origin — Playwright's context
  `extraHTTPHeaders` — they would reach third parties such as the Web Analytics beacon, and would
  add a CORS preflight that is itself a console error.
- **To rotate it.** Zero Trust → Access controls → Service credentials → Service Tokens →
  `yokohama-worker` → **Rotate secret**. The client id stays the same, so `DEV_DOOR_CLIENT_ID` does
  not change; Access issues a new secret, and keeps the old one valid for the grace period you choose
  (one hour to 30 days, or revoke at once). Write the new secret into `.dev.vars` before the grace
  period ends. **Refresh** extends the expiry by a year, and a token can be turned OFF without being
  deleted. A NEW token has a new client id: update `DEV_DOOR_CLIENT_ID`, deploy, and add the token to
  "yokohama ST bypass". Secrets issued since 2026-08-26 have the shape `cfast_` and 48 characters,
  which secret scanners recognize.

### 6.5 Reading it

- **Health**, over WARP as the owner: `curl -s https://yokohama.moore.nyc/api/health`. It reads
  `identity: "cf-access"`, `signal: "ws"`, and the archive switch.
- **The tail**: `npx wrangler tail yokohama-web --format json > "$SCRATCH/tail.json"`, left running
  beside the readings. Every event is one invocation, pretty-printed. The Worker's own invocations
  have `executionModel: "stateless"`; an object's have `"durableObject"` and its class as
  `entrypoint`. The request's headers arrive `REDACTED`, the assertion among them. Count and cost
  them with:

  ```sh
  jq -c '{model: .executionModel, entry: .entrypoint, url: .event.request.url, cpu: .cpuTime, wall: .wallTime, outcome}' \
    "$SCRATCH/tail.json" | sort | uniq -c
  ```

- **The shell's own log lines** — `trap`, `load-trap`, `poison`, `identity-failed`, `init-refused`,
  `frame-failed` and the rest (spec §2.8) — are the events' `logs`, one JSON object per line, and
  Workers Logs (observability is on) keeps them. The layer's own `eprintln!` lines are mute on this
  host. The house and the slot (spec §2.5; Task 12) add these:
  - **`slot-pushed`** `{id, generation, revision, removal, attempt}`: a table pushes its slot to the
    house — its scrubbed copy, or a REMOVAL when `removal` is true — logged before the RPC.
    `attempt` counts this slot's pushes that failed before it.
  - **`slot-acked`** `{id, generation, revision}`: the house acknowledged that push, whether it
    applied it or ignored it. On the happy path every `slot-pushed` has one.
  - **`slot-failed`** `{id, generation, revision, attempt, retry_in?, error}`: the push's RPC
    failed. With `retry_in` (milliseconds) the slot still holds it: the failure is counted and the
    alarm re-armed on the backoff — 30 s, 2 min, 10 min, then hourly. Without it, a newer slot has
    replaced it, with a push and a retry of its own, or the object has ended (a removal acknowledged
    meanwhile, then `deleteAll()`).
  - **`slot-ignored`** `{id, generation, revision, current}`, logged by the HOUSE: a push it did not
    apply, and acknowledged all the same. The reasons: a revision no higher than the current one's;
    a retired generation; another generation while one is current; or, with no current row
    (`current: null`), a generation that does not order after every retired one — how a re-create
    in the same second reads, until Task 14's marker.
  - **`copy-refused`** `{id, error}`: `house_copy` refused the table's document, and `error` is the
    module's sentence, verbatim. No slot is written, so the house keeps listing the last copy it
    received. Only a corrupted store reaches it; the operator's force (Task 14) is its path.
  - **`trap`** with `call: "house_copy"` `{id, call, error}`: computing the copy trapped, or its
    module could not be built (`error` keeps the error's class). The request's documents commit and
    its reply stands. The copy is owed: `meta.copy_owed` counts the traps in a row, the alarm
    retries on the backoff, and every request computes the copy until it is made.
  - **`house-chat-dropped`** `{id: "house", traps, error}`: the house chat failed to load twice in
    a row, so its row was deleted and the lobby serves without it. A first failure logs `load-trap`
    with `kind: "house"`.
  - **`alarm-failed`** `{id, error}`: a `setAlarm` or `deleteAlarm` issued beside a batch was
    rejected.
- **The Origin check**, over WARP: an upgrade from the site's own origin answers 101, one from
  another answers 403 with the host's sentence (reading 2d).

  ```sh
  WS=(-H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==')
  curl --http1.1 -s -i -N --max-time 5 "${WS[@]}" -H 'Origin: https://yokohama.moore.nyc' https://yokohama.moore.nyc/api/stream | head -1
  curl --http1.1 -s -i --max-time 5 "${WS[@]}" -H 'Origin: https://evil.example' https://yokohama.moore.nyc/api/stream | sed -n '1p;$p'
  ```

- **`node scripts/read-deploy.mjs`** reads a live deployment: `assets` checks every precache entry
  of the DEPLOYED `/sw.js` (200, the type's essence, the SHA-256), `/index.html` answering 200, and a
  missing file answering 404 with an empty body; `install` loads the site in headless Chromium and
  reads the service worker's install; `console` records one fresh load and one controlled load, with
  every console message and every failed or cross-origin request.

### 6.6 Rollback

- **At the fourteenth deployment there is no earlier version.** The fallback is the NAS, which is
  untouched. To take the Worker off its hostname, remove the custom domain in the dashboard
  (Workers & Pages → `yokohama-web` → Domains): the objects and their storage stay. **Never
  `wrangler delete`**, which deletes the Worker "and all associated Cloudflare developer platform
  resources" (the command's own reference): the objects ARE the store.
- **Later**, `npx wrangler rollback <version-id>` (the ids from `npx wrangler deployments list`)
  goes back to an earlier version, but never across a Durable Object migration: a version from
  before a class was added cannot be restored (Cloudflare, *Rollbacks — Bindings*).

### 6.7 Still to come in this cycle

Each lands with the task that builds it, and records its own readings here:

- the bucket `yokohama-archive` bound as `ARCHIVE`, the switch at `r2`, and the conditional put read
  live (Task 13);
- expiry at `YOKOHAMA_WEB_TABLE_TTL_DAYS` `90`, the marker, the operator surface with its curl
  playbook, and `YOKOHAMA_WEB_ADMINS` as a secret (Task 14);
- the live Playwright tier and the live corpus replay, with the replay's ids forced before and after
  each run (Tasks 14–15).

### 6.8 M6.5c's skeleton is live — the fourteenth deployment, 2026-09-23 (a record, not a plan)

The first deploy of the Worker (spec §3.2), from `756dc759` — Task 11's reviewed head, tree clean:
the Worker, the three classes in migration `v1`, the static assets, identity, the body bound and
the Origin check — and the spike's measurements (spec §0.3) re-read with the real code before
anything else lands on it. **Version `8b1bdcf8-610d-4b04-9e21-727e216cd61a`.** Every reading
matched the spike's in substance. One differs in form (reading 1, the zone's challenge page), and
the spike's one unexplained error (reading 6, the beacon) is explained; neither stops the cycle.

- **Before the deploy — the NAS's body bound, read live (2026-09-23, over WARP):** a
  2,097,153-byte `POST /api/games` answered **400**
  `{"error":"the body could not be read: length limit exceeded"}`, and exactly 2,097,152 bytes
  passed the bound (a 401, from identity). The Worker's `BODY_LIMIT_SENTENCE` is that text.
- **Build:** `web/`'s `npm run build` — wasm-opt 133, the browser module **1,175,106 B**,
  `grep -c debug_trap web/src/local/wasm/yokohama_wasm.js` **0**; `precache: 153 entries, 9,938,226
  B`, the cache `yokohama-release-10e4457c62975466`. `worker/`'s `npm run build` — the module
  **1,854,072 → 1,220,664 B** (wasm-opt 133); the assembly **153 entries verified**, 156 files,
  9,967,639 B; the release smoke **200**.
- **The deploy (`npm run deploy`):** the guard PASSED; **154 assets** uploaded in 2.65 s (the 156
  assembled files less `_headers` and `_redirects`, which configure the asset layer); Total Upload
  **1260.83 KiB / gzip 484.78 KiB**; **Worker Startup Time 2 ms**; the bindings `TABLE`, `HOUSE` and
  `ARCHIVER` (Durable Objects), `ASSETS`, and the eight variables of §6.3's table; the custom domain
  `yokohama.moore.nyc`.
- **Reading 1 — protection, from an unenrolled vantage** (a throwaway `alpine:3.22` container on the
  NAS, curl). **Nothing was served.** Two request shapes read differently, and both are recorded:
  - with curl's default User-Agent: `/` **403**, `/sw.js` **302** to the Access login
    (`bdvc.cloudflareaccess.com`), `/api/health` **403**, `/enter` **403**, `/index.html` **403**,
    `/assets/x.js` **302** to the login; a WebSocket upgrade with no credential **403**;
    `yokohama-web.bdvc.workers.dev` **404**. The 403s carry `cf-mitigated: challenge` and a
    5,259-byte "Just a moment…" body: the zone's Bot Fight Mode and JavaScript detections —
    kept ON by the owner's decision, 2026-09-23 — challenging a bot-shaped document request
    BEFORE Access sees it;
  - with a browser User-Agent and `Accept: text/html`: `/` and `/api/health` **302** to the Access
    login, whose `meta` reads `auth_status NONE`, `is_warp false`, `service_token_status false` —
    exactly the spike's row P;
  - the NAS host, from the same vantage, reads the same (`/` 403, `/api/health` 403, `/sw.js` 302),
    so the challenge page is the zone's behaviour and not this Worker's. Ruled a difference in
    form, recorded, not a finding that stops the cycle.
- **Reading 2 — identity on an upgrade** (`curl --http1.1` against `/api/stream`):
  - **2a**, the owner over WARP, no credential: **101**. It is the first run of the production key
    source, `keySource` over workerd's `fetch`, which no cell runs: it verified a real assertion;
  - **2b**, the credential with `X-Yokohama-Claimed-User: probe@deploy.test`: **101**;
  - **2c**, the credential alone: **401** `{"error":"no identity reached the server: this deployment
    expects X-Yokohama-User from the edge Worker"}` — the layer's own `worker`-mode sentence.

  All three as the spike (row 1).
- **Reading 2d — the Origin check, live**, over WARP, taken after the re-review of `756dc759`
  suggested it (§6.5 has the two commands):
  - an upgrade with `Origin: https://yokohama.moore.nyc`: **101** — a production request URL's
    origin is the site's own, so a page's socket is not refused;
  - an upgrade with `Origin: https://evil.example`: **403** `{"error":"this socket is for the site's
    own pages"}`, unupgraded;
  - a plain `GET /api/health` with that foreign `Origin`: **200** — a request that is not an
    upgrade is never checked.
- **Reading 3 — the static assets:**
  - `read-deploy.mjs assets`, over WARP: `/sw.js` 200, `cache-control: no-store`, **153** precache
    entries; **152 byte-exact** and **1 EDGE** — `/index.html`, into which the edge injected its
    `challenge-platform` script, and which is the release's bytes once that is cut away; `/index.html`
    **200**, never a redirect; `/assets/x.js` **404** with an empty body → **HOLDS**;
  - `read-deploy.mjs install`, the credential sent to the Worker's origin only: the service worker
    **activated**; its cache holds **154** entries for the 153 listed (the `/__release__` stamp is
    the 154th); `/index.html` cached at **2,387 B**, the release's bytes; the door reads "Ready to
    play offline." → **HOLDS**;
  - the tail (`wrangler tail --format json`, 1,605 lines): the Worker was invoked for `/api/me` ×6,
    `/api/stream` ×3, `/api/health` ×1 and `/assets/x.js` ×1 — the missing file, exactly ONCE — and
    ZERO times for any precache path, `/sw.js`, `/index.html`, `/art/*`, `/help/*` or the fonts.

  As the spike (row 7).
- **Reading 4 — cold start and CPU,** off the tail:
  - cold start, the deploy's "Worker Startup Time": **2 ms** (the spike: 1–3 ms);
  - the stateless Worker on `/api/*`, JWT verification included: **cpu 0–2 ms**, 1 ms typical, wall
    10–19 ms; the first `/api/stream` upgrade 2 ms cpu, 393 ms wall. R-6.5c.5 left this unmeasured:
    it is about 1 ms, far inside Free's 10 ms;
  - the objects: the house's first `/api/health` **7 ms** cpu, `/api/me` 0–2 ms, the first
    `/api/stream` upgrade **9 ms**;
  - outcomes: 19 `ok`, and 4 `canceled` — the upgrades curl's `--max-time` cut; `identity-failed`
    lines: **0**.
- **Reading 5 — health,** over WARP: `identity: "cf-access"`, `signal: "ws"`, `archive: "off"`,
  `multiplayer: true`, `semantics_rev` 7, `rules_hash` `d85445c479cb1ead`; `/api/me` names the
  owner's address, through the real assertion; `GET /` answers 200 at **3,325 B** — the edge's
  JavaScript-detection injection, as spike row 8 (2,387 B built).
- **Reading 6 — the Web Analytics beacon, diagnosed** (`read-deploy.mjs console`, two loads each):
  - over WARP, NO custom header anywhere — a real browser's shape: console messages **none**,
    failed requests **none**; the beacon script **200**; the RUM post to
    `https://yokohama.moore.nyc/cdn-cgi/rum` **204**;
  - with the credential sent to the Worker's origin only: the same, and **no error**;
  - **the diagnosis:** the spike's CORS error was an artifact of the spike's HARNESS, which sent the
    credential as CONTEXT-WIDE headers. Custom headers on the cross-origin beacon request force a
    CORS preflight the beacon's host does not answer. A real browser sends no custom header and
    sees no error, and neither does a harness that scopes the credential to this origin. The zone
    stays exactly as it is — Web Analytics, Bot Fight Mode and JavaScript detections ON — no
    allow-list is needed, and the live test tiers scope the credential per origin.
- **Against the spike (spec §0.3):**
  - row 1, identity — the same;
  - row 2, startup and size — the same class: 2 ms, and 1,260.83 KiB against the spike's 1,172.84 KiB,
    because the Worker now carries the shell;
  - row 7, the static assets — the same: 152 exact plus `/index.html` edge-rewritten, no invocation
    for a real file, one for a missing file, the service worker installed whole;
  - row 8, the edge — the same 3,325 B injection, and the beacon's error explained above;
  - row P, protection — the same in substance, with the zone's challenge page the one new shape.

#### What this deployment does NOT claim

- **The raw-address rule (§6.3; Task 2's note E1 and Task 11's I1) was not read live**: no Access
  identity with a non-ASCII address exists to try it with. The pool's cells are its evidence.
- **No game was played and no table was created** live. The lobby lists nothing until Task 12's
  copies; the archive is off until Task 13 binds the bucket; there is no expiry or operator surface
  until Task 14.
- **No phone and no browser off WARP.** Reading 1's unenrolled vantage was curl, not a browser.

#### Rollback

§6.6: there is no earlier version; the NAS, untouched, is the fallback. The migration `v1` is FROZEN
from this deployment on: a class added later takes a new tag, and no version can be rolled back
across one.

### 6.9 The house and the slot are live — the fifteenth deployment, 2026-09-24 (a record, not a plan)

From `b24149b5`, Task 12's reviewed head, its fix round included, tree clean. It brings the house
over scrubbed copies, a durable ordered slot per table, the corpus replayed through the objects in
the pool, and `/api/art` projected onto the layer's nine maps. **Version
`e355c676-cfb2-400b-926d-05def2e725fd`.** Every reading holds. The readings ran with the credential
and claimed `.test` identities, the tail listening, and the header file deleted afterwards.

- **Build:** `worker/`'s `npm run build`. The module is **1,220,664 B**; the assembly **153 entries
  verified**, 156 files, 9,967,639 B; the release smoke **200**.
- **The deploy:** the guard PASSED its six checks, the comparison of the two configurations' object
  blocks among them. Total Upload **1288.14 KiB / gzip 493.52 KiB** (the fourteenth's: 1260.83
  KiB); **Worker Startup Time 2 ms**; the custom domain `yokohama.moore.nyc`.
- **Reading 1 — a locked create** as `probe@deploy.test` through the credential, its seed
  `9007199254740993` (above 2^53): **201** `sim-81e5fae99a122963`, status `open`, `locked: true`.
- **Reading 2 — the lobby, listed from the house's copies:** for the creator (`scope=mine`,
  `my_seats [0]`) and for `second@deploy.test` (`scope=open`, `my_seats []`). In both listings the
  seed's digits appear **0** times, runs of 32 or more hex characters **0** times, and the keys
  `salt`, `hash` and `seed` **0** times. What the lobby serves carries none of them, live. The
  scrub itself — the copy the house holds — is read in the pool.
- **Reading 3 — the lobby socket** as `watcher@deploy.test`, over `curl --http1.1`:
  `HTTP/1.1 101 Switching Protocols`. On `second`'s join (**200**, with the password) it received
  exactly `{"event":"house","data":{"seq":2,"chat":0}}`. `seq` is the house's in-memory count of
  the copies and removals it applied, so 2 matches the create's copy and then the join's.
- **Reading 4 — abandon:**
  - the creator's DELETE: **204**, and the id is gone from `scope=mine` (0 rows);
  - a re-create naming the SAME id: **201**. Under an unacknowledged removal it would be 409, so the
    removal was acknowledged and the object ended. The tail shows a NEW generation: `at`
    1790206923, with a new `rand`;
  - its DELETE: **204**.
- **Reading 5 — `/api/art`:** its keys, unsorted, are `orders, techs, sites, achievements, areas,
  five_power, mercenaries, backs, tokens` — the nine maps in the struct's order, with no
  `area_duplicates` and no `provenance`.
- **Reading 6 — the tail:** `slot-pushed`/`slot-acked` pairs for revisions 1 (the create), 2 (the
  join) and 3 (the REMOVAL), then for the re-created generation's 1 and 2 (its removal). No
  `copy-refused`, `trap` or `alarm-failed` line, and no alarm fired.
- **Against the spike (spec §0.3):**
  - row 2, startup: 2 ms, within the spike's 1–3 ms, and the fourteenth's 2 ms. The upload is
    larger than the spike's 1,172.84 KiB because the Worker carries the shell;
  - row 3, `getrandom` in an object: the same. Reading 1's create was LOCKED, so `Lock::new` drew
    its 16-byte salt through the module's `getrandom` inside the table object, and it worked: the
    create answered 201 `locked: true`, and `second`'s join with the password answered 200.

#### What this deployment does NOT claim

- **Spike rows 4 and 5 were not re-read here:** no trap was raised live, and no object's CPU was
  measured.
- **No alarm fired, by design.** Every push was acknowledged, and the acknowledgement deletes the
  slot's alarm. So a retry's own delivery, and `ctx.id.name` inside a production `alarm()` (the
  plan's pool fact 8), were not read live. The pool's cells are their evidence.
- **The pool-only paths:** an owed copy (`copy_owed`) and its backoff, a refused copy, a failed
  push and its retry, and the house chat's load traps were not raised live.
- **The corpus replay ran in the pool only**; the live replay is Task 15's. The slot's `last_active`
  and `bytes` reached the house, but the operator's listing that shows them is Task 14's.

#### Rollback

§6.6. The earlier version is now the fourteenth's, `8b1bdcf8-610d-4b04-9e21-727e216cd61a`, and no
migration was added, so `npx wrangler rollback` can return to it. Its code reads no copies, so its
lobby would list nothing; the copies stay in the house's storage. Every table and its game stay as
they are: the fourteenth's code never reads the `meta` keys this deployment added, and a pending
slot's alarm finds nothing to do there.
