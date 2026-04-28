//! nullslop-plugin: built-in plugin implementations.
//!
//! Contains all plugins that handle commands and events for the nullslop
//! application. Each plugin that handles messages is defined using the [`define_handler!`](nullslop_plugin_core::define_handler)
//! macro from `nullslop-plugin-core`.

pub mod chat_input_box;
pub mod chat_log;
pub mod extension_command;
pub mod quit_handler;

use nullslop_plugin_core::Bus;
use nullslop_plugin_ui::UiRegistry;

/// Register all built-in plugins with the bus and UI registry.
///
/// Called once during application startup.
pub fn register_all(bus: &mut Bus, registry: &mut UiRegistry) {
    quit_handler::register(bus, registry);
    extension_command::register(bus, registry);
    chat_input_box::register(bus, registry);
    chat_log::register(bus, registry);
}
