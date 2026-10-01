// The edge's check, as an overlay (cycle F, R-F6; spec §2.6).
//
// **One overlay, mounted once beside the app, and the seam's only handler**: a
// request that meets the edge's challenge asks `passChallenge()`, and this is
// what it opens — the widget, once, under one sentence and a Cancel. The widget
// is a FAKE `window.turnstile` held beside these cells (a third party's shape,
// never a wire payload); the script is never fetched.

import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Challenge } from "./Challenge";
import {
  EDGE_CHECK,
  EDGE_CHECK_REPEATED,
  passChallenge,
  resetTurnstileForTests,
  type TurnstileApi,
  type TurnstileOptions,
} from "./turnstile";

type Platform = Window & { turnstile?: TurnstileApi; __yokohamaTurnstile?: () => void };

/** The widget's script arriving, with a fake API that records what it is
 *  asked. */
function arrive() {
  const rendered: { container: HTMLElement; options: TurnstileOptions }[] = [];
  const removed: string[] = [];
  const w = window as Platform;
  w.turnstile = {
    render: (container, options) => {
      rendered.push({ container, options });
      return `widget-${rendered.length}`;
    },
    remove: (id) => {
      removed.push(id);
    },
  };
  w.__yokohamaTurnstile?.();
  return { rendered, removed };
}

const SENTENCE = "Checking that this is a person…";

/** Let a chain of promise turns settle inside React's act. */
const turns = () =>
  act(async () => {
    for (let i = 0; i < 5; i += 1) await Promise.resolve();
  });

beforeEach(() => {
  resetTurnstileForTests();
  vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", "site-key-fixture");
});
afterEach(() => {
  resetTurnstileForTests();
  for (const s of document.querySelectorAll("script")) s.remove();
  delete (window as Platform).turnstile;
});

describe("the challenge overlay", () => {
  it("draws nothing at mount, and asks for no script", () => {
    render(<Challenge />);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.queryByText(SENTENCE)).toBeNull();
    expect(document.querySelectorAll("script")).toHaveLength(0);
  });

  it("opens on a challenge with the widget in .challenge-widget, and closes when it passes", async () => {
    render(<Challenge />);
    let passed = false;
    let pass!: Promise<void>;
    act(() => {
      pass = passChallenge().then(() => {
        passed = true;
      });
    });
    const dialog = screen.getByRole("dialog", { name: SENTENCE });
    expect(dialog).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    const widget = arrive();
    await turns();
    expect(widget.rendered).toHaveLength(1);
    expect(widget.rendered[0]!.container).toBe(dialog.querySelector(".challenge-widget"));
    expect(passed).toBe(false);
    act(() => widget.rendered[0]!.options.callback("a-token-nobody-reads"));
    await act(() => pass);
    expect(passed).toBe(true);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(widget.removed).toEqual(["widget-1"]);
  });

  it("refuses the challenge on Cancel, takes the widget down and closes", async () => {
    render(<Challenge />);
    let pass!: Promise<void>;
    act(() => {
      pass = passChallenge();
    });
    const outcome = pass.then(
      () => "passed",
      (e: Error) => e.message,
    );
    const widget = arrive();
    await turns();
    await userEvent.setup().click(screen.getByRole("button", { name: "Cancel" }));
    expect(await outcome).toBe(EDGE_CHECK_REPEATED);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(widget.removed).toEqual(["widget-1"]);
  });

  it("refuses the challenge when the widget fails, and closes", async () => {
    render(<Challenge />);
    let pass!: Promise<void>;
    act(() => {
      pass = passChallenge();
    });
    const outcome = pass.then(
      () => "passed",
      (e: Error) => e.message,
    );
    const widget = arrive();
    await turns();
    act(() => widget.rendered[0]!.options["error-callback"]("300030"));
    expect(await act(() => outcome)).toBe(EDGE_CHECK_REPEATED);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("is the seam's handler only while it is mounted", async () => {
    const { unmount } = render(<Challenge />);
    unmount();
    await expect(passChallenge()).rejects.toThrow(EDGE_CHECK);
  });
});
