// The one line that offers a new release (M6.5c2 cycle F, R-F3, R-FP14; spec
// §2.1): on the door, in the lobby's actions and in a game's banner.
//
// **It is drawn for two states and its click means a different thing in
// each.** `updateWaiting` — a release is installed behind the running one —
// takes it (`applyUpdate`: post skip-waiting, reload once the new worker is in
// charge). `staleTab` — ANOTHER tab's apply already handed this one to the new
// worker, and nothing waits any more — is a plain `reload`. The sentence is the
// same, because to the player it is the same fact: this tab is behind, and the
// click brings it up to date.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ReadinessStatus } from "./swRegister";
import { UpdateLine } from "./UpdateLine";

const status = (over: Partial<ReadinessStatus> = {}): ReadinessStatus => ({
  readiness: "ready",
  reason: null,
  updateWaiting: false,
  staleTab: false,
  applyUpdate: vi.fn(),
  reload: vi.fn(),
  retry: vi.fn(),
  ...over,
});

describe("the update line", () => {
  it("draws nothing when nothing is waiting and the tab is not behind", () => {
    const { container } = render(<UpdateLine status={status()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("offers a waiting release in one quiet sentence, and takes it only on the click", async () => {
    const s = status({ updateWaiting: true });
    render(<UpdateLine status={s} />);
    const line = screen.getByText(/A new version is ready\./);
    expect(line.tagName).toBe("P");
    expect(line).toHaveClass("muted", "update-line");
    expect(line).toHaveTextContent(/^A new version is ready\. Reload to use it$/);
    const button = screen.getByRole("button", { name: "Reload to use it" });
    expect(button).toHaveClass("linkish");
    expect(button).toHaveAttribute("type", "button");
    expect(s.applyUpdate, "nothing switches before the click").not.toHaveBeenCalled();
    await userEvent.click(button);
    expect(s.applyUpdate).toHaveBeenCalledTimes(1);
    expect(s.reload, "a waiting release is taken, not merely reloaded over").not.toHaveBeenCalled();
  });

  // **A stale tab has nothing left to take**: the worker it would have posted
  // to is the active one now, so `applyUpdate` would find no waiting worker and
  // do nothing. Its click is a plain reload.
  it("says the same sentence for a stale tab, whose click is a plain reload", async () => {
    const s = status({ staleTab: true });
    render(<UpdateLine status={s} />);
    expect(screen.getByText(/A new version is ready\./)).toHaveTextContent(
      /^A new version is ready\. Reload to use it$/,
    );
    await userEvent.click(screen.getByRole("button", { name: "Reload to use it" }));
    expect(s.reload).toHaveBeenCalledTimes(1);
    expect(s.applyUpdate).not.toHaveBeenCalled();
  });

  // Stale, and then a NEWER release waits behind the one it was handed to:
  // taking that one reloads the tab too, on the new worker's own terms.
  it("takes a waiting release when the tab is stale as well", async () => {
    const s = status({ updateWaiting: true, staleTab: true });
    render(<UpdateLine status={s} />);
    expect(screen.getAllByRole("button", { name: "Reload to use it" })).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Reload to use it" }));
    expect(s.applyUpdate).toHaveBeenCalledTimes(1);
    expect(s.reload).not.toHaveBeenCalled();
  });
});
