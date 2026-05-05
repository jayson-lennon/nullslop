//! Main application state and per-frame rendering.

use derive_more::Debug;
use nullslop_component::AppUiRegistry;
use nullslop_core::{AppCore, AppMsg};
use nullslop_protocol::{Command, Mode};
use ratatui::Frame;
use ratatui_tabs::TabManager;
use ratatui_which_key::{CrosstermKeymapExt, WhichKeyState};

use crate::keymap;
use crate::msg::Msg;
use crate::render;
use crate::scope::Scope;
use crate::suspend::{Suspend, SuspendAction};
use crate::{AppStatus, MsgHandler};

/// Type alias for the which-key state parameterized for nullslop.
pub type WhichKeyInstance =
    WhichKeyState<nullslop_protocol::KeyEvent, Scope, Command, crate::keymap::KeyCategory>;

/// Top-level application state and event loop.
#[derive(Debug)]
pub struct TuiApp {
    /// Application core (bus, state, message channel).
    pub core: AppCore,
    /// UI element registry.
    pub ui_registry: AppUiRegistry,
    /// Message channel for the event loop.
    pub events: MsgHandler,
    /// Which-key keybinding system state.
    #[debug(skip)]
    pub which_key: WhichKeyInstance,
    /// Deferred suspend action queue (e.g., for external editor).
    pub suspend: Suspend,
    /// Background event stream. Set by [`run`](crate::run::run).
    #[debug(skip)]
    pub event_task: Option<tokio::task::JoinHandle<()>>,
    /// Runtime services.
    pub services: nullslop_services::Services,
    /// Current application lifecycle status.
    pub status: AppStatus,
    /// Tab manager for rendering the tab bar.
    pub tab_manager: TabManager,
}

impl TuiApp {
    /// Creates a new application with the given services.
    #[must_use]
    pub fn new(services: nullslop_services::Services) -> Self {
        let mut core = AppCore::new(services.clone());
        let mut ui_registry = AppUiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut ui_registry);
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);

        Self {
            core,
            ui_registry,
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services,
            status: AppStatus::Starting,
            tab_manager: crate::render::init_tab_manager(),
        }
    }

    /// Creates a new application with pre-built core and services.
    ///
    /// Use this when the caller has already registered components
    /// and set up the actor host on the core.
    #[must_use]
    pub fn new_with_core(
        services: nullslop_services::Services,
        core: nullslop_core::AppCore,
    ) -> Self {
        let mut ui_registry = AppUiRegistry::new();
        nullslop_component::register_tui_elements(&mut ui_registry);
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);

        Self {
            core,
            ui_registry,
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services,
            status: AppStatus::Starting,
            tab_manager: crate::render::init_tab_manager(),
        }
    }

    /// Processes a single message.
    pub fn handle_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Tick => {}
            Msg::Input(event) => {
                match event {
                    crossterm::event::Event::Key(key) => {
                        if key.kind != crossterm::event::KeyEventKind::Press {
                            return;
                        }
                        let Some(protocol_key) = crate::convert::from_crossterm(key) else {
                            tracing::info!(
                                crossterm_code = ?key.code,
                                crossterm_mods = ?key.modifiers,
                                "key converted to None"
                            );
                            return;
                        };
                        tracing::info!(
                            key = ?protocol_key.key,
                            mods = ?protocol_key.modifiers,
                            scope = ?self.which_key.scope(),
                            "key event received"
                        );
                        let Some(cmd) = self.which_key.handle_key(protocol_key) else {
                            return;
                        };
                        self.route_command(cmd);
                    }
                    crossterm::event::Event::Mouse(mouse) => {
                        let scope = self.which_key.scope().clone();
                        let Some(cmd) = self
                            .which_key
                            .keymap()
                            .mouse_handler()
                            .and_then(|h| h(mouse, &scope))
                        else {
                            return;
                        };
                        self.route_command(cmd);
                    }
                    _ => {}
                }
            }
            Msg::Command(cmd) => {
                self.route_command(cmd);
            }
        }
    }

    /// Routes a command to the appropriate handler.
    ///
    /// Commands that need `TuiApp`-level state (which-key toggle, editor suspend)
    /// are handled directly. All other commands go through the core channel.
    fn route_command(&mut self, cmd: Command) {
        match cmd {
            Command::ToggleWhichKey => {
                self.which_key.toggle();
            }
            Command::EditInput => {
                let initial_content = self.core.state.read().active_chat_input().text().to_owned();
                self.suspend.request(SuspendAction::Edit {
                    initial_content,
                    on_result: Box::new(|result| result),
                });
            }
            _ => {
                let _ = self.core.sender().send(AppMsg::Command {
                    command: cmd,
                    source: None,
                });
            }
        }
    }

    /// Renders the application for a single frame.
    pub fn render(&mut self, frame: &mut Frame<'_>) {
        render::render(self, frame);
    }
}

/// Returns the scope corresponding to the given mode.
pub fn scope_for_mode(mode: Mode) -> Scope {
    match mode {
        Mode::Normal => Scope::Normal,
        Mode::Input => Scope::Input,
        Mode::Picker => Scope::Picker,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_for_mode_maps_correctly() {
        // Given all Mode variants.
        // When mapping each mode to a scope.
        // Then each mode maps to its corresponding scope.
        assert_eq!(scope_for_mode(Mode::Normal), Scope::Normal);
        assert_eq!(scope_for_mode(Mode::Input), Scope::Input);
        assert_eq!(scope_for_mode(Mode::Picker), Scope::Picker);
    }
}
