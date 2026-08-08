// Regression test for the six-lens lens-6 finding: `safeTable` in background.js
// must be "in step" with the relay's `capture::safe_table_id`, as its own
// comment claims. The relay's rule is the `[A-Za-z0-9._-]` allowlist PLUS a
// leading-alphanumeric requirement; an id that satisfies only the allowlist
// (leading `-`/`.`/`_`) is emitted by the extension, refused 400 by the relay,
// and then DROPPED from the outbox silently. So every id the relay refuses must
// map here to one the relay ACCEPTS (leading-alphanumeric).
//
//   node bridge/tests/safe_table_parity.js [path-to-background.js]
//
// Exits 0 on pass, 1 on failure. Plain `node`, no dependencies.

const fs = require("fs");
const path = require("path");
const vm = require("vm");

const target =
  process.argv[2] || path.join(__dirname, "..", "extension", "background.js");
const source = fs.readFileSync(target, "utf8");

// A stub chrome just complete enough for background.js to LOAD (it registers
// listeners at top level); we only reach in for the `safeTable` function.
const context = {
  chrome: {
    storage: { local: { get: async () => ({}), set: async () => {} } },
    runtime: {
      onMessage: { addListener: () => {} },
      onStartup: { addListener: () => {} },
      onInstalled: { addListener: () => {} },
    },
    action: {
      setBadgeBackgroundColor: async () => {},
      setBadgeText: async () => {},
      onClicked: { addListener: () => {} },
    },
    tabs: { sendMessage: async () => {} },
  },
  fetch: () => Promise.reject(new Error("relay down")),
  console: { ...console, error: () => {}, warn: () => {}, info: () => {} },
  setTimeout,
  clearTimeout,
  AbortController,
  Promise,
  JSON,
  String,
  Object,
  Date,
  RegExp,
};
context.globalThis = context;
vm.createContext(context);
vm.runInContext(source, context, { filename: target });

const safeTable = context.safeTable;
if (typeof safeTable !== "function") {
  console.error("could not reach safeTable in background.js — did it stop being a function declaration?");
  process.exit(1);
}

// Exactly the relay's own hostile list (capture.rs
// `table_ids_that_could_escape_the_capture_directory_are_refused`): the relay
// 400s every one of these, so safeTable must NOT hand one back verbatim.
const relayRefuses = [
  "-rf", "-", ".ssh", ".bashrc", "_private", "--dir",
  "..", ".", "", "../secrets", "a/b", "a b",
];
const relayAcceptsLeading = /^[A-Za-z0-9]/;

let failures = 0;
function check(name, ok, detail) {
  console.log(`  [${ok ? "PASS" : "FAIL"}] ${name}${detail ? "  - " + detail : ""}`);
  if (!ok) failures++;
}

console.log("safeTable / safe_table_id parity");
console.log("  target:", target);
for (const id of relayRefuses) {
  const out = safeTable(id);
  const ok = relayAcceptsLeading.test(out);
  check(
    `an id the relay refuses maps to one it accepts: ${JSON.stringify(id)} -> ${JSON.stringify(out)}`,
    ok,
    ok ? "leading-alphanumeric" : "would be 400'd and DROPPED by the relay"
  );
}
// A real (numeric) BGA table id survives verbatim.
check("a numeric table id survives verbatim", safeTable("895724349") === "895724349");
// The documented fallback is itself relay-acceptable.
check("the 'unknown' fallback is relay-acceptable", relayAcceptsLeading.test(safeTable("")));

console.log(failures ? `\nFAIL - ${failures} failing check(s)` : "\nPASS - 0 failing check(s)");
process.exit(failures ? 1 : 0);
