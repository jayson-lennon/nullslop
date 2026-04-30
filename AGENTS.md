# Style Guide

This document defines the coding conventions and patterns for the `nullslop` codebase. For system design and data flow, see [ARCHITECTURE.md](./ARCHITECTURE.md).

## 1. Overview

This style guide ensures consistent, maintainable Rust code across the codebase. It covers error handling, trait-based design, testing patterns, documentation standards, and module organization. Following these patterns enables dependency injection for testability and clear separation of concerns.

## 2. Core Patterns

### Error Handling

Use `wherror::Error` with `error_stack::Report` for all fallible operations.

**Error type:**

```rust
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct ExternalEditorError;
```

**Result with error context:**

```rust
use error_stack::{Report, ResultExt};

pub fn load() -> Result<Config, Report<ConfigError>> {
    let content = std::fs::read_to_string(&path)
        .change_context(ConfigError)
        .attach("failed to read config file")?;
    Ok(config)
}
```

**Document errors in functions:**

```rust
/// # Errors
///
/// Returns an error if the terminal setup fails.
pub fn run(tick_rate: Duration) -> Result<(), Report<TuiRunError>>
```

### Trait Usage

Every external dependency or service must have a trait abstraction.

**Trait pattern:**

```rust
pub trait CommandHandler<C: 'static, S> {
    fn handle(&self, cmd: &C, state: &mut S, out: &mut Out) -> CommandAction;
}

use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct FooBackendError;

pub trait FooBackend {
    fn fetch_all(&self) -> Result<Vec<Foo>, Report<FooBackendError>>;
}
```

**Service wrapper pattern:**

```rust
use std::sync::Arc;
use derive_more::Debug;

#[derive(Debug, Clone)]
pub struct ExtensionHostService {
    #[debug("ExtensionHost<{}>", self.backend.name())]
    host: Arc<dyn ExtensionHost>,
}

impl ExtensionHostService {
    pub fn new(host: Arc<dyn ExtensionHost>) -> Self {
        Self { host }
    }
}
```

**Key trait design rules:**

- Use `#[async_trait]` for async methods
- Include a `name(&self) -> &'static str` method for debugging on service traits
- Service structs wrap `Arc<dyn Trait>` for shared ownership

### Module Structure

**Workspace organization:**

```
Cargo.toml          # Workspace with members = ["crates/*", "extensions/*"]
crates/
  nullslop/            # Main binary crate
    src/
      lib.rs
      main.rs
      app.rs
  nullslop-protocol/   # Command, Event, Mode, Key — wire types
  nullslop-component-core/  # Bus, handler traits, define_handler! macro
  nullslop-component-ui/    # UiElement trait, UiRegistry
  nullslop-component/       # Built-in components (chat input, chat log, quit, etc.)
  nullslop-core/       # State wrapper, AppCore loop, extension registry
  nullslop-services/   # Services container (runtime dependencies)
  nullslop-tui/        # Terminal, renderer, keymap, event loop
  nullslop-ext-host/   # Extension host implementations
  nullslop-extension/  # Extension author SDK
  nullslop-cli/        # CLI argument parsing
extensions/
  nullslop-echo/       # Example echo extension
```

**Component module pattern (under `nullslop-component/src/`):**

```
chat_input_box/
├── mod.rs      # register(bus, registry) wiring
├── handler.rs  # Bus handler via define_handler! macro
├── element.rs  # UiElement<AppState> rendering
└── state.rs    # Component-specific state (e.g., ChatInputBoxState)
```

Not every component needs all four files. A display-only component (like chat log) may only have `mod.rs` and `element.rs`.

### Dependency Injection

**Services container (in `nullslop-services`):**

```rust
#[derive(Debug, Clone)]
pub struct Services {
    handle: Handle,
    ext_host: ExtensionHostService,
}
```

Created once at startup and shared throughout the application.

All services within the `Services` struct must either:

- Be cheap to clone
- Use the "service wrapper" pattern detailed above.

## 3. Data Flow

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full data flow diagram and bus dispatch details.

## 4. Tests

Important:

- Tests should only verify _observable behavior_
- Testing internal details is an _anti-pattern_.
- Prefer testing observable behavior ONLY. If observable behavior cannot be tested, then an abstraction needs to be created. Ask the user how to proceed in this case.

### BDD-Style Tests (Given/When/Then)

Structure tests with clear Given/When/Then sections, and name the test so it can be read as a standalone program behavior in the test report:

```rust
fn pop_returns_none_when_stack_empty() {
    // Given an empty stack.
    let mut stack = Stack::default();

    // When popping from the stack.
    let item = stack.pop();

    // Then we get nothing back.
    assert!(item.is_none());
}
```

**Example with bus and state:**

```rust
#[test]
fn quit_command_sets_should_quit_in_state() {
    // Given a bus with AppQuitHandler registered.
    let mut bus: Bus<AppState> = Bus::new();
    AppQuitHandler.register(&mut bus);

    // When processing the AppQuit command.
    bus.submit_command(Command::AppQuit);

    let mut state = AppState::new();
    bus.process_commands(&mut state);

    // Then should_quit should be set to true.
    assert!(state.should_quit);
}
```

### Parameterized Tests with rstest

If a test has many inputs, the prefer parametrizing with `rstest`:

```rust
#[rstest::rstest]
#[case(Key::Tab, "Tab")]
#[case(Key::Enter, "Enter")]
fn key_display(#[case] key: Key, #[case] expected: &str) {
    // Given / When / Then inline for simple cases
    assert_eq!(key.display(), expected);
}
```

For edge cases that don't easily fit into "expected", prefer a BDD-styled test instead.

### Async Tests

```rust
#[tokio::test]
async fn extension_host_loads_manifest() {
    // Given an in-memory extension host.
    let host = InMemoryExtensionHost::new();

    // When loading extensions.
    let result = host.discover().await;

    // Then discovery succeeds.
    assert!(result.is_ok());
}
```

### Test Utilities

**`test_utils` module structure:**

```rust
// test_utils/mod.rs
pub mod context;
pub mod fakes;
pub mod fixtures;
pub mod services;
```

**Testing components via the bus:**

```rust
#[test]
fn insert_char_appends_to_buffer() {
    // Given a bus with ChatInputBoxHandler registered.
    let mut bus: Bus<AppState> = Bus::new();
    ChatInputBoxHandler.register(&mut bus);

    // When processing the ChatBoxInsertChar('x') command.
    bus.submit_command(Command::ChatBoxInsertChar {
        payload: ChatBoxInsertChar { ch: 'x' },
    });
    let mut state = AppState::new();
    bus.process_commands(&mut state);

    // Then "x" is appended to the chat_input.input_buffer.
    assert_eq!(state.chat_input.input_buffer, "x");
}
```

### Fake Implementations

**Simple fake (from `nullslop-component-core`):**

```rust
pub struct FakeCommandHandler<C, S> { /* ... */ }

// Used in bus dispatch tests to verify handler registration
// without real logic:
let (fake, fake_calls) = FakeCommandHandler::<AppQuit, AppState>::continuing();
bus.register_command_handler::<AppQuit, _>(fake);
```

## 5. Documentation

### Module-Level Documentation

Module level documentation should explain what it's purpose and high-level behaviors. Only explain technical details as necessary to make the high-level documentation understandable.

```rust
//! Chat input box — where the user composes and sends messages.
//!
//! This component manages the text input experience end to end: handling keystrokes,
//! displaying the in-progress message, and switching between browsing and typing modes.
```

### Type Documentation

```rust
/// The user's in-progress message being composed in the input box.
#[derive(Debug)]
pub struct ChatInputBoxState {
    /// The text the user has typed so far.
    input_buffer: String,
}
```

## 6. Modification Guide

When implementing features:

1. **Search for related patterns** — Find similar components in `nullslop-component/src/`
2. **Identify impacted types** — Check if new commands, events, or state fields are needed
3. **Add protocol types first** — Define new `Command`/`Event` variants in `nullslop-protocol`
4. **Create the component directory** — Add `handler.rs`, `element.rs`, `state.rs` as needed
5. **Register** — Wire into `register_all()` in `nullslop-component/src/lib.rs`
6. **Write tests** — Use Given/When/Then structure, test via the bus
7. **Add documentation** — Module docs, type docs, error docs. Describe behavior and purpose, not technical implementation.

## 8. Tooling

Read the `justfile` to determine what additional tooling is related to this project. Prioritize running commands from the `justfile` instead of manual invocation. If there is a `just test` command, then use that instead of `cargo test`, etc.

## 9. Misc

- NEVER manually split a string using `.chars` or by indexing. Use the `unicode-segmentation` crate.
- No trivial setters for struct methods. Prefer meaningful semantic actions. It's an anti-pattern to directly inspect and manipulate state.
