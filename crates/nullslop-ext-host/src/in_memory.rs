//! In-memory extension host.
//!
//! Runs extensions as OS threads. Events and commands are routed via kanal channels.
//! No child processes or serialization.

use std::sync::{Arc, Mutex};

use nullslop_core::{Command, Event, ExtHostSender, ExtensionHost, RegisteredExtension};
use nullslop_extension::{ChannelCommandSink, CommandSink, Context, ContextKind, InMemoryExtension};

/// Message sent to an extension's thread.
enum ExtensionMessage {
    /// An event to handle.
    Event(Event),
    /// A command to handle.
    Command(Command),
    /// Shut down the extension.
    Shutdown,
}

/// A running in-memory extension.
struct ManagedExtension {
    /// Channel sender for routing messages to the extension thread.
    sender: kanal::Sender<ExtensionMessage>,
    /// Command names this extension registered.
    commands: Vec<String>,
    /// Event type names this extension subscribed to.
    subscriptions: Vec<String>,
    /// Handle for the extension thread.
    #[allow(dead_code)]
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Hosts extensions in-memory (no child process, no serialization).
///
/// Each extension runs on its own OS thread. Events and commands are routed
/// to extensions via individual kanal channels. Commands from extensions are
/// forwarded to the application through [`ExtHostSender`].
pub struct InMemoryExtensionHost {
    extensions: Mutex<Vec<ManagedExtension>>,
}

impl InMemoryExtensionHost {
    /// Starts the in-memory host with the given extensions.
    ///
    /// For each extension:
    /// 1. Creates a kanal channel for messages (events + commands + shutdown)
    /// 2. Creates a kanal channel for extension → host commands
    /// 3. Spawns an OS thread that activates the extension and enters its message loop
    /// 4. Spawns an async task to forward commands from the extension to [`ExtHostSender`]
    /// 5. Reports registrations via `sender.send_extensions_ready()`
    #[allow(clippy::missing_panics_doc)]
    #[allow(clippy::needless_pass_by_value)]
    pub fn start(
        sender: Arc<dyn ExtHostSender>,
        extensions: Vec<Box<dyn InMemoryExtension>>,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let mut managed = Vec::new();
        let mut registrations = Vec::new();

        for mut ext in extensions {
            let (msg_tx, msg_rx) = kanal::unbounded::<ExtensionMessage>();
            let (cmd_tx, cmd_rx) = kanal::unbounded::<Command>();

            let sink: Arc<dyn CommandSink> = Arc::new(ChannelCommandSink::new(cmd_tx));
            let kind = ContextKind::InMemory {
                handle: handle.clone(),
            };
            let mut ctx = Context::new(sink, kind);

            // Activate and collect registrations.
            ext.activate(&mut ctx);
            let (commands, subscriptions) = ctx.take_registrations();

            // Spawn async task to forward commands from extension to host.
            let cmd_sender = sender.clone();
            handle.spawn(async move {
                let async_rx = cmd_rx.as_async();
                while let Ok(cmd) = async_rx.recv().await {
                    cmd_sender.send_command(cmd);
                }
            });

            // Spawn OS thread for extension message loop.
            let ext_name = format!("ext-{}", managed.len());
            let thread = std::thread::Builder::new()
                .name(ext_name)
                .spawn(move || {
                    while let Ok(msg) = msg_rx.recv() {
                        match msg {
                            ExtensionMessage::Event(event) => ext.on_event(&event, &ctx),
                            ExtensionMessage::Command(command) => {
                                ext.on_command(&command, &ctx);
                            }
                            ExtensionMessage::Shutdown => break,
                        }
                    }
                    ext.deactivate();
                })
                .expect("failed to spawn extension thread");

            let name = format!("in-memory-{}", managed.len());
            registrations.push(RegisteredExtension {
                name,
                commands: commands.clone(),
                subscriptions: subscriptions.clone(),
            });

            managed.push(ManagedExtension {
                sender: msg_tx,
                commands,
                subscriptions,
                thread: Some(thread),
            });
        }

        sender.send_extensions_ready(registrations);

        Self {
            extensions: Mutex::new(managed),
        }
    }
}

impl ExtensionHost for InMemoryExtensionHost {
    fn name(&self) -> &'static str {
        "InMemoryExtensionHost"
    }

    fn send_event(&self, event: &Event) {
        let Some(event_type) = event.type_name() else {
            return;
        };
        let exts = self.extensions.lock().unwrap();
        for ext in exts.iter() {
            if ext.subscriptions.iter().any(|s| s == event_type) {
                let _ = ext.sender.send(ExtensionMessage::Event(event.clone()));
            }
        }
    }

    fn send_command(&self, command: &Command) {
        let name = match command {
            Command::CustomCommand { payload } => &payload.name,
            _ => return,
        };
        let exts = self.extensions.lock().unwrap();
        for ext in exts.iter() {
            if ext.commands.iter().any(|c| c == name) {
                let _ = ext.sender.send(ExtensionMessage::Command(command.clone()));
            }
        }
    }

    fn shutdown(&self) {
        let mut exts = self.extensions.lock().unwrap();
        for ext in exts.drain(..) {
            let _ = ext.sender.send(ExtensionMessage::Shutdown);
            // Drop sender to ensure thread exits if it's blocking on recv.
            drop(ext.sender);
            // JoinHandle doesn't block on drop, threads clean up independently.
            drop(ext.thread);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nullslop_extension::{Context, Extension};
    use nullslop_protocol::command::CustomCommand;

    use super::*;

    /// Captures extension registrations from the host.
    struct TestSender {
        registrations: Mutex<Vec<RegisteredExtension>>,
        commands: Mutex<Vec<Command>>,
    }

    impl TestSender {
        fn new() -> Self {
            Self {
                registrations: Mutex::new(Vec::new()),
                commands: Mutex::new(Vec::new()),
            }
        }
    }

    impl ExtHostSender for TestSender {
        fn send_extensions_ready(&self, registrations: Vec<RegisteredExtension>) {
            *self.registrations.lock().unwrap() = registrations;
        }

        fn send_command(&self, command: Command) {
            self.commands.lock().unwrap().push(command);
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("create runtime")
    }

    /// Extension that registers "echo" and subscribes to `EventChatMessageSubmitted`.
    struct EchoLikeExtension;

    impl Extension for EchoLikeExtension {
        fn activate(ctx: &mut Context) -> Self {
            ctx.register_command("echo");
            ctx.subscribe("EventChatMessageSubmitted");
            Self
        }

        fn on_command(&mut self, _command: &Command, _ctx: &Context) {}
        fn on_event(&mut self, _event: &Event, _ctx: &Context) {}
        fn deactivate(&mut self) {}
    }

    /// Extension that registers multiple commands and subscriptions.
    struct SimpleExtension;

    impl Extension for SimpleExtension {
        fn activate(ctx: &mut Context) -> Self {
            ctx.register_command("foo");
            ctx.register_command("bar");
            ctx.subscribe("EventApplicationReady");
            ctx.subscribe("EventChatMessageSubmitted");
            Self
        }

        fn on_command(&mut self, _command: &Command, _ctx: &Context) {}
        fn on_event(&mut self, _event: &Event, _ctx: &Context) {}
        fn deactivate(&mut self) {}
    }

    /// No-op extension for lifecycle testing.
    struct NoopExtension;

    impl Extension for NoopExtension {
        fn activate(_ctx: &mut Context) -> Self {
            Self
        }
        fn on_command(&mut self, _command: &Command, _ctx: &Context) {}
        fn on_event(&mut self, _event: &Event, _ctx: &Context) {}
        fn deactivate(&mut self) {}
    }

    #[test]
    fn in_memory_host_routes_events_and_commands() {
        // Given an in-memory host with an echo-like extension.
        let runtime = rt();
        let sender = Arc::new(TestSender::new());

        let ext: Box<dyn InMemoryExtension> = Box::new(EchoLikeExtension);
        let host = InMemoryExtensionHost::start(sender.clone(), vec![ext], runtime.handle());

        // Then the registration was reported.
        let regs = sender.registrations.lock().unwrap();
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].commands, vec!["echo"]);
        assert_eq!(regs[0].subscriptions, vec!["EventChatMessageSubmitted"]);
        drop(regs);

        // When sending a subscribed event — no panic.
        let event = Event::EventChatMessageSubmitted {
            payload: nullslop_protocol::event::EventChatMessageSubmitted {
                entry: nullslop_core::ChatEntry::user("hello"),
            },
        };
        host.send_event(&event);

        // When sending a non-subscribed event — no panic, not routed.
        host.send_event(&Event::EventApplicationReady);

        // When sending a registered command — no panic.
        host.send_command(&Command::CustomCommand {
            payload: CustomCommand {
                name: "echo".to_string(),
                args: serde_json::json!({}),
            },
        });

        // When sending an unregistered command — no panic, not routed.
        host.send_command(&Command::AppQuit);

        // Then shutdown completes cleanly.
        host.shutdown();
    }

    #[test]
    fn in_memory_host_reports_registrations() {
        // Given an in-memory host.
        let runtime = rt();
        let sender = Arc::new(TestSender::new());

        // When starting with one extension.
        let ext: Box<dyn InMemoryExtension> = Box::new(SimpleExtension);
        let _host = InMemoryExtensionHost::start(sender.clone(), vec![ext], runtime.handle());

        // Then registrations are reported.
        let regs = sender.registrations.lock().unwrap();
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].commands, vec!["foo", "bar"]);
        assert_eq!(
            regs[0].subscriptions,
            vec!["EventApplicationReady", "EventChatMessageSubmitted"]
        );
    }

    #[test]
    fn in_memory_host_name() {
        // Given an in-memory host.
        let runtime = rt();
        let sender = Arc::new(TestSender::new());
        let host = InMemoryExtensionHost::start(sender.clone(), vec![], runtime.handle());

        // Then name is correct.
        assert_eq!(host.name(), "InMemoryExtensionHost");
    }

    #[test]
    fn in_memory_host_shutdown_completes() {
        // Given a running in-memory host.
        let runtime = rt();
        let sender = Arc::new(TestSender::new());

        let ext: Box<dyn InMemoryExtension> = Box::new(NoopExtension);
        let host = InMemoryExtensionHost::start(sender.clone(), vec![ext], runtime.handle());

        // When calling shutdown.
        host.shutdown();

        // Then it completes without hanging.
    }
}
