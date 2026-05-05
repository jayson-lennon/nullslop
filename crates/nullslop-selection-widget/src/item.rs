//! Picker item trait — the consumer-facing contract.
//!
//! Consumers implement [`PickerItem`] for their domain type (e.g., `ProviderEntry`,
//! `ActorEntry`). The widget uses [`PickerItem::display_label`] for fuzzy matching and
//! [`PickerItem::render_row`] for styled display.

use ratatui::text::Line;

/// An item that can be displayed and selected in a picker.
///
/// The widget uses [`display_label`](PickerItem::display_label) for fuzzy matching and
/// [`render_row`](PickerItem::render_row) for styled display in the picker list.
///
/// # Examples
///
/// ```ignore
/// struct MyItem { name: String }
///
/// impl PickerItem for MyItem {
///     fn display_label(&self) -> &str { &self.name }
///     fn render_row(&self, is_selected: bool) -> Line<'static> {
///         if is_selected {
///             Line::from(format!("> {}", self.name))
///         } else {
///             Line::from(self.name.clone())
///         }
///     }
/// }
/// ```
pub trait PickerItem: std::fmt::Debug + 'static {
    /// Returns searchable text used for fuzzy matching.
    ///
    /// Should contain all text the user might search by (name, model, backend, etc.).
    fn display_label(&self) -> &str;

    /// Renders this item as a styled line for display in the picker.
    ///
    /// `is_selected` indicates whether this row is currently highlighted.
    /// The consumer controls all styling — colors, markers, icons, dimming, etc.
    fn render_row(&self, is_selected: bool) -> Line<'static>;
}
