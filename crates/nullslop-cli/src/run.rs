//! Command dispatch: routes parsed CLI args to the right handler.

use error_stack::Report;

use crate::app::App;
use crate::cli::{Cli, Commands};

/// Dispatches the CLI command to the appropriate handler.
///
/// # Errors
///
/// Returns an error if the TUI or headless runner fails.
pub fn dispatch(app: &mut App, cli: Cli) -> Result<(), Report<nullslop_tui::TuiRunError>> {
    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => run_tui(app),
        Commands::Headless => run_headless(app),
    }
}

/// Launches the TUI application.
///
/// # Errors
///
/// Returns an error if terminal setup, the event loop, or teardown fails.
fn run_tui(app: &mut App) -> Result<(), Report<nullslop_tui::TuiRunError>> {
    let tui_app = nullslop_tui::TuiApp::new();
    nullslop_tui::run(tui_app, &app.handle())
}

/// Runs in headless mode (no TUI).
///
/// # Errors
///
/// Returns an error if the headless runner fails (currently always returns `Ok`).
#[allow(clippy::unnecessary_wraps)]
fn run_headless(_app: &mut App) -> Result<(), Report<nullslop_tui::TuiRunError>> {
    // TODO: implement headless mode.
    tracing::info!("headless mode not yet implemented");
    Ok(())
}
