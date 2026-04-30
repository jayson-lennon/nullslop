//! Runner dispatch for TUI and headless modes.

use error_stack::{Report, ResultExt};

use crate::app::AppError;
use crate::headless::HeadlessApp;

/// Mode-specific runner.
///
/// Each variant owns the state needed for its execution mode.
/// The [`Runner::run`] method delegates to the appropriate event loop.
pub enum Runner {
    /// Terminal UI mode.
    Tui(Box<nullslop_tui::TuiApp>),
    /// Headless (non-interactive) mode.
    Headless(Box<HeadlessApp>),
}

impl Runner {
    /// Runs the selected mode to completion.
    ///
    /// For TUI mode, runs the terminal event loop.
    /// For headless mode, runs until settled, prints history, and shuts down.
    ///
    /// # Errors
    ///
    /// Returns an error if the runner fails.
    pub fn run(self) -> Result<(), Report<AppError>> {
        match self {
            Runner::Tui(app) => {
                nullslop_tui::run(*app).change_context(AppError)?;
            }
            Runner::Headless(mut app) => {
                app.run_until_settled();
                app.print_history();
                app.shutdown();
            }
        }
        Ok(())
    }
}
