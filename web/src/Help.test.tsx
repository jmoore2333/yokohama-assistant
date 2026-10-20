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
    expect(text).toContain("Your tables");
    expect(text).toContain("Open tables");
    expect(text).toMatch(/link/i);
    expect(text).toMatch(/password/i);
    expect(text).toMatch(/abandon/i);
    expect(text).not.toContain("the list on the start screen");
    expect(text).not.toContain("back to the start screen");
  });

  // **The first screen is the DOOR since M6.5b2** (R-6.5b2.1), and a page that
  // went on opening "Sign in first" would be describing the screen this cycle
  // replaced — the exact drift this file exists for. What a reader needs is the
  // door's cards by the names they carry (M6.5c2, mock 01), and that the lobby
  // is somewhere they can get back FROM.
  it("names the door's ways on, and the lobby at its own address", () => {
    render(<Help />);
    const text = section("Starting a game");
    expect(text).toContain("You against the bot");
    expect(text).toContain("Play with friends");
    expect(text).toContain("On this device");
    expect(text).toContain("link back to the door");
    // The door's words before M6.5c2, gone with the controls they named.
    expect(text).not.toContain("Play with others");
    expect(text).not.toContain("Play on this device");
    expect(text).not.toContain("Sign in first");
  });

  // **The corner, in the paragraph that describes the LOBBY** (spec §6.6's
  // third edit, ruling R-7-1). The same fact is stated again in the Talking
  // section, deliberately — a player setting up a table and a player about to
  // say something both need it, and neither reads the other's section first.
  //
  // So this cell is SCOPED, and that is the whole of its value: read over the
  // whole panel it would be satisfied by the Talking section's copy and would
  // go on passing with the lobby's own sentence gone.
  it("says in the lobby's own paragraph what the corner is for", () => {
    render(<Help />);
    const text = section("Starting a game");
    expect(text).toContain("playing as");
    expect(text).toContain("switch player");
    expect(text).toContain("change name");
    expect(text).toContain("this browser remembers it");
    expect(text).toContain("leave it blank and your address is used");
    expect(text).not.toContain("Beside You are at the top of the lobby");
  });

  // **The lobby has THREE lists since M6.3** (final review, minor 8): this
  // paragraph said two while the Talking section three paragraphs down said
  // three, on the same page. The needle is the count AND the section that was
  // missing, so a page that fixed the number without naming the list fails.
  // **And the finished games folded under them** (M6.5c2 cycle C,
  // R-6.5c2.22): somebody else's, under Finished, with View.
  it("names all three of the lobby's lists, and the finished games folded under them", () => {
    render(<Help />);
    const text = section("Starting a game");
    expect(text).toContain("three lists");
    expect(text).toContain("Your tables");
    expect(text).toContain("Open tables");
    expect(text).toContain("Being played");
    expect(text).not.toContain("two lists");
    // The list closes before New table, and Finished takes a sentence of its
    // own (Task 10's M3): the fold is not a clause inside the three lists.
    expect(text).toContain("the ones you can spectate \u2014 and New table, which is the form.");
    expect(text).toContain("Other people\u2019s finished games sit folded under Finished, with View.");
  });
});

describe("the help panel and spectating (R-8.1; M6.5c2 R-6.5c2.7)", () => {
  /** The whole "Spectating a game" SECTION — the shared reader. */
  const spectatingSection = () => section("Spectating a game");

  // **The ways in**: the lobby's third list, quoted as the lobby itself names
  // it (mock 02); the Finished fold under it since M6.5c2 cycle C
  // (R-6.5c2.22), whose rows say View; and a link somebody sent. Being played
  // no longer holds a finished game, so the page no longer says it does.
  it("names the lobby's third list, the Finished fold under it, and the link as the ways in", () => {
    render(<Help />);
    const text = spectatingSection();
    expect(text).toContain("Being played");
    expect(text).toContain("still under way");
    // The caller's OWN finished games stay under Your tables, so the fold is
    // other people's, as the lobby paragraph says (the whole-branch review's
    // M-3): not "every game already over".
    expect(text).toContain("Finished, folded below it, holds other people\u2019s finished games, and View opens one");
    expect(text).not.toContain("every game already over");
    expect(text).not.toContain("already finished");
    expect(text).toMatch(/link/i);
  });

  it("says a locked table asks for its password once", () => {
    render(<Help />);
    const text = spectatingSection();
    expect(text).toMatch(/password/i);
    expect(text).toContain("once");
  });

  // **And a spectator can TALK (M6.4, R-6.4.1)** — the one write they are allowed.
  it("says a spectator can talk, and that what they say is marked", () => {
    render(<Help />);
    const text = spectatingSection();
    expect(text).toContain("You can talk");
    expect(text).toContain("spectator");
  });

  it("says what spectating looks like and what it does not offer", () => {
    render(<Help />);
    const text = spectatingSection();
    expect(text).toContain("You are spectating");
    expect(text).toMatch(/count/i);
    expect(text).toContain("as they happen");
    expect(text).toMatch(/cannot play/i);
    expect(text).toMatch(/undo/i);
    expect(text).toMatch(/suggest/i);
  });

  // **Every word a person reads is the spectate family** (R-6.5c2.7): the WHOLE
  // panel, because a section-scoped cell would pass with "watch" left in the
  // lobby's paragraph or the Talking one. The wire's `watching` and `can_watch`
  // are protocol keys and appear nowhere on this page.
  it("says spectate and never watch, anywhere on the page", () => {
    const { container } = render(<Help />);
    expect(container.textContent).not.toMatch(/\bwatch/i);
    expect(container.textContent).toMatch(/\bspectat/i);
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
  // work out from the screen**: a seat is named by its seat and a spectator is
  // marked, so a name typed into the lobby is not what a seated player's line
  // carries.
  it("says how a seat and a spectator are labelled", () => {
    render(<Help />);
    const text = section("Talking");
    expect(text).toContain("seat");
    expect(text).toContain("colour");
    expect(text).toContain("spectating");
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
  it("names the lobby chat, its switch and where the name comes from", () => {
    render(<Help />);
    const text = section("Talking");
    expect(text).toContain("Lobby chat");
    expect(text).toContain("switch it off");
    // Where the name comes from, named by where it sits — the lobby's corner —
    // and bound to THIS sentence, the one that tells a reader which name their
    // messages carry (`&rsquo;` renders as the typographic apostrophe).
    expect(text).toContain("The name in the lobby\u2019s corner");
    expect(text).toContain("your address is used");
  });

  // **The games in this browser** (M6.5b, R-6.5b.6, R-6.5b.7). Three facts a
  // player cannot discover by looking: the file save is offered at ANY point
  // rather than only at the end, a finished game can be sent to the server's
  // archive, and abandoning one sends it there first. Read from the SAVING
  // section, so a sentence about it somewhere else does not satisfy this.
  it("says a game on this device is saved as a file at any time, sent to the archive, or abandoned and saved", () => {
    render(<Help />);
    const text = section("Saving, and coming back");
    expect(text).toContain("On this device");
    expect(text).toContain("at any point");
    expect(text).toContain("Send to the archive");
    // And where it LIVES, which is the fact that decides whether a player
    // treats the browser as a backup: nowhere else, until they send or save it.
    expect(text).toContain("only in this browser");
  });

  it("says where an abandoned game goes when there is an archive, and how a finished one is saved", () => {
    render(<Help />);
    const saving = screen.getByRole("heading", { name: "Saving, and coming back" }).parentElement!;
    expect(saving.textContent).toMatch(/keeps an archive/);
    expect(saving.textContent).toMatch(/saved there before it is abandoned/);
    expect(saving.textContent).toMatch(/Abandon and save/);
    expect(saving.textContent).toMatch(/Save game file/);
  });

  // **The creator's two controls before the game starts** (security S3, D9 and
  // design §3.2 rule 3): Abandon on the table's own screen while it is filling
  // AND while it deals, and Reopen beside a seat somebody else took while it is
  // still filling — which nothing on the screen explains, and whose limit (not
  // once the cards are dealt) a player cannot discover by looking.
  it("says a creator abandons from the table's screen while it fills or deals, and reopens a taken seat only while it fills", () => {
    render(<Help />);
    const text = section("Saving, and coming back");
    expect(text).toMatch(/while it is still filling or dealing/);
    expect(text).toMatch(/Reopen/);
    expect(text).toMatch(/until the cards are dealt/i);
    // A seat the creator BOUND for a named person is reopened too, not only one
    // somebody joined.
    expect(text).toMatch(/whether they joined or you named them for it/);
  });

  // **The door draws the help WITHOUT its screenshots** (M6.5c2): they are
  // photographs of the card art (`ASSETS.md`, the `web/public/help/` row), and
  // no setup screen draws card art. The game's surface keeps them.
  it("draws its screenshots unless its host asks for the words alone", () => {
    const { container, unmount } = render(<Help />);
    expect(container.querySelectorAll("figure.shot")).toHaveLength(5);
    unmount();
    const words = render(<Help shots={false} />);
    expect(words.container.querySelectorAll("figure.shot, img")).toHaveLength(0);
    expect(words.container.textContent).toContain("The three parts of the screen");
  });
});
