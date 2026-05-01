//! Actor context for subscriptions, peer references, and sending messages.
//!
//! [`ActorContext`] is provided to actor methods. During [`activate`](crate::Actor::activate),
//! the context accumulates subscriptions and provides peer [`ActorRef<M>`](crate::ActorRef)
//! handles via [`take_actor_ref`](ActorContext::take_actor_ref). During `handle`,
//! the context can send commands and events back to the application via the
//! [`MessageSink`](crate::MessageSink) trait.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use nullslop_protocol::actor::{ActorShutdownCompleted, ActorStarted};
use nullslop_protocol::{Command, CommandMsg, CommandName, Event, EventMsg, EventTypeName};

use crate::ActorRef;
use crate::error::SendResult;
use crate::message_sink::MessageSink;

/// Context provided to actor methods.
///
/// During [`activate`](crate::Actor::activate), the context accumulates subscriptions
/// and provides peer [`ActorRef<M>`](crate::ActorRef) handles via
/// [`take_actor_ref`](ActorContext::take_actor_ref). During `handle`, the context
/// can send commands and events back to the application via the
/// [`MessageSink`](crate::MessageSink) trait.
pub struct ActorContext {
    /// The actor's host-assigned name.
    name: String,
    /// Accumulated event subscriptions (by type name).
    subscriptions: Vec<EventTypeName>,
    /// Accumulated command registrations (by name).
    commands: Vec<CommandName>,
    /// Type-keyed actor ref storage, keyed by `TypeId::of::<M>()`.
    actor_refs: HashMap<TypeId, Box<dyn Any + Send + Sync>>, // Actually Box<ActorRef<M>>
    /// Message sink for sending commands/events to the application.
    sink: Arc<dyn MessageSink>,
}

impl ActorContext {
    /// Creates a new actor context with the given name and message sink.
    ///
    /// Called by the actor host during startup — actor authors typically
    /// don't construct this directly.
    #[must_use]
    pub fn new(name: &str, sink: Arc<dyn MessageSink>) -> Self {
        Self {
            name: name.to_string(),
            subscriptions: Vec::new(),
            commands: Vec::new(),
            actor_refs: HashMap::new(),
            sink,
        }
    }

    /// Returns the actor's host-assigned name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Subscribes to a bus event by name.
    ///
    /// For compile-time-checked subscriptions, prefer
    /// [`subscribe_event`](Self::subscribe_event).
    pub fn subscribe_event_by_name(&mut self, name: impl Into<EventTypeName>) {
        self.subscriptions.push(name.into());
    }

    /// Subscribes to a typed bus event.
    ///
    /// Uses the [`EventMsg::TYPE_NAME`] constant for routing,
    /// providing compile-time validation.
    pub fn subscribe_event<T: EventMsg>(&mut self) {
        self.subscriptions.push(T::TYPE_NAME.to_string());
    }

    /// Subscribes to a bus command by name.
    ///
    /// For compile-time-checked subscriptions, prefer
    /// [`subscribe_command`](Self::subscribe_command).
    pub fn subscribe_command_by_name(&mut self, name: impl Into<CommandName>) {
        self.commands.push(name.into());
    }

    /// Subscribes to a typed bus command.
    ///
    /// Uses the [`CommandMsg::NAME`] constant for routing,
    /// providing compile-time validation.
    pub fn subscribe_command<T: CommandMsg>(&mut self) {
        self.commands.push(T::NAME.to_string());
    }

    /// Stores an [`ActorRef<M>`] keyed by the message type `M`.
    ///
    /// The actor retrieves it during activation with
    /// [`take_actor_ref::<M>()`](Self::take_actor_ref).
    pub fn set_actor_ref<M: Send + 'static>(&mut self, actor_ref: ActorRef<M>) {
        self.actor_refs
            .insert(TypeId::of::<M>(), Box::new(actor_ref));
    }

    /// Removes and returns the [`ActorRef<M>`] for message type `M`.
    ///
    /// Returns `None` if no `ActorRef` was stored for this message type.
    /// This is a take (not a clone) — subsequent calls return `None`.
    pub fn take_actor_ref<M: Send + 'static>(&mut self) -> Option<ActorRef<M>> {
        self.actor_refs
            .remove(&TypeId::of::<M>())
            .and_then(|boxed| boxed.downcast::<ActorRef<M>>().ok())
            .map(|boxed| *boxed)
    }

    /// Sends a command to the application via the message sink.
    ///
    /// # Errors
    ///
    /// Returns an error if the message sink fails to deliver.
    pub fn send_command(&self, command: Command) -> SendResult {
        self.sink.send_command(command)
    }

    /// Sends an event to the application via the message sink.
    ///
    /// # Errors
    ///
    /// Returns an error if the message sink fails to deliver.
    pub fn send_event(&self, event: Event) -> SendResult {
        self.sink.send_event(event)
    }

    /// Returns the accumulated event subscriptions and command registrations,
    /// clearing them from the context.
    ///
    /// Returns `(event_subscriptions, command_registrations)`.
    /// The host calls this after activation to set up bus routing.
    pub fn take_registrations(&mut self) -> (Vec<EventTypeName>, Vec<CommandName>) {
        let subscriptions = std::mem::take(&mut self.subscriptions);
        let commands = std::mem::take(&mut self.commands);
        (subscriptions, commands)
    }

    /// Announces that this actor has finished starting up.
    ///
    /// Sends `Event::ActorStarted` with the actor's name. Fire-and-forget —
    /// logs a warning on send failure but does not propagate the error.
    pub fn announce_started(&self) {
        if let Err(e) = self.send_event(Event::ActorStarted {
            payload: ActorStarted {
                name: self.name.clone(),
            },
        }) {
            tracing::warn!(name = %self.name, err = ?e, "failed to announce ActorStarted");
        }
    }

    /// Announces that this actor has completed shutdown.
    ///
    /// Sends `Event::ActorShutdownCompleted` with the actor's name. Fire-and-forget —
    /// logs a warning on send failure but does not propagate the error.
    pub fn announce_shutdown_completed(&self) {
        if let Err(e) = self.send_event(Event::ActorShutdownCompleted {
            payload: ActorShutdownCompleted {
                name: self.name.clone(),
            },
        }) {
            tracing::warn!(name = %self.name, err = ?e, "failed to announce ActorShutdownCompleted");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ActorEnvelope;

    fn test_sink() -> Arc<dyn MessageSink> {
        Arc::new(crate::message_sink::TestSink::new())
    }

    fn test_sink_as_concrete() -> Arc<crate::message_sink::TestSink> {
        Arc::new(crate::message_sink::TestSink::new())
    }

    #[test]
    fn subscribe_event_accumulates() {
        // Given a new context.
        let mut ctx = ActorContext::new("test", test_sink());

        // When subscribing to two events.
        ctx.subscribe_event_by_name("system::KeyDown");
        ctx.subscribe_event_by_name("chat_input::ChatEntrySubmitted");

        // Then take_registrations returns both subscriptions.
        let (subscriptions, _) = ctx.take_registrations();
        assert_eq!(
            subscriptions,
            vec!["system::KeyDown", "chat_input::ChatEntrySubmitted"]
        );
    }

    #[test]
    fn subscribe_command_accumulates() {
        // Given a new context.
        let mut ctx = ActorContext::new("test", test_sink());

        // When subscribing to two commands.
        ctx.subscribe_command_by_name("echo");
        ctx.subscribe_command_by_name("reverse");

        // Then take_registrations returns both commands.
        let (_, commands) = ctx.take_registrations();
        assert_eq!(commands, vec!["echo", "reverse"]);
    }

    #[test]
    fn take_registrations_clears() {
        // Given a context with registrations.
        let mut ctx = ActorContext::new("test", test_sink());
        ctx.subscribe_command_by_name("echo");
        ctx.subscribe_event_by_name("system::KeyDown");

        let first = ctx.take_registrations();
        let second = ctx.take_registrations();

        // Then first has data and second is empty.
        assert!(!first.0.is_empty());
        assert!(!first.1.is_empty());
        assert!(second.0.is_empty());
        assert!(second.1.is_empty());
    }

    #[test]
    fn set_and_take_actor_ref() {
        // Given a context with an ActorRef<String> stored.
        let mut ctx = ActorContext::new("test", test_sink());

        let (tx_actor, _) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx_actor);
        ctx.set_actor_ref(actor_ref);

        // When taking the ActorRef<String>.
        let result = ctx.take_actor_ref::<String>();

        // Then it is Some.
        assert!(result.is_some());
    }

    #[test]
    fn take_actor_ref_returns_none_when_empty() {
        // Given a context with no actor refs.
        let mut ctx = ActorContext::new("test", test_sink());

        // When taking an ActorRef<String>.
        let result = ctx.take_actor_ref::<String>();

        // Then it is None.
        assert!(result.is_none());
    }

    #[test]
    fn take_actor_ref_removes_from_context() {
        // Given a context with an ActorRef<String> stored.
        let mut ctx = ActorContext::new("test", test_sink());

        let (tx_actor, _) = kanal::unbounded::<ActorEnvelope<String>>();
        ctx.set_actor_ref(ActorRef::new(tx_actor));

        // When taking it twice.
        let first = ctx.take_actor_ref::<String>();
        let second = ctx.take_actor_ref::<String>();

        // Then first is Some and second is None.
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn send_command_delegates_to_sink() {
        // Given a context with a test sink.
        let sink = test_sink_as_concrete();
        let ctx = ActorContext::new("test", sink.clone());

        // When sending a command.
        ctx.send_command(Command::Quit)
            .expect("send should succeed");

        // Then the sink recorded the command.
        let commands = sink.commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], Command::Quit));
    }

    #[test]
    fn send_event_delegates_to_sink() {
        // Given a context with a test sink.
        let sink = test_sink_as_concrete();
        let ctx = ActorContext::new("test", sink.clone());

        // When sending a KeyDown event.
        ctx.send_event(Event::KeyDown {
            payload: nullslop_protocol::system::KeyDown {
                key: nullslop_protocol::KeyEvent {
                    key: nullslop_protocol::Key::Enter,
                    modifiers: nullslop_protocol::Modifiers::none(),
                },
            },
        })
        .expect("send should succeed");

        // Then the sink recorded the event.
        let events = sink.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::KeyDown { .. }));
    }

    #[test]
    fn name_returns_host_assigned_name() {
        // Given a context with a name.
        let ctx = ActorContext::new("my-actor", test_sink());

        // Then name returns the assigned name.
        assert_eq!(ctx.name(), "my-actor");
    }

    #[test]
    fn announce_started_sends_actor_started_event() {
        // Given a context with a test sink.
        let sink = test_sink_as_concrete();
        let ctx = ActorContext::new("my-actor", sink.clone());

        // When announcing started.
        ctx.announce_started();

        // Then the sink recorded an ActorStarted event with the actor's name.
        let events = sink.events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::ActorStarted { payload } => {
                assert_eq!(payload.name, "my-actor");
            }
            other => panic!("expected ActorStarted, got {other:?}"),
        }
    }

    #[test]
    fn announce_shutdown_completed_sends_actor_shutdown_completed_event() {
        // Given a context with a test sink.
        let sink = test_sink_as_concrete();
        let ctx = ActorContext::new("my-actor", sink.clone());

        // When announcing shutdown completed.
        ctx.announce_shutdown_completed();

        // Then the sink recorded an ActorShutdownCompleted event with the actor's name.
        let events = sink.events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::ActorShutdownCompleted { payload } => {
                    assert_eq!(payload.name, "my-actor");
            }
            other => panic!("expected ActorShutdownCompleted, got {other:?}"),
        }
    }
}
