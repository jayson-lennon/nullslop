//! Wire protocol codec for extension↔host communication.
//!
//! Extensions communicate with the host via JSON lines over stdio.
//! Each line on stdin is an [`InboundMessage`] from the host;
//! each line on stdout is an [`OutboundMessage`] from the extension.

use std::io::Write;

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use wherror::Error;

/// Codec error for message read/write failures.
#[derive(Debug, Error)]
#[error(debug)]
pub struct CodecError;

/// Inbound message from host to extension (read from stdin).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InboundMessage {
    /// Host instructs extension to activate.
    #[serde(rename = "initialize")]
    Initialize,
    /// Host dispatches a command to the extension.
    #[serde(rename = "command")]
    Command {
        /// The command to handle.
        command: nullslop_protocol::Command,
    },
    /// Host sends a subscribed event to the extension.
    #[serde(rename = "event")]
    Event {
        /// The event payload.
        event: nullslop_protocol::Event,
    },
    /// Host requests graceful shutdown.
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Outbound message from extension to host (written to stdout).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutboundMessage {
    /// Extension registers its commands and subscriptions.
    #[serde(rename = "register")]
    Register {
        /// Command names this extension handles.
        commands: Vec<String>,
        /// Event type names this extension subscribes to.
        subscriptions: Vec<String>,
    },
    /// Extension sends a command to the host.
    #[serde(rename = "command")]
    Command {
        /// The command to send.
        command: nullslop_protocol::Command,
    },
    /// Extension sends an event to the host.
    #[serde(rename = "event")]
    Event {
        /// The event to send.
        event: nullslop_protocol::Event,
    },
}

/// Reads the next inbound message from stdin.
///
/// Blocks until a complete line is available. Returns `None` on EOF
/// (host closed stdin).
///
/// # Errors
///
/// Returns [`CodecError`] if the line cannot be read or parsed.
pub fn read_message() -> Result<Option<InboundMessage>, Report<CodecError>> {
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) => {
            let line = line.trim();
            serde_json::from_str(line)
                .change_context(CodecError)
                .attach("failed to parse inbound message")
                .map(Some)
        }
        Err(e) => Err(Report::new(CodecError)
            .attach(e)
            .attach("failed to read from stdin")),
    }
}

/// Writes an outbound message to stdout.
///
/// Serializes the message as a single JSON line and flushes stdout.
///
/// # Errors
///
/// Returns [`CodecError`] if serialization or I/O fails.
pub fn write_message(msg: &OutboundMessage) -> Result<(), Report<CodecError>> {
    let json = serde_json::to_string(msg)
        .change_context(CodecError)
        .attach("failed to serialize outbound message")?;
    println!("{json}");
    std::io::stdout()
        .flush()
        .change_context(CodecError)
        .attach("failed to flush stdout")?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::match_wildcard_for_single_variants)]
mod tests {
    use super::*;
    use npr::command::CustomCommand;
    use nullslop_protocol as npr;
    use nullslop_protocol::Command;

    #[test]
    fn inbound_initialize_roundtrip() {
        // Given an Initialize message.
        let msg = InboundMessage::Initialize;

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: InboundMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert!(matches!(back, InboundMessage::Initialize));
    }

    #[test]
    fn inbound_command_roundtrip() {
        // Given a Command message.
        let msg = InboundMessage::Command {
            command: Command::CustomCommand {
                payload: CustomCommand {
                    name: "echo".to_string(),
                    args: serde_json::json!({"text": "hi"}),
                },
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: InboundMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it preserves the command.
        match back {
            InboundMessage::Command { command } => match command {
                Command::CustomCommand { payload } => assert_eq!(payload.name, "echo"),
                other => panic!("expected CustomCommand, got {other:?}"),
            },
            other => panic!("expected Command, got {other:?}"),
        }
    }

    #[test]
    fn outbound_register_roundtrip() {
        // Given a Register message.
        let msg = OutboundMessage::Register {
            commands: vec!["echo".to_string()],
            subscriptions: vec!["NewChatEntry".to_string()],
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: OutboundMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it preserves commands and subscriptions.
        match back {
            OutboundMessage::Register {
                commands,
                subscriptions,
            } => {
                assert_eq!(commands, vec!["echo"]);
                assert_eq!(subscriptions, vec!["NewChatEntry"]);
            }
            other => panic!("expected Register, got {other:?}"),
        }
    }

    #[test]
    fn outbound_command_roundtrip() {
        // Given a Command outbound message.
        let msg = OutboundMessage::Command {
            command: Command::AppQuit,
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: OutboundMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it preserves the command.
        match back {
            OutboundMessage::Command { command } => {
                assert!(matches!(command, Command::AppQuit));
            }
            other => panic!("expected Command, got {other:?}"),
        }
    }

    #[test]
    fn outbound_event_roundtrip() {
        // Given an Event outbound message.
        let msg = OutboundMessage::Event {
            event: nullslop_protocol::Event::EventApplicationReady,
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: OutboundMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it preserves the event.
        match back {
            OutboundMessage::Event { event } => {
                assert!(matches!(
                    event,
                    nullslop_protocol::Event::EventApplicationReady
                ));
            }
            other => panic!("expected Event, got {other:?}"),
        }
    }
}
