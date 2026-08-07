use std::io::Write;

fn main() {
    yokohama_cli::style::init_from_tty();
    yokohama_cli::screen::init_from_tty();
    // The third of the three arm-once-from-the-real-streams switches, and the
    // only one that reads stdin (see `input::editor_gate`). It has to run
    // before the reader is built, and only `main` may call it -- that is what
    // keeps the line editor out of every test and every pipe.
    yokohama_cli::input::init_from_tty();
    let rules = yokohama_core::Rules::standard();
    let args: Vec<String> = std::env::args().collect();
    let mut lines = yokohama_cli::input::LineReader::from_stdin();

    if yokohama_cli::screen::enabled() {
        println!("{}", yokohama_cli::screen::banner());
    }

    let mut repl = if let Some(path) = args.get(1) {
        match yokohama_cli::session::Session::load(std::path::Path::new(path), rules) {
            Ok(s) => yokohama_cli::repl::Repl::new(s),
            Err(e) => {
                eprintln!("failed to load {path}: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let mut echo = |s: &str| {
            print!("{s}");
            std::io::stdout().flush().ok();
        };
        match yokohama_cli::wizard::run_wizard(&mut lines, &mut echo) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("setup aborted: {e}");
                std::process::exit(1);
            }
        }
    };

    // Everything a load puts on screen before the loop reads a line: the bot
    // pump (call site A, spec §9 -- a bot on the START seat has to move before
    // the human ever sees a prompt, since nothing else in the REPL runs until
    // the human types), the provenance warnings from a resumed save, and the
    // opening frame. Composed in the library so the relation between the two
    // screens is testable; see `Opening`.
    let opening = repl.opening();
    // THE ONE PLACE A LINE IS CONSUMED OUTSIDE THE LOOP, and it is inside the
    // `if`: no review, no read. A review is offered only where `input::may_pause`
    // says a human is at a terminal, so a piped run reaches the loop with its
    // stream untouched and its first command still its first command.
    if let Some(review) = &opening.review {
        print!("{review}");
        std::io::stdout().flush().ok();
        lines.next();
    }
    print!("{}", opening.frame);
    std::io::stdout().flush().ok();
    for line in lines {
        let out = repl.step(&line);
        if out.quit {
            print!("{}", out.text);
            std::io::stdout().flush().ok();
            break;
        }
        print!("{}", yokohama_cli::screen::wrap(&repl, &out.text));
        std::io::stdout().flush().ok();
    }
}
