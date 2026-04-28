//! Application-wide runtime services.
//!
//! This crate defines the [`Services`] container, which holds the tokio
//! runtime handle and other long-lived infrastructure that subsystems
//! need access to. It is created once during startup and shared via
//! [`Arc`](std::sync::Arc) or cloning throughout the application.

use std::sync::Arc;

use nullslop_core::ExtensionHostService;
use tokio::runtime::Handle;

/// Runtime services shared across the application.
///
/// Holds references to all services, enabling dependency injection
/// and making it easy to swap implementations for testing.
#[derive(Debug, Clone)]
pub struct Services {
    /// Handle to the tokio runtime for spawning async tasks.
    handle: Handle,
    /// Extension host service (optional — set during startup).
    ext_host: Option<ExtensionHostService>,
}

impl Services {
    /// Creates a new `Services` with the given tokio handle.
    #[must_use]
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            ext_host: None,
        }
    }

    /// Returns a reference to the tokio runtime handle.
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
