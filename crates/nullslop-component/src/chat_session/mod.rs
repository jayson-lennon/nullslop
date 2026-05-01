//! Session state for a single conversation.
//!
//! [`ChatSessionState`] owns the history and streaming state for one chat session.
//! Multiple sessions can exist concurrently in the application, each identified
//! by a [`SessionId`](nullslop_protocol::SessionId).

mod state;

pub use state::ChatSessionState;
