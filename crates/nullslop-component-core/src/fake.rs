//! Test fakes that record handled messages for assertions.
//!
//! [`FakeCommandHandler`] and [`FakeEventHandler`] capture every message they
//! receive so tests can verify dispatch behavior. Each constructor returns the
//! handler together with a shared call log that remains accessible after the
//! handler is registered with the bus.
//!
//! # Usage
//!
//! ```ignore
//! let (handler, calls) = FakeCommandHandler::<Quit, TestState>::continuing();
//! bus.register_command_handler::<Quit, _>(handler);
//! bus.process_commands(&mut state);
//! assert_eq!(calls.borrow().len(), 1);
//! ```

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use nullslop_protocol::CommandAction;

use crate::handler::{CommandHandler, EventHandler, HandlerContext};
use crate::out::Out;

/// Fake command handler that records every command it receives.
///
/// Returns a configurable [`CommandAction`] on each call, allowing tests to
/// exercise both continuation and stop-propagation paths.
pub struct FakeCommandHandler<C, S, Sv> {
    /// Recorded command invocations.
    calls: Rc<RefCell<Vec<C>>>,
    /// The action to return from each handle call.
    action: CommandAction,
    /// Marker for the unused state type parameter.
    _phantom: PhantomData<(S, Sv)>,
}

impl<C, S, Sv> FakeCommandHandler<C, S, Sv>
where
    C: Clone + 'static,
{
    /// Create a new fake that returns the given action.
    ///
    /// Returns a tuple of `(handler, call_log)`. The handler should be registered
    /// with the bus; the call log is kept by the test for assertions.
    pub fn new(action: CommandAction) -> (Self, Rc<RefCell<Vec<C>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let handler = Self {
            calls: Rc::clone(&calls),
            action,
            _phantom: PhantomData,
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

impl<C, S, Sv> CommandHandler<C, S, Sv> for FakeCommandHandler<C, S, Sv>
where
    C: Clone + 'static,
{
    fn handle(&self, cmd: &C, _ctx: &mut HandlerContext<'_, S, Sv>) -> CommandAction {
        self.calls.borrow_mut().push(cmd.clone());
        self.action
    }
}

/// Fake event handler that records every event it receives.
pub struct FakeEventHandler<E, S, Sv> {
    /// Recorded event invocations.
    calls: Rc<RefCell<Vec<E>>>,
    /// Marker for the unused state type parameter.
    _phantom: PhantomData<(S, Sv)>,
}

impl<E, S, Sv> FakeEventHandler<E, S, Sv>
where
    E: Clone + 'static,
{
    /// Create a new fake event handler.
    ///
    /// Returns a tuple of `(handler, call_log)`. The handler should be registered
    /// with the bus; the call log is kept by the test for assertions.
    pub fn new() -> (Self, Rc<RefCell<Vec<E>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let handler = Self {
            calls: Rc::clone(&calls),
            _phantom: PhantomData,
        };
        (handler, calls)
    }
}

impl<E, S, Sv> Default for FakeEventHandler<E, S, Sv>
where
    E: Clone + 'static,
{
    fn default() -> Self {
        Self::new().0
    }
}

impl<E, S, Sv> EventHandler<E, S, Sv> for FakeEventHandler<E, S, Sv>
where
    E: Clone + 'static,
{
    fn handle(&self, evt: &E, _ctx: &mut HandlerContext<'_, S, Sv>) {
        self.calls.borrow_mut().push(evt.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use npr::system::Quit;
    use nullslop_protocol as npr;

    /// Simple state type for testing fake handlers.
    #[derive(Debug, Default)]
    struct TestState;

    #[test]
    fn fake_command_handler_records_call() {
        // Given a continuing fake handler.
        let (handler, calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut state = TestState;
        let services = ();
        let mut out = Out::new();
        let mut ctx = HandlerContext::new(&mut state, &services, &mut out);

        // When handling a command.
        let action = handler.handle(&Quit, &mut ctx);

        // Then the action is Continue and the call was recorded.
        assert_eq!(action, CommandAction::Continue);
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn fake_command_handler_stopping() {
        // Given a stopping fake handler.
        let (handler, calls) = FakeCommandHandler::<Quit, TestState, ()>::stopping();
        let mut state = TestState;
        let services = ();
        let mut out = Out::new();
        let mut ctx = HandlerContext::new(&mut state, &services, &mut out);

        // When handling a command.
        let action = handler.handle(&Quit, &mut ctx);

        // Then the action is Stop.
        assert_eq!(action, CommandAction::Stop);
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn fake_command_handler_shared_call_log() {
        // Given a handler whose call_log is cloned.
        let (handler, calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let calls_clone = Rc::clone(&calls);

        // When moving the handler (simulating bus registration).
        drop(handler);

        // Then the call log is still accessible via the Rc.
        assert!(calls_clone.borrow().is_empty());
    }

    #[test]
    fn fake_event_handler_records_call() {
        // Given a fake event handler for KeyDown.
        use npr::system::KeyDown;
        let (handler, calls) = FakeEventHandler::<KeyDown, TestState, ()>::new();
        let mut state = TestState;
        let services = ();
        let mut out = Out::new();
        let mut ctx = HandlerContext::new(&mut state, &services, &mut out);

        // When handling an event.
        handler.handle(
            &KeyDown {
                key: npr::KeyEvent {
                    key: npr::Key::Enter,
                    modifiers: npr::Modifiers::none(),
                },
            },
            &mut ctx,
        );

        // Then the call was recorded.
        assert_eq!(calls.borrow().len(), 1);
    }
}
