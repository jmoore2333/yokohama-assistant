#!/usr/bin/env bash
# **The self-host image, smoke-run under both of its doors** (M6.5c2, R-6.5c2.14;
# spec §2.9). `.github/workflows/docker.yml` runs it on every image it builds,
# and a self-hoster can run it on theirs before putting it behind anything:
#
#     bash deploy/smoke-image.sh <image> [port]
#
# It proves three things about the IMAGE and nothing about a deployment:
#
# 1. no door is a refusal to start, never a guess: the container exits 1 and
#    its output names `YOKOHAMA_WEB_IDENTITY`;
# 2. under `cf-access`, the repository's door, the server reads ONLY
#    `Cf-Access-Authenticated-User-Email`, and only an address (the mode rule):
#    a plain name in that header is no one;
# 3. under `open`, which this image must ship (`open-door` is a default
#    feature), a typed name is a caller, keyed as `User::parse` keys it.
#
# `/api/health` alone cannot tell a working door from a broken one — it answers
# the same either way — so every door is also read through `/api/me` and
# `/api/games`, and the static release and one art file are fetched under each.
# It prints the image's size as Docker reports it and each serving container's
# memory. Every container it starts is removed on exit WITH its volume — the
# image declares `VOLUME ["/data"]`, and `docker rm -f` without `-v` leaves an
# anonymous volume behind per container — and the two that serve are
# published on 127.0.0.1 only.
set -euo pipefail

image="${1:?usage: deploy/smoke-image.sh <image> [port]}"
port="${2:-18080}"
base="http://127.0.0.1:${port}"
name="yokohama-smoke-$$"
nodoor="yokohama-smoke-nodoor-$$"
current="$name"
status="" body=""

cleanup() { docker rm -f -v "$name" "$nodoor" >/dev/null 2>&1 || true; }
trap cleanup EXIT

fail() {
  echo "smoke: FAIL — $*" >&2
  docker logs "$current" 2>&1 | tail -n 20 >&2 || true
  exit 1
}

# field <json> <key>: the value, JSON-encoded, so null reads `null` and a string
# `"x"`. A body that is not a JSON object reads `<not JSON>` and fails the next
# `expect` by name.
field() {
  python3 -c 'import json,sys; print(json.dumps(json.loads(sys.argv[1])[sys.argv[2]]))' "$1" "$2" 2>/dev/null \
    || echo "<not JSON>"
}

# get <path> [curl args…]: sets $status and $body. A refused connection reads as
# status 000 and fails the next `expect` by name, rather than exiting under -e.
get() {
  local path="$1"; shift
  local tmp; tmp="$(mktemp)"
  status="$(curl -sS -o "$tmp" -w '%{http_code}' "$@" "${base}${path}" || true)"
  body="$(cat "$tmp")"; rm -f "$tmp"
}

expect() { # expect <what> <got> <want>
  [ "$2" = "$3" ] || fail "$1: got $2, want $3"
  echo "smoke:   $1 = $2"
}

running() { [ "$(docker inspect -f '{{.State.Running}}' "$1" 2>/dev/null)" = true ]; }

start() {
  docker rm -f -v "$name" >/dev/null 2>&1 || true
  current="$name"
  docker run -d --name "$name" -p "127.0.0.1:${port}:8080" -e "YOKOHAMA_WEB_IDENTITY=$1" "$image" >/dev/null
  for _ in $(seq 1 60); do
    curl -fsS -o /dev/null "${base}/api/health" 2>/dev/null && return 0
    running "$name" || fail "the container exited under YOKOHAMA_WEB_IDENTITY=$1"
    sleep 1
  done
  fail "no /api/health 200 within 60 s under $1"
}

# door <door> <its header> <a caller as sent> <that caller's key> <the other door's header line>
door() {
  echo "smoke: door $1"
  start "$1"
  get /api/health
  expect "GET /api/health status" "$status" 200
  expect "health.identity" "$(field "$body" identity)" "\"$1\""
  expect "health.signal" "$(field "$body" signal)" '"sse"'
  echo "smoke:   health.server_version = $(field "$body" server_version)"
  get /api/me
  expect "GET /api/me, no header: mode" "$(field "$body" mode)" "\"$1\""
  expect "GET /api/me, no header: user" "$(field "$body" user)" null
  get /api/games
  expect "GET /api/games, no header: status" "$status" 401
  get /api/me -H "$2: $3"
  expect "GET /api/me as '$3': user" "$(field "$body" user)" "\"$4\""
  get /api/games -H "$2: $3"
  expect "GET /api/games as '$3': status" "$status" 200
  get /api/me -H "$5"
  expect "GET /api/me with the other door's header: user" "$(field "$body" user)" null
  get /api/me -H "X-Yokohama-User: smoke@ci.test"
  expect "GET /api/me with the worker door's header: user" "$(field "$body" user)" null
  get /
  expect "GET / status" "$status" 200
  get /sw.js
  expect "GET /sw.js status" "$status" 200
  get /art/areas/Church.webp
  expect "GET /art/areas/Church.webp status" "$status" 200
  docker stats --no-stream --format 'smoke:   memory {{.MemUsage}}' "$name"
}

# 1. No door: the process refuses to start, exits 1 and names the variable.
#    Detached and BOUNDED, because an image that guessed a door would serve for
#    ever and a foreground `docker run` would never return to say so.
echo "smoke: no door"
current="$nodoor"
docker run -d --name "$nodoor" "$image" >/dev/null
for _ in $(seq 1 30); do
  running "$nodoor" || break
  sleep 1
done
running "$nodoor" && fail "the image is still serving 30 s after starting with no YOKOHAMA_WEB_IDENTITY; it must refuse"
code="$(docker inspect -f '{{.State.ExitCode}}' "$nodoor")"
out="$(docker logs "$nodoor" 2>&1)"
expect "no door: exit code" "$code" 1
grep -q "YOKOHAMA_WEB_IDENTITY" <<<"$out" || fail "the refusal does not name YOKOHAMA_WEB_IDENTITY: $out"
echo "smoke:   no door: refused, naming YOKOHAMA_WEB_IDENTITY — $out"
docker rm -f -v "$nodoor" >/dev/null

# 2. cf-access: only Access's header, and only an address.
door cf-access "Cf-Access-Authenticated-User-Email" "Smoke@CI.test" "smoke@ci.test" \
  "X-Yokohama-Claimed-User: Smoke Runner"
get /api/me -H "Cf-Access-Authenticated-User-Email: ben"
expect "GET /api/me as 'ben' in Access's header (a name, not an address): user" "$(field "$body" user)" null

# 3. open: a typed name, trimmed, its spaces collapsed, lowercased.
door open "X-Yokohama-Claimed-User" "  Smoke   Runner " "smoke runner" \
  "Cf-Access-Authenticated-User-Email: smoke@ci.test"

# The size is Docker's own `.Size`, the figure `docker image ls` shows, and
# what it counts depends on the image store: the unpacked layers under the
# classic store; under the containerd snapshotter, the compressed content plus
# whatever has been unpacked — measured on Docker 29.8.0, one image read
# 42,348,543 B straight after its build and 151,777,279 B once this script had
# run it. Compare a reading only with another from the same store.
docker image inspect --format 'smoke: image {{.Size}} bytes (docker image inspect .Size)' "$image"
echo "smoke: PASS — $image"
