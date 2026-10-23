// The refusal strip, and the one refusal that comes with a way out of it
// (M6.5b2, spec §2.1 and §4; final review I-3).
//
// **Two cells and they prove different things.** The first is the component:
// the expired-session sentence draws `/enter` beside it and every other
// sentence is drawn as it always was. The second is the FENCE, and it is the
// one this file exists for — the component was right from the day it was
// written, and the link still reached four strips of about seventeen, because
// every other host drew `<div className="error">{error}</div>` of its own. A
// cell per host would have said the five hosts the review happened to name
// were fixed; this one says no host draws one at all, including the hosts
// nobody has written yet.
//
// **It reads the SOURCE**, in this lane's own idiom (`precache.test.ts` reads
// `assets.rs` arm for arm; `platform_calls_stay_behind_the_server_feature`
// reads the crate). A rendered-tree assertion cannot be written for screens
// that do not exist.
import { readdirSync, readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { ACCESS_CHALLENGE, SIGN_IN_NEEDED } from "./api";
import { Refusal } from "./Refusal";

describe("the refusal strip", () => {
  it("draws the turnstile's link for the expired session and for nothing else", () => {
    const { unmount } = render(<Refusal sentence={ACCESS_CHALLENGE} />);
    expect(screen.getByText(ACCESS_CHALLENGE, { exact: false })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter");
    unmount();

    // The anti-vacuity partner: an ordinary refusal is the server's own
    // sentence and `/enter` would do nothing about it.
    render(<Refusal sentence="no game named &quot;wire&quot;" />);
    expect(screen.queryByRole("link", { name: "Sign in again" })).toBeNull();
  });

  // **The auth cycle's sign-in (Task 6; design §4.3)**: a provider-signed
  // Worker's 401 reads as `SIGN_IN_NEEDED`, and the way out is `/enter` with the
  // screen the player is on as `return_to` — a NAVIGATION, never a fetch, which
  // starts the sign-in and lands the player back where they were. The Worker
  // accepts only `/#/…` there (Task 4's `safeReturnTo`), so the hash goes with a
  // leading `/`; no hash is the lobby. Access's "Sign in again" keeps its bare
  // `/enter`: Access ignores the query and lands every sign-in on the lobby.
  // Mutants: the hash sent without its leading `/`; the hash unencoded; no
  // `return_to` at all; the link drawn for an ordinary sentence.
  describe("beside SIGN_IN_NEEDED", () => {
    afterEach(() => {
      window.location.hash = "";
    });

    it("draws the /enter link beside SIGN_IN_NEEDED, with return_to the current hash", () => {
      window.location.hash = "#/t/abc-123";
      const { unmount } = render(<Refusal sentence={SIGN_IN_NEEDED} />);
      expect(screen.getByText(SIGN_IN_NEEDED, { exact: false })).toBeInTheDocument();
      expect(screen.getByRole("link", { name: "Sign in" })).toHaveAttribute(
        "href",
        "/enter?return_to=%2F%23%2Ft%2Fabc-123",
      );
      expect(screen.queryByRole("link", { name: "Sign in again" })).toBeNull();
      unmount();

      // No route at all: the lobby.
      window.location.hash = "";
      const lobby = render(<Refusal sentence={SIGN_IN_NEEDED} />);
      expect(screen.getByRole("link", { name: "Sign in" })).toHaveAttribute(
        "href",
        `/enter?return_to=${encodeURIComponent("/#/lobby")}`,
      );
      lobby.unmount();

      // Access's expired session keeps its own words and its bare link.
      window.location.hash = "#/t/abc-123";
      render(<Refusal sentence={ACCESS_CHALLENGE} />);
      expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter");
      expect(screen.queryByRole("link", { name: "Sign in" })).toBeNull();
    });

    // **R-A14, the transparency notice**: who signs the player in, and what this
    // site keeps — read BEFORE the click, directly under the link. A per-host
    // build value; a build with none (a self-host with no provider) draws
    // nothing at all, not an empty line. Mutants: the notice drawn when unset;
    // the notice drawn beside an ordinary sentence; a class other than `muted`.
    it("draws the build's sign-in notice under the link, and nothing when the build sets none", () => {
      const NOTICE = "Sign-in is by WorkOS. This site keeps only your verified email address, to seat you.";
      vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", NOTICE);
      const { container, unmount } = render(<Refusal sentence={SIGN_IN_NEEDED} />);
      const notice = screen.getByText(NOTICE);
      expect(notice.tagName).toBe("P");
      expect(notice).toHaveClass("muted");
      // Under the link, inside the strip.
      const link = screen.getByRole("link", { name: "Sign in" });
      expect(link.compareDocumentPosition(notice) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(container.querySelector(".error")).toContainElement(notice);
      unmount();

      // Every sign-in link the strip draws carries it: Access's too.
      const expired = render(<Refusal sentence={ACCESS_CHALLENGE} />);
      expect(screen.getByText(NOTICE)).toHaveClass("muted");
      expired.unmount();

      render(<Refusal sentence="no game named wire" />);
      expect(screen.queryByText(NOTICE), "an ordinary refusal names no sign-in").toBeNull();

      vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", null);
      const bare = render(<Refusal sentence={SIGN_IN_NEEDED} />);
      expect(bare.container.querySelector("p"), "no notice set, nothing drawn").toBeNull();
    });
  });

  it("keeps the class its host already drew", () => {
    const { container } = render(<Refusal sentence="x" className="error restore-error" />);
    expect(container.querySelector(".error.restore-error")).not.toBeNull();
  });
});

describe("every error strip in this client", () => {
  // The strips this fence is about are the `<div>` ones: a screen that reports
  // a refusal it was HANDED. `BackupControls`'s three `<p className="error">`
  // lines are this browser's own sentences about its own database, they are
  // wrapped prose rather than a strip, and `/enter` says nothing about them —
  // so the pattern below is deliberately the div and not the class.
  /** vitest runs from `web/`, which is where `precache.test.ts` reads the Rust
   *  crate from too. */
  const SRC = "src";
  const strip = /<div className="error"/;

  it("goes through Refusal, so the expired-session sentence never loses its link", () => {
    const sources = readdirSync(SRC).filter(
      (f) => f.endsWith(".tsx") && !f.endsWith(".test.tsx") && f !== "Refusal.tsx",
    );
    // **The premise, so a mis-pathed read could not pass as a clean one**: the
    // scan really is looking at this client's screens, including the five the
    // review named as the places a session expires mid-flow.
    expect(sources).toEqual(
      expect.arrayContaining(["Dealing.tsx", "OpenTable.tsx", "NewGame.tsx", "Unlock.tsx", "Chat.tsx"]),
    );
    const read = (f: string) => readFileSync(`${SRC}/${f}`, "utf8");
    expect(
      sources.filter((f) => strip.test(read(f))),
      "these draw a refusal without the way out of one",
    ).toEqual([]);
    // And they draw it through this component rather than having dropped the
    // strip: every screen that reports a sentence it was handed imports it.
    for (const f of ["Dealing.tsx", "OpenTable.tsx", "NewGame.tsx", "Unlock.tsx", "Chat.tsx"]) {
      expect(read(f), f).toContain('from "./Refusal"');
    }
  });
});
