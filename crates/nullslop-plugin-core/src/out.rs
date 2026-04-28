//! Buffered output for handler submissions.
//!
//! [`Out`] is created by the [`Bus`](crate::Bus) for each command or event being
//! dispatched. Handlers submit new commands and events through it. After all
//! handlers for a given item run, the bus flushes the buffer into its queues,
//! ensuring no re-entrancy within a single dispatch cycle.

use nullslop_protocol::{Command, Event};

/// Buffered output for handlers to submit new commands and events.
///
/// Items submitted during handler execution are buffered internally
/// and flushed to the [`Bus`](crate::Bus) queues after the handler returns.
/// This prevents re-entrancy and ensures consistent state snapshots.
#[derive(Debug, Default)]
pub struct Out {
    commands: Vec<Command>,
    events: Vec<Event>,
}

impl Out {
    /// Create a new empty `Out`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a command to be processed in a future bus cycle.
    pub fn submit_command(&mut self, cmd: Command) {
        self.commands.push(cmd);
    }

    /// Submit an event to be processed in a future bus cycle.
    pub fn submit_event(&mut self, evt: Event) {
        self.events.push(evt);
    }

    /// Take all buffered commands, leaving the buffer empty.
    pub fn drain_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }

    /// Take all buffered events, leaving the buffer empty.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Returns `true` if both command and event buffers are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use npr::command::ChatBoxInsertChar;
    use nullslop_protocol as npr;

    #[test]
    fn new_out_is_empty() {
        // Given a new Out.
        let out = Out::new();

        // Then it is empty.
        assert!(out.is_empty());
    }

    #[test]
    fn submit_command_buffers_item() {
        // Given an empty Out.
        let mut out = Out::new();

        // When submitting a command.
        out.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });

        // Then it is not empty and has one command.
        assert!(!out.is_empty());
        assert_eq!(out.commands.len(), 1);
    }

    #[test]
    fn submit_event_buffers_item() {
        // Given an empty Out.
        let mut out = Out::new();

        // When submitting an event.
        out.submit_event(Event::EventApplicationReady);

        // Then it is not empty and has one event.
        assert!(!out.is_empty());
        assert_eq!(out.events.len(), 1);
    }

    #[test]
    fn drain_commands_takes_and_clears() {
        // Given an Out with a command.
        let mut out = Out::new();
        out.submit_command(Command::AppQuit);

        // When draining commands.
        let cmds = out.drain_commands();

        // Then commands are returned and buffer is empty.
        assert_eq!(cmds.len(), 1);
        assert!(out.is_empty());
    }

    #[test]
    fn drain_events_takes_and_clears() {
        // Given an Out with an event.
        let mut out = Out::new();
        out.submit_event(Event::EventApplicationReady);

        // When draining events.
        let evts = out.drain_events();

        // Then events are returned and buffer is empty.
        assert_eq!(evts.len(), 1);
        assert!(out.is_empty());
    }

    #[test]
    fn drain_on_empty_returns_empty_vec() {
        // Given an empty Out.
        let mut out = Out::new();

        // When draining from empty buffers.
        let cmds = out.drain_commands();
        let evts = out.drain_events();

        // Then both are empty.
        assert!(cmds.is_empty());
        assert!(evts.is_empty());
    }

    #[test]
    fn mixed_commands_and_events() {
        // Given an Out.
        let mut out = Out::new();

        // When submitting both commands and events.
        out.submit_command(Command::AppQuit);
        out.submit_event(Event::EventApplicationReady);
        out.submit_command(Command::ChatBoxDeleteGrapheme);

        // Then both buffers contain their items.
        assert_eq!(out.commands.len(), 2);
        assert_eq!(out.events.len(), 1);
        assert!(!out.is_empty());
    }
}
