// The backup and restore CONTROLS (M6.5b2, R-6.5b2.10 – .12; spec §4).
//
// `backup.test.ts` is about the rules — the clash, the transaction, the
// duplicate refusal. This file is about what a player sees and can click: the
// two downloads and what each is NAMED, the report's five outcomes in the words
// spec §4 asks for, and the unplayable list that exists because the module's own
// listing cannot carry those rows.
//
// **Every string comes from the committed envelope fixture**, as in
// `backup.test.ts`, and the verdicts come from the fake named in
// `test/fakeWorker.ts`'s own header.
import "fake-indexeddb/auto";
import { IDBFactory } from "fake-indexeddb";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { BackUpEverything, BackUpOne, Restore, Unplayable } from "./BackupControls";
import { buildEnvelope, listUnplayable, restorePairs, type Envelope } from "./backup";
import { DB_NAME, openDb, writeDocs } from "./local/db";
import { resetLocalHostForTests } from "./local/client";
import { resetFactsForTest } from "./local/facts";
import { breakTheWorker, stubWorker } from "./test/fakeWorker";
import committed from "./wire-script/backup-envelope.json";

const fixture = committed as unknown as Envelope;
const PLAYED_ID = "local-242a5429453c593c";
const RECORD_TEXT = fixture.games[0]!.record!;
const TABLE_TEXT = fixture.games[0]!.table;
const OTHER_RECORD = fixture.games[2]!.record!;

function tableFor(id: string): string {
  const out = TABLE_TEXT.replace(`"${PLAYED_ID}"`, `"${id}"`);
  if (out === TABLE_TEXT) throw new Error("the re-address changed nothing — the fixture moved");
  return out;
}

async function putPair(id: string, record: string | null, table: string): Promise<void> {
  const db = await openDb();
  await writeDocs(db, id, 0, { table, ...(record === null ? {} : { record }) });
}

/** The download's globals, replaced for one cell and put back —
 *  `App.test.tsx`'s `catchDownload`, which jsdom needs because it has no
 *  `URL.createObjectURL` at all. */
function catchDownload() {
  const clicks: { download: string }[] = [];
  const click = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(function (this: HTMLAnchorElement) {
      clicks.push({ download: this.download });
    });
  const create = vi.fn().mockReturnValue("blob:backup");
  Object.defineProperty(URL, "createObjectURL", { value: create, configurable: true });
  Object.defineProperty(URL, "revokeObjectURL", { value: vi.fn(), configurable: true });
  return { clicks, create, restore: () => click.mockRestore() };
}

const readBlob = (b: Blob): Promise<string> => b.text();

/** A file input takes a `File`, and jsdom's `DataTransfer` is not one userEvent
 *  can fill — `upload` is what drives it. */
const fileOf = (text: string): File =>
  new File([text], "backup.json", { type: "application/json" });

beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
  resetLocalHostForTests();
  resetFactsForTest();
  stubWorker();
});

afterEach(() => {
  resetLocalHostForTests();
});

describe("the backup controls", () => {
  // **The control a player reaches for when nothing else works**, and it is
  // offered with the module dead because that is the state it is for.
  it("writes one file for the whole device, with the module dead", async () => {
    await putPair("local-1", RECORD_TEXT, tableFor("local-1"));
    breakTheWorker();
    const d = catchDownload();
    try {
      render(<BackUpEverything />);
      await userEvent.click(screen.getByRole("button", { name: "Back up everything" }));
      await waitFor(() => expect(d.clicks).toHaveLength(1));
      expect(d.clicks[0]!.download).toMatch(/^yokohama-backup-\d{8}-\d{4}\.json$/);
      const blob = d.create.mock.calls[0]![0] as Blob;
      expect(blob.type).toBe("application/json");
      const env = JSON.parse(await readBlob(blob)) as Envelope;
      expect(env.games.map((g) => g.id)).toEqual(["local-1"]);
      // The RECORD's bytes are carried through, not re-serialized.
      expect(env.games[0]!.record).toBe(RECORD_TEXT);
    } finally {
      d.restore();
    }
  });

  // **Two files, two names, and they are not the same file.** "Save game file"
  // is `<id>.json`, the record alone, which is what the terminal loads; this is
  // `<id>.backup.json`, which is what this app restores.
  it("writes ONE game under a name that is not the record's", async () => {
    await putPair("local-2", RECORD_TEXT, tableFor("local-2"));
    const d = catchDownload();
    try {
      render(<BackUpOne id="local-2" />);
      await userEvent.click(screen.getByRole("button", { name: "Back up" }));
      await waitFor(() => expect(d.clicks).toHaveLength(1));
      expect(d.clicks[0]!.download).toBe("local-2.backup.json");
      expect(d.clicks[0]!.download).not.toBe("local-2.json");
      const env = JSON.parse(await readBlob(d.create.mock.calls[0]![0] as Blob)) as Envelope;
      expect(env.games[0]!.table).toBe(tableFor("local-2"));
    } finally {
      d.restore();
    }
  });

  // **The two controls say different things**, because they are different
  // files and a player reading two buttons has no other way to tell.
  it("names what each file is for, and the two names differ", async () => {
    render(<BackUpEverything />);
    expect(screen.getByText(/which this app can restore/)).toHaveTextContent(
      "Save game file",
    );
    render(<BackUpOne id="local-2" />);
    expect(screen.getByRole("button", { name: "Back up" })).toHaveAttribute(
      "title",
      "a backup this app can restore",
    );
  });

  it("says why it could not, where it could not", async () => {
    const d = catchDownload();
    try {
      render(<BackUpOne id="local-nothing" />);
      await userEvent.click(screen.getByRole("button", { name: "Back up" }));
      expect(await screen.findByText(/no game named local-nothing/)).toBeInTheDocument();
      expect(d.clicks).toHaveLength(0);
    } finally {
      d.restore();
    }
  });
});

// ---- another tab holding the OLD storage version (Review Focus 5) ---------
//
// **The control's half**, beside `backup.test.ts`'s cell for the bound itself:
// the sentence reaches the SCREEN, and the one-file-per-click guard (`busy`)
// is given back — the second click, once the old tab is closed, writes the
// file. Before the bound, the first click answered nothing and left `busy`
// set, so the second click was swallowed as well: a button that never worked
// again on that page (straw poll row 12).
//
// Here the click's OWN open is the upgrade the old tab blocks, so the browser
// says `blocked` to it and the answer is immediate — no timer is needed, and a
// control that ignored `blocked` would wait out the whole bound past the
// cell's `findByText`.
describe("Back up everything while another tab holds the old storage version", () => {
  const holdVersionOne = (): Promise<IDBDatabase> =>
    new Promise((resolve, reject) => {
      const r = indexedDB.open(DB_NAME, 1);
      r.onupgradeneeded = () => {
        const db = r.result;
        if (!db.objectStoreNames.contains("docs"))
          db.createObjectStore("docs", { keyPath: ["doc", "id"] });
        if (!db.objectStoreNames.contains("meta")) db.createObjectStore("meta", { keyPath: "id" });
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });

  it("names the other tab, and the button works again once it is closed (Review Focus 5)", async () => {
    const old = await holdVersionOne();
    const d = catchDownload();
    try {
      render(<BackUpEverything />);
      const button = screen.getByRole("button", { name: "Back up everything" });
      await userEvent.click(button);
      expect(
        await screen.findByText("another tab holds this app's storage; close it and try again"),
      ).toBeInTheDocument();
      expect(d.clicks).toHaveLength(0);

      old.close();
      await putPair("local-1", RECORD_TEXT, tableFor("local-1"));
      await userEvent.click(button);
      await waitFor(() => expect(d.clicks).toHaveLength(1));
      // The sentence goes with the refusal it was about.
      expect(screen.queryByText(/another tab holds/)).toBeNull();
    } finally {
      d.restore();
    }
  });
});

describe("the restore report", () => {
  // The five outcomes, in one render, in spec §4's words — and the CLASH names
  // its row, which is R-6.5b2.11's own sentence.
  it("renders every outcome, and the clash names its row", async () => {
    await putPair("here", RECORD_TEXT, tableFor("here"));
    await putPair("same", RECORD_TEXT, tableFor("same"));
    stubWorker({
      restore: () => [
        { id: "fresh", verdict: { kind: "accepted", players: 2, events: 595, over: true } },
        { id: "same", verdict: { kind: "accepted", players: 2, events: 595, over: true } },
        { id: "here", verdict: { kind: "accepted", players: 2, events: 595, over: true } },
        { id: "old", verdict: { kind: "unplayable", reason: "saved at semantics rev 6" } },
        { id: "junk", verdict: { kind: "rejected", reason: "the record does not parse" } },
      ],
    });
    const env = buildEnvelope([
      { id: "fresh", record: RECORD_TEXT, table: tableFor("fresh") },
      { id: "same", record: RECORD_TEXT, table: tableFor("same") },
      { id: "here", record: OTHER_RECORD, table: tableFor("here") },
      { id: "old", record: OTHER_RECORD, table: tableFor("old") },
      { id: "junk", record: "nope", table: tableFor("junk") },
    ]);
    render(<Restore />);
    await userEvent.upload(screen.getByLabelText("Restore from a backup"), fileOf(env));

    const row = async (id: string) => (await screen.findByText(id)).closest("li")!;
    expect(await row("fresh")).toHaveTextContent("restored");
    expect(await row("same")).toHaveTextContent("already on this device — nothing changed");
    expect(await row("here")).toHaveTextContent(
      "not restored — a different game is already on this device under that name",
    );
    expect(await row("old")).toHaveTextContent("restored, but it will not open here");
    expect(await row("old")).toHaveTextContent("semantics rev 6");
    expect(await row("junk")).toHaveTextContent("not restored — the record does not parse");
  });

  // A refusal about the FILE is not five rows saying nothing: it is one
  // sentence, and no report at all.
  it("shows a whole-file refusal on its own, with no rows", async () => {
    stubWorker({ restore: () => ({ refused: "this is a version 2 backup; this build reads version 1" }) });
    render(<Restore />);
    await userEvent.upload(
      screen.getByLabelText("Restore from a backup"),
      fileOf(buildEnvelope([{ id: "local-x", record: RECORD_TEXT, table: tableFor("local-x") }])),
    );
    expect(await screen.findByText(/version 2 backup/)).toBeInTheDocument();
    expect(screen.queryByText("local-x")).not.toBeInTheDocument();
  });
});

describe("the games that will not open", () => {
  // **A cross-rev pair restored on a fresh profile IS IN THE LISTING with its
  // reason** — which is a different claim from the report calling it
  // unplayable, and needs its own mechanism because the layer's listing skips
  // any table document whose shape check fails (spec §2.5).
  it("lists a restored cross-rev game with its reason, backs it up and forgets it", async () => {
    const env = JSON.parse(
      buildEnvelope([{ id: "local-old", record: OTHER_RECORD, table: tableFor("local-old") }]),
    ) as Envelope;
    env.games[0]!.semantics_rev = 6;
    await restorePairs(JSON.stringify(env));

    const d = catchDownload();
    try {
      render(<Unplayable />);
      const row = (await screen.findByText("local-old")).closest("li")!;
      expect(row).toHaveTextContent("semantics rev 6");

      // Exportable — the bytes are never rewritten, so they can be taken
      // somewhere that does read them.
      await userEvent.click(screen.getByRole("button", { name: "Back up" }));
      await waitFor(() => expect(d.clicks).toHaveLength(1));
      const out = JSON.parse(await readBlob(d.create.mock.calls[0]![0] as Blob)) as Envelope;
      expect(out.games[0]!.record).toBe(OTHER_RECORD);

      // And removable, which is the only way to be rid of one — in TWO clicks,
      // both naming the game (final review I-1).
      await userEvent.click(screen.getByRole("button", { name: "remove local-old" }));
      await userEvent.click(screen.getByRole("button", { name: "confirm removing local-old" }));
      await waitFor(() => expect(screen.queryByText("local-old")).not.toBeInTheDocument());
    } finally {
      d.restore();
    }
  });


  // **ONE click removes nothing** (final review I-1). This row's whole reason
  // for existing is R-6.5b2.12 — a save this build cannot open is "kept, listed
  // and downloadable, never erased" — and the control sat unfenced beside "Back
  // up" on the same row, which is the shape M5's review found deleting the
  // wrong game. `Abandon.tsx`'s header carries the reasoning; this cell is the
  // claim for this row.
  it("removes nothing on the first click, and names the game on both", async () => {
    const env = JSON.parse(
      buildEnvelope([{ id: "local-keep", record: OTHER_RECORD, table: tableFor("local-keep") }]),
    ) as Envelope;
    env.games[0]!.semantics_rev = 6;
    await restorePairs(JSON.stringify(env));
    render(<Unplayable />);
    await screen.findByText("local-keep");

    await userEvent.click(screen.getByRole("button", { name: "remove local-keep" }));
    // The row is still drawn, and — the claim the screen cannot make on its own
    // — the documents are still on the device.
    expect(screen.getByText("local-keep")).toBeInTheDocument();
    expect(await listUnplayable()).toEqual([
      expect.objectContaining({ id: "local-keep" }),
    ]);

    // And the way out is the one this lane always offers beside a delete.
    await userEvent.click(screen.getByRole("button", { name: "cancel removing local-keep" }));
    expect(screen.queryByRole("button", { name: "confirm removing local-keep" })).toBeNull();
    expect(await listUnplayable()).toHaveLength(1);
  });

  // Nothing to say, nothing drawn: a heading over an empty list would tell a
  // player something is wrong on a device where nothing is.
  it("draws nothing when every game opens", async () => {
    const { container } = render(<Unplayable />);
    await waitFor(() => expect(container.querySelector(".unplayable")).toBeNull());
  });
});
