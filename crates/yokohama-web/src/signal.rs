//! **The change signal, as a trait (M6.5a, R-6.5a.8): the layer's bump sites
//! call five methods over `&str`** (four until M6.5c2 cycle D added the house's
//! revision bump, R-D4). [`Signals`] — tokio `watch` channels,
//! everything below the `use` lines — implements them for the axum host, and
//! [`NoSignal`] is the unconditional no-op a host with nothing to wake passes.
//! Subscribing is a transport concern and stays on `Signals`: a stream is the
//! only thing that reads a bump back, and there is no stream to hold where
//! there are no channels.
//!
//! The trait sits ABOVE the `use` lines deliberately. It is the whole of what
//! [`crate::games`] needs, it borrows nothing from tokio, and the split is
//! where the `server` feature's gate will fall: everything from
//! `use std::collections::HashMap;` down is the tokio half.
//!
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
//! SAYS which of the two moved. **A third since M6.5c2 cycle D** (R-D4): `rev`,
//! the house chat's revision, which a message moves with `chat` and a DELETE
//! moves alone.
//!
//! **The lobby reads `chat` and `rev` off the frame** (M6.5c2 cycle D, row
//! 130): its chat panel is a COUNT host on them (the client's `Source.peek`),
//! reading its delta when `chat` passes the `last` of the page it holds and
//! re-reading what it holds when `rev` moves further than `chat` — so a
//! table's move, which moves `seq` alone, costs the panel nothing. **Those two
//! are the house chat DOCUMENT's `last` and `rev` on both hosts**: the Worker
//! reads them off the stored document for every frame it sends, and the NAS seeds
//! [`Signals`] from the stored document when it starts
//! (`Signals::seed_house`, the final review's C1) and moves them in step with
//! it after. `seq` is the frame's own sequence and names nothing stored.
//! **The lobby's LISTINGS still take every frame as a wake-up** and refetch
//! all three, a house message included (recorded in the M6.4 close as a LEAVE
//! item): telling a message from a listing write there is the half no consumer
//! takes yet.

/// What a write site calls when something it wrote has changed.
///
/// Five methods over `&str` and nothing else: this trait knows no game, no
/// table and no seat, and returns nothing, so it is object-safe and a layer
/// holds one as `Arc<dyn Signal>`. **A bump follows its write and never
/// precedes it** — the contract is the caller's, and it is why every call sits
/// on a success path.
pub trait Signal: Send + Sync {
    /// The table's channel AND the house's LISTING counter.
    fn bump(&self, id: &str);
    /// The table's channel only (a keep that does not finalize).
    fn bump_table(&self, id: &str);
    /// The house's CHAT counter only — a house message wakes every lobby and
    /// moves no table listing (M6.4, R-6.4.7).
    fn bump_house_chat(&self);
    /// The house's chat REVISION only — a house message DELETED (M6.5c2 cycle
    /// D, R-D4), which adds no message and so must not move `chat`.
    fn bump_house_chat_rev(&self);
    /// The terminal bump: the table is gone and every stream on it ends.
    fn end(&self, id: &str);
}

/// The signal of a host with nothing to wake.
///
/// Not a test double: a layer compiled without the transport half has no
/// channels to bump, and the write sites are the same writes either way. It
/// keeps nothing, so a bump costs a virtual call and no allocation.
pub struct NoSignal;

impl Signal for NoSignal {
    fn bump(&self, _id: &str) {}
    fn bump_table(&self, _id: &str) {}
    fn bump_house_chat(&self) {}
    fn bump_house_chat_rev(&self) {}
    fn end(&self, _id: &str) {}
}

/// The house frame's DATA: the three counters as `{"seq":…,"chat":…,"rev":…}`,
/// in that order (M6.4, R-6.4.7; `rev` since M6.5c2 cycle D, R-D4). `pub` and
/// outside `server` since M6.5c2 (spec §2.6's restatements): `house_stream`
/// writes it as each `house` event's data, and the Worker module's lobby socket
/// writes the same bytes by calling this, not a copy pinned to this crate's
/// source text. The lobby reads `chat` and `rev` off it, as the house chat
/// document's own numbers — see the module doc.
pub fn house_frame(seq: u64, chat: u64, rev: u64) -> String {
    format!("{{\"seq\":{seq},\"chat\":{chat},\"rev\":{rev}}}")
}

#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::sync::Mutex;
#[cfg(feature = "server")]
use tokio::sync::watch;
#[cfg(feature = "server")]
use crate::store::GameId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg(feature = "server")]
pub struct Bump {
    pub seq: u64,
    /// Sent once, by `end`: the table was deleted and every stream on it closes.
    pub end: bool,
}

/// The house frame's three counters (M6.4, R-6.4.7; `rev` since M6.5c2 cycle
/// D, R-D4).
///
/// `seq` moves on every write a lobby LISTING reads — a create, a join, a
/// finalize, a move, a delete — and `chat` moves on a house message and on
/// nothing else. `rev` moves on a house message AND on a house message
/// deleted, so a frame whose `rev` moved further than its `chat` says that a
/// message a reader holds has changed. They are separate so that a consumer
/// can tell them apart and refetch only the half that moved. **The lobby's chat
/// panel does** (M6.5c2 cycle D, row 130): it reads when `chat` passes what it
/// holds and re-reads when `rev` moves further than `chat`, so a table's move
/// costs it nothing. **The lobby's listings do not**: they refetch all three
/// on any frame, chat included, which is the M6.4 close's own LEAVE item.
///
/// `chat` and `rev` START at the stored house chat document's `last` and `rev`
/// (`Signals::seed_house`, the final review's C1), so they are the document's
/// numbers and not a count this process began at 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg(feature = "server")]
pub struct House {
    pub seq: u64,
    pub chat: u64,
    pub rev: u64,
}

#[derive(Default)]
#[cfg(feature = "server")]
pub struct Signals {
    tables: Mutex<HashMap<GameId, watch::Sender<Bump>>>,
    house: Mutex<Option<watch::Sender<House>>>,
}

#[cfg(feature = "server")]
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

    pub fn subscribe(&self, id: &str) -> watch::Receiver<Bump> {
        self.table_sender(id).subscribe()
    }

    pub fn subscribe_house(&self) -> watch::Receiver<House> {
        self.house_sender().subscribe()
    }

    /// Start the house's `chat` and `rev` at the stored house chat document's
    /// `last` and `rev` (M6.5c2 cycle D, the final review's C1), and `seq` at 0
    /// — it is the frame's own sequence and names nothing stored.
    ///
    /// **Called ONCE, by `AppState::with_store_chat_and_archive`, before the
    /// state serves anything**: it REPLACES the two counters rather than adding
    /// to them, so a call after the first bump would put an older number back.
    /// After it the pair stays the document's with no further read, because
    /// the NAS is one process and the only writer: [`Signal::bump_house_chat`]
    /// adds one exactly where an append moves `last` and `rev`, and
    /// [`Signal::bump_house_chat_rev`] exactly where a delete moves `rev` — a
    /// trim moves neither.
    pub(crate) fn seed_house(&self, counts: crate::chat::ChatCounts) {
        self.house_sender().send_modify(|h| {
            h.chat = counts.last;
            h.rev = counts.rev;
        });
    }

    #[cfg(test)]
    pub fn senders(&self) -> usize {
        self.tables.lock().expect("signals lock").len()
    }
}

/// The five bumps, over the channels above.
///
/// They are the trait's since M6.5a rather than inherent, so a layer holding
/// `Arc<dyn Signal>` reaches exactly these; `subscribe`, `subscribe_house` and
/// the two private senders stay inherent, because reading a bump back is the
/// transport's side and the trait has no business naming a `watch::Receiver`.
#[cfg(feature = "server")]
impl Signal for Signals {
    fn bump_table(&self, id: &str) {
        let tx = self.table_sender(id);
        tx.send_modify(|b| b.seq += 1);
    }

    fn bump(&self, id: &str) {
        self.bump_table(id);
        self.house_sender().send_modify(|h| h.seq += 1);
    }

    fn bump_house_chat(&self) {
        self.house_sender().send_modify(|h| {
            h.chat += 1;
            h.rev += 1;
        });
    }

    fn bump_house_chat_rev(&self) {
        self.house_sender().send_modify(|h| h.rev += 1);
    }

    /// The terminal bump, then the sender is REMOVED so every stream on the
    /// table ends; the house is bumped so a lobby drops the row.
    fn end(&self, id: &str) {
        let tx = self.tables.lock().expect("signals lock").remove(id);
        if let Some(tx) = tx {
            tx.send_modify(|b| {
                b.seq += 1;
                b.end = true;
            });
        }
        self.house_sender().send_modify(|h| h.seq += 1);
    }
}

// ---- end server ----

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
    /// tell a message from a listing write. (The lobby's chat panel does; its
    /// listings wake on either — see [`House`].) The second half of this cell is the
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
        assert_eq!((h.seq, h.chat, h.rev), (0, 1, 1), "a message is a revision too (R-D4)");
        s.bump("t");
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat, h.rev), (1, 1, 1), "a table bump leaves both chat counters");
    }

    /// **A house DELETE moves the revision and nothing else** (M6.5c2 cycle D,
    /// R-D4): it adds no message, so `chat` stays, and it changes no listing,
    /// so `seq` stays — but a lobby must still wake, and the frame must SAY
    /// that what it holds changed. The catcher for "a delete bumps `last`" on
    /// the NAS's house channel.
    #[test]
    fn a_house_delete_bump_moves_rev_alone() {
        let s = Signals::default();
        let mut house = s.subscribe_house();
        s.bump_house_chat();
        house.borrow_and_update();
        s.bump_house_chat_rev();
        assert!(house.has_changed().unwrap(), "a lobby wakes on a delete");
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat, h.rev), (0, 1, 2));
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

    /// **The signal of a host with nothing to wake answers every call** (M6.5a,
    /// R-6.5a.8).
    ///
    /// [`NoSignal`] is what a layer built without the server half holds, so
    /// every bump site calls it on every write path there is. The cell walks
    /// all five methods and then one through the trait object the
    /// layer actually stores, because a `dyn Signal` is the shape that has to
    /// compile: a trait taking `&str` and returning nothing is object-safe, and
    /// a method that later grew a generic would fail HERE rather than at the
    /// one call site that happened to be built.
    #[test]
    fn no_signal_accepts_every_call_and_keeps_nothing() {
        let s = NoSignal;
        s.bump("a");
        s.bump_table("a");
        s.bump_house_chat();
        s.bump_house_chat_rev();
        s.end("a");
        // The trait object shape the layer holds.
        let boxed: std::sync::Arc<dyn Signal> = std::sync::Arc::new(NoSignal);
        boxed.bump("b");
    }

    /// **A bump through the trait is the same bump** (M6.5a, R-6.5a.8).
    ///
    /// The four methods moved from inherent `impl Signals` to `impl Signal for
    /// Signals`. The cells above this one hold a `Signals` by value and reach
    /// them by ordinary method resolution, which says nothing about the shape
    /// the layer stores; this one subscribes and then calls through
    /// `Arc<dyn Signal>`, so a dynamic dispatch landing on a different sender —
    /// or a `NoSignal` wired in its place — reads as a channel that never
    /// moved.
    #[test]
    fn signals_through_the_trait_are_the_same_bumps() {
        let s = std::sync::Arc::new(Signals::default());
        let rx = s.subscribe("t");
        let dynamic: std::sync::Arc<dyn Signal> = s.clone();
        dynamic.bump("t");
        assert_eq!(rx.borrow().seq, 1, "a bump through the trait reaches the same channel");
        dynamic.end("t");
        assert!(rx.borrow().end);
    }

    /// **The house frame's data, exported** (M6.5c2, spec §2.6's restatements;
    /// `rev` since cycle D, R-D4): `{"seq":…,"chat":…,"rev":…}`, in that order,
    /// whole `u64`s — `house_stream`'s frame, and the Worker module's
    /// lobby-socket frame, which imports this.
    #[test]
    fn the_house_frame_is_seq_then_chat_then_rev() {
        assert_eq!(house_frame(3, 7, 8), r#"{"seq":3,"chat":7,"rev":8}"#);
        assert_eq!(house_frame(0, 0, 0), r#"{"seq":0,"chat":0,"rev":0}"#);
        assert_eq!(
            house_frame(u64::MAX, 1, u64::MAX),
            format!("{{\"seq\":{},\"chat\":1,\"rev\":{}}}", u64::MAX, u64::MAX),
        );
    }
}
