//! Config storage abstraction — trait for provider config I/O.
//!
//! Defines [`ConfigStorage`] as the abstraction for loading and saving
//! provider configuration. [`FilesystemConfigStorage`] is the production
//! implementation; [`InMemoryConfigStorage`] is for testing.

use std::path::PathBuf;
use std::sync::Arc;

use error_stack::{Report, ResultExt as _};
use parking_lot::RwLock;

use crate::config::{ConfigError, ProvidersConfig, config_path};

/// Trait for provider config I/O.
///
/// Every external dependency must have a trait abstraction (AGENTS.md §2).
/// Filesystem I/O is an external dependency — this trait abstracts it so
/// tests can use in-memory storage instead of touching the real filesystem.
pub trait ConfigStorage: Send + Sync + 'static {
    /// Returns the storage backend name (for debugging).
    fn name(&self) -> &'static str;

    /// Loads the provider config.
    ///
    /// Creates a default config if none exists.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Io`] if the file cannot be read or created.
    /// Returns [`ConfigError::Parse`] if the TOML is malformed.
    fn load(&self) -> Result<ProvidersConfig, Report<ConfigError>>;

    /// Saves the provider config.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Io`] if writing fails.
    /// Returns [`ConfigError::Parse`] if serialization fails.
    fn save(&self, config: &ProvidersConfig) -> Result<(), Report<ConfigError>>;
}

/// Filesystem-backed config storage.
///
/// Reads from and writes to `providers.toml` at a configurable path.
/// Production uses `dirs::config_dir()`.
pub struct FilesystemConfigStorage {
    /// Path to the config file.
    path: PathBuf,
}

impl FilesystemConfigStorage {
    /// Creates a storage backed by an explicit path.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Creates a storage backed by the default config path.
    #[must_use]
    pub fn default_path() -> Self {
        Self {
            path: config_path(),
        }
    }
}

impl ConfigStorage for FilesystemConfigStorage {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn load(&self) -> Result<ProvidersConfig, Report<ConfigError>> {
        crate::config::load_config_from(&self.path)
    }

    fn save(&self, config: &ProvidersConfig) -> Result<(), Report<ConfigError>> {
        crate::config::save_config_to(config, &self.path)
    }
}

/// In-memory config storage for testing.
///
/// Stores the serialized TOML in memory. `load()` returns the default
/// config if nothing has been saved. `save()` stores the serialized config.
pub struct InMemoryConfigStorage {
    /// Serialized TOML content.
    content: Arc<RwLock<Option<String>>>,
}

impl InMemoryConfigStorage {
    /// Creates an empty in-memory storage (loads default config).
    #[must_use]
    pub fn new() -> Self {
        Self {
            content: Arc::new(RwLock::new(None)),
        }
    }

    /// Creates an in-memory storage pre-populated with the given config.
    ///
    /// # Panics
    ///
    /// Panics if the config cannot be serialized to TOML.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "convenience for tests — serializing a valid ProvidersConfig cannot fail"
    )]
    pub fn with_config(config: &ProvidersConfig) -> Self {
        let toml = toml::to_string_pretty(config).expect("serialize config");
        Self {
            content: Arc::new(RwLock::new(Some(toml))),
        }
    }
}

impl Default for InMemoryConfigStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigStorage for InMemoryConfigStorage {
    fn name(&self) -> &'static str {
        "in-memory"
    }

    fn load(&self) -> Result<ProvidersConfig, Report<ConfigError>> {
        let guard = self.content.read();
        match guard.as_ref() {
            Some(content) => toml::from_str(content)
                .change_context(ConfigError::Parse)
                .attach("failed to parse in-memory config"),
            None => {
                // No config saved — return defaults (empty).
                Ok(ProvidersConfig {
                    providers: vec![],
                    aliases: vec![],
                    default_provider: None,
                })
            }
        }
    }

    fn save(&self, config: &ProvidersConfig) -> Result<(), Report<ConfigError>> {
        let content = toml::to_string_pretty(config)
            .change_context(ConfigError::Parse)
            .attach("failed to serialize config")?;
        let mut guard = self.content.write();
        *guard = Some(content);
        Ok(())
    }
}

/// Service wrapper for config storage.
///
/// Wraps `Arc<dyn ConfigStorage>` for shared ownership across the application.
/// Follows the service wrapper pattern from the project style guide.
#[derive(Debug, Clone)]
pub struct ConfigStorageService {
    /// The underlying config storage implementation.
    svc: Arc<dyn ConfigStorage>,
}

impl ConfigStorageService {
    /// Creates a new config storage service.
    #[must_use]
    pub fn new(storage: Arc<dyn ConfigStorage>) -> Self {
        Self { svc: storage }
    }

    /// Loads the provider config.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] if the stored content is malformed.
    pub fn load(&self) -> Result<ProvidersConfig, Report<ConfigError>> {
        self.svc.load()
    }

    /// Saves the provider config.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] if serialization fails.
    pub fn save(&self, config: &ProvidersConfig) -> Result<(), Report<ConfigError>> {
        self.svc.save(config)
    }
}

impl std::fmt::Debug for dyn ConfigStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigStorage")
            .field("name", &self.name())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::config::ProviderEntry;

    #[test]
    fn in_memory_load_returns_default_when_empty() {
        // Given an empty InMemoryConfigStorage.
        let storage = InMemoryConfigStorage::new();

        // When loading.
        let config = storage.load().expect("load");

        // Then an empty config is returned.
        assert!(config.providers.is_empty());
        assert!(config.aliases.is_empty());
        assert!(config.default_provider.is_none());
    }

    #[test]
    fn in_memory_save_then_load_round_trips() {
        // Given an InMemoryConfigStorage with a config.
        let storage = InMemoryConfigStorage::new();
        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: Some("ollama".to_owned()),
        };

        // When saving and reloading.
        storage.save(&config).expect("save");
        let reloaded = storage.load().expect("load");

        // Then the round-tripped config matches.
        assert_eq!(reloaded.providers.len(), 1);
        assert_eq!(reloaded.providers[0].name, "ollama");
        assert_eq!(reloaded.default_provider.as_deref(), Some("ollama"));
    }

    #[test]
    fn in_memory_with_config_pre_populates() {
        // Given an InMemoryConfigStorage pre-populated with a config.
        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "test".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let storage = InMemoryConfigStorage::with_config(&config);

        // When loading.
        let loaded = storage.load().expect("load");

        // Then the pre-populated config is returned.
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].name, "test");
    }

    #[test]
    fn filesystem_load_creates_default_when_missing() {
        // Given a temp directory with no config file.
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.toml");
        let storage = FilesystemConfigStorage::new(path.clone());

        assert!(!path.exists());

        // When loading config.
        let config = storage.load().expect("load");

        // Then the file is created and parseable.
        assert!(path.exists());
        assert!(!config.providers.is_empty());
    }

    #[test]
    fn filesystem_save_then_load_round_trips() {
        // Given a FilesystemConfigStorage in a temp dir.
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.toml");
        let storage = FilesystemConfigStorage::new(path);

        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: None,
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: Some("ollama".to_owned()),
        };

        // When saving and reloading.
        storage.save(&config).expect("save");
        let reloaded = storage.load().expect("load");

        // Then the round-tripped config matches.
        assert_eq!(reloaded.providers.len(), 1);
        assert_eq!(reloaded.providers[0].name, "ollama");
        assert_eq!(reloaded.default_provider.as_deref(), Some("ollama"));
    }
}
