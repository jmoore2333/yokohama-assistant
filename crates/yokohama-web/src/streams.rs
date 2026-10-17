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
//! **The figures mirror the Worker's** so the two hosts read alike: 8 per caller a room, 64 a table. The verdicts in
//! the Worker's order — the room's total first, then the caller's — each a constant sentence and never a caller
//! value. The maps hold an entry only while a stream is open (B8's lesson: a map nobody has written costs nothing).
//!
//! **The per-caller key is the identity the door hands over** — the verified address under Access, the claimed name
//! under the open door, where a name rotation escapes the per-caller figure and meets the room's total alone (the
//! Worker's sibling is `docs/web-lane.md` §15's socket-cap row, S2b's finding D3, and this module's own is the NAS
//! stream-cap row beside it; recorded for S4). The room's total
//! is the bound a process relies on, and it judges a seated player as it judges anyone: a full room refuses them, in
//! the TABLE's sentence that takes them to the lobby (S2c's D-2), until streams close. Neither figure is a budget a
//! feature may lean on; a seat-aware admission is S4's.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::identity::User;

/// Streams one caller may hold open at one table: a table page holds one, the owner's "a couple of games at once"
/// a few; eight leaves room for every tab of one person and refuses a script. The Worker's `SOCKETS_PER_CALLER`.
pub const STREAMS_PER_CALLER: usize = 8;
/// Streams one table may hold open in all: four seats and their watchers at eight each. The Worker's
/// `SOCKETS_PER_TABLE`.
pub const STREAMS_PER_TABLE: usize = 64;
/// Lobby streams one caller may hold open. The Worker's `LOBBY_PER_CALLER`.
pub const HOUSE_STREAMS_PER_CALLER: usize = 8;
/// Lobby streams the process holds open in all: 32 people at eight. A house frame reads nothing, but each lobby it
/// reaches re-reads its three listings on a bump that re-lists, on the one 512-thread blocking pool — 768
/// dispatches at this figure (the Worker's lobby object holds 1,024 because its lobbies' re-reads are requests of
/// their own).
pub const HOUSE_STREAMS_TOTAL: usize = 256;

pub const TOO_MANY_STREAMS_CALLER: &str = "too many open streams from you at this table — close one and try again";
pub const TOO_MANY_STREAMS_TABLE: &str = "this table has too many open streams — try again in a moment";
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
}

impl Streams {
    /// The verdict for one more stream by `caller` in `room`: the room's total first, then the caller's — `Err`
    /// carries the sentence the 429 repeats; `Ok` holds the slot until it is dropped.
    pub fn admit(self: &Arc<Self>, room: Room, caller: &User) -> Result<Slot, &'static str> {
        let (per_caller, total, caller_sentence, total_sentence) = match room {
            Room::Table(_) => (STREAMS_PER_CALLER, STREAMS_PER_TABLE, TOO_MANY_STREAMS_CALLER, TOO_MANY_STREAMS_TABLE),
            Room::House => (HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL, TOO_MANY_HOUSE_CALLER, TOO_MANY_HOUSE_TOTAL),
        };
        let key = (room.clone(), caller.as_str().to_string());
        let mut counts = self.counts.lock().expect("streams lock");
        if counts.by_room.get(&room).copied().unwrap_or(0) >= total {
            return Err(total_sentence);
        }
        if counts.by_caller.get(&key).copied().unwrap_or(0) >= per_caller {
            return Err(caller_sentence);
        }
        *counts.by_room.entry(room.clone()).or_insert(0) += 1;
        *counts.by_caller.entry(key.clone()).or_insert(0) += 1;
        Ok(Slot { streams: Arc::clone(self), room, caller: key.1 })
    }

    /// Open streams in `room` — for the cells and a reading.
    pub fn open(&self, room: &Room) -> usize {
        self.counts.lock().expect("streams lock").by_room.get(room).copied().unwrap_or(0)
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
        f.debug_struct("Slot").field("room", &self.room).finish_non_exhaustive()
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        // Poison-tolerant, in `games::enter`'s form (the plan review's AD-18): nothing under this lock panics today,
        // and an `expect` here would turn a poisoned lock into a panic inside a drop.
        let mut counts = self.streams.counts.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        release(&mut counts.by_room, &self.room);
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
            held.push(streams.admit(table.clone(), &user("red@wire.test")).unwrap_or_else(|e| panic!("stream {n}: {e}")));
        }
        assert_eq!(streams.admit(table.clone(), &user("red@wire.test")).unwrap_err(), TOO_MANY_STREAMS_CALLER);
        let blue = streams.admit(table.clone(), &user("blue@wire.test"));
        assert!(blue.is_ok(), "another caller is unaffected");
        assert_eq!(streams.open(&table), STREAMS_PER_CALLER + 1);
        drop(held.pop());
        assert!(streams.admit(table.clone(), &user("red@wire.test")).is_ok(), "a closed stream frees the slot");
        // The table's total: the c-callers fill the table to 64 beside red's seven and blue's one; the sixty-fifth, a
        // ninth caller's first, is refused with the TABLE's sentence (the total's verdict comes first).
        let mut fill = Vec::new();
        for caller in 1..=8 {
            for _ in 0..STREAMS_PER_CALLER {
                let who = format!("c{caller}@wire.test");
                if let Ok(slot) = streams.admit(table.clone(), &user(&who)) { fill.push(slot); }
            }
        }
        assert_eq!(streams.open(&table), STREAMS_PER_TABLE);
        assert_eq!(streams.admit(table.clone(), &user("ninth@wire.test")).unwrap_err(), TOO_MANY_STREAMS_TABLE);
        // **The ORDER of the two verdicts, where both would fail** (M25's catcher — Task 1's probe: a fresh caller's
        // first stream cannot tell the order, since only one check can fail for it): a caller AT its own figure in a
        // FULL room reads the TABLE's sentence, so the total's verdict comes first.
        assert_eq!(streams.admit(table.clone(), &user("c1@wire.test")).unwrap_err(), TOO_MANY_STREAMS_TABLE);
        // **The per-caller figure is per ROOM** (Task 1's probe P4b: a count keyed by the caller alone survived every
        // cell): c1 at eight here may still open at another table.
        assert!(streams.admit(Room::Table("u".to_string()), &user("c1@wire.test")).is_ok(), "the per-caller figure is per room");
        drop(fill);
        drop(held);
        drop(blue);
        assert_eq!(streams.open(&table), 0, "every slot released");
        assert!(streams.counts.lock().unwrap().by_room.is_empty() && streams.counts.lock().unwrap().by_caller.is_empty(), "nothing kept for a quiet room");
        // The house: its own per-caller figure and its own total sentence.
        let mut house = Vec::new();
        for _ in 0..HOUSE_STREAMS_PER_CALLER { house.push(streams.admit(Room::House, &user("red@wire.test")).unwrap()); }
        assert_eq!(streams.admit(Room::House, &user("red@wire.test")).unwrap_err(), TOO_MANY_HOUSE_CALLER);
        for caller in 1..=31 {
            for _ in 0..HOUSE_STREAMS_PER_CALLER { house.push(streams.admit(Room::House, &user(&format!("h{caller}@wire.test"))).unwrap()); }
        }
        assert_eq!(streams.open(&Room::House), HOUSE_STREAMS_TOTAL);
        assert_eq!(streams.admit(Room::House, &user("late@wire.test")).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        // The order again, in the house: h1 holds eight in the full house and reads the TOTAL's sentence.
        assert_eq!(streams.admit(Room::House, &user("h1@wire.test")).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        assert_eq!((STREAMS_PER_CALLER, STREAMS_PER_TABLE, HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL), (8, 64, 8, 256));
    }
}
