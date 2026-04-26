//! Terminal setup, event loop, and teardown.
//!
//! Sets up the terminal (raw mode + alternate screen), runs the
//! main event loop, and restores the terminal on exit. Also manages
//! the background event stream task lifecycle, cancelling it before
//! terminal suspension and restarting it afterward.

use std::io::{self, Stdout};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use error_stack::{Report, ResultExt};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::runtime::Handle;
use wherror::Error;

use crate::TuiApp;

/// Error type for TUI run operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(debug)]
pub struct TuiRunError;

/// Runs the TUI application.
///
/// Sets up the terminal, runs the main event loop, and restores
/// the terminal on exit.
///
/// # Errors
///
/// Returns an error if terminal setup, the event loop, or teardown fails.
pub fn run(mut app: TuiApp, handle: &Handle) -> Result<(), Report<TuiRunError>> {
    let mut stdout = io::stdout();
    enable_raw_mode()
        .change_context(TuiRunError)
        .attach("failed to enable raw mode")?;
    execute!(stdout, EnterAlternateScreen)
        .change_context(TuiRunError)
        .attach("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .change_context(TuiRunError)
        .attach("failed to create terminal")?;

    // Start the event stream task.
    app.event_task = Some(app.events.event_task(handle));

    // Start extension host.
    let base_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("nullslop")
        .join("extensions");
    let mut services = crate::services::Services::new(handle.clone());
    let ext_host =
        crate::ext::process::ProcessExtensionHost::start(app.events.sender(), base_dir, handle);
    services.register_extension_host(std::sync::Arc::new(ext_host));
    app.services = Some(services);

    let result = run_main_loop(&mut terminal, &mut app, handle);

    // Clean up event task.
    if let Some(task) = app.event_task.take() {
        task.abort();
    }

    // Shut down extension host.
    if let Some(svc) = app.services.as_ref()
        && let Some(ext) = svc.ext_host()
    {
        ext.shutdown();
    }

    // Restore terminal.
    if let Err(e) = disable_raw_mode() {
        tracing::error!(err = ?e, "failed to disable raw mode");
    }
    if let Err(e) = execute!(terminal.backend_mut(), LeaveAlternateScreen) {
        tracing::error!(err = ?e, "failed to leave alternate screen");
    }
    if let Err(e) = terminal.show_cursor() {
        tracing::error!(err = ?e, "failed to show cursor");
    }

    result
}

fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut TuiApp,
    handle: &Handle,
) -> Result<(), Report<TuiRunError>> {
    loop {
        let event = app
            .events
            .recv()
            .change_context(TuiRunError)
            .attach("event channel closed")?;
        app.handle_msg(event);

        while let Some(event) = app.events.try_recv() {
            app.handle_msg(event);
        }

        // Check for pending suspend after event batch processing.
        if let Some(action) = app.suspend.take_action() {
            handle_suspend_action(terminal, app, action, handle)?;
        }

        terminal
            .draw(|frame| {
                app.render(frame);
            })
            .change_context(TuiRunError)
            .attach("failed to draw frame")?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Executes a suspend/restore cycle for the given action.
///
/// 1. Aborts the background event stream task
/// 2. Drains stale messages from the channel
/// 3. Suspends the terminal via [`TerminalGuard`](crate::terminal::TerminalGuard)
/// 4. Runs the external editor via `dialoguer::Editor`
/// 5. Invokes the `on_result` closure to produce a [`TuiCommand`]
/// 6. Restarts the event stream task
/// 7. Redraws the terminal
/// 8. Dispatches the resulting command (if any)
fn handle_suspend_action(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut TuiApp,
    action: crate::suspend::SuspendAction,
    handle: &Handle,
) -> Result<(), Report<TuiRunError>> {
    // Cancel the event stream so crossterm stops polling the terminal.
    if let Some(task) = app.event_task.take() {
        task.abort();
    }
    app.events.drain();

    let result_cmd = crate::terminal::suspend_and_run(terminal, || match action {
        crate::suspend::SuspendAction::Edit {
            initial_content,
            on_result,
        } => {
            let edited = dialoguer::Editor::new()
                .edit(&initial_content)
                .ok()
                .flatten();

            let changed = edited.filter(|c| c != &initial_content);
            on_result(changed)
        }
    })
    .change_context(TuiRunError)
    .attach("failed to suspend terminal for editor")?;

    // Restart the event stream with a fresh crossterm EventStream.
    app.event_task = Some(app.events.event_task(handle));

    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .change_context(TuiRunError)
        .attach("failed to redraw after suspend")?;

    if let Some(cmd) = result_cmd {
        crate::command::dispatch(app, &cmd);
    }

    Ok(())
}
