//! Registry of active extensions and their capabilities.
//!
//! Tracks which extensions are running, the commands they handle, and the events
//! they subscribe to. Used for command dispatch and event routing throughout the
//! application.

pub mod manifest;

// Re-export manifest types
pub use manifest::ExtensionManifest;

/// Registry of active extensions known to the host.
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    extensions: Vec<RegisteredExtension>,
}

/// A known extension process.
///
/// Represents an extension that has completed discovery and registration.
/// Tracks the extension's name, the commands it handles, and the events
/// it subscribes to — used for command dispatch and event routing.
#[derive(Debug, PartialEq)]
pub struct RegisteredExtension {
    /// The extension's unique name.
    pub name: String,
    /// Commands this extension handles.
    pub commands: Vec<String>,
    /// Events this extension is subscribed to.
    pub subscriptions: Vec<String>,
}

impl ExtensionRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new extension.
    ///
    /// # Panics
    ///
    /// Panics if an extension with the same name is already registered.
    pub fn register(&mut self, ext: RegisteredExtension) {
        assert!(
            !self.extensions.iter().any(|e| e.name == ext.name),
            "extension already registered: {}",
            ext.name
        );
        self.extensions.push(ext);
    }

    /// Look up which extension handles a given command name.
    #[must_use]
    pub fn find_command_handler(&self, command: &str) -> Option<&RegisteredExtension> {
        self.extensions
            .iter()
            .find(|ext| ext.commands.iter().any(|c| c == command))
    }

    /// Get all registered extensions.
    #[must_use]
    pub fn extensions(&self) -> &[RegisteredExtension] {
        &self.extensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_finds_nothing() {
        // Given an empty registry.
        let registry = ExtensionRegistry::new();

        // When looking up any command.
        let result = registry.find_command_handler("echo");

        // Then result is None.
        assert_eq!(result, None);
    }

    #[test]
    fn register_then_find() {
        // Given a registry with an extension handling command "echo".
        let mut registry = ExtensionRegistry::new();
        registry.register(RegisteredExtension {
            name: "echo-ext".to_string(),
            commands: vec!["echo".to_string()],
            subscriptions: vec![],
        });

        // When finding "echo".
        let result = registry.find_command_handler("echo");

        // Then returns that extension.
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "echo-ext");
    }

    #[test]
    #[should_panic(expected = "extension already registered: echo-ext")]
    fn register_duplicate_panics() {
        // Given a registered extension.
        let mut registry = ExtensionRegistry::new();
        registry.register(RegisteredExtension {
            name: "echo-ext".to_string(),
            commands: vec!["echo".to_string()],
            subscriptions: vec![],
        });

        // When registering another with the same name, it panics.
        registry.register(RegisteredExtension {
            name: "echo-ext".to_string(),
            commands: vec!["other".to_string()],
            subscriptions: vec![],
        });
    }
}
