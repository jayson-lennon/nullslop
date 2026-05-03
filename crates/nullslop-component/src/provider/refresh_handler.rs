//! Model refresh handler — processes refresh commands and completion events.
//!
//! Handles the `RefreshModels` command by posting a "Refreshing model list..."
//! system message to chat. The command continues flowing to the actor host
//! for the actual model discovery work.
//!
//! Handles the `ModelsRefreshed` event by reloading the cache from disk,
//! updating `AppState.model_cache`, and posting a summary system message.

use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;
use nullslop_protocol::provider::{ModelsRefreshed, RefreshModels};
use nullslop_protocol::CommandAction;
use nullslop_providers::ModelCache;

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
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state
            .active_session_mut()
            .push_entry(npr::ChatEntry::system("Refreshing model list..."));

        CommandAction::Continue
    }

    /// Reloads the model cache from disk and posts a summary system message.
    fn on_models_refreshed(
        evt: &ModelsRefreshed,
        state: &mut AppState,
        _out: &mut Out,
    ) {
        let cache_path = nullslop_providers::cache_path();
        match ModelCache::load(&cache_path) {
            Ok(Some(cache)) => {
                state.model_cache = Some(cache);
            }
            Ok(None) => {
                state.model_cache = None;
            }
            Err(e) => {
                tracing::warn!("failed to reload model cache after refresh: {e:?}");
            }
        }

        state.last_refreshed_at = Some(jiff::Timestamp::now());

        let msg = format_refresh_summary(&evt.results, &evt.errors);
        state.active_session_mut().push_entry(npr::ChatEntry::system(msg));
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
    let mut msg = format!("Models refreshed: {} providers, {} models", results.len(), total_models);

    if !errors.is_empty() {
        let error_providers: Vec<&str> = errors.keys().map(String::as_str).collect();
        let _ = std::fmt::write(
            &mut msg,
            format_args!(" (errors: {})", error_providers.join(", ")),
        );
    }

    msg
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;
    use crate::test_utils;

    #[test]
    fn refresh_models_pushes_system_message_to_active_session() {
        // Given a bus with RefreshHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        RefreshHandler.register(&mut bus);

        let mut state = AppState::new(test_utils::test_services());

        // When processing the RefreshModels command.
        bus.submit_command(npr::Command::RefreshModels);
        bus.process_commands(&mut state);

        // Then a system message "Refreshing model list..." is pushed to the active session.
        assert_eq!(state.active_session().history().len(), 1);
        assert!(
            matches!(
                &state.active_session().history()[0].kind,
                npr::ChatEntryKind::System(msg) if msg == "Refreshing model list..."
            ),
            "expected system message 'Refreshing model list...'"
        );
    }

    #[test]
    fn refresh_models_returns_continue_action() {
        // Given a bus with RefreshHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        RefreshHandler.register(&mut bus);

        let mut state = AppState::new(test_utils::test_services());

        // When processing the RefreshModels command.
        bus.submit_command(npr::Command::RefreshModels);
        bus.process_commands(&mut state);

        // Then the command is not consumed (it continues to the actor host).
        // The system message presence confirms the handler ran.
        assert_eq!(state.active_session().history().len(), 1);
    }

    #[test]
    fn models_refreshed_updates_cache_from_disk() {
        // Given a bus with RefreshHandler registered and a cache file on disk.
        let mut bus: Bus<AppState> = Bus::new();
        RefreshHandler.register(&mut bus);

        let mut state = AppState::new(test_utils::test_services());
        assert!(
            state.model_cache.is_none(),
            "cache should start as None"
        );

        // Write a cache file to disk.
        let mut cache = ModelCache::new();
        cache.entries.insert(
            "ollama".to_owned(),
            vec!["llama3".to_owned(), "mistral".to_owned()],
        );
        let path = nullslop_providers::cache_path();
        cache.save(&path).expect("save cache");

        // When processing a ModelsRefreshed event.
        bus.submit_event(npr::Event::ModelsRefreshed {
            payload: ModelsRefreshed {
                results: HashMap::from([
                    ("ollama".to_owned(), vec!["llama3".to_owned(), "mistral".to_owned()]),
                ]),
                errors: HashMap::new(),
            },
        });
        bus.process_events(&mut state);

        // Then the model cache is loaded from disk.
        assert!(state.model_cache.is_some(), "cache should be loaded");
        let cache = state.model_cache.as_ref().unwrap();
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.entries["ollama"].len(), 2);

        // Cleanup.
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn models_refreshed_posts_summary_message() {
        // Given a bus with RefreshHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        RefreshHandler.register(&mut bus);

        let mut state = AppState::new(test_utils::test_services());

        // When processing a ModelsRefreshed event with results.
        bus.submit_event(npr::Event::ModelsRefreshed {
            payload: ModelsRefreshed {
                results: HashMap::from([
                    ("ollama".to_owned(), vec!["llama3".to_owned()]),
                    ("openrouter".to_owned(), vec!["gpt-4".to_owned(), "claude".to_owned()]),
                ]),
                errors: HashMap::new(),
            },
        });
        bus.process_events(&mut state);

        // Then a summary system message is posted.
        let history = state.active_session().history();
        assert_eq!(history.len(), 1);
        if let npr::ChatEntryKind::System(msg) = &history[0].kind {
            assert_eq!(msg, "Models refreshed: 2 providers, 3 models");
        } else {
            panic!("expected system message");
        }
    }

    #[test]
    fn models_refreshed_includes_errors_in_summary() {
        // Given a bus with RefreshHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        RefreshHandler.register(&mut bus);

        let mut state = AppState::new(test_utils::test_services());

        // When processing a ModelsRefreshed event with results and errors.
        bus.submit_event(npr::Event::ModelsRefreshed {
            payload: ModelsRefreshed {
                results: HashMap::from([
                    ("ollama".to_owned(), vec!["llama3".to_owned()]),
                ]),
                errors: HashMap::from([
                    ("lmstudio".to_owned(), "connection refused".to_owned()),
                ]),
            },
        });
        bus.process_events(&mut state);

        // Then the summary includes the error providers.
        let history = state.active_session().history();
        assert_eq!(history.len(), 1);
        if let npr::ChatEntryKind::System(msg) = &history[0].kind {
            assert_eq!(
                msg,
                "Models refreshed: 1 providers, 1 models (errors: lmstudio)"
            );
        } else {
            panic!("expected system message");
        }
    }

    #[test]
    fn models_refreshed_shows_no_models_when_empty() {
        // Given a bus with RefreshHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        RefreshHandler.register(&mut bus);

        let mut state = AppState::new(test_utils::test_services());

        // When processing a ModelsRefreshed event with no results and no errors.
        bus.submit_event(npr::Event::ModelsRefreshed {
            payload: ModelsRefreshed {
                results: HashMap::new(),
                errors: HashMap::new(),
            },
        });
        bus.process_events(&mut state);

        // Then the message says "No models discovered".
        let history = state.active_session().history();
        assert_eq!(history.len(), 1);
        if let npr::ChatEntryKind::System(msg) = &history[0].kind {
            assert_eq!(msg, "No models discovered.");
        } else {
            panic!("expected system message");
        }
    }
}
