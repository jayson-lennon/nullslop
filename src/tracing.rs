//! Tracing initialization for nullslop.
//!
//! Sets up the global tracing subscriber based on the application's run mode.
//! In TUI mode, traces are written exclusively to a file to avoid corrupting
//! the terminal in raw mode. In headless mode, traces go to the terminal by
//! default, with an optional file layer.

use std::{env, fs::File, path::PathBuf, sync::Arc};

use clap_verbosity_flag::{Verbosity, WarnLevel};
use error_stack::{Report, ResultExt};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use wherror::Error;

/// Error type returned when tracing subscriber initialization fails.
#[derive(Debug, Error)]
#[error(debug)]
pub struct TracingInitError;

/// Log file name used in TUI mode.
const LOG_FILE_NAME: &str = "nullslop.log";

/// Decides how the tracing subscriber is configured based on run mode.
#[derive(Debug)]
pub enum TracingMode {
    /// TUI mode: file-only logging to avoid corrupting the terminal.
    Tui {
        /// Directory for the log file. Defaults to the current directory.
        log_dir: Option<PathBuf>,
    },
    /// Headless mode: terminal logging, optionally also to a file.
    Headless {
        /// Optional file path for additional file logging.
        log_file: Option<PathBuf>,
    },
}

/// Initializes the global tracing subscriber.
///
/// If the `RUST_LOG` environment variable is set, it takes precedence over
/// the verbosity parameter for filtering log output.
///
/// # Arguments
///
/// * `verbosity` - The verbosity level from CLI flags.
/// * `mode` - The [`TracingMode`] controlling where traces are written.
///
/// # Errors
///
/// Returns a [`TracingInitError`] if a log file cannot be opened.
///
/// # Panics
///
/// Panics if called more than once or if another tracer has already been set.
pub fn init(
    verbosity: Verbosity<WarnLevel>,
    mode: TracingMode,
) -> Result<(), Report<TracingInitError>> {
    let filter = match env::var("RUST_LOG") {
        Ok(filter_str) => filter_str,
        Err(_) => format!("nullslop={verbosity}"),
    };

    match mode {
        TracingMode::Tui { log_dir } => {
            let dir = log_dir.unwrap_or_else(|| PathBuf::from("."));
            let path = dir.join(LOG_FILE_NAME);

            let logfile = File::options()
                .create(true)
                .append(true)
                .open(&path)
                .change_context(TracingInitError)
                .attach_with(|| format!("failed to open file '{}' for tracing", path.display()))?;

            let file_layer = tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(true)
                .with_writer(Arc::new(logfile))
                .with_filter(EnvFilter::new(filter));

            tracing_subscriber::registry().with(file_layer).init();
        }
        TracingMode::Headless { log_file } => match log_file {
            Some(path) => {
                let logfile = File::options()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .change_context(TracingInitError)
                    .attach_with(|| {
                        format!("failed to open file '{}' for tracing", path.display())
                    })?;

                let file_layer: Box<dyn Layer<_> + Send + Sync + 'static> =
                    tracing_subscriber::fmt::layer()
                        .with_file(true)
                        .with_line_number(true)
                        .with_target(true)
                        .with_writer(Arc::new(logfile))
                        .with_filter(EnvFilter::new(filter.clone()))
                        .boxed();

                let terminal_layer =
                    tracing_subscriber::fmt::layer().with_filter(EnvFilter::new(filter));

                tracing_subscriber::registry()
                    .with(file_layer)
                    .with(terminal_layer)
                    .init();
            }
            None => {
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::new(filter))
                    .init();
            }
        },
    }

    tracing::info!("");
    tracing::info!("--- new session started ---");
    tracing::info!("");

    Ok(())
}
