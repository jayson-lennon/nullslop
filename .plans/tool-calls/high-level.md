# Tool Calls — High-Level Plan

## Problem

The app currently supports basic LLM streaming (text in, text out) but has no tool calling capability. When an LLM wants to use a tool (e.g., read a file, search the web), the system needs to:

1. **Stream tool call data** from the LLM alongside text (the `llm` crate's `StreamChunk` supports this via `ToolUseStart`, `ToolUseInputDelta`, `ToolUseComplete`)
2. **Execute tools asynchronously** via a dedicated orchestrator actor that spawns a tokio task per tool call
3. **Display tool progress/results** in the chat log with streaming argument display
4. **Loop multi-turn**: the LLM can call tools → receive results → call more tools → until it produces a final text-only response
5. **Use our own types** — no `llm` crate types leak outside `nullslop-providers`

Currently: `LlmMessage` is `role + String`. `ChatEntryKind` has no tool variants. The LLM actor uses `chat_stream` (plain text tokens). No tool types exist anywhere in the protocol.

## Architecture

The multi-turn tool loop is driven by the LLM actor through a state machine, with all coordination happening over the bus via commands/events.

```
                     Bus (commands/events)
                          │
        ┌─────────────────┼──────────────────────┐
        │                 │                      │
  LLM Actor         Tool Orchestrator       Chat Log Handler
        │                 │                      │
        │  SendToLlmProvider                   PushChatEntry
        │  ──────────>                          │
        │                 │                      │
        │  (spawns stream task)                 │
        │  StreamToken ──────────────────────>  │
        │  ToolCallStreaming ────────────────>  │
        │                 │                      │
        │  ExecuteToolBatch ──>                  │
        │                 │                      │
        │                 │ (spawns task/tool)
        │                 │                      │
        │  ToolBatchCompleted <──               │
        │  PushToolResult ───────────────────>  │
        │                 │                      │
        │  (new stream with results)             │
        │  StreamToken ──────────────────────>  │
        │  StreamCompleted ──────────────────>  │
```

**LLM Actor session state machine:**

```
Idle → Streaming(text + tool_calls) → AwaitingToolResults → Streaming(text) → ... → Idle
```

When the stream task detects `stop_reason: "tool_use"`, it emits `ExecuteToolBatch` and the LLM actor transitions to `AwaitingToolResults`. When `ToolBatchCompleted` arrives (from the orchestrator), the LLM actor builds new messages with tool results and starts a new stream. This repeats until `stop_reason: "end_turn"`.

**Tool registration:** Actors register tools by sending `RegisterTools` command at startup. The tool orchestrator stores definitions and the provider actor name. For execution, the orchestrator routes `ExecuteTool` commands to the provider actor by name. Built-in tools are handled directly by the orchestrator.

## Design Decisions

- `LlmMessage` becomes an enum (`User`, `Assistant`, `Tool` variants) instead of a struct. Breaking change is acceptable — clean design over backward compat.
- Raw JSON for tool argument streaming in chat log. Pretty-printing is out of scope.
- Built-in tools: `echo` (returns input), `get_time` (returns current timestamp), `file_read` (reads a file from disk). No security precautions — that's the responsibility of the host OS and wrapper scripts.
- No `llm` crate types in `nullslop-protocol`, `nullslop-component`, or `nullslop-core`. All conversion happens at the `nullslop-providers` boundary.

## Implementation Phases

- [x] **Phase 1: Protocol Types** (`nullslop-protocol`) — COMPLETED
  - Create `tool/` module: `ToolCall` (id, name, arguments JSON), `ToolResult` (id, name, content, success), `ToolDefinition` (name, description, parameters schema)
  - Convert `LlmMessage` from struct to enum: `User { content }`, `Assistant { content, tool_calls: Option<Vec<ToolCall>> }`, `Tool { tool_call_id, name, content }`
  - Remove `LlmRole` (role is now implied by variant)
  - Extend `ChatEntryKind`: add `ToolCall { id, name, arguments }` and `ToolResult { id, name, content, success }` variants
  - New commands: `RegisterTools { definitions }`, `ExecuteToolBatch { session_id, tool_calls }`, `ExecuteTool { tool_call }`, `ToolCallReceived { session_id, tool_call }`, `ToolCallStreaming { session_id, index, partial_json }`, `PushToolResult { session_id, result }`
  - New events: `ToolBatchCompleted { session_id, results }`, `ToolExecutionCompleted { session_id, result }`, `ToolsRegistered { provider, definitions }`
  - Update `Command` and `Event` enums with all new variants
  - Update `entries_to_messages` to handle new `ChatEntryKind` variants (produce `LlmMessage::Assistant` with tool_calls and `LlmMessage::Tool` for tool results)

  > **Phase 1 completed.** `serde_json` was already a full dep (not a dev-dep), so no Cargo.toml change was needed. No other divergences. All 152 tests pass. Downstream breakage in `nullslop-providers`, `nullslop-component`, and `actors/nullslop-llm` is expected — `LlmRole` removed, `LlmMessage` struct→enum, `ChatEntryKind` has 2 new variants.

- [x] **Phase 2: Provider Layer** (`nullslop-providers`) — COMPLETED
  - Define internal `StreamEvent` enum: `Text(String)`, `ToolUseStart { index, id, name }`, `ToolUseInputDelta { index, partial_json }`, `ToolUseComplete { index, tool_call }`, `Done { stop_reason }`
  - Add `LlmService::chat_stream_with_tools(messages, tools)` method returning `ToolStream` (our own `StreamEvent` enum)
  - Conversion: our `ToolDefinition` → `llm::Tool`, our `ToolCall` → `llm::ToolCall`, our `LlmMessage` → `llm::ChatMessage` (including tool-use and tool-result message types)
  - Conversion: `llm::StreamChunk` → our `StreamEvent` (boundary translation)
  - Update `llm_messages_to_chat_messages` for the new `LlmMessage` enum
  - Update `FakeLlmServiceFactory` to support the new method

  > **Phase 2 completed.** All 100 tests pass. Divergences: (1) `llm` crate's sub-modules (`message`, `stream`, `tool`) are private — imports use `llm::chat::*` re-exports instead. (2) `FunctionTool` is re-exported from `llm::chat`, not `llm` root. (3) `FakeLlmService::chat_stream_with_tools` builds event stream inline instead of calling default impl to avoid infinite recursion (VTable dispatch). Downstream breakage in `actors/nullslop-llm` and `nullslop-services` is expected — fixed in Phase 4.

- [x] **Phase 3: Tool Orchestrator Actor** (`actors/nullslop-tool-orchestrator`) — COMPLETED
  - New actor that subscribes to `RegisterTools` and `ExecuteToolBatch` commands
  - Maintains tool registry: `HashMap<String, ToolRegistration>` mapping tool name → (definition, provider actor name or builtin)
  - On `RegisterTools`: stores definitions + provider source name, emits `ToolsRegistered` event
  - On `ExecuteToolBatch`: for each tool call:
    - If built-in: spawn tokio task (use `tokio::fs` for `file_read`, not blocking `std::fs`)
    - If actor-provided: route `ExecuteTool` command to the registered provider actor
  - Tracks pending tool calls per batch. When all complete, emits `ToolBatchCompleted` event
  - Built-in tools registered at activation: `echo`, `get_time`, `file_read`
  - Subscribes to `ToolExecutionCompleted` events from provider actors to aggregate batch results
  - No batch timeout — if an actor-provided tool never responds, the batch stays pending forever. Acceptable for local dev tool; can add timeout in a later pass.
  - Actor-provided tool routing: the orchestrator sends `ExecuteTool` commands on the bus; provider actors must subscribe to `ExecuteTool` and emit `ToolExecutionCompleted` when done. If no actor handles a tool, the batch never completes. Phase 6 (Wiring) handles provider actor subscription.

  > **Phase 3 completed.** All 13 tests pass. Divergence: `ToolRegistration::Builtin` uses `fn(ToolCall) -> Pin<Box<dyn Future<Output = ToolResult> + Send>>` instead of `fn(&ToolCall) -> ToolResult` to support async `file_read` via `tokio::fs`. Added `jiff` and `tempfile` (dev-dep) dependencies. Unknown tools produce an error `ToolResult` synchronously (not via spawned task). `ToolRegistration::definition` fields are unused but kept for future tool listing — custom `Debug` impl provided.

- [x] **Phase 4: LLM Actor Rewrite** (`actors/nullslop-llm`) — COMPLETED
  - Switch from `chat_stream` to `chat_stream_with_tools`
  - Per-session state machine: `Idle | Streaming | AwaitingToolResults`
  - Store accumulated `messages: Vec<LlmMessage>` per session (survives across tool loops)
  - Subscribe to `ToolBatchCompleted`, `ToolsRegistered`, and `StreamCompleted` events
  - Extended `StreamCompleted` protocol type with `ToolUse` reason variant and `assistant_content`/`tool_calls` fields for stream-task-to-actor data flow
  - Stream task accumulates text and tool calls locally, sends them back via `StreamCompleted { reason: ToolUse, assistant_content, tool_calls }` event
  - `StreamEvent::Text` → emit `StreamToken` command
  - `StreamEvent::ToolUseStart` → emit `ToolUseStarted` command
  - `StreamEvent::ToolUseInputDelta` → emit `ToolCallStreaming` command
  - `StreamEvent::ToolUseComplete` → emit `ToolCallReceived` command
  - `StreamEvent::Done("tool_use")` → emit `ExecuteToolBatch` command + `StreamCompleted { reason: ToolUse }` event
  - `StreamEvent::Done("end_turn")` → emit `StreamCompleted { reason: Finished }` event
  - On `StreamCompleted { reason: ToolUse }`: store accumulated data in session, transition to `AwaitingToolResults`
  - On `ToolBatchCompleted`: emit `PushToolResult` for each result, build messages (assistant + tool results), start new stream
  - On `ToolsRegistered`: cache tool definitions for passing to `chat_stream_with_tools`
  - On `CancelStream`: abort stream task, remove session, emit `StreamCompleted { reason: Canceled }`
  - Removed broken `llm_messages_to_chat_messages` import (function never existed publicly)
  - Removed `llm` crate direct dependency, added `nullslop-providers` direct dependency
  - Fixed downstream: bus.rs (added new command/event dispatch arms), chat_log element (added ToolCall/ToolResult rendering), request_handler tests, protocol event tests
  - 10 tests pass in `nullslop-llm`, all 662 workspace tests pass

  > **Phase 4 completed.** Divergences: (1) Added `StreamCompleted` subscription — the actor receives its own `StreamCompleted` events back from the bus to transition state (the plan didn't explicitly state this subscription). (2) Added `StreamCompleted { reason: Finished }` also carries `assistant_content` (for consistency, not strictly necessary). (3) Fixed pre-existing compilation issues in `bus.rs` (missing dispatch arms for Phase 1 tool command/event variants) and `chat_log/element.rs` (missing match arms for `ChatEntryKind::ToolCall`/`ToolResult`). (4) The `ToolUseStarted` command was emitted (plan said `ToolCallStreaming` for `ToolUseStart` but the protocol has a separate `ToolUseStarted` command). (5) The stream task passes empty messages to `chat_stream_with_tools` — this is because `FakeLlmService` doesn't use messages. The real provider implementation will receive the correct messages. This needs to be fixed in Phase 6 when wiring the actual message passing.

- [x] **Phase 5: Chat Log & UI** (`nullslop-component/chat_log`) — COMPLETED
  - Extended `ChatSessionState`: added `streaming_tool_call_indices: HashMap<usize, usize>` field for tracking in-progress tool call entries
  - Added `begin_tool_call(index, id, name)` — creates placeholder `ToolCall` entry with empty arguments, stores index mapping
  - Added `append_tool_call_delta(index, partial_json)` — appends incremental delta to tool call arguments
  - Added `finalize_tool_call(id, name, arguments)` — overwrites arguments with final complete value (searches history by ID)
  - Updated `finish_streaming` and `cancel_streaming` to clear `streaming_tool_call_indices`
  - Extended `ProviderHandler` with 4 new command handlers: `ToolUseStarted`, `ToolCallStreaming`, `ToolCallReceived`, `PushToolResult`
  - Extracted `ensure_streaming` helper to handle tool calls arriving before text tokens
  - No changes to `chat_log/element.rs` (rendering already done in Phase 4)
  - 14 new tests (7 unit tests for `ChatSessionState`, 7 bus tests for `ProviderHandler`)
  - All 676 workspace tests pass

  > **Phase 5 completed.** No divergences from the plan. The `finalize_tool_call` method was made `pub(crate)` (plan said private) since it's called from `ProviderHandler` in a sibling module. The `ensure_streaming` helper handles the `is_sending` → streaming transition correctly for both `StreamToken` and `ToolUseStarted`.

- [x] **Phase 6: Wiring & Integration** — COMPLETED
  - Register tool orchestrator actor in app startup (alongside LLM actor)
  - Inject `LlmServiceFactoryService` and tool definitions into LLM actor context
  - Register new bus handlers for tool-related commands
  - Update `register_all()` in `nullslop-component/src/lib.rs`
  - End-to-end test: user sends message → LLM calls echo tool → result displayed → LLM continues
  - **Fix stream task message passing**: the stream task currently passes empty messages to `chat_stream_with_tools`. Wire the actual session messages through so the real provider receives the conversation history.

  > **Phase 6 completed.** Spawned tool orchestrator actor in `create_core_with_actor_host` alongside echo, LLM, and discover actors. Fixed stream task to pass actual messages (cloned before insertion into `SessionData`) instead of `vec![]`. Tool orchestrator now emits `ToolsRegistered` event for built-in tools in `activate()`. Lifecycle events (`ActorStarting`/`ActorStarted`) emitted for tool orchestrator. 677 tests pass.

## Acceptance Criteria

- [ ] LLM can request tool calls during a streaming response
- [ ] Multiple tool calls in a single LLM response are all executed
- [ ] Multi-turn tool loops: LLM calls tools → receives results → may call more tools → eventually produces final text
- [ ] Tool calls stream their arguments in real-time in the chat log (like text tokens)
- [ ] Tool results are displayed in the chat log with success/error status
- [ ] Tool orchestrator actor manages registration and async execution
- [ ] Actors register tools by sending `RegisterTools` command at startup
- [ ] Built-in tools (echo, get_time, file_read) prove the pipeline works end-to-end
- [ ] No `llm` crate types appear in `nullslop-protocol`, `nullslop-component`, or `nullslop-core`
- [ ] All existing tests continue to pass
- [ ] New code follows Given/When/Then testing patterns
- [ ] Consumers of streams can handle `Canceled` without a corresponding `StreamCompleted` event — cancellation aborts the stream task directly; the `StreamCompleted { reason: Canceled }` event is emitted by the actor's `cancel_stream` method, not by the stream task itself
