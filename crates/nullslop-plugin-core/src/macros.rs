//! Declarative macro for defining plugins that handle messages (commands and events).
//!
//! The [`define_handler!`] macro reduces boilerplate by generating:
//! - The handler struct definition (unit struct)
//! - `impl CommandHandler<C>` for each command entry
//! - `impl EventHandler<E>` for each event entry
//! - A `register(&self, bus: &mut Bus)` method
//!
//! Users provide method implementations in a separate `impl` block for full
//! IDE support (autocomplete, type checking, inline errors).

/// Define a plugin that handles messages (commands and events).
///
/// A plugin can be a command/event handler (defined via this macro),
/// a UI element (implementing `UiElement`), or both. This macro
/// generates the handler struct and typed dispatch wiring.
///
/// Generates:
/// - The handler struct definition (unit struct)
/// - `impl CommandHandler<C>` for each command entry (forwards `CommandAction` return value)
/// - `impl EventHandler<E>` for each event entry
/// - A `register(&self, bus: &mut Bus)` method
///
/// # Syntax
///
/// ```ignore
/// define_handler! {
///     /// Optional doc comments.
///     pub struct MyHandler;
///
///     commands {
///         CmdTypeA: method_a,
///         CmdTypeB: method_b,
///     }
///
///     events {
///         EvtTypeX: method_x,
///     }
/// }
/// ```
///
/// # Handler methods
///
/// Command handler methods must have this signature:
/// `fn method(cmd: &C, state: &mut AppData, out: &mut Out) -> CommandAction`
///
/// Event handler methods must have this signature:
/// `fn method(evt: &E, state: &mut AppData, out: &mut Out)`
///
/// Command methods return `CommandAction` directly — the macro forwards the return value.
/// Event methods return `()`.
#[macro_export]
macro_rules! define_handler {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident;

        commands {
            $($cmd_type:ty: $cmd_method:ident),* $(,)?
        }

        events {
            $($evt_type:ty: $evt_method:ident),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone)]
        $vis struct $name;

        // Generate CommandHandler impls (forward return value)
        $(
            impl $crate::CommandHandler<$cmd_type> for $name {
                #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
                fn handle(
                    &self,
                    cmd: &$cmd_type,
                    state: &mut ::nullslop_protocol::AppData,
                    out: &mut $crate::Out,
                ) -> ::nullslop_protocol::CommandAction {
                    Self::$cmd_method(cmd, state, out)
                }
            }
        )*

        // Generate EventHandler impls
        $(
            impl $crate::EventHandler<$evt_type> for $name {
                #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
                fn handle(
                    &self,
                    evt: &$evt_type,
                    state: &mut ::nullslop_protocol::AppData,
                    out: &mut $crate::Out,
                ) {
                    Self::$evt_method(evt, state, out);
                }
            }
        )*

        // Generate register method
        impl $name {
            #[doc = concat!("Register all handlers with the bus.\n\n⚠️ This must be called during application startup. Add a `", stringify!($name), ".register(&mut bus);` call in the plugin registration section of `run.rs`.")]
            pub fn register(&self, bus: &mut $crate::Bus) {
                $(
                    bus.register_command_handler::<$cmd_type, Self>(*self);
                )*
                $(
                    bus.register_event_handler::<$evt_type, Self>(*self);
                )*
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::fake::FakeCommandHandler;
    use crate::{Bus, Out};
    use npr::command::{AppQuit, ChatBoxInsertChar};
    use npr::event::EventApplicationReady;
    use npr::{AppData, Command, CommandAction, Event};
    use nullslop_protocol as npr;

    // --- Test handler: command handler returning Continue ---

    define_handler! {
        struct ContinueHandler;

        commands {
            ChatBoxInsertChar: on_insert_char,
        }

        events {}
    }

    impl ContinueHandler {
        fn on_insert_char(
            cmd: &ChatBoxInsertChar,
            state: &mut AppData,
            _out: &mut Out,
        ) -> CommandAction {
            state.chat_input.input_buffer.push(cmd.ch);
            CommandAction::Continue
        }
    }

    #[test]
    fn command_handler_returning_continue() {
        // Given a handler registered with the bus.
        let mut bus = Bus::new();
        ContinueHandler.register(&mut bus);

        // When submitting a ChatBoxInsertChar command.
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'x' },
        });
        let mut state = AppData::new();
        bus.process_commands(&mut state);

        // Then the handler ran and mutated state.
        assert_eq!(state.chat_input.input_buffer, "x");
    }

    // --- Test handler: command handler returning Stop ---

    define_handler! {
        struct StopHandler;

        commands {
            AppQuit: on_quit,
        }

        events {}
    }

    impl StopHandler {
        fn on_quit(_cmd: &AppQuit, state: &mut AppData, _out: &mut Out) -> CommandAction {
            state.should_quit = true;
            CommandAction::Stop
        }
    }

    #[test]
    fn command_handler_returning_stop_prevents_later_handlers() {
        // Given a StopHandler and a fake handler both registered for AppQuit.
        let mut bus = Bus::new();
        StopHandler.register(&mut bus);
        let (fake, fake_calls) = FakeCommandHandler::<AppQuit>::continuing();
        bus.register_command_handler::<AppQuit, _>(fake);

        // When processing an AppQuit command.
        bus.submit_command(Command::AppQuit);
        let mut state = AppData::new();
        bus.process_commands(&mut state);

        // Then the stop handler ran and prevented the fake from running.
        assert!(state.should_quit);
        assert!(fake_calls.borrow().is_empty());
    }

    // --- Test handler: event handler ---

    define_handler! {
        struct EventHandlerTest;

        commands {}

        events {
            EventApplicationReady: on_ready,
        }
    }

    impl EventHandlerTest {
        fn on_ready(_evt: &EventApplicationReady, state: &mut AppData, _out: &mut Out) {
            state.should_quit = true;
        }
    }

    #[test]
    fn event_handler_mutates_state() {
        // Given an EventHandlerTest registered with the bus.
        let mut bus = Bus::new();
        EventHandlerTest.register(&mut bus);

        // When processing an EventApplicationReady event.
        bus.submit_event(Event::EventApplicationReady);
        let mut state = AppData::new();
        bus.process_events(&mut state);

        // Then the handler ran and mutated state.
        assert!(state.should_quit);
    }

    // --- Test handler: multiple command + event handlers ---

    define_handler! {
        /// A handler with multiple message handlers.
        struct MultiHandler;

        commands {
            ChatBoxInsertChar: on_insert_char,
            AppQuit: on_quit,
        }

        events {
            EventApplicationReady: on_ready,
        }
    }

    impl MultiHandler {
        fn on_insert_char(
            cmd: &ChatBoxInsertChar,
            state: &mut AppData,
            _out: &mut Out,
        ) -> CommandAction {
            state.chat_input.input_buffer.push(cmd.ch);
            CommandAction::Continue
        }

        fn on_quit(_cmd: &AppQuit, state: &mut AppData, _out: &mut Out) -> CommandAction {
            state.should_quit = true;
            CommandAction::Continue
        }

        fn on_ready(_evt: &EventApplicationReady, state: &mut AppData, _out: &mut Out) {
            state.chat_input.input_buffer.push('!');
        }
    }

    #[test]
    fn multiple_handlers_dispatch_correctly() {
        // Given a MultiHandler with 2 command handlers and 1 event handler.
        let mut bus = Bus::new();
        MultiHandler.register(&mut bus);

        // When processing a ChatBoxInsertChar command.
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'h' },
        });
        let mut state = AppData::new();
        bus.process_commands(&mut state);

        // Then the command handler ran.
        assert_eq!(state.chat_input.input_buffer, "h");
        assert!(!state.should_quit);

        // When also processing AppQuit.
        bus.submit_command(Command::AppQuit);
        bus.process_commands(&mut state);

        // Then should_quit is now true.
        assert!(state.should_quit);

        // When processing an EventApplicationReady.
        bus.submit_event(Event::EventApplicationReady);
        bus.process_events(&mut state);

        // Then the event handler ran (chat_input.input_buffer has "h!").
        assert_eq!(state.chat_input.input_buffer, "h!");
    }
}
