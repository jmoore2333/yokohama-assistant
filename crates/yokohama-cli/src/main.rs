use std::io::{BufRead, Write};

fn main() {
    let rules = yokohama_core::Rules::standard();
    let args: Vec<String> = std::env::args().collect();
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines().map_while(Result::ok);

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

    print!("{}", repl.prompt());
    std::io::stdout().flush().ok();
    for line in lines {
        let out = repl.step(&line);
        print!("{}", out.text);
        std::io::stdout().flush().ok();
        if out.quit {
            break;
        }
    }
}
