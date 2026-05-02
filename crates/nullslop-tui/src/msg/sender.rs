//! The sending end of the message channel.

use derive_more::Debug;
use kanal::Sender;

use super::Msg;

/// A sender for [`Msg`] values.
///
/// The sending half of the message channel. Since the channel is unbounded,
/// [`Self::send`] never blocks.
#[derive(Debug, Clone)]
pub struct MsgSender {
    /// The underlying kanal sender.
    #[debug(skip)]
    inner: Sender<Msg>,
}

impl MsgSender {
    /// Creates a new message sender wrapping the given channel sender.
    pub(super) fn new(sender: Sender<Msg>) -> Self {
        Self { inner: sender }
    }

    /// Sends a message into the channel.
    ///
    /// Non-blocking (unbounded channel). Discards send errors — if the
    /// receiver has been dropped the message is simply lost.
    pub fn send(&self, msg: Msg) {
        let _ = self.inner.send(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msg_sender_sends_msg() {
        // Given a MsgSender.
        let (tx, rx) = kanal::unbounded();
        let sender = MsgSender::new(tx);

        // When sending an Input event.
        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('x'),
            crossterm::event::KeyModifiers::NONE,
        ));
        sender.send(Msg::Input(event.clone()));

        // Then recv returns that event.
        let msg = rx.recv().expect("should receive");
        assert!(matches!(msg, Msg::Input(e) if e == event));
    }
}
