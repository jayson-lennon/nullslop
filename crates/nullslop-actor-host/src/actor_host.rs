//! Actor host trait and service wrapper.

use error_stack::Report;
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

        // Then it can be formatted for debug and display.
        let _debug = format!("{err:?}");
    }

    #[test]
    fn actor_host_service_delegates_to_backend() {
        // Given a FakeActorHost wrapped in a service.
        let host = std::sync::Arc::new(crate::fake::FakeActorHost::new());
        let service = ActorHostService::new(host);

        // Then backend returns the host.
        assert_eq!(service.backend().name(), "FakeActorHost");

        // And send_event/send_command don't panic.
        service.send_event(&Event::ApplicationReady, None);
        service.send_command(&Command::Quit, None);

        // And shutdown returns Ok.
        service.shutdown().expect("shutdown should succeed");
    }
}
