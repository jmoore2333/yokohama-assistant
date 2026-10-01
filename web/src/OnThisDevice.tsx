// The games in THIS BROWSER, and the button that starts one (R-6.5b.8).
//
// **The first section of the first screen, and it renders before the server
// has said anything.** Its rows come from the local dispatch's own
// `GET /api/games?scope=mine` — the module in this tab's Worker, over this
// browser's IndexedDB — so the guarantee R-6.5b.2 makes is architectural
// rather than a fallback: there is no reachable API in the path that draws
// this, and the cells prove it with every `fetch` failing.
//
// **The wording is the lobby's own**, drawn by `lobbyRow`, because a game is a
// game: "your turn", "over · you won", the name or the id, the link to
// `#/t/<id>`. What it does NOT draw is an abandon control (P-6) — a local game
// is abandoned on its own screen, where this tab holds its lock.

import { useCallback, useEffect, useState } from "react";
import { api, ApiError, useIdentityMode } from "./api";
import { unplayableIds } from "./backup";
import { BackUpEverything, BackUpOne, Restore, Unplayable } from "./BackupControls";
import { lobbyRow } from "./Lobby";
import { onLocalFault, onLocalWrite } from "./local/client";
import { dismissInstall, promptInstall, useInstallState } from "./install";
import { reasonWords, sendNow, useOutbox, type Pending } from "./outbox";
import { navigate } from "./route";
import { humanBytes, onPersistenceAsked, storageReport, type StorageReport } from "./storage";
import type { TableView } from "./types";

export function OnThisDevice() {
  const [games, setGames] = useState<TableView[] | null>(null);
  /** Why there are no rows, when the reason is the HOST and not the store.
   *
   *  **A visible line and never an empty list** (spec §2.3). A Worker that
   *  fails to start, a module that will not load and an IndexedDB that will
   *  not open all end here, and "no game on this device yet" would be a false
   *  sentence in every one of those cases — the games are there and this
   *  browser cannot reach them, which is a different thing and is actionable
   *  (a private window, a blocked worker, a bad build). */
  const [fault, setFault] = useState<string | null>(null);

  /** The ids this device cannot open, so a game is listed ONCE (review M-2).
   *
   *  **A separate read from the listing's**, because it comes from a different
   *  place: the layer builds a row from the TABLE document, which a restored
   *  cross-rev pair may have in a shape this build reads perfectly well, while
   *  the thing that says the game will not open is this browser's own
   *  compatibility row. A failure to read it is not a failure to list — the
   *  games are still there — so it falls back to hiding nothing. */
  const [hidden, setHidden] = useState<Set<string>>(() => new Set());

  const read = useCallback(
    () =>
      Promise.all([
        api.list("mine", undefined, "local"),
        unplayableIds().catch(() => new Set<string>()),
      ])
        .then(([l, cannot]) => {
          setGames(l.games);
          setHidden(cannot);
          setFault(null);
        })
        .catch((e: unknown) => setFault(e instanceof ApiError ? e.message : String(e))),
    [],
  );

  useEffect(() => {
    void read();
    // **Every local write re-reads this list.** A create, a move and a delete
    // all change what a row says — whose turn it is, whether it is over — and
    // nothing else tells this section that they happened: a local game has no
    // signal, because the bot's trailing run comes back inside the reply that
    // caused it.
    return onLocalWrite(() => void read());
  }, [read]);

  // The fault arrives on its own channel as well as through a rejected read:
  // the Worker can die while nothing of this section's is in flight, and a
  // list drawn a minute ago would otherwise go on looking like the truth.
  useEffect(() => onLocalFault(setFault), []);

  /** Bumped when a restore has landed, so the unplayable list re-reads
   *  (R-6.5b2.12). The PLAYABLE list re-reads on its own: `localAdopt` fires
   *  the write handlers this section already listens to. */
  const [restored, setRestored] = useState(0);

  /** The rows a player can actually open. A game with a compatibility row is
   *  drawn by `Unplayable` below instead, with the reason it will not. */
  const playable = (games ?? []).filter((t) => !hidden.has(t.id));

  return (
    <section className="saved on-this-device">
      <h3>On this device</h3>
      <button
        type="button"
        className="primary"
        onClick={() => navigate({ kind: "new", local: true })}
      >
        Play the bot here
      </button>
      {fault !== null ? (
        <p className="muted local-fault">the games on this device could not be read &mdash; {fault}</p>
      ) : games === null ? (
        <p className="muted">loading&hellip;</p>
      ) : games.length === 0 ? (
        <p className="muted">no game on this device yet &mdash; play the bot here.</p>
      ) : playable.length === 0 ? (
        /* **Games, but none this build can open.** "No game on this device yet"
           would be false — they are here and they are listed, under "Will not
           open here" below with the reason. So this draws nothing and lets that
           section be the answer, rather than saying there is nothing. */
        null
      ) : (
        <ul>{playable.map((t) => lobbyRow(t, <BackUpOne id={t.id} />))}</ul>
      )}
      <Outbox />
      {/* **What the browser has promised, and how to get the games off it**
          (R-6.5b2.9, spec §2.4). Both are statements ABOUT the rows above, so
          they stand under them: a promise about storage read on another screen
          would be a promise about nothing a player can see. */}
      <StorageLine hasGames={(games?.length ?? 0) > 0} />
      <InstallOffer />
      {/* **Offered in EVERY state above, including the fault one** (spec §2.5):
          a module that will not load is when you most need your games out, and
          `backupEverything` reads IndexedDB from this page rather than through
          the Worker for exactly that reason.

          **And it is the control the install offer points at** (spec §2.4): an
          installed iOS app has its own storage container, so the envelope is
          the only bridge. `InstallOffer` sits immediately above for that
          reason, and a cell asserts the adjacency rather than the presence. */}
      <div className="backup-controls">
        <BackUpEverything />
        <Restore onDone={() => setRestored((n) => n + 1)} />
      </div>
      <Unplayable reload={restored} />
    </section>
  );
}

/**
 * Whether this browser has promised to keep these games (R-6.5b2.9, spec §2.4).
 *
 * **Three states, and the third is not a synonym for the second.** A browser
 * that says it has not promised has answered; a browser with no storage
 * manager, or one in a private window that throws on the question, has not —
 * and telling a player their games are unprotected on the strength of a
 * question that was never answered is a different claim from the one the
 * browser made.
 *
 * **No wording here claims safety** (spec §2.4). A grant means the browser
 * will not clear this origin to reclaim space. It is not a copy: it does not
 * survive a cleared browser, a lost device, or an iOS home-screen install,
 * which gets a storage container of its own. So every state points at the
 * backup control below rather than reassuring.
 *
 * **It asks two questions and never the third.** `persisted()` and
 * `estimate()` are reads; `persist()` is the ASK, and the ask belongs at the
 * first committed move (`local/client.ts`), never on a door a visitor who will
 * not play is looking at.
 */
function StorageLine({ hasGames }: { hasGames: boolean }): React.ReactNode {
  const [report, setReport] = useState<StorageReport | null | "reading">("reading");
  useEffect(() => {
    let live = true;
    const read = () =>
      void storageReport().then(
        (r) => live && setReport(r),
        // `storageReport` swallows its own failures, so this is a rejection no
        // browser should produce. It still must not leave the line saying
        // "reading" for ever.
        () => live && setReport(null),
      );
    read();
    // **And again once the browser has been ASKED** (review M-5). The ask is
    // at the first committed move, on the game screen; without this the line
    // here reported the answer as it stood when this section last mounted.
    const stop = onPersistenceAsked(read);
    return () => {
      live = false;
      stop();
    };
  }, []);

  if (report === "reading") return null;
  // **The subject is what there is** (review M-6): with no rows, "these games"
  // names nothing on screen and "back them up" instructs about nothing. The
  // line is still worth drawing — it says what this browser will do with the
  // game a visitor is about to start.
  const them = hasGames ? "these games" : "games played here";
  const sizes =
    report !== null && report.quota > 0
      ? ` ${humanBytes(report.usage)} of ${humanBytes(report.quota)} is in use.`
      : "";
  return (
    <p className="muted storage-line">
      {report === null
        ? `This browser does not say whether it will keep ${them}.`
        : report.persisted
          ? `This browser has promised to keep ${them}.${sizes}`
          : `This browser has not promised to keep ${them} and may clear them to make room.${sizes}`}
      {hasGames ? " Back them up to hold a copy you can restore." : ""}
    </p>
  );
}

/**
 * The way onto a home screen, or the one sentence that stands in for it
 * (R-6.5b2.9, spec §2.4).
 *
 * **Never a modal, and never repeated.** It is a block in the flow with the
 * games still on screen beside it, and the iOS instruction carries its own
 * dismiss — remembered per browser, so a player who waves it away never sees
 * it again on that browser.
 *
 * **The WebKit sentence is the load-bearing one.** An installed iOS app gets
 * its own storage container and shares nothing with Safari, so the games
 * played in the browser do not appear in it. Offering the install without
 * saying so would send a player to a home screen that looks like their app and
 * is empty. The backup control is the next element on the page for that
 * reason, and the envelope is the only bridge across that boundary.
 */
function InstallOffer(): React.ReactNode {
  // **The hook is what redraws after the dismissal.** `dismissInstall` writes
  // to `localStorage`, which nothing in React watches — so the module tells
  // its listeners, and this component is one of them through the hook. A piece
  // of local state here would be a second mechanism doing the same job.
  const state = useInstallState();

  if (state === "available") {
    return (
      <div className="install-offer">
        <button type="button" className="ghost install" onClick={() => void promptInstall()}>
          Install
        </button>
        <p className="muted">
          keep this app on your device, with the games it holds. They stay in this browser&rsquo;s
          storage.
        </p>
      </div>
    );
  }
  if (state !== "ios-instructions") return null;
  return (
    <div className="install-offer ios">
      {/* Each sentence is ONE text node: a `<strong>` inside would split it,
          and the cells match on the sentence a player reads. */}
      <p className="muted">
        To keep this app on your home screen: tap Share, then Add to Home Screen.
      </p>
      <p className="muted install-container-note">
        An app added to the home screen keeps its own storage, so it will not see the games played
        here in the browser. Back them up first and restore them there.
      </p>
      <button type="button" className="linkish install-dismiss" onClick={() => dismissInstall()}>
        Dismiss
      </button>
    </div>
  );
}

/**
 * The games that have finished here and have not reached the archive
 * (M6.5b2, R-6.5b2.13; spec §2.6).
 *
 * **On the door, under the games it is about.** A game waiting to be sent is a
 * fact about a row above, and this cycle's whole reason for the queue is that
 * the send is attempted when nobody is watching — so the one place it can be
 * seen is the screen the player comes back to.
 *
 * **It draws NOTHING when nothing is waiting**, which is the ordinary state: a
 * heading over an empty list would be a permanent reminder of a mechanism that
 * is working.
 */
function Outbox(): React.ReactNode {
  const rows = useOutbox();
  if (rows.length === 0) return null;
  return (
    <div className="outbox">
      <h4>Waiting to reach the archive</h4>
      <ul>
        {rows.map((r) => (
          <OutboxRow key={r.id} row={r} />
        ))}
      </ul>
    </div>
  );
}

function OutboxRow({ row }: { row: Pending }): React.ReactNode {
  const [sending, setSending] = useState(false);
  // **A `refused` row has nothing to retry** (spec §2.6): a 400 or a
  // non-`identical` 409 is about these bytes and will answer the same way for
  // ever, so the row says what the server said and offers no button that would
  // only ask again.
  const retryable = row.reason !== "refused";
  return (
    <li className={`outbox-row ${row.reason}`}>
      <span className="name">{row.id}</span>
      <span className="muted why">{reasonWords(row.reason, row.sentence)}</span>
      {row.reason === "signed-out" ? <Remedy /> : null}
      {retryable ? (
        <button
          type="button"
          className="ghost"
          aria-label={`send ${row.id} to the archive now`}
          disabled={sending}
          onClick={() => {
            setSending(true);
            void sendNow(row.id).finally(() => setSending(false));
          }}
        >
          Send now
        </button>
      ) : null}
    </li>
  );
}

/**
 * What to DO about "not signed in" — and under `open` it is two steps, not
 * one (spec §2.6).
 *
 * **Read off the door's own name.** Passing `/enter` gets the Access cookie;
 * `POST /api/archive` then still refuses in this API's own JSON shape until the
 * in-app sign-in has supplied `X-Yokohama-Claimed-User`. Under `cf-access` and
 * `worker` the turnstile is the whole of it, and a second step named there
 * would send a player looking for a form that does not exist.
 *
 * **A door this browser has never been told about gets the turnstile alone**,
 * which is the first step under every mode: it is the instruction that is true
 * whatever the answer turns out to be.
 */
function Remedy(): React.ReactNode {
  const open = useIdentityMode() === "open";
  return (
    <span className="muted remedy">
      <a href="/enter">Sign in at the door</a>
      {open ? ", then sign in here with your email." : "."}
    </span>
  );
}
