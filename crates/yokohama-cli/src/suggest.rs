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
//!
//! **EVERY LINE FITS THE COMPOSED FRAME** (T10 review M1). `screen::wrap` sizes
//! the game screen to its widest line, so a body wider than the frame shoves the
//! board sideways for one command and back on the next. The budget is therefore
//! per PLAYER COUNT, because the frame is — `screen::frame_width`, **198/175/200
//! at 2/3/4 players** (the 2P entry moved at T3, when the designed dashboard
//! joined beside the board; the 4P one at T10, when it stacked below in seat
//! columns; the 3P one at the T10 review, when the block was widened to hold
//! its own compressed rows).
//!
//! **That budget no longer BINDS organically, and saying so is the point** (T2
//! review M1). The widest thing real play hands this screen is a placement
//! naming three areas and their fee recipients, about **135 columns**; against
//! the pre-redesign frames (114/132/108) that overflowed two counts outright,
//! and it is why the machinery below exists at all — the flat `<= 100` pin it
//! replaced asserted a property this render did not have. T2's V3 board then
//! widened every frame past 135 and T3 widened the 2P one further, so no
//! organic position reaches the bound any more. What the machinery is still for is stated rather than assumed: the
//! explicit-budget seam ([`render_suggestion_within`], which is what the two
//! pins that prove reflow and elision work now drive), and the guard if a frame
//! ever narrows again — a body that measured itself against a frame it happens
//! to fit today is a body that breaks silently the day the layout moves. Two
//! treatments, chosen per line by what its tail is worth:
//!
//! * **The suggested move WRAPS.** It is the answer, and the tail a cut would
//!   take is the fee clause — the part a reader is deciding on.
//! * **Context rows ELIDE** (continuation steps, runner-ups). A table that
//!   keeps its shape is worth more than a complete area name, and those rows
//!   carry their own tighter editorial budget ([`ROW_TEXT_WIDTH`]) which binds
//!   first at every player count; the frame budget is what makes the property
//!   TOTAL rather than true only at the sites someone remembered.
//!
//! [`Body`] owns both, so "no line is wider than the frame" is a property of the
//! one type every line goes through.

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

/// The runner-up block's heading, without its trailing candidate count.
///
/// `pub` because the guide pair quotes a captured `suggest` screen verbatim and
/// `guide_sync` holds the quote to THIS string rather than to a copy of it —
/// the same rule `HELP_TEXT` follows (authority lives in code; the guides
/// follow). Same for [`TABLE_HEADINGS`] and [`TOTAL_LABEL`].
pub const RUNNER_UPS_LABEL: &str = "Runner-ups (margin against the suggestion, so 0 means tied)";

/// The explanation table's column headings, in render order.
pub const TABLE_HEADINGS: [&str; 5] = ["feature", "you", "best opp", "weight", "product"];

/// The explanation table's summing row.
pub const TOTAL_LABEL: &str = "TOTAL";

/// One explanation-table row at the table's column widths.
///
/// Every row goes through here — headings, feature terms, the terminal standing
/// and the total — so the columns are stated once instead of at four format
/// sites that have to be kept identical by inspection. Cells arrive as strings
/// because the rows are not all numbers (the standing prints `#2 of 4`).
pub fn table_row(cells: [&str; 5]) -> String {
    format!("  {:<26}{:>8}{:>11}{:>9}{:>12}", cells[0], cells[1], cells[2], cells[3], cells[4])
}

/// Longest move description a LIST row prints before it is elided — the
/// EDITORIAL budget, narrower than any frame on purpose so a continuation step
/// or a runner-up stays a compact row beside its number rather than a paragraph
/// with a margin in front of it.
///
/// It is not the width GUARANTEE: that is the composed frame's own width, held
/// by [`Body`] over every line. This one binds first for rows at every player
/// count (80 plus the widest row prefix is 94, inside the narrowest frame's
/// 175 — still the 3P one), which is why the rows a reader sees are
/// unchanged by the frame bound.
const ROW_TEXT_WIDTH: usize = 80;

/// The closing honesty note (spec §7: the explanation text states term
/// contributions, it does not give play advice). Pre-wrapped as fixed lines
/// rather than reflowed at render time: its longest line is 80 columns, inside
/// the narrowest frame this driver composes, so re-flowing it per player count
/// would move a sentence a reader may have been shown before to no purpose. The
/// long CAVEATS below are the opposite case — at 169 and 238 characters neither
/// can be said in one line of ANY frame this driver composes, so they are
/// wrapped ([`Body::prose`]).
const HONESTY_NOTE: [&str; 3] = [
    "This is the bot's arithmetic, not advice: each row is (your value − the best",
    "opponent's) × the weight for this stage, summed. It read ONE guess at the hidden",
    "cards, and its weights are hand-set rather than tuned.",
];

/// The line-was-searched-in-a-sampled-world caveat, printed when the fold in
/// [`describe_line`] stopped matching the real position.
const DRIFT_NOTE: &str = "(the line was searched inside ONE guess at the hidden cards; the steps \
                          after the point it stops matching your real position are named against \
                          the last position that did)";

/// What a nonzero ply-cap counter licenses saying, and **no more** (T10 review
/// S1). The counter is per-DECISION — see [`Suggestion::search_hit_ply_cap`] —
/// so the claim is about the search, not about the line printed above it.
const PLY_CAP_NOTE: &str = "(this search stopped SOME continuation at its ply guard, so a line \
                            here may have been valued mid-turn instead of at the hand-over; the \
                            guard's counter is kept per decision and carries no identity, so which \
                            line it stopped is not recorded)";

/// Why there is no suggestion.
///
/// Split by CAUSE rather than collapsed to one string because the three mean
/// different things to the person reading them: a refused view is the driver
/// declining to work from something it cannot honestly read, a search error is
/// the bot finding nothing at a position that is still perfectly playable by
/// hand, and an inconsistency is a BUG in this program that the reader should
/// report rather than work around.
#[derive(Debug)]
pub enum SuggestError {
    /// `observe` refused the seat's view.
    View(String),
    /// The agent refused or dead-ended (a belief-sampler refusal, or no
    /// candidate at this decision point).
    Search(String),
    /// The agent's chosen move is not in the TRUE position's legal list
    /// (T10 review S2). Carries the move's `Debug`, which is what a bug report
    /// needs; `describe_move` is deliberately not used, since naming a move
    /// against a state that does not admit it is how a formatter gets to index
    /// something that is not there.
    Inconsistent(String),
}

impl std::fmt::Display for SuggestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::View(m) => write!(f, "the seat's own view could not be taken: {m}"),
            Self::Search(m) => write!(f, "{m}"),
            Self::Inconsistent(mv) => write!(
                f,
                "the bot chose a move this position does not offer, so there is nothing safe to\n\
                 show you. That is an inconsistency between the sampled world and the engine —\n\
                 a bug worth reporting, not something you did. The game is untouched.\n\
                 Move: {mv}"
            ),
        }
    }
}

/// The repo's standing treatment for a public error type (`MoveError`,
/// `ViewError`, `BeliefError`, `ArenaError`, `ApplyError` and four more): a
/// caller boxing this into `Box<dyn Error>` or wrapping it with `?` should not
/// have to special-case the one error type that is only `Display`.
impl std::error::Error for SuggestError {}

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
    /// `MAX_TURN_PLIES` stopped SOME continuation in this search — **not,
    /// necessarily, the one in [`continuation`](Self::continuation)** (T10
    /// review S1, which found the old name `ply_capped` and its rendered line
    /// claiming the principal line had been cut short).
    ///
    /// `SearchStats::ply_cap_hits` is summed over every world, every beam entry
    /// and every root candidate one decision expanded, so a nonzero value says
    /// the guard fired somewhere inside this search and nothing narrower: the
    /// counter carries no identity, and the trace a continuation is read back
    /// off does not record why it ended. The stronger claim would need a
    /// per-line marker the search does not keep, so the render says the weaker
    /// true thing ([`PLY_CAP_NOTE`]) rather than the stronger convenient one.
    ///
    /// Expected false: the guard sits 3.6× above the longest turn ever measured
    /// and recorded 0 hits across 5,890 research decisions.
    pub search_hit_ply_cap: bool,
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
/// `M` = 8, `WeightTable::V2` (V1 until the 2026-08-10 v2 freeze — reading the
/// defaults off the constructor is exactly what let this line lag it once).
///
/// **The answer is checked against the TRUE position before it is returned**
/// ([`ensure_offered_by_the_true_state`], T10 review S2).
pub fn suggest(
    state: &GameState,
    rules: &Rules,
    seat: PlayerId,
    stream_seed: u64,
) -> Result<Suggestion, SuggestError> {
    let view = observe(state, seat).map_err(|e| SuggestError::View(e.to_string()))?;
    let mut agent = lookahead::LookaheadAgent::new(rules, stream_seed);
    let report = agent.choose(&view, rules).map_err(SuggestError::Search)?;
    ensure_offered_by_the_true_state(&report.mv, state, rules)?;

    // `choose` cannot return without explaining (spec §0.3(b)), so a missing
    // explanation is a broken contract rather than a state to render around.
    let explanation = agent.last_explanation().cloned().ok_or_else(|| {
        SuggestError::Search("the agent returned a move it did not explain".to_string())
    })?;
    let mut ranking = agent.last_roots().to_vec();
    // Stable, so tied candidates keep the candidate scheme's own order and two
    // presses at one position cannot disagree about which tie is printed first.
    ranking.sort_by_key(|e| std::cmp::Reverse(e.value));

    Ok(Suggestion {
        seat,
        mv: report.mv,
        explanation,
        continuation: agent.last_continuation().to_vec(),
        ranking,
        stream_seed,
        search_hit_ply_cap: agent.last_stats().is_some_and(|s| s.ply_cap_hits > 0),
    })
}

/// Refuse a move the TRUE position does not offer (T10 review S2).
///
/// **Why this is not paranoia about a chain that looks closed.** The root
/// candidates a press ranks are enumerated inside a world the belief sampler
/// DETERMINIZED — a guess at the deck order and at every opponent's hidden
/// order card — and only the seat's own view of that world is honest by
/// construction. A legality question the guess can answer differently from the
/// table (anything reading deck content) would produce a move that is perfectly
/// legal where it was found and not offered where it would be played, and the
/// screen would then name it in the confident voice of an evaluation. Printing
/// an inapplicable move is worse than printing nothing: a tester would try to
/// enter it, the REPL would refuse it as illegal, and the tool would look
/// broken in the one place it is meant to be trustworthy.
///
/// The authority is the engine's own list — `get_legal_moves`, which is what
/// `Session::legal` and `apply_move`'s checked boundary both read — so this
/// adds no second opinion about legality, only the question. A sweep over 7,920
/// suggestions found no miss, which makes this the arm that stays quiet and
/// stays honest rather than the arm that fires.
fn ensure_offered_by_the_true_state(
    mv: &Move,
    state: &GameState,
    rules: &Rules,
) -> Result<(), SuggestError> {
    if get_legal_moves(state, rules).contains(mv) {
        return Ok(());
    }
    Err(SuggestError::Inconsistent(format!("{mv:?}")))
}

/// The `suggest` screen under assembly, holding the width budget every line it
/// emits is measured against.
///
/// **Why a type rather than `push_str` at each site.** The screen has to fit the
/// composed frame line by line (module doc), and the widest thing it can print —
/// a 135-column placement — outran two of the three pre-redesign frames and
/// would outrun any frame narrow enough, so the bound has to hold at
/// every site, including the ones a later edit adds. Routing every line through
/// here makes that a property of one type instead of a habit at fourteen call
/// sites, and it keeps the paint on the OUTSIDE of the measurement, which is
/// `style`'s alignment rule: escapes carry zero display width, so a cut applied
/// after painting could land inside one.
struct Body {
    out: String,
    /// Visible columns one line may occupy: `screen::frame_width` for this
    /// game's player count.
    width: usize,
}

impl Body {
    fn new(width: usize) -> Self {
        Self { out: String::new(), width }
    }

    /// A plain line, elided to the budget. **Every other method ends here**, so
    /// the bound cannot be bypassed by adding a shape.
    fn line(&mut self, text: &str) {
        self.out.push_str(&elide(text, self.width));
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    /// A line painted whole, with `indent` left OUTSIDE the paint: the plain
    /// text is elided FIRST, so what the budget measures is what the terminal
    /// shows, and piped bytes are the indent plus the text with nothing between.
    fn painted(&mut self, tone: Tone, indent: &str, text: &str) {
        let room = self.width.saturating_sub(indent.chars().count());
        self.out.push_str(indent);
        self.out.push_str(&paint(tone, &elide(text, room)));
        self.out.push('\n');
    }

    /// A painted head with a plain tail (the seat line): the tail is elided
    /// against whatever the head leaves of the budget.
    fn painted_head(&mut self, tone: Tone, head: &str, tail: &str) {
        let room = self.width.saturating_sub(head.chars().count());
        self.out.push_str(&paint(tone, head));
        self.out.push_str(&elide(tail, room));
        self.out.push('\n');
    }

    /// A list row: `prefix` prints verbatim (it is aligned padding, and eliding
    /// it would shear the column) and `text` is cut to `budget` or to whatever
    /// the frame leaves after the prefix, whichever is tighter.
    fn row(&mut self, prefix: &str, text: &str, budget: usize) {
        let room = self.width.saturating_sub(prefix.chars().count()).min(budget);
        self.line(&format!("{prefix}{}", elide(text, room)));
    }

    /// A caveat too long to say in one line of any frame, wrapped at spaces
    /// under `indent`. Wrapped rather than elided because half a caveat reads as
    /// a whole one.
    fn prose(&mut self, indent: &str, text: &str) {
        let room = self.width.saturating_sub(indent.chars().count());
        for piece in wrap_words(text, room) {
            self.line(&format!("{indent}{piece}"));
        }
    }
}

/// The whole `suggest` screen.
///
/// Plain text throughout except for three whole-line paints (the seat line, the
/// suggested move, the closing note); the aligned table is deliberately
/// unpainted, since escapes carry zero display width and every column here is
/// padded (`style`'s alignment rule — pad from the PLAIN text). Piped output is
/// therefore byte-identical to a terminal's minus the escapes, and `NO_COLOR`
/// is honoured by `paint` itself.
///
/// The width budget comes from the player count in `state` (module doc): the
/// composed frame this body will be wrapped in is sized per count, and the body
/// cannot measure a frame that does not exist yet.
pub fn render_suggestion(s: &Suggestion, state: &GameState, rules: &Rules) -> String {
    render_suggestion_within(s, state, rules, crate::screen::frame_width(state.players.len()))
}

/// [`render_suggestion`] at an explicit column budget.
///
/// The seam exists because the budget stopped BINDING. Phase-3.5a T2's V3 board
/// widened every frame (108 to 180 at four players), which carried them past
/// the widest move description real play produces — about 135 columns, a
/// placement naming three areas and every fee recipient. The reflow-and-elide
/// machinery below is still correct and still the guard against a move name
/// outgrowing its screen; what changed is that no ORGANIC position reaches it
/// any more, so the two pins that prove it works drive the budget here rather
/// than depending on a frame width that happens to be narrow. They said so
/// themselves when it stopped being true, which is why this seam is a repair
/// and not a convenience.
fn render_suggestion_within(
    s: &Suggestion,
    state: &GameState,
    rules: &Rules,
    budget: usize,
) -> String {
    let mut body = Body::new(budget);
    body.line(render::section("Suggest").trim_end_matches('\n'));

    let color = state.player(s.seat).color;
    // Every knob printed comes from the const the agent itself reads, so this
    // line cannot describe a rung the press did not run.
    //
    // **`evaluator impl {}` is `EVAL_IMPL_VERSION`, and it used to read
    // `eval v{}`, which was the wrong words for the right number** (T11). The
    // rung on the same line is `lookahead-v1`, so a header reading
    // `lookahead-v1 … eval v2` invited exactly one reading — that some newer
    // evaluator had replaced the v1 everything else names — and that reading was
    // false. The number is the evaluator MODULE's implementation revision, which
    // is what a reader needs to know two suggestions are comparable. Saying
    // "impl" says so; the weight table's own identity is a 64-bit digest and
    // belongs in provenance, not on screen.
    //
    // Since the 2026-08-10 v2 freeze it reads **3**, and the weights this rung
    // runs are `WeightTable::V2` — so the guide's §5 prose now has to keep TWO
    // numbers apart (the weight table's `v2` and the module's `impl 3`) where
    // before it only had to explain one. It does; the captured screen and that
    // paragraph move together, and `guide_sync` fails if this line drifts from
    // either format.
    body.painted_head(
        Tone::Player(color),
        &format!("Seat {} ({})", s.seat.0, render::color_name(color)),
        &format!(
            " · {} · beam {}, K={}, M={}, evaluator impl {} · stream {:#018x}",
            lookahead::LOOKAHEAD_CONTROLLER,
            lookahead::DEFAULT_BEAM,
            lookahead::DEFAULT_WORLDS,
            greedy::DEFAULT_RETREAT_SAMPLES,
            eval::EVAL_IMPL_VERSION,
            s.stream_seed,
        ),
    );
    body.line("Suggested move:");
    // The ANSWER wraps: a placement naming three areas and their fee recipients
    // runs past every frame, and the tail a cut would take is the fee — which is
    // the part a reader is deciding on. Continuation lines indent under the
    // first so the block still reads as one item.
    let answer = describe::describe_move(&s.mv, state, rules);
    let indents = [ANSWER_FIRST_INDENT, ANSWER_WRAP_INDENT];
    for (i, piece) in wrap_answer(&answer, body.width).into_iter().enumerate() {
        body.painted(Tone::Prompt, indents[usize::from(i > 0)], &piece);
    }

    // -- the line the value was read at ------------------------------------
    let (steps, drifted) = describe_line(&s.continuation, state, rules);
    body.blank();
    if steps.is_empty() {
        body.line("No line to show: the score below was read at this position itself.");
    } else {
        // The reach claim is dropped, not restated, when the ply guard fired
        // anywhere in this search: `search_hit_ply_cap` cannot tell us whether
        // THIS line reached the hand-over (S1), and the note below says so.
        let reach = if s.search_hit_ply_cap { "move(s)" } else { "move(s) to the hand-over" };
        body.line(&format!(
            "The line it searched, {} {reach} (the move above is step 1):",
            steps.len(),
        ));
        for (i, step) in steps.iter().enumerate() {
            body.row(&format!("  {}. ", i + 1), step, ROW_TEXT_WIDTH);
        }
        if s.search_hit_ply_cap {
            body.prose("  ", PLY_CAP_NOTE);
        }
        if drifted {
            body.prose("  ", DRIFT_NOTE);
        }
    }

    // -- the explanation ----------------------------------------------------
    body.blank();
    body.line(&format!(
        "Score at that point: {}   ·   stage {}/1000000 ppm (0 = opening, 1000000 = end trigger)",
        s.explanation.total, s.explanation.progress_ppm,
    ));
    body.line(&table_row(TABLE_HEADINGS));
    for term in &s.explanation.terms {
        body.line(&table_row([
            term.feature.name(),
            &term.viewer_value.to_string(),
            &term.opponent_value.to_string(),
            &term.weight.to_string(),
            &term.product.to_string(),
        ]));
    }
    if let Some(t) = &s.explanation.terminal {
        // On a finished game the positional vector does not govern at all
        // (§2.6), so every weight above reads 0 and the standing IS the score.
        body.line(&table_row([
            "final standing",
            &format!("#{} of {}", t.rank, t.seats),
            &t.viewer_total.to_string(),
            "",
            &t.value.to_string(),
        ]));
        body.line(&format!(
            "  (game over: place {} of {}, your re-derived final {} against the best other {})",
            t.rank, t.seats, t.viewer_total, t.best_other_total,
        ));
    }
    body.line(&table_row([TOTAL_LABEL, "", "", "", &s.explanation.total.to_string()]));

    // -- runner-ups ---------------------------------------------------------
    let best = s.best_value();
    let others: Vec<&lookahead::RootScore> =
        s.ranking.iter().filter(|r| r.mv != s.mv).take(RUNNER_UPS_SHOWN).collect();
    body.blank();
    if others.is_empty() {
        body.line("No runner-up: this decision point offered exactly one move.");
    } else {
        body.line(&format!("{RUNNER_UPS_LABEL} — {} candidate(s) ranked:", s.ranking.len()));
        for r in others {
            body.row(
                &format!("  {:>10}  ", r.value - best),
                &describe::describe_move(&r.mv, state, rules),
                ROW_TEXT_WIDTH,
            );
        }
    }
    let tied = s.tied_at_top();
    if tied > 1 {
        body.line(&format!(
            "  {tied} candidates tie at the top; which one you see is this position's own"
        ));
        body.line("  keyed draw, so a second press will not change it.");
    }

    // Painted line by line, never as one block: `paint` wraps whatever it is
    // given, so a trailing newline INSIDE the argument would put the reset
    // escape on a line of its own and add a row to the frame.
    for note in HONESTY_NOTE {
        body.painted(Tone::Dim, "", note);
    }
    body.out
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
///
/// Returns the step TEXTS, unnumbered and unelided: numbering and width are the
/// render's business ([`Body::row`]), and a describer that also formatted rows
/// would need the frame budget threaded into it for nothing.
fn describe_line(line: &[Move], state: &GameState, rules: &Rules) -> (Vec<String>, bool) {
    let mut at = state.clone();
    let mut drifted = false;
    let mut steps = Vec::with_capacity(line.len());
    for mv in line {
        steps.push(describe::describe_move(mv, &at, rules));
        if !drifted {
            match apply_move(&at, mv, rules) {
                Ok(next) => at = next,
                Err(_) => drifted = true,
            }
        }
    }
    (steps, drifted)
}

/// Trim `text` to `width` display columns, marking the cut with an ellipsis.
///
/// Counts CHARS, which is the same measurement `screen::visible_width` makes on
/// text carrying no ANSI — and everything reaching here is plain, because
/// [`Body`] elides before it paints and never after.
fn elide(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    // The marker costs a column, so a budget of 0 has nowhere to put it. Only a
    // pathological frame reaches this; returning empty beats panicking or
    // emitting a line one column wider than asked for.
    if width == 0 {
        return String::new();
    }
    text.chars().take(width - 1).collect::<String>() + "…"
}

/// The wrapped answer's FIRST line sits at the body's ordinary two-space
/// margin; [`ANSWER_WRAP_INDENT`] carries the rest.
const ANSWER_FIRST_INDENT: &str = "  ";

/// Continuation lines of the wrapped answer indent under the first line's text,
/// so a two-line move still reads as one item rather than two steps.
const ANSWER_WRAP_INDENT: &str = "     ";

/// Wrap the answer with each line sized against the indent it will actually be
/// printed at — two passes, because the first line is three columns wider than
/// the rest.
///
/// One pass at the wider indent's budget is what this replaces, and it
/// under-filled line 1 by up to three columns (six-lens review, cosmetic: no
/// width guarantee was at risk, since [`Body::painted`] elides against the
/// indent it is handed).
fn wrap_answer(answer: &str, width: usize) -> Vec<String> {
    let room_for = |indent: &str| width.saturating_sub(indent.chars().count());
    let mut pieces = wrap_words(answer, room_for(ANSWER_FIRST_INDENT));
    if pieces.len() > 1 {
        // `wrap_words` splits on whitespace and joins with single spaces, so
        // re-joining what it produced loses nothing a second pass could use.
        let rest = pieces.split_off(1).join(" ");
        pieces.extend(wrap_words(&rest, room_for(ANSWER_WRAP_INDENT)));
    }
    pieces
}

/// Break `text` at spaces into runs of at most `width` display columns.
///
/// A single word longer than the budget is left WHOLE and cut later by
/// [`elide`], on the line it lands on: an area name and its id cannot be broken
/// anywhere useful, so hyphenating one would only move the loss. That split of
/// duties is deliberate — wrapping is for readability, eliding is the guarantee,
/// and only one of them needs to be total.
///
/// Always yields at least one piece, so a caller printing an answer always
/// prints a line.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut pieces: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            pieces.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    pieces.push(line);
    pieces
}

#[cfg(test)]
mod tests {
    /// The 2-player simulation the guide's captured `suggest` screen is taken
    /// from: a fresh game at a fixed seed, walked to its first Placement
    /// decision, where `suggest` has a real list to rank.
    ///
    /// **A named fixture rather than a remembered set of keystrokes**, so the
    /// guide's block can be REGENERATED from one run instead of edited by hand
    /// — the discipline `repl`'s Movement captures and `reveal`'s reveal
    /// captures already hold, extended at T11 to the third block a tester reads
    /// line by line. Until now the suggest capture had no regeneration pin at
    /// all: `guide_sync` checked four strings inside it and nothing checked the
    /// other forty lines.
    pub(crate) fn guide_capture_fixture() -> crate::repl::Repl {
        let session = crate::session::Session::new_sim(
            yokohama_core::GameConfig::for_player_count(2),
            4,
            vec![yokohama_core::StartingOrderChoice::Keep(0); 2],
            yokohama_core::Rules::standard(),
        )
        .expect("a standard 2-player simulation");
        let mut r = crate::repl::Repl::new(session);
        // The opening ceremony: bare Enter applies each forced single option
        // until a real decision arrives.
        while !r.prompt().contains("PLACEMENT") {
            r.step("");
        }
        r
    }

    /// The screen that fixture's `suggest` draws, plain — what the guide block
    /// must be, byte for byte.
    ///
    /// The SCREEN, not the step's whole output: `out_view` appends the standing
    /// input hint under every view, and the hint is documented in §4 on its own
    /// rather than reprinted under a capture of something else.
    pub(crate) fn guide_capture(r: &crate::repl::Repl) -> String {
        crate::style::force(false);
        let seat = r.session.state().current_player;
        let seed = super::stream_seed(r.session.record(), seat);
        let s = super::suggest(r.session.state(), r.session.rules(), seat, seed)
            .expect("the fixture sits at a decision with a legal list");
        super::render_suggestion(&s, r.session.state(), r.session.rules())
            .trim_end_matches('\n')
            .to_string()
    }

    /// **The guide's captured `suggest` screen IS what the fixture renders**
    /// (T11), the third block held to this discipline after `repl`'s Movement
    /// captures and `reveal`'s reveal captures.
    ///
    /// It closes the gap those two were written for: before this, a reviewer's
    /// memory was the only thing standing between the guide and a hand-edited
    /// forty-line screen, and `guide_sync`'s four string checks would have
    /// passed a block whose every number was invented. That is the exact defect
    /// the T4 review found in the Movement transcript.
    ///
    /// Re-capture by running `guide_capture(&guide_capture_fixture())` and
    /// pasting the whole block into BOTH formats — never by editing it, and
    /// never by splicing two runs together.
    #[test]
    fn the_guide_suggest_capture_is_what_the_fixture_renders() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        let rendered = guide_capture(&guide_capture_fixture());
        let opening = rendered.lines().next().expect("the screen's rule header");
        let block = GUIDE_MD
            .split("```")
            .map(|part| part.trim_matches('\n'))
            .find(|part| part.starts_with(opening))
            .unwrap_or_else(|| panic!("cli-guide.md carries no captured suggest screen"));
        assert_eq!(
            block, rendered,
            "docs/cli-guide.md's captured suggest screen no longer matches what \
             `guide_capture_fixture()` renders. Re-capture it in BOTH formats from ONE run of \
             that fixture — never by editing the block by hand. The rendered screen is above; \
             `guide_sync` pins the .html twin against the .md.",
        );
    }

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

    /// A `players`-player simulation driven a real distance in and stopped at
    /// the first position `stop` accepts — the width pins' fixture, and mid-game
    /// on purpose: the widest move descriptions a player ever sees are placements
    /// that name three areas AND their fee recipients, which cannot exist until
    /// opponents have Presidents on the board.
    ///
    /// The walk ROTATES through the legal list rather than always taking `[0]`
    /// (`screen`'s own game-walk pin does the same, for the same reason): a
    /// take-the-first line degenerates, never builds anything, and never pays a
    /// fee, so it would reach none of the positions these pins exist for.
    fn sim_mid_game(players: u8, seed: u64, mut stop: impl FnMut(&Session) -> bool) -> Session {
        let mut session = Session::new_sim(
            GameConfig::for_player_count(players),
            seed,
            vec![StartingOrderChoice::Keep(0); players as usize],
            Rules::standard(),
        )
        .expect("a standard sim");
        for step in 0..400usize {
            let legal = session.legal();
            if legal.is_empty() || session.is_over() {
                break;
            }
            if step > 0 && stop(&session) {
                return session;
            }
            let mv = legal[step % legal.len()].clone();
            session.apply(mv, vec![]).expect("a move from the engine's own list applies");
        }
        panic!("{players}P: no position matching the fixture's condition within 400 moves");
    }

    /// The widest move description the position offers, with its column count —
    /// what the answer line has to survive. Ties go to the first, which keeps
    /// the fixture deterministic.
    fn widest_legal_move(session: &Session) -> (Move, usize) {
        session
            .legal()
            .into_iter()
            .map(|m| {
                let w = describe::describe_move(&m, session.state(), session.rules()).chars().count();
                (m, w)
            })
            .max_by_key(|(_, w)| *w)
            .expect("a legal move")
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
            "the event index keys it — otherwise every press of a whole game shares one draw",
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

    /// **The ply-cap note claims the SEARCH, not the line** (T10 review S1).
    ///
    /// `SearchStats::ply_cap_hits` is a whole-DECISION counter, merged across
    /// worlds, beam entries and root candidates, so a nonzero value cannot say
    /// the printed line was the one cut short — which is exactly what the old
    /// label ("the line it searched, cut short by the search's own ply guard")
    /// said. Two things must hold, and the second is the repair: the reach claim
    /// "to the hand-over" is DROPPED when the counter fired (it is no longer
    /// derivable), and what replaces it is scoped to the search.
    ///
    /// Doctored rather than driven, and it cannot be otherwise: the guard sits
    /// 3.6× above the longest turn ever measured and recorded 0 hits in 5,890
    /// research decisions, which the fixture assertion below states as a
    /// property rather than a hope.
    #[test]
    fn the_ply_cap_note_claims_the_search_and_not_the_printed_line() {
        let session = sim_at_a_branching_point(4);
        let mut s = suggestion_for(&session);
        assert!(!s.continuation.is_empty(), "fixture: there must be a line to label");
        assert!(!s.search_hit_ply_cap, "fixture: a real run does not hit the guard");
        let honest = render_suggestion(&s, session.state(), session.rules());
        assert!(honest.contains("to the hand-over"), "an uncapped search reached it: {honest}");
        assert!(!honest.contains("ply guard"), "and says nothing about the guard: {honest}");

        s.search_hit_ply_cap = true;
        let capped = render_suggestion(&s, session.state(), session.rules());
        assert!(
            !capped.contains("to the hand-over"),
            "the reach claim is not derivable once the guard fired:\n{capped}",
        );
        assert!(capped.contains("ply guard"), "the note must be there:\n{capped}");
        assert!(
            capped.contains("SOME continuation"),
            "and must scope itself to the search, not to the line above it:\n{capped}",
        );
        assert!(
            capped.contains("not recorded"),
            "...saying outright that which line was stopped is unknown:\n{capped}",
        );
        // The line's own step count still prints: it is a fact about the line
        // either way, and dropping it would leave the rows unlabelled.
        assert!(
            capped.contains(&format!("{} move(s)", s.continuation.len())),
            "the step count survives:\n{capped}",
        );
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

    /// **Piped output is plain, and every line fits the frame — at each player
    /// count** (T10 review M1).
    ///
    /// What this replaced was asserting a property the render did not have. The
    /// old pin measured against a flat `<= 100`, which was not any frame's width:
    /// the composed frame was 114/132/108 columns at 2/3/4 players then (it is
    /// 198/175/200 since the T10 review — `screen::frame_width` is the authority either
    /// way), so 100 was simultaneously too strict for 2P/3P and too slack for
    /// the case that actually broke — 4P, where the frame was NARROWEST while
    /// the board was widest. It also drove `repl_mid_game(4)`-style 2P fixtures
    /// at every count, so the 4P screen was never rendered at all.
    ///
    /// Measured per line with `screen::visible_width` (the escape-stripping
    /// measure), on a real mid-game position at each count. The plain half of the
    /// old pin is kept unchanged: a pipe must see no ANSI whatsoever.
    ///
    /// **Two renders per count, because the organic one is not enough.** An
    /// arbitrary branching position's own suggestion usually fits every frame
    /// without help — measured, this pin passes on a render with the bound ripped
    /// out if that is all it looks at — so each count also renders the screen
    /// ANSWERING WITH THE WIDEST MOVE THE POSITION OFFERS, which is the case the
    /// bound exists for and a move a player could really be handed.
    ///
    /// **The bound binds at every count, and since T2 it does so by
    /// construction rather than by luck.** It used to depend on the frames: the
    /// widest move at 2 players ran 120 columns against a 114-column frame and
    /// 128 against 108 at 4, while the 3-player frame (132, the widest of the
    /// three then) cut nothing at all — so the tally could only demand that SOME
    /// count bind. Every frame is now wider than any move description real play
    /// produces, so instead each count measures against the TIGHTER of its frame
    /// and one column under the widest move the position offers. That budget is
    /// below the widest move by construction, which is why the tally below can
    /// demand all three.
    #[test]
    fn the_render_is_plain_and_fits_the_frame_at_every_player_count() {
        let mut counts_where_the_bound_binds = Vec::new();
        for players in [2u8, 3, 4] {
            // The frame's own budget is what the shipped path uses; the
            // fixture then measures against the TIGHTER of it and the widest
            // move the position offers, so the bound still binds somewhere
            // (see `render_suggestion_within` on why the frame stopped
            // binding on its own at T2).
            let frame = crate::screen::frame_width(players as usize);
            // Prefer a position that offers an over-wide move to a merely deep
            // one: the risky position is the one worth pinning.
            let session = sim_mid_game(players, 4, |s| {
                s.legal().len() > 1
                    && (widest_legal_move(s).1 > frame || s.record().events.len() > 25)
            });
            let mut s = suggestion_for(&session);
            // The shipped path's own answer, at every count, is a move the ENGINE
            // offers here -- S2's quiet arm, swept rather than argued.
            assert!(
                session.legal().contains(&s.mv),
                "{players}P: the suggestion must be in the true position's legal list: {:?}",
                s.mv,
            );

            let (widest, width) = widest_legal_move(&session);
            // A budget the widest move here genuinely outruns, so the render
            // has to DO something about it rather than fitting by luck.
            let budget = frame.min(width.saturating_sub(1)).max(40);
            if width > budget {
                counts_where_the_bound_binds.push((players, width, budget));
            }
            for answer in [s.mv.clone(), widest] {
                s.mv = answer;
                for (label, text) in [
                    ("shipped", render_suggestion(&s, session.state(), session.rules())),
                    (
                        "bound",
                        render_suggestion_within(&s, session.state(), session.rules(), budget),
                    ),
                ] {
                    assert!(
                        !text.contains('\u{1b}'),
                        "{players}P {label} piped output must carry no ANSI:\n{text}",
                    );
                    let limit = if label == "bound" { budget } else { frame };
                    for line in text.lines() {
                        let w = crate::screen::visible_width(line);
                        assert!(
                            w <= limit,
                            "{players}P {label}: a line is {w} columns against {limit}: {line:?}",
                        );
                    }
                }
            }
        }
        assert!(
            counts_where_the_bound_binds.len() == 3,
            "every player count must render its widest move against a budget that budget \
             genuinely cuts -- that is the condition under which the lines above measure \
             reflow and elision rather than a screen that happened to fit: \
             {counts_where_the_bound_binds:?}",
        );
    }

    /// **The width bound is load-bearing, not decoration** (T10 review M1(c)).
    ///
    /// A real 4-player legal list offers moves whose description alone runs to
    /// 135 columns — a placement naming three areas plus every fee recipient.
    /// That used to be past the WHOLE 4P frame, which was the narrowest at 108;
    /// since T2 it is the widest, at 180 and then 200 from T10, so the cut is
    /// driven by an explicit
    /// budget one column under the move. What did not change is where the move
    /// comes from, and that is the anti-vacuity: the fixture walks until the
    /// position offers such a move and the suggestion is doctored to it — one
    /// from the fixture's own legal list, not an invented string, so the screen
    /// rendered is a screen a player could really be shown.
    ///
    /// Both treatments are checked where each applies: the ANSWER must reflow
    /// (and lose no word — a cut answer is the failure this repair exists to
    /// prevent), while a RUNNER-UP row carrying the same move must be cut with an
    /// ellipsis (a row that reflowed would break the margin column). A render
    /// with no bound at all fails the first assertion in the list, at 137
    /// columns.
    #[test]
    fn a_move_too_wide_for_the_frame_is_reflowed_and_rows_are_elided() {
        // The 4P frame was the NARROWEST (108) until T2's V3 board made it the
        // widest (180, and 200 since T10), which put it past the ~135 columns a real move
        // description reaches — so the position no longer offers a move wider
        // than its own frame, and the fixture drives the budget instead. See
        // `render_suggestion_within`.
        let session = sim_mid_game(4, 4, |s| s.legal().len() > 1 && widest_legal_move(s).1 > 120);
        let (widest, width) = widest_legal_move(&session);
        let budget = width - 1;
        assert!(
            width > budget,
            "fixture: the position must offer a move wider than the budget ({width} vs {budget})",
        );
        let widest_text = describe::describe_move(&widest, session.state(), session.rules());

        let mut s = suggestion_for(&session);
        s.mv = widest.clone();
        assert!(!s.ranking.is_empty(), "fixture: a runner-up row to elide");
        // A runner-up that is NOT the suggestion, so the row actually prints.
        let other = s.ranking.iter_mut().find(|r| r.mv != widest).expect("a second candidate");
        other.mv = widest.clone();
        let text = render_suggestion_within(&s, session.state(), session.rules(), budget);

        for line in text.lines() {
            let w = crate::screen::visible_width(line);
            assert!(w <= budget, "a line is {w} columns against the frame's {budget}: {line:?}");
        }

        let answer: Vec<&str> = text
            .lines()
            .skip_while(|l| *l != "Suggested move:")
            .skip(1)
            .take_while(|l| l.starts_with("  "))
            .collect();
        assert!(
            answer.len() >= 2,
            "a {width}-column answer must reflow onto more than one line, got {answer:?}",
        );
        assert_eq!(
            answer.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" "),
            widest_text.split_whitespace().collect::<Vec<_>>().join(" "),
            "wrapping the answer must not drop or reorder a word",
        );
        let rows: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.starts_with("Runner-ups ("))
            .skip(1)
            .take_while(|l| l.starts_with("  "))
            .collect();
        assert!(
            rows.iter().any(|l| l.ends_with('…')),
            "the runner-up row carrying the same move must be CUT, not reflowed (a reflowed row \
             would break the margin column):\n{rows:#?}",
        );
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
        s.ranking.sort_by_key(|e| std::cmp::Reverse(e.value));
        assert_eq!(s.tied_at_top(), 1, "fixture: the tie must actually be broken");

        let untied = render_suggestion(&s, session.state(), session.rules());
        assert!(!untied.contains("tie at the top"), "the note must not print untied:\n{untied}");
        assert!(untied.contains("Runner-ups ("), "and the rows themselves must stay:\n{untied}");
    }

    /// `elide` bounds text at whatever budget it is given and leaves short text
    /// alone. The degenerate budgets are pinned too: a width that leaves no room
    /// for the marker returns nothing rather than a line one column too wide,
    /// since the whole point of the helper is that its result fits.
    #[test]
    fn elide_cuts_at_the_budget_it_is_given() {
        let short = "Move President to Bank (A04)";
        assert_eq!(elide(short, ROW_TEXT_WIDTH), short);
        assert_eq!(elide(short, short.chars().count()), short, "exactly at the budget is not a cut");

        let long: String = std::iter::repeat_n('x', ROW_TEXT_WIDTH + 20).collect();
        let cut = elide(&long, ROW_TEXT_WIDTH);
        assert_eq!(cut.chars().count(), ROW_TEXT_WIDTH);
        assert!(cut.ends_with('…'));

        // The same text against a frame-sized budget: one helper, two budgets.
        assert_eq!(elide(&long, 108).chars().count(), 100, "a 100-char text fits 108 whole");
        assert_eq!(elide(&long, 12).chars().count(), 12);
        assert_eq!(elide(&long, 0), "", "no room for the marker means no line");
    }

    /// `wrap_words` never emits a line past its budget, keeps every word in
    /// order, and always emits at least one piece (so a caller printing an
    /// answer always prints a line).
    ///
    /// The over-long single word is the deliberate hole: wrapping leaves it
    /// whole and `elide` cuts it at the line it lands on, because an area name
    /// and its id have no useful break point. Pinned so the division of labour is
    /// a decision rather than an oversight.
    #[test]
    fn wrap_words_breaks_at_spaces_and_keeps_every_word() {
        let text = "Place 1 assistant in each of: Tea Plantation (A04), Fishing Ground (A05), \
                    Employment Agency (A07) — fee 1 yen (1 to Blue)";
        let pieces = wrap_words(text, 40);
        assert!(pieces.len() > 1, "a 120-column sentence must break: {pieces:?}");
        for p in &pieces {
            assert!(p.chars().count() <= 40, "{p:?} is {} columns", p.chars().count());
        }
        assert_eq!(
            pieces.join(" ").split_whitespace().collect::<Vec<_>>(),
            text.split_whitespace().collect::<Vec<_>>(),
            "every word, in order",
        );

        assert_eq!(wrap_words("", 40), vec![String::new()], "always at least one piece");
        let word = "x".repeat(50);
        assert_eq!(wrap_words(&word, 10), vec![word.clone()], "an unbreakable word is left to elide");
        assert_eq!(wrap_words("aa bb", 0).len(), 2, "a zero budget degrades to one word per line");
    }

    /// Each answer line is sized against the indent it is PRINTED at: the first
    /// at two columns, the rest at five (six-lens review — one budget for both
    /// under-filled line 1 by the three-column difference).
    ///
    /// Every word survives in order, and no line plus its own indent exceeds the
    /// frame — which is the property the two budgets exist to buy.
    #[test]
    fn the_answer_wraps_against_the_indent_each_line_actually_uses() {
        const WIDTH: usize = 40;
        let text = "Place 1 assistant in each of: Tea Plantation (A04), Fishing Ground (A05), \
                    Employment Agency (A07) — fee 1 yen (1 to Blue)";
        let pieces = wrap_answer(text, WIDTH);
        assert!(pieces.len() > 2, "the fixture must exercise more than one continuation line");
        for (i, piece) in pieces.iter().enumerate() {
            let indent = if i == 0 { ANSWER_FIRST_INDENT } else { ANSWER_WRAP_INDENT };
            assert!(
                piece.chars().count() + indent.chars().count() <= WIDTH,
                "line {i} ({piece:?}) plus its indent exceeds the frame",
            );
        }
        assert_eq!(
            pieces.join(" ").split_whitespace().collect::<Vec<_>>(),
            text.split_whitespace().collect::<Vec<_>>(),
            "every word, in order",
        );

        // The first line really does use its extra room: a word that fits at
        // the 2-space indent but not at the 5-space one lands on line 1.
        let fitted = "aaaa bbbb cc dd";
        assert_eq!(wrap_answer(fitted, 14), vec!["aaaa bbbb cc".to_string(), "dd".to_string()]);
        assert_eq!(
            wrap_words(fitted, 14 - ANSWER_WRAP_INDENT.chars().count()),
            vec!["aaaa bbbb".to_string(), "cc dd".to_string()],
            "anti-vacuity: the one-budget wrap this replaced breaks a word earlier",
        );
    }

    /// **The true-state check** (T10 review S2). The root candidates a press
    /// ranks come from a world the belief sampler determinized, so the shipped
    /// path asks the ENGINE whether the answer is offered here before it is
    /// rendered. Both directions are pinned on a real position: every move the
    /// legal list offers passes (no accidental narrowing — a check that refused
    /// legal moves would break `suggest` outright), and a move the list does not
    /// offer is refused as an INCONSISTENCY, not as a search failure.
    ///
    /// The refused arm is reached by handing the check a move directly rather
    /// than by making a real search go wrong: nothing in-tree can force the
    /// sampler into a divergent world, and scaffolding a fake agent would test
    /// the scaffold. What is pinned instead is the whole of what the arm does —
    /// its variant, and a message that names the cause and says the game is
    /// untouched, since the reader's next question after "the bot did what?" is
    /// whether their game survived it.
    #[test]
    fn a_move_the_true_position_does_not_offer_is_refused_as_an_inconsistency() {
        let session = sim_at_a_branching_point(4);
        let (state, rules) = (session.state(), session.rules());
        let legal = session.legal();
        assert!(legal.len() > 1, "fixture: a branching point");
        for mv in &legal {
            assert!(
                ensure_offered_by_the_true_state(mv, state, rules).is_ok(),
                "every legal move must pass: {mv:?}",
            );
        }

        // A move this position does not offer. Picked by asking the legal list
        // rather than by assuming a phase: `EndAdditionalPhase` is illegal
        // mid-turn, and the assertion below fails loudly if that ever changes
        // instead of passing vacuously.
        let absent = Move::EndAdditionalPhase;
        assert!(!legal.contains(&absent), "fixture: the chosen move must really be absent");

        let err = ensure_offered_by_the_true_state(&absent, state, rules)
            .expect_err("a move the position does not offer must be refused");
        assert!(matches!(err, SuggestError::Inconsistent(_)), "{err:?}");
        let shown = err.to_string();
        assert!(shown.contains("does not offer"), "{shown}");
        assert!(shown.contains("inconsistency"), "the cause must be named: {shown}");
        assert!(shown.contains("bug worth reporting"), "{shown}");
        assert!(shown.contains("game is untouched"), "and the reader's own game: {shown}");
        assert!(shown.contains("EndAdditionalPhase"), "with the move itself: {shown}");
    }
}
