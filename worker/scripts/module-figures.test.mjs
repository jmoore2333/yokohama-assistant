import { gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { moduleFigures } from "./module-figures.mjs";

// **The module's figures, read off its own sections** (R1's cleanup debt 6;
// spec §2.7): the one reading the root `Cargo.toml`'s `release-wasm` comment
// records for the Worker's module. A hand-built module, so the cells need no
// build: a memory of 18 pages, and two active data segments — the second at
// offset 11, whose constant's one byte is 0x0b, the byte that also ends a
// constant expression, which a reader that scans for `end` misreads.
const MODULE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // \0asm, version 1
  0x05, 0x03, 0x01, 0x00, 0x12, // memory: one, no maximum, 18 pages
  0x0b, 0x11, 0x02, // data: 17 bytes, two segments
  0x00, 0x41, 0x80, 0x08, 0x0b, 0x03, 0x61, 0x62, 0x63, // active, i32.const 1024, "abc"
  0x00, 0x41, 0x0b, 0x0b, 0x02, 0x64, 0x65, // active, i32.const 11, "de"
]);

describe("moduleFigures", () => {
  it("reads the module's size, its gzip -9 size, its memory's pages and its data", () => {
    expect(WebAssembly.validate(MODULE), "premise: the hand-built module is a module").toBe(true);
    // The gzip figure is the whole module's at level 9, computed HERE rather
    // than read back from the script.
    const gzip9 = gzipSync(MODULE, { level: 9 }).length;
    expect(moduleFigures(MODULE)).toEqual({
      bytes: 32,
      gzip9,
      pages: 18,
      segments: 2,
      dataBytes: 5,
      dataSection: 17,
    });
  });
  it("refuses bytes that are no module, naming the magic, and a module with no memory", () => {
    expect(() => moduleFigures(new Uint8Array([0x7f, 0x45, 0x4c, 0x46]))).toThrow("\\0asm");
    expect(() => moduleFigures(MODULE.slice(0, 8))).toThrow("the module declares no memory");
  });
});
