//! Buffered output for producing new messages during handler execution.
//!
//! Handlers receive an [`Out`] reference to submit commands and events that
//! should be processed after the current dispatch cycle completes. This
//! decouples message production from immediate processing.

use nullslop_protocol::{Command, Event};

/// Buffer for submitting commands and events during handler execution.
///
/// Submitted messages are held until the current dispatch finishes, then
/// queued for a future processing cycle.
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
    use npr::chat_input::InsertChar;
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
        out.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
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
        out.submit_event(Event::ApplicationReady);

        // Then it is not empty and has one event.
        assert!(!out.is_empty());
        assert_eq!(out.events.len(), 1);
    }

    #[test]
    fn drain_commands_takes_and_clears() {
        // Given an Out with a command.
        let mut out = Out::new();
        out.submit_command(Command::Quit);

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
        out.submit_event(Event::ApplicationReady);

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
        out.submit_command(Command::Quit);
        out.submit_event(Event::ApplicationReady);
        out.submit_command(Command::DeleteGrapheme);

        // Then both buffers contain their items.
        assert_eq!(out.commands.len(), 2);
        assert_eq!(out.events.len(), 1);
        assert!(!out.is_empty());
    }
}
