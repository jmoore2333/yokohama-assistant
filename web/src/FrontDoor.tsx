// The front door at `#/` (R-6.5b2.1, R-6.5c2.6; mocks 01, 06, 11): the product's
// name and its board, and three ways on — and everyone sees it.
//
// **This screen asks the server NOTHING, and that is a ruling rather than a
// tidiness** (R-6.5b2.7): a single-player visitor is meant to cost zero Worker
// invocations. Everything it draws comes from the BUILD or from this browser:
// whether the friends card is drawn (`VITE_MULTIPLAYER`), the project link
// (`YOKOHAMA_PROJECT_URL`, `projectLink.ts`), the bot's version (the committed
// contract file `wire/production_bot.json`, which `wire_fixtures` refuses when
// it is stale), and the games in this browser (the local host's own listing).
// The runtime switch (`YOKOHAMA_WEB_MULTIPLAYER`) is what ENFORCES; this only
// informs the door, and where a deployment builds the client one way and runs
// the server the other, the link lands on `/enter`'s own 404, whose body says
// so.
//
// **"Go to the lobby" is an `<a>` and never a `fetch`.** Access answers a
// navigation with a `302` to another origin, which a `fetch` follows and then
// fails opaquely; a navigation is what the browser already knows how to do,
// and passing `/enter` is what mints the token every later request carries.
//
// **No card art, here or on any setup screen** (`ASSETS.md`): the board is an
// original drawing (`BoardMark`), and "How to play" draws the help panel
// without its screenshots, which are photographs of the art.

import { useEffect, useId, useRef, useState } from "react";
import { BoardMark } from "./BoardMark";
import { VersionTag } from "./Chips";
import { Help } from "./Help";
import { OnThisDevice } from "./OnThisDevice";
import type { ProjectLink } from "./projectLink";
import { Refusal } from "./Refusal";
import { navigate } from "./route";
import type { ReadinessStatus } from "./swRegister";
import productionBot from "./wire/production_bot.json";

/** Whether this BUILD offers the other channel.
 *
 *  Absent is ON, so a deployment that sets nothing serves both — and the one
 *  string that shuts the door is the literal `"false"`, not any falsy value: a
 *  `VITE_MULTIPLAYER=0` is a deployment that has said nothing this client can
 *  act on, and guessing would draw a door for a server that serves one.
 *
 *  A function and not a module constant, so a cell can stub the environment and
 *  read it back; it is called once per render of one screen and costs nothing. */
export function multiplayerOffered(): boolean {
  return import.meta.env.VITE_MULTIPLAYER !== "false";
}

export function FrontDoor({
  multiplayer,
  notice,
  readiness,
  link,
}: {
  /** Whether to draw the friends card. `multiplayerOffered()`'s answer, passed
   *  as a prop so this component is a screen and not a reader of the build —
   *  which is also what lets both arms be rendered in a cell (state E). */
  multiplayer: boolean;
  /** A sentence from OUTSIDE the door, drawn above the cards: a local game that
   *  ends, is abandoned or refuses to open lands here, where its siblings are
   *  listed. The lobby carries the same sentence for a table of the server's. */
  notice?: string | null;
  /** What the release's service worker is doing, from `useReadiness()` —
   *  drawn in the device card's foot. OPTIONAL: "nothing to say" is a real
   *  answer (a development build, a browser with no service workers, a page
   *  the browser walk has flagged). */
  readiness?: ReadinessStatus;
  /** The project link, from the build (`projectLink()`): a prop for
   *  `multiplayer`'s reason. Absent or `null` draws nothing. */
  link?: ProjectLink | null;
}) {
  const [help, setHelp] = useState(false);
  const helpTitle = useRef<HTMLHeadingElement | null>(null);
  const ids = useId();
  // The heading a player lands on when the help opens — focus, and never
  // `scrollIntoView` (ruling R-F11: it scrolls every scrollable ancestor).
  useEffect(() => {
    if (help) helpTitle.current?.focus();
  }, [help]);
  return (
    <div className="front-door">
      {/* **The hero, and not `header.banner`**: that is the GAME's header, and
          the browser walk's "am I in a game" waits are scoped to it
          (`e2e/harness.ts`'s `atTheGame`). */}
      <header className="door-hero">
        <div className="door-hero-in">
          <div className="door-title">
            <h1>Yokohama</h1>
            {link && (
              <a className="project-link" href={link.href} target="_blank" rel="noopener noreferrer">
                {link.text}
                <span aria-hidden="true"> &#8599;</span>
              </a>
            )}
          </div>
          <BoardMark width={360} />
        </div>
      </header>
      <main className="door-main">
        {notice && <Refusal sentence={notice} className="error door-notice" />}
        {/* **The three ways on, the channel that needs nothing first** (Task 3
            review, Minor 3): the bot card's New game needs no server, the
            friends card needs a login, and the games already here close the
            row. Under `VITE_MULTIPLAYER=false` the friends card is absent and
            the other two widen (state E). */}
        <div className={multiplayer ? "door-cards" : "door-cards two-up"}>
          <section className="raised-card door-card bot-card" aria-labelledby={`${ids}-bot`}>
            <div className="card-band yen" aria-hidden="true" />
            <div className="card-in">
              <p className="card-cap">2 players</p>
              <h2 id={`${ids}-bot`}>
                You against the bot <VersionTag label={productionBot.label} />
              </h2>
              <p className="lede">Plays entirely on this device. No account needed, and it works offline.</p>
              <button
                type="button"
                className="primary yen"
                onClick={() => navigate({ kind: "new", local: true })}
              >
                New game
              </button>
            </div>
          </section>
          {multiplayer && (
            <section className="raised-card door-card friends-card" aria-labelledby={`${ids}-friends`}>
              <div className="card-band vp" aria-hidden="true" />
              <div className="card-in">
                <p className="card-cap">2 to 4 players</p>
                <h2 id={`${ids}-friends`}>Play with friends</h2>
                <p className="lede">
                  Open a table and send the link, or take a free seat at someone else&rsquo;s. You
                  can spectate games in progress, too.
                </p>
                <a className="primary-link enter" href="/enter">
                  Go to the lobby
                </a>
              </div>
            </section>
          )}
          <OnThisDevice readiness={readiness} />
        </div>
        <div className="door-foot">
          <button
            type="button"
            className="linkish"
            aria-expanded={help}
            aria-controls={`${ids}-help`}
            onClick={() => setHelp((h) => !h)}
          >
            How to play
          </button>
        </div>
        {help && (
          <section id={`${ids}-help`} className="raised-card door-help" aria-labelledby={`${ids}-help-title`}>
            <div className="card-in">
              <div className="help-head">
                <h2 id={`${ids}-help-title`} ref={helpTitle} tabIndex={-1}>
                  How to play
                </h2>
                <button type="button" className="ghost" onClick={() => setHelp(false)}>
                  Close
                </button>
              </div>
              <Help shots={false} />
            </div>
          </section>
        )}
      </main>
    </div>
  );
}
