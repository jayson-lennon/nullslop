//! Application core: bus, state, and processing loop.
//!
//! [`AppCore`] owns the processing pipeline — the bus, shared state,
//! an internal message channel for [`AppMsg`], and an optional extension host.
//! The caller (TUI or headless runner) feeds messages into [`AppCore`] and
//! drives the processing loop.

use nullslop_component_core::Bus;
use nullslop_component_core::AppState;

use crate::{AppMsg, ExtensionHostService, State};

/// Result of a [`AppCore::tick`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickResult {
    /// Whether `should_quit` is set after processing.
    pub should_quit: bool,
    /// Whether any messages were processed or the bus had pending work.
    pub did_work: bool,
}

/// Application core: bus, state, and processing.
///
/// Owns the processing pipeline. The caller feeds [`AppMsg`] values
/// via [`Self::sender`] and drives processing with [`Self::tick`].
pub struct AppCore {
    /// Component command/event bus.
    pub bus: Bus,
    /// Shared application state.
    pub state: State,
    /// Internal message channel sender.
    sender: kanal::Sender<AppMsg>,
    /// Internal message channel receiver.
    receiver: kanal::Receiver<AppMsg>,
    /// Optional extension host service (shared `Arc` with [`Services`](nullslop_services::Services)).
    ext_host: Option<ExtensionHostService>,
}

impl std::fmt::Debug for AppCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppCore")
            .field("state", &self.state)
            .field("ext_host", &self.ext_host)
            .finish_non_exhaustive()
    }
}

impl AppCore {
    /// Creates a new `AppCore` with default state and empty bus.
    ///
    /// The caller registers components on the returned bus via
    /// [`Bus::register_command_handler`] / [`Bus::register_event_handler`],
    /// and optionally sets the extension host via [`Self::set_ext_host`].
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = kanal::unbounded();
        Self {
            bus: Bus::new(),
            state: State::new(AppState::new()),
            sender,
            receiver,
            ext_host: None,
        }
    }

    /// Returns a sender for submitting messages to the core.
    #[must_use]
    pub fn sender(&self) -> kanal::Sender<AppMsg> {
        self.sender.clone()
    }

    /// Sets the extension host service.
    ///
    /// `AppCore` holds its own `ExtensionHostService` so that [`tick()`](Self::tick)
    /// can forward processed events without depending on the `Services` container.
    /// The underlying `Arc<dyn ExtensionHost>` is typically shared with `Services`.
    pub fn set_ext_host(&mut self, svc: ExtensionHostService) {
        self.ext_host = Some(svc);
    }

    /// Returns a reference to the extension host, if set.
    #[must_use]
    pub fn ext_host(&self) -> Option<&ExtensionHostService> {
        self.ext_host.as_ref()
    }

    /// Submits a command to the core's message channel.
    ///
    /// Convenience method equivalent to
    /// `self.sender().send(AppMsg::Command { command: cmd, source: None })`.
    pub fn submit_command(&self, cmd: nullslop_protocol::Command) {
        let _ = self.sender.send(AppMsg::Command {
            command: cmd,
            source: None,
        });
    }

    /// Processes one batch of pending messages.
    ///
    /// Drains all available [`AppMsg`] values from the internal channel,
    /// routes them, processes the bus (commands then events), and forwards
    /// processed events to the extension host.
    ///
    /// Returns a [`TickResult`] indicating whether quit was requested and
    /// whether any work was performed.
    pub fn tick(&mut self) -> TickResult {
        let mut received_messages = false;

        // Drain all available messages.
        while let Ok(Some(msg)) = self.receiver.try_recv() {
            received_messages = true;
            match msg {
                AppMsg::Command { command, source } => {
                    self.route_command(command, source);
                }
                AppMsg::Event { event, source } => {
                    self.bus.submit_event_from(event, source);
                }
                AppMsg::ExtensionsReady(registrations) => {
                    for reg in registrations {
                        self.state.write().extensions_mut().register(reg);
                    }
                    tracing::info!("extensions ready");
                }
            }
        }

        // Check if bus has pending items from previous ticks or just-routed commands.
        let had_pending = self.bus.has_pending();

        // Process the bus: commands then events.
        {
            let mut guard = self.state.write();
            self.bus.process_commands(&mut guard);
            self.bus.process_events(&mut guard);
        }

        // Forward processed events to extension host.
        let processed_events = self.bus.drain_processed_events();
        if !processed_events.is_empty()
            && let Some(ext) = &self.ext_host
        {
            for (evt, source) in &processed_events {
                ext.send_event(evt, source.as_deref());
            }
        }

        // Forward processed commands to extension host for routing to extensions.
        let processed_commands = self.bus.drain_processed_commands();
        if !processed_commands.is_empty()
            && let Some(ext) = &self.ext_host
        {
            for (cmd, source) in &processed_commands {
                ext.send_command(cmd, source.as_deref());
            }
        }

        TickResult {
            should_quit: self.state.read().should_quit,
            did_work: received_messages || had_pending,
        }
    }

    /// Routes a command through the bus.
    fn route_command(&mut self, cmd: nullslop_protocol::Command, source: Option<String>) {
        self.bus.submit_command_from(cmd, source);
    }
}

impl Default for AppCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::Mode;

    #[test]
    fn new_core_has_empty_state() {
        // Given a new AppCore.
        let core = AppCore::new();

        // Then state has empty history and should_quit is false.
        let guard = core.state.read();
        assert!(guard.chat_history.is_empty());
        assert!(!guard.should_quit);
    }

    #[test]
    fn submit_command_processes_through_bus() {
        // Given an AppCore with components registered.
        let mut core = AppCore::new();
        let mut registry = nullslop_component_ui::UiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut registry);

        // When submitting a quit command and ticking.
        core.submit_command(nullslop_protocol::Command::AppQuit);
        let result = core.tick();

        // Then should_quit is true and work was done.
        assert!(result.should_quit);
        assert!(result.did_work);
    }

    #[test]
    fn tick_processes_extensions_ready() {
        // Given an AppCore.
        let mut core = AppCore::new();

        // When sending ExtensionsReady with a registration.
        let _ = core
            .sender()
            .send(AppMsg::ExtensionsReady(vec![crate::RegisteredExtension {
                name: "test-ext".to_string(),
                commands: vec!["echo".to_string()],
                subscriptions: vec![],
            }]));
        core.tick();

        // Then the extension is registered in state.
        let guard = core.state.read();
        assert_eq!(guard.extensions().extensions().len(), 1);
    }

    #[test]
    fn tick_returns_false_when_not_quit() {
        // Given an AppCore with no messages.
        let mut core = AppCore::new();

        // When ticking with no messages.
        let result = core.tick();

        // Then returns false for both.
        assert!(!result.should_quit);
        assert!(!result.did_work);
    }

    #[test]
    fn tick_processes_insert_char_command() {
        // Given an AppCore with components registered, in Input mode.
        let mut core = AppCore::new();
        let mut registry = nullslop_component_ui::UiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut registry);
        core.state.write().mode = Mode::Input;

        // When submitting ChatBoxInsertChar and ticking.
        core.submit_command(nullslop_protocol::Command::ChatBoxInsertChar {
            payload: nullslop_protocol::command::ChatBoxInsertChar { ch: 'x' },
        });
        core.tick();

        // Then the character appears in chat_input.input_buffer.
        assert_eq!(core.state.read().chat_input.input_buffer, "x");
    }
}
