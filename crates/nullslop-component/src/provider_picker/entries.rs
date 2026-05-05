//! Provider entries for the picker.
//!
//! Builds the list of providers and aliases available for selection,
//! and implements [`PickerItem`] so [`SelectionState`] can fuzzy-filter
//! and render them. Also provides footer formatting utilities for the
//! provider picker overlay.
//!
//! [`PickerItem`]: nullslop_selection_widget::PickerItem
//! [`SelectionState`]: nullslop_selection_widget::SelectionState

/// A provider entry ready for display in the picker.
#[derive(Debug, Clone)]
pub struct PickerEntry {
    /// Full provider ID in `{name}/{model}` format (e.g., `"ollama/llama3"`).
    /// For aliases, this is the resolved target's full ID.
    /// For remote entries, this is `{provider_name}/{model}`.
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
    /// Whether this entry was discovered from a remote provider (not in static config).
    pub is_remote: bool,
    /// Whether this entry is the currently active provider.
    pub is_active: bool,
}

impl nullslop_selection_widget::PickerItem for PickerEntry {
    fn display_label(&self) -> &str {
        // Use model as the primary label. Fuzzy matching via SelectionState
        // searches this plus name/backend through the matcher.
        &self.model
    }

    fn render_row(&self, is_selected: bool) -> ratatui::text::Line<'static> {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};

        let active_marker = Span::styled(
            if self.is_active { "> " } else { "  " },
            if self.is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        );

        let status = if !self.is_available {
            "\u{2717} " // ✗
        } else if self.is_alias {
            "\u{2192} " // →
        } else if self.is_remote {
            "* "
        } else {
            "  "
        };

        let label = if self.is_alias {
            format!(
                "{}{} → {} ({})",
                status, self.name, self.model, self.provider_name
            )
        } else {
            format!("{}{} ({})", status, self.model, self.provider_name)
        };

        let label_style = if is_selected {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else if !self.is_available {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        Line::from(vec![active_marker, Span::styled(label, label_style)])
    }
}

/// Reorders entries so that available entries appear first (sorted by model name),
/// followed by unavailable entries (sorted by model name). When `filter` is empty,
/// the entry matching `active_provider` is promoted to the very top and marked active.
///
/// `active_provider` is in `{name}/{model}` format (e.g., `"ollama/llama3"`).
pub fn sorted_entries(
    entries: &[PickerEntry],
    filter: &str,
    active_provider: &str,
) -> Vec<PickerEntry> {
    // Split into available and unavailable blocks.
    let mut available: Vec<PickerEntry> =
        entries.iter().filter(|e| e.is_available).cloned().collect();
    let mut unavailable: Vec<PickerEntry> = entries
        .iter()
        .filter(|e| !e.is_available)
        .cloned()
        .collect();

    // Sort each block alphabetically by model name (case-insensitive).
    available.sort_by(|a, b| a.model.to_lowercase().cmp(&b.model.to_lowercase()));
    unavailable.sort_by(|a, b| a.model.to_lowercase().cmp(&b.model.to_lowercase()));

    // Promote active provider to top when filter is empty.
    if filter.is_empty()
        && active_provider != nullslop_providers::NO_PROVIDER_ID
        && let Some(pos) = available
            .iter()
            .position(|e| e.provider_id == active_provider)
        && pos > 0
    {
        #[expect(
            clippy::indexing_slicing,
            reason = "pos comes from iter().position() on the same vec"
        )]
        available[0..=pos].rotate_right(1);
    }

    // Mark active entries.
    for entry in &mut available {
        entry.is_active = entry.provider_id == active_provider;
    }
    // Unavailable entries are never active.
    // (is_active defaults to false from load_provider_entries)

    // Merge: available first, then unavailable.
    available.extend(unavailable);
    available
}

/// Formats the footer line showing refresh keybind and last update time.
///
/// Returns a styled [`Line`] with the pipe separator in dark gray.
/// Format: `CTRL+R to refresh | Updated <timestamp> (<humantime> ago)`
pub fn format_footer(
    last_refreshed_at: Option<&jiff::Timestamp>,
    width: usize,
) -> ratatui::text::Line<'static> {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};

    let gray = Style::default().fg(Color::DarkGray);
    let orange = Style::default().fg(Color::Rgb(255, 165, 0));

    if let Some(ts) = last_refreshed_at {
        let elapsed = jiff::Timestamp::now() - *ts;
        let secs = elapsed.total(jiff::Unit::Second).unwrap_or(0.0).round() as u64;
        let duration = std::time::Duration::from_secs(secs);
        let human = humantime::format_duration(duration);
        let age_color = age_color(secs);

        // Format timestamp without fractional seconds.
        let formatted_ts = format!("{ts:.0}");

        let left = "CTRL+R to refresh ";
        let pipe = "|";
        let mid = format!(" Updated {formatted_ts} (");
        let right = format!("{human} ago)");

        let line = Line::from(vec![
            Span::styled(left.to_owned(), orange),
            Span::styled(pipe.to_owned(), gray),
            Span::styled(mid, gray),
            Span::styled(right, Style::default().fg(age_color)),
        ]);
        truncate_line(line, width)
    } else {
        let left = "CTRL+R to refresh ";
        let pipe = "|";
        let right = " Updated never";

        let line = Line::from(vec![
            Span::styled(left.to_owned(), orange),
            Span::styled(pipe.to_owned(), gray),
            Span::styled(right.to_owned(), gray),
        ]);
        truncate_line(line, width)
    }
}

/// Returns the age-based color for the "time ago" text.
///
/// - `<= 2 weeks` → light green
/// - `> 2 weeks, <= 4 weeks` → yellow
/// - `> 4 weeks` → red
pub fn age_color(secs: u64) -> ratatui::style::Color {
    use ratatui::style::Color;

    const TWO_WEEKS: u64 = 14 * 24 * 60 * 60;
    const FOUR_WEEKS: u64 = 28 * 24 * 60 * 60;
    if secs <= TWO_WEEKS {
        Color::LightGreen
    } else if secs <= FOUR_WEEKS {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Truncates a styled line to fit within `width` terminal columns.
pub fn truncate_line(
    line: ratatui::text::Line<'static>,
    width: usize,
) -> ratatui::text::Line<'static> {
    use ratatui::text::{Line, Span};

    let total_len: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    if total_len <= width {
        return line;
    }

    // Rebuild spans, trimming characters that overflow.
    let mut remaining = width;
    let mut spans = Vec::new();
    for span in line.spans {
        let char_count = span.content.chars().count();
        if remaining == 0 {
            break;
        }
        if char_count <= remaining {
            spans.push(span);
            remaining -= char_count;
        } else {
            let truncated: String = span.content.chars().take(remaining).collect();
            spans.push(Span::styled(truncated, span.style));
            remaining = 0;
        }
    }
    Line::from(spans)
}

/// Loads all provider and alias entries from the registry, ready for `set_items()`.
///
/// Reads the provider registry, API keys, and optional model cache
/// to produce the full list of entries. No filtering is applied — that is
/// handled by [`SelectionState`] via fuzzy matching on [`PickerItem::display_label`].
///
/// Remote models from the cache are merged in after static entries. Static entries
/// win on collision (same `{provider_name}/{model}` key). Remote entries are marked
/// with `is_remote: true`.
///
/// [`SelectionState`]: nullslop_selection_widget::SelectionState
/// [`PickerItem`]: nullslop_selection_widget::PickerItem
pub fn load_provider_entries(
    registry: &nullslop_providers::ProviderRegistry,
    api_keys: &nullslop_providers::ApiKeys,
    model_cache: Option<&nullslop_providers::ModelCache>,
) -> Vec<PickerEntry> {
    let mut entries = Vec::new();

    // Collect static provider IDs for collision detection.
    let mut static_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

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
            is_remote: false,
            is_active: false,
        };

        static_ids.insert(entry.provider_id.clone());
        entries.push(entry);
    }

    for alias in registry.aliases() {
        let resolved = registry.resolve_alias(&alias.name);
        let is_available = resolved.is_some_and(|r| registry.is_available(&r.id.clone(), api_keys));

        let entry = PickerEntry {
            provider_id: resolved.map(|r| r.id.to_string()).unwrap_or_default(),
            name: alias.name.clone(),
            provider_name: resolved.map(|r| r.name.clone()).unwrap_or_default(),
            backend: resolved.map(|r| r.backend.clone()).unwrap_or_default(),
            model: resolved.map(|r| r.model.clone()).unwrap_or_default(),
            is_alias: true,
            alias_target: resolved.map(|r| r.id.to_string()),
            is_available,
            is_remote: false,
            is_active: false,
        };

        entries.push(entry);
    }

    // Merge remote models from cache.
    if let Some(cache) = model_cache {
        let config = registry.config();
        for (provider_name, models) in &cache.entries {
            // Find the provider entry for backend/availability info.
            let provider_entry = config.providers.iter().find(|p| &p.name == provider_name);

            let (backend, is_available) = match provider_entry {
                Some(pe) => {
                    let avail = if pe.requires_key {
                        pe.api_key_env
                            .as_ref()
                            .is_some_and(|env| api_keys.is_set(env))
                    } else {
                        true
                    };
                    (pe.backend.clone(), avail)
                }
                None => {
                    // Unknown provider in cache — still show it but mark unavailable.
                    ("unknown".to_owned(), false)
                }
            };

            for model in models {
                let provider_id = format!("{provider_name}/{model}");

                // Static wins on collision.
                if static_ids.contains(&provider_id) {
                    continue;
                }

                let entry = PickerEntry {
                    provider_id,
                    name: provider_name.clone(),
                    provider_name: provider_name.clone(),
                    backend: backend.clone(),
                    model: model.clone(),
                    is_alias: false,
                    alias_target: None,
                    is_available,
                    is_remote: true,
                    is_active: false,
                };

                entries.push(entry);
            }
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
    fn load_provider_entries_returns_all_providers() {
        // Given a registry with one keyless and one key-required provider (key present).
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let mut api_keys = ApiKeys::new();
        api_keys.insert("OPENROUTER_API_KEY".to_owned(), "sk-test".to_owned());

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, None);

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
    fn load_provider_entries_includes_all_regardless_of_text() {
        // Given a registry with "ollama" and "openrouter".
        let config = make_config(vec![ollama_entry(), openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When loading entries (no filter — returns everything).
        let entries = load_provider_entries(&registry, &api_keys, None);

        // Then both providers are returned.
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_provider_entries_marks_key_required_unavailable_when_key_missing() {
        // Given a registry with a key-required provider and no API key.
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, None);

        // Then the provider is marked unavailable.
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].is_available);
    }

    #[test]
    fn load_provider_entries_marks_key_required_available_when_key_present() {
        // Given a registry with a key-required provider and the key set.
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let mut api_keys = ApiKeys::new();
        api_keys.insert("OPENROUTER_API_KEY".to_owned(), "sk-test".to_owned());

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, None);

        // Then the provider is marked available.
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_available);
    }

    #[test]
    fn load_provider_entries_marks_keyless_always_available() {
        // Given a registry with a keyless provider and no API keys.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, None);

        // Then the keyless provider is always available.
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_available);
    }

    #[test]
    fn load_provider_entries_includes_aliases() {
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

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, None);

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
    fn load_provider_entries_alias_inherits_availability() {
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

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, None);

        // Then the "fast" alias (→ollama) is available and "cloud" alias (→openrouter) is not.
        let fast = entries.iter().find(|e| e.name == "fast").expect("fast");
        let cloud = entries.iter().find(|e| e.name == "cloud").expect("cloud");
        assert!(fast.is_available);
        assert!(!cloud.is_available);
    }

    // --- Remote model cache tests ---

    #[test]
    fn load_provider_entries_merges_remote_models_from_cache() {
        // Given a registry with one keyless provider (ollama/llama3).
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // And a cache with an additional model for the same provider.
        let mut cache = nullslop_providers::ModelCache::new();
        cache
            .entries
            .insert("ollama".to_owned(), vec!["mistral".to_owned()]);

        // When loading provider entries with the cache.
        let entries = load_provider_entries(&registry, &api_keys, Some(&cache));

        // Then both static and remote entries are present.
        assert_eq!(entries.len(), 2);
        let static_entry = entries
            .iter()
            .find(|e| e.model == "llama3")
            .expect("static");
        assert!(!static_entry.is_remote);
        let remote_entry = entries
            .iter()
            .find(|e| e.model == "mistral")
            .expect("remote");
        assert!(remote_entry.is_remote);
        assert_eq!(remote_entry.provider_id, "ollama/mistral");
        assert!(remote_entry.is_available); // Keyless provider
    }

    #[test]
    fn load_provider_entries_static_wins_on_collision_with_cache() {
        // Given a registry with ollama/llama3.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        // And a cache that also contains ollama/llama3 (collision).
        let mut cache = nullslop_providers::ModelCache::new();
        cache.entries.insert(
            "ollama".to_owned(),
            vec!["llama3".to_owned(), "mistral".to_owned()],
        );

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, Some(&cache));

        // Then the static entry is kept (not duplicated) and only the new remote model is added.
        let llama3_entries: Vec<_> = entries.iter().filter(|e| e.model == "llama3").collect();
        assert_eq!(llama3_entries.len(), 1);
        assert!(!llama3_entries[0].is_remote);

        let mistral_entries: Vec<_> = entries.iter().filter(|e| e.model == "mistral").collect();
        assert_eq!(mistral_entries.len(), 1);
        assert!(mistral_entries[0].is_remote);
    }

    #[test]
    fn load_provider_entries_marks_remote_unavailable_when_key_missing() {
        // Given a registry with a key-required provider (openrouter).
        let config = make_config(vec![openrouter_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new(); // No keys set.

        // And a cache with additional models.
        let mut cache = nullslop_providers::ModelCache::new();
        cache
            .entries
            .insert("openrouter".to_owned(), vec!["claude-3".to_owned()]);

        // When loading provider entries.
        let entries = load_provider_entries(&registry, &api_keys, Some(&cache));

        // Then the remote model is marked unavailable (no API key).
        let remote = entries
            .iter()
            .find(|e| e.model == "claude-3")
            .expect("remote");
        assert!(remote.is_remote);
        assert!(!remote.is_available);
    }

    #[test]
    fn load_provider_entries_includes_all_remote_models() {
        // Given a registry and cache with remote models.
        let config = make_config(vec![ollama_entry()], vec![], None);
        let registry = ProviderRegistry::from_config(config).expect("registry");
        let api_keys = ApiKeys::new();

        let mut cache = nullslop_providers::ModelCache::new();
        cache.entries.insert(
            "ollama".to_owned(),
            vec!["mistral".to_owned(), "codellama".to_owned()],
        );

        // When loading entries (no filter — returns everything).
        let entries = load_provider_entries(&registry, &api_keys, Some(&cache));

        // Then all 3 entries are present (1 static + 2 remote).
        assert_eq!(entries.len(), 3);
    }

    // --- sorted_entries tests ---

    #[test]
    fn sorted_entries_moves_active_to_top_when_filter_empty() {
        // Given entries ["a/model", "b/model", "c/model"] with active_provider "c/model" and empty filter.
        let entries = vec![
            PickerEntry {
                provider_id: "a/model".into(),
                name: "a".into(),
                provider_name: "a".into(),
                backend: "a".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "b/model".into(),
                name: "b".into(),
                provider_name: "b".into(),
                backend: "b".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "c/model".into(),
                name: "c".into(),
                provider_name: "c".into(),
                backend: "c".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
        ];

        // When sorting with empty filter and active_provider "c/model".
        let result = sorted_entries(&entries, "", "c/model");

        // Then "c/model" is first (promoted) and marked active.
        assert_eq!(result[0].provider_id, "c/model");
        assert!(result[0].is_active);
        assert_eq!(result[1].provider_id, "a/model");
        assert!(!result[1].is_active);
        assert_eq!(result[2].provider_id, "b/model");
        assert!(!result[2].is_active);
    }

    #[test]
    fn sorted_entries_preserves_order_when_filtering() {
        // Given entries ["a/model", "b/model"] with active_provider "b/model" and non-empty filter.
        let entries = vec![
            PickerEntry {
                provider_id: "a/model".into(),
                name: "a".into(),
                provider_name: "a".into(),
                backend: "a".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "b/model".into(),
                name: "b".into(),
                provider_name: "b".into(),
                backend: "b".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
        ];

        // When sorting with filter "a" and active_provider "b/model".
        let result = sorted_entries(&entries, "a", "b/model");

        // Then order is unchanged (filter is non-empty).
        assert_eq!(result[0].provider_id, "a/model");
        assert_eq!(result[1].provider_id, "b/model");
    }

    #[test]
    fn sorted_entries_preserves_order_when_no_active() {
        // Given entries with active_provider "__no_provider__" and empty filter.
        let entries = vec![
            PickerEntry {
                provider_id: "a/model".into(),
                name: "a".into(),
                provider_name: "a".into(),
                backend: "a".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "b/model".into(),
                name: "b".into(),
                provider_name: "b".into(),
                backend: "b".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
        ];

        // When sorting with empty filter and no active provider.
        let result = sorted_entries(&entries, "", "__no_provider__");

        // Then entries are sorted by model name (both "model", so relative order preserved).
        assert_eq!(result[0].provider_id, "a/model");
        assert_eq!(result[1].provider_id, "b/model");
    }

    #[test]
    fn sorted_entries_available_before_unavailable() {
        // Given entries with mixed availability.
        let entries = vec![
            PickerEntry {
                provider_id: "z/model".into(),
                name: "z".into(),
                provider_name: "z".into(),
                backend: "z".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: false,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "a/model".into(),
                name: "a".into(),
                provider_name: "a".into(),
                backend: "a".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "b/model".into(),
                name: "b".into(),
                provider_name: "b".into(),
                backend: "b".into(),
                model: "model".into(),
                is_alias: false,
                alias_target: None,
                is_available: false,
                is_remote: false,
                is_active: false,
            },
        ];

        // When sorting with empty filter and no active provider.
        let result = sorted_entries(&entries, "", "__no_provider__");

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
            PickerEntry {
                provider_id: "a/zebra".into(),
                name: "a".into(),
                provider_name: "a".into(),
                backend: "a".into(),
                model: "zebra".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
            PickerEntry {
                provider_id: "b/alpha".into(),
                name: "b".into(),
                provider_name: "b".into(),
                backend: "b".into(),
                model: "alpha".into(),
                is_alias: false,
                alias_target: None,
                is_available: true,
                is_remote: false,
                is_active: false,
            },
        ];

        // When sorting with empty filter and no active provider.
        let result = sorted_entries(&entries, "", "__no_provider__");

        // Then entries are sorted alphabetically by model name.
        assert_eq!(result[0].provider_id, "b/alpha");
        assert_eq!(result[1].provider_id, "a/zebra");
    }

    // --- format_footer / age_color / truncate_line tests ---

    #[test]
    fn format_footer_without_timestamp_shows_never() {
        // Given no last_refreshed_at timestamp.
        // When formatting the footer.
        let line = format_footer(None, 80);

        // Then the footer contains "Updated never".
        let text: String = line.spans.iter().map(|s| &*s.content).collect();
        assert!(text.contains("Updated never"));
        assert!(text.contains("CTRL+R to refresh"));
    }

    #[test]
    fn format_footer_with_timestamp_shows_age() {
        // Given a recent timestamp (1 second ago).
        let ts = jiff::Timestamp::now().checked_sub(
            jiff::Span::new().try_seconds(1).unwrap(),
        ).unwrap();

        // When formatting the footer.
        let line = format_footer(Some(&ts), 120);

        // Then the footer contains "Updated" and "ago".
        let text: String = line.spans.iter().map(|s| &*s.content).collect();
        assert!(text.contains("Updated"));
        assert!(text.contains("ago"));
        assert!(text.contains("CTRL+R to refresh"));
    }

    #[test]
    fn format_footer_truncates_to_width() {
        // Given no timestamp and a very narrow width.
        // When formatting the footer with width 10.
        let line = format_footer(None, 10);

        // Then the total character count fits within 10.
        let total_len: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        assert!(total_len <= 10);
    }

    #[test]
    fn age_color_returns_light_green_within_two_weeks() {
        // Given 1 second (well within 2 weeks).
        // When computing age color.
        let color = age_color(1);

        // Then the color is LightGreen.
        assert_eq!(color, ratatui::style::Color::LightGreen);
    }

    #[test]
    fn age_color_returns_light_green_at_exactly_two_weeks() {
        // Given exactly 2 weeks in seconds.
        let two_weeks = 14 * 24 * 60 * 60;

        // When computing age color.
        let color = age_color(two_weeks);

        // Then the color is LightGreen.
        assert_eq!(color, ratatui::style::Color::LightGreen);
    }

    #[test]
    fn age_color_returns_yellow_between_two_and_four_weeks() {
        // Given 3 weeks in seconds (between 2 and 4 weeks).
        let three_weeks = 21 * 24 * 60 * 60;

        // When computing age color.
        let color = age_color(three_weeks);

        // Then the color is Yellow.
        assert_eq!(color, ratatui::style::Color::Yellow);
    }

    #[test]
    fn age_color_returns_yellow_at_exactly_four_weeks() {
        // Given exactly 4 weeks in seconds.
        let four_weeks = 28 * 24 * 60 * 60;

        // When computing age color.
        let color = age_color(four_weeks);

        // Then the color is Yellow.
        assert_eq!(color, ratatui::style::Color::Yellow);
    }

    #[test]
    fn age_color_returns_red_beyond_four_weeks() {
        // Given 5 weeks in seconds (beyond 4 weeks).
        let five_weeks = 35 * 24 * 60 * 60;

        // When computing age color.
        let color = age_color(five_weeks);

        // Then the color is Red.
        assert_eq!(color, ratatui::style::Color::Red);
    }

    #[test]
    fn truncate_line_noop_when_fits() {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        // Given a line that is 10 characters wide.
        let line = Line::from(vec![
            Span::styled("hello ".to_owned(), Style::default()),
            Span::styled("world".to_owned(), Style::default().fg(Color::Red)),
        ]);

        // When truncating to width 20.
        let result = truncate_line(line.clone(), 20);

        // Then the line is unchanged.
        assert_eq!(result.spans.len(), 2);
        assert_eq!(result.spans[0].content, "hello ");
        assert_eq!(result.spans[1].content, "world");
    }

    #[test]
    fn truncate_line_fits_within_width() {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        // Given a line that is 20 characters wide.
        let line = Line::from(vec![
            Span::styled("hello world ".to_owned(), Style::default()),
            Span::styled("test12345".to_owned(), Style::default().fg(Color::Red)),
        ]);

        // When truncating to width 8.
        let result = truncate_line(line, 8);

        // Then the total character count is exactly 8.
        let total_len: usize = result.spans.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(total_len, 8);
    }

    #[test]
    fn truncate_line_preserves_style_on_partial_span() {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        // Given a line where the second span will be partially truncated.
        let line = Line::from(vec![
            Span::styled("hello ".to_owned(), Style::default()),
            Span::styled("world".to_owned(), Style::default().fg(Color::Red)),
        ]);

        // When truncating to width 8.
        let result = truncate_line(line, 8);

        // Then the first span is kept whole and the second is truncated.
        assert_eq!(result.spans.len(), 2);
        assert_eq!(result.spans[0].content, "hello ");
        assert_eq!(result.spans[1].content, "wo");
        // And the partial span retains its style.
        assert_eq!(result.spans[1].style.fg, Some(Color::Red));
    }
}
