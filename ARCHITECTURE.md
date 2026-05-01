# Architecture

nullslop is a TUI chat application built on a component system with an actor bridge. Components are synchronous and have direct access to shared state. Actors are asynchronous processes that communicate only via messages.

## Data Flow

```
  Command/Event (input)  <--- keymap input (produces commands)
         |
         v
  +---- Bus (sync loop) ----------------------+    +-- AppState (single source of truth) --+
  |                                           |    |                                       |
  |  dispatch --> component handlers          |    |  component handlers read/write here   |
  |                actor host                 |    |              |                        |
  |                    |                      |    |              v                        |
  |                    v                      |    |         state mutated                 |
  |               Out buffer                  |    |              |                        |
  |                    |                      |    |              v                        |
  |              new cmd/evt                  |    |         renderer reads                |
  |                    |                      |    |              |                        |
  |      loops back until queue empty         |    |              v                        |
  |                    ^                      |    |         draws to screen               |
  +-------------------------------------------+    +---------------------------------------+
```

### Bus properties

- **Typed dispatch** — handlers register for specific `Command` or `Event` variants
- **Command interception** — first handler returning `Stop` halts propagation for that command
- **Events are fire-and-forget** — all handlers always run, no interception
- **Cascading** — handlers submit new messages via `Out`, processed in subsequent iterations

## Components

A **component** is a directory under `crates/nullslop-component/src/` having various submodules, depending on what it needs:

| File         | Purpose                                                      |
| ------------ | ------------------------------------------------------------ |
| `mod.rs`     | Registration — wires the component into the bus and registry |
| `handler.rs` | Bus access — reacts to commands/events, mutates `AppState`. When adding a handler for a **new** command or event, also add the variant to the `Command`/`Event` enum in `nullslop-protocol`. |
| `element.rs` | Rendering — implements `UiElement`, draws to a `Frame`       |
| `state.rs`   | State — component-specific data held in `AppState`           |

For example, a chat input box needs all four since it handles input, rendering, and state. A clock display would only need `mod.rs` and `element.rs` since it just renders to the screen.

> **Reminder:** Every command or event struct in a domain module must have a corresponding variant on the `Command` or `Event` enum in `nullslop-protocol`. The bus dispatches by enum variant — a struct without a variant is invisible.

### Handler pattern

Handlers are unit structs defined with the `define_handler!` macro, which wires specific command and event types to methods:

```rust
define_handler! {
    pub(crate) struct ChatInputBoxHandler;

    commands {
        ChatBoxInsertChar: on_insert_char,
        ChatBoxDeleteGrapheme: on_delete_grapheme,
    }

    events {}
}

impl ChatInputBoxHandler {
    fn on_insert_char(cmd: &ChatBoxInsertChar, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.chat_input.input_buffer.push(cmd.ch);
        CommandAction::Continue
    }
}
```

Handlers are mechanical — "when this command or event arrives, call this method on state." Each component's `mod.rs` registers its handler and element in a single `register(bus, registry)` function called once at startup.

### State

`AppState` (in `nullslop-component`) is the single source of truth. All components read and write the same `AppState`. Component state within `AppState` _must always_ use a struct:

```rust
#[derive(Debug)]
pub struct AppState {
    pub chat_input: ChatInputBoxState,
    pub shutdown_tracker: ShutdownTrackerState,

    // (not a component)
    pub should_quit: bool,
}
```

`State` (in `nullslop-core`) wraps `AppState` and the actor registry in an `RwLock` for cross-thread access. The processing loop acquires a write lock, processes all pending commands and events, then releases it.

### Rendering

UI elements implement `UiElement<AppState>` from `nullslop-component-ui`. They read `AppState` and draw to a ratatui `Frame`. Elements don't know about the bus or handlers — they just read state and draw stuff.

## Actors

Actors run asynchronously on separate threads or processes. They cannot access `AppState` — they communicate only through the command/event message protocol.

```
Host (nullslop)                                  Actor process
────────────────                                 ──────────────
Actor host      ---> JSON over stdin/stdout -->  Actor SDK
(nullslop-actor-host)                            (nullslop-actor)
```

- **`nullslop-actor-host`** — host side: discovers actors, manages lifecycle, bridges messages between the bus and actor processes
- **`nullslop-actor`** — SDK for authors: provides the `Actor` trait, JSON codec, and a `run!` macro

Two host implementations exist: `ProcessActorHost` (subprocess, JSON over stdio) and `InMemoryActorHost` (OS thread, no serialization).

## Crate Structure

| Crate                     | Responsibility                                                          |
| ------------------------- | ----------------------------------------------------------------------- |
| `nullslop-protocol`       | `Command`, `Event`, `CommandAction`, `Mode`, `Key` — wire types         |
| `nullslop-component-core` | `CommandHandler`, `EventHandler`, `Bus`, `Out`, `define_handler!`       |
| `nullslop-component-ui`   | `UiElement` trait, `UiRegistry` — renderable element infrastructure     |
| `nullslop-component`      | Built-in components (chat input, chat log, quit, shutdown tracking)     |
| `nullslop-core`           | `State` (RwLock wrapper), `AppCore` processing loop, actor registry |
| `nullslop-services`       | `Services` container — runtime services shared across the application   |
| `nullslop-tui`            | Terminal setup, event loop, keymap, renderer, top-level `TuiApp`        |
| `nullslop-actor-host`    | Actor host implementations (process-based, in-memory)                    |
| `nullslop-actor`         | SDK for actor authors (`Actor` trait, codec, `run!` macro)               |
| `nullslop-cli`            | CLI argument parsing                                                    |

## Keymap

The keymap (in `nullslop-tui`) binds physical keys to `Command` variants scoped by mode (`Normal`, `Input`). When a key matches, the which-key system produces a `Command` and feeds it into the bus.

## Testing Strategy

- **Component handlers** — test via the bus: register handler, submit command, assert `AppState` changes
- **UI elements** — test with ratatui `TestBackend`: render with known state, assert buffer contents
- **Bus** — dispatch, propagation, cascading, and guard rail tests (uses `FakeCommandHandler`)
- **State types** — unit tests for behavior (grapheme handling, cursor bounds, etc.)

Tests follow Given/When/Then structure. See `AGENTS.md` for detailed testing patterns.
