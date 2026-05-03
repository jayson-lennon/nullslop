//! Provider configuration — TOML schema and I/O.
//!
//! Defines [`ProvidersConfig`] as the root of `providers.toml`,
//! along with loading, saving, and auto-creation logic. The config file
//! lives at `~/.config/nullslop/providers.toml` and is auto-created on
//! first run with commented-out examples for every known backend.

use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt as _};
use serde::{Deserialize, Serialize};
use wherror::Error;

/// Default provider configuration template, embedded at compile time.
const DEFAULT_CONFIG: &str = include_str!("default_providers.toml");

/// Errors that can occur during config I/O or parsing.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Filesystem I/O failure.
    #[error("config I/O error")]
    Io,
    /// TOML parsing or structural error.
    #[error("config parse error")]
    Parse,
    /// Semantic validation error (duplicate names, missing targets, etc.).
    #[error("config validation error")]
    Validation,
}

/// Root of `providers.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// User-defined provider entries.
    pub providers: Vec<ProviderEntry>,
    /// User-defined aliases (short names → provider entries).
    #[serde(default)]
    pub aliases: Vec<AliasEntry>,
    /// The last-selected default provider (persisted across sessions).
    #[serde(default)]
    pub default_provider: Option<String>,
}

/// A single configured provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    /// Unique user-visible name (also used as the `ProviderId`).
    pub name: String,
    /// Backend type string, parsed via `LLMBackend::from_str`.
    /// E.g. `"openrouter"`, `"ollama"`, `"openai"`.
    pub backend: String,
    /// Model identifiers available under this provider.
    /// E.g. `["openai/gpt-oss-120b", "anthropic/claude-sonnet-4-20250514"]`.
    pub models: Vec<String>,
    /// Optional base URL override. Required for OpenAI-compatible local
    /// providers like `LMStudio`. Omitted for cloud providers with
    /// well-known endpoints.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Environment variable name holding the API key.
    /// E.g. `"OPENROUTER_API_KEY"`.
    /// Ignored for providers where `requires_key` is `false`.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Whether this provider type requires an API key.
    /// Defaults to `true`. Set to `false` for local providers (Ollama).
    #[serde(default = "default_true")]
    pub requires_key: bool,
}

/// A named alias for a provider entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasEntry {
    /// Short name shown in the picker.
    pub name: String,
    /// Full provider ID of the target entry. E.g. `"ollama/llama3"`.
    pub target: String,
}

/// Default value for boolean fields that default to `true`.
const fn default_true() -> bool {
    true
}

/// Returns the path to the provider config file.
///
/// Uses `dirs::config_dir()` → `~/.config/nullslop/providers.toml`.
#[must_use]
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nullslop")
        .join("providers.toml")
}

/// Loads the provider config from disk.
///
/// If the file does not exist, creates the default config and then loads it.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] if the file cannot be read or created.
/// Returns [`ConfigError::Parse`] if the TOML is malformed.
pub fn load_config() -> Result<ProvidersConfig, Report<ConfigError>> {
    let path = config_path();
    load_config_from(&path)
}

/// Loads config from a specific path (testable without touching real config).
pub(crate) fn load_config_from<P>(path: P) -> Result<ProvidersConfig, Report<ConfigError>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    if !path.exists() {
        create_default_config_to(path)?;
    }

    let content = std::fs::read_to_string(path)
        .change_context(ConfigError::Io)
        .attach("failed to read providers config")?;

    toml::from_str(&content)
        .change_context(ConfigError::Parse)
        .attach("failed to parse providers config")
}

/// Creates the default config file at the standard location.
///
/// Creates parent directories as needed and writes the embedded template.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] if directory creation or file writing fails.
pub fn create_default_config() -> Result<PathBuf, Report<ConfigError>> {
    let path = config_path();
    create_default_config_to(&path)?;
    Ok(path)
}

/// Creates the default config file at an explicit path.
pub(crate) fn create_default_config_to<P>(path: P) -> Result<(), Report<ConfigError>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .change_context(ConfigError::Io)
            .attach("failed to create config directory")?;
    }

    std::fs::write(path, DEFAULT_CONFIG)
        .change_context(ConfigError::Io)
        .attach("failed to write default providers config")
}

/// Saves the config back to disk.
///
/// Serializes the full config as pretty-printed TOML. Note: this may
/// reorder or remove user comments. Comment preservation is a future
/// improvement.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] if writing fails.
/// Returns [`ConfigError::Parse`] if serialization fails.
pub fn save_config(config: &ProvidersConfig) -> Result<(), Report<ConfigError>> {
    let path = config_path();
    save_config_to(config, &path)
}

/// Saves config to a specific path.
pub(crate) fn save_config_to<P>(
    config: &ProvidersConfig,
    path: P,
) -> Result<(), Report<ConfigError>>
where
    P: AsRef<Path>,
{
    let content = toml::to_string_pretty(config)
        .change_context(ConfigError::Parse)
        .attach("failed to serialize providers config")?;

    std::fs::write(path.as_ref(), content)
        .change_context(ConfigError::Io)
        .attach("failed to write providers config")
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn load_config_parses_well_formed_toml() {
        // Given a temp directory with a well-formed TOML config.
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.toml");
        let toml = r#"
[[providers]]
name = "ollama"
backend = "ollama"
models = ["llama3", "codellama"]
requires_key = false

[[aliases]]
name = "fast"
target = "ollama/llama3"
"#;
        std::fs::write(&path, toml).expect("write");

        // When loading the config.
        let config = load_config_from(&path).expect("load");

        // Then providers and aliases are parsed correctly.
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].name, "ollama");
        assert_eq!(config.providers[0].backend, "ollama");
        assert_eq!(config.providers[0].models, vec!["llama3", "codellama"]);
        assert!(!config.providers[0].requires_key);
        assert_eq!(config.aliases.len(), 1);
        assert_eq!(config.aliases[0].name, "fast");
        assert_eq!(config.aliases[0].target, "ollama/llama3");
    }

    #[test]
    fn load_config_creates_default_when_missing() {
        // Given a temp directory with no config file.
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.toml");

        assert!(!path.exists());

        // When loading config.
        let config = load_config_from(&path).expect("load");

        // Then the file is created and parseable.
        assert!(path.exists());
        // The default config has one uncommented provider.
        assert!(!config.providers.is_empty());
    }

    #[test]
    fn save_config_writes_valid_toml() {
        // Given a config with providers.
        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "test".to_owned(),
                backend: "openrouter".to_owned(),
                models: vec!["gpt-4".to_owned()],
                base_url: None,
                api_key_env: Some("TEST_KEY".to_owned()),
                requires_key: true,
            }],
            aliases: vec![],
            default_provider: Some("test/gpt-4".to_owned()),
        };

        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("providers.toml");

        // When saving and reloading.
        save_config_to(&config, &path).expect("save");
        let reloaded = load_config_from(&path).expect("reload");

        // Then the round-tripped config matches.
        assert_eq!(reloaded.providers.len(), 1);
        assert_eq!(reloaded.providers[0].name, "test");
        assert_eq!(reloaded.default_provider.as_deref(), Some("test/gpt-4"));
    }

    #[test]
    fn config_path_uses_dirs_config_dir() {
        // Given the standard config path.
        let path = config_path();

        // Then it ends with nullslop/providers.toml.
        assert!(path.to_string_lossy().ends_with("nullslop/providers.toml"));
    }
}
