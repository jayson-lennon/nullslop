//! Event types for the component event pipeline.
//!
//! The [`Event`] enum is the unified type the host broadcasts to
//! inform internal handlers and actors about state changes and input.
//!
//! Individual event structs live in domain modules ([`chat_input`], [`system`],
//! [`custom`], [`actor`]). Consumers import structs directly from those modules —
//! this facade only re-exports infrastructure types.
//!
//! # When adding a new event
//!
//! Every new event struct **must** be added as a variant on the [`Event`] enum
//! below. Creating the struct alone is not enough — the bus broadcasts based on
//! enum variants, so a missing variant means the event is invisible to the system.

use serde::{Deserialize, Serialize};

// Re-export infrastructure types only. Domain structs are imported from their modules.
pub use crate::custom::EventMsg;

// Internal imports for enum definition, type_name(), and tests.
use crate::actor::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::chat_input::ChatEntrySubmitted;
use crate::provider::{ProviderSwitched, StreamCompleted};
use crate::system::{KeyDown, KeyUp, ModeChanged};

/// Every event the host can broadcast.
///
/// Actors subscribe to relevant variants; the host also
/// uses them internally to drive UI updates.
///
/// **When adding a new event struct**, you must add a corresponding variant to
/// this enum. An event struct defined in a domain module without an enum variant
/// here will not be broadcast by the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// A key was pressed down.
    #[serde(rename = "key_down")]
    KeyDown {
        /// Which key was pressed.
        #[serde(flatten)]
        payload: KeyDown,
    },
    /// A key was released.
    #[serde(rename = "key_up")]
    KeyUp {
        /// Which key was released.
        #[serde(flatten)]
        payload: KeyUp,
    },
    /// A chat entry was added to the conversation history.
    #[serde(rename = "chat_entry_submitted")]
    ChatEntrySubmitted {
        /// The chat entry that was added.
        #[serde(flatten)]
        payload: ChatEntrySubmitted,
    },
    /// The application mode changed.
    #[serde(rename = "mode_changed")]
    ModeChanged {
        /// The previous and new mode.
        #[serde(flatten)]
        payload: ModeChanged,
    },
    /// An actor is starting up.
    #[serde(rename = "actor_starting")]
    ActorStarting {
        /// Which actor is starting.
        #[serde(flatten)]
        payload: ActorStarting,
    },
    /// An actor has finished starting up.
    #[serde(rename = "actor_started")]
    ActorStarted {
        /// Which actor finished starting.
        #[serde(flatten)]
        payload: ActorStarted,
    },
    /// An actor has completed shutdown.
    #[serde(rename = "actor_shutdown_completed")]
    ActorShutdownCompleted {
        /// Which actor finished shutting down.
        #[serde(flatten)]
        payload: ActorShutdownCompleted,
    },
    /// A streaming LLM response completed.
    #[serde(rename = "stream_completed")]
    StreamCompleted {
        /// The session whose stream completed.
        #[serde(flatten)]
        payload: StreamCompleted,
    },
    /// The active provider was switched.
    #[serde(rename = "provider_switched")]
    ProviderSwitched {
        /// The provider switch confirmation.
        #[serde(flatten)]
        payload: ProviderSwitched,
    },
}

impl Event {
    /// Returns the subscription-relevant type name for event routing.
    #[must_use]
    pub fn type_name(&self) -> Option<&str> {
        match self {
            Self::ChatEntrySubmitted { .. } => Some(ChatEntrySubmitted::TYPE_NAME),
            Self::ActorStarting { .. } => Some(ActorStarting::TYPE_NAME),
            Self::ActorStarted { .. } => Some(ActorStarted::TYPE_NAME),
            Self::ActorShutdownCompleted { .. } => Some(ActorShutdownCompleted::TYPE_NAME),
            Self::KeyDown { .. } => Some(KeyDown::TYPE_NAME),
            Self::KeyUp { .. } => Some(KeyUp::TYPE_NAME),
            Self::ModeChanged { .. } => Some(ModeChanged::TYPE_NAME),
            Self::StreamCompleted { .. } => Some(StreamCompleted::TYPE_NAME),
            Self::ProviderSwitched { .. } => Some(ProviderSwitched::TYPE_NAME),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::StreamCompletedReason;
    use crate::{ChatEntry, Key, KeyEvent, Mode, Modifiers, SessionId};

    #[test]
    fn event_chat_entry_submitted_preserves_entry() {
        // Given a ChatEntrySubmitted event with a user entry.
        let entry = ChatEntry::user("hello");
        let event = Event::ChatEntrySubmitted {
            payload: ChatEntrySubmitted {
                session_id: SessionId::new(),
                entry,
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: Event = serde_json::from_str(&json).expect("deserialize");

        // Then entry text is preserved.
        match back {
            Event::ChatEntrySubmitted { payload } => {
                assert_eq!(
                    payload.entry.kind,
                    crate::ChatEntryKind::User("hello".to_owned())
                );
            }
            other => panic!("expected ChatEntrySubmitted, got {other:?}"),
        }
    }

    #[rstest::rstest]
    #[case::key_down(Event::KeyDown { payload: KeyDown { key: KeyEvent { key: Key::Char('a'), modifiers: Modifiers::none() } } })]
    #[case::key_up(Event::KeyUp { payload: KeyUp { key: KeyEvent { key: Key::Enter, modifiers: Modifiers::none() } } })]
    #[case::chat_submitted(Event::ChatEntrySubmitted { payload: ChatEntrySubmitted { session_id: SessionId::new(), entry: ChatEntry::user("test") } })]
    #[case::mode_changed(Event::ModeChanged { payload: ModeChanged { from: Mode::Normal, to: Mode::Input } })]
    #[case::actor_starting(Event::ActorStarting { payload: ActorStarting { name: "actor-a".into() } })]
    #[case::actor_started(Event::ActorStarted { payload: ActorStarted { name: "actor-a".into() } })]
    #[case::actor_shutdown_completed(Event::ActorShutdownCompleted { payload: ActorShutdownCompleted { name: "actor-a".into() } })]
    #[case::stream_completed(Event::StreamCompleted { payload: StreamCompleted { session_id: SessionId::new(), reason: StreamCompletedReason::Finished } })]
    #[case::provider_switched(Event::ProviderSwitched { payload: ProviderSwitched { provider_name: "Ollama".into() } })]
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
    fn event_type_name_exhaustive_coverage() {
        // Given all Event variants.
        // When calling type_name() on each variant.
        // Then subscribable events return their EventMsg TYPE_NAME.
        assert_eq!(
            Event::ChatEntrySubmitted {
                payload: ChatEntrySubmitted {
                    session_id: SessionId::new(),
                    entry: ChatEntry::user("test"),
                }
            }
            .type_name(),
            Some(ChatEntrySubmitted::TYPE_NAME)
        );
        assert_eq!(
            Event::ActorStarting {
                payload: ActorStarting {
                    name: "actor-a".into(),
                }
            }
            .type_name(),
            Some(ActorStarting::TYPE_NAME)
        );
        assert_eq!(
            Event::ActorStarted {
                payload: ActorStarted {
                    name: "actor-a".into(),
                }
            }
            .type_name(),
            Some(ActorStarted::TYPE_NAME)
        );
        assert_eq!(
            Event::ActorShutdownCompleted {
                payload: ActorShutdownCompleted {
                    name: "actor-a".into(),
                }
            }
            .type_name(),
            Some(ActorShutdownCompleted::TYPE_NAME)
        );

        // Then key and mode events return their TYPE_NAME.
        assert_eq!(
            Event::KeyDown {
                payload: KeyDown {
                    key: KeyEvent {
                        key: Key::Enter,
                        modifiers: Modifiers::none(),
                    },
                }
            }
            .type_name(),
            Some(KeyDown::TYPE_NAME)
        );
        assert_eq!(
            Event::KeyUp {
                payload: KeyUp {
                    key: KeyEvent {
                        key: Key::Char('a'),
                        modifiers: Modifiers::none(),
                    },
                }
            }
            .type_name(),
            Some(KeyUp::TYPE_NAME)
        );
        assert_eq!(
            Event::ModeChanged {
                payload: ModeChanged {
                    from: Mode::Normal,
                    to: Mode::Input,
                }
            }
            .type_name(),
            Some(ModeChanged::TYPE_NAME)
        );

        // Then TYPE_NAME constants match the expected module-scoped values.
        assert_eq!(
            ChatEntrySubmitted::TYPE_NAME,
            "chat_input::ChatEntrySubmitted"
        );
        assert_eq!(ActorStarting::TYPE_NAME, "actor::ActorStarting");
        assert_eq!(ActorStarted::TYPE_NAME, "actor::ActorStarted");
        assert_eq!(
            ActorShutdownCompleted::TYPE_NAME,
            "actor::ActorShutdownCompleted"
        );
        assert_eq!(KeyDown::TYPE_NAME, "system::KeyDown");
        assert_eq!(KeyUp::TYPE_NAME, "system::KeyUp");
        assert_eq!(ModeChanged::TYPE_NAME, "system::ModeChanged");

        // Then StreamCompleted has the correct TYPE_NAME.
        assert_eq!(StreamCompleted::TYPE_NAME, "provider::StreamCompleted");

        // Then ProviderSwitched has the correct TYPE_NAME.
        assert_eq!(ProviderSwitched::TYPE_NAME, "provider::ProviderSwitched");
    }
}
