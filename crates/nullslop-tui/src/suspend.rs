//! Temporary suspension of the TUI to run external terminal applications.
//!
//! When the TUI needs to spawn an external program that takes over the
//! terminal (such as `$EDITOR`), it must first exit raw mode and leave the
//! alternate screen. The external program runs with full terminal control,
//! and the TUI restores its state once the program exits.
//!
//! [`Suspend`] holds a deferred [`SuspendAction`] that the event loop
//! consumes to initiate the suspend/restore cycle.

use derive_more::Debug;

/// An action requesting the TUI temporarily suspend itself.
///
/// The event loop consumes this to exit raw mode, leave the alternate screen,
/// spawn the requested external process, and restore the TUI on completion.
///
/// The `on_result` closure maps the editor's output to the new input buffer
/// content (or `None` if no change).
#[derive(Debug)]
pub enum SuspendAction {
    /// Open `$EDITOR` with the given initial content.
    Edit {
        /// The text to pre-fill in the editor.
        initial_content: String,
        /// Maps the edited content (if changed) to the new buffer content.
        /// Receives `Some(content)` if the user made changes, `None` otherwise.
        #[debug("<closure>")]
        on_result: Box<dyn FnOnce(Option<String>) -> Option<String>>,
    },
}

/// Holds an optional deferred suspend action to be consumed by the event loop.
#[derive(Debug, Default)]
pub struct Suspend {
    /// The pending suspend action, if any.
    action: Option<SuspendAction>,
}

impl Suspend {
    /// Creates a new [`Suspend`] with no pending action.
    #[must_use]
    pub const fn new() -> Self {
        Self { action: None }
    }

    /// Takes and returns the pending action (if any), clearing it.
    pub fn take_action(&mut self) -> Option<SuspendAction> {
        self.action.take()
    }

    /// Queues a suspend action to be consumed by the event loop.
    ///
    /// If an action is already pending, it is replaced.
    pub fn request(&mut self, action: SuspendAction) {
        self.action = Some(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspend_new_has_no_action() {
        // Given a new Suspend.
        let mut suspend = Suspend::new();

        // When calling take_action.
        let result = suspend.take_action();

        // Then it returns None.
        assert!(result.is_none());
    }

    #[test]
    fn suspend_request_then_take() {
        // Given a Suspend.
        let mut suspend = Suspend::new();
        let action = SuspendAction::Edit {
            initial_content: "hello".to_owned(),
            on_result: Box::new(|_| None),
        };

        // When requesting an action.
        suspend.request(action);

        // Then take_action returns Some.
        let result = suspend.take_action();
        assert!(result.is_some());
    }

    #[test]
    fn suspend_take_action_clears() {
        // Given a Suspend with a pending action.
        let mut suspend = Suspend::new();
        suspend.request(SuspendAction::Edit {
            initial_content: "hello".to_owned(),
            on_result: Box::new(|_| None),
        });

        // When calling take_action twice.
        let first = suspend.take_action();
        let second = suspend.take_action();

        // Then first returns Some and second returns None.
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn suspend_request_replaces() {
        // Given a Suspend with a pending action.
        let mut suspend = Suspend::new();
        suspend.request(SuspendAction::Edit {
            initial_content: "first".to_owned(),
            on_result: Box::new(|_| None),
        });

        // When requesting a new action.
        suspend.request(SuspendAction::Edit {
            initial_content: "second".to_owned(),
            on_result: Box::new(|_| None),
        });

        // Then take_action returns the new action.
        let action = suspend.take_action().expect("should have action");
        match action {
            SuspendAction::Edit {
                initial_content, ..
            } => {
                assert_eq!(initial_content, "second");
            }
        }
    }
}
