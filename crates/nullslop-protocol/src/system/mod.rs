//! System domain: application-level commands and events.

mod command;
mod event;

pub use command::{
    AppEditInput, AppQuit, AppSetMode, AppToggleWhichKey, ProviderCancelStream, ProviderSendMessage,
};
pub use event::{EventApplicationReady, EventKeyDown, EventKeyUp, EventModeChanged};
