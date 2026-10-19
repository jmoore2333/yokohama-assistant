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
    expect(limiterFor("POST", { kind: "create" }, limits, false)).toEqual({ limiter: create, kind: "create" });
    expect(limiterFor("POST", { kind: "table", name: "x" }, limits, false)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("DELETE", { kind: "table", name: "x" }, limits, false)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "archive" }, limits, false)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "admin", rest: "/tables/x/expire" }, limits, false)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("GET", { kind: "table", name: "x" }, limits, false)).toBeUndefined();
    expect(limiterFor("HEAD", { kind: "house" }, limits, false)).toBeUndefined();
  });

  // The anonymous archive (the auth cycle, Task 5; design §4.4's ceiling 2): a caller-less upload is counted like a
  // create, by the connecting IP — never by its device or game id, which a client can rotate at will. Mutants:
  // `limitKey` keying an anonymous caller by its tag; `limiterFor` sending the anonymous post to the write limiter, or
  // every archive post (a signed-in one among them) to the create limiter; an anonymous flag widening another target.
  it("counts an anonymous caller by IP under either door, and spends the create limiter on its archive post alone", async () => {
    const anonymous = { kind: "anonymous", device: "0123456789abcdef" } as const;
    const byGame = { kind: "anonymous", device: "local-aaaaaaaaaaaaaaaa" } as const;
    expect(await limitKey("worker", anonymous, "9.9.9.9", "create")).toBe("ip:9.9.9.9");
    expect(await limitKey("worker", byGame, "9.9.9.9", "create"), "a game id is no budget of its own").toBe("ip:9.9.9.9");
    expect(await limitKey("worker", anonymous, null, "create")).toBe(`ip:${UNKNOWN_IP}`);
    expect(await limitKey("open", anonymous, "9.9.9.9", "write"), "never the IP-plus-claim key").toBe("ip:9.9.9.9");
    const session = { kind: "session", email: "red@wire.test", sid: null, renew: false } as const;
    expect(await limitKey("worker", session, "9.9.9.9", "write"), "a session is its own budget").toBe(await callerTag("red@wire.test"));

    const create = { limit: async () => ({ success: true }) };
    const write = { limit: async () => ({ success: true }) };
    const limits = { create, write };
    expect(limiterFor("POST", { kind: "archive" }, limits, true)).toEqual({ limiter: create, kind: "create" });
    expect(limiterFor("POST", { kind: "archive" }, limits, false), "a signed-in upload is a write").toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "table", name: "x" }, limits, true)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "house" }, limits, true)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("GET", { kind: "archive" }, limits, true)).toBeUndefined();
  });
});
