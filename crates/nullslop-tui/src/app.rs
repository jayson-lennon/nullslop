//! Main application state and per-frame rendering.

use derive_more::Debug;
use nullslop_component::AppUiRegistry;
use nullslop_core::{AppCore, AppMsg};
use nullslop_protocol::{ActiveTab, Command, Mode};
use ratatui::Frame;
use ratatui_tabs::TabManager;
use ratatui_which_key::{CrosstermKeymapExt, WhichKeyState};

use std::mem;

use crossterm::event::{MouseButton, MouseEventKind};

use crate::config::TuiConfig;
use crate::keymap;
use crate::msg::Msg;
use crate::render;
use crate::scope::Scope;
use crate::selection::{SelectableRects, SelectionState};
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
    /// Mouse text selection state.
    pub(crate) selection: SelectionState,
    /// Selectable screen regions, rebuilt each frame during rendering.
    pub(crate) selectable_rects: SelectableRects,
    /// Set to `true` when a selection is finalized and the selected text
    /// should be copied to the system clipboard during the next render.
    pub(crate) pending_clipboard: bool,
    /// TUI configuration (mouse capture, etc.).
    pub(crate) config: TuiConfig,
}

impl TuiApp {
    /// Creates a new application with the given services and default config.
    #[must_use]
    pub fn new(services: nullslop_services::Services) -> Self {
        Self::new_with_config(services, TuiConfig::default())
    }

    /// Creates a new application with the given services and config.
    #[must_use]
    pub fn new_with_config(
        services: nullslop_services::Services,
        config: TuiConfig,
    ) -> Self {
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
            selection: SelectionState::Idle,
            selectable_rects: SelectableRects::default(),
            pending_clipboard: false,
            config,
        }
    }

    /// Creates a new application with pre-built core, services, and default config.
    ///
    /// Use this when the caller has already registered components
    /// and set up the actor host on the core.
    #[must_use]
    pub fn new_with_core(
        services: nullslop_services::Services,
        core: nullslop_core::AppCore,
    ) -> Self {
        Self::new_with_core_and_config(services, core, TuiConfig::default())
    }

    /// Creates a new application with pre-built core, services, and config.
    ///
    /// Use this when the caller has already registered components
    /// and set up the actor host on the core.
    #[must_use]
    pub fn new_with_core_and_config(
        services: nullslop_services::Services,
        core: nullslop_core::AppCore,
        config: TuiConfig,
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
            selection: SelectionState::Idle,
            selectable_rects: SelectableRects::default(),
            pending_clipboard: false,
            config,
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
                        // Selection handling — intercept before keymap
                        // (only when mouse capture is enabled).
                        if self.config.mouse_selection
                            && self.handle_selection_mouse(mouse)
                        {
                            return; // consumed by selection
                        }
                        // Fall through to keymap for scroll, etc.
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

    /// Handles mouse events for text selection. Returns `true` if the event was consumed.
    fn handle_selection_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> bool {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(bounds) =
                    self.selectable_rects.find_for_position(mouse.column, mouse.row)
                {
                    self.selection = SelectionState::start_drag(mouse.column, mouse.row, bounds);
                    return true;
                }
                false
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.selection.is_active() {
                    self.selection =
                        mem::take(&mut self.selection).update_focus(mouse.column, mouse.row);
                    return true;
                }
                false
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.selection.is_active() {
                    self.selection = mem::take(&mut self.selection).finalize();
                    self.pending_clipboard = true;
                    return true;
                }
                false
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if self.selection.is_active() {
                    self.selection = mem::take(&mut self.selection).cancel();
                    return true;
                }
                false
            }
            _ => false,
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
                let initial_content =
                    self.core.state.read().active_chat_input().text().to_owned();
                self.suspend.request(SuspendAction::Edit {
                    initial_content,
                    on_result: Box::new(|result| result),
                });
            }
            Command::SetMode { payload } => {
                // Cancel any active selection when mode changes.
                // The selectable rects are rebuilt next frame, but the selection's
                // `bounds` may reference a now-invalid rect (e.g. a closed picker popup).
                self.selection = mem::take(&mut self.selection).cancel();
                let _ = self.core.sender().send(AppMsg::Command {
                    command: Command::SetMode { payload },
                    source: None,
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

/// Returns the scope corresponding to the given mode and active tab.
pub fn scope_for_mode(mode: Mode, active_tab: ActiveTab) -> Scope {
    match mode {
        Mode::Normal => match active_tab {
            ActiveTab::Dashboard => Scope::Dashboard,
            ActiveTab::Chat => Scope::Normal,
        },
        Mode::Input => Scope::Input,
        Mode::Picker => Scope::Picker,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    use super::*;

    /// Creates a minimal TuiApp for testing.
    fn test_app() -> TuiApp {
        let services = nullslop_services::test_services::TestServices::builder().build();
        TuiApp::new(services)
    }

    #[test]
    fn scope_for_mode_maps_correctly() {
        // Given all Mode variants.
        // When mapping each mode to a scope.
        // Then each mode maps to its corresponding scope.
        assert_eq!(scope_for_mode(Mode::Normal, ActiveTab::Chat), Scope::Normal);
        assert_eq!(scope_for_mode(Mode::Normal, ActiveTab::Dashboard), Scope::Dashboard);
        assert_eq!(scope_for_mode(Mode::Input, ActiveTab::Chat), Scope::Input);
        assert_eq!(scope_for_mode(Mode::Picker, ActiveTab::Chat), Scope::Picker);
    }

    #[test]
    fn mouse_down_left_in_selectable_rect_starts_dragging() {
        // Given an app with a registered selectable rect.
        let mut app = test_app();
        let rect = Rect::new(5, 5, 20, 10);
        app.selectable_rects.rebuild(vec![rect]);

        // When sending a left-click inside the rect.
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 8,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the selection is Dragging with anchor at (10, 8).
        assert_eq!(
            app.selection,
            SelectionState::Dragging {
                anchor: (10, 8),
                focus: (10, 8),
                bounds: rect,
            }
        );
    }

    #[test]
    fn mouse_down_left_outside_selectable_rect_does_not_start_dragging() {
        // Given an app with a registered selectable rect.
        let mut app = test_app();
        app.selectable_rects.rebuild(vec![Rect::new(5, 5, 10, 10)]);

        // When sending a left-click outside the rect.
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 30,
            row: 30,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the selection remains Idle.
        assert_eq!(app.selection, SelectionState::Idle);
    }

    #[test]
    fn mouse_drag_updates_focus_while_dragging() {
        // Given an app with an active drag.
        let mut app = test_app();
        let rect = Rect::new(0, 0, 40, 24);
        app.selectable_rects.rebuild(vec![rect]);
        app.selection = SelectionState::start_drag(5, 5, rect);

        // When sending a drag event.
        let mouse = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 15,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the focus is updated to (15, 10).
        assert_eq!(
            app.selection,
            SelectionState::Dragging {
                anchor: (5, 5),
                focus: (15, 10),
                bounds: rect,
            }
        );
    }

    #[test]
    fn mouse_up_left_finalizes_selection() {
        // Given an app with an active drag.
        let mut app = test_app();
        let rect = Rect::new(0, 0, 40, 24);
        app.selection = SelectionState::start_drag(2, 3, rect).update_focus(10, 12);

        // When sending a mouse-up event.
        let mouse = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 12,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the selection is Active with the same anchor and focus.
        assert_eq!(
            app.selection,
            SelectionState::Active {
                anchor: (2, 3),
                focus: (10, 12),
                bounds: rect,
            }
        );
    }

    #[test]
    fn mouse_down_right_cancels_selection() {
        // Given an app with an active selection.
        let mut app = test_app();
        let rect = Rect::new(0, 0, 40, 24);
        app.selection = SelectionState::start_drag(5, 5, rect);

        // When sending a right-click.
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 5,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the selection is cancelled to Idle.
        assert_eq!(app.selection, SelectionState::Idle);
    }

    #[test]
    fn scroll_events_still_route_to_keymap() {
        // Given an app in Normal scope.
        let mut app = test_app();
        let initial_selection = app.selection.clone();

        // When sending a scroll-up mouse event.
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the selection is unchanged (event fell through to keymap).
        assert_eq!(app.selection, initial_selection);
    }

    #[test]
    fn mouse_events_not_handled_when_mouse_selection_disabled() {
        // Given an app with mouse selection disabled and a registered selectable rect.
        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app =
            TuiApp::new_with_config(services, crate::config::TuiConfig::new(false));
        let rect = Rect::new(5, 5, 20, 10);
        app.selectable_rects.rebuild(vec![rect]);

        // When sending a left-click inside the rect.
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 8,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        app.handle_msg(Msg::Input(crossterm::event::Event::Mouse(mouse)));

        // Then the selection remains Idle (event was not handled).
        assert_eq!(app.selection, SelectionState::Idle);
    }
}
