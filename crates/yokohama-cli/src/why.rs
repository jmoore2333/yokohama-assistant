//! `why`: the account of a move a CLI BOT already played (Phase 3.5c W5 task
//! 5, design §2 — gate ingredient 3's CLI reading, and clause (b) of OD-4(i)'s
//! three-clause executable form: *every CLI bot move retrievable via `why`*).
//!
//! **The split with [`crate::suggest`], stated once**: `suggest` answers *what
//! would you play on MY seat*, before you move; `why` answers *how did the BOT
//! choose the move it already played*, after. The two share a renderer and
//! nothing else, and each refusal here points at the other where that is the
//! honest advice.
//!
//! **RECONSTRUCT BY REPLAY, REFUSE ON MISMATCH.** For event index `n` the
//! decision state is `state_at(record, n)` (the state the mover decided IN),
//! the mover is that state's `current_player`, and the decision key is
//! `BotSeats::decision_seed(n)` — the same `(bot_seed, events.len())` keying
//! the pump used when it appended event `n`. `why` then re-runs [`crate::bot::decide`],
//! the LITERAL function the pump ran, and asserts the reconstructed move equals
//! the recorded one. On a mismatch it REFUSES, naming both moves and both
//! version consts.
//!
//! That gate is load-bearing three ways, and none of them is belt-and-braces:
//!
//! 1. It is a per-invocation DETERMINISM check — a keying regression surfaces
//!    as a refusal rather than as a confident, wrong table.
//! 2. It resolves the mid-history-downgrade ambiguity BY CONSTRUCTION. A seat
//!    downgraded to HUMAN mid-session leaves human moves under a bot-named
//!    provenance; no per-event attribution exists (a `RecordedEvent::Move`
//!    carries no seat, and W5 adds no new event kinds), but reconstruction
//!    mismatches on exactly those moves and refuses instead of fabricating an
//!    explanation for a move the bot never chose.
//! 3. It keeps `why` honest ACROSS BUILDS: a record written by an older impl
//!    reconstructs differently, so it refuses naming the versions.
//!
//! **READ-ONLY, by type.** [`why`] takes `&Session` and applies nothing,
//! records nothing and autosaves nothing — the compiler enforces the sandbox,
//! exactly as it does for `suggest`. Two calls at one address are byte-identical
//! because every input (the record, the event index, the derived seed) is a
//! function of the address alone.
//!
//! **Game over is NOT a refusal.** `suggest` refuses a finished game because
//! there is nothing left to advise; `why` explains history, which survives the
//! end — and post-game reading is exactly the checkpoint's use for it.

use yokohama_core::*;

use crate::bot::BotSeats;
use crate::session::{Session, TrackingMode};

/// The `why` screen for `arg` (or for the most recent explainable bot move
/// when `arg` is `None`), or the typed refusal that stands in for it.
#[allow(dead_code)]
pub(crate) fn why(session: &Session, bots: &BotSeats, arg: Option<usize>) -> String {
    why_over(session.mode, session.record(), session.rules(), bots, arg)
}

/// [`why`] over the record directly, which is all it ever reads.
///
/// The seam exists so a refusal row whose fixture the sim driver never writes
/// — a reveal event inside a simulation record — can be driven at all: the
/// arms below are a function of `(mode, record, rules, bots, arg)` and of
/// nothing on a `Session`.
fn why_over(
    mode: TrackingMode,
    record: &GameRecord,
    rules: &Rules,
    bots: &BotSeats,
    arg: Option<usize>,
) -> String {
    let _ = (mode, record, rules, bots, arg);
    "why: not implemented\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::{self, PROVISIONAL_DEFAULT_BOT};
    use crate::repl::Repl;
    use crate::session::Session;
    use std::sync::OnceLock;

    /// The bot stream seed every fixture here keys with. A LITERAL rather than
    /// a draw: `why`'s whole claim is that it re-derives the pump's own key, so
    /// a fixture whose seed moved per run could not tell a reconstruction that
    /// used the right stream from one that used any stream at all.
    const FIXTURE_BOT_SEED: u64 = 0x5A17_3B0C_9E44_D218;

    /// A 2P simulation whose seat 1 is played by `contestant`, built the way
    /// the WIZARD builds one — provenance stamped on the session, `Repl::new`
    /// reading the table back out of it — so every fixture here goes through
    /// the same resume path a save does rather than a test-only constructor.
    fn bot_repl(seed: u64, contestant: Contestant) -> Repl {
        let bots = BotSeats::new(vec![None, Some(contestant)], FIXTURE_BOT_SEED);
        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            seed,
            vec![StartingOrderChoice::Keep(0); 2],
            Rules::standard(),
        )
        .expect("a standard 2-player simulation");
        session.set_run_seats(bots.seat_provenance());
        Repl::new(session)
    }

    /// A 2P simulation with no bot seat and no run provenance at all — the
    /// shape a game started before anyone answered the bot question has.
    fn human_repl(seed: u64) -> Repl {
        Repl::new(
            Session::new_sim(
                GameConfig::for_player_count(2),
                seed,
                vec![StartingOrderChoice::Keep(0); 2],
                Rules::standard(),
            )
            .expect("a standard 2-player simulation"),
        )
    }

    /// A LIVE session, the one mode `why` refuses outright.
    fn live_repl(seed: u64) -> Repl {
        Repl::new(
            Session::new_live(
                GameConfig::for_player_count(2),
                seed,
                SetupObservations::default(),
                vec![StartingOrderChoice::Keep(0); 2],
                Rules::standard(),
            )
            .expect("a standard 2-player live game"),
        )
    }

    /// Play `r` forward until `done` holds (or the game ends), and say how many
    /// steps it took.
    ///
    /// **The BOT half runs the shipped pump** — `pump_bots`, keyed by
    /// `(bot_seed, events.len())` — because that is precisely the decision
    /// `why` claims to reconstruct; a fixture that chose the bot's moves itself
    /// would pin nothing. The HUMAN half takes moves off the ENGINE's own legal
    /// list, ROTATING the index rather than always taking `[0]` (`suggest`'s
    /// `sim_mid_game` idiom, for its reason: a take-the-first line degenerates,
    /// never builds anything and never pays a fee).
    fn play(r: &mut Repl, limit: usize, mut done: impl FnMut(&Repl) -> bool) -> usize {
        for step in 0..limit {
            if r.session.is_over() || done(r) {
                return step;
            }
            if r.bots.is_bot(r.session.state().current_player) {
                r.pump_bots();
            } else {
                let legal = r.session.legal();
                let mv = legal[step % legal.len()].clone();
                r.session.apply(mv, vec![]).expect("a move from the engine's own list applies");
            }
        }
        panic!("the fixture's condition was not reached within {limit} steps");
    }

    /// Every event index whose mover was `seat`.
    ///
    /// An INDEPENDENT forward fold, deliberately: this module's own address
    /// resolution is the thing under test, so a fixture that asked it which
    /// events were the bot's would be checking a claim against itself.
    fn move_indices_of(record: &GameRecord, rules: &Rules, seat: PlayerId) -> Vec<usize> {
        let mut state = state_at(record, 0, rules).expect("the post-setup state");
        let mut out = Vec::new();
        for (i, ev) in record.events.iter().enumerate() {
            if matches!(ev, RecordedEvent::Move(_)) && state.current_player == seat {
                out.push(i);
            }
            state = replay_from(&state, std::slice::from_ref(ev), rules).expect("the record folds");
        }
        out
    }

    /// The move recorded at `index`, in the words the screen names it with.
    fn described_move_at(record: &GameRecord, rules: &Rules, index: usize) -> String {
        let state = state_at(record, index, rules).expect("the decision state");
        let RecordedEvent::Move(mv) = &record.events[index] else { panic!("event {index} is not a move") };
        crate::describe::describe_move(mv, &state, rules)
    }

    /// Whitespace-normalized, so a move description the render REFLOWED across
    /// an indented second line still compares equal to the one-line original
    /// (`wrap_words` splits on whitespace and joins with single spaces, and the
    /// answer wraps rather than eliding — `suggest`'s two treatments).
    fn flat(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The marker a real `why` screen carries and no refusal does.
    const ACCOUNTED: &str = "Score at that point:";

    /// A completed 2P human-vs-bot game, played ONCE for the whole binary.
    ///
    /// Two tests need a finished game (the after-game-over pin and the
    /// every-move pin) and it is the most expensive fixture here — a full game
    /// of turn-boundary search — so it is memoized rather than replayed, the
    /// `policy_diversity` precedent. Only the RECORD and the seat table travel:
    /// `why` reads nothing else, which is itself worth stating.
    struct Completed {
        record: GameRecord,
        bots: BotSeats,
        rules: Rules,
        bot_moves: Vec<usize>,
    }

    fn completed_game() -> &'static Completed {
        static GAME: OnceLock<Completed> = OnceLock::new();
        GAME.get_or_init(|| {
            let mut r = bot_repl(20_260_812, PROVISIONAL_DEFAULT_BOT);
            play(&mut r, 5_000, |_| false);
            assert!(r.session.is_over(), "the fixture must reach a finished game");
            let rules = Rules::standard();
            let record = r.session.record().clone();
            let bot_moves = move_indices_of(&record, &rules, PlayerId::new(1));
            Completed { record, bots: r.bots.clone(), rules, bot_moves }
        })
    }

    // ---- the refusal table (design §2.3), one test per row -----------------

    /// Live tracking has no bot seats at all — a live save that names one drops
    /// it loudly on load — so there is no bot decision here to reconstruct.
    #[test]
    fn why_refuses_in_live_mode() {
        let r = live_repl(21);
        let out = why(&r.session, &r.bots, None);
        assert!(out.contains("simulation-only"), "{out}");
        assert!(out.contains("live"), "the refusal must name the mode it is about: {out}");
        assert!(!out.contains(ACCOUNTED), "a refusal must not print an account: {out}");
        assert_eq!(why(&r.session, &r.bots, Some(0)), out, "the indexed form refuses the same way");
    }

    /// Bare `why` before any bot has moved says so, rather than reaching for
    /// the human's own last move.
    #[test]
    fn why_refuses_with_no_bot_move_yet() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let rules = Rules::standard();
        assert!(
            move_indices_of(r.session.record(), &rules, PlayerId::new(1)).is_empty(),
            "fixture: the bot must not have moved yet",
        );

        let out = why(&r.session, &r.bots, None);
        assert!(out.contains("No bot move"), "{out}");
        assert!(out.contains("suggest"), "and it points at the command that DOES answer: {out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    /// An index past the end names the valid RANGE, and never the underlying
    /// `ReplayError`'s `Display` form (whose "replay failed at event N" prefix
    /// would claim a failure at an event that replayed perfectly — T2 review
    /// NOTE-2, carried to this task).
    #[test]
    fn why_refuses_an_out_of_range_index_naming_the_range() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let n = r.session.record().events.len();

        for asked in [n, n + 1, 9_999] {
            let out = why(&r.session, &r.bots, Some(asked));
            assert!(out.contains(&format!("0-{}", n - 1)), "the valid range, for {asked}: {out}");
            assert!(out.contains(&asked.to_string()), "and what was asked for: {out}");
            assert!(!out.contains("replay failed"), "never the ReplayError Display form: {out}");
            assert!(!out.contains(ACCOUNTED), "{out}");
        }

        // A game with nothing recorded has no range to name at all, which is a
        // different sentence rather than the degenerate `0--1`.
        let fresh = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let out = why(&fresh.session, &fresh.bots, Some(0));
        assert!(out.contains("no events"), "{out}");
        assert!(!out.contains("0-"), "there is no range to print: {out}");
    }

    /// An event that is a REVEAL is not a decision, and the refusal says which
    /// kind of reveal it is.
    ///
    /// The fixture SPLICES the reveal onto a simulation record, because the sim
    /// driver never writes one (reveals are live-mode choreography) while the
    /// record TYPE admits one everywhere. It is spliced at the END and
    /// addressed directly, so `state_at(record, n)` folds `events[..n]` and
    /// never the spliced event itself — which is what lets the starting-order
    /// row (unreplayable in a simulation, where no seat's order is hidden) be
    /// covered on the same footing as the two deck reveals.
    #[test]
    fn why_refuses_a_reveal_index_naming_it() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let rules = Rules::standard();
        let base = r.session.record().clone();
        let top_order = *r.session.state().order_deck.last().expect("a card on the order deck");
        let top_tech = *r.session.state().tech_deck.last().expect("a card on the tech deck");

        for (event, word) in [
            (RecordedEvent::RevealOrders(vec![top_order]), "order"),
            (RecordedEvent::RevealTechs(vec![top_tech]), "technology"),
            (
                RecordedEvent::RevealStartingOrder { player: PlayerId::new(1), card: top_order },
                "starting-order",
            ),
        ] {
            let mut record = base.clone();
            record.events.push(event);
            let n = record.events.len() - 1;
            let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(n));
            assert!(out.contains("reveal"), "{out}");
            assert!(out.contains(word), "the refusal must say WHICH kind: {out}");
            assert!(out.contains(&n.to_string()), "and which event: {out}");
            assert!(!out.contains(ACCOUNTED), "{out}");
        }
    }

    /// A human seat's move is `suggest`'s question, asked before you play it —
    /// the OD-4(i) split made a sentence.
    #[test]
    fn why_points_a_human_move_at_suggest() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 40, |r| r.session.record().events.len() >= 3);
        let out = why(&r.session, &r.bots, Some(0));
        assert!(out.contains("suggest"), "{out}");
        assert!(out.contains("seat 0"), "the refusal names the seat: {out}");
        assert!(!out.contains(ACCOUNTED), "{out}");

        // ...and a record carrying no run provenance at all reads the same way:
        // nothing there was played by a bot.
        let mut plain = human_repl(4);
        play(&mut plain, 40, |r| r.session.record().events.len() >= 3);
        let out = why(&plain.session, &plain.bots, Some(0));
        assert!(out.contains("suggest"), "{out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    /// A rollout RUNG samples its move from a distribution: there is no
    /// valuation to show, which is a different fact from a zero one. The
    /// refusal names the rung, and bare `why` skips rung moves entirely rather
    /// than landing on one and refusing.
    #[test]
    fn why_names_a_rung_seat_as_sampled_not_valued() {
        let rung = Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
        let mut r = bot_repl(4, rung);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        assert!(!played.is_empty(), "fixture: the rung seat must have moved");

        let out = why(&r.session, &r.bots, Some(played[0]));
        assert!(out.contains("family-weighted"), "the rung is named: {out}");
        assert!(out.contains("sampled"), "{out}");
        assert!(!out.contains(ACCOUNTED), "{out}");

        let bare = why(&r.session, &r.bots, None);
        assert!(
            bare.contains("No bot move"),
            "bare `why` targets EXPLAINABLE moves only, so a rung game has none: {bare}",
        );
    }

    /// A reconstruction that disagrees with the record REFUSES, naming both
    /// moves and both version consts, rather than presenting an account of a
    /// move that was never played.
    ///
    /// The fixture is the mid-history-downgrade shape design §2.2 names: a
    /// record whose provenance says `lookahead-v1` over a move a human typed.
    /// The human's move is chosen AGAINST the reconstruction — the test asks
    /// what the search would play and then plays something else — so the pin
    /// cannot pass by a coincidence of the seed.
    #[test]
    fn why_refuses_a_reconstruction_mismatch_naming_both_moves() {
        let bots = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), None], FIXTURE_BOT_SEED);
        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            4,
            vec![StartingOrderChoice::Keep(0); 2],
            Rules::standard(),
        )
        .expect("a 2P simulation");
        let rules = Rules::standard();

        // Walk seat 0 to a branching decision point, then play a move the
        // search does NOT choose at this position's own key.
        let mut n = 0usize;
        loop {
            assert!(n < 40, "no branching seat-0 decision point within 40 moves");
            let legal = session.legal();
            if legal.len() > 1 && session.state().current_player == PlayerId::new(0) {
                break;
            }
            session.apply(legal[0].clone(), vec![]).expect("legal applies");
            n += 1;
        }
        let seat = session.state().current_player;
        let searched = bot::decide(
            PROVISIONAL_DEFAULT_BOT,
            session.state(),
            &rules,
            seat,
            bots.decision_seed(n),
        )
        .expect("the search decides")
        .mv;
        let typed = session
            .legal()
            .into_iter()
            .find(|m| *m != searched)
            .expect("fixture: a branching point offers a second move");
        session.apply(typed.clone(), vec![]).expect("legal applies");
        session.set_run_seats(bots.seat_provenance());

        let out = why(&session, &bots, Some(n));
        assert!(!out.contains(ACCOUNTED), "a mismatch must not print an account: {out}");
        assert!(out.contains(&format!("{searched:?}")), "the reconstructed move: {out}");
        assert!(
            flat(&out).contains(&flat(&described_move_at(session.record(), &rules, n))),
            "and the move the record says was played: {out}",
        );
        let version = PROVISIONAL_DEFAULT_BOT.descriptor().1;
        assert!(out.contains(&format!("v{version}")), "both versions are named: {out}");
    }

    /// A DIAGNOSTIC controller refuses in the diagnostic's own words, not the
    /// human seat's: the seat resumed HUMAN because this driver will not run
    /// that arm, and saying "a human played it" would be false about the
    /// record.
    #[test]
    fn why_refuses_a_diagnostic_controller_in_its_own_words() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let mut record = r.session.record().clone();
        let (controller, version, params) = Contestant::MCTS_V1_DEFAULT.descriptor();
        record.run.as_mut().expect("provenance").seats[1] =
            SeatProvenance { controller, version, params, agent_seed: Some(FIXTURE_BOT_SEED), observation: OBSERVATION_VIEW.to_string() };

        let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(played[0]));
        assert!(out.contains("mcts-v1"), "{out}");
        assert!(out.contains("diagnostic"), "{out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    /// A controller name this build cannot parse refuses naming it AND naming
    /// what the parser said — the same discipline `BotSeats::from_provenance`
    /// follows for the resume warning. `bga-human` is the live case: imported
    /// records carry it on every seat.
    #[test]
    fn why_refuses_an_unknown_controller_naming_it() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let mut record = r.session.record().clone();
        record.run.as_mut().expect("provenance").seats[1].controller = "bga-human".to_string();

        let out = why_over(TrackingMode::Simulation, &record, &rules, &r.bots, Some(played[0]));
        assert!(out.contains("bga-human"), "{out}");
        assert!(out.contains("unknown controller"), "the parser's own words reach the reader: {out}");
        assert!(!out.contains(ACCOUNTED), "{out}");
    }

    // ---- the account itself ------------------------------------------------

    /// Bare `why` accounts for the most recent bot move: the header names the
    /// seat, the contestant's ECHO, the event index and the decision seed the
    /// pump keyed that decision with, and the body carries the explanation
    /// table and the move the record says was played.
    #[test]
    fn why_explains_the_last_bot_move_with_the_contestant_echo_header() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let played = move_indices_of(r.session.record(), &rules, PlayerId::new(1));
        let last = *played.last().expect("fixture: the bot must have moved");

        let out = why(&r.session, &r.bots, None);
        assert!(out.contains(&PROVISIONAL_DEFAULT_BOT.echo()), "the contestant echo: {out}");
        assert!(out.contains(&format!("event {last}")), "the log's own index: {out}");
        assert!(
            out.contains(&format!("{:#018x}", r.bots.decision_seed(last))),
            "the decision seed the pump used: {out}",
        );
        assert!(out.contains("Seat 1"), "{out}");
        assert!(out.contains("reconstructed by replay"), "{out}");
        assert!(out.contains(ACCOUNTED), "{out}");
        assert!(out.contains("TOTAL") && out.contains("victory_points"), "the term table: {out}");
        assert!(
            flat(&out).contains(&flat(&described_move_at(r.session.record(), &rules, last))),
            "and it names the move the record carries: {out}",
        );
        // The same address by number is the same answer.
        assert_eq!(why(&r.session, &r.bots, Some(last)), out);
    }

    /// **Read-only, asserted rather than implied.** Two calls agree byte for
    /// byte; the state, the record and the legal list are where they were; and
    /// an autosave target that a commit WOULD rewrite is left absent.
    #[test]
    fn why_is_byte_identical_on_repeat_and_applies_records_autosaves_nothing() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());

        let path = std::env::temp_dir().join(format!("yokohama_why_autosave_{}.json", std::process::id()));
        std::fs::remove_file(&path).ok();
        r.session.save_path = Some(path.clone());
        let state_before = r.session.state().clone();
        let events_before = r.session.record().events.len();

        let first = why(&r.session, &r.bots, None);
        let second = why(&r.session, &r.bots, None);

        assert_eq!(first, second, "the same address must give the same bytes");
        assert!(first.contains(ACCOUNTED), "a real account, not a refusal: {first}");
        assert_eq!(*r.session.state(), state_before, "`why` must not move the game on");
        assert_eq!(r.session.record().events.len(), events_before, "and must record nothing");
        assert!(!path.exists(), "and must not autosave: {}", path.display());
        std::fs::remove_file(&path).ok();
    }

    /// A finished game still answers: `why` explains history, and history
    /// survives the end (design §2.3 — game over is NOT a refusal).
    #[test]
    fn why_still_answers_after_game_over() {
        let game = completed_game();
        let out = why_over(TrackingMode::Simulation, &game.record, &game.rules, &game.bots, None);
        assert!(out.contains(ACCOUNTED), "a finished game still accounts for its bot moves: {out}");
        assert!(out.contains(&PROVISIONAL_DEFAULT_BOT.echo()), "{out}");
    }

    /// **The every-move pin — OD-4(i) clause (b) made executable**: over a
    /// seeded 2P human-vs-bot game played to completion, EVERY event the bot
    /// made is retrievable via `why`, and each answer names the move the record
    /// carries. Zero refusals.
    ///
    /// Quadratic in the event count BY CONSTRUCTION and said out loud (T2
    /// review NOTE-5): each call replays the record to its own position, so the
    /// sweep is `O(events²)` checked applies plus one search per bot move.
    /// That is fine at CLI scales — a few hundred events — and it is the price
    /// of a substrate that caches nothing, which is design §3's own choice.
    #[test]
    fn why_reconstructs_every_bot_move_of_a_completed_game() {
        let game = completed_game();
        assert!(
            game.bot_moves.len() > 30,
            "anti-vacuity: the fixture must be a whole game, not a handful of moves ({})",
            game.bot_moves.len(),
        );
        for &i in &game.bot_moves {
            let out = why_over(TrackingMode::Simulation, &game.record, &game.rules, &game.bots, Some(i));
            assert!(out.contains(ACCOUNTED), "event {i} was refused:\n{out}");
            assert!(
                flat(&out).contains(&flat(&described_move_at(&game.record, &game.rules, i))),
                "event {i}'s account must name the move the record carries:\n{out}",
            );
        }
    }

    /// **Greedy renders the LEANER block.** A one-ply argmax searches no line
    /// and reports no per-candidate vector, so the two blocks a search fills
    /// are replaced by one sentence saying why — never by "no runner-up: this
    /// decision point offered exactly one move", which would be false about a
    /// branching position.
    #[test]
    fn a_greedy_mover_renders_the_leaner_block() {
        let mut r = bot_repl(4, Contestant::GREEDY_V1_DEFAULT);
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());

        let out = why(&r.session, &r.bots, None);
        assert!(out.contains(ACCOUNTED), "{out}");
        assert!(out.contains(&Contestant::GREEDY_V1_DEFAULT.echo()), "{out}");
        assert!(out.contains("one-ply"), "the leaner block says WHY it is leaner: {out}");
        assert!(!out.contains("The line it searched"), "{out}");
        assert!(!out.contains("No line to show"), "{out}");
        assert!(!out.contains("Runner-ups ("), "{out}");
        assert!(!out.contains("No runner-up"), "a branching point DID offer others: {out}");
    }

    // ---- the command ------------------------------------------------------

    /// `why` and `why <n>` reach the same body through the REPL, `why` on a
    /// non-number says so, and the command is in `help`.
    #[test]
    fn the_repl_dispatches_both_forms_of_why() {
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        play(&mut r, 200, |r| !move_indices_of(r.session.record(), &Rules::standard(), PlayerId::new(1)).is_empty());
        let last = *move_indices_of(r.session.record(), &rules, PlayerId::new(1)).last().expect("a bot move");

        let bare = r.step("why");
        assert!(bare.text.contains(ACCOUNTED), "{}", bare.text);
        assert!(!bare.quit);
        let numbered = r.step(&format!("why {last}"));
        assert_eq!(numbered.text, bare.text, "the two forms address the same event here");

        let bad = r.step("why banana");
        assert!(bad.text.contains("banana"), "{}", bad.text);
        assert!(!bad.text.contains(ACCOUNTED), "{}", bad.text);

        assert!(
            crate::repl::HELP_TEXT.lines().any(|l| l.split_whitespace().next() == Some("why")),
            "`why` must be listed in the in-app help",
        );
    }
}
