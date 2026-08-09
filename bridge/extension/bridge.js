// bridge.js - runs in the ISOLATED world at document_start, in every frame of
// boardgamearena.com.
//
// It is a courier and nothing else. `main-world.js` can see the page's objects
// but not `chrome.*`; this file can see `chrome.*` but not the page's objects.
// Everything the extension does crosses between them here.
//
//   main-world.js  --postMessage-->  bridge.js  --runtime--> background.js
//   background.js  --tabs.sendMessage--> bridge.js --postMessage--> main-world.js
//
// It also owns the download fallback, because saving a file needs a document
// and a service worker does not have one.

(() => {
  "use strict";

  const MARKER = "yokohama-bridge/v1";

  // Captures travel up to the service worker. A rejected send is normal and
  // expected in one case - the extension was reloaded while this page stayed
  // open, which invalidates the context until the page reloads - so it is
  // logged rather than thrown.
  const forward = async (message) => {
    try {
      await chrome.runtime.sendMessage(message);
    } catch (e) {
      console.warn(
        "[yokohama-bridge] could not reach the extension service worker (reload the tab if you " +
          "just reloaded the extension):",
        e
      );
    }
  };

  window.addEventListener("message", (event) => {
    // Same-window only, and only our own marker: BGA posts between its own
    // frames and none of that is ours.
    if (event.source !== window) return;
    const data = event.data;
    if (!data || data.marker !== MARKER) return;
    // Messages carrying `request` are ours going the OTHER way (background ->
    // main world); ignore them so nothing loops.
    if (!data.kind) return;

    forward({
      type: "capture",
      table: data.table || "unknown",
      kind: data.kind,
      payload: data.payload,
    });
  });

  const download = (filename, text) => {
    try {
      const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = filename;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      // Revoked late so the download has certainly started.
      setTimeout(() => URL.revokeObjectURL(url), 30_000);
      return true;
    } catch (e) {
      console.error("[yokohama-bridge] download fallback failed:", e);
      return false;
    }
  };

  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (!message || typeof message.type !== "string") return undefined;

    if (message.type === "snapshot-request") {
      // Broadcast into this frame's page world. Only the frame that actually
      // has `gameui.gamedatas` will answer, so this is sent to every frame and
      // the game iframe self-identifies.
      window.postMessage({ marker: MARKER, request: "snapshot" }, "/");
      sendResponse({ asked: true });
      return undefined;
    }

    if (message.type === "download") {
      sendResponse({ ok: download(message.filename, message.text) });
      return undefined;
    }

    return undefined;
  });
})();
