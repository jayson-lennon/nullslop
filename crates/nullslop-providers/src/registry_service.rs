//! Service wrapper for the provider registry.
//!
//! Wraps [`ProviderRegistry`] in a shared, cheap-to-clone container.
//! All clones of [`ProviderRegistryService`] share the same underlying
//! registry via `Arc<RwLock<...>>`. Callers that need multiple operations
//! should use [`read`](Self::read) to acquire a guard and work with
//! `&ProviderRegistry` directly.

use std::sync::Arc;

use error_stack::Report;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;

use crate::api_keys::ApiKeys;
use crate::config::{AliasEntry, ProvidersConfig};
use crate::provider_id::ProviderId;
use crate::registry::ProviderRegistry;
use crate::resolved_provider::ResolvedProvider;
use crate::service::{LlmServiceError, LlmServiceFactory};

/// Shared service wrapper for the provider registry.
///
/// Wraps `ProviderRegistry` in an `Arc<RwLock<...>>` so that all clones
/// share the same data. Cloning is cheap — only an Arc refcount bump.
///
/// Follows the project's service wrapper pattern.
#[derive(Debug, Clone)]
pub struct ProviderRegistryService {
    /// The wrapped registry, protected by an [`RwLock`] for shared access.
    inner: Arc<RwLock<ProviderRegistry>>,
}

impl ProviderRegistryService {
    /// Creates a new service wrapper around the given registry.
    #[must_use]
    pub fn new(registry: ProviderRegistry) -> Self {
        Self {
            inner: Arc::new(RwLock::new(registry)),
        }
    }

    /// Returns a read guard to the underlying registry.
    ///
    /// Use this when you need to make multiple calls without
    /// repeated locking (e.g., `filtered_entries`).
    pub fn read(&self) -> RwLockReadGuard<'_, ProviderRegistry> {
        self.inner.read()
    }

    /// Returns all expanded (per-model) providers.
    ///
    /// Acquires a read guard and clones the resolved provider list.
    /// Acceptable for small datasets (typically 2–10 entries).
    #[must_use]
    pub fn providers(&self) -> Vec<ResolvedProvider> {
        self.read().providers().to_vec()
    }

    /// Returns all configured aliases.
    ///
    /// Acquires a read guard and clones the alias list.
    #[must_use]
    pub fn aliases(&self) -> Vec<AliasEntry> {
        self.read().aliases().to_vec()
    }

    /// Looks up a resolved provider by ID.
    ///
    /// Acquires a read guard and clones the entry if found.
    #[must_use]
    pub fn get(&self, id: &ProviderId) -> Option<ResolvedProvider> {
        self.read().get(id).cloned()
    }

    /// Checks whether a provider is available given the resolved API keys.
    #[must_use]
    pub fn is_available(&self, id: &ProviderId, api_keys: &ApiKeys) -> bool {
        self.read().is_available(id, api_keys)
    }

    /// Resolves an alias name to its target resolved provider.
    ///
    /// Acquires a read guard and clones the target entry if found.
    #[must_use]
    pub fn resolve_alias<S>(&self, alias_name: S) -> Option<ResolvedProvider>
    where
        S: AsRef<str>,
    {
        self.read().resolve_alias(alias_name.as_ref()).cloned()
    }

    /// Returns the configured default provider ID, if set and valid.
    #[must_use]
    pub fn default_provider_id(&self) -> Option<ProviderId> {
        self.read().default_provider_id()
    }

    /// Creates an `LlmServiceFactory` for the given provider.
    ///
    /// Acquires a read guard and delegates to the underlying registry.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not found or the factory
    /// cannot be built.
    pub fn create_factory(
        &self,
        id: &ProviderId,
        api_keys: &ApiKeys,
    ) -> Result<Box<dyn LlmServiceFactory>, Report<LlmServiceError>> {
        self.read().create_factory(id, api_keys)
    }

    /// Creates an `LlmServiceFactory` for a remote (cache-discovered) model.
    ///
    /// Delegates to [`ProviderRegistry::create_factory_for_model`].
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is not found or the factory cannot be built.
    pub fn create_factory_for_model(
        &self,
        provider_name: &str,
        model: &str,
        api_keys: &ApiKeys,
    ) -> Result<Box<dyn LlmServiceFactory>, Report<LlmServiceError>> {
        self.read().create_factory_for_model(provider_name, model, api_keys)
    }

    /// Updates the default provider in the config.
    pub fn set_default_provider(&self, name: Option<String>) {
        self.inner.write().set_default_provider(name);
    }

    /// Returns a snapshot of the current config for persistence.
    ///
    /// Acquires a read guard and clones the config. The caller
    /// passes this to [`ConfigStorageService::save`](crate::ConfigStorageService::save).
    #[must_use]
    pub fn config_snapshot(&self) -> ProvidersConfig {
        self.inner.read().config().clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::ProvidersConfig;
    use crate::registry_service::ProviderRegistryService;

    #[test]
    fn clone_shares_data() {
        // Given a service with one provider.
        let config = ProvidersConfig {
            providers: vec![crate::config::ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let registry = crate::registry::ProviderRegistry::from_config(config).expect("registry");
        let service = ProviderRegistryService::new(registry);
        let clone = service.clone();

        // When reading from both.
        let original_providers = service.providers();
        let cloned_providers = clone.providers();

        // Then both see the same data.
        assert_eq!(original_providers.len(), 1);
        assert_eq!(cloned_providers.len(), 1);
        assert_eq!(original_providers[0].name, "ollama");
        assert_eq!(original_providers[0].model, "llama3");
        assert_eq!(cloned_providers[0].name, "ollama");
    }

    /// Helper: build a service with an ollama (keyless) and openrouter (key-required) provider.
    fn service_with_providers() -> ProviderRegistryService {
        let config = ProvidersConfig {
            providers: vec![
                crate::config::ProviderEntry {
                    name: "ollama".to_owned(),
                    backend: "ollama".to_owned(),
                    models: vec!["llama3".to_owned()],
                    base_url: None,
                    api_key_env: None,
                    requires_key: false,
                },
                crate::config::ProviderEntry {
                    name: "openrouter".to_owned(),
                    backend: "openrouter".to_owned(),
                    models: vec!["gpt-4".to_owned()],
                    base_url: None,
                    api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
                    requires_key: true,
                },
            ],
            aliases: vec![crate::config::AliasEntry {
                name: "fast".to_owned(),
                target: "ollama/llama3".to_owned(),
            }],
            default_provider: None,
        };
        let registry = crate::registry::ProviderRegistry::from_config(config).expect("registry");
        ProviderRegistryService::new(registry)
    }

    #[test]
    fn providers_delegates_to_registry() {
        // Given a service with two providers.
        let service = service_with_providers();

        // When calling providers().
        let providers = service.providers();

        // Then both expanded providers are returned (one per model).
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].name, "ollama");
        assert_eq!(providers[0].model, "llama3");
        assert_eq!(providers[1].name, "openrouter");
        assert_eq!(providers[1].model, "gpt-4");
    }

    #[test]
    fn aliases_delegates_to_registry() {
        // Given a service with one alias.
        let service = service_with_providers();

        // When calling aliases().
        let aliases = service.aliases();

        // Then the alias is returned.
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "fast");
        assert_eq!(aliases[0].target, "ollama/llama3");
    }

    #[test]
    fn get_returns_entry_for_known_provider() {
        // Given a service with providers.
        let service = service_with_providers();

        // When looking up a known provider by full expanded ID.
        let entry = service.get(&crate::provider_id::ProviderId::new("ollama/llama3".to_owned()));

        // Then the resolved provider is returned with the correct name and model.
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.name, "ollama");
        assert_eq!(entry.model, "llama3");
    }

    #[test]
    fn get_returns_none_for_unknown() {
        // Given a service with providers.
        let service = service_with_providers();

        // When looking up an unknown provider.
        let entry = service.get(&crate::provider_id::ProviderId::new(
            "nonexistent/model".to_owned(),
        ));

        // Then None is returned.
        assert!(entry.is_none());
    }

    #[test]
    fn is_available_delegates_to_registry() {
        // Given a service with a keyless provider.
        let service = service_with_providers();
        let api_keys = crate::api_keys::ApiKeys::new();

        // When checking availability of the keyless provider.
        let id = crate::provider_id::ProviderId::new("ollama/llama3".to_owned());

        // Then it is available.
        assert!(service.is_available(&id, &api_keys));
    }

    #[test]
    fn resolve_alias_delegates_to_registry() {
        // Given a service with an alias.
        let service = service_with_providers();

        // When resolving the alias.
        let resolved = service.resolve_alias("fast");

        // Then the target resolved provider is returned.
        assert!(resolved.is_some());
        let resolved = resolved.unwrap();
        assert_eq!(resolved.name, "ollama");
        assert_eq!(resolved.model, "llama3");
    }

    #[test]
    fn default_provider_id_delegates_to_registry() {
        // Given a service with a configured default provider.
        let config = ProvidersConfig {
            providers: vec![crate::config::ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: Some("ollama/llama3".to_owned()),
        };
        let registry = crate::registry::ProviderRegistry::from_config(config).expect("registry");
        let service = ProviderRegistryService::new(registry);

        // When asking for the default provider.
        let id = service.default_provider_id();

        // Then the configured default is returned.
        assert!(id.is_some());
        assert_eq!(
            id.as_ref().map(crate::provider_id::ProviderId::as_str),
            Some("ollama/llama3")
        );
    }

    #[test]
    fn create_factory_delegates_to_registry() {
        // Given a service with a sample provider.
        let config = ProvidersConfig {
            providers: vec![crate::config::ProviderEntry {
                name: "sample".to_owned(),
                backend: "sample".to_owned(),
                models: vec!["sample".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let registry = crate::registry::ProviderRegistry::from_config(config).expect("registry");
        let service = ProviderRegistryService::new(registry);
        let api_keys = crate::api_keys::ApiKeys::new();

        // When creating a factory via the service.
        let id = crate::provider_id::ProviderId::new("sample/sample".to_owned());
        let factory = service.create_factory(&id, &api_keys);

        // Then it succeeds and returns a factory named "Sample".
        assert!(factory.is_ok());
        assert_eq!(factory.unwrap().name(), "Sample");
    }

    #[test]
    fn set_default_provider_updates_via_service() {
        // Given a service with a provider.
        let service = service_with_providers();

        // When setting the default provider.
        service.set_default_provider(Some("ollama/llama3".to_owned()));

        // Then default_provider_id returns the updated value.
        let id = service.default_provider_id();
        assert!(id.is_some());
        assert_eq!(
            id.as_ref().map(crate::provider_id::ProviderId::as_str),
            Some("ollama/llama3")
        );
    }

    #[test]
    fn config_snapshot_returns_current_config() {
        // Given a service with providers.
        let service = service_with_providers();

        // When modifying and taking a snapshot.
        service.set_default_provider(Some("ollama/llama3".to_owned()));
        let config = service.config_snapshot();

        // Then the snapshot reflects the change.
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.default_provider.as_deref(), Some("ollama/llama3"));
    }
}
