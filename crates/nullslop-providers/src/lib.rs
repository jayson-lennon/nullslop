//! LLM service abstraction — streaming chat completions.
//!
//! Defines the [`LlmService`] trait for streaming LLM responses and
//! [`LlmServiceFactory`] for creating per-call service instances.
//! Includes an `OpenRouter` implementation, a sample provider for UI testing,
//! and a generic factory that supports any `LLMBackend` via config.

mod api_keys;
mod api_keys_service;
mod config;
mod config_storage;
mod convert;
mod fake;
mod generic_factory;
mod model_cache;
mod no_providers;
mod openrouter;
mod provider_id;
mod registry;
mod registry_service;
mod resolved_provider;
mod sample;
mod service;
mod service_wrapper;

pub use api_keys::ApiKeys;
pub use api_keys_service::ApiKeysService;
pub use config::{
    AliasEntry, ConfigError, ProviderEntry, ProvidersConfig, config_path, create_default_config,
    load_config, save_config,
};
pub use config_storage::{
    ConfigStorage, ConfigStorageService, FilesystemConfigStorage, InMemoryConfigStorage,
};
pub use convert::llm_messages_to_chat_messages;
pub use fake::FakeLlmServiceFactory;
pub use generic_factory::GenericLlmServiceFactory;
pub use model_cache::{ModelCache, ModelCacheError, cache_path};
pub use no_providers::{NO_PROVIDER_ID, NoProvidersAvailableFactory};
pub use openrouter::{ApiKey, OpenRouterLlmServiceFactory};
pub use provider_id::ProviderId;
pub use registry::ProviderRegistry;
pub use registry_service::ProviderRegistryService;
pub use resolved_provider::ResolvedProvider;
pub use sample::SampleLlmServiceFactory;
pub use service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory};
pub use service_wrapper::LlmServiceFactoryService;
