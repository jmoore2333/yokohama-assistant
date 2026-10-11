// **A framed door does not act** (security S2a, finding B4; spec §2.2 B4's catcher). The review framed `#/lobby`
// at low opacity from another site and two clicks landed on Abandon and its confirm. Now the local binary answers
// the shell with `X-Frame-Options: DENY` (Task 3) and the browser refuses to render it inside a frame: the shell's
// bytes are fetched, nothing in them is parsed, so none of the release's scripts is asked for and nothing under
// `/api/` is touched. The control at the end navigates the SAME page to the door directly and reads the release's
// scripts being asked for, so "no script was asked for" is the frame's refusal and not a server that is down.
// The second cell strips the host's fences from the framed document and reads the client's own refusal — the one
// sentence `main.tsx` puts in `#root` instead of mounting — which is the catcher of the `framed(window)` branch.
//
// **The framer is a second LOOPBACK origin** (measured, the plan review): a framer on `about:blank`, a `data:` URL
// or an https page is stopped by Chromium's Local Network Access check,
// `net::ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`, before the host answers, so a cell framing from one would time
// out with AND without the fence. The worker is OFF (the `./sw` fixture): the question is what the HOST answers a
// framed navigation.
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { expect, test } from "./sw";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly — the guard
// `every_spec_can_skip.spec.ts` reads this directory for.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The host's frame fences, stripped by the second cell's route so the client's own is what answers: the header,
 *  and the policy under both names (the e2e build serves it ENFORCED, and its `frame-ancestors 'none'` refuses
 *  the frame as the header does). */
const FRAME_FENCES = ["x-frame-options", "content-security-policy", "content-security-policy-report-only"];

let framer: Server;
let framerUrl = "";
test.beforeAll(async () => {
  framer = createServer((_req, res) => {
    res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    res.end(`<!doctype html><title>elsewhere</title><iframe id="f" src="${BASE_URL}/#/lobby" width="900" height="700"></iframe>`);
  });
  await new Promise<void>((r) => framer.listen(0, "127.0.0.1", r));
  framerUrl = `http://localhost:${(framer.address() as AddressInfo).port}/`;
});
test.afterAll(() => framer.close());

/** How long the frame is watched after its document answered, for a request it would make if it had rendered. */
const SETTLE_MS = 2_000;

test("a page on another origin that frames the lobby gets a refused frame: X-Frame-Options DENY on the answer, the policy ENFORCED beside it, and none of the release asked for", async ({ page }) => {
  const origin = new URL(BASE_URL).origin;
  const asked: string[] = [];
  page.on("request", (r) => {
    if (r.url().startsWith(origin)) asked.push(new URL(r.url()).pathname);
  });
  const shell = page.waitForResponse((r) => r.url() === `${BASE_URL}/`);
  await page.goto(framerUrl);
  const answer = await shell;
  expect(answer.headers()["x-frame-options"], "the host's fence").toBe("DENY");
  expect(answer.headers()["content-security-policy"], "the e2e build serves the policy ENFORCED (decision D-9)").toBeDefined();
  await page.waitForTimeout(SETTLE_MS);
  expect(asked.filter((p) => p.startsWith("/assets/")), "a refused frame parses nothing, so it asks for no script").toEqual([]);
  expect(asked.filter((p) => p === "/api" || p.startsWith("/api/")), "and touches the API not at all").toEqual([]);

  // The control: the same page, navigated to the door directly, asks for the release's scripts.
  asked.length = 0;
  await page.goto(`${BASE_URL}/#/`);
  await page.getByRole("heading", { name: /Yokohama/ }).first().waitFor();
  expect(asked.some((p) => p.startsWith("/assets/")), "the door itself loads its scripts").toBe(true);
});

test("the client's own refusal: with the host's frame fences stripped from the framed document, the page says its one sentence and mounts nothing", async ({ page }) => {
  await page.route(`${BASE_URL}/`, async (route) => {
    const response = await route.fetch();
    const headers = { ...response.headers() };
    for (const name of FRAME_FENCES) delete headers[name];
    await route.fulfill({ response, headers });
  });
  await page.goto(framerUrl);
  const frame = page.frameLocator("#f");
  await expect(frame.locator("#root")).toHaveText("Yokohama does not run inside another page. Open it in a tab of its own.");
  await expect(frame.locator("h1")).toHaveCount(0);
});
