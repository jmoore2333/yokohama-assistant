//! Piped-mode dogfood (Task 6, design note §7): drive the REAL binary
//! through a FULL 2-player sim game over process pipes — the honesty
//! baseline. Proves, on the shipped executable rather than the library:
//!
//! * piped output is completely plain — not one ESC byte in the whole
//!   transcript (frames and color are TTY-only);
//! * every game action is reachable through the line protocol alone
//!   (numbered picks, the pickers' toggle/confirm flows, guided retreat,
//!   bare-Enter on forced options) all the way to a WINNER screen;
//! * the wizard scripts line-by-line with no TTY.
//!
//! The driver is adaptive: it reads each prompt and answers it (random
//! numbered pick from what is SHOWN, one toggled area + confirm in the
//! Option-A picker, one typed area id + confirm in the Option-B picker,
//! max recovery in the guided retreat, Enter on forced options), with a
//! hard input cap so a regression can never hang the suite.
//!
//! TWO games run here, sharing that one driver: all-human, and (Phase 3.2
//! Task 12) human-vs-bot with seat 1 played by the shipped family-weighted
//! rung. The driver answers only the HUMAN's prompts in both -- a bot's turn
//! arrives already played, since the REPL pumps it before printing the next
//! prompt -- so a pump that stopped working would hang this test on a prompt
//! addressed to a seat the driver does not play, rather than passing quietly.
//!
//! TIER: release gate. This ran per-edit from its introduction until
//! 2026-07-21 (T6 review), then moved to the release tier per the repo's
//! two-tier policy: a ~12s full-game process test is a distributional
//! regression net, not per-edit correctness — the per-edit slice of the
//! same honesty properties stays covered by the fast unit pins
//! (`screen::wrap` identity at rest, `style` NO_COLOR/paint-off, the
//! strip-ANSI alignment equalities, and the line-protocol REPL suite).

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Read chunks from the child's stdout on a thread; the driver consumes
/// them with a timeout so a stalled child fails the test instead of
/// hanging it.
fn spawn_reader(child: &mut Child) -> Receiver<Vec<u8>> {
    let mut stdout = child.stdout.take().expect("child stdout is piped");
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    rx
}

/// Accumulate output until the newest text ends in a prompt line (starts
/// with `"> "`), or panic with the transcript tail once the 20s per-read
/// deadline is hit.
fn read_to_prompt(rx: &Receiver<Vec<u8>>, transcript: &mut Vec<u8>) -> String {
    let start = transcript.len();
    loop {
        let ready = {
            let text = String::from_utf8_lossy(&transcript[start..]);
            text.lines().last().is_some_and(|l| l.starts_with("> "))
        };
        if ready {
            return String::from_utf8_lossy(&transcript[start..]).into_owned();
        }
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(chunk) => transcript.extend_from_slice(&chunk),
            Err(_) => panic!(
                "child produced no prompt within 20s; transcript tail:\n{}",
                String::from_utf8_lossy(&transcript[transcript.len().saturating_sub(2000)..])
            ),
        }
    }
}

/// The displayed option indices (`[k] ...` lines) on this screen.
fn shown_indices(screen: &str) -> Vec<usize> {
    // Since the T4 decision zone a long option list packs SEVERAL entries onto
    // one row, so this takes every `[k]` on an option line rather than the one
    // at a line's start. A line COUNTS as an option line when its first visible
    // character is the bracket — which is the main zone's indented rows and the
    // sub-prompts' unindented ones, and is not a line that merely contains a
    // bracket somewhere (a `[St]` on a board tile, a track's `[4][5]`).
    screen
        .lines()
        .filter(|l| l.trim_start().starts_with('['))
        .flat_map(|l| {
            l.split('[').skip(1).filter_map(|chunk| {
                let end = chunk.find(']')?;
                chunk[..end].parse::<usize>().ok()
            })
        })
        .collect()
}

/// Every `A<dd>` area-id token on this screen (the Option-A picker's
/// directory; label parentheses also match — all are real ids).
fn area_ids(screen: &str) -> Vec<u8> {
    let bytes = screen.as_bytes();
    let mut out = Vec::new();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'A' && bytes[i + 1].is_ascii_digit() && bytes[i + 2].is_ascii_digit() {
            let id = (bytes[i + 1] - b'0') * 10 + (bytes[i + 2] - b'0');
            if id < 18 && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

/// The eligible area ids listed by the Option-B picker, read from its OWN
/// listing block only (between the `(by id):` header and the `Selected:`
/// echo). Scanning the whole screen would also pick up ids out of the
/// previous step's delta lines, which are not necessarily eligible here --
/// the picker would refuse them and the driver would spin.
fn option_b_area_ids(screen: &str) -> Vec<u8> {
    let start = match screen.find(OPTION_B_HEADER) {
        Some(i) => i + OPTION_B_HEADER.len(),
        None => return Vec::new(),
    };
    let rest = &screen[start..];
    let end = rest.find("Selected:").unwrap_or(rest.len());
    area_ids(&rest[..end])
}

/// The Option-B picker's header line (C1: areas are addressed by id, and the
/// list carries no `[N]` numbering at all, so the generic numbered-pick
/// fallback below cannot answer this prompt).
const OPTION_B_HEADER: &str = "Option B — place 2 assistants in one area (by id):";

/// The `[0-N]` max of a guided-retreat question.
fn recover_max(screen: &str) -> Option<u32> {
    let start = screen.rfind("[0-")? + 3;
    let rest = &screen[start..];
    let end = rest.find(']')?;
    rest[..end].parse().ok()
}

#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): full-game process test over pipes; run with cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored before merges/phase closes (gate PASSES; see module docs)"]
fn full_piped_2p_game_runs_plain_to_winner() {
    play_a_full_2p_game("all-human", "");
}

/// The bot line the variant below looks for: `describe_move`'s output for a
/// bot seat, prefixed by the seat it was played on (`Repl::pump_bots`).
const BOT_MOVE_PREFIX: &str = "Seat 1 (Blue) \u{2014}";

/// Phase 3.2 Task 12: the same full game with seat 1 played by the shipped
/// family-weighted bot. Same binary, same driver, same honesty assertions --
/// the driver answers the HUMAN's prompts and the bot's turns arrive already
/// played, because the REPL pumps them before printing the prompt. That is the
/// property this variant exists to prove on the shipped executable: a piped
/// human-vs-bot game reaches a WINNER with no ESC byte and no prompt the driver
/// cannot answer (a bot turn that failed to pump would hang here on a prompt
/// addressed to a seat the driver does not play).
///
/// Lives in THIS binary deliberately: the documented gate command runs the
/// whole binary, so the variant needs no new gate bullet (one command per test
/// binary, CLAUDE.md's two-tier policy).
#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): full-game process test over pipes; run with cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored before merges/phase closes (gate PASSES; see module docs)"]
fn full_piped_2p_human_vs_bot_game_runs_plain_to_winner() {
    let transcript = play_a_full_2p_game("human-vs-bot", "1");
    assert!(
        transcript.contains(BOT_MOVE_PREFIX),
        "the bot's own moves must be reported on screen, not applied invisibly; transcript tail:\n{}",
        &transcript[transcript.len().saturating_sub(2000)..]
    );
    assert!(
        transcript.contains("bot seed:"),
        "the drawn bot seed must be echoed so the game is reproducible from its transcript"
    );
}

/// One full piped 2P game to a winner. `bots` is the wizard's bot-seats answer
/// (`""` for none); `label` keeps the two runs' save files apart, since both
/// tests share one process id.
fn play_a_full_2p_game(label: &str, bots: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_yokohama-cli"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the real binary");
    let rx = spawn_reader(&mut child);
    let mut stdin = child.stdin.take().expect("child stdin is piped");
    let mut transcript: Vec<u8> = Vec::new();
    let mut rng = StdRng::seed_from_u64(0x59_4F_4B_4F); // "YOKO"

    let save = std::env::temp_dir().join(format!("yokohama_dogfood_{label}_{}.json", std::process::id()));
    let send = |line: &str, stdin: &mut std::process::ChildStdin| {
        stdin.write_all(line.as_bytes()).expect("write stdin");
        stdin.write_all(b"\n").expect("write newline");
        stdin.flush().expect("flush stdin");
    };

    // Wizard: 2P sim, fixed seed, save into the temp dir, then the bot-seats
    // answer (blank = none).
    //
    // The two `"1"`s are the per-seat KEEP answers, and they are `1` rather
    // than `0` because the wizard's dealt-card menu numbers from
    // `screen_model::NUMBER_BASE` (T5's ruling: every numbered list starts at
    // 1). The `"0"` before the seed is the START SEAT -- a seat id, which the
    // ruling deliberately leaves alone.
    for line in [
        "sim", "2", "red", "blue", "0", "20260721",
        &save.display().to_string(), "1", "1", bots, "confirm",
    ] {
        send(line, &mut stdin);
    }

    let mut won = false;
    for _input in 0..20_000 {
        let screen = read_to_prompt(&rx, &mut transcript);

        if screen.contains("WINNER:") || won {
            won = true;
            break;
        }
        let answer: String = if screen.contains("type an area id to toggle") {
            // Option-A picker: with nothing selected, toggle one random area;
            // once a VALID area is selected, place it. §3.1 made the picker keep
            // the selection open when a toggled area is not a legal placement
            // right now ("NOT currently offered") instead of exit-discarding on
            // confirm, so escape that state with 'cancel' (the explicit exit) --
            // exactly as the old exit-on-invalid-confirm let this random-play
            // driver move on to a different move.
            if screen.contains("NOT currently offered") {
                "cancel".to_string()
            } else if screen.contains("confirm places no assistants") {
                let ids = area_ids(&screen);
                assert!(!ids.is_empty(), "picker directory must list areas:\n{screen}");
                ids[rng.gen_range(0..ids.len())].to_string()
            } else {
                "confirm".to_string()
            }
        } else if screen.contains(OPTION_B_HEADER) {
            // Option-B picker (C1): type an eligible area id, then confirm.
            if screen.contains("Selected: (none)") {
                let ids = option_b_area_ids(&screen);
                assert!(!ids.is_empty(), "Option-B picker must list eligible areas:\n{screen}");
                ids[rng.gen_range(0..ids.len())].to_string()
            } else {
                "confirm".to_string()
            }
        } else if screen.contains("recover how many from") {
            recover_max(&screen).expect("retreat question carries its [0-N] range").to_string()
        } else if screen.lines().last().is_some_and(|l| l.starts_with("> confirm / cancel")) {
            "confirm".to_string()
        } else if screen.contains("only option — press Enter to apply") {
            String::new() // the bare-Enter ceremony path, exercised for real
        } else {
            let shown = shown_indices(&screen);
            if shown.is_empty() {
                if screen.contains("the game is over") {
                    break; // final WINNER text already passed by
                }
                panic!("no answerable prompt found:\n{screen}");
            }
            shown[rng.gen_range(0..shown.len())].to_string()
        };
        send(&answer, &mut stdin);
    }

    // Winner text may have arrived in an earlier chunk than the last prompt.
    let full = String::from_utf8_lossy(&transcript).into_owned();
    assert!(
        won || full.contains("WINNER:"),
        "game must reach the WINNER screen; transcript tail:\n{}",
        &full[full.len().saturating_sub(2000)..]
    );

    send("quit", &mut stdin);
    drop(stdin);
    let status = child.wait().expect("child exits");
    assert!(status.success(), "clean exit expected, got {status:?}");

    // Drain whatever the reader still holds, then the honesty checks.
    while let Ok(chunk) = rx.recv_timeout(Duration::from_millis(200)) {
        transcript.extend_from_slice(&chunk);
    }
    assert!(
        !transcript.contains(&0x1b),
        "piped output must be byte-plain: found an ESC byte in the transcript"
    );
    let full = String::from_utf8_lossy(&transcript).into_owned();
    assert!(full.contains("WINNER:"), "final score table expected in the transcript");
    std::fs::remove_file(&save).ok();
    full
}
