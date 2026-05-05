# Explore: Application-Level Mouse Text Selection

## Problem

nullslop uses ratatui + crossterm for its TUI. Terminal-native text selection doesn't work well with split panes — it selects entire terminal rows rather than constraining to a single pane. opencode solves this by implementing **application-level text selection** using OpenTUI's built-in hit grid and selection system. We need to replicate this approach in nullslop using ratatui primitives.

### How opencode/OpenTUI does it (research summary)

opencode uses OpenTUI (`@opentui/core`), a Zig + TypeScript TUI framework with built-in application-level text selection. The key mechanism:

1. **Hit Grid** — a screen-sized array where each cell stores the ID of the `Renderable` (UI component) that occupies it. Double-buffered to avoid race conditions with rendering.

2. **Mouse Capture** — OpenTUI enables the terminal's SGR mouse protocol (`useMouse: true`). Mouse events are parsed from stdin escape sequences into structured events with cell coordinates.

3. **Selection State Machine** — When a left-click drag is detected, the renderer creates a `Selection` object with an anchor (start position, relative to a renderable) and a focus (current drag position). The hit grid determines which renderables are within the selection bounds.

4. **Visual Highlight** — During rendering, each renderable checks if the global selection intersects its bounds. If so, it renders a highlight (inverted colors) on the selected cells.

5. **Text Extraction** — `getSelectedText()` asks each intersected renderable to extract its content, clipped to selection bounds.

6. **Copy on release** — On mouse-up, selected text is copied to the clipboard.

**The critical insight**: This is NOT terminal-native selection. The terminal's built-in selection is disabled because the app captures mouse events. Selection is implemented entirely in application space, with highlights drawn directly into the cell buffer. This allows constraining selection to individual panes because the app knows exactly which component each cell belongs to.

### How this maps to nullslop (ratatui + crossterm)

| OpenTUI Concept     | nullslop Equivalent                                      |
|----------------------|----------------------------------------------------------|
| Hit Grid             | A `SelectionRects` registry mapping `Rect`s per frame    |
| Mouse event parsing  | crossterm `Event::Mouse` (already enabled)               |
| Selection state      | New `SelectionState` enum on `TuiApp`                    |
| Visual highlight     | Post-process ratatui `Buffer`: swap fg/bg in selected cells |
| Text extraction      | Read characters from ratatui `Buffer` within bounds      |
| Copy to clipboard    | `arboard` crate (system clipboard)                       |
| Constrained to pane  | Click position matched against registered `Rect`s        |

### Existing codebase context

**Mouse handling already exists:**
- `crates/nullslop-tui/src/run.rs` — already calls `EnableMouseCapture` / `DisableMouseCapture` in setup/teardown
- `crates/nullslop-tui/src/keymap.rs` — already has `on_mouse()` handler that maps scroll events to `Command::MouseScrollUp` / `MouseScrollDown`
- `crates/nullslop-tui/src/app.rs` — `handle_msg()` already routes `Event::Mouse` through the keymap's mouse handler
- `crates/nullslop-protocol/src/system/command.rs` — `MouseScrollUp` / `MouseScrollDown` command structs exist

**Rendering flow:**
- `crates/nullslop-tui/src/render.rs` — `render(app, frame)` computes `AppLayout` (tab bar, content, indicator, queue, counter, input `Rect`s) then calls `element.render(frame, layout_rect, &state)` for each UI element
- `crates/nullslop-tui/src/run.rs` — `run_main_loop()` calls `terminal.draw(|frame| { app.render(frame); })` — the closure is where we can post-process the buffer after rendering
- ratatui's `Frame` gives us `buffer_mut()` which returns a `&mut Buffer` — we can iterate cells and swap fg/bg

**Config:**
- No formal config file exists yet. The `mouse_selection: bool` setting will need a config source. For now, use a simple TOML file at a well-known path (e.g., `~/.config/nullslop/config.toml`) or an environment variable. The subagent implementing Phase 5 should check if there's an existing config mechanism and extend it, or create a minimal one.

**Important architectural notes from AGENTS.md:**
- Colocate errors with their related types (no standalone `error.rs` files)
- Colocate traits with their related types (no standalone `traits.rs` files)
- Use `wherror::Error` with `error_stack::Report` for all fallible operations
- Use `where` clause for all generics
- Tests should only verify observable behavior
- Prefer BDD-style Given/When/Then test structure
- Use `unicode-segmentation` crate for any string splitting — never manually split using `.chars` or indexing

## Acceptance Criteria

- User can click-and-drag within a chat log pane to select text, constrained to that pane's `Rect`
- Selection is visually highlighted (inverted fg/bg in selected cells)
- On mouse release, selected text is copied to the system clipboard via arboard
- Selection can be constrained to any `Rect` defined by the application (works for popups, panels, etc.)
- Right-click or Escape dismisses the current selection
- Mouse capture is configurable (on/off) so users who prefer terminal-native selection can opt out
- No changes to existing keyboard-driven workflow

## Implementation Phases

- [x] **Phase 1: Selection state machine and core types** (`nullslop-tui`) — DONE
  - Created `crates/nullslop-tui/src/selection.rs` with:
    - `SelectionState` enum: `Idle`, `Dragging { anchor: (u16, u16), focus: (u16, u16), bounds: Rect }`, `Active { anchor: (u16, u16), focus: (u16, u16), bounds: Rect }`
      - **Design change from original plan:** `Dragging` includes a `focus` field (same shape as `Active`). This lets `selection_rect()` and `extract_text()` work uniformly across both states without external focus tracking.
      - `bounds` is the constraining `Rect` that selection is clipped to
      - `anchor` is where the drag started (absolute screen coords)
      - `focus` is the current drag position (absolute screen coords), clamped to `bounds`
    - Methods: `start_drag`, `update_focus` (clamps to bounds), `finalize`, `cancel`, `selection_rect` (normalized, intersected with bounds), `is_active`, `extract_text` (reads buffer row-by-row, trims trailing whitespace via `unicode-segmentation`)
    - `Default` impl returns `Idle`
  - Added `mod selection;` to `crates/nullslop-tui/src/lib.rs`
  - Added `pub(crate) selection: SelectionState` field to `TuiApp` (in `app.rs`), initialized as `SelectionState::Idle` in both constructors
  - Error types: `SelectionError` struct using `wherror::Error` in `selection.rs`
  - **Tests (9 tests, all passing):**
    - `start_drag_creates_dragging_state`
    - `update_focus_clamps_to_bounds`
    - `finalize_transitions_dragging_to_active`
    - `cancel_returns_to_idle`
    - `idle_returns_none_for_selection_rect`
    - `idle_returns_none_for_extract_text`
    - `extract_text_reads_single_row`
    - `extract_text_reads_multiple_rows`
    - `selection_rect_anchor_can_be_after_focus`
  - **Dependencies:** no new crate deps (ratatui `Buffer` used directly, `unicode-segmentation` already in workspace)

- [x] **Phase 2: Rect registration and mouse event routing** (`nullslop-tui`) — DONE
  - **`SelectableRects` struct** in `selection.rs` — `Vec<Rect>` wrapper with `new()`, `rebuild(Vec<Rect>)`, and `find_for_position(x, y) -> Option<Rect>` (returns smallest-area rect containing the point). Colocated with `SelectionState` per project conventions.
  - **`selectable_rects: SelectableRects` field** on `TuiApp` in `app.rs`, initialized as `SelectableRects::default()` in both constructors.
  - **Populated rects in `render.rs`** — at end of `render()`, pushes `layout.content` and picker popup rect (when `Mode::Picker` is active). In `render_too_small()`, clears rects with `rebuild(vec![])`.
  - **`handle_selection_mouse(&mut self, mouse: MouseEvent) -> bool`** method on `TuiApp` in `app.rs`:
    - `Down(Left)`: find matching rect via `selectable_rects.find_for_position()`, if found call `selection.start_drag(x, y, bounds)` and return `true`
    - `Drag(Left)`: if `selection.is_active()`, call `update_focus` and return `true`
    - `Up(Left)`: if `selection.is_active()`, call `finalize()` and return `true`
    - `Down(Right)`: if `selection.is_active()`, call `cancel()` and return `true`
    - All others: return `false` (fall through to keymap for scroll events)
  - **Intercepted mouse events** in `handle_msg()` — calls `handle_selection_mouse()` before the keymap's `mouse_handler()`. If it returns `true`, skips keymap processing.
  - **Extracted `SetMode` from `_` wildcard in `route_command()`** — cancels selection before forwarding `SetMode` to the core bus (prevents stale selection when picker closes).
  - **Implementation detail:** Used `std::mem::take` for state transitions since `SelectionState` methods take `self` by value. `SelectionState: Default` makes this work cleanly.
  - **No changes to `keymap.rs`** — selection events intercepted before reaching the keymap; existing scroll handling untouched.
  - **Tests (13 new = 4 SelectableRects + 6 integration + 3 existing, all passing):**
    - `selectable_rects_find_returns_smallest_matching` — overlapping rects, smallest returned
    - `selectable_rects_find_returns_none_for_position_outside_all`
    - `selectable_rects_find_returns_none_when_empty`
    - `selectable_rects_rebuild_replaces_previous_rects`
    - `mouse_down_left_in_selectable_rect_starts_dragging`
    - `mouse_down_left_outside_selectable_rect_does_not_start_dragging`
    - `mouse_drag_updates_focus_while_dragging`
    - `mouse_up_left_finalizes_selection`
    - `mouse_down_right_cancels_selection`
    - `scroll_events_still_route_to_keymap`
  - **Dependencies:** no new crate deps

- [ ] **Phase 3: Selection rendering (visual highlight)** (`nullslop-tui`) — DONE
  - **`apply_selection_highlight(app: &TuiApp, buf: &mut Buffer)`** standalone function in `render.rs` — iterates cells within `app.selection.selection_rect()`, swaps `fg`/`bg` on each. No-op when selection is `Idle`.
  - **Called at end of `render()`** — after `app.selectable_rects.rebuild(rects)`, calls `apply_selection_highlight(app, frame.buffer_mut())`. NOT called in `render_too_small()` path.
  - **Design decision:** Implemented as a standalone function inside `render()` rather than in `run.rs` — the highlight is a rendering concern and `frame.buffer_mut()` is available within `render()`. Keeps `run.rs` unchanged.
  - **Import:** `SelectionState` imported in the test module only (not needed in production code since `app.selection` is accessed via `TuiApp`).
  - **Tests (3 new, all passing):**
    - `selection_highlight_inverts_cells_within_selection` — cells inside selection have swapped fg/bg, outside cells unchanged
    - `selection_highlight_respects_constraining_bounds` — anchor outside bounds, only clamped cells inverted
    - `selection_highlight_does_nothing_when_idle` — Idle selection, no cells changed
  - **Total test count:** 53 tests passing (50 existing + 3 new)
  - **Dependencies:** no new crate deps
  - **No changes** to `run.rs`, `app.rs`, `selection.rs`, or `keymap.rs`

- [x] **Phase 4: Clipboard via arboard** (`nullslop-tui`) — DONE
  - **`arboard = "3"`** added to workspace dependencies and `crates/nullslop-tui/Cargo.toml`.
  - **`pending_clipboard: bool` field** (`pub(crate)`) on `TuiApp` — set `true` on mouse-up finalize, initialized `false` in both constructors.
  - **`flush_pending_clipboard(app: &mut TuiApp, buf: &Buffer)`** standalone function in `render.rs` — called after `apply_selection_highlight` at end of `render()`. Extracts text via `SelectionState::extract_text()`, copies to system clipboard via `arboard`, clears flag. Empty text is skipped. Errors logged via `tracing::warn`, never panic.
  - **`drop(state)` added** in `render()` before `selectable_rects.rebuild()` — the state read guard was borrowed from `app.core.state.read()` and lived until end of function, blocking the `&mut app` needed by `flush_pending_clipboard`. Explicit drop releases the lock after last use.
  - **Design decision:** Deferred clipboard approach (flag set on mouse-up, flushed on next render) because ratatui buffer is only available during `terminal.draw()`. One-frame delay is imperceptible.
  - **Tests (3 new, all passing):**
    - `clipboard_copy_clears_pending_flag_on_idle_selection` — flag cleared even when Idle
    - `clipboard_copy_skips_empty_selection` — flag cleared for whitespace-only selection
    - `clipboard_copy_extracts_selected_text` — `#[ignore]`-gated full integration test (requires clipboard access)
  - **Total test count:** 56 tests (55 passing + 1 ignored)
  - **Dependencies:** added `arboard = "3"` (brings platform clipboard libs: x11rb on Linux, objc2 on macOS, windows-sys on Windows)
  - **No changes** to `run.rs`, `selection.rs`, or `keymap.rs`

- [x] **Phase 5: Configuration for mouse capture** (`nullslop-tui`) — DONE
  - **`TuiConfig` struct** in `crates/nullslop-tui/src/config.rs` — plain data struct with `mouse_selection: bool` field, `new()` constructor, and `Default` impl (returns `mouse_selection: true`). NO env var access in library crates.
  - **Module registration** — `pub mod config;` added to `crates/nullslop-tui/src/lib.rs`.
  - **`TuiApp` updates** in `app.rs` — added `config: TuiConfig` field (pub(crate)). Added `new_with_config()` and `new_with_core_and_config()` constructors. Existing `new()` and `new_with_core()` delegate to these with `TuiConfig::default()` (backward compatible).
  - **Selection gating** — `handle_selection_mouse()` in `handle_msg()` gated on `self.config.mouse_selection`. When disabled, mouse events fall through to keymap (no selection handling).
  - **Conditional mouse capture** in `run.rs` — `EnableMouseCapture`/`DisableMouseCapture` only executed when `mouse_selection` is true. Bool copied out of `app` before mutable borrows to avoid borrow checker issues.
  - **Entry point** in `src/app.rs` — reads `NULLSLOP_MOUSE_SELECTION` env var at startup (alongside `ApiKeys`). Values `false` or `0` disable; missing or any other value enables. Constructs `TuiConfig::new(mouse_selection)` and passes to `TuiApp::new_with_core_and_config()`.
  - **Tests (4 new BDD-style, all passing):**
    - `default_config_has_mouse_selection_enabled` — `TuiConfig::default().mouse_selection` is `true`
    - `new_config_with_false_disables_mouse_selection` — `TuiConfig::new(false).mouse_selection` is `false`
    - `new_config_with_true_enables_mouse_selection` — `TuiConfig::new(true).mouse_selection` is `true`
    - `mouse_events_not_handled_when_mouse_selection_disabled` — app with disabled config, left-click inside selectable rect, selection remains Idle
  - **Total test count:** 60 tests (59 passing + 1 ignored clipboard integration test)
  - **Convention followed:** Env vars read ONLY in `src/app.rs` at startup, stored in structs, passed as constructor args (same pattern as `ApiKeys`). Library crates never access env vars.

- [x] **Phase 6: Register selectable Rects from elements** (`nullslop-component-ui` + `nullslop-tui`) — DONE
  - **`is_selectable(&self) -> bool` default method** added to `UiElement` trait in `crates/nullslop-component-ui/src/element.rs` — returns `false` by default, object-safe (works with `Box<dyn UiElement<S>>`).
  - **`ChatLogElement`** in `crates/nullslop-component/src/chat_log/element.rs` — overrides `is_selectable()` → `true`.
  - **`DashboardElement`** in `crates/nullslop-component/src/dashboard/element.rs` — overrides `is_selectable()` → `true`.
  - **`render.rs` refactored** — `rects` vec declared before element rendering. After each `element.render()` call, checks `element.is_selectable()` and pushes the passed rect. Popup remains a special case (not a UiElement).
  - **Tests (5 new BDD-style, all passing):**
    - `element::tests::default_is_selectable_returns_false` — `FakeUiElement` trait object returns `false`
    - `chat_log::element::tests::chat_log_element_is_selectable` — returns `true`
    - `dashboard::element::tests::dashboard_element_is_selectable` — returns `true`
    - `render::tests::render_registers_content_rect_for_selectable_chat_log` — after rendering Chat tab, `selectable_rects` contains content area rect
    - `render::tests::render_registers_picker_popup_rect_when_active` — after rendering Picker mode, `selectable_rects` contains popup rect and content rect
  - **Total test count:** 65 tests (64 passing + 1 ignored clipboard integration test)
  - **No changes** to `selection.rs`, `app.rs`, `keymap.rs`, `run.rs`, `config.rs`, or `src/app.rs`
