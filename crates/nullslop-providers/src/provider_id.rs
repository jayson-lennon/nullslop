//! Unique identifier for configured providers.
//!
//! [`ProviderId`] holds a `{name}/{model}` string (e.g., `"ollama/llama3"`,
//! `"openrouter/openai/gpt-oss-120b"`). Created during registry expansion,
//! one per model in each provider block. Used in protocol types, app state,
//! and the picker to unambiguously identify which provider+model is in play.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Uniquely identifies a configured provider+model combination.
///
/// Format: `{provider_name}/{model_name}` (e.g., `"ollama/llama3"`).
/// Used in protocol types, app state, and the picker.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(String);

impl ProviderId {
    /// Create a new provider ID from a string.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self(name)
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for ProviderId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_returns_inner() {
        // Given a ProviderId.
        let id = ProviderId::new("openrouter-gpt".to_owned());

        // When displaying.
        // Then the inner name is shown.
        assert_eq!(id.to_string(), "openrouter-gpt");
    }

    #[test]
    fn equality_works() {
        // Given two ProviderIds with the same name.
        let a = ProviderId::new("fast".to_owned());
        let b = ProviderId::new("fast".to_owned());

        // Then they are equal.
        assert_eq!(a, b);
    }

    #[test]
    fn from_string_creates_id() {
        // Given a String.
        let s = String::from("ollama");

        // When converting to ProviderId.
        let id = ProviderId::from(s);

        // Then it holds the value.
        assert_eq!(id.as_str(), "ollama");
    }
}
