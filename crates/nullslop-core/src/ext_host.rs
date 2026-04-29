//! Extension host trait, service wrapper, and sender abstraction.

use std::sync::Arc;

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

use nullslop_protocol::command::Command;

use crate::RegisteredExtension;

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

    /// Broadcasts an event to all subscribed extensions, skipping the source.
    fn send_event(&self, event: &nullslop_protocol::Event, source: Option<&str>);

    /// Routes a command to extensions that registered for it, skipping the source.
    fn send_command(&self, command: &Command, source: Option<&str>);

    /// Shuts down all extension processes gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if any extensions fail to shut down within the timeout.
    fn shutdown(&self, core: &mut crate::AppCore) -> Result<(), Report<ExtensionError>>;
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

    /// Returns a reference to the inner `Arc<dyn ExtensionHost>`.
    #[must_use]
    pub fn backend(&self) -> &Arc<dyn ExtensionHost> {
        &self.svc
    }

    /// Delegates to [`ExtensionHost::send_event`].
    pub fn send_event(&self, event: &nullslop_protocol::Event, source: Option<&str>) {
        self.svc.send_event(event, source);
    }

    /// Delegates to [`ExtensionHost::send_command`].
    pub fn send_command(&self, command: &Command, source: Option<&str>) {
        self.svc.send_command(command, source);
    }

    /// Delegates to [`ExtensionHost::shutdown`].
    ///
    /// # Errors
    ///
    /// Returns an error if any extensions fail to shut down.
    pub fn shutdown(&self, core: &mut crate::AppCore) -> Result<(), Report<ExtensionError>> {
        self.svc.shutdown(core)
    }
}

/// Abstraction for sending messages from the extension host to the application.
///
/// Implementations map extension host events into the application's message type.
/// This trait decouples the extension host from TUI-specific message types,
/// enabling headless mode to receive extension events without
/// depending on crossterm or the TUI message enum.
pub trait ExtHostSender: Send + Sync + 'static {
    /// Called when extensions have completed discovery and registration.
    fn send_extensions_ready(&self, registrations: Vec<RegisteredExtension>);
    /// Called when an extension sends a command.
    fn send_command(&self, command: Command, source: Option<&str>);
    /// Called when an extension sends an event to the host.
    fn send_extension_event(&self, event: nullslop_protocol::Event, source: Option<&str>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_error_display() {
        // Given an ExtensionError.
        let err = ExtensionError;

        // Then its Debug representation is meaningful.
        assert!(format!("{err:?}").contains("ExtensionError"));
    }
}
