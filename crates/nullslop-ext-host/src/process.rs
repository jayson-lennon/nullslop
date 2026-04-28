//! Real extension host that spawns child processes via an async tokio task.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nullslop_core::{Event, ExtHostSender, ExtensionHost};

/// Real extension host that spawns child processes via an async tokio task.
///
/// [`ProcessExtensionHost::start`] spawns the background task that discovers
/// manifests, spawns processes, and routes events. The trait methods are
/// synchronous entry points that communicate with the task via channels.
pub struct ProcessExtensionHost {
    /// Channel to send events to the host task for broadcasting.
    event_sender: kanal::Sender<Event>,
    /// Handle to the background host task.
    host_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl ProcessExtensionHost {
    /// Creates and starts the extension host task.
    ///
    /// The task immediately begins discovery and registration.
    /// It calls [`ExtHostSender::send_extensions_ready`] when done.
    ///
    /// # Errors
    ///
    /// Does not return errors directly, but the spawned host task may call
    /// `send_extensions_ready` with an empty list if discovery fails.
    pub fn start(
        sender: Arc<dyn ExtHostSender>,
        base_dir: PathBuf,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let (event_tx, event_rx) = kanal::unbounded();
        let task = handle.spawn(crate::host::run_extension_host(sender, event_rx, base_dir));
        Self {
            event_sender: event_tx,
            host_task: Mutex::new(Some(task)),
        }
    }
}

impl ExtensionHost for ProcessExtensionHost {
    fn name(&self) -> &'static str {
        "ProcessExtensionHost"
    }

    fn send_event(&self, event: &Event) {
        let _ = self.event_sender.send(event.clone());
    }

    fn send_command(&self, _command: &nullslop_core::Command) {
        // Process-mode command routing is not yet implemented.
        // The process host receives commands from extensions via stdout
        // and routes events to extensions via stdin. Bidirectional command
        // routing to process extensions will be added in a future phase.
    }

    fn shutdown(&self) {
        if let Some(task) = self.host_task.lock().unwrap().take() {
            task.abort();
        }
    }
}
