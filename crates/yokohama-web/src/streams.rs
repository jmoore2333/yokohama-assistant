//! Open-stream caps on the NAS (security S2c, finding D3's NAS half; S2b's Worker half is the socket caps in
//! `worker/src/table/sockets.ts` and `house.ts`, decision D-4 there).
//!
//! **Counted at admission, OPEN streams only.** [`Streams::admit`] hands back a [`Slot`] that the SSE body holds
//! ([`crate::Admitted`]) and releases on its drop — so a connection the client abandons frees its slot exactly when
//! hyper drops the body, never before and never later, and nothing else can free one. The review's measurement:
//! 500 streams as 500 watchers in 3.1 s, one chat post fanned out to 500 admission reads, since each table-stream
//! frame costs a `spawn_blocking` read on the one 512-thread pool. A house frame reads nothing, but every lobby it
//! reaches re-reads its three listings on a bump that re-lists, three dispatches on that same pool — 768 at 256
//! lobbies — which is why the house's total is 256 here where the Worker's lobby holds 1,024 sockets per object,
//! its lobbies' re-reads landing on requests of their own.
//!
//! **The figures mirror the Worker's** so the two hosts read alike: 8 per caller a room, 64 a table. A table also
//! holds at most [`WATCHERS_PER_TABLE`] streams admitted for callers who held no seat at it. The verdicts run the
//! room's total first, then — for a caller with no seat — the watchers' figure, then the caller's, each a constant
//! sentence and never a caller value. The maps hold an entry only while a stream is open (B8's lesson: a map nobody
//! has written costs nothing).
//!
//! **The per-caller key is the identity the door hands over** — the verified address under Access, the claimed name
//! under the open door, where a name rotation escapes the per-caller figure and meets the watchers' figure and the
//! room's total alone; that is the open door's property, recorded in the ledger's D3 and the lane's §15 (the
//! Worker's sibling is §15's socket-cap row, S2b's finding D3, and this module's own is the NAS stream-cap row
//! beside it). The room's total is the bound a process relies on, and it judges a seated player as it judges
//! anyone: a full room refuses them, in the TABLE's sentence that takes them to the lobby (S2c's D-2), until streams
//! close. No figure here is a budget a feature may lean on; a seated caller is counted apart (`seated`) and refused
//! only by the total and by their own figure.
//!
//! **A stream is classified by its caller's standing when it is admitted, and never again**: the bit is the one the
//! route's admission read answers ([`crate::api::events_at_seated`]). A joiner's earlier stream counts as a
//! watcher's until it reconnects, and a stream opened by a seat holder whose seat the creator then reopens counts as
//! a seated one until it does.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::identity::User;

/// Streams one caller may hold open at one table: a table page holds one, the owner's "a couple of games at once"
/// a few; eight leaves room for every tab of one person and refuses a script. The Worker's `SOCKETS_PER_CALLER`.
pub const STREAMS_PER_CALLER: usize = 8;
/// Streams one table may hold open in all: four seats and their watchers at eight each. The Worker's
/// `SOCKETS_PER_TABLE`.
pub const STREAMS_PER_TABLE: usize = 64;
/// Streams one table may hold open for callers who hold no seat at it: the total less four seats at eight each, so
/// that while watchers sit at this figure the room keeps the seated callers' room (S4-pre, the spec's Issue 1).
pub const WATCHERS_PER_TABLE: usize = 32;
/// Lobby streams one caller may hold open. The Worker's `LOBBY_PER_CALLER`.
pub const HOUSE_STREAMS_PER_CALLER: usize = 8;
/// Lobby streams the process holds open in all: 32 people at eight. A house frame reads nothing, but each lobby it
/// reaches re-reads its three listings on a bump that re-lists, on the one 512-thread blocking pool — 768
/// dispatches at this figure (the Worker's lobby object holds 1,024 because its lobbies' re-reads are requests of
/// their own).
pub const HOUSE_STREAMS_TOTAL: usize = 256;

pub const TOO_MANY_STREAMS_CALLER: &str = "too many open streams from you at this table — close one and try again";
pub const TOO_MANY_STREAMS_TABLE: &str = "this table has too many open streams — try again in a moment";
pub const TOO_MANY_WATCHERS: &str = "this table has too many open watcher streams — try again in a moment";
pub const TOO_MANY_HOUSE_CALLER: &str = "too many open lobby streams from you — close one and try again";
pub const TOO_MANY_HOUSE_TOTAL: &str = "the lobby has too many open streams — try again in a moment";

/// A room a stream is about: one table, or the house.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Room {
    Table(String),
    House,
}

#[derive(Default)]
struct Counts {
    by_room: HashMap<Room, usize>,
    by_caller: HashMap<(Room, String), usize>,
    /// The OPEN seated streams of each table, counted inside `by_room` too: a table's watchers are its `by_room`
    /// less this. The house never has an entry.
    seated_by_room: HashMap<Room, usize>,
}

/// The counts, one per process — `AppState.streams`.
#[derive(Default)]
pub struct Streams {
    counts: Mutex<Counts>,
}

/// One admitted stream's place in the counts, released on drop.
pub struct Slot {
    streams: Arc<Streams>,
    room: Room,
    caller: String,
    /// Whether this slot counts in `seated_by_room` — the caller's standing at admission, never revisited; always
    /// `false` in the house.
    seated: bool,
}

impl Streams {
    /// The verdict for one more stream by `caller` in `room`, `seated` saying whether the caller holds a seat at
    /// the table: the room's total first, then — for a table's caller with no seat — the watchers' figure, then the
    /// caller's. `Err` carries the sentence the 429 repeats; `Ok` holds the slot until it is dropped. The house has
    /// no seats and ignores the bit: its slot is never seated and its two verdicts are as they were.
    pub fn admit(self: &Arc<Self>, room: Room, caller: &User, seated: bool) -> Result<Slot, &'static str> {
        let (per_caller, total, caller_sentence, total_sentence) = match room {
            Room::Table(_) => (STREAMS_PER_CALLER, STREAMS_PER_TABLE, TOO_MANY_STREAMS_CALLER, TOO_MANY_STREAMS_TABLE),
            Room::House => (HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL, TOO_MANY_HOUSE_CALLER, TOO_MANY_HOUSE_TOTAL),
        };
        let table = matches!(room, Room::Table(_));
        let seated = seated && table;
        let key = (room.clone(), caller.as_str().to_string());
        let mut counts = self.counts.lock().expect("streams lock");
        let open = counts.by_room.get(&room).copied().unwrap_or(0);
        if open >= total {
            return Err(total_sentence);
        }
        // `saturating_sub`, though every seated slot is counted in `by_room` under this same lock: nothing under
        // this lock panics (the drop's comment says why that matters).
        if table && !seated && open.saturating_sub(counts.seated_by_room.get(&room).copied().unwrap_or(0)) >= WATCHERS_PER_TABLE {
            return Err(TOO_MANY_WATCHERS);
        }
        if counts.by_caller.get(&key).copied().unwrap_or(0) >= per_caller {
            return Err(caller_sentence);
        }
        *counts.by_room.entry(room.clone()).or_insert(0) += 1;
        *counts.by_caller.entry(key.clone()).or_insert(0) += 1;
        if seated {
            *counts.seated_by_room.entry(room.clone()).or_insert(0) += 1;
        }
        Ok(Slot { streams: Arc::clone(self), room, caller: key.1, seated })
    }

    /// Open streams in `room` — for the cells and a reading.
    pub fn open(&self, room: &Room) -> usize {
        self.counts.lock().expect("streams lock").by_room.get(room).copied().unwrap_or(0)
    }

    /// Open SEATED streams in `room` — those admitted with the bit — for the cells and a reading. Always 0 for the
    /// house.
    pub fn seated(&self, room: &Room) -> usize {
        self.counts.lock().expect("streams lock").seated_by_room.get(room).copied().unwrap_or(0)
    }

    /// Whether every map is empty — what a process with no open stream holds (B8's lesson).
    #[cfg(test)]
    pub(crate) fn is_quiet(&self) -> bool {
        let counts = self.counts.lock().expect("streams lock");
        counts.by_room.is_empty() && counts.by_caller.is_empty() && counts.seated_by_room.is_empty()
    }
}

fn release<K: std::hash::Hash + Eq>(map: &mut HashMap<K, usize>, key: &K) {
    if let Some(n) = map.get_mut(key) {
        *n -= 1;
        if *n == 0 {
            map.remove(key);
        }
    }
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot").field("room", &self.room).field("seated", &self.seated).finish_non_exhaustive()
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        // Poison-tolerant, in `games::enter`'s form (the plan review's AD-18): nothing under this lock panics today,
        // and an `expect` here would turn a poisoned lock into a panic inside a drop.
        let mut counts = self.streams.counts.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        release(&mut counts.by_room, &self.room);
        if self.seated {
            release(&mut counts.seated_by_room, &self.room);
        }
        release(&mut counts.by_caller, &(self.room.clone(), std::mem::take(&mut self.caller)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn user(s: &str) -> crate::identity::User {
        crate::identity::User::parse(s).expect("a key")
    }

    /// **The verdicts, the room's total first** (the Worker's order, `sockets.unit.test.ts`): one caller's ninth
    /// stream at a table is refused with the caller's sentence; the sixty-fifth of a table whatever its caller with
    /// the table's; the house's figures are its own; and a dropped slot frees the count — OPEN streams alone are
    /// counted, which is what D-4 of S2b measured the Worker needed. The maps hold nothing for a room with no open
    /// stream (B8's lesson).
    #[test]
    fn the_caps_refuse_the_ninth_of_a_caller_and_the_sixty_fifth_of_a_table_and_a_dropped_slot_frees_the_count() {
        let streams = Arc::new(Streams::default());
        let table = Room::Table("t".to_string());
        let mut held = Vec::new();
        for n in 1..=STREAMS_PER_CALLER {
            held.push(streams.admit(table.clone(), &user("red@wire.test"), false).unwrap_or_else(|e| panic!("stream {n}: {e}")));
        }
        assert_eq!(streams.admit(table.clone(), &user("red@wire.test"), false).unwrap_err(), TOO_MANY_STREAMS_CALLER);
        let blue = streams.admit(table.clone(), &user("blue@wire.test"), false);
        assert!(blue.is_ok(), "another caller is unaffected");
        assert_eq!(streams.open(&table), STREAMS_PER_CALLER + 1);
        drop(held.pop());
        assert!(streams.admit(table.clone(), &user("red@wire.test"), false).is_ok(), "a closed stream frees the slot");
        // The table's total: the c-callers fill the table to 64 beside red's seven and blue's one; the sixty-fifth, a
        // ninth caller's first, is refused with the TABLE's sentence (the total's verdict comes first). The c-callers
        // are SEATED (S4-pre): watchers' streams would stop at `WATCHERS_PER_TABLE`, short of the total, and this
        // module believes the bit it is handed.
        let mut fill = Vec::new();
        for caller in 1..=8 {
            for _ in 0..STREAMS_PER_CALLER {
                let who = format!("c{caller}@wire.test");
                if let Ok(slot) = streams.admit(table.clone(), &user(&who), true) { fill.push(slot); }
            }
        }
        assert_eq!(streams.open(&table), STREAMS_PER_TABLE);
        assert_eq!(streams.admit(table.clone(), &user("ninth@wire.test"), false).unwrap_err(), TOO_MANY_STREAMS_TABLE);
        // **The ORDER of the two verdicts, where both would fail** (M25's catcher — Task 1's probe: a fresh caller's
        // first stream cannot tell the order, since only one check can fail for it): a caller AT its own figure in a
        // FULL room reads the TABLE's sentence, so the total's verdict comes first.
        assert_eq!(streams.admit(table.clone(), &user("c1@wire.test"), true).unwrap_err(), TOO_MANY_STREAMS_TABLE);
        // **The per-caller figure is per ROOM** (Task 1's probe P4b: a count keyed by the caller alone survived every
        // cell): c1 at eight here may still open at another table.
        assert!(streams.admit(Room::Table("u".to_string()), &user("c1@wire.test"), true).is_ok(), "the per-caller figure is per room");
        drop(fill);
        drop(held);
        drop(blue);
        assert_eq!(streams.open(&table), 0, "every slot released");
        assert!(streams.is_quiet(), "nothing kept for a quiet room");
        // The house: its own per-caller figure and its own total sentence.
        let mut house = Vec::new();
        for _ in 0..HOUSE_STREAMS_PER_CALLER { house.push(streams.admit(Room::House, &user("red@wire.test"), false).unwrap()); }
        assert_eq!(streams.admit(Room::House, &user("red@wire.test"), false).unwrap_err(), TOO_MANY_HOUSE_CALLER);
        for caller in 1..=31 {
            for _ in 0..HOUSE_STREAMS_PER_CALLER { house.push(streams.admit(Room::House, &user(&format!("h{caller}@wire.test")), false).unwrap()); }
        }
        assert_eq!(streams.open(&Room::House), HOUSE_STREAMS_TOTAL);
        assert_eq!(streams.admit(Room::House, &user("late@wire.test"), false).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        // The order again, in the house: h1 holds eight in the full house and reads the TOTAL's sentence.
        assert_eq!(streams.admit(Room::House, &user("h1@wire.test"), false).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        assert_eq!((STREAMS_PER_CALLER, STREAMS_PER_TABLE, HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL), (8, 64, 8, 256));
    }

    /// `n` callers named `{prefix}1@wire.test`.. each holding `STREAMS_PER_CALLER` streams in `room`, every one
    /// admitted with `seated` — the slots pushed onto `held`, so they stay counted until the cell drops them.
    fn fill(streams: &Arc<Streams>, room: &Room, prefix: &str, n: usize, seated: bool, held: &mut Vec<Slot>) {
        for caller in 1..=n {
            let who = format!("{prefix}{caller}@wire.test");
            for k in 1..=STREAMS_PER_CALLER {
                held.push(streams.admit(room.clone(), &user(&who), seated).unwrap_or_else(|e| panic!("{who}'s stream {k}: {e}")));
            }
        }
    }

    /// **A seated caller is admitted into a room whose watchers sit at their figure** (S4-pre, the spec's Issue 1):
    /// four strangers' eight streams each fill the watchers' figure, and a seat holder's stream is still admitted,
    /// counted in the room's total and apart as a seated one. Mutants: the seat bit ignored (the stream reads the
    /// watchers' sentence), or a seated stream never counted apart (`seated` reads 0).
    #[test]
    fn a_seated_callers_stream_is_admitted_into_a_room_whose_watchers_sit_at_their_figure() {
        let streams = Arc::new(Streams::default());
        let room = Room::Table("t".to_string());
        let mut held = Vec::new();
        fill(&streams, &room, "w", 4, false, &mut held);
        assert_eq!(streams.open(&room), WATCHERS_PER_TABLE, "premise: the watchers sit at their figure");
        assert_eq!(streams.seated(&room), 0, "premise: no seated stream yet");
        let seat = streams.admit(room.clone(), &user("seat@t.test"), true);
        assert!(seat.is_ok(), "a seat holder's stream is admitted: {seat:?}");
        assert_eq!(streams.open(&room), WATCHERS_PER_TABLE + 1, "counted in the room's total");
        assert_eq!(streams.seated(&room), 1, "and apart, as a seated stream");
        assert_eq!(WATCHERS_PER_TABLE, 32, "the watchers' figure: the total less four seats at eight");
    }

    /// **A watcher's stream past the watchers' figure is refused in the watchers' sentence, with room to spare in
    /// the total**, and the refusal takes no slot. The second half reads the figure as the room's count LESS its
    /// seated streams: with eight seated streams in the room and 31 watchers, the thirty-second watcher is still
    /// admitted (the mutant that compares the room's whole count against the watchers' figure refuses it), and the
    /// thirty-third is not. The sentence is the owner's (2026-10-23), pinned at its literal.
    #[test]
    fn a_watchers_stream_past_the_watchers_figure_is_refused_in_the_watchers_sentence_with_room_to_spare_in_the_total() {
        let streams = Arc::new(Streams::default());
        let room = Room::Table("t".to_string());
        let mut held = Vec::new();
        fill(&streams, &room, "w", 4, false, &mut held);
        assert_eq!(streams.admit(room.clone(), &user("w5@wire.test"), false).unwrap_err(), TOO_MANY_WATCHERS);
        assert_eq!(streams.open(&room), WATCHERS_PER_TABLE, "the refusal took no slot");
        assert!(streams.open(&room) < STREAMS_PER_TABLE, "the total had room to spare");
        assert_eq!(
            TOO_MANY_WATCHERS,
            "this table has too many open watcher streams \u{2014} try again in a moment",
        );
        // Watchers are the room's count less its seated streams.
        drop(held.pop());
        fill(&streams, &room, "s", 1, true, &mut held);
        assert_eq!((streams.open(&room), streams.seated(&room)), (WATCHERS_PER_TABLE - 1 + STREAMS_PER_CALLER, STREAMS_PER_CALLER));
        let last = streams.admit(room.clone(), &user("w5@wire.test"), false);
        assert!(last.is_ok(), "the thirty-second watcher is admitted beside eight seated streams: {last:?}");
        assert_eq!(streams.admit(room.clone(), &user("w6@wire.test"), false).unwrap_err(), TOO_MANY_WATCHERS);
    }

    /// **A seated caller is still held to the per-caller figure and to the room's total.** A seat holder's ninth
    /// stream is refused in the caller's sentence; and a room filled to its total — five seated callers at eight and
    /// 24 watchers, since this module knows no seat count and believes the bit it is handed — refuses a sixth
    /// seated caller's FIRST stream in the table's. Mutants: the per-caller arm skipped for a seated caller, or the
    /// total skipped for one.
    #[test]
    fn a_seated_caller_is_still_held_to_the_per_caller_figure_and_to_the_rooms_total() {
        let streams = Arc::new(Streams::default());
        let room = Room::Table("t".to_string());
        let mut held = Vec::new();
        fill(&streams, &room, "s", 1, true, &mut held);
        assert_eq!(streams.admit(room.clone(), &user("s1@wire.test"), true).unwrap_err(), TOO_MANY_STREAMS_CALLER);
        assert_eq!(streams.open(&room), STREAMS_PER_CALLER, "the refusal took no slot");
        drop(held);
        let mut held = Vec::new();
        fill(&streams, &room, "s", 5, true, &mut held);
        fill(&streams, &room, "w", 3, false, &mut held);
        assert_eq!((streams.open(&room), streams.seated(&room)), (STREAMS_PER_TABLE, 5 * STREAMS_PER_CALLER), "premise: the room at its total");
        assert_eq!(streams.admit(room.clone(), &user("s6@wire.test"), true).unwrap_err(), TOO_MANY_STREAMS_TABLE);
        assert_eq!(streams.open(&room), STREAMS_PER_TABLE, "the refusal took no slot");
    }

    /// **The verdicts run the room's total, then the watchers' figure, then the caller's** — each pinned by a
    /// TWO-defect probe, where both of a pair would refuse and only the order says which sentence is read (a
    /// single-defect probe pins nothing about order, CLAUDE.md's lesson): total AND watchers (a fresh watcher in a
    /// full room of 32 seated and 32 watching streams) reads the table's sentence; watchers AND caller (a watcher
    /// at its own eight among 32 watching) reads the watchers'; total AND caller, for a seat holder at its eight in
    /// a full room, reads the table's — and for a watcher at its eight in a room full with 40 seated streams and 24
    /// watching, where the watchers' figure is not reached, the table's too. Every one of the five other orders
    /// fails one of these.
    #[test]
    fn the_verdicts_run_total_then_watchers_then_caller() {
        let streams = Arc::new(Streams::default());
        let room = Room::Table("t".to_string());
        // Watchers AND caller: 32 watching, a room with room to spare.
        let mut held = Vec::new();
        fill(&streams, &room, "w", 4, false, &mut held);
        assert_eq!(streams.admit(room.clone(), &user("w1@wire.test"), false).unwrap_err(), TOO_MANY_WATCHERS, "watchers before caller");
        // Total AND watchers, and total AND caller: 32 seated beside the 32 watching fill the room.
        fill(&streams, &room, "s", 4, true, &mut held);
        assert_eq!((streams.open(&room), streams.seated(&room)), (STREAMS_PER_TABLE, WATCHERS_PER_TABLE), "premise: the room at its total");
        assert_eq!(streams.admit(room.clone(), &user("fresh@wire.test"), false).unwrap_err(), TOO_MANY_STREAMS_TABLE, "total before watchers");
        assert_eq!(streams.admit(room.clone(), &user("s1@wire.test"), true).unwrap_err(), TOO_MANY_STREAMS_TABLE, "total before caller, seated");
        assert_eq!(streams.admit(room.clone(), &user("w1@wire.test"), false).unwrap_err(), TOO_MANY_STREAMS_TABLE, "total before both");
        drop(held);
        // Total AND caller for a watcher, the watchers' figure unreached: 40 seated and 24 watching.
        let mut held = Vec::new();
        fill(&streams, &room, "s", 5, true, &mut held);
        fill(&streams, &room, "w", 3, false, &mut held);
        assert_eq!((streams.open(&room), streams.seated(&room)), (STREAMS_PER_TABLE, 5 * STREAMS_PER_CALLER), "premise: full, 24 watching");
        assert_eq!(streams.admit(room.clone(), &user("w1@wire.test"), false).unwrap_err(), TOO_MANY_STREAMS_TABLE, "total before caller, watching");
    }

    /// **A dropped stream frees its seated slot, and a quiet room leaves nothing in any map** (B8's lesson, the
    /// module doc): one seated slot reads `seated` 1 — and still does when a watcher's slot beside it drops, which
    /// releases no seated count — and its own drop leaves the room at 0 in both counts and every map empty. Mutants:
    /// the seated count never released, or released for every slot. The house has no seats: it ignores the bit — a stream passed `true` is never counted seated —
    /// and its figures are its own, so the watchers' figure never refuses a lobby stream (the house fills to its
    /// total of 256, far past 32, under its own two sentences, whichever bit its callers pass).
    #[test]
    fn a_dropped_stream_frees_its_seated_slot_and_a_quiet_room_leaves_nothing_in_either_map() {
        let streams = Arc::new(Streams::default());
        let room = Room::Table("t".to_string());
        let seat = streams.admit(room.clone(), &user("seat@t.test"), true).expect("a seated stream");
        assert_eq!((streams.open(&room), streams.seated(&room)), (1, 1));
        let watcher = streams.admit(room.clone(), &user("watch@t.test"), false).expect("a watcher's stream");
        assert_eq!((streams.open(&room), streams.seated(&room)), (2, 1));
        drop(watcher);
        assert_eq!((streams.open(&room), streams.seated(&room)), (1, 1), "a watcher's drop releases no seated count");
        drop(seat);
        assert_eq!((streams.open(&room), streams.seated(&room)), (0, 0), "the drop freed the seated slot");
        assert!(streams.is_quiet(), "nothing kept for a quiet room");
        // The house: the bit ignored, its figures unchanged.
        let mut house = Vec::new();
        for _ in 0..HOUSE_STREAMS_PER_CALLER {
            house.push(streams.admit(Room::House, &user("h0@wire.test"), true).expect("a lobby stream"));
        }
        assert_eq!(streams.seated(&Room::House), 0, "the house counts no seated stream, whatever the bit");
        assert_eq!(streams.admit(Room::House, &user("h0@wire.test"), true).unwrap_err(), TOO_MANY_HOUSE_CALLER);
        fill(&streams, &Room::House, "h", HOUSE_STREAMS_TOTAL / HOUSE_STREAMS_PER_CALLER - 1, false, &mut house);
        assert_eq!(streams.open(&Room::House), HOUSE_STREAMS_TOTAL);
        assert_eq!(streams.admit(Room::House, &user("late@wire.test"), false).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        assert_eq!(streams.admit(Room::House, &user("late@wire.test"), true).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        drop(house);
        assert!(streams.is_quiet(), "nothing kept once the lobby is quiet");
    }
}
