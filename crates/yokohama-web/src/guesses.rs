//! The password oracle's throttle (security S3, D2; design §3.3, decision D-7): a failure counter keyed by
//! (table, caller key), [`WRONG_PASSWORDS_PER_WINDOW`] wrong passwords per [`WINDOW_SECS`] seconds.
//!
//! **Inside [`crate::games::Games`], so both hosts carry it**: the NAS binary's one process, and the Worker's Table
//! object, where the object IS the process and the zone's rule answers by IP in front of it. The figures mirror that
//! rule, 20 requests per 10 seconds, so both hosts read alike.
//!
//! **What counts.** A WRONG password counts one; a right one clears the caller's entry; a request this counter
//! refuses counts nothing; a request refused before the compare (the table's status, the seat, an absent or empty
//! password) is not a guess, and neither asks nor counts. `join` and `unlock` share the counter, because the oracle
//! is the table's password whichever route asks it.
//!
//! **The key is the door's identity for the request**: the verified address under `cf-access` and `worker`, the
//! claimed name under `open`, where a name is free and the counter is per NAME — the open door's recorded
//! limitation, on a network that door trusts by its own rule (`deploy/README.md` §2). No connecting address is read
//! into the layer (D-7).
//!
//! **A window is fixed, from its first failure**: it lapses once [`WINDOW_SECS`] have passed since it started,
//! whatever happened in it, and every [`Guesses::check`] prunes every lapsed entry on the way, as
//! `Games::with_table_lock` prunes a lock entry, so the map holds the open windows and never grows with history.
//!
//! **One lock per call.** Each of [`Guesses::check`], [`Guesses::failed`] and [`Guesses::cleared`] takes the map's
//! lock once and never across another lock. The check and the count of one guess are two calls, made under the
//! TABLE's lock (`Games::with_table_lock`) with one clock reading, so two guesses at one table are serialized there
//! and a count-then-write cannot interleave (the lesson of security S1b).
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

/// One caller's open window at one table.
#[derive(Debug, Clone, Copy)]
struct Window {
    /// The clock reading of the window's first failure.
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
    /// The map, poisoned or not: each call below is one map operation that leaves it whole, so a poisoned map is
    /// still a true count, and a throttle that answered every password with a panic after one would be a lockout.
    fn windows(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Window>> {
        self.windows.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// May `caller` send `table` a password now? Prunes every lapsed entry first, then refuses with
    /// [`TOO_MANY_WRONG_PASSWORDS`] when the caller's window already holds [`WRONG_PASSWORDS_PER_WINDOW`] failures.
    /// It counts nothing.
    pub fn check(&self, table: &str, caller: &str, now: u64) -> Result<(), &'static str> {
        let mut windows = self.windows();
        windows.retain(|_, w| !w.lapsed(now));
        match windows.get(&(table.to_string(), caller.to_string())) {
            Some(w) if w.failures >= WRONG_PASSWORDS_PER_WINDOW => Err(TOO_MANY_WRONG_PASSWORDS),
            _ => Ok(()),
        }
    }

    /// Count one wrong password: a window starts at `now` when the caller has none, else its count goes up by
    /// one. Called after [`Self::check`] at the same reading, which pruned every lapsed window.
    pub fn failed(&self, table: &str, caller: &str, now: u64) {
        self.windows()
            .entry((table.to_string(), caller.to_string()))
            .and_modify(|w| w.failures = w.failures.saturating_add(1))
            .or_insert(Window { started: now, failures: 1 });
    }

    /// A right password: the caller's entry at `table` goes, so the next wrong one counts from one.
    pub fn cleared(&self, table: &str, caller: &str) {
        self.windows().remove(&(table.to_string(), caller.to_string()));
    }

    /// How many windows are open at `now`, after the same prune [`Self::check`] makes.
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
    /// cell about the prune `check` makes reads around it.
    fn raw_len(g: &Guesses) -> usize {
        g.windows.lock().expect("the counter's lock").len()
    }

    /// **Twenty wrong passwords are each asked; the twenty-first is not.** The
    /// figures are pinned at their literals (the S1 stack review's CB-F5: a
    /// cell written relative to a constant passes a moved constant), and the
    /// sentence names the window it states.
    #[test]
    fn the_twenty_first_failure_in_a_window_is_refused_and_the_twentieth_is_not() {
        assert_eq!(WRONG_PASSWORDS_PER_WINDOW, 20, "the figure mirrors the zone rule (design §3.3)");
        assert_eq!(WINDOW_SECS, 10, "the window mirrors the zone rule (design §3.3)");
        assert_eq!(
            TOO_MANY_WRONG_PASSWORDS, "too many wrong passwords for this table \u{2014} wait ten seconds and try again",
            "the sentence names the wait, ten seconds, which is WINDOW_SECS"
        );
        let g = Guesses::default();
        for n in 1..=20 {
            assert_eq!(g.check("t", "guest", 0), Ok(()), "wrong password {n} is asked");
            g.failed("t", "guest", 0);
        }
        assert_eq!(g.check("t", "guest", 0), Err(TOO_MANY_WRONG_PASSWORDS), "the twenty-first is refused");
    }

    /// **A right password clears the caller's entry**, and the next wrong one
    /// counts from one: twenty more are asked after nineteen and a clear.
    #[test]
    fn a_clear_resets_the_window_and_the_next_failure_counts_from_one() {
        let g = Guesses::default();
        for _ in 0..19 {
            assert_eq!(g.check("t", "guest", 0), Ok(()));
            g.failed("t", "guest", 0);
        }
        g.cleared("t", "guest");
        assert_eq!(raw_len(&g), 0, "a clear removes the entry rather than zeroing it");
        for n in 1..=20 {
            assert_eq!(g.check("t", "guest", 1), Ok(()), "wrong password {n} after the clear is asked");
            g.failed("t", "guest", 1);
        }
        assert_eq!(g.check("t", "guest", 1), Err(TOO_MANY_WRONG_PASSWORDS), "and the twenty-first after it is not");
    }

    /// **One caller's guessing never refuses another caller** at the same table
    /// (design D-7: a per-table bucket is the anti-goal).
    #[test]
    fn two_callers_at_one_table_count_apart() {
        let g = Guesses::default();
        for _ in 0..20 {
            g.failed("t", "guest", 0);
        }
        assert_eq!(g.check("t", "guest", 0), Err(TOO_MANY_WRONG_PASSWORDS), "premise: the guest is refused");
        assert_eq!(g.check("t", "friend", 0), Ok(()), "a second caller at the same table is asked");
    }

    /// **One caller's window at one table says nothing about another table**,
    /// and the key's two halves are a pair, never a joined string: `("a1", "b")`
    /// and `("a", "1b")` are two keys.
    #[test]
    fn one_caller_at_two_tables_counts_apart() {
        let g = Guesses::default();
        for _ in 0..20 {
            g.failed("t1", "guest", 0);
        }
        assert_eq!(g.check("t1", "guest", 0), Err(TOO_MANY_WRONG_PASSWORDS), "premise: refused at t1");
        assert_eq!(g.check("t2", "guest", 0), Ok(()), "the same caller at a second table is asked");
        for _ in 0..20 {
            g.failed("a1", "b", 0);
        }
        assert_eq!(g.check("a1", "b", 0), Err(TOO_MANY_WRONG_PASSWORDS), "premise: refused at (a1, b)");
        assert_eq!(g.check("a", "1b", 0), Ok(()), "(a, 1b) is another key");
    }

    /// **A refused check counts nothing**: two refusals at t=5 leave the entry
    /// at twenty failures and its window started at 0, so it is still refused
    /// at t=9 and lapses at t=10 on schedule. Under a fixed window a refusal
    /// that was counted would change nothing a caller sees, so the count is
    /// read from the entry itself.
    #[test]
    fn a_refused_check_counts_nothing_so_the_window_lapses_on_schedule() {
        let g = Guesses::default();
        for _ in 0..20 {
            assert_eq!(g.check("t", "guest", 0), Ok(()));
            g.failed("t", "guest", 0);
        }
        assert_eq!(g.check("t", "guest", 5), Err(TOO_MANY_WRONG_PASSWORDS));
        assert_eq!(g.check("t", "guest", 5), Err(TOO_MANY_WRONG_PASSWORDS));
        assert_eq!(g.failures_of("t", "guest"), Some(20), "the two refusals counted nothing");
        assert_eq!(g.check("t", "guest", 9), Err(TOO_MANY_WRONG_PASSWORDS), "the window is open at started + 9");
        assert_eq!(g.check("t", "guest", 10), Ok(()), "and lapsed at started + 10");
        assert_eq!(g.live(10), 0, "the lapsed entry is gone");
    }

    /// **Every check prunes every lapsed entry, whatever its key**, so the map
    /// holds the open windows and never grows with history. Read raw, since
    /// `live` prunes for itself.
    #[test]
    fn a_lapsed_entry_is_pruned_on_the_next_check_whatever_its_key() {
        let g = Guesses::default();
        g.failed("t", "a", 0);
        g.failed("u", "a", 3);
        assert_eq!(raw_len(&g), 2, "premise: two entries");
        assert_eq!(g.check("t", "b", 10), Ok(()));
        assert_eq!(raw_len(&g), 1, "the check under key (t, b) pruned (t, a), lapsed at 10, and kept (u, a)");
        assert_eq!(g.failures_of("u", "a"), Some(1), "the open window is untouched");
        assert_eq!(g.live(13), 0, "and live prunes too");
    }

    /// **The window arithmetic holds at the clock's two ends**: a window started
    /// at `u64::MAX` neither overflows nor lapses at once, and a reading that
    /// went BACKWARDS keeps a window open rather than lapsing it early.
    #[test]
    fn the_window_arithmetic_neither_overflows_nor_lapses_early_at_the_clocks_ends() {
        let g = Guesses::default();
        for _ in 0..20 {
            assert_eq!(g.check("t", "guest", u64::MAX), Ok(()));
            g.failed("t", "guest", u64::MAX);
        }
        assert_eq!(g.check("t", "guest", u64::MAX), Err(TOO_MANY_WRONG_PASSWORDS), "refused at the clock's end");
        let g = Guesses::default();
        for _ in 0..20 {
            g.failed("t", "guest", 100);
        }
        assert_eq!(g.check("t", "guest", 50), Err(TOO_MANY_WRONG_PASSWORDS), "a backwards reading keeps the window");
        assert_eq!(g.live(50), 1);
    }
}
