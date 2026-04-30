//! Top-level application state and dispatch.
//!
//! [`App`] is the root of the ownership hierarchy. It creates the tokio
//! runtime, builds shared [`Services`], and dispatches to the appropriate
//! [`Runner`] variant (TUI or headless).

use std::sync::Arc;

use error_stack::{Report, ResultExt};
use nullslop_cli::Cli;
use nullslop_core::{AppCore, CoreExtSender, ExtensionHostService};
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

        match cli.command.unwrap_or(Commands::Tui) {
            Commands::Tui => {
                let (core, ext_arc) = create_core_with_ext_host(&self.handle());
                let services = nullslop_services::Services::new(self.handle(), ext_arc);
                let runner = Runner::Tui(Box::new(nullslop_tui::TuiApp::new_with_core(
                    services, core,
                )));
                runner.run().change_context(AppError)?;
            }
            Commands::Headless { command } => {
                let (core, ext_arc) = create_core_with_ext_host(&self.handle());
                let services = nullslop_services::Services::new(self.handle(), ext_arc);
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

impl Default for App {
    fn default() -> Self {
        Self::new().expect("failed to create default App")
    }
}

/// Creates an `AppCore` with all components registered and the extension host started.
fn create_core_with_ext_host(
    handle: &tokio::runtime::Handle,
) -> (AppCore, Arc<dyn nullslop_core::ExtensionHost>) {
    let mut core = AppCore::new();
    let mut registry = nullslop_component::AppUiRegistry::new();
    nullslop_component::register_all(&mut core.bus, &mut registry);

    let sender = CoreExtSender::new(core.sender());
    let echo_ext: Box<dyn nullslop_extension::InMemoryExtension> =
        Box::new(nullslop_echo::EchoExtension);
    let ext_host =
        nullslop_ext_host::InMemoryExtensionHost::start(Arc::new(sender), vec![echo_ext], handle);
    let ext_arc: Arc<dyn nullslop_core::ExtensionHost> = Arc::new(ext_host);
    core.set_ext_host(ExtensionHostService::new(ext_arc.clone()));

    (core, ext_arc)
}

#[cfg(test)]
mod tests {
    use nullslop_cli::cli::{Cli, Commands, HeadlessCommands};

    use super::*;

    #[test]
    fn dispatch_headless_script_completes_successfully() {
        // Given a script file containing "q".
        let dir = tempfile::tempdir().expect("temp dir");
        let script_path = dir.path().join("test.script");
        std::fs::write(&script_path, "q").expect("write script");

        let mut app = App::new().expect("create app");
        let cli = Cli {
            command: Some(Commands::Headless {
                command: Some(HeadlessCommands::Script {
                    path: script_path.to_str().expect("path to str").to_string(),
                }),
            }),
        };

        // When dispatching the headless script command.
        let result = app.dispatch(cli);

        // Then it completes without error.
        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_headless_script_returns_error_for_missing_file() {
        // Given a nonexistent script path.
        let mut app = App::new().expect("create app");
        let cli = Cli {
            command: Some(Commands::Headless {
                command: Some(HeadlessCommands::Script {
                    path: "/no/such/file.script".to_string(),
                }),
            }),
        };

        // When dispatching the headless script command.
        let result = app.dispatch(cli);

        // Then an error is returned.
        assert!(result.is_err());
    }
}
