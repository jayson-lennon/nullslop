//! Event types for the plugin event pipeline.
//!
//! Each event is a separate struct with an `Event` prefix.
//! The [`Event`] wrapper enum provides a single type for
//! serialization and the wire protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ChatEntry, KeyEvent, Mode};

/// A key was pressed down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventKeyDown {
    /// The key event.
    pub key: KeyEvent,
}

/// A key was released.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventKeyUp {
    /// The key event.
    pub key: KeyEvent,
}

/// A chat message was submitted by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventChatMessageSubmitted {
    /// The chat entry that was submitted.
    pub entry: ChatEntry,
}

/// The application mode changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventModeChanged {
    /// The previous mode.
    pub from: Mode,
    /// The new mode.
    pub to: Mode,
}

/// The application has finished starting up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventApplicationReady;

/// A custom event from an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCustom {
    /// The event name.
    pub name: String,
    /// The event data.
    pub data: Value,
}

/// Wrapper enum for all events.
///
/// Used for serialization and the wire protocol between host and extensions.
/// Each variant wraps its corresponding event struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type")]
pub enum Event {
    /// A key was pressed down.
    #[serde(rename = "event_key_down")]
    EventKeyDown {
        /// The event payload.
        #[serde(flatten)]
        payload: EventKeyDown,
    },
    /// A key was released.
    #[serde(rename = "event_key_up")]
    EventKeyUp {
        /// The event payload.
        #[serde(flatten)]
        payload: EventKeyUp,
    },
    /// A chat message was submitted.
    #[serde(rename = "event_chat_message_submitted")]
    EventChatMessageSubmitted {
        /// The event payload.
        #[serde(flatten)]
        payload: EventChatMessageSubmitted,
    },
    /// The application mode changed.
    #[serde(rename = "event_mode_changed")]
    EventModeChanged {
        /// The event payload.
        #[serde(flatten)]
        payload: EventModeChanged,
    },
    /// The application has finished starting up.
    #[serde(rename = "event_application_ready")]
    EventApplicationReady,
    /// A custom event.
    #[serde(rename = "event_custom")]
    EventCustom {
        /// The event payload.
        #[serde(flatten)]
        payload: EventCustom,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Key, Modifiers};

    #[test]
    fn event_application_ready_serialization() {
        // Given an EventApplicationReady.
        let event = Event::EventApplicationReady;

        // When serialized.
        let json = serde_json::to_string(&event).expect("serialize");

        // Then it is {"type":"event_application_ready"}.
        assert_eq!(json, r#"{"type":"event_application_ready"}"#);
    }

    #[test]
    fn event_chat_message_submitted_preserves_entry() {
        // Given an EventChatMessageSubmitted with a user entry.
        let entry = ChatEntry::user("hello");
        let event = Event::EventChatMessageSubmitted {
            payload: EventChatMessageSubmitted { entry },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: Event = serde_json::from_str(&json).expect("deserialize");

        // Then entry text is preserved.
        match back {
            Event::EventChatMessageSubmitted { payload } => {
                assert_eq!(
                    payload.entry.kind,
                    crate::ChatEntryKind::User("hello".to_string())
                );
            }
            other => panic!("expected EventChatMessageSubmitted, got {other:?}"),
        }
    }

    #[rstest::rstest]
    #[case::key_down(Event::EventKeyDown { payload: EventKeyDown { key: KeyEvent { key: Key::Char('a'), modifiers: Modifiers::none() } } })]
    #[case::key_up(Event::EventKeyUp { payload: EventKeyUp { key: KeyEvent { key: Key::Enter, modifiers: Modifiers::none() } } })]
    #[case::chat_submitted(Event::EventChatMessageSubmitted { payload: EventChatMessageSubmitted { entry: ChatEntry::user("test") } })]
    #[case::mode_changed(Event::EventModeChanged { payload: EventModeChanged { from: Mode::Normal, to: Mode::Input } })]
    #[case::application_ready(Event::EventApplicationReady)]
    #[case::custom(Event::EventCustom { payload: EventCustom { name: "my_event".into(), data: serde_json::json!({"key": "value"}) } })]
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
