//! Events that occur in the application.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ChatEntry, KeyEvent};

/// An event that occurs in the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type")]
pub enum Event {
    /// Application has finished starting up.
    #[serde(rename = "application_ready")]
    ApplicationReady,
    /// A key was pressed down.
    #[serde(rename = "key_down")]
    KeyDown {
        /// The key event.
        key: KeyEvent,
    },
    /// A key was released.
    #[serde(rename = "key_up")]
    KeyUp {
        /// The key event.
        key: KeyEvent,
    },
    /// A key was pressed (finalized input).
    #[serde(rename = "key_press")]
    KeyPress {
        /// The key event.
        key: KeyEvent,
    },
    /// A new entry was added to the chat history.
    #[serde(rename = "new_chat_entry")]
    NewChatEntry {
        /// The entry that was added.
        entry: ChatEntry,
    },
    /// A custom event from an extension.
    #[serde(rename = "custom")]
    Custom {
        /// The event name.
        name: String,
        /// The event data.
        data: Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Key, Modifiers};

    #[test]
    fn event_serialization_application_ready() {
        // Given an ApplicationReady event.
        let event = Event::ApplicationReady;

        // When serialized.
        let json = serde_json::to_string(&event).expect("serialize");

        // Then it is {"type":"application_ready"}.
        assert_eq!(json, r#"{"type":"application_ready"}"#);
    }

    #[test]
    fn event_serialization_new_chat_entry() {
        // Given a NewChatEntry event with a user entry.
        let entry = ChatEntry::user("hello");
        let event = Event::NewChatEntry { entry };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: Event = serde_json::from_str(&json).expect("deserialize");

        // Then entry text is preserved.
        match back {
            Event::NewChatEntry { entry } => {
                assert_eq!(entry.kind, ChatEntryKind::User("hello".to_string()));
            }
            other => panic!("expected NewChatEntry, got {other:?}"),
        }
    }

    use crate::ChatEntryKind;

    #[rstest::rstest]
    #[case::application_ready(Event::ApplicationReady)]
    #[case::key_down(Event::KeyDown { key: KeyEvent { key: Key::Char('a'), modifiers: Modifiers::none() } })]
    #[case::key_up(Event::KeyUp { key: KeyEvent { key: Key::Enter, modifiers: Modifiers::none() } })]
    #[case::key_press(Event::KeyPress { key: KeyEvent { key: Key::Esc, modifiers: Modifiers::ctrl() } })]
    #[case::new_chat_entry(Event::NewChatEntry { entry: ChatEntry::user("test") })]
    #[case::custom(Event::Custom { name: "my_event".into(), data: serde_json::json!({"key": "value"}) })]
    fn event_roundtrip_all_variants(#[case] event: Event) {
        // Given an event variant.
        let json = serde_json::to_string(&event).expect("serialize");

        // When deserialized.
        let back: Event = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original when re-serialized.
        let back_json = serde_json::to_string(&back).expect("re-serialize");
        assert_eq!(json, back_json);
    }
}
