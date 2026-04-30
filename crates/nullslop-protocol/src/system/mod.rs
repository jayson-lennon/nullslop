//! System domain: application-level commands and events.

mod command;
mod event;

pub use command::{
    AppEditInput, AppQuit, AppSetMode, AppSwitchTab, AppToggleWhichKey, ProviderCancelStream,
    ProviderSendMessage, TabDirection,
};
pub use event::{EventApplicationReady, EventKeyDown, EventKeyUp, EventModeChanged};
