//! Application-wide runtime services.
//!
//! This crate defines the [`Services`] container, which holds long-lived
//! runtime infrastructure that subsystems need access to. It is created
//! once during startup and shared throughout the application.

pub use nullslop_providers as providers;

use std::sync::Arc;

use nullslop_actor_host::ActorHostService;
use nullslop_providers::{
    ApiKeysService, ConfigStorageService, LlmServiceFactoryService, ProviderRegistryService,
};
use tokio::runtime::Handle;

/// Runtime services shared across the application.
///
/// Holds references to all services, enabling dependency injection
/// and making it easy to swap implementations for testing.
#[derive(Debug, Clone)]
pub struct Services {
    /// Async runtime handle for spawning background tasks.
    handle: Handle,
    /// Actor host service.
    actor_host: ActorHostService,
    /// LLM service factory for creating streaming chat instances.
    llm_service: LlmServiceFactoryService,
    /// Provider registry for looking up and validating provider configs.
    provider_registry: ProviderRegistryService,
    /// Resolved API keys for provider availability checks and factory creation.
    api_keys: ApiKeysService,
    /// Config storage for persisting provider configuration.
    config_storage: ConfigStorageService,
}

impl Services {
    /// Creates a new `Services` with the given components.
    #[must_use]
    pub fn new(
        handle: Handle,
        actor_host: Arc<dyn nullslop_actor_host::ActorHost>,
        llm_service: LlmServiceFactoryService,
        provider_registry: ProviderRegistryService,
        api_keys: ApiKeysService,
        config_storage: ConfigStorageService,
    ) -> Self {
        Self {
            handle,
            actor_host: ActorHostService::new(actor_host),
            llm_service,
            provider_registry,
            api_keys,
            config_storage,
        }
    }

    /// Returns a reference to the async runtime handle.
    #[must_use]
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Returns a reference to the actor host service.
    #[must_use]
    pub fn actor_host(&self) -> &ActorHostService {
        &self.actor_host
    }

    /// Returns a reference to the LLM service factory.
    #[must_use]
    pub fn llm_service(&self) -> &LlmServiceFactoryService {
        &self.llm_service
    }

    /// Returns a reference to the provider registry service.
    #[must_use]
    pub fn provider_registry(&self) -> &ProviderRegistryService {
        &self.provider_registry
    }

    /// Returns a reference to the resolved API keys service.
    #[must_use]
    pub fn api_keys(&self) -> &ApiKeysService {
        &self.api_keys
    }

    /// Returns a reference to the config storage service.
    #[must_use]
    pub fn config_storage(&self) -> &ConfigStorageService {
        &self.config_storage
    }
}

pub mod test_services;
