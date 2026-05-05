# Reusable Selection Widget

## Problem

The provider picker implements a "search + filter + select" UI pattern (filter text input, fuzzy matching, scrollable result list, selection highlight) that will be reused frequently across the application. Currently all of this — state, commands, handler, rendering, keymap wiring — is hardcoded to providers in `nullslop-component/src/provider_picker/`. We need to extract it into a reusable crate so future pickers (actors, tools, sessions, etc.) can be built by just defining an item type and a confirm action.

## Key Decisions

- **New crate** `nullslop-selection-widget` in `crates/`
- **Generic state**: `SelectionState<T: PickerItem>` — the generic makes it obvious what the widget holds. If a dynamic/text-only picker is needed, implement `PickerItem` on `String`
- **Consumer pre-sorts** the item list; the widget preserves that order and applies fuzzy filtering on top
- **Fuzzy matching** via the `fuzzy-matcher` crate, matching against `PickerItem::display_label()`
- **Styled rows**: `PickerItem` returns `ratatui::text::Line<'static>` so consumers can style entries (greyed out, active markers, icons, etc.)
- **Traditional ratatui widget pattern**: state and widget are separate structs
- **Single shared picker infrastructure**: the 7 `Picker*` commands, `Mode::Picker`, `Scope::Picker`, and keymap bindings are shared across ALL pickers — adding a new picker requires zero new commands, bus changes, or keymap changes. A `PickerKind` enum on `AppState` determines which `SelectionState<T>` is active.

## Current Architecture (what we're extracting from)

The provider picker currently spans multiple crates:

| Layer | File(s) | What it does |
|-------|---------|--------------|
| **State** | `nullslop-component/src/provider_picker/state.rs` | `ProviderPickerState` — filter text, cursor, selection index, scroll offset |
| **Protocol** | `nullslop-protocol/src/provider_picker/command.rs` | 7 command structs: `PickerInsertChar`, `PickerBackspace`, `PickerConfirm`, `PickerMoveUp`, `PickerMoveDown`, `PickerMoveCursorLeft`, `PickerMoveCursorRight` |
| **Protocol** | `nullslop-protocol/src/command.rs` | `Command` enum with 7 `Picker*` variants |
| **Protocol** | `nullslop-protocol/src/mode.rs` | `Mode::Picker` variant |
| **Handler** | `nullslop-component/src/provider_picker/handler.rs` | `PickerHandler` via `define_handler!` — routes 7 commands to state methods. `on_confirm` is provider-specific |
| **Entries** | `nullslop-component/src/provider_picker/entries.rs` | `PickerEntry` struct, `filtered_entries()`, `sorted_entries()` — provider-specific item construction and sorting |
| **Rendering** | `nullslop-tui/src/render.rs` | `render_provider_picker()` — popup with filter, separator, results, footer |
| **Keymap** | `nullslop-tui/src/keymap.rs` | `Scope::Picker` with standard picker bindings |
| **Scope** | `nullslop-tui/src/scope.rs` | `Scope::Picker` variant |
| **App state** | `nullslop-component/src/app_state.rs` | `AppState.picker: ProviderPickerState` |
| **Mode switch** | `nullslop-component/src/chat_input_box/handler.rs` | `SetMode` handler resets picker when entering `Mode::Picker` |

## Acceptance Criteria

- A new `nullslop-selection-widget` crate provides a self-contained, reusable selection widget
- `PickerItem` trait supports styled rendering (`Line<'static>`) and fuzzy matching
- `SelectionState<T: PickerItem>` owns the item list, caches filtered results, handles all navigation/filter input
- Filtering uses `fuzzy-matcher` on the display label, preserves consumer's pre-sorted order
- A ratatui-style `SelectionWidget` renders the popup (filter input, separator, scrollable results, optional footer)
- A single shared set of picker commands handles ALL picker types via `PickerKind` dispatch
- The existing provider picker is fully migrated to use the new widget with no behavioral regressions
- All existing tests continue to pass
- Adding a new picker requires: one `PickerKind` variant, one `SelectionState<T>` field, one match arm in the handler, one render branch, and one confirm method — zero new commands, bus changes, or keymap changes

## Implementation Phases

- [x] **Phase 1: Create `nullslop-selection-widget` crate with core types**

  Create `crates/nullslop-selection-widget/` with `Cargo.toml` (depends on `ratatui`, `unicode-segmentation`, `fuzzy-matcher`). Add to workspace in root `Cargo.toml`.

  **`PickerItem` trait**, **`SelectionState<T: PickerItem>`**, and all associated tests.

  **Phase 1 divergence note:** `filtered` field uses `Vec<usize>` (indices) instead of `Vec<T>` to avoid requiring `Clone`. Added `with_items()`, `selection()`, `items()`, `filtered_count()` accessors.

- [x] **Phase 2: Add `SelectionWidget` ratatui renderer**

  `SelectionWidget<'a, T: PickerItem>` with builder pattern, `compute_popup_rect()`, and rendering logic. 12 widget tests (4 ported + 8 new).

  **Phase 2 divergence note:** `title` is `Line<'a>` not `String`. `render(self, frame, area)` consumes self (builder pattern).

- [x] **Phase 3: Unify picker infrastructure with `PickerKind`**

  Introduce `PickerKind` enum and refactor the handler/rendering to dispatch based on which picker is active. Migrate the provider picker to use `SelectionState<PickerEntry>`. The result: one shared set of 7 commands, one `Mode::Picker`, one `Scope::Picker`, one keymap scope — adding future pickers requires zero protocol/bus/keymap changes.

  **Changes:**
  - Add `PickerKind` enum to `nullslop-protocol` (starting with `Provider` variant)
  - Add `active_picker_kind: Option<PickerKind>` to `AppState`
  - Replace `ProviderPickerState` with `SelectionState<PickerEntry>` in `AppState`
  - Implement `PickerItem` for `PickerEntry` (display_label + render_row with current styling)
  - Add `is_active` field to `PickerEntry`, set during `sorted_entries()`
  - Rename `filtered_entries()` to `load_provider_entries()` (returns all entries, no filter param)
  - Refactor `PickerHandler`: 6 standard methods dispatch on `active_picker_kind`, `on_confirm` delegates to kind-specific confirm
  - Refactor rendering: `render_provider_picker()` dispatches via `render_picker()` on kind, uses `SelectionWidget`
  - Update `SetMode` handler to set `active_picker_kind`, load items via `set_items()`, and reset picker
  - Remove `ProviderPickerState` (absorbed by `SelectionState`)
  - Remove `picker_entry_count()` (filtering is internal to `SelectionState`)
  - Remove `build_result_lines()` and `compute_popup_rect()` from `render.rs` (moved to widget crate)
  - Add `set_selection()` method to `SelectionState` for e2e test use
  - Port all existing tests

  **Divergence note:** `filtered_entries()` was renamed to `load_provider_entries()` and no longer accepts a filter parameter — all entries are loaded unconditionally, with fuzzy filtering handled by `SelectionState`. This is a semantic change from the plan's `filtered_entries("", ...)` approach. Added `is_active` field to `PickerEntry` that the plan didn't explicitly mention as a struct field but implied through `render_row`. `load_provider_picker_items` was made `pub` (not `pub(crate)`) so e2e tests can access it.

  **What adding a new picker looks like after this phase:**
  1. Add `PickerKind::MyPicker` variant
  2. Add `my_picker: SelectionState<MyEntry>` field to `AppState`
  3. Add match arms in handler (insert_char, backspace, move_up/down, cursor, confirm)
  4. Add render branch in `render_picker()`
  5. Implement `PickerItem` for `MyEntry`

  **Zero new**: commands, Command enum variants, bus dispatch arms, Mode variants, Scope variants, keymap bindings.

- [x] **Phase 4: Clean up provider picker domain code**

  With the migration complete, clean up the provider-specific code:
  - ~~Move `filtered_entries()` and `sorted_entries()` into a `load_provider_entries()` function that returns `Vec<PickerEntry>` ready for `set_items()`~~ Already done in Phase 3
  - Move `format_footer()`, `age_color()`, and `truncate_line()` from `nullslop-tui/src/render.rs` to `nullslop-component/src/provider_picker/entries.rs`
  - ~~Move `truncate_line()` utility if still needed~~ Moved to entries.rs alongside the other functions
  - ~~Remove dead code: old `render_provider_picker()`, `build_result_lines()`, `compute_popup_rect()` from `render.rs`~~ Already done in Phase 3
  - Add `humantime` dependency to `nullslop-component/Cargo.toml`
  - Remove `jiff` and `humantime` from `nullslop-tui/Cargo.toml` (no longer used after move)
  - Remove unused `Line`, `Span` imports from `render.rs`
  - Add 11 unit tests for moved functions (BDD Given/When/Then style)
  - Verify all tests pass with no regressions
  - Update high-level plan with final divergence notes

  **Divergence note:** Functions were made `pub` instead of `pub(crate)` because `nullslop-tui` is a separate crate from `nullslop-component` and needs to call `entries::format_footer()`. The plan suggested `pub(crate)` visibility which doesn't work across crates. Removed `Line` and `Span` imports from render.rs as they became unused after the move.

## Old Plan (superseded)

~- [ ] **Phase 3: Add command/handler wiring helper** (superseded by new Phase 3 above)~

~- [ ] **Phase 4: Migrate provider picker to use `nullslop-selection-widget`** (absorbed into new Phase 3)~
