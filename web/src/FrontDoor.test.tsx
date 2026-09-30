// The front door at `#/` (R-6.5b2.1, R-6.5c2.6), which everyone sees.
//
// **The three cards are not three of a kind, and that is what these cells
// pin.** "You against the bot" opens a form this browser draws and plays with
// no server at all; "Go to the lobby" is a plain link to `/enter`, so the login
// is an ordinary navigation — a `fetch` there would follow Access's redirect to
// another origin and fail opaquely (spec §0.3), which is why the `href` is
// asserted by value; and "On this device" is this browser's own games.
//
// **Everything the door draws comes from the BUILD or from this browser**
// (R-6.5b2.7): the friends card is a build value, the project link is a build
// value, the bot's version is the committed contract file, and the games are
// the local host's. The door asks the server nothing, and a cell below says so
// with `fetch` as the setup file's throwing stub.

import { readFileSync } from "node:fs";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrontDoor, multiplayerOffered } from "./FrontDoor";
import * as values from "./buildValues";
import { linkFor } from "./buildValues";
import type { ReadinessStatus } from "./swRegister";
import { resetLocalHostForTests } from "./local/client";
import { stubWorker } from "./test/fakeWorker";
import productionBot from "./wire/production_bot.json";
import { topLevelRules } from "./test/css";

/** The row the fake local host lists — the corpus's own `TableView`, named for
 *  this device (`fakeWorker.ts`'s one permitted edit). */
const LOCAL = "local-1";

describe("the front door", () => {
  beforeEach(() => {
    resetLocalHostForTests();
    stubWorker();
  });
  afterEach(() => {
    resetLocalHostForTests();
  });

  it("offers multiplayer as a navigation to the turnstile", () => {
    render(<FrontDoor multiplayer />);
    expect(screen.getByRole("link", { name: "Go to the lobby" })).toHaveAttribute("href", "/enter");
    expect(screen.getByRole("heading", { name: "Play with friends" })).toBeInTheDocument();
  });

  // The product, named once, at the top of the document (Task 3 review, Minor
  // 5) — in the hero, and NOT in `header.banner`, which is the game's header
  // and the one the browser walk's `atTheGame` is scoped to.
  it("names the product at the top of the document", () => {
    render(<FrontDoor multiplayer />);
    const title = screen.getByRole("heading", { level: 1, name: "Yokohama" });
    expect(title.closest("header")).toHaveClass("door-hero");
    expect(document.querySelector("header.banner")).toBeNull();
  });

  // **The channel that needs nothing comes first** (Task 3 review, Minor 3) —
  // since M6.5c2 that is the bot card, whose New game needs no server; the
  // device card is third, in the approved mock's order.
  it("puts the channel that needs nothing above the one that needs a login", () => {
    render(<FrontDoor multiplayer />);
    const bot = screen.getByRole("heading", { name: /You against the bot/ });
    const friends = screen.getByRole("heading", { name: "Play with friends" });
    expect(bot.compareDocumentPosition(friends) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const titles = [...document.querySelectorAll(".door-cards > .door-card h2")].map((h) => h.textContent);
    expect(titles).toEqual([`You against the bot ${productionBot.label}`, "Play with friends", "On this device"]);
  });

  // **State E** (R-6.5c2.4, mock 06): nothing to enter, nothing offered — and
  // no sentence either: the friends card is simply not there, and the other
  // two widen.
  it("draws no friends card when the build serves none, and the other two widen", () => {
    render(<FrontDoor multiplayer={false} />);
    expect(screen.queryByRole("link", { name: "Go to the lobby" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Play with friends" })).toBeNull();
    const grid = document.querySelector(".door-cards")!;
    expect(grid).toHaveClass("two-up");
    expect(grid.querySelectorAll(":scope > .door-card")).toHaveLength(2);
  });

  // **The door asks the server NOTHING** (R-6.5b2.7): `fetch` is the setup
  // file's throwing stub, so a door that asked the server for any part of
  // itself — the bot's label included — fails here rather than being slower.
  it("draws the games on this device, and asks the server for nothing", async () => {
    render(<FrontDoor multiplayer />);
    expect(await screen.findByRole("link", { name: LOCAL })).toHaveAttribute("href", `#/t/${LOCAL}`);
    expect(screen.getByRole("button", { name: "New game" })).toBeInTheDocument();
    expect(fetch).not.toHaveBeenCalled();
  });

  it("opens the local form from New game, which needs no door", async () => {
    render(<FrontDoor multiplayer />);
    await userEvent.setup().click(screen.getByRole("button", { name: "New game" }));
    expect(window.location.hash).toBe("#/new/local");
  });

  // **The bot's version, and never a request for it** (R-6.5c2.10, spec §2.4):
  // the label is the committed contract file's, which `wire_fixtures` refuses
  // when it is stale — so a re-pointed production bot changes this card at the
  // next regeneration with no edit here.
  it("labels the bot with the version this build ships, from the contract file", () => {
    render(<FrontDoor multiplayer />);
    const title = screen.getByRole("heading", { name: /You against the bot/ });
    expect(productionBot.label).not.toBe("");
    expect(within(title).getByText(productionBot.label)).toHaveClass("version-tag");
  });

  // The sentence a game left behind — a local table that no longer opens — is
  // shown where its siblings are listed, which for a local game is here.
  it("carries a sentence from the screen a game was left for", () => {
    render(<FrontDoor multiplayer notice="this game is no longer available" />);
    expect(screen.getByText("this game is no longer available")).toBeInTheDocument();
  });

  // **The project link** (R-6.5c2.6, spec §2.3): only where the build names
  // one, its text the address without the scheme, in a new tab.
  it("draws the project link only when the build names one, in a new tab", () => {
    const { unmount } = render(<FrontDoor multiplayer />);
    expect(document.querySelector(".project-link")).toBeNull();
    unmount();
    render(<FrontDoor multiplayer link={linkFor("https://github.com/you/yokohama-assistant")} />);
    // **The new tab is announced** (M6.5c2 cycle C, the a11y pass): its name
    // says so, in words a screen reader reads and the page does not draw —
    // the drawn arrow is decoration and reads as nothing.
    const link = screen.getByRole("link", { name: "github.com/you/yokohama-assistant (opens in a new tab)" });
    expect(within(link).getByText("(opens in a new tab)")).toHaveClass("sr-only");
    expect(link).toHaveAttribute("href", "https://github.com/you/yokohama-assistant");
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
  });

  // **The blurb** (R-6.5c2.40): the build's words sit between the title and the
  // link, and are absent where the build set none.
  it("draws the blurb between the title and the link, and none where the build set none", () => {
    const spy = vi.spyOn(values, "siteBlurb").mockReturnValue("First sentence. Second sentence.");
    const { unmount, container } = render(
      <FrontDoor multiplayer link={linkFor("https://github.com/you/yokohama-assistant")} />,
    );
    const blurb = container.querySelector("p.blurb");
    expect(blurb?.textContent).toBe("First sentence. Second sentence.");
    expect(container.querySelector("h1 + .blurb")).toBe(blurb);
    expect(blurb?.nextElementSibling).toHaveClass("project-link");
    unmount();
    spy.mockReturnValue(null);
    const again = render(<FrontDoor multiplayer />);
    expect(again.container.querySelector(".blurb")).toBeNull();
  });

  // **"How to play" opens the help in place, WITHOUT its screenshots**: they
  // are photographs of the card art (`ASSETS.md`, the `web/public/help/` row),
  // and no setup screen draws card art.
  it("opens the help in place, with its words and none of its pictures", async () => {
    const { container } = render(<FrontDoor multiplayer />);
    const user = userEvent.setup();
    const toggle = screen.getByRole("button", { name: "How to play" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    const panel = screen.getByRole("region", { name: "How to play" });
    expect(within(panel).getByRole("heading", { name: "The three parts of the screen" })).toBeInTheDocument();
    expect(container.querySelectorAll("img, figure.shot")).toHaveLength(0);
    expect(within(panel).getByRole("heading", { name: "How to play" })).toHaveFocus();
    await user.click(within(panel).getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("region", { name: "How to play" })).toBeNull();
  });

  // **Close hands focus back to what opened the help** (M6.5c2 cycle C, the
  // a11y pass): the panel and its Close are unmounted by the click, and focus
  // on a removed element falls to the document's body — a keyboard player was
  // sent back to the top of the page.
  it("returns focus to How to play when the help is closed", async () => {
    render(<FrontDoor multiplayer />);
    const user = userEvent.setup();
    const toggle = screen.getByRole("button", { name: "How to play" });
    await user.click(toggle);
    const panel = screen.getByRole("region", { name: "How to play" });
    expect(within(panel).getByRole("heading", { name: "How to play" }), "the premise: focus went in").toHaveFocus();
    await user.click(within(panel).getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("region", { name: "How to play" })).toBeNull();
    expect(toggle).toHaveFocus();
  });

  // **The build value, read once and never asked for.** Absent is ON, so a
  // deployment that sets nothing serves both channels; the one string that
  // turns it off is the literal `"false"`.
  it("reads the multiplayer entry off the build and nothing else", () => {
    expect(multiplayerOffered()).toBe(true);
    vi.stubEnv("VITE_MULTIPLAYER", "false");
    expect(multiplayerOffered()).toBe(false);
    // Anti-vacuity, and the reason the test is `!== "false"` rather than a
    // truthiness read: any other value is a deployment that did not ask for the
    // door to be shut.
    vi.stubEnv("VITE_MULTIPLAYER", "true");
    expect(multiplayerOffered()).toBe(true);
    vi.stubEnv("VITE_MULTIPLAYER", "0");
    expect(multiplayerOffered()).toBe(true);
  });
});


// **What the door says about the release** (M6.5b2, spec §4's four sentences).
//
// The status is a PROP, so these cells render each state directly rather than
// arranging a service worker jsdom does not have; `swRegister.test.ts` is where
// the lifecycle that produces each one is driven.
describe("the door's readiness line", () => {
  // **The line is the device card's since M6.5c2** (mock 01), so every cell
  // here mounts `OnThisDevice`, which starts the local host: the fake one.
  beforeEach(() => {
    resetLocalHostForTests();
    stubWorker();
  });
  afterEach(() => {
    resetLocalHostForTests();
  });

  const status = (over: Partial<ReadinessStatus> = {}): ReadinessStatus => ({
    readiness: "none",
    reason: null,
    updateWaiting: false,
    staleTab: false,
    applyUpdate: vi.fn(),
    reload: vi.fn(),
    retry: vi.fn(),
    ...over,
  });

  it("says the install is happening, and then that it is done", () => {
    const { unmount } = render(<FrontDoor multiplayer readiness={status({ readiness: "installing" })} />);
    expect(screen.getByText(/preparing to play offline/i)).toBeInTheDocument();
    unmount();
    render(<FrontDoor multiplayer readiness={status({ readiness: "ready" })} />);
    expect(screen.getByText(/ready to play offline/i)).toBeInTheDocument();
    expect(screen.queryByText(/preparing to play offline/i)).toBeNull();
  });

  // **A NAMED failure with a retry** (spec §4), because the install is
  // all-or-nothing and a door that only said "failed" would leave a player with
  // nothing to do and nothing to report.
  it("names a failure and offers the install again", async () => {
    const retry = vi.fn();
    render(
      <FrontDoor
        multiplayer
        readiness={status({ readiness: "failed", reason: "/art/orders/O01.webp answered 404", retry })}
      />,
    );
    expect(screen.getByText(/\/art\/orders\/O01\.webp answered 404/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /try again/i }));
    expect(retry).toHaveBeenCalledTimes(1);
  });

  // **One sentence, one full stop.** The browser's own registration sentence
  // ends in a period and the line appends one, so a refused `/sw.js` read
  // "…fetching the script.. Try again" (measured, PR #37's retry script). A
  // worker-reported reason carries no period and must keep the line's own.
  it("ends the failure line in ONE period whether or not the reason brings its own", () => {
    const line = (reason: string) => {
      const { container, unmount } = render(
        <FrontDoor multiplayer readiness={status({ readiness: "failed", reason })} />,
      );
      const text = container.querySelector(".offline-state p")?.textContent ?? "";
      unmount();
      return text;
    };
    expect(
      line("A bad HTTP response code (503) was received when fetching the script."),
    ).toBe(
      "Could not prepare to play offline — A bad HTTP response code (503) was received when fetching the script. Try again",
    );
    expect(line("/art/orders/O01.webp answered 404")).toBe(
      "Could not prepare to play offline — /art/orders/O01.webp answered 404. Try again",
    );
  });

  // "none" is a development build, a browser without service workers, or a page
  // the walk has flagged — three ways of having nothing to say, none of which
  // is a failure and none of which should read as one.
  it("says nothing at all when there is nothing to say", () => {
    render(<FrontDoor multiplayer readiness={status()} />);
    expect(screen.queryByText(/play offline/i)).toBeNull();
    render(<FrontDoor multiplayer />);
    expect(screen.queryByText(/play offline/i)).toBeNull();
  });

  // **The door's LINE switches only when it is pressed** (spec §2.3): the offer
  // is one quiet line, and this component does nothing until a player presses
  // it. **The App takes the release by itself on the door** since M6.5c2 cycle
  // F (R-F3; `useDoorAutoApply`, `swRegister.test.ts`'s "the door's own
  // apply"), so what a player sees on a live door is that line for a moment
  // and then a reload; the line stays the switch wherever the App's apply is
  // held, and it is the only one off the door.
  it("the door's release line switches only when it is pressed, and the App applies by itself", async () => {
    const applyUpdate = vi.fn();
    render(
      <FrontDoor multiplayer readiness={status({ readiness: "ready", updateWaiting: true, applyUpdate })} />,
    );
    const button = screen.getByRole("button", { name: /reload/i });
    expect(applyUpdate).not.toHaveBeenCalled();
    await userEvent.click(button);
    expect(applyUpdate).toHaveBeenCalledTimes(1);
  });

  it("offers no update when none is waiting", () => {
    render(<FrontDoor multiplayer readiness={status({ readiness: "ready" })} />);
    expect(screen.queryByRole("button", { name: /reload/i })).toBeNull();
  });
});

// ---- the door's two filled controls (cycle E, straw poll row 58) -----------
//
// `a.primary-link` is a navigation drawn in `button.primary`'s box (the door's
// "Go to the lobby", the lobby states' "Sign in"), and its own padding drew it
// ~1.6px taller and narrower than the New game beside it. **ONE declaration
// block now gives both their box**, and no other top-level rule gives the link
// a box of its own. Read from the source (jsdom loads no stylesheet), the
// lane's idiom (`BoardMark.test.tsx`); the widths are the headed look's.
describe("the door's two filled controls", () => {
  // The stylesheet's top-level rules: `test/css.ts`'s one parser.
  const BOX = /(^|[;\s])(padding|border|border-radius|background|font|font-weight)\s*:/;

  it("gives a.primary-link and button.primary their box in ONE declaration block", () => {
    const rules = topLevelRules(readFileSync("src/styles.css", "utf8"));
    const shared = rules.filter((r) => r.selectors.includes("button.primary") && r.selectors.includes("a.primary-link"));
    expect(shared, "one block names both").toHaveLength(1);
    expect(shared[0]!.body).toMatch(/padding\s*:\s*0\.45rem 1\.1rem/);
    // Anti-vacuity partner: no other top-level rule gives either its own box.
    const own = rules.filter(
      (r) =>
        r !== shared[0] &&
        (r.selectors.includes("a.primary-link") || r.selectors.includes("button.primary")) &&
        BOX.test(r.body),
    );
    expect(own.map((r) => r.selectors.join(", ")), "no second box for either").toEqual([]);
  });
});
