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

    let api_key = if cli.fake_llm {
        String::new()
    } else {
        read_api_key()
    };

    if let Err(e) = app.dispatch(cli, api_key) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}

/// Reads the `OPENROUTER_API_KEY` environment variable.
///
/// Aborts with a clear error message if the variable is missing or empty.
fn read_api_key() -> String {
    match std::env::var("OPENROUTER_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            eprintln!("error: OPENROUTER_API_KEY environment variable is required");
            std::process::exit(1);
        }
    }
}
