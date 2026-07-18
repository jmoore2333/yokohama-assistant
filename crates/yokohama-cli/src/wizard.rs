//! Interactive setup wizard: scripted-testable sim/live game-start Q&A that
//! produces a ready-to-play `Repl` (design docs/plans/2026-07-18-phase2-cli-driver-design.md).
//! No stdin/stdout of its own -- `main.rs` drives `run_wizard` with real I/O
//! and `ask`'s `out` closure, same separation as `repl::Repl::step`.

use crate::session::Session;
use yokohama_core::*;

/// Ask one question, retrying up to 3 times on invalid input. `parse` reports
/// success/failure only (`Ok(())`/`Err(reason)`) -- callers capture the
/// parsed value into a local `Option` from inside the closure, which is
/// guaranteed `Some` once `ask` returns `Ok(())` (the closure only returns
/// `Ok(())` after setting it). Every prompt and error goes through `out` as
/// it happens, so scripted tests can assert on the transcript and `main` can
/// print interactively from the exact same code path.
fn ask(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    prompt: &str,
    parse: &mut dyn FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    for _ in 0..3 {
        out(prompt);
        out("\n");
        let line = input.next().ok_or("input ended mid-wizard")?;
        match parse(line.trim()) {
            Ok(()) => return Ok(()),
            Err(e) => out(&format!("  ! {e}\n")),
        }
    }
    Err(format!("gave up after 3 invalid answers to: {prompt}"))
}

/// Run the interactive setup wizard to completion: mode, player count,
/// seat colors, start seat, seed, save path, then a mode-specific tail
/// (sim: per-seat dealt-card preview + keep choice; live: per-seat
/// known/hidden deal plus full physical-table observations), producing a
/// `Session` wrapped in a `Repl`. The blank-seed `SystemTime` derivation
/// below is the only nondeterminism anywhere in this crate -- everything
/// else is driven entirely by `input`.
pub fn run_wizard(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
) -> Result<crate::repl::Repl, String> {
    let rules = Rules::standard();

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
    let is_live = is_live.unwrap(); // `ask` returned Ok(()) only after this was set

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
    let players = players.unwrap();

    // Colors: one per seat, distinct.
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
    let start_player = start.unwrap();

    // Seed: blank derives from wall-clock nanos -- the crate's one and only
    // permitted nondeterminism -- and echoes it so the game stays
    // reproducible from the transcript/GameRecord alone.
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
    let seed = match seed_answer.unwrap() {
        Some(n) => n,
        None => {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            out(&format!("seed: {n}\n"));
            n
        }
    };

    let mut path_answer: Option<String> = None;
    ask(input, out, "Save path? (blank for default)", &mut |s: &str| -> Result<(), String> {
        path_answer = Some(s.to_string());
        Ok(())
    })?;
    let save_path = match path_answer.unwrap() {
        s if s.is_empty() => format!("yokohama-game-{seed}.json"),
        s => s,
    };

    if is_live {
        let mut config = GameConfig::for_player_count(players);
        config.player_colors = colors;
        config.start_player = start_player;

        // Per-seat deal: known (report both cards + a keep choice) or
        // hidden (no deals entry; StartingOrderChoice::Unknown).
        let mut deals: Vec<(PlayerId, [OrderCardId; 2])> = Vec::new();
        let mut choices: Vec<StartingOrderChoice> = Vec::new();
        for seat in 0..players {
            let pid = PlayerId::try_new(seat).map_err(|e| e.to_string())?;
            let mut known: Option<bool> = None;
            ask(
                input,
                out,
                &format!("Seat {seat}: deal known or hidden? (known/hidden)"),
                &mut |s: &str| -> Result<(), String> {
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
                },
            )?;
            if known.unwrap() {
                let mut c1: Option<OrderCardId> = None;
                ask(input, out, &format!("Seat {seat}: first dealt card? (O<id>)"), &mut |s: &str| -> Result<(), String> {
                    c1 = Some(parse_order_token(s)?);
                    Ok(())
                })?;
                let mut c2: Option<OrderCardId> = None;
                ask(input, out, &format!("Seat {seat}: second dealt card? (O<id>)"), &mut |s: &str| -> Result<(), String> {
                    c2 = Some(parse_order_token(s)?);
                    Ok(())
                })?;
                let keep = ask_keep(input, out, seat)?;
                deals.push((pid, [c1.unwrap(), c2.unwrap()]));
                choices.push(StartingOrderChoice::Keep(keep));
            } else {
                choices.push(StartingOrderChoice::Unknown);
            }
        }

        // Board: every cell's tile, then a building-site index for every
        // non-Canal cell (Canal cells carry no site and are not asked about).
        let (rows, cols) = grid_dimensions(players);
        let n = rows as usize * cols as usize;
        let mut tiles: Vec<AreaType> = Vec::with_capacity(n);
        for i in 0..n {
            let mut chosen: Option<AreaType> = None;
            ask(
                input,
                out,
                &format!("Cell {i}/{n} tile? (cm/sm/tp/fg/bank/pa/pb/la/lb/ea/ct/ch/cu/canal)"),
                &mut |s: &str| -> Result<(), String> {
                    chosen = Some(parse_tile_abbrev(s)?);
                    Ok(())
                },
            )?;
            tiles.push(chosen.unwrap());
        }
        let mut sites: Vec<Option<u8>> = Vec::with_capacity(n);
        for (i, tile) in tiles.iter().enumerate() {
            if *tile == AreaType::Canal {
                sites.push(None);
                continue;
            }
            let mut chosen: Option<u8> = None;
            ask(
                input,
                out,
                &format!("Cell {i} building-site index? (S<idx> or a number)"),
                &mut |s: &str| -> Result<(), String> {
                    chosen = Some(parse_site_index(s)?);
                    Ok(())
                },
            )?;
            sites.push(Some(chosen.unwrap()));
        }

        // Achievements: 3 distinct entries out of the 12 canonical types.
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
        let achievements: [AchievementType; 3] = achievement_list.try_into().unwrap();

        let port_a: [OrderCardId; 6] = ask_orders(input, out, "Port A", 6)?.try_into().unwrap();
        let port_b = if config.has_port_b {
            Some(ask_orders(input, out, "Port B", 6)?.try_into().unwrap())
        } else {
            None
        };
        let lab_a: [TechCardId; 6] = ask_techs(input, out, "Laboratory A", 6, &rules.techs)?.try_into().unwrap();
        let lab_b = if config.has_lab_b {
            Some(ask_techs(input, out, "Laboratory B", 6, &rules.techs)?.try_into().unwrap())
        } else {
            None
        };

        let obs = SetupObservations {
            board: Some(ObservedBoard { tiles, sites }),
            achievements: Some(achievements),
            port_a: Some(port_a),
            port_b,
            lab_a: Some(lab_a),
            lab_b,
            deals,
        };

        let mut session = Session::new_live(config, seed, obs, choices, rules)?;
        session.save_path = Some(std::path::PathBuf::from(save_path));
        Ok(crate::repl::Repl::new(session))
    } else {
        // Sim: preview each seat's dealt cards (re-running new_game with the
        // chosen seed/player-count -- Session::new_sim redoes this internally
        // too, deterministically, so the preview and the real deal always agree).
        let pending = new_game(GameConfig::for_player_count(players), &rules, seed).map_err(|e| e.to_string())?;
        let mut choices: Vec<StartingOrderChoice> = Vec::new();
        for seat in 0..players {
            out(&format!("Seat {seat} dealt:\n"));
            for &card_id in &pending.deals[seat as usize] {
                if let Some(card) = rules.orders.get(card_id) {
                    out(&format!("  {}\n", order_prompt_line(card)));
                }
            }
            let keep = ask_keep(input, out, seat)?;
            choices.push(StartingOrderChoice::Keep(keep));
        }

        let mut session = Session::new_sim(players, seed, choices, rules)?;
        session.save_path = Some(std::path::PathBuf::from(save_path));
        Ok(crate::repl::Repl::new(session))
    }
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

/// Ask `n` sequential order-card slots left-to-right (Port A/B contents).
fn ask_orders(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    label: &str,
    n: usize,
) -> Result<Vec<OrderCardId>, String> {
    let mut cards = Vec::with_capacity(n);
    for slot in 0..n {
        let mut chosen: Option<OrderCardId> = None;
        ask(
            input,
            out,
            &format!("{label} slot {} of {n}? (O<id>)", slot + 1),
            &mut |s: &str| -> Result<(), String> {
                chosen = Some(parse_order_token(s)?);
                Ok(())
            },
        )?;
        cards.push(chosen.unwrap());
    }
    Ok(cards)
}

/// Ask `n` sequential tech-card slots left-to-right (Laboratory A/B contents).
fn ask_techs(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    label: &str,
    n: usize,
    techs: &TechRegistry,
) -> Result<Vec<TechCardId>, String> {
    let mut cards = Vec::with_capacity(n);
    for slot in 0..n {
        let mut chosen: Option<TechCardId> = None;
        ask(
            input,
            out,
            &format!("{label} slot {} of {n}? (T<id> or a name)", slot + 1),
            &mut |s: &str| -> Result<(), String> {
                chosen = Some(parse_tech_token(s, techs)?);
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

/// `O<id>` or a bare integer.
fn parse_order_token(s: &str) -> Result<OrderCardId, String> {
    let rest = s.strip_prefix(['O', 'o']).unwrap_or(s);
    rest.parse::<u8>().map(OrderCardId).map_err(|_| format!("'{s}' is not a valid order-card code"))
}

/// `T<id>` or a case-insensitive name prefix, resolved only when it matches
/// exactly one registry entry (some titles circulate as 2 physical copies
/// under the same name -- an ambiguous prefix is rejected rather than
/// guessing which physical card is on the board).
fn parse_tech_token(s: &str, techs: &TechRegistry) -> Result<TechCardId, String> {
    if let Some(rest) = s.strip_prefix(['T', 't']) {
        if let Ok(n) = rest.parse::<u8>() {
            return Ok(TechCardId(n));
        }
    }
    let needle = s.to_lowercase();
    let matches: Vec<TechCardId> =
        techs.iter().filter(|c| c.name.to_lowercase().starts_with(&needle)).map(|c| c.id).collect();
    match matches.len() {
        1 => Ok(matches[0]),
        0 => Err(format!("'{s}' does not match any tech card")),
        _ => Err(format!("'{s}' matches more than one tech card -- use T<id>")),
    }
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
        ]);
        let repl = repl.unwrap();
        assert!(transcript.contains("keep"), "{transcript}");
        assert_eq!(repl.session.state().config.player_count, 2);
        assert_eq!(repl.session.record().seed, 42);
        assert!(repl.session.save_path.is_some());
        assert!(!repl.session.legal().is_empty());
        std::fs::remove_file("yokohama-game-42.json").ok(); // in case autosave fired
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
        for a in table.state.achievements.iter() {
            lines.push(format!("{:?}", a.achievement_type)); // menu accepts the type name
        }
        for i in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_a.slots[i].unwrap().0));
        }
        for i in 0..6 {
            lines.push(format!("T{}", table.state.lab_state.lab_a.slots[i].unwrap().0));
        }
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
}
