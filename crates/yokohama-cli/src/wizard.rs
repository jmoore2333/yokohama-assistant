//! Interactive setup wizard: scripted-testable sim/live game-start Q&A that
//! produces a ready-to-play `Repl` (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout of its own -- `main.rs` drives `run_wizard` with real I/O
//! and `ask`'s `out` closure, same separation as `repl::Repl::step`.
//!
//! Robustness rework (design docs/plans/2026-07-18-choice-encoding-design.md
//! §5.5): every card/tile question validates at parse time against a
//! running context (registry membership, duplicates, the legal tile
//! multiset) instead of deferring to `reconcile_setup`'s tail check;
//! `ask` re-asks indefinitely on invalid input (the only exits are a valid
//! answer or the literal `abort`); and the wizard ends with a review/edit
//! loop over a summary of every collected answer before handing off to
//! `reconcile_setup` + `finalize_setup`.

use std::cell::Cell;
use std::collections::HashMap;

use crate::card_prompt::{self, entry_hint, CardEntry, CardPrompt, Numbering};
use crate::input::tech_name;
use crate::screen_model::{
    achievement_registry, build_index, menu_index, menu_number, menu_range, CardLocation, CardRef,
    IndexFilter, IndexKind, IndexModel, RecordingTarget,
};
use crate::session::Session;
use yokohama_core::*;

/// What one line at a wizard question turned out to be.
///
/// **The middle variant is T7's re-plumb, and it is the reason this enum
/// exists.** A validator could previously only accept or refuse, so there was
/// nowhere to put a full-screen surface: `?` at a setup prompt had no way to
/// draw the card index and hand the same question back. That is the F5 gap at
/// the exact place the owner hit it — *"not seeing the ID cards or having a
/// lookup table in some way makes it hard to enter"* — since setup is where a
/// player is holding physical cards and typing what is on them.
///
/// A screen is NOT a refusal and is deliberately not worded as one: nothing was
/// wrong with the line, nothing is lost, and every answer already collected is
/// still collected — the question simply comes round again with the surface
/// above it.
enum Ask {
    /// The line answered the question; `ask` returns.
    Taken,
    /// The line asked to LOOK: print this, then ask the same question again.
    Screen(String),
    /// The line was not usable; print the reason and ask again.
    Refused(String),
}

/// Ask one question, re-asking indefinitely on invalid input. `parse`
/// reports success/failure only (`Ok(())`/`Err(reason)`) -- callers capture
/// the parsed value into a local `Option` from inside the closure, which is
/// guaranteed `Some` once `ask` returns `Ok(())` (the closure only returns
/// `Ok(())` after setting it). Every prompt and error goes through `out` as
/// it happens, so scripted tests can assert on the transcript and `main`
/// can print interactively from the exact same code path.
///
/// The plain front door to [`ask_screened`]: a question with no surface to
/// open answers `Ok`/`Err` and never has to name a third outcome.
fn ask(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    prompt: &str,
    parse: &mut dyn FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    ask_screened(input, out, &|| prompt.to_string(), &mut |line| match parse(line) {
        Ok(()) => Ask::Taken,
        Err(reason) => Ask::Refused(reason),
    })
}

/// [`ask`], plus the arm that lets a validator answer with a SCREEN.
///
/// The only two exits are a valid answer, or the literal (case-insensitive)
/// `abort`, checked BEFORE `parse` sees the line so it can never be shadowed
/// by a question's own validation -- `Err("aborted")` is the sentinel every
/// caller up to `run_wizard` propagates via `?` (design §5.5 item 2; this
/// replaces the old 3-strikes give-up). A screen cannot shadow it either: the
/// check is above the dispatch, not inside any one question's grammar.
///
/// **The prompt is a closure, not a string, because a screen can change what
/// the next answer MEANS.** Opening a card index makes a bare number a pick
/// from that screen rather than a card id, and a prompt fixed before the loop
/// could not say so — it would still be offering the grammar that applied
/// before the player looked something up. Re-worded each time round, the line
/// above the caret always describes the state the caret is actually in.
fn ask_screened(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    prompt: &dyn Fn() -> String,
    parse: &mut dyn FnMut(&str) -> Ask,
) -> Result<(), String> {
    loop {
        out(&prompt());
        out("\n");
        let line = input.next().ok_or("input ended mid-wizard")?;
        let line = line.trim();
        if line.eq_ignore_ascii_case("abort") {
            return Err("aborted".to_string());
        }
        match parse(line) {
            Ask::Taken => return Ok(()),
            // Printed as it stands, with no `!` and no indent: it is a screen,
            // not a complaint about the line that opened it.
            Ask::Screen(text) => out(&text),
            Ask::Refused(e) => out(&format!("  ! {e}\n")),
        }
    }
}

// ---------------------------------------------------------------------
// Card entry (T7: spec §0.2/§0.6, findings F4/F5/F6)
// ---------------------------------------------------------------------

/// Everything a card question needs beyond the question itself.
///
/// **`placed` is the leak fence, and at setup it is trivially satisfied:**
/// every entry in it is a card the OPERATOR typed off a face-up table, so the
/// index can annotate all of it without knowing anything they do not. (The
/// fence matters in the REPL, where the same list is assembled from public
/// board state only — see `screen_model::build_order_index`.)
struct CardCtx<'a> {
    rules: &'a Rules,
    /// Seat colors, for the one card face that names players.
    colors: &'a [PlayerColor],
    placed: &'a [(CardRef, CardLocation)],
}

impl CardCtx<'_> {
    /// The index this question's `?` opens, at the current filter.
    ///
    /// The achievement population is the whole twelve-card REGISTRY rather than
    /// the three in play: at setup, deciding which three are in play is the
    /// question being asked.
    ///
    /// `target` is an `Option` because one registry opens its index as a
    /// LOOK-UP rather than as a pick list — see [`numbered_screen`].
    fn index(
        &self,
        kind: IndexKind,
        target: Option<RecordingTarget>,
        filter: Option<IndexFilter>,
    ) -> IndexModel {
        build_index(kind, self.rules, self.placed, &achievement_registry(), target, filter)
    }
}

/// Whether this registry's index numbers its cards for PICKING at a wizard
/// prompt.
///
/// **Everything but the building sites does.** A site card's number IS its
/// identity — `S<n>`, 0-based, printed nowhere on the card but recorded in the
/// save and named by every other CLI surface — so a screen that also numbered
/// the cards by position would put TWO numbers on each one, differing by
/// exactly one: `[1] SITE 0`, `[2] SITE 1`. That is the one collision this
/// grammar must not have, because unlike an order card (where the id and the
/// position differ wildly and a wrong pick is obvious in the echo) an
/// off-by-one lands on a real, plausible, wrong card.
///
/// So the site index opens as what it actually is at that prompt: a look-up
/// with no selection numbers, where a bare number keeps meaning the identity
/// the question asked for. Nothing on screen is a number the prompt refuses,
/// which is F3 read from the other side — the same reading the T6 review gave
/// the REPL's cross-registry captures.
fn numbered_screen(kind: IndexKind) -> bool {
    !matches!(kind, IndexKind::Sites)
}

/// Ask for ONE card, in the crate's one card grammar (spec §0.2/§0.6).
///
/// **This is F4, F5 and F6 closed at the same place, because they were one
/// defect.** Every card question in the wizard used to bring its own grammar —
/// orders took a bare number, technologies refused one and wanted an id
/// printed on no physical card, sites took `S<n>`, achievements took a name
/// and refused the numbers they themselves printed. Now every one of them goes
/// through `card_prompt::resolve`, so a number, an id, a name, `?` and a filter
/// mean the same thing at all four; `?` draws the registry's own index; a
/// duplicate is refused BY LOCATION; and what was entered echoes back as a
/// drawn card before the next question.
///
/// `menu_on_screen` is the [`Numbering`] state, and it is a parameter rather
/// than a guess for the reason `card_prompt`'s header gives: a bare number is a
/// SELECTION only while something numbered is on screen. It starts true where
/// the question prints its own menu (achievements) and false where the cards
/// run to twenty or thirty-six and the player types an id or opens the index —
/// after which it stays true, because in a scrolling wizard the index the
/// player opened is still there above the prompt.
///
/// `validate` is the CALLER's pool check, run before the entry is taken: this
/// helper knows the grammar and the duplicates, and nothing about which cards
/// a given prompt can legitimately be told about.
fn ask_card(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    ctx: &CardCtx<'_>,
    question: &str,
    target: RecordingTarget,
    menu_on_screen: bool,
    validate: &mut dyn FnMut(CardRef) -> Result<(), String>,
) -> Result<CardRef, String> {
    let kind = target.kind();
    let numbered = numbered_screen(kind);
    let showing = Cell::new(menu_on_screen && numbered);
    let filter: Cell<Option<IndexFilter>> = Cell::new(None);
    let mut chosen: Option<CardRef> = None;
    let screen_target = numbered.then_some(target);

    out(&already_recorded_line(kind, ctx));
    ask_screened(
        input,
        out,
        &|| format!("{question} ({})", entry_hint(kind, showing.get())),
        &mut |line| {
            let model = ctx.index(kind, screen_target, filter.get());
            let prompt = CardPrompt {
                kind,
                candidates: None,
                placed: ctx.placed,
                numbering: if showing.get() {
                    Numbering::Selection(&model)
                } else {
                    Numbering::Ids
                },
            };
            match card_prompt::resolve(&prompt, line, ctx.rules) {
                CardEntry::Card(card) => match validate(card) {
                    Ok(()) => {
                        chosen = Some(card);
                        Ask::Taken
                    }
                    Err(reason) => Ask::Refused(reason),
                },
                // INLINE, never an overlay: `ask_screened` prints this into the
                // transcript and asks the same question underneath it, so the
                // screen's own way-out line has to say that rather than offer
                // four keys that close nothing, and it draws no caret of its own
                // (T7 review M3).
                CardEntry::OpenIndex => {
                    showing.set(numbered);
                    Ask::Screen(inline_index(&model, ctx))
                }
                CardEntry::Filter(narrowed) => {
                    filter.set(narrowed);
                    showing.set(numbered);
                    Ask::Screen(inline_index(&ctx.index(kind, screen_target, narrowed), ctx))
                }
                // There is nothing to cancel BACK to inside the wizard: every
                // question is one the setup cannot be built without, and the
                // one way out has always been `abort`. Saying so beats an empty
                // line silently re-asking with no explanation.
                CardEntry::Cancel => Ask::Refused(format!(
                    "this question needs an answer -- {}, or 'abort' to leave setup",
                    entry_hint(kind, showing.get()),
                )),
                CardEntry::Refused(message) => Ask::Refused(message),
            }
        },
    )?;
    let card = chosen.expect("`ask_screened` returned Ok only after this was set");
    // The card drawn back before the next question (spec §0.6). It is an echo,
    // not a confirmation step: R6's minimum ceremony at the table means the
    // operator reads it and keeps typing, and `edit <section>` is what a wrong
    // one costs.
    out(&card_prompt::echo(card, ctx.rules, ctx.colors));
    Ok(card)
}

/// One card index drawn INLINE, at the real terminal's width, behind a
/// one-time size notice when that width is under the design floor.
///
/// **The notice is §0.1's own device, moved to where the wizard can use it**
/// (T7 review SHOULD-6). §0.1 says the layout is designed at 200 columns and
/// degrades below that "with a one-time size notice"; the composed frame has
/// one, but the wizard runs BEFORE `Repl` exists and so had no way to say
/// anything. What the player saw instead was a grid laid out for 200 columns
/// wrapping mid-card in a 120-column window, with nothing on screen to explain
/// it. The grid now fits ([`crate::index_render::index_view`]'s width
/// parameter) and the notice says what was traded for the fit.
///
/// Once per wizard RUN rather than per index: three registries are entered in a
/// live setup and the terminal does not change size between them.
fn inline_index(model: &IndexModel, ctx: &CardCtx<'_>) -> String {
    let width = crate::screen::index_width();
    format!(
        "{}{}",
        size_notice_once(model.kind, width),
        crate::index_render::index_view(
            model,
            ctx.colors,
            crate::index_render::IndexContext::Inline,
            width,
        ),
    )
}

thread_local! {
    /// Whether this wizard run has already printed the sub-floor size notice.
    /// Thread-local for the reason `screen::force` is (parallel tests stay
    /// hermetic), and RESET at [`run_wizard`] rather than left to the thread's
    /// lifetime, so "once per run" is what it means in a test that runs two.
    static SIZE_NOTICE_SHOWN: Cell<bool> = const { Cell::new(false) };
}

/// The notice, the first time an index is drawn under the design floor; the
/// empty string afterwards, and always when the terminal is wide enough or its
/// width is unknown (a piped run measures nothing and prints nothing — spec
/// §0.12's byte-identical protocol is not negotiable for a convenience).
fn size_notice_once(kind: IndexKind, width: usize) -> String {
    if width >= crate::screen::FLOOR_W || SIZE_NOTICE_SHOWN.with(|f| f.get()) {
        return String::new();
    }
    SIZE_NOTICE_SHOWN.with(|f| f.set(true));
    let card = crate::card_render::card_width(kind);
    let per_row = (width + 1) / (card + 1);
    if per_row == 0 {
        // Narrower than ONE card: the grid cannot help and says so, rather than
        // promising a fit it is about to break.
        return format!(
            "  (your terminal is {width} columns and one card needs {card}, so the cards below \
             will wrap. The index is drawn for {} — widen the window to read it.)\n",
            crate::screen::FLOOR_W,
        );
    }
    format!(
        "  (your terminal is {width} columns; the index is drawn for {}, so it packs {} to a row \
         here. Nothing is cut — widen the window for the full grid.)\n",
        crate::screen::FLOOR_W,
        crate::card_render::plural(per_row, "card"),
    )
}

/// The running already-placed summary R8 asks setup screens to carry, on one
/// line — and F6's other half.
///
/// The refusal names where ONE card already is; this says what is already
/// recorded before the operator types anything, so a card that is going to be
/// refused can be spotted rather than discovered. The full detail (every card,
/// dimmed, with its location on the card face) is the index a keystroke away,
/// which is where it fits and where it stays current.
///
/// Empty while nothing is recorded, so the first question of a section is not
/// preceded by a line saying nothing.
fn already_recorded_line(kind: IndexKind, ctx: &CardCtx<'_>) -> String {
    let mut tags: Vec<String> = ctx
        .placed
        .iter()
        .filter(|(card, _)| card.kind() == kind)
        .map(|(card, _)| entry_tag(*card, ctx.rules))
        .collect();
    if tags.is_empty() {
        return String::new();
    }
    let total = tags.len();
    // Bounded by COLUMNS rather than by a count: the tags are three characters
    // for an order card and a whole title for a technology, so a fixed cap
    // would either clip the short list or overrun the long one.
    let mut room = TAG_BUDGET;
    let shown = tags
        .iter()
        .take_while(|tag| {
            room = room.saturating_sub(tag.chars().count() + TAG_SEPARATOR.chars().count());
            room > 0
        })
        .count()
        .max(1);
    tags.truncate(shown);
    let more = if shown < total { format!(" +{} more", total - shown) } else { String::new() };
    let registry = if total == 1 {
        crate::describe::registry_name_singular(kind)
    } else {
        crate::describe::registry_name(kind)
    };
    format!(
        "  already recorded -- {total} {registry}: {}{more}  ('?' shows each one and where)\n",
        tags.join(TAG_SEPARATOR),
    )
}

/// Columns this line spends on tags before it starts counting instead
/// ([`already_recorded_line`]).
const TAG_BUDGET: usize = 60;

/// What separates two already-recorded tags.
///
/// **A space is not enough once a tag has a space in it** (T7 review S2). An
/// order tag is `O04`, so `O04 O17 O21` reads as three cards on sight; a
/// TECHNOLOGY tag is the printed title, so the same join produced
/// `Postal Service Textile Mill Telegram` — one string a reader has to already
/// know the registry to cut up, on the line whose whole job is letting them see
/// at a glance what is spoken for. The mid-dot is the separator this crate
/// already uses between fields a person has to read apart.
const TAG_SEPARATOR: &str = " · ";

/// How the already-recorded line names one card: the thing the operator would
/// recognize. A technology by its printed NAME, never its id (R8); everything
/// else by the identity it is entered with.
fn entry_tag(card: CardRef, rules: &Rules) -> String {
    match card {
        CardRef::Tech(id) => tech_name(id, &rules.techs),
        other => card_prompt::card_tag(other),
    }
}

/// Run the interactive setup wizard to completion: mode, player count,
/// seat colors, start seat, seed, save path (the common preamble, none of
/// which depends on mode), then a mode-specific tail (`run_sim_tail` /
/// `run_live_tail`) that collects the rest, runs a review/edit loop, and
/// produces a `Session` wrapped in a `Repl`. The blank-seed `SystemTime`
/// derivation in `ask_seed` is the only nondeterminism anywhere in this
/// crate -- everything else is driven entirely by `input`.
pub fn run_wizard(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
) -> Result<crate::repl::Repl, String> {
    let rules = Rules::standard();
    // One notice per RUN, so a second wizard in the same process (or the same
    // test thread) states the size again rather than inheriting a flag.
    SIZE_NOTICE_SHOWN.with(|f| f.set(false));

    let is_live = ask_mode(input, out)?;
    let players = ask_player_count(input, out)?;
    let colors = ask_colors(input, out, players)?;
    let start_player = ask_start_player(input, out, players)?;
    let seed = ask_seed(input, out)?;
    let save_path = ask_save_path(input, out, seed)?;

    if is_live {
        run_live_tail(input, out, rules, players, colors, start_player, seed, save_path)
    } else {
        run_sim_tail(input, out, rules, players, colors, start_player, seed, save_path)
    }
}

fn ask_mode(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str)) -> Result<bool, String> {
    let mut is_live: Option<bool> = None;
    ask(input, out, "Mode? (sim/live)", &mut |s: &str| -> Result<(), String> {
        match s.to_lowercase().as_str() {
            "sim" => {
                is_live = Some(false);
                Ok(())
            }
            "live" => {
                is_live = Some(true);
                Ok(())
            }
            _ => Err(format!("'{s}' is not 'sim' or 'live'")),
        }
    })?;
    Ok(is_live.unwrap()) // `ask` returned Ok(()) only after this was set
}

fn ask_player_count(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str)) -> Result<u8, String> {
    let mut players: Option<u8> = None;
    ask(input, out, "Number of players? (2/3/4)", &mut |s: &str| -> Result<(), String> {
        match s.parse::<u8>() {
            Ok(n) if (2..=4).contains(&n) => {
                players = Some(n);
                Ok(())
            }
            _ => Err(format!("'{s}' is not 2, 3, or 4")),
        }
    })?;
    Ok(players.unwrap())
}

/// One per seat, distinct.
fn ask_colors(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    players: u8,
) -> Result<Vec<PlayerColor>, String> {
    let mut colors: Vec<PlayerColor> = Vec::new();
    for seat in 0..players {
        let mut chosen: Option<PlayerColor> = None;
        ask(
            input,
            out,
            &format!("Seat {seat} color? (red/blue/yellow/green)"),
            &mut |s: &str| -> Result<(), String> {
                let c = parse_color(s)?;
                if colors.contains(&c) {
                    return Err(format!("{c:?} is already taken"));
                }
                chosen = Some(c);
                Ok(())
            },
        )?;
        colors.push(chosen.unwrap());
    }
    Ok(colors)
}

fn ask_start_player(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    players: u8,
) -> Result<PlayerId, String> {
    let mut start: Option<PlayerId> = None;
    ask(
        input,
        out,
        &format!("Which seat starts? (0-{})", players - 1),
        &mut |s: &str| -> Result<(), String> {
            let n: u8 = s.parse().map_err(|_| format!("'{s}' is not a number"))?;
            if n >= players {
                return Err(format!("seat must be 0-{}", players - 1));
            }
            start = Some(PlayerId::try_new(n).map_err(|e| e.to_string())?);
            Ok(())
        },
    )?;
    Ok(start.unwrap())
}

/// Blank derives from wall-clock nanos -- the crate's one and only
/// permitted nondeterminism -- and echoes it so the game stays reproducible
/// from the transcript/GameRecord alone.
fn ask_seed(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str)) -> Result<u64, String> {
    let mut seed_answer: Option<Option<u64>> = None;
    ask(input, out, "Seed? (integer, or blank for random)", &mut |s: &str| -> Result<(), String> {
        if s.is_empty() {
            seed_answer = Some(None);
            return Ok(());
        }
        match s.parse::<u64>() {
            Ok(n) => {
                seed_answer = Some(Some(n));
                Ok(())
            }
            Err(_) => Err(format!("'{s}' is not a valid integer seed")),
        }
    })?;
    Ok(match seed_answer.unwrap() {
        Some(n) => n,
        None => {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            out(&format!("seed: {n}\n"));
            n
        }
    })
}

fn ask_save_path(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str), seed: u64) -> Result<String, String> {
    let mut path_answer: Option<String> = None;
    ask(input, out, "Save path? (blank for default)", &mut |s: &str| -> Result<(), String> {
        path_answer = Some(s.to_string());
        Ok(())
    })?;
    Ok(match path_answer.unwrap() {
        s if s.is_empty() => format!("yokohama-game-{seed}.json"),
        s => s,
    })
}

/// `Bot seats?` -- sim only (spec §9: `observe` refuses a live `Unknown`
/// seat, and reveal choreography would deadlock on a card a bot cannot read).
///
/// Blank is the default and means "none", so the answer costs an Enter for
/// anyone who does not want a bot. At least one seat must stay human: a
/// bot-vs-bot spectator mode is explicitly out of scope for v1, and a wizard
/// that let you build one would produce a game with no one to prompt.
///
/// The bot seed is DRAWN, not asked: it is a full-`u64` stream key, which is
/// not something to type at a prompt, and it is echoed + recorded in the
/// record's run provenance so a game stays reproducible from its save alone.
/// It comes from `SeedSchedule::from_os_entropy`, the constructor that owns
/// the entropy rule for bot-facing seeds (core's `game/seeds.rs`), rather than
/// from a second local derivation that could drift from it.
fn ask_bots(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    players: u8,
) -> Result<crate::bot::BotSeats, String> {
    let mut seats: Option<Vec<Option<RolloutPolicy>>> = None;
    ask(
        input,
        out,
        &format!(
            "Bot seats? (blank for none, or seat numbers 0-{} e.g. '1' or '0 2' — sim only, one seat must stay human)",
            players - 1
        ),
        &mut |s: &str| -> Result<(), String> {
            let mut chosen = vec![None; players as usize];
            for tok in s.split_whitespace() {
                let seat: u8 = tok.parse().map_err(|_| format!("'{tok}' is not a seat number"))?;
                if seat >= players {
                    return Err(format!("seat must be 0-{}", players - 1));
                }
                chosen[seat as usize] = Some(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT);
            }
            if chosen.iter().all(Option::is_some) {
                return Err("at least one seat must stay human (bot-vs-bot is not in this version)".to_string());
            }
            seats = Some(chosen);
            Ok(())
        },
    )?;
    let seats = seats.unwrap(); // `ask` returned Ok(()) only after this was set
    // A seed is drawn only when there is a bot to key with it -- an all-human
    // table records a full set of human seats and no stream at all. The vector
    // stays `players` long either way, so provenance always describes every
    // seat rather than an empty run.
    let bot_seed = if seats.iter().any(Option::is_some) {
        let (_schedule, seed) = SeedSchedule::from_os_entropy();
        out(&format!("bot seed: {seed}\n"));
        seed
    } else {
        0
    };
    Ok(crate::bot::BotSeats::new(seats, bot_seed))
}

/// `Seat <n>: keep card 1 or 2?` -- shared by the sim tail and a live
/// "known" seat.
///
/// **The numbers are the ones printed beside the two cards** (`[1]`/`[2]` in
/// [`ask_sim_deal`]'s listing), which since T5 means the crate's numbering base
/// (`screen_model::NUMBER_BASE`) rather than the engine's 0-based
/// `StartingOrderChoice::Keep` index. The conversion happens HERE, once, in the
/// same place the question is worded -- the returned value is the engine's.
fn ask_keep(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str), seat: u8) -> Result<u8, String> {
    let mut keep: Option<u8> = None;
    let lo = menu_number(0);
    let hi = menu_number(1);
    ask(input, out, &format!("Seat {seat}: keep card {lo} or {hi}?"), &mut |s: &str| -> Result<(), String> {
        match s.parse::<usize>().ok().and_then(|n| menu_index(n, 2)) {
            Some(k) => {
                keep = Some(k as u8);
                Ok(())
            }
            None => Err(format!("'{s}' is not {lo} or {hi}")),
        }
    })?;
    Ok(keep.unwrap())
}

// ---------------------------------------------------------------------
// Live tail
// ---------------------------------------------------------------------

/// Everything the live tail collects, bundled so the review/edit loop can
/// dispatch `edit <section>` to a function taking `&mut LiveWizardState`
/// instead of threading a dozen separate `&mut` locals around.
///
/// **`placed` is the one duplicate tracker, and it replaced two id sets at T7**
/// (design §5.5 item 1 kept, its representation changed). `seen_order_ids` and
/// `seen_tech_ids` could say THAT a card was already entered and never WHERE,
/// which is finding F6 exactly — the owner's Port entry refused `2`, `4` and
/// `6` as "already entered" while they sat in the seats' recorded deals, and
/// nothing on screen could say so. A location per card answers that, and it
/// also feeds the index's dimming, so the refusal and the screen are one fact.
///
/// It covers sites as well as cards; the achievements are collected against
/// their own local list, since a category answer cannot collide with anything
/// outside its own question.
struct LiveWizardState {
    players: u8,
    colors: Vec<PlayerColor>,
    start_player: PlayerId,
    seed: u64,
    save_path: String,
    config: GameConfig,
    legal_tile_counts: HashMap<AreaType, u32>,
    all_tech_ids: Vec<TechCardId>,
    placed: Vec<(CardRef, CardLocation)>,
    known_deals: Vec<Option<[OrderCardId; 2]>>,
    deal_choices: Vec<StartingOrderChoice>,
    tiles: Vec<AreaType>,
    sites: Vec<Option<u8>>,
    tokens: Vec<Option<FivePowerEffect>>,
    achievements: [AchievementType; 3],
    port_a: [OrderCardId; 6],
    port_b: Option<[OrderCardId; 6]>,
    lab_a: [TechCardId; 6],
    lab_b: Option<[TechCardId; 6]>,
}

#[allow(clippy::too_many_arguments)]
fn run_live_tail(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    rules: Rules,
    players: u8,
    colors: Vec<PlayerColor>,
    start_player: PlayerId,
    seed: u64,
    save_path: String,
) -> Result<crate::repl::Repl, String> {
    let mut config = GameConfig::for_player_count(players);
    config.player_colors = colors.clone();
    config.start_player = start_player;

    // Legal per-type tile counts for this player count (design §5.5 item
    // 1): derived from a throwaway seeded board exactly the way
    // `reconcile_board` treats the seeded layout as the multiset of
    // record -- which physical duplicate a seeded RNG happens to drop
    // doesn't change the per-type COUNTS, so any seed for this config
    // yields the same legal multiset.
    let ghost = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
    let mut legal_tile_counts: HashMap<AreaType, u32> = HashMap::new();
    for area in ghost.state.board_layout.areas.values() {
        *legal_tile_counts.entry(area.area_type).or_insert(0) += 1;
    }
    let all_tech_ids: Vec<TechCardId> = rules.techs.iter().map(|c| c.id).collect();

    let mut placed: Vec<(CardRef, CardLocation)> = Vec::new();
    let mut known_deals: Vec<Option<[OrderCardId; 2]>> = vec![None; players as usize];
    let mut deal_choices: Vec<StartingOrderChoice> = vec![StartingOrderChoice::Unknown; players as usize];
    for seat in 0..players {
        let (deal, choice) = ask_live_deal(input, out, seat, &rules, &colors, &mut placed)?;
        known_deals[seat as usize] = deal;
        deal_choices[seat as usize] = choice;
    }

    let tiles = ask_tiles(input, out, players, &legal_tile_counts)?;
    let sites = ask_sites(input, out, &tiles, &rules, &colors, players, &mut placed)?;
    let tokens = ask_tokens(input, out, &tiles, &rules, players)?;
    let achievements = ask_achievements(input, out, &rules, &colors)?;

    let port_a: [OrderCardId; 6] =
        ask_orders(input, out, false, &rules, &colors, &mut placed)?.try_into().unwrap();
    let port_b: Option<[OrderCardId; 6]> = if config.has_port_b {
        Some(ask_orders(input, out, true, &rules, &colors, &mut placed)?.try_into().unwrap())
    } else {
        None
    };
    let lab_a: [TechCardId; 6] =
        ask_techs(input, out, false, &rules, &colors, &all_tech_ids, &mut placed)?.try_into().unwrap();
    let lab_b: Option<[TechCardId; 6]> = if config.has_lab_b {
        Some(ask_techs(input, out, true, &rules, &colors, &all_tech_ids, &mut placed)?.try_into().unwrap())
    } else {
        None
    };

    let mut ws = LiveWizardState {
        players,
        colors,
        start_player,
        seed,
        save_path,
        config,
        legal_tile_counts,
        all_tech_ids,
        placed,
        known_deals,
        deal_choices,
        tiles,
        sites,
        tokens,
        achievements,
        port_a,
        port_b,
        lab_a,
        lab_b,
    };

    // Two nested loops: the inner one is the answer review/edit step, the
    // outer one is the BOARD CHECK (third review I1). `confirm` no longer
    // starts the game -- it builds the session, draws the resulting board, and
    // asks the operator whether it matches the table in front of them. A `no`
    // falls back into the review loop, where `edit tiles` re-enters the board.
    let mut session = loop {
        loop {
            out(&render_live_summary(&ws, &rules));
            out("confirm / edit <section> / abort?\n");
            let line = input.next().ok_or("input ended mid-wizard")?;
            let line = line.trim().to_string();
            if line.eq_ignore_ascii_case("abort") {
                return Err("aborted".to_string());
            }
            if line == "confirm" {
                break;
            }
            match line.strip_prefix("edit ") {
                Some(section) => match perform_live_edit(&mut ws, section.trim(), input, out, &rules) {
                    Ok(()) => {}
                    Err(e) if e == "aborted" => return Err(e),
                    Err(e) => out(&format!("  ! {e}\n")),
                },
                None => out(&format!("  ! '{line}' is not 'confirm', 'edit <section>', or 'abort'\n")),
            }
        }

        // Building the real session here (rather than after the loop) is what
        // lets the check draw the board with the renderer play itself uses. It
        // also moves reconcile failures INSIDE the loop: a setup that doesn't
        // add up now sends the operator back to the review step instead of
        // killing the wizard ~40 answers in.
        match Session::new_live(
            ws.config.clone(),
            ws.seed,
            live_observations(&ws),
            ws.deal_choices.clone(),
            rules.clone(),
        ) {
            Ok(candidate) => {
                if ask_board_check(input, out, &candidate)? {
                    break candidate;
                }
                out("  Board not confirmed -- back to the setup review. 'edit tiles' re-enters the\n");
                out("  board, and the building-site cards with it (they sit on the tiles).\n");
            }
            Err(e) => {
                out(&format!("  ! these answers do not make a playable setup: {e}\n"));
                out("  Back to the setup review -- correct the section it names, then confirm again.\n");
            }
        }
    };
    session.save_path = Some(std::path::PathBuf::from(ws.save_path));
    Ok(crate::repl::Repl::new(session))
}

/// Everything the live tail collected, in the shape `reconcile_setup` wants.
/// Cloned rather than moved because the board check builds a candidate session
/// on every pass through the review loop, and a `no` has to leave `ws` intact
/// to edit.
fn live_observations(ws: &LiveWizardState) -> SetupObservations {
    SetupObservations {
        board: Some(ObservedBoard {
            tiles: ws.tiles.clone(),
            sites: ws.sites.clone(),
            tokens: Some(ws.tokens.clone()),
        }),
        achievements: Some(ws.achievements),
        port_a: Some(ws.port_a),
        port_b: ws.port_b,
        lab_a: Some(ws.lab_a),
        lab_b: ws.lab_b,
        deals: ws
            .known_deals
            .iter()
            .enumerate()
            .filter_map(|(seat, d)| d.map(|cards| (PlayerId::new(seat as u8), cards)))
            .collect(),
    }
}

/// Perform one `edit <section>` from the live review loop: retract the
/// section's previous contribution from `placed`, then re-run exactly that
/// section's ask-block (design §5.5 item 3). `start`/`mode`/`players` are
/// deliberately NOT editable sections -- changing any of them would invalidate
/// every question already answered (color count, grid size, Port B/Lab B
/// presence, ...); the wizard must be aborted and re-run for those.
///
/// **Retraction is by LOCATION now, and that is what the T7 representation
/// buys.** Each section used to hand back its own ids one by one, which meant
/// every section carried a bespoke retract step that had to stay in step with
/// its ask step; a location IS the section, so `edit porta` drops exactly the
/// cards recorded as Port A slots and nothing else, by the same predicate the
/// re-ask then re-fills.
fn perform_live_edit(
    ws: &mut LiveWizardState,
    section: &str,
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    rules: &Rules,
) -> Result<(), String> {
    let mut parts = section.split_whitespace();
    match parts.next() {
        Some("colors") => {
            ws.colors = ask_colors(input, out, ws.players)?;
            ws.config.player_colors = ws.colors.clone();
        }
        Some("seed") => {
            ws.seed = ask_seed(input, out)?;
            ws.config.setup_seed = ws.seed;
        }
        Some("save") => {
            ws.save_path = ask_save_path(input, out, ws.seed)?;
        }
        Some("deal") => {
            let seat: u8 = parts
                .next()
                .and_then(|t| t.parse().ok())
                .ok_or_else(|| "usage: edit deal <seat>".to_string())?;
            if seat >= ws.players {
                return Err(format!("seat must be 0-{}", ws.players - 1));
            }
            let seat_id = PlayerId::new(seat);
            ws.placed.retain(|(_, at)| !matches!(at, CardLocation::Deal { seat } if *seat == seat_id));
            let (deal, choice) = ask_live_deal(input, out, seat, rules, &ws.colors, &mut ws.placed)?;
            ws.known_deals[seat as usize] = deal;
            ws.deal_choices[seat as usize] = choice;
        }
        // The site cards AND the 5-Power tokens both sit ON the tiles, so a
        // re-entered board moves them too: re-asking tiles alone would silently
        // re-attach the OLD site cards / tokens to the NEW tile order -- the
        // same class of unflagged mis-mapping the board check exists to catch,
        // reintroduced by the very path that recovers from it.
        Some("tiles") => {
            ws.tiles = ask_tiles(input, out, ws.players, &ws.legal_tile_counts)?;
            out("  The building-site cards and 5-Power tokens sit on the tiles, so they move\n");
            out("  with them: re-entering the sites and tokens now as well.\n");
            ws.sites = ask_sites(input, out, &ws.tiles, rules, &ws.colors, ws.players, &mut ws.placed)?;
            ws.tokens = ask_tokens(input, out, &ws.tiles, rules, ws.players)?;
        }
        Some("sites") => {
            ws.sites = ask_sites(input, out, &ws.tiles, rules, &ws.colors, ws.players, &mut ws.placed)?;
        }
        Some("tokens") => {
            ws.tokens = ask_tokens(input, out, &ws.tiles, rules, ws.players)?;
        }
        Some("achievements") => {
            ws.achievements = ask_achievements(input, out, rules, &ws.colors)?;
        }
        Some("porta") => {
            ws.port_a =
                ask_orders(input, out, false, rules, &ws.colors, &mut ws.placed)?.try_into().unwrap();
        }
        Some("portb") if ws.config.has_port_b => {
            ws.port_b =
                Some(ask_orders(input, out, true, rules, &ws.colors, &mut ws.placed)?.try_into().unwrap());
        }
        Some("laba") => {
            ws.lab_a = ask_techs(input, out, false, rules, &ws.colors, &ws.all_tech_ids, &mut ws.placed)?
                .try_into()
                .unwrap();
        }
        Some("labb") if ws.config.has_lab_b => {
            ws.lab_b = Some(
                ask_techs(input, out, true, rules, &ws.colors, &ws.all_tech_ids, &mut ws.placed)?
                    .try_into()
                    .unwrap(),
            );
        }
        _ => {
            return Err(format!(
                "'{section}' is not an editable section (colors/seed/save/deal <seat>/tiles/sites/tokens/achievements/porta/portb/laba/labb)"
            ))
        }
    }
    Ok(())
}

/// Plain-words summary of every answer collected so far -- reused verbatim
/// for the initial review and after every edit, so a redisplay always
/// reflects the current `ws` (design §5.5 item 3: "the summary reflects
/// the change"). Card/tech display goes through the existing `tech_name`
/// helper and `O{:02}`/`T{:02}` codes, matching the rest of the CLI's
/// plain-words style (`describe.rs`/`render.rs`).
fn render_live_summary(ws: &LiveWizardState, rules: &Rules) -> String {
    let mut out = String::new();
    out.push_str("--- Setup review ---\n");
    out.push_str("mode: live\n");
    out.push_str(&format!("players: {}\n", ws.players));
    out.push_str(&format!("colors: {}\n", ws.colors.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("start seat: {}\n", ws.start_player.0));
    out.push_str(&format!("seed: {}\n", ws.seed));
    out.push_str(&format!("save path: {}\n", ws.save_path));
    // Echoed in the SAME coordinate system the questions used -- a bare
    // `cell 7: Bank` told an operator nothing about which physical tile that
    // was, which is half of why a mis-ordered board used to survive review.
    let rows = row_lengths(ws.players);
    out.push_str("tiles (widest row first, left to right):\n");
    for (i, t) in ws.tiles.iter().enumerate() {
        let site = ws.sites[i].map(|s| format!(" S{s}")).unwrap_or_default();
        // Compact face letter, matching the board (`render::five_power_face`);
        // blank at the Canal cell, which carries no token.
        let token = ws.tokens[i].map(|e| format!(" 5P:{}", crate::render::five_power_face(e).0)).unwrap_or_default();
        out.push_str(&format!(
            "  cell {i} ({}): {}{}{}\n",
            cell_coords(i, &rows),
            crate::describe::area_type_name(*t),
            site,
            token
        ));
    }
    // One line per card, naming its CATEGORY: the three are one-per-category by
    // construction now (F1), and a summary that listed three bare type names
    // could not show that the rule had been honored.
    out.push_str("achievements:\n");
    for ty in ws.achievements {
        out.push_str(&format!(
            "  {}: {} ({ty:?})\n",
            crate::describe::category_letter(ty.category()),
            crate::describe::achievement_desc(ty),
        ));
    }
    out.push_str(&format!("Port A: {}\n", ws.port_a.iter().map(|c| format!("O{:02}", c.0)).collect::<Vec<_>>().join(", ")));
    if let Some(pb) = &ws.port_b {
        out.push_str(&format!("Port B: {}\n", pb.iter().map(|c| format!("O{:02}", c.0)).collect::<Vec<_>>().join(", ")));
    }
    out.push_str(&format!(
        "Laboratory A: {}\n",
        ws.lab_a.iter().map(|c| tech_name(*c, &rules.techs)).collect::<Vec<_>>().join(", ")
    ));
    if let Some(lb) = &ws.lab_b {
        out.push_str(&format!("Laboratory B: {}\n", lb.iter().map(|c| tech_name(*c, &rules.techs)).collect::<Vec<_>>().join(", ")));
    }
    for seat in 0..ws.players {
        match ws.known_deals[seat as usize] {
            Some(cards) => {
                let keep = match ws.deal_choices[seat as usize] {
                    StartingOrderChoice::Keep(k) => k,
                    StartingOrderChoice::Unknown => 0, // unreachable: a known deal always carries Keep
                };
                // Echoed as the number the operator TYPED, not as the engine's
                // 0-based index: they answered `keep card 1 or 2`, and a review
                // that reads `keep 0` back is the F3 defect in a summary.
                out.push_str(&format!(
                    "Seat {seat} deal: known O{:02}, O{:02} (keep card {})\n",
                    cards[0].0,
                    cards[1].0,
                    menu_number(keep as usize),
                ));
            }
            None => out.push_str(&format!("Seat {seat} deal: hidden\n")),
        }
    }
    out
}

// ---------------------------------------------------------------------
// Sim tail
// ---------------------------------------------------------------------

struct SimWizardState {
    players: u8,
    colors: Vec<PlayerColor>,
    start_player: PlayerId,
    seed: u64,
    save_path: String,
    config: GameConfig,
    pending: PendingSetup,
    choices: Vec<StartingOrderChoice>,
    bots: crate::bot::BotSeats,
}

#[allow(clippy::too_many_arguments)]
fn run_sim_tail(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    rules: Rules,
    players: u8,
    colors: Vec<PlayerColor>,
    start_player: PlayerId,
    seed: u64,
    save_path: String,
) -> Result<crate::repl::Repl, String> {
    let mut config = GameConfig::for_player_count(players);
    config.player_colors = colors.clone();
    config.start_player = start_player;

    // Preview each seat's dealt cards (re-running new_game with the chosen
    // seed/config -- Session::new_sim redoes this internally too,
    // deterministically, so the preview and the real deal always agree).
    let pending = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
    let mut choices: Vec<StartingOrderChoice> = Vec::with_capacity(players as usize);
    for seat in 0..players {
        let keep = ask_sim_deal(input, out, seat, &pending, &rules)?;
        choices.push(StartingOrderChoice::Keep(keep));
    }

    // Asked AFTER the deals, so handing a seat to a bot never changes which
    // questions the setup asked or what they meant: the operator sets the
    // table (including each seat's kept card), and the bot takes the seat over
    // from its first MOVE. That also means the operator has already seen a bot
    // seat's deal preview -- see `bot.rs` on what the panel's hand mask is and
    // is not.
    let bots = ask_bots(input, out, players)?;

    let mut ws =
        SimWizardState { players, colors, start_player, seed, save_path, config, pending, choices, bots };

    loop {
        out(&render_sim_summary(&ws));
        out("confirm / edit <section> / abort?\n");
        let line = input.next().ok_or("input ended mid-wizard")?;
        let line = line.trim().to_string();
        if line.eq_ignore_ascii_case("abort") {
            return Err("aborted".to_string());
        }
        if line == "confirm" {
            break;
        }
        match line.strip_prefix("edit ") {
            Some(section) => match perform_sim_edit(&mut ws, section.trim(), input, out, &rules) {
                Ok(()) => {}
                Err(e) if e == "aborted" => return Err(e),
                Err(e) => out(&format!("  ! {e}\n")),
            },
            None => out(&format!("  ! '{line}' is not 'confirm', 'edit <section>', or 'abort'\n")),
        }
    }

    let mut session = Session::new_sim(ws.config, ws.seed, ws.choices, rules)?;
    session.save_path = Some(std::path::PathBuf::from(ws.save_path));
    // Stamp WHO plays each seat BEFORE the Repl is built: `Repl::new` reads
    // its bot table back out of provenance, so a fresh game and a resumed save
    // take one path (and every save this session writes carries the labels).
    session.set_run_seats(ws.bots.seat_provenance());
    Ok(crate::repl::Repl::new(session))
}

/// Sim's editable sections are the subset of the live tail's that apply
/// without a board/ports/labs: `colors`, `seed`, `save`, `deal <seat>`
/// (re-preview + re-ask that seat's keep choice). Editing `seed`
/// recomputes `pending` so the preview stays honest about what the new
/// seed actually deals.
fn perform_sim_edit(
    ws: &mut SimWizardState,
    section: &str,
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    rules: &Rules,
) -> Result<(), String> {
    let mut parts = section.split_whitespace();
    match parts.next() {
        Some("colors") => {
            ws.colors = ask_colors(input, out, ws.players)?;
            ws.config.player_colors = ws.colors.clone();
        }
        Some("seed") => {
            ws.seed = ask_seed(input, out)?;
            ws.config.setup_seed = ws.seed;
            ws.pending = new_game(ws.config.clone(), rules, ws.seed).map_err(|e| e.to_string())?;
            // A new seed deals different cards, so every previously-answered
            // keep choice referred to a preview the operator can no longer
            // see -- `StartingOrderChoice::Keep(k)` is just an index (fix
            // wave, review of 0a01c95 finding 1: it stays "valid" against
            // ANY deal, so nothing would otherwise catch a stale answer
            // silently keeping the wrong card under the new deal). Re-ask
            // every seat from scratch, re-showing each one's actual new
            // preview -- the same "re-collect, don't reuse" treatment
            // `edit deal <seat>` already gives a single seat.
            let mut choices = Vec::with_capacity(ws.players as usize);
            for seat in 0..ws.players {
                let keep = ask_sim_deal(input, out, seat, &ws.pending, rules)?;
                choices.push(StartingOrderChoice::Keep(keep));
            }
            ws.choices = choices;
        }
        Some("save") => {
            ws.save_path = ask_save_path(input, out, ws.seed)?;
        }
        Some("deal") => {
            let seat: u8 = parts
                .next()
                .and_then(|t| t.parse().ok())
                .ok_or_else(|| "usage: edit deal <seat>".to_string())?;
            if seat >= ws.players {
                return Err(format!("seat must be 0-{}", ws.players - 1));
            }
            let keep = ask_sim_deal(input, out, seat, &ws.pending, rules)?;
            ws.choices[seat as usize] = StartingOrderChoice::Keep(keep);
        }
        // Re-asks the whole question (and re-draws the seed when the new
        // answer has any bot in it), exactly like every other section here:
        // "re-collect, don't patch" is the rule that keeps a stale answer from
        // surviving an edit.
        Some("bots") => {
            ws.bots = ask_bots(input, out, ws.players)?;
        }
        _ => return Err(format!("'{section}' is not an editable section (colors/seed/save/deal <seat>/bots)")),
    }
    Ok(())
}

fn render_sim_summary(ws: &SimWizardState) -> String {
    let mut out = String::new();
    out.push_str("--- Setup review ---\n");
    out.push_str("mode: sim\n");
    out.push_str(&format!("players: {}\n", ws.players));
    out.push_str(&format!("colors: {}\n", ws.colors.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("start seat: {}\n", ws.start_player.0));
    out.push_str(&format!("seed: {}\n", ws.seed));
    out.push_str(&format!("save path: {}\n", ws.save_path));
    let bot_seats: Vec<String> = (0..ws.players)
        .filter(|s| ws.bots.is_bot(PlayerId::new(*s)))
        .map(|s| s.to_string())
        .collect();
    if bot_seats.is_empty() {
        out.push_str("bots: none\n");
    } else {
        out.push_str(&format!(
            "bots: seat {} playing {} (seed {})\n",
            bot_seats.join(", "),
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.as_str(),
            ws.bots.seed(),
        ));
    }
    for (seat, choice) in ws.choices.iter().enumerate() {
        if let StartingOrderChoice::Keep(k) = choice {
            // The typed number again, for the reason the live summary states.
            out.push_str(&format!("Seat {seat} keep: card {}\n", menu_number(*k as usize)));
        }
    }
    out
}

/// One seat's live-observed deal: known (2 order cards through the card
/// grammar, plus a keep choice) or hidden (nothing else asked).
fn ask_live_deal(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    seat: u8,
    rules: &Rules,
    colors: &[PlayerColor],
    placed: &mut Vec<(CardRef, CardLocation)>,
) -> Result<(Option<[OrderCardId; 2]>, StartingOrderChoice), String> {
    let mut known: Option<bool> = None;
    ask(input, out, &format!("Seat {seat}: deal known or hidden? (known/hidden)"), &mut |s: &str| -> Result<(), String> {
        match s.to_lowercase().as_str() {
            "known" => {
                known = Some(true);
                Ok(())
            }
            "hidden" => {
                known = Some(false);
                Ok(())
            }
            _ => Err(format!("'{s}' is not 'known' or 'hidden'")),
        }
    })?;
    if !known.unwrap() {
        return Ok((None, StartingOrderChoice::Unknown));
    }
    let seat_id = PlayerId::new(seat);
    let mut dealt: Vec<OrderCardId> = Vec::with_capacity(2);
    for which in 0..2usize {
        let ctx = CardCtx { rules, colors, placed };
        let card = ask_card(
            input,
            out,
            &ctx,
            &format!("Seat {seat}: dealt card {} of 2?", menu_number(which)),
            RecordingTarget::DealSetup { seat: seat_id, which, of: 2 },
            false,
            &mut |card| known_order(card, rules),
        )?;
        let CardRef::Order(id) = card else { unreachable!("an order prompt yields order cards") };
        dealt.push(id);
        placed.push((card, CardLocation::Deal { seat: seat_id }));
    }
    let keep = ask_keep(input, out, seat)?;
    Ok((Some([dealt[0], dealt[1]]), StartingOrderChoice::Keep(keep)))
}

/// The order-card pool check every live entry point shares: the card has to be
/// one of the 36 printed ones.
///
/// The grammar hands back an identity without judging it (`card_prompt`'s
/// module doc: it parses, it does not decide what is legal), so this is the
/// wizard's own answer to "is that a real card" — the same check the old
/// `ask_order_answer` made, minus the duplicate half, which now lives with the
/// locations that can explain it.
fn known_order(card: CardRef, rules: &Rules) -> Result<(), String> {
    match card {
        CardRef::Order(id) if rules.orders.get(id).is_some() => Ok(()),
        CardRef::Order(id) => Err(format!("O{:02} is not one of the 36 order cards", id.0)),
        other => Err(format!("{} is not an order card", card_prompt::card_tag(other))),
    }
}

/// One seat's sim-preview + keep choice: prints the actual seeded deal
/// (from `pending`, which the caller re-derives on a seed edit) and asks
/// which of the two to keep.
fn ask_sim_deal(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    seat: u8,
    pending: &PendingSetup,
    rules: &Rules,
) -> Result<u8, String> {
    out(&format!("Seat {seat} dealt:\n"));
    for (i, &card_id) in pending.deals[seat as usize].iter().enumerate() {
        if let Some(card) = rules.orders.get(card_id) {
            // The Keep valuation the BOTS use, shown beside the card the
            // operator is choosing for (spec §8: the Keep exists and is
            // explained). It is the same `greedy::score_starting_order` both
            // evaluator rungs argmax over -- read here rather than restated, so
            // the wizard cannot describe a Keep the bots do not make.
            out(&format!(
                "  [{}] {}  (bot Keep score {})\n",
                menu_number(i),
                order_prompt_line(card),
                greedy::score_starting_order(card, rules),
            ));
        }
    }
    // Stated every time rather than once at the top of the wizard: it is the
    // one line that stops a bare number from reading as an authority.
    out(KEEP_SCORE_NOTE);
    ask_keep(input, out, seat)
}

/// What the `bot Keep score` beside each dealt card is, and what it is not.
///
/// Spec §8's own terms: `score_starting_order` reads the card's PRINTED
/// economics (VP, money, goods, movement and warehouse rewards, minus its
/// requirements, plus a country-scarcity term) and is **hand-set and
/// unmeasured**, revisited at 3.5. Saying so beside it is the difference
/// between a factual number and an implied recommendation -- the same rule the
/// `suggest` screen closes with.
///
/// Hand-wrapped to two lines under 80 columns for the same reason every other
/// width in this driver is a fixed budget: std exposes no terminal size, and
/// the guide promises a 140-column window clears every layout.
const KEEP_SCORE_NOTE: &str = "  (the Keep score is what the built-in bot would prefer, from the card's printed\n   \
     economics alone -- hand-set, unmeasured, and no substitute for your own read)\n";

// ---------------------------------------------------------------------
// Board entry (third review, finding I1)
//
// Cells are asked for in row-major order over the staggered pyramid, widest
// row FIRST -- which is the vertical mirror of the way the rulebook's p.3
// diagram prints the same board. `reconcile_board` accepts any permutation of
// the tile multiset without complaint, so an operator who follows that picture
// plays an entire game on a scrambled graph and is never told. The three
// functions below are the guard rails: the shape and the rules stated up front
// (`tile_entry_preamble`), the cell's position repeated on every single
// question (`cell_coords`, `row_banner`), and the finished board drawn back
// for confirmation (`ask_board_check`).
// ---------------------------------------------------------------------

/// Where cell `i` sits, in the words both the tile and the site question use:
/// `row 2 of 4, tile 1 of 3`. Rows and positions count from 1 (an operator at
/// a table counts tiles, not indices); the CELL number stays 0-based because
/// it is the area id every other CLI surface names.
fn cell_coords(i: usize, rows: &[u8]) -> String {
    let (r, c) = area_id_to_cell(AreaId::new(i as u8), rows);
    format!("row {} of {}, tile {} of {}", r + 1, rows.len(), c + 1, rows[r as usize])
}

/// Announces each row as entry reaches it, so the coordinates survive the
/// skeleton scrolling off the top of a real terminal.
fn row_banner(r: usize, rows: &[u8]) -> String {
    let first: usize = rows[..r].iter().map(|&l| l as usize).sum();
    let len = rows[r] as usize;
    format!(
        "-- row {} of {}: {} tile{}, cells {}-{}, left to right --\n",
        r + 1,
        rows.len(),
        len,
        if len == 1 { "" } else { "s" },
        first,
        first + len - 1
    )
}

/// What the operator reads before typing a single tile: the numbered skeleton
/// of the real shape, then the orientation in words. The picture alone still
/// leaves a mirror ambiguity for someone holding the rulebook, so the words
/// carry the load -- which physical row is entered first, which tile is cell
/// 0, and the fact that the printed diagram is upside down relative to this.
fn tile_entry_preamble(rows: &[u8], players: u8) -> String {
    let lengths: Vec<String> = rows.iter().map(|l| l.to_string()).collect();
    let (last, rest) = lengths.split_last().expect("every board has rows");
    let widest = rows[0];
    let narrowest = *rows.last().unwrap();
    let mut out = String::from("\n--- Board tiles ---\n");
    out.push_str(&format!(
        "This board is a staggered pyramid: {} rows, of {} and {last} tiles, widest row first.\n\
         The wizard asks for the cells in this order:\n\n",
        rows.len(),
        rest.join(", ")
    ));
    out.push_str(&crate::render::board_skeleton(players));
    out.push_str(&format!(
        "\nOrient the board before you type:\n\
         * Turn the assembled board so its widest row ({widest} tiles) is the row FARTHEST\n\
         \x20 from you, and the narrowest row ({narrowest} tile{}) is the one NEAREST you.\n\
         * Cell 0 is the leftmost tile of that widest far row. Numbering runs left to right\n\
         \x20 along a row, then down to the next row, the way you read a page.\n\
         * Read every row in the SAME direction. Which end you start from does not matter --\n\
         \x20 a consistently mirrored board is the same board to the engine -- but switching\n\
         \x20 direction partway is not, and nor is changing the row order.\n\
         * The rulebook's p.3 diagram prints this pyramid THE OTHER WAY UP, narrowest row at\n\
         \x20 the top. Do not follow that picture's reading order: enter the widest row first.\n\
         \x20 The engine models the adjacency graph, not the printed picture.\n\
         Each question lists the tiles still to place, with how many of each are left --\n\
         \x20 type the number beside one, its full name, or its old short code (cm, sm, ...).\n\
         The finished board is drawn back for you to check before play begins.\n\n",
        if narrowest == 1 { "" } else { "s" }
    ));
    out
}

/// Board tiles: every grid cell's `AreaType`, validated against the legal
/// per-type multiset (design §5.5 item 1). A fresh working copy of
/// `legal_counts` is taken here every call, so both the initial pass and
/// an `edit tiles` re-run start from the same full pool -- and both get the
/// full preamble, since `edit tiles` is exactly the path an operator takes
/// after the board check told them the order was wrong.
fn ask_tiles(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    players: u8,
    legal_counts: &HashMap<AreaType, u32>,
) -> Result<Vec<AreaType>, String> {
    let rows = row_lengths(players);
    let n: usize = rows.iter().map(|&l| l as usize).sum();
    out(&tile_entry_preamble(&rows, players));

    let roster = tile_roster(legal_counts);
    let mut remaining = legal_counts.clone();
    let mut tiles: Vec<AreaType> = Vec::with_capacity(n);
    let mut row = 0usize;
    let mut row_end = rows[0] as usize;
    out(&row_banner(0, &rows));
    for i in 0..n {
        if i == row_end {
            row += 1;
            row_end += rows[row] as usize;
            out(&row_banner(row, &rows));
        }
        // The offer is fixed for the duration of ONE cell -- the counts only
        // change when an answer is taken, and that ends this question -- so the
        // menu is built once and rides in the prompt, where a re-ask reprints it
        // beside the question rather than leaving it scrolled away.
        let offer = tile_offer(&roster, &remaining);
        let mut chosen: Option<AreaType> = None;
        ask(
            input,
            out,
            &format!(
                "{}Cell {i}/{n} ({}) tile? (a number, a name, or its short code)",
                tile_menu(&offer, players),
                cell_coords(i, &rows)
            ),
            &mut |s: &str| -> Result<(), String> {
                let area = parse_tile_entry(s, &offer, legal_counts, players)?;
                match remaining.get_mut(&area) {
                    Some(k) if *k > 0 => {
                        *k -= 1;
                        chosen = Some(area);
                        Ok(())
                    }
                    // Unreachable from `parse_tile_entry`, which refuses an
                    // exhausted type with the count it ran out of; kept total
                    // rather than unwrapping on a map the caller supplied.
                    _ => Err(format!(
                        "no more {} tiles for this board",
                        crate::describe::area_type_name(area)
                    )),
                }
            },
        )?;
        tiles.push(chosen.unwrap());
    }
    Ok(tiles)
}

/// Every tile type THIS BOARD uses, in `AREA_TYPE_ENTRY` order — computed once
/// before the pass and never recomputed.
///
/// **This is the numbering, and its being fixed is the point** (T7 review S1).
/// The offer used to be the types with copies LEFT, so the numbers were assigned
/// over a list that shrank: at 2 players, answering `1` for the single Copper
/// Mine dropped that row and made `1` mean Silk Mill at the very next question,
/// then Tea Plantation, and so on — ten questions, ten different meanings for
/// the same key, at the one prompt an operator answers by rote. The roster is
/// the whole legal set including the types that are used up, so a number names
/// one tile type for the entire pass.
fn tile_roster(legal_counts: &HashMap<AreaType, u32>) -> Vec<AreaType> {
    AREA_TYPE_ENTRY
        .iter()
        .filter(|(area, _)| legal_counts.contains_key(area))
        .map(|(area, _)| *area)
        .collect()
}

/// The roster with how many of each type is still to place — zeros included, so
/// a position in this list is a stable answer number ([`tile_roster`]).
///
/// The COUNTS still shrink, which is F2's half of this prompt: the menu says
/// what is left, and a type that has run out drops off the screen. What no
/// longer moves is where the surviving rows are numbered.
fn tile_offer(roster: &[AreaType], remaining: &HashMap<AreaType, u32>) -> Vec<(AreaType, u32)> {
    roster.iter().map(|area| (*area, remaining.get(area).copied().unwrap_or(0))).collect()
}

/// The offer as a numbered menu — **finding F2, fixed at the surface where the
/// owner hit it.**
///
/// The prompt used to end `(cm/sm/tp/fg/bank/pa/pb/la/lb/ea/ct/ch/cu/canal)`:
/// one static list of every abbreviation in the game, at every player count. So
/// a 2-player setup offered `canal` — a tile that is *"Removed from the board
/// entirely at 2 players"* — the operator typed it, and the validator refused
/// it. The engine was right the whole time; the prompt was advertising an
/// answer it would not take. This offers what is actually placeable, with the
/// count still to place, shrinking as the board fills.
///
/// Full names, with the short codes kept as accepted aliases (R1), and numbers
/// that work (F3) and stay put ([`tile_roster`]).
///
/// A type with nothing left is DROPPED rather than shown at zero: the menu
/// answers "what can I still place", and a row reading `Copper Mine x0` is a row
/// whose number the next answer would refuse. Its number goes with it and is
/// never handed to another tile.
fn tile_menu(offer: &[(AreaType, u32)], players: u8) -> String {
    numbered_offer(
        &format!("  Tiles still to place on this {players}-player board:\n"),
        offer.iter().enumerate().filter(|(_, (_, left))| *left > 0).map(|(position, (area, left))| {
            (
                menu_number(position),
                format!("{} x{left}", crate::describe::area_type_name(*area)),
            )
        }),
    )
}

/// A numbered offer, three to a row — the layout both board-entry menus use.
///
/// **The number is the CALLER's, not this list's position**, since S1: the tile
/// menu numbers over a fixed roster and prints only the rows with copies left,
/// so its third printed cell can legitimately be `[5]`. Rows still break three
/// to a printed cell, which is what keeps the grid rectangular with gaps in it.
///
/// Numbered from [`menu_number`] by both callers, and 26 columns to a cell. The
/// row fits 80 columns because only the FIRST TWO cells pay their padding — the
/// third is trimmed to its own width — so a full row costs
/// `3 + 26 + 26 + widest`, not `3 + 26 * 3`, which would be 81 and wrap.
/// `numbered_offer_fits_three_cells_in_eighty_columns` measures `widest` over
/// every menu both callers can produce rather than leaving the figure in prose;
/// at today's data it is 25 (`[10] Employment Agency x2`), i.e. one column of
/// headroom.
fn numbered_offer(header: &str, cells: impl Iterator<Item = (usize, String)>) -> String {
    let mut out = header.to_string();
    let mut count = 0usize;
    for (i, (number, cell)) in cells.enumerate() {
        count = i + 1;
        let cell = format!("[{number}] {cell}");
        if i % 3 == 0 {
            out.push_str("   ");
        }
        out.push_str(&format!("{cell:<26}"));
        if i % 3 == 2 {
            out.push('\n');
        }
    }
    if !count.is_multiple_of(3) {
        out.push('\n');
    }
    out
}

/// One tile answer: a menu number, a short code, or a name (any unambiguous
/// start of one).
///
/// The three refusals say DIFFERENT things, which is the whole point of F2: a
/// type this player count does not use is not a mistake about the board's
/// progress, it is a tile that is in the box; a type whose copies are all placed
/// is a real tile on this board with nothing left; and a number outside the
/// roster is neither.
///
/// **The number indexes the ROSTER, not the visible rows** ([`tile_roster`]), so
/// a number whose row has dropped off the menu reaches the exhausted-type
/// refusal — by name, with the set size — rather than landing on whichever tile
/// happens to sit in that print position now.
fn parse_tile_entry(
    s: &str,
    offer: &[(AreaType, u32)],
    legal_counts: &HashMap<AreaType, u32>,
    players: u8,
) -> Result<AreaType, String> {
    let typed = s.trim();
    if typed.is_empty() {
        return Err("type a number from the list, a tile name, or its short code".to_string());
    }
    if typed.chars().all(|c| c.is_ascii_digit()) {
        return match typed.parse::<usize>().ok().and_then(|k| menu_index(k, offer.len())) {
            Some(k) if offer[k].1 > 0 => Ok(offer[k].0),
            Some(k) => Err(exhausted_message(offer[k].0, legal_counts, players)),
            // The roster is the player count's whole tile set, so it is empty
            // only for a board with no tile types at all — which no `GameConfig`
            // produces and only a hand-built fixture could.
            None => Err(match menu_range(offer.len()) {
                Some((lo, hi)) => format!("'{typed}' is not on the list -- pick {lo}-{hi}"),
                None => "no tiles are left to place".to_string(),
            }),
        };
    }
    let area = match_tile_name(typed, legal_counts)?;
    match (legal_counts.get(&area), offer.iter().any(|(a, left)| *a == area && *left > 0)) {
        (_, true) => Ok(area),
        // In the game but not on THIS board: `canal` at 2 players, the finding
        // by name.
        (None, _) => Err(format!(
            "{} is not part of a {players}-player board -- that tile stays in the box",
            crate::describe::area_type_name(area),
        )),
        (Some(_), false) => Err(exhausted_message(area, legal_counts, players)),
    }
}

/// A tile whose copies are all placed, refused with the set size — the same
/// sentence whether the operator named it or typed its still-valid number.
fn exhausted_message(
    area: AreaType,
    legal_counts: &HashMap<AreaType, u32>,
    players: u8,
) -> String {
    match legal_counts.get(&area) {
        Some(total) => format!(
            "no more {} tiles for this board -- the {players}-player set has {total}",
            crate::describe::area_type_name(area),
        ),
        // Not on this board at all: only reachable through a roster built from
        // a different `legal_counts` than the one being checked against.
        None => format!(
            "{} is not part of a {players}-player board -- that tile stays in the box",
            crate::describe::area_type_name(area),
        ),
    }
}

/// A tile named by its short code or by any unambiguous start of its name.
///
/// Ambiguity is refused rather than guessed -- the same discipline
/// `card_prompt` holds for goods, flags and technology titles, and it earns its
/// keep here: `c` starts Copper Mine, Chinatown, Church, Customs and the Canal.
///
/// **Matched over all FOURTEEN types but disambiguated against THIS board.**
/// Both halves are deliberate. Matching over all fourteen is what lets `canal`
/// at 2 players reach the message that names the finding — a tile that is in
/// the box, not a word the wizard has never heard of. Disambiguating against
/// the player count's own set is what keeps that from making the prompt worse:
/// a prefix matching several types of which only ONE is on this board is not
/// ambiguous at this table, and one that is genuinely ambiguous lists only the
/// tiles the operator could actually be holding.
fn match_tile_name(
    typed: &str,
    legal_counts: &HashMap<AreaType, u32>,
) -> Result<AreaType, String> {
    let needle = typed.to_lowercase();
    if let Some((area, _)) = AREA_TYPE_ENTRY.iter().find(|(_, code)| *code == needle) {
        return Ok(*area);
    }
    let hits: Vec<AreaType> = AREA_TYPE_ENTRY
        .iter()
        .filter(|(area, _)| crate::describe::area_type_name(*area).to_lowercase().starts_with(&needle))
        .map(|(area, _)| *area)
        .collect();
    if hits.is_empty() {
        return Err(format!(
            "'{typed}' is not a tile -- type a number from the list, a name, or a short code"
        ));
    }
    let on_this_board: Vec<AreaType> =
        hits.iter().copied().filter(|area| legal_counts.contains_key(area)).collect();
    match (hits.as_slice(), on_this_board.as_slice()) {
        ([only], _) => Ok(*only),
        // Several types start this way, but only one of them is in the game at
        // this player count -- so there is exactly one tile it can mean.
        (_, [only]) => Ok(*only),
        // Genuinely ambiguous. Name the candidates that are on this board; if
        // none is, name them all rather than refusing with an empty list.
        (all, some) => {
            let candidates = if some.is_empty() { all } else { some };
            let names: Vec<&str> =
                candidates.iter().map(|a| crate::describe::area_type_name(*a)).collect();
            Err(format!("'{typed}' could be {} -- type more of the name", names.join(", ")))
        }
    }
}

/// A building-site index for every non-Canal cell (Canal cells carry no
/// site and are not asked about). Distinctness is self-contained to this
/// call (a fresh tracker every time, initial pass or `edit sites` alike).
///
/// This pass walks the same cells in the same order as `ask_tiles`, so it
/// carries the same coordinates -- plus the tile already entered there, which
/// is what actually lets an operator find the physical card they are holding.
fn ask_sites(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    tiles: &[AreaType],
    rules: &Rules,
    colors: &[PlayerColor],
    players: u8,
    placed: &mut Vec<(CardRef, CardLocation)>,
) -> Result<Vec<Option<u8>>, String> {
    let rows = row_lengths(players);
    let n = tiles.len();
    // Re-entrant like the port and lab passes: this walks every cell, so an
    // `edit sites` (or the `edit tiles` that drags the sites with it) starts
    // from the full registry rather than from its own previous answers.
    placed.retain(|(card, _)| !matches!(card, CardRef::Site(_)));
    out("\n--- Building-site cards ---\n");
    out("Same cells in the same order as the tiles: the widest row first, left to right.\n");
    out("Each question names the tile you entered there. Canal cells carry no site and\n");
    out("are skipped. The cards print no number, so '?' draws all 24 with their spaces --\n");
    out("the index number and the S<n> the wizard records are the same number.\n");

    let mut sites: Vec<Option<u8>> = Vec::with_capacity(n);
    let mut row = 0usize;
    let mut row_end = rows[0] as usize;
    out(&row_banner(0, &rows));
    for (i, tile) in tiles.iter().enumerate() {
        if i == row_end {
            row += 1;
            row_end += rows[row] as usize;
            out(&row_banner(row, &rows));
        }
        if *tile == AreaType::Canal {
            sites.push(None);
            continue;
        }
        let area = AreaId::new(i as u8);
        let ctx = CardCtx { rules, colors, placed };
        let card = ask_card(
            input,
            out,
            &ctx,
            &format!(
                "Cell {i}/{n} ({}) {} -- building-site card?",
                cell_coords(i, &rows),
                crate::describe::area_type_name(*tile)
            ),
            RecordingTarget::SiteSetup { area },
            false,
            &mut |card| match card {
                CardRef::Site(index) if usize::from(index) < rules.building_sites.len() => Ok(()),
                CardRef::Site(index) => Err(format!(
                    "S{index} is not one of the {} building-site cards",
                    rules.building_sites.len()
                )),
                other => {
                    Err(format!("{} is not a building-site card", card_prompt::card_tag(other)))
                }
            },
        )?;
        let CardRef::Site(index) = card else { unreachable!("a site prompt yields site cards") };
        sites.push(Some(index));
        placed.push((card, CardLocation::OnArea { area }));
    }
    Ok(sites)
}

/// A face-up 5-Power token for every non-Canal cell (Canal cells carry no
/// token and are not asked about -- `None` is pushed there). Rulebook p.3
/// step 5: the tokens are placed FACE-UP, so their arrangement is open
/// information observed at setup, exactly like the building-site cards --
/// which is why this pass exists at all, and why the redundant claim-time
/// reveal is retired (the layout already holds the true faces).
///
/// Walks the same cells in the same order as `ask_tiles`/`ask_sites`, so it
/// carries the same coordinates -- plus the tile already entered there. Each
/// answer is validated live against a decrementing copy of the canonical token
/// pool (`rules.five_power_tokens`: 10 import, 2 each of copper/silk/tea/
/// seafood/yen), so a third copper is refused AT THE QUESTION and re-asks --
/// the same sub-multiset tripwire `reconcile_board` applies as a backstop,
/// caught early like the duplicate-site check.
fn ask_tokens(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    tiles: &[AreaType],
    rules: &Rules,
    players: u8,
) -> Result<Vec<Option<FivePowerEffect>>, String> {
    let rows = row_lengths(players);
    let n = tiles.len();
    out("\n--- 5-Power tokens ---\n");
    out("The 5-Power tokens are placed FACE-UP (rulebook p.3 step 5): enter the one you\n");
    out("see on each tile. Same cells in the same order as the tiles, each question naming\n");
    out("the tile you entered there; Canal cells carry no token and are skipped. The menu\n");
    out("counts down as you go, and the old letters (i/c/s/t/f/y) still work.\n");

    let roster = token_roster(&rules.five_power_tokens);
    let mut pool = rules.five_power_tokens.clone();
    let mut tokens: Vec<Option<FivePowerEffect>> = Vec::with_capacity(n);
    let mut row = 0usize;
    let mut row_end = rows[0] as usize;
    out(&row_banner(0, &rows));
    for (i, tile) in tiles.iter().enumerate() {
        if i == row_end {
            row += 1;
            row_end += rows[row] as usize;
            out(&row_banner(row, &rows));
        }
        if *tile == AreaType::Canal {
            tokens.push(None);
            continue;
        }
        // Fixed for this one question, exactly as the tile offer is.
        let offer = token_offer(&roster, &pool);
        let mut chosen: Option<FivePowerEffect> = None;
        ask(
            input,
            out,
            &format!(
                "{}Cell {i}/{n} ({}) {} -- 5-Power token? (a number or its letter)",
                token_menu(&offer),
                cell_coords(i, &rows),
                crate::describe::area_type_name(*tile)
            ),
            &mut |s: &str| -> Result<(), String> {
                let effect = parse_token_entry(s, &offer)?;
                match pool.iter().position(|p| *p == effect) {
                    Some(idx) => {
                        // ORDER-PRESERVING, and it has to be: `token_offer`
                        // reads the menu off this pool in first-seen order, so a
                        // `swap_remove` here — which is what shipped — dropped
                        // the pool's LAST face into the hole and put it at the
                        // head of the next menu. Answering `1` on cell 0 gave
                        // an imported good; answering `1` on cell 1 gave yen.
                        // The rows moved, silently, mid-pass (T7 review S3,
                        // found by the pin rather than by reading).
                        pool.remove(idx);
                        chosen = Some(effect);
                        Ok(())
                    }
                    // Unreachable from `parse_token_entry`, which refuses a face
                    // the pool has run out of; kept total all the same.
                    None => Err(format!(
                        "no more {} tokens available for this board",
                        crate::describe::five_power_effect_label(effect)
                    )),
                }
            },
        )?;
        tokens.push(Some(chosen.unwrap()));
    }
    Ok(tokens)
}

/// Every face the box holds, in the canonical pool order
/// (`rules.five_power_tokens`' own first-seen order) — computed once before the
/// pass and never recomputed.
///
/// **This is the numbering, and its being fixed is the point** (T7 review
/// round 2, item 3 — the tile treatment S1 got, finished here). The offer was
/// the faces still IN the pool, numbered over that list, so the two-copy faces
/// took their numbers with them as they ran out and everything below shuffled
/// up: `4` was seafood for fourteen questions and then tea. The pass is asked
/// up to seventeen times in a row off a physical board, which is exactly the
/// question a person stops re-reading.
fn token_roster(pool: &[FivePowerEffect]) -> Vec<FivePowerEffect> {
    let mut roster: Vec<FivePowerEffect> = Vec::new();
    for face in pool {
        if !roster.contains(face) {
            roster.push(*face);
        }
    }
    roster
}

/// The roster with how many of each face is still to place — zeros included, so
/// a position in this list is a stable answer number ([`token_roster`]).
///
/// The COUNTS still count down, which is what makes the menu an answer to "what
/// is left"; a face that has run out drops off the screen. What no longer moves
/// is where the surviving rows are numbered.
fn token_offer(
    roster: &[FivePowerEffect],
    pool: &[FivePowerEffect],
) -> Vec<(FivePowerEffect, usize)> {
    roster
        .iter()
        .map(|face| (*face, pool.iter().filter(|p| *p == face).count()))
        .collect()
}

/// The token offer as a numbered menu, in plain words.
///
/// The same treatment the tile question gets and for the same reason (R1/F3):
/// the prompt used to end `(i/c/s/t/f/y)` over a six-line legend, which is six
/// letters to memorize for a question asked seventeen times.
/// A face with none left is DROPPED rather than shown at zero, on
/// [`tile_menu`]'s reasoning: the menu answers "what can I still place", and its
/// number goes with it rather than to another face.
fn token_menu(offer: &[(FivePowerEffect, usize)]) -> String {
    numbered_offer(
        "  Tokens still to place:\n",
        offer.iter().enumerate().filter(|(_, (_, left))| *left > 0).map(|(position, (face, left))| {
            (
                menu_number(position),
                format!("{} x{left}", crate::describe::five_power_effect_label(*face)),
            )
        }),
    )
}

/// One token answer: a menu number, or the face's letter (the alias the guide
/// and every existing script use).
fn parse_token_entry(
    s: &str,
    offer: &[(FivePowerEffect, usize)],
) -> Result<FivePowerEffect, String> {
    let typed = s.trim();
    if typed.chars().all(|c| c.is_ascii_digit()) && !typed.is_empty() {
        // The number indexes the ROSTER, not the visible rows ([`token_roster`]),
        // so a number whose row has dropped off reaches the exhausted-face
        // refusal by NAME rather than landing on whichever face now sits in that
        // print position.
        return match typed.parse::<usize>().ok().and_then(|k| menu_index(k, offer.len())) {
            Some(k) if offer[k].1 > 0 => Ok(offer[k].0),
            Some(k) => Err(token_exhausted_message(offer[k].0)),
            // The roster is every face the box holds, so it is empty only for a
            // pool with no faces at all — which no `Rules` produces.
            None => Err(match menu_range(offer.len()) {
                Some((lo, hi)) => format!("'{typed}' is not on the list -- pick {lo}-{hi}"),
                None => "no tokens are left to place".to_string(),
            }),
        };
    }
    let effect = crate::reveal::parse_token_letter(&[typed])?;
    if offer.iter().any(|(face, left)| *face == effect && *left > 0) {
        Ok(effect)
    } else {
        Err(token_exhausted_message(effect))
    }
}

/// A face whose copies are all placed, refused with the same sentence whether
/// the operator typed its letter or its still-valid number.
fn token_exhausted_message(effect: FivePowerEffect) -> String {
    format!(
        "no more {} tokens available for this board",
        crate::describe::five_power_effect_label(effect)
    )
}

/// The confirmation checkpoint the live gate turns on (required outcome 3):
/// draw the assembled board with the SAME renderer play uses, and make the
/// operator say out loud that it matches the physical table before a single
/// move is entered. Nothing else is on screen for it, deliberately -- buried
/// inside the setup summary this is a picture people skim past, and it is the
/// only check that can catch a board entered in the wrong order (the engine's
/// own `reconcile_board` accepts every permutation).
///
/// `no` is not a failure: the caller returns to the review loop, where
/// `edit tiles` re-enters the board. `abort` still works here, like every
/// other question -- `ask` checks it before this parser ever runs.
fn ask_board_check(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    session: &Session,
) -> Result<bool, String> {
    out("\n--- Board check ---\n");
    out("This is the board the engine will play. Compare it with the physical table: the\n");
    out("SHAPE first (widest row on top), then the tile in every cell.\n\n");
    // No legal list (`&[]`): the board check is a SETUP screen, and the `◈`
    // destination marks belong to a Movement decision that has not been reached.
    out(&crate::render::board_grid(session.state(), session.rules(), &[]));
    out("\nA00 is cell 0, the leftmost tile of the widest top row; ids run left to right\n");
    out("along each row, then down to the next row.\n");
    let notice = dummy_placement_notice(session.state());
    out(&notice);
    // §1.2 -- the observing half: the board-match question the operator MUST
    // answer NAMES the dummy pawns for a 2-player table, so a "yes" explicitly
    // confirms the physical pawns are placed. Their positions are rules-fixed,
    // so this catches only operator OMISSION -- the one way a 2P table diverges
    // from the tracked game. A 3-/4-player board has no dummy (empty notice) and
    // gets the plain question.
    let question = if notice.is_empty() {
        "Does this match your physical board? (yes/no)".to_string()
    } else {
        "Does this match your physical board? (At 2 players a 'yes' also confirms the two neutral \
         dummy pawns above are physically placed.) (yes/no)"
            .to_string()
    };
    let mut matches: Option<bool> = None;
    ask(input, out, &question, &mut |s: &str| -> Result<(), String> {
        match s.to_lowercase().as_str() {
            "yes" | "y" => {
                matches = Some(true);
                Ok(())
            }
            "no" | "n" => {
                matches = Some(false);
                Ok(())
            }
            _ => Err(format!("'{s}' is not 'yes' or 'no'")),
        }
    })?;
    Ok(matches.unwrap()) // `ask` returned Ok(()) only after this was set
}

/// The 2-player dummy is the one piece of setup nothing in the live path
/// observes: `SetupObservations` carries no track state, so the engine places
/// both neutral assistants itself (`game::setup::place_setup_dummies`) and
/// never asks whether the table did. An operator who forgets the physical
/// pawns then plays a whole game against a tool that refuses two spaces they
/// see as open, ends the game a pawn early on either track, and awards part
/// of both majority bonuses to a rival nobody at the table can see. Observing
/// them is core work and deferred; SAYING where they go is not, and the board
/// check is where it belongs -- the last screen with a question still to
/// answer, and so the last one a real terminal shows before the first
/// composed frame clears everything above it (main.rs's I-1 note).
///
/// Derived from the built session rather than from the player count, so the
/// spaces named are the ones the engine actually occupied and cannot drift
/// from `place_setup_dummies`. A board carrying no dummy -- every 3- and
/// 4-player game -- yields the empty string, which `out` swallows.
fn dummy_placement_notice(state: &GameState) -> String {
    let mut spots: Vec<String> = state
        .church_track
        .slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| matches!(slot, Some(s) if s.player.is_dummy()))
        .map(|(i, _)| format!("Church faith-{}", ChurchTrack::index_to_faith(i)))
        .collect();
    spots.extend(
        state
            .customs_track
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| matches!(slot, Some(s) if s.player.is_dummy()))
            .map(|(i, _)| format!("Customs space-{}", CustomsTrack::index_to_space(i))),
    );
    if spots.is_empty() {
        return String::new();
    }
    // `track_trigger` counts REAL pawns -- `total_assistants` skips the dummy
    // -- so it is exactly the ordinal an operator counts to at the table. Its
    // three values (4/5/6) all take "th".
    format!(
        "\n{}-player setup -- place the neutral dummy assistants before the first move:\n\
         * One goes on {}.\n\
         \x20 Use pawns of an unused player color. They are NOT printed board art: nothing\n\
         \x20 you entered above describes them, and the engine has both either way, so only\n\
         \x20 your table can be missing them.\n\
         * Each one BLOCKS its space against both of you, counts toward the occupied\n\
         \x20 spaces that end the game on its track (so your {}th assistant there is the\n\
         \x20 last one), and COMPETES in that track's majority bonus at final scoring.\n\
         * The 'tracks' view shows them as D once play starts.\n\n",
        state.config.player_count,
        spots.join(" and another on "),
        state.config.track_trigger(),
    )
}

/// The three Achievement cards in play — **one from each category, asked one
/// category at a time** (finding F1, spec §0.10).
///
/// **The rule is structural here, not a validation bolted onto a free-for-all.**
/// The rulebook deals one card from each of the three categories (KB
/// §Achievement Cards; `game::setup` does exactly that from `ACHIEVEMENTS_A/B/C`
/// and the per-category VP tiers depend on it), and the old prompt accepted any
/// three distinct types — so a live game could be set up with three category-A
/// cards and nothing anywhere would say otherwise. Asking per category means
/// the illegal setup cannot be typed rather than being caught: the only way to
/// answer wrongly is to name a card from another category, and that refusal
/// says which category the card belongs to and which one this question wants.
///
/// **Wizard-side only, and deliberately so.** The engine is already correct and
/// this is a UI validation ahead of it, which is why it is rev-inert: nothing
/// about `reconcile_setup`'s acceptance moves (spec §0.10 — a reconcile-side
/// check is a separate, owner-gated question).
///
/// The numbers are the INDEX's own, over all twelve cards, so the four rows
/// this question prints carry the same numbers the `?` screen does — a menu
/// that shows numbers accepts those numbers (F3), and it accepts the ones the
/// other screen shows too.
fn ask_achievements(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    rules: &Rules,
    colors: &[PlayerColor],
) -> Result<[AchievementType; 3], String> {
    out("\n--- Achievement cards ---\n");
    out("Three cards are in play, ONE FROM EACH CATEGORY -- so the wizard asks for them\n");
    out("one category at a time. Type the number beside a card, its name, or '?' for the\n");
    out("full index of all twelve with their scoring.\n");

    let mut chosen: Vec<AchievementType> = Vec::with_capacity(3);
    let mut placed: Vec<(CardRef, CardLocation)> = Vec::new();
    for category in [AchievementCategory::A, AchievementCategory::B, AchievementCategory::C] {
        let letter = crate::describe::category_letter(category);
        let target = RecordingTarget::AchievementSetup { category };
        let card = {
            let ctx = CardCtx { rules, colors, placed: &placed };
            // The menu rides IN the question, exactly as the tile menu does and
            // for the reason that prompt already states (T7 review S5): a re-ask
            // reprints it beside the question rather than leaving the four cards
            // scrolled away above an index screen or a refusal. It is fixed for
            // the duration of this one question — `placed` only changes when an
            // answer is taken, and that ends it.
            let menu = category_menu(&ctx.index(target.kind(), Some(target), None), category);
            ask_card(
                input,
                out,
                &ctx,
                &format!("{menu}Which category {letter} card is in play?"),
                target,
                // The four cards are printed directly above with their numbers,
                // so a bare number is a selection from the moment the question
                // appears rather than only after a `?`.
                true,
                &mut |card| match card {
                    CardRef::Achievement(ty) if ty.category() == category => Ok(()),
                    CardRef::Achievement(ty) => Err(format!(
                        "{ty:?} is a category {} card -- this question is asking for the category {letter} one",
                        crate::describe::category_letter(ty.category()),
                    )),
                    other => Err(format!(
                        "{} is not an achievement card",
                        card_prompt::card_tag(other)
                    )),
                },
            )?
        };
        let CardRef::Achievement(ty) = card else {
            unreachable!("an achievement prompt yields achievement cards")
        };
        chosen.push(ty);
        placed.push((card, CardLocation::AchievementSlot { category }));
    }
    Ok(chosen.try_into().expect("three categories, three cards"))
}

/// One category's four cards, numbered as the full index numbers them.
fn category_menu(index: &IndexModel, category: AchievementCategory) -> String {
    let mut out = format!(
        "  Category {} -- one of these four:\n",
        crate::describe::category_letter(category)
    );
    for card in index.cards() {
        let crate::screen_model::CardFaceModel::Achievement(a) = &card.face else { continue };
        if a.category != category {
            continue;
        }
        let Some(number) = card.selection else { continue };
        let desc = crate::describe::achievement_desc(a.achievement_type);
        // The number field is padded, not just the description: `[12]` is a
        // column wider than `[1]`, and padding only what follows it shears the
        // VP column on exactly the rows category C occupies.
        out.push_str(&format!(
            "    {:<5}{desc:<46} {}/{} VP   '{:?}'\n",
            format!("[{number}]"),
            a.first_place_vp,
            a.subsequent_vp,
            a.achievement_type,
        ));
    }
    out
}

/// One Port display's six order cards, left to right.
///
/// Duplicates are refused across the WHOLE tail — both seats' deals, Port A and
/// Port B share `placed` (design §5.5 item 1) — and since T7 the refusal names
/// which of those the card is already in, which is finding F6.
///
/// Re-entrant: an `edit porta` drops this board's previous six first, so the
/// re-ask starts from the same pool the first pass did.
fn ask_orders(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    board_b: bool,
    rules: &Rules,
    colors: &[PlayerColor],
    placed: &mut Vec<(CardRef, CardLocation)>,
) -> Result<Vec<OrderCardId>, String> {
    const N: usize = 6;
    let board = crate::describe::port_board_name(board_b);
    placed.retain(|(_, at)| !matches!(at, CardLocation::PortSlot { board_b: b, .. } if *b == board_b));
    out(&format!("\n--- {board} ---\n"));
    out("The six order cards on the display, left to right as you face the board.\n");

    let mut cards = Vec::with_capacity(N);
    for slot in 0..N {
        let ctx = CardCtx { rules, colors, placed };
        let card = ask_card(
            input,
            out,
            &ctx,
            &format!("{board} slot {} of {N}?", menu_number(slot)),
            RecordingTarget::PortSetup { board_b, slot, of: N },
            false,
            &mut |card| known_order(card, rules),
        )?;
        let CardRef::Order(id) = card else { unreachable!("an order prompt yields order cards") };
        cards.push(id);
        placed.push((card, CardLocation::PortSlot { board_b, slot }));
    }
    Ok(cards)
}

/// One Laboratory display's six technology cards, left to right.
///
/// **Name-first, and `T<id>` is never required** (R8/F4): nothing is printed on
/// a technology card to identify it but its NAME, so that is what the prompt
/// takes — and the two copies of a title are one answer, resolved to whichever
/// is still unrecorded (`card_prompt::free_copy`). The old prompt refused a
/// name matching both copies and told the operator to use an id printed on
/// neither card, which is the F4 defect in its purest form.
fn ask_techs(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    board_b: bool,
    rules: &Rules,
    colors: &[PlayerColor],
    all_tech_ids: &[TechCardId],
    placed: &mut Vec<(CardRef, CardLocation)>,
) -> Result<Vec<TechCardId>, String> {
    const N: usize = 6;
    let board = crate::describe::lab_board_name(board_b);
    placed.retain(|(_, at)| !matches!(at, CardLocation::LabSlot { board_b: b, .. } if *b == board_b));
    out(&format!("\n--- {board} ---\n"));
    out("The six technology cards on the display, left to right. Type each card's NAME\n");
    out("(any unambiguous start of it will do); '?' draws all twenty with their effects.\n");

    let mut cards = Vec::with_capacity(N);
    for slot in 0..N {
        let ctx = CardCtx { rules, colors, placed };
        let card = ask_card(
            input,
            out,
            &ctx,
            &format!("{board} slot {} of {N}?", menu_number(slot)),
            RecordingTarget::LabSetup { board_b, slot, of: N },
            false,
            &mut |card| match card {
                CardRef::Tech(id) if all_tech_ids.contains(&id) => Ok(()),
                CardRef::Tech(id) => Err(format!("T{:02} is not one of the technology cards", id.0)),
                other => Err(format!("{} is not a technology", card_prompt::card_tag(other))),
            },
        )?;
        let CardRef::Tech(id) = card else { unreachable!("a tech prompt yields tech cards") };
        cards.push(id);
        placed.push((card, CardLocation::LabSlot { board_b, slot }));
    }
    Ok(cards)
}

fn parse_color(s: &str) -> Result<PlayerColor, String> {
    match s.to_lowercase().as_str() {
        "red" => Ok(PlayerColor::Red),
        "blue" => Ok(PlayerColor::Blue),
        "yellow" => Ok(PlayerColor::Yellow),
        "green" => Ok(PlayerColor::Green),
        _ => Err(format!("'{s}' is not red/blue/yellow/green")),
    }
}

/// The fourteen tile types and their short codes, in the order the tile menu
/// lists them: the four production areas, then the commercial ones, then the
/// Canal.
///
/// **The codes are ALIASES now, not the interface** (R1). They are what the
/// prompt used to print and what every existing script types, so they keep
/// working — but the menu shows full names, and the entry takes those too.
const AREA_TYPE_ENTRY: [(AreaType, &str); 14] = [
    (AreaType::CopperMine, "cm"),
    (AreaType::SilkMill, "sm"),
    (AreaType::TeaPlantation, "tp"),
    (AreaType::FishingGround, "fg"),
    (AreaType::Bank, "bank"),
    (AreaType::PortA, "pa"),
    (AreaType::PortB, "pb"),
    (AreaType::LaboratoryA, "la"),
    (AreaType::LaboratoryB, "lb"),
    (AreaType::EmploymentAgency, "ea"),
    (AreaType::Chinatown, "ct"),
    (AreaType::Church, "ch"),
    (AreaType::Customs, "cu"),
    (AreaType::Canal, "canal"),
];

#[cfg(test)]
mod tests {
    use super::*;
    // `wizard`'s own top-level already glob-imports yokohama_core (needed by
    // the impl code), and `super::*` carries that into this module too --
    // this second import is the brief's Step-1 test text verbatim, kept
    // as-is and just silenced rather than diverging from the spec (same
    // precedent as `session::tests`/`repl::tests`).
    #[allow(unused_imports)]
    use yokohama_core::*;

    /// The three achievement answers every scripted live test gives, **one per
    /// category, in the A/B/C order the questions are asked** (F1).
    ///
    /// This exact trio is the one the owner's own live-gate attempt had on the
    /// table (brief §1.1 F1: Copper5 = A, Money10 = B, Production4 = C) — the
    /// setup that was legal all along while the wizard would equally have taken
    /// three category-A cards. The scripts used to feed `Copper5 Silk6 Tea7`,
    /// which is three of category A: an illegal table the old prompt accepted
    /// without a word, and which is now untypeable.
    const P2_ACHIEVEMENTS: [&str; 3] = ["Copper5", "Money10", "Production4"];

    fn feed(lines: &[&str]) -> (Result<crate::repl::Repl, String>, String) {
        let mut transcript = String::new();
        let mut it = lines.iter().map(|s| s.to_string());
        let result = {
            let mut sink = |s: &str| transcript.push_str(s);
            run_wizard(&mut it, &mut sink)
        };
        (result, transcript)
    }

    #[test]
    fn sim_wizard_builds_a_playable_session() {
        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "42", "", // mode players colors start seed savepath(default)
            "1", "2",                                  // keep choices for seats 0 and 1
            "",                                        // bot seats: none (T12)
            "confirm",                                 // review/edit step (design §5.5 item 3)
        ]);
        let repl = repl.unwrap();
        assert!(transcript.contains("keep"), "{transcript}");
        assert_eq!(repl.session.state().config.player_count, 2);
        assert_eq!(repl.session.record().seed, 42);
        assert!(repl.session.save_path.is_some());
        assert!(!repl.session.legal().is_empty());
        std::fs::remove_file("yokohama-game-42.json").ok(); // in case autosave fired
    }

    /// Regression for the sim tail silently discarding the wizard's collected
    /// colors/start-seat answers (final-review Important finding): a
    /// NON-default color order and a NON-default start seat must both reach
    /// the built session, and `new_game` seeding `current_player` from
    /// `config.start_player` (game/setup.rs) means the opening turn must
    /// honor it too.
    #[test]
    fn sim_wizard_honors_start_seat_and_color_choices() {
        let (repl, _transcript) = feed(&[
            "sim", "2", "blue", "red", "1", "43", "", // mode players colors(seat0=blue,seat1=red) start=1 seed savepath(default)
            "1", "2",                                  // keep choices for seats 0 and 1
            "",                                        // bot seats: none (T12)
            "confirm",                                 // review/edit step (design §5.5 item 3)
        ]);
        let repl = repl.unwrap();
        let state = repl.session.state();
        assert_eq!(
            state.config.player_colors,
            vec![PlayerColor::Blue, PlayerColor::Red],
            "wizard-chosen seat colors must reach the built session"
        );
        assert_eq!(
            state.config.start_player,
            PlayerId::new(1),
            "wizard-chosen start seat must reach the built session"
        );
        assert_eq!(
            state.current_player,
            PlayerId::new(1),
            "new_game seeds current_player from config.start_player"
        );
        std::fs::remove_file("yokohama-game-43.json").ok(); // in case autosave fired
    }

    #[test]
    fn live_wizard_accepts_full_observations() {
        // "Physical table" = a different seed's setup, exactly as in the
        // core live tests.
        let rules = Rules::standard();
        let table = new_game(GameConfig::for_player_count(2), &rules, 555).unwrap();
        let n = table.state.board_layout.areas.len();
        let abbrev = |t: AreaType| match t {
            AreaType::CopperMine => "cm", AreaType::SilkMill => "sm",
            AreaType::TeaPlantation => "tp", AreaType::FishingGround => "fg",
            AreaType::Bank => "bank", AreaType::PortA => "pa", AreaType::PortB => "pb",
            AreaType::LaboratoryA => "la", AreaType::LaboratoryB => "lb",
            AreaType::EmploymentAgency => "ea", AreaType::Chinatown => "ct",
            AreaType::Church => "ch", AreaType::Customs => "cu", AreaType::Canal => "canal",
        };
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "77".into(), "".into(),
            // seat 0 known deal + keep, seat 1 hidden
            "known".into(),
            format!("O{}", table.deals[0][0].0), format!("O{}", table.deals[0][1].0), "1".into(),
            "hidden".into(),
        ];
        for i in 0..n {
            lines.push(abbrev(table.state.board_layout.areas[&AreaId::new(i as u8)].area_type).into());
        }
        let mut site = 0u8;
        for i in 0..n {
            let t = table.state.board_layout.areas[&AreaId::new(i as u8)].area_type;
            if t != AreaType::Canal { lines.push(format!("{site}")); site += 1; }
        }
        lines.extend(token_letters_for_count(site as usize)); // one token per non-Canal cell
        for a in table.state.achievements.iter() {
            lines.push(format!("{:?}", a.achievement_type)); // menu accepts the type name
        }
        for i in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_a.slots[i].unwrap().0));
        }
        for i in 0..6 {
            lines.push(format!("T{}", table.state.lab_state.lab_a.slots[i].unwrap().0));
        }
        lines.push("confirm".into()); // review/edit step (design §5.5 item 3)
        lines.push("yes".into()); // board check (third review I1)
        let mut transcript = String::new();
        let mut it = lines.into_iter();
        let repl = {
            let mut sink = |s: &str| transcript.push_str(s);
            run_wizard(&mut it, &mut sink).unwrap()
        };
        assert!(matches!(repl.session.mode, crate::session::TrackingMode::Live));
        // Observed zones took effect.
        for i in 0..6 {
            assert_eq!(repl.session.state().port_state.port_a.slots[i], table.state.port_state.port_a.slots[i]);
        }
        assert_eq!(repl.session.state().players[0].starting_order, Some(table.deals[0][0]));
        assert!(repl.session.state().players[1].starting_order_hidden);
        std::fs::remove_file("yokohama-game-77.json").ok();
    }

    // ---- Task 14: parse-time validation, unlimited re-ask, abort, review/edit ----
    // (design docs/plans/2026-07-18-choice-encoding-design.md §5.5)

    /// Shared prefix for the scripted validation tests below: a 2-player
    /// live game through mode/players/colors/start/seed/savepath, both
    /// seats' deal (hidden, to keep the prefix short -- these tests target
    /// Port A / tiles, not the deal questions), the full 10-cell board
    /// (2p's legal tile multiset is exactly 1 each of these 10 types --
    /// `board_gen::tiles_for_player_count`: PortB/LabB/Chinatown/Canal
    /// removed entirely, 1 of each production type removed), 10 site
    /// indices (0-9, safely inside the 24-card registry), and 3 distinct
    /// achievements. Callers continue from here into Port A / Laboratory A
    /// with their own scripted (possibly deliberately-invalid) answers, and
    /// must append "confirm" once done.
    fn live_2p_prefix(seed: u64) -> Vec<String> {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), seed.to_string(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(["cm", "sm", "tp", "fg", "bank", "pa", "la", "ea", "ch", "cu"].iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10)); // 5-Power tokens: 2P is 10 cells, no Canal
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines
    }

    fn feed_owned(lines: Vec<String>) -> (Result<crate::repl::Repl, String>, String) {
        let mut transcript = String::new();
        let mut it = lines.into_iter();
        let result = {
            let mut sink = |s: &str| transcript.push_str(s);
            run_wizard(&mut it, &mut sink)
        };
        (result, transcript)
    }

    /// (a) A duplicate order id in the live Port-A tail re-asks AT THAT
    /// QUESTION -- no abort, no deferral to the ~30-question reconcile_setup
    /// tail -- and since T7 the refusal NAMES WHERE the card already is, which
    /// is finding F6: the owner's own Port entry refused `2`, `4` and `6` as
    /// "already entered" while they sat in the seats' recorded deals, and
    /// nothing on screen could say so. Here the earlier place is Port A slot 1,
    /// and the message says exactly that.
    #[test]
    fn duplicate_order_id_in_port_a_reasks_with_the_location_it_is_already_in() {
        let mut lines = live_2p_prefix(601);
        lines.extend(["O1", "O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string())); // O1 repeated
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(
            transcript.contains("O01 is Port A slot 1 — a card cannot be in two places."),
            "the refusal must name where the card already is (F6)\n{transcript}"
        );
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert_eq!(repl.session.state().port_state.port_a.slots[0], Some(OrderCardId(1)));
        assert_eq!(repl.session.state().port_state.port_a.slots[1], Some(OrderCardId(2)));
        std::fs::remove_file("yokohama-game-601.json").ok();
    }

    /// (b) An order id outside the 1-36 registry re-asks, naming the registry
    /// it is not in.
    #[test]
    fn order_id_outside_registry_reasks_naming_the_registry() {
        let mut lines = live_2p_prefix(602);
        lines.extend(["O99", "O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string())); // O99 not in registry
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(transcript.contains("O99 is not one of the 36 order cards"), "{transcript}");
        assert!(repl.is_ok(), "{transcript}");
        std::fs::remove_file("yokohama-game-602.json").ok();
    }

    /// (c) A tile answer that over-fills the player-count multiset re-asks --
    /// 2p's legal multiset has exactly 1 Copper Mine, so a second "cm" is
    /// rejected, by the FULL NAME and with the set size that explains it (R1;
    /// the short code is still what this script types, as an alias).
    #[test]
    fn tile_over_fill_reasks_with_no_tiles_remaining() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "603".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(["cm", "cm", "sm", "tp", "fg", "bank", "pa", "la", "ea", "ch", "cu"].iter().map(|s| s.to_string())); // cell 1's first attempt repeats cell 0's "cm"
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10)); // 5-Power tokens
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(
            transcript.contains("no more Copper Mine tiles for this board -- the 2-player set has 1"),
            "{transcript}"
        );
        assert!(repl.is_ok(), "{transcript}");
        std::fs::remove_file("yokohama-game-603.json").ok();
    }

    /// (d) Three consecutive invalid answers do NOT abort -- the 4th valid
    /// answer is accepted and the wizard proceeds to the NEXT question
    /// (proven by reaching a *different* error -- "input ended mid-wizard"
    /// from the color question's input running out -- rather than the old
    /// "gave up after 3 invalid answers" 3-strikes message).
    #[test]
    fn three_invalid_answers_do_not_abort_fourth_valid_answer_is_accepted() {
        let (repl, transcript) = feed(&["live", "x", "y", "z", "2"]); // mode, 3 bad player-counts, 1 good one; input ends before colors
        assert_eq!(transcript.matches("is not 2, 3, or 4").count(), 3, "{transcript}");
        match repl {
            Err(e) => assert_eq!(e, "input ended mid-wizard", "the 4th valid answer must be accepted, reaching the NEXT question rather than giving up"),
            Ok(_) => panic!("expected input to run out after the color question was reached"),
        }
    }

    /// (e) `abort` at any question exits the wizard cleanly.
    #[test]
    fn abort_exits_the_wizard_cleanly_at_any_question() {
        let (repl, _transcript) = feed(&["live", "2", "red", "abort"]); // abort mid seat-1-color question
        match repl {
            Err(e) => assert_eq!(e, "aborted"),
            Ok(_) => panic!("expected Err(\"aborted\")"),
        }
    }

    /// (f) After the last question, a summary block prints (containing the
    /// entered port cards and tiles); `edit porta` re-runs the port
    /// questions and the summary reflects the change; `confirm` proceeds
    /// to a live session.
    #[test]
    fn review_summary_edit_porta_updates_it_then_confirm_builds_session() {
        let mut lines = live_2p_prefix(604);
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("edit porta".into());
        lines.extend(["O10", "O11", "O12", "O13", "O14", "O15"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(transcript.contains("O01"), "summary must show the originally-entered Port A cards\n{transcript}");
        // The summary names tiles in FULL WORDS (R1) -- and it is the summary
        // line that is asserted, not the substring `cm`, which the tile
        // preamble's alias hint would satisfy without the summary existing.
        assert!(
            transcript.contains("cell 0 (row 1 of 4, tile 1 of 4): Copper Mine"),
            "summary must show the entered tiles\n{transcript}"
        );
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        for i in 0..6 {
            assert_eq!(
                repl.session.state().port_state.port_a.slots[i],
                Some(OrderCardId(10 + i as u8)),
                "the edited Port A answers must reach the built session"
            );
        }
        std::fs::remove_file("yokohama-game-604.json").ok();
    }

    /// (g) sim tail: same re-ask behavior on an invalid keep answer (no
    /// 3-strikes give-up).
    #[test]
    fn sim_tail_reasks_indefinitely_on_invalid_keep_answer() {
        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "605", "",
            "bogus", "still-bad", "nope", "1", // seat 0: 3 invalid + 1 valid keep answer
            "2",                                // seat 1 keep
            "",                                 // bot seats: none (T12)
            "confirm",
        ]);
        assert_eq!(transcript.matches("is not 1 or 2").count(), 3, "{transcript}");
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(!repl.session.legal().is_empty());
        std::fs::remove_file("yokohama-game-605.json").ok();
    }

    /// **Spec §8's Keep, surfaced where the Keep is actually made** (Phase 3.3
    /// Task 10). Each dealt card is offered with the bot's own
    /// `score_starting_order` valuation beside it, and with the note that says
    /// what that number is not — the same honesty rule the `suggest` screen
    /// closes with, at the one other place this driver prints an evaluator's
    /// opinion.
    ///
    /// Anti-vacuity: the two scores must actually DIFFER on this deal, so a
    /// stub returning a constant would fail here rather than pass by printing
    /// two identical numbers.
    #[test]
    fn the_sim_deal_question_carries_the_bots_keep_valuation() {
        let (repl, transcript) =
            feed(&["sim", "2", "red", "blue", "0", "605", "", "1", "2", "", "confirm"]);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-605.json").ok();

        assert_eq!(
            transcript.matches("bot Keep score").count(),
            4,
            "one valuation per dealt card, both seats: {transcript}"
        );
        assert!(transcript.contains("hand-set, unmeasured"), "the note must ride with it: {transcript}");

        let scores: Vec<&str> = transcript
            .split("bot Keep score ")
            .skip(1)
            .map(|tail| tail.split(')').next().unwrap_or(""))
            .collect();
        assert_eq!(scores.len(), 4, "{transcript}");
        assert!(
            scores[0] != scores[1] || scores[2] != scores[3],
            "anti-vacuity: a constant valuation would tell an operator nothing ({scores:?})"
        );
    }

    // ---- Third review, finding I1: unambiguous tile-entry order ----
    //
    // The live wizard used to ask `Cell 3/10 tile?` and say nothing about
    // which physical tile that was. `reconcile_board` checks only that the
    // observed tiles are a PERMUTATION of the player count's multiset, so any
    // entry order at all is accepted in silence -- and this branch's board is
    // a staggered pyramid ordered WIDEST ROW FIRST, the vertical mirror of the
    // rulebook's own p.3 diagram, so an operator entering tiles in the order
    // that picture reads supplies the apex first and plays the whole game on a
    // scrambled graph. These tests pin the three things that now prevent it:
    // the numbered skeleton + orientation rules before entry, per-cell
    // coordinates on every prompt, and the board check after `confirm`.

    /// 2P legal multiset in cell order, matching `live_2p_prefix`.
    const P2_TILES: [&str; 10] = ["cm", "sm", "tp", "fg", "bank", "pa", "la", "ea", "ch", "cu"];

    /// `live_2p_prefix` + Port A + Laboratory A -- everything up to (not
    /// including) the review loop, so a caller only appends its own
    /// `confirm`/`edit`/board-check answers.
    fn live_2p_through_labs(seed: u64) -> Vec<String> {
        let mut lines = live_2p_prefix(seed);
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines
    }

    // ---- §1.1: face-up 5-Power tokens are observed at live setup ----
    //
    // Rulebook p.3 step 5 places the 5-Power tokens FACE-UP, so a live game
    // observes the arrangement at setup, exactly like the building sites. The
    // wizard now has a token-entry pass after the sites, `reconcile_board`
    // applies the observed faces, and the board renders them.

    /// A valid token-letter sequence covering `count` non-Canal cells: the
    /// letters for the first `count` faces of the canonical 20-token pool,
    /// which is always a sub-multiset (a board has at most 17 non-Canal cells).
    /// The live scripts observe tokens independently of the seeded board, so
    /// any valid sub-multiset drives the wizard -- this one is deterministic.
    fn token_letters_for_count(count: usize) -> Vec<String> {
        canonical_five_power_tokens()
            .iter()
            .take(count)
            .map(|e| match e {
                FivePowerEffect::ImportedGoods(_) => "i",
                FivePowerEffect::Copper(_) => "c",
                FivePowerEffect::Silk(_) => "s",
                FivePowerEffect::Tea(_) => "t",
                FivePowerEffect::Seafood(_) => "f",
                FivePowerEffect::Money(_) => "y",
            })
            .map(str::to_string)
            .collect()
    }

    /// The token-entry pass runs after the sites, names each cell and the tile
    /// there, applies the observed faces to the built session (each on its own
    /// cell), and the setup summary echoes each cell's token. 2P is 10 cells,
    /// no Canal, so a token sits on every cell.
    #[test]
    fn live_wizard_collects_face_up_tokens_that_reach_the_session() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "620".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string())); // sites
        // A distinct, valid sub-multiset: 1c 1s 1t 1f 1y + 5 imports = 10.
        lines.extend(["c", "s", "t", "f", "y", "i", "i", "i", "i", "i"].iter().map(|s| s.to_string()));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        // The token pass ran, with a header and per-cell prompts naming the tile.
        assert!(transcript.contains("--- 5-Power tokens ---"), "{transcript}");
        assert!(
            transcript.contains("Cell 0/10 (row 1 of 4, tile 1 of 4) Copper Mine -- 5-Power token?"),
            "token prompts must name the cell AND the tile entered there\n{transcript}"
        );
        // Each observed face reached the built session, on its own cell.
        let expect = [
            FivePowerEffect::Copper(2), FivePowerEffect::Silk(2), FivePowerEffect::Tea(3),
            FivePowerEffect::Seafood(3), FivePowerEffect::Money(3),
            FivePowerEffect::ImportedGoods(1), FivePowerEffect::ImportedGoods(1),
            FivePowerEffect::ImportedGoods(1), FivePowerEffect::ImportedGoods(1),
            FivePowerEffect::ImportedGoods(1),
        ];
        let toks = &repl.session.state().board_layout.five_power_tokens;
        for (i, e) in expect.iter().enumerate() {
            assert_eq!(toks[&AreaId::new(i as u8)], Some(*e), "cell {i} token must be the observed face");
        }
        // The setup summary echoes each cell's token (cell 0 is copper).
        assert!(transcript.contains("5P:c"), "the setup summary must show each cell's token\n{transcript}");
        std::fs::remove_file("yokohama-game-620.json").ok();
    }

    /// The canonical pool has exactly 2 coppers, so a third is refused AT the
    /// question with a clear message and re-asks that same cell -- the same
    /// sub-multiset tripwire `reconcile_board` applies as a backstop, caught
    /// live like the duplicate-site check. The recovered valid face then lands.
    #[test]
    fn a_third_copper_token_is_rejected_at_the_question_and_reasks() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "621".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        // cells 0,1 copper; cell 2's first answer is a THIRD copper (rejected),
        // then 'i'; cells 3-9 import. 3 c-attempts (1 rejected) + 8 i = 11 lines.
        lines.extend(["c", "c", "c", "i", "i", "i", "i", "i", "i", "i", "i"].iter().map(|s| s.to_string()));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        assert!(
            transcript.contains("no more 2 copper tokens available for this board"),
            "a third copper must be refused at the question\n{transcript}"
        );
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        let toks = &repl.session.state().board_layout.five_power_tokens;
        assert_eq!(toks[&AreaId::new(0)], Some(FivePowerEffect::Copper(2)));
        assert_eq!(toks[&AreaId::new(1)], Some(FivePowerEffect::Copper(2)));
        assert_eq!(toks[&AreaId::new(2)], Some(FivePowerEffect::ImportedGoods(1)), "cell 2 is the recovered import");
        std::fs::remove_file("yokohama-game-621.json").ok();
    }

    /// `edit tokens` re-asks the whole token pass and the edit reaches the
    /// built session.
    #[test]
    fn edit_tokens_re_asks_the_face_up_tokens_and_the_edit_reaches_the_session() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "622".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10)); // initial: 10 imports
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("edit tokens".into());
        lines.extend(["c", "s", "t", "f", "y", "i", "i", "i", "i", "i"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert_eq!(
            repl.session.state().board_layout.five_power_tokens[&AreaId::new(0)],
            Some(FivePowerEffect::Copper(2)),
            "the edited token must reach the built session\n{transcript}"
        );
        std::fs::remove_file("yokohama-game-622.json").ok();
    }

    /// Required outcome 1+2: before the first tile question the wizard prints
    /// the numbered skeleton of the ACTUAL shape for this player count, and
    /// states the orientation in words -- including the trap, that the
    /// rulebook's diagram prints the pyramid the other way up. The skeleton is
    /// asserted as the whole verbatim picture (`render::board_skeleton`, which
    /// derives from `row_lengths`), so it can never drift from the shape the
    /// engine actually plays.
    #[test]
    fn tile_entry_opens_with_the_numbered_skeleton_and_states_the_orientation() {
        let mut lines = live_2p_through_labs(610);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(
            transcript.contains(&crate::render::board_skeleton(2)),
            "the numbered skeleton of the real 2P shape must precede the tile questions\n{transcript}"
        );
        // Matched case-insensitively: the wording SHOUTS the load-bearing
        // words at the operator, but which ones are capitalized is styling,
        // and a test that pins styling just punishes rewording.
        let said = transcript.to_lowercase();
        for needle in [
            "widest row",            // which row is entered first
            "cell 0",                // which physical tile cell 0 is
            "same direction",        // the one degree of freedom that IS free
            "other way up",          // the rulebook p.3 mirror trap
            "adjacency graph",       // why the picture is not the authority
        ] {
            assert!(said.contains(needle), "orientation wording missing {needle:?}\n{transcript}");
        }
    }

    /// Required outcome 1, at the moment of entry: every tile prompt names the
    /// cell's row and position, so the coordinates survive the skeleton
    /// scrolling off the top of a real terminal. 2P rows are 4/3/2/1.
    #[test]
    fn every_tile_prompt_names_its_row_and_position_on_the_board() {
        let mut lines = live_2p_through_labs(611);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (_repl, transcript) = feed_owned(lines);
        for needle in [
            "row 1 of 4: 4 tiles, cells 0-3",       // row banner
            "row 4 of 4: 1 tile, cells 9-9",
            "Cell 0/10 (row 1 of 4, tile 1 of 4)",  // first cell of the widest row
            "Cell 3/10 (row 1 of 4, tile 4 of 4)",  // last cell of the widest row
            "Cell 4/10 (row 2 of 4, tile 1 of 3)",  // first cell of the next row
            "Cell 9/10 (row 4 of 4, tile 1 of 1)",  // the apex, entered LAST
        ] {
            assert!(transcript.contains(needle), "tile prompts missing {needle:?}\n{transcript}");
        }
    }

    /// Required outcome 5: the site pass walks the same cells in the same
    /// order and had the same ambiguity, so it carries the same coordinates --
    /// plus the tile already entered there, which is what actually lets an
    /// operator find the physical card they are reading from.
    #[test]
    fn every_site_prompt_names_its_cell_and_the_tile_entered_there() {
        let mut lines = live_2p_through_labs(612);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (_repl, transcript) = feed_owned(lines);
        assert!(
            transcript.contains("Cell 0/10 (row 1 of 4, tile 1 of 4) Copper Mine -- building-site"),
            "site prompts must name the cell AND the tile entered there\n{transcript}"
        );
        assert!(
            transcript.contains("Cell 9/10 (row 4 of 4, tile 1 of 1) Customs -- building-site"),
            "{transcript}"
        );
    }

    /// Required outcome 3: `confirm` no longer starts the game. The wizard
    /// builds the real session, draws the board with the REAL renderer
    /// (asserted byte-for-byte against `render::board_grid` of the session it
    /// returned -- "exactly as the game will show it"), and only `yes` starts
    /// play. A junk answer re-asks rather than being read as consent.
    #[test]
    fn board_check_draws_the_assembled_board_and_only_yes_starts_the_game() {
        let mut lines = live_2p_through_labs(613);
        lines.push("confirm".into());
        lines.push("maybe".into()); // must NOT be taken for a yes
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(transcript.contains("--- Board check ---"), "{transcript}");
        assert!(
            transcript.contains(&crate::render::board_grid(repl.session.state(), repl.session.rules(), &[])),
            "the check must draw the board with the same renderer play uses\n{transcript}"
        );
        assert!(transcript.contains("is not 'yes' or 'no'"), "a junk answer must re-ask\n{transcript}");
        std::fs::remove_file("yokohama-game-613.json").ok();
    }

    /// Required outcome 3, the recovery half: answering `no` returns to the
    /// review loop, where `edit tiles` re-enters the board -- and the corrected
    /// board reaches the built session, with the check running again over it.
    /// This is the path an operator takes after spotting a mirrored board.
    #[test]
    fn board_check_no_returns_to_review_where_edit_tiles_re_enters_the_board() {
        let mut lines = live_2p_through_labs(614);
        lines.push("confirm".into());
        lines.push("no".into()); // the drawn board does not match the table
        lines.push("edit tiles".into());
        let mut corrected = P2_TILES;
        corrected.reverse();
        lines.extend(corrected.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string())); // sites come with the tiles
        lines.extend(token_letters_for_count(10)); // tokens come with the tiles too
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert_eq!(
            repl.session.state().board_layout.areas[&AreaId::new(0)].area_type,
            AreaType::Customs,
            "the re-entered board must reach the built session\n{transcript}"
        );
        assert_eq!(
            transcript.matches("--- Board check ---").count(),
            2,
            "the check must run again over the corrected board\n{transcript}"
        );
        std::fs::remove_file("yokohama-game-614.json").ok();
    }

    /// Building-site cards sit ON the tiles, so a re-entered board moves them
    /// too: `edit tiles` re-asks the sites in the same pass. Without this the
    /// recovery path silently re-attaches the OLD site cards to the NEW tile
    /// order -- exactly the class of silent mis-mapping this whole finding is
    /// about, reintroduced by its own fix.
    #[test]
    fn edit_tiles_re_asks_the_building_sites_because_they_sit_on_the_tiles() {
        let mut lines = live_2p_through_labs(615);
        lines.push("edit tiles".into());
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10)); // edit tiles re-asks the tokens too
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert_eq!(
            transcript.matches("building-site card?").count(),
            20,
            "10 site questions in the initial pass + 10 more after `edit tiles`\n{transcript}"
        );
        std::fs::remove_file("yokohama-game-615.json").ok();
    }

    // ---- Third review, wave D: the 2-player dummy is unobserved setup ----
    //
    // `SetupObservations` carries no track state, so nothing in the live path
    // asks whether the two neutral assistants are physically on the table --
    // the engine simply has them. An operator who never places them plays a
    // whole game against a tool that blocks two spaces they see as open, ends
    // the game a pawn early on either track, and awards both majority bonuses
    // partly to a rival that is not on their board. Observing them is core
    // work and deferred; TELLING the operator is not, and lands on the last
    // screen before play.

    /// The instruction reaches a 2-player live operator inside the board
    /// check -- the last screen with a question still to answer, and so the
    /// last one a real terminal shows before the first composed frame clears
    /// everything above it (main.rs's I-1 note). It names both spaces and
    /// says what goes wrong without them.
    #[test]
    fn board_check_tells_a_2p_live_operator_to_place_the_dummy_assistants() {
        let mut lines = live_2p_through_labs(616);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        for needle in [
            "Church faith-7",  // where the first one goes
            "Customs space-4", // where the second one goes
        ] {
            assert!(transcript.contains(needle), "dummy instruction missing {needle:?}\n{transcript}");
        }
        // The rest matched case-insensitively -- the wording shouts some of
        // these, and pinning capitalization only punishes rewording.
        let said = transcript.to_lowercase();
        for needle in ["not printed board art", "block", "end the game", "majority bonus"] {
            assert!(said.contains(needle), "dummy instruction missing the {needle:?} reason\n{transcript}");
        }
        // Inside the board-check screen, ahead of the question it is asked to
        // be read with -- not stranded after the last answer, where the first
        // frame would wipe it unread.
        let header = transcript.find("--- Board check ---").expect("board check ran");
        let question = transcript.find("Does this match your physical board?").expect("board check asked");
        let notice = transcript.find("Church faith-7").expect("checked above");
        assert!(header < notice && notice < question, "the instruction must sit inside the board check\n{transcript}");
        // §1.2 observing half: the board-match question the operator MUST answer
        // NAMES the dummy pawns at 2 players, so a "yes" explicitly confirms
        // they are physically placed (not just that the tiles match).
        assert!(
            transcript[question..].to_lowercase().contains("two neutral dummy pawns"),
            "the 2P board-match question must name the dummy pawns so 'yes' confirms them\n{transcript}"
        );
        std::fs::remove_file("yokohama-game-616.json").ok();
    }

    /// 3 players have no dummy, so the instruction must not appear -- an
    /// operator told to place a pawn that does not exist would corrupt a
    /// setup the tool then has no way to detect.
    #[test]
    fn no_dummy_instruction_in_the_3p_live_wizard() {
        let rules = Rules::standard();
        let table = new_game(GameConfig::for_player_count(3), &rules, 617).unwrap();
        let n = table.state.board_layout.areas.len();
        let tile_of = |i: usize| table.state.board_layout.areas[&AreaId::new(i as u8)].area_type;
        let mut lines: Vec<String> = vec![
            "live".into(), "3".into(), "red".into(), "blue".into(), "yellow".into(),
            "0".into(), "617".into(), "".into(),
            "hidden".into(), "hidden".into(), "hidden".into(),
        ];
        lines.extend((0..n).map(|i| tile_abbrev_for_test(tile_of(i)).to_string()));
        let mut site = 0u8;
        for i in 0..n {
            if tile_of(i) != AreaType::Canal {
                lines.push(format!("{site}"));
                site += 1;
            }
        }
        lines.extend(token_letters_for_count(site as usize)); // one 5-Power token per non-Canal cell
        lines.extend(table.state.achievements.iter().map(|a| format!("{:?}", a.achievement_type)));
        for slot in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_a.slots[slot].unwrap().0));
        }
        for slot in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_b.as_ref().unwrap().slots[slot].unwrap().0));
        }
        for slot in 0..6 {
            lines.push(format!("T{}", table.state.lab_state.lab_a.slots[slot].unwrap().0));
        }
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(transcript.contains("--- Board check ---"), "the board check still runs at 3P\n{transcript}");
        let said = transcript.to_lowercase();
        assert!(!said.contains("dummy"), "3 players have no dummy -- no instruction\n{transcript}");
        std::fs::remove_file("yokohama-game-617.json").ok();
    }

    /// Sim mode has no physical board to put anything on, so even at 2
    /// players -- where the engine's dummy is just as real -- there is
    /// nothing for the operator to do and nothing to say.
    #[test]
    fn no_dummy_instruction_in_the_2p_sim_wizard() {
        let (repl, transcript) = feed(&["sim", "2", "red", "blue", "0", "618", "", "1", "2", "", "confirm"]);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        let said = transcript.to_lowercase();
        assert!(!said.contains("dummy"), "sim has no table to place pawns on\n{transcript}");
        std::fs::remove_file("yokohama-game-618.json").ok();
    }

    /// 4 players is the remaining player count, and scripting its whole
    /// 18-cell wizard to assert an ABSENCE would pin the script, not the
    /// rule. The rule is that the notice is read off the built state: a board
    /// the engine put no dummy on produces nothing to print, at any player
    /// count, however the state was reached.
    #[test]
    fn the_dummy_notice_is_empty_for_every_board_without_a_dummy() {
        let rules = Rules::standard();
        for players in [2u8, 3, 4] {
            let state = new_game(GameConfig::for_player_count(players), &rules, 619).unwrap().state;
            let notice = dummy_placement_notice(&state);
            if players == 2 {
                assert!(notice.contains("Church faith-7"), "2P names the church space: {notice}");
                assert!(notice.contains("Customs space-4"), "2P names the customs space: {notice}");
                assert!(notice.contains("4th assistant"), "the ordinal comes from track_trigger: {notice}");
            } else {
                assert_eq!(notice, "", "{players} players have no dummy, so nothing to say");
            }
        }
    }

    /// Tile abbreviation for the scripted live tests -- the inverse of
    /// `parse_tile_abbrev`, shared by the tests that drive a generated table
    /// back through the wizard.
    fn tile_abbrev_for_test(t: AreaType) -> &'static str {
        match t {
            AreaType::CopperMine => "cm", AreaType::SilkMill => "sm",
            AreaType::TeaPlantation => "tp", AreaType::FishingGround => "fg",
            AreaType::Bank => "bank", AreaType::PortA => "pa", AreaType::PortB => "pb",
            AreaType::LaboratoryA => "la", AreaType::LaboratoryB => "lb",
            AreaType::EmploymentAgency => "ea", AreaType::Chinatown => "ct",
            AreaType::Church => "ch", AreaType::Customs => "cu", AreaType::Canal => "canal",
        }
    }

    /// Required outcome 4: the whole live wizard -- board check included --
    /// still runs off ONE pre-built script with no interactive-only step. Run
    /// at 3P (a shape with a Canal, so the site pass skips a cell) against a
    /// real generated table, and asserted by EXHAUSTING the iterator: every
    /// question consumed exactly one scripted line, and none was left over.
    /// The Forge PTY recipe and every scripted test depend on this holding.
    #[test]
    fn the_whole_live_wizard_still_scripts_in_a_single_write_at_3p() {
        let rules = Rules::standard();
        let table = new_game(GameConfig::for_player_count(3), &rules, 909).unwrap();
        let n = table.state.board_layout.areas.len();
        let tile_of = |i: usize| table.state.board_layout.areas[&AreaId::new(i as u8)].area_type;
        let abbrev = |t: AreaType| match t {
            AreaType::CopperMine => "cm", AreaType::SilkMill => "sm",
            AreaType::TeaPlantation => "tp", AreaType::FishingGround => "fg",
            AreaType::Bank => "bank", AreaType::PortA => "pa", AreaType::PortB => "pb",
            AreaType::LaboratoryA => "la", AreaType::LaboratoryB => "lb",
            AreaType::EmploymentAgency => "ea", AreaType::Chinatown => "ct",
            AreaType::Church => "ch", AreaType::Customs => "cu", AreaType::Canal => "canal",
        };
        let mut lines: Vec<String> = vec![
            "live".into(), "3".into(), "red".into(), "blue".into(), "yellow".into(),
            "0".into(), "909".into(), "".into(),
            "hidden".into(), "hidden".into(), "hidden".into(),
        ];
        lines.extend((0..n).map(|i| abbrev(tile_of(i)).to_string()));
        let mut site = 0u8;
        for i in 0..n {
            if tile_of(i) != AreaType::Canal {
                lines.push(format!("{site}"));
                site += 1;
            }
        }
        lines.extend(token_letters_for_count(site as usize)); // one 5-Power token per non-Canal cell
        lines.extend(table.state.achievements.iter().map(|a| format!("{:?}", a.achievement_type)));
        for slot in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_a.slots[slot].unwrap().0));
        }
        for slot in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_b.as_ref().unwrap().slots[slot].unwrap().0));
        }
        for slot in 0..6 {
            lines.push(format!("T{}", table.state.lab_state.lab_a.slots[slot].unwrap().0));
        }
        lines.push("confirm".into());
        lines.push("yes".into());

        let mut transcript = String::new();
        let mut it = lines.into_iter();
        let repl = {
            let mut sink = |s: &str| transcript.push_str(s);
            run_wizard(&mut it, &mut sink)
                .unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"))
        };
        assert_eq!(it.next(), None, "the script must be consumed exactly, with nothing left over");
        for i in 0..n {
            assert_eq!(
                repl.session.state().board_layout.areas[&AreaId::new(i as u8)].area_type,
                tile_of(i),
                "cell {i} must hold the tile the script put there"
            );
        }
        std::fs::remove_file("yokohama-game-909.json").ok();
    }

    /// The orientation wording tells the operator that reading EVERY row from
    /// the other end is harmless ("a consistently mirrored board is the same
    /// board to the engine") while switching direction partway is not. That is
    /// a claim about the pyramid's graph, made to someone who cannot check it,
    /// so it is pinned here: mirroring every row maps the board onto itself --
    /// tile types, adjacency, and canal crossings all transported by the
    /// mirror map, on a REAL 4P board (the only shape with a Canal and 6-wide
    /// rows). If this ever stops holding, the guide sentence must go.
    #[test]
    fn a_consistently_mirrored_entry_is_the_same_board_to_the_engine() {
        let rules = Rules::standard();
        let table = new_game(GameConfig::for_player_count(4), &rules, 4242).unwrap();
        let rows = row_lengths(4);
        let n = table.state.board_layout.areas.len();
        // The mirror map: reverse each row in place.
        let mirror = |id: AreaId| {
            let (r, c) = area_id_to_cell(id, &rows);
            cell_to_area_id(r, rows[r as usize] - 1 - c, &rows)
        };

        let tiles: Vec<AreaType> =
            (0..n).map(|i| table.state.board_layout.areas[&AreaId::new(i as u8)].area_type).collect();
        let mut sites: Vec<Option<u8>> = Vec::with_capacity(n);
        let mut next = 0u8;
        for t in &tiles {
            if *t == AreaType::Canal {
                sites.push(None);
            } else {
                sites.push(Some(next));
                next += 1;
            }
        }
        // What the operator types when they read every row from the other end.
        let mut m_tiles = vec![AreaType::Canal; n];
        let mut m_sites = vec![None; n];
        for i in 0..n {
            let j = mirror(AreaId::new(i as u8)).0 as usize;
            m_tiles[j] = tiles[i];
            m_sites[j] = sites[i];
        }

        let build = |tiles: Vec<AreaType>, sites: Vec<Option<u8>>| {
            Session::new_live(
                GameConfig::for_player_count(4),
                4242,
                SetupObservations {
                    board: Some(ObservedBoard { tiles, sites, tokens: None }),
                    ..Default::default()
                },
                vec![StartingOrderChoice::Keep(0); 4],
                Rules::standard(),
            )
            .expect("a permutation of the seeded multiset is always reconcilable")
        };
        let plain = build(tiles, sites);
        let mirrored = build(m_tiles, m_sites);

        for i in 0..n {
            let id = AreaId::new(i as u8);
            let m = mirror(id);
            assert_eq!(
                mirrored.state().board_layout.areas[&m].area_type,
                plain.state().board_layout.areas[&id].area_type,
                "tile at cell {i} must land on its mirror image"
            );
            let mut want: Vec<u8> =
                plain.state().board_layout.adjacent_areas(id).iter().map(|a| mirror(*a).0).collect();
            let mut got: Vec<u8> =
                mirrored.state().board_layout.adjacent_areas(m).iter().map(|a| a.0).collect();
            want.sort_unstable();
            got.sort_unstable();
            assert_eq!(got, want, "neighbors of cell {i} must transport under the mirror");
        }
        let norm = |mut v: Vec<(u8, u8)>| {
            for p in v.iter_mut() {
                if p.0 > p.1 {
                    *p = (p.1, p.0);
                }
            }
            v.sort_unstable();
            v
        };
        assert_eq!(
            norm(mirrored.state().board_layout.canal_crossings.iter().map(|(a, b)| (a.0, b.0)).collect()),
            norm(
                plain
                    .state()
                    .board_layout
                    .canal_crossings
                    .iter()
                    .map(|(a, b)| (mirror(*a).0, mirror(*b).0))
                    .collect()
            ),
            "the Canal's tolled transit pairs must transport under the mirror too"
        );
    }

    // ---- Task 14 fix wave (review of commit 0a01c95) ----

    /// Finding 1: sim `edit seed` used to recompute `ws.pending` under the
    /// new seed but leave `ws.choices` untouched. Since `StartingOrderChoice
    /// ::Keep(k)` is just an index (0 or 1) with no reference to which
    /// actual card it names, an old keep answer stays "syntactically valid"
    /// against a completely different deal -- finalize_setup never errors,
    /// it just silently keeps whatever card sits at that index in the NEW
    /// deal, which the operator never saw or confirmed. The fix re-asks
    /// every seat's keep choice (re-showing that seat's ACTUAL new-seed
    /// preview) whenever the seed changes. The `Seat 0 dealt:` count is the
    /// signal that actually distinguishes old from new behavior here: the
    /// buggy code still returns `Ok` (Keep(0)/Keep(1) are unconditionally
    /// valid regardless of deal identity) and even happens to keep the
    /// "right" card at that index by construction, so asserting on the
    /// final hand ALONE would not catch this -- the bug is that the
    /// operator's answer was never re-collected against what they'd
    /// actually see, which only the re-printed preview proves.
    #[test]
    fn sim_edit_seed_reasks_every_seat_keep_choice_and_final_hands_match_new_seed() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending_a = new_game(config.clone(), &rules, 700).unwrap();
        let pending_b = new_game(config.clone(), &rules, 701).unwrap();
        assert_ne!(pending_a.deals, pending_b.deals, "test precondition: the two seeds must deal different cards");

        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "700", "",
            "1", "2",       // seat 0/1 keep choices, answered while looking at seed 700's deal
            "",             // bot seats: none (T12)
            "edit seed",
            "701",          // change the seed -- the seed-700 preview the operator just answered against is now stale
            "1", "2",       // seat 0/1 keep choices must be RE-ASKED against seed 701's (different) deal
            "confirm",
        ]);
        assert_eq!(
            transcript.matches("Seat 0 dealt:").count(),
            2,
            "a seed edit must re-preview + re-ask every seat's keep choice, not silently keep the stale answers\n{transcript}"
        );
        assert_eq!(transcript.matches("Seat 1 dealt:").count(), 2, "{transcript}");
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        let state = repl.session.state();
        assert_eq!(
            state.players[0].starting_order,
            Some(pending_b.deals[0][0]),
            "seat 0 kept index 0 of seed 701's deal (the deal actually previewed the second time), not seed 700's"
        );
        assert_eq!(
            state.players[1].starting_order,
            Some(pending_b.deals[1][1]),
            "seat 1 kept index 1 of seed 701's deal"
        );
        assert_eq!(repl.session.record().seed, 701);
        std::fs::remove_file("yokohama-game-700.json").ok(); // save path was set before the seed edit; not retroactively renamed
        std::fs::remove_file("yokohama-game-701.json").ok(); // in case autosave fired under a different path
    }

    // ---- Phase 3.2 Task 12: the sim tail's bot-seats question ----

    /// The question is asked, its answer reaches the built session (through
    /// provenance, the one path a resumed save also takes), and `edit bots`
    /// re-collects it like every other section -- including back to none.
    #[test]
    fn sim_tail_asks_bots_and_edit_bots_round_trips() {
        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "801", "",
            "1", "2",     // keep choices
            "1",          // bots: seat 1
            "edit bots",
            "",           // ...and back to none
            "confirm",
        ]);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(transcript.contains("Bot seats?"), "the question must be asked: {transcript}");
        assert!(
            transcript.contains("bots: seat 1 playing family-weighted"),
            "the first review must name the bot seat and its policy: {transcript}"
        );
        assert!(transcript.contains("bot seed:"), "a drawn seed must be echoed: {transcript}");
        assert!(
            transcript.contains("bots: none"),
            "after `edit bots` with a blank answer the review must say none: {transcript}"
        );
        let seats = &repl.session.record().run.as_ref().expect("provenance is always stamped").seats;
        assert_eq!(seats.len(), 2);
        assert!(
            seats.iter().all(|s| s.controller == yokohama_core::CONTROLLER_HUMAN),
            "the edited answer, not the first one, must reach the session: {seats:?}"
        );
        std::fs::remove_file("yokohama-game-801.json").ok();
    }

    /// A bot answer that survives to `confirm` reaches the session as
    /// provenance -- name, knobs and stream seed, none of it hand-typed.
    #[test]
    fn sim_wizard_bot_answer_reaches_the_record_as_provenance() {
        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "802", "",
            "1", "2",     // keep choices (menu numbers)
            "1",          // bots: SEAT 1 -- a seat id, unaffected by the menu base
            "confirm",
        ]);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        let seats = &repl.session.record().run.as_ref().expect("provenance").seats;
        assert_eq!(seats[0].controller, yokohama_core::CONTROLLER_HUMAN);
        assert_eq!(seats[1].controller, RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.as_str());
        assert_eq!(seats[1].params["recovery_min_of"], 5, "{seats:?}");
        assert!(seats[1].agent_seed.is_some(), "a bot seat records the stream it played on");
        assert!(seats.iter().all(|s| s.observation == yokohama_core::OBSERVATION_VIEW));
        std::fs::remove_file("yokohama-game-802.json").ok();
    }

    /// Bot-vs-bot is out of scope for v1 (spec §9), so the question refuses an
    /// all-bot table and re-asks rather than building a game with nobody to
    /// prompt. An out-of-range seat is refused the same way.
    #[test]
    fn bots_question_refuses_an_all_bot_table_and_a_bad_seat() {
        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "803", "",
            "1", "2",     // keep choices (menu numbers)
            "0 1",        // every seat a bot -- refused (SEAT ids, not menu numbers)
            "5",          // not a seat -- refused
            "x",          // not a number -- refused
            "0",          // fine: seat 0 bot, seat 1 human
            "confirm",
        ]);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(transcript.contains("at least one seat must stay human"), "{transcript}");
        assert!(transcript.contains("seat must be 0-1"), "{transcript}");
        assert!(transcript.contains("is not a seat number"), "{transcript}");
        let seats = &repl.session.record().run.as_ref().expect("provenance").seats;
        assert_eq!(seats[0].controller, RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.as_str());
        assert_eq!(seats[1].controller, yokohama_core::CONTROLLER_HUMAN);
        std::fs::remove_file("yokohama-game-803.json").ok();
    }

    // ---- Phase 3.5a T7: the wizard redesign (findings F1, F2, F5, F6) ----
    //
    // These pin the four things the owner hit at the live-gate attempt, at the
    // exact prompts they were hit at. The brief's own words are in each test.

    /// **F1, the structural half.** The rulebook puts ONE card from each of the
    /// three categories in play (KB §Achievement Cards; `game::setup` deals from
    /// `ACHIEVEMENTS_A/B/C`), and the old wizard took any three distinct types —
    /// so `Copper5 Silk6 Tea7`, three category-A cards, built a live game nothing
    /// would ever question. The question is now asked per category, in A/B/C
    /// order, so the illegal table cannot be typed: a category-B card offered to
    /// the category-A question is refused NAMING BOTH categories, and the
    /// question comes round again.
    #[test]
    fn achievements_are_asked_one_category_at_a_time_and_a_wrong_category_is_refused() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "630".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        // Category A asked first: Money10 (a B card) and Tea7 (an A card) —
        // the first refused, the second taken.
        lines.extend(["Money10", "Tea7", "Tokens5", "Commercial4"].iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(
            transcript.contains(
                "Money10 is a category B card -- this question is asking for the category A one"
            ),
            "the refusal must name the card's category AND the question's\n{transcript}"
        );
        for (order, letter) in ["A", "B", "C"].iter().enumerate() {
            let question = format!("Which category {letter} card is in play?");
            let at = transcript.find(&question).unwrap_or_else(|| panic!("category {letter} asked\n{transcript}"));
            if order > 0 {
                let previous = format!("Which category {} card is in play?", ["A", "B", "C"][order - 1]);
                assert!(
                    transcript.find(&previous).expect("asked") < at,
                    "the categories are asked in A/B/C order\n{transcript}"
                );
            }
        }
        // One card per category reached the built game, in that order — which is
        // also the order `new_game` deals them, so a live setup and a seeded one
        // describe their achievements the same way.
        let in_play: Vec<AchievementCategory> = repl
            .session
            .state()
            .achievements
            .iter()
            .map(|c| c.achievement_type.category())
            .collect();
        assert_eq!(
            in_play,
            vec![AchievementCategory::A, AchievementCategory::B, AchievementCategory::C],
            "one card per category, A/B/C"
        );
        assert_eq!(repl.session.state().achievements[0].achievement_type, AchievementType::Tea7);
        std::fs::remove_file("yokohama-game-630.json").ok();
    }

    /// The two-digit rows line up with the one-digit ones, and a count of one
    /// reads as one. Both are cosmetic, and both were found by looking at the
    /// real screen rather than at a string: `[10]` is a column wider than `[1]`,
    /// so padding only the description sheared the VP column on exactly the
    /// four rows category C occupies, and the running line said "1 achievement
    /// cards".
    #[test]
    fn the_category_menu_and_the_recorded_line_read_as_written() {
        let index = crate::screen_model::build_index(
            IndexKind::Achievements,
            &Rules::standard(),
            &[],
            &achievement_registry(),
            Some(RecordingTarget::AchievementSetup { category: AchievementCategory::C }),
            None,
        );
        let menu = category_menu(&index, AchievementCategory::C);
        let vp_columns: Vec<usize> = menu
            .lines()
            .filter(|l| l.contains(" VP "))
            .map(|l| l.find(" VP ").expect("checked"))
            .collect();
        assert_eq!(vp_columns.len(), 4, "category C has four cards: {menu}");
        assert!(
            vp_columns.windows(2).all(|w| w[0] == w[1]),
            "every row's VP column lands in the same place: {menu}"
        );

        let rules = Rules::standard();
        let one = [(CardRef::Order(OrderCardId(4)), CardLocation::Deal { seat: PlayerId::new(0) })];
        let ctx = CardCtx { rules: &rules, colors: &[], placed: &one };
        assert!(
            already_recorded_line(IndexKind::Orders, &ctx).contains("1 order card:"),
            "{}",
            already_recorded_line(IndexKind::Orders, &ctx)
        );
        let two = [
            one[0],
            (CardRef::Order(OrderCardId(5)), CardLocation::PortSlot { board_b: false, slot: 0 }),
        ];
        let ctx = CardCtx { rules: &rules, colors: &[], placed: &two };
        assert!(
            already_recorded_line(IndexKind::Orders, &ctx).contains("2 order cards:"),
            "{}",
            already_recorded_line(IndexKind::Orders, &ctx)
        );
        // Nothing recorded in a registry says nothing at all.
        assert_eq!(already_recorded_line(IndexKind::Techs, &ctx), "");
    }

    /// **F3 at the prompt that named it.** The owner's report is exact: the
    /// achievements menu printed `1) Copper5 …` and typing `1` answered
    /// *"'1' is not a valid achievement type"*. The menu's numbers are now the
    /// card INDEX's own numbers, so the number on screen and the number the
    /// question accepts are the same number by construction — and the same one
    /// the `?` screen shows.
    #[test]
    fn the_achievements_menu_accepts_the_numbers_it_prints() {
        // The numbers the menu will print, read off the model the menu is built
        // from — not transcribed here, which would pin the test to today's
        // ordering rather than to the agreement between screen and parser.
        let index = crate::screen_model::build_index(
            IndexKind::Achievements,
            &Rules::standard(),
            &[],
            &achievement_registry(),
            Some(RecordingTarget::AchievementSetup { category: AchievementCategory::A }),
            None,
        );
        let pick = |category: AchievementCategory| -> (usize, AchievementType) {
            let card = index
                .cards()
                .find(|c| match &c.face {
                    crate::screen_model::CardFaceModel::Achievement(a) => a.category == category,
                    _ => false,
                })
                .expect("a card in this category");
            let crate::screen_model::CardFaceModel::Achievement(a) = &card.face else {
                unreachable!()
            };
            (card.selection.expect("a recording screen numbers its cards"), a.achievement_type)
        };
        let (a_number, a_type) = pick(AchievementCategory::A);
        let (b_number, b_type) = pick(AchievementCategory::B);
        let (c_number, c_type) = pick(AchievementCategory::C);

        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "631".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        lines.extend([a_number, b_number, c_number].iter().map(|n| n.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(
            transcript.contains(&format!("[{a_number}] ")),
            "the menu must print the number it accepts\n{transcript}"
        );
        assert!(
            !transcript.contains("is not a valid achievement type"),
            "the F3 refusal must be gone\n{transcript}"
        );
        let got: Vec<AchievementType> =
            repl.session.state().achievements.iter().map(|c| c.achievement_type).collect();
        assert_eq!(got, vec![a_type, b_type, c_type], "each number picked the card beside it");
        std::fs::remove_file("yokohama-game-631.json").ok();
    }

    /// **F2, by name.** The tile prompt used to end
    /// `(cm/sm/tp/fg/bank/pa/pb/la/lb/ea/ct/ch/cu/canal)` at every player count,
    /// so a 2-player setup advertised `canal` — a tile the 2-player game does not
    /// have — and the validator then refused what the prompt had just offered.
    /// The owner's summary: *"it's the CLI just not having a clean and clear UX
    /// as opposed to the engine itself."* Now the menu offers only what is
    /// placeable, and the Canal's refusal says where the tile actually is.
    #[test]
    fn the_tile_menu_never_offers_the_canal_at_two_players() {
        let mut lines = live_2p_prefix(632);
        // Replace the first tile answer with an attempt at the Canal, then the
        // real one. (The prefix's own ten answers follow it.)
        lines.insert(9, "canal".into());
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(
            transcript.contains("Canal is not part of a 2-player board -- that tile stays in the box"),
            "the refusal must say where the tile is\n{transcript}"
        );
        let menu_lines: Vec<&str> =
            transcript.lines().filter(|l| l.contains("Tiles still to place")).collect();
        assert!(!menu_lines.is_empty(), "the menu must be printed\n{transcript}");
        // Nothing in any tile menu row names the Canal. The menu rows are the
        // ones carrying `x<count>`, which is the shape only an offer has.
        for line in transcript.lines().filter(|l| l.contains("] ") && l.contains(" x")) {
            assert!(!line.contains("Canal"), "the Canal must never be offered at 2P: {line}");
        }
    }

    /// **F2, the live counts.** The offer shrinks as the board fills, which is
    /// what makes it an answer to "what is left" rather than a static list: the
    /// first cell is offered all ten of the 2-player types, the last is offered
    /// one. Asserted as a monotone run over the menus in the transcript, so it
    /// cannot pass by the menu being printed once.
    #[test]
    fn the_tile_offer_shrinks_as_the_board_fills() {
        let mut lines = live_2p_through_labs(633);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (_repl, transcript) = feed_owned(lines);

        // Menu blocks are `Tiles still to place…` followed by rows of offers;
        // count the offers in each block by its `[n] Name xk` cells.
        let counts: Vec<usize> = transcript
            .split("Tiles still to place")
            .skip(1)
            .map(|block| {
                block
                    .lines()
                    .take_while(|l| !l.starts_with("Cell "))
                    .map(|l| l.matches(" x").count())
                    .sum::<usize>()
            })
            .collect();
        assert_eq!(counts.len(), 10, "one menu per cell: {counts:?}");
        assert_eq!(counts[0], 10, "the first cell is offered every 2P type: {counts:?}");
        assert_eq!(counts[9], 1, "the last cell is offered the one type left: {counts:?}");
        for pair in counts.windows(2) {
            assert!(pair[1] <= pair[0], "the offer never grows: {counts:?}");
        }
    }

    /// **F2's other half: the answer grammar.** A menu number, the full name and
    /// the old short code all name the same tile, and a prefix that names two
    /// tiles is refused rather than guessed (`c` starts five of them). One
    /// board, entered three ways, with the engine's own board as the check.
    #[test]
    fn a_tile_is_named_by_number_by_name_or_by_its_short_code() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "634".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        // Cell 0: an ambiguous prefix (refused), then the full name. Cell 1: the
        // menu number of Silk Mill, which is its position in the FIXED roster
        // (S1) and so is still 2 with the single Copper Mine used up — under the
        // old shrinking offer the same tile was `1` here and something else
        // again at the next cell. Cells 2-9: short codes.
        lines.extend(["c", "Copper Mine", "2"].iter().map(|s| s.to_string()));
        lines.extend(P2_TILES[2..].iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        // The candidates named are the ones ON THIS BOARD: Chinatown and the
        // Canal also start with `c` and are in the box at 2 players, so listing
        // them would offer the operator tiles they cannot be holding.
        assert!(
            transcript.contains("'c' could be Copper Mine, Church, Customs -- type more of the name"),
            "an ambiguous prefix is refused naming what it could mean HERE\n{transcript}"
        );
        let areas = &repl.session.state().board_layout.areas;
        assert_eq!(areas[&AreaId::new(0)].area_type, AreaType::CopperMine, "entered by full name");
        assert_eq!(areas[&AreaId::new(1)].area_type, AreaType::SilkMill, "entered by menu number");
        assert_eq!(areas[&AreaId::new(2)].area_type, AreaType::TeaPlantation, "entered by short code");
        std::fs::remove_file("yokohama-game-634.json").ok();
    }

    /// **F5, at every card prompt in the tail.** *"Not seeing the ID cards or
    /// having a lookup table in some way makes it hard to enter — there should
    /// be some kind of lookup mode."* `?` now draws the right registry's index
    /// at the right question, and the question comes round again with everything
    /// already entered still entered.
    #[test]
    fn a_question_mark_opens_the_right_index_at_every_card_prompt_and_returns() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "635".into(), "".into(),
            // Seat 0's deal: `?` at the first dealt card opens the ORDER index.
            "known".into(), "?".into(), "O30".into(), "O31".into(), "1".into(),
            "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        // Sites: `?` opens the BUILDING-SITE index.
        lines.push("?".into());
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        // Achievements: `?` opens the ACHIEVEMENTS index.
        lines.push("?".into());
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        // Labs: `?` opens the TECHNOLOGY index.
        lines.push("?".into());
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        for title in
            ["ORDER CARD INDEX", "BUILDING-SITE CARD INDEX", "ACHIEVEMENTS", "TECHNOLOGY INDEX"]
        {
            assert!(transcript.contains(title), "'?' must open {title}\n{transcript}");
        }
        // The index is not a refusal: nothing was wrong with the line.
        assert!(
            !transcript.contains("! ORDER CARD INDEX"),
            "a screen is not printed as an error\n{transcript}"
        );
        // ...and the prompt that comes back says what a bare number means NOW,
        // which is the one thing opening a screen changes about the grammar.
        assert!(
            transcript.contains("dealt card 1 of 2? (a number from the screen, 'O14', or '?' to redraw it)"),
            "the re-asked prompt must state the reading the index put it in\n{transcript}"
        );
        // And every answer given before a `?` survived it — the whole board and
        // both dealt cards are in the built session.
        assert_eq!(repl.session.state().players[0].starting_order, Some(OrderCardId(30)));
        assert_eq!(
            repl.session.state().board_layout.areas[&AreaId::new(0)].area_type,
            AreaType::CopperMine,
        );
        std::fs::remove_file("yokohama-game-635.json").ok();
    }

    /// **F5's payoff: a number typed at an OPEN index selects from the screen.**
    /// The card the index numbers `1` is not `O01` — the screen is grouped by
    /// flag — so this is also the pin that the two readings of a bare number
    /// never collide: an id before the index is open, a selection after.
    #[test]
    fn a_number_after_the_index_is_open_selects_the_card_on_screen() {
        let rules = Rules::standard();
        let first_shown = crate::screen_model::build_index(
            IndexKind::Orders,
            &rules,
            &[],
            &achievement_registry(),
            Some(RecordingTarget::PortSetup { board_b: false, slot: 0, of: 6 }),
            None,
        )
        .select(crate::screen_model::NUMBER_BASE)
        .expect("a first card");
        let CardRef::Order(shown) = first_shown else { panic!("an order screen") };

        let mut lines = live_2p_prefix(636);
        // Port A slot 1: open the index, then type 1 — which selects the FIRST
        // CARD SHOWN, not O01. The other five slots are filled with ids chosen
        // to avoid it, since a repeat would be refused as a duplicate and the
        // script would run out mid-tail.
        let fills: Vec<String> =
            (1u8..=36).filter(|n| *n != shown.0).take(5).map(|n| format!("O{n}")).collect();
        lines.push("?".into());
        lines.push("1".into());
        lines.extend(fills);
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert_eq!(
            repl.session.state().port_state.port_a.slots[0],
            Some(shown),
            "the number picked the card the screen numbered\n{transcript}"
        );
        std::fs::remove_file("yokohama-game-636.json").ok();
    }

    /// **F6, both halves.** A duplicate refusal names WHERE the card already is
    /// (pinned at Port A above), and the entry screens carry a running
    /// already-recorded line so a card that is going to be refused can be seen
    /// before it is typed. Here the earlier place is a SEAT'S DEAL — which is
    /// the exact case that baffled the owner, since a dealt card is not on the
    /// board anywhere.
    #[test]
    fn a_card_recorded_in_a_seats_deal_is_refused_by_name_at_the_port_question() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "637".into(), "".into(),
            "known".into(), "O2".into(), "O4".into(), "1".into(),
            "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        // Port A slot 1: O02 is already Seat 0's dealt card.
        lines.extend(["O2", "O1", "O3", "O5", "O6", "O7", "O8"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(
            transcript.contains("O02 is Seat 0's deal — a card cannot be in two places."),
            "the refusal must name the seat's deal (F6)\n{transcript}"
        );
        assert!(
            transcript.contains("already recorded -- 2 order cards: O02 · O04"),
            "the running already-recorded line must ride the card questions\n{transcript}"
        );
        assert_eq!(repl.session.state().port_state.port_a.slots[0], Some(OrderCardId(1)));
        std::fs::remove_file("yokohama-game-637.json").ok();
    }

    /// **A technology is entered by its NAME, and a title's two copies are one
    /// answer** (R8/F4, and the ruling that retired the wizard's old refusal).
    /// Ten of the twenty titles circulate as two identical cards; the old prompt
    /// refused the name as ambiguous and told the operator to type a `T<id>`
    /// printed on neither card. Typing the same name twice now fills two slots
    /// with the two copies, and a third time is refused BY LOCATION.
    #[test]
    fn a_tech_name_fills_both_copies_and_the_third_is_refused_by_location() {
        let rules = Rules::standard();
        // A title with two physical copies, read off the registry rather than
        // hard-coded so a data change cannot make this test vacuous — and
        // chosen by LOWEST ID rather than by iteration order, because
        // `TechRegistry` is a `HashMap` and `iter()` is unordered. (Every
        // shipped path that cares sorts: `card_prompt::tech_matches`,
        // `build_tech_index`, `index_card_for`, `input::parse_tech_entry`.)
        let mut titles: Vec<(u8, String)> =
            rules.techs.iter().map(|c| (c.id.0, c.name.clone())).collect();
        titles.sort();
        let (_, title) = titles
            .iter()
            .find(|(_, name)| rules.techs.iter().filter(|o| o.name == *name).count() == 2)
            .expect("ten titles circulate as two copies")
            .clone();
        let mut copies: Vec<u8> =
            rules.techs.iter().filter(|c| c.name == title).map(|c| c.id.0).collect();
        copies.sort_unstable();
        assert_eq!(copies.len(), 2);

        let mut lines = live_2p_prefix(638);
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        // The same NAME three times: two copies exist, so the third is refused.
        lines.extend([title.clone(), title.clone(), title.clone()]);
        // ...and four more ids, none of them this title's own copies, which
        // would be refused as the duplicates they are.
        lines.extend(
            (1u8..=30).filter(|n| !copies.contains(n)).take(4).map(|n| format!("T{n}")),
        );
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(
            !transcript.contains("matches more than one tech card"),
            "the retired refusal must not come back\n{transcript}"
        );
        assert!(
            transcript.contains(&format!(
                "T{:02} is Laboratory A slot 1 — a card cannot be in two places.",
                copies[0]
            )),
            "the third attempt is refused by LOCATION\n{transcript}"
        );
        let slots = &repl.session.state().lab_state.lab_a.slots;
        let named: Vec<String> = slots[..2]
            .iter()
            .map(|s| tech_name(s.expect("filled"), &rules.techs))
            .collect();
        assert_eq!(named, vec![title.clone(), title], "both copies of the title landed");
        assert_ne!(slots[0], slots[1], "...and they are different physical cards");
        std::fs::remove_file("yokohama-game-638.json").ok();
    }

    /// **The card echoes back before the next question** (spec §0.6). An id
    /// typed one digit wrong is exactly the entry an operator wants to catch on
    /// sight, and at a live table the card is in their hand to compare against.
    #[test]
    fn an_entered_card_echoes_the_whole_card_back() {
        let mut lines = live_2p_prefix(639);
        lines.extend(["O14", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        // The whole card, exactly as `card_prompt::echo` draws it — not a line
        // about it.
        let echo = card_prompt::echo(
            CardRef::Order(OrderCardId(14)),
            &Rules::standard(),
            &[PlayerColor::Red, PlayerColor::Blue],
        );
        assert!(!echo.is_empty());
        assert!(transcript.contains(&echo), "the card must echo back in full\n{transcript}");
        std::fs::remove_file("yokohama-game-639.json").ok();
    }

    /// **`abort` cannot be shadowed by a card grammar.** It is checked above the
    /// dispatch, so it works at a card question exactly as it does at every
    /// other — including with the index open, where every other line is being
    /// read as a card.
    #[test]
    fn abort_still_works_at_a_card_question_with_the_index_open() {
        let mut lines = live_2p_prefix(640);
        lines.extend(["?", "abort"].iter().map(|s| s.to_string()));
        let (repl, transcript) = feed_owned(lines);
        assert!(transcript.contains("ORDER CARD INDEX"), "{transcript}");
        match repl {
            Err(e) => assert_eq!(e, "aborted"),
            Ok(_) => panic!("expected Err(\"aborted\")"),
        }
    }

    /// **The review echoes the answer the operator gave.** They were asked
    /// `keep card 1 or 2` and answered `1`; the review used to read `keep 0`,
    /// the engine's own index, which is the same off-by-one F3 is about wearing
    /// a summary's clothes. Both tails, since both print a keep line.
    #[test]
    fn the_review_echoes_the_keep_answer_as_the_number_that_was_typed() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "642".into(), "".into(),
            "known".into(), "O14".into(), "O23".into(), "2".into(),
            "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(
            transcript.contains("Seat 0 deal: known O14, O23 (keep card 2)"),
            "the live review echoes the typed number\n{transcript}"
        );
        // ...and it is still the ENGINE's second card that was kept, so the
        // display change moved a label and not a choice.
        assert_eq!(repl.session.state().players[0].starting_order, Some(OrderCardId(23)));
        std::fs::remove_file("yokohama-game-642.json").ok();

        let (repl, transcript) =
            feed(&["sim", "2", "red", "blue", "0", "643", "", "1", "2", "", "confirm"]);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert!(transcript.contains("Seat 0 keep: card 1"), "{transcript}");
        assert!(transcript.contains("Seat 1 keep: card 2"), "{transcript}");
        std::fs::remove_file("yokohama-game-643.json").ok();
    }

    /// **The piped protocol stays byte-plain through the new surfaces**
    /// (spec §0.12's honesty net). T7 is the first task to print full-screen
    /// card indexes and drawn cards from inside the WIZARD, which runs before
    /// the composed frame exists and writes straight to stdout — so a
    /// hand-written escape here would reach a redirected transcript with
    /// nothing above it to catch the difference. Color is FORCED ON in the same
    /// test, so the check cannot pass by the wizard drawing nothing at all.
    #[test]
    fn the_wizard_is_escape_free_with_color_off_and_painted_with_it_on() {
        let script = |seed: u64| {
            let mut lines = live_2p_prefix(seed);
            // A `?` at Port A slot 1, so an index screen is in the transcript.
            lines.push("?".into());
            lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
            lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
            lines.push("confirm".into());
            lines.push("yes".into());
            lines
        };
        let (repl, plain) = feed_owned(script(644));
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{plain}"));
        std::fs::remove_file("yokohama-game-644.json").ok();
        assert!(plain.contains("ORDER CARD INDEX"), "the index really was drawn\n{plain}");
        assert!(!plain.contains('\u{1b}'), "piped output must carry no escapes");

        crate::style::force(true);
        let (repl, painted) = feed_owned(script(645));
        crate::style::force(false);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{painted}"));
        std::fs::remove_file("yokohama-game-645.json").ok();
        assert!(painted.contains('\u{1b}'), "a real terminal gets color");
    }

    /// **The numbering sweep reaches the wizard** (the T5 hand-off). Every
    /// listing the wizard prints starts at `NUMBER_BASE`, nothing anywhere shows
    /// a `[0]`, and the identities the ruling excludes are untouched: cell
    /// numbers, area ids, seat numbers and the `S<n>` site index all still count
    /// from zero, because they are printed on components or recorded in saves.
    #[test]
    fn wizard_listings_start_at_the_same_base_and_identities_are_left_alone() {
        let first = format!("[{}]", crate::screen_model::NUMBER_BASE);
        let mut lines = live_2p_through_labs(641);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        // Every listing: the tile menu, the token menu, the achievements menu,
        // and the sim tail's keep menu (asserted in its own test).
        for (surface, marker) in [
            ("tile menu", "Tiles still to place"),
            ("token menu", "Tokens still to place"),
            ("achievements menu", "one of these four"),
        ] {
            let block = transcript
                .split(marker)
                .nth(1)
                .unwrap_or_else(|| panic!("{surface} must be printed\n{transcript}"));
            assert!(
                block.lines().take(4).any(|l| l.contains(&first)),
                "{surface} must start at {first}\n{block}"
            );
        }
        assert!(!transcript.contains("[0]"), "a zero-based list survived\n{transcript}");

        // ...and the identities, which are NOT menu numbers.
        assert!(transcript.contains("Cell 0/10 "), "cell numbers are area ids\n{transcript}");
        assert!(transcript.contains("A00"), "area ids start at A00\n{transcript}");
        assert!(transcript.contains("Seat 0"), "seats count from 0\n{transcript}");
        assert!(
            transcript.contains("cell 0 (row 1 of 4, tile 1 of 4): Copper Mine S0"),
            "the site index `S0` is the card's own identity\n{transcript}"
        );
        std::fs::remove_file("yokohama-game-641.json").ok();
    }

    // -------------------------------------------------------------------
    // T7 review fix wave
    // -------------------------------------------------------------------

    /// **A tile type's number does not move for the whole pass** (T7 review S1).
    ///
    /// The offer was the types with copies LEFT, numbered over that shrinking
    /// list — so at 2 players, where five of the ten types have exactly one
    /// copy, answering `1` for the Copper Mine made `1` mean Silk Mill at the
    /// very next question, then Tea Plantation, then Fishing Ground. Ten
    /// questions asked back to back, one key, five different tiles. An operator
    /// entering a board off a printed sheet is exactly the person who reads the
    /// number once and then stops reading it.
    ///
    /// Pinned as the AGREEMENT between the printed menu and the roster, over
    /// every menu the pass prints: whatever number a row carries at cell 0 it
    /// still carries at cell 9, and the rows that vanish take their numbers with
    /// them. The counts still shrink, which is F2 and is asserted here too so a
    /// fix that froze the whole menu could not pass.
    #[test]
    fn a_tile_types_menu_number_is_the_same_at_every_cell_of_the_pass() {
        let mut lines = live_2p_through_labs(646);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-646.json").ok();

        // Every tile menu in the transcript, as (number -> tile name) maps.
        let menus: Vec<HashMap<usize, String>> = transcript
            .split("Tiles still to place")
            .skip(1)
            .map(|block| {
                block
                    .lines()
                    .take_while(|l| !l.starts_with("Cell "))
                    .flat_map(|line| {
                        line.split('[').skip(1).filter_map(|cell| {
                            let (number, rest) = cell.split_once("] ")?;
                            let (name, _) = rest.trim_end().rsplit_once(" x")?;
                            Some((number.parse::<usize>().ok()?, name.trim().to_string()))
                        })
                    })
                    .collect()
            })
            .collect();
        assert_eq!(menus.len(), 10, "one menu per cell");
        assert_eq!(menus[0].len(), 10, "the first cell offers every 2P type");
        assert_eq!(menus[9].len(), 1, "the last offers the one type left");

        // The number a type carries never moves, at any menu it appears in.
        let first = &menus[0];
        for (i, menu) in menus.iter().enumerate() {
            for (number, name) in menu {
                assert_eq!(
                    first.get(number),
                    Some(name),
                    "cell {i}: [{number}] is {name:?} here and {:?} at the first menu\n{transcript}",
                    first.get(number),
                );
            }
        }
        // ...and the numbers that vanish are never handed to another tile: the
        // union of every menu's (number, name) pairs is exactly the first
        // menu's, so nothing was renumbered into a freed slot.
        let mut seen: HashMap<usize, String> = HashMap::new();
        for menu in &menus {
            seen.extend(menu.clone());
        }
        assert_eq!(&seen, first, "a freed number was reused\n{transcript}");
    }

    /// **A number whose row has dropped off the menu is refused BY NAME**, with
    /// the set size — not silently bound to whatever tile now occupies that
    /// print position (T7 review S1's other half). This is the failure mode the
    /// fixed roster exists to make impossible, so it is asserted rather than
    /// argued: `1` is the Copper Mine at every cell, and at 2 players there is
    /// one of those.
    #[test]
    fn an_exhausted_tile_number_is_refused_by_name_not_bound_to_the_next_row() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "647".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        // Cell 0 takes the Copper Mine by number; cell 1 asks for it again by
        // the SAME number, which must now be a refusal rather than a Silk Mill.
        lines.extend(["1", "1", "sm"].iter().map(|s| s.to_string()));
        lines.extend(P2_TILES[2..].iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));

        assert!(
            transcript.contains(
                "no more Copper Mine tiles for this board -- the 2-player set has 1"
            ),
            "the refusal names the tile the number still means\n{transcript}"
        );
        let areas = &repl.session.state().board_layout.areas;
        assert_eq!(areas[&AreaId::new(0)].area_type, AreaType::CopperMine);
        assert_eq!(
            areas[&AreaId::new(1)].area_type,
            AreaType::SilkMill,
            "the second `1` entered nothing at all",
        );
        std::fs::remove_file("yokohama-game-647.json").ok();
    }

    /// **The category menu comes back with the question** (T7 review S5). The
    /// tile prompt already states the rule this follows: a menu that rides in
    /// the prompt is reprinted by a re-ask, and one printed once is scrolled
    /// away by the first refusal or `?` screen — leaving the operator answering
    /// "type the number beside a card" with no cards on screen.
    #[test]
    fn the_category_menu_is_reprinted_when_the_question_is_asked_again() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "648".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        // Category A: a wrong-category card (refused), then `?` (a screen), then
        // the real answer. Three printings of the menu, one per ask.
        lines.extend(["Money10", "?", "Copper5", "Money10", "Production4"].iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-648.json").ok();

        assert_eq!(
            transcript.matches("Category A -- one of these four:").count(),
            3,
            "category A was asked three times, so its four cards were drawn three times\n{transcript}"
        );
        // ...and each printing really is above its own question, not batched.
        for block in transcript.split("Category A -- one of these four:").skip(1) {
            assert!(
                block.lines().take(6).any(|l| l.contains("Which category A card is in play?")),
                "the menu sits directly above the question it answers:\n{block:.400}"
            );
        }
        assert_eq!(
            transcript.matches("Category B -- one of these four:").count(),
            1,
            "a question answered first time is asked — and drawn — once\n{transcript}"
        );
    }

    /// **The index the wizard prints says how to get back to the QUESTION**
    /// (T7 review M3), because nothing there is an overlay: `ask_screened`
    /// writes the screen into the transcript and asks again underneath it. The
    /// overlay sentence promised four keys that close it, two of which the very
    /// next prompt refuses, and contradicted the prompt's own `'?' to redraw it`
    /// hint printed two lines later.
    #[test]
    fn a_wizard_index_states_the_inline_way_out_and_agrees_with_the_prompt_hint() {
        let mut lines = live_2p_prefix(649);
        lines.push("?".into());
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-649.json").ok();

        assert!(transcript.contains("ORDER CARD INDEX"), "the index really was drawn\n{transcript}");
        assert!(
            transcript.contains("the question continues below, so answer it ('?' redraws this index)"),
            "the inline way out\n{transcript}"
        );
        assert!(
            !transcript.contains("closes this index"),
            "no wizard screen may promise to close an overlay that is not there\n{transcript}"
        );
        // The two surfaces agree about `?`: the screen says it redraws, and so
        // does the prompt printed under it.
        assert!(entry_hint(IndexKind::Orders, true).contains("redraw"));
        assert!(
            transcript.contains("or '?' to redraw it"),
            "the prompt's own hint is in the transcript beside the screen's\n{transcript}"
        );
        // **And it leaves no dead caret behind** (the real-terminal addendum).
        // An overlay's `> ` is the line a player answers; here the question is
        // printed underneath with its own, so this one read nothing and just sat
        // there — once per index, three times in a live setup run.
        assert!(
            !transcript.lines().any(|l| l.trim() == ">"),
            "a wizard index must draw no caret of its own\n{transcript}"
        );
    }

    /// **The wizard says so when the terminal is under the design floor** (T7
    /// review SHOULD-6), once per run, before the first index — §0.1's own
    /// device, which the wizard could not use because it runs before `Repl` and
    /// its width machinery exist.
    ///
    /// The screen it introduces now FITS: `index_view` lays the grid out in the
    /// measured width, so what the notice explains is the trade (fewer cards to
    /// a row), not a defect it is apologising for.
    #[test]
    fn a_sub_floor_terminal_gets_one_size_notice_before_the_first_index() {
        crate::screen::force_width(Some(120));
        let mut lines = live_2p_prefix(653);
        // Two `?` at Port A, and a third at Laboratory A: three indexes, one
        // notice.
        lines.extend(["?", "?", "O1"].iter().map(|s| s.to_string()));
        lines.extend(["O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.push("?".into());
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        crate::screen::clear_forced_width();
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-653.json").ok();

        assert_eq!(
            transcript.matches("your terminal is 120 columns").count(),
            1,
            "the notice states the real width, ONCE for the run\n{transcript}"
        );
        assert!(
            transcript.contains("the index is drawn for 200, so it packs 4 cards to a row here"),
            "it says what the narrow window costs, in cards\n{transcript}"
        );
        // Before the index it introduces, not after it.
        let notice = transcript.find("your terminal is 120 columns").expect("the notice");
        let index = transcript.find("ORDER CARD INDEX").expect("the index");
        assert!(notice < index, "the notice must lead the screen it explains\n{transcript}");

        // ...and the screen it introduced actually fits 120 columns.
        for row in transcript
            .lines()
            .filter(|l| l.starts_with('┌') || l.starts_with('│') || l.starts_with('└'))
        {
            assert!(
                crate::screen::visible_width(row) <= 120,
                "a card row ran to {} columns in a 120-column terminal: {row:?}",
                crate::screen::visible_width(row),
            );
        }
    }

    /// **A terminal at or above the floor is told nothing, and a PIPED run is
    /// told nothing** — the second is spec §0.12 and is not negotiable for a
    /// convenience: a transcript that carried a notice would differ between a
    /// developer's window and CI, and `piped_dogfood` compares bytes.
    #[test]
    fn a_wide_terminal_and_a_piped_run_get_no_size_notice() {
        for width in [None, Some(200), Some(240)] {
            crate::screen::force_width(width);
            let mut lines = live_2p_prefix(654);
            lines.push("?".into());
            lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
            lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
            lines.push("confirm".into());
            lines.push("yes".into());
            let (repl, transcript) = feed_owned(lines);
            crate::screen::clear_forced_width();
            repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
            std::fs::remove_file("yokohama-game-654.json").ok();
            assert!(transcript.contains("ORDER CARD INDEX"), "the index was drawn");
            assert!(
                !transcript.contains("your terminal is"),
                "{width:?} needs no notice\n{transcript}"
            );
        }
    }

    /// **A token face's menu number does not move for the whole pass either**
    /// (T7 review round 2, item 3 — the tile treatment S1 got, finished on the
    /// question asked right after it).
    ///
    /// The order bug this pass already fixed (`swap_remove` reshuffling the
    /// pool) was the loud half; this is the quiet one. The offer was the faces
    /// still IN the pool, so a face running out took its number and everything
    /// below it shuffled up — and the two-copy faces run out first, in a
    /// seventeen-question pass answered off a physical board.
    ///
    /// Driven by EXHAUSTING two faces early rather than by playing the pass out:
    /// at 2 players the canonical pool never runs a face down in ten cells
    /// unless the operator spends one, so a test that simply answered ten
    /// imports would prove nothing about a dropped row.
    #[test]
    fn a_token_faces_menu_number_is_the_same_at_every_cell_of_the_pass() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "655".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        // Both coppers, then both silks — the two rows in the middle of the
        // roster — then imports for the rest. Copper `[2]` and silk `[3]` are
        // gone by cell 4, and tea/seafood/yen must not slide up into them.
        lines.extend(["c", "c", "s", "s"].iter().map(|s| s.to_string()));
        lines.extend(std::iter::repeat_n("i".to_string(), 6));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-655.json").ok();

        let menus: Vec<HashMap<usize, String>> = transcript
            .split("Tokens still to place")
            .skip(1)
            .map(|block| {
                block
                    .lines()
                    .take_while(|l| !l.starts_with("Cell "))
                    .flat_map(|line| {
                        line.split('[').skip(1).filter_map(|cell| {
                            let (number, rest) = cell.split_once("] ")?;
                            let (name, _) = rest.trim_end().rsplit_once(" x")?;
                            Some((number.parse::<usize>().ok()?, name.trim().to_string()))
                        })
                    })
                    .collect()
            })
            .collect();
        assert_eq!(menus.len(), 10, "one menu per cell");
        assert_eq!(menus[0].len(), 6, "the first cell offers all six faces");

        // The rows really did drop — otherwise the pin below is vacuous.
        assert!(menus[4].len() < menus[0].len(), "two faces must be spent by cell 4: {menus:?}");
        assert!(!menus[4].values().any(|v| v.contains("copper")), "copper is gone: {:?}", menus[4]);
        assert!(!menus[4].values().any(|v| v.contains("silk")), "silk is gone: {:?}", menus[4]);

        let first = &menus[0];
        for (i, menu) in menus.iter().enumerate() {
            for (number, name) in menu {
                assert_eq!(
                    first.get(number),
                    Some(name),
                    "cell {i}: [{number}] is {name:?} here and {:?} at the first menu\n{transcript}",
                    first.get(number),
                );
            }
        }
        // ...and a freed number is never handed to another face.
        let mut seen: HashMap<usize, String> = HashMap::new();
        for menu in &menus {
            seen.extend(menu.clone());
        }
        assert_eq!(&seen, first, "a freed number was reused\n{transcript}");
    }

    /// **The PARSER refuses an exhausted face itself**, by number and by
    /// letter, rather than leaning on `ask_tokens`' pool check to catch it.
    ///
    /// Pinned as a unit because the end-to-end test CANNOT distinguish the two:
    /// the pool check's own refusal is byte-identical, so dropping either guard
    /// leaves the transcript unchanged and a mutation of the parser passes
    /// through a whole-wizard test. That is a fine belt-and-braces arrangement
    /// and a poor pin, so the contract is stated where it lives. (The tile twin
    /// needs no equivalent: its backstop word its message differently, so the
    /// end-to-end pin already separates them.)
    #[test]
    fn the_token_parser_refuses_a_spent_face_by_number_and_by_letter() {
        let roster = token_roster(&Rules::standard().five_power_tokens);
        // Every copper spent, everything else untouched.
        let pool: Vec<FivePowerEffect> = Rules::standard()
            .five_power_tokens
            .into_iter()
            .filter(|f| !matches!(f, FivePowerEffect::Copper(_)))
            .collect();
        let offer = token_offer(&roster, &pool);
        let copper_at = offer
            .iter()
            .position(|(f, _)| matches!(f, FivePowerEffect::Copper(_)))
            .expect("copper keeps its roster row");
        assert_eq!(offer[copper_at].1, 0, "fixture: no coppers left");

        let want = "no more 2 copper tokens available for this board";
        assert_eq!(
            parse_token_entry(&menu_number(copper_at).to_string(), &offer),
            Err(want.to_string()),
            "the number that still means copper is refused as copper",
        );
        assert_eq!(parse_token_entry("c", &offer), Err(want.to_string()), "and so is its letter");
        // ...while its NEIGHBOURS are untouched and still answer their own rows.
        for (position, (face, left)) in offer.iter().enumerate() {
            if *left > 0 {
                assert_eq!(
                    parse_token_entry(&menu_number(position).to_string(), &offer),
                    Ok(*face),
                    "position {position} must still name its own face",
                );
            }
        }
    }

    /// **An exhausted token number is refused BY NAME**, with the same sentence
    /// its letter gets — not bound to the face that now occupies that print
    /// position (T7 review round 2, item 3; the twin of the tile pin above).
    #[test]
    fn an_exhausted_token_number_is_refused_by_name_not_bound_to_the_next_row() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "656".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        // Spend both coppers by number, then ask for `2` again (refused), then
        // `c` again (refused the same way), then carry on with imports.
        lines.extend(["2", "2", "2", "c", "i"].iter().map(|s| s.to_string()));
        lines.extend(std::iter::repeat_n("i".to_string(), 7));
        lines.extend(P2_ACHIEVEMENTS.iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-656.json").ok();

        assert_eq!(
            transcript.matches("no more 2 copper tokens available for this board").count(),
            2,
            "the number and the letter get the SAME refusal\n{transcript}"
        );
        // The two coppers landed on cells 0 and 1, and cell 2 took an import —
        // the refused `2` entered nothing at all.
        let toks = &repl.session.state().board_layout.five_power_tokens;
        assert!(
            matches!(toks[&AreaId::new(0)], Some(FivePowerEffect::Copper(_)))
                && matches!(toks[&AreaId::new(1)], Some(FivePowerEffect::Copper(_))),
            "the two coppers are on the first two cells: {toks:?}"
        );
        assert!(
            matches!(toks[&AreaId::new(2)], Some(FivePowerEffect::ImportedGoods(_))),
            "and the refused `2` entered nothing, so cell 2 took the import: {toks:?}"
        );
        assert_eq!(
            toks.values().filter(|t| matches!(t, Some(FivePowerEffect::Copper(_)))).count(),
            2,
            "exactly the two coppers the box holds"
        );
    }

    /// The notice's two shapes, at the boundary and past it. Below one card's
    /// width the grid cannot help, and the notice says THAT rather than
    /// promising a fit it is about to break.
    #[test]
    fn the_size_notice_states_what_the_width_can_and_cannot_do() {
        let fresh = |width: usize, kind: IndexKind| {
            SIZE_NOTICE_SHOWN.with(|f| f.set(false));
            size_notice_once(kind, width)
        };
        assert_eq!(fresh(200, IndexKind::Orders), "", "at the floor there is nothing to say");
        assert_eq!(fresh(240, IndexKind::Orders), "", "and nothing above it");

        let narrow = fresh(120, IndexKind::Orders);
        assert!(narrow.contains("packs 4 cards to a row"), "{narrow}");
        assert!(narrow.contains("Nothing is cut"), "{narrow}");

        // An achievement card is 66 columns, so 120 fits exactly one...
        let one = fresh(120, IndexKind::Achievements);
        assert!(one.contains("packs 1 card to a row"), "singular, not '1 cards': {one}");
        // ...and 40 fits none, which is a different sentence.
        let none = fresh(40, IndexKind::Achievements);
        assert!(none.contains("one card needs 66"), "{none}");
        assert!(none.contains("will wrap"), "it does not promise a fit: {none}");
        assert!(!none.contains("Nothing is cut"), "{none}");

        // Once per run, whatever registry asks second.
        SIZE_NOTICE_SHOWN.with(|f| f.set(false));
        assert!(!size_notice_once(IndexKind::Orders, 120).is_empty());
        assert_eq!(size_notice_once(IndexKind::Techs, 120), "", "the second index says nothing");
    }

    /// **The already-recorded line separates tags a person has to read apart**
    /// (T7 review S2), and stops at a COLUMN budget with a count for the rest.
    ///
    /// Both halves in one test because they are one line's contract. The join
    /// mattered as soon as T7 pointed this line at the technologies: an order
    /// tag is `O04`, so a space read fine, while a tech tag is a printed title
    /// and `Postal Service Textile Mill` is one string until you already know
    /// the registry. The budget is the reason it is a column count and not a
    /// card count — 60 columns is nine order tags or three tech titles.
    #[test]
    fn the_already_recorded_line_separates_its_tags_and_stops_at_a_column_budget() {
        let rules = Rules::standard();

        // Multi-word tags, which is what the separator is for.
        let mut titles: Vec<(u8, String)> =
            rules.techs.iter().map(|c| (c.id.0, c.name.clone())).collect();
        titles.sort();
        let two_word: Vec<TechCardId> = titles
            .iter()
            .filter(|(_, name)| name.contains(' '))
            .map(|(id, _)| TechCardId(*id))
            .take(2)
            .collect();
        assert_eq!(two_word.len(), 2, "fixture: at least two titles are two words");
        let placed: Vec<(CardRef, CardLocation)> = two_word
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (CardRef::Tech(*id), CardLocation::LabSlot { board_b: false, slot: i })
            })
            .collect();
        let ctx = CardCtx { rules: &rules, colors: &[], placed: &placed };
        let line = already_recorded_line(IndexKind::Techs, &ctx);
        assert!(
            line.contains(&format!(
                "{} · {}",
                tech_name(two_word[0], &rules.techs),
                tech_name(two_word[1], &rules.techs),
            )),
            "two multi-word titles must be readable as two: {line}"
        );

        // The budget: order tags are `Oxx` at 3 columns + 3 for the separator,
        // so 60 columns takes nine of them and the rest are counted.
        let many: Vec<(CardRef, CardLocation)> = (1u8..=20)
            .map(|n| {
                (
                    CardRef::Order(OrderCardId(n)),
                    CardLocation::PortSlot { board_b: false, slot: 0 },
                )
            })
            .collect();
        let ctx = CardCtx { rules: &rules, colors: &[], placed: &many };
        let line = already_recorded_line(IndexKind::Orders, &ctx);
        let shown = line.matches("O0").count() + line.matches("O1").count() + line.matches("O2").count();
        assert!(line.contains("20 order cards:"), "the TOTAL is stated in full: {line}");
        assert!(shown < 20 && shown > 0, "the tags are cut to the budget: {line}");
        assert!(
            line.contains(&format!("+{} more", 20 - shown)),
            "and what was cut is counted, not dropped: {line}"
        );
        // The tag run really does stay inside the budget it claims.
        let tags = line
            .split(": ")
            .nth(1)
            .and_then(|rest| rest.split("  ('?'").next())
            .expect("a tag run");
        let tags = tags.split(" +").next().expect("before the +n more");
        assert!(
            tags.chars().count() <= TAG_BUDGET,
            "{} columns of tags against a {TAG_BUDGET}-column budget: {tags:?}",
            tags.chars().count(),
        );
    }

    /// **A card question with nothing to answer it with says so** (T7 review
    /// S3). Enter, `cancel` and `back` all reach `CardEntry::Cancel`, and in the
    /// wizard there is nothing to cancel BACK to — every question is one the
    /// setup cannot be built without. The message has to name both the grammar
    /// and the one real exit, or an operator who typed Enter to "skip this one"
    /// learns nothing from the re-ask.
    #[test]
    fn an_empty_answer_at_a_card_question_names_the_grammar_and_the_way_out() {
        let mut lines = live_2p_prefix(650);
        lines.extend(["", "cancel", "back", "O1"].iter().map(|s| s.to_string()));
        lines.extend(["O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-650.json").ok();

        let want = format!(
            "  ! this question needs an answer -- {}, or 'abort' to leave setup\n",
            entry_hint(IndexKind::Orders, false),
        );
        assert_eq!(
            transcript.matches(&want).count(),
            3,
            "all three ways of saying nothing get the same answer: {want:?}\n{transcript}"
        );
    }

    /// **A prefix that names several tiles but only ONE on this board is not
    /// ambiguous at this table** (T7 review S3). `match_tile_name` matches over
    /// all fourteen types — which is what lets `canal` at 2 players reach the
    /// message naming the finding — and then disambiguates against the player
    /// count's own set, so the widened match cannot make the prompt worse.
    ///
    /// `ch` is the case: Church and Chinatown both start that way, Chinatown is
    /// in the box at 2 players, and at 3+ the same prefix really is ambiguous.
    #[test]
    fn a_prefix_matching_several_types_resolves_when_only_one_is_on_this_board() {
        let two = |types: &[(AreaType, u32)]| -> HashMap<AreaType, u32> {
            types.iter().copied().collect()
        };
        // 2 players: no Port B, so `port` can only be Port A.
        let p2 = two(&[(AreaType::PortA, 1), (AreaType::Church, 1), (AreaType::CopperMine, 1)]);
        assert_eq!(match_tile_name("port", &p2), Ok(AreaType::PortA));
        // 3+ players: both are on the board, so it is refused naming both — and
        // naming only the ones the operator could be holding.
        let p3 = two(&[
            (AreaType::PortA, 1),
            (AreaType::PortB, 1),
            (AreaType::Church, 1),
            (AreaType::CopperMine, 1),
        ]);
        let refusal = match_tile_name("port", &p3).expect_err("two tiles start with 'port' here");
        assert!(refusal.contains("Port A") && refusal.contains("Port B"), "{refusal}");
        assert!(!refusal.contains("Church"), "only the candidates, not the board: {refusal}");
        // A type in the game but not on THIS board still matches by name, which
        // is what routes it to the in-the-box refusal rather than "not a tile".
        assert_eq!(match_tile_name("chinatown", &p2), Ok(AreaType::Chinatown));
        // ...and a SHORT CODE is exact, so it never reaches either arm: `ch` is
        // the Church's code even where Chinatown is on the board too.
        let with_chinatown =
            two(&[(AreaType::Church, 1), (AreaType::Chinatown, 1), (AreaType::CopperMine, 1)]);
        assert_eq!(match_tile_name("ch", &with_chinatown), Ok(AreaType::Church));
    }

    /// **The token menu is in the canonical pool order** (T7 review S3), which
    /// is the order the box lists the faces in and the order the letters
    /// (`i/c/s/t/f/y`) were taught in.
    ///
    /// Pinned over the LIVE sequence rather than over one call, because the
    /// menu is rebuilt from the shrinking pool at every cell and the order is a
    /// property of that whole run: a pool edited in place can reorder itself
    /// under the operator between two questions, which is worse than
    /// renumbering — the rows move.
    #[test]
    fn every_token_menu_of_the_pass_is_in_the_canonical_pool_order() {
        let canonical: Vec<FivePowerEffect> = {
            let mut seen: Vec<FivePowerEffect> = Vec::new();
            for face in canonical_five_power_tokens() {
                if !seen.contains(&face) {
                    seen.push(face);
                }
            }
            seen
        };
        let labels: Vec<String> = canonical
            .iter()
            .map(|f| crate::describe::five_power_effect_label(*f).to_string())
            .collect();

        let mut lines = live_2p_through_labs(651);
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-651.json").ok();

        let menus: Vec<Vec<String>> = transcript
            .split("Tokens still to place")
            .skip(1)
            .map(|block| {
                block
                    .lines()
                    .take_while(|l| !l.starts_with("Cell "))
                    .flat_map(|line| {
                        line.split('[').skip(1).filter_map(|cell| {
                            let (_, rest) = cell.split_once("] ")?;
                            let (name, _) = rest.trim_end().rsplit_once(" x")?;
                            Some(name.trim().to_string())
                        })
                    })
                    .collect()
            })
            .collect();
        assert_eq!(menus.len(), 10, "one menu per cell");
        for (i, menu) in menus.iter().enumerate() {
            let expected: Vec<&String> =
                labels.iter().filter(|label| menu.contains(label)).collect();
            let got: Vec<&String> = menu.iter().collect();
            assert_eq!(got, expected, "cell {i}'s menu is out of canonical order\n{transcript}");
        }
    }

    /// **Both board-entry menus fit three cells on an 80-column terminal** (T7
    /// review S3). The layout was prose in `numbered_offer`'s doc — 26-column
    /// cells, three to a row, "the widest either menu produces is `[10]
    /// Employment Agency x2` at 25, so three of those plus the indent clear 80"
    /// — and nothing checked any of it against the menus that actually exist.
    ///
    /// The arithmetic is `INDENT + 2 * CELL_W + widest`, not `INDENT + 3 *
    /// CELL_W`, and the difference is exactly the finding: at three padded cells
    /// the row is 81 columns and wraps. It fits because the LAST cell on a row
    /// is trimmed to its own width, so the row costs the two padded columns plus
    /// the real width of whatever is in the third — which leaves precisely one
    /// column of headroom at the widest cell this data produces. That is a
    /// figure to MEASURE, not to restate: it is what a longer area name or a
    /// two-digit count would spend.
    ///
    /// **The sweep CONSTRUCTS its worst case; it used to roll dice for it** (T8
    /// review). The exhaustion order came from `remaining.iter().find(..)` over a
    /// `HashMap`, and Rust seeds `RandomState` per PROCESS — so which tile type
    /// was retired next varied run to run, and some orders never printed the
    /// widest cell in a third-of-a-row position at all. `widest` then came back
    /// 79 against an expected 80 and the last assertion failed, at about 7% of
    /// runs, on a test whose subject does not depend on order in the slightest.
    /// Both SAFETY assertions held in every one of those runs; it was only the
    /// claim that the worst case had actually been drawn that flaked.
    ///
    /// So the order is now the ROSTER's, which is a deterministic `Vec` and the
    /// one order that makes the worst case reachable ON PURPOSE. Retiring types
    /// in roster order hides a PREFIX of the menu, and `tile_menu` numbers from
    /// the roster position rather than the print position, so a type's cell text
    /// never changes while its print position walks down by one per step: a cell
    /// at roster index `j` is printed at every position from `j` to `0`, and
    /// therefore lands on a third-of-row position (`j % 3 == 2` counting from the
    /// row start) for some prefix whenever `j >= 2`. That is asserted below as
    /// `widest_third == widest_cell` rather than assumed — if a future roster put
    /// the widest cell in the first two positions, this fails and says to
    /// construct the state deliberately instead of widening the claim.
    #[test]
    fn numbered_offer_fits_three_cells_in_eighty_columns() {
        const CELL_W: usize = 26;
        const INDENT: usize = 3;
        const TERMINAL_W: usize = 80;

        let rules = Rules::standard();
        let mut widest = 0usize;
        let mut widest_cell = 0usize;
        // The widest cell ever drawn in a THIRD-of-a-full-row position — the only
        // place a cell pays its real width instead of its padded one.
        let mut widest_third = 0usize;
        for players in 2..=4u8 {
            // The real per-count tile multiset, derived the way `run_live_tail`
            // derives it — a throwaway seeded board — so this sweeps the menus
            // the wizard actually prints rather than a hand-written roster.
            let ghost = new_game(GameConfig::for_player_count(players), &rules, 1)
                .expect("a standard board");
            let mut legal: HashMap<AreaType, u32> = HashMap::new();
            for area in ghost.state.board_layout.areas.values() {
                *legal.entry(area.area_type).or_insert(0) += 1;
            }
            let roster = tile_roster(&legal);
            // Every state the offer passes through: full, then one type at a time
            // exhausted IN ROSTER ORDER. `remaining` stays a map because
            // `tile_offer` reads it by key — what is gone is iterating it, which
            // is where the order came from.
            let mut remaining = legal.clone();
            let mut sweep = |remaining: &HashMap<AreaType, u32>| {
                let offer = tile_offer(&roster, remaining);
                measure(
                    &tile_menu(&offer, players),
                    &mut widest,
                    &mut widest_cell,
                    &mut widest_third,
                );
            };
            sweep(&remaining);
            for area in &roster {
                remaining.insert(*area, 0);
                sweep(&remaining);
            }
        }
        // The token menu's own, over the whole pool as it empties. Already
        // deterministic — a `Vec` popped from the end.
        let roster = token_roster(&rules.five_power_tokens);
        let mut pool = rules.five_power_tokens.clone();
        while !pool.is_empty() {
            measure(
                &token_menu(&token_offer(&roster, &pool)),
                &mut widest,
                &mut widest_cell,
                &mut widest_third,
            );
            pool.pop();
        }

        assert!(
            widest_cell < CELL_W,
            "a cell of {widest_cell} columns does not fit its {CELL_W}-column column",
        );
        assert!(
            INDENT + 2 * CELL_W + widest_cell <= TERMINAL_W,
            "a full row is {} columns, past the {TERMINAL_W} the layout claims",
            INDENT + 2 * CELL_W + widest_cell,
        );
        // The worst case was CONSTRUCTED, not stumbled into: the widest cell in
        // the data is a cell this sweep actually printed third, where its full
        // width is what the row pays. Without this the assertion below is a
        // statement about whichever menus happened to be drawn.
        assert_eq!(
            widest_third, widest_cell,
            "the widest cell ({widest_cell} columns) was never drawn as the THIRD cell of a full \
             row (widest third cell: {widest_third}), so the row bound was never actually \
             exercised. Roster-order exhaustion places a cell third whenever its roster index is \
             at least 2 -- if that no longer holds, build the state deliberately rather than \
             relaxing this claim.",
        );
        assert_eq!(
            widest,
            INDENT + 2 * CELL_W + widest_cell,
            "the widest drawn row is a full one, so the bound above is the real worst case",
        );
        // Anti-vacuity: the sweep really did draw full three-cell rows.
        assert!(widest > 2 * CELL_W, "only {widest} columns were ever drawn");
    }

    /// The widest trimmed row, the widest cell, and the widest cell drawn in a
    /// THIRD-of-a-full-row position, over one menu.
    ///
    /// The third column is the one that matters to the 80-column bound: the first
    /// two cells of a row are padded to `CELL_W` whatever they contain, so only a
    /// third cell spends its real width. A line carrying three cells is always a
    /// full row -- `numbered_offer` breaks the line exactly there.
    fn measure(menu: &str, widest: &mut usize, widest_cell: &mut usize, widest_third: &mut usize) {
        for line in menu.lines().skip(1) {
            let line = line.trim_end();
            *widest = (*widest).max(line.chars().count());
            // Cells are `[n] text` padded to a fixed column, so the raw cells
            // are the row split back on the marker that starts each one.
            let cells: Vec<usize> = line
                .split('[')
                .skip(1)
                .map(|cell| format!("[{}", cell.trim_end()).chars().count())
                .collect();
            for width in &cells {
                *widest_cell = (*widest_cell).max(*width);
            }
            if let [_, _, third] = cells[..] {
                *widest_third = (*widest_third).max(third);
            }
        }
    }

    /// **The Achievement card in play is located by its CATEGORY** (T7 review
    /// S3), which is the whole of `CardLocation::AchievementSlot`'s design: the
    /// three cards sit on the table for a game with no slot, shelf or board to
    /// be in, so numbering them 1/2/3 would invent a position nothing prints.
    ///
    /// Pinned where it is READ — the duplicate refusal and the dim note on the
    /// card face both word it through `describe::card_location`, and the wizard
    /// is the only surface that produces the variant.
    #[test]
    fn an_achievement_already_in_play_is_refused_by_its_category() {
        for (category, letter) in [
            (AchievementCategory::A, "A"),
            (AchievementCategory::B, "B"),
            (AchievementCategory::C, "C"),
        ] {
            assert_eq!(
                crate::describe::card_location(CardLocation::AchievementSlot { category }),
                format!("category {letter} in play"),
            );
        }

        // ...and it reaches the operator, at the question after the one that
        // recorded it: category B asked for the card already in play for A.
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "652".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(P2_TILES.iter().map(|s| s.to_string()));
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10));
        lines.extend(["Copper5", "Copper5", "Money10", "Production4"].iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into());
        let (repl, transcript) = feed_owned(lines);
        repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        std::fs::remove_file("yokohama-game-652.json").ok();

        assert!(
            transcript.contains("Own 5 Copper is category A in play — a card cannot be in two places."),
            "the refusal names the category the card is already in\n{transcript}"
        );
    }
}
