//! The evaluation surface on the wire: what the bot would play, and why it
//! played what it played.
//!
//! **These are DTOs, and that is a rule rather than a style.** `Explanation`,
//! `TermContribution`, `TerminalStanding`, `RootScore` and `SearchStats` all
//! live in `yokohama-core` and none of them derives `Serialize`. `serde` IS a
//! dependency there, so a derive would compile — the reason not to is that this
//! lane's contract is that `yokohama-core` stays byte-identical to master, and
//! the one core change this lane attempted was written, reviewed and reverted.
//! So the shapes are restated here, in the crate that owns the wire.
//!
//! **What is NOT restated: the arithmetic, the wording, and the refusals.**
//! Every number below is copied out of the engine's own explanation; every move
//! is worded by `describe::describe_move`; every refusal is
//! `yokohama_cli::why`'s, raised by `why::account_parts`. That function's own
//! doc is the instruction this module follows:
//!
//! > A consumer that draws its own table wants the first half and none of the
//! > second, and it must not reimplement the first half: the refusals ARE the
//! > honesty gate.

use serde::Serialize;
use yokohama_cli::bot::DecisionExplanation;
use yokohama_cli::suggest::Suggestion;
use yokohama_cli::why::{Reconstruction, Refusal, WhyAccount};
use yokohama_core::game::driven_runner;
use yokohama_core::game::eval::{Explanation as CoreExplanation, TerminalStanding};
use yokohama_core::game::lookahead::RootScore;
use yokohama_core::{GameState, Move, PlayerId, Rules};

/// One feature's contribution, exactly as `eval::explain` computed it.
///
/// `feature` is `Feature::name()` — the STABLE snake_case token, not a display
/// string. It is the same key the arena's report, `eval_probe` and the CLI's
/// table all use, so a figure recorded against one instrument can be grepped
/// against another's. Human wording is served from `/api/labels`, which is where
/// every other enum→string mapping on this API lives.
///
/// `product` is `weight · (viewer_value − opponent_value)` and is CARRIED rather
/// than left to the client to recompute — the client that re-multiplied would be
/// a second implementation of the one identity this surface exists to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Term {
    pub feature: &'static str,
    pub viewer_value: i64,
    pub opponent_value: i64,
    pub weight: i64,
    pub product: i64,
}

/// A FINISHED game's re-derived standing, and then the whole of `total`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Terminal {
    pub rank: u8,
    pub seats: u8,
    pub viewer_total: i64,
    pub best_other_total: i64,
    pub value: i64,
}

/// A score, term by term.
///
/// **THE EXACT-SUM IDENTITY travels with the payload**, because a chart the
/// reader cannot foot is a picture rather than an explanation:
///
/// ```text
/// total == sum(terms[].product) + (terminal ? terminal.value : 0)
/// ```
///
/// Two consequences, both load-bearing and neither negotiable:
///
/// 1. **Every one of the 34 terms is here, including the zero-product ones.**
///    `Explanation` guarantees one term per feature always, and the identity
///    only holds over all of them. Dropping the zeros server-side would shrink
///    the payload and make the sum unverifiable by the reader — the one thing
///    this surface must not do. The client collapses them behind a disclosure
///    and prints the count, as the terminal dims them and prints "N of 34".
/// 2. **A terminal explanation is a DIFFERENT PICTURE, not an empty one.** On a
///    finished game `explain` reports every term at `weight == 0` and
///    `product == 0` — reporting an interpolated weight there would be a lie
///    about what `score` used — and the whole score sits in `terminal.value`. A
///    bar chart over features would render empty against a nonzero total, so a
///    client with `terminal` non-null draws the standing instead of the bars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Explanation {
    pub terms: Vec<Term>,
    pub terminal: Option<Terminal>,
    pub total: i64,
    /// The stage proxy the weights were interpolated at, in ppm. Carried so the
    /// explanation is self-contained: every weight above is recomputable from
    /// this and the table — except on a terminal view, where the reported
    /// weights are 0 by the rule above and this is informational only.
    pub progress_ppm: u32,
    /// How many of `terms` carry a nonzero product. Computed here rather than in
    /// the browser because the CLI's summary line already states it and the two
    /// must not be able to disagree.
    pub nonzero: usize,
}

impl Explanation {
    pub fn of(e: &CoreExplanation) -> Self {
        let terms: Vec<Term> = e
            .terms
            .iter()
            .map(|t| Term {
                feature: t.feature.name(),
                viewer_value: t.viewer_value,
                opponent_value: t.opponent_value,
                weight: t.weight,
                product: t.product,
            })
            .collect();
        Self {
            nonzero: terms.iter().filter(|t| t.product != 0).count(),
            terms,
            terminal: e.terminal.as_ref().map(Terminal::of),
            total: e.total,
            progress_ppm: e.progress_ppm,
        }
    }
}

impl Terminal {
    fn of(t: &TerminalStanding) -> Self {
        Self {
            rank: t.rank,
            seats: t.seats,
            viewer_total: t.viewer_total,
            best_other_total: t.best_other_total,
            value: t.value,
        }
    }
}

/// One root candidate, worded and priced.
///
/// `margin` is `value − best`, so **0 means tied** — the sense
/// `suggest::RUNNER_UPS_LABEL` already states in the terminal, kept identical
/// here so the two surfaces cannot mean different things by the same word.
///
/// **`chosen` is not a convenience, and it is not `index == 0`.** The ranking is
/// sorted best-first, but the agent's pick is *one of the maximisers* — ties are
/// broken by a keyed draw, never by "whichever the generator listed first",
/// which is exactly what `Suggestion::tied_at_top` exists to report. Measured on
/// the committed fixture: two candidates tie at 78 and the chosen one is row
/// **1**. A client that dropped row 0 to get the runner-ups would therefore list
/// the move the bot actually played as something it beat, and hide a real
/// runner-up. So the server marks it, rather than leaving the browser to infer
/// identity from a display string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ranked {
    pub text: String,
    pub value: i64,
    pub margin: i64,
    pub chosen: bool,
}

fn rank_all(ranking: &[RootScore], chosen: &Move, state: &GameState, rules: &Rules) -> Vec<Ranked> {
    let best = ranking.first().map_or(0, |r| r.value);
    // `Move` is `PartialEq`, so identity is compared as identity. Comparing the
    // WORDS would be a display string doing a type's job, and two distinct
    // candidates are not guaranteed to word differently.
    let mut marked = false;
    ranking
        .iter()
        .map(|r| {
            let is_chosen = !marked && &r.mv == chosen;
            marked |= is_chosen;
            Ranked {
                text: yokohama_cli::describe::describe_move(&r.mv, state, rules),
                value: r.value,
                margin: r.value - best,
                chosen: is_chosen,
            }
        })
        .collect()
}

fn word_line(line: &[Move], state: &GameState, rules: &Rules) -> Vec<String> {
    line.iter().map(|m| yokohama_cli::describe::describe_move(m, state, rules)).collect()
}

/// The fifteen search counters, named exactly as every other instrument names
/// them.
///
/// The names are `driven_runner::counter_rows`' own and are carried VERBATIM: a
/// friendlier spelling here would mean a reader could not grep this screen's
/// output with the arena report's vocabulary. `group` is the CLI's own
/// three-way split, so the browser can lay them out the way the terminal does
/// without re-deciding which counter is a cost and which is a guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Counter {
    pub name: &'static str,
    pub value: u64,
    pub group: &'static str,
}

/// Which line a counter belongs on.
///
/// A MIRROR of `why.rs`'s `grouping_of`, and the `_` arm is the same
/// deliberate one: core's `counter_rows` destructures `SearchStats` with no
/// `..`, so a sixteenth counter is a compile error THERE and lands here as
/// `ungrouped` rather than vanishing. A counter this crate has never heard of
/// is still shown.
fn group_of(name: &str) -> &'static str {
    match name {
        "root_candidates" | "nodes" | "handles" | "route_dfs_nodes" | "forced_nodes" | "leaves"
        | "scored" | "root_scored" | "max_line" | "free_extension_steps" => "work",
        "budget_binds" | "budget_starved" | "placement_cap_binds" | "ply_cap_hits"
        | "free_cap_hits" => "limit",
        _ => "ungrouped",
    }
}

fn counters(stats: &yokohama_core::game::lookahead::SearchStats) -> Vec<Counter> {
    driven_runner::counter_rows(stats)
        .into_iter()
        .map(|(name, value)| Counter { name, value, group: group_of(name) })
        .collect()
}

/// `GET /api/games/{id}/suggest` — what the production contestant would play on
/// the viewing seat's own view.
#[derive(Debug, Clone, Serialize)]
pub struct Suggest {
    pub seat: PlayerId,
    /// The chosen move, worded. Moves cross this wire PRE-WORDED: the client's
    /// standing rule is that it never constructs a move and never prices one,
    /// and typing `Move` in the browser would be a hand-written union mirroring
    /// the engine — the `ts-rs` question, which M3 deliberately does not force.
    pub text: String,
    pub explanation: Explanation,
    /// The principal continuation, root move first — so step 1 is the answer
    /// again. Carried as the search reported it rather than trimmed, because
    /// the terminal shows it that way.
    pub continuation: Vec<String>,
    pub ranking: Vec<Ranked>,
    /// The stream this press was keyed by, as a STRING — a `u64` past 2^53 is
    /// not safe as a JSON number in a browser, the same rule the setup seed
    /// already follows.
    pub stream_seed: String,
    /// The ply guard stopped SOME continuation in this search — **not
    /// necessarily the one in `continuation`**. The counter carries no identity
    /// and the trace does not record why a line ended, so this says the weaker
    /// true thing rather than the stronger convenient one.
    pub search_hit_ply_cap: bool,
    /// How many candidates tie with the suggestion at the top.
    pub tied_at_top: usize,
}

impl Suggest {
    pub fn of(s: &Suggestion, state: &GameState, rules: &Rules) -> Self {
        Self {
            seat: s.seat,
            text: yokohama_cli::describe::describe_move(&s.mv, state, rules),
            explanation: Explanation::of(&s.explanation),
            continuation: word_line(&s.continuation, state, rules),
            ranking: rank_all(&s.ranking, &s.mv, state, rules),
            stream_seed: s.stream_seed.to_string(),
            search_hit_ply_cap: s.search_hit_ply_cap,
            tied_at_top: s.tied_at_top(),
        }
    }
}

/// What the reconstruction claims, so the arithmetic beside it has a claim
/// attached.
#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    /// The contestant's name, as `descriptor()` composes it — one display
    /// form for every contestant, so the browser and the terminal cannot spell
    /// the same bot differently.
    pub contestant: String,
    /// The controller impl version THIS build runs. When `recorded_version`
    /// differs, the move still reproduced — the account is this build's reading
    /// of a move an older one played, and the client says so.
    pub controller_version: u32,
    /// `eval::EVAL_IMPL_VERSION`: a build-wide constant, not a per-contestant
    /// one.
    pub evaluator_version: u32,
    /// The log's printed 0-based event index.
    pub index: usize,
    pub seat: PlayerId,
    /// The key the bot's own pump decided under, as a STRING.
    pub decision_seed: String,
    /// The engine's legal list held exactly one move at the decision state, so
    /// the valuation below is an account of a move nothing else was competing
    /// with.
    pub forced: bool,
    pub recorded_version: Option<u32>,
    pub current_version: u32,
}

/// `GET /api/games/{id}/why` — a bot move, reconstructed by replay.
///
/// **Exactly one of `ok` and `refusal` is non-null**, and a refusal is a 200:
/// `why` declining because the last move was a human's is the ANSWER, not a
/// client error. The twelve typed refusals are data the browser renders, and
/// M2.5's finding was that a branch nobody has ever seen is exactly where a
/// defect lives.
#[derive(Debug, Clone, Serialize)]
pub struct Why {
    pub ok: Option<WhyOk>,
    pub refusal: Option<RefusalDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhyOk {
    pub provenance: Provenance,
    pub text: String,
    pub explanation: Explanation,
    pub continuation: Vec<String>,
    /// `None` for a greedy contestant, which searches no line at all — a
    /// different fact from an empty count, so it is not flattened to zeros.
    pub counters: Option<Vec<Counter>>,
    pub search_hit_ply_cap: bool,
    pub ranking: Vec<Ranked>,
}

impl Why {
    pub fn granted(a: &WhyAccount, rules: &Rules) -> Self {
        let e: &DecisionExplanation = &a.explain;
        Self {
            ok: Some(WhyOk {
                provenance: Provenance::of(&a.reconstruction),
                text: yokohama_cli::describe::describe_move(&a.mv, &a.state, rules),
                explanation: Explanation::of(&e.explanation),
                continuation: word_line(&e.continuation, &a.state, rules),
                counters: e.stats.as_ref().map(counters),
                search_hit_ply_cap: e.hit_ply_cap,
                ranking: rank_all(&e.ranking, &a.mv, &a.state, rules),
            }),
            refusal: None,
        }
    }

    pub fn refused(r: &Refusal) -> Self {
        Self { ok: None, refusal: Some(RefusalDto::of(r)) }
    }
}

impl Provenance {
    fn of(r: &Reconstruction) -> Self {
        let (controller, _, _) = r.contestant.descriptor();
        Self {
            contestant: controller,
            controller_version: r.current_version,
            evaluator_version: yokohama_core::game::eval::EVAL_IMPL_VERSION,
            index: r.index,
            seat: r.seat,
            decision_seed: format!("{:#018x}", r.decision_seed),
            forced: r.forced,
            recorded_version: r.recorded_version,
            current_version: r.current_version,
        }
    }
}

/// A typed refusal, tagged and worded.
///
/// **`message` is the CLI's own `Display` for the variant, never re-authored.**
/// `guide_sync` pins the tester guide to those strings, so a second wording here
/// would drift from the screen the guide documents — and the two frontends would
/// be refusing the same thing in different words.
///
/// `kind` is a stable snake_case tag, and `RefusalKind` below is what makes it
/// impossible to add a variant and forget one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefusalDto {
    pub kind: &'static str,
    pub message: String,
}

impl RefusalDto {
    pub fn of(r: &Refusal) -> Self {
        Self { kind: kind_of(r), message: r.to_string() }
    }
}

/// The wire tag for every refusal `why` can raise.
///
/// **A `match` with NO `_` arm, deliberately.** This is the same fence M2.5 put
/// under `PromptState` after the review found that surface's "compile error"
/// guarantee was a hand-written list that a tenth variant walked straight past.
/// A thirteenth `Refusal` variant fails to COMPILE here, which is the only kind
/// of exhaustiveness guarantee worth stating.
pub fn kind_of(r: &Refusal) -> &'static str {
    match r {
        Refusal::LiveMode => "live_mode",
        Refusal::NoBotMoveYet => "no_bot_move_yet",
        Refusal::OutOfRange { .. } => "out_of_range",
        Refusal::NotAMove { .. } => "not_a_move",
        Refusal::HumanSeat { .. } => "human_seat",
        Refusal::Rung { .. } => "rung",
        Refusal::Diagnostic { .. } => "diagnostic",
        Refusal::UnknownController { .. } => "unknown_controller",
        Refusal::Mismatch(_) => "mismatch",
        Refusal::Reconstruction { .. } => "reconstruction",
        Refusal::Unaccounted { .. } => "unaccounted",
        Refusal::Replay { .. } => "replay",
    }
}

/// Every tag `kind_of` can return, as data.
///
/// The LOAD-BEARING half of the fence: the `match` above proves every variant
/// has a tag, and this proves the corpus can be checked against the full set.
/// The two are pinned to each other by
/// `every_refusal_tag_is_produced_by_exactly_one_variant`, so neither can go
/// stale while the other moves — which is precisely how the M2.5 review's
/// vacuous guard failed.
pub const REFUSAL_KINDS: [&str; 12] = [
    "live_mode",
    "no_bot_move_yet",
    "out_of_range",
    "not_a_move",
    "human_seat",
    "rung",
    "diagnostic",
    "unknown_controller",
    "mismatch",
    "reconstruction",
    "unaccounted",
    "replay",
];

/// `suggest`'s own refusals, which do NOT live in `suggest.rs`.
///
/// Three of the four are raised by `repl::suggest_cmd` — the terminal's command
/// wrapper — and the fourth by `Repl::step`'s lockdown gate, so a handler that
/// called `suggest::suggest` directly would inherit NONE of them and would
/// happily search a live game, which the module doc forbids. They are restated
/// here rather than reached because the alternative is widening a
/// rendering-adjacent function that returns a rendered `StepOutput`; the
/// conditions are one line each and every one is asserted by a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestRefusal {
    LiveMode,
    GameOver,
    BotSeat,
    Locked,
}

impl SuggestRefusal {
    pub fn kind(self) -> &'static str {
        match self {
            Self::LiveMode => "live_mode",
            Self::GameOver => "game_over",
            Self::BotSeat => "bot_seat",
            Self::Locked => "locked",
        }
    }

    /// The same four refusals the terminal gives, UNWRAPPED.
    ///
    /// The CLI's are hand-wrapped to a view body — a 215-column paragraph would
    /// size the composed frame to itself — and carrying those embedded newlines
    /// into a browser would be shipping a terminal's line-breaks to a layout
    /// engine. So the sentences say the same thing in the same order and the
    /// wrapping is the reader's. **This is the one place M3 re-words rather than
    /// reuses**, and it is deliberate: `Refusal`'s twelve are carried verbatim
    /// because `guide_sync` pins them, while these four live inside
    /// `repl::suggest_cmd`'s rendered output and have no such pin.
    pub fn message(self) -> &'static str {
        match self {
            Self::LiveMode => {
                "`suggest` is simulation-only. A live-tracked game's hidden cards are a model \
                 reconciled from the table, not the engine's own deck, so the bot would be \
                 searching a position nobody is playing."
            }
            Self::GameOver => "The game is over — there is no move to suggest.",
            Self::BotSeat => {
                "That seat is played by a bot, and `suggest` answers for YOUR seat only — \
                 answering for a bot's seat would be advising it against you. Ask `why` after \
                 it plays to see the account of the move it chose."
            }
            Self::Locked => {
                "The session is locked after a driver or engine fault and accepts no gameplay."
            }
        }
    }
}

/// The `suggest` payload, refusal included, in the shape `why` uses.
#[derive(Debug, Clone, Serialize)]
pub struct SuggestResponse {
    pub ok: Option<Suggest>,
    pub refusal: Option<RefusalDto>,
}

impl SuggestResponse {
    pub fn granted(s: Suggest) -> Self {
        Self { ok: Some(s), refusal: None }
    }

    pub fn refused(kind: &'static str, message: impl Into<String>) -> Self {
        Self { ok: None, refusal: Some(RefusalDto { kind, message: message.into() }) }
    }
}


/// A feature's snake_case token, in words a player can read.
///
/// **Authored HERE, and deliberately not in `yokohama-core`.** `Feature::name()`
/// returns the stable token and its doc says report columns, explanation
/// rendering and the probe all come from there "so none of them is hand-typed";
/// a `Feature::label()` beside it would be a core edit on a crate this lane
/// keeps byte-identical to master, and a wording change would move a file the
/// evaluator's own tests read. Wording is presentation, and presentation is the
/// web layer's — which is where every other enum→string map on this API already
/// lives.
///
/// **The token stays the identity on the wire.** These words are served from
/// `/api/labels` beside them, never in place of them: the snake_case name is
/// the cross-reference against `eval_probe`, the scenario suite and every
/// recorded figure, and a reader comparing this screen with a recorded number
/// needs to be able to grep for it.
///
/// The list is EXHAUSTIVE over `Feature::ALL` and a test says so, so a
/// thirty-fifth feature cannot reach the browser as a bare token.
pub fn feature_labels() -> std::collections::BTreeMap<String, String> {
    yokohama_core::game::eval::Feature::ALL
        .iter()
        .map(|f| (f.name().to_string(), words_for(f.name()).to_string()))
        .collect()
}

fn words_for(name: &str) -> &'static str {
    match name {
        "victory_points" => "Victory points",
        "country_bonus_now" => "Country-set bonus, if the game ended now",
        "tech_bonus_now" => "Technology majority, if the game ended now",
        "church_bonus_now" => "Church majority, if the game ended now",
        "customs_bonus_now" => "Customs majority, if the game ended now",
        "assistants_in_hand_core" => "Assistants in hand (the working core)",
        "assistants_in_hand_excess" => "Assistants in hand beyond what a turn can spend",
        "assistants_in_warehouse" => "Assistants in the warehouse",
        "assistants_on_board" => "Assistants on the board",
        "shophouses_built" => "Shophouses built",
        "trading_houses_built" => "Trading houses built",
        "five_power_tokens" => "5-Power tokens taken",
        "imported_goods" => "Imported goods",
        "achievement_progress_sum" => "Progress toward the achievements",
        "mercenaries_total" => "Mercenaries held",
        "hand_size" => "Order cards in hand",
        "production_access" => "Areas whose production this seat can reach",
        "track_access" => "Church and customs spaces this seat can reach",
        "assistant_areas_distinct" => "Distinct areas holding an assistant",
        "president_local_access" => "What the President's own area offers",
        "bank_access" => "Reach to the Bank",
        "employment_agency_access" => "Reach to the Employment Agency",
        "president_reachable_areas" => "Areas the President can reach",
        "money_stock" => "Yen",
        "trade_goods_stock" => "Trade goods held",
        "building_tokens_ready" => "Building tokens ready to place",
        "held_order_value" => "Value of the orders in hand",
        "order_matched_goods" => "Goods that match an order in hand",
        "executable_orders" => "Orders that could be filled now",
        "flag_icon_marginal" => "The next flag icon's marginal worth",
        "held_order_icons" => "Icons the orders in hand ask for",
        "lab_takeable" => "Technologies this seat could take",
        "port_takeable" => "Port cards this seat could take",
        "matched_production_access" => "Production this seat can reach that an order wants",
        // No `_` fallback to a prettified token: a feature added to the engine
        // and not to this list should be caught by the test below, not shipped
        // as a guess. `Feature::ALL` is the closed set the test walks.
        other => panic!("feature {other:?} has no wording — add it to `words_for`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tag list and the `match` are pinned TO EACH OTHER, so neither can go
    /// stale while the other moves.
    ///
    /// This is the cell M2.5's review would have wanted and did not find: there,
    /// the "a new variant is a compile error" claim rested on two hand-written
    /// lists, a reviewer added a tenth variant, and all three tests passed. Here
    /// the count is load-bearing and `kind_of` has no `_` arm, so a thirteenth
    /// variant cannot reach the wire without failing to compile.
    #[test]
    fn every_refusal_tag_is_produced_by_exactly_one_variant() {
        let produced = [
            kind_of(&Refusal::LiveMode),
            kind_of(&Refusal::NoBotMoveYet),
            kind_of(&Refusal::OutOfRange { asked: 0, events: 0 }),
            kind_of(&Refusal::NotAMove { index: 0, kind: "x" }),
            kind_of(&Refusal::HumanSeat { index: 0, seat: PlayerId::new(0) }),
            kind_of(&Refusal::Rung { index: 0, seat: PlayerId::new(0), rung: String::new() }),
            kind_of(&Refusal::Diagnostic { index: 0, seat: PlayerId::new(0), why: String::new() }),
            kind_of(&Refusal::UnknownController {
                index: 0,
                seat: PlayerId::new(0),
                controller: String::new(),
                why: String::new(),
            }),
            kind_of(&Refusal::Mismatch(Box::new(yokohama_cli::why::Mismatch {
                index: 0,
                seat: PlayerId::new(0),
                controller: String::new(),
                reconstructed: String::new(),
                recorded: String::new(),
                recorded_version: None,
                current_version: 0,
            }))),
            kind_of(&Refusal::Reconstruction {
                index: 0,
                seat: PlayerId::new(0),
                why: String::new(),
            }),
            kind_of(&Refusal::Unaccounted { index: 0, contestant: String::new() }),
            kind_of(&Refusal::Replay { index: 0, message: String::new() }),
        ];

        let mut sorted = produced.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), produced.len(), "two variants share a tag: {produced:?}");

        let mut declared = REFUSAL_KINDS.to_vec();
        declared.sort_unstable();
        let mut actual = produced.to_vec();
        actual.sort_unstable();
        assert_eq!(actual, declared, "REFUSAL_KINDS is not what kind_of produces");
    }

    /// A refusal is worded by the CLI, not here.
    #[test]
    fn a_refusal_carries_the_clis_own_sentence() {
        let r = Refusal::LiveMode;
        let dto = RefusalDto::of(&r);
        assert_eq!(dto.kind, "live_mode");
        assert_eq!(dto.message, r.to_string(), "the wording is the CLI's Display, verbatim");
        assert!(dto.message.contains("simulation-only"));
    }

    /// The four `suggest` refusals are distinct and each says something.
    #[test]
    fn every_suggest_refusal_has_its_own_tag_and_sentence() {
        let all = [
            SuggestRefusal::LiveMode,
            SuggestRefusal::GameOver,
            SuggestRefusal::BotSeat,
            SuggestRefusal::Locked,
        ];
        let mut kinds: Vec<&str> = all.iter().map(|r| r.kind()).collect();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), all.len(), "two suggest refusals share a tag");
        for r in all {
            assert!(r.message().len() > 20, "{:?} says nothing useful", r.kind());
        }
    }

    /// Every feature the evaluator has can be WORDED, and the panic arm proves
    /// the list is closed rather than defaulted.
    ///
    /// **The fence is the `panic!` arm in `words_for`, not this test.** A
    /// `_ => prettify(token)` fallback would let a thirty-fifth feature reach
    /// the browser looking like a label while nobody had chosen its words —
    /// the same silent-fallback shape the M2.5 record names for the four
    /// uncovered queued-choice heads. Here the set is closed and knowable, so
    /// it is closed, and this test is what walks it.
    ///
    /// What this test deliberately does NOT assert is whether the wording is
    /// GOOD. An earlier draft tried: it rejected any label equal to the
    /// underscore-stripped token, on the theory that such a label had been
    /// prettified rather than chosen. It failed on `imported_goods` →
    /// "Imported goods", which is simply the right answer. A test that cannot
    /// tell a good label from a lazy one should not pretend to — the mechanical
    /// property (every feature has words, chosen by a human, or the build
    /// panics) is the one worth pinning.
    #[test]
    fn every_feature_the_evaluator_scores_has_words() {
        let labels = feature_labels();
        assert_eq!(
            labels.len(),
            yokohama_core::game::eval::Feature::ALL.len(),
            "one wording per feature, no duplicates by token"
        );
        assert_eq!(labels.len(), 34, "the evaluator's feature vector is 34 wide");
        for (token, words) in &labels {
            assert!(!words.is_empty(), "{token} has no wording");
            assert_ne!(words, token, "{token} was not worded, only echoed");
            // Not `is_uppercase`: "5-Power tokens taken" starts with a digit and
            // is correctly worded. What is being ruled out is a token that was
            // lower-cased and passed off as prose.
            assert!(
                !words.chars().next().is_some_and(char::is_lowercase),
                "{token}'s wording reads as a label: {words:?}"
            );

        }
    }

    /// The counter grouping covers the fifteen the engine actually has, and an
    /// unknown name lands in `ungrouped` rather than vanishing.
    #[test]
    fn every_search_counter_is_grouped_and_an_unknown_one_is_still_shown() {
        let stats = yokohama_core::game::lookahead::SearchStats::default();
        let rows = counters(&stats);
        assert_eq!(rows.len(), 15, "the engine's counter block is fifteen wide");
        assert!(
            rows.iter().all(|c| c.group != "ungrouped"),
            "every counter this build has is grouped: {:?}",
            rows.iter().filter(|c| c.group == "ungrouped").collect::<Vec<_>>()
        );
        assert_eq!(group_of("a_counter_from_the_future"), "ungrouped");
    }
}
