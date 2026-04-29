//! Test fakes for command and event handlers.
//!
//! [`FakeCommandHandler`] and [`FakeEventHandler`] record calls for test assertions.
//! Since handler traits take `&self`, these fakes use [`RefCell`] for interior
//! mutability and [`Rc`] for shared access after the handler is moved into a
//! [`Bus`](crate::Bus).
//!
//! # Usage
//!
//! ```ignore
//! let (handler, calls) = FakeCommandHandler::<AppQuit>::continuing();
//! bus.register_command_handler::<AppQuit, _>(handler);
//! bus.process_commands(&mut state);
//! assert_eq!(calls.borrow().len(), 1);
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use nullslop_protocol::CommandAction;

use crate::handler::{CommandHandler, EventHandler};
use crate::out::Out;
use crate::AppState;

/// Fake command handler that records calls.
///
/// Returns a configurable [`CommandAction`] and records every command it receives.
/// Uses [`Rc<RefCell>`] so the test retains access to the call log after
/// the handler is moved into the bus.
pub struct FakeCommandHandler<C> {
    calls: Rc<RefCell<Vec<C>>>,
    action: CommandAction,
}

impl<C: Clone + 'static> FakeCommandHandler<C> {
    /// Create a new fake that returns the given action.
    ///
    /// Returns a tuple of `(handler, call_log)`. The handler should be registered
    /// with the bus; the call log is kept by the test for assertions.
    pub fn new(action: CommandAction) -> (Self, Rc<RefCell<Vec<C>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let handler = Self {
            calls: Rc::clone(&calls),
            action,
        };
        (handler, calls)
    }

    /// Create a fake that returns [`CommandAction::Continue`].
    pub fn continuing() -> (Self, Rc<RefCell<Vec<C>>>) {
        Self::new(CommandAction::Continue)
    }

    /// Create a fake that returns [`CommandAction::Stop`].
    pub fn stopping() -> (Self, Rc<RefCell<Vec<C>>>) {
        Self::new(CommandAction::Stop)
    }
}

impl<C: Clone + 'static> CommandHandler<C> for FakeCommandHandler<C> {
    fn handle(&self, cmd: &C, _state: &mut AppState, _out: &mut Out) -> CommandAction {
        self.calls.borrow_mut().push(cmd.clone());
        self.action
    }
}

/// Fake event handler that records calls.
///
/// Records every event it receives. Uses [`Rc<RefCell>`] so the test retains
/// access to the call log after the handler is moved into the bus.
pub struct FakeEventHandler<E> {
    calls: Rc<RefCell<Vec<E>>>,
}

impl<E: Clone + 'static> FakeEventHandler<E> {
    /// Create a new fake event handler.
    ///
    /// Returns a tuple of `(handler, call_log)`. The handler should be registered
    /// with the bus; the call log is kept by the test for assertions.
    pub fn new() -> (Self, Rc<RefCell<Vec<E>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let handler = Self {
            calls: Rc::clone(&calls),
        };
        (handler, calls)
    }
}

impl<E: Clone + 'static> Default for FakeEventHandler<E> {
    fn default() -> Self {
        Self::new().0
    }
}

impl<E: Clone + 'static> EventHandler<E> for FakeEventHandler<E> {
    fn handle(&self, evt: &E, _state: &mut AppState, _out: &mut Out) {
        self.calls.borrow_mut().push(evt.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use npr::command::AppQuit;
    use nullslop_protocol as npr;

    #[test]
    fn fake_command_handler_records_call() {
        // Given a continuing fake handler.
        let (handler, calls) = FakeCommandHandler::<AppQuit>::continuing();
        let mut state = AppState::new();
        let mut out = Out::new();

        // When handling a command.
        let action = handler.handle(&AppQuit, &mut state, &mut out);

        // Then the action is Continue and the call was recorded.
        assert_eq!(action, CommandAction::Continue);
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn fake_command_handler_stopping() {
        // Given a stopping fake handler.
        let (handler, calls) = FakeCommandHandler::<AppQuit>::stopping();
        let mut state = AppState::new();
        let mut out = Out::new();

        // When handling a command.
        let action = handler.handle(&AppQuit, &mut state, &mut out);

        // Then the action is Stop.
        assert_eq!(action, CommandAction::Stop);
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn fake_command_handler_shared_call_log() {
        // Given a handler whose call_log is cloned.
        let (handler, calls) = FakeCommandHandler::<AppQuit>::continuing();
        let calls_clone = Rc::clone(&calls);

        // When moving the handler (simulating bus registration).
        drop(handler);

        // Then the call log is still accessible via the Rc.
        assert!(calls_clone.borrow().is_empty());
    }

    #[test]
    fn fake_event_handler_records_call() {
        // Given a fake event handler.
        use npr::event::EventApplicationReady;
        let (handler, calls) = FakeEventHandler::<EventApplicationReady>::new();
        let mut state = AppState::new();
        let mut out = Out::new();

        // When handling an event.
        handler.handle(&EventApplicationReady, &mut state, &mut out);

        // Then the call was recorded.
        assert_eq!(calls.borrow().len(), 1);
    }
}
