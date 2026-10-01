//! **The recording signal** (spec §2.2): the layer's `Signal`, implemented by
//! appending each call to a buffer that `dispatch` drains into its reply.
//!
//! It knows no socket. Sending is the shell's, after the request's batch has
//! committed the documents the bumps announce (spec §2.4) — which is why a bump
//! is RECORDED here rather than acted on: when the layer calls it, the write it
//! announces has reached only this module's in-memory store.

use std::sync::Mutex;

use serde::Serialize;
use yokohama_web::signal::Signal;

/// One call the layer made, tagged by the method it called — the list the
/// shell reads out of `Reply.bumps`: `{"bump": id}`, `{"bump_table": id}`,
/// `{"bump_house_chat": null}`, `{"bump_house_chat_rev": null}` (M6.5c2 cycle
/// D, R-D4: a house message deleted) and `{"end": id}`.
///
/// **`BumpHouseChat` and `BumpHouseChatRev` carry `()`**, which is what
/// serializes them as `{"bump_house_chat":null}` and
/// `{"bump_house_chat_rev":null}`: a unit variant would be the bare string, and
/// every entry of the list is an object with one key, so the shell reads all
/// five the same way.
///
/// The variants are named for the trait's five methods, because the names ARE
/// the wire's tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bump {
    /// `Signal::bump`: the table's channel AND the house's listing counter.
    Bump(String),
    /// `Signal::bump_table`: the table's channel only.
    BumpTable(String),
    /// `Signal::bump_house_chat`: the house's chat counter only.
    BumpHouseChat(()),
    /// `Signal::bump_house_chat_rev`: the house's chat revision only — a
    /// message deleted.
    BumpHouseChatRev(()),
    /// `Signal::end`: the table is gone and every socket on it ends.
    End(String),
}

/// The layer's `Signal` for a host that answers one call at a time: every call
/// is appended in call order, and [`RecordingSignal::drain`] hands the list
/// over and leaves the buffer empty.
#[derive(Default)]
pub struct RecordingSignal {
    calls: Mutex<Vec<Bump>>,
}

impl RecordingSignal {
    /// Every call since the last drain, in call order.
    pub fn drain(&self) -> Vec<Bump> {
        std::mem::take(&mut *self.calls.lock().expect("the recorded bumps"))
    }

    fn push(&self, bump: Bump) {
        self.calls.lock().expect("the recorded bumps").push(bump);
    }
}

impl Signal for RecordingSignal {
    fn bump(&self, id: &str) {
        self.push(Bump::Bump(id.to_string()));
    }

    fn bump_table(&self, id: &str) {
        self.push(Bump::BumpTable(id.to_string()));
    }

    fn bump_house_chat(&self) {
        self.push(Bump::BumpHouseChat(()));
    }

    fn bump_house_chat_rev(&self) {
        self.push(Bump::BumpHouseChatRev(()));
    }

    fn end(&self, id: &str) {
        self.push(Bump::End(id.to_string()));
    }
}
