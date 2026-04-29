//! nullslop-component: built-in component implementations.
//!
//! Contains all components that handle commands and events for the nullslop
//! application. All components use the [`define_handler!`](nullslop_component_core::define_handler)
//! macro from `nullslop-component-core`.

pub mod char_counter;
pub mod chat_input_box;
pub mod chat_log;
pub mod custom_command;
pub mod quit_handler;
pub mod shutdown;

use nullslop_component_core::Bus;
use nullslop_component_ui::UiRegistry;

/// Register all built-in components with the bus and UI registry.
///
/// Called once during application startup.
pub fn register_all(bus: &mut Bus, registry: &mut UiRegistry) {
    quit_handler::register(bus, registry);
    custom_command::register(bus, registry);
    chat_input_box::register(bus, registry);
    chat_log::register(bus, registry);
    char_counter::register(bus, registry);
    shutdown::register(bus, registry);
}
