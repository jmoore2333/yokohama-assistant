// The paste-in-the-console debugging harness (design doc §0, ruling 2).
//
// The extension is the real vehicle. This snippet exists so a `gamedatas`
// snapshot can be banked from a table with NO extension installed, and so that
// when the extension misbehaves you can tell whether the page or the extension
// is at fault.
//
// HOW TO USE
//   1. Open the BGA table.
//   2. DevTools -> Console -> switch the frame selector (top-left of the
//      console) from `top` to the game frame (`/N/yokohama?table=...`).
//      This matters: `gameui` lives in the game IFRAME, not the top page.
//   3. Paste this whole file and press Enter. A `.ndjson.json` file downloads.
//   4. Append it to the table's capture file:
//        cat ~/Downloads/895608640-snapshot-*.ndjson.json >> bridge-captures/895608640.ndjson
//
// WHAT IT CANNOT DO, and why the extension exists: it cannot fetch the
// notification history. That endpoint is session-token guarded and a bare
// page-context fetch of it returns "Invalid session information" (verified
// live, design §1). The page's own machinery can fetch it; the extension
// copies the response on its way past. A pasted snippet has no such hook.
//
// It also does not POST to the relay. The relay grants CORS to
// `chrome-extension://` origins only, and this snippet runs as the BGA page,
// so it saves a file instead. That file is ONE line in the relay's exact
// format, which is what makes the `cat` above a valid recovery.

(() => {
  const gamedatas = window.gameui && window.gameui.gamedatas;
  if (!gamedatas) {
    console.error(
      "No `gameui.gamedatas` in this frame. Switch the console's frame selector to the " +
        "game iframe (the entry ending in `yokohama?table=...`) and paste again."
    );
    return;
  }

  const table =
    new URLSearchParams(window.location.search).get("table") ||
    String(window.gameui.table_id || "unknown");

  const line =
    JSON.stringify({
      received_at_ms: Date.now(),
      kind: "snapshot",
      payload: {
        captured_at_ms: Date.now(),
        href: window.location.href,
        gamedatas: JSON.parse(JSON.stringify(gamedatas)),
      },
    }) + "\n";

  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const filename = `${table}-snapshot-${stamp}.ndjson.json`;
  const url = URL.createObjectURL(new Blob([line], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30000);

  console.log(
    `[yokohama-bridge] saved ${filename} (${line.length} bytes) for table ${table}. ` +
      `Top-level gamedatas keys: ${Object.keys(gamedatas).join(", ")}`
  );
})();
