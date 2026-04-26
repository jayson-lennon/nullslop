//! Real extension host that spawns child processes via an async tokio task.

use std::path::PathBuf;
use std::sync::Mutex;

use nullslop_core::Event;

use super::ExtensionHost;

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
    /// It sends `Msg::ExtensionsReady` when done.
    ///
    /// # Errors
    ///
    /// Does not return errors directly, but the spawned host task may send
    /// an `ExtensionsReady` message with an empty list if discovery fails.
    pub fn start(
        msg_sender: crate::msg::MsgSender,
        base_dir: PathBuf,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let (event_tx, event_rx) = kanal::unbounded();
        let task = handle.spawn(crate::ext::host::run_extension_host(
            msg_sender, event_rx, base_dir,
        ));
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

    fn shutdown(&self) {
        if let Some(task) = self.host_task.lock().unwrap().take() {
            task.abort();
        }
    }
}
