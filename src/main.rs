//! Binary entry point for nullslop.

use clap::Parser;
use nullslop::tracing::{TracingMode, init as init_tracing};
use nullslop_cli::Cli;

fn main() {
    // Load .env if present. Not fatal if missing.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let mode = match &cli.command {
        None | Some(nullslop_cli::cli::Commands::Tui) => TracingMode::Tui {
            log_dir: cli.log_dir.clone(),
        },
        Some(nullslop_cli::cli::Commands::Headless { log_file, .. }) => TracingMode::Headless {
            log_file: log_file.clone(),
        },
    };

    if let Err(e) = init_tracing(cli.verbosity, mode) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }

    let mut app = match nullslop::App::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("error: {e:?}");
            std::process::exit(1);
        }
    };

    if let Err(e) = app.dispatch(cli) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}
