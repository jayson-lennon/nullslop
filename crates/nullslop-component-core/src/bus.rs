//! Central message router for commands and events.
//!
//! The [`Bus`] accepts handler registrations for specific message types, then
//! routes submitted commands and events to the matching handlers.
//!
//! # Processing model
//!
//! - [`process_commands`](Bus::process_commands) drains the command queue and
//!   dispatches each command to its registered handlers. If handlers submit new
//!   commands via [`Out`](crate::Out), those are processed in subsequent iterations
//!   (with a configurable [`max_iterations`](Bus::with_max_iterations) guard).
//! - [`process_events`](Bus::process_events) drains the event queue in a single
//!   pass. All handlers for each event always run (no interception).
//!
//! # Consistency
//!
//! Each command or event receives a fresh [`Out`](crate::Out) buffer. New messages
//! submitted by handlers are only queued after all handlers for the current item
//! have finished, ensuring a consistent state snapshot per dispatch.

/// A processed event ready for forwarding, with its source actor.
pub struct ProcessedEvent {
    /// The dispatched event.
    pub event: Event,
    /// The actor that originated this event, if any.
    pub source: Option<ActorName>,
}

/// A processed command ready for forwarding, with its source actor.
pub struct ProcessedCommand {
    /// The dispatched command.
    pub command: Command,
    /// The actor that originated this command, if any.
    pub source: Option<ActorName>,
}

use std::any::{Any, TypeId};
use std::collections::HashMap;

use nullslop_protocol::chat_input::{
    Clear, DeleteGrapheme, DeleteGraphemeForward, MoveCursorLeft, MoveCursorRight, MoveCursorToEnd,
    MoveCursorToStart, MoveCursorWordLeft, MoveCursorWordRight,
};
use nullslop_protocol::provider::CancelStream;
use nullslop_protocol::system::{EditInput, Quit, ToggleWhichKey};
use nullslop_protocol::{ActorName, Command, CommandAction, Event};

use crate::handler::{CommandHandler, EventHandler};
use crate::out::Out;

/// Type-erased command handler ready for dispatch.
struct AnyCommandHandler<S> {
    handler: Box<dyn Any>,
    invoke: fn(&dyn Any, &dyn Any, &mut S, &mut Out) -> CommandAction,
}

/// Type-erased event handler ready for dispatch.
struct AnyEventHandler<S> {
    handler: Box<dyn Any>,
    invoke: fn(&dyn Any, &dyn Any, &mut S, &mut Out),
}

/// Invokes a command handler with its concrete types.
fn invoke_command<C, H, S>(
    handler: &dyn Any,
    cmd: &dyn Any,
    state: &mut S,
    out: &mut Out,
) -> CommandAction
where
    H: CommandHandler<C, S> + 'static,
    C: 'static,
{
    let h = handler.downcast_ref::<H>().expect("handler type mismatch");
    let c = cmd.downcast_ref::<C>().expect("command type mismatch");
    h.handle(c, state, out)
}

/// Invokes an event handler with its concrete types.
fn invoke_event<E, H, S>(handler: &dyn Any, evt: &dyn Any, state: &mut S, out: &mut Out)
where
    H: EventHandler<E, S> + 'static,
    E: 'static,
{
    let h = handler.downcast_ref::<H>().expect("handler type mismatch");
    let e = evt.downcast_ref::<E>().expect("event type mismatch");
    h.handle(e, state, out);
}

/// A queued command together with its origin.
struct QueuedCommand {
    command: Command,
    source: Option<ActorName>,
}

/// A queued event together with its origin.
struct QueuedEvent {
    event: Event,
    source: Option<ActorName>,
}

/// Central message router that dispatches commands and events to registered handlers.
///
/// Commands and events are submitted to queues and processed in order. Each
/// message is routed to every handler registered for its type. The processing
/// model ensures consistent state snapshots across handlers.
pub struct Bus<S> {
    command_handlers: HashMap<TypeId, Vec<AnyCommandHandler<S>>>,
    event_handlers: HashMap<TypeId, Vec<AnyEventHandler<S>>>,
    command_queue: Vec<QueuedCommand>,
    event_queue: Vec<QueuedEvent>,
    /// Events dispatched during the last processing cycle, with source.
    /// Available via [`drain_processed_events`](Self::drain_processed_events).
    processed_events: Vec<ProcessedEvent>,
    /// Commands dispatched during the last processing cycle, with source.
    /// Available via [`drain_processed_commands`](Self::drain_processed_commands).
    processed_commands: Vec<ProcessedCommand>,
    max_iterations: usize,
}

impl<S> Bus<S> {
    /// Create a new bus with default settings.
    ///
    /// The default `max_iterations` is 100, which prevents infinite loops
    /// from misbehaving handlers that resubmit their own command type.
    #[must_use]
    pub fn new() -> Self {
        Self {
            command_handlers: HashMap::new(),
            event_handlers: HashMap::new(),
            command_queue: Vec::new(),
            event_queue: Vec::new(),
            processed_events: Vec::new(),
            processed_commands: Vec::new(),
            max_iterations: 100,
        }
    }

    /// Set the maximum number of processing iterations for [`process_commands`](Self::process_commands).
    ///
    /// Prevents infinite loops when handlers resubmit commands during processing.
    /// The default is 100.
    #[must_use]
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Register a handler for a specific command type `C`.
    ///
    /// Multiple handlers can be registered for the same command type.
    /// They are called in registration order. The first handler to return
    /// [`CommandAction::Stop`] halts propagation.
    pub fn register_command_handler<C, H>(&mut self, handler: H)
    where
        C: 'static,
        H: CommandHandler<C, S> + 'static,
    {
        let type_id = TypeId::of::<C>();
        let invoke = invoke_command::<C, H, S>;
        let entry = AnyCommandHandler {
            handler: Box::new(handler),
            invoke,
        };
        self.command_handlers
            .entry(type_id)
            .or_default()
            .push(entry);
    }

    /// Register a handler for a specific event type `E`.
    ///
    /// Multiple handlers can be registered for the same event type.
    /// All handlers always run — events have no interception.
    pub fn register_event_handler<E, H>(&mut self, handler: H)
    where
        E: 'static,
        H: EventHandler<E, S> + 'static,
    {
        let type_id = TypeId::of::<E>();
        let invoke = invoke_event::<E, H, S>;
        let entry = AnyEventHandler {
            handler: Box::new(handler),
            invoke,
        };
        self.event_handlers.entry(type_id).or_default().push(entry);
    }

    /// Submit a command to the bus queue.
    ///
    /// The command will be dispatched when [`process_commands`](Self::process_commands) is called.
    /// The source is `None` (originated from the user or host, not an actor).
    pub fn submit_command(&mut self, cmd: Command) {
        self.submit_command_from(cmd, None);
    }

    /// Submit a command to the bus queue with an optional source actor name.
    ///
    /// The command will be dispatched when [`process_commands`](Self::process_commands) is called.
    pub fn submit_command_from(&mut self, cmd: Command, source: Option<ActorName>) {
        self.command_queue.push(QueuedCommand {
            command: cmd,
            source,
        });
    }

    /// Submit an event to the bus queue.
    ///
    /// The event will be dispatched when [`process_events`](Self::process_events) is called.
    /// The source is `None` (originated from the user or host, not an actor).
    pub fn submit_event(&mut self, evt: Event) {
        self.submit_event_from(evt, None);
    }

    /// Submit an event to the bus queue with an optional source actor name.
    ///
    /// The event will be dispatched when [`process_events`](Self::process_events) is called.
    pub fn submit_event_from(&mut self, evt: Event, source: Option<ActorName>) {
        self.event_queue.push(QueuedEvent { event: evt, source });
    }

    /// Process all pending commands, including those submitted by handlers.
    ///
    /// Drains the command queue, dispatches each command to its registered
    /// handlers, and repeats if handlers submitted new commands. Stops when
    /// the queue is empty or `max_iterations` is reached.
    pub fn process_commands(&mut self, state: &mut S) {
        let mut iterations = 0;
        loop {
            let commands = std::mem::take(&mut self.command_queue);
            if commands.is_empty() {
                break;
            }
            iterations += 1;
            if iterations > self.max_iterations {
                break;
            }
            for queued in commands {
                self.dispatch_command(queued.command, queued.source, state);
            }
        }
    }

    /// Process all pending events in a single pass.
    ///
    /// Drains the event queue and dispatches each event to its registered
    /// handlers. All handlers always run. Events submitted by handlers during
    /// processing are queued for a future call.
    pub fn process_events(&mut self, state: &mut S) {
        let events = std::mem::take(&mut self.event_queue);
        for queued in events {
            self.dispatch_event(queued.event, queued.source, state);
        }
    }

    /// Returns `true` if there are pending commands or events in the queues.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.command_queue.is_empty() || !self.event_queue.is_empty()
    }

    /// Drain all events that were dispatched during processing.
    ///
    /// Returns tuples of `(event, source)` and clears the internal buffer.
    /// Useful for forwarding processed events to external systems
    /// (e.g., actor host) after bus processing completes.
    pub fn drain_processed_events(&mut self) -> Vec<ProcessedEvent> {
        std::mem::take(&mut self.processed_events)
    }

    /// Drain all commands that were dispatched during processing.
    ///
    /// Returns tuples of `(command, source)` and clears the internal buffer.
    /// Useful for forwarding processed commands to external systems
    /// (e.g., actor host) after bus processing.
    pub fn drain_processed_commands(&mut self) -> Vec<ProcessedCommand> {
        std::mem::take(&mut self.processed_commands)
    }

    /// Drain all processed events and commands.
    ///
    /// Convenience method that returns both
    /// [`drain_processed_events`](Self::drain_processed_events) and
    /// [`drain_processed_commands`](Self::drain_processed_commands) as a tuple.
    pub fn drain_all(&mut self) -> (Vec<ProcessedEvent>, Vec<ProcessedCommand>) {
        let events = self.drain_processed_events();
        let commands = self.drain_processed_commands();
        (events, commands)
    }

    /// Dispatch a single command to its registered handlers.
    fn dispatch_command(&mut self, cmd: Command, source: Option<ActorName>, state: &mut S) {
        // Record the command before dispatching so consumers can drain it later.
        self.processed_commands.push(ProcessedCommand {
            command: cmd.clone(),
            source,
        });
        let mut out = Out::new();
        match cmd {
            Command::InsertChar { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
            Command::DeleteGrapheme => {
                let cmd = DeleteGrapheme;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::SubmitMessage { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
            Command::Clear => {
                let cmd = Clear;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::MoveCursorLeft => {
                let cmd = MoveCursorLeft;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::MoveCursorRight => {
                let cmd = MoveCursorRight;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::MoveCursorToStart => {
                let cmd = MoveCursorToStart;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::MoveCursorToEnd => {
                let cmd = MoveCursorToEnd;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::DeleteGraphemeForward => {
                let cmd = DeleteGraphemeForward;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::MoveCursorWordLeft => {
                let cmd = MoveCursorWordLeft;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::MoveCursorWordRight => {
                let cmd = MoveCursorWordRight;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::SetMode { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
            Command::Quit => {
                let cmd = Quit;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::EditInput => {
                let cmd = EditInput;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::ToggleWhichKey => {
                let cmd = ToggleWhichKey;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::SwitchTab { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
            Command::SendMessage { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
            Command::CancelStream => {
                let cmd = CancelStream;
                self.dispatch_command_to_handlers(&cmd, state, &mut out);
            }
            Command::PushChatEntry { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
            Command::ProceedWithShutdown { payload } => {
                self.dispatch_command_to_handlers(&payload, state, &mut out);
            }
        }
        self.flush_out(out);
    }

    /// Look up and invoke handlers for a concrete command type `C`.
    fn dispatch_command_to_handlers<C: 'static>(&self, cmd: &C, state: &mut S, out: &mut Out) {
        let type_id = TypeId::of::<C>();
        if let Some(handlers) = self.command_handlers.get(&type_id) {
            for h in handlers {
                let action = (h.invoke)(&*h.handler, cmd as &dyn Any, state, out);
                if action == CommandAction::Stop {
                    break;
                }
            }
        }
    }

    /// Dispatch a single event to its registered handlers.
    fn dispatch_event(&mut self, evt: Event, source: Option<ActorName>, state: &mut S) {
        // Record the event before dispatching so consumers can drain it later.
        self.processed_events.push(ProcessedEvent {
            event: evt.clone(),
            source,
        });
        let mut out = Out::new();
        match evt {
            Event::KeyDown { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
            Event::KeyUp { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
            Event::ChatEntrySubmitted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
            Event::ModeChanged { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
            Event::ActorStarting { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
            Event::ActorStarted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
            Event::ActorShutdownCompleted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, &mut out);
            }
        }
        self.flush_out(out);
    }

    /// Look up and invoke handlers for a concrete event type `E`.
    fn dispatch_event_to_handlers<E: 'static>(&self, evt: &E, state: &mut S, out: &mut Out) {
        let type_id = TypeId::of::<E>();
        if let Some(handlers) = self.event_handlers.get(&type_id) {
            for h in handlers {
                (h.invoke)(&*h.handler, evt as &dyn Any, state, out);
            }
        }
    }

    /// Flush buffered output from a handler into the bus queues.
    fn flush_out(&mut self, mut out: Out) {
        for cmd in out.drain_commands() {
            self.command_queue.push(QueuedCommand {
                command: cmd,
                source: None,
            });
        }
        for evt in out.drain_events() {
            self.event_queue.push(QueuedEvent {
                event: evt,
                source: None,
            });
        }
    }
}

impl<S> Default for Bus<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeCommandHandler, FakeEventHandler};
    use npr::chat_input::{DeleteGrapheme, InsertChar};
    use npr::provider::SendMessage;
    use npr::system::{KeyDown, ModeChanged, Quit, SetMode};
    use nullslop_protocol as npr;

    /// Simple state type for testing bus dispatch.
    #[derive(Debug, Default)]
    struct TestState;

    // --- Command dispatch tests ---

    #[test]
    fn command_dispatch_reaches_handler() {
        // Given a bus with a handler for InsertChar.
        let (handler, calls) = FakeCommandHandler::<InsertChar, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<InsertChar, _>(handler);

        // When submitting and processing the command.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the handler was called with the correct payload.
        assert_eq!(calls.borrow().len(), 1);
        assert_eq!(calls.borrow()[0].ch, 'x');
    }

    #[test]
    fn multiple_command_handlers_all_run() {
        // Given a bus with two handlers for the same command type.
        let (h1, calls1) = FakeCommandHandler::<Quit, TestState>::continuing();
        let (h2, calls2) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(h1);
        bus.register_command_handler::<Quit, _>(h2);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then both handlers were called.
        assert_eq!(calls1.borrow().len(), 1);
        assert_eq!(calls2.borrow().len(), 1);
    }

    #[test]
    fn stop_halts_propagation() {
        // Given a bus where the first handler returns Stop.
        let (stopper, stopper_calls) = FakeCommandHandler::<Quit, TestState>::stopping();
        let (continuer, continuer_calls) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(stopper);
        bus.register_command_handler::<Quit, _>(continuer);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then only the first handler was called.
        assert_eq!(stopper_calls.borrow().len(), 1);
        assert!(continuer_calls.borrow().is_empty());
    }

    #[test]
    fn continue_allows_propagation() {
        // Given a bus where the first handler returns Continue.
        let (c1, calls1) = FakeCommandHandler::<Quit, TestState>::continuing();
        let (c2, calls2) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(c1);
        bus.register_command_handler::<Quit, _>(c2);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then both handlers were called.
        assert_eq!(calls1.borrow().len(), 1);
        assert_eq!(calls2.borrow().len(), 1);
    }

    #[test]
    fn unregistered_command_is_ignored() {
        // Given a bus with no handlers.
        let mut bus: Bus<TestState> = Bus::new();

        // When submitting a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then no panic occurs and the queue is empty.
        assert!(!bus.has_pending());
    }

    #[test]
    fn unit_command_dispatches_correctly() {
        // Given a bus with a handler for DeleteGrapheme (unit struct).
        let (handler, calls) = FakeCommandHandler::<DeleteGrapheme, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<DeleteGrapheme, _>(handler);

        // When processing a unit command.
        bus.submit_command(Command::DeleteGrapheme);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the handler was called.
        assert_eq!(calls.borrow().len(), 1);
    }

    // --- Event dispatch tests ---

    #[test]
    fn event_dispatch_reaches_handler() {
        // Given a bus with a handler for KeyDown.
        let (handler, calls) = FakeEventHandler::<KeyDown, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_event_handler::<KeyDown, _>(handler);

        // When processing an event.
        let key = npr::KeyEvent {
            key: npr::Key::Char('a'),
            modifiers: npr::Modifiers::none(),
        };
        bus.submit_event(Event::KeyDown {
            payload: KeyDown { key },
        });
        let mut state = TestState;
        bus.process_events(&mut state);

        // Then the handler was called.
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn unit_event_dispatches_correctly() {
        // Given a bus with a handler for KeyDown (struct with payload).
        let (handler, calls) = FakeEventHandler::<KeyDown, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_event_handler::<KeyDown, _>(handler);

        // When processing a KeyDown event.
        bus.submit_event(Event::KeyDown {
            payload: KeyDown {
                key: npr::KeyEvent {
                    key: npr::Key::Char('a'),
                    modifiers: npr::Modifiers::none(),
                },
            },
        });
        let mut state = TestState;
        bus.process_events(&mut state);

        // Then the handler was called.
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn all_event_handlers_run() {
        // Given a bus with two event handlers for ModeChanged.
        let (h1, calls1) = FakeEventHandler::<ModeChanged, TestState>::new();
        let (h2, calls2) = FakeEventHandler::<ModeChanged, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_event_handler::<ModeChanged, _>(h1);
        bus.register_event_handler::<ModeChanged, _>(h2);

        // When processing an event.
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        let mut state = TestState;
        bus.process_events(&mut state);

        // Then both handlers were called.
        assert_eq!(calls1.borrow().len(), 1);
        assert_eq!(calls2.borrow().len(), 1);
    }

    // --- Out / cascading tests ---

    /// Handler that submits an `AppQuit` command when it sees `InsertChar`.
    struct CascadeHandler;

    impl CommandHandler<InsertChar, TestState> for CascadeHandler {
        fn handle(
            &self,
            _cmd: &InsertChar,
            _state: &mut TestState,
            out: &mut Out,
        ) -> CommandAction {
            out.submit_command(Command::Quit);
            CommandAction::Continue
        }
    }

    #[test]
    fn cascading_commands_are_processed() {
        // Given a bus where InsertChar handler submits AppQuit.
        let (quit_handler, quit_calls) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<InsertChar, _>(CascadeHandler);
        bus.register_command_handler::<Quit, _>(quit_handler);

        // When processing the initial command.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the cascaded Quit was also processed.
        assert_eq!(quit_calls.borrow().len(), 1);
    }

    /// Handler that resubmits itself, creating a potential infinite loop.
    struct LoopHandler;

    impl CommandHandler<InsertChar, TestState> for LoopHandler {
        fn handle(
            &self,
            _cmd: &InsertChar,
            _state: &mut TestState,
            out: &mut Out,
        ) -> CommandAction {
            out.submit_command(Command::InsertChar {
                payload: InsertChar { ch: 'x' },
            });
            CommandAction::Continue
        }
    }

    #[test]
    fn max_iterations_prevents_infinite_loop() {
        // Given a bus where the handler resubmits itself, with a low max_iterations.
        let mut bus: Bus<TestState> = Bus::new().with_max_iterations(3);
        bus.register_command_handler::<InsertChar, _>(LoopHandler);

        // When processing commands.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then it terminates without hanging.
    }

    // --- has_pending tests ---

    #[test]
    fn has_pending_reflects_queue_state() {
        // Given an empty bus.
        let mut bus: Bus<TestState> = Bus::new();

        // Then the bus has no pending messages.
        assert!(!bus.has_pending());

        // When submitting a command.
        bus.submit_command(Command::Quit);

        // Then the bus has pending messages.
        assert!(bus.has_pending());

        // When processing commands.
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the bus has no pending messages again.
        assert!(!bus.has_pending());
    }

    #[test]
    fn has_pending_with_events() {
        // Given an empty bus.
        let mut bus: Bus<TestState> = Bus::new();

        // Then the bus has no pending messages.
        assert!(!bus.has_pending());

        // When submitting an event.
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });

        // Then the bus has pending messages.
        assert!(bus.has_pending());

        // When processing events.
        let mut state = TestState;
        bus.process_events(&mut state);

        // Then the bus has no pending messages again.
        assert!(!bus.has_pending());
    }

    // --- Mixed dispatch: struct variant with payload ---

    #[test]
    fn struct_command_with_payload_dispatches() {
        // Given a bus with handlers for multiple struct commands.
        let (set_mode_handler, set_mode_calls) =
            FakeCommandHandler::<SetMode, TestState>::continuing();
        let (send_handler, send_calls) = FakeCommandHandler::<SendMessage, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<SetMode, _>(set_mode_handler);
        bus.register_command_handler::<SendMessage, _>(send_handler);

        // When submitting multiple commands.
        bus.submit_command(Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Input,
            },
        });
        bus.submit_command(Command::SendMessage {
            payload: SendMessage {
                text: "hello".into(),
            },
        });
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then both handlers were called with correct payloads.
        assert_eq!(set_mode_calls.borrow().len(), 1);
        assert_eq!(send_calls.borrow().len(), 1);
        assert_eq!(send_calls.borrow()[0].text, "hello");
    }

    /// Handler that submits an event when processing a command.
    struct CommandToEventHandler;

    impl CommandHandler<Quit, TestState> for CommandToEventHandler {
        fn handle(&self, _cmd: &Quit, _state: &mut TestState, out: &mut Out) -> CommandAction {
            out.submit_event(Event::ModeChanged {
                payload: ModeChanged {
                    from: npr::Mode::Normal,
                    to: npr::Mode::Input,
                },
            });
            CommandAction::Continue
        }
    }

    #[test]
    fn command_handler_can_submit_events() {
        // Given a bus where Quit handler submits ModeChanged.
        let (event_handler, event_calls) = FakeEventHandler::<ModeChanged, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(CommandToEventHandler);
        bus.register_event_handler::<ModeChanged, _>(event_handler);

        // When processing a command that submits an event.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the event is in the event queue (not yet processed).
        assert!(bus.has_pending());

        // When processing events.
        bus.process_events(&mut state);

        // Then the event handler was called.
        assert_eq!(event_calls.borrow().len(), 1);
    }

    // --- drain_processed_events tests ---

    #[test]
    fn drain_processed_events_returns_dispatched_events() {
        // Given a bus with an event handler.
        let (handler, _calls) = FakeEventHandler::<ModeChanged, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_event_handler::<ModeChanged, _>(handler);

        // When processing an event.
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        let mut state = TestState;
        bus.process_events(&mut state);

        // Then drain_processed_events returns the dispatched event with no source.
        let processed = bus.drain_processed_events();
        assert_eq!(processed.len(), 1);
        assert!(matches!(processed[0].event, Event::ModeChanged { .. }));
        assert!(processed[0].source.is_none());
    }

    #[test]
    fn drain_processed_events_clears_buffer() {
        // Given a bus with a processed event.
        let (handler, _calls) = FakeEventHandler::<ModeChanged, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_event_handler::<ModeChanged, _>(handler);
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        let mut state = TestState;
        bus.process_events(&mut state);

        // When draining twice.
        let first = bus.drain_processed_events();
        let second = bus.drain_processed_events();

        // Then first has the event and second is empty.
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    // --- drain_processed_commands tests ---

    #[test]
    fn drain_processed_commands_returns_dispatched_commands() {
        // Given a bus with a command handler.
        let (handler, _calls) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(handler);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then drain_processed_commands returns the dispatched command with no source.
        let processed = bus.drain_processed_commands();
        assert_eq!(processed.len(), 1);
        assert!(matches!(processed[0].command, Command::Quit));
        assert!(processed[0].source.is_none());
    }

    #[test]
    fn drain_processed_commands_clears_buffer() {
        // Given a bus with a processed command.
        let (handler, _calls) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(handler);
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // When draining twice.
        let first = bus.drain_processed_commands();
        let second = bus.drain_processed_commands();

        // Then first has the command and second is empty.
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    // --- Source tagging tests ---

    #[test]
    fn submit_command_from_preserves_source() {
        // Given a bus with a command handler.
        let (handler, _calls) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(handler);

        // When submitting a command with a source.
        bus.submit_command_from(Command::Quit, Some(ActorName::new("ext-test")));
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the source is preserved through drain.
        let processed = bus.drain_processed_commands();
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].source.as_deref(), Some("ext-test"));
    }

    #[test]
    fn submit_event_from_preserves_source() {
        // Given a bus with an event handler.
        let (handler, _calls) = FakeEventHandler::<ModeChanged, TestState>::new();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_event_handler::<ModeChanged, _>(handler);

        // When submitting an event with a source.
        bus.submit_event_from(
            Event::ModeChanged {
                payload: ModeChanged {
                    from: npr::Mode::Normal,
                    to: npr::Mode::Input,
                },
            },
            Some(ActorName::new("ext-test")),
        );
        let mut state = TestState;
        bus.process_events(&mut state);

        // Then the source is preserved through drain.
        let processed = bus.drain_processed_events();
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].source.as_deref(), Some("ext-test"));
    }

    #[test]
    fn submit_command_without_source_has_none() {
        // Given a bus with a command handler.
        let (handler, _calls) = FakeCommandHandler::<Quit, TestState>::continuing();
        let mut bus: Bus<TestState> = Bus::new();
        bus.register_command_handler::<Quit, _>(handler);

        // When submitting a command without source.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        bus.process_commands(&mut state);

        // Then the source is None.
        let processed = bus.drain_processed_commands();
        assert!(processed[0].source.is_none());
    }
}
