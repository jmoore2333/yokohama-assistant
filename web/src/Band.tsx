// The band across the top of every setup screen but the door, and the corner
// that says who is playing (M6.5c2, R-6.5c2.3 – .5; spec §2.1 and §2.2; mocks
// 02, 03, 05 and 11).
//
// **The corner answers "who am I here?" in the door's own terms** (spec §2.1).
// Under the OPEN door the name a player chose IS their identity — its key is
// what every seat of theirs is bound to — so the one thing to offer is to
// become somebody else: "switch player". Behind Cloudflare Access (the
// `cf-access` and `worker` doors) the identity is Access's and nothing this
// browser stores can change it, so what is offered is the name OTHERS see:
// "change name". Either way the identity shown is the SERVER's `me.user` —
// never what this browser happens to store under `yokohama.claimed_user`, which
// behind Access is a stale header the server ignores (the stale-name cell
// Task 4 adds to `App.test.tsx`).

import { Fragment, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { clearDisplayName, displayName, setDisplayName } from "./api";
import type { Me } from "./types";

/** The app mark at band size: the icon's own pyramid and its two colours
 *  (`web/public/icons/icon.svg`, whose geometry this is), so the band and the
 *  installed icon are one object. `[x, y, lit, weight]`. */
const ICON_TILES = [
  [35, 113, false, 0.92],
  [149, 113, false, 0.92],
  [263, 113, false, 0.92],
  [377, 113, false, 0.92],
  [92, 213, false, 0.78],
  [206, 213, true, 1],
  [320, 213, false, 0.78],
  [149, 313, false, 0.62],
  [263, 313, false, 0.62],
] as const;

function BandMark() {
  return (
    <svg
      className="band-mark"
      width={30}
      height={22}
      viewBox="30 110 450 292"
      aria-hidden="true"
      focusable="false"
    >
      {ICON_TILES.map(([x, y, lit, weight]) => (
        <rect
          key={`${x}-${y}`}
          x={x}
          y={y}
          width={100}
          height={86}
          rx={14}
          className={lit ? "lit" : undefined}
          fillOpacity={weight}
        />
      ))}
    </svg>
  );
}

/** The band: the mark, the product, where you are, and the corner.
 *
 *  **The LAST crumb is the page's heading** (`<h2>`); the ones before it are
 *  the path to it, not headings. So the lobby's band is the lobby's heading,
 *  and a form under "Lobby / New table" is headed "New table" here — a card
 *  below the band titles itself at a lower level.
 *
 *  **The path is an element of its own** (cycle E, straw poll row 60). At
 *  desktop width it is no box at all (`display: contents`), so the crumbs run
 *  on after the product's name as they always did; on a phone the product and
 *  the corner's pill take the first line and the path the line under both —
 *  it wrapped under the pill, and beside it, before. */
export function LobbyBand({ crumbs, corner }: { crumbs: string[]; corner: ReactNode }) {
  return (
    <header className="lobby-band">
      <div className="band-brand">
        <BandMark />
        <h1>Yokohama</h1>
        <div className="band-crumbs">
          {crumbs.map((crumb, i) => (
            <Fragment key={`${i}-${crumb}`}>
              <span className="crumb-sep" aria-hidden="true">
                /
              </span>
              {i === crumbs.length - 1 ? (
                <h2 className="crumb">{crumb}</h2>
              ) : (
                <span className="crumb">{crumb}</span>
              )}
            </Fragment>
          ))}
        </div>
      </div>
      <div className="band-corner">{corner}</div>
    </header>
  );
}

/** Who is playing, and the one thing the door lets them change.
 *
 *  **One set of controls, two layouts.** At desktop width the menu sits
 *  inline ("Playing as Ben · switch player"); at phone width the stylesheet
 *  hides it behind the pill ("Ben ▾"), which is a disclosure over the SAME
 *  element — so there is never a second "switch player" for anything to find.
 *
 *  `onChange` fires on switch player (the open door) and on sign out (an edge
 *  door; the auth cycle, ruling R-37), AFTER the display name is dropped — it
 *  labels whoever signs in next. App passes `signOut`, which asks the server
 *  where to go to end the session and goes, or, where it answers nowhere (the
 *  open door), drops the claimed name and re-asks `/api/me`. A display-name
 *  edit changes no identity and tells nobody. */
export function IdentityCorner({ me, onChange }: { me: Me; onChange: () => void }) {
  const menuId = useId();
  const fieldId = useId();
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState(false);
  const [shown, setShown] = useState<string | null>(() => displayName());
  const [draft, setDraft] = useState("");
  const field = useRef<HTMLInputElement | null>(null);
  /** The two controls a step backs out TO (M6.5c2 cycle C, the a11y pass):
   *  the pill, which opened the menu, and "change name", which opened the
   *  edit. Leaving a step unmounts whatever held focus inside it, and focus
   *  on a removed element falls to the document's body. */
  const pill = useRef<HTMLButtonElement | null>(null);
  const rename = useRef<HTMLButtonElement | null>(null);
  /** Set when the edit closes, and spent by the effect below once "change
   *  name" is back in the DOM — a ref, so the first render (not editing)
   *  moves no focus anywhere. */
  const backToRename = useRef(false);
  useEffect(() => {
    if (editing) field.current?.focus();
    else if (backToRename.current) {
      backToRename.current = false;
      rename.current?.focus();
    }
  }, [editing]);
  if (me.user === null) return null;
  const name = shown ?? me.user;
  const edge = me.mode !== "open";
  const stopEditing = () => {
    backToRename.current = true;
    setEditing(false);
  };
  const save = () => {
    const trimmed = draft.trim();
    if (trimmed) setDisplayName(trimmed);
    else clearDisplayName();
    setShown(trimmed || null);
    stopEditing();
  };
  return (
    <div
      className="identity-corner"
      data-open={open ? "true" : "false"}
      // **Escape backs out ONE step** (cycle C, the a11y pass): the edit to the
      // menu, keeping nothing half-typed; then the open menu to the pill. At
      // desktop width the menu is always drawn and `open` changes nothing a
      // player sees, so the second step is the phone's.
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        if (editing) {
          e.preventDefault();
          stopEditing();
        } else if (open) {
          e.preventDefault();
          setOpen(false);
          pill.current?.focus();
        }
      }}
    >
      <button
        ref={pill}
        type="button"
        className="identity-pill"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => setOpen((o) => !o)}
      >
        <i aria-hidden="true" />
        {/* **Named for what it is** (cycle C, the a11y pass): its face is a
            bare name and a caret, which read aloud as "Ben, button" — whose
            name, and why press it. The corner's own words, heard and not
            drawn. The space is a text node of its own, as the door's project
            link has it: Chromium names the pill the same either way
            (measured), but the name the cells compute trims each element's
            text ("Playing asBen"). Between flex items it draws nothing. */}
        <span className="sr-only">Playing as</span>{" "}
        <span className="identity-pill-name">{name}</span>
        <span aria-hidden="true"> ▾</span>
      </button>
      <div className="identity-menu" id={menuId}>
        {editing ? (
          <form
            className="identity-edit"
            onSubmit={(e) => {
              e.preventDefault();
              save();
            }}
          >
            <label className="sr-only" htmlFor={fieldId}>
              Your name
            </label>
            <input
              ref={field}
              id={fieldId}
              value={draft}
              maxLength={64}
              onChange={(e) => setDraft(e.target.value)}
            />
            <button type="submit" className="ghost">
              save
            </button>
            <button type="button" className="ghost" onClick={stopEditing}>
              cancel
            </button>
          </form>
        ) : (
          <>
            <span className="identity-who">
              Playing as <b title={me.user}>{name}</b>
            </span>
            {edge && shown !== null && <span className="identity-address">{me.user}</span>}
            {edge ? (
              <>
                <button
                  ref={rename}
                  type="button"
                  className="ghost"
                  onClick={() => {
                    setDraft(shown ?? "");
                    setEditing(true);
                  }}
                >
                  change name
                </button>
                {/* **The way out behind an edge door** (the auth cycle, Task 6;
                    ruling R-37): the identity is the edge's to end, and the
                    host asks the server where that is. */}
                <button
                  type="button"
                  className="ghost"
                  onClick={() => {
                    clearDisplayName();
                    onChange();
                  }}
                >
                  sign out
                </button>
              </>
            ) : (
              <button
                type="button"
                className="ghost"
                onClick={() => {
                  clearDisplayName();
                  onChange();
                }}
              >
                switch player
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}
