//! Filtered provider entries for the picker.
//!
//! Shared between the picker handler (component layer) and the picker
//! renderer (TUI layer). Computes the list of providers and aliases
//! matching the current filter text.

/// A provider entry ready for display in the picker.
#[derive(Debug, Clone)]
pub struct PickerEntry {
    /// Full provider ID in `{name}/{model}` format (e.g., `"ollama/llama3"`).
    /// For aliases, this is the resolved target's full ID.
    pub provider_id: String,
    /// Display name for the entry (provider block name or alias name).
    pub name: String,
    /// Provider block name (e.g., `"ollama"`). Used for display.
    pub provider_name: String,
    /// Backend type string.
    pub backend: String,
    /// Model identifier (primary display text).
    pub model: String,
    /// Whether this entry is an alias.
    pub is_alias: bool,
    /// Alias display target (e.g., `"ollama/llama3"`). Only set for aliases.
    pub alias_target: Option<String>,
    /// Whether this provider is available (API key present or keyless).
    pub is_available: bool,
}

/// Reorders entries so that available entries appear first (sorted by model name),
/// followed by unavailable entries (sorted by model name). When `filter` is empty,
/// the entry matching `active_provider` is promoted to the very top.
///
/// `active_provider` is in `{name}/{model}` format (e.g., `"ollama/llama3"`).
pub fn sorted_entries(
    entries: Vec<PickerEntry>,
    filter: &str,
    active_provider: &str,
) -> Vec<PickerEntry> {
    // Split into available and unavailable blocks.
    let mut available: Vec<PickerEntry> = entries
        .iter()
        .filter(|e| e.is_available)
        .cloned()
        .collect();
    let mut unavailable: Vec<PickerEntry> = entries
        .iter()
        .filter(|e| !e.is_available)
        .cloned()
        .collect();

    // Sort each block alphabetically by model name (case-insensitive).
    available.sort_by(|a, b| a.model.to_lowercase().cmp(&b.model.to_lowercase()));
    unavailable.sort_by(|a, b| a.model.to_lowercase().cmp(&b.model.to_lowercase()));

    // Promote active provider to top when filter is empty.
    if filter.is_empty() && active_provider != nullslop_providers::NO_PROVIDER_ID {
        if let Some(pos) = available.iter().position(|e| e.provider_id == active_provider) {
            if pos > 0 {
                available[0..=pos].rotate_right(1);
            }
        }
    }

    // Merge: available first, then unavailable.
    available.extend(unavailable);
    available
}
///
/// Reads the provider registry, API keys, and filter text to produce
/// a list of matching entries. Providers and aliases are included if
/// their name, backend, or model contains the filter text (case-insensitive).
pub fn filtered_entries(
    registry: &nullslop_providers::ProviderRegistry,
    api_keys: &nullslop_providers::ApiKeys,
    filter: &str,
) -> Vec<PickerEntry> {
    let filter_lower = filter.to_lowercase();
    let mut entries = Vec::new();

    for provider in registry.providers() {
        let entry = PickerEntry {
            provider_id: provider.id.to_string(),
            name: provider.name.clone(),
            provider_name: provider.name.clone(),
            backend: provider.backend.clone(),
            model: provider.model.clone(),
            is_alias: false,
            alias_target: None,
            is_available: registry.is_available(&provider.id.clone(), api_keys),
        };

        if filter_lower.is_empty()
            || entry.name.to_lowercase().contains(&filter_lower)
            || entry.backend.to_lowercase().contains(&filter_lower)
            || entry.model.to_lowercase().contains(&filter_lower)
        {
            entries.push(entry);
        }
    }

    for alias in registry.aliases() {
        let resolved = registry.resolve_alias(&alias.name);
        let is_available = resolved
            .is_some_and(|r| registry.is_available(&r.id.clone(), api_keys));

        let entry = PickerEntry {
            provider_id: resolved.map(|r| r.id.to_string()).unwrap_or_default(),
            name: alias.name.clone(),
            provider_name: resolved.map(|r| r.name.clone()).unwrap_or_default(),
            backend: resolved.map(|r| r.backend.clone()).unwrap_or_default(),
            model: resolved.map(|r| r.model.clone()).unwrap_or_default(),
            is_alias: true,
            alias_target: resolved.map(|r| r.id.to_string()),
            is_available,
        };

        if filter_lower.is_empty()
            || entry.name.to_lowercase().contains(&filter_lower)
            || entry.backend.to_lowercase().contains(&filter_lower)
            || entry.model.to_lowercase().contains(&filter_lower)
        {
            entries.push(entry);
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use nullslop_providers::{ApiKeys, ProviderEntry, ProviderRegistry, ProvidersConfig};

    use super::*;

    fn ollama_entry() -> ProviderEntry {
        ProviderEntry {
            name: "ollama".to_owned(),
            backend: "ollama".to_owned(),
            models: vec!["llama3".to_owned()],
            base_url: Some("http://localhost:11434".to_owned()),
            api_key_env: None,
            requires_key: false,
        }
    }

    fn openrouter_entry() -> ProviderEntry {
        ProviderEntry {
            name: "openrouter".to_owned(),
            backend: "openrouter".to_owned(),
            models: vec!["gpt-4".to_owned()],
            base_url: None,
            api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
            requires_key: true,
        }
    }

    fn make_config(
        providers: Vec<ProviderEntry>,
        aliases: Vec<nullslop_providers::AliasEntry>,
        default_provider: Option<&str>,
    ) -> ProvidersConfig {
        ProvidersConfig {
            providers,
            aliases,
            default_provider: default_provider.map(String::from),
        }
    }

    #[test]
    fn filtered_entries_returns_all_providers_with_empty_filter() {
        // Given a registry with one keyless and one key-required provider (key present).
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let mut api_keys = ApiKeys::new();
        api_keys.insert("OPENROUTER_API_KEY".to_owned(), "sk-test".to_owned());

        // When filtering with an empty string.
        let entries = filtered_entries(&registry, &api_keys, "");

        // Then both providers are returned with correct availability.
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].provider_id, "ollama/llama3");
        assert_eq!(entries[0].provider_name, "ollama");
        assert_eq!(entries[0].model, "llama3");
        assert!(entries[0].is_available);
        assert_eq!(entries[1].provider_id, "openrouter/gpt-4");
        assert_eq!(entries[1].provider_name, "openrouter");
        assert_eq!(entries[1].model, "gpt-4");
        assert!(entries[1].is_available);
    }

    #[test]
    fn filtered_entries_filters_by_name() {
        // Given a registry with "ollama" and "openrouter".
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When filtering by name fragment "oll".
        let entries = filtered_entries(&registry, &api_keys, "oll");

        // Then only the "ollama" entry is returned.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "ollama");
    }

    #[test]
    fn filtered_entries_filters_by_backend() {
        // Given a registry with providers using different backends.
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When filtering by backend fragment "openr".
        let entries = filtered_entries(&registry, &api_keys, "openr");

        // Then only the "openrouter" entry is returned.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "openrouter");
    }

    #[test]
    fn filtered_entries_filters_by_model() {
        // Given a registry with providers using different models.
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When filtering by model fragment "gpt".
        let entries = filtered_entries(&registry, &api_keys, "gpt");

        // Then only the "openrouter" entry (model "gpt-4") is returned.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "openrouter");
    }

    #[test]
    fn filtered_entries_filter_is_case_insensitive() {
        // Given a registry with "ollama".
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When filtering with uppercase "OLLA".
        let entries = filtered_entries(&registry, &api_keys, "OLLA");

        // Then "ollama" is matched.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "ollama");
    }

    #[test]
    fn filtered_entries_marks_key_required_unavailable_when_key_missing() {
        // Given a registry with a key-required provider and no API key.
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When computing filtered entries.
        let entries = filtered_entries(&registry, &api_keys, "");

        // Then the provider is marked unavailable.
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].is_available);
    }

    #[test]
    fn filtered_entries_marks_key_required_available_when_key_present() {
        // Given a registry with a key-required provider and the key set.
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let mut api_keys = ApiKeys::new();
        api_keys.insert("OPENROUTER_API_KEY".to_owned(), "sk-test".to_owned());

        // When computing filtered entries.
        let entries = filtered_entries(&registry, &api_keys, "");

        // Then the provider is marked available.
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_available);
    }

    #[test]
    fn filtered_entries_marks_keyless_always_available() {
        // Given a registry with a keyless provider and no API keys.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When computing filtered entries.
        let entries = filtered_entries(&registry, &api_keys, "");

        // Then the keyless provider is always available.
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_available);
    }

    #[test]
    fn filtered_entries_includes_aliases() {
        // Given a registry with a provider and an alias.
        let config = make_config(
            vec![ollama_entry()],
            vec![nullslop_providers::AliasEntry {
                name: "fast".to_owned(),
                target: "ollama/llama3".to_owned(),
            }],
            None,
        );
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When computing filtered entries.
        let entries = filtered_entries(&registry, &api_keys, "");

        // Then both the provider and alias are present.
        assert_eq!(entries.len(), 2);
        let alias = entries.iter().find(|e| e.is_alias).expect("alias entry");
        assert_eq!(alias.name, "fast");
        assert_eq!(alias.provider_id, "ollama/llama3");
        assert_eq!(alias.alias_target.as_deref(), Some("ollama/llama3"));
        assert_eq!(alias.provider_name, "ollama");
        assert_eq!(alias.backend, "ollama");
        assert_eq!(alias.model, "llama3");
    }

    #[test]
    fn filtered_entries_alias_inherits_availability() {
        // Given a registry with an alias pointing to an available provider
        // and an alias pointing to an unavailable provider.
        let config = make_config(
            vec![ollama_entry(), openrouter_entry()],
            vec![
                nullslop_providers::AliasEntry {
                    name: "fast".to_owned(),
                    target: "ollama/llama3".to_owned(),
                },
                nullslop_providers::AliasEntry {
                    name: "cloud".to_owned(),
                    target: "openrouter/gpt-4".to_owned(),
                },
            ],
            None,
        );
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new(); // No keys — openrouter is unavailable.

        // When computing filtered entries.
        let entries = filtered_entries(&registry, &api_keys, "");

        // Then the "fast" alias (→ollama) is available and "cloud" alias (→openrouter) is not.
        let fast = entries.iter().find(|e| e.name == "fast").expect("fast");
        let cloud = entries.iter().find(|e| e.name == "cloud").expect("cloud");
        assert!(fast.is_available);
        assert!(!cloud.is_available);
    }

    #[test]
    fn filtered_entries_returns_empty_for_no_match() {
        // Given a registry with providers.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When filtering with a string that matches nothing.
        let entries = filtered_entries(&registry, &api_keys, "xyzzy");

        // Then no entries are returned.
        assert!(entries.is_empty());
    }

    // --- sorted_entries tests ---

    #[test]
    fn sorted_entries_moves_active_to_top_when_filter_empty() {
        // Given entries ["a/model", "b/model", "c/model"] with active_provider "c/model" and empty filter.
        let entries = vec![
        PickerEntry { provider_id: "a/model".into(), name: "a".into(), provider_name: "a".into(), backend: "a".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
        PickerEntry { provider_id: "b/model".into(), name: "b".into(), provider_name: "b".into(), backend: "b".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
        PickerEntry { provider_id: "c/model".into(), name: "c".into(), provider_name: "c".into(), backend: "c".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
    ];

        // When sorting with empty filter and active_provider "c/model".
        let result = sorted_entries(entries, "", "c/model");

        // Then "c/model" is first and order of others is preserved.
        assert_eq!(result[0].provider_id, "c/model");
        assert_eq!(result[1].provider_id, "a/model");
        assert_eq!(result[2].provider_id, "b/model");
    }

    #[test]
    fn sorted_entries_preserves_order_when_filtering() {
        // Given entries ["a/model", "b/model"] with active_provider "b/model" and non-empty filter.
        let entries = vec![
        PickerEntry { provider_id: "a/model".into(), name: "a".into(), provider_name: "a".into(), backend: "a".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
        PickerEntry { provider_id: "b/model".into(), name: "b".into(), provider_name: "b".into(), backend: "b".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
    ];

        // When sorting with filter "a" and active_provider "b/model".
        let result = sorted_entries(entries, "a", "b/model");

        // Then order is unchanged (filter is non-empty).
        assert_eq!(result[0].provider_id, "a/model");
        assert_eq!(result[1].provider_id, "b/model");
    }

    #[test]
    fn sorted_entries_preserves_order_when_no_active() {
        // Given entries with active_provider "__no_provider__" and empty filter.
        let entries = vec![
        PickerEntry { provider_id: "a/model".into(), name: "a".into(), provider_name: "a".into(), backend: "a".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
        PickerEntry { provider_id: "b/model".into(), name: "b".into(), provider_name: "b".into(), backend: "b".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
    ];

        // When sorting with empty filter and no active provider.
        let result = sorted_entries(entries, "", "__no_provider__");

        // Then entries are sorted by model name (both "model", so relative order preserved).
        assert_eq!(result[0].provider_id, "a/model");
        assert_eq!(result[1].provider_id, "b/model");
    }

    #[test]
    fn sorted_entries_available_before_unavailable() {
        // Given entries with mixed availability.
        let entries = vec![
        PickerEntry { provider_id: "z/model".into(), name: "z".into(), provider_name: "z".into(), backend: "z".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: false },
        PickerEntry { provider_id: "a/model".into(), name: "a".into(), provider_name: "a".into(), backend: "a".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: true },
        PickerEntry { provider_id: "b/model".into(), name: "b".into(), provider_name: "b".into(), backend: "b".into(), model: "model".into(), is_alias: false, alias_target: None, is_available: false },
    ];

        // When sorting with empty filter and no active provider.
        let result = sorted_entries(entries, "", "__no_provider__");

        // Then available entry comes first, followed by unavailable entries sorted by model.
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].provider_id, "a/model");
        assert!(result[0].is_available);
        assert!(!result[1].is_available);
        assert!(!result[2].is_available);
    }

    #[test]
    fn sorted_entries_sorts_by_model_name_within_blocks() {
        // Given entries with different model names.
        let entries = vec![
        PickerEntry { provider_id: "a/zebra".into(), name: "a".into(), provider_name: "a".into(), backend: "a".into(), model: "zebra".into(), is_alias: false, alias_target: None, is_available: true },
        PickerEntry { provider_id: "b/alpha".into(), name: "b".into(), provider_name: "b".into(), backend: "b".into(), model: "alpha".into(), is_alias: false, alias_target: None, is_available: true },
    ];

        // When sorting with empty filter and no active provider.
        let result = sorted_entries(entries, "", "__no_provider__");

        // Then entries are sorted alphabetically by model name.
        assert_eq!(result[0].provider_id, "b/alpha");
        assert_eq!(result[1].provider_id, "a/zebra");
    }
}
