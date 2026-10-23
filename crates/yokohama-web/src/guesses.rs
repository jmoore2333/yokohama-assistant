//! The password oracle's throttle (security S3, D2; design §3.3, decision D-7): a failure counter keyed by
//! (table, caller key), [`WRONG_PASSWORDS_PER_WINDOW`] wrong passwords per [`WINDOW_SECS`] seconds.
//!
//! **Inside [`crate::games::Games`], so both hosts carry it**: the NAS binary's one process, and the Worker's Table
//! object, where the object IS the process and the zone's rule answers by IP in front of it. The figures mirror that
//! rule, 20 requests per 10 seconds, so both hosts read alike.
//!
//! **What counts.** A WRONG password counts one; a right one clears the caller's entry; a guess this counter
//! refuses counts nothing, and a full window refuses the right password too, so its compare's result is hidden; a
//! request refused before the compare (the table's status, the seat, an absent or empty password) is not a guess,
//! and never reaches this module. `join` and `unlock` share the counter, because the oracle is the table's password
//! whichever route asks it.
//!
//! **The key is the door's identity for the request**: the verified address under `cf-access` and `worker`, the
//! claimed name under `open`, where a name is free and the counter is per NAME — the open door's recorded
//! limitation, on a network that door trusts by its own rule (`deploy/README.md` §2). No connecting address is read
//! into the layer (D-7).
//!
//! **A window is fixed, from its first failure**: it starts at the first wrong password, a failure inside it never
//! moves that start, and it lapses once [`WINDOW_SECS`] have passed since, whatever happened in it. Every
//! [`Guesses::guess`] prunes every lapsed entry on the way, as `Games::with_table_lock` prunes a lock entry, so the map
//! holds the open windows and never grows with history.
//!
//! **One guess, one acquisition** (P-17, after the task's review): [`Guesses::guess`] prunes, asks, and counts or
//! clears under a single acquisition of the counter's own lock, so the count-then-write is one operation (the design's
//! sentence; the lesson of security S1b). It leans on no other lock: the table's lock is keyed by the REQUEST's
//! spelling of the id, this counter by the STORED one, and a case-folding store gives two spellings two table locks.
//!
//! **No `Retry-After`**: the wait is in the sentence. Neither `ApiError` nor the Worker's reply carries a header
//! channel, and the stream caps' 429 carries none either (D-7).

use std::collections::HashMap;
use std::sync::Mutex;

/// Wrong passwords one caller may send one table in a window — the zone rule's figure.
pub const WRONG_PASSWORDS_PER_WINDOW: u32 = 20;

/// A window's length, in the layer clock's seconds — the zone rule's period.
pub const WINDOW_SECS: u64 = 10;

/// The one sentence a refused guess answers with, as a 429; it names the wait, so no header has to.
pub const TOO_MANY_WRONG_PASSWORDS: &str = "too many wrong passwords for this table — wait ten seconds and try again";

/// What one guess at a table's password comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guess {
    /// The caller's window is full: refused whatever was sent, and counted as nothing.
    Refused,
    /// A wrong password, counted.
    Wrong,
    /// The right password: the caller's entry is gone.
    Right,
}

/// One caller's open window at one table.
#[derive(Debug, Clone, Copy)]
struct Window {
    /// The clock reading of the window's first failure, never moved by a later one.
    started: u64,
    /// Wrong passwords counted in it, never more than [`WRONG_PASSWORDS_PER_WINDOW`].
    failures: u32,
}

impl Window {
    /// Whether `WINDOW_SECS` have passed since the window started. `saturating_sub`, so a reading at the clock's
    /// end cannot overflow and a reading that went backwards keeps the window open rather than lapsing it early.
    fn lapsed(&self, now: u64) -> bool {
        now.saturating_sub(self.started) >= WINDOW_SECS
    }
}

/// The counter: one entry per (table, caller key) with an open window.
#[derive(Debug, Default)]
pub struct Guesses {
    windows: Mutex<HashMap<(String, String), Window>>,
}

impl Guesses {
    /// The map, poisoned or not: nothing below panics while holding it and each step leaves it whole, so a poisoned
    /// map is still a true count, and a throttle that answered every password with a panic after one would be a
    /// lockout.
    fn windows(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Window>> {
        self.windows.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// One guess by `caller` at `table`'s password, whose compare came out `correct`, at `now` — under ONE
    /// acquisition of the counter's lock. Every lapsed entry is pruned first. A window already holding
    /// [`WRONG_PASSWORDS_PER_WINDOW`] failures answers [`Guess::Refused`] whatever `correct` says, and counts nothing.
    /// Otherwise a wrong password starts a window at `now` when the caller has none, or counts one more in the open
    /// window without moving its start, and answers [`Guess::Wrong`]; a right one removes the caller's entry and
    /// answers [`Guess::Right`].
    pub fn guess(&self, table: &str, caller: &str, now: u64, correct: bool) -> Guess {
        let mut windows = self.windows();
        windows.retain(|_, w| !w.lapsed(now));
        let key = (table.to_string(), caller.to_string());
        if windows.get(&key).is_some_and(|w| w.failures >= WRONG_PASSWORDS_PER_WINDOW) {
            return Guess::Refused;
        }
        if correct {
            windows.remove(&key);
            return Guess::Right;
        }
        windows
            .entry(key)
            .and_modify(|w| w.failures = w.failures.saturating_add(1))
            .or_insert(Window { started: now, failures: 1 });
        Guess::Wrong
    }

    /// How many windows are open at `now`, after the same prune [`Self::guess`] makes.
    pub fn live(&self, now: u64) -> usize {
        let mut windows = self.windows();
        windows.retain(|_, w| !w.lapsed(now));
        windows.len()
    }

    /// The failures counted in `caller`'s entry at `table`, read without a prune — a cell's view of what a refusal
    /// did NOT count, which under a fixed window nothing a caller sees can show.
    #[cfg(test)]
    pub(crate) fn failures_of(&self, table: &str, caller: &str) -> Option<u32> {
        self.windows().get(&(table.to_string(), caller.to_string())).map(|w| w.failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The map as it stands, read raw: [`Guesses::live`] prunes for itself, so a
    /// cell about the prune a guess makes reads around it.
    fn raw_len(g: &Guesses) -> usize {
        g.windows.lock().expect("the counter's lock").len()
    }

    /// `n` wrong passwords from `caller` at `table` at `now`, each asserted counted.
    fn wrong(g: &Guesses, table: &str, caller: &str, now: u64, n: u32) {
        for k in 1..=n {
            assert_eq!(g.guess(table, caller, now, false), Guess::Wrong, "wrong password {k} from {caller} at {table}");
        }
    }

    /// **Twenty wrong passwords are each answered; the twenty-first is not,
    /// and neither is the RIGHT one inside the full window** — a full window
    /// hides the compare's result. The figures are pinned at their literals
    /// (the S1 stack review's CB-F5: a cell written relative to a constant
    /// passes a moved constant), and the sentence names the window it states.
    #[test]
    fn the_twenty_first_failure_in_a_window_is_refused_and_the_twentieth_is_not() {
        assert_eq!(WRONG_PASSWORDS_PER_WINDOW, 20, "the figure mirrors the zone rule (design §3.3)");
        assert_eq!(WINDOW_SECS, 10, "the window mirrors the zone rule (design §3.3)");
        assert_eq!(
            TOO_MANY_WRONG_PASSWORDS, "too many wrong passwords for this table \u{2014} wait ten seconds and try again",
            "the sentence names the wait, ten seconds, which is WINDOW_SECS"
        );
        let g = Guesses::default();
        wrong(&g, "t", "guest", 0, 20);
        assert_eq!(g.guess("t", "guest", 0, false), Guess::Refused, "the twenty-first is refused");
        assert_eq!(g.guess("t", "guest", 0, true), Guess::Refused, "and so is the right password inside the full window");
    }

    /// **A right password clears the caller's entry**, and the next wrong one
    /// counts from one: twenty more are answered after nineteen and a clear.
    #[test]
    fn a_clear_resets_the_window_and_the_next_failure_counts_from_one() {
        let g = Guesses::default();
        wrong(&g, "t", "guest", 0, 19);
        assert_eq!(g.guess("t", "guest", 0, true), Guess::Right);
        assert_eq!(raw_len(&g), 0, "a clear removes the entry rather than zeroing it");
        wrong(&g, "t", "guest", 1, 20);
        assert_eq!(g.guess("t", "guest", 1, false), Guess::Refused, "and the twenty-first after it is not");
    }

    /// **One caller's guessing never refuses another caller** at the same table
    /// (design D-7: a per-table bucket is the anti-goal).
    #[test]
    fn two_callers_at_one_table_count_apart() {
        let g = Guesses::default();
        wrong(&g, "t", "guest", 0, 20);
        assert_eq!(g.guess("t", "guest", 0, false), Guess::Refused, "premise: the guest is refused");
        assert_eq!(g.guess("t", "friend", 0, true), Guess::Right, "a second caller's right password is taken");
        assert_eq!(g.guess("t", "other", 0, false), Guess::Wrong, "and a third caller's wrong one counts for them");
    }

    /// **One caller's window at one table says nothing about another table**,
    /// and the key's two halves are a pair, never a joined string: `("a1", "b")`
    /// and `("a", "1b")` are two keys.
    #[test]
    fn one_caller_at_two_tables_counts_apart() {
        let g = Guesses::default();
        wrong(&g, "t1", "guest", 0, 20);
        assert_eq!(g.guess("t1", "guest", 0, false), Guess::Refused, "premise: refused at t1");
        assert_eq!(g.guess("t2", "guest", 0, false), Guess::Wrong, "the same caller at a second table is answered");
        wrong(&g, "a1", "b", 0, 19);
        assert_eq!(g.guess("a1", "b", 0, false), Guess::Wrong, "premise: (a1, b) now full");
        assert_eq!(g.guess("a1", "b", 0, false), Guess::Refused, "premise: refused at (a1, b)");
        assert_eq!(g.guess("a", "1b", 0, false), Guess::Wrong, "(a, 1b) is another key");
    }

    /// **A refused guess counts nothing**: two refusals at t=5 and a right
    /// password refused there too leave the entry at twenty failures and its
    /// window started at 0, so it is still refused at t=9 and lapses at t=10
    /// on schedule, where a wrong password starts a new window counting from
    /// one. Under a fixed window a refusal that was counted would change
    /// nothing a caller sees, so the count is read from the entry itself
    /// (P-14).
    #[test]
    fn a_refused_check_counts_nothing_so_the_window_lapses_on_schedule() {
        let g = Guesses::default();
        wrong(&g, "t", "guest", 0, 20);
        assert_eq!(g.guess("t", "guest", 5, false), Guess::Refused);
        assert_eq!(g.guess("t", "guest", 5, false), Guess::Refused);
        assert_eq!(g.guess("t", "guest", 5, true), Guess::Refused);
        assert_eq!(g.failures_of("t", "guest"), Some(20), "the three refusals counted nothing");
        assert_eq!(g.guess("t", "guest", 9, false), Guess::Refused, "the window is open at started + 9");
        assert_eq!(g.guess("t", "guest", 10, false), Guess::Wrong, "and lapsed at started + 10");
        assert_eq!(g.failures_of("t", "guest"), Some(1), "a new window, counting from one");
    }

    /// **A window starts at its FIRST failure and a failure inside it never
    /// moves that start** (the review's one surviving mutant; P-17): nineteen
    /// failures at t=0 and the twentieth at t=9 fill a window that lapses at
    /// t=10, where a window restarted by each failure would refuse until t=19.
    #[test]
    fn a_failure_inside_an_open_window_never_moves_its_start() {
        let g = Guesses::default();
        wrong(&g, "t", "guest", 0, 19);
        wrong(&g, "t", "guest", 9, 1);
        assert_eq!(g.guess("t", "guest", 9, false), Guess::Refused, "premise: twenty in the window");
        assert_eq!(g.guess("t", "guest", 10, false), Guess::Wrong, "the window started at 0 lapsed at 10");
    }

    /// **Every guess prunes every lapsed entry, whatever its key**, so the map
    /// holds the open windows and never grows with history. Read raw, since
    /// `live` prunes for itself (P-14). The guess under the new key is a right
    /// one, which leaves no entry of its own, so the raw length reads exactly
    /// what the prune removed.
    #[test]
    fn a_lapsed_entry_is_pruned_on_the_next_check_whatever_its_key() {
        let g = Guesses::default();
        wrong(&g, "t", "a", 0, 1);
        wrong(&g, "u", "a", 3, 1);
        assert_eq!(raw_len(&g), 2, "premise: two entries");
        assert_eq!(g.guess("t", "b", 10, true), Guess::Right);
        assert_eq!(raw_len(&g), 1, "the guess under key (t, b) pruned (t, a), lapsed at 10, and kept (u, a)");
        assert_eq!(g.failures_of("u", "a"), Some(1), "the open window is untouched");
        assert_eq!(g.live(13), 0, "and live prunes too");
    }

    /// **The window arithmetic holds at the clock's two ends** (P-13): a window
    /// started at `u64::MAX` neither overflows nor lapses at once, and a reading
    /// that went BACKWARDS keeps a window open rather than lapsing it early.
    #[test]
    fn the_window_arithmetic_neither_overflows_nor_lapses_early_at_the_clocks_ends() {
        let g = Guesses::default();
        wrong(&g, "t", "guest", u64::MAX, 20);
        assert_eq!(g.guess("t", "guest", u64::MAX, false), Guess::Refused, "refused at the clock's end");
        let g = Guesses::default();
        wrong(&g, "t", "guest", 100, 20);
        assert_eq!(g.guess("t", "guest", 50, false), Guess::Refused, "a backwards reading keeps the window");
        assert_eq!(g.live(50), 1);
    }
}
