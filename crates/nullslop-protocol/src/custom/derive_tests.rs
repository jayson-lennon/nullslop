//! Tests for `EventMsg` and `CommandMsg` derive macro code generation.

#[cfg(test)]
mod derive_tests {
    use crate::{CommandMsg, EventMsg};

    // -- Test-only structs with derive macros applied --

    /// Test fixture: a simple event struct with `EventMsg` derived.
    #[derive(Debug, Clone, EventMsg)]
    #[event_msg("test_mod")]
    struct TestEvent;

    /// Test fixture: a simple command struct with `CommandMsg` derived.
    #[derive(Debug, Clone, CommandMsg)]
    #[cmd("test_mod")]
    struct TestCommand;

    #[test]
    fn event_msg_type_name_is_module_scoped() {
        // Given a struct with #[derive(EventMsg)] and #[event_msg("test_mod")].
        // When accessing TYPE_NAME.
        // Then the value is "test_mod::TestEvent".
        assert_eq!(TestEvent::TYPE_NAME, "test_mod::TestEvent");
    }

    #[test]
    fn command_msg_name_is_module_scoped() {
        // Given a struct with #[derive(CommandMsg)] and #[cmd("test_mod")].
        // When accessing NAME.
        // Then the value is "test_mod::TestCommand".
        assert_eq!(TestCommand::NAME, "test_mod::TestCommand");
    }

    #[test]
    fn event_msg_type_name_is_static_str() {
        // Given a derived EventMsg implementation.
        // When binding TYPE_NAME to a local.
        // Then it has the correct type and value.
        let name: &'static str = TestEvent::TYPE_NAME;
        assert_eq!(name, "test_mod::TestEvent");
    }

    #[test]
    fn command_msg_name_is_static_str() {
        // Given a derived CommandMsg implementation.
        // When binding NAME to a local.
        // Then it has the correct type and value.
        let name: &'static str = TestCommand::NAME;
        assert_eq!(name, "test_mod::TestCommand");
    }

    /// Test fixture: event in a different module scope.
    #[derive(Debug, Clone, EventMsg)]
    #[event_msg("chat_input")]
    struct ChatEntrySubmitted;

    /// Test fixture: command in a different module scope.
    #[derive(Debug, Clone, CommandMsg)]
    #[cmd("chat_input")]
    struct InsertChar;

    #[test]
    fn event_msg_different_module_scopes() {
        // Given two structs in different module scopes.
        // When comparing their TYPE_NAME values.
        // Then they include the correct module prefix.
        assert_eq!(
            ChatEntrySubmitted::TYPE_NAME,
            "chat_input::ChatEntrySubmitted"
        );
        assert_eq!(TestEvent::TYPE_NAME, "test_mod::TestEvent");
    }

    #[test]
    fn command_msg_different_module_scopes() {
        // Given two structs in different module scopes.
        // When comparing their NAME values.
        // Then they include the correct module prefix.
        assert_eq!(InsertChar::NAME, "chat_input::InsertChar");
        assert_eq!(TestCommand::NAME, "test_mod::TestCommand");
    }
}
