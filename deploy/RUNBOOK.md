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
- **The store is not rolled back by either.** It is the named volume
  `yokohama-web-data`; the records are the whole state, which is why they
  replay.
