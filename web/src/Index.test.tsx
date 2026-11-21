// The card indexes and the final score, over server-emitted payloads.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { IndexView, OutcomeView } from "./Index";
import { SURFACES } from "./Surfaces";
import { spaceWords } from "./Cards";
import { wire } from "./wire/contract";
import type { PlayerColor, SpaceReward } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function index(kind: string, first = wire.indexOrders) {
  return render(
    <IndexView
      first={first}
      kind={kind}
      game="wire"
      art={wire.artManifest}
      labels={wire.labels}
      colors={COLORS}
    />,
  );
}

describe("an index is a view, and it says how much of the population it shows", () => {
  it.each([
    ["orders", wire.indexOrders, 36],
    ["techs", wire.indexTechs, 20],
    ["goals", wire.indexGoals, 3],
    ["sites", wire.indexSites, 24],
  ])("%s draws its whole population without throwing", (kind, model, total) => {
    index(kind, model);
    expect(screen.getByText(`showing ${total} of ${total}`)).toBeInTheDocument();
  });

  it("numbers nothing, because this API has no card capture", () => {
    // In the terminal an index open inside a capture numbers its cards. Here
    // the input grammar refuses the index words outright, so `target` is null
    // and so is every selection — and a number on screen would be a number that
    // answers nothing.
    for (const m of [wire.indexOrders, wire.indexTechs, wire.indexGoals, wire.indexSites]) {
      expect(m.target).toBeNull();
      for (const g of m.groups) for (const c of g.cards) expect(c.selection).toBeNull();
    }
  });

  it("shows how close EVERY seat is, counted per card, which the nav blurb promises", () => {
    // **The first draft did not show it at all, and the blurb said it did** —
    // the M2.5 lesson in miniature. The second draft showed it but the TEST
    // could not tell: it asserted each `current/needed` string appeared
    // somewhere on screen, and in this fixture seat 1's strings are all
    // duplicated by some seat-0 row, so rendering one seat satisfied it.
    // A reviewer proved that by slicing the progress list to one entry and
    // watching all ten cells pass.
    //
    // So this counts ROWS PER CARD instead. `current >= needed` is "ready by
    // the numbers" and NOT "claimable" — a claim also needs an assistant in
    // hand — which is why the bar fills and nothing says "claim this".
    const { container } = index("goals", wire.indexGoals);
    const cards = container.querySelectorAll("li.index-card");
    const goals = wire.indexGoals.groups.flatMap((g) => g.cards);
    expect(cards).toHaveLength(goals.length);
    expect(goals.length).toBeGreaterThan(0);

    goals.forEach((c, i) => {
      const face = c.face;
      if (!("Achievement" in face)) throw new Error("the goals index carries achievements");
      const rows = cards[i]!.querySelectorAll(".progress-row");
      expect(rows, `goal ${i} shows one row per seat`).toHaveLength(
        face.Achievement.progress.length,
      );
      face.Achievement.progress.forEach(([seat, current, needed], j) => {
        void seat;
        expect(rows[j]!.querySelector(".num")!.textContent).toBe(`${current}/${needed}`);
      });
    });
  });

  it("words every location the corpus carries, and never prints a raw Rust tag", () => {
    // **The defect this replaces.** `locationWords` handled five of eight
    // `CardLocation` variants and fell through to `String(tag)`, so ten of the
    // twenty-four site cards captioned themselves with the literal word
    // `OnArea` — where the terminal prints `on area A03`. `types.ts` ended in a
    // permissive `Record<string, unknown>` arm, so neither the typecheck nor
    // the contract could see the missing variant. The arm is gone, the switch is
    // exhaustive with a `never` check, and this walks the whole corpus.
    const tags = new Set<string>();
    for (const [kind, model] of [
      ["orders", wire.indexOrders],
      ["techs", wire.indexTechs],
      ["goals", wire.indexGoals],
      ["sites", wire.indexSites],
    ] as const) {
      const { container, unmount } = index(kind, model);
      for (const g of model.groups) {
        for (const c of g.cards) {
          if (c.location && typeof c.location === "object") {
            tags.add(Object.keys(c.location)[0]!);
          } else if (typeof c.location === "string") {
            tags.add(c.location);
          }
        }
      }
      // No caption anywhere is a bare serde tag.
      const text = container.textContent ?? "";
      for (const tag of ["OnArea", "OwnedBy", "PortSlot", "LabSlot", "AchievementSlot", "Deal"]) {
        expect(text.includes(tag), `${kind} prints the raw tag ${tag}`).toBe(false);
      }
      unmount();
    }
    // Anti-vacuity: the corpus must actually CARRY located cards, or the loop
    // above proves nothing.
    expect(tags.size).toBeGreaterThan(0);
    expect(tags.has("OnArea")).toBe(true);
  });

  it("says where EVERY copy of a technology title is, which the nav blurb promises", () => {
    // Ten of the twenty titles circulate as two copies. The first draft showed
    // only the card's single `location` — the FIRST copy's — so a title owned by
    // both seats read as owned by one.
    const { container } = index("techs", wire.indexTechs);
    const cards = container.querySelectorAll("li.index-card");
    const titles = wire.indexTechs.groups.flatMap((g) => g.cards);
    let doubled = 0;
    titles.forEach((c, i) => {
      const face = c.face;
      if (!("Tech" in face)) throw new Error("the techs index carries techs");
      if (face.Tech.placed.length > 1) doubled += 1;
      const rows = cards[i]!.querySelectorAll(".copies li");
      const unseen = face.Tech.copies.length - face.Tech.placed.length;
      expect(rows.length, `title ${i} lists every copy it can`).toBe(
        face.Tech.placed.length + (unseen > 0 ? 1 : 0),
      );
    });
    expect(doubled, "the fixture must contain a title held by two seats").toBeGreaterThan(0);
  });

  it("shows what every space on a site card pays, which the nav blurb promises", () => {
    // The first draft printed a COUNT of spaces and no reward, so all 24 site
    // cards rendered identically.
    const { container } = index("sites", wire.indexSites);
    const cards = container.querySelectorAll("li.index-card");
    const sites = wire.indexSites.groups.flatMap((g) => g.cards);
    sites.forEach((c, i) => {
      const face = c.face;
      if (!("Site" in face)) throw new Error("the sites index carries sites");
      const chips = cards[i]!.querySelectorAll(".chip");
      // One trading-house chip plus at least one per paying space.
      expect(chips.length, `site ${i} shows its rewards`).toBeGreaterThan(1);
    });
    // And the number on the card is the 0-BASED registry index every other
    // surface prints — a player who clicks a board tile reading `Site 2` must
    // land on the card labelled `Site 2`.
    const first = sites[0]!.face;
    if (!("Site" in first)) throw new Error("sites");
    expect(container.textContent).toContain(`Site ${first.Site.index}`);
    expect(first.Site.index).toBe(0);
  });

  it("sends the filter to the SERVER rather than narrowing in the browser", async () => {
    // There is exactly one card-entry grammar in this project. A client-side
    // filter would be a second one, and the two would disagree on the first
    // ambiguous prefix a player typed.
    const fetchMock = vi.fn(
      async (input: RequestInfo | URL): Promise<Response> => {
        void input;
        return new Response(JSON.stringify(wire.indexFiltered), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      },
    );
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    index("orders");
    await user.type(screen.getByLabelText("Filter"), "copper");
    await screen.findByText(`showing ${wire.indexFiltered.shown} of ${wire.indexFiltered.total}`);
    const url = String(fetchMock.mock.calls.at(-1)?.[0]);
    expect(url).toContain("/index/orders");
    expect(url).toContain("filter=copper");
  });
});

describe("every nav blurb describes what its panel actually does", () => {
  // **The M2.5 lesson, made mechanical for the one thing that can be.** Three of
  // these blurbs were wrong at once: goals promised "how close every seat is"
  // over a panel that showed no progress, and both card indexes named an
  // ORDERING ("by printed corner index", "by gear cost") that the server does
  // not group by — it groups by country. A blurb is a claim, and the review that
  // found the first one is the reason the other two were looked for.
  it.each([
    ["orders", wire.indexOrders],
    ["techs", wire.indexTechs],
    ["goals", wire.indexGoals],
    ["sites", wire.indexSites],
  ] as const)("%s: the blurb does not name an ordering the payload contradicts", (kind, model) => {
    const blurb = SURFACES.find((s) => s.key === kind)!.blurb;
    const tags = new Set(
      model.groups.map((g) =>
        g.key && typeof g.key === "object" ? Object.keys(g.key)[0] : String(g.key),
      ),
    );
    if (tags.has("Country")) {
      expect(blurb, `${kind} groups by country and must say so`).toContain("country");
      expect(blurb).not.toContain("by gear cost");
    }
    // And the count it names is the population the payload reports.
    const stated = blurb.match(/\b(\d+)\b/);
    if (stated) expect(Number(stated[1])).toBe(model.total);
  });
});

describe("the final score shows the parts, not just the total", () => {
  it("ranks the seats and itemises every category the engine scored", () => {
    render(<OutcomeView outcome={wire.result} colors={COLORS} />);
    for (const p of wire.result.standings) {
      expect(screen.getByText(String(p.breakdown.total))).toBeInTheDocument();
    }
    for (const label of ["VP", "Church", "Customs", "Tech", "Country", "Remainder"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("marks the winner rather than letting a margin imply one", () => {
    // A margin of 0 is not a win: two seats can tie on points and be separated
    // by the engine's own tie-break, so `won` is what the row is marked from.
    const { container } = render(<OutcomeView outcome={wire.result} colors={COLORS} />);
    const won = container.querySelectorAll("tr.won");
    expect(won).toHaveLength(1);
    expect(wire.result.standings.filter((p) => p.won)).toHaveLength(1);
    expect(wire.result.standings.find((p) => p.won)!.seat).toBe(wire.result.winner);
  });

  it("the margins are mirror images, because they are measured against each other", () => {
    const [first, second] = wire.result.standings;
    expect(first!.margin).toBe(-second!.margin);
    expect(first!.margin).toBeGreaterThan(0);
  });
});

// ---- M3.5: one vocabulary for a printed space, and it is the terminal's ----

describe("the printed spaces read as the terminal reads them", () => {
  // **The client cannot import a Rust function, so the two are frozen together
  // instead.** `wire_fixtures.rs` emits `card_render::space_reward_words` for
  // all 24 cards and refuses when the server's output is not what is committed;
  // this asserts the client's own wording against that. Neither side can be
  // re-worded while both believe they agree.
  //
  // What it caught: `Index.tsx` built the words from serde field names, and
  // disagreed on 55 of the 96 printed spaces. Two of those state a rule wrongly
  // rather than reading oddly — `movement` is the ASSISTANT-move icon, and was
  // rendered as "movement"; `warehouse_tokens` is a number of TAKES, and was
  // rendered as "warehouse tokens".
  it("words all 96 spaces exactly as the CLI does", () => {
    const expected = wire.siteSpaceWords.cards;
    const faces = wire.indexSites.groups
      .flatMap((g) => g.cards)
      .flatMap((c) => ("Site" in c.face ? [{ index: c.face.Site.index, site: c.face.Site.site }] : []));
    expect(faces, "the index must carry all 24 printed sites").toHaveLength(expected.length);

    let compared = 0;
    for (const { index, site } of faces) {
      const row = expected.find((e) => e.index === index);
      expect(row, `no terminal wording for site ${index}`).toBeDefined();
      site.spaces.forEach((space: SpaceReward, i: number) => {
        expect(spaceWords(space), `site ${index}, space ${i}`).toBe(row!.spaces[i]);
        compared += 1;
      });
    }
    // Anti-vacuity: an empty corpus agrees with everything.
    expect(compared, "every printed space must actually be compared").toBe(96);
  });

  // The two the old vocabulary got wrong are named, so a future edit that
  // reverts them fails with the reason rather than with a diff.
  it("calls a movement icon an assistant move, and a warehouse icon a take", () => {
    const all = wire.siteSpaceWords.cards.flatMap((c) => c.spaces);
    expect(all.some((w) => /assistant move/.test(w)), "the corpus must contain one").toBe(true);
    expect(all.some((w) => /warehouse take/.test(w)), "the corpus must contain one").toBe(true);
    // `SpaceReward.movement` is the ASSISTANT-move icon (`types/board.rs:109-111`),
    // so the bare word "movement" on a space reads as President movement.
    expect(all.some((w) => /\bmovement\b/.test(w))).toBe(false);
    expect(all.some((w) => /warehouse tokens/.test(w))).toBe(false);
  });
});

// ---- #75: the claim drawn on the card, in the goals index ----------------
describe("the claim band in the goals index", () => {
  const goals = wire.indexGoals.groups.flatMap((g) => g.cards).map((c) => {
    if (!("Achievement" in c.face)) throw new Error("the goals index carries achievements");
    return c.face.Achievement;
  });

  it("draws the corpus's one claim as a band with one cube in the ringed place", () => {
    // The premise, from the corpus's own index payload: one card is claimed,
    // by seat 1 (Blue at this 2P table), and the other two are not.
    expect(goals.map((g) => g.claimed_by)).toEqual([[1], [], []]);
    expect(goals[0]!.first_place_vp).toBe(8);
    const { container } = index("goals", wire.indexGoals);
    const cards = [...container.querySelectorAll("li.index-card")];
    expect(cards).toHaveLength(3);
    const band = cards[0]!.querySelector("figure.card-face.achievement > .card-pic > .claims");
    expect(band, "the claimed card draws its band on the picture box").not.toBeNull();
    expect(band!.getAttribute("role")).toBe("img");
    expect(band!.getAttribute("aria-label")).toBe("Blue claimed first — 8 VP");
    const cubes = [...band!.querySelectorAll(":scope > .claim")];
    expect(cubes).toHaveLength(1);
    expect(cubes[0]!.classList.contains("first")).toBe(true);
    expect(cubes[0]!.classList.contains("seat-blue")).toBe(true);
    expect(cubes[0]!.getAttribute("title")).toBe("Blue claimed first — 8 VP");
    expect(cubes[0]!.querySelector("svg.piece.cube")).not.toBeNull();
    expect(band!.querySelector(".claims-vp.later")).toBeNull();
    for (const c of cards.slice(1)) {
      expect(c.querySelector(".card-pic"), "an unclaimed card still draws its picture box").not.toBeNull();
      expect(c.querySelector(".claims")).toBeNull();
    }
  });

  // Design §6 asks for the band "in the strip, the panel and the index, with
  // art and without", and for the four-claim card: the two cells below are the
  // index's (the whole-branch review, L-2 / MB-9). The corpus's index payload is
  // a 2P walk whose cards carry at most one claim, so the four-claim card is a
  // JSON round-trip clone of it with one field changed, the first card's
  // `claimed_by`, set to the committed fixture's claim order 2, 0, 3, 1 — which
  // neither seat order nor "the lowest seat first" can pass for.
  it("draws a card claimed four times in claim order, the first claim in the ringed place", () => {
    const FOUR_COLORS: PlayerColor[] = ["Red", "Blue", "Yellow", "Green"];
    expect(wire.goalsClaimed4p.achievements.cards[0]!.claimed_by, "the fixture's claim order").toEqual([2, 0, 3, 1]);
    const clone: typeof wire.indexGoals = JSON.parse(JSON.stringify(wire.indexGoals));
    const first = clone.groups.flatMap((g) => g.cards)[0]!.face;
    if (!("Achievement" in first)) throw new Error("the goals index carries achievements");
    first.Achievement.claimed_by = [2, 0, 3, 1];
    expect([first.Achievement.first_place_vp, first.Achievement.subsequent_vp]).toEqual([8, 6]);
    const { container } = render(
      <IndexView first={clone} kind="goals" game="wire" art={wire.artManifest} labels={wire.labels} colors={FOUR_COLORS} />,
    );
    const card = container.querySelectorAll("li.index-card")[0]!;
    const band = card.querySelector("figure.card-face.achievement > .card-pic > .claims");
    expect(band, "the claimed card draws its band on the picture box").not.toBeNull();
    const words = ["Yellow claimed first — 8 VP", "Red claimed — 6 VP", "Green claimed — 6 VP", "Blue claimed — 6 VP"];
    expect(band!.getAttribute("aria-label")).toBe(words.join(" · "));
    const cubes = [...band!.querySelectorAll(":scope > .claim")];
    expect(
      cubes.map((c) => [...c.classList].filter((k) => k.startsWith("seat-"))),
      "claim order: seats 2, 0, 3, 1",
    ).toEqual([["seat-yellow"], ["seat-red"], ["seat-green"], ["seat-blue"]]);
    expect(cubes.map((c) => c.classList.contains("first")), "only the first claim is ringed").toEqual([true, false, false, false]);
    expect(cubes.map((c) => c.getAttribute("title"))).toEqual(words);
    expect(band!.querySelector(".claims-vp.first")!.textContent).toBe("8");
    expect(band!.querySelector(".claims-vp.later")!.textContent).toBe("6");
  });

  it("draws the one claim's band over the blank when there is no art", () => {
    expect(goals[0]!.claimed_by).toEqual([1]);
    const { container } = render(
      <IndexView first={wire.indexGoals} kind="goals" game="wire" art={null} labels={wire.labels} colors={COLORS} />,
    );
    const cards = [...container.querySelectorAll("li.index-card")];
    expect(cards).toHaveLength(3);
    const pic = cards[0]!.querySelector("figure.card-face.achievement > .card-pic.blank");
    expect(pic, "with no art the picture box is the blank").not.toBeNull();
    expect(pic!.querySelector(":scope > .card-blank"), "the picture is the blank").not.toBeNull();
    expect(pic!.querySelector("img")).toBeNull();
    const band = pic!.querySelector(":scope > .claims");
    expect(band, "the band stands on the blank").not.toBeNull();
    expect(band!.getAttribute("aria-label")).toBe("Blue claimed first — 8 VP");
    const cubes = [...band!.querySelectorAll(":scope > .claim")];
    expect(cubes).toHaveLength(1);
    expect(cubes[0]!.classList.contains("first")).toBe(true);
    expect(cubes[0]!.classList.contains("seat-blue")).toBe(true);
    for (const c of cards.slice(1)) expect(c.querySelector(".claims")).toBeNull();
  });

  it("draws no first or also chip in any caption", () => {
    const { container } = index("goals", wire.indexGoals);
    const chips = [...container.querySelectorAll(".card-face.achievement .chip")];
    expect(chips.length).toBeGreaterThan(0);
    expect(container.querySelectorAll(".chip.claimed")).toHaveLength(0);
    for (const c of chips) expect(c.textContent).not.toMatch(/^(first|also) /);
  });
});
