//! Provider events.

use serde::{Deserialize, Serialize};

use crate::tool::ToolCall;
use crate::EventMsg;
use crate::SessionId;

/// Why the stream completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamCompletedReason {
    /// The stream finished normally (all tokens received).
    Finished,
    /// The stream was cancelled by the user.
    Canceled,
    /// The stream stopped because the model requested tool use.
    ToolUse,
}

/// Streaming response completed for a session.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("provider")]
pub struct StreamCompleted {
    /// The session whose stream completed.
    pub session_id: SessionId,
    /// Why the stream completed.
    pub reason: StreamCompletedReason,
    /// Accumulated text content from the assistant response (populated when reason is `ToolUse`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assistant_content: Option<String>,
    /// Tool calls requested by the assistant (populated when reason is `ToolUse`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// The active provider was switched.
///
/// Emitted after a successful [`ProviderSwitch`](super::ProviderSwitch) command.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("provider")]
pub struct ProviderSwitched {
    /// The display name of the new provider.
    pub provider_name: String,
}

/// Models refresh completed with results and errors.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("provider")]
pub struct ModelsRefreshed {
    /// Provider name to list of discovered models.
    pub results: std::collections::HashMap<String, Vec<String>>,
    /// Provider name to error message for providers that failed.
    pub errors: std::collections::HashMap<String, String>,
}
