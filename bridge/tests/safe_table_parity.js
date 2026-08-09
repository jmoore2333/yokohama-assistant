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
//
// The worker is loaded into `mv3_stub.js` purely so we can reach in for the
// `safeTable` function; nothing here exercises delivery.

const path = require("path");
const { loadWorker, reporter } = require("./mv3_stub");

const target = process.argv[2] || path.join(__dirname, "..", "extension", "background.js");
const { context } = loadWorker({ target });

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

const { check, done } = reporter(`safeTable / safe_table_id parity\n  target: ${target}\n`);

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

done();
