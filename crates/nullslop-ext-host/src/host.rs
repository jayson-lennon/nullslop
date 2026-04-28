//! Async extension host task.
//!
//! The host task discovers extension manifests, spawns child processes,
//! performs the registration handshake, and routes events between
//! the main loop and extension processes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use nullslop_core::{Event, ExtHostSender, ExtensionManifest, RegisteredExtension};
use nullslop_extension::codec::OutboundMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::discovery;

/// A running extension child process managed by the host task.
///
/// Holds the process's stdin writer for sending messages, the background
/// stdout reader task handle, and the extension's subscription list for
/// event routing.
struct ManagedExtension {
    /// The extension's name.
    name: String,
    /// Events this extension is subscribed to (event type names).
    subscriptions: Vec<String>,
    /// Async write handle to the child's stdin.
    stdin: tokio::io::BufWriter<tokio::process::ChildStdin>,
    /// Handle for the background stdout reader task.
    #[allow(dead_code)]
    reader_task: tokio::task::JoinHandle<()>,
}

/// Runs the extension host lifecycle.
///
/// 1. Discovers extension manifests
/// 2. Spawns child processes
/// 3. Waits for registration messages
/// 4. Sends `ExtensionsReady` with registrations
/// 5. Enters event loop: route events to extensions
/// 6. On channel close: sends shutdown to each extension
pub async fn run_extension_host(
    sender: Arc<dyn ExtHostSender>,
    event_receiver: kanal::Receiver<Event>,
    base_dir: PathBuf,
) {
    // Phase 1: Discovery.
    let manifests = match discovery::discover_manifests(&base_dir) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(err = ?e, "extension discovery failed");
            sender.send_extensions_ready(vec![]);
            return;
        }
    };

    // Phase 2: Spawn processes and collect registrations.
    let mut extensions: Vec<ManagedExtension> = Vec::new();
    let mut registrations: Vec<RegisteredExtension> = Vec::new();

    let handle = tokio::runtime::Handle::current();

    for (dir, manifest) in &manifests {
        match spawn_and_register(&sender, dir, manifest, &handle).await {
            Some((ext, reg)) => {
                registrations.push(reg);
                extensions.push(ext);
            }
            None => {
                tracing::error!(name = %manifest.name, "failed to spawn extension");
            }
        }
    }

    // Phase 3: Signal ready.
    sender.send_extensions_ready(registrations);

    // Phase 4: Event routing loop (async receive via kanal).
    let async_rx = event_receiver.as_async();
    while let Ok(event) = async_rx.recv().await {
        route_event(&mut extensions, &event).await;
    }

    // Phase 5: Shutdown — send shutdown to each extension.
    for mut ext in extensions {
        send_shutdown(&mut ext).await;
    }
}

/// Spawns an extension process, sends initialize, waits for registration.
async fn spawn_and_register(
    sender: &Arc<dyn ExtHostSender>,
    dir: &Path,
    manifest: &ExtensionManifest,
    handle: &tokio::runtime::Handle,
) -> Option<(ManagedExtension, RegisteredExtension)> {
    let binary_path = dir.join(&manifest.binary);
    let mut child = tokio::process::Command::new(&binary_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    let stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;

    // Wrap stdin in BufWriter for efficient writes.
    let mut stdin_writer = tokio::io::BufWriter::new(stdin);

    // Send initialize.
    let init =
        serde_json::to_string(&nullslop_extension::codec::InboundMessage::Initialize).ok()?;
    stdin_writer.write_all(init.as_bytes()).await.ok()?;
    stdin_writer.write_all(b"\n").await.ok()?;
    stdin_writer.flush().await.ok()?;

    // Read the registration line directly (Option A handshake).
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).await.ok()?;

    let msg: OutboundMessage = serde_json::from_str(first_line.trim()).ok()?;
    let OutboundMessage::Register {
        commands,
        subscriptions,
    } = msg
    else {
        return None;
    };

    let name = manifest.name.clone();

    // Spawn the long-lived reader task on the remaining stdout stream.
    let reader_sender = Arc::clone(sender);
    let reader_name = name.clone();
    let reader_task = handle.spawn(read_extension_stdout(reader_sender, reader_name, reader));

    let reg = RegisteredExtension {
        name: name.clone(),
        commands,
        subscriptions: subscriptions.clone(),
    };

    let ext = ManagedExtension {
        name,
        subscriptions,
        stdin: stdin_writer,
        reader_task,
    };

    Some((ext, reg))
}

/// Reads lines from an extension's stdout and forwards commands to the main loop.
async fn read_extension_stdout(
    sender: Arc<dyn ExtHostSender>,
    name: String,
    reader: tokio::io::BufReader<tokio::process::ChildStdout>,
) {
    let mut lines = reader.lines();
    loop {
        if let Ok(Some(line)) = lines.next_line().await {
            match serde_json::from_str::<OutboundMessage>(&line) {
                Ok(OutboundMessage::Command { command }) => {
                    sender.send_command(command);
                }
                Ok(OutboundMessage::Register { .. }) => {
                    // Unexpected after init — ignore.
                }
                Err(e) => {
                    tracing::warn!(name, err = ?e, "invalid message from extension");
                }
            }
        } else {
            // EOF or error — extension died.
            tracing::warn!(name, "extension process exited");
            break;
        }
    }
}

/// Returns the subscription-relevant type name for an event, if any.
///
/// Returns `None` for key events and other non-routable events.
fn event_type_name(event: &Event) -> Option<&str> {
    match event {
        Event::EventChatMessageSubmitted { .. } => Some("EventChatMessageSubmitted"),
        Event::EventApplicationReady => Some("EventApplicationReady"),
        Event::EventCustom { payload, .. } => Some(payload.name.as_str()),
        _ => None,
    }
}

/// Routes an event to all extensions subscribed to its type.
async fn route_event(extensions: &mut [ManagedExtension], event: &Event) {
    let Some(event_type) = event_type_name(event) else {
        return; // Skip key events etc.
    };

    let msg = nullslop_extension::codec::InboundMessage::Event {
        event: event.clone(),
    };
    let Ok(json) = serde_json::to_string(&msg) else {
        return;
    };
    let bytes = format!("{json}\n");

    for ext in extensions {
        if ext.subscriptions.iter().any(|s| s == event_type) {
            if let Err(e) = ext.stdin.write_all(bytes.as_bytes()).await {
                tracing::warn!(name = %ext.name, err = ?e, "failed to write to extension");
            } else if let Err(e) = ext.stdin.flush().await {
                tracing::warn!(name = %ext.name, err = ?e, "failed to flush extension stdin");
            }
        }
    }
}

/// Sends a shutdown message to an extension process.
async fn send_shutdown(ext: &mut ManagedExtension) {
    let msg = nullslop_extension::codec::InboundMessage::Shutdown;
    let Ok(json) = serde_json::to_string(&msg) else {
        return;
    };
    let bytes = format!("{json}\n");
    if let Err(e) = ext.stdin.write_all(bytes.as_bytes()).await {
        tracing::warn!(name = %ext.name, err = ?e, "failed to send shutdown");
    } else if let Err(e) = ext.stdin.flush().await {
        tracing::warn!(name = %ext.name, err = ?e, "failed to flush shutdown");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use npr::event::EventChatMessageSubmitted;
    use nullslop_core::{ChatEntry, Key, KeyEvent, Modifiers};
    use nullslop_protocol as npr;

    #[test]
    fn new_chat_entry_maps_to_name() {
        // Given an EventChatMessageSubmitted event.
        let event = Event::EventChatMessageSubmitted {
            payload: EventChatMessageSubmitted {
                entry: ChatEntry::user("test"),
            },
        };

        // Then event_type_name returns "EventChatMessageSubmitted".
        assert_eq!(event_type_name(&event), Some("EventChatMessageSubmitted"));
    }

    #[test]
    fn application_ready_maps_to_name() {
        assert_eq!(
            event_type_name(&Event::EventApplicationReady),
            Some("EventApplicationReady")
        );
    }

    #[test]
    fn custom_event_maps_to_its_name() {
        // Given a Custom event.
        let event = Event::EventCustom {
            payload: npr::event::EventCustom {
                name: "my-event".to_string(),
                data: serde_json::json!(null),
            },
        };

        // Then event_type_name returns the custom name.
        assert_eq!(event_type_name(&event), Some("my-event"));
    }

    #[test]
    fn key_events_are_not_routable() {
        // Given key events.
        let key_event = KeyEvent {
            key: Key::Enter,
            modifiers: Modifiers::none(),
        };

        // Then they return None (not routable to extensions).
        assert_eq!(
            event_type_name(&Event::EventKeyDown {
                payload: npr::event::EventKeyDown {
                    key: key_event.clone(),
                },
            }),
            None
        );
        assert_eq!(
            event_type_name(&Event::EventKeyUp {
                payload: npr::event::EventKeyUp {
                    key: key_event.clone(),
                },
            }),
            None
        );
    }
}
