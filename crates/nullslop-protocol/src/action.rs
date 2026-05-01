//! Command action returned by command handlers.

/// The action to take after a command handler runs.
///
/// `Continue` allows the next handler to process the command.
/// `Stop` prevents further handlers from seeing this command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    /// Continue to the next handler.
    Continue,
    /// Stop propagation — no further handlers see this command.
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_action_variants_are_distinct() {
        // Given both variants.
        let actions = [CommandAction::Continue, CommandAction::Stop];

        // When comparing the two variants.
        assert_ne!(actions[0], actions[1]);
    }
}
