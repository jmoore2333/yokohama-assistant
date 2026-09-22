# The Yokohama browser client and its API, as one image.
#
# **One artifact, two targets.** This is what runs under Portainer on the NAS
# today and what Modal runs later; the deployment decision is where to point a
# tunnel, not what to build. Nothing in here is host-specific — every path the
# server needs is an environment variable with a default that resolves inside
# the image.
#
# Built for `linux/amd64` deliberately. The DS1517+ is an Intel Atom C2538, so
# amd64 is right for it AND for Modal — but the build happens on a workstation,
# because compiling this workspace on that Atom is where the plan would fall
# over rather than where it would be tested.

# ---------------------------------------------------------------------------
# 1. The server, AND the browser host's WebAssembly module. `rust-toolchain.toml`
#    pins the compiler, so this stage resolves the SAME 1.97.1 both workstations
#    use rather than whatever the base image happens to ship — the pin is the
#    point of having one.
#
#    **Why the module is built here and copied down.** Since M6.5b the client's
#    `npm run build` is `build:wasm && build:client`, and `build:wasm` needs
#    cargo, the `wasm32-unknown-unknown` target and `wasm-bindgen` — none of
#    which a `node:22-slim` image has, and the client stage is not even given
#    the workspace's manifests. So this stage, which has all three, emits the
#    generated glue to `/out/wasm` and the client stage copies it in and runs
#    `build:client` alone. That is also why this stage now comes FIRST:
#    `COPY --from=` needs its source stage defined earlier in the file.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS server
WORKDIR /src

# `cargo build` needs no C toolchain for this workspace today (no build.rs
# anywhere, checked), but pkg-config and a linker are cheap insurance and this
# stage is not in the final image. `curl` is here for the step below and for
# nothing else.
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config curl \
 && rm -rf /var/lib/apt/lists/*

# **`wasm-opt` is not insurance — it is a prerequisite** (M6.5b2, R-6.5b2.15).
# The module is built with `--profile release-wasm` and then run through
# `wasm-opt -Oz`, which is 1.78 MB -> 1.17 MB; `npm run build:wasm` refuses
# outright when it is absent, and this stage runs the same steps by hand.
#
# **Pinned to the SAME upstream binary the workflow installs, not to apt.**
# Debian trixie ships `binaryen 120-4` (`wasm-opt version 120`, read off an
# `apt-cache policy` in this very base image) where the workstation and CI run
# 132, and that was a measured difference and not a theoretical one: over the
# same input, apt's 120 emitted 1,174,709 B where 132 emits 1,173,055 B. The
# image is `linux/amd64`, so the workflow's own asset applies byte for byte.
# One static binary out of a 22-entry release; `libbinaryen` is a static
# archive, so nothing else needs unpacking.
#
# **What pinning the optimizer did NOT close, measured rather than assumed.**
# This stage's module is **1,173,055 B** against the workstation's
# **1,169,222 B** — 3,833 B, 0.33% — with the SAME `wasm-opt 132` on both. The
# optimizer is converged (running 132 again over this stage's output shaves
# 620 B), so the residue is in what `cargo` and `wasm-bindgen` emit on an
# x86_64-linux build host versus an aarch64-darwin one. Recorded, not
# diagnosed; `-Oz` is semantics-preserving and the corpus replay passes on the
# workstation's. **A ceiling on the module must clear the LARGER figure**,
# because this stage is what a deployment ships.
RUN curl -fsSL -o /tmp/binaryen.tar.gz \
      https://github.com/WebAssembly/binaryen/releases/download/version_132/binaryen-version_132-x86_64-linux.tar.gz \
 && tar -xzf /tmp/binaryen.tar.gz -C /tmp binaryen-version_132/bin/wasm-opt \
 && install -m 0755 /tmp/binaryen-version_132/bin/wasm-opt /usr/local/bin/wasm-opt \
 && rm -rf /tmp/binaryen.tar.gz /tmp/binaryen-version_132 \
 && wasm-opt --version

# **The pin first, and the target onto the pinned toolchain.** `rustup target
# add` applies to whichever toolchain rustup resolves at that moment, so this
# COPY has to precede it: with `rust-toolchain.toml` in the working directory
# rustup installs 1.97.1 and adds the target to THAT, where cargo will look for
# it. Without the file first, the target would land on the image's own default
# toolchain and `cargo build --target wasm32-unknown-unknown` would then fail
# to find it — the same trap `.github/workflows/test.yml` names for CI.
COPY rust-toolchain.toml ./
RUN rustup target add wasm32-unknown-unknown

# `wasm-bindgen-cli` at the version `crates/yokohama-wasm` depends on
# (`wasm-bindgen = "=0.2.128"`); generator and crate must match exactly or the
# glue refuses the module at load. Built from source with `--locked`, the same
# command CI runs, rather than fetching a release tarball: one fewer download to
# verify, and no per-architecture asset name to keep right. It is a few minutes
# once, and it sits ABOVE the source COPY so Docker caches the layer and a
# source edit never pays for it again.
RUN cargo install wasm-bindgen-cli --version 0.2.128 --locked

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# `-p yokohama-web` pulls `yokohama-cli` and `yokohama-core` as dependencies
# and leaves the oracle, difftest, bridge and gen-data crates unbuilt — they
# are development tooling and nothing at runtime calls them.
#
# Both builds, the `wasm-bindgen` run and the `wasm-opt` pass are ONE step
# because `target/` is a cache mount: it does not survive the layer, so anything
# that reads what a build wrote has to read it here. `/out/wasm` is outside the
# mount, which is what makes the glue a real artifact the next stage can copy —
# and the optimizer therefore runs on the copy in `/out/wasm`, after
# `wasm-bindgen`, which is the same order `npm run build:wasm` uses.
#
# **The server binary stays on `--release` and the module moves to
# `--profile release-wasm`** (M6.5b2, R-6.5b2.15): `opt-level = "z"` is a trade
# a phone's cold start wants and a long-running server does not.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p yokohama-web \
 && cp target/release/yokohama-web /usr/local/bin/yokohama-web \
 && cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-wasm \
 && wasm-bindgen --target web --out-dir /out/wasm \
      target/wasm32-unknown-unknown/release-wasm/yokohama_wasm.wasm \
 && wasm-opt -Oz --strip-debug --strip-producers \
      /out/wasm/yokohama_wasm_bg.wasm -o /out/wasm/yokohama_wasm_bg.wasm \
 && ls -l /out/wasm/yokohama_wasm_bg.wasm

# ---------------------------------------------------------------------------
# 2. The browser client. Vite output only; none of this reaches the runtime
#    image except `dist/`.
# ---------------------------------------------------------------------------
FROM node:22-slim AS client
WORKDIR /app

# Manifests first so `npm ci` is cached independently of the source. A source
# edit then costs a rebuild of the bundle and not of node_modules.
COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./
# The card art, for the precache generator, which hashes it (M6.5b2 Task 6).
# The runtime stage below copies the same tree to `/srv/assets/art` and that is
# where the SERVER reads it; this copy exists because `build:client` now has a
# step that walks every file the release contains. The path is chosen by what
# the generator asks for, which is `../crates/yokohama-web/assets/art` relative
# to `web/` — and this stage's `web/` is `/app`, so the tree has to sit at the
# image root for that `..` to land on it.
COPY crates/yokohama-web/assets/art /crates/yokohama-web/assets/art
# The generated glue, from the stage that has the toolchain. It lands where
# `npm run build:wasm` would have written it — `web/src/local/wasm/`, a source
# directory, so Vite bundles the loader and emits the `.wasm` as a hashed asset
# (a static import from `public/` fails Rollup's resolution).
# **`deploy/Dockerfile.dockerignore`** keeps a developer's own copy out of the
# context, so this is the only way it arrives — which is what makes the module
# in the image the one the stage above built. It is named for the Dockerfile
# rather than sitting at the context root because BuildKit prefers
# `<dockerfile>.dockerignore` when it exists; a root `.dockerignore` beside it
# would be read by nothing and would drift in silence.
COPY --from=server /out/wasm ./src/local/wasm
# `build:client`, not `build`: the module is already here, and running
# `build:wasm` in this stage is exactly what has no toolchain to run it.
RUN npm run build:client

# ---------------------------------------------------------------------------
# 3. The runtime. No toolchain, no source, no node_modules.
# ---------------------------------------------------------------------------
FROM debian:trixie-slim
LABEL org.opencontainers.image.title="yokohama-web"
LABEL org.opencontainers.image.description="Yokohama engine, HTTP API and browser client"
LABEL org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# An unprivileged user, and the data directory owned by it. The server writes
# exactly one place — the game store — and nothing else in the image is
# writable by the process.
RUN useradd --system --create-home --uid 10001 yokohama \
 && mkdir -p /data/games \
 && chown -R yokohama:yokohama /data

WORKDIR /srv
COPY --from=server /usr/local/bin/yokohama-web /usr/local/bin/yokohama-web
COPY --from=client /app/dist /srv/web/dist
COPY crates/yokohama-web/assets/art /srv/assets/art

# **`0.0.0.0` here and NOWHERE ELSE.** The binary's own default is loopback,
# deliberately. Since M6.1 this API reads an identity of its own — one header,
# named by `YOKOHAMA_WEB_IDENTITY`, required on every `/api/games` route, with
# 401 for no identity and 403 for a caller with no seat — but that is not the
# same as being safe to expose: `cf-access` and `worker` trust a header only
# their own edge can set, and `claimed` authenticates the door and not the seat.
# Binding wide is still a statement that something in FRONT of this container is
# doing identity — a Cloudflare Tunnel with an Access policy, or Modal's
# proxy-auth layer plus the edge Worker — and it belongs in the deployment
# artifact where that is true, not in the program.
#
# **`YOKOHAMA_WEB_IDENTITY` is deliberately NOT set here either**, for the same
# reason: there is no default mode, and an image started without it exits 1
# naming the three accepted values. The NAS sets it on the container
# (`RUNBOOK.md` §3, `claimed`); `modal_app.py` sets it on the image it builds
# from this file (`worker`, because there the Worker is the thing that
# authenticates).
# The three chat variables (`YOKOHAMA_WEB_LOBBY_CHAT`, `YOKOHAMA_WEB_CHAT_KEEP`,
# `YOKOHAMA_WEB_CHAT_TEXT`) are the other way round: each has a default, so an
# image started without them runs the lobby chat on with a keep of 500
# messages and 500 characters per message (M6.4, R-6.4.6, R-6.4.8); a bad value
# still exits 1 naming the variable.
ENV YOKOHAMA_WEB_HOST=0.0.0.0 \
    YOKOHAMA_WEB_PORT=8080 \
    YOKOHAMA_WEB_DIST=/srv/web/dist \
    YOKOHAMA_WEB_ART=/srv/assets/art \
    YOKOHAMA_WEB_DIR=/data/games

# The one durable thing. Every applied move is written here as its `GameRecord`
# and a game is rebuilt from it, so a container that is replaced or scaled to
# zero loses no acknowledged move — provided this is a real volume and not the
# container's own writable layer.
VOLUME ["/data"]

EXPOSE 8080
USER yokohama

# The server drains on SIGTERM (`with_graceful_shutdown`), so it must receive
# it directly. Exec form, no shell wrapper: a shell as PID 1 would swallow the
# signal and the drain would never run.
STOPSIGNAL SIGTERM
CMD ["yokohama-web"]
