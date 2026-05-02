//! State for a single chat session — history, input box, and streaming progress.

use std::collections::VecDeque;

use crate::chat_input_box::ChatInputBoxState;
use nullslop_protocol::ChatEntry;

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
    pub fn append_stream_token(&mut self, token: &str) {
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
            text.push_str(token);
        } else {
            panic!("streaming entry is not an Assistant entry");
        }
    }

    /// Mark streaming as finished (normal completion).
    pub fn finish_streaming(&mut self) {
        self.is_streaming = false;
        self.is_sending = false; // defensive: clear both on finish
        self.streaming_entry_index = None;
    }

    /// Cancel streaming but keep partial text in history.
    pub fn cancel_streaming(&mut self) {
        self.is_streaming = false;
        self.is_sending = false; // defensive: clear both on cancel
        self.streaming_entry_index = None;
    }

    /// Whether an LLM stream is actively producing tokens.
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
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

    /// Whether the session is completely idle (not sending, not streaming).
    pub fn is_idle(&self) -> bool {
        !self.is_sending && !self.is_streaming
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
}
