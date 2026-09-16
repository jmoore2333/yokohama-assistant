//! **The change signal (M6.3, spec §2): one `watch` channel per table, one for
//! the house.** A bump is sent AFTER the write it announces lands, never before,
//! so a subscriber that wakes and reads sees the new state. `watch` coalesces:
//! a subscriber that missed three bumps sees one — the property wanted per
//! table, and the reason the house channel carries a bare sequence (R-3.4).

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

#[derive(Default)]
pub struct Signals {
    tables: Mutex<HashMap<GameId, watch::Sender<Bump>>>,
    house: Mutex<Option<watch::Sender<u64>>>,
}

impl Signals {
    fn table_sender(&self, id: &str) -> watch::Sender<Bump> {
        let mut tables = self.tables.lock().expect("signals lock");
        tables
            .entry(id.to_string())
            .or_insert_with(|| watch::channel(Bump::default()).0)
            .clone()
    }

    fn house_sender(&self) -> watch::Sender<u64> {
        let mut house = self.house.lock().expect("signals lock");
        house.get_or_insert_with(|| watch::channel(0).0).clone()
    }

    /// The table's channel only (a keep that does not finalize).
    pub fn bump_table(&self, id: &str) {
        let tx = self.table_sender(id);
        tx.send_modify(|b| b.seq += 1);
    }

    /// The table's channel AND the house.
    pub fn bump(&self, id: &str) {
        self.bump_table(id);
        self.house_sender().send_modify(|n| *n += 1);
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
        self.house_sender().send_modify(|n| *n += 1);
    }

    pub fn subscribe(&self, id: &str) -> watch::Receiver<Bump> {
        self.table_sender(id).subscribe()
    }

    pub fn subscribe_house(&self) -> watch::Receiver<u64> {
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
        assert_eq!(*house.borrow_and_update(), 1);
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
