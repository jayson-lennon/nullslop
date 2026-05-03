//! Top-level application state and dispatch.
//!
//! [`App`] is the root of the ownership hierarchy. It creates the tokio
//! runtime, builds shared [`Services`], and dispatches to the appropriate
//! [`Runner`] variant (TUI or headless).

use std::sync::Arc;

use error_stack::{Report, ResultExt};
use nullslop_actor::{Actor, ActorContext, ActorEnvelope, ActorRef, MessageSink};
use nullslop_actor_host::{ActorHostService, InMemoryActorHost, spawn_actor};
use nullslop_cli::Cli;
use nullslop_component::AppState;
use nullslop_component_core::Bus;
use nullslop_core::{ActorMessageSink, AppCore, AppMsg, State};
use nullslop_echo::EchoActor;
use nullslop_llm::LlmActor;
use nullslop_llm_discover::DiscoverActor;
use nullslop_protocol::Event;
use nullslop_protocol::actor::{ActorStarted, ActorStarting};
use nullslop_providers::ApiKeys;
use nullslop_providers::ApiKeysService;
use nullslop_providers::ConfigStorageService;
use nullslop_providers::FilesystemConfigStorage;
use nullslop_providers::LlmServiceFactoryService;
use nullslop_providers::ModelCache;
use nullslop_providers::NoProvidersAvailableFactory;
use nullslop_providers::ProviderId;
use nullslop_providers::ProviderRegistry;
use nullslop_providers::ProviderRegistryService;
use nullslop_providers::cache_path;
use nullslop_services::Services;
use tokio::runtime::Runtime;
use wherror::Error;

use crate::headless::HeadlessApp;
use crate::runner::Runner;

/// Error type for top-level application initialization.
#[derive(Debug, Error)]
#[error(debug)]
pub struct AppError;

/// Top-level application state.
///
/// Created once in [`main`](crate::main) and dispatched to whichever
/// runner handles the command. Owns the tokio runtime and delegates
/// to [`Runner`] variants.
pub struct App {
    /// The tokio runtime.
    runtime: Runtime,
}

impl App {
    /// Creates a new top-level app with a default multi-threaded runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokio runtime cannot be created.
    pub fn new() -> Result<Self, Report<AppError>> {
        let runtime = Runtime::new()
            .change_context(AppError)
            .attach("failed to create tokio runtime")?;
        Ok(Self { runtime })
    }

    /// Returns a handle to the tokio runtime for spawning tasks.
    #[must_use]
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }

    /// Dispatches the CLI command to the appropriate runner.
    ///
    /// # Errors
    ///
    /// Returns an error if the runner fails.
    pub fn dispatch(&mut self, cli: Cli) -> Result<(), Report<AppError>> {
        use nullslop_cli::cli::{Commands, HeadlessCommands};

        // Load config from providers.toml (auto-creates on first run).
        let config_storage =
            ConfigStorageService::new(Arc::new(FilesystemConfigStorage::default_path()));
        let provider_config = config_storage
            .load()
            .change_context(AppError)
            .attach("failed to load provider config")?;

        // Resolve API keys at startup from environment variables.
        let mut api_keys = ApiKeys::new();
        for provider in &provider_config.providers {
            if let Some(ref env_var) = provider.api_key_env
                && let Ok(value) = std::env::var(env_var)
                && !value.is_empty()
            {
                api_keys.insert(env_var.clone(), value);
            }
        }
        let resolved_api_keys = ApiKeysService::new(api_keys);

        // Build provider registry.
        let provider_registry = ProviderRegistryService::new(
            ProviderRegistry::from_config(provider_config).change_context(AppError)?,
        );

        // Determine initial provider and factory.
        let (llm_service, initial_provider) =
            resolve_initial_factory(&provider_registry, &resolved_api_keys);

        match cli.command.unwrap_or(Commands::Tui) {
            Commands::Tui => {
                let (core, services) = create_core_with_actor_host(
                    &self.handle(),
                    llm_service.clone(),
                    provider_registry.clone(),
                    resolved_api_keys.clone(),
                    config_storage.clone(),
                );
                core.state.write().active_provider = initial_provider;
                load_model_cache(&core);
                let runner = Runner::Tui(Box::new(nullslop_tui::TuiApp::new_with_core(
                    services, core,
                )));
                runner.run().change_context(AppError)?;
            }
            Commands::Headless { command, .. } => {
                let (core, services) = create_core_with_actor_host(
                    &self.handle(),
                    llm_service.clone(),
                    provider_registry,
                    resolved_api_keys,
                    config_storage,
                );
                core.state.write().active_provider = initial_provider;
                load_model_cache(&core);
                let mut headless = HeadlessApp::new(core, services);
                match command {
                    Some(HeadlessCommands::SendChat { message }) => {
                        headless.send_chat(&message).change_context(AppError)?;
                    }
                    Some(HeadlessCommands::Script { path }) => {
                        let file = std::fs::File::open(&path)
                            .change_context(AppError)
                            .attach("failed to open script file")?;
                        headless.run_script(file).change_context(AppError)?;
                    }
                    None => {}
                }
                let runner = Runner::Headless(Box::new(headless));
                runner.run().change_context(AppError)?;
            }
        }

        Ok(())
    }
}

/// Resolves the initial LLM factory and provider name at startup.
///
/// Tries the configured default provider first, then falls back to the
/// first available provider. If none are available, returns a
/// [`NoProvidersAvailableFactory`] that streams a helpful setup message.
fn resolve_initial_factory(
    registry: &ProviderRegistryService,
    api_keys: &ApiKeysService,
) -> (LlmServiceFactoryService, String) {
    let registry_guard = registry.read();
    let api_keys_guard = api_keys.read();

    // Try configured default.
    if let Some(id) = registry_guard.default_provider_id()
        && registry_guard.is_available(&id, &api_keys_guard)
        && let Ok(factory) = registry_guard.create_factory(&id, &api_keys_guard)
    {
        tracing::info!("using configured default provider: {}", id.as_str());
        return (
            LlmServiceFactoryService::new(Arc::from(factory)),
            id.to_string(),
        );
    }

    // Fallback: first available provider.
    for provider in registry_guard.providers() {
        let id = ProviderId::new(provider.name.clone());
        if registry_guard.is_available(&id, &api_keys_guard)
            && let Ok(factory) = registry_guard.create_factory(&id, &api_keys_guard)
        {
            tracing::info!("using first available provider: {}", provider.name);
            return (
                LlmServiceFactoryService::new(Arc::from(factory)),
                provider.name.clone(),
            );
        }
    }

    // No provider available — use the no-provider factory.
    tracing::warn!("no provider configured or available; use the picker to select one");
    (
        LlmServiceFactoryService::new(Arc::new(NoProvidersAvailableFactory)),
        nullslop_providers::NO_PROVIDER_ID.to_owned(),
    )
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("failed to create default App")
    }
}

/// Creates an `AppCore` with all components registered and the actor host started.
fn create_core_with_actor_host(
    handle: &tokio::runtime::Handle,
    llm_service: LlmServiceFactoryService,
    provider_registry: ProviderRegistryService,
    api_keys: ApiKeysService,
    config_storage: ConfigStorageService,
) -> (AppCore, Services) {
    // Create channel first — actors need the sender, but AppCore needs services
    // which needs the actor host which needs actors. Break the cycle by creating
    // the channel independently.
    let (sender, receiver) = kanal::unbounded::<AppMsg>();

    // Create the message sink that bridges actor output to AppCore's channel.
    let sink = Arc::new(ActorMessageSink::new(sender.clone()));

    // Create echo actor using two-phase startup.
    let (echo_tx, echo_rx) = kanal::unbounded::<ActorEnvelope<nullslop_echo::EchoDirectMsg>>();
    let echo_ref = ActorRef::new(echo_tx);
    let mut echo_ctx = ActorContext::new("nullslop-echo", sink.clone());
    let echo_actor = EchoActor::activate(&mut echo_ctx);
    let echo_result = spawn_actor(
        "nullslop-echo",
        echo_actor,
        &echo_ref,
        echo_rx,
        echo_ctx,
        handle,
    );

    // Create LLM actor with data injection.
    let (llm_tx, llm_rx) = kanal::unbounded::<ActorEnvelope<nullslop_llm::LlmDirectMsg>>();
    let llm_ref = ActorRef::new(llm_tx);
    let mut llm_ctx = ActorContext::new("nullslop-llm", sink.clone());
    llm_ctx.set_data(llm_service.clone());
    let llm_actor = LlmActor::activate(&mut llm_ctx);
    let llm_result = spawn_actor("nullslop-llm", llm_actor, &llm_ref, llm_rx, llm_ctx, handle);

    // Create discover actor with data injection.
    let (discover_tx, discover_rx) =
        kanal::unbounded::<ActorEnvelope<nullslop_llm_discover::DiscoverDirectMsg>>();
    let discover_ref = ActorRef::new(discover_tx);
    let mut discover_ctx = ActorContext::new("nullslop-llm-discover", sink.clone());
    discover_ctx.set_data(provider_registry.clone());
    discover_ctx.set_data(api_keys.clone());
    let discover_actor = DiscoverActor::activate(&mut discover_ctx);
    let discover_result = spawn_actor(
        "nullslop-llm-discover",
        discover_actor,
        &discover_ref,
        discover_rx,
        discover_ctx,
        handle,
    );

    // Emit lifecycle events.
    let _ = sink.send_event(Event::ActorStarting {
        payload: ActorStarting {
            name: "nullslop-echo".to_string(),
        },
    });
    let _ = sink.send_event(Event::ActorStarted {
        payload: ActorStarted {
            name: "nullslop-echo".to_string(),
        },
    });
    let _ = sink.send_event(Event::ActorStarting {
        payload: ActorStarting {
            name: "nullslop-llm".to_string(),
        },
    });
    let _ = sink.send_event(Event::ActorStarted {
        payload: ActorStarted {
            name: "nullslop-llm".to_string(),
        },
    });
    let _ = sink.send_event(Event::ActorStarting {
        payload: ActorStarting {
            name: "nullslop-llm-discover".to_string(),
        },
    });
    let _ = sink.send_event(Event::ActorStarted {
        payload: ActorStarted {
            name: "nullslop-llm-discover".to_string(),
        },
    });

    let host =
        InMemoryActorHost::from_actors_with_handle(vec![echo_result, llm_result, discover_result], handle.clone());
    let host_arc: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(host);

    // Build services with the actor host.
    let services = Services::new(
        handle.clone(),
        host_arc.clone(),
        llm_service,
        provider_registry,
        api_keys,
        config_storage,
    );

    // Build AppCore with services in its state.
    let mut core = AppCore {
        bus: Bus::new(),
        state: State::new(AppState::new(services.clone())),
        sender,
        receiver,
        actor_host: Some(ActorHostService::new(host_arc)),
    };
    let mut registry = nullslop_component::AppUiRegistry::new();
    nullslop_component::register_all(&mut core.bus, &mut registry);

    (core, services)
}

/// Loads the model cache from disk into the application state.
///
/// Called once after core creation. Failures are logged but not fatal —
/// the cache is optional and will be populated on first refresh.
fn load_model_cache(core: &AppCore) {
    let cache = ModelCache::load(&cache_path()).unwrap_or_else(|e| {
        tracing::warn!("failed to load model cache: {e:?}");
        None
    });
    if let Some(ref c) = cache {
        tracing::info!(providers = c.entries.len(), "loaded model cache");
    }
    let mut state = core.state.write();
    state.last_refreshed_at = cache.as_ref().and_then(|c| c.last_updated_at);
    state.model_cache = cache;
}

#[cfg(test)]
mod tests {
    use clap_verbosity_flag::Verbosity;
    use nullslop_cli::cli::{Cli, Commands, HeadlessCommands};

    use super::*;

    fn test_cli(command: Option<Commands>) -> Cli {
        Cli {
            verbosity: Verbosity::new(0, 0),
            log_dir: None,
            command,
        }
    }

    #[test]
    fn dispatch_headless_script_completes_successfully() {
        // Given a script file containing "q".
        let dir = tempfile::tempdir().expect("temp dir");
        let script_path = dir.path().join("test.script");
        std::fs::write(&script_path, "q").expect("write script");

        let mut app = App::new().expect("create app");
        let cli = test_cli(Some(Commands::Headless {
            log_file: None,
            command: Some(HeadlessCommands::Script {
                path: script_path.to_str().expect("path to str").to_string(),
            }),
        }));

        // When dispatching the headless script command.
        let result = app.dispatch(cli);

        // Then it completes without error.
        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_headless_script_returns_error_for_missing_file() {
        // Given a nonexistent script path.
        let mut app = App::new().expect("create app");
        let cli = test_cli(Some(Commands::Headless {
            log_file: None,
            command: Some(HeadlessCommands::Script {
                path: "/no/such/file.script".to_string(),
            }),
        }));

        // When dispatching the headless script command.
        let result = app.dispatch(cli);

        // Then an error is returned.
        assert!(result.is_err());
    }
}
