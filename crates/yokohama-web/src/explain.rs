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
/// 1. **Every one of the 34 terms is here, including the zero-product ones** —
///    but NOT because the sum needs them. An earlier version of this doc said
///    "the identity only holds over all of them", and a reviewer did the
///    arithmetic: a zero-product term contributes zero, so the sum over the
///    carrying terms alone is the same number. Measured on both fixtures.
///
///    The real reason is `Explanation`'s own, and it is about the READER:
///    reporting one term per feature always lets a reader "see that a term
///    contributed nothing rather than wonder whether it was computed", and it
///    is what makes the client's "8 of 34" a count of something it is showing
///    rather than of something it was told. A payload that shipped only the
///    carrying terms could still foot; it could not be audited.
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
    /// How many of `terms` carry a nonzero product — **feature terms only**.
    ///
    /// An earlier version of this doc said the count was computed here "because
    /// the CLI's summary line already states it and the two must not be able to
    /// disagree". That was wrong, and a review measured the disagreement: the
    /// terminal counts STORY ROWS, and on a finished game it deliberately pushes
    /// the `final standing` row through the same counter (`suggest.rs:783-806`,
    /// with a comment saying a count over feature terms alone "would read 0 of
    /// 34 over a table whose whole content sits in the row it skipped"). So the
    /// terminal reads `1 of 35` there and this field reads `0` over 34 terms.
    ///
    /// **Both are right about different things**, and the field is defined as
    /// the narrower one on purpose: it is a fact about `terms`, and a client
    /// with `terminal` non-null must draw the standing rather than a bar chart
    /// anyway. What is NOT true is that the two numbers are the same number.
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
/// which is exactly what `Suggestion::tied_at_top` exists to report. A client
/// that dropped row 0 to get the runner-ups would therefore list the move the
/// bot actually played as something it beat, and hide a real runner-up,
/// whenever the draw lands past row 0. Where it lands is the fixture's and
/// moves with the stream's key: A1 re-keyed the stream and the committed
/// fixture's draw landed on the top row, so the fixture no longer separates
/// "the non-chosen rows" from "the tail". The fence that does is the client
/// cell that swaps two `chosen` flags of the committed payload
/// (`Explain.test.tsx`, "the runner-ups are the non-chosen rows, not the
/// tail"). So the server marks it, rather than leaving the browser to infer
/// identity from a display string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ranked {
    pub text: String,
    pub value: i64,
    pub margin: i64,
    pub chosen: bool,
}

/// Whose hand the ranking's candidates are drawn from, and therefore whether
/// their wording may name the cards in it.
///
/// **The root candidate list is a list of moves a seat COULD have made, and at
/// an AdditionalAction node that enumerates its hand.** `Move::FulfillOrder`'s
/// only generator reads `p.order_cards_in_hand` (`game/legal_moves.rs`), and
/// `describe_move` words it as the printed card — country, requirements,
/// rewards. So a ranking served whole is a readout of the mover's hand.
///
/// The distinction is not "is it a bot", it is WHOSE hand:
/// - `suggest` runs on YOUR seat's view and ranks YOUR moves. Your own hand is
///   already on your own screen; there is nothing to hide.
/// - `why` accounts for a move ANOTHER seat made. `build_dashboard` serves that
///   seat's hand as `cards: Vec::new()`, so one route hid it by construction
///   while a sibling route listed its contents.
///
/// **The move that was PLAYED is never redacted**, in either mode: it happened,
/// so it is public, and the card it names has left the hand. Only the roads not
/// taken are hidden, which is exactly the information the mask exists to keep.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HandVisibility {
    /// The ranking is the viewer's own; word every candidate.
    Own,
    /// The ranking belongs to another seat; word the unplayed candidates
    /// without naming cards still in that seat's hand.
    Masked,
}

/// What an unplayed candidate says when its wording would name a hidden card.
///
/// Deliberately keeps the ROW — its value and margin are the shape of the
/// bot's preference, which is the whole point of the panel and reveals nothing
/// about which card it is. Only the identity goes.
const MASKED_FULFILL: &str = "Fulfill an order from hand";

fn rank_all(
    ranking: &[RootScore],
    chosen: &Move,
    state: &GameState,
    rules: &Rules,
    hand: HandVisibility,
) -> Vec<Ranked> {
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
            // `FulfillOrder` is the ONLY `Move` variant that carries an
            // `OrderCardId` (`types/moves.rs`) and the only `describe_move` arm
            // that reads `rules.orders` — so this match is the whole surface,
            // not a sample of it.
            let hides_a_card =
                hand == HandVisibility::Masked && !is_chosen && matches!(r.mv, Move::FulfillOrder { .. });
            Ranked {
                text: if hides_a_card {
                    MASKED_FULFILL.to_string()
                } else {
                    yokohama_cli::describe::describe_move(&r.mv, state, rules)
                },
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

/// `why`'s continuation, worded with its fulfil lines masked as [`rank_all`]
/// masks the ranking's rows (finding B1's third channel): the FIRST step is the
/// move PLAYED — public, never masked, a fulfil included — and a later
/// `FulfillOrder` step may name a card still in the mover's hand, so it reads
/// as [`MASKED_FULFILL`]. A later fulfil may also name a card the mover took
/// from the PUBLIC port earlier in the same line ("Port: take O19", then the
/// fulfil of O19): it is masked too — over-masking, which is safe, and the
/// mask does not try to tell the two apart.
fn word_line_masked(line: &[Move], state: &GameState, rules: &Rules) -> Vec<String> {
    line.iter()
        .enumerate()
        .map(|(i, m)| {
            if i > 0 && matches!(m, Move::FulfillOrder { .. }) {
                MASKED_FULFILL.to_string()
            } else {
                yokohama_cli::describe::describe_move(m, state, rules)
            }
        })
        .collect()
}

/// Whether the mover's account may be served to the requester (the 2026-10-08
/// finding B1). The account — the feature table and its total, the
/// continuation, the ranking's values, the counters, the forced fact — is a
/// function of the mover's HAND: while the mover's starting order is hidden
/// from the requester, every one of them narrows the unseen cards to it (the
/// review read the card off four feature values from one payload). So the
/// account is WITHHELD until the card is public — or, to a seat holder, until
/// the game is over — and the `withheld` half carries the move and the claim
/// alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Withheld,
}

/// [`Visibility`] of `mover`'s account to a requester who holds `held` (empty
/// for a spectator) at `now` — **the engine's own view, plus the record
/// route's own game-over condition — nothing else restated here**: `Withheld`
/// exactly while `observe(now, seat)` for the requester shows a
/// `HiddenStartingOrder` slot in the mover's hand, unless the game is over and
/// the requester holds a seat. A holder of the mover's seat reads their own
/// hand; a spectator reads as any seat that is not the mover's, since every
/// non-mover seat sees the same masked hand (`observe.rs`); a SEAT HOLDER at
/// game over reads everything — the record route serves them the whole
/// record, seed included, from there, and refuses a spectator, so a watcher
/// reads the engine's view at game over too. A view the engine refuses is a
/// view that shows nothing: withheld.
pub fn visibility_of(now: &GameState, mover: PlayerId, held: &[PlayerId]) -> Visibility {
    if yokohama_core::is_game_over(now) && !held.is_empty() {
        return Visibility::Public;
    }
    if held.contains(&mover) {
        return Visibility::Public;
    }
    let as_seat = held
        .first()
        .copied()
        .or_else(|| now.players.iter().map(|p| p.id).find(|id| *id != mover));
    let Some(as_seat) = as_seat else { return Visibility::Withheld };
    match yokohama_core::game::observe(now, as_seat) {
        Ok(view) => {
            let masked = view
                .players
                .iter()
                .find(|p| p.id == mover)
                .is_some_and(|p| p.hand.contains(&yokohama_core::HandCard::HiddenStartingOrder));
            if masked { Visibility::Withheld } else { Visibility::Public }
        }
        Err(_) => Visibility::Withheld,
    }
}

/// The one sentence the withheld half carries.
pub const WITHHELD_REASON: &str = "the bot's account is withheld while its starting order is hidden from you — \
                                   it is served once that card is public, or to a seat holder once the game is over";

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
///
/// **No stream key** (security S1a, finding A1): the key this press was made
/// under was served as a string and was a bijective fold of the setup seed —
/// the first member of the hidden set — so any seat holder inverted it from
/// one request. The CLI's derivation is one-way since the same cycle and the
/// web wire carries nothing of it; the press is still byte-identical on
/// repeat, which is the property a reader could use the key for.
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
            // `suggest` runs on YOUR seat's view and ranks YOUR moves, so the
            // candidates enumerate a hand already on your own screen.
            ranking: rank_all(&s.ranking, &s.mv, state, rules, HandVisibility::Own),
            search_hit_ply_cap: s.search_hit_ply_cap,
            tied_at_top: s.tied_at_top(),
        }
    }
}

/// What the reconstruction claims, so the arithmetic beside it has a claim
/// attached — and NOTHING the search found: no key (the pump's
/// `decision_seed` inverted to the bot's stream seed, finding B1) and no
/// forced fact, which is the account's (`WhyOk::forced`).
#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    /// The contestant, as `Contestant::echo()` composes it — `name@vN` or
    /// `name@vN{k=v,…}`.
    ///
    /// **`echo()` and not `descriptor().0`.** The first draft sent the bare
    /// name and its doc claimed that was "one display form for every
    /// contestant, so the browser and the terminal cannot spell the same bot
    /// differently" — while the CLI's own `why` header prints `echo()`. For the
    /// production contestant the knobs are constants of the name, so nothing was
    /// lost in practice; a record resumed from a CLI save with a non-default
    /// `lookahead-v1` seat would have rendered as an undifferentiated
    /// `lookahead-v1` in the browser and `lookahead-v1@v2{b=8,cap=200,…}` in the
    /// terminal. `echo()` is the one composer, so now it cannot.
    pub contestant: String,
    /// `eval::EVAL_IMPL_VERSION`: a build-wide constant, not a per-contestant
    /// one.
    pub evaluator_version: u32,
    /// The log's printed 0-based event index.
    pub index: usize,
    pub seat: PlayerId,
    pub recorded_version: Option<u32>,
    pub current_version: u32,
}

/// `GET /api/games/{id}/why` — a bot move, reconstructed by replay.
///
/// **Exactly one of `ok`, `withheld` and `refusal` is non-null**, and neither
/// of the last two is a 4xx: `why` declining because the last move was a
/// human's is the ANSWER, and so is `why` withholding the account while the
/// mover's starting order is hidden from the requester (finding B1;
/// [`Visibility`]). The twelve typed refusals are data the browser renders,
/// and M2.5's finding was that a branch nobody has ever seen is exactly where
/// a defect lives.
#[derive(Debug, Clone, Serialize)]
pub struct Why {
    pub ok: Option<WhyOk>,
    pub withheld: Option<WhyWithheld>,
    pub refusal: Option<RefusalDto>,
}

/// The withheld half: the move played, the claim, and the sentence. Nothing
/// the search found — pinned as bytes by
/// `a_withheld_account_is_the_same_bytes_whatever_the_search_found`.
#[derive(Debug, Clone, Serialize)]
pub struct WhyWithheld {
    pub provenance: Provenance,
    pub text: String,
    pub reason: &'static str,
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
    /// The engine's legal list held exactly one move at the decision state,
    /// so the valuation above is an account of a move nothing else was
    /// competing with. On the ACCOUNT since security S1a: a fact about the
    /// mover's hand (a fulfil in the list is a card in the hand), withheld with
    /// the rest.
    pub forced: bool,
}

impl Why {
    /// The account at `visibility` ([`visibility_of`] is the route's reading).
    pub fn account(a: &WhyAccount, rules: &Rules, visibility: Visibility) -> Self {
        let provenance = Provenance::of(&a.reconstruction);
        let text = yokohama_cli::describe::describe_move(&a.mv, &a.state, rules);
        match visibility {
            Visibility::Withheld => Self {
                ok: None,
                withheld: Some(WhyWithheld { provenance, text, reason: WITHHELD_REASON }),
                refusal: None,
            },
            Visibility::Public => {
                let e: &DecisionExplanation = &a.explain;
                Self {
                    ok: Some(WhyOk {
                        provenance,
                        text,
                        explanation: Explanation::of(&e.explanation),
                        continuation: word_line_masked(&e.continuation, &a.state, rules),
                        counters: e.stats.as_ref().map(counters),
                        search_hit_ply_cap: e.hit_ply_cap,
                        // `why` accounts for a move ANOTHER seat made, and the same
                        // server serves that seat's hand as `cards: Vec::new()`.
                        ranking: rank_all(&e.ranking, &a.mv, &a.state, rules, HandVisibility::Masked),
                        forced: a.reconstruction.forced,
                    }),
                    withheld: None,
                    refusal: None,
                }
            }
        }
    }

    pub fn refused(r: &Refusal) -> Self {
        Self { ok: None, withheld: None, refusal: Some(RefusalDto::of(r)) }
    }

    /// A locked session. Not one of `why`'s twelve — the lockdown is the
    /// DRIVER's state rather than a fact about an event — so it carries its own
    /// tag and the driver's own sentence.
    pub fn locked(reason: String) -> Self {
        Self {
            ok: None,
            withheld: None,
            refusal: Some(RefusalDto {
                kind: "locked",
                message: format!(
                    "The session is locked after a driver or engine fault, so there is nothing \
                     here worth trusting: `why` would re-run the bot's search against the record \
                     that faulted. ({reason})"
                ),
            }),
        }
    }

    /// A move made by `played`, a bot this build does not run (spec §2.4's
    /// second mask), and `why` it does not — the CLI's reason, cut to its
    /// clause ([`crate::bots::Runs::No`]; cycle F, spec §2.3: every refusal
    /// keeps its reason). Answered BEFORE `why::account_parts`, which would
    /// read the converted seat as "a controller this driver will not run …
    /// resumed as a HUMAN seat" — a CLI fact that is false on the web, where
    /// nothing converts a bot's seat. A deliberate exception of `suggest`'s
    /// kind: the twelve `Refusal`s stay `account_parts`' own strings, carried
    /// verbatim.
    pub fn foreign(played: &str, why: &str) -> Self {
        Self {
            ok: None,
            withheld: None,
            refusal: Some(RefusalDto {
                kind: FOREIGN_BOT_KIND,
                message: format!("This move was made by {played}, which this server doesn't run: {why}"),
            }),
        }
    }
}

/// The tag of `why`'s THIRTEENTH refusal shape (M6.5c2, R-6.5c2.10): a move
/// made by a bot this build does not run — since cycle F, a bot the CLI's
/// resume does not seat exactly ([`crate::bots::foreign`]), so a move by a
/// runnable bot that is not the production one is reconstructed by replay like
/// any other. The layer's own, not the CLI's, so it
/// is outside [`kind_of`]'s fence and [`refusal_samples`]' list — the corpus
/// carries it as a sample of its own (`why_foreign.json`).
pub const FOREIGN_BOT_KIND: &str = "foreign_bot";

/// `why`'s foreign-bot pre-empt: `Some` when the event `why` addresses is a
/// move made by a FOREIGN bot seat of `record`, `None` to let
/// `why::account_parts` answer.
///
/// - **Bare `why`** addresses the most recent bot move, and is answered here
///   with the FIRST foreign seat's reason whenever the record has one. A web
///   table deals every bot one contestant under one seed
///   (`tables::contestants_of`), so in a record a web table made the bots are
///   foreign together or not at all, and that seat is the one bare `why`
///   means. A record made elsewhere — one whose later bot seat carries a
///   second stream seed, say, which the CLI's resume holds against that seat
///   alone — can carry a runnable seat beside a foreign one, and bare `why`
///   then answers with the foreign seat's reason even where the runnable seat
///   moved last: a shape no web table reaches.
/// - **`why <n>`** is answered here only when event `n` is a move whose mover
///   — `state_at(record, n).current_player`, the CLI's own attribution — is a
///   foreign seat. A human's move, an event out of range, a setup reveal or a
///   replay that fails are `account_parts`' to word.
pub fn foreign_why(record: &yokohama_core::GameRecord, rules: &Rules, event: Option<usize>) -> Option<Why> {
    let foreign = crate::bots::foreign(record);
    let (_, first, first_why) = foreign.first()?;
    // The `played` half of the refusal's pair (R-C5): a bot that shares this
    // build's label is told apart by its version, then by its knobs, so a
    // same-name re-point reads "made by V4.2" — never "made by V4" on a server
    // that runs V4.
    let running = crate::bots::production();
    let played = |d: &crate::bots::BotDescriptor| crate::bots::distinguishing_labels(d, &running).0;
    let Some(n) = event else { return Some(Why::foreign(&played(first), first_why)) };
    if !matches!(record.events.get(n), Some(yokohama_core::RecordedEvent::Move(_))) {
        return None;
    }
    let mover = yokohama_core::state_at(record, n, rules).ok()?.current_player.0;
    foreign.iter().find(|(seat, _, _)| *seat == mover).map(|(_, d, why)| Why::foreign(&played(d), why))
}

impl Provenance {
    fn of(r: &Reconstruction) -> Self {
        Self {
            contestant: r.contestant.echo(),
            evaluator_version: yokohama_core::game::eval::EVAL_IMPL_VERSION,
            index: r.index,
            seat: r.seat,
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

/// **The one hand-written list of refusals, and the only one.**
///
/// An adversarial review of this milestone found the fence it replaces was
/// weaker than its own doc claimed. That doc described a three-link chain — a
/// 13th variant fails to compile in [`kind_of`]; adding an arm there but not the
/// corpus fails the fixture test; adding it to the corpus but not to the tag
/// list fails a third — and the MIDDLE LINK did not exist. There were THREE
/// hand-written lists (the tag array, the test's constructed samples, and the
/// fixture builder's own vec), all agreeing with each other and none pinned to
/// the enum, so a 13th variant plus a one-line `kind_of` arm reached the wire
/// with all 54 tests green. That is exactly the shape M2.5's review had found
/// and this had claimed to fix.
///
/// **What is true now, stated precisely rather than optimistically:**
///
/// 1. A new `Refusal` variant is a COMPILE ERROR in [`kind_of`] — a `match`
///    with no `_` arm, mutation-proven. That link is real and always was.
/// 2. Everything downstream — `REFUSAL_KINDS`, the committed corpus, the
///    client's coverage test — now derives from THIS FUNCTION, so they cannot
///    disagree with each other and there is one place to edit rather than three.
/// 3. **The residual hole, named:** a variant added to `Refusal` AND to
///    `kind_of` but NOT to the list below still reaches the wire untagged by
///    any fixture. Plain Rust cannot close it — there is no stable way to
///    enumerate an enum's variants — so closing it would mean a derive
///    dependency (`strum`) on `yokohama-cli`, which is a dependency decision and
///    not a test change. Until then the guard is that link 1 forces a reader to
///    open this file, and this comment is what they find when they do.
///
/// The field values are placeholders and are deliberately obvious as such: what
/// this list pins is the SHAPE and the SET, not any particular message.
pub fn refusal_samples() -> Vec<Refusal> {
    use yokohama_cli::why::Mismatch;
    let seat = PlayerId::new(1);
    vec![
        Refusal::LiveMode,
        Refusal::NoBotMoveYet,
        Refusal::OutOfRange { asked: 99, events: 12 },
        Refusal::NotAMove { index: 3, kind: "a setup event" },
        Refusal::HumanSeat { index: 4, seat },
        Refusal::Rung { index: 5, seat, rung: "family-weighted".to_string() },
        Refusal::Diagnostic { index: 6, seat, why: "a controller this driver will not run".into() },
        Refusal::UnknownController {
            index: 7,
            seat,
            controller: "not-a-contestant".to_string(),
            why: "no such controller".to_string(),
        },
        Refusal::Mismatch(Box::new(Mismatch {
            index: 8,
            seat,
            controller: "lookahead".to_string(),
            reconstructed: "MovePresident { .. }".to_string(),
            recorded: "place 1 assistant in Area 3".to_string(),
            recorded_version: Some(1),
            current_version: 2,
        })),
        Refusal::Reconstruction { index: 9, seat, why: "the sampler refused the view".to_string() },
        Refusal::Unaccounted { index: 10, contestant: "greedy-v1".to_string() },
        Refusal::Replay { index: 11, message: "the record does not replay".to_string() },
    ]
}

/// Every tag `kind_of` can return, DERIVED from [`refusal_samples`] rather than
/// restated beside it. See that function for what this fence does and does not
/// catch.
pub fn refusal_kinds() -> Vec<&'static str> {
    refusal_samples().iter().map(kind_of).collect()
}


/// `suggest`'s own refusals, which do NOT live in `suggest.rs`.
///
/// Three of the four are raised by `repl::suggest_cmd` — the terminal's command
/// wrapper — and the fourth by `Repl::step`'s lockdown gate, so a handler that
/// called `suggest::suggest` directly would inherit NONE of them and would
/// happily search a live game, which the module doc forbids. They are restated
/// here rather than reached because the alternative is widening a
/// rendering-adjacent function that returns a rendered `StepOutput`.
///
/// **How much of this is REACHABLE through this API, measured rather than
/// assumed** (an earlier version of this doc claimed "every one is asserted by a
/// test", and a review found that only the enum was walked — no route test
/// constructed any of these states):
///
/// - `GameOver` and `Locked` are reachable and are now pinned by route tests.
/// - `BotSeat` never fires at rest: `create` and `input` both pump bot seats
///   before returning, so a bot is never the seat to move when a request is
///   answered. It is kept because a HALTED pump (the `PUMP_MOVE_CAP` arm) can
///   leave one, and because a handler that assumed otherwise would advise a bot
///   against the player. **Since M6.1 it is unreachable over HTTP entirely**,
///   and this is the line an audit of "which refusals can a browser see" will
///   find: R-1.8 refuses a caller who does not hold the acting seat, a bot seat
///   is held by nobody, and that 403 sits ahead of this rung in `suggest`'s
///   ladder. The variant stays because this crate carries `repl::suggest_cmd`'s
///   four refusals verbatim rather than a subset of them.
/// - `LiveMode` cannot be reached by a game this API deals: the dealing is
///   `tables.rs`'s (`Session::new_sim`), and `games.rs` rebuilds a game
///   through `Session::from_record` alone. `TrackingMode::Live` is derived from
///   a record's setup observations, so only a live-tracked CLI save could
///   produce one, arriving through the store directory or the browser's
///   restore. It is kept because those ARE ways in, and because the module doc
///   it enforces is the standing 3.2 scope rather than a property of this crate.
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

    /// The four refusals the terminal gives — **and what this does to each, one
    /// by one, because an earlier version of this doc claimed all four were the
    /// CLI's words merely unwrapped and a reviewer found three were not.**
    ///
    /// - `GameOver` — the CLI's line, BYTE-IDENTICAL (`repl.rs`'s
    ///   `suggest_cmd`). Nothing was done to it.
    /// - `LiveMode` — the CLI's words UNWRAPPED. This one really is
    ///   hand-wrapped to a view body (a 215-column paragraph would size the
    ///   composed frame to itself), and carrying a terminal's newlines into a
    ///   layout engine would be shipping the wrong thing.
    /// - `BotSeat` — **AUTHORED.** The CLI names the seat number; this drops it
    ///   (the payload already carries the seat) and adds a pointer to `why`,
    ///   which is the answer a player who asked wants and which the terminal has
    ///   no room for.
    /// - `Locked` — **AUTHORED.** The CLI's line ends "only save/log/quit",
    ///   which is a list of terminal commands and means nothing in a browser.
    ///
    /// **These four are the one place M3 re-words at all**, and the licence is
    /// specific: they live inside `repl::suggest_cmd`'s rendered output and no
    /// `guide_sync` cell pins them. `Refusal`'s twelve are carried verbatim,
    /// because those strings ARE pinned to the tester guide.
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

    /// A real 2P state, so `describe_move` has orders to word.
    fn a_state() -> (yokohama_core::GameState, Rules) {
        let rules = Rules::standard();
        let games = crate::games::Games::new(
            std::sync::Arc::new(rules.clone()),
            std::sync::Arc::new(crate::store::MemStore::default())
                as std::sync::Arc<dyn crate::store::Store>,
        );
        // create -> keep at both seats -> the handle. A table is not a game
        // until every human seat has chosen (M6.1 §4.3), and both seats here are
        // one person's, so one identity answers for both.
        let red = crate::identity::User::parse("red@t.test").expect("an email");
        let seat = |who: &str| crate::tables::SeatSpec {
            user: Some(who.to_string()),
            bot: false,
            open: false,
        };
        let id = games
            .create(
                &crate::tables::NewGame {
                    players: 2,
                    colors: vec![yokohama_core::PlayerColor::Red, yokohama_core::PlayerColor::Blue],
                    start_seat: 0,
                    seed: Some(crate::tables::WireSeed(20_260_831)),
                    seats: vec![seat("red@t.test"), seat("red@t.test")],
                    name: None,
                    id: Some("rank".to_string()),
                    display_name: None,
                    password: None,
                },
                &red,
            )
            .expect("a table")
            .id;
        while let Some(s) =
            games.table(&id, &red).expect("seated").next_unkept_seat_of(&red)
        {
            games.keep(&id, &red, s, 1, None).expect("a keep");
        }
        let handle = games.handle(&id).expect("a game");
        let repl = handle.lock().expect("the handle");
        (repl.session.state().clone(), rules)
    }

    /// **`why`'s unplayed candidates do not name cards in the mover's hand;
    /// `suggest`'s do, and the move that was PLAYED always does.**
    ///
    /// `Move::FulfillOrder` is the only variant carrying an `OrderCardId`
    /// (`types/moves.rs`) and the only `describe_move` arm that reads
    /// `rules.orders`, so this covers the whole surface rather than a sample of
    /// it. The generator for those moves reads `p.order_cards_in_hand`, so a
    /// ranking served whole is a readout of the hand the same server masks in
    /// `build_hand`.
    ///
    /// **Constructed rather than taken from the corpus, and that is a finding
    /// in itself**: the committed `why` fixture's candidates are all
    /// `Place ...`, because its account is a Placement decision. Its rows are
    /// real; they are not evidence of this leak, and a fence anchored to
    /// them would have passed while proving nothing. `rank_all` is a pure wording
    /// function, so the candidates here are built directly rather than reached
    /// through a game that happens to offer them.
    #[test]
    fn only_the_played_fulfill_is_worded_when_the_hand_is_another_seats() {
        let (state, rules) = a_state();
        // Sorted by the printed corner index, so the pair is the same pair on
        // every run rather than whatever the registry's iteration order is.
        let mut ids: Vec<_> = rules.orders.iter().map(|c| c.id).collect();
        ids.sort_unstable_by_key(|id| id.0);
        let (order, other) = (ids[0], ids[1]);
        let played = Move::FulfillOrder { order_id: order };
        let ranking = vec![
            RootScore { mv: played.clone(), value: 10 },
            RootScore { mv: Move::FulfillOrder { order_id: other }, value: 4 },
        ];

        let masked = rank_all(&ranking, &played, &state, &rules, HandVisibility::Masked);
        assert!(masked[0].chosen, "the played move is row 0 here");
        assert_ne!(
            masked[0].text, MASKED_FULFILL,
            "the PLAYED move is public — it happened, and the card has left the hand",
        );
        assert_eq!(masked[1].text, MASKED_FULFILL, "the road not taken names no card");
        // The ROW survives: its value and margin are the shape of the bot's
        // preference and say nothing about which card it is. Dropping rows
        // would have been the other way to close this, and it is not this one.
        assert_eq!(masked[1].value, 4);
        assert_eq!(masked[1].margin, -6);

        let own = rank_all(&ranking, &played, &state, &rules, HandVisibility::Own);
        assert_ne!(
            own[1].text, MASKED_FULFILL,
            "`suggest` ranks YOUR moves from YOUR view — your own hand is already on your screen",
        );
        assert_ne!(own[1].text, masked[1].text, "the two modes must actually differ");
    }

    /// A vs-bot game built in process under a fixed bot seed (the corpus's
    /// shape, `wire_fixtures.rs`'s `constructed()`): seat 0 the production
    /// bot, seat 1 a person; `pumped` is how many times the bot pump runs —
    /// ONCE stops at the person's first turn with the bot's starting order
    /// surely unfulfilled, `usize::MAX` plays the game out with both seats
    /// pumped as bots.
    fn vs_bot(pumped: usize) -> (yokohama_cli::repl::Repl, yokohama_cli::bot::BotSeats) {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, PlayerColor, StartingOrderChoice};
        const SEED: u64 = 20_260_831;
        const BOT_SEED: u64 = 0x4649_5854_5552_4553;
        let seats = if pumped == usize::MAX {
            vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)]
        } else {
            vec![Some(PROVISIONAL_DEFAULT_BOT), None]
        };
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
        let mut session = Session::new_sim(config, SEED, choices, Rules::standard()).expect("a game");
        let bots = BotSeats::new(seats, BOT_SEED);
        session.set_run_seats(bots.seat_provenance());
        let mut repl = yokohama_cli::repl::Repl::new(session);
        for _ in 0..pumped.min(400) {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        (repl, bots)
    }

    /// `why`'s account of `event` (the last bot move when `None`) over the
    /// pumped game, as the route builds it.
    fn account_of(
        repl: &yokohama_cli::repl::Repl,
        bots: &yokohama_cli::bot::BotSeats,
        event: Option<usize>,
    ) -> WhyAccount {
        yokohama_cli::why::account_parts(repl.session.mode, repl.session.record(), repl.session.rules(), bots, event)
            .unwrap_or_else(|r| panic!("a bot move to account for: {r}"))
    }

    /// **Visibility is the engine's own view** (finding B1): withheld while
    /// the requester's view masks the mover's starting order, public to the
    /// mover's own seat, public once the card is out of the hand — and at game
    /// over the record route's own condition (review I-1): public to a SEAT
    /// HOLDER whatever the hand holds, while a watcher still reads the
    /// engine's view, withheld for a seat whose starting order never left its
    /// hand.
    #[test]
    fn visibility_follows_the_requesters_view_of_the_movers_card_and_the_record_routes_game_over_rule() {
        let (repl, _) = vs_bot(1);
        let now = repl.session.state();
        let mover = PlayerId::new(0);
        let view = yokohama_core::game::observe(now, PlayerId::new(1)).expect("seat 1's view");
        assert!(
            view.players[0].hand.contains(&yokohama_core::HandCard::HiddenStartingOrder),
            "premise: after one bot turn the bot's starting order is a masked slot in its hand",
        );
        assert_eq!(visibility_of(now, mover, &[PlayerId::new(1)]), Visibility::Withheld, "the opponent");
        assert_eq!(visibility_of(now, mover, &[]), Visibility::Withheld, "a spectator");
        assert_eq!(visibility_of(now, mover, &[PlayerId::new(0)]), Visibility::Public, "the mover's own seat");
        assert_eq!(
            visibility_of(now, mover, &[PlayerId::new(1), PlayerId::new(0)]),
            Visibility::Public,
            "a holder of both seats holds the mover's",
        );
        // The card out of the hand — fulfilled, in the engine's representation
        // `completed_orders` — and the same view no longer masks it.
        let mut fulfilled = now.clone();
        let card = fulfilled.players[0].starting_order.expect("a Keep-shaped seat records its card");
        fulfilled.players[0].order_cards_in_hand.retain(|c| *c != card);
        fulfilled.players[0].completed_orders.push(card);
        assert_eq!(visibility_of(&fulfilled, mover, &[PlayerId::new(1)]), Visibility::Public, "a public card");
        // Game over: the record route's own condition. The holder of EITHER
        // seat reads everything, whatever the hand still holds; a watcher
        // reads the engine's view, which still masks a starting order that
        // never left the hand — read here, seat by seat, not assumed.
        let (over, _) = vs_bot(usize::MAX);
        assert!(over.session.is_over(), "premise: the both-bot pump finishes the game");
        let end = over.session.state();
        let masked: Vec<bool> = [0u8, 1]
            .iter()
            .map(|&seat| {
                yokohama_core::game::observe(end, PlayerId::new(1 - seat))
                    .expect("the other seat's view")
                    .players
                    .iter()
                    .find(|p| p.id == PlayerId::new(seat))
                    .is_some_and(|p| p.hand.contains(&yokohama_core::HandCard::HiddenStartingOrder))
            })
            .collect();
        assert!(
            masked.contains(&true),
            "premise: at game over some seat still holds its starting order, or the holder rows prove nothing",
        );
        for seat in [0u8, 1] {
            let mover = PlayerId::new(seat);
            for holder in [0u8, 1] {
                assert_eq!(
                    visibility_of(end, mover, &[PlayerId::new(holder)]),
                    Visibility::Public,
                    "seat {seat}'s account to the holder of seat {holder} at game over",
                );
            }
            let watcher = if masked[usize::from(seat)] { Visibility::Withheld } else { Visibility::Public };
            assert_eq!(visibility_of(end, mover, &[]), watcher, "seat {seat}'s account to a watcher at game over");
        }
    }

    /// **A withheld account is the same bytes whatever the search found**
    /// (the independence pin, finding B1): two accounts of the same move that
    /// differ in every card-dependent field — the feature table, the total,
    /// the ranking, the continuation, the counters, the decision seed, the
    /// forced fact — serialize to one `withheld` payload. What survives is
    /// the move, the claim and the sentence.
    #[test]
    fn a_withheld_account_is_the_same_bytes_whatever_the_search_found() {
        let (repl, bots) = vs_bot(1);
        let rules = Rules::standard();
        let a = account_of(&repl, &bots, None);
        let mut b = account_of(&repl, &bots, None);
        b.explain.explanation.total += 1000;
        for t in &mut b.explain.explanation.terms {
            t.viewer_value += 1;
        }
        b.explain.ranking.clear();
        b.explain.continuation.clear();
        b.explain.stats = None;
        b.explain.hit_ply_cap = !b.explain.hit_ply_cap;
        b.reconstruction.decision_seed ^= 0xDEAD_BEEF;
        b.reconstruction.forced = !b.reconstruction.forced;
        let one = serde_json::to_string(&Why::account(&a, &rules, Visibility::Withheld)).expect("json");
        let two = serde_json::to_string(&Why::account(&b, &rules, Visibility::Withheld)).expect("json");
        assert_eq!(one, two, "the withheld payload depends on something the search found");
        // And the public shape of the same two accounts DOES differ — the
        // anti-vacuity: the fields above are fields the full account carries.
        let one = serde_json::to_string(&Why::account(&a, &rules, Visibility::Public)).expect("json");
        let two = serde_json::to_string(&Why::account(&b, &rules, Visibility::Public)).expect("json");
        assert_ne!(one, two, "premise: the public account carries what the mutation moved");
        let withheld: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&Why::account(&a, &rules, Visibility::Withheld)).expect("json"),
        )
        .expect("a value");
        assert!(withheld["ok"].is_null() && withheld["refusal"].is_null());
        assert_eq!(withheld["withheld"]["reason"], WITHHELD_REASON);
        assert_eq!(withheld["withheld"]["provenance"]["seat"], 0);
        for key in ["decision_seed", "forced"] {
            assert!(withheld["withheld"]["provenance"].get(key).is_none(), "{key} rides the claim: {withheld}");
        }
    }

    /// **The served continuation masks its fulfil lines as the ranking masks
    /// its rows** (finding B1's third channel, on the public shape): a
    /// `FulfillOrder` step past the first words as [`MASKED_FULFILL`]; the
    /// first step — the move PLAYED — is never masked, even when it is a
    /// fulfil. Found by scanning a finished game for a line that fulfils
    /// later than it starts and for a move that is itself a fulfil — the
    /// anti-vacuity asserts both, so a reseed cannot skip either half.
    #[test]
    fn the_served_continuation_masks_a_later_fulfil_and_never_the_played_move() {
        let (over, bots) = vs_bot(usize::MAX);
        let rules = Rules::standard();
        let record = over.session.record();
        let mut later_fulfil = None;
        let mut played_fulfil = None;
        for n in 0..record.events.len() {
            let Ok(a) = yokohama_cli::why::account_parts(over.session.mode, record, &rules, &bots, Some(n)) else { continue };
            let line = &a.explain.continuation;
            if later_fulfil.is_none() && line.iter().skip(1).any(|m| matches!(m, Move::FulfillOrder { .. })) {
                later_fulfil = Some(a);
                continue;
            }
            if played_fulfil.is_none() && matches!(a.mv, Move::FulfillOrder { .. }) {
                played_fulfil = Some(a);
            }
            if later_fulfil.is_some() && played_fulfil.is_some() {
                break;
            }
        }
        let a = later_fulfil.expect("some bot line in a finished game fulfils later than it starts");
        let out = Why::account(&a, &rules, Visibility::Public);
        let ok = out.ok.expect("public");
        let raw: Vec<String> = a.explain.continuation.iter().map(|m| yokohama_cli::describe::describe_move(m, &a.state, &rules)).collect();
        assert_eq!(ok.continuation.len(), raw.len());
        assert_eq!(ok.continuation[0], raw[0], "step 1 is the move played, never masked");
        let masked: Vec<usize> = (1..raw.len()).filter(|&i| ok.continuation[i] == MASKED_FULFILL).collect();
        assert!(!masked.is_empty(), "no later fulfil was masked: {:?}", ok.continuation);
        for i in masked {
            assert!(matches!(a.explain.continuation[i], Move::FulfillOrder { .. }), "step {i} masked but not a fulfil");
            assert_ne!(ok.continuation[i], raw[i], "the raw wording names the card");
        }
        let played = played_fulfil.expect("some bot move of the finished game is itself a fulfil");
        let ok = Why::account(&played, &rules, Visibility::Public).ok.expect("public");
        assert_eq!(ok.text, ok.continuation[0], "a played fulfil is worded in full at step 1");
        assert_ne!(ok.continuation[0], MASKED_FULFILL);
    }

    /// The sample set is complete, distinct, and each sample really is the
    /// variant its tag names.
    ///
    /// **This test no longer claims to pin the list to the ENUM**, because it
    /// cannot and the previous version's doc said it could. What it pins is
    /// that the ONE hand-written list is internally sound: twelve samples,
    /// twelve distinct tags, no variant sampled twice. `refusal_samples`' doc
    /// names the residual hole.
    #[test]
    fn the_refusal_samples_are_complete_distinct_and_correctly_tagged() {
        let samples = refusal_samples();
        assert_eq!(samples.len(), 12, "twelve variants, twelve samples");

        let mut tags = refusal_kinds();
        let count = tags.len();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), count, "two samples share a tag: {:?}", refusal_kinds());

        // Anti-vacuity: every sample must word itself through the CLI's own
        // `Display`, so a placeholder that failed to construct is visible.
        for r in &samples {
            assert!(r.to_string().len() > 20, "{} says nothing: {r:?}", kind_of(r));
        }
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
