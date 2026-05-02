//! Resolved provider — a single model expanded from a provider block.

use crate::provider_id::ProviderId;

/// A fully resolved provider entry — one per model.
///
/// Created by expanding each [`ProviderEntry`](crate::config::ProviderEntry)'s
/// `models` list. This is the internal representation used by the registry
/// for lookup, availability checks, and factory creation.
#[derive(Debug, Clone)]
pub struct ResolvedProvider {
    /// Unique ID in `{name}/{model}` format (e.g., `"ollama/llama3"`).
    pub id: ProviderId,
    /// Provider block name (e.g., `"ollama"`).
    pub name: String,
    /// Model identifier (e.g., `"llama3"`).
    pub model: String,
    /// Backend type string.
    pub backend: String,
    /// Optional base URL override.
    pub base_url: Option<String>,
    /// Environment variable name holding the API key.
    pub api_key_env: Option<String>,
    /// Whether this provider requires an API key.
    pub requires_key: bool,
}
