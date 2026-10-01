//! **The layer's log on this host** (M6.5c2, spec §2.6): the layer's `Log`,
//! RECORDING each line for the call's reply — `from_the_layer` drains it into
//! `Reply.logs`, and the shell's `readReply` logs each as one of its own lines
//! (`worker/src/module.ts`). A line is recorded rather than printed for
//! `signal.rs`'s reason and one more: this module cannot print at all
//! (`eprintln!` is mute on `wasm32-unknown-unknown`, the reason the seam
//! exists).
//!
//! **One line is dropped, by this crate's own word.** On this host every
//! game-over put is deferred first: `ReceiptArchive::put` records the plan and
//! answers `Failed(DEFERRED)`, which the layer logs as `archive-over-refused` —
//! true of the put, false of the save, whose plan rides the same reply and
//! whose upload the shell logs itself (`archive-landed`, `archive-failed`, …).
//! Forwarded, it would be a "refused" line at every game over. The line is
//! recognised by the layer's event CONSTANT and this crate's own `DEFERRED`,
//! which only the put that records a plan answers — so nothing else can match.

use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use yokohama_web::log::{Log, OVER_PUT_REFUSED};

/// One line the layer logged, as `Reply.logs` lists them: `{"event", "fields"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogLine {
    pub event: &'static str,
    pub fields: Value,
}

/// The layer's `Log` for a host that answers one call at a time: every line is
/// appended in call order, and [`ShellLog::drain`] hands the list over and
/// leaves the buffer empty.
#[derive(Default)]
pub struct ShellLog {
    lines: Mutex<Vec<LogLine>>,
}

impl ShellLog {
    /// Every line since the last drain, in call order.
    pub fn drain(&self) -> Vec<LogLine> {
        std::mem::take(&mut *self.lines.lock().expect("the recorded lines"))
    }
}

impl Log for ShellLog {
    fn line(&self, event: &'static str, fields: Value) {
        if event == OVER_PUT_REFUSED && fields.get("error").and_then(Value::as_str) == Some(crate::receipt::DEFERRED) {
            return;
        }
        self.lines.lock().expect("the recorded lines").push(LogLine { event, fields });
    }
}
