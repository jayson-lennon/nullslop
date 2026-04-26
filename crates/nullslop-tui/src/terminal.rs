//! Terminal suspend/resume via RAII guard.
//!
//! [`TerminalGuard`] suspends the TUI when created (exits raw mode, leaves
//! alternate screen) and automatically restores it when dropped — even if
//! the closure passed to [`suspend_and_run`] panics.

use std::io;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use error_stack::{Report, ResultExt};
use ratatui::{Terminal, backend::CrosstermBackend};
use wherror::Error;

/// Error type for terminal suspend/resume operations.
#[derive(Debug, Error)]
#[error(debug)]
pub struct TerminalSuspendError;

/// RAII guard for terminal suspend/resume.
///
/// Suspends the TUI when created (exits raw mode, leaves alternate screen)
/// and automatically restores it when dropped. Used to temporarily return
/// to the normal terminal for external editor sessions.
pub struct TerminalGuard<'a> {
    terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
}

impl<'a> TerminalGuard<'a> {
    /// Creates a new guard, suspending the TUI.
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal cannot be suspended.
    pub fn new(
        terminal: &'a mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<Self, Report<TerminalSuspendError>> {
        disable_raw_mode()
            .change_context(TerminalSuspendError)
            .attach("failed to disable raw mode")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .change_context(TerminalSuspendError)
            .attach("failed to leave alternate screen")?;
        terminal
            .show_cursor()
            .change_context(TerminalSuspendError)
            .attach("failed to show cursor")?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard<'_> {
    fn drop(&mut self) {
        let _ = enable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), EnterAlternateScreen);
        let _ = self.terminal.hide_cursor();
        let _ = self.terminal.clear();
    }
}

/// Suspends the TUI, runs the closure, then resumes the TUI.
///
/// Automatically handles cleanup on drop, even if the closure panics.
///
/// # Errors
///
/// Returns an error if the terminal cannot be suspended.
pub fn suspend_and_run<F, T>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    f: F,
) -> Result<T, Report<TerminalSuspendError>>
where
    F: FnOnce() -> T,
{
    let _guard = TerminalGuard::new(terminal)?;
    Ok(f())
}
