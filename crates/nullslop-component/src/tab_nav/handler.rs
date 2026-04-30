//! Handler for the tab switch command.

use crate::AppState;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol::{CommandAction, TabDirection};
use nullslop_protocol::command::AppSwitchTab;

define_handler! {
    pub(crate) struct TabNavHandler;

    commands {
        AppSwitchTab: on_switch_tab,
    }

    events {}
}

impl TabNavHandler {
    fn on_switch_tab(cmd: &AppSwitchTab, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.active_tab = match cmd.direction {
            TabDirection::Next => state.active_tab.next(),
            TabDirection::Prev => state.active_tab.prev(),
        };
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use nullslop_component_core::Bus;
    use nullslop_protocol::{ActiveTab, Command, TabDirection};

    use super::*;

    #[test]
    fn switch_tab_next_from_chat_goes_to_dashboard() {
        // Given a bus with TabNavHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        TabNavHandler.register(&mut bus);

        // When processing an AppSwitchTab(Next) command.
        bus.submit_command(Command::AppSwitchTab {
            payload: AppSwitchTab {
                direction: TabDirection::Next,
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then the active tab is Dashboard.
        assert_eq!(state.active_tab, ActiveTab::Dashboard);
    }

    #[test]
    fn switch_tab_next_wraps_from_dashboard_to_chat() {
        // Given a bus with TabNavHandler registered and state on Dashboard.
        let mut bus: Bus<AppState> = Bus::new();
        TabNavHandler.register(&mut bus);
        let mut state = AppState::new();
        state.active_tab = ActiveTab::Dashboard;

        // When processing an AppSwitchTab(Next) command.
        bus.submit_command(Command::AppSwitchTab {
            payload: AppSwitchTab {
                direction: TabDirection::Next,
            },
        });
        bus.process_commands(&mut state);

        // Then the active tab wraps back to Chat.
        assert_eq!(state.active_tab, ActiveTab::Chat);
    }

    #[test]
    fn switch_tab_prev_from_chat_wraps_to_dashboard() {
        // Given a bus with TabNavHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        TabNavHandler.register(&mut bus);

        // When processing an AppSwitchTab(Prev) command.
        bus.submit_command(Command::AppSwitchTab {
            payload: AppSwitchTab {
                direction: TabDirection::Prev,
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then the active tab wraps to Dashboard.
        assert_eq!(state.active_tab, ActiveTab::Dashboard);
    }
}
