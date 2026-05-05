//! State for a single chat session — history, input box, and streaming progress.

use std::collections::{HashMap, VecDeque};

use crate::chat_input_box::ChatInputBoxState;
use nullslop_protocol::{ChatEntry, ChatEntryKind};

/// The state of a single chat session.
///
/// Owns the conversation history and tracks whether an LLM response is
/// currently streaming in. The streaming entry is an in-progress `Assistant`
/// entry at a known index — tokens are appended to it until the stream
/// completes or is cancelled.
#[derive(Debug)]
pub struct ChatSessionState {
    /// All messages in this conversation.
    history: Vec<ChatEntry>,
    /// The user's in-progress message for this session.
    chat_input: ChatInputBoxState,
    /// Index into `history` for the entry currently receiving stream tokens.
    streaming_entry_index: Option<usize>,
    /// Whether an LLM stream is actively producing tokens.
    is_streaming: bool,
    /// Messages waiting to be sent to the LLM, one at a time.
    message_queue: VecDeque<String>,
    /// Whether a message has been dispatched to the LLM but no tokens have arrived yet.
    is_sending: bool,
    /// Whether a prompt assembly request is in progress.
    is_assembling: bool,
    /// The active prompt strategy for this session.
    active_strategy: nullslop_protocol::PromptStrategyId,
    /// Maps stream tool call index to history index for in-progress tool calls.
    streaming_tool_call_indices: HashMap<usize, usize>,
    /// Number of lines to skip from the top when rendering (ratatui scroll offset).
    scroll_offset: u16,
}

impl ChatSessionState {
    /// Create a new session with empty history and no active stream.
    #[must_use]
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            chat_input: ChatInputBoxState::new(),
            streaming_entry_index: None,
            is_streaming: false,
            message_queue: VecDeque::new(),
            is_sending: false,
            is_assembling: false,
            active_strategy: nullslop_protocol::PromptStrategyId::passthrough(),
            streaming_tool_call_indices: HashMap::new(),
            scroll_offset: 0,
        }
    }

    /// Read-only access to this session's input box state.
    pub fn chat_input(&self) -> &ChatInputBoxState {
        &self.chat_input
    }

    /// Mutable access to this session's input box state.
    pub fn chat_input_mut(&mut self) -> &mut ChatInputBoxState {
        &mut self.chat_input
    }

    /// Read-only access to the conversation history.
    pub fn history(&self) -> &[ChatEntry] {
        &self.history
    }

    /// Append an entry to the history and return its index.
    ///
    /// Resets scroll to the bottom so new messages are visible.
    pub fn push_entry(&mut self, entry: ChatEntry) -> usize {
        let index = self.history.len();
        self.history.push(entry);
        self.reset_scroll();
        index
    }

    /// Begin a new streaming response.
    ///
    /// Creates an empty `Assistant` entry, marks the session as streaming,
    /// and returns the index of the new entry.
    ///
    /// # Panics
    ///
    /// Panics if the session is already streaming. This is a programming error —
    /// the caller must ensure the previous stream has finished or been cancelled
    /// before starting a new one.
    pub fn begin_streaming(&mut self) -> usize {
        assert!(
            !self.is_streaming,
            "begin_streaming called while already streaming"
        );
        let entry = ChatEntry::assistant("");
        let index = self.push_entry(entry);
        self.streaming_entry_index = Some(index);
        self.is_streaming = true;
        index
    }

    /// Append a token to the streaming assistant entry.
    ///
    /// # Panics
    ///
    /// Panics if the session is not streaming. This is a programming error.
    #[expect(
        clippy::indexing_slicing,
        reason = "index comes from push_entry which always returns a valid index"
    )]
    #[expect(
        clippy::expect_used,
        reason = "streaming_entry_index invariant guaranteed by begin_streaming"
    )]
    #[expect(
        clippy::panic,
        reason = "streaming invariant violated: entry must be Assistant during active stream"
    )]
    pub fn append_stream_token<S>(&mut self, token: S)
    where
        S: AsRef<str>,
    {
        assert!(
            self.is_streaming,
            "append_stream_token called while not streaming"
        );
        let index = self
            .streaming_entry_index
            .expect("streaming_entry_index must be set when is_streaming");
        if let ChatEntry {
            kind: nullslop_protocol::ChatEntryKind::Assistant(ref mut text),
            ..
        } = self.history[index]
        {
            text.push_str(token.as_ref());
        } else {
            panic!("streaming entry is not an Assistant entry");
        }
    }

    /// Mark streaming as finished (normal completion).
    pub fn finish_streaming(&mut self) {
        self.is_streaming = false;
        self.is_sending = false; // defensive: clear both on finish
        self.streaming_entry_index = None;
        self.streaming_tool_call_indices.clear();
    }

    /// Cancel streaming but keep partial text in history.
    pub fn cancel_streaming(&mut self) {
        self.is_streaming = false;
        self.is_sending = false; // defensive: clear both on cancel
        self.streaming_entry_index = None;
        self.streaming_tool_call_indices.clear();
    }

    /// Whether an LLM stream is actively producing tokens.
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }

    // --- Tool call streaming ---

    /// Create a placeholder `ToolCall` entry and record its history index.
    ///
    /// Called when `ToolUseStarted` arrives — the tool name is known but arguments
    /// are still streaming in.
    pub fn begin_tool_call(&mut self, index: usize, id: &str, name: &str) {
        let entry = ChatEntry::tool_call(id, name, "");
        let history_index = self.push_entry(entry);
        self.streaming_tool_call_indices
            .insert(index, history_index);
    }

    /// Append an incremental delta to a streaming tool call's arguments.
    ///
    /// `partial_json` is appended to the existing arguments string — it is *not*
    /// the accumulated total.
    ///
    /// # Panics
    ///
    /// Panics if no tool call entry is tracked for the given stream index.
    #[expect(
        clippy::indexing_slicing,
        reason = "index comes from push_entry which always returns a valid index"
    )]
    pub fn append_tool_call_delta(&mut self, index: usize, partial_json: &str) {
        let history_index = self
            .streaming_tool_call_indices
            .get(&index)
            .copied()
            .expect("append_tool_call_delta: no entry tracked for this stream index");
        if let ChatEntryKind::ToolCall {
            ref mut arguments,
            ..
        } = self.history[history_index].kind
        {
            arguments.push_str(partial_json);
        }
    }

    /// Overwrite a tool call entry with the final complete arguments.
    ///
    /// Searches recent history for a `ToolCall` entry matching the given ID.
    /// If not found (shouldn't happen in normal flow), pushes a new entry.
    pub(crate) fn finalize_tool_call(&mut self, id: &str, name: &str, arguments: &str) {
        for entry in self.history.iter_mut().rev() {
            if let ChatEntryKind::ToolCall {
                id: ref entry_id,
                ..
            } = entry.kind
            {
                if entry_id == id {
                    entry.kind = ChatEntryKind::ToolCall {
                        id: id.to_owned(),
                        name: name.to_owned(),
                        arguments: arguments.to_owned(),
                    };
                    return;
                }
            }
        }
        // If not found (shouldn't happen), push a new entry.
        self.push_entry(ChatEntry::tool_call(id, name, arguments));
    }

    // --- Queue ---

    /// Read-only access to the message queue.
    pub fn queue(&self) -> &VecDeque<String> {
        &self.message_queue
    }

    /// Number of messages waiting in the queue.
    pub fn queue_len(&self) -> usize {
        self.message_queue.len()
    }

    /// Push a message onto the back of the queue.
    pub fn enqueue_message(&mut self, text: String) {
        self.message_queue.push_back(text);
    }

    /// Pop the front message from the queue, if any.
    pub fn dequeue_message(&mut self) -> Option<String> {
        self.message_queue.pop_front()
    }

    /// Drain all queued messages, returning them in order.
    pub fn drain_queue(&mut self) -> VecDeque<String> {
        std::mem::take(&mut self.message_queue)
    }

    // --- Assembling ---

    /// Mark the session as having a prompt assembly in progress.
    ///
    /// # Panics
    ///
    /// Panics if already sending, streaming, or assembling.
    pub fn begin_assembling(&mut self) {
        assert!(
            !self.is_sending && !self.is_streaming && !self.is_assembling,
            "begin_assembling called while already busy"
        );
        self.is_assembling = true;
    }

    /// Clear the assembling flag (called when prompt assembly completes).
    pub fn finish_assembling(&mut self) {
        assert!(self.is_assembling, "finish_assembling called while not assembling");
        self.is_assembling = false;
    }

    /// Whether a prompt assembly is in progress.
    pub fn is_assembling(&self) -> bool {
        self.is_assembling
    }

    /// Switch the active prompt strategy for this session.
    pub fn switch_strategy(&mut self, strategy_id: nullslop_protocol::PromptStrategyId) {
        self.active_strategy = strategy_id;
    }

    /// The currently active prompt strategy.
    pub fn active_strategy(&self) -> &nullslop_protocol::PromptStrategyId {
        &self.active_strategy
    }

    // --- Sending ---

    /// Mark the session as having dispatched a message to the LLM.
    ///
    /// # Panics
    ///
    /// Panics if already sending or streaming. This is a programming error —
    /// the caller must ensure the session is idle before dispatching.
    pub fn begin_sending(&mut self) {
        assert!(
            !self.is_sending && !self.is_streaming,
            "begin_sending called while already sending or streaming"
        );
        self.is_sending = true;
    }

    /// Clear the sending flag (called when the first stream token arrives).
    ///
    /// # Panics
    ///
    /// Panics if not currently sending.
    pub fn finish_sending(&mut self) {
        assert!(self.is_sending, "finish_sending called while not sending");
        self.is_sending = false;
    }

    /// Whether a message has been dispatched but no tokens have arrived yet.
    pub fn is_sending(&self) -> bool {
        self.is_sending
    }

    // --- Combined status ---

    /// Whether the session is completely idle (not sending, not streaming, not assembling).
    pub fn is_idle(&self) -> bool {
        !self.is_sending && !self.is_streaming && !self.is_assembling
    }

    /// The current scroll offset (lines to skip from top).
    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    /// Scroll up (toward older messages) by the given number of lines.
    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down (toward newer messages) by the given number of lines.
    ///
    /// Capped at `u16::MAX` — the element clamps during render.
    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    /// Reset scroll to show the bottom of the conversation.
    pub fn reset_scroll(&mut self) {
        self.scroll_offset = u16::MAX;
    }
}

impl Default for ChatSessionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use nullslop_protocol::ChatEntry;

    use super::*;

    #[test]
    fn push_entry_adds_to_history() {
        // Given a new ChatSessionState.
        let mut session = ChatSessionState::new();

        // When pushing a user entry.
        let index = session.push_entry(ChatEntry::user("hello"));

        // Then the index is 0 and history has one entry.
        assert_eq!(index, 0);
        assert_eq!(session.history().len(), 1);
    }

    #[test]
    fn begin_streaming_creates_assistant_entry_and_sets_streaming() {
        // Given a session with one entry.
        let mut session = ChatSessionState::new();
        session.push_entry(ChatEntry::user("hello"));

        // When beginning streaming.
        let index = session.begin_streaming();

        // Then the index is 1, is_streaming is true, and history has an Assistant entry.
        assert_eq!(index, 1);
        assert!(session.is_streaming());
        assert_eq!(session.history().len(), 2);
        assert!(matches!(
            session.history()[1].kind,
            nullslop_protocol::ChatEntryKind::Assistant(ref text) if text.is_empty()
        ));
    }

    #[test]
    fn append_stream_token_appends_to_assistant_entry() {
        // Given a session that is streaming.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // When appending a token.
        session.append_stream_token("Hello");
        session.append_stream_token(" world");

        // Then the assistant entry text is "Hello world".
        assert_eq!(
            session.history()[0].kind,
            nullslop_protocol::ChatEntryKind::Assistant("Hello world".to_owned())
        );
    }

    #[test]
    fn finish_streaming_clears_streaming_state() {
        // Given a session that is streaming with some tokens.
        let mut session = ChatSessionState::new();
        session.begin_streaming();
        session.append_stream_token("Hi");

        // When finishing streaming.
        session.finish_streaming();

        // Then is_streaming is false and text is preserved.
        assert!(!session.is_streaming());
        assert_eq!(
            session.history()[0].kind,
            nullslop_protocol::ChatEntryKind::Assistant("Hi".to_owned())
        );
    }

    #[test]
    fn cancel_streaming_keeps_partial_text() {
        // Given a session that is streaming with partial tokens.
        let mut session = ChatSessionState::new();
        session.begin_streaming();
        session.append_stream_token("Partial");

        // When cancelling streaming.
        session.cancel_streaming();

        // Then is_streaming is false but partial text is kept.
        assert!(!session.is_streaming());
        assert_eq!(
            session.history()[0].kind,
            nullslop_protocol::ChatEntryKind::Assistant("Partial".to_owned())
        );
    }

    #[test]
    #[should_panic(expected = "begin_streaming called while already streaming")]
    fn begin_streaming_twice_panics() {
        // Given a session that is already streaming.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // When calling begin_streaming again.
        // Then it panics.
        session.begin_streaming();
    }

    #[test]
    #[should_panic(expected = "append_stream_token called while not streaming")]
    fn append_stream_token_when_not_streaming_panics() {
        // Given a session that is not streaming.
        let mut session = ChatSessionState::new();

        // When calling append_stream_token.
        // Then it panics.
        session.append_stream_token("oops");
    }

    #[test]
    fn scroll_up_decrements_offset() {
        // Given a session with a high scroll offset.
        let mut session = ChatSessionState::new();
        session.reset_scroll();
        assert_eq!(session.scroll_offset(), u16::MAX);

        // When scrolling up by 10.
        session.scroll_up(10);

        // Then the offset decreased by 10.
        assert_eq!(session.scroll_offset(), u16::MAX - 10);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        // Given a session with scroll_offset = 5.
        let mut session = ChatSessionState::new();
        session.scroll_down(5);
        assert_eq!(session.scroll_offset(), 5);

        // When scrolling up by 20.
        session.scroll_up(20);

        // Then the offset saturates at 0.
        assert_eq!(session.scroll_offset(), 0);
    }

    #[test]
    fn scroll_down_increments_offset() {
        // Given a session with scroll_offset = 0.
        let mut session = ChatSessionState::new();
        assert_eq!(session.scroll_offset(), 0);

        // When scrolling down by 10.
        session.scroll_down(10);

        // Then the offset increased by 10.
        assert_eq!(session.scroll_offset(), 10);
    }

    #[test]
    fn reset_scroll_sets_to_max() {
        // Given a session with scroll_offset = 0.
        let mut session = ChatSessionState::new();
        assert_eq!(session.scroll_offset(), 0);

        // When resetting scroll.
        session.reset_scroll();

        // Then the offset is u16::MAX.
        assert_eq!(session.scroll_offset(), u16::MAX);
    }

    #[test]
    fn push_entry_resets_scroll() {
        // Given a session with scroll_offset = 0.
        let mut session = ChatSessionState::new();
        assert_eq!(session.scroll_offset(), 0);

        // When pushing an entry.
        session.push_entry(ChatEntry::user("hello"));

        // Then scroll_offset is u16::MAX (reset by push_entry).
        assert_eq!(session.scroll_offset(), u16::MAX);
    }

    // --- Queue tests ---

    #[test]
    fn enqueue_message_adds_to_queue() {
        // Given a new session with an empty queue.
        let mut session = ChatSessionState::new();
        assert_eq!(session.queue_len(), 0);

        // When enqueuing a message.
        session.enqueue_message("hello".to_owned());

        // Then the queue has one message.
        assert_eq!(session.queue_len(), 1);
        assert_eq!(session.queue()[0], "hello");
    }

    #[test]
    fn dequeue_message_returns_first_in_order() {
        // Given a session with two queued messages.
        let mut session = ChatSessionState::new();
        session.enqueue_message("first".to_owned());
        session.enqueue_message("second".to_owned());

        // When dequeuing a message.
        let msg = session.dequeue_message();

        // Then it returns the first message and the queue has one left.
        assert_eq!(msg.as_deref(), Some("first"));
        assert_eq!(session.queue_len(), 1);
    }

    #[test]
    fn dequeue_message_returns_none_when_empty() {
        // Given a session with an empty queue.
        let mut session = ChatSessionState::new();

        // When dequeuing a message.
        let msg = session.dequeue_message();

        // Then it returns None.
        assert!(msg.is_none());
    }

    #[test]
    fn drain_queue_empties_and_returns_all() {
        // Given a session with three queued messages.
        let mut session = ChatSessionState::new();
        session.enqueue_message("a".to_owned());
        session.enqueue_message("b".to_owned());
        session.enqueue_message("c".to_owned());

        // When draining the queue.
        let drained = session.drain_queue();

        // Then all messages are returned in order and the queue is empty.
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0], "a");
        assert_eq!(drained[1], "b");
        assert_eq!(drained[2], "c");
        assert_eq!(session.queue_len(), 0);
    }

    // --- Sending tests ---

    #[test]
    fn begin_sending_sets_is_sending() {
        // Given a new session (idle).
        let mut session = ChatSessionState::new();
        assert!(!session.is_sending());

        // When beginning sending.
        session.begin_sending();

        // Then is_sending is true.
        assert!(session.is_sending());
    }

    #[test]
    #[should_panic(expected = "begin_sending called while already sending or streaming")]
    fn begin_sending_panics_when_already_sending() {
        // Given a session that is already sending.
        let mut session = ChatSessionState::new();
        session.begin_sending();

        // When calling begin_sending again.
        // Then it panics.
        session.begin_sending();
    }

    #[test]
    #[should_panic(expected = "begin_sending called while already sending or streaming")]
    fn begin_sending_panics_when_streaming() {
        // Given a session that is streaming.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // When calling begin_sending.
        // Then it panics.
        session.begin_sending();
    }

    #[test]
    fn finish_sending_clears_flag() {
        // Given a session that is sending.
        let mut session = ChatSessionState::new();
        session.begin_sending();

        // When finishing sending.
        session.finish_sending();

        // Then is_sending is false.
        assert!(!session.is_sending());
    }

    #[test]
    #[should_panic(expected = "finish_sending called while not sending")]
    fn finish_sending_panics_when_not_sending() {
        // Given a session that is not sending.
        let mut session = ChatSessionState::new();

        // When calling finish_sending.
        // Then it panics.
        session.finish_sending();
    }

    // --- Combined status tests ---

    #[test]
    fn is_idle_true_when_not_sending_or_streaming() {
        // Given a fresh session.
        let session = ChatSessionState::new();

        // Then it is idle.
        assert!(session.is_idle());
    }

    #[test]
    fn is_idle_false_when_sending() {
        // Given a session that is sending.
        let mut session = ChatSessionState::new();
        session.begin_sending();

        // Then it is not idle.
        assert!(!session.is_idle());
    }

    #[test]
    fn is_idle_false_when_streaming() {
        // Given a session that is streaming.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // Then it is not idle.
        assert!(!session.is_idle());
    }

    #[test]
    fn cancel_streaming_clears_sending_too() {
        // Given a session that was sending before streaming started.
        let mut session = ChatSessionState::new();
        session.begin_sending();
        // Simulate: stream started (sending still set until first token clears it).
        // We need to manipulate internals since normally begin_streaming would panic
        // when is_sending is true. So we manually set is_streaming.
        session.is_streaming = true;
        assert!(session.is_sending());
        assert!(session.is_streaming());

        // When cancelling streaming.
        session.cancel_streaming();

        // Then both flags are cleared.
        assert!(!session.is_sending());
        assert!(!session.is_streaming());
    }

    #[test]
    fn finish_streaming_clears_sending_too() {
        // Given a session that was sending before streaming started.
        let mut session = ChatSessionState::new();
        session.begin_sending();
        // Manually set is_streaming to simulate the transition.
        session.is_streaming = true;
        session.streaming_entry_index = Some(session.push_entry(ChatEntry::assistant("")));

        // When finishing streaming.
        session.finish_streaming();

        // Then both flags are cleared.
        assert!(!session.is_sending());
        assert!(!session.is_streaming());
    }

    // --- Tool call streaming tests ---

    #[test]
    fn begin_tool_call_creates_entry_with_empty_arguments() {
        // Given a streaming session.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // When beginning a tool call.
        session.begin_tool_call(0, "call_1", "echo");

        // Then history has an assistant entry and a tool call entry with empty arguments.
        assert_eq!(session.history().len(), 2);
        assert!(matches!(
            session.history()[0].kind,
            ChatEntryKind::Assistant(_)
        ));
        assert_eq!(
            session.history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: String::new(),
            }
        );
    }

    #[test]
    fn append_tool_call_delta_accumulates_arguments() {
        // Given a streaming session with a tool call entry.
        let mut session = ChatSessionState::new();
        session.begin_streaming();
        session.begin_tool_call(0, "call_1", "echo");

        // When appending tool call deltas.
        session.append_tool_call_delta(0, r#"{"input":"#);
        session.append_tool_call_delta(0, r#""hello"}"#);

        // Then the tool call entry has the accumulated arguments.
        assert_eq!(
            session.history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"input":"hello"}"#.to_owned(),
            }
        );
    }

    #[test]
    fn finalize_tool_call_overwrites_arguments() {
        // Given a streaming session with a tool call that has partial arguments.
        let mut session = ChatSessionState::new();
        session.begin_streaming();
        session.begin_tool_call(0, "call_1", "echo");
        session.append_tool_call_delta(0, r#"{"input":"#);

        // When finalizing the tool call with the complete arguments.
        session.finalize_tool_call("call_1", "echo", r#"{"input":"world"}"#);

        // Then the arguments are overwritten with the final value.
        assert_eq!(
            session.history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"input":"world"}"#.to_owned(),
            }
        );
    }

    #[test]
    fn finalize_tool_call_pushes_new_entry_when_not_found() {
        // Given a streaming session with no tool call entry for the given ID.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // When finalizing a tool call that was never started (shouldn't happen normally).
        session.finalize_tool_call("call_99", "echo", r#"{"input":"hi"}"#);

        // Then a new entry is pushed to history.
        assert_eq!(session.history().len(), 2); // assistant + new tool call
        assert_eq!(
            session.history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_99".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"input":"hi"}"#.to_owned(),
            }
        );
    }

    #[test]
    fn multiple_tool_calls_track_independently() {
        // Given a streaming session.
        let mut session = ChatSessionState::new();
        session.begin_streaming();

        // When beginning two tool calls with different indices.
        session.begin_tool_call(0, "call_1", "echo");
        session.append_tool_call_delta(0, r#"{"a":1}"#);

        session.begin_tool_call(1, "call_2", "get_time");
        session.append_tool_call_delta(1, "{}");

        // Then each entry tracks its own arguments.
        assert_eq!(
            session.history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"a":1}"#.to_owned(),
            }
        );
        assert_eq!(
            session.history()[2].kind,
            ChatEntryKind::ToolCall {
                id: "call_2".to_owned(),
                name: "get_time".to_owned(),
                arguments: "{}".to_owned(),
            }
        );
    }

    #[test]
    fn finish_streaming_clears_tool_call_indices() {
        // Given a streaming session with a tool call entry.
        let mut session = ChatSessionState::new();
        session.begin_streaming();
        session.begin_tool_call(0, "call_1", "echo");

        // When finishing streaming.
        session.finish_streaming();

        // Then the tool call indices are cleared (entries remain in history).
        assert!(!session.is_streaming());
        assert_eq!(session.history().len(), 2); // assistant + tool call still there
    }

    #[test]
    fn cancel_streaming_clears_tool_call_indices() {
        // Given a streaming session with a tool call entry.
        let mut session = ChatSessionState::new();
        session.begin_streaming();
        session.begin_tool_call(0, "call_1", "echo");

        // When cancelling streaming.
        session.cancel_streaming();

        // Then the tool call indices are cleared (entries remain in history).
        assert!(!session.is_streaming());
        assert_eq!(session.history().len(), 2); // assistant + tool call still there
    }

    // --- Strategy switching tests ---

    #[test]
    fn default_strategy_is_passthrough() {
        // Given a new session.
        let session = ChatSessionState::new();

        // Then the default strategy is passthrough.
        assert_eq!(session.active_strategy(), &nullslop_protocol::PromptStrategyId::passthrough());
    }

    #[test]
    fn switch_strategy_updates_active_strategy() {
        // Given a new session.
        let mut session = ChatSessionState::new();

        // When switching to sliding_window.
        session.switch_strategy(nullslop_protocol::PromptStrategyId::sliding_window());

        // Then the active strategy is updated.
        assert_eq!(session.active_strategy(), &nullslop_protocol::PromptStrategyId::sliding_window());
    }
}
