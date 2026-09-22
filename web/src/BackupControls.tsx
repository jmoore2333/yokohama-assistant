// The backup and restore controls (M6.5b2, R-6.5b2.10 – .12; spec §2.5, §4).
//
// **`BackupControls.tsx` and not `Backup.tsx`, deliberately.** The rules live
// in `backup.ts` beside it, and two files whose names differ only in case
// cannot coexist on a case-insensitive filesystem: on macOS `import … from
// "./Backup"` resolves to `backup.ts` — Vite tries `Backup.ts` before
// `Backup.tsx` and the filesystem answers with the lower-case file — while on
// Linux it resolves to the component. A build that differs by platform is
// worse than a longer name.
//
// **Two exports, each saying in a few words what it is for**, because they are
// different files and a player has to be able to tell them apart:
//
//   - "Save game file" (`App.tsx`) is the raw `<id>.json` record — the file the
//     terminal loads. It is unchanged by this cycle.
//   - "Back up" here is an ENVELOPE — a file this app can restore, carrying the
//     record AND the table document, one game or every game on the device. It
//     is named `<id>.backup.json`, and `Abandon`'s "Back it up to a file" — the
//     other control that writes one — names it the same way since the
//     whole-branch review found it using the record's own name.
//
// **All the words a player reads are here or in `backup.ts`'s report**, and the
// verdict sentences are the VALIDATOR's own, carried rather than reworded: the
// reason a game will not open is a fact about the game, and this file is not
// the authority on it.

import { useCallback, useEffect, useRef, useState } from "react";
import {
  backupEverything,
  backupOne,
  download,
  forgetUnplayable,
  listUnplayable,
  restorePairs,
  type Outcome,
  type RestoreReport,
  type UnplayableGame,
} from "./backup";

/** What one outcome reads as. The clash NAMES its row — the id is beside it in
 *  the list — and says what is in the way rather than only that something is. */
function outcomeWords(outcome: Outcome, reason: string | undefined): string {
  switch (outcome) {
    case "restored":
      return "restored";
    case "already-here":
      return "already on this device — nothing changed";
    case "clash":
      return `not restored — ${reason ?? "a different game is already on this device under that name"}`;
    case "unplayable":
      return `restored, but it will not open here${reason ? ` — ${reason}` : ""}`;
    case "rejected":
      return `not restored — ${reason ?? "this build will not read it"}`;
  }
}

/** A stamp for a file name, so two backups from one day do not overwrite each
 *  other in a downloads folder. Local time, because a player reads it. */
function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}`;
}

/** A refusal this control met, shown where it happened rather than thrown. */
const sentenceOf = (e: unknown): string => (e instanceof Error ? e.message : String(e));

/**
 * Every game in this browser, as one file (R-6.5b2.10).
 *
 * **Offered in every state of the door**, including the one where the module
 * will not load: that is the state it is most for, and `backupEverything` reads
 * IndexedDB from this page rather than through the Worker for exactly that
 * reason.
 */
export function BackUpEverything(): React.ReactNode {
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);
  const save = async () => {
    // One file per click. `useRef` and not state, for `sendToArchive`'s reason:
    // React batches, so two clicks in a tick both read a state flag as false.
    if (busy.current) return;
    busy.current = true;
    setError(null);
    try {
      download(`yokohama-backup-${stamp()}.json`, await backupEverything());
    } catch (e) {
      setError(sentenceOf(e));
    } finally {
      busy.current = false;
    }
  };
  return (
    <>
      <button type="button" className="ghost back-up-all" onClick={() => void save()}>
        Back up everything
      </button>
      {/* **The two files, told apart in one line** (R-6.5b2.10). They are not
          the same file and a player has no way to know that from the button
          names alone: one is a record the terminal reads, the other is an
          envelope this app restores. */}
      <p className="muted backup-note">
        one file with every game on this device, which this app can restore.
        &ldquo;Save game file&rdquo; on a game is the record alone &mdash; the file the terminal
        loads.
      </p>
      {error !== null && <p className="error backup-error">{error}</p>}
    </>
  );
}

/** ONE game, as a file this app can restore — beside "Save game file", which is
 *  the record alone and is what the terminal loads. */
export function BackUpOne({ id, label = "Back up" }: { id: string; label?: string }): React.ReactNode {
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);
  const save = async () => {
    if (busy.current) return;
    busy.current = true;
    setError(null);
    try {
      download(`${id}.backup.json`, await backupOne(id));
    } catch (e) {
      setError(sentenceOf(e));
    } finally {
      busy.current = false;
    }
  };
  return (
    <>
      <button
        type="button"
        className="ghost back-up-one"
        title="a backup this app can restore"
        onClick={() => void save()}
      >
        {label}
      </button>
      {error !== null && <span className="error backup-error">{error}</span>}
    </>
  );
}

/**
 * Put a backup file's games back on this device.
 *
 * A file input and nothing clever: the browser's own picker is the one control
 * every platform agrees about, and an installed app on iOS reaches its Files
 * app through it.
 */
export function Restore({ onDone }: { onDone?: () => void }): React.ReactNode {
  const [report, setReport] = useState<RestoreReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement | null>(null);
  /** One restore at a time. A ref and not state, for `BackUpEverything`'s
   *  reason: React batches, so two changes in a tick both read a state flag as
   *  false. Two overlapping restores of one file are not destructive — the
   *  transaction serialises them and the loser reads `already-here` — but the
   *  REPORT on screen would be whichever finished last, which is a screen that
   *  says less than it knows. */
  const busy = useRef(false);

  const take = async (file: File) => {
    if (busy.current) return;
    busy.current = true;
    setError(null);
    setReport(null);
    try {
      const out = await restorePairs(await file.text());
      setReport(out);
      onDone?.();
    } catch (e) {
      // A refusal about the FILE — a version this build does not read, two
      // games under one name, something that is not a backup at all. Nothing
      // was written.
      setError(sentenceOf(e));
    } finally {
      // So the same file can be chosen twice: a `change` does not fire for an
      // unchanged value.
      if (input.current) input.current.value = "";
      busy.current = false;
    }
  };

  return (
    <div className="restore">
      <label className="field restore-field">
        <span>Restore from a backup</span>
        <input
          ref={input}
          type="file"
          accept="application/json,.json"
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void take(file);
          }}
        />
      </label>
      {error !== null && <p className="error restore-error">{error}</p>}
      {report !== null && (
        <ul className="restore-report">
          {report.games.length === 0 ? (
            <li className="muted">this file carries no games.</li>
          ) : (
            report.games.map((g) => (
              <li key={g.id} className={`restore-${g.outcome}`}>
                <span className="restore-id">{g.id}</span>{" "}
                <span className="meta">{outcomeWords(g.outcome, g.reason)}</span>
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  );
}

/**
 * "Remove", in the two-click shape `Abandon` uses and for `Abandon`'s reason
 * (final review I-1).
 *
 * **The row it sits on is the one row where a stray click costs the only copy
 * of a game.** R-6.5b2.12's whole claim is that a save this build cannot open
 * is kept, listed and downloadable, never erased — and this control stood
 * unfenced beside "Back up" on the same line, which is exactly the shape M5's
 * final review found removing the game next to the one that was meant (see
 * `Abandon.tsx`'s header). There is no undo: the pair is deleted from this
 * device and nothing else has a copy.
 *
 * **Both buttons NAME the game**, so a confirmation can never be about a row
 * other than the one that opened it — the same rule, in the same words, as the
 * three buttons `Abandon` draws.
 */
function Forget({ id, onForget }: { id: string; onForget: (id: string) => Promise<void> }) {
  const [confirming, setConfirming] = useState(false);
  /** One DELETE per confirmation. A ref and not state, for the reason every
   *  other guard in this client is one: React batches, so two clicks in the
   *  same tick both read a state flag as false. */
  const inFlight = useRef(false);
  const go = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      await onForget(id);
      setConfirming(false);
    } finally {
      inFlight.current = false;
    }
  };
  if (!confirming)
    return (
      <button
        type="button"
        className="ghost danger"
        aria-label={`remove ${id}`}
        onClick={() => setConfirming(true)}
      >
        Remove
      </button>
    );
  return (
    <>
      <button
        type="button"
        className="ghost danger"
        aria-label={`confirm removing ${id}`}
        onClick={() => void go()}
      >
        confirm remove
      </button>
      <button
        type="button"
        className="ghost"
        aria-label={`cancel removing ${id}`}
        onClick={() => setConfirming(false)}
      >
        cancel
      </button>
    </>
  );
}

/**
 * The games on this device this build cannot open (R-6.5b2.12).
 *
 * **Its own listing, because the module's cannot carry them**: the layer skips
 * any table document whose shape check fails, and a restored cross-rev pair can
 * be foreign on both sides. The compatibility row beside the pair is what puts
 * the game here, with the reason it will not open — and the pair's own bytes
 * are never rewritten, so it can be backed up again and taken somewhere that
 * does read it.
 *
 * `reload` is a number the caller bumps after a restore, so the list re-reads
 * without this component owning a subscription of its own.
 */
export function Unplayable({ reload = 0 }: { reload?: number }): React.ReactNode {
  const [rows, setRows] = useState<UnplayableGame[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const read = useCallback(() => {
    listUnplayable().then(setRows, (e: unknown) => setError(sentenceOf(e)));
  }, []);
  useEffect(() => read(), [read, reload]);

  const forget = async (id: string) => {
    try {
      await forgetUnplayable(id);
      read();
    } catch (e) {
      setError(sentenceOf(e));
    }
  };

  // **Wrapped, not the bare exception** (review M-10). A browser that cannot
  // open IndexedDB already draws `OnThisDevice`'s own fault line; a second,
  // unlabelled `indexedDB is not defined` beside it says nothing about which
  // read failed or what it was for.
  if (error !== null)
    return <p className="error">the games that will not open could not be read &mdash; {error}</p>;
  if (rows === null || rows.length === 0) return null;
  return (
    <div className="unplayable">
      <h4>Will not open here</h4>
      <ul>
        {rows.map((g) => (
          <li key={g.id}>
            <span className="restore-id">{g.id}</span>{" "}
            <span className="meta">
              {g.note.reason ||
                (g.note.semantics_rev === null
                  ? "this build does not read it"
                  : `saved at semantics rev ${g.note.semantics_rev}`)}
            </span>{" "}
            <BackUpOne id={g.id} />
            <Forget id={g.id} onForget={forget} />
          </li>
        ))}
      </ul>
    </div>
  );
}
