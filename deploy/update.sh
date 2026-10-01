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
#   2. `git fetch --tags` and `git checkout <tag>`;
#   3. `docker build --platform linux/amd64 -f deploy/Dockerfile`, with the
#      build values read from the file `YOKOHAMA_WEB_BUILD_ENV` names, tagged
#      `yokohama-web:<tag>`;
#   4. `deploy/smoke-image.sh` on that image (no door, `cf-access`, `open`),
#      while the old container still serves;
#   5. only then the swap: `docker stop`, `docker rm`, `docker run` with
#      `--env-file "$YOKOHAMA_WEB_ENV"` and the named volume;
#   6. `docker logs --tail 5`, the server's first lines: "listening on …", or
#      the refusal that names what is wrong.
#
# **The two files** are `NAME=value` lines, a value taken literally to the end
# of its line: no quotes, no shell expansion. Blank lines and lines beginning
# `#` are skipped. `YOKOHAMA_WEB_BUILD_ENV` may name only the six build values
# (`YOKOHAMA_PROJECT_URL`, `YOKOHAMA_SITE_BLURB`, `YOKOHAMA_SITE_NOTICE`,
# `YOKOHAMA_RELEASE_FEED`, `YOKOHAMA_TURNSTILE_SITEKEY`, `YOKOHAMA_SITE_URL`);
# an empty file builds with none. `YOKOHAMA_WEB_ENV` is the container's
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
  run git checkout "$TAG"

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
  run docker run "${run_args[@]}"

  # 6. the server's first lines: "listening on …", or the refusal that names why
  if [ "$DRY_RUN" = 0 ]; then
    sleep 3
  fi
  run docker logs --tail 5 "$CONTAINER"

  if [ "$DRY_RUN" = 1 ]; then
    echo "update: DRY RUN complete — nothing was fetched, built, stopped or started"
  else
    echo "update: $CONTAINER now runs $IMAGE; /data was not touched"
    if [ -n "$previous" ]; then
      echo "update: to roll back, stop and remove $CONTAINER and run it again from $previous with the same arguments (deploy/README.md §9)"
    fi
  fi
}

main "$@"
exit $?
