//! Application interaction mode.

use serde::{Deserialize, Serialize};

/// The current input mode of the application.
///
/// In `Normal` mode, keystrokes trigger commands (quit, enter input, etc.).
/// In `Input` mode, keystrokes are typed into the input buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Command mode — keystrokes trigger actions.
    Normal,
    /// Text input mode — keystrokes type into the buffer.
    Input,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_serialization_roundtrip() {
        // Given both mode variants.
        for mode in [Mode::Normal, Mode::Input] {
            // When serialized and deserialized.
            let json = serde_json::to_string(&mode).expect("serialize");
            let back: Mode = serde_json::from_str(&json).expect("deserialize");

            // Then it matches the original.
            assert_eq!(back, mode);
        }
    }
}
