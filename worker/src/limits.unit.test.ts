import { describe, expect, it } from "vitest";
import { callerTag } from "./redact";
import { IP_HEADER, limiterFor, limitKey, RETRY_AFTER_CREATE, RETRY_AFTER_WRITE, TOO_MANY, UNKNOWN_IP } from "./limits";

describe("limitKey (security S2b, C2; decision D-3)", () => {
  // Mutants: the worker door keyed by the raw address (an `@` in the key); the open door's writes keyed by IP
  // alone; a missing IP throwing or reading `null`.
  it("keys a verified caller by their digest, an open-door claim by IP for a create and IP plus claim for a write, and nobody by IP", async () => {
    const person = { kind: "person", email: "red@wire.test" } as const;
    const claimed = { kind: "claimed", name: "ben" } as const;
    const nobody = { kind: "none" } as const;
    expect(await limitKey("worker", person, "203.0.113.9", "create")).toBe(await callerTag("red@wire.test"));
    expect(await limitKey("worker", person, "203.0.113.9", "write")).toBe(await callerTag("red@wire.test"));
    expect(await limitKey("worker", { kind: "dev", email: "a@e2e.test" }, null, "write")).toBe(await callerTag("a@e2e.test"));
    expect(await limitKey("open", claimed, "203.0.113.9", "create")).toBe("ip:203.0.113.9");
    expect(await limitKey("open", claimed, "203.0.113.9", "write")).toBe(`ip:203.0.113.9|${await callerTag("ben")}`);
    expect(await limitKey("open", claimed, null, "write")).toBe(`ip:${UNKNOWN_IP}|${await callerTag("ben")}`);
    expect(await limitKey("worker", nobody, "203.0.113.9", "write")).toBe("ip:203.0.113.9");
    expect(await limitKey("open", nobody, null, "create")).toBe(`ip:${UNKNOWN_IP}`);
    expect(IP_HEADER).toBe("cf-connecting-ip");
    expect([RETRY_AFTER_CREATE, RETRY_AFTER_WRITE], "each kind's own window, in seconds").toEqual(["60", "10"]);
    expect(TOO_MANY).toBe("too many requests from you just now — wait a moment and try again");
  });

  it("spends the create limiter on a create, the write limiter on every other unsafe method, and nothing on a read", () => {
    const create = { limit: async () => ({ success: true }) };
    const write = { limit: async () => ({ success: true }) };
    const limits = { create, write };
    expect(limiterFor("POST", { kind: "create" }, limits)).toEqual({ limiter: create, kind: "create" });
    expect(limiterFor("POST", { kind: "table", name: "x" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("DELETE", { kind: "table", name: "x" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "archive" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "admin", rest: "/tables/x/expire" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("GET", { kind: "table", name: "x" }, limits)).toBeUndefined();
    expect(limiterFor("HEAD", { kind: "house" }, limits)).toBeUndefined();
  });
});
