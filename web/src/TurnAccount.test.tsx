// The account of the last turn: the run a reply added, and — with no run — the
// reply's prose lines. The four cells that pinned this box before it grew a run
// are carried here (the live-region pair merged into one, keeping the
// unfolded-className assertion), and the run's own cells join them.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TurnAccount, isRefusal } from "./TurnAccount";
import { buildRun } from "./walk";
import { wire } from "./wire/contract";
import type { HistoricalFrame, PlayerColor, PlayerId } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];
const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

const noop = () => {};
const handlers = {
  onCadence: noop, onPlay: noop, onPause: noop, onBack: noop, onForward: noop,
  onJump: noop, onSkip: noop, onReplay: noop,
};

/** A two-step run by seat 1, whose Players row says it is a bot.
 *
 *  `phase` moves it off the fixture's own `current_phase`, which is the only
 *  way to tell a phase LABEL from the wire key it is looked up by. `firstMover`
 *  makes the run change hands part-way — the ORDINARY shape, since a reply's
 *  run normally opens with the human's own answer and continues into the bot's
 *  steps — which is the only way to tell a per-row mover from the run's last
 *  one. Both default to the single-mover fixture every other cell was written
 *  against. */
function botRun({ phase, firstMover = 1 }: { phase?: number; firstMover?: PlayerId } = {}) {
  const f0 = clone(wire.atK);
  f0.to_move = firstMover;
  if (phase !== undefined) f0.frame.decision.current_phase = phase;
  const players = f0.frame.dashboard.find((p) => "Players" in p);
  if (!players || !("Players" in players)) throw new Error("no Players panel");
  players.Players.players.find((p) => p.seat === 1)!.is_bot = true;
  const f1 = clone(f0);
  f1.to_move = 1;
  f1.frame.board.tiles[0]!.assistants[1]![1] += 2;
  const f2 = clone(f1);
  f2.frame.decision.current_phase += 1;
  // The position AFTER the last step is a NEW turn. Without this the fixture
  // clones one frame three times, `before` and `after` carry the same
  // `turn_number`, and the heading cell below cannot tell them apart.
  f2.frame.decision.turn_number += 1;
  return buildRun(20, [f0, f1, f2], {
    total: 22,
    entries: [
      { index: 20, text: `${firstMover === 1 ? "Blue" : "Red"}: Place 2 assistants in Tea Plantation (A00)` },
      { index: 21, text: "Blue: Return President to hand (recover no assistants)" },
    ],
  });
}

function show(over: Partial<Parameters<typeof TurnAccount>[0]>) {
  return render(
    <TurnAccount
      messages={[]}
      run={null}
      at={null}
      playing={false}
      cadence={2000}
      labels={wire.labels}
      colors={COLORS}
      {...handlers}
      {...over}
    />,
  );
}

describe("the turn account with a run", () => {
  it("is headed by the mover's turn, one row per event with its phase, mover and line", () => {
    // The run changes hands: seat 0 (Red) answered, then the bot at seat 1
    // (Blue) played on. Without that the fixture has ONE mover and a component
    // marking every row with `last.mover` reads correctly by accident.
    const run = botRun({ firstMover: 0 });
    show({ run });
    // **Not the whole `<h2>`.** The cadence select's option text lives inside it,
    // so its textContent reads "Blue's turn 921 s2 s3 shide" — and a heading
    // printing NO number at all still matches /Blue's turn \d+/ through the
    // select's own "1 s". The seat span is the heading's own words, and the
    // match is exact: the run's last step is BEFORE turn 92 and AFTER turn 93,
    // so reading `last.after` here prints 93 and fails.
    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading.querySelector("span")!.textContent).toBe("Blue's turn 92");
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent(/Place 2 assistants in Tea Plantation/);
    const phaseKey = run.steps[0]!.phase!;
    expect(rows[0]).toHaveTextContent(wire.labels.phases[phaseKey] ?? phaseKey);
    // Each row's OWN mover, and the heading is the LAST step's.
    expect(rows[0]!.querySelector(".mover")?.className).toMatch(/seat-red/);
    expect(rows[0]!.querySelector(".mover")?.textContent).toBe("Red");
    expect(rows[1]!.querySelector(".mover")?.className).toMatch(/seat-blue/);
  });

  it("lights the step on the table and offers the walk's controls while it runs", async () => {
    const run = botRun();
    const onPause = vi.fn();
    const onSkip = vi.fn();
    const onForward = vi.fn();
    show({ run, at: 1, playing: true, onPause, onSkip, onForward });
    const rows = screen.getAllByRole("listitem");
    expect(rows[1]).toHaveClass("at");
    expect(rows[0]).not.toHaveClass("at");
    // Colour alone does not say which step is on the table.
    expect(rows[1]).toHaveAttribute("aria-current", "step");
    expect(rows[0]).not.toHaveAttribute("aria-current");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Pause" }));
    await user.click(screen.getByRole("button", { name: "Step forward" }));
    await user.click(screen.getByRole("button", { name: "skip" }));
    expect(onPause).toHaveBeenCalledTimes(1);
    expect(onForward).toHaveBeenCalledTimes(1);
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "replay" })).toBeNull();
  });

  it("offers replay once the walk is over, and a row jumps to its step", async () => {
    const run = botRun();
    const onReplay = vi.fn();
    const onJump = vi.fn();
    show({ run, at: null, onReplay, onJump });
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "replay" }));
    expect(onReplay).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: /Return President to hand/ }));
    expect(onJump).toHaveBeenCalledWith(1);
    expect(screen.queryByRole("button", { name: "skip" })).toBeNull();
  });

  // **The phase is shown in the labels' WORDS, not in the wire's key**, and the
  // cell above cannot see that: its fixture sits in `Building`, whose label is
  // the string "Building" as well, so replacing the lookup with `s.phase` alone
  // leaves that cell green — measured, not supposed. `AreaAction` labels as
  // "Area action" and is what separates them.
  it("words the phase from the labels, not from the wire's key", () => {
    const run = botRun({ phase: 3 });
    expect(run.steps[0]!.phase, "phase_chain[3] must be AreaAction").toBe("AreaAction");
    expect(wire.labels.phases.AreaAction, "its label must differ from its key").toBe("Area action");
    show({ run });
    const rows = screen.getAllByRole("listitem");
    expect(rows[0]).toHaveTextContent("Area action");
    expect(rows[0]!.textContent).not.toMatch(/AreaAction/);
  });

  // Paused, the header offers Play where it offered Pause. `Step back` is
  // exercised here and nowhere else: the cell above never clicks it and never
  // reads its `disabled`, so a Step back wired to any other handler — measured
  // against `onPause` — left the whole suite green.
  it("offers Play while paused, and Step back is its own control, dead at the first step", async () => {
    const run = botRun();
    const onPlay = vi.fn();
    const onBack = vi.fn();
    const onPause = vi.fn();
    const user = userEvent.setup();
    const first = show({ run, at: 0, playing: false, onPlay, onBack, onPause });
    expect(screen.queryByRole("button", { name: "Pause" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Play" }));
    expect(onPlay).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Step back" })).toBeDisabled();
    first.unmount();
    show({ run, at: 1, playing: false, onPlay, onBack, onPause });
    await user.click(screen.getByRole("button", { name: "Step back" }));
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(onPause).not.toHaveBeenCalled();
  });

  it("remembers the cadence the player picks", async () => {
    const onCadence = vi.fn();
    show({ onCadence });
    await userEvent.setup().selectOptions(screen.getByLabelText("Cadence"), "3000");
    expect(onCadence).toHaveBeenCalledWith(3000);
  });
});

describe("the account keeps its lit row in view", () => {
  // **Spec §3.2.5: "the rows scroll to keep it in view".** It did not — the
  // final review found no scrolling of any kind in the client (m4). Above
  // 1201px a nine-row account fits, which is why it went unnoticed; below it
  // the box is capped at 9rem, so from about the fifth step the walk lights a
  // row inside the scroller's hidden part.
  //
  // **And it must scroll THAT BOX AND NOTHING ELSE (ruling R-F11).** The first
  // fix used `scrollIntoView({ block: "nearest" })`, which by specification
  // scrolls EVERY scrollable ancestor — including the window. Measured live at
  // 1000x523: `window.scrollY` went 0 → 1826 → 1851 → 1877 over three steps,
  // dragging the board the walk was highlighting off screen and parking the lit
  // row at the viewport's bottom edge underneath the sticky walk strip. In the
  // stack the window is an ancestor scroller and the page is the player's to
  // scroll (M5 spec R8), so the effect moves the account's own `scrollTop`.
  //
  // **The cap lives on `.turn-account .messages`** — `max-height: 9rem;
  // overflow: auto`, at EVERY width except inside `@media (min-width: 1201px)`,
  // where `.turn-account .messages:not(.folded)` releases it and `.side`
  // becomes the scroller instead. So the scroller is the `<div id=
  // "turn-account-body">`, neither the panel around it nor the `<ol>` inside
  // it, and above 1201px it is not a scroller at all.
  //
  // jsdom computes no layout, so every figure a real browser would measure is
  // stubbed here: `scrollHeight`/`clientHeight` on the box and
  // `offsetTop`/`offsetHeight` on the rows all read 0 otherwise (measured).
  // `scrollTop` is NOT stubbed — jsdom stores what is assigned to it, so it is
  // the one real reading in the cell.
  const geometry = (
    container: HTMLElement,
    g: { clientHeight: number; scrollHeight: number; rows: [top: number, height: number][] },
  ) => {
    const box = container.querySelector<HTMLElement>("#turn-account-body");
    if (!box) throw new Error("no account body to scroll");
    Object.defineProperty(box, "clientHeight", { configurable: true, get: () => g.clientHeight });
    Object.defineProperty(box, "scrollHeight", { configurable: true, get: () => g.scrollHeight });
    [...container.querySelectorAll<HTMLElement>("ol.account li")].forEach((li, i) => {
      const [top, height] = g.rows[i] ?? [0, 0];
      Object.defineProperty(li, "offsetTop", { configurable: true, get: () => top });
      Object.defineProperty(li, "offsetHeight", { configurable: true, get: () => height });
    });
    return box;
  };

  const account = (run: ReturnType<typeof botRun> | null, at: number | null) => (
    <TurnAccount
      messages={[]}
      run={run}
      at={at}
      playing={at !== null}
      cadence={2000}
      labels={wire.labels}
      colors={COLORS}
      {...handlers}
    />
  );

  /** Mount with the run but NO lit step, so the rows exist to be measured
   *  before any lit step arrives — which is also the order the App renders in,
   *  the account being on screen before a walk starts. */
  const mounted = (run: ReturnType<typeof botRun>, g: Parameters<typeof geometry>[1]) => {
    const r = render(account(run, null));
    return { ...r, box: geometry(r.container, g) };
  };

  // Two rows 30 tall in a 40-tall box: row 0 fits, row 1 does not, and going
  // back to row 0 is the third branch. One fixture, all three.
  const TALL = { clientHeight: 40, scrollHeight: 100, rows: [[0, 30], [30, 30]] as [number, number][] };

  it("moves its own scroller so the lit row lies inside the box, minimally, both ways", () => {
    const run = botRun();
    const { box, rerender } = mounted(run, TALL);
    expect(box.scrollTop, "nothing is lit yet, so nothing has moved").toBe(0);

    // Row 0 spans [0, 30] and the box shows [0, 40]: already inside, so
    // "nearest" moves nothing. An effect that centred instead would move here.
    rerender(account(run, 0));
    expect(box.scrollTop, "the lit row was already in view and the box moved anyway").toBe(0);

    // Row 1 spans [30, 60] against a view of [0, 40]: the minimum that brings
    // its bottom edge in is 20, which shows [20, 60].
    rerender(account(run, 1));
    expect(box.scrollTop).toBe(20);

    // Back up: row 0's top at 0 is above the view's 20, so the box comes back
    // to 0 rather than staying put. `back` in the walk's controls does this.
    rerender(account(run, 0));
    expect(box.scrollTop).toBe(0);
  });

  it("leaves the scroller alone when the account is not a scroller", () => {
    // Above 1201px `max-height: none` releases the cap, so the box grows to
    // its content and the page is what scrolls — the player's, not the walk's.
    const run = botRun();
    const { box, rerender } = mounted(run, {
      clientHeight: 100,
      scrollHeight: 100,
      rows: [[0, 30], [30, 30]],
    });
    box.scrollTop = 7;
    rerender(account(run, 1));
    expect(box.scrollTop, "an uncapped account was scrolled by the walk").toBe(7);
  });

  // **The regression this ruling exists for.** `scrollIntoView` propagates to
  // every scrollable ancestor by specification, so nothing may call it — and
  // the window must never be scrolled by any other route either. Both are
  // spied rather than reasoned about, because "we no longer call it" is a
  // sentence and this is a check.
  it("never calls scrollIntoView and never scrolls the window", () => {
    const intoView = vi.fn();
    const before = Element.prototype.scrollIntoView;
    Element.prototype.scrollIntoView = intoView as typeof Element.prototype.scrollIntoView;
    const scrollTo = vi.spyOn(window, "scrollTo").mockImplementation(() => {});
    try {
      const run = botRun();
      const { box, container, rerender } = mounted(run, TALL);
      rerender(account(run, 0));
      rerender(account(run, 1));
      // Anti-vacuity: the account really did scroll on this fixture, so the two
      // spies are silent about a path that ran rather than one that did not.
      expect(box.scrollTop, "the fixture must actually scroll the account").toBe(20);
      expect(intoView).not.toHaveBeenCalled();
      expect(scrollTo).not.toHaveBeenCalled();

      // The other way a lit step arrives: a fresh run landing at step 0.
      const next = botRun();
      rerender(account(next, null));
      const box2 = geometry(container, { clientHeight: 40, scrollHeight: 100, rows: [[70, 30], [100, 30]] });
      rerender(account(next, 0));
      expect(box2.scrollTop, "the fresh run's first row must be out of view").toBe(60);
      expect(intoView).not.toHaveBeenCalled();
      expect(scrollTo).not.toHaveBeenCalled();
    } finally {
      Element.prototype.scrollIntoView = before;
      scrollTo.mockRestore();
    }
  });

  it("scrolls nothing when there is no lit row", () => {
    const run = botRun();
    const { box, rerender } = mounted(run, TALL);
    box.scrollTop = 12;
    // A finished walk still shows its run — this is the account after `skip`,
    // and it must not haul the box around behind the player's back.
    rerender(account(run, null));
    expect(box.scrollTop).toBe(12);
    // And nor must the prose box, which has no rows at all.
    rerender(account(null, null));
    expect(box.scrollTop).toBe(12);
  });
});

describe("the turn account without a run", () => {
  it("holds every prose line the server sent, refusals as warnings", () => {
    const state = wire.refused;
    const refusals = state.messages.filter(isRefusal);
    expect(refusals.length, "the fixture must carry a refusal").toBeGreaterThan(0);
    show({ messages: state.messages });
    for (const m of state.messages) expect(screen.getByText(new RegExp(escape(m)))).toBeInTheDocument();
    for (const m of refusals) expect(screen.getByText(new RegExp(escape(m)))).toHaveClass("warn");
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("Last move");
  });

  // **The leading-bang branch, which the prose regex misses entirely.**
  // `repl.rs` refuses with eighteen literals that start with a bang and contain
  // none of `illegal|refus|cannot|not legal|out of range`, so before that branch
  // every one of them rendered as an ordinary applied-move line. The regex is
  // asserted to MISS the sample first, or this cell would pass through the other
  // half of the OR and prove nothing about the branch it is named for.
  it("warns on a bang-prefixed refusal the prose regex cannot see", () => {
    const bang = "! A05 is not one of the eligible areas";
    expect(/illegal|refus|cannot|not legal|out of range/i.test(bang)).toBe(false);
    show({ messages: [bang, "Turn 1 Red -> Turn 2 Blue"] });
    expect(screen.getByText(new RegExp(escape(bang)))).toHaveClass("warn");
    expect(screen.getByText(/Turn 1 Red/)).not.toHaveClass("warn");
  });

  // A live region has to exist BEFORE its text arrives to be announced, and it
  // has to survive the fold — which is why folding is `max-height: 0` and not
  // an unmount. The unfolded className is asserted to be exactly `messages`,
  // with no trailing space left behind by the folded arm: the CSS does not
  // care, but anything reading the className back does.
  it("keeps its live region mounted while empty, and folds without unmounting it", async () => {
    const first = show({});
    const { container } = first;
    const region = container.querySelector('[role="status"]');
    expect(region).not.toBeNull();
    expect(region!.textContent).toBe("");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "hide" }));
    expect(container.querySelector(".messages.folded")).not.toBeNull();
    expect(container.querySelector('[role="status"]')).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "show" }));
    expect(container.querySelector(".messages.folded")).toBeNull();
    expect(container.querySelector('[role="status"]')!.className).toBe("messages");

    // **Folding CLIPS, it does not hide** (`max-height: 0; overflow: hidden`),
    // so a folded account with rows in it would otherwise leave every
    // `.log-jump` in the tab order: a keyboard user tabbing into controls
    // nobody can see. Taking them out of the tab order rather than out of the
    // tree is what keeps the live region announcing (ruling R-T8-1).
    first.unmount();
    const withRun = show({ run: botRun() });
    const jumps = () => [...withRun.container.querySelectorAll(".log-jump")];
    expect(jumps().length, "the run must put rows in the body").toBe(2);
    await user.click(screen.getByRole("button", { name: "hide" }));
    for (const b of jumps()) expect(b).toHaveAttribute("tabindex", "-1");
    await user.click(screen.getByRole("button", { name: "show" }));
    for (const b of jumps()) expect(b).toHaveAttribute("tabindex", "0");
  });
});
