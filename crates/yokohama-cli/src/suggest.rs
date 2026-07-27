//! `suggest`: the Phase-3.3 evaluator's reading of YOUR OWN position
//! (spec `docs/plans/2026-07-26-phase3.3-evaluator-search-spec.md` §7).
//!
//! **SIMULATION ONLY, and for the HUMAN seat that is to move.** Both halves
//! matter and neither is decoration:
//!
//! * *Sim only* is 3.2's CLI scope held, not re-argued. A live-tracked game's
//!   hidden model is a FICTION the driver reconciles against the physical table
//!   as cards turn face up — the seeded deck order is not the deck on the table
//!   — so a search over worlds sampled from that model would be searching a
//!   position nobody is playing, and would say so in the confident voice of an
//!   evaluation. [`crate::bot`] refuses live seats for the neighbouring
//!   structural reason (`observe` cannot take a live `Unknown` seat's view).
//! * *The human's own seat* is what makes this honest by construction rather
//!   than by promise. The chain is `observe(state, seat)` -> `BeliefSampler`
//!   -> `LookaheadAgent::choose`, and the view it starts from is the view the
//!   person at the keyboard is already looking at. There is no masking question
//!   here of the kind a bot seat raises: the suggestion cannot show you anything
//!   you were not entitled to see, because everything it reads is in your own
//!   view.
//!
//! **DETERMINISTIC-REPEATABLE (spec §7).** A second press at the same state
//! returns the same move and the same explanation, byte for byte. Two decisions
//! buy that together:
//!
//! 1. The agent is built FRESH for each press and thrown away after it (see
//!    [`suggest`]) — nothing about a suggestion survives into the session, so
//!    pressing it cannot change the game, and `undo`/save/reload cannot leave a
//!    half-consumed RNG behind.
//! 2. Its stream is keyed by [`stream_seed`] from `(record identity, seat, event
//!    index)` — three PUBLIC quantities, all of which a resumed save carries —
//!    so "the same state" and "the same seed" are the same condition. A
//!    long-lived agent would give the opposite behaviour: the second press would
//!    draw further down the stream and could answer differently at an unchanged
//!    position, which reads as the tool changing its mind.
//!
//! **What the explanation IS** (spec §0.3): the `eval::explain` output of the
//! state the search VALUED, rendered. Nothing here re-derives a score, re-ranks
//! a candidate, or re-implements the evaluator — the runner-up margins come
//! from [`LookaheadAgent::last_roots`] for exactly that reason, since a second
//! independently-keyed search could rank a different move first and there would
//! then be no fact of the matter about what the bot said.

use yokohama_core::eval::Explanation;
use yokohama_core::*;

use crate::style::{paint, Tone};
use crate::{describe, render};

/// Domain-separation salt for [`stream_seed`]. A named constant so the CLI's
/// suggest stream can never collide with `bot.rs`'s decision stream or the
/// arena's play streams even at an identical `(seed, seat, count)` triple.
const SUGGEST_SALT: u64 = 0x5966_2026_0727_0001;

/// Runner-ups shown below the suggestion (spec §7's "top-k", k small).
const RUNNER_UPS_SHOWN: usize = 3;

/// Longest move description a LIST row prints before it is elided. The
/// suggested move itself is never truncated — it is the answer — but a
/// continuation step or a runner-up is context, and a frame is composed to its
/// widest line (`screen::wrap`), so an unbounded row here would push the whole
/// game screen sideways on a real terminal.
const ROW_TEXT_WIDTH: usize = 80;

/// The closing honesty note (spec §7: the explanation text states term
/// contributions, it does not give play advice). Pre-wrapped as fixed lines
/// rather than reflowed at render time: std exposes no terminal width, every
/// other width in this driver is a fixed budget for the same reason, and a
/// hand-wrapped constant is one fewer thing that can silently exceed one.
const HONESTY_NOTE: [&str; 3] = [
    "This is the bot's arithmetic, not advice: each row is (your value − the best",
    "opponent's) × the weight for this stage, summed. It read ONE guess at the hidden",
    "cards, and its weights are hand-set rather than tuned.",
];

/// Why there is no suggestion.
///
/// Split by CAUSE rather than collapsed to one string because the two mean
/// different things to the person reading them: a refused view is the driver
/// declining to work from something it cannot honestly read, while a search
/// error is the bot finding nothing at a position that is still perfectly
/// playable by hand.
#[derive(Debug)]
pub enum SuggestError {
    /// `observe` refused the seat's view.
    View(String),
    /// The agent refused or dead-ended (a belief-sampler refusal, or no
    /// candidate at this decision point).
    Search(String),
}

impl std::fmt::Display for SuggestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::View(m) => write!(f, "the seat's own view could not be taken: {m}"),
            Self::Search(m) => write!(f, "{m}"),
        }
    }
}

/// One press's answer: everything the render needs, and nothing the session
/// keeps.
pub struct Suggestion {
    /// The seat this was computed for — always the seat to move.
    pub seat: PlayerId,
    /// The move the agent chose.
    pub mv: Move,
    /// `eval::explain` at the hand-off state the search valued the move at
    /// (spec §0.3(b)) — NOT at its one-ply successor, which is a different
    /// state and a different number.
    pub explanation: Explanation,
    /// The principal continuation, root move first: the line the search read
    /// that value at.
    pub continuation: Vec<Move>,
    /// Every root candidate with its folded value, SORTED best first. The sort
    /// is stable, so candidates that tie keep the order the candidate scheme
    /// offered them in.
    pub ranking: Vec<lookahead::RootScore>,
    /// The stream this press was keyed by. Printed, so a reader can tell two
    /// presses apart when the position HAS moved.
    pub stream_seed: u64,
    /// `MAX_TURN_PLIES` stopped the continuation, so the line (and hence the
    /// explanation) was taken MID-TURN rather than at a hand-off. Expected
    /// false; reported because nothing in the line itself marks it.
    pub ply_capped: bool,
}

impl Suggestion {
    /// The suggestion's own value — the maximum of the ranking.
    pub fn best_value(&self) -> i64 {
        self.ranking.first().map_or(self.explanation.total, |r| r.value)
    }

    /// How many candidates tie with the suggestion at the top.
    pub fn tied_at_top(&self) -> usize {
        let best = self.best_value();
        self.ranking.iter().filter(|r| r.value == best).count()
    }
}

/// The agent stream for `seat`'s decision at the record's CURRENT length
/// (spec §7's `(game record identity, seat, event index)` keying).
///
/// **Record identity** is the tuple that determines the game a record replays
/// to: its seed, its player count and start seat, and each seat's kept starting
/// order. Seat colours are deliberately excluded — they are presentation, and
/// two records differing only in colour are the same position. **Event index**
/// is `record.events.len()`, the public counter `bot.rs` already keys its own
/// per-decision stream with; it advances on every applied move, so a press
/// before and after a move key differently, and `undo` walks it back to the
/// value that produced the earlier answer.
///
/// Deriving an agent stream from the world seed is the arena's own shape
/// (`game::arena::agent_stream_seed` mixes `WorldSeed` with seat and slot), not
/// a shortcut taken here: what the SEED HYGIENE rule forbids is deriving a
/// sampler seed from state the agent may not see, and in sim mode the seed is
/// the operator's own — the wizard printed it and the summary shows it.
pub fn stream_seed(record: &GameRecord, seat: PlayerId) -> u64 {
    let mut components: Vec<u64> = Vec::with_capacity(record.choices.len() + 5);
    components.push(record.seed);
    components.push(u64::from(record.config.player_count));
    components.push(u64::from(record.config.start_player.0));
    for choice in &record.choices {
        // `Keep(k)` maps to `1 + k` so it can never collide with `Unknown`'s 0.
        components.push(match choice {
            StartingOrderChoice::Keep(k) => 1 + u64::from(*k),
            StartingOrderChoice::Unknown => 0,
        });
    }
    components.push(u64::from(seat.0));
    components.push(record.events.len() as u64);

    let mut acc = SUGGEST_SALT;
    for component in components {
        let mut step = acc ^ component;
        acc = crate::bot::splitmix64(&mut step);
    }
    acc
}

/// Ask Lookahead-v1 what it would play for `seat`, from `seat`'s own view.
///
/// **Nothing here touches the session.** `state` is read to take one view and
/// nothing else; the agent is a local built from `stream_seed` and dropped when
/// this returns. That is the whole of the sandbox property: there is no place
/// for a suggestion to leave a mark, so `record.events`, the state, and every
/// autosave are untouched by construction rather than by care.
///
/// The knobs are the shipped rung's, taken from `LookaheadAgent::new`'s own
/// defaults rather than restated here: S1 turn-boundary at `B` = 4, `K` = 1,
/// `M` = 8, `WeightTable::V1`.
pub fn suggest(
    state: &GameState,
    rules: &Rules,
    seat: PlayerId,
    stream_seed: u64,
) -> Result<Suggestion, SuggestError> {
    let view = observe(state, seat).map_err(|e| SuggestError::View(e.to_string()))?;
    let mut agent = lookahead::LookaheadAgent::new(rules, stream_seed);
    let report = agent.choose(&view, rules).map_err(SuggestError::Search)?;

    // `choose` cannot return without explaining (spec §0.3(b)), so a missing
    // explanation is a broken contract rather than a state to render around.
    let explanation = agent.last_explanation().cloned().ok_or_else(|| {
        SuggestError::Search("the agent returned a move it did not explain".to_string())
    })?;
    let mut ranking = agent.last_roots().to_vec();
    // Stable, so tied candidates keep the candidate scheme's own order and two
    // presses at one position cannot disagree about which tie is printed first.
    ranking.sort_by(|a, b| b.value.cmp(&a.value));

    Ok(Suggestion {
        seat,
        mv: report.mv,
        explanation,
        continuation: agent.last_continuation().to_vec(),
        ranking,
        stream_seed,
        ply_capped: agent.last_stats().is_some_and(|s| s.ply_cap_hits > 0),
    })
}

/// The whole `suggest` screen.
///
/// Plain text throughout except for three whole-line paints (the seat line, the
/// suggested move, the closing note); the aligned table is deliberately
/// unpainted, since escapes carry zero display width and every column here is
/// padded (`style`'s alignment rule — pad from the PLAIN text). Piped output is
/// therefore byte-identical to a terminal's minus the escapes, and `NO_COLOR`
/// is honoured by `paint` itself.
pub fn render_suggestion(s: &Suggestion, state: &GameState, rules: &Rules) -> String {
    let mut out = render::section("Suggest");

    let color = state.player(s.seat).color;
    // Every knob printed comes from the const the agent itself reads, so this
    // line cannot describe a rung the press did not run. `eval v{}` is
    // `EVAL_IMPL_VERSION` — the evaluator IMPLEMENTATION, which is what a
    // reader needs to know two suggestions are comparable; the weight table's
    // own identity is a 64-bit digest and belongs in provenance, not on screen.
    out.push_str(&format!(
        "{} · {} · beam {}, K={}, M={}, eval v{} · stream {:#018x}\n",
        paint(Tone::Player(color), &format!("Seat {} ({})", s.seat.0, render::color_name(color))),
        lookahead::LOOKAHEAD_CONTROLLER,
        lookahead::DEFAULT_BEAM,
        lookahead::DEFAULT_WORLDS,
        greedy::DEFAULT_RETREAT_SAMPLES,
        eval::EVAL_IMPL_VERSION,
        s.stream_seed,
    ));
    out.push_str("Suggested move:\n");
    out.push_str(&format!(
        "  {}\n",
        paint(Tone::Prompt, &describe::describe_move(&s.mv, state, rules))
    ));

    // -- the line the value was read at ------------------------------------
    let (line, drifted) = describe_line(&s.continuation, state, rules);
    if line.is_empty() {
        out.push_str("\nNo line to show: the score below was read at this position itself.\n");
    } else {
        out.push_str(&format!(
            "\nThe line it searched, {} (the move above is step 1):\n",
            if s.ply_capped {
                "cut short by the search's own ply guard".to_string()
            } else {
                format!("{} move(s) to the hand-over", line.len())
            },
        ));
        for row in &line {
            out.push_str(row);
            out.push('\n');
        }
        if drifted {
            out.push_str(
                "  (the line was searched inside ONE guess at the hidden cards; the steps after \
                 the point it stops matching your real position are named against the last \
                 position that did)\n",
            );
        }
    }

    // -- the explanation ----------------------------------------------------
    out.push_str(&format!(
        "\nScore at that point: {}   ·   stage {}/1000000 ppm (0 = opening, 1000000 = end trigger)\n",
        s.explanation.total, s.explanation.progress_ppm,
    ));
    out.push_str(&format!(
        "  {:<26}{:>8}{:>11}{:>9}{:>12}\n",
        "feature", "you", "best opp", "weight", "product"
    ));
    for term in &s.explanation.terms {
        out.push_str(&format!(
            "  {:<26}{:>8}{:>11}{:>9}{:>12}\n",
            term.feature.name(),
            term.viewer_value,
            term.opponent_value,
            term.weight,
            term.product,
        ));
    }
    if let Some(t) = &s.explanation.terminal {
        // On a finished game the positional vector does not govern at all
        // (§2.6), so every weight above reads 0 and the standing IS the score.
        out.push_str(&format!(
            "  {:<26}{:>8}{:>11}{:>9}{:>12}\n",
            "final standing",
            format!("#{} of {}", t.rank, t.seats),
            t.viewer_total,
            "",
            t.value,
        ));
        out.push_str(&format!(
            "  (game over: place {} of {}, your re-derived final {} against the best other {})\n",
            t.rank, t.seats, t.viewer_total, t.best_other_total,
        ));
    }
    out.push_str(&format!(
        "  {:<26}{:>8}{:>11}{:>9}{:>12}\n",
        "TOTAL", "", "", "", s.explanation.total
    ));

    // -- runner-ups ---------------------------------------------------------
    let best = s.best_value();
    let others: Vec<&lookahead::RootScore> =
        s.ranking.iter().filter(|r| r.mv != s.mv).take(RUNNER_UPS_SHOWN).collect();
    if others.is_empty() {
        out.push_str("\nNo runner-up: this decision point offered exactly one move.\n");
    } else {
        out.push_str(&format!(
            "\nRunner-ups (margin against the suggestion, so 0 means tied) — {} candidate(s) ranked:\n",
            s.ranking.len(),
        ));
        for r in others {
            out.push_str(&format!(
                "  {:>10}  {}\n",
                r.value - best,
                elide(&describe::describe_move(&r.mv, state, rules)),
            ));
        }
    }
    let tied = s.tied_at_top();
    if tied > 1 {
        out.push_str(&format!(
            "  {tied} candidates tie at the top; which one you see is this position's own\n  \
             keyed draw, so a second press will not change it.\n",
        ));
    }

    // Painted line by line, never as one block: `paint` wraps whatever it is
    // given, so a trailing newline INSIDE the argument would put the reset
    // escape on a line of its own and add a row to the frame.
    for note in HONESTY_NOTE {
        out.push_str(&paint(Tone::Dim, note));
        out.push('\n');
    }
    out
}

/// Name each step of a continuation against the position it would be played
/// from, folding the TRUE state forward with the CHECKED `apply_move`.
///
/// The line was searched inside one sampled world, so it need not stay legal in
/// the real position — a step that depends on determinized deck content is the
/// obvious case. Rather than describe every step against the opening state
/// (which mis-names movement fee recipients and printed prices as soon as the
/// line moves anything), the fold stops at the first step the real position
/// refuses and the remaining steps are named against the last position that
/// held. The bool says whether that happened, so the caller can print the
/// caveat instead of quietly implying the line is playable as written.
fn describe_line(line: &[Move], state: &GameState, rules: &Rules) -> (Vec<String>, bool) {
    let mut at = state.clone();
    let mut drifted = false;
    let mut rows = Vec::with_capacity(line.len());
    for (i, mv) in line.iter().enumerate() {
        rows.push(format!("  {}. {}", i + 1, elide(&describe::describe_move(mv, &at, rules))));
        if !drifted {
            match apply_move(&at, mv, rules) {
                Ok(next) => at = next,
                Err(_) => drifted = true,
            }
        }
    }
    (rows, drifted)
}

/// Trim a list row's move text to [`ROW_TEXT_WIDTH`] display columns.
///
/// Counts CHARS, which is the same measurement `screen::visible_width` makes;
/// the strings passed here are `describe_move` output, which carries no ANSI.
fn elide(text: &str) -> String {
    if text.chars().count() <= ROW_TEXT_WIDTH {
        return text.to_string();
    }
    text.chars().take(ROW_TEXT_WIDTH - 1).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use yokohama_core::eval::TerminalStanding;

    /// A 2P simulation at a fixed seed, driven to a decision point with a real
    /// choice. Returns the session so a test can read record AND state off the
    /// same object the REPL would.
    fn sim_at_a_branching_point(seed: u64) -> Session {
        let mut session = Session::new_sim(
            GameConfig::for_player_count(2),
            seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .expect("2P sim");
        for _ in 0..40 {
            let legal = session.legal();
            if legal.len() > 1 {
                return session;
            }
            session.apply(legal[0].clone(), vec![]).expect("apply");
        }
        panic!("no branching decision point within 40 moves of setup");
    }

    fn suggestion_for(session: &Session) -> Suggestion {
        let seat = session.state().current_player;
        let seed = stream_seed(session.record(), seat);
        suggest(session.state(), session.rules(), seat, seed).expect("a suggestion")
    }

    /// The key is a pure function of `(record identity, seat, event index)`:
    /// the same triple always re-derives the same stream (what makes a second
    /// press agree), and each component genuinely moves it (what stops that
    /// from being true of a constant).
    #[test]
    fn the_stream_key_is_pure_and_every_component_moves_it() {
        let rules = Rules::standard();
        let base = GameRecord::new(
            7,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            &rules,
        );
        let seat = PlayerId::new(0);
        assert_eq!(stream_seed(&base, seat), stream_seed(&base, seat), "pure");
        assert_ne!(stream_seed(&base, seat), stream_seed(&base, PlayerId::new(1)), "seat keys it");

        let other_seed = GameRecord::new(
            8,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            &rules,
        );
        assert_ne!(stream_seed(&base, seat), stream_seed(&other_seed, seat), "the game seed keys it");

        let other_keep = GameRecord::new(
            7,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(1), StartingOrderChoice::Keep(0)],
            &rules,
        );
        assert_ne!(stream_seed(&base, seat), stream_seed(&other_keep, seat), "the Keep keys it");

        let mut advanced = base.clone();
        advanced.events.push(RecordedEvent::Move(Move::EndAdditionalPhase));
        assert_ne!(
            stream_seed(&base, seat),
            stream_seed(&advanced, seat),
            "the event index keys it -- otherwise every press of a whole game shares one draw",
        );
    }

    /// The sandbox property, at the level below the REPL: computing a
    /// suggestion leaves the state and the record exactly as it found them.
    #[test]
    fn computing_a_suggestion_changes_nothing() {
        let session = sim_at_a_branching_point(4);
        let state_before = session.state().clone();
        let events_before = session.record().events.len();
        let s = suggestion_for(&session);
        assert_eq!(*session.state(), state_before, "a suggestion must not move the game on");
        assert_eq!(session.record().events.len(), events_before);
        assert!(
            session.legal().contains(&s.mv),
            "and it must suggest a move the ENGINE offers: {:?}",
            s.mv,
        );
    }

    /// Two presses at one position agree byte for byte -- spec §7's
    /// deterministic-repeatable clause, at the computation level (the REPL's own
    /// pin drives the same property through `step`).
    #[test]
    fn two_presses_at_one_position_agree() {
        let session = sim_at_a_branching_point(4);
        let a = suggestion_for(&session);
        let b = suggestion_for(&session);
        assert_eq!(a.mv, b.mv);
        assert_eq!(a.explanation, b.explanation);
        assert_eq!(a.continuation, b.continuation);
        assert_eq!(a.ranking, b.ranking);
        assert_eq!(
            render_suggestion(&a, session.state(), session.rules()),
            render_suggestion(&b, session.state(), session.rules()),
        );
    }

    /// **The terminal-standing line.** A search whose hand-off is a FINISHED
    /// game explains with a standing instead of a positional total (spec §2.6,
    /// and §0.3(a)'s amended identity `total == Σterms + standing`), and the
    /// render must SAY the game ended there — otherwise the screen is sixteen
    /// zero-weight rows under a number with no visible source.
    ///
    /// Doctored rather than driven, and deliberately: the property under test
    /// belongs to the RENDER, while reaching a terminal hand-off organically
    /// takes most of a game (the spec records 607 terminal scorings in 5M).
    /// Everything else about the fixture is a genuine suggestion.
    #[test]
    fn a_terminal_hand_off_prints_its_standing() {
        let session = sim_at_a_branching_point(4);
        let mut s = suggestion_for(&session);
        let standing = TerminalStanding {
            rank: 1,
            seats: 2,
            viewer_total: 61,
            best_other_total: 54,
            value: 1_007_000_000,
        };
        // Exactly the shape `eval::explain` produces on a terminal view: every
        // positional weight and product zeroed (no interpolated weight was
        // used), the standing carrying the whole total.
        for term in &mut s.explanation.terms {
            term.weight = 0;
            term.product = 0;
        }
        s.explanation.terminal = Some(standing);
        s.explanation.total = standing.value;

        let text = render_suggestion(&s, session.state(), session.rules());
        assert!(text.contains("final standing"), "the standing needs its own row:\n{text}");
        assert!(text.contains("game over: place 1 of 2"), "and its reading in words:\n{text}");
        assert!(text.contains("61") && text.contains("54"), "with both totals: \n{text}");
        assert!(text.contains(&standing.value.to_string()), "{text}");
    }

    /// The two degenerate shapes the render has to survive without printing a
    /// misleading empty block: a decision point offering exactly ONE move (no
    /// runner-up exists, so "no runner-up" is the honest line rather than an
    /// empty list) and an empty principal continuation (the value was read at
    /// the root itself — unreachable through the shipped search, whose trace
    /// always carries the root move, and rendered rather than left to print an
    /// empty heading).
    #[test]
    fn the_one_candidate_shape_says_so_rather_than_printing_an_empty_block() {
        let session = sim_at_a_branching_point(4);
        let mut s = suggestion_for(&session);
        s.ranking.retain(|r| r.mv == s.mv);
        s.continuation.clear();
        assert_eq!(s.ranking.len(), 1, "fixture: the chosen move must survive the filter");

        let text = render_suggestion(&s, session.state(), session.rules());
        assert!(text.contains("No runner-up"), "{text}");
        assert!(text.contains("No line to show"), "{text}");
        assert!(!text.contains("Runner-ups ("), "the empty list must not print its header:\n{text}");
    }

    /// A ply-capped line was taken MID-TURN, so the screen must not describe it
    /// as reaching the hand-over -- the one case where the line's own shape says
    /// nothing (`LookaheadAgent::last_continuation`'s caveat).
    #[test]
    fn a_ply_capped_line_is_labelled_as_cut_short() {
        let session = sim_at_a_branching_point(4);
        let mut s = suggestion_for(&session);
        assert!(!s.continuation.is_empty(), "fixture: there must be a line to label");
        assert!(!s.ply_capped, "fixture: a real run does not hit the guard");
        let honest = render_suggestion(&s, session.state(), session.rules());
        assert!(honest.contains("to the hand-over"), "{honest}");

        s.ply_capped = true;
        let capped = render_suggestion(&s, session.state(), session.rules());
        assert!(capped.contains("cut short"), "{capped}");
        assert!(!capped.contains("to the hand-over"), "{capped}");
    }

    /// The rendered shape (spec §7's "ordered term contributions ... + total,
    /// principal continuation ... + top-k runner-ups"): every feature gets a
    /// row in `Feature::ALL` order, the total closes the table, the margins are
    /// non-positive against a suggestion that is by construction the maximum,
    /// and the closing note says what the numbers are.
    #[test]
    fn the_render_carries_every_term_the_total_and_the_margins() {
        let session = sim_at_a_branching_point(4);
        let s = suggestion_for(&session);
        let text = render_suggestion(&s, session.state(), session.rules());

        assert!(text.starts_with("─ Suggest "), "the shared view header: {:?}", text.lines().next());
        assert!(text.contains(lookahead::LOOKAHEAD_CONTROLLER), "{text}");
        assert!(text.contains("Suggested move:"), "{text}");

        // Every feature, in the canonical order, exactly once.
        let mut cursor = 0usize;
        for feature in eval::Feature::ALL {
            let at = text[cursor..]
                .find(feature.name())
                .unwrap_or_else(|| panic!("no row for {} in:\n{text}", feature.name()));
            cursor += at + feature.name().len();
        }
        assert!(text.contains("TOTAL"), "{text}");
        assert!(text.contains(&s.explanation.total.to_string()), "{text}");
        assert!(text.contains("not advice"), "the honesty note must survive edits: {text}");

        // Margins: every runner-up sits at or below the suggestion.
        let best = s.best_value();
        assert!(s.ranking.iter().all(|r| r.value <= best));
        assert_eq!(
            s.ranking.iter().find(|r| r.mv == s.mv).map(|r| r.value),
            Some(best),
            "the suggestion must BE the maximum, or the printed margins have the wrong sign",
        );
    }

    /// Piped output is plain: nothing in the render emits an escape while color
    /// is off (the at-rest default, and what every pipe gets), and no line is
    /// wide enough to widen a composed frame -- the narrowest of the three is
    /// 108 columns at 4 players.
    #[test]
    fn the_render_is_plain_and_stays_inside_the_frame_widths() {
        let session = sim_at_a_branching_point(4);
        let s = suggestion_for(&session);
        let text = render_suggestion(&s, session.state(), session.rules());
        assert!(!text.contains('\u{1b}'), "piped output must carry no ANSI:\n{text}");
        for line in text.lines() {
            assert!(
                crate::screen::visible_width(line) <= 100,
                "line is {} columns, past the 100 the frame budget allows: {line:?}",
                crate::screen::visible_width(line),
            );
        }
    }

    /// The same render with color FORCED on paints only whole lines, so every
    /// line's VISIBLE width is unchanged -- the alignment rule `style` states,
    /// made executable here because the term table is the crate's most
    /// column-sensitive surface.
    #[test]
    fn painting_never_moves_a_column() {
        let session = sim_at_a_branching_point(4);
        let s = suggestion_for(&session);
        let plain = render_suggestion(&s, session.state(), session.rules());
        crate::style::force(true);
        let painted = render_suggestion(&s, session.state(), session.rules());
        crate::style::force(false);
        assert!(painted.contains('\u{1b}'), "the fixture must actually paint something");
        assert_eq!(
            plain.lines().map(crate::screen::visible_width).collect::<Vec<_>>(),
            painted.lines().map(crate::screen::visible_width).collect::<Vec<_>>(),
        );
    }

    /// **The tied-optimal note** (spec §7: "the explanation notes tied-optimal
    /// alternatives via the already-rendered runner-up margins"). A runner-up at
    /// margin 0 is not a near miss — it is an equally-valued move the keyed tie
    /// draw did not pick — so the screen has to print BOTH the `0` margin and the
    /// sentence that says what a `0` means, or the reader is left to infer that
    /// the suggestion was strictly best.
    ///
    /// **Driven, not doctored** — and it turns out ties are not a corner at all:
    /// this fixture's decision point ties 36 candidates at the top, which is
    /// exactly why the note is not optional. A first press picks one of them by
    /// the §3.2 keyed draw; without the sentence, a player who sees a `0` next to
    /// a move they preferred has no way to know the bot was indifferent.
    ///
    /// Anti-vacuity runs in the INVERSE direction from the usual (the organic
    /// screen already carries the note, so "add a tie and watch it appear" proves
    /// nothing): the tie is BROKEN by demoting every other maximiser one point,
    /// and the note must then disappear while the runner-up rows stay. A note
    /// printed unconditionally fails that half.
    #[test]
    fn a_tie_at_the_top_is_named_and_printed_as_a_zero_margin() {
        let session = sim_at_a_branching_point(4);
        let mut s = suggestion_for(&session);
        let tied = s.tied_at_top();
        assert!(tied > 1, "fixture: this decision point is expected to tie at the top");

        let text = render_suggestion(&s, session.state(), session.rules());
        assert!(
            text.contains(&format!("{tied} candidates tie at the top")),
            "the note must name the measured count ({tied}):\n{text}"
        );
        assert!(text.contains("so 0 means tied"), "and the margin column must say what 0 is:\n{text}");
        let zero_margin_row = text
            .lines()
            .skip_while(|l| !l.starts_with("Runner-ups ("))
            .any(|l| l.trim_start().starts_with("0  "));
        assert!(zero_margin_row, "a tied runner-up must print a literal 0 margin:\n{text}");

        // Break the tie: every maximiser except the suggestion drops one point.
        // The vector stays descending (the demoted entries sat at the maximum and
        // now sit just below it), which is the shape `suggest` hands the render.
        let best = s.best_value();
        for r in s.ranking.iter_mut() {
            if r.value == best && r.mv != s.mv {
                r.value = best - 1;
            }
        }
        s.ranking.sort_by(|a, b| b.value.cmp(&a.value));
        assert_eq!(s.tied_at_top(), 1, "fixture: the tie must actually be broken");

        let untied = render_suggestion(&s, session.state(), session.rules());
        assert!(!untied.contains("tie at the top"), "the note must not print untied:\n{untied}");
        assert!(untied.contains("Runner-ups ("), "and the rows themselves must stay:\n{untied}");
    }

    /// `elide` bounds a row at its budget and leaves a short row alone.
    #[test]
    fn long_rows_are_elided_at_the_budget() {
        let short = "Move President to Bank (A04)";
        assert_eq!(elide(short), short);
        let long: String = std::iter::repeat_n('x', ROW_TEXT_WIDTH + 20).collect();
        let cut = elide(&long);
        assert_eq!(cut.chars().count(), ROW_TEXT_WIDTH);
        assert!(cut.ends_with('…'));
    }
}
