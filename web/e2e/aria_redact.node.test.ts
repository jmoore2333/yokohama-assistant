// **The README sidecar's redaction** (Task 8, fix round 1, review I-1).
//
// `readme_shots.spec.ts` masks every lobby row that is not one of the run's
// own tables before it photographs a page, because live the lobby is shared
// and a stranger's table, with its seat addresses, must never reach a picture.
// The accessibility snapshot written beside each picture is read by an agent,
// so it leaves out exactly what the picture masks, and a guard refuses a text
// that still names a table or an address the run did not make. These cells
// hold that function, which is pure: no browser, so `npm run test:wasm` runs it.
//
// The snapshots below are modelled on the SHAPE of a local dry run's
// `lobby-desktop.aria.txt` (two-space YAML, a `listitem` per row, the row's
// link with its `/url`); the stranger and their table are invented.

import { describe, expect, it } from "vitest";
import { MASKED_ROW, redactSidecar } from "./aria_redact";

const OWN_ID = "sim-e905ce3afac6a66a";
const STRANGER_ID = "sim-0123456789abcdef";
const STRANGER = "stranger@elsewhere.test";
const OWN = { ids: [OWN_ID], addresses: ["alice@readme.test", "ben@readme.test", "carol@readme.test"] };

const head = `- banner:
  - heading "Yokohama" [level=1]
  - heading "Lobby" [level=2]
  - text: Playing as ben@readme.test
  - button "switch player"
- button "New table"
- link "Games on this device":
  - /url: "#/"
- heading "Your tables" [level=3]
- paragraph: none yet — open a table, or take a seat at one below.
- heading "Open tables" [level=3]
- text: take a free seat
- list:
`;

const ownRow = `  - listitem:
    - link "Friday game":
      - /url: "#/t/${OWN_ID}"
    - img "password required": Password
    - text: 2 players · opened just now
    - img "Red"
    - text: alice@readme.test open seat
    - 'button "Take a seat: Friday game"': Take a seat
`;

const strangerRow = `  - listitem:
    - link "Somebody's table":
      - /url: "#/t/${STRANGER_ID}"
    - text: 2 players · opened 3 min ago
    - img "Red"
    - text: ${STRANGER} open seat
    - 'button "Take a seat: Somebody's table"': Take a seat
`;

const tail = `- heading "Being played" [level=3]
- text: spectate a game
- paragraph: nothing is being played right now.
- heading "Lobby chat show" [level=3]:
  - text: Lobby chat
  - button "show"
`;

describe("the README sidecar leaves out what the picture masks", () => {
  it("a stranger's row loses its name, link and seat labels, and the run's own row stays whole", () => {
    const text = redactSidecar(head + strangerRow + ownRow + tail, [`#/t/${STRANGER_ID}`], OWN);
    expect(text).not.toContain(STRANGER_ID);
    expect(text).not.toContain(STRANGER);
    expect(text).not.toContain("Somebody's table");
    expect(text).toContain(`  - listitem: ${MASKED_ROW}\n`);
    // the own row, byte for byte, and everything around it
    expect(text).toBe(head + `  - listitem: ${MASKED_ROW}\n` + ownRow + tail);
  });

  it("a snapshot with no stranger in it is unchanged", () => {
    const page = head + ownRow + tail;
    expect(redactSidecar(page, [], OWN)).toBe(page);
  });

  it("a stranger's row in the Being played list is left out too", () => {
    const playing = `- heading "Being played" [level=3]
- text: spectate a game
- list:
${strangerRow}`;
    const text = redactSidecar(head + ownRow + playing, [`#/t/${STRANGER_ID}`], OWN);
    expect(text).toBe(head + ownRow + `- heading "Being played" [level=3]\n- text: spectate a game\n- list:\n  - listitem: ${MASKED_ROW}\n`);
  });

  it("refuses a text in which a table id that is not the run's survives — and does not name it", () => {
    // The page reported no masked row, so the stranger's row is not redacted:
    // the guard is what stops it, and its message must not carry the id.
    const run = () => redactSidecar(head + strangerRow + ownRow + tail, [], OWN);
    expect(run).toThrow(/table id that is not this run's/);
    try {
      run();
    } catch (e) {
      expect(String(e)).not.toContain(STRANGER_ID);
    }
  });

  it("refuses a text that names an address that is not one of the run's people", () => {
    const chat = `- log:\n  - list:\n    - listitem: ${STRANGER} hello\n`;
    const run = () => redactSidecar(head + ownRow + chat, [], OWN);
    expect(run).toThrow(/address that is not one of this run's people/);
    try {
      run();
    } catch (e) {
      expect(String(e)).not.toContain(STRANGER);
    }
  });

  it("keeps the run's own ids where they appear outside a row — the invite link", () => {
    const invite = `- textbox "Invite link": http://127.0.0.1:8479/#/t/${OWN_ID}\n- button "Copy link"\n`;
    expect(redactSidecar(invite, [], OWN)).toBe(invite);
  });
});
