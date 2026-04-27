//! Plugin definitions for the TUI application.
//!
//! Each plugin handles a specific domain of commands and events
//! using the [`define_plugin!`](nullslop_plugin::define_plugin) macro.
//! All plugins are registered with the [`Bus`](nullslop_plugin::Bus) during startup.

pub mod core_dispatcher;
pub mod extension_command;
pub mod input_mode;
pub mod normal_mode;

use nullslop_plugin::Bus;

/// Register all TUI plugins with the bus.
///
/// Called once during application startup in [`run`](crate::run::run).
pub fn register_all(bus: &mut Bus) {
    core_dispatcher::CoreDispatcher.register(bus);
    extension_command::ExtensionCommandPlugin.register(bus);
    input_mode::InputModePlugin.register(bus);
    normal_mode::NormalModePlugin.register(bus);
}
