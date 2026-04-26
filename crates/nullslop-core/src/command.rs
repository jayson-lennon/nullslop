//! Commands that can be dispatched in the application.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A command that can be dispatched.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type")]
pub enum Command {
    /// Submit chat text.
    #[serde(rename = "submit_chat")]
    SubmitChat {
        /// The text to submit.
        text: String,
    },
    /// Clear the chat history.
    #[serde(rename = "clear_chat")]
    ClearChat,
    /// Quit the application.
    #[serde(rename = "quit")]
    Quit,
    /// Edit the current input in an external editor.
    #[serde(rename = "edit_input")]
    EditInput,
    /// A custom command from an extension.
    #[serde(rename = "custom")]
    Custom {
        /// The command name.
        name: String,
        /// The command arguments.
        args: Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_serialization_submit_chat() {
        // Given a SubmitChat command.
        let cmd = Command::SubmitChat {
            text: "hello".to_string(),
        };

        // When serialized.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // Then it contains "type":"submit_chat" and the text.
        assert!(json.contains(r#""type":"submit_chat""#));
        assert!(json.contains(r#""text":"hello""#));
    }

    #[test]
    fn command_serialization_quit() {
        // Given a Quit command.
        let cmd = Command::Quit;

        // When serialized.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // Then it is {"type":"quit"}.
        assert_eq!(json, r#"{"type":"quit"}"#);
    }

    #[test]
    fn command_serialization_custom() {
        // Given a Custom command with name "echo".
        let cmd = Command::Custom {
            name: "echo".to_string(),
            args: serde_json::json!({"text": "hi"}),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: Command = serde_json::from_str(&json).expect("deserialize");

        // Then name is preserved.
        match back {
            Command::Custom { name, .. } => assert_eq!(name, "echo"),
            other => panic!("expected Custom, got {other:?}"),
        }
    }

    #[rstest::rstest]
    #[case::submit_chat(Command::SubmitChat { text: "hello".into() })]
    #[case::clear_chat(Command::ClearChat)]
    #[case::quit(Command::Quit)]
    #[case::edit_input(Command::EditInput)]
    #[case::custom(Command::Custom { name: "echo".into(), args: serde_json::json!({}) })]
    fn command_roundtrip_all_variants(#[case] cmd: Command) {
        // Given a command variant.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // When deserialized.
        let back: Command = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original when re-serialized.
        let back_json = serde_json::to_string(&back).expect("re-serialize");
        assert_eq!(json, back_json);
    }
}
