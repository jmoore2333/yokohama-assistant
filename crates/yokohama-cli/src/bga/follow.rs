//! Live-follow: the H3 spectator (plan §6, design §2.3).
//!
//! `yokohama-cli import <capture.ndjson> --follow` TAILS the capture file as
//! the relay appends to it, and renders the game as a READ-ONLY spectate frame
//! through the CLI's existing composed-frame machinery (`Repl` + `screen::wrap`
//! — no new render stack; the follower never reads a line of input, so the
//! frame's prompt zone carries a status line instead of a prompt).
//!
//! **The core design decision: every change RECOMPUTES from scratch.** When
//! the ingested prefix grows, the follower re-runs the whole batch pipeline —
//! collapse → synthesize → resolve → hypothesis-looped replay through the
//! checked `Session::apply` boundary — rather than patching a live session
//! incrementally. That is not laziness, it is correctness three ways over:
//! (1) socket SUPERSESSION can retract packets the follower already applied
//! (a live BGA takeback), and a rebuild rewinds the spectate state honestly
//! where an incremental applier would need rollback machinery; (2) later
//! input can RE-RESOLVE earlier ambiguity (thin-take pruning reads future
//! display observations; an inert achievement assumption becomes a replay
//! hypothesis the moment it is claimed), which only a re-run can honor; and
//! (3) RESTART SAFETY falls out for free — a killed and re-run follower
//! performs the same computation over the same file, so it reaches the
//! byte-identical state (pinned by tests here and over the real corpus). The
//! measured cost bounds it: the full 194-move corpus rebuild is ~0.2s in a
//! debug build, against a poll cadence of [`FOLLOW_POLL_MS`].
//!
//! **Ingestion is complete-lines-only.** The relay appends whole NDJSON lines,
//! but a poll can catch a line mid-write, so the follower consumes the file
//! prefix up to the LAST newline and holds the partial tail for the next poll.
//! Dedupe (feed collapse, N-4) makes re-ingestion idempotent: appending the
//! same content twice changes nothing, which is the property restart safety
//! rides on.
//!
//! **A point the pipeline cannot pass is a HOLD, never an exit.** A pipeline
//! error (no snapshot yet, evidence not yet arrived, a feed shape refusal) or
//! a replay STOP (the documented D3 engine-rules divergence at the corpus's
//! m154) is printed ONCE — deduplicated on its own text / on the rendered
//! state fingerprint — and the follower keeps tailing; if later input makes
//! progress possible, the next recompute advances past it on its own. The
//! only spontaneous exit is game completion, where the final-score
//! cross-check (the honesty dividend) prints and the loop ends.

use super::import::{self, Replays};
use super::ImportError;
use std::path::PathBuf;
use yokohama_bridge::feed::read_and_collapse;
use yokohama_core::Rules;

/// The poll cadence `main.rs` sleeps between polls — the "never busy-spin"
/// half of the loop (the other half is [`Follower::poll`]'s metadata
/// short-circuit: an unchanged file length does no read and no recompute).
pub const FOLLOW_POLL_MS: u64 = 1000;

pub struct FollowOptions {
    pub path: PathBuf,
    pub table: Option<u64>,
    /// Re-saved on every rendered advance, so a crash or Ctrl-C loses nothing
    /// past the last state change.
    pub out: Option<PathBuf>,
}

/// What one poll produced. `lines` are pipeline HOLDs with no renderable state
/// behind them (each printed once); `frame` is the composed spectate frame
/// (plain text when frames are off — a pipe, or tests), present only when the
/// rendered state CHANGED.
#[derive(Default)]
pub struct Poll {
    pub lines: Vec<String>,
    pub frame: Option<String>,
    pub complete: bool,
}

/// Sentinel for "no file length seen yet" — forces the first successful
/// metadata read to proceed to a real read.
const LEN_UNSEEN: u64 = u64::MAX;

pub struct Follower {
    opts: FollowOptions,
    rules: Rules,
    /// The ingested complete-line prefix of the capture file, verbatim.
    text: String,
    /// File length at the last poll (metadata short-circuit).
    last_file_len: u64,
    /// Last HOLD message printed, for print-once dedupe.
    last_hold: Option<String>,
    /// Fingerprint of the last RENDERED state, for render-once dedupe: new
    /// input that changes nothing replay-observable (post-STOP chatter,
    /// duplicate deliveries) re-renders nothing and re-prints nothing.
    fingerprint: Option<String>,
    /// The latest rendered state, held as the `Repl` the frame was composed
    /// from — the read-only reuse of the CLI's own render machinery.
    latest: Option<crate::repl::Repl>,
}

impl Follower {
    pub fn new(opts: FollowOptions, rules: Rules) -> Follower {
        Follower {
            opts,
            rules,
            text: String::new(),
            last_file_len: LEN_UNSEEN,
            last_hold: None,
            fingerprint: None,
            latest: None,
        }
    }

    /// The session behind the latest rendered frame, if any state has been
    /// reached — the test surface for the state-identity pins.
    pub fn session(&self) -> Option<&crate::session::Session> {
        self.latest.as_ref().map(|r| &r.session)
    }

    fn hold(&mut self, msg: String, out: &mut Poll) {
        if self.last_hold.as_deref() != Some(msg.as_str()) {
            out.lines.push(format!("HOLD: {msg} — still tailing"));
            self.last_hold = Some(msg);
        }
    }

    /// One poll: ingest any new complete lines, recompute if anything was
    /// ingested, render if the reached state changed. Never returns an error
    /// and never panics on file trouble — everything short of progress is a
    /// HOLD, because the capture is being written by another process and the
    /// follower's job is to outlast its hiccups.
    pub fn poll(&mut self) -> Poll {
        let mut out = Poll::default();

        // Metadata short-circuit: an unchanged length is no read, no work.
        let len = match std::fs::metadata(&self.opts.path) {
            Ok(m) => m.len(),
            Err(e) => {
                self.hold(
                    format!("waiting for capture file {}: {e}", self.opts.path.display()),
                    &mut out,
                );
                return out;
            }
        };
        if len == self.last_file_len {
            return out;
        }
        self.last_file_len = len;

        let bytes = match std::fs::read(&self.opts.path) {
            Ok(b) => b,
            Err(e) => {
                self.hold(format!("cannot read {}: {e}", self.opts.path.display()), &mut out);
                return out;
            }
        };

        // An append-only file keeps its ingested prefix byte-identical. If it
        // shrank or was rewritten, re-ingest from the start — dedupe makes
        // that idempotent, so the honest response to a rewritten file is the
        // same computation a fresh follower would run.
        if bytes.len() < self.text.len() || bytes[..self.text.len()] != *self.text.as_bytes() {
            out.lines.push(format!(
                "capture file {} changed under the follower — re-ingesting from the start",
                self.opts.path.display()
            ));
            self.text.clear();
            self.fingerprint = None;
        }

        // Complete lines only: consume up to the last newline, hold the tail.
        let prefix_len = match bytes.iter().rposition(|b| *b == b'\n') {
            Some(i) => i + 1,
            None => return out,
        };
        if prefix_len <= self.text.len() {
            return out;
        }
        match std::str::from_utf8(&bytes[self.text.len()..prefix_len]) {
            Ok(new) => self.text.push_str(new),
            Err(e) => {
                self.hold(
                    format!("capture file {} is not valid UTF-8: {e}", self.opts.path.display()),
                    &mut out,
                );
                return out;
            }
        }

        self.recompute(&mut out);
        out
    }

    fn recompute(&mut self, out: &mut Poll) {
        let (feed, reps) = match self.pipeline() {
            Ok(v) => v,
            Err(e) => {
                self.hold(e.to_string(), out);
                return;
            }
        };
        let Replays { resolved, mut attempts, survivors, best } = reps;
        let survivor_labels: Vec<String> =
            survivors.iter().map(|i| attempts[*i].label.clone()).collect();
        let a = &attempts[best];

        // Everything replay-observable about the reached state — INCLUDING
        // the record itself, not just its counts. Honest scope (H3 review,
        // SHOULD-1, per the S-2/M5 claim-precision precedent): the record
        // component is belt-and-braces for an ENRICHMENT-ONLY arrival — a
        // socket frame for a move the history spine already covers, whose
        // extra detail re-resolves an EARLIER event's identity (the ledger
        // learning a lab refill's true card) while every counter stays put.
        // NO COMMITTED TEST DISCRIMINATES it: on this corpus, every arrival
        // that changes the record also moves `last_move_id` (the reviewer's
        // counts-only-fingerprint mutation passed the entire suite, and the
        // full corpus streamed line by line renders identically either way),
        // and the committed excerpt cannot exhibit the case — its only socket
        // frames sit past the fixture's history spine, while the corpus's
        // first socket `techAcquisition` is at m120, so a real-data pin would
        // need the whole 1..120 spine committed. The component stays because
        // a LIVE socket can deliver exactly that arrival, and without it the
        // follower would silently keep rendering (and saving) a stale
        // identity. New input that moves none of this (post-STOP chatter,
        // duplicate deliveries) is silent.
        let record_json =
            serde_json::to_string(a.session.record()).expect("a GameRecord serializes");
        let fp = format!(
            "{}|{}|{}|{}|{:?}|{:?}|{}|{record_json}",
            a.report.last_move_id,
            a.report.engine_moves,
            a.report.reveal_events,
            a.report.checkpoints_passed,
            a.stop,
            survivor_labels,
            a.session.is_over(),
        );
        if self.fingerprint.as_deref() == Some(fp.as_str()) {
            return;
        }
        self.fingerprint = Some(fp);
        // A rendered state supersedes any standing hold; if the same trouble
        // recurs later it deserves to be printed again.
        self.last_hold = None;

        let r = &a.report;
        let mut body = format!(
            "following table {} — {} BGA moves captured ({} socket-enriched), {} takeback(s) \
             collapsed, {} snapshot(s)\n",
            feed.table_id,
            feed.report.moves,
            feed.report.enriched,
            feed.report.superseded.len(),
            feed.report.snapshots,
        );
        body.push_str(&format!(
            "replayed through BGA move {} — {} engine moves, {} reveal events, {} checkpoints \
             green\n",
            r.last_move_id, r.engine_moves, r.reveal_events, r.checkpoints_passed,
        ));
        if survivor_labels.len() > 1 {
            body.push_str(&format!(
                "  {} hypothesis assignments still undiscriminated ({survivor_labels:?}) — \
                 rendering the first; the replay decides as the claims arrive\n",
                survivor_labels.len(),
            ));
        }
        match &a.stop {
            Some(stop) => body.push_str(&format!(
                "HOLD at the replay bound: {stop}\n  still tailing — the follower resumes on \
                 its own if later input makes progress possible\n"
            )),
            None if a.session.is_over() => {
                body.push_str(&import::final_check_text(&a.session, &resolved, &feed));
                body.push_str("game complete — follower exiting\n");
                out.complete = true;
            }
            None => {}
        }

        let attempt = attempts.swap_remove(best);
        if let Some(out_path) = &self.opts.out {
            match attempt.session.save(out_path) {
                Ok(()) => body.push_str(&format!("record saved to {}\n", out_path.display())),
                Err(e) => body.push_str(&format!(
                    "WARNING: saving the record to {} failed: {e}\n",
                    out_path.display()
                )),
            }
        }

        // The read-only frame: the same composed screen the driver plays in,
        // asked to draw this session's state above the status body. Frames
        // off (a pipe, tests) yields the body verbatim — the piped-plain rule.
        let repl = crate::repl::Repl::new(attempt.session);
        out.frame = Some(crate::screen::wrap(&repl, &body));
        self.latest = Some(repl);
    }

    fn pipeline(
        &self,
    ) -> Result<(yokohama_bridge::feed::CollapsedFeed, Replays), ImportError> {
        let feed = read_and_collapse(&self.text, self.opts.table)?;
        let answers = super::setup::synthesize(&feed, &self.rules)?;
        let reps = import::replay_attempts(answers, &feed, &self.rules, None)?;
        Ok((feed, reps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const EXCERPT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
    );

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bga-follow-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn append(path: &std::path::Path, chunk: &[u8]) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(chunk).unwrap();
    }

    /// The batch reference: the excerpt imported in one shot, its record saved.
    fn batch_record_bytes(dir: &std::path::Path) -> Vec<u8> {
        let out = dir.join("batch.json");
        let args = import::parse_args(&[EXCERPT.to_string(), "--out".into(), out.display().to_string()])
            .unwrap();
        let outcome = import::run(&args, Rules::standard()).unwrap();
        assert!(outcome.session_built, "{}", outcome.text);
        std::fs::read(&out).unwrap()
    }

    /// Requirement 1+2 core pin: streaming the excerpt through the follower in
    /// arbitrary byte chunks — including boundaries that cut lines and the
    /// 30KB snapshot line in half — reaches the batch import's record
    /// BYTE-IDENTICALLY, holding gracefully at every prefix the pipeline
    /// cannot yet pass.
    #[test]
    fn streaming_the_excerpt_in_chunks_reaches_the_batch_state_byte_identically() {
        let dir = temp_dir("stream");
        let batch = batch_record_bytes(&dir);

        let live = dir.join("live.ndjson");
        let follow_out = dir.join("follow.json");
        let mut follower = Follower::new(
            FollowOptions { path: live.clone(), table: None, out: Some(follow_out.clone()) },
            Rules::standard(),
        );

        // Before the file exists: a single "waiting" hold, printed once.
        let p0 = follower.poll();
        assert_eq!(p0.lines.len(), 1, "{:?}", p0.lines);
        assert!(p0.lines[0].contains("waiting for capture file"));
        assert!(follower.poll().lines.is_empty(), "the waiting hold prints once");

        let bytes = std::fs::read(EXCERPT).unwrap();
        let mut held = 0usize;
        let mut frames = 0usize;
        for chunk in bytes.chunks(7_000) {
            append(&live, chunk);
            let p = follower.poll();
            held += p.lines.len();
            frames += usize::from(p.frame.is_some());
            assert!(!p.complete, "the excerpt ends mid-game");
        }
        // The pipeline held at least once on an incomplete prefix (the
        // excerpt's snapshot does not arrive in the first 7KB), and rendered
        // at least once when the evidence sufficed.
        assert!(held >= 1, "no hold was ever printed");
        assert!(frames >= 1, "no frame was ever rendered");

        // A quiet poll after the stream: no re-render, no re-hold.
        let quiet = follower.poll();
        assert!(quiet.lines.is_empty() && quiet.frame.is_none());

        let followed = std::fs::read(&follow_out).unwrap();
        assert_eq!(
            batch, followed,
            "the streamed follower's record differs from the batch import's"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Requirement 2: kill-and-restart on the same file reaches the identical
    /// state — the fresh follower re-ingests everything, dedupe does the
    /// work, and the remainder of the stream continues from there.
    #[test]
    fn a_restarted_follower_reaches_the_identical_state() {
        let dir = temp_dir("restart");
        let batch = batch_record_bytes(&dir);

        let live = dir.join("live.ndjson");
        let follow_out = dir.join("follow.json");
        let mk = || {
            Follower::new(
                FollowOptions { path: live.clone(), table: None, out: Some(follow_out.clone()) },
                Rules::standard(),
            )
        };
        let bytes = std::fs::read(EXCERPT).unwrap();
        let chunks: Vec<&[u8]> = bytes.chunks(7_000).collect();
        let mid = chunks.len() / 2;

        let mut follower = mk();
        for chunk in &chunks[..mid] {
            append(&live, chunk);
            follower.poll();
        }
        // The kill: drop the follower, state and all.
        drop(follower);
        let mut follower = mk();
        follower.poll(); // re-ingests the whole existing file
        for chunk in &chunks[mid..] {
            append(&live, chunk);
            follower.poll();
        }
        assert_eq!(
            batch,
            std::fs::read(&follow_out).unwrap(),
            "the restarted follower's record differs from the batch import's"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The idempotence pin the plan asks for by name ("dedupe does the work"):
    /// appending the ENTIRE capture a second time — every packet a verbatim
    /// duplicate, exactly what at-least-once delivery produces — changes
    /// nothing: no re-render, no hold, and the reached state is untouched.
    #[test]
    fn re_appending_the_whole_capture_is_a_deduped_no_op() {
        let dir = temp_dir("dedupe");
        let live = dir.join("live.ndjson");
        let bytes = std::fs::read(EXCERPT).unwrap();
        append(&live, &bytes);

        let mut follower = Follower::new(
            FollowOptions { path: live.clone(), table: None, out: None },
            Rules::standard(),
        );
        let first = follower.poll();
        assert!(first.frame.is_some(), "{:?}", first.lines);
        let state_before = serde_json::to_string(follower.session().unwrap().record()).unwrap();

        append(&live, &bytes);
        let second = follower.poll();
        assert!(second.frame.is_none(), "a duplicate delivery re-rendered");
        assert!(second.lines.is_empty(), "a duplicate delivery held: {:?}", second.lines);
        let state_after = serde_json::to_string(follower.session().unwrap().record()).unwrap();
        assert_eq!(state_before, state_after);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The hold arm's print-once discipline on a PIPELINE-level refusal: a
    /// capture with packets but no snapshot holds with the NoSnapshot story
    /// exactly once however many polls pass, then resumes ON ITS OWN when the
    /// snapshot arrives.
    #[test]
    fn a_pipeline_hold_prints_once_and_resumes_when_input_allows() {
        let dir = temp_dir("hold");
        let live = dir.join("live.ndjson");
        let excerpt = std::fs::read_to_string(EXCERPT).unwrap();
        let (snapshots, packets): (Vec<&str>, Vec<&str>) = excerpt.lines().partition(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .ok()
                .and_then(|v| v.get("kind").and_then(|k| k.as_str().map(String::from)))
                .as_deref()
                == Some("snapshot")
        });
        assert!(!snapshots.is_empty(), "the excerpt carries a snapshot");

        append(&live, (packets.join("\n") + "\n").as_bytes());
        let mut follower = Follower::new(
            FollowOptions { path: live.clone(), table: None, out: None },
            Rules::standard(),
        );
        let p1 = follower.poll();
        assert_eq!(p1.lines.len(), 1, "{:?}", p1.lines);
        assert!(p1.lines[0].contains("no gamedatas snapshot"), "{:?}", p1.lines);
        assert!(p1.frame.is_none());

        // More polls, same trouble: silent (metadata short-circuit + dedupe).
        assert!(follower.poll().lines.is_empty());

        // The snapshot arrives; the next poll resumes without being asked.
        append(&live, (snapshots.join("\n") + "\n").as_bytes());
        let p2 = follower.poll();
        assert!(p2.frame.is_some(), "the follower did not resume: {:?}", p2.lines);
        std::fs::remove_dir_all(&dir).ok();
    }
}
