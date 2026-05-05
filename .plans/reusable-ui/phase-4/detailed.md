# Phase 4: Clean up provider picker domain code

## Context

Phase 3 completed the migration of the provider picker to use `SelectionState<PickerEntry>` and `SelectionWidget`. Most of the original Phase 4 cleanup work was absorbed into Phase 3:

- ✅ `filtered_entries()` renamed to `load_provider_entries()` (no filter param)
- ✅ `build_result_lines()` absorbed into `PickerEntry::render_row()`
- ✅ `compute_popup_rect()` moved to widget crate in Phase 2
- ✅ `ProviderPickerState` deleted
- ✅ `picker_entry_count()` deleted
- ✅ Old `render_provider_picker()` body replaced with `SelectionWidget` call

This phase handles the remaining provider-specific code that still lives in the generic rendering layer.

## Acceptance Criteria

- `format_footer()`, `age_color()`, and `truncate_line()` are moved out of `nullslop-tui/src/render.rs` into the provider picker rendering path, leaving no provider-specific logic in the generic renderer
- `jiff` and `humantime` dependencies are removed from `nullslop-tui` if they become unused after the move
- All existing tests continue to pass
- `cargo nextest run --workspace --all-features --exclude nullslop-e2e` passes
- No dead code warnings

## Current State

Three provider-specific functions sit in `nullslop-tui/src/render.rs`:

| Function | Lines | Purpose |
|----------|-------|---------|
| `format_footer()` | ~25 | Builds styled footer line with timestamp, age color, refresh hint |
| `age_color()` | ~8 | Maps elapsed seconds to green/yellow/red |
| `truncate_line()` | ~15 | Truncates a styled `Line<'static>` to fit width |

These are only called from `render_provider_picker()`. The `jiff` and `humantime` crates are used exclusively by these functions.

## Files to Modify

### 1. `crates/nullslop-tui/src/render.rs` (MODIFY)

Move `format_footer()`, `age_color()`, and `truncate_line()` out of this file. The `render_provider_picker()` function will call the moved versions.

**Option A — Inline module in render.rs**: Create a `mod provider_picker_footer` within `render.rs` (or a separate file) that houses these functions. This keeps the provider-specific rendering close to where it's used.

**Option B — Move to entries.rs**: Move into `nullslop-component/src/provider_picker/entries.rs` alongside the other provider-specific code. This is the most natural home since `entries.rs` already houses `PickerEntry` and `load_provider_entries()`.

**Recommended: Option B** — `entries.rs` is the provider picker's domain module. The footer formatting is provider-specific (it knows about refresh keybinds, model cache timestamps, age coloring). Moving it there keeps `render.rs` as a pure layout/dispatch layer with no domain knowledge.

After the move:
- `render_provider_picker()` calls `entries::format_footer()` instead of a local `format_footer()`
- `render.rs` no longer depends on `jiff` or `humantime` directly (used transitively through the function call)

### 2. `crates/nullslop-tui/Cargo.toml` (MODIFY)

Remove `jiff` and `humantime` from dependencies if they are no longer used directly in `nullslop-tui/src/render.rs` after the move. Check if any other file in `nullslop-tui` uses them first.

**Current status**: `jiff` and `humantime` are used only by `format_footer()` and `age_color()` in `render.rs`. After moving those functions, both dependencies can be removed from `nullslop-tui`.

### 3. `crates/nullslop-component/Cargo.toml` (MODIFY)

Add `jiff` and `humantime` as dependencies if they are not already present. Check first — `jiff` is already present for `AppState::last_refreshed_at`.

### 4. `crates/nullslop-component/src/provider_picker/entries.rs` (MODIFY)

Add the three moved functions: `format_footer()`, `age_color()`, `truncate_line()`. These become `pub(crate)` visibility so `render.rs` can call them.

Add unit tests for `format_footer()` and `age_color()` that were previously only tested implicitly through rendering tests.

### 5. `crates/nullslop-tui/src/render.rs` — render_provider_picker update (MODIFY)

Update `render_provider_picker()` to call `entries::format_footer()` instead of the local (now removed) version.

## Test Plan

### Tests to move

| Test | From | To |
|------|------|-----|
| Implicit footer tests in render tests | `nullslop-tui render.rs` | Stay in render.rs (they test the full rendering pipeline) |

### New tests

| Test | What it verifies |
|------|------------------|
| `format_footer_with_timestamp_shows_age` | Footer with a recent timestamp shows "Updated ... ago" with appropriate color |
| `format_footer_without_timestamp_shows_never` | Footer with `None` timestamp shows "Updated never" |
| `age_color_returns_correct_colors` | `age_color` returns LightGreen for ≤2w, Yellow for ≤4w, Red for >4w |
| `truncate_line_fits_within_width` | `truncate_line` truncates a long styled line to fit |
| `truncate_line_noop_when_fits` | `truncate_line` returns the line unchanged when it already fits |

## Verification

1. `cargo check --workspace` — no errors
2. `cargo nextest run --workspace --all-features --exclude nullslop-e2e` — all tests pass
3. `cargo clippy -p nullslop-component -p nullslop-tui --lib` — no new warnings

## What the codebase looks like after this phase

```
nullslop-tui/src/render.rs           — Pure layout/dispatch. No domain knowledge.
nullslop-component/src/provider_picker/
  entries.rs                         — PickerEntry, load_provider_entries(), sorted_entries(),
                                       format_footer(), age_color(), truncate_line()
  handler.rs                         — PickerHandler dispatch, load_provider_picker_items()
  mod.rs                             — Module registration
```

This is the final phase. After this, adding a new picker (e.g., actor picker) requires:
1. `PickerKind::Actor` variant in `nullslop-protocol`
2. `ActorEntry` struct + `PickerItem` impl in a new module
3. `actor_picker: SelectionState<ActorEntry>` field on `AppState`
4. Match arms in `PickerHandler` (7 methods)
5. Render branch in `render_picker()`
6. Footer formatting specific to actors (if needed)

**Zero new**: commands, Command enum variants, bus dispatch arms, Mode variants, Scope variants, keymap bindings.
