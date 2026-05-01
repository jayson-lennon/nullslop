//! Closure-based routing for heterogeneous actor message dispatch.
//!
//! [`RoutingEntry`] wraps a typed actor sender in closures, enabling the host
//! to route events and commands to actors with different message types without
//! generics on the host itself.

use nullslop_protocol::{Command, CommandName, Event, EventTypeName};

/// A routing entry that wraps a typed actor sender in closures.
///
/// Created during [`spawn_actor`](crate::spawn_actor) by capturing a cloned
/// [`ActorRef<M>`](nullslop_actor::ActorRef). Stored in
/// `HashMap<String, Vec<RoutingEntry>>` — no type parameter, enabling
/// heterogeneous collections of actors with different message types.
pub struct RoutingEntry {
    /// The actor's unique name (for source filtering).
    pub name: String,
    /// Event type names this actor subscribed to during activation.
    pub subscriptions: Vec<EventTypeName>,
    /// Command names this actor registered for during activation.
    pub commands: Vec<CommandName>,
    /// Sends an event to this actor (wraps in `ActorEnvelope::Event`).
    pub send_event: Box<dyn Fn(Event) + Send + Sync>,
    /// Sends a command to this actor (wraps in `ActorEnvelope::Command`).
    pub send_command: Box<dyn Fn(Command) + Send + Sync>,
    /// Sends a system message to this actor (wraps in `ActorEnvelope::System`).
    pub send_system: Box<dyn Fn(nullslop_actor::SystemMessage) + Send + Sync>,
    /// Sends a shutdown signal to this actor.
    pub send_shutdown: Box<dyn Fn() + Send + Sync>,
}

#[cfg(test)]
mod tests {
    use nullslop_actor::{ActorEnvelope, ActorRef};

    fn make_actor_ref_and_rx() -> (ActorRef<String>, kanal::Receiver<ActorEnvelope<String>>) {
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<String>>();
        (ActorRef::new(tx), rx)
    }

    #[test]
    fn send_event_closure_wraps_and_delivers() {
        // Given a RoutingEntry built from an ActorRef<String>.
        let (actor_ref, rx) = make_actor_ref_and_rx();
        let ref_clone = actor_ref.clone();
        let entry = super::RoutingEntry {
            name: "test".to_string(),
            subscriptions: vec![],
            commands: vec![],
            send_event: Box::new(move |event| {
                let _ = ref_clone.send_event(event);
            }),
            send_command: Box::new(|_| {}),
            send_system: Box::new(|_| {}),
            send_shutdown: Box::new(|| {}),
        };

        // When calling send_event with a ModeChanged event.
        (entry.send_event)(nullslop_protocol::Event::ModeChanged {
            payload: nullslop_protocol::system::ModeChanged {
                from: nullslop_protocol::Mode::Normal,
                to: nullslop_protocol::Mode::Input,
            },
        });

        // Then it is received as an Event envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            msg,
            ActorEnvelope::Event(nullslop_protocol::Event::ModeChanged { .. })
        ));
    }

    #[test]
    fn send_command_closure_wraps_and_delivers() {
        // Given a RoutingEntry built from an ActorRef<String>.
        let (actor_ref, rx) = make_actor_ref_and_rx();
        let ref_clone = actor_ref.clone();
        let entry = super::RoutingEntry {
            name: "test".to_string(),
            subscriptions: vec![],
            commands: vec![],
            send_event: Box::new(|_| {}),
            send_command: Box::new(move |command| {
                let _ = ref_clone.send_command(command);
            }),
            send_system: Box::new(|_| {}),
            send_shutdown: Box::new(|| {}),
        };

        // When calling send_command.
        (entry.send_command)(nullslop_protocol::Command::Quit);

        // Then it is received as a Command envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            msg,
            ActorEnvelope::Command(nullslop_protocol::Command::Quit)
        ));
    }

    #[test]
    fn send_shutdown_closure_sends_shutdown() {
        // Given a RoutingEntry built from an ActorRef<String>.
        let (actor_ref, rx) = make_actor_ref_and_rx();
        let ref_clone = actor_ref.clone();
        let entry = super::RoutingEntry {
            name: "test".to_string(),
            subscriptions: vec![],
            commands: vec![],
            send_event: Box::new(|_| {}),
            send_command: Box::new(|_| {}),
            send_system: Box::new(|_| {}),
            send_shutdown: Box::new(move || {
                let _ = ref_clone.shutdown();
            }),
        };

        // When calling send_shutdown.
        (entry.send_shutdown)();

        // Then a Shutdown envelope is received.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Shutdown));
    }

    #[test]
    fn send_system_closure_wraps_and_delivers() {
        // Given a RoutingEntry built from an ActorRef<String>.
        let (actor_ref, rx) = make_actor_ref_and_rx();
        let ref_clone = actor_ref.clone();
        let entry = super::RoutingEntry {
            name: "test".to_string(),
            subscriptions: vec![],
            commands: vec![],
            send_event: Box::new(|_| {}),
            send_command: Box::new(|_| {}),
            send_system: Box::new(move |msg| {
                let _ = ref_clone.send_system(msg);
            }),
            send_shutdown: Box::new(|| {}),
        };

        // When calling send_system with ApplicationReady.
        (entry.send_system)(nullslop_actor::SystemMessage::ApplicationReady);

        // Then it is received as a System envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            msg,
            ActorEnvelope::System(nullslop_actor::SystemMessage::ApplicationReady)
        ));
    }
}
