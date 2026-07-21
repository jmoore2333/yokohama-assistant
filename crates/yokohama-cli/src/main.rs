use std::io::{BufRead, Write};

fn main() {
    yokohama_cli::style::init_from_tty();
    yokohama_cli::screen::init_from_tty();
    let rules = yokohama_core::Rules::standard();
    let args: Vec<String> = std::env::args().collect();
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines().map_while(Result::ok);

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

    // Provenance warnings from a resumed save open the first frame's BODY
    // rather than going to stderr before it: the frame leads with
    // clear-screen + home (screen::wrap), so anything printed ahead of it is
    // erased before it can be read (I-1, final whole-branch review).
    let mut opening = String::new();
    for warning in &repl.session.load_warnings {
        opening.push_str(warning);
        opening.push('\n');
    }
    opening.push_str(&repl.prompt());
    print!("{}", yokohama_cli::screen::wrap(&repl, &opening));
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
