// The footer's notice (R-6.5c2.40): a build value drawn as one line at the foot
// of every screen. The reader is mocked — it is a build value, not a payload.

import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Footer } from "./Footer";
import * as values from "./buildValues";

describe("Footer", () => {
  it("draws the notice and marks the page, and unmounting takes the mark away", () => {
    vi.spyOn(values, "siteNotice").mockReturnValue("Not affiliated with Asmodee");
    const { unmount } = render(<Footer />);
    const footer = document.querySelector("footer.site-notice");
    expect(footer).not.toBeNull();
    expect(footer?.textContent).toBe("Not affiliated with Asmodee");
    expect(document.documentElement).toHaveClass("has-notice");
    unmount();
    expect(document.documentElement).not.toHaveClass("has-notice");
  });

  it("draws nothing, and marks nothing, where the build set no notice", () => {
    vi.spyOn(values, "siteNotice").mockReturnValue(null);
    render(<Footer />);
    expect(document.querySelector("footer")).toBeNull();
    expect(document.documentElement).not.toHaveClass("has-notice");
  });
});
