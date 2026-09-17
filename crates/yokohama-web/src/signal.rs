//! **The change signal (M6.3, spec §2): one `watch` channel per table, one for
//! the house.** A bump is sent AFTER the write it announces lands, never before,
//! so a subscriber that wakes and reads sees the new state. `watch` coalesces:
//! a subscriber that missed three bumps sees one — the property wanted per
//! table, and the reason the house channel carries COUNTERS and never a
//! payload naming a table (R-3.4): two bumps naming two tables inside one wake
//! would lose one, so a frame that named one would sometimes name the wrong one.
//!
//! **Since M6.4 the house carries TWO counters** ([`House`], R-6.4.7): `seq`
//! for the writes a lobby listing reads and `chat` for a house message. One
//! channel because a lobby holds one connection; two numbers so that a frame
//! SAYS which of the two moved — which is a capability and not yet a saving.
//! At HEAD the lobby is a `"wake"` consumer and drops the frame, so a house
//! message costs every visible lobby its three listing reads (recorded in the
//! M6.4 close as a LEAVE item); the second counter is what a `"value"` consumer
//! would need to tell them apart.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::watch;

use crate::store::GameId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bump {
    pub seq: u64,
    /// Sent once, by `end`: the table was deleted and every stream on it closes.
    pub end: bool,
}

/// The house frame's two counters (M6.4, R-6.4.7).
///
/// `seq` moves on every write a lobby LISTING reads — a create, a join, a
/// finalize, a move, a delete — and `chat` moves on a house message and on
/// nothing else. The two are separate so that a consumer CAN tell them apart
/// and refetch only the half that moved. **No consumer does today**: the lobby
/// reads this channel in `"wake"` mode and refetches all three listings on any
/// frame, chat included, which is the M6.4 close's own LEAVE item. Read this as
/// the shape a later `"value"` consumer needs, not as a saving already taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct House {
    pub seq: u64,
    pub chat: u64,
}

#[derive(Default)]
pub struct Signals {
    tables: Mutex<HashMap<GameId, watch::Sender<Bump>>>,
    house: Mutex<Option<watch::Sender<House>>>,
}

impl Signals {
    fn table_sender(&self, id: &str) -> watch::Sender<Bump> {
        let mut tables = self.tables.lock().expect("signals lock");
        tables
            .entry(id.to_string())
            .or_insert_with(|| watch::channel(Bump::default()).0)
            .clone()
    }

    fn house_sender(&self) -> watch::Sender<House> {
        let mut house = self.house.lock().expect("signals lock");
        house.get_or_insert_with(|| watch::channel(House::default()).0).clone()
    }

    /// The table's channel only (a keep that does not finalize).
    pub fn bump_table(&self, id: &str) {
        let tx = self.table_sender(id);
        tx.send_modify(|b| b.seq += 1);
    }

    /// The table's channel AND the house's LISTING counter.
    pub fn bump(&self, id: &str) {
        self.bump_table(id);
        self.house_sender().send_modify(|h| h.seq += 1);
    }

    /// The house's CHAT counter only — a house message wakes every lobby and
    /// moves no table listing (M6.4, R-6.4.7).
    pub fn bump_house_chat(&self) {
        self.house_sender().send_modify(|h| h.chat += 1);
    }

    /// The terminal bump, then the sender is REMOVED so every stream on the
    /// table ends; the house is bumped so a lobby drops the row.
    pub fn end(&self, id: &str) {
        let tx = self.tables.lock().expect("signals lock").remove(id);
        if let Some(tx) = tx {
            tx.send_modify(|b| {
                b.seq += 1;
                b.end = true;
            });
        }
        self.house_sender().send_modify(|h| h.seq += 1);
    }

    pub fn subscribe(&self, id: &str) -> watch::Receiver<Bump> {
        self.table_sender(id).subscribe()
    }

    pub fn subscribe_house(&self) -> watch::Receiver<House> {
        self.house_sender().subscribe()
    }

    #[cfg(test)]
    pub fn senders(&self) -> usize {
        self.tables.lock().expect("signals lock").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subscriber_before_the_first_bump_wakes_on_it_and_reads_the_latest_seq() {
        let s = Signals::default();
        let mut rx = s.subscribe("t");
        assert_eq!(rx.borrow().seq, 0);
        s.bump_table("t");
        s.bump_table("t");
        s.bump_table("t");
        assert!(rx.has_changed().unwrap(), "three bumps, one wake");
        let b = *rx.borrow_and_update();
        assert_eq!(b.seq, 3);
        assert!(!b.end);
        assert!(!rx.has_changed().unwrap(), "coalesced: nothing further pending");
    }

    #[test]
    fn bump_reaches_the_house_and_bump_table_does_not() {
        let s = Signals::default();
        let mut house = s.subscribe_house();
        s.bump_table("t");
        assert!(!house.has_changed().unwrap());
        s.bump("t");
        assert!(house.has_changed().unwrap());
        assert_eq!(house.borrow_and_update().seq, 1);
    }

    /// **A house MESSAGE moves the chat counter and leaves the listing counter
    /// alone** (M6.4, R-6.4.7), and a table write moves the listing counter and
    /// leaves the chat counter alone.
    ///
    /// Two counters on one coalescing channel rather than two channels: a lobby
    /// holds ONE connection, and the frame is what a consumer would read to
    /// tell a message from a listing write. (None does at HEAD — the lobby
    /// wakes on either; see [`House`].) The second half of this cell is the
    /// half that can fail — a `bump_house_chat` that reached for `seq`, or a
    /// `bump` that touched `chat`, would make the frame's two numbers one
    /// number wearing two names.
    #[test]
    fn a_house_chat_bump_moves_chat_and_leaves_seq() {
        let s = Signals::default();
        let mut house = s.subscribe_house();
        s.bump_house_chat();
        assert!(house.has_changed().unwrap());
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat), (0, 1));
        s.bump("t");
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat), (1, 1), "a table bump leaves the chat counter");
    }

    #[test]
    fn end_sends_the_terminal_bump_removes_the_sender_and_bumps_the_house() {
        let s = Signals::default();
        let mut rx = s.subscribe("t");
        // Not `mut`: this cell only asks `has_changed`, which borrows shared.
        let house = s.subscribe_house();
        s.end("t");
        assert!(rx.borrow_and_update().end, "the terminal bump");
        assert!(house.has_changed().unwrap());
        // The sender is gone: the receiver reports the channel closed.
        assert!(rx.has_changed().is_err() || s.senders() == 0);
        // A later subscribe makes a fresh sender that has seen nothing.
        let fresh = s.subscribe("t");
        assert_eq!(fresh.borrow().seq, 0);
        assert!(!fresh.borrow().end);
    }
}
