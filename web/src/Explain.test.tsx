// The explanation surface: the chart that has to add up, and the refusals that
// have to be shown rather than swallowed.
//
// Every payload here is server-emitted — `wire.suggest`, `wire.why`,
// `wire.whyRefused`, `wire.whyRefusals` — for the reason the harness states: a
// hand-built explanation would keep rendering after the server dropped a field,
// and this is the one screen whose whole value is that its numbers are the
// engine's own.

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ExplanationView, SuggestView, WhyView } from "./Explain";
import { wire } from "./wire/contract";
import type { Explanation } from "./types";

const labels = wire.labels;

describe("the chart adds up, and says so on screen", () => {
  it("foots: the printed total is the sum of the printed rows", () => {
    // **The claim the whole surface rests on.** A reader can add the visible
    // column and get the number under the rule; if they could not, this would
    // be a picture rather than an explanation.
    const e = wire.suggest.ok!.explanation;
    render(<ExplanationView e={e} labels={labels} />);
    const sum = e.terms.reduce((a, t) => a + t.product, 0);
    expect(sum).toBe(e.total);
    expect(screen.getByText(/^TOTAL/)).toBeInTheDocument();
    expect(screen.getByText(`+${e.total}`, { exact: true })).toBeInTheDocument();
  });

  it("collapses the features contributing nothing, and counts them out loud", async () => {
    const user = userEvent.setup();
    const e = wire.suggest.ok!.explanation;
    const silent = e.terms.length - e.nonzero;
    render(<ExplanationView e={e} labels={labels} />);

    // Collapsed is not dropped: the count is printed, the total still foots,
    // and one click brings them back.
    expect(screen.getAllByRole("listitem")).toHaveLength(e.nonzero);
    expect(screen.getByText(new RegExp(`${e.nonzero} of ${e.terms.length} features`))).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: new RegExp(`Show the ${silent} features`) }));
    expect(screen.getAllByRole("listitem")).toHaveLength(e.terms.length);
  });

  it("sorts by magnitude, so the terms that decided the move are first", () => {
    const e = wire.suggest.ok!.explanation;
    render(<ExplanationView e={e} labels={labels} />);
    const rows = screen.getAllByRole("listitem");
    const magnitudes = rows.map((r) => {
      const value = r.querySelector(".bar-value")!.textContent!;
      return Math.abs(Number(value.replace("+", "")));
    });
    expect([...magnitudes].sort((a, b) => b - a)).toEqual(magnitudes);
  });

  it("shows the arithmetic PARENTHESISED, because precedence would misread it", () => {
    // `5 - 0 × 14` reads as `5 - (0 × 14)` = 5, not the 70 in the column beside
    // it. The notation has to say (yours − theirs) × weight, and this is the
    // cell that keeps it saying so.
    const e = wire.suggest.ok!.explanation;
    render(<ExplanationView e={e} labels={labels} />);
    const first = [...e.terms].sort((a, b) => Math.abs(b.product) - Math.abs(a.product))[0]!;
    const row = screen.getAllByRole("listitem")[0]!;
    expect(within(row).getByText(`(${first.viewer_value} − ${first.opponent_value}) × ${first.weight}`))
      .toBeInTheDocument();
  });

  it("puts a helping term on the right of zero and a hurting one on the left", () => {
    // Colour is never the only encoding: side and sign both carry it, so the
    // chart survives greyscale and a reader who cannot separate the two hues.
    const e = wire.suggest.ok!.explanation;
    render(<ExplanationView e={e} labels={labels} />);
    for (const row of screen.getAllByRole("listitem")) {
      const value = Number(row.querySelector(".bar-value")!.textContent!.replace("+", ""));
      const side = value > 0 ? ".bar-half.right" : ".bar-half.left";
      expect(row.querySelector(`${side} .bar`), `${value} is on the wrong side of zero`)
        .not.toBeNull();
    }
  });

  it("words a feature but keeps its token, because the token is what greps", () => {
    const e = wire.suggest.ok!.explanation;
    render(<ExplanationView e={e} labels={labels} />);
    const carrying = e.terms.filter((t) => t.product !== 0);
    for (const t of carrying) {
      expect(screen.getByText(t.feature)).toBeInTheDocument();
      expect(screen.getAllByText(labels.features[t.feature]!).length).toBeGreaterThan(0);
    }
  });

  it("draws a FINISHED game as a standing, not as thirty-four empty bars", () => {
    // On a terminal explanation every product is 0 by design and the whole
    // score is the standing, so bars would render empty under a nonzero total.
    // Built from a server-emitted explanation with its terminal half set, which
    // is the one shape the corpus cannot reach: this API's games do not run to
    // a bot-vs-bot finish inside a fixture walk.
    const base = wire.why.ok!.explanation;
    const terminal: Explanation = {
      ...base,
      terms: base.terms.map((t) => ({ ...t, weight: 0, product: 0 })),
      terminal: { rank: 1, seats: 2, viewer_total: 82, best_other_total: 81, value: 1000 },
      total: 1000,
      nonzero: 0,
    };
    render(<ExplanationView e={terminal} labels={labels} />);
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
    expect(screen.getByText("1 of 2")).toBeInTheDocument();
    expect(screen.getByText("82")).toBeInTheDocument();
    expect(screen.getByText("1000")).toBeInTheDocument();
  });
});

describe("suggest shows what it would play and what it beat", () => {
  it("names the move, the line it read it at, and the runner-ups by margin", () => {
    const s = wire.suggest.ok!;
    const { container } = render(<SuggestView data={wire.suggest} labels={labels} />);
    expect(container.querySelector(".headline")).toHaveTextContent(s.text);
    expect(screen.getByText(/margin against the suggestion, so 0 means tied/)).toBeInTheDocument();
    // The move appears TWICE and that is the design: step 1 of the continuation
    // is the answer again, which is how the terminal shows it, so a reader can
    // see the line the value was read at without losing where it starts.
    expect(screen.getAllByText(s.text)).toHaveLength(2);
    // Step 1 of the continuation IS the answer, and the terminal shows it that
    // way, so the client does not quietly trim it.
    for (const step of s.continuation) {
      expect(screen.getAllByText(step).length).toBeGreaterThan(0);
    }
    expect(screen.getByText(/Stream/)).toBeInTheDocument();
    expect(screen.getByText(s.stream_seed)).toBeInTheDocument();
  });

  it("does not offer to show all candidates until there are more than it shows", async () => {
    const user = userEvent.setup();
    render(<SuggestView data={wire.suggest} labels={labels} />);
    const more = screen.queryByRole("button", { name: /Show all \d+ candidates/ });
    // The fixture carries 186, so the disclosure exists and lifting it is a
    // click rather than a server round trip.
    expect(more).not.toBeNull();
    await user.click(more!);
    expect(screen.getByText(wire.suggest.ok!.ranking.at(-1)!.text)).toBeInTheDocument();
  });
});

describe("why shows the account and the claim attached to it", () => {
  it("names the contestant, the decision seed and the event it reconstructed", () => {
    const p = wire.why.ok!.provenance;
    render(<WhyView data={wire.why} labels={labels} />);
    expect(screen.getByText(p.contestant)).toBeInTheDocument();
    expect(screen.getByText(p.decision_seed)).toBeInTheDocument();
    expect(screen.getByText(new RegExp(`Event ${p.index}`))).toBeInTheDocument();
    // The honesty gate, stated on screen rather than only in a doc comment.
    expect(screen.getByText(/would have refused instead of showing you this/)).toBeInTheDocument();
  });

  it("shows the whole counter surface, not a chosen two of it", () => {
    const counters = wire.why.ok!.counters!;
    render(<WhyView data={wire.why} labels={labels} />);
    expect(counters).toHaveLength(15);
    for (const c of counters) expect(screen.getByText(c.name)).toBeInTheDocument();
  });

  it("warns when the record's controller version is not this build's", () => {
    // The move still reproduced, so this is a WARNING and not a refusal — the
    // account is this build's reading of a move an older one played.
    const drifted = {
      ...wire.why,
      ok: {
        ...wire.why.ok!,
        provenance: { ...wire.why.ok!.provenance, recorded_version: 1, current_version: 2 },
      },
    };
    render(<WhyView data={drifted} labels={labels} />);
    expect(screen.getByText(/THIS\s+build's reading of a move an older one played/)).toBeInTheDocument();
  });

  it("says nothing about drift when the versions agree", () => {
    render(<WhyView data={wire.why} labels={labels} />);
    expect(screen.queryByText(/an older one played/)).toBeNull();
  });
});

describe("a refusal is shown, because a branch nobody renders is where defects live", () => {
  it("renders the refusal the server actually sent, tag and sentence", () => {
    render(<WhyView data={wire.whyRefused} labels={labels} />);
    expect(screen.getByText("no_bot_move_yet")).toBeInTheDocument();
    expect(screen.getByText(wire.whyRefused.refusal!.message)).toBeInTheDocument();
    // A refusal is the answer, so nothing else is drawn beside it.
    expect(screen.queryByText(/^TOTAL/)).toBeNull();
  });

  it.each(wire.whyRefusals.map((r) => [r.kind, r] as const))(
    "renders the %s refusal without throwing and shows its sentence",
    (kind, refusal) => {
      // **All twelve, from the server's own corpus.** M2.5's finding was that a
      // branch the client renders and nobody has ever seen is exactly the shape
      // of the two defects M2 found by clicking. There is no exemption list
      // here: every variant is constructible, so every variant is rendered.
      render(<WhyView data={{ ok: null, refusal }} labels={labels} />);
      expect(screen.getByText(kind)).toBeInTheDocument();
      // `getByText` normalises whitespace in the DOM but not in the expected
      // string, and the `mismatch` sentence carries a deliberate newline
      // between the two moves it names. Normalising both sides compares the
      // WORDS, which is what is being asserted; the break itself is preserved
      // by `white-space: pre-line` and is a layout property, not a text one.
      const flat = (t: string) => t.replace(/\s+/g, " ").trim();
      expect(
        screen.getByText((_, el) => el?.tagName === "P" && flat(el.textContent ?? "") === flat(refusal.message)),
      ).toBeInTheDocument();
    },
  );

  it("renders a suggest refusal through the same shape", () => {
    render(
      <SuggestView
        data={{ ok: null, refusal: { kind: "bot_seat", message: "That seat is played by a bot." } }}
        labels={labels}
      />,
    );
    expect(screen.getByText("bot_seat")).toBeInTheDocument();
  });
});
