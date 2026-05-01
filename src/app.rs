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
use nullslop_core::{ActorMessageSink, AppCore};
use nullslop_echo::EchoActor;
use nullslop_llm::LlmActor;
use nullslop_protocol::Event;
use nullslop_protocol::actor::{ActorStarted, ActorStarting};
use nullslop_services::providers::ApiKey;
use nullslop_services::providers::LlmServiceFactoryService;
use nullslop_services::providers::OpenRouterLlmServiceFactory;
use nullslop_services::providers::SampleLlmServiceFactory;
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
    /// Returns an error if the runner fails or if the API key is missing.
    pub fn dispatch(&mut self, cli: Cli, api_key: String) -> Result<(), Report<AppError>> {
        use nullslop_cli::cli::{Commands, HeadlessCommands};

        let llm_service = if cli.fake_llm {
            create_sample_llm_service()
        } else {
            create_llm_service(&api_key)?
        };

        match cli.command.unwrap_or(Commands::Tui) {
            Commands::Tui => {
                let (core, host_arc) =
                    create_core_with_actor_host(&self.handle(), llm_service.clone());
                let services =
                    nullslop_services::Services::new(self.handle(), host_arc, llm_service);
                let runner = Runner::Tui(Box::new(nullslop_tui::TuiApp::new_with_core(
                    services, core,
                )));
                runner.run().change_context(AppError)?;
            }
            Commands::Headless { command, .. } => {
                let (core, host_arc) =
                    create_core_with_actor_host(&self.handle(), llm_service.clone());
                let services =
                    nullslop_services::Services::new(self.handle(), host_arc, llm_service);
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

/// Creates the LLM service factory from the provided API key.
///
/// If the key is empty, returns an error — the program cannot function
/// without an LLM backend.
///
/// # Errors
///
/// Returns an error if `api_key` is empty.
fn create_llm_service(api_key: &str) -> Result<LlmServiceFactoryService, Report<AppError>> {
    if api_key.is_empty() {
        return Err(Report::new(AppError))
            .attach("OPENROUTER_API_KEY environment variable is required");
    }

    tracing::info!("using OpenRouter LLM backend");
    Ok(LlmServiceFactoryService::new(Arc::new(
        OpenRouterLlmServiceFactory::with_key_and_model(
            ApiKey::new(api_key.to_string()),
            OpenRouterLlmServiceFactory::default_model().to_string(),
        ),
    )))
}

/// Creates the sample LLM service factory for `--fake-llm` mode.
fn create_sample_llm_service() -> LlmServiceFactoryService {
    tracing::info!("using Sample LLM backend (--fake-llm)");
    LlmServiceFactoryService::new(Arc::new(SampleLlmServiceFactory))
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
) -> (AppCore, Arc<dyn nullslop_actor_host::ActorHost>) {
    let mut core = AppCore::new();
    let mut registry = nullslop_component::AppUiRegistry::new();
    nullslop_component::register_all(&mut core.bus, &mut registry);

    // Create the message sink that bridges actor output to AppCore's channel.
    let sink = Arc::new(ActorMessageSink::new(core.sender()));

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
    llm_ctx.set_data(llm_service);
    let llm_actor = LlmActor::activate(&mut llm_ctx);
    let llm_result = spawn_actor("nullslop-llm", llm_actor, &llm_ref, llm_rx, llm_ctx, handle);

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

    let host =
        InMemoryActorHost::from_actors_with_handle(vec![echo_result, llm_result], handle.clone());
    let host_arc: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(host);
    core.set_actor_host(ActorHostService::new(host_arc.clone()));

    (core, host_arc)
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
            fake_llm: false,
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
        let result = app.dispatch(cli, "test-key-for-ci".to_string());

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
        let result = app.dispatch(cli, "test-key-for-ci".to_string());

        // Then an error is returned.
        assert!(result.is_err());
    }
}
