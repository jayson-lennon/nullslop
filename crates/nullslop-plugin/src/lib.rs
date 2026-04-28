//! nullslop-plugin: built-in plugin implementations.
//!
//! Contains all plugins that handle commands and events for the nullslop
//! application. Each plugin is defined using the [`define_plugin!`](nullslop_plugin_core::define_plugin)
//! macro from `nullslop-plugin-core`.

pub mod chat_input_box;
pub mod chat_log;
pub mod core_dispatcher;
pub mod extension_command;

use nullslop_plugin_core::Bus;
use nullslop_plugin_ui::UiRegistry;

/// Register all built-in plugins with the bus and UI registry.
///
/// Called once during application startup.
pub fn register_all(bus: &mut Bus, registry: &mut UiRegistry) {
    core_dispatcher::CoreDispatcher.register(bus);
    extension_command::ExtensionCommandPlugin.register(bus);
    chat_input_box::ChatInputBoxHandler.register(bus);
    registry.register(Box::new(chat_input_box::ChatInputBoxElement));
    registry.register(Box::new(chat_log::ChatLogElement));
}
