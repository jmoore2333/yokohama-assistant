// **A page inside another page's frame does not render** (security S2a, finding B4's third gap; ruling A5). The
// host's `X-Frame-Options: DENY` is the fence; this is the client's own refusal, for a browser or a frame that
// did not honour it, and it costs a comparison.
import { describe, expect, it } from "vitest";
import { FRAMED_SENTENCE, framed } from "./framed";

describe("framed", () => {
  it("is false when the window is its own top, true when it is not", () => {
    const top = {};
    expect(framed({ top, self: top })).toBe(false);
    expect(framed({ top, self: {} })).toBe(true);
    expect(framed({ top: null, self: {} })).toBe(true);
  });
  it("has one sentence, which names no address and no other page", () => {
    expect(FRAMED_SENTENCE).toBe("Yokohama does not run inside another page. Open it in a tab of its own.");
  });
});
