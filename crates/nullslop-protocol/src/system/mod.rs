//! System domain: application-level commands, events, and built-in actor commands.

mod command;
mod event;

pub use command::{DashboardSelectDown, DashboardSelectFirst, DashboardSelectLast, DashboardSelectUp, EditInput, MouseScrollDown, MouseScrollUp, Quit, ScrollDown, ScrollLineDown, ScrollLineUp, ScrollToBottom, ScrollToTop, ScrollUp, SetMode, ToggleWhichKey};
pub use event::{KeyDown, KeyUp, ModeChanged};
