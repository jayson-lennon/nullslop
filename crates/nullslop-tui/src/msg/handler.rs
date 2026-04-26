//! Message channel handler and background event task.
//!
//! [`MsgHandler`] manages the kanal channel, providing synchronous receive
//! for the main loop and an async event task that merges crossterm events
//! and periodic ticks.

use std::time::Duration;

use derive_more::Debug;
use futures::StreamExt;
use kanal::Receiver;

use super::{Msg, MsgSender};

/// Manages the message channel for the TUI event loop.
///
/// Use [`Self::event_task`] to obtain a spawnable future for the background
/// event task, and [`Self::drain`] to discard stale messages after cancelling it.
#[derive(Debug)]
pub struct MsgHandler {
    #[debug(skip)]
    sender: kanal::Sender<Msg>,
    #[debug(skip)]
    receiver: Receiver<Msg>,
}

impl MsgHandler {
    /// Creates a new message handler with an unbounded kanal channel.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = kanal::unbounded();
        Self { sender, receiver }
    }

    /// Returns a clone of the channel sender.
    pub fn sender(&self) -> MsgSender {
        MsgSender::new(self.sender.clone())
    }

    /// Blocks until the next message is available.
    ///
    /// # Errors
    ///
    /// Returns [`kanal::ReceiveError`] if the channel sender has been dropped.
    pub fn recv(&self) -> Result<Msg, kanal::ReceiveError> {
        self.receiver.recv()
    }

    /// Non-blocking receive. Returns `None` if no message is available.
    pub fn try_recv(&self) -> Option<Msg> {
        self.receiver.try_recv().ok().flatten()
    }

    /// Discards all pending messages from the channel.
    pub fn drain(&self) {
        while self.try_recv().is_some() {}
    }

    /// Creates a tokio task that merges crossterm events and periodic ticks.
    ///
    /// The task runs until the tokio runtime is shut down.
    pub fn event_task(&self, handle: &tokio::runtime::Handle) -> tokio::task::JoinHandle<()> {
        let sender = self.sender();
        handle.spawn(run_event_loop(sender))
    }
}

impl Default for MsgHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the background event loop that merges crossterm events and periodic ticks.
///
/// This function runs indefinitely until the tokio runtime is shut down.
/// It sends [`Msg::Tick`] at a fixed interval and [`Msg::Input`] for each
/// crossterm terminal event.
async fn run_event_loop(sender: MsgSender) {
    let mut reader = crossterm::event::EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    loop {
        let crossterm_event = reader.next();
        tokio::select! {
            _ = tick.tick() => {
                sender.send(Msg::Tick);
            }
            Some(Ok(evt)) = crossterm_event => {
                sender.send(Msg::Input(evt));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msg_handler_send_recv() {
        // Given a MsgHandler.
        let handler = MsgHandler::new();

        // When sending a Tick.
        handler.sender().send(Msg::Tick);

        // Then recv returns Tick.
        let msg = handler.recv().expect("should receive");
        assert!(matches!(msg, Msg::Tick));
    }

    #[test]
    fn msg_handler_try_recv_empty() {
        // Given an empty handler.
        let handler = MsgHandler::new();

        // When try_recv.
        let result = handler.try_recv();

        // Then None.
        assert!(result.is_none());
    }

    #[test]
    fn msg_handler_drain() {
        // Given a handler with 3 messages.
        let handler = MsgHandler::new();
        handler.sender().send(Msg::Tick);
        handler.sender().send(Msg::Tick);
        handler.sender().send(Msg::Tick);

        // When draining.
        handler.drain();

        // Then try_recv returns None.
        assert!(handler.try_recv().is_none());
    }
}
