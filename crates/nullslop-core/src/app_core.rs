//! Application core: bus, state, and processing loop.
//!
//! [`AppCore`] owns the processing pipeline — the bus, shared state,
//! an internal message channel for [`AppMsg`], and an optional actor host.
//! The caller (TUI or headless runner) feeds messages into [`AppCore`] and
//! drives the processing loop.

use std::time::{Duration, Instant};

use kanal::{Receiver, Sender};
use nullslop_actor::SystemMessage;
use nullslop_actor_host::ActorHostService;
use nullslop_component::AppState;
use nullslop_component_core::Bus;

use crate::{AppMsg, State};

/// How long to wait between ticks during coordinated shutdown.
const SHUTDOWN_TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Default timeout for coordinated shutdown.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of a [`AppCore::tick`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickResult {
    /// The application has requested to quit.
    pub should_quit: bool,
    /// At least one message was processed or the bus had pending work.
    pub did_work: bool,
}

/// Application core: bus, state, and processing.
///
/// Owns the processing pipeline. The caller feeds [`AppMsg`] values
/// via [`Self::sender`] and drives processing with [`Self::tick`].
pub struct AppCore {
    /// Command and event bus for routing between components.
    pub bus: Bus<AppState>,
    /// Shared application state.
    pub state: State,
    /// Sender half of the internal message channel.
    pub sender: Sender<AppMsg>,
    /// Receiver half of the internal message channel.
    pub receiver: Receiver<AppMsg>,
    /// Optional actor host for forwarding processed messages.
    pub actor_host: Option<ActorHostService>,
}

impl std::fmt::Debug for AppCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppCore")
            .field("state", &self.state)
            .field("actor_host", &self.actor_host)
            .finish_non_exhaustive()
    }
}

impl AppCore {
    /// Creates a new `AppCore` with default state and empty bus.
    ///
    /// The caller registers components on the returned bus via
    /// [`Bus::register_command_handler`] / [`Bus::register_event_handler`],
    /// and optionally sets the actor host via [`Self::set_actor_host`].
    #[must_use]
    pub fn new(services: nullslop_services::Services) -> Self {
        let (sender, receiver) = kanal::unbounded();
        Self {
            bus: Bus::new(),
            state: State::new(AppState::new(services)),
            sender,
            receiver,
            actor_host: None,
        }
    }

    /// Returns a sender for submitting messages to the core.
    #[must_use]
    pub fn sender(&self) -> Sender<AppMsg> {
        self.sender.clone()
    }

    /// Sets the actor host service.
    ///
    /// `AppCore` holds its own [`ActorHostService`] so that [`tick()`](Self::tick)
    /// can forward processed messages without depending on the [`Services`](nullslop_services::Services) container.
    pub fn set_actor_host(&mut self, svc: ActorHostService) {
        self.actor_host = Some(svc);
    }

    /// Returns a reference to the actor host, if set.
    #[must_use]
    pub fn actor_host(&self) -> Option<&ActorHostService> {
        self.actor_host.as_ref()
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
    /// processed events to the actor host.
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

        // Forward processed items to actor host.
        let (events, commands) = self.bus.drain_all();
        self.forward_events_to_actor_host(&events);
        self.forward_commands_to_actor_host(&commands);

        TickResult {
            should_quit: self.state.read().should_quit,
            did_work: received_messages || had_pending,
        }
    }

    /// Forwards drained events to the actor host.
    ///
    /// No-op when no actor host is set.
    fn forward_events_to_actor_host(&self, items: &[nullslop_component_core::bus::ProcessedEvent]) {
        if let Some(host) = &self.actor_host {
            for item in items {
                host.send_event(&item.event, item.source.as_ref());
            }
        }
    }

    /// Forwards drained commands to the actor host.
    ///
    /// No-op when no actor host is set.
    fn forward_commands_to_actor_host(
        &self,
        items: &[nullslop_component_core::bus::ProcessedCommand],
    ) {
        if let Some(host) = &self.actor_host {
            for item in items {
                host.send_command(&item.command, item.source.as_ref());
            }
        }
    }

    /// Runs coordinated shutdown of the actor system.
    ///
    /// 1. Marks shutdown active on the tracker.
    /// 2. Sends `SystemMessage::ApplicationShuttingDown` to all actors.
    /// 3. Tick loop: drains actor events through the bus until the shutdown
    ///    tracker reports complete or the timeout expires.
    /// 4. Joins actor tasks via the host.
    ///
    /// Pass the default timeout with [`SHUTDOWN_TIMEOUT`] or a custom duration.
    pub fn coordinated_shutdown(
        &mut self,
        actor_host: &dyn nullslop_actor_host::ActorHost,
        timeout: Duration,
    ) {
        // 1. Mark shutdown active.
        self.state.write().shutdown_tracker.begin_shutdown();

        // 2. Send ApplicationShuttingDown to all actors.
        actor_host.send_system(SystemMessage::ApplicationShuttingDown);

        // 3. Tick loop: drain actor events through bus until tracker complete or timeout.
        let start = Instant::now();
        loop {
            self.tick();
            if self.state.read().shutdown_tracker.is_complete() {
                break;
            }
            if start.elapsed() > timeout {
                break;
            }
            std::thread::sleep(SHUTDOWN_TICK_INTERVAL);
        }

        // 4. Join actor tasks.
        if let Err(e) = actor_host.shutdown() {
            tracing::error!(err = ?e, "actor host shutdown error");
        }
    }

    /// Routes a command through the bus.
    fn route_command(
        &mut self,
        cmd: nullslop_protocol::Command,
        source: Option<nullslop_protocol::ActorName>,
    ) {
        self.bus.submit_command_from(cmd, source);
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::Mode;

    fn test_services() -> nullslop_services::Services {
        nullslop_services::test_services::TestServices::builder().build()
    }

    #[test]
    fn submit_command_processes_through_bus() {
        // Given an AppCore with components registered.
        let mut core = AppCore::new(test_services());
        let mut registry = nullslop_component::AppUiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut registry);

        // When submitting a quit command and ticking.
        core.submit_command(nullslop_protocol::Command::Quit);
        let result = core.tick();

        // Then should_quit is true and work was done.
        assert!(result.should_quit);
        assert!(result.did_work);
    }

    #[test]
    fn tick_returns_false_when_not_quit() {
        // Given an AppCore with no messages.
        let mut core = AppCore::new(test_services());

        // When ticking with no messages.
        let result = core.tick();

        // Then returns false for both.
        assert!(!result.should_quit);
        assert!(!result.did_work);
    }

    #[test]
    fn tick_processes_insert_char_command() {
        // Given an AppCore with components registered, in Input mode.
        let mut core = AppCore::new(test_services());
        let mut registry = nullslop_component::AppUiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut registry);
        core.state.write().mode = Mode::Input;

        // When submitting InsertChar and ticking.
        core.submit_command(nullslop_protocol::Command::InsertChar {
            payload: nullslop_protocol::chat_input::InsertChar { ch: 'x' },
        });
        core.tick();

        // Then the character appears in chat_input.input_buffer.
        assert_eq!(core.state.read().active_chat_input().text(), "x");
    }
}
