//! Active tab for the tabbed interface.
//!
//! Determines which view is currently displayed in the main area.

use serde::{Deserialize, Serialize};

/// The currently active tab in the main area.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveTab {
    /// The chat conversation view.
    #[default]
    Chat,
    /// The dashboard view showing actor status.
    Dashboard,
}

impl ActiveTab {
    /// All tabs in display order.
    const ALL: [ActiveTab; 2] = [ActiveTab::Chat, ActiveTab::Dashboard];

    /// Returns the label shown in the tab bar.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            ActiveTab::Chat => "Chat",
            ActiveTab::Dashboard => "Dashboard",
        }
    }

    /// Returns all tabs in display order.
    #[must_use]
    pub const fn all() -> &'static [ActiveTab] {
        &Self::ALL
    }

    /// Advance to the next tab, wrapping around.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "modular arithmetic guarantees idx is within bounds of ALL"
    )]
    pub fn next(self) -> Self {
        let idx = self.index();
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Go to the previous tab, wrapping around.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "modular arithmetic guarantees idx is within bounds of ALL"
    )]
    pub fn prev(self) -> Self {
        let idx = self.index();
        let len = Self::ALL.len();
        Self::ALL[(idx + len - 1) % len]
    }

    /// Returns the index of this tab in the display order.
    const fn index(self) -> usize {
        match self {
            ActiveTab::Chat => 0,
            ActiveTab::Dashboard => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wraps_from_last_to_first() {
        // Given the last tab.
        // When advancing.
        // Then it wraps to the first tab.
        assert_eq!(ActiveTab::Dashboard.next(), ActiveTab::Chat);
    }

    #[test]
    fn prev_wraps_from_first_to_last() {
        // Given the first tab.
        // When going back.
        // Then it wraps to the last tab.
        assert_eq!(ActiveTab::Chat.prev(), ActiveTab::Dashboard);
    }

    #[test]
    fn next_then_prev_returns_to_start() {
        // Given any tab.
        for tab in ActiveTab::all() {
            // When advancing then going back.
            // Then we return to the original tab.
            assert_eq!(tab.next().prev(), *tab);
        }
    }

    #[test]
    fn labels_are_distinct() {
        // Given all tabs.
        // When collecting labels from each tab.
        let labels: Vec<&str> = ActiveTab::all()
            .iter()
            .map(super::ActiveTab::label)
            .collect();

        // Then no two labels are the same.
        for i in 0..labels.len() {
            for j in (i + 1)..labels.len() {
                assert_ne!(labels[i], labels[j]);
            }
        }
    }
}
