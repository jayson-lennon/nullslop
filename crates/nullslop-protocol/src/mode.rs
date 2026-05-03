//! Application interaction mode.

use serde::{Deserialize, Serialize};

/// The current input mode of the application.
///
/// In `Normal` mode, keystrokes trigger commands (quit, enter input, etc.).
/// In `Input` mode, keystrokes are typed into the input buffer.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Command mode — keystrokes trigger actions.
    #[default]
    Normal,
    /// Text input mode — keystrokes type into the buffer.
    Input,
    /// Provider picker mode — keystrokes filter/select a provider.
    Picker,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Input => write!(f, "input"),
            Self::Picker => write!(f, "picker"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_serialization_roundtrip() {
        // Given both mode variants.
        for mode in [Mode::Normal, Mode::Input, Mode::Picker] {
            // When serialized and deserialized.
            let json = serde_json::to_string(&mode).expect("serialize");
            let back: Mode = serde_json::from_str(&json).expect("deserialize");

            // Then it matches the original.
            assert_eq!(back, mode);
        }
    }
}
