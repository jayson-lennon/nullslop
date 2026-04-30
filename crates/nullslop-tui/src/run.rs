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
use wherror::Error;

use crate::TuiApp;
use crate::app::scope_for_mode;
use nullslop_protocol::ActiveTab;

/// Error type for TUI run operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(debug)]
pub struct TuiRunError;

/// Runs the TUI application.
///
/// Sets up the terminal, runs the main event loop, and restores
/// the terminal on exit. The caller must provide a fully-initialized
/// [`TuiApp`] with services already set.
///
/// # Errors
///
/// Returns an error if terminal setup, the event loop, or teardown fails.
pub fn run(mut app: TuiApp) -> Result<(), Report<TuiRunError>> {
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
    let handle = app.services.handle().clone();
    app.event_task = Some(app.events.event_task(&handle));

    let result = run_main_loop(&mut terminal, &mut app, &handle);

    // Clean up event task.
    if let Some(task) = app.event_task.take() {
        task.abort();
    }

    // Shut down extension host.
    if let Err(e) = app.services.ext_host().shutdown(&mut app.core) {
        tracing::error!(err = ?e, "extension host shutdown error");
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
    handle: &tokio::runtime::Handle,
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

        // Core processing: drain messages, process bus, forward events.
        let should_quit = app.core.tick().should_quit;

        // Sync which_key scope from AppState.mode.
        let scope = scope_for_mode(app.core.state.read().mode);
        app.which_key.set_scope(scope);

        // Sync tab manager active tab from AppState.active_tab.
        sync_tab_manager(app);

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

        if should_quit {
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
/// 5. Invokes the `on_result` closure to produce the new input buffer content
/// 6. Restarts the event stream task
/// 7. Redraws the terminal
/// 8. Writes the result directly to `AppState.chat_input.input_buffer`
fn handle_suspend_action(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut TuiApp,
    action: crate::suspend::SuspendAction,
    handle: &tokio::runtime::Handle,
) -> Result<(), Report<TuiRunError>> {
    // Cancel the event stream so crossterm stops polling the terminal.
    if let Some(task) = app.event_task.take() {
        task.abort();
    }
    app.events.drain();

    let result_content = crate::terminal::suspend_and_run(terminal, || match action {
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

    // Handle the suspend result directly — set input_buffer on AppState.
    if let Some(content) = result_content {
        app.core.state.write().chat_input.input_buffer = content;
    }

    Ok(())
}

/// Sync the tab manager's active tab index to match `AppState.active_tab`.
fn sync_tab_manager(app: &mut TuiApp) {
    let active_tab = app.core.state.read().active_tab;
    let target_idx = match active_tab {
        ActiveTab::Chat => 0,
        ActiveTab::Dashboard => 1,
    };
    if let Some(current) = app.tab_manager.active_id()
        && app.tab_manager.index_of(current) != Some(target_idx)
        && let Some(tab) = app.tab_manager.tabs().get(target_idx)
    {
        app.tab_manager.switch_to(tab.id);
    }
}
