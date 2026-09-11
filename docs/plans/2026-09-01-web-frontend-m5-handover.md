# Web frontend — M5 "the table": the execution handover

**Written 2026-09-01 at `web-frontend` `1cf763c`, for the session that executes the plan.**
One screen. It points; it does not restate. The spec and the plan are the authority.

## What exists, and where

| Document | Role |
|---|---|
| `docs/plans/2026-09-01-web-frontend-m5-spec.md` (`bc62080`, corrected in `1cf763c`) | **The authority.** Owner rulings R1–R11 + R6′ (§1), what was verified against the assets (§2), the design (§3), the work with its guard (§4), the visual gates (§4.1), what is not claimed (§5), upgrades (§6). |
| `docs/plans/2026-09-01-web-frontend-m5-plan.md` (`1cf763c`) | **What to execute.** Sixteen tasks, each with the failing test, the code, the command, the commit. Eleven deviations from the spec, decided while planning, listed at its end — do not re-decide them; record them in the close (Task 16). |
| `docs/plans/2026-09-01-web-frontend-m4-spec.md`, `-m3.5-spec.md` | The rulings M5 inherits (scrubber in the Log drawer; the 10rem floor; caption-below, now moot). Read §1 of each, nothing more. |

The worktree is `.claude/worktrees/cli-web-api-port-936d56` (branch `web-frontend`). Every
command in the plan runs from there. `git status` is clean at `1cf763c`.

## How to start

1. Read `CLAUDE.md` (the worktree's), then the spec §1 and §3, then the plan's header,
   file table and Task 1. Do not re-read the M4/M3.5 specs beyond their §1.
2. Invoke `superpowers:subagent-driven-development` on the plan. Tasks run in the
   plan's order; the order is load-bearing (the fit rule needs the grid it measures
   against — plan deviation 1).
3. **Do not re-explore.** Every measurement the design rests on is in the spec §0 and
   §0.1 (the live site at the owner's 1710×869, the CSS-only mock). The contact sheet
   of the area arts and the mock screenshots are gitignored under `docs/temp/web-review/`
   and may be gone; the numbers that came off them are in the spec and in the plan's
   Task 2 JSON.

## The gates, mechanically

Tasks 2 (V0), 6 (V1), 13 (V2) and 14 (V3) end with **`SendUserFile` of the pictures and
a STOP**. A stop means the turn ends with the pictures sent and a one-line question;
the owner answers in their own time. Nothing past a stop is dispatched until the answer
arrives. The owner said: *"if you are not highly confident you can also have these
gates stop to ask me to see how it looks at different resolutions"* — and the plan
defines "not highly confident" at V2 to include any fit reading more than 0.5rem from
the spec's §3.3 prediction.

## Tooling facts learned this session (not in any doc)

- **Claude-in-Chrome carries the Cloudflare Access cookie**; the DevTools MCP's own
  Chrome does not, so `yokohama.moore.nyc` is reachable only through the former. Use it
  for Task 15's live check. **The owner's saved game there is `sim-d7d5918bb4f74a2a`
  and is never played on** — open it read-only, screenshot, leave.
- For every layout look before the gates, run the local copy of the SAME build:
  `cargo build --release -p yokohama-web && cd web && npm run build && cd .. &&
  YOKOHAMA_WEB_PORT=8491 YOKOHAMA_WEB_DIR=docs/temp/web-review/games ./target/release/yokohama-web`
  and drive `http://127.0.0.1:8491/` with the DevTools MCP (`resize_page` to 1710×869 —
  the owner's viewport, DPR 2). Stop it with `pkill -f target/release/yokohama-web`.
- **DevTools screenshots can only be saved under the workspace root**; `docs/temp/`
  is gitignored and is where they go. `Read` every picture you send before sending it.
- The e2e walk builds `dist` and starts the release server itself (`npm run e2e`);
  never point it at a running server. `npx playwright install chromium` once.
- `python3 docs/tools/mkcardart.py` needs Pillow and PyYAML; both are present on this
  machine.

## Blast radius, restated

`git diff master..HEAD -- crates/yokohama-core` EMPTY; `SEMANTICS_REV` 7; the web crate
changes only under `assets/art/**` and `art.rs:404`'s achievements row. Check at Task 16.

## What is NOT open

The deck's home (top of the side column), the table model, which surfaces stay drawers,
the art-only tile with the site card rotated −90° and SVG pieces, the fit rule's floor
and ceiling, the Boards disclosure, the image tag `0.5.0-m5`, management-board art
deferred to M6/M7. All ruled or decided; none re-opened by an implementer.

## After the close

M6 is the multi-game layer (spec §6 item 2): lobby, chat, switching, loading saves,
another player's game id, from the Game menu Task 10 builds. It inherits the M4
close's hosting list first (Access header verification, the enumerable id space).
