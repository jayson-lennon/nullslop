//! Fake actor host for testing.

use error_stack::Report;
use nullslop_protocol::{ActorName, Command, Event};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::actor_host::ActorHost;
use crate::error::ActorHostError;

/// A fake actor host that records sent events, commands, and shutdown calls.
///
/// Use in tests to verify routing behavior without spawning real actors.
pub struct FakeActorHost {
    events_sent: Mutex<Vec<Event>>,
    commands_sent: Mutex<Vec<Command>>,
    shutdown_called: AtomicBool,
}

impl FakeActorHost {
    /// Creates a new empty fake host.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events_sent: Mutex::new(Vec::new()),
            commands_sent: Mutex::new(Vec::new()),
            shutdown_called: AtomicBool::new(false),
        }
    }

    /// Returns all events that were routed through this host.
    #[must_use]
    pub fn events_sent(&self) -> Vec<Event> {
        self.events_sent.lock().clone()
    }

    /// Returns all commands that were routed through this host.
    #[must_use]
    pub fn commands_sent(&self) -> Vec<Command> {
        self.commands_sent.lock().clone()
    }

    /// Returns whether shutdown was called.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_called.load(Ordering::SeqCst)
    }
}

impl Default for FakeActorHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ActorHost for FakeActorHost {
    fn name(&self) -> &'static str {
        "FakeActorHost"
    }

    fn send_event(&self, event: &Event, _source: Option<&ActorName>) {
        self.events_sent.lock().push(event.clone());
    }

    fn send_command(&self, command: &Command, _source: Option<&ActorName>) {
        self.commands_sent.lock().push(command.clone());
    }

    fn shutdown(&self) -> Result<(), Report<ActorHostError>> {
        self.shutdown_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_host_tracks_events() {
        // Given a fake host.
        let host = FakeActorHost::new();

        // When sending an event.
        host.send_event(&Event::ApplicationReady, None);

        // Then the event is recorded.
        assert_eq!(host.events_sent().len(), 1);
        assert!(matches!(host.events_sent()[0], Event::ApplicationReady));
    }

    #[test]
    fn fake_host_tracks_commands() {
        // Given a fake host.
        let host = FakeActorHost::new();

        // When sending a command.
        host.send_command(&Command::Quit, None);

        // Then the command is recorded.
        assert_eq!(host.commands_sent().len(), 1);
        assert!(matches!(host.commands_sent()[0], Command::Quit));
    }

    #[test]
    fn fake_host_tracks_shutdown() {
        // Given a fake host.
        let host = FakeActorHost::new();

        // When calling shutdown.
        host.shutdown().expect("shutdown should succeed");

        // Then shutdown was recorded.
        assert!(host.is_shutdown());
    }

    #[test]
    fn fake_host_name() {
        // Given a fake host.
        let host = FakeActorHost::new();

        // Then name is correct.
        assert_eq!(host.name(), "FakeActorHost");
    }
}
