import { describe, expect, it } from "vitest";
import { adminRoute, FORCE_TAKES_NO_VALUE, operatorGate } from "./admin";

/** Two operators, as `shellConfig` hands them over: trimmed and lowercased. */
const ADMINS = ["chief@t.test", "boss@t.test"];

// Spec §2.9's gate, as a pure function (`test/operator.test.ts` runs it through
// `handle`). Battery 32: an identity reached through the dev door NEVER counts,
// even when the address it claims is on the list. Mutants: a dev identity
// counted; any signed-in person counted; no one counted; the gate run with no
// list (a 403 where the layer's 404 belongs).
describe("the operator gate (spec §2.9)", () => {
  it("passes a listed person, with their address", () => {
    expect(operatorGate("worker", ADMINS, { kind: "person", email: "boss@t.test" })).toEqual({ kind: "operator", email: "boss@t.test" });
  });

  it("refuses a dev-door identity claiming a listed address (battery 32)", () => {
    expect(operatorGate("worker", ADMINS, { kind: "dev", email: "boss@t.test" })).toEqual({ kind: "refused" });
  });

  it("refuses a person not on the list, and no one", () => {
    expect(operatorGate("worker", ADMINS, { kind: "person", email: "red@t.test" })).toEqual({ kind: "refused" });
    expect(operatorGate("worker", ADMINS, { kind: "none" })).toEqual({ kind: "refused" });
  });

  it("is off with no list, whoever calls", () => {
    for (const identity of [{ kind: "person", email: "boss@t.test" }, { kind: "dev", email: "boss@t.test" }, { kind: "none" }] as const) {
      expect(operatorGate("worker", null, identity), identity.kind).toEqual({ kind: "off" });
    }
  });

  // M6.5c2, spec §2.1 (review R2.2-M1): under the OPEN door no one is an
  // operator — a claimed name is never an operator's rights — so the gate is
  // `off`, the routes the layer's 404, whatever the list and whoever calls.
  // `shellConfig` refuses a list beside `open`, and a person cannot reach the
  // gate there, but the gate reads the door first all the same. A claimed name
  // never counts under `worker` either. Mutants: the door not read (a claim
  // meets the 403, a listed person the surface); `refused` under `open`; a
  // claim read as a person.
  it("is off under the open door, whatever the list and whoever calls; a claim is never an operator", () => {
    const callers = [
      { kind: "claimed", name: "boss@t.test" },
      { kind: "person", email: "boss@t.test" },
      { kind: "none" },
    ] as const;
    for (const identity of callers) {
      expect(operatorGate("open", ADMINS, identity), identity.kind).toEqual({ kind: "off" });
      expect(operatorGate("open", null, identity), `${identity.kind}, no list`).toEqual({ kind: "off" });
    }
    expect(operatorGate("worker", ADMINS, { kind: "claimed", name: "boss@t.test" })).toEqual({ kind: "refused" });
  });
});

// Spec §2.9's two routes, and the orchestrator's ruling I-72: `?force` is the
// BARE key — present with an empty value it forces; with ANY value it is a
// refusal naming the switch, never a force and never a plain expire, since
// `?force=false` forcing would drop a save the operator meant to keep; and a
// query holding any other key is unrouted, as an unknown path is. Mutants: a
// force read from any value; a value's refusal answered as a plain expire; a
// key compared without regard to case; a route taken whatever its method; an
// id no table object can have routed to one.
describe("the operator's routes (spec §2.9; ruling I-72)", () => {
  const expire = "/api/admin/tables/wire/expire";

  it("takes the listing on a GET with no query, and on nothing else", () => {
    expect(adminRoute("GET", "/api/admin/tables", "")).toEqual({ kind: "listing" });
    for (const method of ["POST", "HEAD", "DELETE"]) {
      expect(adminRoute(method, "/api/admin/tables", ""), method).toEqual({ kind: "unrouted" });
    }
    expect(adminRoute("GET", "/api/admin/tables", "scope=all"), "a query it does not take").toEqual({ kind: "unrouted" });
  });

  it("takes a plain expire on a POST with no query, for a name a table object can have", () => {
    expect(adminRoute("POST", expire, "")).toEqual({ kind: "expire", name: "wire", force: false });
    expect(adminRoute("GET", expire, ""), "the method").toEqual({ kind: "unrouted" });
    expect(adminRoute("POST", `/api/admin/tables/${"a".repeat(65)}/expire`, ""), "65 bytes: no table object's name").toEqual({
      kind: "unrouted",
    });
  });

  it("forces on the bare key, written with or without its =", () => {
    expect(adminRoute("POST", expire, "force")).toEqual({ kind: "expire", name: "wire", force: true });
    expect(adminRoute("POST", expire, "force=")).toEqual({ kind: "expire", name: "wire", force: true });
  });

  it("refuses the switch with any value, naming it", () => {
    for (const value of ["false", "0", "1", "true", "yes"]) {
      expect(adminRoute("POST", expire, `force=${value}`), value).toEqual({ kind: "refused", sentence: FORCE_TAKES_NO_VALUE });
    }
    expect(FORCE_TAKES_NO_VALUE).toBe("?force takes no value — send ?force alone, or no query for an expire that saves first");
  });

  it("leaves unrouted a query holding any key but the one it takes", () => {
    for (const query of ["Force", "FORCE", "forced", "dry=1", "force&dry=1", "force&force"]) {
      expect(adminRoute("POST", expire, query), query).toEqual({ kind: "unrouted" });
    }
  });

  it("leaves every other path unrouted", () => {
    for (const [method, path] of [
      ["GET", "/api/admin"],
      ["POST", "/api/admin/tables/wire"],
      ["POST", "/api/admin/tables/wire/expire/now"],
      ["POST", "/api/admin/tables//expire"],
    ] as const) {
      expect(adminRoute(method, path, ""), `${method} ${path}`).toEqual({ kind: "unrouted" });
    }
  });
});
