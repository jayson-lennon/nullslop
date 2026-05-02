//! Provider switch handler — processes provider switch commands.
//!
//! Validates the target provider against the registry, creates a new factory,
//! swaps it in, updates `AppState`, and emits a confirmation event.

use std::sync::Arc;

use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;
use nullslop_protocol::CommandAction;
use nullslop_protocol::provider::{ProviderSwitch, ProviderSwitched};
use nullslop_providers::ProviderId;

use crate::AppState;

define_handler! {
    pub(crate) struct SwitchHandler;

    commands {
        ProviderSwitch: on_provider_switch,
    }

    events {}
}

impl SwitchHandler {
    /// Processes a provider switch command.
    fn on_provider_switch(
        cmd: &ProviderSwitch,
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        let services = &state.services;

        let id = ProviderId::new(cmd.provider_id.clone());

        // Validate and create factory while holding read guards.
        // Extract the provider name for the confirmation event.
        let provider_name = {
            let registry = services.provider_registry().read();
            let api_keys = services.api_keys().read();

            // Validate: provider must exist.
            let Some(entry) = registry.get(&id) else {
                if let Some(session) = state.sessions.get_mut(&state.active_session) {
                    session.push_entry(npr::ChatEntry::system(format!(
                        "Unknown provider: {}",
                        cmd.provider_id
                    )));
                }
                return CommandAction::Continue;
            };

            if !registry.is_available(&id, &api_keys) {
                if let Some(session) = state.sessions.get_mut(&state.active_session) {
                    session.push_entry(npr::ChatEntry::system(format!(
                        "Provider '{}' is unavailable (API key not set).",
                        entry.name
                    )));
                }
                return CommandAction::Continue;
            }

            let Ok(new_factory) = registry.create_factory(&id, &api_keys) else {
                if let Some(session) = state.sessions.get_mut(&state.active_session) {
                    session.push_entry(npr::ChatEntry::system(format!(
                        "Failed to create factory for provider '{}'.",
                        entry.name
                    )));
                }
                return CommandAction::Continue;
            };

            // Swap the factory while still in scope (read guards are compatible).
            services.llm_service().swap(Arc::from(new_factory));

            entry.name.clone()
        };
        // Read guards dropped here.

        // Update active provider.
        state.active_provider.clone_from(&cmd.provider_id);

        // Persist the selection to config (best-effort).
        services
            .provider_registry()
            .set_default_provider(Some(cmd.provider_id.clone()));
        let config = services.provider_registry().config_snapshot();
        if let Err(e) = services.config_storage().save(&config) {
            tracing::warn!("failed to persist provider selection: {e:?}");
        }

        // Emit confirmation event.
        out.submit_event(npr::Event::ProviderSwitched {
            payload: ProviderSwitched { provider_name },
        });

        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;
    use nullslop_protocol::provider::ProviderSwitch;
    use nullslop_providers::{
        ApiKeys, ApiKeysService, ConfigStorageService, InMemoryConfigStorage, ProviderEntry,
        ProviderRegistry, ProviderRegistryService, ProvidersConfig,
    };

    use super::*;
    use crate::AppState;

    /// Helper: create a `ConfigStorageService` for tests.
    fn test_config_storage() -> ConfigStorageService {
        ConfigStorageService::new(std::sync::Arc::new(InMemoryConfigStorage::new()))
    }

    /// Helper: create an [`AppState`] with a registry containing an "ollama" provider.
    fn state_with_registry() -> AppState {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let guard = rt.enter();

        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: Some("http://localhost:11434".to_owned()),
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let registry =
            ProviderRegistryService::new(ProviderRegistry::from_config(config).expect("registry"));
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let services = nullslop_services::Services::new(
            tokio::runtime::Handle::current(),
            std::sync::Arc::new(
                nullslop_actor_host::InMemoryActorHost::from_actors_with_handle(
                    vec![],
                    tokio::runtime::Handle::current(),
                ),
            ),
            nullslop_providers::LlmServiceFactoryService::new(std::sync::Arc::new(
                nullslop_providers::FakeLlmServiceFactory::new(vec![]),
            )),
            registry,
            api_keys,
            test_config_storage(),
        );
        let state = AppState::new(services);
        drop(guard);
        drop(rt);
        state
    }

    #[test]
    fn provider_switch_updates_active_provider() {
        // Given a bus with SwitchHandler registered and an AppState with services.
        let mut bus: Bus<AppState> = Bus::new();
        SwitchHandler.register(&mut bus);

        let mut state = state_with_registry();

        // When processing ProviderSwitch for "ollama/llama3".
        bus.submit_command(npr::Command::ProviderSwitch {
            payload: ProviderSwitch {
                provider_id: "ollama/llama3".to_owned(),
            },
        });
        bus.process_commands(&mut state);

        // Then active_provider is set.
        assert_eq!(state.active_provider, "ollama/llama3");
    }

    #[test]
    fn provider_switch_emits_switched_event() {
        // Given a bus with SwitchHandler registered and an AppState with services.
        let mut bus: Bus<AppState> = Bus::new();
        SwitchHandler.register(&mut bus);

        let mut state = state_with_registry();

        // When processing ProviderSwitch for "ollama/llama3".
        bus.submit_command(npr::Command::ProviderSwitch {
            payload: ProviderSwitch {
                provider_id: "ollama/llama3".to_owned(),
            },
        });
        bus.process_commands(&mut state);
        bus.process_events(&mut state);

        // Then a ProviderSwitched event is emitted.
        let events = bus.drain_processed_events();
        let switched = events
            .iter()
            .find(|e| matches!(e.event, npr::Event::ProviderSwitched { .. }));
        assert!(switched.is_some(), "expected ProviderSwitched event");
    }

    #[test]
    fn provider_switch_rejects_unknown_provider() {
        // Given a bus with SwitchHandler registered and an AppState with services.
        let mut bus: Bus<AppState> = Bus::new();
        SwitchHandler.register(&mut bus);

        let mut state = state_with_registry();

        // When processing ProviderSwitch for an unknown provider.
        bus.submit_command(npr::Command::ProviderSwitch {
            payload: ProviderSwitch {
                provider_id: "nonexistent".to_owned(),
            },
        });
        bus.process_commands(&mut state);

        // Then active_provider is NOT set and a system error is pushed.
        assert_eq!(state.active_provider, nullslop_providers::NO_PROVIDER_ID);
        assert_eq!(state.active_session().history().len(), 1);
        assert!(
            matches!(
                state.active_session().history()[0].kind,
                npr::ChatEntryKind::System(_)
            ),
            "expected system entry"
        );
    }

    #[test]
    fn provider_switch_rejects_unavailable_provider() {
        // Given a bus with SwitchHandler registered and an AppState with a key-required provider.
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let guard = rt.enter();

        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "openrouter".to_owned(),
                backend: "openrouter".to_owned(),
                models: vec!["gpt-4".to_owned()],
                base_url: None,
                api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
                requires_key: true,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let registry =
            ProviderRegistryService::new(ProviderRegistry::from_config(config).expect("registry"));
        let api_keys = ApiKeysService::new(ApiKeys::new()); // No keys set.
        let services = nullslop_services::Services::new(
            tokio::runtime::Handle::current(),
            std::sync::Arc::new(
                nullslop_actor_host::InMemoryActorHost::from_actors_with_handle(
                    vec![],
                    tokio::runtime::Handle::current(),
                ),
            ),
            nullslop_providers::LlmServiceFactoryService::new(std::sync::Arc::new(
                nullslop_providers::FakeLlmServiceFactory::new(vec![]),
            )),
            registry,
            api_keys,
            test_config_storage(),
        );
        let mut state = AppState::new(services);

        drop(guard);
        drop(rt);

        let mut bus: Bus<AppState> = Bus::new();
        SwitchHandler.register(&mut bus);

        // When processing ProviderSwitch for "openrouter/gpt-4" (no API key).
        bus.submit_command(npr::Command::ProviderSwitch {
            payload: ProviderSwitch {
                provider_id: "openrouter/gpt-4".to_owned(),
            },
        });
        bus.process_commands(&mut state);

        // Then active_provider is NOT set and a system error is pushed.
        assert_eq!(state.active_provider, nullslop_providers::NO_PROVIDER_ID);
        assert_eq!(state.active_session().history().len(), 1);
        assert!(
            matches!(
                state.active_session().history()[0].kind,
                npr::ChatEntryKind::System(_)
            ),
            "expected system entry"
        );
    }
}
