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

use std::collections::{BTreeSet, HashMap};

use crate::input::{parse_order_id, parse_tech_entry, tech_name, TechAmbiguity, TechEntryError};
use crate::session::Session;
use yokohama_core::*;

/// Ask one question, re-asking indefinitely on invalid input. `parse`
/// reports success/failure only (`Ok(())`/`Err(reason)`) -- callers capture
/// the parsed value into a local `Option` from inside the closure, which is
/// guaranteed `Some` once `ask` returns `Ok(())` (the closure only returns
/// `Ok(())` after setting it). Every prompt and error goes through `out` as
/// it happens, so scripted tests can assert on the transcript and `main`
/// can print interactively from the exact same code path.
///
/// The only two exits are a valid answer, or the literal (case-insensitive)
/// `abort`, checked BEFORE `parse` sees the line so it can never be shadowed
/// by a question's own validation -- `Err("aborted")` is the sentinel every
/// caller up to `run_wizard` propagates via `?` (design §5.5 item 2; this
/// replaces the old 3-strikes give-up).
fn ask(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    prompt: &str,
    parse: &mut dyn FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    loop {
        out(prompt);
        out("\n");
        let line = input.next().ok_or("input ended mid-wizard")?;
        let line = line.trim();
        if line.eq_ignore_ascii_case("abort") {
            return Err("aborted".to_string());
        }
        match parse(line) {
            Ok(()) => return Ok(()),
            Err(e) => out(&format!("  ! {e}\n")),
        }
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

/// `Seat <n>: keep card 0 or 1?` -- shared by the sim tail and a live
/// "known" seat.
fn ask_keep(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str), seat: u8) -> Result<u8, String> {
    let mut keep: Option<u8> = None;
    ask(input, out, &format!("Seat {seat}: keep card 0 or 1?"), &mut |s: &str| -> Result<(), String> {
        match s.parse::<u8>() {
            Ok(k) if k <= 1 => {
                keep = Some(k);
                Ok(())
            }
            _ => Err(format!("'{s}' is not 0 or 1")),
        }
    })?;
    Ok(keep.unwrap())
}

// ---------------------------------------------------------------------
// Live tail
// ---------------------------------------------------------------------

/// Everything the live tail collects, bundled so the review/edit loop can
/// dispatch `edit <section>` to a function taking `&mut LiveWizardState`
/// instead of threading a dozen separate `&mut` locals around. `seen_*`
/// are the cross-section duplicate trackers (design §5.5 item 1): order
/// ids span the deals + Port A + Port B, tech ids span Lab A + Lab B --
/// tiles and sites are each self-contained to their own section, so they
/// don't need a persistent tracker (their `ask_*` functions start fresh
/// every time, initial pass or edit alike).
struct LiveWizardState {
    players: u8,
    colors: Vec<PlayerColor>,
    start_player: PlayerId,
    seed: u64,
    save_path: String,
    config: GameConfig,
    legal_tile_counts: HashMap<AreaType, u32>,
    all_tech_ids: Vec<TechCardId>,
    seen_order_ids: BTreeSet<u8>,
    seen_tech_ids: BTreeSet<u8>,
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

    let mut seen_order_ids: BTreeSet<u8> = BTreeSet::new();
    let mut seen_tech_ids: BTreeSet<u8> = BTreeSet::new();
    let mut known_deals: Vec<Option<[OrderCardId; 2]>> = vec![None; players as usize];
    let mut deal_choices: Vec<StartingOrderChoice> = vec![StartingOrderChoice::Unknown; players as usize];
    for seat in 0..players {
        let (deal, choice) = ask_live_deal(input, out, seat, &rules, &mut seen_order_ids)?;
        known_deals[seat as usize] = deal;
        deal_choices[seat as usize] = choice;
    }

    let tiles = ask_tiles(input, out, players, &legal_tile_counts)?;
    let sites = ask_sites(input, out, &tiles, &rules, players)?;
    let tokens = ask_tokens(input, out, &tiles, &rules, players)?;
    let achievements = ask_achievements(input, out)?;

    let port_a: [OrderCardId; 6] = ask_orders(input, out, "Port A", 6, &rules, &mut seen_order_ids)?.try_into().unwrap();
    let port_b: Option<[OrderCardId; 6]> = if config.has_port_b {
        Some(ask_orders(input, out, "Port B", 6, &rules, &mut seen_order_ids)?.try_into().unwrap())
    } else {
        None
    };
    let lab_a: [TechCardId; 6] =
        ask_techs(input, out, "Laboratory A", 6, &rules, &all_tech_ids, &mut seen_tech_ids)?.try_into().unwrap();
    let lab_b: Option<[TechCardId; 6]> = if config.has_lab_b {
        Some(ask_techs(input, out, "Laboratory B", 6, &rules, &all_tech_ids, &mut seen_tech_ids)?.try_into().unwrap())
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
        seen_order_ids,
        seen_tech_ids,
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
/// section's previous contribution from the cross-section `seen_*`
/// trackers where applicable, then re-run exactly that section's ask-block
/// (design §5.5 item 3). `start`/`mode`/`players` are deliberately NOT
/// editable sections -- changing any of them would invalidate every
/// question already answered (color count, grid size, Port B/Lab B
/// presence, ...); the wizard must be aborted and re-run for those.
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
            if let Some(old) = ws.known_deals[seat as usize] {
                for c in old {
                    ws.seen_order_ids.remove(&c.0);
                }
            }
            let (deal, choice) = ask_live_deal(input, out, seat, rules, &mut ws.seen_order_ids)?;
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
            ws.sites = ask_sites(input, out, &ws.tiles, rules, ws.players)?;
            ws.tokens = ask_tokens(input, out, &ws.tiles, rules, ws.players)?;
        }
        Some("sites") => {
            ws.sites = ask_sites(input, out, &ws.tiles, rules, ws.players)?;
        }
        Some("tokens") => {
            ws.tokens = ask_tokens(input, out, &ws.tiles, rules, ws.players)?;
        }
        Some("achievements") => {
            ws.achievements = ask_achievements(input, out)?;
        }
        Some("porta") => {
            for c in ws.port_a {
                ws.seen_order_ids.remove(&c.0);
            }
            ws.port_a = ask_orders(input, out, "Port A", 6, rules, &mut ws.seen_order_ids)?.try_into().unwrap();
        }
        Some("portb") if ws.config.has_port_b => {
            if let Some(old) = ws.port_b {
                for c in old {
                    ws.seen_order_ids.remove(&c.0);
                }
            }
            ws.port_b = Some(ask_orders(input, out, "Port B", 6, rules, &mut ws.seen_order_ids)?.try_into().unwrap());
        }
        Some("laba") => {
            for c in ws.lab_a {
                ws.seen_tech_ids.remove(&c.0);
            }
            ws.lab_a =
                ask_techs(input, out, "Laboratory A", 6, rules, &ws.all_tech_ids, &mut ws.seen_tech_ids)?.try_into().unwrap();
        }
        Some("labb") if ws.config.has_lab_b => {
            if let Some(old) = ws.lab_b {
                for c in old {
                    ws.seen_tech_ids.remove(&c.0);
                }
            }
            ws.lab_b = Some(
                ask_techs(input, out, "Laboratory B", 6, rules, &ws.all_tech_ids, &mut ws.seen_tech_ids)?.try_into().unwrap(),
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
    out.push_str(&format!(
        "achievements: {}\n",
        ws.achievements.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>().join(", ")
    ));
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
                out.push_str(&format!("Seat {seat} deal: known O{:02}, O{:02} (keep {keep})\n", cards[0].0, cards[1].0));
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
            out.push_str(&format!("Seat {seat} keep: card {k}\n"));
        }
    }
    out
}

/// One seat's live-observed deal: known (2 order-card tokens, validated
/// against the registry and deduped against `seen_order_ids`, plus a keep
/// choice) or hidden (nothing else asked).
fn ask_live_deal(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    seat: u8,
    rules: &Rules,
    seen_order_ids: &mut BTreeSet<u8>,
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
    let mut c1: Option<OrderCardId> = None;
    ask(input, out, &format!("Seat {seat}: first dealt card? (O<id>)"), &mut |s: &str| -> Result<(), String> {
        c1 = Some(ask_order_answer(s, rules, seen_order_ids)?);
        Ok(())
    })?;
    let mut c2: Option<OrderCardId> = None;
    ask(input, out, &format!("Seat {seat}: second dealt card? (O<id>)"), &mut |s: &str| -> Result<(), String> {
        c2 = Some(ask_order_answer(s, rules, seen_order_ids)?);
        Ok(())
    })?;
    let keep = ask_keep(input, out, seat)?;
    Ok((Some([c1.unwrap(), c2.unwrap()]), StartingOrderChoice::Keep(keep)))
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
                "  [{i}] {}  (bot Keep score {})\n",
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

/// Validate one order-card answer: syntax (via the shared `parse_order_id`),
/// registry membership, then not-already-entered -- inserting into
/// `seen_order_ids` on success (design §5.5 item 1).
fn ask_order_answer(s: &str, rules: &Rules, seen_order_ids: &mut BTreeSet<u8>) -> Result<OrderCardId, String> {
    let n = parse_order_id(s).ok_or_else(|| format!("'{s}' is not a valid order-card code"))?;
    let id = OrderCardId(n);
    if rules.orders.get(id).is_none() {
        return Err(format!("'{s}' is not a valid order card"));
    }
    if !seen_order_ids.insert(n) {
        return Err(format!("'{s}' (O{n:02}) was already entered"));
    }
    Ok(id)
}

/// Validate one tech-card answer: the shared `parse_tech_entry` (registry
/// membership folded into its candidate check, `TechAmbiguity::Reject` --
/// the wizard records physical identity, so an ambiguous name prefix must
/// be refused, not guessed), then not-already-entered. Renders its own
/// wizard-flavored wording from `TechEntryError` (unchanged from before the
/// fix wave) -- `reveal.rs`'s `parse_tech_reveal` renders its own
/// deck-flavored wording from the SAME error type instead of sharing text.
fn ask_tech_answer(
    s: &str,
    rules: &Rules,
    all_tech_ids: &[TechCardId],
    seen_tech_ids: &mut BTreeSet<u8>,
) -> Result<TechCardId, String> {
    let id = parse_tech_entry(s, &rules.techs, all_tech_ids, TechAmbiguity::Reject).map_err(|e| match e {
        TechEntryError::NotInPool(_) => format!("'{s}' is not a valid tech card"),
        TechEntryError::NoMatch => format!("'{s}' does not match any tech card"),
        TechEntryError::Ambiguous => format!("'{s}' matches more than one tech card -- use T<id>"),
    })?;
    if !seen_tech_ids.insert(id.0) {
        return Err(format!("'{s}' ({}) was already entered", tech_name(id, &rules.techs)));
    }
    Ok(id)
}

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
        let mut chosen: Option<AreaType> = None;
        ask(
            input,
            out,
            &format!(
                "Cell {i}/{n} ({}) tile? (cm/sm/tp/fg/bank/pa/pb/la/lb/ea/ct/ch/cu/canal)",
                cell_coords(i, &rows)
            ),
            &mut |s: &str| -> Result<(), String> {
                let area = parse_tile_abbrev(s)?;
                match remaining.get_mut(&area) {
                    Some(k) if *k > 0 => {
                        *k -= 1;
                        chosen = Some(area);
                        Ok(())
                    }
                    _ => Err(format!("no {} tiles remaining for this board", s.to_lowercase())),
                }
            },
        )?;
        tiles.push(chosen.unwrap());
    }
    Ok(tiles)
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
    players: u8,
) -> Result<Vec<Option<u8>>, String> {
    let rows = row_lengths(players);
    let n = tiles.len();
    out("\n--- Building-site cards ---\n");
    out("Same cells in the same order as the tiles: the widest row first, left to right.\n");
    out("Each question names the tile you entered there. Canal cells carry no site and\n");
    out("are skipped.\n");

    let mut seen_sites: BTreeSet<u8> = BTreeSet::new();
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
        let mut chosen: Option<u8> = None;
        ask(
            input,
            out,
            &format!(
                "Cell {i}/{n} ({}) {} -- building-site index? (S<idx> or a number)",
                cell_coords(i, &rows),
                crate::describe::area_type_name(*tile)
            ),
            &mut |s: &str| -> Result<(), String> {
                let idx = parse_site_index(s)?;
                if idx as usize >= rules.building_sites.len() {
                    return Err(format!("'{s}' is not a valid building-site index"));
                }
                if !seen_sites.insert(idx) {
                    return Err(format!("'{s}' (site {idx}) was already entered"));
                }
                chosen = Some(idx);
                Ok(())
            },
        )?;
        sites.push(Some(chosen.unwrap()));
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
    out("the tile you entered there; Canal cells carry no token and are skipped. Letters:\n");
    out("  i = 1 imported good   c = 2 copper   s = 2 silk\n");
    out("  t = 3 tea   f = 3 seafood   y = 3 yen\n");

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
        let mut chosen: Option<FivePowerEffect> = None;
        ask(
            input,
            out,
            &format!(
                "Cell {i}/{n} ({}) {} -- 5-Power token? (i/c/s/t/f/y)",
                cell_coords(i, &rows),
                crate::describe::area_type_name(*tile)
            ),
            &mut |s: &str| -> Result<(), String> {
                let effect = crate::reveal::parse_token_letter(&[s])?;
                match pool.iter().position(|p| *p == effect) {
                    Some(idx) => {
                        pool.swap_remove(idx);
                        chosen = Some(effect);
                        Ok(())
                    }
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

/// 3 distinct entries out of the 12 canonical achievement types.
fn ask_achievements(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str)) -> Result<[AchievementType; 3], String> {
    out("Achievements -- enter 3 distinct types (name is case-insensitive):\n");
    for (i, ty) in ALL_ACHIEVEMENTS.iter().enumerate() {
        out(&format!("  {}) {:?} -- {}\n", i + 1, ty, crate::describe::achievement_desc(*ty)));
    }
    let mut achievement_list: Vec<AchievementType> = Vec::new();
    for slot in 0..3 {
        let mut chosen: Option<AchievementType> = None;
        ask(input, out, &format!("Achievement {} of 3?", slot + 1), &mut |s: &str| -> Result<(), String> {
            let ty = parse_achievement(s)?;
            if achievement_list.contains(&ty) {
                return Err(format!("{ty:?} was already chosen"));
            }
            chosen = Some(ty);
            Ok(())
        })?;
        achievement_list.push(chosen.unwrap());
    }
    Ok(achievement_list.try_into().unwrap())
}

/// Ask `n` sequential order-card slots left-to-right (Port A/B contents),
/// validated/deduped against `seen_order_ids` (design §5.5 item 1: shared
/// across Port A + Port B + the deals).
fn ask_orders(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    label: &str,
    n: usize,
    rules: &Rules,
    seen_order_ids: &mut BTreeSet<u8>,
) -> Result<Vec<OrderCardId>, String> {
    let mut cards = Vec::with_capacity(n);
    for slot in 0..n {
        let mut chosen: Option<OrderCardId> = None;
        ask(
            input,
            out,
            &format!("{label} slot {} of {n}? (O<id>)", slot + 1),
            &mut |s: &str| -> Result<(), String> {
                chosen = Some(ask_order_answer(s, rules, seen_order_ids)?);
                Ok(())
            },
        )?;
        cards.push(chosen.unwrap());
    }
    Ok(cards)
}

/// Ask `n` sequential tech-card slots left-to-right (Laboratory A/B
/// contents), validated/deduped against `seen_tech_ids` (design §5.5 item
/// 1: shared across Lab A + Lab B).
fn ask_techs(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    label: &str,
    n: usize,
    rules: &Rules,
    all_tech_ids: &[TechCardId],
    seen_tech_ids: &mut BTreeSet<u8>,
) -> Result<Vec<TechCardId>, String> {
    let mut cards = Vec::with_capacity(n);
    for slot in 0..n {
        let mut chosen: Option<TechCardId> = None;
        ask(
            input,
            out,
            &format!("{label} slot {} of {n}? (T<id> or a name)", slot + 1),
            &mut |s: &str| -> Result<(), String> {
                chosen = Some(ask_tech_answer(s, rules, all_tech_ids, seen_tech_ids)?);
                Ok(())
            },
        )?;
        cards.push(chosen.unwrap());
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

fn parse_tile_abbrev(s: &str) -> Result<AreaType, String> {
    match s.to_lowercase().as_str() {
        "cm" => Ok(AreaType::CopperMine),
        "sm" => Ok(AreaType::SilkMill),
        "tp" => Ok(AreaType::TeaPlantation),
        "fg" => Ok(AreaType::FishingGround),
        "bank" => Ok(AreaType::Bank),
        "pa" => Ok(AreaType::PortA),
        "pb" => Ok(AreaType::PortB),
        "la" => Ok(AreaType::LaboratoryA),
        "lb" => Ok(AreaType::LaboratoryB),
        "ea" => Ok(AreaType::EmploymentAgency),
        "ct" => Ok(AreaType::Chinatown),
        "ch" => Ok(AreaType::Church),
        "cu" => Ok(AreaType::Customs),
        "canal" => Ok(AreaType::Canal),
        _ => Err(format!("'{s}' is not a valid tile abbreviation (cm/sm/tp/fg/bank/pa/pb/la/lb/ea/ct/ch/cu/canal)")),
    }
}

/// `S<idx>` or a bare integer.
fn parse_site_index(s: &str) -> Result<u8, String> {
    let rest = s.strip_prefix(['S', 's']).unwrap_or(s);
    rest.parse::<u8>().map_err(|_| format!("'{s}' is not a valid building-site index"))
}

/// Match against the Debug-formatted variant name, case-insensitively.
fn parse_achievement(s: &str) -> Result<AchievementType, String> {
    ALL_ACHIEVEMENTS
        .iter()
        .copied()
        .find(|ty| format!("{ty:?}").eq_ignore_ascii_case(s))
        .ok_or_else(|| format!("'{s}' is not a valid achievement type"))
}

const ALL_ACHIEVEMENTS: [AchievementType; 12] = [
    AchievementType::Copper5,
    AchievementType::Silk6,
    AchievementType::Tea7,
    AchievementType::Seafood7,
    AchievementType::Money10,
    AchievementType::Tokens5,
    AchievementType::Orders5,
    AchievementType::Technologies4,
    AchievementType::Production4,
    AchievementType::Commercial4,
    AchievementType::Prod3Comm2,
    AchievementType::Prod2Comm3,
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
            "0", "1",                                  // keep choices for seats 0 and 1
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
            "0", "1",                                  // keep choices for seats 0 and 1
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
            format!("O{}", table.deals[0][0].0), format!("O{}", table.deals[0][1].0), "0".into(),
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
        lines.extend(["Copper5", "Silk6", "Tea7"].iter().map(|s| s.to_string()));
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
    /// QUESTION with an "already entered" message -- no abort, no
    /// deferral to the ~30-question reconcile_setup tail.
    #[test]
    fn duplicate_order_id_in_port_a_reasks_with_already_entered() {
        let mut lines = live_2p_prefix(601);
        lines.extend(["O1", "O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string())); // O1 repeated
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(transcript.contains("already entered"), "{transcript}");
        let repl = repl.unwrap_or_else(|e| panic!("expected Ok, got Err({e}); transcript:\n{transcript}"));
        assert_eq!(repl.session.state().port_state.port_a.slots[0], Some(OrderCardId(1)));
        assert_eq!(repl.session.state().port_state.port_a.slots[1], Some(OrderCardId(2)));
        std::fs::remove_file("yokohama-game-601.json").ok();
    }

    /// (b) An order id outside the 1-36 registry re-asks with "not a valid
    /// order card".
    #[test]
    fn order_id_outside_registry_reasks_with_not_a_valid_order_card() {
        let mut lines = live_2p_prefix(602);
        lines.extend(["O99", "O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string())); // O99 not in registry
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(transcript.contains("not a valid order card"), "{transcript}");
        assert!(repl.is_ok(), "{transcript}");
        std::fs::remove_file("yokohama-game-602.json").ok();
    }

    /// (c) A tile answer that over-fills the player-count multiset re-asks
    /// with "no cm tiles remaining for this board" -- 2p's legal multiset
    /// has exactly 1 Copper Mine, so a second "cm" is rejected.
    #[test]
    fn tile_over_fill_reasks_with_no_tiles_remaining() {
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "603".into(), "".into(),
            "hidden".into(), "hidden".into(),
        ];
        lines.extend(["cm", "cm", "sm", "tp", "fg", "bank", "pa", "la", "ea", "ch", "cu"].iter().map(|s| s.to_string())); // cell 1's first attempt repeats cell 0's "cm"
        lines.extend((0u8..10).map(|i| i.to_string()));
        lines.extend(token_letters_for_count(10)); // 5-Power tokens
        lines.extend(["Copper5", "Silk6", "Tea7"].iter().map(|s| s.to_string()));
        lines.extend(["O1", "O2", "O3", "O4", "O5", "O6"].iter().map(|s| s.to_string()));
        lines.extend(["T1", "T2", "T3", "T4", "T5", "T6"].iter().map(|s| s.to_string()));
        lines.push("confirm".into());
        lines.push("yes".into()); // board check (third review I1)
        let (repl, transcript) = feed_owned(lines);
        assert!(transcript.contains("no cm tiles remaining for this board"), "{transcript}");
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
        assert!(transcript.contains("cm"), "summary must show the entered tiles\n{transcript}");
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
            "bogus", "still-bad", "nope", "0", // seat 0: 3 invalid + 1 valid keep answer
            "1",                                // seat 1 keep
            "",                                 // bot seats: none (T12)
            "confirm",
        ]);
        assert_eq!(transcript.matches("is not 0 or 1").count(), 3, "{transcript}");
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
            feed(&["sim", "2", "red", "blue", "0", "605", "", "0", "1", "", "confirm"]);
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
        lines.extend(["Copper5", "Silk6", "Tea7"].iter().map(|s| s.to_string()));
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
        lines.extend(["Copper5", "Silk6", "Tea7"].iter().map(|s| s.to_string()));
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
        lines.extend(["Copper5", "Silk6", "Tea7"].iter().map(|s| s.to_string()));
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
            transcript.matches("building-site index?").count(),
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
        let (repl, transcript) = feed(&["sim", "2", "red", "blue", "0", "618", "", "0", "1", "", "confirm"]);
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
            "0", "1",       // seat 0/1 keep choices, answered while looking at seed 700's deal
            "",             // bot seats: none (T12)
            "edit seed",
            "701",          // change the seed -- the seed-700 preview the operator just answered against is now stale
            "0", "1",       // seat 0/1 keep choices must be RE-ASKED against seed 701's (different) deal
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
            "0", "1",     // keep choices
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
            "0", "1",
            "1",          // bots: seat 1
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
            "0", "1",
            "0 1",        // every seat a bot -- refused
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
}
