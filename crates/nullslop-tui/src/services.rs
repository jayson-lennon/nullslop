//! Runtime services available to the TUI application.
//!
//! [`Services`] holds the tokio runtime handle and any other
//! long-lived infrastructure that subsystems need access to.
//! It is stored on [`TuiApp`](crate::TuiApp) once during startup
//! and borrowed by anything that needs to spawn tasks.

use std::sync::Arc;

use tokio::runtime::Handle;

use crate::ext::ExtensionHostService;

/// Runtime services shared across the TUI application.
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
    pub fn register_extension_host(&mut self, svc: Arc<dyn crate::ext::ExtensionHost>) {
        self.ext_host = Some(ExtensionHostService::new(svc));
    }
}
