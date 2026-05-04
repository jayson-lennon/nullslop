# Cucumber Test Expansion

## Problem

Four handler test files were converted to cucumber (`chat_input_box/handler.rs`, `provider/request_handler.rs`, `provider_picker/handler.rs`, `tui/app.rs` partially). Several other test files in the codebase contain integration/e2e-level tests with complex setup and behavioral scenarios that would benefit from the same treatment — particularly provider switching, model refresh, chat log handling, shutdown tracking, and headless script execution. Additionally, the original `tui/app.rs` tests are now redundant with the existing `tui_app.feature` and should be removed.

## Acceptance Criteria

- All original `tui/app.rs` tests that are covered by `tui_app.feature` are deleted (keeping `scope_for_mode_maps_correctly` as a trivial unit test).
- `provider/switch_handler.rs`, `provider/refresh_handler.rs`, `chat_log/handler.rs`, and `shutdown_tracker/handler.rs` tests are converted to cucumber features using the existing `BusWorld`.
- `headless.rs` integration tests (`run_script_*`) are converted to a cucumber feature using the existing `TuiWorld`.
- Original `#[cfg(test)]` blocks are removed from converted files.
- All cucumber tests pass (`cargo test -p nullslop-e2e`).
- Full test suite passes (`just test`).

## Implementation Phases

- [ ] Phase 1: Delete redundant tui/app.rs tests + convert provider/switch_handler.rs
  - Delete the 9 redundant test functions from `crates/nullslop-tui/src/app.rs` (keep `scope_for_mode_maps_correctly` as it's a trivial pure-function mapping not covered by cucumber)
  - Add BusWorld step definitions for `ProviderSwitch` command submission, `ProviderSwitched` event assertions, active provider assertions, and system message assertions
  - Create `tests/nullslop-e2e/tests/features/bus/provider_switch.feature` covering all 6 scenarios: valid switch, emits switched event, rejects unknown, rejects unavailable, handles remote model, rejects unknown remote
  - Delete `#[cfg(test)]` block from `crates/nullslop-component/src/provider/switch_handler.rs`
  - Verify: `cargo test -p nullslop-e2e --test bus_cucumber` and `just test`

- [ ] Phase 2: Convert provider/refresh_handler.rs and chat_log/handler.rs
  - Add BusWorld step definitions for `RefreshModels` command, `ModelsRefreshed` event, model cache assertions, and system message content assertions
  - Create `tests/nullslop-e2e/tests/features/bus/provider_refresh.feature` covering all 6 scenarios from refresh_handler.rs
  - Add BusWorld step definitions for `PushChatEntry` command, `ScrollUp`/`ScrollDown` commands, and chat history entry-kind assertions (Assistant, Actor, System)
  - Create `tests/nullslop-e2e/tests/features/bus/chat_log.feature` covering all 5 scenarios from chat_log/handler.rs
  - Delete `#[cfg(test)]` blocks from `crates/nullslop-component/src/provider/refresh_handler.rs` and `crates/nullslop-component/src/chat_log/handler.rs`
  - Verify: `cargo test -p nullslop-e2e --test bus_cucumber` and `just test`

- [ ] Phase 3: Convert shutdown_tracker/handler.rs and headless integration tests
  - Add BusWorld step definitions for `ActorStarting`/`ActorStarted`/`ActorShutdown` event submission and shutdown tracker completion assertions
  - Create `tests/nullslop-e2e/tests/features/bus/shutdown_tracker.feature` covering all 4 scenarios from shutdown_tracker/handler.rs
  - Add TuiWorld step definitions for running a keystroke script and asserting quit state
  - Create `tests/nullslop-e2e/tests/features/tui/headless_script.feature` covering the 2 integration scenarios from headless.rs (`run_script_sets_should_quit`, `run_script_is_noop_for_empty_content`)
  - Delete `#[cfg(test)]` integration test block from `src/headless.rs` (keep the 6 `parse_script` unit tests — they test a pure function, not behavioral)
  - Delete `#[cfg(test)]` block from `crates/nullslop-component/src/shutdown_tracker/handler.rs`
  - Verify: `cargo test -p nullslop-e2e` and `just test`
