//! Compaction session data — state that persists across assembly invocations.
//!
//! In the full implementation, this will store compaction summaries keyed by
//! `ChatEntryId` ranges. For the stub, it carries a compaction counter
//! to validate the [`StrategySessionData`] round-trip path.

use error_stack::Report;
use serde::{Deserialize, Serialize};

use crate::strategy::types::{PromptAssemblyError, StrategySessionData};

/// Placeholder session data for the compaction strategy.
///
/// Validates that the strategy state persistence plumbing works end-to-end
/// before the full implementation adds complex state (summaries, entry ranges).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSessionData {
    /// The number of compactions performed (for future use).
    compaction_count: usize,
}

impl CompactionSessionData {
    /// Create new empty session data.
    #[must_use]
    pub fn new() -> Self {
        Self {
            compaction_count: 0,
        }
    }
}

impl Default for CompactionSessionData {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategySessionData for CompactionSessionData {
    fn serialize(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self).ok()
    }

    fn deserialize(
        value: serde_json::Value,
    ) -> Result<Box<dyn StrategySessionData>, Report<PromptAssemblyError>>
    where
        Self: Sized,
    {
        let data: CompactionSessionData = serde_json::from_value(value)
            .map_err(|e| Report::new(PromptAssemblyError).attach(e.to_string()))?;
        Ok(Box::new(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::types::StrategySessionData;

    #[test]
    fn compaction_session_data_serialize_roundtrip() {
        // Given compaction session data with a count.
        let data = CompactionSessionData {
            compaction_count: 3,
        };

        // When serializing and deserializing via StrategySessionData.
        let blob = StrategySessionData::serialize(&data).expect("serialize");
        let back = <CompactionSessionData as StrategySessionData>::deserialize(blob.clone())
            .expect("deserialize");

        // Then the data round-trips correctly (verify via re-serialization).
        let back_blob = StrategySessionData::serialize(&*back).expect("re-serialize");
        assert_eq!(blob, back_blob);
    }

    #[test]
    fn compaction_session_data_starts_at_zero() {
        // Given new compaction session data.
        let data = CompactionSessionData::new();

        // Then the compaction count is zero.
        assert_eq!(data.compaction_count, 0);
    }
}
