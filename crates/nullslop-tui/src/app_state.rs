//! Application lifecycle status.

/// The current status of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppStatus {
    /// Application is initializing.
    #[default]
    Starting,
    /// Application is running and ready.
    Ready,
    /// Application is shutting down.
    ShuttingDown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_is_starting() {
        // Given a default AppStatus.
        let status = AppStatus::default();

        // Then it is Starting.
        assert_eq!(status, AppStatus::Starting);
    }

    #[test]
    fn all_variants_are_distinct() {
        // Given all three variants.
        let variants = [
            AppStatus::Starting,
            AppStatus::Ready,
            AppStatus::ShuttingDown,
        ];

        // Then no two are equal.
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
