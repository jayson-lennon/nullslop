//! Strategy identification types.
//!
//! [`PromptStrategyId`] is a wire type used in commands and events to identify
//! which prompt assembly strategy a session should use.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies a prompt assembly strategy.
///
/// Used as a key to look up the factory that creates the strategy instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromptStrategyId(String);

impl PromptStrategyId {
    /// Create a new strategy ID.
    #[must_use]
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// The passthrough strategy ID.
    #[must_use]
    pub fn passthrough() -> Self {
        Self::new("passthrough")
    }

    /// The sliding window strategy ID.
    #[must_use]
    pub fn sliding_window() -> Self {
        Self::new("sliding_window")
    }

    /// The token budget strategy ID.
    #[must_use]
    pub fn token_budget() -> Self {
        Self::new("token_budget")
    }

    /// The compaction strategy ID.
    #[must_use]
    pub fn compaction() -> Self {
        Self::new("compaction")
    }
}

impl fmt::Display for PromptStrategyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_strategy_id_passthrough_is_passthrough() {
        let id = PromptStrategyId::passthrough();
        assert_eq!(id.to_string(), "passthrough");
    }

    #[test]
    fn prompt_strategy_id_equality() {
        let id1 = PromptStrategyId::new("sliding_window");
        let id2 = PromptStrategyId::new("sliding_window");
        assert_eq!(id1, id2);
    }

    #[test]
    fn prompt_strategy_id_serialization_roundtrip() {
        let id = PromptStrategyId::passthrough();
        let json = serde_json::to_string(&id).expect("serialize");
        let back: PromptStrategyId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }
}
