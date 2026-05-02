//! Built-in components for the nullslop application.
//!
//! A *component* is a self-contained piece of application behavior — it may handle
//! user actions, react to lifecycle events, render part of the interface, or any
//! combination of these. Each component owns a clear domain responsibility and is
//! wired into the application through [`register_all`], which is called once at
//! startup.
//!
//! The components in this crate together provide the core chat experience:
//! accepting user input, displaying conversation history, counting characters,
//! processing actor commands, and coordinating a clean shutdown.
//!
//! # Type aliases
//!
//! - [`AppBus`] — the standard message bus for the application.
//! - [`AppUiRegistry`] — the standard UI element registry.

pub mod app_quit;
pub mod app_state;
pub mod char_counter;
pub mod chat_input_box;
pub mod chat_log;
pub mod chat_session;
pub mod dashboard;
pub mod provider;
pub mod provider_picker;
pub mod shutdown_tracker;
pub mod tab_nav;

pub use app_state::AppState;
pub use chat_input_box::ChatInputBoxState;
pub use chat_session::ChatSessionState;
pub use dashboard::DashboardState;
pub use shutdown_tracker::ShutdownTrackerState;

/// Test utilities shared across the crate.
///
/// Only available in `#[cfg(test)]` builds.
#[cfg(test)]
pub(crate) mod test_utils {
    /// Create a [`nullslop_services::Services`] with fake implementations for tests.
    pub fn test_services() -> nullslop_services::Services {
        nullslop_services::test_services::TestServices::builder().build()
    }
}

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
    app_quit::register(bus, registry);
    shutdown_tracker::register(bus, registry);
    chat_input_box::register(bus, registry);
    chat_log::register(bus, registry);
    char_counter::register(bus, registry);
    dashboard::register(bus, registry);
    tab_nav::register(bus, registry);
    provider::register(bus, registry);
    provider_picker::register(bus, registry);
}

/// Register only TUI elements (no bus handlers).
///
/// Use when bus handlers have already been registered elsewhere
/// (e.g., by [`register_all`] during core creation) and only
/// the UI element registry needs to be populated.
pub fn register_tui_elements(registry: &mut AppUiRegistry) {
    registry.register(Box::new(chat_input_box::ChatInputBoxElement));
    registry.register(Box::new(chat_log::ChatLogElement));
    registry.register(Box::new(char_counter::CharCounterElement));
    registry.register(Box::new(dashboard::DashboardElement));
    registry.register(Box::new(provider::indicator::StreamingIndicatorElement));
    registry.register(Box::new(provider::queue_element::QueueDisplayElement));
}

#[cfg(test)]
mod macro_tests {
    use npr::chat_input::InsertChar;
    use npr::system::{ModeChanged, Quit};
    use npr::{Command, CommandAction, Event};
    use nullslop_component_core::fake::FakeCommandHandler;
    use nullslop_component_core::{Bus, Out};
    use nullslop_protocol as npr;

    use crate::AppState;
    use crate::test_utils;

    // --- Test handler: command handler returning Stop ---

    nullslop_component_core::define_handler! {
        struct StopHandler;

        commands {
            Quit: on_quit,
        }

        events {}
    }

    impl StopHandler {
        fn on_quit(_cmd: &Quit, state: &mut AppState, _out: &mut Out) -> CommandAction {
            state.should_quit = true;
            CommandAction::Stop
        }
    }

    #[test]
    fn command_handler_returning_stop_prevents_later_handlers() {
        // Given a StopHandler and a fake handler both registered for Quit.
        let mut bus: Bus<AppState> = Bus::new();
        StopHandler.register(&mut bus);
        let (fake, fake_calls) = FakeCommandHandler::<Quit, AppState>::continuing();
        bus.register_command_handler::<Quit, _>(fake);

        // When processing a Quit command.
        bus.submit_command(Command::Quit);
        let mut state = AppState::new(test_utils::test_services());
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
            ModeChanged: on_mode_changed,
        }
    }

    impl EventHandlerTest {
        fn on_mode_changed(_evt: &ModeChanged, state: &mut AppState, _out: &mut Out) {
            state.should_quit = true;
        }
    }

    #[test]
    fn event_handler_mutates_state() {
        // Given an EventHandlerTest registered with the bus.
        let mut bus: Bus<AppState> = Bus::new();
        EventHandlerTest.register(&mut bus);

        // When processing a ModeChanged event.
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        let mut state = AppState::new(test_utils::test_services());
        bus.process_events(&mut state);

        // Then the handler ran and mutated state.
        assert!(state.should_quit);
    }

    // --- Test handler: multiple command + event handlers ---

    nullslop_component_core::define_handler! {
        /// A handler with multiple message handlers.
        struct MultiHandler;

        commands {
            InsertChar: on_insert_char,
            Quit: on_quit,
        }

        events {
            ModeChanged: on_mode_changed,
        }
    }

    impl MultiHandler {
        fn on_insert_char(cmd: &InsertChar, state: &mut AppState, _out: &mut Out) -> CommandAction {
            state
                .active_chat_input_mut()
                .insert_grapheme_at_cursor(cmd.ch);
            CommandAction::Continue
        }

        fn on_quit(_cmd: &Quit, state: &mut AppState, _out: &mut Out) -> CommandAction {
            state.should_quit = true;
            CommandAction::Continue
        }

        fn on_mode_changed(_evt: &ModeChanged, state: &mut AppState, _out: &mut Out) {
            state.active_chat_input_mut().insert_grapheme_at_cursor('!');
        }
    }

    #[test]
    fn multiple_handlers_dispatch_correctly() {
        // Given a MultiHandler with 2 command handlers and 1 event handler.
        let mut bus: Bus<AppState> = Bus::new();
        MultiHandler.register(&mut bus);

        // When processing an InsertChar command.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'h' },
        });
        let mut state = AppState::new(test_utils::test_services());
        bus.process_commands(&mut state);

        // Then the command handler ran.
        assert_eq!(state.active_chat_input().text(), "h");
        assert!(!state.should_quit);

        // When also processing Quit.
        bus.submit_command(Command::Quit);
        bus.process_commands(&mut state);

        // Then should_quit is now true.
        assert!(state.should_quit);

        // When processing a ModeChanged event.
        bus.submit_event(Event::ModeChanged {
            payload: ModeChanged {
                from: npr::Mode::Normal,
                to: npr::Mode::Input,
            },
        });
        bus.process_events(&mut state);

        // Then the event handler ran (chat_input.text() has "h!").
        assert_eq!(state.active_chat_input().text(), "h!");
    }
}
