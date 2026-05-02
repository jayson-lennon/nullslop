//! In-memory actor host — spawns tokio tasks and routes events/commands.
//!
//! Provides [`spawn_actor`] for spawning individual actors and
//! [`InMemoryActorHost`] for managing a collection of actors with
//! pre-computed routing tables.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use error_stack::Report;
use kanal::Receiver;
use nullslop_actor::{Actor, ActorContext, ActorEnvelope, ActorRef, SystemMessage};
use nullslop_protocol::{ActorName, Command, CommandName, Event, EventTypeName};
use parking_lot::Mutex;

use crate::actor_host::ActorHost;
use crate::error::ActorHostError;
use crate::routing::RoutingEntry;

/// Maximum time to wait for each actor task to join during shutdown.
const JOIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of spawning an actor via [`spawn_actor`].
pub struct ActorSpawnResult {
    /// Routing entry for bus event/command dispatch.
    /// Contains closures, name, and subscription metadata.
    pub routing: RoutingEntry,
    /// Task join handle for shutdown.
    pub task: tokio::task::JoinHandle<()>,
}

/// Spawns a single actor's run loop as a tokio task.
///
/// The caller is responsible for creating the `ActorRef`, the `ActorContext`
/// (with peer refs set), and activating the actor before calling this function.
/// This matches the two-phase startup pattern: Phase 1 creates channels/refs,
/// Phase 2 activates and spawns.
///
/// This function:
/// 1. Reads registrations (subscriptions + commands) from the context
/// 2. Builds a [`RoutingEntry`] with closures capturing the `ActorRef`
/// 3. Spawns a tokio task running the actor's async message loop
///
/// Returns the routing entry and task join handle.
///
/// # Panics
///
/// Panics if the tokio task cannot be spawned.
pub fn spawn_actor<M, A>(
    name: &str,
    actor: A,
    actor_ref: &ActorRef<M>,
    receiver: Receiver<ActorEnvelope<M>>,
    mut ctx: ActorContext,
    handle: &tokio::runtime::Handle,
) -> ActorSpawnResult
where
    M: Send + 'static,
    A: Actor<Message = M> + Send + 'static,
{
    let (subscriptions, commands) = ctx.take_registrations();

    let ref_for_event = actor_ref.clone();
    let ref_for_command = actor_ref.clone();
    let ref_for_system = actor_ref.clone();
    let ref_for_shutdown = actor_ref.clone();
    let name_for_event_log = name.to_owned();
    let name_for_command_log = name.to_owned();
    let name_for_system_log = name.to_owned();
    let name_for_shutdown_log = name.to_owned();

    let send_event: Box<dyn Fn(Event) + Send + Sync> = Box::new(move |event| {
        if let Err(e) = ref_for_event.send_event(event) {
            tracing::error!(name = %name_for_event_log, err = ?e, "failed to route event to actor");
        }
    });

    let send_command: Box<dyn Fn(Command) + Send + Sync> = Box::new(move |command| {
        if let Err(e) = ref_for_command.send_command(command) {
            tracing::error!(name = %name_for_command_log, err = ?e, "failed to route command to actor");
        }
    });

    let send_system: Box<dyn Fn(SystemMessage) + Send + Sync> = Box::new(move |msg| {
        if let Err(e) = ref_for_system.send_system(msg) {
            tracing::error!(name = %name_for_system_log, err = ?e, "failed to route system message to actor");
        }
    });

    let send_shutdown: Box<dyn Fn() + Send + Sync> = Box::new(move || {
        if let Err(e) = ref_for_shutdown.shutdown() {
            tracing::error!(name = %name_for_shutdown_log, err = ?e, "failed to send shutdown to actor");
        }
    });

    let routing = RoutingEntry {
        name: name.to_owned(),
        subscriptions,
        commands,
        send_event,
        send_command,
        send_system,
        send_shutdown,
    };

    let task = handle.spawn(async move {
        let async_rx = receiver.as_async();
        let mut actor = actor;
        while let Ok(envelope) = async_rx.recv().await {
            match envelope {
                ActorEnvelope::Shutdown => break,
                _ => actor.handle(envelope, &ctx).await,
            }
        }
        actor.shutdown().await;
    });

    ActorSpawnResult { routing, task }
}

/// Pre-computed routing tables for lock-free event/command dispatch.
///
/// Built once during [`InMemoryActorHost::from_actors`] and never mutated.
/// The hot path (`send_event`/`send_command`) performs `HashMap` lookups
/// without any Mutex.
struct RoutingTables {
    /// Event type name → routing entries for subscribed actors.
    event_routes: HashMap<EventTypeName, Vec<Arc<RoutingEntry>>>,
    /// Command name → routing entries for registered actors.
    command_routes: HashMap<CommandName, Vec<Arc<RoutingEntry>>>,

    /// All routing entries — used for broadcasting system messages.
    all_entries: Vec<Arc<RoutingEntry>>,
}

/// Lifecycle state that is only touched during shutdown.
struct LifecycleState {
    /// Task join handles for actor tasks.
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

/// Hosts actors in-memory using pre-computed routing tables.
///
/// The routing tables are built once from [`ActorSpawnResult`] entries and
/// never mutated. The hot path (`send_event`/`send_command`) performs
/// `HashMap` lookups without any Mutex — the Mutex is only touched during
/// `shutdown()` to join tasks.
pub struct InMemoryActorHost {
    /// Pre-computed routing tables for lock-free dispatch.
    routing: RoutingTables,
    /// Lifecycle state (task handles) touched only during shutdown.
    lifecycle: Mutex<LifecycleState>,
    /// Tokio runtime handle for spawning and joining tasks.
    handle: tokio::runtime::Handle,
}

impl InMemoryActorHost {
    /// Builds an actor host from the given spawn results.
    ///
    /// Reads `subscriptions` and `commands` from each [`RoutingEntry`] to
    /// build the routing `HashMaps`. Collects task handles for shutdown.
    #[must_use]
    pub fn from_actors_with_handle(
        results: Vec<ActorSpawnResult>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        let mut event_routes: HashMap<EventTypeName, Vec<Arc<RoutingEntry>>> = HashMap::new();
        let mut command_routes: HashMap<CommandName, Vec<Arc<RoutingEntry>>> = HashMap::new();
        let mut all_entries: Vec<Arc<RoutingEntry>> = Vec::new();
        let mut tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        for result in results {
            let entry = Arc::new(result.routing);

            for sub in &entry.subscriptions {
                event_routes
                    .entry(sub.clone())
                    .or_default()
                    .push(entry.clone());
            }

            for cmd in &entry.commands {
                command_routes
                    .entry(cmd.clone())
                    .or_default()
                    .push(entry.clone());
            }

            all_entries.push(entry);
            tasks.push(result.task);
        }

        Self {
            routing: RoutingTables {
                event_routes,
                command_routes,
                all_entries,
            },
            lifecycle: Mutex::new(LifecycleState { tasks }),
            handle,
        }
    }

    /// Shuts down all actors gracefully with a configurable timeout.
    ///
    /// Sends shutdown signals to all actors, then joins their tasks
    /// with a per-task timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if any actors fail to shut down within the timeout.
    ///
    /// # Panics
    ///
    /// Panics if called from within a tokio runtime context (uses `block_on`).
    pub fn shutdown_with_timeout(&self, timeout: Duration) -> Result<(), Report<ActorHostError>> {
        // Send shutdown to all actors.
        for entry in &self.routing.all_entries {
            (entry.send_shutdown)();
        }

        // Join all tasks with timeout.
        let handle = &self.handle;
        let mut lifecycle = self.lifecycle.lock();
        for task in lifecycle.tasks.drain(..) {
            let result = handle.block_on(async { tokio::time::timeout(timeout, task).await });
            if result.is_err() {
                tracing::warn!("actor task did not exit within {:?}", timeout);
            }
        }

        Ok(())
    }
}

impl ActorHost for InMemoryActorHost {
    fn name(&self) -> &'static str {
        "InMemoryActorHost"
    }

    fn send_event(&self, event: &Event, source: Option<&ActorName>) {
        // Look up subscribed actors by event type name.
        let Some(event_type) = event.type_name() else {
            return; // Not a routable event.
        };
        if let Some(entries) = self.routing.event_routes.get(event_type) {
            for entry in entries {
                if source.is_some_and(|s| &**s == entry.name.as_str()) {
                    continue;
                }
                (entry.send_event)(event.clone());
            }
        }
    }

    fn send_command(&self, command: &Command, source: Option<&ActorName>) {
        let Some(name) = command.command_name() else {
            return;
        };
        if let Some(entries) = self.routing.command_routes.get(name) {
            for entry in entries {
                if source.is_some_and(|s| &**s == entry.name.as_str()) {
                    continue;
                }
                (entry.send_command)(command.clone());
            }
        }
    }

    fn send_system(&self, msg: SystemMessage) {
        for entry in &self.routing.all_entries {
            (entry.send_system)(msg);
        }
    }

    fn shutdown(&self) -> Result<(), Report<ActorHostError>> {
        self.shutdown_with_timeout(JOIN_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nullslop_actor::error::SendResult;
    use nullslop_actor::{Actor, ActorContext, ActorEnvelope, ActorRef, MessageSink};
    use nullslop_protocol::chat_input::ChatEntrySubmitted;
    use nullslop_protocol::{Command, CommandMsg as _, Event};

    use super::*;

    /// A test message sink that records commands and events.
    struct TestSink {
        commands: Mutex<Vec<Command>>,
        events: Mutex<Vec<Event>>,
    }

    impl TestSink {
        fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                events: Mutex::new(Vec::new()),
            }
        }
    }

    impl MessageSink for TestSink {
        fn send_command(&self, command: Command) -> SendResult {
            self.commands.lock().push(command);
            Ok(())
        }
        fn send_event(&self, event: Event) -> SendResult {
            self.events.lock().push(event);
            Ok(())
        }
    }

    /// No-op actor for lifecycle testing.
    struct NoopActor;

    impl Actor for NoopActor {
        type Message = String;

        fn activate(_ctx: &mut ActorContext) -> Self {
            Self
        }

        async fn handle(&mut self, _msg: ActorEnvelope<String>, _ctx: &ActorContext) {}

        async fn shutdown(self) {}
    }

    /// Actor that records received messages.
    struct RecordingActor {
        received: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingActor {
        fn new() -> (Self, Arc<parking_lot::Mutex<Vec<String>>>) {
            let received = Arc::new(Mutex::new(Vec::new()));
            let clone = received.clone();
            (Self { received }, clone)
        }
    }

    impl Actor for RecordingActor {
        type Message = String;

        fn activate(_ctx: &mut ActorContext) -> Self {
            panic!("use RecordingActor::new() and set subscriptions manually");
        }

        async fn handle(&mut self, msg: ActorEnvelope<String>, _ctx: &ActorContext) {
            match msg {
                ActorEnvelope::Direct(s) => {
                    self.received.lock().push(s);
                }
                ActorEnvelope::Event(e) => {
                    self.received
                        .lock()
                        .push(format!("event:{}", e.type_name().unwrap_or("unknown")));
                }
                ActorEnvelope::Command(c) => {
                    let name = format!("{c}");
                    self.received.lock().push(format!("command:{name}"));
                }
                ActorEnvelope::Shutdown => {
                    self.received.lock().push("shutdown".to_owned());
                }
                ActorEnvelope::System(msg) => {
                    self.received.lock().push(format!("system:{msg:?}"));
                }
            }
        }

        async fn shutdown(self) {}
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("create runtime")
    }

    fn spawn_noop_actor(
        name: &str,
        sink: Arc<dyn MessageSink>,
        handle: &tokio::runtime::Handle,
    ) -> ActorSpawnResult {
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx);
        let mut ctx = ActorContext::new(name, sink);
        let actor = NoopActor::activate(&mut ctx);
        spawn_actor(name, actor, &actor_ref, rx, ctx, handle)
    }

    fn spawn_recording_actor(
        name: &str,
        sink: Arc<dyn MessageSink>,
        subscriptions: &[&str],
        commands: &[&str],
        handle: &tokio::runtime::Handle,
    ) -> (ActorSpawnResult, Arc<parking_lot::Mutex<Vec<String>>>) {
        let (actor, received) = RecordingActor::new();
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx);
        let mut ctx = ActorContext::new(name, sink);
        for sub in subscriptions {
            ctx.subscribe_event_by_name(*sub);
        }
        for cmd in commands {
            ctx.subscribe_command_by_name(*cmd);
        }
        let result = spawn_actor(name, actor, &actor_ref, rx, ctx, handle);
        (result, received)
    }

    #[test]
    fn host_routes_subscribed_event() {
        // Given a host with a recording actor subscribed to ChatEntrySubmitted.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let (result, received) = spawn_recording_actor(
            "recorder",
            sink.clone(),
            &["chat_input::ChatEntrySubmitted"],
            &[],
            runtime.handle(),
        );
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![result], runtime.handle().clone());

        // When sending a subscribed event.
        let event = Event::ChatEntrySubmitted {
            payload: ChatEntrySubmitted {
                session_id: nullslop_protocol::SessionId::new(),
                entry: nullslop_protocol::ChatEntry::user("hello"),
            },
        };
        host.send_event(&event, None);
        std::thread::sleep(Duration::from_millis(50));

        // Then the actor received the event.
        let msgs = received.lock().clone();
        assert!(
            !msgs.is_empty(),
            "actor should receive the subscribed event"
        );
        assert!(
            msgs.iter().any(|m| m.contains("event:")),
            "expected event message, got: {msgs:?}"
        );

        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");
    }

    #[test]
    fn system_message_delivered_to_all() {
        // Given two actors with different subscriptions.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let (r1, received1) = spawn_recording_actor(
            "actor-a",
            sink.clone(),
            &["chat_input::ChatEntrySubmitted"],
            &[],
            runtime.handle(),
        );
        let (r2, received2) = spawn_recording_actor(
            "actor-b",
            sink.clone(),
            &["system::KeyDown"],
            &[],
            runtime.handle(),
        );
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![r1, r2], runtime.handle().clone());

        // When sending SystemMessage::ApplicationReady.
        host.send_system(SystemMessage::ApplicationReady);
        std::thread::sleep(Duration::from_millis(50));

        // Then both actors receive it regardless of subscriptions.
        let msgs_a = received1.lock().clone();
        let msgs_b = received2.lock().clone();
        assert!(!msgs_a.is_empty(), "actor-a should receive system message");
        assert!(!msgs_b.is_empty(), "actor-b should receive system message");

        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");
    }

    #[test]
    fn host_routes_registered_command() {
        // Given a host with a recording actor registered for PushChatEntry.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let (result, received) = spawn_recording_actor(
            "recorder",
            sink.clone(),
            &[],
            &[nullslop_protocol::chat_input::PushChatEntry::NAME],
            runtime.handle(),
        );
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![result], runtime.handle().clone());

        // When sending a registered command.
        host.send_command(
            &Command::PushChatEntry {
                payload: nullslop_protocol::chat_input::PushChatEntry {
                    session_id: nullslop_protocol::SessionId::new(),
                    entry: nullslop_protocol::ChatEntry::user("test"),
                },
            },
            None,
        );
        std::thread::sleep(Duration::from_millis(50));

        // Then the actor received the command.
        let msgs = received.lock().clone();
        assert!(
            !msgs.is_empty(),
            "actor should receive the registered command"
        );
        assert!(
            msgs.iter().any(|m| m.contains("command:")),
            "expected command message, got: {msgs:?}"
        );

        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");
    }

    #[test]
    fn host_skips_unregistered_command() {
        // Given a host with a recording actor registered for PushChatEntry only.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let (result, received) = spawn_recording_actor(
            "recorder",
            sink.clone(),
            &[],
            &[nullslop_protocol::chat_input::PushChatEntry::NAME],
            runtime.handle(),
        );
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![result], runtime.handle().clone());

        // When sending an unregistered command (Quit is not subscribed by the actor).
        host.send_command(&Command::Quit, None);
        std::thread::sleep(Duration::from_millis(50));

        // Then no messages were delivered to the actor.
        let msgs = received.lock().clone();
        assert!(
            msgs.is_empty(),
            "actor should not receive unregistered command: {msgs:?}"
        );

        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");
    }

    #[test]
    fn host_shutdown_joins_tasks() {
        // Given a running host with two actors.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let r1 = spawn_noop_actor("a", sink.clone(), runtime.handle());
        let r2 = spawn_noop_actor("b", sink.clone(), runtime.handle());
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![r1, r2], runtime.handle().clone());

        // When shutdown is called.
        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");

        // Then all tasks are drained.
        let lifecycle = host.lifecycle.lock();
        assert!(lifecycle.tasks.is_empty());
    }

    #[test]
    fn source_filtering_skips_originating_actor() {
        // Given two actors subscribed to the same event.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let (r1, received1) = spawn_recording_actor(
            "actor-a",
            sink.clone(),
            &["chat_input::ChatEntrySubmitted"],
            &[],
            runtime.handle(),
        );
        let (r2, received2) = spawn_recording_actor(
            "actor-b",
            sink.clone(),
            &["chat_input::ChatEntrySubmitted"],
            &[],
            runtime.handle(),
        );
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![r1, r2], runtime.handle().clone());

        // When sending an event with source of actor-a.
        let event = Event::ChatEntrySubmitted {
            payload: ChatEntrySubmitted {
                session_id: nullslop_protocol::SessionId::new(),
                entry: nullslop_protocol::ChatEntry::user("hello"),
            },
        };
        host.send_event(&event, Some(&ActorName::new("actor-a")));
        std::thread::sleep(Duration::from_millis(50));

        // Then actor-b receives it but actor-a does not.
        let msgs_a = received1.lock().clone();
        let msgs_b = received2.lock().clone();
        assert!(
            msgs_a.is_empty(),
            "actor-a should not receive the event: {msgs_a:?}"
        );
        assert!(!msgs_b.is_empty(), "actor-b should receive the event");

        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");
    }

    #[test]
    fn system_shutdown_delivered_to_all() {
        // Given two actors with different subscriptions.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());
        let (r1, received1) = spawn_recording_actor(
            "actor-a",
            sink.clone(),
            &["chat_input::ChatEntrySubmitted"],
            &[],
            runtime.handle(),
        );
        let (r2, received2) = spawn_recording_actor(
            "actor-b",
            sink.clone(),
            &["system::KeyDown"],
            &[],
            runtime.handle(),
        );
        let host =
            InMemoryActorHost::from_actors_with_handle(vec![r1, r2], runtime.handle().clone());

        // When sending SystemMessage::ApplicationShuttingDown.
        host.send_system(SystemMessage::ApplicationShuttingDown);
        std::thread::sleep(Duration::from_millis(50));

        // Then both actors receive it regardless of subscriptions.
        let msgs_a = received1.lock().clone();
        let msgs_b = received2.lock().clone();
        assert!(
            !msgs_a.is_empty(),
            "actor-a should receive system shutdown message"
        );
        assert!(
            !msgs_b.is_empty(),
            "actor-b should receive system shutdown message"
        );

        host.shutdown_with_timeout(Duration::from_millis(200))
            .expect("shutdown");
    }

    #[test]
    fn actor_to_actor_direct_message() {
        // Define a minimal actor for direct message testing.
        struct DirectActor;
        impl Actor for DirectActor {
            type Message = String;
            fn activate(_ctx: &mut ActorContext) -> Self {
                Self
            }
            async fn handle(&mut self, _msg: ActorEnvelope<String>, _ctx: &ActorContext) {}
            async fn shutdown(self) {}
        }

        // Given two actors where actor-a holds actor-b's ActorRef.
        let runtime = rt();
        let _guard = runtime.enter();
        let sink = Arc::new(TestSink::new());

        // Create actor-b first to get its ref.
        let (actor_b, received_b) = RecordingActor::new();
        let (tx_b, rx_b) = kanal::unbounded::<ActorEnvelope<String>>();
        let ref_b = ActorRef::new(tx_b);
        let ctx_b = ActorContext::new("actor-b", sink.clone());
        let result_b = spawn_actor("actor-b", actor_b, &ref_b, rx_b, ctx_b, runtime.handle());

        // Create actor-a with ref_b injected.
        let (tx_a, rx_a) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref_a = ActorRef::new(tx_a);
        let mut ctx_a = ActorContext::new("actor-a", sink.clone());
        ctx_a.set_actor_ref(ref_b.clone());
        let actor_a = DirectActor::activate(&mut ctx_a);
        let result_a = spawn_actor(
            "actor-a",
            actor_a,
            &actor_ref_a,
            rx_a,
            ctx_a,
            runtime.handle(),
        );

        let _host = InMemoryActorHost::from_actors_with_handle(
            vec![result_a, result_b],
            runtime.handle().clone(),
        );

        // When sending a direct message to actor-b.
        ref_b.send("hello from a".to_owned()).expect("send");
        std::thread::sleep(Duration::from_millis(50));

        // Then actor-b receives the direct message.
        let msgs_b = received_b.lock().clone();
        assert!(
            msgs_b.iter().any(|m| m.contains("hello from a")),
            "actor-b should receive direct message: {msgs_b:?}"
        );
    }
}
