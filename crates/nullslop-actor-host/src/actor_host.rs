//! Actor host trait and service wrapper.

use error_stack::Report;
use nullslop_actor::SystemMessage;
use nullslop_protocol::{ActorName, Command, Event};

use crate::error::ActorHostError;

/// Trait for managing actors.
///
/// Implemented by [`InMemoryActorHost`](crate::InMemoryActorHost) for production
/// and [`FakeActorHost`](crate::FakeActorHost) for testing. Provides routing
/// of events/commands to actors and graceful shutdown.
pub trait ActorHost: Send + Sync + 'static {
    /// Returns the host's name.
    fn name(&self) -> &'static str;

    /// Routes an event to subscribed actors, skipping the source.
    fn send_event(&self, event: &Event, source: Option<&ActorName>);

    /// Routes a command to registered actors, skipping the source.
    fn send_command(&self, command: &Command, source: Option<&ActorName>);

    /// Sends a system message to all actors (no subscription needed).
    fn send_system(&self, msg: SystemMessage);

    /// Shuts down all actors gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if any actors fail to shut down within the timeout.
    fn shutdown(&self) -> Result<(), Report<ActorHostError>>;
}

/// Service wrapper for the actor host.
///
/// Wraps `Arc<dyn ActorHost>` for shared ownership across the application.
/// Follows the service wrapper pattern from the project style guide.
#[derive(Clone)]
pub struct ActorHostService {
    /// The underlying actor host implementation.
    svc: std::sync::Arc<dyn ActorHost>,
}

impl std::fmt::Debug for ActorHostService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorHostService")
            .field("name", &self.svc.name())
            .finish()
    }
}

impl ActorHostService {
    /// Creates a new actor host service wrapping the given host.
    #[must_use]
    pub fn new(host: std::sync::Arc<dyn ActorHost>) -> Self {
        Self { svc: host }
    }

    /// Returns a reference to the underlying host trait object.
    #[must_use]
    pub fn backend(&self) -> &dyn ActorHost {
        self.svc.as_ref()
    }

    /// Routes an event to subscribed actors via the backend.
    pub fn send_event(&self, event: &Event, source: Option<&ActorName>) {
        self.svc.send_event(event, source);
    }

    /// Routes a command to registered actors via the backend.
    pub fn send_command(&self, command: &Command, source: Option<&ActorName>) {
        self.svc.send_command(command, source);
    }

    /// Sends a system message to all actors via the backend.
    pub fn send_system(&self, msg: SystemMessage) {
        self.svc.send_system(msg);
    }

    /// Shuts down all actors via the backend.
    ///
    /// # Errors
    ///
    /// Returns an error if any actors fail to shut down.
    pub fn shutdown(&self) -> Result<(), Report<ActorHostError>> {
        self.svc.shutdown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_host_error_has_debug_display() {
        // Given an ActorHostError.
        let err = crate::error::ActorHostError;

        // When formatting the error.
        let _debug = format!("{err:?}");

        // Then it can be formatted for debug and display.
    }

    #[test]
    fn actor_host_service_delegates_to_backend() {
        // Given a FakeActorHost wrapped in a service.
        let host = std::sync::Arc::new(crate::fake::FakeActorHost::new());
        let service = ActorHostService::new(host);

        // When querying the backend name.
        assert_eq!(service.backend().name(), "FakeActorHost");

        // Then backend returns the host.

        // And send_event/send_command/send_system don't panic.
        service.send_event(
            &Event::KeyDown {
                payload: nullslop_protocol::system::KeyDown {
                    key: nullslop_protocol::KeyEvent {
                        key: nullslop_protocol::Key::Enter,
                        modifiers: nullslop_protocol::Modifiers::none(),
                    },
                },
            },
            None,
        );
        service.send_command(&Command::Quit, None);
        service.send_system(nullslop_actor::SystemMessage::ApplicationReady);

        // And shutdown returns Ok.
        service.shutdown().expect("shutdown should succeed");
    }
}
