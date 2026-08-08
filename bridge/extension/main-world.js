// main-world.js - runs in the PAGE's JavaScript world, at document_start, in
// every frame of boardgamearena.com.
//
// Two jobs, both read-only:
//
//   1. WRAP AND CAPTURE. `fetch` and `XMLHttpRequest` are wrapped so that a
//      response the page *already* asked for can be copied on its way past.
//      This extension issues ZERO requests to BGA of its own.
//
//      Why wrapping instead of fetching the log ourselves: the notification
//      history endpoint is session-token guarded, and a bare page-context fetch
//      of it comes back "Invalid session information" (verified live against
//      table #895608640, design doc §1). The page's own machinery holds the
//      token and succeeds. So rather than reverse-engineer the token, we let
//      the page do the asking and take a copy of the answer. It is also the
//      politer posture: no extra load on BGA, and nothing that looks like a
//      scraper.
//
//   2. SNAPSHOT. Serialize `gameui.gamedatas` on request. That object is a
//      MAIN-world global, so an isolated-world content script cannot see it at
//      all - which is the whole reason this file exists separately from
//      `bridge.js`.
//
// This world has no access to `chrome.*`, so everything leaves through
// `window.postMessage` and `bridge.js` picks it up on the other side.

(() => {
  "use strict";

  // Marker on every message we post, so `bridge.js` can tell our traffic from
  // the page's own (BGA uses postMessage between its frames).
  const MARKER = "yokohama-bridge/v1";

  // Which response URLs are worth copying.
  //
  // Only the first is verified against the live table; it is the complete
  // structured move log and it is what H1/H2 translate. The second is shaped
  // for the same game shard in case this build also polls a shorter sibling
  // endpoint - it costs nothing when it never matches.
  //
  // WIDENING SEAM (H3): if the live-follow feed turns out to arrive by some
  // other path, add the pattern here and nowhere else. Note that BGA's realtime
  // channel is a socket, not fetch/XHR, so a socket feed would need a different
  // mechanism rather than another entry in this list.
  const CAPTURE_PATTERNS = [/notificationhistory/i, /\/yokohama\/yokohama\/notif/i];

  const shouldCapture = (url) => {
    if (typeof url !== "string" || url.length === 0) return false;
    return CAPTURE_PATTERNS.some((pattern) => pattern.test(url));
  };

  // The table this frame belongs to. The game iframe's own query string carries
  // it; `gameui` is the fallback for any frame shape we have not seen.
  const tableId = () => {
    try {
      const fromQuery = new URLSearchParams(window.location.search).get("table");
      if (fromQuery) return String(fromQuery);
      const fromGameui = window.gameui && window.gameui.table_id;
      if (fromGameui) return String(fromGameui);
    } catch (_) {
      // A frame with an opaque origin can throw on location access; fall
      // through to the sentinel rather than lose the capture.
    }
    return "unknown";
  };

  // Hand something to `bridge.js`. Target origin "/" means "only deliver this
  // if the receiving document is same-origin" - it never crosses to another
  // site's frame.
  const emit = (message) => {
    try {
      window.postMessage({ marker: MARKER, ...message }, "/");
    } catch (e) {
      console.warn("[yokohama-bridge] could not post capture:", e);
    }
  };

  // A captured response. `body_text` is the response VERBATIM, as a string,
  // deliberately: re-parsing and re-serializing would reorder object keys and
  // normalize whitespace, and the cancel/supersession analysis this data exists
  // for should read exactly what BGA sent.
  const emitPackets = (fields) =>
    emit({
      kind: "packets",
      table: tableId(),
      payload: { captured_at_ms: Date.now(), ...fields },
    });

  // ---------------------------------------------------------------- fetch ---

  const nativeFetch = window.fetch;
  if (typeof nativeFetch === "function") {
    window.fetch = function (...args) {
      const pending = nativeFetch.apply(this, args);
      let url = "";
      try {
        const [resource] = args;
        url = typeof resource === "string" ? resource : (resource && resource.url) || "";
      } catch (_) {
        url = "";
      }
      if (!shouldCapture(url)) return pending;

      // Everything below must be invisible to the page: the original promise's
      // resolution and rejection both pass straight through, and the body is
      // read from a CLONE so the page's own reader still sees a fresh stream.
      return pending.then((response) => {
        try {
          response
            .clone()
            .text()
            .then((text) =>
              emitPackets({
                source: "fetch",
                url: response.url || url,
                status: response.status,
                body_text: text,
              })
            )
            .catch((e) => console.warn("[yokohama-bridge] fetch body read failed:", e));
        } catch (e) {
          console.warn("[yokohama-bridge] fetch clone failed:", e);
        }
        return response;
      });
    };
  }

  // ------------------------------------------------------------------ XHR ---

  const NativeXHR = window.XMLHttpRequest;
  if (NativeXHR && NativeXHR.prototype) {
    const nativeOpen = NativeXHR.prototype.open;
    const nativeSend = NativeXHR.prototype.send;

    NativeXHR.prototype.open = function (method, url, ...rest) {
      // Stashed under an unlikely name; the page never reads it.
      this.__yokohamaBridgeUrl = typeof url === "string" ? url : String(url || "");
      this.__yokohamaBridgeMethod = method;
      return nativeOpen.call(this, method, url, ...rest);
    };

    NativeXHR.prototype.send = function (...args) {
      try {
        if (shouldCapture(this.__yokohamaBridgeUrl)) {
          this.addEventListener("load", () => {
            try {
              // `responseText` throws unless the response type is text-ish, so
              // it is guarded rather than assumed.
              const textual = this.responseType === "" || this.responseType === "text";
              emitPackets({
                source: "xhr",
                url: this.responseURL || this.__yokohamaBridgeUrl,
                method: this.__yokohamaBridgeMethod,
                status: this.status,
                body_text: textual ? this.responseText : null,
                body_skipped_response_type: textual ? undefined : this.responseType,
              });
            } catch (e) {
              console.warn("[yokohama-bridge] xhr capture failed:", e);
            }
          });
        }
      } catch (e) {
        console.warn("[yokohama-bridge] xhr wrap failed:", e);
      }
      return nativeSend.apply(this, args);
    };
  }

  // ------------------------------------------------------------- snapshot ---

  // A snapshot request is broadcast to every frame; only the one that actually
  // holds `gameui.gamedatas` answers, which is how the game iframe identifies
  // itself without hard-coding the `/4/` shard from the URL.
  const takeSnapshot = () => {
    const gamedatas = window.gameui && window.gameui.gamedatas;
    if (!gamedatas) return false;
    let plain;
    try {
      // Round-tripping through JSON does two things at once: it drops anything
      // non-serializable that would break the structured clone on the way to
      // `bridge.js`, and it gives us a detached copy so a later page mutation
      // cannot edit what we are about to send.
      plain = JSON.parse(JSON.stringify(gamedatas));
    } catch (e) {
      console.warn("[yokohama-bridge] gamedatas is not serializable:", e);
      return false;
    }
    emit({
      kind: "snapshot",
      table: tableId(),
      payload: {
        captured_at_ms: Date.now(),
        href: window.location.href,
        gamedatas: plain,
      },
    });
    return true;
  };

  window.addEventListener("message", (event) => {
    if (event.source !== window) return;
    const data = event.data;
    if (!data || data.marker !== MARKER || data.request !== "snapshot") return;
    takeSnapshot();
  });

  console.debug("[yokohama-bridge] capture wrapper installed in", window.location.href);
})();
