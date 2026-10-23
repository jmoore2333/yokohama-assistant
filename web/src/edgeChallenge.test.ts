// The edge's challenge predicate, the one copy the page and the service worker both import (security S3, W-3).

import { describe, expect, it } from "vitest";

import { isEdgeChallengeResponse } from "./edgeChallenge";

describe("the edge's challenge, as an answer shows it", () => {
  it("is a 403 carrying cf-mitigated: challenge, and never one of the two alone", () => {
    expect(isEdgeChallengeResponse(403, "challenge")).toBe(true);
    // A 403 is also this API's own refusal of a locked table or a stranger's seat.
    expect(isEdgeChallengeResponse(403, null)).toBe(false);
    expect(isEdgeChallengeResponse(200, "challenge")).toBe(false);
    expect(isEdgeChallengeResponse(429, "challenge")).toBe(false);
    // The edge's other verdicts are not a challenge a widget can pass.
    expect(isEdgeChallengeResponse(403, "block")).toBe(false);
    expect(isEdgeChallengeResponse(403, "")).toBe(false);
  });

  // **The value is compared exactly, and both callers hand it `Headers.get`'s reading**, which has already
  // folded the header NAME's case and trimmed the value's surrounding whitespace. So a header written
  // `CF-Mitigated: challenge ` is a challenge, while a value in another case is not, as it was not before.
  it("reads the header as Headers.get gives it: the name in any case, the value trimmed, its case kept", () => {
    const read = (name: string, value: string) => new Headers({ [name]: value }).get("cf-mitigated");
    expect(isEdgeChallengeResponse(403, read("CF-Mitigated", "challenge"))).toBe(true);
    expect(isEdgeChallengeResponse(403, read("cf-mitigated", " challenge\t"))).toBe(true);
    expect(isEdgeChallengeResponse(403, read("cf-mitigated", "Challenge"))).toBe(false);
  });
});
