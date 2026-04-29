//! Extension host sender for headless mode.
//!
//! [`HeadlessExtSender`] adapts `kanal::Sender<AppMsg>` to the
//! [`ExtHostSender`](nullslop_core::ExtHostSender) trait, bridging
//! extension events into the headless `AppCore` message channel.

use nullslop_core::{AppMsg, Command, Event, ExtHostSender, RegisteredExtension};

/// Sender that bridges the extension host into the headless `AppCore` channel.
pub(crate) struct HeadlessExtSender(kanal::Sender<AppMsg>);

impl HeadlessExtSender {
    /// Creates a new sender wrapping the given channel.
    pub(crate) fn new(sender: kanal::Sender<AppMsg>) -> Self {
        Self(sender)
    }
}

impl ExtHostSender for HeadlessExtSender {
    fn send_extensions_ready(&self, registrations: Vec<RegisteredExtension>) {
        let _ = self.0.send(AppMsg::ExtensionsReady(registrations));
    }

    fn send_command(&self, command: Command) {
        let _ = self.0.send(AppMsg::Command(command));
    }

    fn send_extension_event(&self, event: Event) {
        let _ = self.0.send(AppMsg::Event(event));
    }
}
