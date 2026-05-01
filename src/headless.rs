//! Headless application state.
//!
//! [`HeadlessApp`] owns the processing pipeline for non-interactive mode.
//! It receives commands, runs the core tick loop until settled, and
//! shuts down the actor host.

use std::time::{Duration, Instant};

use error_stack::{Report, ResultExt};
use nullslop_core::{AppCore, AppMsg, TickResult};
use nullslop_protocol::Command;
use nullslop_protocol::chat_input::SubmitMessage;
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
                command: Command::SubmitMessage {
                    payload: SubmitMessage {
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
    /// Each non-empty, non-comment line read from `reader` is parsed as a key
    /// sequence by [`parse_script`]. Keys are fed to the which-key state machine,
    /// which resolves them to commands. Commands are submitted to `AppCore` and
    /// the processing loop runs after each line.
    ///
    /// # Errors
    ///
    /// Returns an error if the script content cannot be read or a command cannot
    /// be sent.
    pub fn run_script<R>(&mut self, mut reader: R) -> Result<(), Report<HeadlessError>>
    where
        R: std::io::Read,
    {
        let keymap = nullslop_tui::keymap::init();
        let mut which_key =
            nullslop_tui::app::WhichKeyInstance::new(keymap, nullslop_tui::Scope::Normal);
        let leader = nullslop_protocol::KeyEvent {
            key: nullslop_protocol::Key::Char('\\'),
            modifiers: nullslop_protocol::Modifiers::none(),
        };

        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .change_context(HeadlessError)
            .attach("failed to read script content")?;

        let lines = parse_script(&content, &leader);

        for keys in lines {
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

    /// Shuts down the actor host gracefully.
    pub fn shutdown(&mut self) {
        self.core.coordinated_shutdown(
            self.services.actor_host().backend(),
            nullslop_core::SHUTDOWN_TIMEOUT,
        );
    }
}

/// Parses a script's content into a list of key sequences.
///
/// Each non-empty, non-comment line is parsed into a `Vec<KeyEvent>` via
/// [`ratatui_which_key::parse_key_sequence`]. Blank lines and lines starting
/// with `#` are skipped. Returns one `Vec<KeyEvent>` per non-skipped line.
pub fn parse_script(
    content: &str,
    leader: &nullslop_protocol::KeyEvent,
) -> Vec<Vec<nullslop_protocol::KeyEvent>> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| ratatui_which_key::parse_key_sequence(line, leader))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use nullslop_actor_host::FakeActorHost;
    use nullslop_protocol::{Key, KeyEvent, Modifiers};

    use super::*;

    fn leader() -> KeyEvent {
        KeyEvent {
            key: Key::Char('\\'),
            modifiers: Modifiers::none(),
        }
    }

    // --- parse_script unit tests ---

    #[test]
    fn parse_script_skips_comment_lines() {
        // Given a script with comment lines.
        let content = "# This is a comment\nq\n# Another comment";

        // When parsing.
        let lines = parse_script(content, &leader());

        // Then only the non-comment line produces a sequence.
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert_eq!(lines[0][0].key, Key::Char('q'));
    }

    #[test]
    fn parse_script_skips_blank_lines() {
        // Given a script with blank and whitespace-only lines.
        let content = "\n   \nq\n\n";

        // When parsing.
        let lines = parse_script(content, &leader());

        // Then only the non-blank line produces a sequence.
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0][0].key, Key::Char('q'));
    }

    #[test]
    fn parse_script_parses_single_key() {
        // Given a script with a single key.
        let content = "q";

        // When parsing.
        let lines = parse_script(content, &leader());

        // Then one line with one key is produced.
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert_eq!(lines[0][0].key, Key::Char('q'));
    }

    #[test]
    fn parse_script_parses_special_key() {
        // Given a script with a special key name.
        let content = "<enter>";

        // When parsing.
        let lines = parse_script(content, &leader());

        // Then one line with one Enter key is produced.
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert_eq!(lines[0][0].key, Key::Enter);
    }

    #[test]
    fn parse_script_parses_multi_key_sequence() {
        // Given a script with a multi-key sequence.
        let content = "ihello<enter>";

        // When parsing.
        let lines = parse_script(content, &leader());

        // Then 7 keys are produced: i, h, e, l, l, o, enter.
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 7);
        assert_eq!(lines[0][0].key, Key::Char('i'));
        assert_eq!(lines[0][1].key, Key::Char('h'));
        assert_eq!(lines[0][2].key, Key::Char('e'));
        assert_eq!(lines[0][3].key, Key::Char('l'));
        assert_eq!(lines[0][4].key, Key::Char('l'));
        assert_eq!(lines[0][5].key, Key::Char('o'));
        assert_eq!(lines[0][6].key, Key::Enter);
    }

    #[test]
    fn parse_script_handles_multiple_lines() {
        // Given a multi-line script.
        let content = "i\nhello<enter>\nq";

        // When parsing.
        let lines = parse_script(content, &leader());

        // Then three sequences are produced.
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].len(), 1); // i
        assert_eq!(lines[1].len(), 6); // h, e, l, l, o, enter
        assert_eq!(lines[2].len(), 1); // q
    }

    // --- Integration tests for run_script ---

    fn create_test_headless() -> HeadlessApp {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();
        let mut core = AppCore::new();
        let mut registry = nullslop_component::AppUiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut registry);

        let actor_host: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(FakeActorHost::new());

        let services = nullslop_services::Services::new(handle, actor_host);
        HeadlessApp::new(core, services)
    }

    #[test]
    fn run_script_sets_should_quit() {
        // Given a headless app and a script containing "q".
        let mut headless = create_test_headless();

        // When running the script.
        headless.run_script(Cursor::new("q")).expect("run_script");

        // Then should_quit is true.
        assert!(headless.core.state.read().should_quit);
    }

    #[test]
    fn run_script_is_noop_for_empty_content() {
        // Given a headless app and an empty script.
        let mut headless = create_test_headless();

        // When running the script.
        headless.run_script(Cursor::new("")).expect("run_script");

        // Then no state changes occurred.
        let state = headless.core.state.read();
        assert!(!state.should_quit);
        assert!(state.chat_history.is_empty());
    }
}
