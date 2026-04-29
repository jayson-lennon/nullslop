//! Application-wide runtime services.
//!
//! This crate defines the [`Services`] container, which holds long-lived
//! runtime infrastructure that subsystems need access to. It is created
//! once during startup and shared throughout the application.

use std::sync::Arc;

use nullslop_core::ExtensionHostService;
use tokio::runtime::Handle;

/// Runtime services shared across the application.
///
/// Holds references to all services, enabling dependency injection
/// and making it easy to swap implementations for testing.
#[derive(Debug, Clone)]
pub struct Services {
    /// Async runtime handle for spawning background tasks.
    handle: Handle,
    /// Extension host service (optional — set during startup).
    ext_host: Option<ExtensionHostService>,
}

impl Services {
    /// Creates a new `Services` with the given async runtime handle.
    #[must_use]
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            ext_host: None,
        }
    }

    /// Returns a reference to the async runtime handle.
    #[must_use]
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Returns a reference to the extension host, if set.
    #[must_use]
    pub fn ext_host(&self) -> Option<&ExtensionHostService> {
        self.ext_host.as_ref()
    }

    /// Registers the extension host service.
    pub fn register_extension_host(&mut self, svc: Arc<dyn nullslop_core::ExtensionHost>) {
        self.ext_host = Some(ExtensionHostService::new(svc));
    }
}
