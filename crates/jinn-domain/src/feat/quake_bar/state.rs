//! Quake bar state — the global overlay console.
//!
//! The payload lives in the slice's own cell (see
//! [`quake_bar_slot`]), not in `FrontendState`: the actor (log writer)
//! and the intent-handler input hook (input writer) share the one write
//! handle minted at activation, and the renderer resolves read handles
//! through the registry.

use crate::common::line_input::LineInput;
use jinn_slices::SliceScopeId;
use jinn_slices::SlotKey;

/// The quake bar's slot in the [`Slices`](jinn_slices::Slices) registry.
///
/// Canonical key shared by activation (which mints the cell), the
/// renderer (which resolves a read handle), and tests.
#[must_use]
pub fn quake_bar_slot() -> SlotKey {
    SlotKey::builtin("quake-bar", "state")
}

/// The quake bar overlay's dynamic focus scope.
///
/// Pushed by the open action's scope signal; popped by close.
#[must_use]
pub fn quake_scope() -> SliceScopeId {
    SliceScopeId::new("quake-bar", "open")
}

/// Maximum number of lines retained in the command log.
///
/// Older entries are dropped once this cap is exceeded.
const MAX_LOG_LINES: usize = 20;

/// The quake bar's command input — a single line.
///
/// Thin wrapper around [`LineInput`] today; exists to give future CLI-style
/// flag/subcommand autocompletion a dedicated home.
#[derive(Debug, Clone, Default)]
pub struct QuakeBarInput {
    /// The editable text + cursor.
    pub text: LineInput,
}

/// Scrollable, capped command log.
///
/// The view is anchored by `scrolled_up` — the number of lines scrolled up
/// *from the bottom*. `0` means pinned to the newest content. Because `0` is
/// always a valid position regardless of viewport size, every
/// [`Self::scroll_up`] press has an immediate, observable effect on the
/// rendered window — unlike a top-index model, which silently no-ops until
/// the offset crawls back into the valid viewport range.
///
/// [`Self::visible_lines`] is the only place that knows the viewport, so it
/// alone performs the per-viewport clamp at the top of the log.
#[derive(Debug, Clone, Default)]
pub struct CommandLog {
    /// Log lines, oldest first.
    lines: Vec<String>,
    /// Lines scrolled up from the bottom. `0` = pinned to newest content.
    scrolled_up: usize,
}

impl CommandLog {
    /// Appends a line, enforcing [`MAX_LOG_LINES`] and recentering on the
    /// newest content.
    ///
    /// The view snaps to the bottom regardless of where it was scrolled to,
    /// so a freshly submitted line is always visible.
    pub fn push(&mut self, line: String) {
        self.lines.push(line);
        if self.lines.len() > MAX_LOG_LINES {
            self.lines.remove(0);
        }
        // Re-pin to the newest content.
        self.scrolled_up = 0;
    }

    /// Scrolls the log one line toward the oldest content.
    ///
    /// Increments the from-bottom count. The top clamp is applied at render
    /// time by [`Self::visible_lines`], which is the only place that knows
    /// the viewport size.
    pub fn scroll_up(&mut self) {
        self.scrolled_up = self.scrolled_up.saturating_add(1);
    }

    /// Scrolls the log one line toward the newest content.
    ///
    /// Decrements the from-bottom count, clamping at `0` (pinned to bottom).
    pub fn scroll_down(&mut self) {
        self.scrolled_up = self.scrolled_up.saturating_sub(1);
    }

    /// Returns the visible slice of lines for a viewport of `viewport` rows.
    ///
    /// When the log fits entirely within the viewport, the whole thing is
    /// shown. Otherwise the window is the `viewport` rows ending
    /// `scrolled_up` lines above the newest line. The from-bottom count is
    /// clamped so the window never scrolls past the oldest line.
    #[must_use]
    pub fn visible_lines(&self, viewport: usize) -> &[String] {
        if self.lines.len() <= viewport {
            return &self.lines;
        }
        let len = self.lines.len();
        let k = self.scrolled_up.min(len - viewport);
        let start = len - viewport - k;
        let end = len - k;
        self.lines.get(start..end).unwrap_or(&self.lines)
    }

    /// Returns the number of stored lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Returns `true` if no lines are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Aggregate quake bar state — the slice's cell payload.
///
/// Two writers, two fields — never cross the streams:
/// - [`QuakeBarState::input`] — written ONLY by the intent-handler
///   input hook (synchronous char editing, via the shared cell handle).
/// - [`QuakeBarState::log`] — written ONLY by the [`QuakeBarActor`]
///   (the command log; submit routes through
///   [`SubmitQuakeBarCommand`](super::command::SubmitQuakeBarCommand) so the
///   actor is the single mutator).
///
/// Both writers hold clones of the one handle minted at activation —
/// the handle is singular per slice even though two clones exist.
#[derive(Debug, Clone, Default)]
pub struct QuakeBarState {
    /// The 1-line command input. OWNER: IntentHandler (input hook).
    pub input: QuakeBarInput,
    /// The persistent command log. OWNER: QuakeBarActor.
    pub log: CommandLog,
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code"
    )]

    use super::*;

    #[rstest::rstest]
    #[test]
    fn push_appends_a_line_to_the_log() {
        // Given an empty command log.
        let mut log = CommandLog::default();

        // When pushing a line.
        log.push("hello".to_owned());

        // Then the log holds that line.
        assert_eq!(log.len(), 1);
        assert_eq!(log.visible_lines(5), &["hello".to_owned()]);
    }

    #[rstest::rstest]
    #[test]
    fn push_beyond_cap_drops_oldest_line() {
        // Given a log filled to the cap.
        let mut log = CommandLog::default();
        for i in 0..MAX_LOG_LINES {
            log.push(format!("line-{i}"));
        }

        // When pushing one more line.
        log.push("overflow".to_owned());

        // Then the oldest line is dropped and the newest is present.
        assert_eq!(log.len(), MAX_LOG_LINES);
        assert_eq!(log.visible_lines(MAX_LOG_LINES)[0], "line-1");
        assert_eq!(
            log.visible_lines(MAX_LOG_LINES)[MAX_LOG_LINES - 1],
            "overflow"
        );
    }

    #[rstest::rstest]
    #[test]
    fn push_recenters_view_on_newest_line() {
        // Given a log scrolled away from the bottom.
        let mut log = CommandLog::default();
        for i in 0..10 {
            log.push(format!("line-{i}"));
        }
        log.scroll_up();
        log.scroll_up();

        // When pushing a new line.
        log.push("new".to_owned());

        // Then the visible window ends at the new line (pinned to the bottom).
        let visible = log.visible_lines(3);
        assert_eq!(visible[visible.len() - 1], "new");
    }

    #[rstest::rstest]
    #[test]
    fn scroll_up_reveals_an_older_line_immediately() {
        // Given a log larger than the viewport, pinned to the bottom.
        let mut log = CommandLog::default();
        for i in 0..10 {
            log.push(format!("line-{i}"));
        }
        // Snapshot the window before scrolling.
        let before = log.visible_lines(3).to_vec();

        // When scrolling up once.
        log.scroll_up();

        // Then the visible window shifts to reveal a different (older) line.
        let after = log.visible_lines(3);
        assert_ne!(
            after,
            before.as_slice(),
            "scroll_up must change the visible window"
        );
        // And the newest line is no longer at the bottom edge.
        assert_ne!(
            after[after.len() - 1],
            "line-9",
            "scroll_up must move the bottom edge off the newest line"
        );
    }

    #[rstest::rstest]
    #[test]
    fn scroll_up_clamps_at_top_of_log() {
        // Given a log scrolled all the way up (viewport smaller than log).
        let mut log = CommandLog::default();
        for i in 0..5 {
            log.push(format!("line-{i}"));
        }
        for _ in 0..20 {
            log.scroll_up();
        }

        // When scrolling up further.
        log.scroll_up();

        // Then the window starts at the first line (oldest).
        let visible = log.visible_lines(2);
        assert_eq!(visible[0], "line-0");
    }

    #[rstest::rstest]
    #[test]
    fn scroll_down_returns_toward_newest_line() {
        // Given a log scrolled up a couple of lines from the bottom.
        let mut log = CommandLog::default();
        for i in 0..10 {
            log.push(format!("line-{i}"));
        }
        log.scroll_up();
        log.scroll_up();
        let before = log.visible_lines(3).to_vec();

        // When scrolling down once.
        log.scroll_down();

        // Then the visible window shifts toward the newest line.
        let after = log.visible_lines(3);
        assert_ne!(
            after,
            before.as_slice(),
            "scroll_down must change the visible window"
        );
    }

    #[rstest::rstest]
    #[test]
    fn scroll_down_clamps_at_bottom_of_log() {
        // Given a log pinned to the bottom.
        let mut log = CommandLog::default();
        for i in 0..10 {
            log.push(format!("line-{i}"));
        }
        let before = log.visible_lines(3).to_vec();

        // When scrolling down past the bottom.
        for _ in 0..20 {
            log.scroll_down();
        }

        // Then the window is unchanged (still pinned at the newest line).
        assert_eq!(log.visible_lines(3), before.as_slice());
    }

    #[rstest::rstest]
    #[test]
    fn visible_lines_returns_full_log_when_smaller_than_viewport() {
        // Given a 3-line log.
        let mut log = CommandLog::default();
        for c in ['a', 'b', 'c'] {
            log.push(c.to_string());
        }

        // When asking for a viewport larger than the log.
        let visible = log.visible_lines(10);

        // Then all lines are returned.
        assert_eq!(visible, &["a".to_owned(), "b".to_owned(), "c".to_owned()]);
    }

    #[rstest::rstest]
    #[test]
    fn visible_lines_returns_tail_when_larger_than_viewport() {
        // Given a 5-line log scrolled to the bottom.
        let mut log = CommandLog::default();
        for c in ['a', 'b', 'c', 'd', 'e'] {
            log.push(c.to_string());
        }

        // When asking for a viewport of 2.
        let visible = log.visible_lines(2);

        // Then the two newest lines are returned.
        assert_eq!(visible, &["d".to_owned(), "e".to_owned()]);
    }

    #[rstest::rstest]
    #[test]
    fn scroll_up_then_down_returns_to_original_window() {
        // Given a log pinned to the bottom.
        let mut log = CommandLog::default();
        for i in 0..10 {
            log.push(format!("line-{i}"));
        }
        let original = log.visible_lines(3).to_vec();

        // When scrolling up twice then down twice.
        log.scroll_up();
        log.scroll_up();
        log.scroll_down();
        log.scroll_down();

        // Then the window returns to the original (bottom) state.
        assert_eq!(log.visible_lines(3), original.as_slice());
    }
}
