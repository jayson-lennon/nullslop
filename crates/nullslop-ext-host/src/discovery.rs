//! Manifest discovery from the filesystem.
//!
//! Scans a base directory for extension manifests, validates them,
//! and returns the discovered extensions.

use std::path::Path;

use error_stack::{Report, ResultExt};

use nullslop_core::{ExtensionError, ExtensionManifest};

/// Discovers extension manifests in the given directory.
///
/// Scans `<base_dir>/*/manifest.json` — one manifest per subdirectory.
/// The `binary` field in each manifest is validated: must be relative,
/// no `..` traversal, no absolute paths.
///
/// # Errors
///
/// Returns [`ExtensionError`] if the base directory cannot be read.
pub fn discover_manifests(
    base_dir: &Path,
) -> Result<Vec<(std::path::PathBuf, ExtensionManifest)>, Report<ExtensionError>> {
    if !base_dir.exists() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(base_dir)
        .change_context(ExtensionError)
        .attach("failed to read extension directory")?;
    let mut manifests = Vec::new();

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&manifest_path) else {
            continue;
        };

        let manifest: ExtensionManifest = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Validate binary path: reject absolute paths and traversal.
        if is_invalid_binary_path(&manifest.binary) {
            tracing::warn!(
                name = %manifest.name,
                binary = %manifest.binary,
                "rejecting extension with invalid binary path"
            );
            continue;
        }

        manifests.push((path, manifest));
    }

    Ok(manifests)
}

/// Checks if a binary path is invalid (absolute or contains traversal).
fn is_invalid_binary_path(path: &str) -> bool {
    // Reject absolute paths.
    if path.starts_with('/') {
        return true;
    }
    // On Windows, reject drive letter paths.
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return true;
    }
    // Reject traversal.
    for component in std::path::Path::new(path).components() {
        if let std::path::Component::ParentDir = component {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn discover_finds_valid_manifest() {
        // Given a temp dir with a valid extension manifest.
        let dir = tempfile::tempdir().expect("tempdir");
        let ext_dir = dir.path().join("my-ext");
        fs::create_dir_all(&ext_dir).expect("mkdir");

        let manifest = r#"{
            "name": "my-ext",
            "version": "0.1.0",
            "binary": "my-ext-bin",
            "activation_events": ["onStartup"],
            "commands": ["foo"],
            "events": []
        }"#;
        fs::write(ext_dir.join("manifest.json"), manifest).expect("write");

        // When discovering manifests.
        let results = discover_manifests(dir.path()).expect("discover");

        // Then one manifest is found.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.name, "my-ext");
    }

    #[test]
    fn discover_missing_dir_returns_empty() {
        // Given a non-existent directory.
        let path = std::path::PathBuf::from("/tmp/nullslop-nonexistent-test-dir");

        // When discovering manifests.
        let results = discover_manifests(&path).expect("discover");

        // Then result is empty.
        assert!(results.is_empty());
    }

    #[test]
    fn discover_rejects_traversal() {
        // Given a temp dir with a manifest containing traversal in binary.
        let dir = tempfile::tempdir().expect("tempdir");
        let ext_dir = dir.path().join("evil-ext");
        fs::create_dir_all(&ext_dir).expect("mkdir");

        let manifest = r#"{
            "name": "evil-ext",
            "version": "0.1.0",
            "binary": "../../usr/bin/evil",
            "activation_events": [],
            "commands": [],
            "events": []
        }"#;
        fs::write(ext_dir.join("manifest.json"), manifest).expect("write");

        // When discovering manifests.
        let results = discover_manifests(dir.path()).expect("discover");

        // Then no manifests are found (traversal rejected).
        assert!(results.is_empty());
    }
}
