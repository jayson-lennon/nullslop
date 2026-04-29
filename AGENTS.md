# Style Guide

This document defines the coding conventions and architectural patterns for the `nullslop` codebase.

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

**Result type pattern:**

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
/// Returns an error if the database connection fails.
pub async fn new(db_path: &str) -> Result<Self, Report<SqliteNoteDbError>>
```

### Trait Usage

Every external dependency or service must have a trait abstraction.

**Backend trait pattern:**

```rust
use async_trait::async_trait;

#[async_trait]
pub trait PlaylistStorage: Send + Sync {
    fn name(&self) -> &'static str;
    async fn load(&self, dir: &CanonicalPath) -> Result<PlaylistData, Report<IoError>>;
}
```

**Service wrapper pattern:**

```rust
use std::sync::Arc;
use derive_more::Debug;

#[derive(Debug, Clone)]
pub struct MpvClientService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MpvClient>,
}

impl MpvClientService {
    pub fn new(backend: Arc<dyn MpvClient>) -> Self {
        Self { backend }
    }
}
```

**Key trait design rules:**

- All traits must be `Send + Sync` for thread safety
- Use `#[async_trait]` for async methods
- Include a `name(&self) -> &'static str` method for debugging
- Service structs wrap `Arc<dyn Trait>` for shared ownership

### Module Structure

**Workspace organization:**

```
Cargo.toml          # Workspace with members = ["crates/*", "tests/*"]
crates/
  null-prophet/        # Main crate
    src/
      lib.rs        # Module declarations and re-exports
      feat/         # Feature modules (domain logic)
      services.rs   # Service container
      system_ctx.rs # Application context
tests/
  acceptance/       # Cucumber acceptance tests
```

**Feature module pattern (`feat/`):**

```rust
// feat/playlist/mod.rs
pub mod storage;  // Submodule with implementations

pub use storage::{PlaylistStorage, PlaylistStorageService};

#[async_trait]
pub trait PlaylistStorage: Send + Sync { ... }

pub struct PlaylistStorageService { ... }
```

**Submodule with implementations:**

```
feat/playlist/storage/
├── mod.rs      # Re-exports
├── sqlite.rs   # Real implementation
└── fake.rs     # Test fake
```

### Dependency Injection

**Services container (shared with any parts of the application):**

```rust
#[derive(Debug, Clone)]
pub struct Services {
    pub chat: ChatService,
    pub rt: tokio::runtime::Handle,
    // more fields as needed.
}
```

**System context:**

```rust
#[derive(Debug, Clone)]
pub struct SystemCtx {
    pub services: Services,
    pub config: Config,
    pub keymap: Keymap,
}
```

## 3. Data Flow

Command → SystemCtx → Services → Backend Trait → Implementation

1. User action creates a `Command` enum variant
2. `execute(ctx, command)` dispatches to domain logic
3. Domain logic accesses services via `ctx.services`
4. Services delegate to trait backends (real or fake)

## 4. Tests

Important:

- Tests should only verify _observable behavior_
- Testing internal details is an _anti-pattern_.
- Prefer testing observable behavior ONLY. If observable behavior cannot be tested, then an abstraction needs to be created. Ask the user how to proceed in this case.

### BDD-Style Tests (Given/When/Then)

Structure tests with clear Given/When/Then sections:

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

**Example with service:**

```rust
fn service_delegates_to_backend() {
    // Given a service with a fake backend.
    let fake = Arc::new(FakeBackend::new());
    let service = MyService::new(fake.clone());

    // When calling the service method.
    let result = service.do_thing();

    // Then the backend was called and result is successful.
    assert!(result.is_ok());
    assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
}
```

### Parameterized Tests with rstest

```rust
#[rstest::rstest]
#[case(Key::Tab, "Tab")]
#[case(Key::Enter, "Enter")]
fn key_display(#[case] key: Key, #[case] expected: &str) {
    // Given / When / Then inline for simple cases
    assert_eq!(key.display(), expected);
}
```

### Async Tests

```rust
#[tokio::test]
async fn storage_loads_data() {
    // Given a storage service with fake backend.
    let storage = PlaylistStorageService::new(Arc::new(FakeStorageBackend::new()));

    // When loading data.
    let result = storage.load(&path).await;

    // Then the operation succeeds.
    assert!(result.is_ok());
}
```

### Test Utilities

**test_utils module structure:**

```rust
// test_utils/mod.rs
pub mod context;
pub mod fakes;
pub mod fixtures;
pub mod services;

pub use context::NoteTestContext;
pub use fakes::FakeMpvBackend;
pub use services::create_test_services;
```

**Test context pattern:**

```rust
pub struct NoteTestContext {
    pub ctx: SystemCtx,
    pub temp_file: NamedTempFile,
}

impl NoteTestContext {
    pub async fn new() -> Self {
        let services = create_test_services().await;
        let ctx = SystemCtx { services, ... };
        Self { ctx, temp_file }
    }
}
```

**Test services factory:**

```rust
pub async fn create_test_services() -> Services {
    let db = Arc::new(SqliteNoteDb::new("sqlite::memory:").await.unwrap());
    Services {
        mpv: MpvClientService::new(Arc::new(FakeMpvBackend)),
        media: MediaQueryService::new(Arc::new(FakeMediaBackend)),
        // ... all services with fakes
    }
}
```

### Fake Implementations

**Simple fake:**

```rust
pub struct FakeMpvBackend;

impl MpvClient for FakeMpvBackend {
    fn name(&self) -> &'static str { "fake" }
    fn load_file(&self, _path: &Path) -> Result<(), Report<MpvError>> {
        Ok(())
    }
}
```

**Stateful fake with call tracking:**

```rust
pub struct FakeStorageBackend {
    data: Arc<RwLock<StorageData>>,
    pub load_called: AtomicUsize,
}

impl FakeStorageBackend {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(StorageData::default())),
            load_called: AtomicUsize::new(0),
        }
    }
}

impl PlaylistStorage for FakeStorageBackend {
    async fn load(&self, _dir: &CanonicalPath) -> Result<PlaylistData, Report<IoError>> {
        self.load_called.fetch_add(1, Ordering::SeqCst);
        Ok(self.data.read().await.clone())
    }
}
```

## 5. Documentation

### Module-Level Documentation

```rust
//! Playlist storage and management.
//!
//! This module handles persisting and loading playlist data.
//!
//! # Notes vs Aliases
//!
//! - **Notes**: Searchable metadata attached to files.
//! - **Aliases**: Display names shown in the TUI.
```

### Type Documentation

```rust
/// A path to a media item, either local file or URL.
///
/// This enum distinguishes between local filesystem paths and web resources,
/// allowing uniform handling while maintaining type safety.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemPath {
    /// A local file path wrapped in [`CanonicalPath`].
    File(CanonicalPath),
    /// A URL string pointing to a web resource.
    Url(String),
}
```

### Service Documentation

```rust
/// Container for all injectable service dependencies.
///
/// Holds references to all services, enabling dependency injection
/// and making it easy to swap implementations for testing.
#[derive(Debug, Clone)]
pub struct Services { ... }
```

## 6. Modification Guide

When implementing features:

1. **Search for related patterns** - Find similar features in `feat/` directory
2. **Identify impacted types** - Check if new traits, services, or commands needed
3. **Create trait first** - Define the abstraction before implementation
4. **Implement real and fake** - Both must satisfy the trait
5. **Wire into Services** - Add to `Services` struct and `create_test_services()`
6. **Write tests** - Use Given/When/Then structure with test context and fakes
7. **Add documentation** - Module docs, type docs, error docs. Documentation should describe the behavior and purpose, not the technical implementation (unless it's relevant to understanding).

## 8. Tooling

Read the `justfile` to determine what additional tooling is related to this project. Prioritize running commands from the `justfile` instead of manual invocation. If there is a `just test` command, then use that instead of `cargo test`, etc.

## 9. Misc

- NEVER manually split a string using `.chars` or by indexing. Use the `unicode-segmentation` crate.
- No trivial setters for struct methods. Prefer meaningful semantic actions. It's an anti-pattern to directly inspect and manipulate state.
