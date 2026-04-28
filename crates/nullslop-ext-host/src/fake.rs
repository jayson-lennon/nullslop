//! Fake extension host for testing.
//!
//! Tracks sent events and shutdown state for test assertions.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use nullslop_core::{Event, ExtensionHost};

/// Fake extension host for testing.
pub struct FakeExtensionHost {
    events_sent: Mutex<Vec<Event>>,
    commands_sent: Mutex<Vec<nullslop_core::Command>>,
    shutdown_called: AtomicBool,
}

impl FakeExtensionHost {
    /// Creates a new fake extension host.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events_sent: Mutex::new(Vec::new()),
            commands_sent: Mutex::new(Vec::new()),
            shutdown_called: AtomicBool::new(false),
        }
    }

    /// Returns all events that were sent via `send_event`.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn events_sent(&self) -> Vec<Event> {
        self.events_sent.lock().unwrap().clone()
    }

    /// Returns all commands that were sent via `send_command`.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn commands_sent(&self) -> Vec<nullslop_core::Command> {
        self.commands_sent.lock().unwrap().clone()
    }

    /// Returns whether `shutdown` was called.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_called.load(Ordering::SeqCst)
    }
}

impl Default for FakeExtensionHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionHost for FakeExtensionHost {
    fn name(&self) -> &'static str {
        "FakeExtensionHost"
    }

    fn send_event(&self, event: &Event) {
        self.events_sent.lock().unwrap().push(event.clone());
    }

    fn send_command(&self, command: &nullslop_core::Command) {
        self.commands_sent.lock().unwrap().push(command.clone());
    }

    fn shutdown(&self) {
        self.shutdown_called.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol as npr;

    #[test]
    fn fake_host_tracks_events() {
        // Given a fake host.
        let host = FakeExtensionHost::new();

        // When sending events.
        host.send_event(&Event::EventApplicationReady);
        host.send_event(&Event::EventCustom {
            payload: npr::event::EventCustom {
                name: "test".to_string(),
                data: serde_json::json!({}),
            },
        });

        // Then events_sent returns both.
        let events = host.events_sent();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn fake_host_tracks_shutdown() {
        // Given a fake host.
        let host = FakeExtensionHost::new();

        // When calling shutdown.
        host.shutdown();

        // Then is_shutdown is true.
        assert!(host.is_shutdown());
    }

    #[test]
    fn fake_host_name() {
        // Given a fake host.
        let host = FakeExtensionHost::new();

        // Then name returns expected.
        assert_eq!(host.name(), "FakeExtensionHost");
    }
}
