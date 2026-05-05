//! Provider switch handler — processes provider switch commands.
//!
//! Validates the target provider against the registry, creates a new factory,
//! swaps it in, updates `AppState`, and emits a confirmation event.
//!
//! Supports both static registry entries and remote (cache-discovered) models.

use std::sync::Arc;

use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_protocol::CommandAction;
use nullslop_protocol::provider::{ProviderSwitch, ProviderSwitched};
use nullslop_providers::ProviderId;
use nullslop_services::Services;

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
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let services = ctx.services;

        let id = ProviderId::new(cmd.provider_id.clone());

        // Validate and create factory while holding read guards.
        // Extract the provider name for the confirmation event.
        let provider_name = {
            let registry = services.provider_registry().read();
            let api_keys = services.api_keys().read();

            // Try static registry first.
            if let Some(entry) = registry.get(&id) {
                if !registry.is_available(&id, &api_keys) {
                    if let Some(session) = ctx.state.sessions.get_mut(&ctx.state.active_session) {
                        session.push_entry(npr::ChatEntry::system(format!(
                            "Provider '{}' is unavailable (API key not set).",
                            entry.name
                        )));
                    }
                    return CommandAction::Continue;
                }

                let Ok(new_factory) = registry.create_factory(&id, &api_keys) else {
                    if let Some(session) = ctx.state.sessions.get_mut(&ctx.state.active_session) {
                        session.push_entry(npr::ChatEntry::system(format!(
                            "Failed to create factory for provider '{}'.",
                            entry.name
                        )));
                    }
                    return CommandAction::Continue;
                };

                // Swap the factory while still in scope.
                services.llm_service().swap(Arc::from(new_factory));

                entry.name.clone()
            } else {
                // Not in static registry — try as a remote model.
                match Self::create_remote_factory(&cmd.provider_id, &registry, &api_keys) {
                    Ok((factory, name)) => {
                        services.llm_service().swap(Arc::from(factory));
                        name
                    }
                    Err(msg) => {
                        if let Some(session) = ctx.state.sessions.get_mut(&ctx.state.active_session)
                        {
                            session.push_entry(npr::ChatEntry::system(msg));
                        }
                        return CommandAction::Continue;
                    }
                }
            }
        };
        // Read guards dropped here.

        // Update active provider.
        ctx.state.active_provider.clone_from(&cmd.provider_id);

        // Persist the selection to config (best-effort).
        services
            .provider_registry()
            .set_default_provider(Some(cmd.provider_id.clone()));
        let config = services.provider_registry().config_snapshot();
        if let Err(e) = services.config_storage().save(&config) {
            tracing::warn!("failed to persist provider selection: {e:?}");
        }

        // Emit confirmation event.
        ctx.out.submit_event(npr::Event::ProviderSwitched {
            payload: ProviderSwitched { provider_name },
        });

        CommandAction::Continue
    }

    /// Attempts to create a factory for a remote (cache-discovered) model.
    ///
    /// Parses the `provider_id` as `{provider_name}/{model}`, finds the `ProviderEntry`
    /// in the registry config, and creates a `GenericLlmServiceFactory` dynamically.
    fn create_remote_factory(
        provider_id: &str,
        registry: &nullslop_providers::ProviderRegistry,
        api_keys: &nullslop_providers::ApiKeys,
    ) -> Result<(Box<dyn nullslop_providers::LlmServiceFactory>, String), String> {
        // Parse provider_name and model from the ID.
        // The model may itself contain slashes (e.g., "anthropic/claude-sonnet-4").
        let (provider_name, model) = provider_id
            .split_once('/')
            .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;
        let provider_name_owned = provider_name.to_owned();

        // Find the ProviderEntry by name and check availability.
        let entry = registry
            .config()
            .providers
            .iter()
            .find(|p| p.name == provider_name)
            .ok_or_else(|| format!("Unknown provider: {provider_id}"))?;

        if entry.requires_key {
            let Some(ref env_var) = entry.api_key_env else {
                return Err(format!(
                    "Provider '{provider_name}' is unavailable (no key env configured)."
                ));
            };
            if !api_keys.is_set(env_var) {
                return Err(format!(
                    "Provider '{provider_name}' is unavailable (API key not set)."
                ));
            }
        }

        let factory = registry
            .create_factory_for_model(provider_name, model, api_keys)
            .map_err(|e| {
                format!("Failed to create factory for remote model '{provider_id}': {e:?}")
            })?;

        Ok((factory, provider_name_owned))
    }
}
