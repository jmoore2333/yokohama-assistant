/**
 * What `Table.onAlarm` does with the name the platform hands `alarm()` (the
 * plan's pool fact 8; the orchestrator's ruling R-09).
 *
 * Inside the object, `ctx.id.name` is the name the object was reached by, in
 * `alarm()` as in `fetch()` — Cloudflare's changelog of 2026-03-15, as the plan
 * quotes it: "When an alarm handler runs, `ctx.id.name` will hold the same name
 * the object was originally accessed with." It is `undefined` for an object
 * never reached by a name — made by `newUniqueId()`, or reached only through
 * `idFromString()` — and for a name longer than 1,024 bytes; the pool gives
 * both of those no name, measured. `meta` stores no id, so an alarm with no
 * name cannot know which table it serves: it is skipped.
 *
 * A pure function in a module of its own, so a node cell pins it and a pool
 * cell can replace it at module scope to prove `onAlarm` consults it.
 */
export function guardAlarmName(name: string | undefined): "run" | "skip" {
  return name === undefined ? "skip" : "run";
}
