// Setup: the two forms (M6.5c2, spec §2.2, R-6.5c2.8, mock 07).
//
// **What a create asks, and nothing else.** The table form (`#/new`): how many
// seats, who holds each (you / a specific person / an open seat / the bot), who
// plays first, what the table is called, whether it wants a password — and,
// folded away, which seed. The form for a game on this device (`#/new/local`):
// its name and who plays first. "Your name" is on NEITHER since M6.5c2: the
// lobby's corner owns it, and a create sends the name this browser keeps.
//
// Every payload rendered is the corpus's; the bot's label is the contract file
// the component itself imports (spec §2.4).

import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { drawSeat, NewGame } from "./NewGame";
import { DISPLAY_NAME_KEY } from "./api";
import { patchServerFacts, resetFactsForTest } from "./local/facts";
import { resetLocalHostForTests } from "./local/client";
import { trimSeed } from "./seedRule";
import { currentWorker, stubWorker } from "./test/fakeWorker";
import { gate, isCreate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import productionBot from "./wire/production_bot.json";

/** A seed at the top of the `u64` range — the shape `draw_seed` actually
 *  produces, and the shape a JSON number cannot hold. */
const BIG_SEED = "18446744073709551557";
/** Who this browser is signed in as — from the DOOR, never from storage. */
const ME = "red@wire.test";
/** The bot this build deals, by its label (the contract file, spec §2.4). */
const BOT = productionBot.label;

/** The create posts, and NOT the other posts beside them. */
const creates = (stub: ReturnType<typeof stubApi>): ApiCall[] =>
  stub.calls.filter(isCreate);

/** The opening reply: a create answers 201 with a DEALING table (§4.2). */
const created = (call: ApiCall): Reply | undefined =>
  isCreate(call)
    ? { status: 201, body: wire.tableDealing }
    : undefined;

/** The table form over a stub. It fetches NOTHING on mount. */
function show(handler: (call: ApiCall) => Reply | Promise<Reply> = created) {
  const onCreated = vi.fn();
  const onCancel = vi.fn();
  const stub = stubApi(handler);
  render(<NewGame me={ME} onCreated={onCreated} onCancel={onCancel} />);
  return { onCreated, onCancel, stub, user: userEvent.setup() };
}

/** One seat's choice, by its colour — the accessible name the form gives it. */
const seat = (color: string) => screen.getByRole("combobox", { name: `${color} seat` });
/** One segment of a segmented control, inside its own group. */
const segment = (group: string, name: string) =>
  within(screen.getByRole("group", { name: group })).getByRole("radio", { name });
/** The create's body. */
const posted = (stub: ReturnType<typeof stubApi>) => creates(stub)[0]!.body as Record<string, unknown>;

beforeEach(() => {
  // The door's name, as `/api/health` says it under the local server: `open`
  // asks for a NAME where Access asks for an address. A cell about the other
  // says so.
  patchServerFacts({ identity: "open" });
});
afterEach(() => {
  resetFactsForTest();
});

describe("the table form's seats", () => {
  it("posts one seat per colour \u2014 you and the bot at two players \u2014 and reads Deal", async () => {
    const { stub, onCreated, user } = show();
    // **Level 3**: the band's LAST crumb is the page's `<h2>` (Task 2), and it
    // reads "New table" too — the card titles itself a level below (mock 07).
    expect(screen.getByRole("heading", { name: "New table", level: 3 })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seats).toEqual([{ user: ME }, { bot: true }]);
    expect(posted(stub).colors).toEqual(["Red", "Blue"]);
    expect(onCreated).toHaveBeenCalledWith(wire.tableDealing);
  });

  // **R-P10, on the screen and not only in the server's refusal**, and the
  // BODY rather than the option list: React marks no option selected when the
  // value matches none, and the browser then shows the first, so a row could
  // READ one thing while the form held another.
  it("at three players offers no bot, opens the other seats, and reads Open table", async () => {
    const { stub, user } = show();
    await user.click(segment("Players", "3"));
    for (const c of ["Blue", "Yellow"]) {
      expect(within(seat(c)).queryByRole("option", { name: "The bot" })).toBeNull();
    }
    expect(screen.getByText(/offered at two players only/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Open table" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seats).toEqual([{ user: ME }, { open: true }, { open: true }]);
  });

  // Two players is where the bot IS offered, with the version it plays as
  // (R-6.5c2.10: shown, never chosen) — what makes the absence above a rule.
  it("offers the bot at two players, and names its version beside the seat it takes", () => {
    show();
    expect(within(seat("Blue")).getByRole("option", { name: "The bot" })).toBeInTheDocument();
    expect(seat("Blue")).toHaveValue("bot");
    expect(within(seat("Blue").closest(".form-seat")!).getByText(BOT)).toBeInTheDocument();
  });

  // **Review Focus 4's key, at the one place a form types a key for somebody
  // else.** On an open-door server a person is a NAME, normalized by the
  // server's own rule (`nameRule.ts`), so "  Ben   Castle " seats `ben castle` —
  // the key they reach the same seats under whichever way they type it.
  it("seats a specific person by NAME on an open door, as the key the server keys it by", async () => {
    const { stub, user } = show();
    await user.selectOptions(seat("Blue"), "person");
    await user.type(screen.getByLabelText("Blue player's name"), "  Ben   Castle ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seats).toEqual([{ user: ME }, { user: "ben castle" }]);
  });

  // ...and an ADDRESS behind Access, where the server's identity is an email.
  it("asks for an ADDRESS behind Access, and posts it normalized", async () => {
    patchServerFacts({ identity: "cf-access" });
    const { stub, user } = show();
    await user.selectOptions(seat("Blue"), "person");
    const field = screen.getByLabelText("Blue player's address");
    expect(field).toHaveAttribute("type", "email");
    await user.type(field, "Blue@Wire.test");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seats).toEqual([{ user: ME }, { user: "blue@wire.test" }]);
  });

  // A person the server would not key is refused IN PLACE, and a blank one
  // holds the button without a sentence — nothing typed is not a mistake yet.
  it("holds the submit while a specific person is blank or unkeyable, and says why once typed", async () => {
    const { stub, user } = show();
    await user.selectOptions(seat("Blue"), "person");
    const deal = screen.getByRole("button", { name: "Deal" });
    expect(deal).toBeDisabled();
    const field = screen.getByLabelText("Blue player's name");
    await user.type(field, "..");
    expect(field).toHaveAttribute("aria-invalid", "true");
    expect(field.closest(".form-seat")!.querySelector(".field-error")).not.toBeNull();
    expect(deal).toBeDisabled();
    await user.clear(field);
    await user.type(field, "Ben");
    expect(field).not.toHaveAttribute("aria-invalid", "true");
    expect(deal).toBeEnabled();
    expect(creates(stub)).toHaveLength(0);
  });

  // **The hot seat is still reachable, ASKED FOR rather than the default.**
  it("seats me in every seat at four players when every row says so", async () => {
    const { stub, user } = show();
    await user.click(segment("Players", "4"));
    for (const c of ["Blue", "Yellow", "Green"]) await user.selectOptions(seat(c), "me");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).players).toBe(4);
    expect(posted(stub).colors).toEqual(["Red", "Blue", "Yellow", "Green"]);
    expect(posted(stub).seats).toEqual([{ user: ME }, { user: ME }, { user: ME }, { user: ME }]);
  });

  // The rows are a derivation of the count and not a one-way latch.
  it("puts the bot back when the count comes back down to two", async () => {
    const { stub, user } = show();
    await user.click(segment("Players", "4"));
    await user.click(segment("Players", "2"));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seats).toEqual([{ user: ME }, { bot: true }]);
  });

  // **The button says which of the two things the click does**, both arms.
  it("reads Deal with every seat filled and Open table with one open", async () => {
    const { user } = show();
    expect(screen.queryByRole("button", { name: "Open table" })).toBeNull();
    await user.selectOptions(seat("Blue"), "open");
    expect(screen.getByRole("button", { name: "Open table" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Deal" })).toBeNull();
  });
});

// ---- who plays first (R-6.5c2.8: closes the "who starts" item) -----------
describe("the first player", () => {
  it("posts the colour chosen, and Red when nobody chose", async () => {
    const { stub, user } = show();
    expect(segment("First player", "Red")).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).start_seat).toBe(0);
  });

  it("posts Blue as seat 1", async () => {
    const { stub, user } = show();
    await user.click(segment("First player", "Blue"));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).start_seat).toBe(1);
  });

  // **Random is drawn when the form is SENT** (R-6.5c2.8), from the browser's
  // own generator — not at render, where a re-render would draw again and the
  // screen would not know which seat it had promised.
  it("draws Random from the browser's generator at the moment the form is sent", async () => {
    const draws = vi.spyOn(crypto, "getRandomValues").mockImplementation(((array: Uint8Array) => {
      array[0] = 5;
      return array;
    }) as typeof crypto.getRandomValues);
    const { stub, user } = show();
    await user.click(segment("First player", "Random"));
    expect(draws, "nothing is drawn before the form is sent").not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(draws).toHaveBeenCalledTimes(1);
    expect(posted(stub).start_seat).toBe(1); // 5 % 2
  });

  it("keeps a first player the count still has, and falls back to Red when it does not", async () => {
    const { stub, user } = show();
    // Kept: Blue is seat 1 at three players and at two (review M-1: this half
    // was named and not asserted).
    await user.click(segment("Players", "3"));
    await user.click(segment("First player", "Blue"));
    await user.click(segment("Players", "2"));
    expect(segment("First player", "Blue")).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).start_seat).toBe(1);
    // Kept: Random is a choice at every count.
    await user.click(segment("First player", "Random"));
    await user.click(segment("Players", "4"));
    expect(segment("First player", "Random")).toBeChecked();
    // Fallen back: Yellow is seat 2, which two players do not have.
    await user.click(segment("First player", "Yellow"));
    await user.click(segment("Players", "2"));
    expect(segment("First player", "Red")).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(2));
    expect((creates(stub)[1]!.body as { start_seat: number }).start_seat).toBe(0);
  });
});

/** `drawSeat` is fair over every count the form offers: a byte at or past the
 *  largest multiple of `n` is drawn again rather than folded onto seat 0. */
describe("the draw", () => {
  it("draws again past the largest multiple of the count", () => {
    const bytes = [255, 254, 4];
    vi.spyOn(crypto, "getRandomValues").mockImplementation(((array: Uint8Array) => {
      array[0] = bytes.shift()!;
      return array;
    }) as typeof crypto.getRandomValues);
    // 256 % 3 is 1, so only 255 is past the last multiple: it is drawn again,
    // and 254 % 3 = 2 is the seat.
    expect(drawSeat(3)).toBe(2);
    expect(drawSeat(4)).toBe(0); // 4 % 4
  });
});

// ---- the name, the display name and the password ---------------------------
describe("what the table is called, and who you are at it", () => {
  // **No subtext** (R-6.5c2.8): the placeholder says it, once.
  it("offers a name field whose placeholder is the whole explanation, and posts what is typed", async () => {
    const { stub, user } = show();
    const name = screen.getByLabelText("Name");
    expect(name).toHaveAttribute("placeholder", "ID shown in the lobby if left blank");
    expect(name.closest(".form-field")!.querySelectorAll("p")).toHaveLength(0);
    await user.type(name, "Thursday");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).name).toBe("Thursday");
  });

  // **"Your name" left the form** (R-6.5c2.8): the lobby's corner owns it. What
  // a create sends is the name this browser KEEPS, read when the form is sent —
  // so a name changed in the corner a moment ago is the one this seat takes.
  it("asks for no name of yours, and sends the one this browser keeps", async () => {
    const { stub, user } = show();
    expect(screen.queryByLabelText("Your name")).toBeNull();
    localStorage.setItem(DISPLAY_NAME_KEY, "  Ben  ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).display_name).toBe("Ben");
  });

  it("sends no display_name where this browser keeps none", async () => {
    const { stub, user } = show();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect("display_name" in posted(stub)).toBe(false);
  });

  // **The toggle says what it guards** — joining AND spectating — and the box
  // it reveals carries no placeholder (R-6.5c2.8).
  it("asks for a password only once ticked, with no placeholder", async () => {
    const { user } = show();
    expect(screen.queryByLabelText("Table password")).toBeNull();
    await user.click(screen.getByLabelText("Ask for a password to join or spectate"));
    expect(screen.getByLabelText("Table password")).not.toHaveAttribute("placeholder");
  });

  it("sends the password only when ticked and typed, and clears it when unticked", async () => {
    const { stub, user } = show();
    const box = screen.getByLabelText("Ask for a password to join or spectate");
    await user.click(box);
    await user.type(screen.getByLabelText("Table password"), "   ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect("password" in posted(stub)).toBe(false);

    await user.clear(screen.getByLabelText("Table password"));
    await user.type(screen.getByLabelText("Table password"), "  rosebud  ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(2));
    expect((creates(stub)[1]!.body as { password: string }).password).toBe("rosebud");

    await user.click(box);
    expect(screen.queryByLabelText("Table password")).toBeNull();
    await user.click(box);
    expect(screen.getByLabelText("Table password")).toHaveValue("");
  });
});

// ---- the seed, folded (spec §2.2 "The seed"; Review Focus 2) ---------------
describe("the seed", () => {
  const fold = () => screen.getByText("Advanced", { selector: "summary" }).closest("details")!;

  it("lives in the Advanced fold, and is sent as the exact text typed", async () => {
    // A seed a JSON number cannot hold — the premise, asserted.
    expect(String(Number(BIG_SEED))).not.toBe(BIG_SEED);
    const { stub, user } = show();
    expect(within(fold()).getByLabelText("Seed")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Seed"), BIG_SEED);
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seed).toBe(BIG_SEED);
  });

  it("sends no seed for a blank field, so the server draws one", async () => {
    const { stub, user } = show();
    await user.type(screen.getByLabelText("Seed"), "   ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(Object.keys(posted(stub)).sort()).toEqual(["colors", "players", "seats", "start_seat"]);
  });

  // **Checked AS IT IS TYPED, in the fold, and the button held** (R-6.5c2.8).
  // The fold is opened first: the field is inside it, so that is the only way
  // a browser lets anyone type there.
  it("explains a bad seed inside the fold and holds the button until it is fixed or cleared", async () => {
    const { user } = show();
    await user.click(screen.getByText("Advanced", { selector: "summary" }));
    const seed = screen.getByLabelText("Seed");
    await user.type(seed, "18446744073709551616");
    expect(within(fold()).getByText(/^Too large: a seed is a whole number/)).toBeInTheDocument();
    expect(fold()).toHaveAttribute("open");
    expect(seed).toHaveAttribute("aria-invalid", "true");
    expect(seed).toHaveAttribute("aria-describedby", "seed-error");
    expect(screen.getByRole("button", { name: "Deal" })).toBeDisabled();
    await user.clear(seed);
    expect(screen.queryByText(/^Too large:/)).toBeNull();
    expect(screen.getByRole("button", { name: "Deal" })).toBeEnabled();
  });

  // **Held open while refused, in the order a browser allows** (review I-2).
  // The seed field is inside the fold, so a refused seed is only ever typed
  // into an OPEN fold; closing it over the refusal would hide the reason the
  // button is held. So the summary's click does nothing while the seed is
  // refused, and once the seed is fixed the fold closes as any fold does.
  it("holds the fold open over a refused seed, and lets it close once the seed is fixed", async () => {
    const { user } = show();
    const summary = screen.getByText("Advanced", { selector: "summary" });
    await user.click(summary);
    expect(fold().open, "the premise: one click opens the fold").toBe(true);
    await user.type(screen.getByLabelText("Seed"), "1e3");
    await user.click(summary);
    expect(fold().open, "held open while refused").toBe(true);
    expect(within(fold()).getByText(/^Not a whole number:/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Deal" })).toBeDisabled();
    await user.clear(screen.getByLabelText("Seed"));
    await user.click(summary);
    expect(fold().open, "once fixed it closes as any fold does").toBe(false);
  });

  // **A summary that will not close says why** (M6.5c2 cycle C, the a11y pass;
  // R-B17): while the seed is refused the summary's click is cancelled, and a
  // keyboard player pressing Enter on it heard nothing happen. It points at the
  // refusal while the fold is held, and at nothing once the seed is fixed.
  it("describes the held summary by the seed's refusal, and only while it is held", async () => {
    const { user } = show();
    const summary = screen.getByText("Advanced", { selector: "summary" });
    expect(summary).not.toHaveAttribute("aria-describedby");
    await user.click(summary);
    await user.type(screen.getByLabelText("Seed"), "1e3");
    expect(summary).toHaveAttribute("aria-describedby", "seed-error");
    expect(document.getElementById("seed-error")).toHaveTextContent(/^Not a whole number:/);
    await user.clear(screen.getByLabelText("Seed"));
    expect(summary).not.toHaveAttribute("aria-describedby");
  });

  // **(Review Focus 2) The fold agrees with the server on every committed
  // input**: set as the exact value (a typed tab would move the focus), the
  // field is quiet and the button live exactly where `WireSeed` takes the text
  // or the text is blank, and refused and held everywhere else.
  it("agrees with the server's verdict on every input in the committed table", () => {
    show();
    const seed = screen.getByLabelText("Seed");
    for (const c of wire.seedCases) {
      fireEvent.change(seed, { target: { value: c.raw } });
      const quiet = c.ok || trimSeed(c.raw) === "";
      const label = JSON.stringify(c.raw);
      expect(screen.queryByText(/^(Too large|Not a whole number):/) === null, label).toBe(quiet);
      expect((screen.getByRole("button", { name: "Deal" }) as HTMLButtonElement).disabled, label).toBe(!quiet);
    }
  });

  it("hides the seed, and sends none, once a seat is another person or open", async () => {
    const { stub, user } = show();
    await user.click(screen.getByText("Advanced", { selector: "summary" }));
    await user.type(screen.getByLabelText("Seed"), "12");
    // Seat 2 becomes an OPEN seat, through the row's own select — the control
    // this file's "specific person" cells drive (`seat("Blue")`, :115).
    await user.selectOptions(seat("Blue"), "open");
    expect(screen.queryByLabelText("Seed")).toBeNull();
    expect(screen.queryByText("Advanced", { selector: "summary" })).toBeNull();
    // With a seat open the submit reads "Open table", not "Deal" (the cell at
    // :177 pins both arms).
    await user.click(screen.getByRole("button", { name: "Open table" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seed).toBeUndefined();
    expect(posted(stub).seats).toEqual([{ user: ME }, { open: true }]);
  });
});

describe("sending the form", () => {
  // **The corpus covers RESPONSES only**: `NewGameRequest` is hand-written on
  // this side, and a field this client stops sending is still a default.
  it("names every field the create endpoint reads, and no others", async () => {
    const { stub, user } = show();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(Object.keys(posted(stub)).sort()).toEqual(["colors", "players", "seats", "start_seat"]);
  });

  // **BOTH clicks inside ONE `act`**, so no render happens between them — the
  // case the ref exists for.
  it("posts create exactly once however fast Deal is clicked", async () => {
    const held = gate<unknown>();
    const { stub } = show((call) =>
      isCreate(call) ? held.promise : undefined,
    );
    const deal = screen.getByRole("button", { name: "Deal" });
    act(() => {
      fireEvent.click(deal);
      fireEvent.click(deal);
    });
    expect(creates(stub)).toHaveLength(1);
    held.resolve({ status: 201, body: wire.tableDealing });
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
  });

  // The refusal is the corpus's own body (`wire.error`), never a hand-written
  // one (the M2.5 rule; review M-4).
  it("shows a refused create's sentence rather than swallowing it", async () => {
    const { user } = show(() => ({ status: 400, body: wire.error }));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect(await screen.findByText(wire.error.error)).toBeInTheDocument();
  });

  it("goes back without sending anything on Cancel", async () => {
    const { stub, onCancel, user } = show();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(creates(stub)).toHaveLength(0);
  });
});

// ---- the LOCAL form (M6.5b, R-6.5b.8; M6.5c2 mock 07) ---------------------
describe("the form for a game on this device", () => {
  beforeEach(() => resetLocalHostForTests());
  afterEach(() => resetLocalHostForTests());

  const local = () => {
    const onCreated = vi.fn();
    const onCancel = vi.fn();
    render(<NewGame local me={ME} onCreated={onCreated} onCancel={onCancel} />);
    return { onCreated, onCancel, user: userEvent.setup() };
  };
  /** The one create this tab's Worker was sent. */
  const localBody = () => {
    const post = currentWorker().posted.find(
      (p) => p.kind === "call" && isCreate(p),
    ) as { body: string } | undefined;
    return JSON.parse(post?.body ?? "{}") as Record<string, unknown>;
  };

  it("names the bot's version, fixes the table to you and it, and asks nothing else", () => {
    stubWorker();
    local();
    expect(screen.getByRole("heading", { name: "New game against the bot" })).toBeInTheDocument();
    expect(screen.getAllByText(BOT).length).toBeGreaterThan(0);
    expect(screen.getByText("Two players: you and the bot.")).toBeInTheDocument();
    expect(screen.getByLabelText("Name")).toHaveAttribute("placeholder", "Optional");
    // Scoped to the seats: "You" and "The bot" are First player segments too.
    const seats = screen.getByRole("list", { name: "Seats" });
    expect(within(seats).getByText("You")).toBeInTheDocument();
    expect(within(seats).getByText("The bot")).toBeInTheDocument();
    // No count, no seat to switch, nobody to keep out, no seed and no Advanced.
    expect(screen.queryByRole("group", { name: "Players" })).toBeNull();
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.queryByLabelText("Ask for a password to join or spectate")).toBeNull();
    expect(screen.queryByLabelText("Seed")).toBeNull();
    expect(screen.queryByText("Advanced")).toBeNull();
  });

  it("submits through the Worker, seated by the local player, with the first player chosen", async () => {
    stubWorker();
    const { onCreated, user } = local();
    await user.click(segment("First player", "The bot"));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    const body = localBody();
    expect(body.seats).toEqual([{ user: "local@browser" }, { bot: true }]);
    expect(body.players).toBe(2);
    expect(body.colors).toEqual(["Red", "Blue"]);
    expect(body.start_seat).toBe(1);
    // The fields a local table has no answer for, and the one the Worker draws.
    expect("seed" in body).toBe(false);
    expect("password" in body).toBe(false);
    expect("id" in body).toBe(false);
    expect(fetch).not.toHaveBeenCalled();
  });

  it("draws Random at the moment it is sent", async () => {
    stubWorker();
    vi.spyOn(crypto, "getRandomValues").mockImplementation(((array: Uint8Array) => {
      array[0] = 7;
      return array;
    }) as typeof crypto.getRandomValues);
    const { onCreated, user } = local();
    await user.click(segment("First player", "Random"));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(localBody().start_seat).toBe(1); // 7 % 2
  });

  it("carries the table's name and the name this browser keeps", async () => {
    stubWorker();
    localStorage.setItem(DISPLAY_NAME_KEY, "Ben");
    const { onCreated, user } = local();
    await user.type(screen.getByLabelText("Name"), "kitchen table");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(localBody().name).toBe("kitchen table");
    expect(localBody().display_name).toBe("Ben");
  });

  it("goes back on Cancel", async () => {
    stubWorker();
    const { onCancel, user } = local();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
