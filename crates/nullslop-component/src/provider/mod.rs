//! Provider component — handles streaming LLM responses and message queuing.
//!
//! `ProviderHandler` processes [`StreamToken`] commands to track streaming state.
//! `MessageQueueHandler` manages the message queue: enqueue, dispatch, cancel-drain.
//! `StreamingIndicatorElement` shows the current streaming/sending/queue state.
//! `QueueDisplayElement` shows stacked dimmed "QUEUED:" entries.

pub mod handler;
pub mod indicator;
pub mod queue_element;
pub mod request_handler;
pub mod switch_handler;

use nullslop_component_core::Bus;
use nullslop_component_ui::UiRegistry;

use crate::AppState;

/// Register the provider component with the bus and UI registry.
pub(crate) fn register(bus: &mut Bus<AppState>, registry: &mut UiRegistry<AppState>) {
    handler::ProviderHandler.register(bus);
    request_handler::MessageQueueHandler.register(bus);
    switch_handler::SwitchHandler.register(bus);
    registry.register(Box::new(indicator::StreamingIndicatorElement::new()));
    registry.register(Box::new(queue_element::QueueDisplayElement));
}
