// The one line that says a newer release exists (M6.5c2 cycle F, R-F2, R-FP9;
// spec §2.2): on the door under the blurb and in the signed-in lobby's actions,
// where `FrontDoor.test.tsx` and `Lobby.test.tsx` pin its places. Here: what it
// says, and when it says nothing.
//
// The feed fixture is GitHub's shape, held beside the cell that reads it — a
// third party's payload is not a wire payload (see `releaseFeed.test.ts`).

import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ReleaseNotice } from "./ReleaseNotice";
import { resetReleaseNoticeForTests } from "./releaseFeed";

const FEED = "https://api.github.com/repos/you/yokohama-assistant/releases/latest";
const RELEASE_PAGE = "https://github.com/you/yokohama-assistant/releases/tag/v0.7.0";

const answering = (body: unknown) =>
  vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => new Response(JSON.stringify(body), { status: 200 }));

function build(over: { feed?: string | null; project?: string | null; version?: string } = {}) {
  vi.stubGlobal("__YOKOHAMA_RELEASE_FEED__", over.feed === undefined ? FEED : over.feed);
  vi.stubGlobal("__YOKOHAMA_PROJECT_URL__", over.project === undefined ? null : over.project);
  vi.stubGlobal("__YOKOHAMA_VERSION__", over.version ?? "0.6.0");
}

beforeEach(() => {
  resetReleaseNoticeForTests();
  build();
});
afterEach(() => {
  resetReleaseNoticeForTests();
});

describe("the release notice", () => {
  // **The exact sentence** (spec §2.2): the tag is the link, this build's own
  // version follows, and there is NO trailing period — the update line has none
  // either. `textContent` and not `toHaveTextContent`, which collapses
  // whitespace and would pass a doubled space.
  it("says a newer release is available and what this site runs, with the tag as the link", async () => {
    const { container } = render(
      <ReleaseNotice fetchImpl={answering({ tag_name: "v0.7.0", html_url: RELEASE_PAGE })} />,
    );
    const line = await waitFor(() => {
      const p = container.querySelector("p.release-notice");
      expect(p).not.toBeNull();
      return p as HTMLElement;
    });
    expect(line.textContent).toBe("A newer release is available: v0.7.0. This site runs 0.6.0");
    expect(line).toHaveClass("muted", "release-notice");
    const link = screen.getByRole("link", { name: "v0.7.0" });
    expect(link).toHaveAttribute("href", RELEASE_PAGE);
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener");
    expect(line.querySelectorAll("a"), "the tag is the only link").toHaveLength(1);
  });

  it("draws nothing before the feed has answered", () => {
    const { container } = render(<ReleaseNotice fetchImpl={answering({ tag_name: "v0.7.0" })} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("draws nothing when the build has no feed, and asks nothing", async () => {
    build({ feed: null });
    const fetchImpl = answering({ tag_name: "v0.7.0", html_url: RELEASE_PAGE });
    const { container } = render(<ReleaseNotice fetchImpl={fetchImpl} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(container).toBeEmptyDOMElement();
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("draws nothing when the feed names the release this site runs", async () => {
    const fetchImpl = answering({ tag_name: "v0.6.0", html_url: RELEASE_PAGE });
    const { container } = render(<ReleaseNotice fetchImpl={fetchImpl} />);
    await waitFor(() => expect(fetchImpl).toHaveBeenCalledTimes(1));
    await new Promise((r) => setTimeout(r, 0));
    expect(container).toBeEmptyDOMElement();
  });

  it("links the project's releases page when the feed names none", async () => {
    build({ project: "https://github.com/you/yokohama-assistant/" });
    render(<ReleaseNotice fetchImpl={answering({ tag_name: "v0.7.0" })} />);
    const link = await screen.findByRole("link", { name: "v0.7.0" });
    expect(link).toHaveAttribute("href", "https://github.com/you/yokohama-assistant/releases");
  });

  // Neither the feed nor the build has an address: the notice is still true, and
  // a tag with nowhere to go is text and not a dead link.
  it("draws the tag as plain text when there is nowhere to link it", async () => {
    const { container } = render(<ReleaseNotice fetchImpl={answering({ tag_name: "v0.7.0" })} />);
    await waitFor(() => expect(container.querySelector("p.release-notice")).not.toBeNull());
    expect(container.querySelector("p.release-notice")?.textContent).toBe(
      "A newer release is available: v0.7.0. This site runs 0.6.0",
    );
    expect(screen.queryByRole("link")).toBeNull();
  });

  // The same component on the door and the lobby, and a player goes from one to
  // the other: the second draws at once and the feed was asked once.
  it("is drawn by a second mount at once, from the one check", async () => {
    const fetchImpl = answering({ tag_name: "v0.7.0", html_url: RELEASE_PAGE });
    const first = render(<ReleaseNotice fetchImpl={fetchImpl} />);
    await screen.findByRole("link", { name: "v0.7.0" });
    first.unmount();
    const second = render(<ReleaseNotice fetchImpl={fetchImpl} />);
    expect(second.container.querySelector("p.release-notice")).not.toBeNull();
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });
});
