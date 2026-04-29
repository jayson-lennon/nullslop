//! nullslop-component: built-in component implementations.
//!
//! Contains all components that handle commands and events for the nullslop
//! application, along with [`AppState`], [`ShutdownTracker`], and [`ChatInputBoxState`].
//!
//! All components use the [`define_handler!`](nullslop_component_core::define_handler)
//! macro from `nullslop-component-core`.
//!
//! # Type aliases
//!
//! - [`AppBus`] = `Bus<AppState>` — the standard bus for the nullslop application.
//! - [`AppUiRegistry`] = `UiRegistry<AppState>` — the standard UI registry.

pub mod app_state;
pub mod char_counter;
pub mod chat_input_box;
pub mod chat_input_state;
pub mod chat_log;
pub mod custom_command;
pub mod quit_handler;
pub mod shutdown;

pub use app_state::{AppState, ShutdownTracker};
pub use chat_input_state::ChatInputBoxState;

use nullslop_component_core::Bus;
use nullslop_component_ui::UiRegistry;

/// Standard bus type for the nullslop application.
pub type AppBus = Bus<AppState>;

/// Standard UI registry type for the nullslop application.
pub type AppUiRegistry = UiRegistry<AppState>;

/// Register all built-in components with the bus and UI registry.
///
/// Called once during application startup.
pub fn register_all(bus: &mut AppBus, registry: &mut AppUiRegistry) {
    quit_handler::register(bus, registry);
    custom_command::register(bus, registry);
    chat_input_box::register(bus, registry);
    chat_log::register(bus, registry);
    char_counter::register(bus, registry);
    shutdown::register(bus, registry);
}

#[cfg(test)]
mod macro_tests {
    use npr::command::{AppQuit, ChatBoxInsertChar};
    use npr::event::EventApplicationReady;
    use npr::{Command, CommandAction, Event};
    use nullslop_component_core::fake::FakeCommandHandler;
    use nullslop_component_core::{Bus, Out};
    use nullslop_protocol as npr;

    use crate::AppState;

    // --- Test handler: command handler returning Continue ---

    nullslop_component_core::define_handler! {
        struct ContinueHandler;

        commands {
            ChatBoxInsertChar: on_insert_char,
        }

        events {}
    }

    impl ContinueHandler {
        fn on_insert_char(
            cmd: &ChatBoxInsertChar,
            state: &mut AppState,
            _out: &mut Out,
        ) -> CommandAction {
            state.chat_input.input_buffer.push(cmd.ch);
            CommandAction::Continue
        }
    }

    #[test]
    fn command_handler_returning_continue() {
        // Given a handler registered with the bus.
        let mut bus: Bus<AppState> = Bus::new();
        ContinueHandler.register(&mut bus);

        // When submitting a ChatBoxInsertChar command.
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'x' },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then the handler ran and mutated state.
        assert_eq!(state.chat_input.input_buffer, "x");
    }

    // --- Test handler: command handler returning Stop ---

    nullslop_component_core::define_handler! {
        struct StopHandler;

        commands {
            AppQuit: on_quit,
        }

        events {}
    }

    impl StopHandler {
        fn on_quit(_cmd: &AppQuit, state: &mut AppState, _out: &mut Out) -> CommandAction {
            state.should_quit = true;
            CommandAction::Stop
        }
    }

    #[test]
    fn command_handler_returning_stop_prevents_later_handlers() {
        // Given a StopHandler and a fake handler both registered for AppQuit.
        let mut bus: Bus<AppState> = Bus::new();
        StopHandler.register(&mut bus);
        let (fake, fake_calls) = FakeCommandHandler::<AppQuit, AppState>::continuing();
        bus.register_command_handler::<AppQuit, _>(fake);

        // When processing an AppQuit command.
        bus.submit_command(Command::AppQuit);
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then the stop handler ran and prevented the fake from running.
        assert!(state.should_quit);
        assert!(fake_calls.borrow().is_empty());
    }

    // --- Test handler: event handler ---

    nullslop_component_core::define_handler! {
        struct EventHandlerTest;

        commands {}

        events {
            EventApplicationReady: on_ready,
        }
    }

    impl EventHandlerTest {
        fn on_ready(_evt: &EventApplicationReady, state: &mut AppState, _out: &mut Out) {
            state.should_quit = true;
        }
    }

    #[test]
    fn event_handler_mutates_state() {
        // Given an EventHandlerTest registered with the bus.
        let mut bus: Bus<AppState> = Bus::new();
        EventHandlerTest.register(&mut bus);

        // When processing an EventApplicationReady event.
        bus.submit_event(Event::EventApplicationReady);
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then the handler ran and mutated state.
        assert!(state.should_quit);
    }

    // --- Test handler: multiple command + event handlers ---

    nullslop_component_core::define_handler! {
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
            state: &mut AppState,
            _out: &mut Out,
        ) -> CommandAction {
            state.chat_input.input_buffer.push(cmd.ch);
            CommandAction::Continue
        }

        fn on_quit(_cmd: &AppQuit, state: &mut AppState, _out: &mut Out) -> CommandAction {
            state.should_quit = true;
            CommandAction::Continue
        }

        fn on_ready(_evt: &EventApplicationReady, state: &mut AppState, _out: &mut Out) {
            state.chat_input.input_buffer.push('!');
        }
    }

    #[test]
    fn multiple_handlers_dispatch_correctly() {
        // Given a MultiHandler with 2 command handlers and 1 event handler.
        let mut bus: Bus<AppState> = Bus::new();
        MultiHandler.register(&mut bus);

        // When processing a ChatBoxInsertChar command.
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'h' },
        });
        let mut state = AppState::new();
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
