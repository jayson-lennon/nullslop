//! Model cache — persisted discovery results for provider models.
//!
//! [`ModelCache`] stores the results of model discovery (provider name → list of
//! model IDs) as a JSON file on disk. It is loaded after a refresh completes and
//! read by the UI to display "last updated" information.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt as _};
use serde::{Deserialize, Serialize};
use wherror::Error;

/// Errors that can occur during model cache I/O.
#[derive(Debug, Error)]
#[error(debug)]
pub struct ModelCacheError;

/// Persisted model discovery results.
///
/// Maps provider names to the list of discovered model IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCache {
    /// Provider name → list of discovered model IDs.
    pub entries: HashMap<String, Vec<String>>,
}

impl ModelCache {
    /// Creates a new, empty model cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Loads the model cache from disk.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    /// Returns `Ok(Some(cache))` if the file was loaded and parsed successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Option<Self>, Report<ModelCacheError>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)
            .change_context(ModelCacheError)
            .attach("failed to read model cache file")?;

        let cache: Self = serde_json::from_str(&content)
            .change_context(ModelCacheError)
            .attach("failed to parse model cache file")?;

        Ok(Some(cache))
    }

    /// Saves the model cache to disk.
    ///
    /// Creates parent directories if they do not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), Report<ModelCacheError>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .change_context(ModelCacheError)
                .attach("failed to create model cache directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .change_context(ModelCacheError)
            .attach("failed to serialize model cache")?;

        std::fs::write(path, content)
            .change_context(ModelCacheError)
            .attach("failed to write model cache file")?;

        Ok(())
    }
}

impl Default for ModelCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the path to the model cache file.
///
/// Uses `dirs::cache_dir()` → `~/.cache/nullslop/model_cache.json`.
#[must_use]
pub fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nullslop")
        .join("model_cache.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cache_is_empty() {
        // Given a new ModelCache.
        let cache = ModelCache::new();

        // Then it has no entries.
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        // Given a cache with entries.
        let mut cache = ModelCache::new();
        cache.entries.insert(
            "ollama".to_owned(),
            vec!["llama3".to_owned(), "mistral".to_owned()],
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model_cache.json");

        // When saving and loading.
        cache.save(&path).expect("save");
        let loaded = ModelCache::load(&path).expect("load");

        // Then the loaded cache matches the original.
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries["ollama"].len(), 2);
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        // Given a path to a nonexistent file.
        let path = PathBuf::from("/tmp/nullslop_test_nonexistent_cache.json");

        // When loading.
        let result = ModelCache::load(&path);

        // Then Ok(None) is returned.
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
