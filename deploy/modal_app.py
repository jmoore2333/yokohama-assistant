"""Run the Yokohama server on Modal.

**The compute layer only.** Authentication lives in front of this — Cloudflare
Access on the public hostname, and a Cloudflare Worker that forwards here
holding a Modal proxy token. Since M6.1 the server does READ an identity
(``YOKOHAMA_WEB_IDENTITY``, ``worker`` on this path), but it verifies nothing:
it trusts the header the Worker sets, which is why the Worker strips any the
client sent. This file's job is to run the same image the NAS runs, to name the
identity source, and to make the game store durable.

Deploy:

    modal deploy deploy/modal_app.py

The endpoint it prints is protected by ``requires_proxy_auth=True``: Modal
rejects a request without valid ``Modal-Key`` / ``Modal-Secret`` headers **at
its proxy, before a container starts**. That is what stops the public
``*.modal.run`` URL from being a way around Cloudflare Access — without it, the
Access policy would be decorative, because anyone with the endpoint URL could
skip it entirely. Create the token pair with::

    modal workspace proxy-tokens create

and put both halves in the Worker's secrets, never in this file.
"""

import os
import subprocess
import threading
import time

import modal

APP_NAME = "yokohama-web"
PORT = 8080

# The store. One JSON file per game, written after every applied move.
VOLUME = modal.Volume.from_name("yokohama-games", create_if_missing=True)
MOUNT = "/data"

# The identity source, and it is REQUIRED: the server has no default mode and
# exits 1 without this variable, naming the accepted values. ``worker`` is the
# right one HERE and ``claimed`` is the right one on the NAS, because the value
# names whatever is actually authenticating in front — on this path that is the
# Cloudflare Worker in ``deploy/worker/``, which reads the Access identity, sets
# ``X-Yokohama-User``, and strips any the client sent. Set it to ``cf-access``
# here and the server would wait for a header this path never sends, so every
# request would be a 401.
IDENTITY_MODE = "worker"

# The SAME Dockerfile the NAS deployment uses. Building the image from it here
# rather than describing the environment twice is the whole reason the two
# targets cannot drift. The Dockerfile deliberately sets no identity mode — it
# cannot know what is in front of it — so the two deployments declare it at the
# level that does: this image for Modal, the container's create body for the NAS
# (``RUNBOOK.md`` §3).
IMAGE = modal.Image.from_dockerfile("deploy/Dockerfile", context_dir=".").env(
    {"YOKOHAMA_WEB_DIR": f"{MOUNT}/games", "YOKOHAMA_WEB_IDENTITY": IDENTITY_MODE}
)

app = modal.App(APP_NAME, image=IMAGE)

# How often to flush the Volume. Modal commits in the background on its own
# schedule; this makes the interval a number we chose rather than one we
# inherited, because the durability claim in the README ("a container that
# sleeps loses no acknowledged move") is only as true as the last commit.
COMMIT_SECONDS = 20


def _commit_loop() -> None:
    """Flush the Volume on a timer, forever.

    **Graceful shutdown is necessary and NOT sufficient here, and this is the
    half that is easy to miss.** The server drains in-flight requests on
    SIGTERM, so a move it answered 200 for has reached the container's
    filesystem. On Modal that filesystem is a Volume whose changes become
    durable at a commit — so without this, a container that scaled to zero
    between background commits would lose writes the client was told had
    landed. Draining the request and persisting the write are two different
    guarantees and the deployment owes both.
    """
    while True:
        time.sleep(COMMIT_SECONDS)
        try:
            VOLUME.commit()
        except Exception as e:  # noqa: BLE001 - a failed flush must not kill the server
            print(f"volume commit failed, will retry in {COMMIT_SECONDS}s: {e}", flush=True)


@app.function(
    volumes={MOUNT: VOLUME},
    # **Exactly one container, and this is a correctness constraint rather than
    # a cost one.** Modal's own Volume guidance is that any particular file
    # should be written by a single container at a time; concurrent writes to
    # one file are last-write-wins. The store is one JSON file per game, and the
    # server's compare-and-swap DETECTS a conflicting write rather than
    # preventing it — nothing serialises read-then-write across processes. Two
    # containers would therefore be two writers to the same game file, which is
    # precisely the shape that guard cannot fix.
    max_containers=1,
    # A game is a long-lived thing and a rebuild from the record is cheap but
    # not free. Stay warm for a while after the last request rather than paying
    # a cold start on every move after a pause for tea.
    scaledown_window=15 * 60,
    timeout=60 * 60,
)
@modal.web_server(
    port=PORT,
    startup_timeout=120,
    # See the module docstring. Without this the endpoint is world-reachable and
    # Cloudflare Access in front of it protects nothing.
    requires_proxy_auth=True,
)
def serve() -> None:
    # `YOKOHAMA_WEB_IDENTITY` is NOT set here on purpose: it rides in from the
    # image env above and through this copy of `os.environ`. Declared once, in
    # one place — a second assignment here would be a second place for the two
    # to disagree, and the server refuses to start rather than guess.
    env = dict(os.environ)
    env["YOKOHAMA_WEB_HOST"] = "0.0.0.0"
    env["YOKOHAMA_WEB_PORT"] = str(PORT)
    env["YOKOHAMA_WEB_DIR"] = f"{MOUNT}/games"

    os.makedirs(env["YOKOHAMA_WEB_DIR"], exist_ok=True)

    threading.Thread(target=_commit_loop, daemon=True).start()

    # Exec'd directly, not through a shell, so the server is the child that
    # receives the signal when Modal stops the container and its
    # `with_graceful_shutdown` actually runs.
    subprocess.Popen(["/usr/local/bin/yokohama-web"], env=env)
