//! Dashboard state — tracks extension names and their startup status.
//!
//! Each extension goes through a lifecycle: `Starting` → `Started`.
//! The dashboard state records the current status for display.

use std::collections::HashMap;

/// The startup status of an extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionStatus {
    /// The extension is currently starting up.
    Starting,
    /// The extension has finished starting and is ready.
    Started,
}

/// Tracks the startup status of all extensions.
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    /// Extension name → current status.
    extensions: HashMap<String, ExtensionStatus>,
    /// Insertion-order keys for stable display.
    order: Vec<String>,
}

impl DashboardState {
    /// Create an empty dashboard state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that an extension has started the startup process.
    pub fn mark_starting(&mut self, name: &str) {
        if !self.extensions.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.extensions
            .insert(name.to_string(), ExtensionStatus::Starting);
    }

    /// Record that an extension has finished starting.
    ///
    /// If the extension was not previously tracked (no `mark_starting` call),
    /// it is added with `Started` status.
    pub fn mark_started(&mut self, name: &str) {
        if !self.extensions.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.extensions
            .insert(name.to_string(), ExtensionStatus::Started);
    }

    /// Returns all tracked extensions in insertion order with their status.
    #[must_use]
    pub fn extensions(&self) -> Vec<(&str, ExtensionStatus)> {
        self.order
            .iter()
            .filter_map(|name| {
                self.extensions
                    .get(name)
                    .map(|&status| (name.as_str(), status))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_starting_then_started() {
        // Given an empty dashboard.
        let mut dashboard = DashboardState::new();

        // When marking "ext-a" as starting, then started.
        dashboard.mark_starting("ext-a");
        dashboard.mark_started("ext-a");

        // Then "ext-a" has Started status.
        let exts = dashboard.extensions();
        assert_eq!(exts.len(), 1);
        assert_eq!(exts[0], ("ext-a", ExtensionStatus::Started));
    }

    #[test]
    fn mark_started_without_starting() {
        // Given an empty dashboard.
        let mut dashboard = DashboardState::new();

        // When marking "ext-a" as started without prior starting.
        dashboard.mark_started("ext-a");

        // Then "ext-a" is tracked with Started status.
        let exts = dashboard.extensions();
        assert_eq!(exts.len(), 1);
        assert_eq!(exts[0], ("ext-a", ExtensionStatus::Started));
    }

    #[test]
    fn extensions_preserve_insertion_order() {
        // Given an empty dashboard.
        let mut dashboard = DashboardState::new();

        // When adding multiple extensions.
        dashboard.mark_starting("alpha");
        dashboard.mark_starting("beta");
        dashboard.mark_started("beta");
        dashboard.mark_started("alpha");

        // Then order reflects first-seen order.
        let exts = dashboard.extensions();
        assert_eq!(exts.len(), 2);
        assert_eq!(exts[0].0, "alpha");
        assert_eq!(exts[0].1, ExtensionStatus::Started);
        assert_eq!(exts[1].0, "beta");
        assert_eq!(exts[1].1, ExtensionStatus::Started);
    }

    #[test]
    fn empty_dashboard_has_no_extensions() {
        // Given an empty dashboard.
        let dashboard = DashboardState::new();

        // Then there are no extensions.
        assert!(dashboard.extensions().is_empty());
    }
}
