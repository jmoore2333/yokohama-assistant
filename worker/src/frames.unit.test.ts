import { describe, expect, it } from "vitest";
import {
  CLOSE_TOO_MANY,
  closeReason,
  END_DELETED,
  END_GONE,
  endAtAdmission,
  frame,
  sentenceOf,
  TOO_MANY_LOBBY,
  TOO_MANY_OWN_LOBBY,
  TOO_MANY_OWN_TABLE,
  TOO_MANY_TABLE,
} from "./frames";

// Spec §2.4, "The framing": every message a table object sends is
// `{"event": <name>, "data": <the SSE frame's data, byte-identical>}`.
describe("the framing (spec §2.4)", () => {
  // Mutant: the data parsed and re-serialized — the spacing goes, and a u64
  // above 2^53 is rounded.
  it("embeds the data raw, byte for byte", () => {
    const data = '{ "events" : 18446744073709551615 }';
    expect(frame("signal", data)).toBe('{"event":"signal","data":{ "events" : 18446744073709551615 }}');
  });

  // The SSE's own `end` data, byte for byte: `Event::default().event("end")`
  // in the `stream` handler of `crates/yokohama-web/src/lib.rs` writes these
  // two strings. Mutant: either reworded, or a sentence added to the one an
  // admission lost mid-stream sends.
  it("carries the SSE's two end frames byte for byte, under the SSE's event names", () => {
    expect(frame("end", END_GONE)).toBe('{"event":"end","data":{"reason":"gone"}}');
    expect(frame("end", END_DELETED)).toBe('{"event":"end","data":{"reason":"deleted"}}');
    expect(frame("house", '{"seq":1,"chat":0}')).toBe('{"event":"house","data":{"seq":1,"chat":0}}');
  });

  // The orchestrator's ruling (Task 8's review): an admission refused at the
  // upgrade has no SSE counterpart, so its `end` carries the layer's sentence
  // for the client to show. Mutant: the sentence dropped, or its key renamed.
  it("an admission refused at the upgrade ends with the layer's sentence", () => {
    expect(endAtAdmission('no game named "x"')).toBe('{"reason":"gone","sentence":"no game named \\"x\\""}');
    expect(endAtAdmission(undefined), "no sentence to carry: the SSE's own").toBe(END_GONE);
  });

  // Mutants: a body's `status` read as the sentence; a body that is not the
  // error shape answered as a sentence (the client would show it).
  it("reads the layer's sentence out of its error body, and nothing out of anything else", () => {
    expect(sentenceOf('{"error":"you are not seated at this table"}')).toBe("you are not seated at this table");
    expect(sentenceOf('{"error":"this table is locked","status":"playing"}')).toBe("this table is locked");
    expect(sentenceOf("not json")).toBeUndefined();
    expect(sentenceOf('{"events":0}')).toBeUndefined();
    expect(sentenceOf('{"error":7}')).toBeUndefined();
    expect(sentenceOf("null")).toBeUndefined();
  });
});

// Pool fact 6: a close reason over 123 UTF-8 bytes makes `close()` THROW and
// leaves the socket open. Every close goes through `closeReason`.
describe("the close reason (pool fact 6)", () => {
  const bytes = (s: string) => new TextEncoder().encode(s).length;

  // Mutant: no cut at all (200 bytes); a cut at 123 CHARACTERS rather than
  // bytes is caught by the multibyte cells below.
  it("cuts a long ASCII sentence to 123 bytes", () => {
    const cut = closeReason("x".repeat(200));
    expect(cut).toBe("x".repeat(123));
    expect(bytes(cut)).toBe(123);
  });

  // Mutants: a cut by bytes that splits the character (a decoder would turn
  // the half into U+FFFD, three bytes — the reason then over the limit); a cut
  // by UTF-16 code units, which can leave half a surrogate pair.
  it("cuts BEFORE a character that straddles byte 123", () => {
    // 122 bytes, then `é` in bytes 123 and 124.
    expect(closeReason("a".repeat(122) + "é" + "tail")).toBe("a".repeat(122));
    // 121 bytes, then `—` in bytes 122 to 124.
    expect(closeReason("a".repeat(121) + "—")).toBe("a".repeat(121));
    // 121 bytes, then an emoji in bytes 122 to 125: two UTF-16 code units.
    expect(closeReason("a".repeat(121) + "😀")).toBe("a".repeat(121));
    // Four-byte characters to the end: 30 fit (120 bytes), the 31st does not.
    expect(closeReason("😀".repeat(40))).toBe("😀".repeat(30));
  });

  // Mutant: the bound made strict (`>=` for `>`) — a reason of exactly 123
  // bytes cut short.
  it("keeps a sentence that fits whole, up to exactly 123 bytes", () => {
    expect(closeReason('no game named "x"')).toBe('no game named "x"');
    const exact = "a".repeat(121) + "é";
    expect(bytes(exact)).toBe(123);
    expect(closeReason(exact)).toBe(exact);
    expect(closeReason("")).toBe("");
  });
});

// Security S2b, finding D3 (decision D-4): a socket refused for its count is closed 4429 after an `end` frame that
// carries one of four sentences. The pool's cap cells (`test/caps.test.ts`) build BOTH sides of each comparison
// from these constants, so a constant moved reads green there: this cell pins them at their LITERALS, as the house
// rule compares a sent frame with literal bytes (`test/sockets.test.ts`). Mutants: `4404` for the code (a deleted
// table's); any sentence reworded.
describe("the socket caps' close (security S2b, D3)", () => {
  it("closes 4429 and carries the four sentences, at their literals", () => {
    expect(CLOSE_TOO_MANY).toBe(4429);
    expect(TOO_MANY_OWN_TABLE).toBe("you have too many connections open to this table — close another tab and try again");
    expect(TOO_MANY_TABLE).toBe("this table has as many connections as it can take — try again later");
    expect(TOO_MANY_OWN_LOBBY).toBe("you have too many connections open to the lobby — close another tab and try again");
    expect(TOO_MANY_LOBBY).toBe("the lobby has as many connections as it can take — try again later");
  });
});
