//! Test utilities for constructing [Services] with fake implementations.
//!
//! [`TestServices`] provides a builder that creates a [`Services`] instance
//! with all fake/noop implementations, suitable for unit tests that need
//! a [`Services`] but don't test provider behavior.

use std::sync::Arc;

use nullslop_actor_host::FakeActorHost;
use nullslop_providers::{
    ApiKeys, ApiKeysService, ConfigStorageService, InMemoryConfigStorage, LlmServiceFactoryService,
    ProviderRegistry, ProviderRegistryService, ProvidersConfig,
};
use tokio::runtime::Handle;

use crate::Services;

/// A builder for constructing [Services] with fake implementations for tests.
///
/// All services default to empty/noop implementations. Use the builder methods
/// to customize specific services when needed.
///
/// Uses a leaked tokio runtime — acceptable for unit tests.
///
/// # Example
///
/// ```ignore
/// let services = TestServices::builder().build();
/// let state = AppState::default();
/// ```
pub struct TestServices {
    /// Provider configuration for the registry.
    providers: ProvidersConfig,
    /// Custom tokio runtime handle (if provided).
    handle: Option<Handle>,
    /// Custom actor host (if provided).
    actor_host: Option<Arc<dyn nullslop_actor_host::ActorHost>>,
    /// Custom LLM service factory (if provided).
    llm_service: Option<LlmServiceFactoryService>,
}

impl Default for TestServices {
    fn default() -> Self {
        Self {
            providers: ProvidersConfig {
                providers: vec![],
                aliases: vec![],
                default_provider: None,
            },
            handle: None,
            actor_host: None,
            llm_service: None,
        }
    }
}

impl TestServices {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the provider configuration.
    #[must_use]
    pub fn with_providers(mut self, config: ProvidersConfig) -> Self {
        self.providers = config;
        self
    }

    /// Set a custom tokio runtime handle.
    #[must_use]
    pub fn handle(mut self, handle: Handle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Set a custom actor host.
    #[must_use]
    pub fn actor_host(mut self, host: Arc<dyn nullslop_actor_host::ActorHost>) -> Self {
        self.actor_host = Some(host);
        self
    }

    /// Set a custom LLM service factory.
    #[must_use]
    pub fn llm_service(mut self, service: LlmServiceFactoryService) -> Self {
        self.llm_service = Some(service);
        self
    }

    /// Build the [`Services`] instance.
    ///
    /// Leaks a tokio runtime if no custom handle is provided — acceptable for unit tests.
    ///
    /// # Panics
    ///
    /// Panics if the tokio runtime fails to create (extremely unlikely in tests).
    #[must_use]
    #[expect(clippy::expect_used, reason = "test-only code, panics are acceptable")]
    pub fn build(self) -> Services {
        let handle = self.handle.unwrap_or_else(|| {
            let rt = Box::leak(Box::new(
                tokio::runtime::Runtime::new().expect("test runtime"),
            ));
            rt.handle().clone()
        });

        let actor_host = self
            .actor_host
            .unwrap_or_else(|| Arc::new(FakeActorHost::new()));
        let llm = self.llm_service.unwrap_or_else(|| {
            LlmServiceFactoryService::new(Arc::new(nullslop_providers::FakeLlmServiceFactory::new(
                vec![],
            )))
        });
        let registry = ProviderRegistryService::new(
            ProviderRegistry::from_config(self.providers).expect("test registry"),
        );
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let config_storage = ConfigStorageService::new(Arc::new(InMemoryConfigStorage::new()));

        Services::new(handle, actor_host, llm, registry, api_keys, config_storage)
    }
}
