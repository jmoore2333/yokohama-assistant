#!/usr/bin/env bash
# **Run the self-host image on this machine, under the open door** (security
# S4a, Task 5; the design §6.10). From a checkout of this repository, in any
# directory:
#
#     bash deploy/run-local.sh [--dry-run | --stop]
#
# It is `deploy/README.md` §3's build and §4's run in one command, for a person
# who wants to play on their own machine: the image built from the checkout this
# script sits in, run under the open door on loopback with a named volume.
# `docs/web-guide.md` §4 is the guide that names it.
#
# A run, in order, stopping at the first failure:
#
#   1. refuse without `docker` or `curl`, naming the one missing;
#   2. refuse outside a checkout: `deploy/Dockerfile` is resolved from this
#      script's own location, never from the working directory;
#   3. refuse when a container of its name exists, whether running or stopped
#      ("run `--stop` first");
#   4. say, before the build, that the image is `linux/amd64` by design and
#      that a first build on Apple silicon runs under emulation and takes
#      minutes;
#   5. from the checkout's root, with no build values:
#      `docker build --platform linux/amd64 -f deploy/Dockerfile -t "$IMAGE" .`
#   6. `docker run -d --name "$CONTAINER" -p "$PUBLISH" -v "$VOLUME:/data"
#      -e YOKOHAMA_WEB_IDENTITY=open -e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on
#      --restart unless-stopped --cap-drop ALL
#      --security-opt no-new-privileges:true --memory 512m "$IMAGE"` — the
#      Host check on with no list, because loopback is in its default set
#      (`deploy/README.md` §4), and §1's three hardening flags;
#   7. read the start instead of assuming it: poll
#      `http://<the publish address>/api/health` every second for up to 60 s.
#      On a 200 it prints `run-local: open http://127.0.0.1:8080/` and the
#      trust model's line; otherwise it prints the last status it read, the
#      container's last 20 log lines, and exits 1, leaving the container for
#      `docker logs`.
#
# **`--stop`** runs `docker stop` and `docker rm` on the container and says
# that the volume stays with the games in it; with no container of its name it
# says nothing is running. Both exit 0. It never touches the volume or the
# image: the next run finds the games where they were.
#
# **`--dry-run`** prints every command a run would execute and runs none: no
# build, no container, not even a `docker inspect` to look for one. It reads
# the files the checkout check reads, and nothing else.
#
# **The environment**, each optional, with its default: `YOKOHAMA_WEB_IMAGE`
# (`yokohama-web:local`), `YOKOHAMA_WEB_CONTAINER` (`yokohama-web-local`),
# `YOKOHAMA_WEB_VOLUME` (`yokohama-web-local-data`) and `YOKOHAMA_WEB_PUBLISH`
# (`127.0.0.1:8080:8080`, Docker's `[address:]port:8080`; the poll reads the
# address and port before the last colon, and loopback for an address of
# `0.0.0.0` or none).
#
# **The trust model is the open door's** (`deploy/README.md` §2): anyone who can
# reach the port may use any name, and a name is a label and a key, never a
# credential. The default binds 127.0.0.1, this machine alone. For a LAN, a
# tunnel or Access, follow the self-host guide instead of widening this
# script's address: with no host list the server answers loopback names alone,
# and any other `Host` reads 421.
#
# A refusal exits 2; a container that does not answer its health exits 1.
set -euo pipefail

die() { echo "run-local: $*" >&2; exit 2; }

# ---- the arguments ----------------------------------------------------------
DRY_RUN=0
STOP=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --stop) STOP=1 ;;
    *) die "unknown argument \`$arg\` (usage: bash deploy/run-local.sh [--dry-run | --stop])" ;;
  esac
done
if [ "$DRY_RUN" = 1 ] && [ "$STOP" = 1 ]; then
  die "\`--dry-run\` and \`--stop\` are one or the other (usage: bash deploy/run-local.sh [--dry-run | --stop])"
fi

IMAGE="${YOKOHAMA_WEB_IMAGE:-yokohama-web:local}"
CONTAINER="${YOKOHAMA_WEB_CONTAINER:-yokohama-web-local}"
VOLUME="${YOKOHAMA_WEB_VOLUME:-yokohama-web-local-data}"
PUBLISH="${YOKOHAMA_WEB_PUBLISH:-127.0.0.1:8080:8080}"

# ---- the tools: a real run needs both, `--stop` docker alone ----------------
if [ "$DRY_RUN" = 0 ]; then
  if [ "$STOP" = 1 ]; then tools="docker"; else tools="docker curl"; fi
  for tool in $tools; do
    command -v "$tool" >/dev/null 2>&1 \
      || die "\`$tool\` is not on PATH; this script needs docker to build and run the image, and curl to read its health"
  done
fi

# ---- show / run: print the line, then run it unless this is a dry run --------
show() {
  printf '+'
  for a in "$@"; do
    case "$a" in
      *"%{"*) printf " '%s'" "$a" ;;   # a curl format: shown in single quotes, as typed
      *) printf ' %q' "$a" ;;
    esac
  done
  printf '\n'
}
run() {
  show "$@"
  if [ "$DRY_RUN" = 1 ]; then return 0; fi
  "$@"
}

# ---- --stop -----------------------------------------------------------------
if [ "$STOP" = 1 ]; then
  if docker container inspect "$CONTAINER" >/dev/null 2>&1; then
    run docker stop "$CONTAINER"
    run docker rm "$CONTAINER"
    echo "run-local: $CONTAINER is stopped and removed; the volume $VOLUME stays, with the games in it"
  else
    echo "run-local: nothing is running — no container is named $CONTAINER; the volume $VOLUME is untouched"
  fi
  exit 0
fi

# ---- the checkout: resolved from this script's own location -----------------
# Pure bash, no `dirname`: a script read from a pipe or a process substitution
# has no file of its own to resolve from, and is refused with the rest.
src="${BASH_SOURCE[0]:-}"
[ -n "$src" ] && [ -f "$src" ] \
  || die "this script must be run from its file in a checkout of the repository (bash deploy/run-local.sh), not from a pipe"
case "$src" in
  */*) here="${src%/*}" ;;
  *) here="." ;;
esac
ROOT="$(cd "$here/.." && pwd)"
[ -f "$ROOT/deploy/Dockerfile" ] \
  || die "no $ROOT/deploy/Dockerfile, the path resolved from this script's own location; run it from a checkout of the repository, where it builds the image from the tree"

# ---- the address the poll reads, from Docker's [address:]port:8080 ----------
host_port="${PUBLISH%:*}"
case "$host_port" in
  *:*) bind="${host_port%:*}"; port="${host_port##*:}" ;;
  *) bind="0.0.0.0"; port="$host_port" ;;
esac
case "$bind" in
  0.0.0.0) poll_host="127.0.0.1" ;;
  "[::]") poll_host="[::1]" ;;
  *) poll_host="$bind" ;;
esac
ADDR="$poll_host:$port"
HEALTH="http://$ADDR/api/health"

if [ "$DRY_RUN" = 1 ]; then
  echo "run-local: DRY RUN — the commands a real run would execute, none of them run"
  echo "run-local: (dry run) a real run refuses here when a container named $CONTAINER exists (run \`--stop\` first)"
elif docker container inspect "$CONTAINER" >/dev/null 2>&1; then
  die "a container named $CONTAINER already exists; run \`bash deploy/run-local.sh --stop\` first (the volume $VOLUME and its games stay)"
fi

# ---- the build --------------------------------------------------------------
echo "run-local: the image is linux/amd64 by design; on Apple silicon a first build runs under emulation and takes minutes"
run cd "$ROOT"
run docker build --platform linux/amd64 -f deploy/Dockerfile -t "$IMAGE" .

# ---- the run ----------------------------------------------------------------
run docker run -d --name "$CONTAINER" -p "$PUBLISH" -v "$VOLUME:/data" \
  -e YOKOHAMA_WEB_IDENTITY=open -e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on \
  --restart unless-stopped \
  --cap-drop ALL --security-opt no-new-privileges:true --memory 512m "$IMAGE"

opened() {
  echo "run-local: open http://$ADDR/"
  echo "run-local: this is the open door — anyone who can reach this port may use any name; it is bound to $bind (deploy/README.md §2 for a LAN)"
}

# ---- the start, read and not assumed ----------------------------------------
if [ "$DRY_RUN" = 1 ]; then
  echo "run-local: (dry run) a real run then polls this every second for up to 60 s:"
  show curl -s --max-time 5 -o /dev/null -w '%{http_code}' "$HEALTH"
  echo "run-local: (dry run) on a 200 it prints these two lines and exits 0:"
  opened
  echo "run-local: (dry run) otherwise it prints the last status read and the container's last 20 log lines, and exits 1:"
  show docker logs --tail 20 "$CONTAINER"
  echo "run-local: DRY RUN complete — nothing was built, started or stopped"
  exit 0
fi

status="000"
deadline=$((SECONDS + 60))
while :; do
  # A refused connection, or no answer within five seconds, reads 000.
  status="$(curl -s --max-time 5 -o /dev/null -w '%{http_code}' "$HEALTH" || true)"
  [ "$status" = 200 ] && break
  [ "$SECONDS" -lt "$deadline" ] || break
  sleep 1
done

if [ "$status" = 200 ]; then
  opened
  exit 0
fi

echo "run-local: no 200 from $HEALTH within 60 s; the last status read was $status" >&2
if [ "$status" = 421 ]; then
  echo "run-local: 421 is the Host check: this script lists no host name, so the server answers loopback names alone (deploy/README.md §4)" >&2
fi
echo "run-local: the container's last 20 log lines:" >&2
docker logs --tail 20 "$CONTAINER" >&2 || true
echo "run-local: $CONTAINER is still there for \`docker logs\`; \`bash deploy/run-local.sh --stop\` removes it" >&2
exit 1
