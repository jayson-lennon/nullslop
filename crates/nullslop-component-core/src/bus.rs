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
use std::marker::PhantomData;

use nullslop_protocol::chat_input::{
    Clear, DeleteGrapheme, DeleteGraphemeForward, Interrupt, MoveCursorDown, MoveCursorLeft,
    MoveCursorRight, MoveCursorToEnd, MoveCursorToStart, MoveCursorUp, MoveCursorWordLeft,
    MoveCursorWordRight,
};
use nullslop_protocol::provider::RefreshModels;
use nullslop_protocol::provider_picker::{
    PickerBackspace, PickerConfirm, PickerMoveCursorLeft, PickerMoveCursorRight, PickerMoveDown,
    PickerMoveUp,
};
use nullslop_protocol::system::{EditInput, MouseScrollDown, MouseScrollUp, Quit, ScrollDown, ScrollUp, ToggleWhichKey};
use nullslop_protocol::{ActorName, Command, CommandAction, Event};

use crate::handler::{CommandHandler, EventHandler, HandlerContext};
use crate::out::Out;

/// Type-erased command handler ready for dispatch.
struct AnyCommandHandler<S, Sv> {
    /// The type-erased handler instance.
    handler: Box<dyn Any>,
    /// Function pointer that downcasts and invokes the handler.
    invoke: fn(&dyn Any, &dyn Any, &mut S, &Sv, &mut Out) -> CommandAction,
    /// Marker for the unused services type parameter.
    _phantom: PhantomData<Sv>,
}

/// Type-erased event handler ready for dispatch.
struct AnyEventHandler<S, Sv> {
    /// The type-erased handler instance.
    handler: Box<dyn Any>,
    /// Function pointer that downcasts and invokes the handler.
    invoke: fn(&dyn Any, &dyn Any, &mut S, &Sv, &mut Out),
    /// Marker for the unused services type parameter.
    _phantom: PhantomData<Sv>,
}

/// Invokes a command handler with its concrete types.
#[expect(
    clippy::expect_used,
    reason = "type is guaranteed by construction via Bus registration"
)]
fn invoke_command<C, H, S, Sv>(
    handler: &dyn Any,
    cmd: &dyn Any,
    state: &mut S,
    services: &Sv,
    out: &mut Out,
) -> CommandAction
where
    H: CommandHandler<C, S, Sv> + 'static,
    C: 'static,
{
    let h = handler.downcast_ref::<H>().expect("handler type mismatch");
    let c = cmd.downcast_ref::<C>().expect("command type mismatch");
    let mut ctx = HandlerContext::new(state, services, out);
    h.handle(c, &mut ctx)
}

/// Invokes an event handler with its concrete types.
#[expect(
    clippy::expect_used,
    reason = "type is guaranteed by construction via Bus registration"
)]
fn invoke_event<E, H, S, Sv>(
    handler: &dyn Any,
    evt: &dyn Any,
    state: &mut S,
    services: &Sv,
    out: &mut Out,
) where
    H: EventHandler<E, S, Sv> + 'static,
    E: 'static,
{
    let h = handler.downcast_ref::<H>().expect("handler type mismatch");
    let e = evt.downcast_ref::<E>().expect("event type mismatch");
    let mut ctx = HandlerContext::new(state, services, out);
    h.handle(e, &mut ctx);
}

/// A queued command together with its origin.
struct QueuedCommand {
    /// The command payload.
    command: Command,
    /// The actor that submitted this command, if any.
    source: Option<ActorName>,
}

/// A queued event together with its origin.
struct QueuedEvent {
    /// The event payload.
    event: Event,
    /// The actor that submitted this event, if any.
    source: Option<ActorName>,
}

/// Central message router that dispatches commands and events to registered handlers.
///
/// Commands and events are submitted to queues and processed in order. Each
/// message is routed to every handler registered for its type. The processing
/// model ensures consistent state snapshots across handlers.
pub struct Bus<S, Sv> {
    /// Registered command handlers keyed by their command type.
    command_handlers: HashMap<TypeId, Vec<AnyCommandHandler<S, Sv>>>,
    /// Registered event handlers keyed by their event type.
    event_handlers: HashMap<TypeId, Vec<AnyEventHandler<S, Sv>>>,
    /// Commands waiting to be dispatched.
    command_queue: Vec<QueuedCommand>,
    /// Events waiting to be dispatched.
    event_queue: Vec<QueuedEvent>,
    /// Events dispatched during the last processing cycle, with source.
    /// Available via [`drain_processed_events`](Self::drain_processed_events).
    processed_events: Vec<ProcessedEvent>,
    /// Commands dispatched during the last processing cycle, with source.
    /// Available via [`drain_processed_commands`](Self::drain_processed_commands).
    processed_commands: Vec<ProcessedCommand>,
    /// Maximum number of processing iterations to prevent infinite loops.
    max_iterations: usize,
    /// Marker for the unused services type parameter.
    _phantom: PhantomData<Sv>,
}

impl<S, Sv> Bus<S, Sv> {
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
            _phantom: PhantomData,
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
        H: CommandHandler<C, S, Sv> + 'static,
    {
        let type_id = TypeId::of::<C>();
        let invoke = invoke_command::<C, H, S, Sv>;
        let entry = AnyCommandHandler {
            handler: Box::new(handler),
            invoke,
            _phantom: PhantomData,
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
        H: EventHandler<E, S, Sv> + 'static,
    {
        let type_id = TypeId::of::<E>();
        let invoke = invoke_event::<E, H, S, Sv>;
        let entry = AnyEventHandler {
            handler: Box::new(handler),
            invoke,
            _phantom: PhantomData,
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
    pub fn process_commands(&mut self, state: &mut S, services: &Sv) {
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
                self.dispatch_command(queued.command, queued.source, state, services);
            }
        }
    }

    /// Process all pending events in a single pass.
    ///
    /// Drains the event queue and dispatches each event to its registered
    /// handlers. All handlers always run. Events submitted by handlers during
    /// processing are queued for a future call.
    pub fn process_events(&mut self, state: &mut S, services: &Sv) {
        let events = std::mem::take(&mut self.event_queue);
        for queued in events {
            self.dispatch_event(queued.event, queued.source, state, services);
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
    /// (e.g., actor host) after bus processing completes.
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
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive match dispatch grows with each command variant"
    )]
    fn dispatch_command(
        &mut self,
        cmd: Command,
        source: Option<ActorName>,
        state: &mut S,
        services: &Sv,
    ) {
        // Record the command before dispatching so consumers can drain it later.
        self.processed_commands.push(ProcessedCommand {
            command: cmd.clone(),
            source,
        });
        let mut out = Out::new();
        match cmd {
            Command::InsertChar { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::DeleteGrapheme => {
                let cmd = DeleteGrapheme;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::SubmitMessage { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::Clear => {
                let cmd = Clear;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::Interrupt => {
                let cmd = Interrupt;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorLeft => {
                let cmd = MoveCursorLeft;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorRight => {
                let cmd = MoveCursorRight;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorToStart => {
                let cmd = MoveCursorToStart;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorToEnd => {
                let cmd = MoveCursorToEnd;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::DeleteGraphemeForward => {
                let cmd = DeleteGraphemeForward;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorWordLeft => {
                let cmd = MoveCursorWordLeft;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorWordRight => {
                let cmd = MoveCursorWordRight;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorUp => {
                let cmd = MoveCursorUp;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MoveCursorDown => {
                let cmd = MoveCursorDown;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::SetMode { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::Quit => {
                let cmd = Quit;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::EditInput => {
                let cmd = EditInput;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::ToggleWhichKey => {
                let cmd = ToggleWhichKey;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::SwitchTab { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::SendMessage { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::CancelStream { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::SendToLlmProvider { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::AssemblePrompt { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::SwitchPromptStrategy { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::RestoreStrategyState { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::StreamToken { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::PushChatEntry { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::EnqueueUserMessage { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::SetChatInputText { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ProceedWithShutdown { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ScrollUp => {
                let cmd = ScrollUp;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::ScrollDown => {
                let cmd = ScrollDown;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MouseScrollUp => {
                let cmd = MouseScrollUp;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::MouseScrollDown => {
                let cmd = MouseScrollDown;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::ProviderSwitch { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::PickerInsertChar { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::PickerBackspace => {
                let cmd = PickerBackspace;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::PickerConfirm => {
                let cmd = PickerConfirm;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::PickerMoveUp => {
                let cmd = PickerMoveUp;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::PickerMoveDown => {
                let cmd = PickerMoveDown;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::PickerMoveCursorLeft => {
                let cmd = PickerMoveCursorLeft;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::PickerMoveCursorRight => {
                let cmd = PickerMoveCursorRight;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::RefreshModels => {
                let cmd = RefreshModels;
                self.dispatch_command_to_handlers(&cmd, state, services, &mut out);
            }
            Command::RegisterTools { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ExecuteToolBatch { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ExecuteTool { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ToolUseStarted { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ToolCallReceived { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::ToolCallStreaming { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
            Command::PushToolResult { payload } => {
                self.dispatch_command_to_handlers(&payload, state, services, &mut out);
            }
        }
        self.flush_out(out);
    }

    /// Look up and invoke handlers for a concrete command type `C`.
    fn dispatch_command_to_handlers<C>(&self, cmd: &C, state: &mut S, services: &Sv, out: &mut Out)
    where
        C: 'static,
    {
        let type_id = TypeId::of::<C>();
        if let Some(handlers) = self.command_handlers.get(&type_id) {
            for h in handlers {
                let action = (h.invoke)(&*h.handler, cmd as &dyn Any, state, services, out);
                if action == CommandAction::Stop {
                    break;
                }
            }
        }
    }

    /// Dispatch a single event to its registered handlers.
    fn dispatch_event(
        &mut self,
        evt: Event,
        source: Option<ActorName>,
        state: &mut S,
        services: &Sv,
    ) {
        // Record the event before dispatching so consumers can drain it later.
        self.processed_events.push(ProcessedEvent {
            event: evt.clone(),
            source,
        });
        let mut out = Out::new();
        match evt {
            Event::KeyDown { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::KeyUp { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ChatEntrySubmitted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ModeChanged { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ActorStarting { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ActorStarted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ActorShutdownCompleted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::StreamCompleted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ProviderSwitched { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ModelsRefreshed { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ToolBatchCompleted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ToolExecutionCompleted { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::ToolsRegistered { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::PromptAssembled { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::PromptStrategySwitched { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
            Event::StrategyStateUpdated { payload } => {
                self.dispatch_event_to_handlers(&payload, state, services, &mut out);
            }
        }
        self.flush_out(out);
    }

    /// Look up and invoke handlers for a concrete event type `E`.
    fn dispatch_event_to_handlers<E>(&self, evt: &E, state: &mut S, services: &Sv, out: &mut Out)
    where
        E: 'static,
    {
        let type_id = TypeId::of::<E>();
        if let Some(handlers) = self.event_handlers.get(&type_id) {
            for h in handlers {
                (h.invoke)(&*h.handler, evt as &dyn Any, state, services, out);
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

impl<S, Sv> Default for Bus<S, Sv> {
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
        let (handler, calls) = FakeCommandHandler::<InsertChar, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<InsertChar, _>(handler);

        // When submitting and processing the command.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the handler was called with the correct payload.
        assert_eq!(calls.borrow().len(), 1);
        assert_eq!(calls.borrow()[0].ch, 'x');
    }

    #[test]
    fn multiple_command_handlers_all_run() {
        // Given a bus with two handlers for the same command type.
        let (h1, calls1) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let (h2, calls2) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(h1);
        bus.register_command_handler::<Quit, _>(h2);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then both handlers were called.
        assert_eq!(calls1.borrow().len(), 1);
        assert_eq!(calls2.borrow().len(), 1);
    }

    #[test]
    fn stop_halts_propagation() {
        // Given a bus where the first handler returns Stop.
        let (stopper, stopper_calls) = FakeCommandHandler::<Quit, TestState, ()>::stopping();
        let (continuer, continuer_calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(stopper);
        bus.register_command_handler::<Quit, _>(continuer);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then only the first handler was called.
        assert_eq!(stopper_calls.borrow().len(), 1);
        assert!(continuer_calls.borrow().is_empty());
    }

    #[test]
    fn continue_allows_propagation() {
        // Given a bus where the first handler returns Continue.
        let (c1, calls1) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let (c2, calls2) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(c1);
        bus.register_command_handler::<Quit, _>(c2);

        // When processing a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then both handlers were called.
        assert_eq!(calls1.borrow().len(), 1);
        assert_eq!(calls2.borrow().len(), 1);
    }

    #[test]
    fn unregistered_command_is_ignored() {
        // Given a bus with no handlers.
        let mut bus: Bus<TestState, ()> = Bus::new();

        // When submitting a command.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then no panic occurs and the queue is empty.
        assert!(!bus.has_pending());
    }

    #[test]
    fn unit_command_dispatches_correctly() {
        // Given a bus with a handler for DeleteGrapheme (unit struct).
        let (handler, calls) = FakeCommandHandler::<DeleteGrapheme, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<DeleteGrapheme, _>(handler);

        // When processing a unit command.
        bus.submit_command(Command::DeleteGrapheme);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the handler was called.
        assert_eq!(calls.borrow().len(), 1);
    }

    // --- Event dispatch tests ---

    #[test]
    fn event_dispatch_reaches_handler() {
        // Given a bus with a handler for KeyDown.
        let (handler, calls) = FakeEventHandler::<KeyDown, TestState, ()>::new();
        let mut bus: Bus<TestState, ()> = Bus::new();
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
        let services = ();
        bus.process_events(&mut state, &services);

        // Then the handler was called.
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn all_event_handlers_run() {
        // Given a bus with two event handlers for ModeChanged.
        let (h1, calls1) = FakeEventHandler::<ModeChanged, TestState, ()>::new();
        let (h2, calls2) = FakeEventHandler::<ModeChanged, TestState, ()>::new();
        let mut bus: Bus<TestState, ()> = Bus::new();
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
        let services = ();
        bus.process_events(&mut state, &services);

        // Then both handlers were called.
        assert_eq!(calls1.borrow().len(), 1);
        assert_eq!(calls2.borrow().len(), 1);
    }

    // --- Out / cascading tests ---

    /// Handler that submits an `AppQuit` command when it sees `InsertChar`.
    struct CascadeHandler;

    impl CommandHandler<InsertChar, TestState, ()> for CascadeHandler {
        fn handle(
            &self,
            _cmd: &InsertChar,
            ctx: &mut HandlerContext<'_, TestState, ()>,
        ) -> CommandAction {
            ctx.out.submit_command(Command::Quit);
            CommandAction::Continue
        }
    }

    #[test]
    fn cascading_commands_are_processed() {
        // Given a bus where InsertChar handler submits AppQuit.
        let (quit_handler, quit_calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<InsertChar, _>(CascadeHandler);
        bus.register_command_handler::<Quit, _>(quit_handler);

        // When processing the initial command.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the cascaded Quit was also processed.
        assert_eq!(quit_calls.borrow().len(), 1);
    }

    /// Handler that resubmits itself, creating a potential infinite loop.
    struct LoopHandler;

    impl CommandHandler<InsertChar, TestState, ()> for LoopHandler {
        fn handle(
            &self,
            _cmd: &InsertChar,
            ctx: &mut HandlerContext<'_, TestState, ()>,
        ) -> CommandAction {
            ctx.out.submit_command(Command::InsertChar {
                payload: InsertChar { ch: 'x' },
            });
            CommandAction::Continue
        }
    }

    #[test]
    fn max_iterations_prevents_infinite_loop() {
        // Given a bus where the handler resubmits itself, with a low max_iterations.
        let mut bus: Bus<TestState, ()> = Bus::new().with_max_iterations(3);
        bus.register_command_handler::<InsertChar, _>(LoopHandler);

        // When processing commands.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then it terminates without hanging.
    }

    // --- has_pending tests ---

    #[test]
    fn has_pending_reflects_queue_state() {
        // Given an empty bus.
        let mut bus: Bus<TestState, ()> = Bus::new();

        // Then the bus has no pending messages.
        assert!(!bus.has_pending());

        // When submitting a command.
        bus.submit_command(Command::Quit);

        // Then the bus has pending messages.
        assert!(bus.has_pending());

        // When processing commands.
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the bus has no pending messages again.
        assert!(!bus.has_pending());
    }

    #[test]
    fn has_pending_with_events() {
        // Given an empty bus.
        let mut bus: Bus<TestState, ()> = Bus::new();

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
        let services = ();
        bus.process_events(&mut state, &services);

        // Then the bus has no pending messages again.
        assert!(!bus.has_pending());
    }

    // --- Mixed dispatch: struct variant with payload ---

    #[test]
    fn struct_command_with_payload_dispatches() {
        // Given a bus with handlers for multiple struct commands.
        let (set_mode_handler, set_mode_calls) =
            FakeCommandHandler::<SetMode, TestState, ()>::continuing();
        let (send_handler, send_calls) =
            FakeCommandHandler::<SendMessage, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
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
                session_id: npr::SessionId::new(),
                text: "hello".into(),
            },
        });
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then both handlers were called with correct payloads.
        assert_eq!(set_mode_calls.borrow().len(), 1);
        assert_eq!(send_calls.borrow().len(), 1);
        assert_eq!(send_calls.borrow()[0].text, "hello");
    }

    /// Handler that submits an event when processing a command.
    struct CommandToEventHandler;

    impl CommandHandler<Quit, TestState, ()> for CommandToEventHandler {
        fn handle(
            &self,
            _cmd: &Quit,
            ctx: &mut HandlerContext<'_, TestState, ()>,
        ) -> CommandAction {
            ctx.out.submit_event(Event::ModeChanged {
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
        let (event_handler, event_calls) = FakeEventHandler::<ModeChanged, TestState, ()>::new();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(CommandToEventHandler);
        bus.register_event_handler::<ModeChanged, _>(event_handler);

        // When processing a command that submits an event.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the event is in the event queue (not yet processed).
        assert!(bus.has_pending());

        // When processing events.
        bus.process_events(&mut state, &services);

        // Then the event handler was called.
        assert_eq!(event_calls.borrow().len(), 1);
    }

    // --- Drain processed tests ---

    #[test]
    fn drain_processed_returns_dispatched_items() {
        // Given a bus with a command and event handler.
        let (cmd_handler, _cmd_calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let (evt_handler, _evt_calls) = FakeEventHandler::<ModeChanged, TestState, ()>::new();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(cmd_handler);
        bus.register_event_handler::<ModeChanged, _>(evt_handler);

        // When processing a command and event.
        bus.submit_command(Command::Quit);
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);
        bus.process_events(&mut state, &services);

        // Then drain returns both with no source.
        let events = bus.drain_processed_events();
        let commands = bus.drain_processed_commands();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].event, Event::ModeChanged { .. }));
        assert!(events[0].source.is_none());
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0].command, Command::Quit));
        assert!(commands[0].source.is_none());
    }

    #[test]
    fn drain_processed_clears_buffers() {
        // Given a bus with a command and event handler.
        let (cmd_handler, _cmd_calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let (evt_handler, _evt_calls) = FakeEventHandler::<ModeChanged, TestState, ()>::new();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(cmd_handler);
        bus.register_event_handler::<ModeChanged, _>(evt_handler);
        bus.submit_command(Command::Quit);
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);
        bus.process_events(&mut state, &services);

        // When draining twice.
        let first_events = bus.drain_processed_events();
        let first_commands = bus.drain_processed_commands();
        let second_events = bus.drain_processed_events();
        let second_commands = bus.drain_processed_commands();

        // Then first has items and second is empty.
        assert_eq!(first_events.len(), 1);
        assert_eq!(first_commands.len(), 1);
        assert!(second_events.is_empty());
        assert!(second_commands.is_empty());
    }

    // --- Source tagging tests ---

    #[test]
    fn submit_command_from_preserves_source() {
        // Given a bus with a command handler.
        let (handler, _calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(handler);

        // When submitting a command with a source.
        bus.submit_command_from(Command::Quit, Some(ActorName::new("ext-test")));
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the source is preserved through drain.
        let processed = bus.drain_processed_commands();
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].source.as_deref(), Some("ext-test"));
    }

    #[test]
    fn submit_event_from_preserves_source() {
        // Given a bus with an event handler.
        let (handler, _calls) = FakeEventHandler::<ModeChanged, TestState, ()>::new();
        let mut bus: Bus<TestState, ()> = Bus::new();
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
        let services = ();
        bus.process_events(&mut state, &services);

        // Then the source is preserved through drain.
        let processed = bus.drain_processed_events();
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].source.as_deref(), Some("ext-test"));
    }

    #[test]
    fn submit_command_without_source_has_none() {
        // Given a bus with a command handler.
        let (handler, _calls) = FakeCommandHandler::<Quit, TestState, ()>::continuing();
        let mut bus: Bus<TestState, ()> = Bus::new();
        bus.register_command_handler::<Quit, _>(handler);

        // When submitting a command without source.
        bus.submit_command(Command::Quit);
        let mut state = TestState;
        let services = ();
        bus.process_commands(&mut state, &services);

        // Then the source is None.
        let processed = bus.drain_processed_commands();
        assert!(processed[0].source.is_none());
    }
}
