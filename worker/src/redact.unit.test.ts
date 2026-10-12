import { describe, expect, it } from "vitest";
import { callerDigest, callerTag, DIGEST_HEX, redactKey } from "./redact";

/** SHA-256 in hex, computed HERE, so no cell checks the module's digest against its own. */
async function sha(text: string): Promise<string> {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
  return [...d].map((b) => b.toString(16).padStart(2, "0")).join("");
}

describe("redact.ts (security S2b, decisions D-3, D-4, D-6)", () => {
  it("tags a caller by the full digest and names them by its first twelve hex", async () => {
    expect(await callerTag("red@wire.test")).toBe(`u:${await sha("red@wire.test")}`);
    expect(await callerDigest("red@wire.test")).toBe((await sha("red@wire.test")).slice(0, 12));
    expect(DIGEST_HEX).toBe(12);
    expect((await callerTag("a".repeat(254))).length, "a tag is at most 256 characters, whatever the address").toBe(66);
  });

  // Mutants: the regex anchored at the start only (a `test/` root's key kept its readable half); the digest group
  // widened (`[0-9a-f]+` swallowing a readable half that is itself hex); the replacement dropping the slash.
  it("drops the readable half of a browser segment and leaves every other key as it is", () => {
    expect(redactKey("browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.json")).toBe("browser/c91ad2a917f7/rev7/over/wire.json");
    expect(redactKey("test/browser/red_at_wire.test-c91ad2a917f7/rev7/abandoned/wire.table.json")).toBe("test/browser/c91ad2a917f7/rev7/abandoned/wire.table.json");
    expect(redactKey("browser/deadbeef0123-456789abcdef/rev7/over/x.json"), "a readable half that is hex is still the readable half").toBe("browser/456789abcdef/rev7/over/x.json");
    expect(redactKey("rev7/over/sim-0123456789abcdef.json"), "a server game's key names no caller").toBe("rev7/over/sim-0123456789abcdef.json");
    expect(redactKey("browser/no-digest-here/rev7/over/x.json"), "a legacy folder with no digest is left whole: nothing files under one since S1c").toBe("browser/no-digest-here/rev7/over/x.json");
  });
});
