import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule, type Exports, type Reply } from "../gen/factory.js";
import { log } from "./respond";

/** One call the layer made on its signal, as `Reply.bumps` lists them (the
 *  plan's names table; `crates/yokohama-worker/src/signal.rs`). */
export type Bump = { bump: string } | { bump_table: string } | { bump_house_chat: null } | { end: string };

/** A reply as plain data. `archive` (the deferred archive plan), `picker`
 *  (the picker row) and `dropped` (the codes of a picker row `restore_picker`
 *  cleared for its code, `{"stored", "running"}`) are present only when the
 *  module set them. */
export type Replied = { status: number; body: string; bumps: Bump[]; archive?: string; picker?: string; dropped?: string };

/**
 * One object's module (spec §2.2, §2.4): the factory's instance, built on first
 * use, initialized with the object's configuration, and thrown away when it
 * traps (the orchestrator's ruling I-29).
 *
 * - `get()` answers the module, building and initializing a new one when there
 *   is none. `fresh: true` means THIS call built it, so it holds no document
 *   yet and the caller loads the object's documents before anything else. A
 *   caller that gets a fresh module and cannot finish loading it must `drop()`
 *   it, or the next `get()` answers that half-loaded module as not fresh.
 * - `guarded(f)` runs `f` over the module `get()` answers. A
 *   `WebAssembly.RuntimeError` — a trap — discards the module and answers
 *   `{ trapped }`, and the next `get()` builds a fresh one. Anything else `f`
 *   throws reaches the caller unchanged and the module stays: a string the
 *   module threw is a refusal it chose to make, not a trap. `guarded` does not
 *   say whether the module it used was fresh, so after a trap or a `drop()` it
 *   runs `f` over an EMPTY module: a caller asks `get()` first, and loads
 *   when the answer is fresh.
 * - An `init` that refuses keeps no module: `get()` (and so `guarded`) throws
 *   the module's own sentence, which names the variable, and the next `get()`
 *   tries again.
 */
export class Instance {
  private mod: Exports | undefined;

  constructor(private readonly config: string) {}

  get(): { mod: Exports; fresh: boolean } {
    if (this.mod) return { mod: this.mod, fresh: false };
    const mod = createModule(compiled);
    mod.init(this.config);
    this.mod = mod;
    return { mod, fresh: true };
  }

  guarded<T>(f: (m: Exports) => T): { ok: T } | { trapped: string } {
    try {
      return { ok: f(this.get().mod) };
    } catch (e) {
      if (!(e instanceof WebAssembly.RuntimeError)) throw e;
      this.drop();
      return { trapped: String(e) };
    }
  }

  drop(): void {
    this.mod = undefined;
  }
}

/**
 * One module call pinned to the reading `at` (the plan's ruling P-2), answered
 * as plain data, with the setting it replaced restored after it: `set_clock`
 * hands back the prior one, so a test configuration's frozen clock survives
 * (spec §2.6). It serves the calls that drive a plan after the reading was
 * taken: an archive object's two dispatches of a browser's save, a table's
 * second dispatch of an abandon, and an owed save's stamp — each pinned to the
 * one reading its plan is made at, spec §2.6's one clock reading per plan.
 */
export function pinnedTo(mod: Exports, at: number, f: (mod: Exports) => Reply): Replied {
  const prior = mod.set_clock(at);
  try {
    return readReply(f(mod));
  } finally {
    mod.set_clock(prior);
  }
}

/** Every key a bump carries, and what its value is. */
const BUMP_VALUES = { bump: "id", bump_table: "id", bump_house_chat: "null", end: "id" } as const;

/**
 * A reply as plain data — how the shell takes every `Reply` (the plan's names
 * table), so that no caller holds one: every getter is read, the reply is
 * FREED, and only then are the lists parsed, so a list that does not parse
 * cannot leak the reply it came from.
 *
 * A bump this shell does not know is refused, named, rather than dropped: a
 * bump nobody reads is a frame nobody sends.
 *
 * **The layer's own log lines are logged HERE, as the shell's** (M6.5c2, spec
 * §2.6): every reply passes through this function, so no caller can forget
 * them — a game-over stamp that did not land, a table the listing skipped. They
 * report what the layer did during the call, whatever the caller's batch then
 * does. Every field is a string (`crates/yokohama-web/src/log.rs`), and a line
 * that breaks that is refused, named, like a bump: a number here would round.
 */
export function readReply(r: Reply): Replied {
  const status = r.status;
  const body = r.body;
  const bumps = r.bumps;
  const archive = r.archive;
  const picker = r.picker;
  const dropped = r.dropped;
  const logs = r.logs;
  r.free();
  const replied: Replied = {
    status,
    body,
    bumps: parseBumps(bumps),
    ...(archive === undefined ? {} : { archive }),
    ...(picker === undefined ? {} : { picker }),
    ...(dropped === undefined ? {} : { dropped }),
  };
  for (const line of parseLogs(logs)) log(line.event, line.fields);
  return replied;
}

function parseBumps(text: string): Bump[] {
  let list: unknown;
  try {
    list = JSON.parse(text);
  } catch {
    throw new Error(`the module's bumps do not parse: ${text}`);
  }
  if (!Array.isArray(list)) throw new Error(`the module's bumps are not a list: ${text}`);
  for (const bump of list) {
    const keys = bump !== null && typeof bump === "object" && !Array.isArray(bump) ? Object.keys(bump) : [];
    const key = keys.length === 1 ? keys[0] : undefined;
    const kind = key !== undefined && Object.hasOwn(BUMP_VALUES, key) ? BUMP_VALUES[key as keyof typeof BUMP_VALUES] : undefined;
    const value = key === undefined ? undefined : (bump as Record<string, unknown>)[key];
    if (kind === undefined || (kind === "id" ? typeof value !== "string" : value !== null)) {
      throw new Error(
        `the module's bumps carry ${JSON.stringify(bump)}, which is no bump ` +
          `(one key of ${Object.keys(BUMP_VALUES).join(" | ")})`,
      );
    }
  }
  return list as Bump[];
}

/** One line the layer logged during a call (`crates/yokohama-worker/src/log.rs`). */
type LayerLine = { event: string; fields: Record<string, string> };

function parseLogs(text: string): LayerLine[] {
  let list: unknown;
  try {
    list = JSON.parse(text);
  } catch {
    throw new Error(`the module's log lines do not parse: ${text}`);
  }
  if (!Array.isArray(list)) throw new Error(`the module's log lines are not a list: ${text}`);
  for (const line of list) {
    const shaped = line !== null && typeof line === "object" && !Array.isArray(line);
    const { event, fields } = (shaped ? line : {}) as { event?: unknown; fields?: unknown };
    const ok =
      shaped &&
      Object.keys(line as object).length === 2 &&
      typeof event === "string" &&
      fields !== null &&
      typeof fields === "object" &&
      !Array.isArray(fields) &&
      Object.values(fields as object).every((v) => typeof v === "string");
    if (!ok) {
      throw new Error(
        `the module's log lines carry ${JSON.stringify(line)}, which is no line ({event, fields}, every field a string)`,
      );
    }
  }
  return list as LayerLine[];
}
