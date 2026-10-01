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
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { ACCESS_CHALLENGE } from "./api";
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
