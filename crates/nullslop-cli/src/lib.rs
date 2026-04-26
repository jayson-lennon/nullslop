//! nullslop-cli: command-line interface for the nullslop agent harness.
//!
//! Provides the main binary entry point with subcommand dispatch.
//! Running without a subcommand launches the TUI by default.

use clap::Parser;

mod app;
mod cli;
mod run;

pub use app::{App, AppError};
pub use cli::Cli;

/// Entry point for the nullslop binary.
///
/// Creates the top-level [`App`] state, parses CLI arguments, and
/// dispatches to the appropriate handler.
pub fn main() {
    // Install a default tracing subscriber so extension host logs are visible.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let mut app = match App::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("error: {e:?}");
            std::process::exit(1);
        }
    };
    let cli = Cli::parse();

    if let Err(e) = run::dispatch(&mut app, cli) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}
