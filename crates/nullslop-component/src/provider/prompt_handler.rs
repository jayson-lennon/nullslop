//! Prompt assembly completion and strategy switch handlers.
//!
//! [`PromptAssemblyHandler`] receives [`PromptAssembled`] events from the prompt
//! assembly actor and submits the assembled messages to the LLM provider.
//! When a system prompt is present, it is prepended as an `LlmMessage::System`.
//!
//! It also receives [`PromptStrategySwitched`] events to sync the active strategy
//! ID into session state.

use crate::AppState;
use npr::context::{PromptAssembled, PromptStrategySwitched, StrategyStateUpdated};
use npr::provider::{LlmMessage, SendToLlmProvider};
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct PromptAssemblyHandler;

    commands {}

    events {
        PromptAssembled: on_prompt_assembled,
        PromptStrategySwitched: on_prompt_strategy_switched,
        StrategyStateUpdated: on_strategy_state_updated,
    }
}

impl PromptAssemblyHandler {
    /// Handle prompt assembly completion by starting the send phase.
    fn on_prompt_assembled(
        evt: &PromptAssembled,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) {
        let session = ctx.state.session_mut(&evt.session_id);
        session.finish_assembling();
        session.begin_sending();

        let mut messages = Vec::new();
        if let Some(ref system_prompt) = evt.system_prompt {
            messages.push(LlmMessage::System {
                content: system_prompt.clone(),
            });
        }
        messages.extend(evt.messages.clone());

        ctx.out.submit_command(npr::Command::SendToLlmProvider {
            payload: SendToLlmProvider {
                session_id: evt.session_id.clone(),
                messages,
                provider_id: None,
            },
        });
    }

    /// Handle a strategy switch by updating the session's active strategy.
    fn on_prompt_strategy_switched(
        evt: &PromptStrategySwitched,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) {
        let session = ctx.state.session_mut(&evt.session_id);
        session.switch_strategy(evt.strategy_id.clone());
    }

    /// Persist an updated strategy state blob.
    fn on_strategy_state_updated(
        evt: &StrategyStateUpdated,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) {
        ctx.state.strategy_state.insert(
            (evt.session_id.clone(), evt.strategy_id.clone()),
            evt.blob.clone(),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Event;
    use npr::context::StrategyStateUpdated;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;
    use nullslop_services::Services;

    use super::*;
    use crate::test_utils;

    #[test]
    fn strategy_state_updated_stores_blob() {
        // Given a bus with PromptAssemblyHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        PromptAssemblyHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let session_id = state.active_session.clone();
        let strategy_id = npr::PromptStrategyId::compaction();

        // When processing a StrategyStateUpdated event.
        bus.submit_event(Event::StrategyStateUpdated {
            payload: StrategyStateUpdated {
                session_id: session_id.clone(),
                strategy_id: strategy_id.clone(),
                blob: serde_json::json!({"compaction_count": 3}),
            },
        });
        bus.process_events(&mut state, &services);

        // Then the blob is stored in AppState.
        let key = (session_id, strategy_id);
        assert!(state.strategy_state.contains_key(&key));
        assert_eq!(state.strategy_state[&key]["compaction_count"], 3);
    }

    #[test]
    fn strategy_state_updated_overwrites_existing() {
        // Given a bus with PromptAssemblyHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        PromptAssemblyHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let session_id = state.active_session.clone();
        let strategy_id = npr::PromptStrategyId::compaction();

        // When processing two StrategyStateUpdated events for the same key.
        let key = (session_id.clone(), strategy_id.clone());
        for count in [1, 2] {
            bus.submit_event(Event::StrategyStateUpdated {
                payload: StrategyStateUpdated {
                    session_id: session_id.clone(),
                    strategy_id: strategy_id.clone(),
                    blob: serde_json::json!({"compaction_count": count}),
                },
            });
            bus.process_events(&mut state, &services);
        }

        // Then the second update overwrites the first.
        assert_eq!(state.strategy_state[&key]["compaction_count"], 2);
    }
}
