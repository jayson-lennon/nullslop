//! Event types for the component event pipeline.
//!
//! The [`Event`] enum is the unified type the host broadcasts to
//! inform internal handlers and extensions about state changes and input.
//!
//! Individual event structs live in domain modules ([`chat_input`], [`system`],
//! [`custom`], [`shutdown`]). This module re-exports them for convenience.
//!
//! # When adding a new event
//!
//! Every new event struct **must** be added as a variant on the [`Event`] enum
//! below. Creating the struct alone is not enough — the bus broadcasts based on
//! enum variants, so a missing variant means the event is invisible to the system.

use serde::{Deserialize, Serialize};

// Re-export event structs and trait from domain modules.
pub use crate::chat_input::EventChatMessageSubmitted;
pub use crate::custom::{EventCustom, EventMsg};
pub use crate::shutdown::{
    EventApplicationShuttingDown, ExtensionShutdownCompleted, ExtensionStarted, ExtensionStarting,
};
pub use crate::system::{EventApplicationReady, EventKeyDown, EventKeyUp, EventModeChanged};

/// Every event the host can broadcast.
///
/// Extensions subscribe to relevant variants; the host also
/// uses them internally to drive UI updates.
///
/// **When adding a new event struct**, you must add a corresponding variant to
/// this enum. An event struct defined in a domain module without an enum variant
/// here will not be broadcast by the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// A key was pressed down.
    #[serde(rename = "event_key_down")]
    EventKeyDown {
        /// Which key was pressed.
        #[serde(flatten)]
        payload: EventKeyDown,
    },
    /// A key was released.
    #[serde(rename = "event_key_up")]
    EventKeyUp {
        /// Which key was released.
        #[serde(flatten)]
        payload: EventKeyUp,
    },
    /// A chat message was submitted.
    #[serde(rename = "event_chat_message_submitted")]
    EventChatMessageSubmitted {
        /// The submitted chat entry.
        #[serde(flatten)]
        payload: EventChatMessageSubmitted,
    },
    /// The application mode changed.
    #[serde(rename = "event_mode_changed")]
    EventModeChanged {
        /// The previous and new mode.
        #[serde(flatten)]
        payload: EventModeChanged,
    },
    /// The application has finished starting up.
    #[serde(rename = "event_application_ready")]
    EventApplicationReady,
    /// A custom event.
    #[serde(rename = "event_custom")]
    EventCustom {
        /// The extension-defined event name and data.
        #[serde(flatten)]
        payload: EventCustom,
    },
    /// An extension is starting up.
    #[serde(rename = "event_extension_starting")]
    EventExtensionStarting {
        /// Which extension is starting.
        #[serde(flatten)]
        payload: ExtensionStarting,
    },
    /// An extension has finished starting up.
    #[serde(rename = "event_extension_started")]
    EventExtensionStarted {
        /// Which extension finished starting.
        #[serde(flatten)]
        payload: ExtensionStarted,
    },
    /// An extension has completed shutdown.
    #[serde(rename = "event_extension_shutdown_completed")]
    EventExtensionShutdownCompleted {
        /// Which extension finished shutting down.
        #[serde(flatten)]
        payload: ExtensionShutdownCompleted,
    },
    /// The application is shutting down.
    #[serde(rename = "event_application_shutting_down")]
    EventApplicationShuttingDown,
}

impl Event {
    /// Returns the subscription-relevant type name for event routing.
    ///
    /// Returns `None` for events that should not be routed to extensions
    /// (e.g., key events).
    #[must_use]
    pub fn type_name(&self) -> Option<&str> {
        match self {
            Self::EventChatMessageSubmitted { .. } => Some(EventChatMessageSubmitted::TYPE_NAME),
            Self::EventApplicationReady => Some(EventApplicationReady::TYPE_NAME),
            Self::EventCustom { payload, .. } => Some(payload.name.as_str()),
            Self::EventExtensionStarting { .. } => Some(ExtensionStarting::TYPE_NAME),
            Self::EventExtensionStarted { .. } => Some(ExtensionStarted::TYPE_NAME),
            Self::EventExtensionShutdownCompleted { .. } => {
                Some(ExtensionShutdownCompleted::TYPE_NAME)
            }
            Self::EventApplicationShuttingDown => Some(EventApplicationShuttingDown::TYPE_NAME),
            Self::EventKeyDown { .. } | Self::EventKeyUp { .. } | Self::EventModeChanged { .. } => {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatEntry, Key, KeyEvent, Mode, Modifiers};

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
    #[case::extension_starting(Event::EventExtensionStarting { payload: ExtensionStarting { name: "ext-a".into() } })]
    #[case::extension_started(Event::EventExtensionStarted { payload: ExtensionStarted { name: "ext-a".into() } })]
    #[case::extension_shutdown_completed(Event::EventExtensionShutdownCompleted { payload: ExtensionShutdownCompleted { name: "ext-a".into() } })]
    #[case::application_shutting_down(Event::EventApplicationShuttingDown)]
    fn event_roundtrip_all_variants(#[case] event: Event) {
        // Given an event variant.
        let json = serde_json::to_string(&event).expect("serialize");

        // When deserialized.
        let back: Event = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original when re-serialized.
        let back_json = serde_json::to_string(&back).expect("re-serialize");
        assert_eq!(json, back_json);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn event_type_name_exhaustive_coverage() {
        // Given all Event variants.
        // Then subscribable events return their EventMsg TYPE_NAME.
        assert_eq!(
            Event::EventChatMessageSubmitted {
                payload: EventChatMessageSubmitted {
                    entry: ChatEntry::user("test"),
                }
            }
            .type_name(),
            Some(EventChatMessageSubmitted::TYPE_NAME)
        );
        assert_eq!(
            Event::EventApplicationReady.type_name(),
            Some(EventApplicationReady::TYPE_NAME)
        );
        assert_eq!(
            Event::EventExtensionStarting {
                payload: ExtensionStarting {
                    name: "ext-a".into(),
                }
            }
            .type_name(),
            Some(ExtensionStarting::TYPE_NAME)
        );
        assert_eq!(
            Event::EventExtensionStarted {
                payload: ExtensionStarted {
                    name: "ext-a".into(),
                }
            }
            .type_name(),
            Some(ExtensionStarted::TYPE_NAME)
        );
        assert_eq!(
            Event::EventExtensionShutdownCompleted {
                payload: ExtensionShutdownCompleted {
                    name: "ext-a".into(),
                }
            }
            .type_name(),
            Some(ExtensionShutdownCompleted::TYPE_NAME)
        );
        assert_eq!(
            Event::EventApplicationShuttingDown.type_name(),
            Some(EventApplicationShuttingDown::TYPE_NAME)
        );

        // Then EventCustom uses the dynamic name.
        assert_eq!(
            Event::EventCustom {
                payload: EventCustom {
                    name: "my-event".to_string(),
                    data: serde_json::json!(null),
                }
            }
            .type_name(),
            Some("my-event")
        );

        // Then non-subscribable events return None.
        assert_eq!(
            Event::EventKeyDown {
                payload: EventKeyDown {
                    key: KeyEvent {
                        key: Key::Enter,
                        modifiers: Modifiers::none(),
                    },
                }
            }
            .type_name(),
            None
        );
        assert_eq!(
            Event::EventKeyUp {
                payload: EventKeyUp {
                    key: KeyEvent {
                        key: Key::Char('a'),
                        modifiers: Modifiers::none(),
                    },
                }
            }
            .type_name(),
            None
        );
        assert_eq!(
            Event::EventModeChanged {
                payload: EventModeChanged {
                    from: Mode::Normal,
                    to: Mode::Input,
                }
            }
            .type_name(),
            None
        );

        // Then TYPE_NAME constants match the expected string values.
        assert_eq!(
            EventChatMessageSubmitted::TYPE_NAME,
            "EventChatMessageSubmitted"
        );
        assert_eq!(EventApplicationReady::TYPE_NAME, "EventApplicationReady");
        assert_eq!(ExtensionStarting::TYPE_NAME, "EventExtensionStarting");
        assert_eq!(ExtensionStarted::TYPE_NAME, "EventExtensionStarted");
        assert_eq!(
            ExtensionShutdownCompleted::TYPE_NAME,
            "EventExtensionShutdownCompleted"
        );
        assert_eq!(
            EventApplicationShuttingDown::TYPE_NAME,
            "EventApplicationShuttingDown"
        );
    }
}
