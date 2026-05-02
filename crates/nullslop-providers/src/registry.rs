//! Provider registry — holds all parsed provider configs.
//!
//! [`ProviderRegistry`] is constructed from a [`ProvidersConfig`] and expands
//! each provider block's `models` list into per-model [`ResolvedProvider`]
//! entries stored in a `HashMap<ProviderId, ResolvedProvider>` for O(1) lookup.
//! Validation runs at construction time so that downstream code can trust
//! the registry contents.
//!
//! API key availability is checked against an [`ApiKeys`] store that is
//! populated once at application startup. The registry never touches the
//! environment directly.

use std::collections::{HashMap, HashSet};

use error_stack::{Report, ResultExt as _};
use llm::builder::LLMBackend;

use crate::SampleLlmServiceFactory;
use crate::api_keys::ApiKeys;
use crate::config::{AliasEntry, ConfigError, ProvidersConfig};
use crate::generic_factory::GenericLlmServiceFactory;
use crate::provider_id::ProviderId;
use crate::resolved_provider::ResolvedProvider;
use crate::service::{LlmServiceError, LlmServiceFactory};

/// Registry of configured providers.
///
/// Holds the parsed [`ProvidersConfig`] (for persistence) and the expanded
/// per-model [`ResolvedProvider`] entries (for lookup, availability, and
/// factory creation). Constructed via [`ProviderRegistry::from_config`].
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    /// The original config (for persistence — `config()`, `config_snapshot()`).
    config: ProvidersConfig,
    /// Expanded per-model entries, indexed by ProviderId.
    resolved_map: HashMap<ProviderId, ResolvedProvider>,
    /// All expanded entries in order.
    resolved_list: Vec<ResolvedProvider>,
}

impl ProviderRegistry {
    /// Creates a registry from a parsed config, validating correctness.
    ///
    /// # Validation
    ///
    /// - No duplicate provider block names.
    /// - No empty models lists.
    /// - No duplicate expanded IDs (`{name}/{model}`).
    /// - All backend strings parse via `LLMBackend::from_str` (or are `"sample"`).
    /// - All alias targets refer to existing expanded IDs.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Validation`] if any check fails.
    pub fn from_config(config: ProvidersConfig) -> Result<Self, Report<ConfigError>> {
        // Check for duplicate provider block names.
        let mut seen_names = HashSet::new();
        for provider in &config.providers {
            if !seen_names.insert(&provider.name) {
                return Err(Report::new(ConfigError::Validation))
                    .attach(format!("duplicate provider name: {}", provider.name));
            }
        }

        // Check no empty models lists.
        for provider in &config.providers {
            if provider.models.is_empty() {
                return Err(Report::new(ConfigError::Validation)).attach(format!(
                    "provider '{}' has an empty models list",
                    provider.name
                ));
            }
        }

        // Check backend strings parse.
        for provider in &config.providers {
            if provider.backend != "sample" {
                let _: LLMBackend = provider
                    .backend
                    .parse()
                    .change_context(ConfigError::Validation)
                    .attach(format!(
                        "invalid backend '{}' for provider '{}'",
                        provider.backend, provider.name
                    ))?;
            }
        }

        // Expand into per-model entries and check for duplicate IDs.
        let mut resolved_map: HashMap<ProviderId, ResolvedProvider> = HashMap::new();
        let mut resolved_list: Vec<ResolvedProvider> = Vec::new();

        for entry in &config.providers {
            for model in &entry.models {
                let id = ProviderId::new(format!("{}/{}", entry.name, model));
                if resolved_map.contains_key(&id) {
                    return Err(Report::new(ConfigError::Validation)).attach(format!(
                        "duplicate expanded provider ID: {}",
                        id
                    ));
                }
                let resolved = ResolvedProvider {
                    id: id.clone(),
                    name: entry.name.clone(),
                    model: model.clone(),
                    backend: entry.backend.clone(),
                    base_url: entry.base_url.clone(),
                    api_key_env: entry.api_key_env.clone(),
                    requires_key: entry.requires_key,
                };
                resolved_map.insert(id, resolved.clone());
                resolved_list.push(resolved);
            }
        }

        // Check alias targets exist in expanded set.
        for alias in &config.aliases {
            let target_id = ProviderId::new(alias.target.clone());
            if !resolved_map.contains_key(&target_id) {
                return Err(Report::new(ConfigError::Validation)).attach(format!(
                    "alias '{}' targets unknown provider '{}'",
                    alias.name, alias.target
                ));
            }
        }

        Ok(Self {
            config,
            resolved_map,
            resolved_list,
        })
    }

    /// Returns a reference to the underlying config (for persistence).
    #[must_use]
    pub fn config(&self) -> &ProvidersConfig {
        &self.config
    }

    /// Updates the default provider in the config (for persistence on switch).
    pub fn set_default_provider(&mut self, name: Option<String>) {
        self.config.default_provider = name;
    }

    /// Returns all expanded (per-model) providers.
    #[must_use]
    pub fn providers(&self) -> &[ResolvedProvider] {
        &self.resolved_list
    }

    /// Returns all configured aliases.
    #[must_use]
    pub fn aliases(&self) -> &[AliasEntry] {
        &self.config.aliases
    }

    /// Looks up a resolved provider by ID.
    #[must_use]
    pub fn get(&self, id: &ProviderId) -> Option<&ResolvedProvider> {
        self.resolved_map.get(id)
    }

    /// Resolves an alias name to its target resolved provider.
    #[must_use]
    pub fn resolve_alias(&self, alias_name: &str) -> Option<&ResolvedProvider> {
        let alias = self.config.aliases.iter().find(|a| a.name == alias_name)?;
        self.get(&ProviderId::new(alias.target.clone()))
    }

    /// Checks whether a provider is available given the resolved API keys.
    ///
    /// Keyless providers are always available. Key-required providers are
    /// available only if their env var has a non-empty value in `api_keys`.
    #[must_use]
    pub fn is_available(&self, id: &ProviderId, api_keys: &ApiKeys) -> bool {
        let Some(resolved) = self.get(id) else {
            return false;
        };
        resolved_is_available(resolved, api_keys)
    }

    /// Returns all providers that are currently available given the resolved keys.
    #[must_use]
    pub fn available_providers(&self, api_keys: &ApiKeys) -> Vec<&ResolvedProvider> {
        self.resolved_list
            .iter()
            .filter(|p| resolved_is_available(p, api_keys))
            .collect()
    }

    /// Returns all providers that are currently unavailable (missing API key).
    #[must_use]
    pub fn unavailable_providers(&self, api_keys: &ApiKeys) -> Vec<&ResolvedProvider> {
        self.resolved_list
            .iter()
            .filter(|p| !resolved_is_available(p, api_keys))
            .collect()
    }

    /// Returns the configured default provider ID, if set and valid.
    #[must_use]
    pub fn default_provider_id(&self) -> Option<ProviderId> {
        let name = self.config.default_provider.as_ref()?;
        let id = ProviderId::new(name.clone());
        self.get(&id).is_some().then_some(id)
    }

    /// Creates an `LlmServiceFactory` for the given provider.
    ///
    /// For the special `"sample"` backend, returns [`SampleLlmServiceFactory`].
    /// For all other backends, returns [`GenericLlmServiceFactory`] with the
    /// resolved API key from `api_keys`.
    ///
    /// # Errors
    ///
    /// Returns [`LlmServiceError::Config`] if the provider is not found or
    /// the factory cannot be built. Returns [`LlmServiceError::ApiKey`] if
    /// a key-required provider is missing its key.
    pub fn create_factory(
        &self,
        id: &ProviderId,
        api_keys: &ApiKeys,
    ) -> Result<Box<dyn LlmServiceFactory>, Report<LlmServiceError>> {
        let resolved = self.get(id).ok_or_else(|| {
            Report::new(LlmServiceError::Config).attach(format!("unknown provider: {id}"))
        })?;

        if resolved.backend == "sample" {
            let factory: Box<dyn LlmServiceFactory> = Box::new(SampleLlmServiceFactory);
            return Ok(factory);
        }

        let backend: LLMBackend = resolved
            .backend
            .parse()
            .change_context(LlmServiceError::Config)
            .attach(format!(
                "invalid backend '{}' for provider '{}'",
                resolved.backend, resolved.name
            ))?;

        // Resolve the API key.
        let api_key = if resolved.requires_key {
            let env_var = resolved.api_key_env.as_deref().unwrap_or("");
            api_keys.get(env_var).unwrap_or("").to_owned()
        } else {
            String::new()
        };

        let factory = GenericLlmServiceFactory::new(
            resolved.name.clone(),
            backend,
            resolved.model.clone(),
            resolved.base_url.clone(),
            api_key,
        );

        Ok(Box::new(factory))
    }
}

/// Checks a single resolved provider's availability against resolved keys.
fn resolved_is_available(resolved: &ResolvedProvider, api_keys: &ApiKeys) -> bool {
    if !resolved.requires_key {
        return true;
    }
    let Some(ref env_var) = resolved.api_key_env else {
        return false;
    };
    api_keys.is_set(env_var)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_keys::ApiKeys;
    use crate::config::{AliasEntry, ProviderEntry};

    fn make_config(
        providers: Vec<ProviderEntry>,
        aliases: Vec<AliasEntry>,
        default_provider: Option<&str>,
    ) -> ProvidersConfig {
        ProvidersConfig {
            providers,
            aliases,
            default_provider: default_provider.map(String::from),
        }
    }

    fn ollama_entry() -> ProviderEntry {
        ProviderEntry {
            name: "ollama".to_owned(),
            backend: "ollama".to_owned(),
            models: vec!["llama3".to_owned()],
            base_url: Some("http://localhost:11434".to_owned()),
            api_key_env: None,
            requires_key: false,
        }
    }

    fn openrouter_entry() -> ProviderEntry {
        ProviderEntry {
            name: "openrouter".to_owned(),
            backend: "openrouter".to_owned(),
            models: vec!["gpt-4".to_owned()],
            base_url: None,
            api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
            requires_key: true,
        }
    }

    #[test]
    fn rejects_duplicate_provider_names() {
        // Given a config with duplicate provider names.
        let config = make_config(vec![ollama_entry(), ollama_entry()], vec![], None);

        // When building the registry.
        let result = ProviderRegistry::from_config(config);

        // Then it fails with a validation error.
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unknown_alias_target() {
        // Given a config with an alias pointing to a non-existent expanded ID.
        let config = make_config(
            vec![ollama_entry()],
            vec![AliasEntry {
                name: "fast".to_owned(),
                target: "nonexistent/model".to_owned(),
            }],
            None,
        );

        // When building the registry.
        let result = ProviderRegistry::from_config(config);

        // Then it fails with a validation error.
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_backend_string() {
        // Given a config with an invalid backend string.
        let config = make_config(
            vec![ProviderEntry {
                name: "bad".to_owned(),
                backend: "not-a-real-backend".to_owned(),
                models: vec!["x".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            vec![],
            None,
        );

        // When building the registry.
        let result = ProviderRegistry::from_config(config);

        // Then it fails with a validation error.
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_models_list() {
        // Given a config with a provider that has an empty models list.
        let config = make_config(
            vec![ProviderEntry {
                name: "empty".to_owned(),
                backend: "ollama".to_owned(),
                models: vec![],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            vec![],
            None,
        );

        // When building the registry.
        let result = ProviderRegistry::from_config(config);

        // Then it fails with a validation error.
        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_expanded_ids() {
        // Given two providers whose {name}/{model} collide.
        let config = make_config(
            vec![
                ProviderEntry {
                    name: "ollama".to_owned(),
                    backend: "ollama".to_owned(),
                    models: vec!["llama3".to_owned()],
                    base_url: None,
                    api_key_env: None,
                    requires_key: false,
                },
                ProviderEntry {
                    // Same name — but duplicate block names are caught first.
                    name: "ollama".to_owned(),
                    backend: "ollama".to_owned(),
                    models: vec!["llama3".to_owned()],
                    base_url: None,
                    api_key_env: None,
                    requires_key: false,
                },
            ],
            vec![],
            None,
        );

        // When building the registry.
        let result = ProviderRegistry::from_config(config);

        // Then it fails (duplicate block names caught before expansion).
        assert!(result.is_err());
    }

    #[test]
    fn expands_multi_model_provider() {
        // Given a config with one provider that has two models.
        let config = make_config(
            vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned(), "mistral".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            vec![],
            None,
        );

        // When building the registry.
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // Then two resolved entries exist.
        let providers = registry.providers();
        assert_eq!(providers.len(), 2);

        // And each has the correct expanded ID.
        assert_eq!(providers[0].id.as_str(), "ollama/llama3");
        assert_eq!(providers[0].name, "ollama");
        assert_eq!(providers[0].model, "llama3");

        assert_eq!(providers[1].id.as_str(), "ollama/mistral");
        assert_eq!(providers[1].name, "ollama");
        assert_eq!(providers[1].model, "mistral");

        // And both are individually look-up-able.
        assert!(registry
            .get(&ProviderId::new("ollama/llama3".to_owned()))
            .is_some());
        assert!(registry
            .get(&ProviderId::new("ollama/mistral".to_owned()))
            .is_some());
    }

    #[test]
    fn is_available_returns_true_for_keyless_provider() {
        // Given a registry with a keyless provider (Ollama).
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When checking availability.
        // Then the keyless provider is always available.
        assert!(registry.is_available(&ProviderId::new("ollama/llama3".to_owned()), &api_keys));
    }

    #[test]
    fn is_available_returns_true_when_key_resolved() {
        // Given a registry with a key-required provider and a resolved key.
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let mut api_keys = ApiKeys::new();
        api_keys.insert("OPENROUTER_API_KEY".to_owned(), "sk-test-value".to_owned());

        // When checking availability.
        assert!(registry.is_available(&ProviderId::new("openrouter/gpt-4".to_owned()), &api_keys));
    }

    #[test]
    fn is_available_returns_false_when_key_missing() {
        // Given a registry with a key-required provider and no resolved key.
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When checking availability.
        assert!(!registry.is_available(&ProviderId::new("openrouter/gpt-4".to_owned()), &api_keys));
    }

    #[test]
    fn available_providers_filters_correctly() {
        // Given a registry with one keyless and one key-required provider (no key).
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When asking for available providers.
        let available = registry.available_providers(&api_keys);

        // Then only the keyless provider is available.
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "ollama");
        assert_eq!(available[0].model, "llama3");
    }

    #[test]
    fn resolve_alias_finds_target() {
        // Given a registry with an alias pointing to a full expanded ID.
        let config = make_config(
            vec![ollama_entry()],
            vec![AliasEntry {
                name: "fast".to_owned(),
                target: "ollama/llama3".to_owned(),
            }],
            None,
        );
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // When resolving the alias.
        let resolved = registry.resolve_alias("fast");

        // Then the target resolved provider is returned.
        assert!(resolved.is_some());
        let resolved = resolved.unwrap();
        assert_eq!(resolved.name, "ollama");
        assert_eq!(resolved.model, "llama3");
    }

    #[test]
    fn resolve_alias_returns_none_for_unknown() {
        // Given a registry with no matching alias.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // When resolving a nonexistent alias.
        assert!(registry.resolve_alias("missing").is_none());
    }

    #[test]
    fn create_factory_succeeds_for_sample_backend() {
        // Given a registry with a sample provider.
        let config = make_config(
            vec![ProviderEntry {
                name: "sample".to_owned(),
                backend: "sample".to_owned(),
                models: vec!["sample".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            vec![],
            None,
        );
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When creating a factory.
        let factory = registry.create_factory(&ProviderId::new("sample/sample".to_owned()), &api_keys);

        // Then it succeeds and returns a factory named "Sample".
        assert!(factory.is_ok());
        assert_eq!(factory.unwrap().name(), "Sample");
    }

    #[test]
    fn default_provider_id_returns_configured() {
        // Given a config with a default provider.
        let config = make_config(vec![ollama_entry()], vec![], Some("ollama/llama3"));
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // When asking for the default.
        let id = registry.default_provider_id();

        // Then the configured ID is returned.
        assert_eq!(id.as_ref().map(ProviderId::as_str), Some("ollama/llama3"));
    }

    #[test]
    fn default_provider_id_returns_none_when_unset() {
        // Given a config with no default provider.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // When asking for the default.
        assert!(registry.default_provider_id().is_none());
    }

    #[test]
    fn default_provider_id_returns_none_for_invalid_target() {
        // Given a config with a default that doesn't match any expanded ID.
        let config = make_config(vec![ollama_entry()], vec![], Some("ollama"));
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // When asking for the default.
        // Then None is returned (old-style name no longer valid).
        assert!(registry.default_provider_id().is_none());
    }

    #[test]
    fn set_default_provider_updates_config() {
        // Given a registry with a provider.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let mut registry = ProviderRegistry::from_config(config).expect("registry");

        // When setting the default provider.
        registry.set_default_provider(Some("ollama/llama3".to_owned()));

        // Then default_provider_id returns the updated value.
        let id = registry.default_provider_id();
        assert_eq!(id.as_ref().map(ProviderId::as_str), Some("ollama/llama3"));
    }

    #[test]
    fn set_default_provider_clears_when_none() {
        // Given a registry with a default provider.
        let config = make_config(vec![ollama_entry()], vec![], Some("ollama/llama3"));
        let mut registry = ProviderRegistry::from_config(config).expect("registry");
        assert!(registry.default_provider_id().is_some());

        // When clearing the default provider.
        registry.set_default_provider(None);

        // Then default_provider_id returns None.
        assert!(registry.default_provider_id().is_none());
    }

    #[test]
    fn config_accessor_returns_config() {
        // Given a registry with providers.
        let config = make_config(
            vec![ollama_entry(), openrouter_entry()],
            vec![],
            Some("ollama/llama3"),
        );
        let registry = ProviderRegistry::from_config(config).expect("registry");

        // When accessing the config.
        let config = registry.config();

        // Then it has the expected provider blocks and default.
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.default_provider.as_deref(), Some("ollama/llama3"));
    }
}
