//! Headless application state.
//!
//! [`HeadlessApp`] owns the processing pipeline for non-interactive mode.
//! It receives commands, runs the core tick loop until settled, and
//! shuts down the extension host.

use std::time::{Duration, Instant};

use error_stack::{Report, ResultExt};
use nullslop_core::{AppCore, AppMsg, TickResult};
use nullslop_protocol::Command;
use nullslop_protocol::command::ChatBoxSubmitMessage;
use wherror::Error;

/// Error type for headless operations.
#[derive(Debug, Error)]
#[error(debug)]
pub struct HeadlessError;

/// Maximum time the headless runner will wait for work to settle.
const HEADLESS_TIMEOUT: Duration = Duration::from_secs(10);

/// Sleep duration between ticks to allow async messages to arrive.
const TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Number of consecutive idle ticks before declaring the system settled.
const IDLE_TICKS_TO_SETTLE: usize = 3;

/// Headless application state.
///
/// Owns an [`AppCore`] and [`Services`](nullslop_services::Services) for
/// non-interactive command processing. Commands are submitted, the core
/// runs until settled, and results can be inspected.
pub struct HeadlessApp {
    /// Application core (bus, state, message channel).
    core: AppCore,
    /// Runtime services.
    services: nullslop_services::Services,
}

impl HeadlessApp {
    /// Creates a new headless app with the given core and services.
    #[must_use]
    pub fn new(core: AppCore, services: nullslop_services::Services) -> Self {
        Self { core, services }
    }

    /// Sends a chat message through the core pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent.
    pub fn send_chat(&self, message: &str) -> Result<(), Report<HeadlessError>> {
        self.core
            .sender()
            .send(AppMsg::Command {
                command: Command::ChatBoxSubmitMessage {
                    payload: ChatBoxSubmitMessage {
                        text: message.to_string(),
                    },
                },
                source: None,
            })
            .change_context(HeadlessError)
            .attach("failed to send chat command")
    }

    /// Runs a keystroke script through the keymap → command → bus → component pipeline.
    ///
    /// Each non-empty, non-comment line in the script file is parsed as a key
    /// sequence. Keys are fed to the which-key state machine, which resolves
    /// them to commands. Commands are submitted to `AppCore` and the processing
    /// loop runs after each line.
    ///
    /// # Errors
    ///
    /// Returns an error if the script file cannot be read or a command cannot
    /// be sent.
    pub fn run_script(&mut self, path: &str) -> Result<(), Report<HeadlessError>> {
        let keymap = nullslop_tui::keymap::init();
        let mut which_key =
            nullslop_tui::app::WhichKeyInstance::new(keymap, nullslop_tui::Scope::Normal);
        let leader = nullslop_protocol::KeyEvent {
            key: nullslop_protocol::Key::Char('\\'),
            modifiers: nullslop_protocol::Modifiers::none(),
        };

        let content = std::fs::read_to_string(path)
            .change_context(HeadlessError)
            .attach("failed to read script file")?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let keys = ratatui_which_key::parse_key_sequence(line, &leader);
            for key in keys {
                let scope = nullslop_tui::app::scope_for_mode(self.core.state.read().mode);
                which_key.set_scope(scope);

                if let Some(cmd) = which_key.handle_key(key) {
                    self.core
                        .sender()
                        .send(AppMsg::Command {
                            command: cmd,
                            source: None,
                        })
                        .change_context(HeadlessError)
                        .attach("failed to send script command")?;
                }
            }
            self.run_until_settled();
        }

        Ok(())
    }

    /// Runs `AppCore::tick()` in a loop until the system settles or times out.
    ///
    /// "Settled" means [`IDLE_TICKS_TO_SETTLE`] consecutive ticks performed no work.
    pub fn run_until_settled(&mut self) {
        let start = Instant::now();
        let mut consecutive_idle = 0;

        loop {
            let TickResult {
                should_quit,
                did_work,
            } = self.core.tick();

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

    /// Prints the chat history to the log for visibility.
    pub fn print_history(&self) {
        let state = self.core.state.read();
        for entry in &state.chat_history {
            tracing::info!("{entry:?}");
        }
    }

    /// Shuts down the extension host gracefully.
    pub fn shutdown(&mut self) {
        let ext = self.services.ext_host().clone();
        if let Err(e) = ext.shutdown(&mut self.core) {
            tracing::error!(err = ?e, "extension host shutdown error");
        }
    }
}
