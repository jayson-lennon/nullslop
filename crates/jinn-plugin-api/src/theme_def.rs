//! Theme contributions — the wire shape of one theme definition.
//!
//! [`ThemeDef`] mirrors the core `jinn-theme::Theme` slot set with every
//! color serialized as a string in any format the core parser accepts
//! (ANSI name like `"cyan"`, ANSI code like `"14"`, hex `"#112233"`, or RGB
//! string). The host translates each entry into the core `Theme` at cache
//! time; an unparseable slot drops that one theme, never the whole batch.

use serde::{Deserialize, Serialize};

/// Every color slot a contributed theme may define, keyed identically to the
/// core `jinn-theme::Theme` fields. Order is the field order of the core
/// struct; the wire uses string keys so slot addition is additive.
pub const THEME_COLOR_SLOTS: &[ThemeColorSlot] = &[
    ThemeColorSlot::FocusAccent,
    ThemeColorSlot::BorderUnfocused,
    ThemeColorSlot::PopupTitle,
    ThemeColorSlot::PrimaryText,
    ThemeColorSlot::MutedText,
    ThemeColorSlot::SubagentFg,
    ThemeColorSlot::ErrorText,
    ThemeColorSlot::Success,
    ThemeColorSlot::Warning,
    ThemeColorSlot::Streaming,
    ThemeColorSlot::GutterBg,
    ThemeColorSlot::GutterContextIncluded,
    ThemeColorSlot::UserBlockBg,
    ThemeColorSlot::ToolFg,
    ThemeColorSlot::ToolSuccessBg,
    ThemeColorSlot::ToolFailureBg,
    ThemeColorSlot::ToolPendingBg,
    ThemeColorSlot::ChallengeAlertBg,
    ThemeColorSlot::ChallengeAlertFg,
    ThemeColorSlot::CompactionBlockBg,
    ThemeColorSlot::SourcesHeaderBg,
    ThemeColorSlot::SourcesHeaderFg,
    ThemeColorSlot::TruncationFg,
    ThemeColorSlot::PickerActiveMarker,
    ThemeColorSlot::PickerSelectedBg,
    ThemeColorSlot::PickerHighlightBg,
    ThemeColorSlot::TabActiveFg,
    ThemeColorSlot::TabActiveBg,
    ThemeColorSlot::TabInactiveFg,
    ThemeColorSlot::SelectionFg,
    ThemeColorSlot::SelectionBg,
    ThemeColorSlot::AccentAction,
    ThemeColorSlot::AgeFresh,
    ThemeColorSlot::AgeStale,
    ThemeColorSlot::ScrollIndicatorBg,
    ThemeColorSlot::SidebarResizeAccent,
    ThemeColorSlot::InputModeQueue,
    ThemeColorSlot::InputModeSteer,
    ThemeColorSlot::InfopopupBg,
    ThemeColorSlot::InfopopupTitle,
    ThemeColorSlot::InfopopupBorder,
    ThemeColorSlot::InfopopupFg,
    ThemeColorSlot::QuakeBarBg,
];

/// One color slot key. Serialized as the snake_case slot name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeColorSlot {
    /// Focused/active accent (borders, selected gutter, active tabs).
    FocusAccent,
    /// Unfocused/inactive border.
    BorderUnfocused,
    /// Popup window title.
    PopupTitle,
    /// Primary content text.
    PrimaryText,
    /// Muted/dim text.
    MutedText,
    /// Subagent session title (sidebar).
    SubagentFg,
    /// Error text.
    ErrorText,
    /// Success/healthy status.
    Success,
    /// Warning/starting status.
    Warning,
    /// Streaming indicator.
    Streaming,
    /// Gutter column background.
    GutterBg,
    /// Gutter color for context-included entries.
    GutterContextIncluded,
    /// User message block background.
    UserBlockBg,
    /// Tool entry foreground.
    ToolFg,
    /// Tool success background.
    ToolSuccessBg,
    /// Tool failure background.
    ToolFailureBg,
    /// Tool pending background.
    ToolPendingBg,
    /// Challenge alert block background (bright).
    ChallengeAlertBg,
    /// Challenge alert block foreground (dark text on bright bg).
    ChallengeAlertFg,
    /// Compaction block background.
    CompactionBlockBg,
    /// Sources (annotation) header background (bright).
    SourcesHeaderBg,
    /// Sources (annotation) header foreground (dark text on bright bg).
    SourcesHeaderFg,
    /// Truncation indicator foreground.
    TruncationFg,
    /// Picker active marker.
    PickerActiveMarker,
    /// Picker selected background.
    PickerSelectedBg,
    /// Picker highlight background.
    PickerHighlightBg,
    /// Active tab foreground.
    TabActiveFg,
    /// Active tab background.
    TabActiveBg,
    /// Inactive tab foreground.
    TabInactiveFg,
    /// Selection foreground.
    SelectionFg,
    /// Selection background.
    SelectionBg,
    /// Action accent.
    AccentAction,
    /// Fresh-age marker.
    AgeFresh,
    /// Stale-age marker.
    AgeStale,
    /// Scroll indicator background.
    ScrollIndicatorBg,
    /// Sidebar resize accent.
    SidebarResizeAccent,
    /// Queue input-mode marker.
    InputModeQueue,
    /// Steer input-mode marker.
    InputModeSteer,
    /// Info popup background.
    InfopopupBg,
    /// Info popup title.
    InfopopupTitle,
    /// Info popup border.
    InfopopupBorder,
    /// Info popup foreground.
    InfopopupFg,
    /// Quake bar background.
    QuakeBarBg,
}

impl ThemeColorSlot {
    /// The snake_case wire key for this slot (identical to the core
    /// `Theme` field name and the theme TOML key).
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            Self::FocusAccent => "focus_accent",
            Self::BorderUnfocused => "border_unfocused",
            Self::PopupTitle => "popup_title",
            Self::PrimaryText => "primary_text",
            Self::MutedText => "muted_text",
            Self::SubagentFg => "subagent_fg",
            Self::ErrorText => "error_text",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Streaming => "streaming",
            Self::GutterBg => "gutter_bg",
            Self::GutterContextIncluded => "gutter_context_included",
            Self::UserBlockBg => "user_block_bg",
            Self::ToolFg => "tool_fg",
            Self::ToolSuccessBg => "tool_success_bg",
            Self::ToolFailureBg => "tool_failure_bg",
            Self::ToolPendingBg => "tool_pending_bg",
            Self::ChallengeAlertBg => "challenge_alert_bg",
            Self::ChallengeAlertFg => "challenge_alert_fg",
            Self::CompactionBlockBg => "compaction_block_bg",
            Self::SourcesHeaderBg => "sources_header_bg",
            Self::SourcesHeaderFg => "sources_header_fg",
            Self::TruncationFg => "truncation_fg",
            Self::PickerActiveMarker => "picker_active_marker",
            Self::PickerSelectedBg => "picker_selected_bg",
            Self::PickerHighlightBg => "picker_highlight_bg",
            Self::TabActiveFg => "tab_active_fg",
            Self::TabActiveBg => "tab_active_bg",
            Self::TabInactiveFg => "tab_inactive_fg",
            Self::SelectionFg => "selection_fg",
            Self::SelectionBg => "selection_bg",
            Self::AccentAction => "accent_action",
            Self::AgeFresh => "age_fresh",
            Self::AgeStale => "age_stale",
            Self::ScrollIndicatorBg => "scroll_indicator_bg",
            Self::SidebarResizeAccent => "sidebar_resize_accent",
            Self::InputModeQueue => "input_mode_queue",
            Self::InputModeSteer => "input_mode_steer",
            Self::InfopopupBg => "infopopup_bg",
            Self::InfopopupTitle => "infopopup_title",
            Self::InfopopupBorder => "infopopup_border",
            Self::InfopopupFg => "infopopup_fg",
            Self::QuakeBarBg => "quake_bar_bg",
        }
    }
}

/// One contributed theme.
///
/// `colors` maps slot keys to color strings; omitted slots fall back to the
/// core default theme's value at translation time. The name `"default"` is
/// reserved by the host — the built-in theme is always present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeDef {
    /// Unique theme name (the picker label and `state.toml` `theme_name`).
    pub name: String,
    /// Optional short description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Slot-key → color-string map (ANSI name, ANSI code, hex, or RGB).
    #[serde(default)]
    pub colors: std::collections::BTreeMap<String, String>,
}
