//! TUI configuration.
//!
//! Plain data struct passed into [`TuiApp`](crate::TuiApp) at construction time.
//! Environment variables are read **only** in the binary crate (`src/app.rs`)
//! at program startup and fed into this struct — library crates never access
//! the environment directly.

/// Configuration for the TUI application.
///
/// Controls whether mouse capture is enabled. When enabled (default),
/// the application captures all mouse events and provides application-level
/// text selection. When disabled, mouse events are not captured — the
/// terminal's native text selection works, but scroll wheel and click-based
/// features are unavailable.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    /// Whether to enable mouse capture (click, drag, scroll).
    pub mouse_selection: bool,
}

impl TuiConfig {
    /// Creates a new config with the given mouse selection setting.
    #[must_use]
    pub fn new(mouse_selection: bool) -> Self {
        Self { mouse_selection }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self { mouse_selection: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_mouse_selection_enabled() {
        // Given no explicit config.
        let config = TuiConfig::default();

        // Then mouse selection is enabled.
        assert!(config.mouse_selection);
    }

    #[test]
    fn new_config_with_false_disables_mouse_selection() {
        // Given an explicit config with mouse selection disabled.
        let config = TuiConfig::new(false);

        // Then mouse selection is disabled.
        assert!(!config.mouse_selection);
    }

    #[test]
    fn new_config_with_true_enables_mouse_selection() {
        // Given an explicit config with mouse selection enabled.
        let config = TuiConfig::new(true);

        // Then mouse selection is enabled.
        assert!(config.mouse_selection);
    }
}
