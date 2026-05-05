//! Provider component — handles streaming LLM responses and message queuing.
//!
//! `ProviderHandler` processes [`StreamToken`] commands to track streaming state.
//! `MessageQueueHandler` manages the message queue: enqueue, dispatch, cancel-drain.
//! `StreamingIndicatorElement` shows the current streaming/sending/queue state.
//! `QueueDisplayElement` shows stacked dimmed "QUEUED:" entries.

pub mod handler;
pub mod indicator;
pub mod prompt_handler;
pub mod queue_element;
pub mod refresh_handler;
pub mod request_handler;
pub mod switch_handler;

use crate::{AppBus, AppUiRegistry};

/// Register the provider component with the bus and UI registry.
pub(crate) fn register(bus: &mut AppBus, registry: &mut AppUiRegistry) {
    handler::ProviderHandler.register(bus);
    request_handler::MessageQueueHandler.register(bus);
    prompt_handler::PromptAssemblyHandler.register(bus);
    refresh_handler::RefreshHandler.register(bus);
    switch_handler::SwitchHandler.register(bus);
    registry.register(Box::new(indicator::StreamingIndicatorElement::new()));
    registry.register(Box::new(queue_element::QueueDisplayElement));
}
