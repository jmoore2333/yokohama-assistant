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

/** The whole SECTION under one heading — every paragraph from it to the next
 *  heading.
 *
 *  **The section, not its first paragraph.** M6.1 split "Starting a game" in
 *  three (the door and the form, the set table, the choice), and a helper that
 *  took the first `<p>` would have pointed at the form and gone on passing for
 *  whatever it happened to find there. Bounded by the next heading rather than
 *  by a count, so a fourth paragraph joins the reading instead of escaping it.
 *
 *  **Parameterized since M6.4**, which is its third reader: two copies of this
 *  loop had already been written out, and a third would have been the cycle
 *  where one of them drifted. */
function section(name: string): string {
  const heading = screen.getByRole("heading", { name });
  let text = "";
  for (let el = heading.nextElementSibling; el; el = el.nextElementSibling) {
    if (/^H[1-6]$/.test(el.tagName)) break;
    text += ` ${el.textContent ?? ""}`;
  }
  if (!text.trim()) throw new Error(`nothing follows the '${name}' heading`);
  return text;
}

/** The section every set-table cell below reads. */
const startingSection = () => section("Starting a game");

describe("the help panel's set-table section", () => {
  // **R-C5, the sentence itself.** The bot's cards are face down and the record
  // is what knows which one it kept; the player's own Keep scores stay.
  it("says the bot's two cards are face down, with the space between the words", () => {
    render(<Help />);
    const text = startingSection();
    expect(text).toContain("two cards face down");
    // The anti-vacuity, and the actual defect: `toContain` on the fixed string
    // would also pass if the glued form appeared somewhere else in the same
    // section, and the glued form is what a reader sees.
    expect(text).not.toContain("cardsface");
    expect(text).toContain("Keep score");
    expect(text).not.toContain("bot keeps this");
  });

  // **What replaced R-C2's hot-seat pass (M6.1 §7.4).** The deal is served per
  // SEAT now, so the secrecy is the payload's rather than a sequence the player
  // has to be told to respect — and the page must not go on describing a "Next
  // seat" button that is gone. What a person holding several seats needs to know
  // is that the next one is dealt to them straight away.
  it("says you see only your own two cards, and that a second seat of yours follows", () => {
    render(<Help />);
    const text = startingSection();
    expect(text).toContain("your own two cards and nobody else");
    expect(text).toContain("more than one seat");
    expect(text).not.toContain("Next seat");
  });

  // The lobby's words, so a player waiting on somebody else knows the screen is
  // not stuck: the other seats read chosen or waiting, and the game opens by
  // itself when the last one answers.
  it("says what the other seats read, and that the game opens by itself", () => {
    render(<Help />);
    const text = startingSection();
    expect(text).toContain("chosen");
    expect(text).toContain("waiting on");
    expect(text).toContain("opens by itself");
  });

  // **R-C3's half.** What the set table has on it is the owner's edition at the
  // step where the two orders are dealt and chosen in secret: the areas, the
  // port and the tracks. The technologies and the goals go out at the two steps
  // after that, so the sentence must not still list them among what is on the
  // table when the cards are drawn.
  it("says the technologies and the goals come out after the choice", () => {
    render(<Help />);
    const text = startingSection();
    expect(text).toContain("the port and the tracks");
    expect(text).toContain("once you have chosen");
    expect(text).not.toContain("the port, the laboratory and the goals");
  });

  // **R-P10, on the page as well as on the form.** A player who wonders where
  // the bot went at three players should find the answer here rather than in a
  // 400.
  it("says the bot is offered at two players only", () => {
    render(<Help />);
    expect(startingSection()).toMatch(/bot is offered at two players only/);
  });
});

describe("the help panel and the lobby", () => {
  // **M6.2's shape, in the words on the screen.** The panel went a whole cycle
  // telling players their game was "in the list on the start screen" after the
  // start screen became the lobby, and nothing could see it: this file renders
  // the prose but read only the set-table section, and the two e2e specs that
  // open the panel assert nothing in its paragraphs.
  //
  // What is asserted is the three things a player cannot do without — where
  // their tables are, how somebody else gets into one, and how to get back —
  // plus the two retired sentences, so the next cycle's stale prose fails here
  // rather than shipping.
  it("names the lobby's two lists, the join link and the password", () => {
    const { container } = render(<Help />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/\blobby\b/i);
    expect(text).toContain("Your games");
    expect(text).toContain("Open tables");
    expect(text).toMatch(/link/i);
    expect(text).toMatch(/password/i);
    expect(text).toMatch(/abandon/i);
    expect(text).not.toContain("the list on the start screen");
    expect(text).not.toContain("back to the start screen");
  });

  // **The name field, in the paragraph that describes the LOBBY** (spec §6.6's
  // third edit, ruling R-7-1). The same fact is stated again in the Talking
  // section, deliberately — a player setting up a table and a player about to
  // say something both need it, and neither reads the other's section first.
  //
  // So this cell is SCOPED, and that is the whole of its value: read over the
  // whole panel it would be satisfied by the Talking section's copy and would
  // go on passing with the lobby's own sentence gone.
  it("says in the lobby's own paragraph what the name field is for", () => {
    render(<Help />);
    const text = section("Starting a game");
    expect(text).toContain("Your name");
    // Named by where it sits, because that is how a reader finds it — and bound
    // to THIS sentence rather than to the two words. The same section already
    // carries "You are shown your own two cards", so a bare `"You are"` went on
    // passing with the new sentence gone (final review, minor 13's sibling).
    expect(text).toContain("Beside You are at the top of the lobby");
    expect(text).toContain("this browser remembers it");
    expect(text).toContain("leave it blank and your address is used");
    // And it is not the form's own display name, which the next paragraph
    // offers a reader could take for the same control.
    expect(text).toContain("not the display name on the form below");
  });

  // **The lobby has THREE lists since M6.3** (final review, minor 8): this
  // paragraph said two while the Talking section three paragraphs down said
  // three, on the same page. The needle is the count AND the section that was
  // missing, so a page that fixed the number without naming the list fails.
  it("names all three of the lobby's lists", () => {
    render(<Help />);
    const text = section("Starting a game");
    expect(text).toContain("three lists");
    expect(text).toContain("Your games");
    expect(text).toContain("Open tables");
    expect(text).toContain("Games in progress");
    expect(text).not.toContain("two lists");
  });
});

describe("the help panel and watching (R-8.1)", () => {
  /** The whole "Watching a game" SECTION — the shared reader, named here so
   *  every cell below says which section it is about. */
  const watchingSection = () => section("Watching a game");

  // **The two ways in.** A player who wonders how to watch somebody else's
  // game should find both here: the lobby's third list, and a link somebody
  // sent them. The list's heading is quoted exactly, so a rename of the section
  // fails here rather than leaving the page pointing at a heading that is gone.
  it("names the lobby's third list and the link as the two ways in", () => {
    render(<Help />);
    const text = watchingSection();
    expect(text).toContain("Games in progress");
    expect(text).toMatch(/link/i);
  });

  // The password, asked ONCE — which is what a watcher needs to know before
  // they decide the lock marker means the table is shut to them.
  it("says a locked table asks for its password once", () => {
    render(<Help />);
    const text = watchingSection();
    expect(text).toMatch(/password/i);
    expect(text).toContain("once");
  });

  // **What the screen is, and what it is not.** The three facts a watcher would
  // otherwise have to discover by trying: the strip, the hidden hands, and that
  // there is nothing to press.
  // **And a watcher can TALK (M6.4, R-6.4.1).** The one write a spectator is
  // allowed, so a page that listed everything they cannot do and stopped there
  // would be describing the screen they had before this cycle.
  it("says a watcher can talk, and that what they say is marked", () => {
    render(<Help />);
    const text = watchingSection();
    expect(text).toContain("You can talk");
    expect(text).toContain("watcher");
  });

  it("says what watching looks like and what it does not offer", () => {
    render(<Help />);
    const text = watchingSection();
    expect(text).toContain("You are watching");
    // Every hand is a COUNT — the one thing a spectator must not be able to
    // read, stated so nobody wonders whether the screen is merely folded.
    expect(text).toMatch(/count/i);
    expect(text).toContain("as they happen");
    expect(text).toMatch(/cannot play/i);
    expect(text).toMatch(/undo/i);
    expect(text).toMatch(/suggest/i);
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
describe("the help panel and talking (M6.4)", () => {
  // **Where it is and that it is SHUT.** A panel folded by default is a panel a
  // player may never find, so the page has to say where it sits and that the
  // number on its header is what arrived while it was closed.
  it("names the panel, its fold and the foot of the column", () => {
    render(<Help />);
    const text = section("Talking");
    expect(text).toContain("foot of the side column");
    expect(text).toContain("Chat");
    expect(text).toContain("folded until you open it");
    expect(text).toContain("count of what has arrived");
  });

  // **Who is labelled how (R-6.4.2), which is the one thing a reader cannot
  // work out from the screen**: a seat is named by its seat and a watcher is
  // marked, so a name typed into the lobby is not what a seated player's line
  // carries.
  it("says how a seat and a watcher are labelled", () => {
    render(<Help />);
    const text = section("Talking");
    expect(text).toContain("seat");
    expect(text).toContain("colour");
    expect(text).toContain("watching");
  });

  // **What a message is NOT (R-6.4.9, R-6.4.13).** Not undone, not in the
  // record, not kept for ever, and gone with the table — the four facts that
  // decide whether a player treats the panel as a place to agree a rule.
  it("says a message is never undone and never part of the record", () => {
    render(<Help />);
    const text = section("Talking");
    expect(text).toContain("never undone");
    expect(text).toContain("never part of the game");
    expect(text).toContain("drops the oldest");
    expect(text).toContain("abandoned");
    // **Whose number it is, not what it is** (final review, minor 8). The
    // retention is a launch variable and may be `all` or twenty, so the page
    // says who sets it rather than naming a figure it cannot know.
    expect(text).toContain("whoever runs it sets how many");
    expect(text).not.toContain("a few hundred");
  });

  // **The lobby's own chat and its switch (R-6.4.6), and the name that feeds
  // both.** An operator can turn the house chat off, and then the section is
  // simply not there — which a player who had read about it would otherwise
  // report as a bug.
  it("names the lobby chat, its switch and the name field", () => {
    render(<Help />);
    const text = section("Talking");
    expect(text).toContain("Lobby chat");
    expect(text).toContain("switch it off");
    // The field the name comes from, named by where it sits rather than by its
    // label — the label is the lobby's own cell, and this is the sentence that
    // tells a reader which of the four fields feeds their messages.
    expect(text).toContain("The name you type beside");
    expect(text).toContain("your address is used");
  });
});
