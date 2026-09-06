// The help panel's PROSE, which nothing rendered until now.
//
// **The defect that forced this file was invisible to every gate.** A line
// ending `...own two cards are` followed by a line starting `<strong>face
// down</strong>` renders as "cards areface down": JSX drops a newline that sits
// next to a tag, and the file's other end-of-line-before-a-tag sites carry an
// explicit `{" "}` for exactly that reason. `typecheck` cannot see it, the
// component suite never rendered `Help`, and the two e2e specs that open the
// panel assert nothing in its paragraphs — so a sentence R-C5 names could ship
// glued together with every suite green.
//
// The second cell is the general form of the same defect, and is the one worth
// having: it walks every `<strong>`/`<em>` in the rendered panel and asks
// whether the text before it ends in a letter while its own text begins with
// one. That is the whole family, not this one sentence.

import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Help } from "./Help";

/** The set-table paragraph, found by what it is about rather than by position:
 *  a heading order that changed would otherwise silently point this at another
 *  paragraph and keep passing. */
function setTableParagraph(): HTMLElement {
  const heading = screen.getByRole("heading", { name: "Starting a game" });
  let el = heading.nextElementSibling;
  while (el && el.tagName !== "P") el = el.nextElementSibling;
  if (!el) throw new Error("no paragraph follows the 'Starting a game' heading");
  return el as HTMLElement;
}

describe("the help panel's set-table paragraph", () => {
  // **R-C5, the sentence itself.** The bot's cards are face down and the record
  // is what knows which one it kept; the player's own Keep scores stay.
  it("says the bot's two cards are face down, with the space between the words", () => {
    render(<Help />);
    const text = setTableParagraph().textContent ?? "";
    expect(text).toContain("two cards are face down");
    // The anti-vacuity, and the actual defect: `toContain` on the fixed string
    // would also pass if the glued form appeared somewhere else in the same
    // paragraph, and the glued form is what a reader sees.
    expect(text).not.toContain("areface");
    expect(text).toContain("Keep score");
    expect(text).not.toContain("bot keeps this");
  });

  // **R-C2's half.** One browser serves every seat at a hot-seat table, so the
  // secrecy is the sequence, and the paragraph has to say so — a player who
  // does not know the screen is passed on will hand it over with their own
  // cards still up.
  it("says the human seats choose in turn and the others stay face down", () => {
    render(<Help />);
    const text = setTableParagraph().textContent ?? "";
    expect(text).toContain("each seat chooses in turn");
    expect(text).toContain("Next seat");
    expect(text).toMatch(/only the seat being asked has its cards face up/);
  });

  // **R-C3's half of the same paragraph.** What the set table has on it is the
  // owner's edition at the step where the two orders are dealt and chosen in
  // secret: the areas, the port and the tracks. The technologies and the goals
  // go out at the two steps after that, so the sentence must not still list
  // them among what is on the table when the cards are drawn.
  it("says the technologies and the goals come out after the choice", () => {
    render(<Help />);
    const text = setTableParagraph().textContent ?? "";
    expect(text).toContain("the port and the tracks");
    expect(text).toContain("once you have chosen");
    expect(text).not.toContain("the port, the laboratory and the goals");
  });
});

describe("the help panel's prose", () => {
  // Every emphasis in the panel, not just the one that broke. A tag preceded by
  // a letter and beginning with a letter is a missing `{" "}` — the JSX
  // whitespace rule, read off the rendered DOM rather than off the source.
  //
  // Punctuation boundaries are legitimate and are not flagged: an opening
  // quote, a bracket or an em dash before the tag all end in a non-letter.
  it("never glues a word to an emphasised word", () => {
    const { container } = render(<Help />);
    const glued: string[] = [];
    for (const el of container.querySelectorAll("strong, em")) {
      const own = el.textContent ?? "";
      const before = el.previousSibling;
      const prior = before?.nodeType === Node.TEXT_NODE ? (before.textContent ?? "") : "";
      if (/[A-Za-z]$/.test(prior) && /^[A-Za-z]/.test(own)) {
        glued.push(`…${prior.slice(-24)}|${own}`);
      }
      const after = el.nextSibling;
      const next = after?.nodeType === Node.TEXT_NODE ? (after.textContent ?? "") : "";
      if (/[A-Za-z]$/.test(own) && /^[A-Za-z]/.test(next)) {
        glued.push(`${own}|${next.slice(0, 24)}…`);
      }
    }
    expect(glued, "a missing {\" \"} before or after an emphasised word").toEqual([]);
  });

  // Anti-vacuity for the cell above: it reads zero when the panel renders
  // nothing at all, which is exactly what a broken import would do.
  it("renders the panel it is checking", () => {
    const { container } = render(<Help />);
    expect(container.querySelectorAll("strong, em").length).toBeGreaterThan(10);
    expect(within(container).getAllByRole("heading").length).toBeGreaterThan(3);
  });
});
