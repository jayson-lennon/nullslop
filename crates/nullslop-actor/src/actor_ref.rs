//! Typed, cloneable handle for sending messages to an actor.
//!
//! [`ActorRef<M>`] wraps a shared [`ActorCell<M>`] containing a
//! [`parking_lot::RwLock`] around a [`kanal::Sender`]. The swappable sender
//! enables actor restart without breaking peer references — all holders of the
//! same `ActorRef` see the new channel after a swap.

use std::sync::Arc;

use error_stack::Report;
use kanal::{SendError, Sender};
use parking_lot::RwLock;
use wherror::Error;

use crate::envelope::ActorEnvelope;

/// Error returned when a message cannot be delivered to an actor.
///
/// This wraps the underlying channel failure (e.g. the actor's receiver
/// was dropped) in a domain error that does not expose internal types.
#[derive(Debug, Error)]
#[error(debug)]
pub struct ActorSendError;

/// Result alias for actor send operations.
pub type SendResult = Result<(), Report<ActorSendError>>;

/// Converts a kanal send failure into a [`SendResult`].
///
/// Attaches a human-readable message to the report for diagnostics.
pub(crate) fn from_kanal_send(
    result: Result<(), SendError>,
    context: &'static str,
) -> SendResult {
    result.map_err(|err| {
        Report::new(ActorSendError)
            .attach(context)
            .attach(err.to_string())
    })
}

/// Shared inner state for an actor's send handle.
///
/// The [`RwLock`] allows the sender to be swapped during actor restart.
/// All holders of the same [`ActorRef`] share this cell via [`Arc`].
struct ActorCell<M> {
    /// The underlying channel sender, wrapped for restart swaps.
    sender: RwLock<Sender<ActorEnvelope<M>>>,
}

/// A typed, cloneable handle for sending messages to an actor.
///
/// `M` is the actor's direct message type (e.g. `LlmPipeDirectMsg`),
/// not the actor type itself. This decouples the sender from the actor's
/// concrete implementation.
///
/// Cheaply cloneable — clones the inner [`Arc`].
pub struct ActorRef<M: Send + 'static> {
    /// Shared inner cell containing the swappable sender.
    cell: Arc<ActorCell<M>>,
}

impl<M: Send + 'static> ActorRef<M> {
    /// Creates a new `ActorRef` wrapping the given sender.
    #[must_use]
    pub fn new(sender: Sender<ActorEnvelope<M>>) -> Self {
        Self {
            cell: Arc::new(ActorCell {
                sender: RwLock::new(sender),
            }),
        }
    }

    /// Sends a direct typed message to the actor.
    ///
    /// The message is wrapped in [`ActorEnvelope::Direct`] before sending.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed.
    pub fn send(&self, msg: M) -> SendResult {
        from_kanal_send(
            self.cell.sender.read().send(ActorEnvelope::Direct(msg)),
            "failed to send direct message to actor",
        )
    }

    /// Sends a bus event to the actor.
    ///
    /// The event is wrapped in [`ActorEnvelope::Event`] before sending.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed.
    pub fn send_event(&self, event: nullslop_protocol::Event) -> SendResult {
        from_kanal_send(
            self.cell.sender.read().send(ActorEnvelope::Event(event)),
            "failed to send event to actor",
        )
    }

    /// Sends a bus command to the actor.
    ///
    /// The command is wrapped in [`ActorEnvelope::Command`] before sending.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed.
    pub fn send_command(&self, command: nullslop_protocol::Command) -> SendResult {
        from_kanal_send(
            self.cell
                .sender
                .read()
                .send(ActorEnvelope::Command(command)),
            "failed to send command to actor",
        )
    }

    /// Sends a system message to the actor.
    ///
    /// The message is wrapped in [`ActorEnvelope::System`] before sending.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed.
    pub fn send_system(&self, msg: crate::SystemMessage) -> SendResult {
        from_kanal_send(
            self.cell.sender.read().send(ActorEnvelope::System(msg)),
            "failed to send system message to actor",
        )
    }

    /// Atomically replaces the inner sender, returning the old one.
    ///
    /// Used during actor restart to redirect all messages to a new channel
    /// without breaking existing peer references.
    pub fn swap_sender(
        &self,
        new_sender: Sender<ActorEnvelope<M>>,
    ) -> Sender<ActorEnvelope<M>> {
        let mut guard = self.cell.sender.write();
        std::mem::replace(&mut *guard, new_sender)
    }

    /// Sends a shutdown signal to the actor.
    ///
    /// Sends [`ActorEnvelope::Shutdown`].
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed.
    pub fn shutdown(&self) -> SendResult {
        from_kanal_send(
            self.cell.sender.read().send(ActorEnvelope::Shutdown),
            "failed to send shutdown signal to actor",
        )
    }
}

impl<M: Send + 'static> Clone for ActorRef<M> {
    fn clone(&self) -> Self {
        Self {
            cell: Arc::clone(&self.cell),
        }
    }
}

impl<M: Send + 'static> std::fmt::Debug for ActorRef<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorRef").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::{Command, Event};

    #[test]
    fn send_delivers_direct_message() {
        // Given an ActorRef with an unbounded channel.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx);

        // When sending a direct message.
        actor_ref
            .send("hello".to_owned())
            .expect("send should succeed");

        // Then it is received as a Direct envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Direct(ref s) if s == "hello"));
    }

    #[test]
    fn send_event_delivers_event() {
        // Given an ActorRef with an unbounded channel.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<()>>();
        let actor_ref = ActorRef::new(tx);

        // When sending an event.
        actor_ref
            .send_event(Event::KeyDown {
                payload: nullslop_protocol::system::KeyDown {
                    key: nullslop_protocol::KeyEvent {
                        key: nullslop_protocol::Key::Enter,
                        modifiers: nullslop_protocol::Modifiers::none(),
                    },
                },
            })
            .expect("send should succeed");

        // Then it is received as an Event envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Event(Event::KeyDown { .. })));
    }

    #[test]
    fn send_command_delivers_command() {
        // Given an ActorRef with an unbounded channel.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<()>>();
        let actor_ref = ActorRef::new(tx);

        // When sending a command.
        actor_ref
            .send_command(Command::Quit)
            .expect("send should succeed");

        // Then it is received as a Command envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Command(Command::Quit)));
    }

    #[test]
    fn send_system_delivers_system_message() {
        // Given an ActorRef with an unbounded channel.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<()>>();
        let actor_ref = ActorRef::new(tx);

        // When sending a system message.
        actor_ref
            .send_system(crate::SystemMessage::ApplicationReady)
            .expect("send should succeed");

        // Then it is received as a System envelope.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            msg,
            ActorEnvelope::System(crate::SystemMessage::ApplicationReady)
        ));
    }

    #[test]
    fn shutdown_sends_shutdown_envelope() {
        // Given an ActorRef with an unbounded channel.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<()>>();
        let actor_ref = ActorRef::new(tx);

        // When calling shutdown.
        actor_ref.shutdown().expect("shutdown should succeed");

        // Then a Shutdown envelope is received.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Shutdown));
    }

    #[test]
    fn swap_sender_replaces_channel() {
        // Given an ActorRef wired to channel A.
        let (tx_a, rx_a) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx_a);

        // When swapping to channel B.
        let (tx_b, rx_b) = kanal::unbounded::<ActorEnvelope<String>>();
        let _old = actor_ref.swap_sender(tx_b);

        // And sending a message.
        actor_ref
            .send("after-swap".to_owned())
            .expect("send should succeed");

        // Then the message arrives on channel B, not A.
        let msg = rx_b
            .try_recv()
            .expect("recv on B should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Direct(ref s) if s == "after-swap"));
        assert!(rx_a.try_recv().expect("recv should succeed").is_none());
    }

    #[test]
    fn clone_shares_cell() {
        // Given an ActorRef.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx);

        // When cloning and sending on the clone.
        let clone = actor_ref.clone();
        clone
            .send("from-clone".to_owned())
            .expect("send should succeed");

        // Then the original's channel receives it.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Direct(ref s) if s == "from-clone"));
    }

    #[test]
    fn swap_sender_affects_all_clones() {
        // Given an ActorRef and its clone.
        let (tx_a, _) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx_a);
        let clone = actor_ref.clone();

        // When swapping the sender on the original.
        let (tx_b, rx_b) = kanal::unbounded::<ActorEnvelope<String>>();
        actor_ref.swap_sender(tx_b);

        // And sending on the clone.
        clone
            .send("via-clone".to_owned())
            .expect("send should succeed");

        // Then the message arrives on the new channel.
        let msg = rx_b
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, ActorEnvelope::Direct(ref s) if s == "via-clone"));
    }

    #[test]
    fn send_returns_error_when_channel_closed() {
        // Given an ActorRef with a dropped receiver.
        let (tx, rx) = kanal::unbounded::<ActorEnvelope<String>>();
        let actor_ref = ActorRef::new(tx);
        drop(rx);

        // When sending a message.
        let result = actor_ref.send("should-fail".to_owned());

        // Then it returns an error.
        assert!(result.is_err());
    }
}
