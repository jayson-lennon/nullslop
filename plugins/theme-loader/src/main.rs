//! The first-party themes plugin.
//!
//! Scans the granted themes directories for `*.toml` files, parses each
//! through the same loader core uses, and pushes the full set to the host
//! as one [`SetThemeEntries`] contribution. The granted directories arrive
//! in the handshake's [`Welcome`] — the plugin never guesses paths.
//!
//! Wire behavior: `Hello` → (await `Welcome`) → one `SetThemeEntries` →
//! exit. The host keeps the contribution cached after guest end.

use std::collections::BTreeMap;
use std::path::Path;

use jinn_plugin_api::{PluginToHost, SetThemeEntries, THEME_COLOR_SLOTS, ThemeColorSlot, ThemeDef};
use jinn_plugin_sdk::{PluginOutput, hello, push, welcome};
use jinn_theme::color::ThemeColor;
use jinn_theme::loader::{ThemeError, discover_themes, load_theme_from_file};

/// Theme-load failures carried between scan helpers.
type ThemeReport = error_stack::Report<ThemeError>;

fn main() {
    let mut out = PluginOutput::stdout();
    if hello(&mut out, "theme-loader").is_err() {
        return note_exit("handshake write failed");
    }
    let Ok(grants) = welcome() else {
        return note_exit("no Welcome from host");
    };

    let themes = collect_themes(&grants.read_dirs);
    if push(
        &mut out,
        PluginToHost::SetThemeEntries(SetThemeEntries { themes }),
    )
    .is_err()
    {
        note_exit("contribution write failed");
    }
}

/// Writes a diagnostic to stderr (host-side diagnostics) and returns.
fn note_exit(message: &str) {
    eprintln!("theme-loader: {message}");
}

/// Scans the granted read dirs (earlier dirs shadow same-name later ones,
/// matching the user-overrides-system rule core had) and translates each
/// loadable theme to its wire shape. Unloadable files are skipped with a
/// note on stderr — one bad file never drops the batch.
fn collect_themes(read_dirs: &[String]) -> Vec<ThemeDef> {
    let mut defs = BTreeMap::new();
    for dir in read_dirs {
        let dir = Path::new(dir);
        match discover_themes(dir) {
            Ok(found) => merge_dir(&mut defs, dir, &found),
            Err(report) => note_scan_failure(dir, &report),
        }
    }
    defs.into_values().collect()
}

/// Merges one directory's discoveries into the accumulated set.
fn merge_dir(
    defs: &mut BTreeMap<String, ThemeDef>,
    dir: &Path,
    found: &[(String, std::path::PathBuf)],
) {
    for (name, path) in found {
        match load_theme_from_file(path) {
            Ok(theme) => {
                defs.insert(name.clone(), to_def(name, &theme));
            }
            Err(report) => note_theme_failure(dir, name, &report),
        }
    }
}

/// Notes one theme's load failure on stderr (host-side diagnostics).
fn note_theme_failure(dir: &Path, name: &str, report: &ThemeReport) {
    eprintln!(
        "theme-loader: skipping theme {name} in {}: {report}",
        dir.display()
    );
}

/// Notes one directory's scan failure on stderr.
fn note_scan_failure(dir: &Path, report: &ThemeReport) {
    eprintln!("theme-loader: cannot scan {}: {report}", dir.display());
}

/// Converts one resolved core theme to its wire shape.
fn to_def(name: &str, theme: &jinn_theme::Theme) -> ThemeDef {
    let mut colors = BTreeMap::new();
    for &slot in THEME_COLOR_SLOTS {
        let color = slot_color(theme, slot);
        colors.insert(slot.key().to_owned(), color);
    }
    ThemeDef {
        name: name.to_owned(),
        description: None,
        colors,
    }
}

/// One slot's color rendered in a format the wire accepts (hex or ANSI name).
fn slot_color(theme: &jinn_theme::Theme, slot: ThemeColorSlot) -> String {
    let color = match slot {
        ThemeColorSlot::FocusAccent => theme.focus_accent,
        ThemeColorSlot::BorderUnfocused => theme.border_unfocused,
        ThemeColorSlot::PopupTitle => theme.popup_title,
        ThemeColorSlot::PrimaryText => theme.primary_text,
        ThemeColorSlot::MutedText => theme.muted_text,
        ThemeColorSlot::SubagentFg => theme.subagent_fg,
        ThemeColorSlot::ErrorText => theme.error_text,
        ThemeColorSlot::Success => theme.success,
        ThemeColorSlot::Warning => theme.warning,
        ThemeColorSlot::Streaming => theme.streaming,
        ThemeColorSlot::GutterBg => theme.gutter_bg,
        ThemeColorSlot::GutterContextIncluded => theme.gutter_context_included,
        ThemeColorSlot::UserBlockBg => theme.user_block_bg,
        ThemeColorSlot::ToolFg => theme.tool_fg,
        ThemeColorSlot::ToolSuccessBg => theme.tool_success_bg,
        ThemeColorSlot::ToolFailureBg => theme.tool_failure_bg,
        ThemeColorSlot::ToolPendingBg => theme.tool_pending_bg,
        ThemeColorSlot::ChallengeAlertBg => theme.challenge_alert_bg,
        ThemeColorSlot::ChallengeAlertFg => theme.challenge_alert_fg,
        ThemeColorSlot::CompactionBlockBg => theme.compaction_block_bg,
        ThemeColorSlot::SourcesHeaderBg => theme.sources_header_bg,
        ThemeColorSlot::SourcesHeaderFg => theme.sources_header_fg,
        ThemeColorSlot::TruncationFg => theme.truncation_fg,
        ThemeColorSlot::PickerActiveMarker => theme.picker_active_marker,
        ThemeColorSlot::PickerSelectedBg => theme.picker_selected_bg,
        ThemeColorSlot::PickerHighlightBg => theme.picker_highlight_bg,
        ThemeColorSlot::TabActiveFg => theme.tab_active_fg,
        ThemeColorSlot::TabActiveBg => theme.tab_active_bg,
        ThemeColorSlot::TabInactiveFg => theme.tab_inactive_fg,
        ThemeColorSlot::SelectionFg => theme.selection_fg,
        ThemeColorSlot::SelectionBg => theme.selection_bg,
        ThemeColorSlot::AccentAction => theme.accent_action,
        ThemeColorSlot::AgeFresh => theme.age_fresh,
        ThemeColorSlot::AgeStale => theme.age_stale,
        ThemeColorSlot::ScrollIndicatorBg => theme.scroll_indicator_bg,
        ThemeColorSlot::SidebarResizeAccent => theme.sidebar_resize_accent,
        ThemeColorSlot::InputModeQueue => theme.input_mode_queue,
        ThemeColorSlot::InputModeSteer => theme.input_mode_steer,
        ThemeColorSlot::InfopopupBg => theme.infopopup_bg,
        ThemeColorSlot::InfopopupTitle => theme.infopopup_title,
        ThemeColorSlot::InfopopupBorder => theme.infopopup_border,
        ThemeColorSlot::InfopopupFg => theme.infopopup_fg,
        ThemeColorSlot::QuakeBarBg => theme.quake_bar_bg,
    };
    ThemeColor::from(color).to_string()
}
