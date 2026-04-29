//! Extension event routing and custom event payloads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Marker trait for event payloads that extensions can subscribe to.
///
/// Each implementation provides a [`TYPE_NAME`](EventMsg::TYPE_NAME)
/// constant used for string-based routing (in-memory and process host).
pub trait EventMsg: Send + Sync + 'static {
    /// The subscription-relevant type name for event routing.
    const TYPE_NAME: &'static str;
}

/// A custom event from an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCustom {
    /// The event name.
    pub name: String,
    /// The event data.
    pub data: Value,
}
