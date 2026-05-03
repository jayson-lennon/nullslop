//! Handler for the tab switch command.

use crate::AppState;
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol::tab::SwitchTab;
use nullslop_protocol::{CommandAction, TabDirection};
use nullslop_services::Services;

define_handler! {
    pub(crate) struct TabNavHandler;

    commands {
        SwitchTab: on_switch_tab,
    }

    events {}
}

impl TabNavHandler {
    /// Switches the active tab in the given direction.
    fn on_switch_tab(
        cmd: &SwitchTab,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_tab = match cmd.direction {
            TabDirection::Next => ctx.state.active_tab.next(),
            TabDirection::Prev => ctx.state.active_tab.prev(),
        };
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use nullslop_component_core::Bus;
    use nullslop_protocol::{ActiveTab, Command, TabDirection};
    use nullslop_services::Services;

    use super::*;
    use crate::test_utils;

    #[test]
    fn switch_tab_next_from_chat_goes_to_dashboard() {
        // Given a bus with TabNavHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        TabNavHandler.register(&mut bus);

        // When processing an SwitchTab(Next) command.
        bus.submit_command(Command::SwitchTab {
            payload: SwitchTab {
                direction: TabDirection::Next,
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then the active tab is Dashboard.
        assert_eq!(state.active_tab, ActiveTab::Dashboard);
    }

    #[test]
    fn switch_tab_next_wraps_from_dashboard_to_chat() {
        // Given a bus with TabNavHandler registered and state on Dashboard.
        let mut bus: Bus<AppState, Services> = Bus::new();
        TabNavHandler.register(&mut bus);
        let services = test_utils::test_services();
        let mut state = AppState {
            active_tab: ActiveTab::Dashboard,
            ..Default::default()
        };

        // When processing an SwitchTab(Next) command.
        bus.submit_command(Command::SwitchTab {
            payload: SwitchTab {
                direction: TabDirection::Next,
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the active tab wraps back to Chat.
        assert_eq!(state.active_tab, ActiveTab::Chat);
    }

    #[test]
    fn switch_tab_prev_from_chat_wraps_to_dashboard() {
        // Given a bus with TabNavHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        TabNavHandler.register(&mut bus);

        // When processing an SwitchTab(Prev) command.
        bus.submit_command(Command::SwitchTab {
            payload: SwitchTab {
                direction: TabDirection::Prev,
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then the active tab wraps to Dashboard.
        assert_eq!(state.active_tab, ActiveTab::Dashboard);
    }
}
