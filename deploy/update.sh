#!/usr/bin/env bash
# **Update a self-hosted Yokohama container to a release tag** (cycle F, R-F12;
# `deploy/README.md` §9). From a checkout of this repository:
#
#     YOKOHAMA_WEB_BUILD_ENV=<file> YOKOHAMA_WEB_ENV=<file> bash deploy/update.sh [--dry-run] <tag>
#
# In order, and it stops at the first failure — so a build or a smoke that
# fails leaves the container that is serving exactly as it was:
#
#   1. refuse a working tree with uncommitted changes (the image is built from
#      the tree, so an edit would ship under the tag's name);
#   2. `git fetch --tags` and `git checkout --detach refs/tags/<tag>`;
#   3. `docker build --platform linux/amd64 -f deploy/Dockerfile`, with the
#      build values read from the file `YOKOHAMA_WEB_BUILD_ENV` names, tagged
#      `yokohama-web:<tag>`;
#   4. `deploy/smoke-image.sh` on that image (no door, `cf-access`, `open`),
#      while the old container still serves;
#   5. only then the swap: `docker stop`, `docker rm`, `docker run` with
#      `--env-file "$YOKOHAMA_WEB_ENV"` and the named volume;
#   6. read the start instead of assuming it: poll the new container for up
#      to 30 s, every 2 s, until `docker inspect` reads it running, not
#      restarting and never restarted AND `docker logs` holds the server's
#      `yokohama-web listening on` line. Otherwise print its last log lines
#      and the rollback line naming the previous image, and exit 1.
#
# **The two files** are `NAME=value` lines, a value taken literally to the end
# of its line: no quotes, no shell expansion. Blank lines and lines beginning
# `#` are skipped. `YOKOHAMA_WEB_BUILD_ENV` may name only the six build values
# (`YOKOHAMA_PROJECT_URL`, `YOKOHAMA_SITE_BLURB`, `YOKOHAMA_SITE_NOTICE`,
# `YOKOHAMA_RELEASE_FEED`, `YOKOHAMA_TURNSTILE_SITEKEY`, `YOKOHAMA_SITE_URL`);
# an empty file builds with none. **It builds KEYLESS images only:** a
# non-empty `YOKOHAMA_TURNSTILE_SITEKEY` is refused up front, before the fetch,
# because step 4's smoke starts the image under `cf-access` and a keyed build
# refuses to start there (a keyed build is the open door's, R-FP4). A keyed
# image is a manual build for an open-door host. `YOKOHAMA_WEB_ENV` is the container's
# environment, `YOKOHAMA_WEB_IDENTITY` first among them: the server refuses to
# start without it.
#
# **Optional**, each with the guide's §4 default: `YOKOHAMA_WEB_CONTAINER`
# (`yokohama-web`), `YOKOHAMA_WEB_VOLUME` (`yokohama-web-data`),
# `YOKOHAMA_WEB_PUBLISH` (`127.0.0.1:8080:8080`; set it EMPTY to publish no
# port) and `YOKOHAMA_WEB_NETWORK` (Docker's default network).
#
# **`--dry-run` prints every command and runs none**: no fetch, no checkout, no
# build, no container change, not even a `git status` or a `docker inspect`.
# It is a preview of the exact lines a real run would execute, and a file it
# cannot read is a placeholder in the preview, where a real run would refuse. This script never removes a file
# or a directory and never touches the volume: the swap is `docker rm` of the
# stopped container alone, so `/data` and every earlier image stay for a
# rollback.
set -euo pipefail

# **The whole script is one function, called on the last line.** `git checkout`
# below may replace this very file with the tag's own copy while bash is still
# reading it, and bash reads a script as it goes; a function is parsed whole
# before it runs, and `exit` stops the shell before it reads anything after
# the call.
main() {
  die() { echo "update: $*" >&2; exit 2; }

  # ---- the arguments --------------------------------------------------------
  DRY_RUN=0
  TAG=""
  for arg in "$@"; do
    case "$arg" in
      --dry-run) DRY_RUN=1 ;;
      -*) die "unknown option $arg (usage: deploy/update.sh [--dry-run] <tag>)" ;;
      *) [ -z "$TAG" ] || die "one release tag only — got \`$TAG\` and \`$arg\`"; TAG="$arg" ;;
    esac
  done
  TAG=${TAG:?a release tag, e.g. v0.6.0 (usage: deploy/update.sh [--dry-run] <tag>)}
  # The tag is also the image's tag, so it must be a valid one; a slash would not be.
  case "$TAG" in
    *[!A-Za-z0-9_.-]* | [.-]*) die "\`$TAG\` is not a usable image tag: letters, digits, \`_\`, \`.\` and \`-\`, not starting with \`.\` or \`-\`" ;;
  esac

  IMAGE="yokohama-web:$TAG"
  CONTAINER="${YOKOHAMA_WEB_CONTAINER:-yokohama-web}"
  VOLUME="${YOKOHAMA_WEB_VOLUME:-yokohama-web-data}"
  PUBLISH="${YOKOHAMA_WEB_PUBLISH-127.0.0.1:8080:8080}"
  NETWORK="${YOKOHAMA_WEB_NETWORK:-}"

  # ---- the two files --------------------------------------------------------
  # A real run refuses an unset or unreadable file before it runs anything. A dry
  # run keeps going with a placeholder, so the preview still shows every line.
  BUILD_ENV="${YOKOHAMA_WEB_BUILD_ENV:-}"
  RUN_ENV="${YOKOHAMA_WEB_ENV:-}"
  build_args=()
  build_args_known=1
  if [ -z "$BUILD_ENV" ] || [ ! -r "$BUILD_ENV" ]; then
    if [ "$DRY_RUN" = 1 ]; then
      echo "update: (dry run) YOKOHAMA_WEB_BUILD_ENV is unset or unreadable; a real run would refuse" >&2
      BUILD_ENV="${BUILD_ENV:-<the file YOKOHAMA_WEB_BUILD_ENV names>}"
      build_args_known=0
    else
      die "name the file of build values in YOKOHAMA_WEB_BUILD_ENV (an empty file builds with none)"
    fi
  fi
  if [ -z "$RUN_ENV" ] || [ ! -r "$RUN_ENV" ]; then
    if [ "$DRY_RUN" = 1 ]; then
      echo "update: (dry run) YOKOHAMA_WEB_ENV is unset or unreadable; a real run would refuse" >&2
      RUN_ENV="${RUN_ENV:-<the file YOKOHAMA_WEB_ENV names>}"
    else
      die "name the container's environment file in YOKOHAMA_WEB_ENV (it must set YOKOHAMA_WEB_IDENTITY)"
    fi
  fi
  if [ "$build_args_known" = 1 ]; then
    while IFS= read -r line || [ -n "$line" ]; do
      case "$line" in
        "" | "#"*) continue ;;
        YOKOHAMA_TURNSTILE_SITEKEY=?*)
          die "$BUILD_ENV sets YOKOHAMA_TURNSTILE_SITEKEY, and this script builds keyless images only: its smoke starts the image under cf-access, which refuses to start a keyed build (a keyed build is the open door's). A keyed image is a manual build for an open-door host (deploy/README.md §3); remove the line, or leave its value empty." ;;
        YOKOHAMA_PROJECT_URL=* | YOKOHAMA_SITE_BLURB=* | YOKOHAMA_SITE_NOTICE=* \
          | YOKOHAMA_RELEASE_FEED=* | YOKOHAMA_TURNSTILE_SITEKEY=* | YOKOHAMA_SITE_URL=*)
          build_args+=(--build-arg "$line") ;;
        *) die "$BUILD_ENV: \`${line%%=*}\` is not one of the six build values (see deploy/README.md §3)" ;;
      esac
    done < "$BUILD_ENV"
  fi
  if [ "$DRY_RUN" = 0 ]; then
    for tool in git docker curl python3; do
      command -v "$tool" >/dev/null 2>&1 || die "\`$tool\` is not on PATH (the swap needs git and docker; the smoke needs curl and python3)"
    done
  fi

  # ---- run: print the line, then run it unless this is a dry run ------------
  run() {
    printf '+'
    for a in "$@"; do
      case "$a" in
        "<"*">") printf ' %s' "$a" ;;   # a dry run's placeholder, never a real argument
        "{{"*) printf " '%s'" "$a" ;;   # a Go template: shown in single quotes, as typed
        *) printf ' %q' "$a" ;;
      esac
    done
    printf '\n'
    if [ "$DRY_RUN" = 1 ]; then return 0; fi
    "$@"
  }

  if [ "$DRY_RUN" = 1 ]; then
    echo "update: DRY RUN — the commands a real run would execute, none of them run"
  fi

  # 1. the working tree must be the tag, and nothing else
  if [ "$DRY_RUN" = 1 ]; then
    echo "update: (dry run) a real run refuses here when \`git status --porcelain --untracked-files=no\` lists a change"
  elif [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    die "the working tree has uncommitted changes; the image would ship them under $TAG's name — commit, stash or discard them first"
  fi

  # 2. the tag
  run git fetch --tags
  run git checkout --detach "refs/tags/$TAG"

  # 3. build, with the build values from the file
  if [ "$build_args_known" = 1 ]; then
    run docker build --platform linux/amd64 -f deploy/Dockerfile ${build_args[@]+"${build_args[@]}"} -t "$IMAGE" .
  else
    run docker build --platform linux/amd64 -f deploy/Dockerfile "<the --build-arg flags, one per line of the file YOKOHAMA_WEB_BUILD_ENV names>" -t "$IMAGE" .
  fi

  # 4. smoke the image while the old container still serves
  run bash deploy/smoke-image.sh "$IMAGE"

  # 5. the swap. The old image is named first, for the rollback line.
  if [ "$DRY_RUN" = 1 ]; then
    echo "update: (dry run) a real run reads the serving container's image here, to name it for a rollback"
    previous=""
  elif previous="$(docker inspect -f '{{.Config.Image}}' "$CONTAINER" 2>/dev/null)"; then
    echo "update: $CONTAINER is running $previous; it is stopped once the swap begins, and the image stays"
  else
    previous=""
    echo "update: no container named $CONTAINER; this run creates it"
  fi
  if [ "$DRY_RUN" = 1 ] || [ -n "$previous" ]; then
    run docker stop "$CONTAINER"
    run docker rm "$CONTAINER"
  fi
  run_args=(-d --name "$CONTAINER")
  if [ -n "$PUBLISH" ]; then run_args+=(-p "$PUBLISH"); fi
  if [ -n "$NETWORK" ]; then run_args+=(--network "$NETWORK"); fi
  run_args+=(-v "$VOLUME:/data" --env-file "$RUN_ENV" --restart unless-stopped "$IMAGE")

  # What an operator does when the new container does not come up. It is a
  # printed line, never a step: the script does not undo its own swap.
  rollback_hint() {
    if [ -n "$previous" ]; then
      echo "update: to roll back, stop and remove $CONTAINER and run it again from $previous with the same arguments (deploy/README.md §9); /data was not touched" >&2
    else
      echo "update: there was no earlier container to return to; /data was not touched" >&2
    fi
  }

  if ! run docker run "${run_args[@]}"; then
    echo "update: \`docker run\` failed, and $CONTAINER is not running" >&2
    rollback_hint
    exit 1
  fi

  # 6. the start, read and not assumed: `docker run -d` returns before the
  # server has read its environment, and a run file the smoke never saw (a TTL
  # with no archive directory, a bad chat value) makes it exit at once — then
  # `--restart unless-stopped` loops it while a script that did not look says
  # all is well.
  start_state='{{.State.Running}} {{.State.Restarting}} {{.RestartCount}}'
  if [ "$DRY_RUN" = 1 ]; then
    echo "update: (dry run) a real run polls these two commands every 2 s for up to 30 s"
    run docker inspect -f "$start_state" "$CONTAINER"
    run docker logs "$CONTAINER"
    echo "update: (dry run) it succeeds only when the state reads \`true false 0\` and the log holds \`yokohama-web listening on\`; otherwise it prints the container's last log lines and the rollback line, and exits 1"
    echo "update: DRY RUN complete — nothing was fetched, built, stopped or started"
    return 0
  fi
  state=""
  logs=""
  started=0
  waited=0
  while [ "$waited" -lt 30 ]; do
    state="$(docker inspect -f "$start_state" "$CONTAINER" 2>&1)" || state="unreadable: $state"
    # Not running, restarting, or already restarted: nothing to wait for.
    [ "$state" = "true false 0" ] || break
    logs="$(docker logs "$CONTAINER" 2>&1 || true)"
    case "$logs" in
      *"yokohama-web listening on"*) started=1; break ;;
    esac
    sleep 2
    waited=$((waited + 2))
  done
  if [ "$started" != 1 ]; then
    echo "update: $CONTAINER did not come up listening (state \`$state\`, after ${waited} s). Its last log lines:" >&2
    docker logs --tail 20 "$CONTAINER" >&2 || true
    echo "update: it is still there for \`docker logs\`; remove it before rolling back" >&2
    rollback_hint
    exit 1
  fi
  echo "update: $CONTAINER is running $IMAGE and listening; /data was not touched"
  if [ -n "$previous" ]; then
    echo "update: to roll back, stop and remove $CONTAINER and run it again from $previous with the same arguments (deploy/README.md §9)"
  fi
}

main "$@"; exit $?
