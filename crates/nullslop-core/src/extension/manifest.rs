//! Extension manifest format for discovery.
//!
//! # Security
//!
//! The `binary` field in the manifest must be a relative path within the
//! extension's own directory. The host must validate this at load time:
//! reject absolute paths, reject `..` traversal, and resolve the path
//! relative to the manifest's parent directory.

use serde::{Deserialize, Serialize};

/// Extension manifest loaded from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Unique extension name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Path to the extension binary, relative to the manifest directory.
    pub binary: String,
    /// When to activate this extension.
    pub activation_events: Vec<String>,
    /// Commands this extension provides.
    pub commands: Vec<String>,
    /// Events this extension subscribes to.
    pub events: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_deserialization() {
        // Given valid JSON manifest data.
        let json = r#"{
            "name": "nullslop-echo",
            "version": "0.1.0",
            "binary": "nullslop-echo",
            "activation_events": ["onStartup"],
            "commands": ["echo"],
            "events": ["NewChatEntry"]
        }"#;

        // When deserializing.
        let manifest: ExtensionManifest = serde_json::from_str(json).expect("deserialize manifest");

        // Then all fields are correct.
        assert_eq!(manifest.name, "nullslop-echo");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.binary, "nullslop-echo");
        assert_eq!(manifest.activation_events, vec!["onStartup"]);
        assert_eq!(manifest.commands, vec!["echo"]);
        assert_eq!(manifest.events, vec!["NewChatEntry"]);
    }
}
