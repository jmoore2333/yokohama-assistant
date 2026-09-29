// The chat panel (M6.4 spec §6.1): one component, told where it is and when to
// look, owning its list, its fetch and its post. Every payload is the corpus's.

import { act, render, screen, waitFor, within } from "@testing-library/react";
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
      return { messages: [], last: wire.chat.last, rev: wire.chat.rev };
    }
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });

const open = () => localStorage.setItem(OPEN_KEY, JSON.stringify(["wire"]));

/** The corpus's page with nothing in it: an empty room, whose cursor is at
 *  nothing. **Derived, never hand-authored** — the `gamesNamed` idiom — and it
 *  is what makes a post's own `seq` 1 land ABOVE what the panel holds, which
 *  the corpus's three-message page does not. */
const emptyRoom: ChatPage = { ...wire.chat, messages: [], last: 0, rev: 0 };

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
    // A seat: a chip in the seat's colour class and the seat's name.
    const red = wire.chat.messages[0]!;
    if (red.author.kind !== "seat") throw new Error("the first corpus message is red's");
    expect(rows[0]!.querySelector(`.chat-who.seat-${colors[red.author.seat]!.toLowerCase()}`)).not.toBeNull();
    expect(rows[0]!).toHaveTextContent(red.author.name!);
    // The watcher: their given name, marked.
    const green = wire.chat.messages[2]!;
    if (green.author.kind !== "watcher") throw new Error("the third corpus message is the watcher's");
    expect(rows[2]!.querySelector(".chat-who.watching")).not.toBeNull();
    expect(rows[2]!).toHaveTextContent(green.author.name!);
    // **The SEPARATOR is in the needle** — the corpus text this row carries is
    // "watching from the rail", which satisfied a bare `"watching"` by the
    // message itself while the marker said that word; the marker says
    // "spectating" since M6.5c2 (R-6.5c2.7), and the separator keeps the
    // needle about the marker and never about what somebody typed.
    expect(rows[2]!).toHaveTextContent("\u00b7 spectating");
    // The text, and a time on every row.
    expect(rows[1]!).toHaveTextContent(wire.chat.messages[1]!.text);
    for (const r of rows) expect(r.querySelector("time")).not.toBeNull();
  });

  // **With no name, "you", the seat, or "someone" — never an address**
  // (M6.5c2 cycle D, R-6.5c2.35): the server sends no author's address, so the
  // fallback is the reader's own `you`, then the seat's number, then, for a
  // watcher, the word.
  it("falls back to you, the seat or someone when an author has no name", async () => {
    open();
    // **DERIVED from the corpus, never hand-authored** — every corpus author
    // carries a name, so the fallbacks had branches no fixture reached. The
    // page with each name taken away is the smallest thing that reaches them:
    // red's own message (`you`), blue's (not `you`) and the watcher's.
    const [red, blue, green] = wire.chat.messages;
    if (red?.author.kind !== "seat" || blue?.author.kind !== "seat" || green?.author.kind !== "watcher") {
      throw new Error("the corpus page is red's, blue's, then the watcher's");
    }
    expect([red.author.you, blue.author.you, green.author.you]).toEqual([true, false, false]);
    const nameless: ChatPage = {
      ...wire.chat,
      messages: [red, blue, green].map((m) => ({ ...m, author: { ...m.author, name: null } })),
    };
    chatStub((call) => (call.method === "GET" ? nameless : undefined));
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    expect(rows.map((r) => r.querySelector(".chat-who")!.firstChild!.textContent)).toEqual([
      "you",
      `seat ${blue.author.seat}`,
      "someone",
    ]);
    for (const r of rows) expect(r.textContent).not.toContain("@");
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

  /** **The badge on a WAKE host, which is where `unread` is the badge at all**
   *  (final review, Importants 2 and 3).
   *
   *  The cell above drives a COUNT host, where the badge is `count - last` and
   *  `unread` is never read — so the unfold's reset was pinned by nothing, and
   *  removing it left every suite green. Here the panel is told only "look
   *  again", `unread` IS the number on the header, and the three claims can
   *  each fail on their own:
   *
   *  - the mount read does not count (Important 3): the retained history was
   *    already there, and a wake host that counted it said "3 new" on every
   *    visit to a room nobody had spoken in since;
   *  - a message arriving while folded does count;
   *  - the unfold clears it, which a RE-FOLD is what shows — folded again with
   *    nothing further arrived, the header carries no badge at all. */
  it("counts nothing at mount on a wake host, counts what arrives, and the unfold clears it", async () => {
    let answer: ChatPage = wire.chat;
    chatStub((call) => (call.method === "GET" ? answer : undefined));
    const { rerender } = render(<Chat scope={TABLE} tick={0} colors={colors} sendName={false} />);
    await settle();
    expect(
      screen.queryByLabelText(/new$/),
      "the history retained at this table did not ARRIVE while folded",
    ).toBeNull();

    // One more message, on the next wake.
    answer = {
      messages: [{ ...wire.chat.messages[0]!, seq: wire.chat.last + 1 }],
      last: wire.chat.last + 1,
      rev: wire.chat.rev + 1,
    };
    rerender(<Chat scope={TABLE} tick={1} colors={colors} sendName={false} />);
    expect(await screen.findByLabelText("1 new")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "show" }));
    expect(screen.queryByLabelText(/new$/)).toBeNull();
    // **Re-folded is the assertion the reset can fail** — an unfold that did
    // not clear the number puts it straight back on the header.
    await userEvent.click(screen.getByRole("button", { name: "hide" }));
    await settle();
    expect(
      screen.queryByLabelText(/new$/),
      "nothing arrived while the panel was open, so folding it shows no badge",
    ).toBeNull();
  });

  /** **A stored clock is DATA** (final review, Important 1): the panel draws a
   *  number it cannot turn into a date rather than throwing out of its render.
   *
   *  `toISOString()` on an out-of-range `Date` throws `RangeError`, and there is
   *  no error boundary over this panel — so one hand-edited or half-restored
   *  document took the whole game screen down for every browser at that table.
   *  The server refuses such a value now; this is the other end of the same
   *  rule, and it has to hold on its own, because a client is not the thing
   *  that decides what is in a document. */
  it("draws a message whose stored clock is not a date, instead of throwing", async () => {
    open();
    const wrong: ChatPage = {
      ...wire.chat,
      messages: [{ ...wire.chat.messages[0]!, created_at: 1e19 }],
    };
    chatStub((call) => (call.method === "GET" ? wrong : undefined));
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(1);
    expect(rows[0]!).toHaveTextContent(wire.chat.messages[0]!.text);
    // No `datetime` at all, which is what the HTML spec asks for when there is
    // no parseable value — and the string form of the number on screen.
    expect(rows[0]!.querySelector("time")).not.toHaveAttribute("datetime");
    expect(rows[0]!.querySelector("time")!.textContent).toBe(String(1e19));
    // And `when` itself, which the panel is only one caller of.
    expect(when(1e19)).toBe(String(1e19));
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

  /** **An own post on an OPEN count-driven panel costs ZERO delta reads** (live
   *  review, M-2).
   *
   *  Spec §6.1 says the frame a post causes is "a no-op compare", because the
   *  201's message is appended locally and `last` advanced first. On the live
   *  tunnel it is not: the table's bump reaches the poster's OWN stream inside
   *  the round trip — the overlap M6.3 already measured — so the count rule
   *  fired against a cursor the 201 had not moved yet, and every post cost one
   *  delta GET whose `after` was one below the message it had just written.
   *
   *  **The ORDER here is the live one, and it is the whole cell**: the POST is
   *  held open, the count arrives WHILE it is out, and only then does the 201
   *  land. A cell that resolved the post first could not fail. */
  it("asks for no delta when its own post is what moved the count", async () => {
    open();
    const posted = gate<Reply>();
    const stub = chatStub((call) =>
      call.method === "POST" ? posted.promise : call.method === "GET" ? emptyRoom : undefined,
    );
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await settle();
    // Anti-vacuity: the mount read, and nothing else. A panel that had asked
    // for nothing at all would satisfy every count below for the wrong reason.
    const before = stub.matching("GET", "/chat").length;
    expect(before, "the mount read happened").toBe(1);

    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "good luck{Enter}");
    await waitFor(() => expect(stub.matching("POST", "/chat")).toHaveLength(1));

    // The bump lands BEFORE the 201: the count is the poster's own message and
    // the cursor has not moved yet.
    rerender(<Chat scope={TABLE} count={wire.chatPost.seq} colors={colors} sendName={false} />);
    await settle();
    expect(
      stub.matching("GET", "/chat"),
      "a count a post in flight will account for must not be asked about",
    ).toHaveLength(before);

    // And when the post settles, the re-check finds the count already covered.
    posted.resolve(wire.chatPost);
    await settle();
    expect(stub.matching("GET", "/chat"), "nor once it has landed").toHaveLength(before);
    const rows = await screen.findAllByRole("listitem");
    expect(rows, "the line is drawn once, by the 201 and by nothing else").toHaveLength(1);
    expect(rows[0]!).toHaveTextContent(wire.chatPost.text);
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
    // **And nobody is "spectating" in the house**, where everyone is (R-6.4.2).
    // The label cell above exercises the marker's PRESENCE at a table; this is
    // the other half, and without it a component that marked every watcher
    // everywhere would pass both.
    expect(screen.queryByText("\u00b7 spectating")).toBeNull();
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
    answer = {
      messages: [{ ...wire.chat.messages[0]!, seq: wire.chat.last + 1 }],
      last: wire.chat.last + 1,
      rev: wire.chat.rev + 1,
    };
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

  /** **A 201 that lands after the panel has changed rooms is DROPPED** (final
   *  review, Important 4).
   *
   *  The post was the one resolution outside the generation guard, and it is
   *  the one that moves the cursor: `take` would draw the old room's message
   *  in the new room and set `last` to the old room's number — after which
   *  every new-room message numbered below it is filtered out, silently, until
   *  that room's numbering passes it. The corpus's own post is numbered well
   *  above the empty room it would land in, which is what makes the second
   *  half of this cell able to fail.
   *
   *  The input is asserted usable again, because the early return runs through
   *  the same `finally`: a panel left disabled for ever would be the other way
   *  to pass the first assertion. */
  it("drops a post's reply when the panel has moved rooms, and leaves the new room's cursor alone", async () => {
    const posted = gate<Reply>();
    const stub = stubApi((call) => {
      if (call.method === "POST") return posted.promise;
      if (call.path.includes("/api/games/wire/chat")) return emptyRoom;
      if (call.path.includes("/api/games/other/chat")) return emptyRoom;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    // Both rooms remembered OPEN, because the count rule only fires on an open
    // panel and the second half of this cell is about the cursor the count rule
    // reads.
    localStorage.setItem(OPEN_KEY, JSON.stringify(["wire", "other"]));
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await settle();
    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "for wire{Enter}");
    await waitFor(() => expect(stub.matching("POST", "/chat")).toHaveLength(1));

    rerender(
      <Chat scope={{ kind: "table", id: "other" }} count={0} colors={colors} sendName={false} />,
    );
    await settle();
    posted.resolve(wire.chatPost);
    await settle();
    expect(screen.queryAllByRole("listitem"), "the old room's message is not in this one").toHaveLength(0);
    expect(screen.getByRole("textbox", { name: "Say something" })).toBeEnabled();

    // And the cursor is still this room's: a count of 1 asks from 0.
    const before = stub.matching("GET", "/api/games/other/chat").length;
    rerender(
      <Chat scope={{ kind: "table", id: "other" }} count={1} colors={colors} sendName={false} />,
    );
    await waitFor(() =>
      expect(stub.matching("GET", "/api/games/other/chat")).toHaveLength(before + 1),
    );
    expect(stub.matching("GET", "/api/games/other/chat").at(-1)!.path).toContain("after=0");
  });

  it("keeps a folded panel's controls out of the tab order", async () => {
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await settle();
    expect(screen.getByRole("textbox", { name: "Say something", hidden: true })).toHaveAttribute(
      "tabindex",
      "-1",
    );
    expect(screen.getByRole("button", { name: "Delete this message", hidden: true })).toHaveAttribute(
      "tabindex",
      "-1",
    );
  });
});

// ---- M6.5c2 cycle D: the delete (R-D4; Review Focus 2) ---------------------

/** The table's page as the server answers it: `start` (by default
 *  `chat.json`) until `set` or `deleted()` — `chat_deleted.json` — replaces
 *  it, each answering a cursor with its own delta, as the server does. */
function roomStub(start: ChatPage = wire.chat) {
  let page: ChatPage = start;
  const stub = chatStub((call) => {
    if (call.method === "DELETE") return new Response(wire.chatDelete, { status: 204 });
    if (call.method !== "GET") return undefined;
    const after = Number(new URL(call.path, "http://x").searchParams.get("after") ?? 0);
    return { ...page, messages: page.messages.filter((m) => m.seq > after) };
  });
  return {
    stub,
    set: (next: ChatPage) => {
      page = next;
    },
    deleted: () => {
      page = wire.chatDeleted;
    },
    reads: () => stub.matching("GET", "/chat").map((c) => new URL(c.path, "http://x").searchParams.get("after") ?? "0"),
  };
}

describe("the chat panel's delete", () => {
  /** **A tombstone is drawn as "message removed" in its author's row**,
   *  read off `chat_deleted.json` — the server's own tombstone, `text: ""` —
   *  and it carries no Delete control of its own. */
  it("draws a tombstone as 'message removed' under its author, with no control", async () => {
    open();
    chatStub((call) => (call.method === "GET" ? wire.chatDeleted : undefined));
    render(<Chat scope={TABLE} count={wire.chatDeleted.last} colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(wire.chatDeleted.messages.length);
    const gone = wire.chatDeleted.messages[0]!;
    expect(gone.deleted, "premise: the corpus's first message is the tombstone").toBe(true);
    if (gone.author.kind !== "seat") throw new Error("premise: red's seat wrote it");
    expect(rows[0]!).toHaveTextContent("message removed");
    expect(rows[0]!).toHaveTextContent(gone.author.name!);
    expect(rows[0]!.querySelector(".chat-who.seat-" + colors[gone.author.seat]!.toLowerCase())).not.toBeNull();
    expect(rows[0]!.querySelector("button"), "a tombstone cannot be deleted again").toBeNull();
    expect(rows[1]!).toHaveTextContent(wire.chatDeleted.messages[1]!.text);
    expect(rows[1]!).not.toHaveTextContent("message removed");
  });

  /** **The control is on the reader's OWN messages, and at a table the reader
   *  created on every message** (R-D4) — the server's rule, drawn: red holds
   *  message 1 (`you: true`); blue's and the watcher's are red's to take down
   *  only as the table's creator. */
  it("offers Delete on the reader's own messages, and on every message at a table they created", async () => {
    open();
    chatStub();
    const { unmount } = render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    let rows = await screen.findAllByRole("listitem");
    const own = wire.chat.messages.map((m) => m.author.kind !== "bot" && m.author.you);
    expect(own, "premise: red's own message, and two that are not").toEqual([true, false, false]);
    expect(rows.map((r) => r.querySelector("button") !== null)).toEqual(own);
    unmount();
    render(<Chat scope={TABLE} count={wire.chat.last} canModerate colors={colors} sendName={false} />);
    rows = await screen.findAllByRole("listitem");
    expect(rows.map((r) => r.querySelector("button") !== null)).toEqual([true, true, true]);
  });

  /** **Delete asks the server, and the SIGNAL shows the result** (R-D4;
   *  Review Focus 2). The 204 changes nothing on screen by itself; the table's
   *  next frame carries `chat_rev` one past its count (`events.json`), and the
   *  panel re-reads what it holds — so the row every OTHER reader holding the
   *  message sees change is the row this one sees change too, by the same
   *  road. */
  it("sends the delete and draws the tombstone when the frame's chat_rev moves", async () => {
    open();
    const room = roomStub();
    const { rerender } = render(
      <Chat scope={TABLE} count={wire.chat.last} rev={wire.chat.rev} colors={colors} sendName={false} />,
    );
    await screen.findAllByRole("listitem");
    await userEvent.click(within(screen.getAllByRole("listitem")[0]!).getByRole("button", { name: "Delete this message" }));
    await waitFor(() => expect(room.stub.matching("DELETE", "/api/games/wire/chat/1")).toHaveLength(1));
    await settle();
    expect(screen.getAllByRole("listitem")[0]!, "the reply alone changes nothing").toHaveTextContent(wire.chat.messages[0]!.text);
    room.deleted();
    rerender(
      <Chat scope={TABLE} count={wire.events.chat} rev={wire.events.chat_rev} colors={colors} sendName={false} />,
    );
    await waitFor(() => expect(screen.getAllByRole("listitem")[0]!).toHaveTextContent("message removed"));
    expect(room.reads().at(-1), "the page it holds, re-read from the start").toBe("0");
  });

  /** **A moved `rev` re-reads the page; an append does not** (the
   *  orchestrator's ruling, Q1) — on a COUNT host. `chat.json`'s numbers are
   *  the panel's to start from; an append is the count and the revision moving
   *  together (`chat_after.json`'s message, one delta read and no re-read), and
   *  a delete is the revision moving alone (`events.json`, whose `chat_rev` is
   *  one past its `chat`: one re-read from 0, `chat_deleted.json`). Folded,
   *  the re-read waits for the unfold, as the count rule's read does. */
  it("re-reads on a moved revision and not on an append, and waits while folded", async () => {
    open();
    // Two held — `chat.json` as it read before its third message — so that
    // message, `chat_after.json`'s, arrives as an append.
    const room = roomStub({ ...wire.chat, messages: wire.chat.messages.slice(0, 2), last: 2, rev: 2 });
    const { rerender } = render(<Chat scope={TABLE} count={2} rev={2} colors={colors} sendName={false} />);
    await settle();
    expect(room.reads(), "the mount").toEqual(["0"]);
    room.set(wire.chat);
    rerender(<Chat scope={TABLE} count={wire.chatAfter.last} rev={wire.chatAfter.rev} colors={colors} sendName={false} />);
    await settle();
    expect(room.reads(), "an append: one delta, no re-read").toEqual(["0", "2"]);
    expect(screen.getAllByRole("listitem")).toHaveLength(3);

    room.deleted();
    await userEvent.click(screen.getByRole("button", { name: "hide" }));
    rerender(<Chat scope={TABLE} count={wire.events.chat} rev={wire.events.chat_rev} colors={colors} sendName={false} />);
    await settle();
    expect(room.reads(), "folded: the re-read waits").toEqual(["0", "2"]);
    await userEvent.click(screen.getByRole("button", { name: "show" }));
    await waitFor(() => expect(room.reads()).toEqual(["0", "2", "0"]));
    await waitFor(() => expect(screen.getAllByRole("listitem")[0]!).toHaveTextContent("message removed"));
  });

  /** **A WAKE host learns of a delete from the page's own `rev`** (Q1): the
   *  open table and the lobby drop their frames and pass a tick, so the delta
   *  read is the panel's only news — and when its `rev - last` has moved, the
   *  panel re-reads what it holds. An append on a tick re-reads nothing. */
  it("re-reads on a tick whose delta shows the revision moved, and not on an append", async () => {
    open();
    const room = roomStub();
    const { rerender } = render(<Chat scope={TABLE} tick={0} colors={colors} sendName={false} />);
    await settle();
    rerender(<Chat scope={TABLE} tick={1} colors={colors} sendName={false} />);
    await settle();
    expect(room.reads(), "a tick with nothing new: the delta alone").toEqual(["0", "3"]);
    room.deleted();
    rerender(<Chat scope={TABLE} tick={2} colors={colors} sendName={false} />);
    await waitFor(() => expect(room.reads()).toEqual(["0", "3", "3", "0"]));
    await waitFor(() => expect(screen.getAllByRole("listitem")[0]!).toHaveTextContent("message removed"));
  });

  /** **A re-read keeps the reader's own post that landed while it was out**
   *  (M6.5c2 cycle D). The re-read is held open; the reader posts meanwhile
   *  and the 201 is drawn; then the re-read lands with the page as the server
   *  built it BEFORE the post (`chat_deleted.json`, `last` 3). Replacing the
   *  list with that page alone would drop the post for good — the cursor has
   *  already passed its number, so no count would fetch it again. The post is
   *  `chat_post.json` renumbered past the page, the room's next number. */
  it("keeps its own post through a re-read that was out when the post landed", async () => {
    open();
    const reread = gate<Reply>();
    let first = true;
    const posted = { ...wire.chatPost, seq: wire.chatDeleted.last + 1 };
    const stub = chatStub((call) => {
      if (call.method === "POST") return posted;
      if (call.method !== "GET") return undefined;
      if (first) {
        first = false;
        return wire.chat;
      }
      return reread.promise;
    });
    const { rerender } = render(
      <Chat scope={TABLE} count={wire.chat.last} rev={wire.chat.rev} colors={colors} sendName={false} />,
    );
    await settle();
    rerender(<Chat scope={TABLE} count={wire.events.chat} rev={wire.events.chat_rev} colors={colors} sendName={false} />);
    await waitFor(() => expect(stub.matching("GET", "/chat")).toHaveLength(2));
    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "still here{Enter}");
    await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(wire.chat.messages.length + 1));
    reread.resolve(wire.chatDeleted);
    await waitFor(() => expect(screen.getAllByRole("listitem")[0]!).toHaveTextContent("message removed"));
    const rows = screen.getAllByRole("listitem");
    expect(rows, "the page's three and the post").toHaveLength(wire.chatDeleted.messages.length + 1);
    expect(rows.at(-1)!).toHaveTextContent(posted.text);
  });

  /** **A refused delete says why and changes nothing** — the server's own
   *  sentence (`chat_delete_403.json`), and the control usable again. */
  it("draws the server's refusal and keeps the message", async () => {
    open();
    chatStub((call) => (call.method === "DELETE" ? { status: 403, body: wire.chatDelete403 } : undefined));
    render(<Chat scope={TABLE} count={wire.chat.last} canModerate colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    await userEvent.click(within(rows[1]!).getByRole("button", { name: "Delete this message" }));
    expect(await screen.findByText(wire.chatDelete403.error)).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")[1]!).toHaveTextContent(wire.chat.messages[1]!.text);
    expect(within(screen.getAllByRole("listitem")[1]!).getByRole("button", { name: "Delete this message" })).toBeEnabled();
  });

  /** **At the house the delete is the house's route** — the author's own
   *  post, from `house_chat_post.json`'s seq. */
  it("sends a house message's delete to the house", async () => {
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    const stub = stubApi((call) => {
      if (call.method === "DELETE") return new Response(wire.houseChatDelete, { status: 204 });
      if (call.method === "GET") return { ...wire.houseChat, messages: [{ ...wire.houseChat.messages[0]!, author: wire.houseChatPost.author }] };
      return undefined;
    });
    render(<Chat scope={{ kind: "house" }} tick={0} sendName heading="h3" />);
    const [row] = await screen.findAllByRole("listitem");
    await userEvent.click(within(row!).getByRole("button", { name: "Delete this message" }));
    await waitFor(() => expect(stub.matching("DELETE", `/api/chat/${wire.houseChatPost.seq}`)).toHaveLength(1));
  });
});
