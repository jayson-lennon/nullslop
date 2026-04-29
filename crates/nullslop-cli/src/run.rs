//! Command dispatch: routes parsed CLI args to the right handler.

use std::sync::Arc;
use std::time::{Duration, Instant};

use error_stack::{Report, ResultExt};
use nullslop_core::{AppCore, AppMsg, Command, TickResult};
use nullslop_protocol::command::ChatBoxSubmitMessage;
use wherror::Error;

use crate::app::App;
use crate::cli::{Cli, Commands, HeadlessCommands};
use crate::headless_ext_sender::HeadlessExtSender;

/// Error type for CLI operations.
#[derive(Debug, Error)]
#[error(debug)]
pub struct CliError;

/// Maximum time the headless runner will wait for work to settle.
const HEADLESS_TIMEOUT: Duration = Duration::from_secs(10);

/// Sleep duration between ticks to allow async messages to arrive.
const TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Number of consecutive idle ticks before declaring the system settled.
const IDLE_TICKS_TO_SETTLE: usize = 3;

/// Dispatches the CLI command to the appropriate handler.
///
/// # Errors
///
/// Returns an error if the TUI or headless runner fails.
pub fn dispatch(app: &mut App, cli: Cli) -> Result<(), Report<CliError>> {
    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => run_tui(app),
        Commands::Headless { command } => run_headless(app, command),
    }
}

/// Launches the TUI application.
///
/// # Errors
///
/// Returns an error if terminal setup, the event loop, or teardown fails.
fn run_tui(app: &mut App) -> Result<(), Report<CliError>> {
    let tui_app = nullslop_tui::TuiApp::new();
    nullslop_tui::run(tui_app, &app.handle()).change_context(CliError)
}

/// Runs in headless mode.
///
/// Creates an `AppCore`, registers plugins, starts the extension host,
/// processes the given commands, and runs until the system settles
/// (no work done for [`IDLE_TICKS_TO_SETTLE`] consecutive ticks)
/// or [`HEADLESS_TIMEOUT`] is exceeded.
///
/// # Errors
///
/// Returns an error if the headless runner fails to send commands.
fn run_headless(app: &mut App, command: Option<HeadlessCommands>) -> Result<(), Report<CliError>> {
    let handle = app.handle();

    // Create AppCore with all plugins registered.
    let mut core = AppCore::new();
    let mut registry = nullslop_plugin_ui::UiRegistry::new();
    nullslop_plugin::register_all(&mut core.bus, &mut registry);

    // Start extension host (in-memory with nullslop-echo).
    let sender = HeadlessExtSender::new(core.sender());
    let echo_ext: Box<dyn nullslop_extension::InMemoryExtension> =
        Box::new(nullslop_echo::EchoExtension);
    let ext_host =
        nullslop_ext_host::InMemoryExtensionHost::start(Arc::new(sender), vec![echo_ext], &handle);
    let ext_arc: Arc<dyn nullslop_core::ExtensionHost> = Arc::new(ext_host);
    core.set_ext_host(nullslop_core::ExtensionHostService::new(ext_arc.clone()));

    let _services = {
        let mut services = nullslop_services::Services::new(handle);
        services.register_extension_host(ext_arc);
        services
    };

    // Process the headless command(s).
    match command {
        Some(HeadlessCommands::SendChat { message }) => {
            core.sender()
                .send(AppMsg::Command(Command::ChatBoxSubmitMessage {
                    payload: ChatBoxSubmitMessage { text: message },
                }))
                .change_context(CliError)
                .attach("failed to send chat command")?;
        }
        Some(HeadlessCommands::Script { path }) => {
            run_script(&mut core, &path)?;
        }
        None => {}
    }

    // Run the processing loop until settled or timed out.
    run_until_settled(&mut core);

    // Print final state for visibility.
    {
        let state = core.state.read();
        for entry in &state.chat_history {
            tracing::info!("{entry:?}");
        }
    }

    // Shut down extension host.
    let ext = core.ext_host().cloned();
    if let Some(ext) = ext
        && let Err(e) = ext.shutdown(&mut core)
    {
        tracing::error!(err = ?e, "extension host shutdown error");
    }

    Ok(())
}

/// Runs a keystroke script through the keymap → command → bus → plugin pipeline.
///
/// Each non-empty, non-comment line in the script file is parsed as a key
/// sequence using `ratatui_which_key::parse_key_sequence`. Keys are fed to
/// the which-key state machine, which resolves them to commands. Commands
/// are submitted to `AppCore` and the processing loop runs after each line.
///
/// # Errors
///
/// Returns an error if the script file cannot be read or a command cannot
/// be sent.
fn run_script(core: &mut AppCore, path: &str) -> Result<(), Report<CliError>> {
    let keymap = nullslop_tui::keymap::init();
    let mut which_key =
        nullslop_tui::app::WhichKeyInstance::new(keymap, nullslop_tui::Scope::Normal);
    let leader = nullslop_core::KeyEvent {
        key: nullslop_core::Key::Char('\\'),
        modifiers: nullslop_core::Modifiers::none(),
    };

    let content = std::fs::read_to_string(path)
        .change_context(CliError)
        .attach("failed to read script file")?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let keys = ratatui_which_key::parse_key_sequence(line, &leader);
        for key in keys {
            // Sync scope from current mode.
            let scope = nullslop_tui::app::scope_for_mode(core.state.read().mode);
            which_key.set_scope(scope);

            if let Some(cmd) = which_key.handle_key(key) {
                core.sender()
                    .send(AppMsg::Command(cmd))
                    .change_context(CliError)
                    .attach("failed to send script command")?;
            }
        }
        // Process after each line.
        run_until_settled(core);
    }

    Ok(())
}

/// Runs `AppCore::tick()` in a loop until the system settles or times out.
///
/// "Settled" means [`IDLE_TICKS_TO_SETTLE`] consecutive ticks performed no work.
/// The extension host sends messages asynchronously (discovery, commands from
/// extensions), so a single idle tick doesn't mean we're done — we wait for
/// a few consecutive idle ticks to let async work arrive.
///
/// A hard timeout of [`HEADLESS_TIMEOUT`] prevents hanging indefinitely.
fn run_until_settled(core: &mut AppCore) {
    let start = Instant::now();
    let mut consecutive_idle = 0;

    loop {
        let TickResult {
            should_quit,
            did_work,
        } = core.tick();

        if should_quit {
            return;
        }

        if did_work {
            consecutive_idle = 0;
        } else {
            consecutive_idle += 1;
            if consecutive_idle >= IDLE_TICKS_TO_SETTLE {
                return;
            }
        }

        if start.elapsed() > HEADLESS_TIMEOUT {
            tracing::warn!("headless runner timed out after {:?}", HEADLESS_TIMEOUT);
            return;
        }

        std::thread::sleep(TICK_INTERVAL);
    }
}
