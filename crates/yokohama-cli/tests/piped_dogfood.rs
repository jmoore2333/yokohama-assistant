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
//! Option-A picker, max recovery in the guided retreat, Enter on forced
//! options), with a hard input cap so a regression can never hang the
//! suite.

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
/// with `"> "`), or `deadline_hit` panics with the transcript tail.
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
    screen
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix('[')?;
            let end = rest.find(']')?;
            rest[..end].parse::<usize>().ok()
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

/// The `[0-N]` max of a guided-retreat question.
fn recover_max(screen: &str) -> Option<u32> {
    let start = screen.rfind("[0-")? + 3;
    let rest = &screen[start..];
    let end = rest.find(']')?;
    rest[..end].parse().ok()
}

#[test]
fn full_piped_2p_game_runs_plain_to_winner() {
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

    let save = std::env::temp_dir().join(format!("yokohama_dogfood_{}.json", std::process::id()));
    let send = |line: &str, stdin: &mut std::process::ChildStdin| {
        stdin.write_all(line.as_bytes()).expect("write stdin");
        stdin.write_all(b"\n").expect("write newline");
        stdin.flush().expect("flush stdin");
    };

    // Wizard: 2P sim, fixed seed, save into the temp dir.
    for line in [
        "sim", "2", "red", "blue", "0", "20260721",
        &save.display().to_string(), "0", "0", "confirm",
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
            // Option-A picker: with nothing selected, toggle one random
            // area; once something is selected, place it.
            if screen.contains("confirm places no assistants") {
                let ids = area_ids(&screen);
                assert!(!ids.is_empty(), "picker directory must list areas:\n{screen}");
                ids[rng.gen_range(0..ids.len())].to_string()
            } else {
                "confirm".to_string()
            }
        } else if screen.contains("recover how many from") {
            recover_max(&screen).expect("retreat question carries its [0-N] range").to_string()
        } else if screen.lines().last().is_some_and(|l| l.starts_with("> confirm / cancel")) {
            "confirm".to_string()
        } else if screen.contains("(only option — press Enter to apply)") {
            String::new() // the bare-Enter ceremony path, exercised for real
        } else {
            let shown = shown_indices(&screen);
            if shown.is_empty() {
                if screen.contains("game over") {
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
    let full = String::from_utf8_lossy(&transcript);
    assert!(full.contains("WINNER:"), "final score table expected in the transcript");
    std::fs::remove_file(&save).ok();
}
