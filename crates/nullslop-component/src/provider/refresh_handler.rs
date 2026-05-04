//! Model refresh handler — processes model discovery results.
//!
//! Handles the `RefreshModels` command (posts a status message) and the
//! `ModelsRefreshed` event (reloads the model cache from disk and posts
//! a summary). The actual discovery work is performed by an actor.

use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_protocol::CommandAction;
use nullslop_protocol::provider::{ModelsRefreshed, RefreshModels};
use nullslop_providers::ModelCache;
use nullslop_services::Services;

use crate::AppState;

define_handler! {
    pub(crate) struct RefreshHandler;

    commands {
        RefreshModels: on_refresh_models,
    }

    events {
        ModelsRefreshed: on_models_refreshed,
    }
}

impl RefreshHandler {
    /// Posts a "Refreshing model list..." system message to the active session.
    fn on_refresh_models(
        _cmd: &RefreshModels,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_session_mut()
            .push_entry(npr::ChatEntry::system("Refreshing model list..."));

        CommandAction::Continue
    }

    /// Reloads the model cache from disk and posts a summary system message.
    fn on_models_refreshed(
        evt: &ModelsRefreshed,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) {
        let cache_path = nullslop_providers::cache_path();
        match ModelCache::load(&cache_path) {
            Ok(Some(cache)) => {
                ctx.state.model_cache = Some(cache);
            }
            Ok(None) => {
                ctx.state.model_cache = None;
            }
            Err(e) => {
                tracing::warn!("failed to reload model cache after refresh: {e:?}");
            }
        }

        ctx.state.last_refreshed_at = Some(jiff::Timestamp::now());

        let msg = format_refresh_summary(&evt.results, &evt.errors);
        ctx.state
            .active_session_mut()
            .push_entry(npr::ChatEntry::system(msg));
    }
}

/// Formats a human-readable summary of the refresh results.
fn format_refresh_summary(
    results: &std::collections::HashMap<String, Vec<String>>,
    errors: &std::collections::HashMap<String, String>,
) -> String {
    if results.is_empty() && errors.is_empty() {
        return "No models discovered.".to_owned();
    }

    let total_models: usize = results.values().map(std::vec::Vec::len).sum();
    let mut msg = format!(
        "Models refreshed: {} providers, {} models",
        results.len(),
        total_models
    );

    if !errors.is_empty() {
        let error_providers: Vec<&str> = errors.keys().map(String::as_str).collect();
        let _ = std::fmt::write(
            &mut msg,
            format_args!(" (errors: {})", error_providers.join(", ")),
        );
    }

    msg
}

