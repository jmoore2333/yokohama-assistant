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
        The <strong>table</strong> fills the left: the area tiles laid out as the staggered
        pyramid the real board makes, each one the printed tile with any building-site card and
        pieces on it and, until a player takes it, the 5-Power token in its printed slot &mdash;
        a spent tile shows the empty slot and says &ldquo;5-Power taken&rdquo;. The Canal, at
        three and four players, has none of the three, because nothing is ever built, placed or
        awarded there. Under the pyramid sit whichever boards you have switched on: the port and the
        laboratory, six card slots to a board, the church and customs tracks, the goals in play,
        and the building sites at full size. They sit side by side only on a very wide screen
        &mdash; at 1710px across they stack, at 2560px they do not &mdash; so on a laptop the
        table scrolls down to them. The <strong>side column</strong> on the right holds the{" "}
        <strong>answer bar</strong> pinned at its top &mdash; whatever the game is asking you is
        always there &mdash; then the account of the last turn, which, when the opponent has just
        moved, first plays out on the table one step at a time, at the pace the account&rsquo;s
        cadence control sets. Each step marks what it changed, with an outline that pulses once
        and then stays until the next step replaces it: the pieces that moved, the seat rows
        whose values changed, and the cards a take moved. A take marks its two ENDS. In the port
        or laboratory it lights the card the deck turned over to refill the strip &mdash; the
        arrival &mdash; and not the card that left, which is no longer there to point at. The
        taken card is marked where it went instead: a technology in its new owner&rsquo;s list,
        an order in your own hand. A card that only shifted along a strip is not marked &mdash;
        a take slides every card behind it toward the empty slot, and outlining all of them
        would bury the one that arrived. Nothing ever marks a card in an opponent&rsquo;s hand,
        because you are never shown one; their order take is read from the arrival in the strip
        and the line that names it. When the walk ends, the marks the turn left &mdash; all of
        them, not just its last step&rsquo;s &mdash; stay on the live table until you answer
        again, so you play on a board that still shows what your opponent did. Skip lands you on
        your prompt, replay runs it again. Under it are every seat&rsquo;s goods, money and
        pieces, and your own hand. Hover or focus any tile to see it, and any site card on it,
        at full size.
      </p>
      <p>
        That is the wide screen. Narrow the window past about 1200px and the same three parts
        stack instead, the table above the column, with the answer bar pinned along the bottom
        edge rather than at the top of a column.
      </p>
      <Shot
        src="/help/screen.png"
        alt="The client at two players on a 1710 by 869 screen: the staggered pyramid of ten area tiles filling the table on the left, each tile the printed area art with its building-site card upright on its top-right, its pieces on it and an id badge along its bottom edge; on the right the answer bar at the top of the side column, carrying the phase ladder, the seat to move and the numbered moves it is offering, and under it the last move and both seats&rsquo; goods, money and pieces."
        caption="The first view at two players: the table on the left, the side column on the right, the answer bar pinned at its top. The pyramid is fitted to the table&rsquo;s height here, so the five switched-on boards sit under it and the table scrolls to them."
      />
      <Shot
        src="/help/strips.png"
        alt="The same table scrolled down: the last row of the pyramid across the top of the frame, then the marks legend, then the Port board drawn as the physical one is &mdash; six slots in a row, each holding an order card at full size with its goods and its victory points under it &mdash; and the head of the Laboratory board below that, its six slots labelled with the surcharge each carries. The answer bar and the seats are unchanged on the right."
        caption="The same screen, scrolled down. This is what is under the pyramid: the boards you switched on, in the shapes the real ones have. The port is first, the laboratory under it, and the tracks, the goals and the building sites below those."
      />

      <h3>Taking a turn</h3>
      <p>
        The answer bar lists the moves the engine says are legal, numbered. Press one. Some of
        them open a further question rather than playing immediately &mdash; choosing where to
        place assistants, which route to walk, how many to recover &mdash; and the bar becomes
        that question until you confirm or cancel it. When a choice has exactly one answer the
        client offers to apply it for you rather than making you find it.
      </p>
      <p>
        While a placement picker is open the <strong>table itself becomes the input</strong>: the
        tiles you can toggle are the ones drawn as buttons, and the running cost of what you have
        selected is printed before you commit to it. At every other moment no tile applies a
        move, and the pyramid is a picture you can read rather than press.
      </p>
      <Shot
        src="/help/placement.png"
        alt="A placement picker open at Option B: the answer bar reads &ldquo;Placement &mdash; Option B&rdquo; over the line &ldquo;Option B &mdash; click one area on the board for two assistants, then confirm&rdquo;, with Confirm and Cancel under it; below that the last-move block lists the eligible areas by name and id, naming the fee beside any that would cost one; and on the table every tile that can be chosen is drawn as a button."
        caption="A placement picker: the table is the input surface. Every eligible area is listed, with the fee named where there is one, and once you pick one the answer bar prices the selection too."
      />

      <h3>Looking things up</h3>
      <p>
        <em>Boards</em> along the top is one menu rather than a row of switches, and its summary
        counts what is on. Open it and there is a switch for each board that can sit under the
        pyramid: the port, the laboratory, the tracks, the goals, the building sites. What you
        switch on stays on in this browser, and Escape closes the menu.
      </p>
      <p>
        The buttons beside it open read-only panels over the game &mdash; what every area pays at
        each power level, the log, the bot&rsquo;s advice, and this page &mdash; and behind{" "}
        <em>Cards</em> are the four full indexes: all 36 orders, all 20 technologies, the
        achievements in play, and the 24 printed sites, each saying where every copy currently
        is, together with the final score once there is one. A panel reads the game and closes
        again; the game waits underneath it, and nothing outside the panel answers until you
        close it.
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
        alt="The Suggest panel open as a drawer over the table: the move it would play under the heading &ldquo;It would play&rdquo;, then a term-by-term chart with a bar either side of a centre zero line for every feature carrying a nonzero product, each row naming its feature, its arithmetic and its product, and a total under them that foots, over a button offering the remaining features that contribute nothing."
        caption="Suggest: the move it would play, and the arithmetic behind it — helping to the right of the zero line, hurting to the left. The line it searched follows further down the panel."
      />

      <h3>Replaying the game</h3>
      <p>
        Open <strong>Log</strong> and you get the whole move trail, each line worded as it read
        when it applied &mdash; and a track above it that walks the game. Drag it, step with the
        arrows, or press any line to jump to the position that move was decided in. On a wide
        screen the panel is a drawer down the right and the table beside it redraws at that
        point, full size, outlined to show you are looking at the past; on a narrow one the panel
        covers the page instead, and there is no outline to see. The port and the laboratory
        draw from that same past position; the building sites say they are hidden while you are
        in the past, since their board is read from the live game. The tracks and the goals keep
        drawing, from that same past position.
      </p>
      <p>
        The track has one more stop than there are moves, because the last stop is{" "}
        <em>now</em> &mdash; a real position with no move to name. <strong>Back to live</strong>{" "}
        returns you to it. Nothing you do here changes the game: it is a projection over the
        record, and the position you scrub to is not playable.
      </p>
      <Shot
        src="/help/replay.png"
        alt="The Log panel open as a drawer on the right: the replay track across its top with step arrows, a readout of which stop it is at and a &ldquo;Back to live&rdquo; button, the numbered move list under it with the current stop highlighted, and the table beside it redrawn at that earlier position inside a green outline."
        caption="The replay track lives in the Log panel; the table redraws beside it at full size, outlined to say you are in the past."
      />

      <h3>Starting a game</h3>
      <p>
        Sign in first: the server has to know who you are before it will show you a table, and
        under this deployment that is an email address you type once and it remembers. What you
        land on is the <strong>lobby</strong>. It has two lists &mdash; <em>Your games</em>, the
        tables you already sit at, and <em>Open tables</em>, every table on the server still
        waiting for players, whoever opened it &mdash; and <em>New table</em>, which is the form.
      </p>
      <p>
        The form is <strong>one row per seat</strong> &mdash; each one you, somebody else&rsquo;s
        email, an open seat, or the bot. The bot is offered at two players only, because that is
        where it has been measured. You can give the table a name, give yourself a display name,
        and ask for a <strong>password</strong> that anyone joining has to type. Fill every seat
        and the button reads Deal: the table opens at once. Leave a seat open and it reads Open
        table, and the table waits. Either way nobody has played yet, and the game does not exist
        until every seat has chosen its first card.
      </p>
      <p>
        A table with a seat still open has a <strong>link</strong> &mdash; its own address, shown
        on the table&rsquo;s screen with a copy button beside it. Send it to whoever is playing:
        they sign in, type the password if the table asks for one, and press <em>sit here</em>.
        They can equally find the table for themselves under Open tables, without a link at all.
        The deal begins by itself the moment the last seat is taken, on everybody&rsquo;s screen at
        once.
      </p>
      <p>
        That choice happens over the <strong>set table</strong>: the areas with their site cards
        and tokens, the port and the tracks &mdash; what the rulebook has on the table at the step
        where the two cards are dealt and chosen in secret. The technologies and the goals go out
        at the two steps after that one, so they are held back here as well and appear once you
        have chosen. <strong>You are shown your own two cards and nobody else&rsquo;s</strong>, each
        with the built-in bot&rsquo;s <em>Keep score</em> under it: that number is its preference
        from the card&rsquo;s printed economics alone &mdash; hand-set and unmeasured, so it is a
        number rather than a recommendation. The other seats are listed beside the table as{" "}
        <em>chosen</em> or <em>waiting</em>, and that is all anyone can learn about them &mdash;
        whether a seat has answered is public at any table, and which card it took is not. A bot
        seat shows two cards <strong>face down</strong>. It keeps one of them by the same reading
        of the printed economics the Keep score shows you; the game&rsquo;s record knows which, and
        you do not see it, any more than you would across a table.
      </p>
      <p>
        Press the card you want. If you are holding more than one seat &mdash; several seats set to
        you, everyone round one screen &mdash; the next of them is dealt to you straight away, so
        the seats are answered in turn and the screen never has two people&rsquo;s cards on it.
        Otherwise it says <em>waiting on</em> whoever has not chosen yet, and the game opens by
        itself the moment the last of them does.
      </p>

      <h3>Watching a game</h3>
      <p>
        You can watch a game you are not playing in. The lobby&rsquo;s third list,{" "}
        <em>Games in progress</em>, is every game on the server still being played or
        already finished, and pressing one opens it &mdash; as does a{" "}
        <strong>link</strong> to the table, from whoever is at it. If the table was
        opened with a password, you are asked for it once, on a screen that names the
        table and who is sitting at it; after that the game opens and stays open to you.
      </p>
      <p>
        The screen is the same table the players are looking at, with one difference:
        the strip in the banner reads <strong>You are watching</strong>, and there is no
        answer bar. Every hand on it is a <em>count</em> rather than cards &mdash; a
        watcher is never shown anybody&rsquo;s hand, exactly as the players are never
        shown each other&rsquo;s &mdash; and moves arrive and walk across the table{" "}
        <em>as they happen</em>, one step at a time, at whatever cadence you set. You
        cannot play, you cannot undo, and Suggest is not offered, because it runs on a
        seat and you have none. Everything you can read still reads: the log and its
        replay track, the boards, the card indexes, <em>Why</em> for the last move a bot
        played, and the final score once there is one. You can talk, too: the chat panel is at
        the foot of the column, and what you say is marked as coming from a watcher.
      </p>

      <h3>Talking</h3>
      <p>
        At the foot of the side column &mdash; under the account of the last turn and the players
        &mdash; is a <strong>Chat</strong> panel, folded until you open it, with a count of what has
        arrived while it was closed. Everyone at the table can talk: each seat&rsquo;s line carries
        the seat&rsquo;s colour and the name it joined under, or its address, and a watcher&rsquo;s
        line says <em>watching</em>. A message is never undone and never part of the game&rsquo;s
        record; the server keeps a few hundred per table and drops the oldest, and the table&rsquo;s
        chat goes with the table when it is abandoned.
      </p>
      <p>
        The lobby has a chat of its own, <strong>Lobby chat</strong>, under the three lists, folded
        the same way &mdash; open it to join in. Whoever runs the server can switch it off, and then
        the section is simply not there. The name you type beside <em>You are</em> on the lobby is
        the name your messages carry, and the name a seat you take is labelled with; leave it blank
        and your address is used.
      </p>

      <h3>Saving, and coming back</h3>
      <p>
        Every move you make is written to the server as it is applied &mdash; there is no save
        button and nothing to remember. Leave and come back and the table is under{" "}
        <em>Your games</em> in the lobby, where it resumes exactly where you left it. You do not
        have to leave to find it: the banner&rsquo;s leftmost control names the game you are in,
        and opening it offers <em>Lobby</em> at the top and your other tables under it to switch
        between. A table you opened yourself can be abandoned, from its own screen while it is
        still filling or from its row in the lobby &mdash; two clicks, and it is gone for everyone
        who was at it.
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
