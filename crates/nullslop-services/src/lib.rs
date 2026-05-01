//! Application-wide runtime services.
//!
//! This crate defines the [`Services`] container, which holds long-lived
//! runtime infrastructure that subsystems need access to. It is created
//! once during startup and shared throughout the application.

use std::sync::Arc;

use nullslop_actor_host::ActorHostService;
use tokio::runtime::Handle;

/// Runtime services shared across the application.
///
/// Holds references to all services, enabling dependency injection
/// and making it easy to swap implementations for testing.
#[derive(Debug, Clone)]
pub struct Services {
    /// Async runtime handle for spawning background tasks.
    handle: Handle,
    /// Actor host service.
    actor_host: ActorHostService,
}

impl Services {
    /// Creates a new `Services` with the given async runtime handle
    /// and actor host.
    #[must_use]
    pub fn new(handle: Handle, actor_host: Arc<dyn nullslop_actor_host::ActorHost>) -> Self {
        Self {
            handle,
            actor_host: ActorHostService::new(actor_host),
        }
    }

    /// Returns a reference to the async runtime handle.
    #[must_use]
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Returns a reference to the actor host service.
    #[must_use]
    pub fn actor_host(&self) -> &ActorHostService {
        &self.actor_host
    }
}
