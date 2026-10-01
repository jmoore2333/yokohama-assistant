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
# 1. The browser client. Vite output only; none of this reaches the runtime
#    image except `dist/`.
# ---------------------------------------------------------------------------
FROM node:22-slim AS client
WORKDIR /app

# Manifests first so `npm ci` is cached independently of the source. A source
# edit then costs a rebuild of the bundle and not of node_modules.
COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

# ---------------------------------------------------------------------------
# 2. The server. `rust-toolchain.toml` pins the compiler, so this stage
#    resolves the SAME 1.97.1 both workstations use rather than whatever the
#    base image happens to ship — the pin is the point of having one.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS server
WORKDIR /src

# `cargo build` needs no C toolchain for this workspace today (no build.rs
# anywhere, checked), but pkg-config and a linker are cheap insurance and this
# stage is not in the final image.
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config \
 && rm -rf /var/lib/apt/lists/*

COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# `-p yokohama-web` pulls `yokohama-cli` and `yokohama-core` as dependencies
# and leaves the oracle, difftest, bridge and gen-data crates unbuilt — they
# are development tooling and nothing at runtime calls them.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p yokohama-web \
 && cp target/release/yokohama-web /usr/local/bin/yokohama-web

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
