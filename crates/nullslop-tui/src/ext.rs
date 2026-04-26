//! Extension host trait and service wrapper.
//!
//! Defines the [`ExtensionHost`] trait for managing extension processes,
//! the [`ExtensionHostService`] service wrapper, and [`ExtensionError`].

use std::sync::Arc;

use derive_more::Debug;
use wherror::Error;

use nullslop_core::Event;

/// Error type for extension operations.
#[derive(Debug, Error)]
#[error(debug)]
pub struct ExtensionError;

/// Trait for managing extension processes.
///
/// Implementations handle discovery, spawning, event routing, and
/// lifecycle management. The real implementation runs an async task
/// internally; the trait methods are synchronous entry points.
pub trait ExtensionHost: Send + Sync + 'static {
    /// Returns the host's name (for debugging/logging).
    fn name(&self) -> &'static str;

    /// Broadcasts an event to all subscribed extensions.
    fn send_event(&self, event: &Event);

    /// Shuts down all extension processes gracefully.
    fn shutdown(&self);
}

/// Service wrapper for the extension host.
///
/// Wraps `Arc<dyn ExtensionHost>` following the service wrapper pattern.
/// This is the type that [`TuiApp`](crate::TuiApp) holds.
#[derive(Clone, Debug)]
pub struct ExtensionHostService {
    #[debug("backend<{}>", self.svc.name())]
    svc: Arc<dyn ExtensionHost>,
}

impl ExtensionHostService {
    /// Creates a new service wrapper.
    #[must_use]
    pub fn new(svc: Arc<dyn ExtensionHost>) -> Self {
        Self { svc }
    }

    /// Delegates to [`ExtensionHost::send_event`].
    pub fn send_event(&self, event: &Event) {
        self.svc.send_event(event);
    }

    /// Delegates to [`ExtensionHost::shutdown`].
    pub fn shutdown(&self) {
        self.svc.shutdown();
    }
}

pub mod discovery;
pub mod fake;
pub mod host;
pub mod process;

#[cfg(test)]
mod tests_ext {
    use super::*;

    #[test]
    fn extension_error_display() {
        // Given an ExtensionError.
        let err = ExtensionError;

        // Then its Debug representation is meaningful.
        assert!(format!("{err:?}").contains("ExtensionError"));
    }
}
