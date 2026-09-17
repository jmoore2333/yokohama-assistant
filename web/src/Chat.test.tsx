// The chat panel (M6.4 spec §6.1): one component, told where it is and when to
// look, owning its list, its fetch and its post. Every payload is the corpus's.

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Chat, when } from "./Chat";
import { DISPLAY_NAME_KEY, setChatText } from "./api";
import { OPEN_KEY } from "./chatFold";
import { gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import type { ChatPage } from "./types";

const TABLE = { kind: "table", id: "wire" } as const;
const colors = wire.table.colors;

/** A stub answering the table's chat from the corpus: everything at `after=0`,
 *  the delta at `after=2`, nothing above. */
const chatStub = (extra?: (c: ApiCall) => Reply | undefined) =>
  stubApi((call) => {
    const mine = extra?.(call);
    if (mine !== undefined) return mine;
    if (call.method === "GET" && call.path.includes("/api/games/wire/chat")) {
      const after = Number(new URL(call.path, "http://x").searchParams.get("after") ?? 0);
      if (after === 0) return wire.chat;
      if (after < wire.chat.last)
        return { ...wire.chat, messages: wire.chat.messages.filter((m) => m.seq > after) };
      return { messages: [], last: wire.chat.last };
    }
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });

const open = () => localStorage.setItem(OPEN_KEY, JSON.stringify(["wire"]));

/** The corpus's page with nothing in it: an empty room, whose cursor is at
 *  nothing. **Derived, never hand-authored** — the `gamesNamed` idiom — and it
 *  is what makes a post's own `seq` 1 land ABOVE what the panel holds, which
 *  the corpus's three-message page does not. */
const emptyRoom: ChatPage = { ...wire.chat, messages: [], last: 0 };

/** Let the panel's fetch chain finish — including the re-check a trigger that
 *  arrived mid-flight defers (ruling R-pre-4).
 *
 *  REAL turns of the event loop, not `act(async () => {})` alone: a body
 *  arrives through the platform's own machinery and not every step of it is a
 *  microtask, which is the measurement `harness.tsx`'s `flush` records about
 *  streams. It is what makes the cells below able to FAIL — a second fetch
 *  issued one turn after the first resolves is not a second fetch a
 *  microtask-deep settle can see. */
const settle = async (rounds = 4) => {
  for (let i = 0; i < rounds; i += 1)
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });
};

beforeEach(() => localStorage.clear());
afterEach(() => vi.unstubAllGlobals());

describe("the chat panel", () => {
  it("is folded by default, and remembers being opened", async () => {
    const stub = chatStub();
    const { unmount } = render(
      <Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />,
    );
    const fold = await screen.findByRole("button", { name: "show" });
    expect(fold).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(fold);
    expect(screen.getByRole("button", { name: "hide" })).toHaveAttribute("aria-expanded", "true");
    expect(JSON.parse(localStorage.getItem(OPEN_KEY)!)).toContain("wire");
    unmount();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    expect(await screen.findByRole("button", { name: "hide" })).toBeInTheDocument();
    // **A mount is ONE fetch, whatever the fold state and whatever the count
    // arrives with** (ruling R-pre-4): the second mount is a REMEMBERED-open
    // panel whose `count` is already above the nothing it holds, which is the
    // shape that asks twice if the count rule fires beside the mount read.
    await settle();
    expect(stub.matching("GET", "/chat")).toHaveLength(2);
  });

  it("fetches everything on mount and draws the three authors their own way", async () => {
    open();
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(wire.chat.messages.length);
    // A seat: a chip in the seat's colour class and the seat's name or email.
    const red = wire.chat.messages[0]!;
    if (red.author.kind !== "seat") throw new Error("the first corpus message is red's");
    expect(rows[0]!.querySelector(`.chat-who.seat-${colors[red.author.seat]!.toLowerCase()}`)).not.toBeNull();
    expect(rows[0]!).toHaveTextContent(red.author.name ?? red.author.user);
    // The watcher: their given name, marked.
    const green = wire.chat.messages[2]!;
    if (green.author.kind !== "watcher") throw new Error("the third corpus message is the watcher's");
    expect(rows[2]!.querySelector(".chat-who.watching")).not.toBeNull();
    expect(rows[2]!).toHaveTextContent(green.author.name!);
    expect(rows[2]!).toHaveTextContent("watching");
    // The text, and a time on every row.
    expect(rows[1]!).toHaveTextContent(wire.chat.messages[1]!.text);
    for (const r of rows) expect(r.querySelector("time")).not.toBeNull();
  });

  it("fetches the delta when the count rises above what it holds, and NOTHING when it does not", async () => {
    open();
    const stub = chatStub();
    const { rerender } = render(
      <Chat scope={TABLE} count={0} colors={colors} sendName={false} />,
    );
    await screen.findAllByRole("listitem");
    const before = stub.matching("GET", "/chat").length;
    // Ten frames whose count did not move past what the panel holds (mutation 10).
    for (let i = 0; i < 10; i += 1) {
      rerender(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
      rerender(<Chat scope={TABLE} count={wire.chat.last - 1} colors={colors} sendName={false} />);
    }
    await settle();
    expect(stub.matching("GET", "/chat")).toHaveLength(before);
    // And a count above it fetches ONCE, with the cursor.
    rerender(<Chat scope={TABLE} count={wire.chat.last + 1} colors={colors} sendName={false} />);
    await waitFor(() => expect(stub.matching("GET", "/chat")).toHaveLength(before + 1));
    expect(stub.matching("GET", "/chat").at(-1)!.path).toContain(`after=${wire.chat.last}`);
    // And not again for the same count, once that delta has landed.
    await settle();
    expect(stub.matching("GET", "/chat")).toHaveLength(before + 1);
  });

  it("fetches on a tick, folded or not, and never draws a held message twice", async () => {
    // **Every chat GET answers the WHOLE page** (ruling R-pre-5): a stub that
    // answered the cursor would hand the tick an EMPTY delta, and the filter
    // this cell names would never meet a message the panel already holds.
    const stub = chatStub((call) =>
      call.method === "GET" && call.path.includes("/chat") ? wire.chat : undefined,
    );
    const { rerender } = render(<Chat scope={TABLE} tick={0} colors={colors} sendName={false} />);
    await settle();
    const before = stub.matching("GET", "/chat").length;
    // **The mount is ONE fetch here too** (ruling R-pre-4): the host's first
    // tick IS the mount's own frame, and a panel that answered both would ask
    // twice before anything had happened.
    expect(before).toBe(1);
    rerender(<Chat scope={TABLE} tick={1} colors={colors} sendName={false} />);
    await waitFor(() => expect(stub.matching("GET", "/chat")).toHaveLength(before + 1));
    // The delta comes back carrying the messages the panel already holds: one
    // row per seq, still.
    await settle();
    await userEvent.click(await screen.findByRole("button", { name: "show" }));
    expect(screen.getAllByRole("listitem")).toHaveLength(wire.chat.messages.length);
  });

  it("counts new messages on the folded header and clears them on unfold", async () => {
    chatStub();
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await settle();
    rerender(<Chat scope={TABLE} count={wire.chat.last + 2} colors={colors} sendName={false} />);
    expect(await screen.findByLabelText("2 new")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "show" }));
    expect(screen.queryByLabelText(/new$/)).toBeNull();
  });

  it("posts on Enter, appends the reply, sends no name for a seat, and advances its cursor", async () => {
    open();
    localStorage.setItem(DISPLAY_NAME_KEY, "Red");
    // **The room starts EMPTY**, so the 201 is the only thing that can put a
    // row on the screen and the only thing that can move the cursor. Against
    // the corpus's own three-message page the post's `seq` 1 is already held,
    // the filter drops it, and every assertion below passes whether or not the
    // reply was taken at all.
    const stub = chatStub((call) =>
      call.method === "POST" ? wire.chatPost : call.method === "GET" ? emptyRoom : undefined,
    );
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await settle();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "good luck{Enter}");
    const posts = await waitFor(() => {
      const p = stub.matching("POST", "/chat");
      expect(p).toHaveLength(1);
      return p;
    });
    expect(posts[0]!.body).toEqual({ text: "good luck" });
    expect(screen.getByRole("textbox", { name: "Say something" })).toHaveValue("");
    // The 201 itself is what is drawn — the room has nothing else in it.
    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(1);
    expect(rows[0]!).toHaveTextContent(wire.chatPost.text);
    // And the cursor it advanced: the frame it causes is a no-op compare,
    const before = stub.matching("GET", "/chat").length;
    rerender(<Chat scope={TABLE} count={wire.chatPost.seq} colors={colors} sendName={false} />);
    await settle();
    expect(stub.matching("GET", "/chat")).toHaveLength(before);
    // and the next frame above it asks from the number the 201 left behind.
    rerender(<Chat scope={TABLE} count={wire.chatPost.seq + 1} colors={colors} sendName={false} />);
    await waitFor(() => expect(stub.matching("GET", "/chat")).toHaveLength(before + 1));
    expect(stub.matching("GET", "/chat").at(-1)!.path).toContain(`after=${wire.chatPost.seq}`);
  });

  it("sends the browser's name for a watcher and at the house", async () => {
    open();
    localStorage.setItem(OPEN_KEY, JSON.stringify(["wire", "house"]));
    localStorage.setItem(DISPLAY_NAME_KEY, "Green");
    const stub = stubApi((call) => {
      if (call.method === "GET" && call.path.includes("/chat")) return wire.houseChat;
      if (call.method === "POST") return wire.chatPostWatcher;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    render(<Chat scope={{ kind: "house" }} tick={0} sendName />);
    const user = userEvent.setup();
    await user.type(await screen.findByRole("textbox", { name: "Say something" }), "hi all{Enter}");
    await waitFor(() => expect(stub.matching("POST", "/api/chat")).toHaveLength(1));
    expect(stub.matching("POST", "/api/chat")[0]!.body).toEqual({ text: "hi all", name: "Green" });
  });

  it("renders the server's refusal in its own line and keeps the draft", async () => {
    open();
    chatStub((call) => (call.method === "POST" ? { status: 400, body: wire.chat400Long } : undefined));
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await screen.findAllByRole("listitem");
    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "too long{Enter}");
    expect(await screen.findByText(wire.chat400Long.error)).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Say something" })).toHaveValue("too long");
  });

  it("sizes its input from the server's cap", async () => {
    open();
    setChatText(140);
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    expect(await screen.findByRole("textbox", { name: "Say something" })).toHaveAttribute(
      "maxlength",
      "140",
    );
    setChatText(500);
  });

  it("dates a message that is not from today", () => {
    const now = Date.UTC(2026, 8, 16, 12, 0, 0);
    expect(when(Math.floor(now / 1000), now, "en-US")).not.toMatch(/Sep|16/);
    expect(when(Math.floor((now - 2 * 86_400_000) / 1000), now, "en-US")).toMatch(/Sep/);
  });

  it("opens no connection of its own (mutation 22)", async () => {
    open();
    const stub = chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await screen.findAllByRole("listitem");
    await settle();
    const streams = stub.calls.filter(
      (c) => c.path.endsWith("/stream") || c.headers["accept"] === "text/event-stream",
    );
    expect(streams).toEqual([]);
  });

  it("keeps the newest message in view, and leaves a reader who has scrolled up alone", async () => {
    // **jsdom has no layout engine**: `scrollHeight`, `clientHeight` and
    // `scrollTop` are all 0 and a write to the last is ignored, so the rule is
    // unobservable until the box is given a height of its own. These getters
    // are the smallest geometry that can tell the two instants apart — the
    // height GROWS with the rows, so a follow computed before the append lands
    // on a different number from one applied after it. The real geometry is the
    // browser walk's to check.
    const ROW = 100;
    let answer: ChatPage = emptyRoom;
    chatStub((call) => (call.method === "GET" ? answer : undefined));
    const { rerender } = render(<Chat scope={TABLE} tick={0} colors={colors} sendName={false} />);
    await settle();
    const box = document.querySelector<HTMLOListElement>("ol.chat-lines")!;
    let top = 0;
    Object.defineProperty(box, "clientHeight", { configurable: true, get: () => ROW });
    Object.defineProperty(box, "scrollHeight", {
      configurable: true,
      get: () => box.querySelectorAll("li").length * ROW,
    });
    Object.defineProperty(box, "scrollTop", {
      configurable: true,
      get: () => top,
      set: (v: number) => {
        top = v;
      },
    });
    // An empty box IS at its bottom, so the three that arrive are followed —
    // to the height AFTER they are in the DOM, which is the whole of R-6-1.
    answer = wire.chat;
    rerender(<Chat scope={TABLE} tick={1} colors={colors} sendName={false} />);
    await waitFor(() =>
      expect(screen.getAllByRole("listitem")).toHaveLength(wire.chat.messages.length),
    );
    await settle();
    expect(top).toBe(wire.chat.messages.length * ROW);
    // Now the reader scrolls back up, and the next message does not yank them
    // down. The fourth is the corpus's own first, renumbered above the page.
    top = 0;
    answer = { messages: [{ ...wire.chat.messages[0]!, seq: wire.chat.last + 1 }], last: wire.chat.last + 1 };
    rerender(<Chat scope={TABLE} tick={2} colors={colors} sendName={false} />);
    await waitFor(() =>
      expect(screen.getAllByRole("listitem")).toHaveLength(wire.chat.messages.length + 1),
    );
    await settle();
    expect(top).toBe(0);
  });

  it("issues the new scope's read at once and drops the answer for the one it left", async () => {
    const held = gate<Reply>();
    const stub = stubApi((call) => {
      if (call.path.includes("/api/games/wire/chat")) return held.promise;
      if (call.path.includes("/api/games/other/chat")) return emptyRoom;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await settle();
    // The first table's answer is still out when the panel is moved. The new
    // scope's mount read is NOT deferred behind it: a panel that waited would
    // show the new table nothing at all until something else happened.
    rerender(
      <Chat scope={{ kind: "table", id: "other" }} count={0} colors={colors} sendName={false} />,
    );
    await waitFor(() => expect(stub.matching("GET", "/api/games/other/chat")).toHaveLength(1));
    expect(stub.matching("GET", "/api/games/other/chat")[0]!.path).toContain("after=0");
    await settle();
    // And when the table it LEFT finally answers, that answer is another
    // room's: three messages, numbered from a cursor this panel has reset.
    held.resolve(wire.chat);
    await settle();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });

  it("keeps a folded panel's controls out of the tab order", async () => {
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await settle();
    expect(screen.getByRole("textbox", { name: "Say something", hidden: true })).toHaveAttribute(
      "tabindex",
      "-1",
    );
  });
});
