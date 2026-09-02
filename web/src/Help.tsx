// How to use this screen, written for someone who has the box open in front of
// them and has never seen this client.
//
// **It teaches the SCREEN, not the game.** The rules live in the rulebook and
// the engine is the only authority on what is legal here — every list this
// client shows you came from it, so "what can I do" is always answered by the
// buttons rather than by this page. What a reader cannot get anywhere else is
// which part of the screen is which, and which of the panels answers which
// question.
//
// The screenshots are OPTIONAL by construction (see `Shot`): they are generated
// from the real app by `web/e2e/shots.spec.ts` into `web/public/help/`, and a
// checkout that has not generated them shows the words alone rather than a
// column of broken images. That is the same posture the card art already takes
// when `crates/yokohama-web/assets/art/` is absent.

import { useState } from "react";

/** A screenshot that removes itself if it is not there.
 *
 *  Generated shots are build output, not source, so a fresh clone legitimately
 *  has none. An `<img>` that 404s renders as a broken-image glyph and an empty
 *  box, which would make this panel look broken rather than look plain. */
function Shot({ src, alt, caption }: { src: string; alt: string; caption: string }) {
  const [missing, setMissing] = useState(false);
  if (missing) return null;
  return (
    <figure className="shot">
      <img src={src} alt={alt} loading="lazy" onError={() => setMissing(true)} />
      <figcaption>{caption}</figcaption>
    </figure>
  );
}

export function Help() {
  return (
    <>
      <p className="lede">
        How this screen works. The rules are the rulebook&rsquo;s; everything below is about
        finding your way around.
      </p>

      <h3>The three parts of the screen</h3>
      <p>
        The <strong>board</strong> fills the left: one tile per area, laid out as the staggered
        pyramid the real board makes, with each tile carrying its assistants, any president, and
        the site or token sitting on it. The <strong>rail</strong> down the right is the
        dashboard &mdash; every seat&rsquo;s goods, money and power, the Church and Customs
        tracks, the achievements, and your own hand. The <strong>answer deck</strong> is pinned
        along the bottom and never scrolls away: whatever the game is asking you is always there,
        however far down the page you are.
      </p>
      <Shot
        src="/help/screen.png"
        alt="The whole client at two players: the staggered pyramid board on the left with its area tiles, the dashboard rail on the right showing both seats, and the pinned answer deck along the bottom carrying the current question."
        caption="The board, the rail, and the answer deck pinned at the bottom."
      />

      <h3>Taking a turn</h3>
      <p>
        The deck lists the moves the engine says are legal, numbered. Press one. Some of them open
        a further question rather than playing immediately &mdash; choosing where to place
        assistants, which route to walk, how many to recover &mdash; and the deck becomes that
        question until you confirm or cancel it. When a choice has exactly one answer the client
        offers to apply it for you rather than making you find it.
      </p>
      <p>
        While a placement picker is open the <strong>board itself becomes the input</strong>: the
        tiles you can toggle are the ones drawn as buttons, and the running cost of what you have
        selected is printed before you commit to it. At every other moment the board is a
        picture and nothing on it is pressable.
      </p>
      <Shot
        src="/help/placement.png"
        alt="A placement picker open at Option B: the deck reads &ldquo;Option B &mdash; click one area on the board for two assistants, then confirm&rdquo;, with Confirm and Cancel buttons and the eligible areas listed beneath it &mdash; Port A among them priced &ldquo;fee 2 yen (2 to Blue)&rdquo; &mdash; while the matching tiles on the board are outlined as the ones that can be chosen."
        caption="A placement picker. The board is the input surface, and every eligible area is priced before you commit to it."
      />

      <h3>Looking things up</h3>
      <p>
        The buttons along the top open read-only reference panels &mdash; the port offer and what
        each zone costs, the laboratory offer with its surcharges, what every area pays at each
        power level, the Church and Customs tracks space by space, and the building-site cards.
        Behind <em>Card reference</em> are the four full indexes: all 36 orders, all 20
        technologies, the achievements in play, and the 24 printed sites, each saying where every
        copy currently is.
      </p>
      <p>
        These never change anything. They read the game and close again, and you can open one
        while a question is waiting for you.
      </p>

      <h3>Asking the bot</h3>
      <p>
        <strong>Suggest</strong> runs the production bot on <em>your</em> seat and shows what it
        would play, the line it looked at, and the arithmetic behind the score &mdash; term by
        term, with the numbers on screen adding up to the total printed under them. It applies
        nothing.
      </p>
      <p>
        <strong>Why</strong> does the same for a move a bot already played: the game is replayed
        to that point and the bot&rsquo;s own decision is re-run, so the account is a
        reconstruction the engine can vouch for rather than a note someone stored. If the
        reconstruction does not match what was played, it refuses and says so.
      </p>
      <Shot
        src="/help/explain.png"
        alt="The suggest panel: the recommended move, the continuation it searched, and a term-by-term chart with bars either side of a centre zero line, each row labelled and the totals footing."
        caption="Suggest: the move, the line, and the arithmetic — helping to the right of the zero line, hurting to the left."
      />

      <h3>Replaying the game</h3>
      <p>
        Open <strong>Log</strong> and you get the whole move trail, each line worded as it read
        when it applied &mdash; and a track above it that walks the game. Drag it, step with the
        arrows, or press any line to jump to the position that move was decided in. The board
        behind the panel redraws at that point, full size, outlined to show you are looking at
        the past.
      </p>
      <p>
        The track has one more stop than there are moves, because the last stop is{" "}
        <em>now</em> &mdash; a real position with no move to name. <strong>Back to live</strong>{" "}
        returns you to it. Nothing you do here changes the game: it is a projection over the
        record, and the position you scrub to is not playable.
      </p>
      <Shot
        src="/help/replay.png"
        alt="The log panel open as a drawer on the right with the replay track above the move list, and the full-size board beside it redrawn at an earlier position with a green outline."
        caption="The replay track lives in the log panel; the board redraws beside it at full size."
      />

      <h3>Saving, and coming back</h3>
      <p>
        Every move you make is written to the server as it is applied &mdash; there is no save
        button and nothing to remember. Leave and come back and the game is in the list on the
        start screen, where it resumes exactly where you left it.
      </p>

      <h3>If something looks wrong</h3>
      <p>
        The engine is the only thing here that decides what is legal, so a move you expected and
        cannot see is a question about the rules rather than about this screen &mdash; the areas
        panel and the card indexes are usually where the answer is. If the driver ever finds a
        fault it locks the game rather than carrying on: the panels still open and the log still
        reads, so the replay track is the instrument for finding where it went wrong.
      </p>
    </>
  );
}
