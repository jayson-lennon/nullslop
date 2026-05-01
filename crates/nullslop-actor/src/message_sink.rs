//! Trait for sending commands and events from actors to the application.
//!
//! [`MessageSink`] abstracts how an actor's output reaches the rest of the
//! application. The actor crate defines the trait; the application provides
//! the implementation (e.g. one that submits `AppMsg` to `AppCore`'s channel).

use nullslop_protocol::{Command, Event};

use crate::error::SendResult;

/// Trait for sending bus messages from actors to the application.
///
/// Implemented by the application wiring layer (Phase 3). Actors call
/// `send_command`/`send_event` through their [`ActorContext`](crate::ActorContext),
/// which delegates to the underlying `MessageSink`.
pub trait MessageSink: Send + Sync + 'static {
    /// Sends a command to the bus.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be delivered.
    fn send_command(&self, command: Command) -> SendResult;

    /// Sends an event to the bus.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be delivered.
    fn send_event(&self, event: Event) -> SendResult;
}

/// A message sink for testing that records sent commands and events.
///
/// Visible within the crate for use in other modules' tests.
#[cfg(test)]
pub(crate) struct TestSink {
    commands: std::sync::Mutex<Vec<Command>>,
    events: std::sync::Mutex<Vec<Event>>,
}

#[cfg(test)]
impl TestSink {
    /// Creates a new empty test sink.
    pub(crate) fn new() -> Self {
        Self {
            commands: std::sync::Mutex::new(Vec::new()),
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Returns all commands sent through this sink.
    pub(crate) fn commands(&self) -> Vec<Command> {
        self.commands.lock().unwrap().clone()
    }

    /// Returns all events sent through this sink.
    pub(crate) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl MessageSink for TestSink {
    fn send_command(&self, command: Command) -> SendResult {
        self.commands.lock().unwrap().push(command);
        Ok(())
    }

    fn send_event(&self, event: Event) -> SendResult {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

}
