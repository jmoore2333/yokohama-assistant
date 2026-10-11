// **The corpus, replayed against the LIVE Worker** (spec §5.3's live half,
// R-6.5c.12; the plan's Task 15). Run by the ORCHESTRATOR from the owner's
// workstation, over WARP, after the quota guard — never by a gate:
//
//   cd worker && node scripts/live-replay.mjs
//
// `crates/yokohama-web/tests/wire_fixtures.rs` walked the axum router through a
// deterministic two-player game and froze every payload the client reads under
// `web/src/wire/`; the pool replays the same walk through the objects
// (`test/corpus.test.ts`). This sends it to the deployed Worker, through the
// edge and Access, as the walk's own `.test` identities — and it is
// REPEATABLE, and safe to re-run after a run that failed half way, for as long
// as the walk's `wire` records are the bytes its two first sends saved (see
// "The reset" below):
//
// 1. **The operator's force** on each table the walk creates (`wire`, `wire2`,
//    `wire4`), `POST /api/admin/tables/<id>/expire?force` — the BARE key, since
//    a value is a 400 (ruling I-72) — sent with NO credential and no claimed
//    header, so WARP makes it the owner's own assertion: the credential is
//    never an operator (R-6.5c.16). A force leaves no marker, and a force on an
//    object holding nothing is 204 writing nothing. Anything but three 204s
//    and the run does not start.
// 2. **The house chat's `last` and `rev`**, read as the walk's own house-chat
//    reader: the base its payloads are taken back by (spec §9.3).
// 3. **Every row the production seed rule lets through**, in `order`'s order:
//    `walk.json`'s, each `walk-host.json` row after the walk row its `after`
//    names (`-1`: before the first), then `archive-walk.json`'s — with the credential and
//    `X-Yokohama-Claimed-User: <who>`, a row with no `who` sending the
//    credential alone. The rule (`productionSeedRule`, security S1b): `wire`
//    and `wire2` are seeded creates beside an open seat, which a production
//    host answers 400 (`SEED_NOT_SOLO`) — each create is sent and held to that
//    answer, and every row at those tables, the host script's among them, and
//    every lobby page, whose committed bodies list them, is skipped and
//    counted (`skippedUnderProductionSeedRule`). A walk whose rule names no
//    table skips nothing: its lobby pages run, each judged through
//    `filterLobby` with up to `LOBBY_REREADS` re-reads. The pool replays the
//    whole walk under its diagnostic switch.
// 4. **Every row's status, and every named row's body** against its committed
//    file — the clock frozen first (`freezeClock`), then three named
//    exemptions (`filterLobby`, `normalizeHouseChat`, `firstSendAccepts`, the
//    pure functions of `replay/exemptions.mjs`) and `HOST_FIELDS`, each COUNTED when it is
//    spent; a host field that reads as the axum host's is refused, because an
//    exemption that differs nowhere is a hole a later host could hide behind.
//    **The walk's house post is DELETED by the run that made it** (M6.5c2
//    cycle D, R-D4): the delete row is sent carried forward to the number
//    that run's post was really given (`houseDeleteRow`, counted like the
//    rest), so every run answers 204 and no run leaves a post behind.
// 5. **The force again** whenever the force before held, however the run then
//    ended — a base that could not be read and a row that stopped the run
//    included; then the report: rows run of the total, rows skipped under the
//    production seed rule, named rows compared, the exemptions spent, the
//    lobby re-reads spent, where the run stopped if it did, the reset's
//    commands after a first send's 409, and the count of mismatches, each
//    printed as it was found. Exit 1
//    on a mismatch, a force that answered other than 204, a run that stopped,
//    or a refusal before the first send.
//
// **The reset** (R3's M-1). The two first sends' saves stay in the bucket for
// good, by the owner's direction, so every later run's first sends meet them:
// the once-rule's 200 `identical`. A `wire` record whose bytes have changed —
// a version bump, a walk that applies other moves, a record format change —
// meets them as a 409 instead. The run stops there, since every row after it
// reads a save it did not make, names the cause, and prints the four keys and
// a `npx wrangler r2 object delete` for each (`resetRecipe`), run in `worker/`
// with wrangler's own login: the owner's R2 key is read-only (spec §7).
//
// **The secret.** `.dev.vars` is read by path and its values are never
// printed, and the credential goes to the Worker's own origin alone — a row
// whose path would leave it is refused before anything is sent, and no
// redirect is followed. This script never prints a request BODY (the walk's
// bodies carry passwords and seeds), nor a redirect's `location` (Access's
// carries a token), and a reported value under a key that could hold a secret
// or a seed prints as `<redacted>`.
//
// **Its parts** (M6.5c2 cycle D, Task 9), under `scripts/replay/`, each with
// its cells in `live-replay.test.mjs`: the credential and the run's bounds
// (`credential.mjs`, the credential itself `dev-vars.mjs`'s, which the live
// tier shares); the scripts, their order and where each row goes
// (`scripts.mjs`, the order `corpus-walk.mjs`'s, which the pool shares); the
// named exemptions (`exemptions.mjs`); the sends and the forces (`send.mjs`);
// and the judging of a reply and the report (`judge.mjs`). `replay`, below,
// is the one place they meet.

import { pathToFileURL } from "node:url";
import { KEYS, LOBBY_PAUSE_MS, LOBBY_REREADS, ORIGIN, readCredential } from "./replay/credential.mjs";
import {
  FIRST_SENDS,
  firstSendAccepts,
  houseDeleteRow,
  isProductionSeedCreate,
  LOBBY,
  productionSeedRule,
  resetRecipe,
  SEED_NOT_SOLO,
  skippedUnderProductionSeedRule,
} from "./replay/exemptions.mjs";
import { errorText, finish, fmt, judge, parsed, said } from "./replay/judge.mjs";
import { loadCorpus, loadScripts, order, tableIds, urlOf } from "./replay/scripts.mjs";
import { forceAll, send } from "./replay/send.mjs";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * The live replay. `fetcher`, `log` and `pause` are the network, the report's
 * sink and the wait between a lobby page's reads — the cells hand in fakes.
 * Answers the run's result; `holds` is whether every force answered 204,
 * every row the production seed rule sends ran, and nothing differed.
 */
export async function replay({ walk, host, archive, corpus, credential, fetcher = fetch, log = console.log, pause = sleep }) {
  const rows = order(walk, host, archive);
  const ids = tableIds(walk);
  const reader = rows.find(({ row }) => row.fixture === "house_chat")?.row;
  if (reader === undefined) throw new Error("no row of the scripts reads the house chat, so there is no base to read");
  // Every row's address, checked BEFORE anything is sent — the forces
  // included: a row that would leave the Worker's origin refuses the run.
  for (const { row } of rows) urlOf(row);
  const result = {
    holds: false,
    run: 0,
    total: rows.length,
    skipped: 0,
    named: 0,
    spent: [],
    rereads: 0,
    mismatches: [],
    forcedBefore: [],
    forcedAfter: [],
    base: null,
    stopped: null,
    reset: null,
  };
  /** A mismatch, kept and printed as it is found. */
  const differs = (label, fixture, diff) => {
    result.mismatches.push({ label, fixture, ...diff });
    const key = diff.path.split(".").at(-1);
    const what = diff.reason ?? `committed ${fmt(diff.committed, key)} · got ${fmt(diff.got, key)}`;
    log(`  MISMATCH ${label} — ${fixture === null ? "" : `${fixture} `}${diff.path}: ${what}`);
  };

  log(`live-replay: ${ORIGIN} — ${KEYS.join(" and ")} to this origin only; the forces bare (over WARP, the owner's device answers Access)`);
  result.forcedBefore = await forceAll(ids, fetcher);
  log(`force before: ${result.forcedBefore.map((f) => `${f.id} ${f.status ?? "—"}${f.status === 204 ? "" : f.said}`).join(", ")}`);
  if (!result.forcedBefore.every((f) => f.status === 204)) {
    result.stopped = "a force before the run answered other than 204, so no row was sent";
    return finish(result, log);
  }

  try {
    const baseReply = await send(reader, credential, fetcher);
    const basePage = baseReply.status === 200 ? parsed(baseReply) : undefined;
    const last = basePage?.last;
    const baseRev = basePage?.rev;
    if (typeof last === "number" && typeof baseRev === "number") {
      result.base = last;
      log(`the house chat's base: last ${last}, rev ${baseRev}, read as ${reader.who}`);
    } else {
      // No base, no rows: the `finally` forces the ids again, and the report
      // follows it, as on every other road out.
      result.stopped = `the house chat's base could not be read: ${baseReply.status}${said(baseReply)}`;
    }
    // `houseShift`: set once the run's own house post is answered.
    const context = { corpus, ids, base: last, baseRev, houseShift: undefined };
    const skipped = productionSeedRule(walk);

    for (const { script, index, row: scripted } of result.stopped === null ? rows : []) {
      let row = scripted;
      try {
        row = houseDeleteRow(scripted, context.houseShift);
      } catch (e) {
        result.stopped = `${script} row ${index}: ${errorText(e)}`;
        break;
      }
      if (row !== scripted) result.spent.push(`houseDeleteRow ${row.fixture}: ${scripted.path} sent as ${row.path}`);
      const label = `${script} row ${index}: ${row.method} ${row.path}${row.query ? `?${row.query}` : ""} as ${row.who ?? "no one"}`;
      if (skippedUnderProductionSeedRule(row, skipped)) {
        result.skipped += 1;
        continue;
      }
      let reply;
      try {
        reply = await send(row, credential, fetcher);
      } catch (e) {
        result.stopped = label;
        differs(label, null, { path: "status", reason: `no answer: ${errorText(e)}` });
        break;
      }
      result.run += 1;
      if (result.run % 500 === 0) log(`… ${result.run} of ${result.total} rows`);
      if (isProductionSeedCreate(row, skipped)) {
        const id = row.body.id;
        if (reply.status === 400 && parsed(reply)?.error === SEED_NOT_SOLO) {
          result.spent.push(`productionSeedRule ${id}: the seeded create beside a seat that is not its creator's answered 400 in SEED_NOT_SOLO's words`);
        } else {
          differs(label, null, { path: "(production seed rule)", reason: `expected 400 in SEED_NOT_SOLO's words · got ${reply.status}${said(reply)}` });
        }
        continue;
      }

      if (row.fixture !== null && corpus[row.fixture] === undefined) throw new Error(`${row.fixture} is named by a script and not committed`);
      if (row.fixture !== null && FIRST_SENDS.has(row.fixture)) {
        result.named += 1;
        if (reply.status === 409) {
          // An earlier run's save of a `wire` record that has since changed:
          // every row after this one would read a save this run did not make.
          result.stopped =
            `${row.fixture} answered 409${said(reply)}. The bucket holds an earlier run's save of a wire record that has ` +
            "since changed (a version bump, a walk that applies other moves, a record format change): delete the " +
            "replay's four saves, then run it again";
          result.reset = resetRecipe(corpus, row.who);
          differs(label, row.fixture, { path: "(first send)", reason: `committed 201, or the once-rule's 200 · got 409${said(reply)}` });
          break;
        }
        const committed = JSON.parse(corpus[row.fixture]);
        // Not JSON reads as `null`: refused below, and reported by its shape.
        const got = parsed(reply);
        const verdict = firstSendAccepts(committed, reply.status, got);
        if (verdict === null) {
          differs(label, row.fixture, {
            path: "(first send)",
            reason: `committed 201 ${fmt(committed)}, or the once-rule's 200 with "identical": true · got ${reply.status}${got === null ? said(reply) : ` ${fmt(got)}`}`,
          });
        } else {
          result.spent.push(`firstSendAccepts ${row.fixture}: ${verdict === "landed" ? "201, the first send landed" : `200, the once-rule's "identical": true`}`);
        }
        continue;
      }

      if (reply.status !== row.status) {
        // Every row after it would read a state this run did not make.
        result.stopped = label;
        differs(label, null, { path: "status", reason: `committed ${row.status} · got ${reply.status}${said(reply)}` });
        break;
      }
      // The run's own house post: the number the delete row is carried to.
      if (row.fixture === "house_chat_post") {
        const seq = parsed(reply)?.seq;
        if (typeof seq === "number") context.houseShift = seq - JSON.parse(corpus.house_chat_post).seq;
      }
      if (row.fixture === null) continue;

      result.named += 1;
      let judged = judge(row.fixture, reply, context);
      for (let again = 0; LOBBY.has(row.fixture) && judged.diffs.length > 0 && again < LOBBY_REREADS; again++) {
        // The table's slot push may not have reached the house yet
        // (`LOBBY_REREADS` says why); what still differs after the last read
        // is reported below as a mismatch.
        await pause(LOBBY_PAUSE_MS);
        const reread = await send(row, credential, fetcher);
        result.rereads += 1;
        if (reread.status !== row.status) {
          judged = { diffs: [{ path: "status", reason: `re-read: committed ${row.status} · got ${reread.status}${said(reread)}` }], spent: [] };
          break;
        }
        judged = judge(row.fixture, reread, context);
      }
      result.spent.push(...judged.spent);
      for (const diff of judged.diffs) differs(label, row.fixture, diff);
    }
  } catch (e) {
    result.stopped = `the run stopped: ${errorText(e)}`;
  } finally {
    result.forcedAfter = await forceAll(ids, fetcher);
    log(`force after: ${result.forcedAfter.map((f) => `${f.id} ${f.status ?? "—"}${f.status === 204 ? "" : f.said}`).join(", ")}`);
  }
  return finish(result, log);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv.length > 2) {
    console.error("usage: node scripts/live-replay.mjs   (no arguments: the origin is the deployed Worker's, and the credential is .dev.vars')");
    process.exit(2);
  }
  try {
    const result = await replay({ ...loadScripts(), corpus: loadCorpus(), credential: readCredential() });
    process.exit(result.holds ? 0 : 1);
  } catch (e) {
    console.error(`live-replay: ${errorText(e)}`);
    process.exit(1);
  }
}
