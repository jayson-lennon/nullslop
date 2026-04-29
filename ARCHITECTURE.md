# Architecture

nullslop is a TUI chat application built on a component system. Everything the app does — handling input, switching modes, quitting, bridging extensions — is a component. Components are stateless adapters that route typed commands to domain objects. All state lives in one place.

## Top-Level Summary

**Three layers, one data store:**

1. **Domain objects** (`InputBox`, `Popup`, etc.) — hold behavior and enforce invariants. They don't know about components or the bus.
2. **Components** — thin routing that maps commands to domain object method calls. Minimal logic of their own.
3. **State** (`AppData` / `TuiState`) — the single source of truth. Holds domain objects. Everyone reads and writes here.

The renderer queries state. It doesn't know or care which component wrote what.

## Detailed Architecture

### Data Flow

```
Key press → Command → Bus → Component → Domain object method → State mutated → Renderer reads state
```

Each step has a single responsibility:

| Step       | Responsibility                                     | Example                                            |
| ---------- | -------------------------------------------------- | -------------------------------------------------- |
| Key press  | Raw terminal event                                 | User presses `x`                                   |
| Command    | Typed, serializable intent                         | `ChatBoxInsertChar { ch: 'x' }`                    |
| Bus        | Dispatches commands by type to registered handlers | Looks up `ChatBoxInsertChar` handlers              |
| Component  | Routes command to the right domain object method   | Calls `state.input_box.insert_char('x')`           |
| Domain obj | Enforces invariants, mutates its own state         | Appends char, validates, updates cursor            |
| Renderer   | Reads state, draws the frame                       | Renders `input_box.text()` at `input_box.cursor()` |

### State

`AppData` (in `nullslop-protocol`) holds shared application state — mode, chat history, the quit flag. `TuiState` (in `nullslop-tui`) holds ephemeral TUI concerns like scroll offset and domain objects such as `InputBox`.

All mutation goes through `&mut AppData`. There is no component-local state.

### Domain Objects

Domain objects encapsulate behavior and hide their internals behind public methods:

```rust
pub struct InputBox {
    text: String,         // private
    cursor: usize,        // private
}

impl InputBox {
    pub fn insert_char(&mut self, ch: char) { ... }
    pub fn delete_grapheme(&mut self) { ... }
    pub fn delete_word(&mut self) { ... }
    pub fn text(&self) -> &str { ... }
    pub fn cursor(&self) -> usize { ... }
}
```

They are tested in isolation — grapheme handling, cursor bounds, empty buffer edge cases, etc.

### Components

Components are unit structs with no fields. They implement `CommandHandler<C>` or `EventHandler<E>` by delegating to domain object methods:

```rust
define_component! {
    pub(crate) struct InputModeComponent;

    commands {
        ChatBoxInsertChar: on_insert_char,
        ChatBoxDeleteGrapheme: on_delete_grapheme,
    }

    events {}
}

impl InputModeComponent {
    fn on_insert_char(cmd: &ChatBoxInsertChar, state: &mut AppData, _out: &mut Out) -> CommandAction {
        state.input_box.insert_char(cmd.ch);
        CommandAction::Continue
    }
}
```

Component code is mechanical — "when this command arrives, make a decision and then call this method." The real logic lives in domain objects.

### Bus

The bus (`nullslop-component`) dispatches commands and events to handlers by `TypeId`. Key properties:

- **Typed dispatch** — handlers only receive the command/event types they register for
- **Command interception** — first handler returning `Stop` halts propagation
- **Events are fire-and-forget** — all handlers always run, no interception
- **Consistent snapshot** — one `&mut AppData` per bus iteration
- **Cascading** — handlers can submit new commands/events via `Out`, processed in subsequent iterations
- **Guard rail** — `max_iterations` (default 100) prevents infinite loops

### Handler Traits

```rust
pub trait CommandHandler<C: 'static> {
    fn handle(&self, cmd: &C, state: &mut AppData, out: &mut Out) -> CommandAction;
}

pub trait EventHandler<E: 'static> {
    fn handle(&self, evt: &E, state: &mut AppData, out: &mut Out);
}
```

Takes `&self` so future scripted components (Lua, Python) can hold their runtime behind interior mutability (e.g., `Rc<RefCell<Runtime>>`). Native components are unit structs — `&self` is unused but harmless.

### Keymap

The keymap binds physical keys to `Command` variants scoped by mode (`Normal`, `Input`). When a key matches, the which-key system produces a `Command` and feeds it into the bus.

### Extensions

Extensions are out-of-process (e.g., subprocess communicating over JSON). They observe events and send commands back asynchronously. The extension host bridges between the external process and the bus. Extensions are separate from components — they use the same `Command`/`Event` wire protocol but don't implement handler traits.

## Crate Structure

| Crate                  | Responsibility                                                                                       |
| ---------------------- | ---------------------------------------------------------------------------------------------------- |
| `nullslop-protocol`    | `Command`, `Event`, `AppData`, `Mode`, `CommandAction` — wire types shared by all crates             |
| `nullslop-component`   | `CommandHandler`, `EventHandler`, `Bus`, `Out`, `define_component!` macro — dispatch infrastructure |
| `nullslop-core`        | `State` (wraps `AppData` + `ExtensionRegistry` in `RwLock`) — shared state container                 |
| `nullslop-tui`         | Components, keymap, renderer, terminal setup — the TUI application                                  |
| `nullslop-extension`   | JSON codec, subprocess management — extension host infrastructure                                    |

## Testing Strategy

- **Domain objects** — thorough unit tests for behavior and edge cases (grapheme handling, cursor bounds, etc.)
- **Components** — thin integration tests verifying the right domain method is called
- **Bus** — dispatch, propagation, cascading, and guard rail tests
- **Renderer** — layout and size constraint tests

Tests follow Given/When/Then structure. Fakes (`FakeCommandHandler`, `FakeEventHandler`) verify dispatch without real logic.
