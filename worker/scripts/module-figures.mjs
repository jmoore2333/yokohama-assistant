// **The Worker module's figures, read off its own sections** (R1's cleanup
// debt 6; spec §2.7): what the root `Cargo.toml`'s `release-wasm` comment
// records for this module, re-read by one command so the reading never
// becomes a sentence nobody can check (M6.5b2's false sentence 6). Run after
// `npm run build:module`, the release build:
//
//   node scripts/module-figures.mjs gen/yokohama_worker_bg.wasm
//
// It prints the module's bytes and its gzip size at level 9 (Node's `gzipSync`,
// whose header names no file: 24 B less than the `gzip -9` command's), the pages its linear
// memory starts at — which every instance pays, a table object's second
// instance for one house copy (`copyOf`, `src/table.ts`) among them — and its
// data: the segments, their bytes, and the data section's encoded size.

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { gzipSync } from "node:zlib";

/** The figures of the module `input` holds (WebAssembly core spec, binary
 *  format §5.5): its size and its gzip size at level 9 (`gzipSync`, not the
 *  `gzip -9` command: its header is 24 B smaller), its one memory's initial
 *  pages, and its data segments' count and bytes and the data section's
 *  size. */
export function moduleFigures(input) {
  const b = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (b.length < 8 || b[0] !== 0x00 || b[1] !== 0x61 || b[2] !== 0x73 || b[3] !== 0x6d) {
    throw new Error("not a WebAssembly module: it does not open with \\0asm");
  }
  let at = 8;
  // A LEB128 `u32`, read as a JavaScript number with NO five-byte bound on the
  // shift: a module this script reads is one `wasm-bindgen` and `wasm-opt`
  // just wrote, so a malformed run is refused only by running off the end.
  const u32 = () => {
    let value = 0;
    let shift = 0;
    let byte;
    do {
      if (at >= b.length) throw new Error("a LEB128 number runs past the module's end");
      byte = b[at++];
      value += (byte & 0x7f) * 2 ** shift;
      shift += 7;
    } while (byte & 0x80);
    return value;
  };
  // An active segment's offset: ONE constant and `end`. Its immediate is
  // skipped by its LEB128 continuation bits, never by looking for 0x0b, which
  // an immediate may hold (offset 11 is the byte 0x0b).
  const offset = () => {
    const op = b[at++];
    if (op !== 0x41 && op !== 0x42) {
      throw new Error(`a data segment's offset is not an i32 or i64 constant (opcode 0x${op.toString(16)})`);
    }
    while (b[at++] & 0x80);
    if (b[at++] !== 0x0b) throw new Error("a data segment's offset is not a single constant");
  };
  let pages = null;
  let segments = 0;
  let dataBytes = 0;
  let dataSection = 0;
  while (at < b.length) {
    const id = b[at++];
    const size = u32();
    // `end` is never checked against the module's length. A truncated section
    // other than the memory section, or a data segment cut short, ends the walk
    // silently and figures are still printed (`segments 0`, or a `dataBytes`
    // larger than the bytes present): a limit of this reading script, recorded.
    const end = at + size;
    if (id === 5) {
      if (u32() !== 1) throw new Error("the module declares more than one memory");
      const flags = u32();
      pages = u32();
      if (flags & 1) u32();
    } else if (id === 11) {
      dataSection = size;
      segments = u32();
      for (let i = 0; i < segments; i++) {
        const mode = u32();
        if (mode === 2) u32();
        if (mode !== 1) offset();
        const length = u32();
        dataBytes += length;
        at += length;
      }
    }
    at = end;
  }
  if (pages === null) throw new Error("the module declares no memory");
  return { bytes: b.length, gzip9: gzipSync(b, { level: 9 }).length, pages, segments, dataBytes, dataSection };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv.length !== 3) {
    console.error("usage: node scripts/module-figures.mjs <module.wasm>");
    process.exit(2);
  }
  console.log(JSON.stringify(moduleFigures(readFileSync(process.argv[2]))));
}
