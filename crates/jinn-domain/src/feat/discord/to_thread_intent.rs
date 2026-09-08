//! Intent handler for `Intent::ToDiscordThread` ("gdc" — continue a jinn session
//! in a Discord forum thread).
//!
//! This is the jinn-side entry point. It runs pure precondition checks over
//! [`AppState`] and, only on success, emits a [`CreateThreadForSession`] bus
//! command. The command rides the bridge actor → request channel → gateway,
//! where the actual Discord thread is created and the mapping recorded. The
//! gateway reports the outcome via [`DiscordThreadCreated`] /
//! [`DiscordThreadCreateFailed`] events, which a feedback actor turns into a
//! [`ChatEntry`] in the session's history.
//!
//! On any precondition failure this handler pushes a [`ChatEntry::error`]
//! directly into the active session's history and returns an empty result —
//! no bus command is emitted. These are the synchronous fast-fail cases
//! (no title, disabled, not connected, no forum channel); the gateway owns
//! the asynchronous failure cases (already-bound, API errors, mapping write).

use crate::common::app_state::AppState;
use crate::common::slices::Slices;
use crate::feat::dashboard::ActorLifecycle;
use crate::feat::dashboard::dashboard_slot;
use crate::feat::discord::protocol::CreateThreadForSession;
use crate::feat::session::chat_entry::ChatEntry;
use crate::protocol::IntentResult;

/// Dashboard actor name for the discord gateway task.
const DISCORD_ACTOR_NAME: &str = "discord";

/// Handle `Intent::ToDiscordThread`.
///
/// Precondition chain (first failure wins):
/// 1. Active session has a title (else "send a message first").
/// 2. `[discord] enabled = true`.
/// 3. The gateway dashboard entry is `Running` (i.e. `Connected`).
/// 4. `[discord] forum_channel` is set.
///
/// On success, emits [`CreateThreadForSession`] with the session id and title.
///
/// # Errors
///
/// This function never returns `Err` — failures push a `ChatEntry::error`
/// into the active session and yield an empty `IntentResult`.
pub fn handle_to_discord_thread(state: &mut AppState, slices: &Slices) -> IntentResult {
    // Precondition 1: title exists. The session title is `None` until the first
    // user message is sent, so this also gates the "empty session" case.
    let Some(title) = state.active_session().title().map(str::to_owned) else {
        push_error(
            state,
            "Can't continue in Discord: this session has no title yet. \
             Send a message first.",
        );
        return IntentResult::empty();
    };

    // Precondition 2: discord enabled in config.
    if !state.frontend.preferences.discord.enabled {
        push_error(
            state,
            "Can't continue in Discord: the Discord bot is not enabled \
             (`[discord] enabled = false`).",
        );
        return IntentResult::empty();
    }

    // Precondition 3: the gateway task is connected. We read the dashboard
    // slice entry's lifecycle (fed by DiscordStatusActor republishing its
    // gateway status); the dashboard actor marks it `Running` on
    // `DiscordStatusUpdate::Connected`.
    if !discord_is_connected(slices) {
        push_error(
            state,
            "Can't continue in Discord: the bot isn't connected. \
             Check the dashboard and your token.",
        );
        return IntentResult::empty();
    }
    // All preconditions pass — request thread creation. The session id is
    // captured before emitting so a subsequent active-session change can't
    // race the binding.
    let session_id = state.session.active_session_id().clone();
    IntentResult::new_message(CreateThreadForSession { session_id, title })
}

/// Returns `true` when the discord dashboard-slice entry is `Running`
/// (connected). Resolves the slice through the [`Slices`] facade; an
/// unregistered or mistyped slot reads as "not connected" — the caller
/// pushes its own error, so no separate failure surface is needed.
fn discord_is_connected(slices: &Slices) -> bool {
    let dashboard_slot = dashboard_slot();
    let Some(dashboard) = slices.reader::<crate::feat::dashboard::DashboardState>(&dashboard_slot)
    else {
        return false;
    };
    dashboard
        .read()
        .actors()
        .iter()
        .find(|e| e.name == DISCORD_ACTOR_NAME)
        .is_some_and(|e| e.lifecycle == ActorLifecycle::Running)
}

/// Push an error `ChatEntry` into the active session's history.
fn push_error(state: &mut AppState, message: &str) {
    state
        .active_session_mut()
        .push_entry(ChatEntry::error(message));
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use super::handle_to_discord_thread;
    use crate::common::app_state::AppState;
    use crate::common::slices::Slices;
    use crate::feat::dashboard::DashboardState;
    use crate::feat::dashboard::dashboard_slot;
    use crate::feat::session::chat_entry::ChatEntryKind;

    /// Build the state + slices with the happy-path preconditions: a titled
    /// session, discord enabled + connected. (The gateway owns
    /// `forum_channel` validation, so the intent handler never reads it.)
    /// The discord connectivity is seeded into the dashboard slice cell.
    fn happy_state() -> (AppState, Slices) {
        let mut state = AppState::default();
        state
            .active_session_mut()
            .set_title("My session".to_owned());
        state.frontend.preferences.discord.enabled = true;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        cell.update(|d| d.mark_running("discord", None));
        (state, slices)
    }

    fn last_entry_kind(state: &AppState) -> &ChatEntryKind {
        &state
            .active_session()
            .history()
            .last()
            .expect("an entry")
            .kind
    }

    #[rstest::rstest]
    fn success_emits_create_thread_for_session() {
        // Given a session with all preconditions met.
        let (mut state, slices) = happy_state();

        // When handling ToDiscordThread.
        let result = handle_to_discord_thread(&mut state, &slices);

        // Then exactly one CreateThreadForSession is emitted.
        assert_eq!(result.message_names.len(), 1);
        assert!(
            result.message_names[0].contains("CreateThreadForSession"),
            "expected CreateThreadForSession; got {:?}",
            result.message_names
        );
    }

    /// Regression: `gdc` on a title-less session emits no request and surfaces
    /// an in-chat error. The precondition is checked synchronously in the intent
    /// handler, so no bus command (and thus no gateway thread creation) is ever
    /// reached.
    #[rstest::rstest]
    fn regression_title_less_session_gdc_emits_no_request_and_pushes_error() {
        // Given a session with no title (default) but all else fine.
        let mut state = AppState::default();
        state.frontend.preferences.discord.enabled = true;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        cell.update(|d| d.mark_running("discord", None));

        // When handling ToDiscordThread.
        let result = handle_to_discord_thread(&mut state, &slices);

        // Then no command is emitted — the gateway is never reached.
        assert!(result.message_names.is_empty());
        // And an error ChatEntry was pushed into the session.
        assert!(matches!(last_entry_kind(&state), ChatEntryKind::Error(_)));
    }

    #[rstest::rstest]
    fn discord_disabled_pushes_error_and_emits_nothing() {
        // Given discord is disabled.
        let (mut state, slices) = happy_state();
        state.frontend.preferences.discord.enabled = false;

        // When handling ToDiscordThread.
        let result = handle_to_discord_thread(&mut state, &slices);

        // Then no command is emitted, and an error entry is pushed.
        assert!(result.message_names.is_empty());
        assert!(matches!(last_entry_kind(&state), ChatEntryKind::Error(_)));
    }

    #[rstest::rstest]
    fn bot_not_connected_pushes_error_and_emits_nothing() {
        // Given the discord dashboard-slice entry is not Running.
        let (mut state, slices) = happy_state();
        let dashboard_slot = dashboard_slot();
        let cell = slices
            .reader::<DashboardState>(&dashboard_slot)
            .expect("seeded");
        cell.update(|d| d.mark_dead("discord", None));

        // When handling ToDiscordThread.
        let result = handle_to_discord_thread(&mut state, &slices);

        // Then no command is emitted, and an error entry is pushed.
        assert!(result.message_names.is_empty());
        assert!(matches!(last_entry_kind(&state), ChatEntryKind::Error(_)));
    }
}
