//! `driven` — the runner for the DRIVEN-FIXTURE harness (Phase B item 1, design
//! `docs/plans/2026-08-17-phase-b-harness-design.md`).
//!
//! Subcommands over `game::driven` and `game::driven_families`' public
//! surfaces: `list` the named roots, `table` ONE decision with every candidate
//! the contestant valued, `run` a root to terminal once, `adjudicate` every root
//! candidate by forcing it and playing the remainder out, `exact` the bounded
//! optimal solve, `family` one declared block of one frozen family (Phase C),
//! `opening` the C-D probe, and `capacity` one predicate's realized ordinal
//! supply at a master (V4 step 2 — no root table is built or read).
//!
//! ```text
//! cargo run --release -p yokohama-core --example driven -- list
//! cargo run --release -p yokohama-core --example driven -- \
//!     table g1-e113-tiemass
//! cargo run --release -p yokohama-core --example driven -- \
//!     table g1-e113-tiemass --contestant lookahead-v1:b=16
//! cargo run --release -p yokohama-core --example driven -- \
//!     table gen:final-round-already-triggered:0x...:0 --decision-seed 0x...
//! cargo run --release -p yokohama-core --example driven -- \
//!     run g3-e178-finalturn --master 0x... --seats family-weighted,lookahead-v1
//! cargo run --release -p yokohama-core --example driven -- \
//!     adjudicate g1-e228-deadrecall --master 0x... --replications 4 \
//!     --seats family-weighted,race-rusher
//! cargo run --release -p yokohama-core --example driven -- \
//!     family end-trigger --block pilot --master 0x... --arm control
//! cargo run --release -p yokohama-core --example driven -- \
//!     opening --master 0x... --games 200
//! cargo run --release -p yokohama-core --example driven -- \
//!     capacity placement-fork-same-count --master 0x... --stop-after 16
//! ```
//!
//! # THIS FILE IS ARGV GLUE AND NOTHING ELSE
//!
//! Parse, call, print, exit. The grammar, the defaults, the refusals and every
//! line of the reports live in `game::driven_runner` — in the LIB, where
//! `cargo test -p yokohama-core` already covers them. That is deliberate and it
//! is the design's own rule (§2): an example's `#[cfg(test)]` module runs under
//! NO command this repo documents unless that command is written down at four
//! doc sites (the `eval_probe` precedent), and the standing rule is that a suite
//! no gate line names is a suite that never runs. So this harness ships no new
//! test command at all — and it can, precisely because nothing testable is here.
//!
//! # NO TESTS LIVE HERE
//!
//! Anything this file grows that would want a test belongs in
//! `game::driven_runner` instead.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match yokohama_core::game::driven_runner::run(&args) {
        Ok(report) => println!("{report}"),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}
