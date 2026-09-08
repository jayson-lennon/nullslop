#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test file, panics are acceptable"
)]

use super::render::*;
use jinn_domain::FocusScope;
use jinn_domain::feat::session::chat_entry::ChatEntry;
use jinn_domain::feat::ui::chat_log::GUTTER_WIDTH;
use jinn_selection_widget::compute_popup_rect;
use jinn_testutil::setup_term;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Creates a minimal `TuiApp` for render testing.
async fn render_test_app() -> crate::TuiApp {
    crate::TuiApp::test_builder().build().await
}

#[rstest::rstest]
#[tokio::test]
async fn render_registers_content_rect_for_selectable_chat_log() {
    // Given a TuiApp rendered in Chat tab with a 80x24 terminal.

    let mut app = render_test_app().await;
    // Default tab is Chat.

    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the chat area rect is registered as selectable, excluding the gutter.
    // Chat log is selectable - the selectable area starts after the gutter column.
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let content = layout.content;
    let expected = Rect {
        x: content.x + GUTTER_WIDTH,
        y: content.y,
        width: content.width.saturating_sub(GUTTER_WIDTH),
        height: content.height,
    };
    let found = app
        .selectable_rects
        .find_for_position(expected.x + 1, expected.y + 1);
    assert!(
        found.is_some(),
        "chat log content rect should be selectable"
    );
    assert_eq!(found.unwrap(), expected);
}

#[rstest::rstest]
#[tokio::test]
async fn picker_popup_rect_is_selectable() {
    // Given a TuiApp rendered with Mode::Picker.

    let mut app = render_test_app().await;
    // Switch to Picker mode with an active provider picker.
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .push(jinn_domain::FocusScope::Picker {
            kind: jinn_domain::PickerKind::Provider,
        });

    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the picker popup rect is registered as selectable.
    let popup_rect = compute_popup_rect(Rect::new(0, 0, 80, 24));
    // Query position inside popup but outside the content area (popup extends
    // further right than the content column which ends at the border).
    let outside_content_x = popup_rect.x + popup_rect.width.saturating_sub(5);
    let found = app.selectable_rects.find_for_position(outside_content_x, 0);
    assert!(found.is_some(), "picker popup rect should be selectable");
    assert_eq!(found.unwrap(), popup_rect);
}

#[rstest::rstest]
#[tokio::test]
async fn content_area_rect_is_selectable() {
    // Given a TuiApp rendered with Mode::Picker.

    let mut app = render_test_app().await;
    // Switch to Picker mode with an active provider picker.
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .push(jinn_domain::FocusScope::Picker {
            kind: jinn_domain::PickerKind::Provider,
        });

    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the content area rect is also still selectable (chat-log is selectable).
    // Query a position inside the gutter-excluded selectable rect.
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let content = layout.content;
    let select_x = content.x + GUTTER_WIDTH + 1;
    let content_found = app
        .selectable_rects
        .find_for_position(select_x, content.y + 1);
    assert!(
        content_found.is_some(),
        "content rect should also be selectable alongside picker"
    );
}

/// Helper to create a Rect matching the terminal dimensions.
fn frame_area(w: u16, h: u16) -> Rect {
    Rect::new(0, 0, w, h)
}

/// Helper to find the minimap arrow cell position.
///
/// The arrow renders at the rightmost column of the chat_log_area at the
/// midpoint row (chat_log_height / 2). The chat_log_area is the content area
/// minus 2 bottom lines.
fn arrow_cell_position(layout: &AppLayout) -> (u16, u16) {
    let bottom_lines: u16 = 2;
    let chat_log_height = layout.content.height.saturating_sub(bottom_lines);
    let midpoint = chat_log_height / 2;
    let x = layout.content.x + layout.content.width.saturating_sub(1);
    let y = layout.content.y + midpoint;
    (x, y)
}

#[rstest::rstest]
#[tokio::test]
async fn minimap_arrow_is_yellow_when_normal_scope() {
    // Given a TuiApp rendered with Normal scope and one chat entry.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .clear_overlays();
    app.core
        .state
        .write_test_no_cap()
        .active_session_mut()
        .push_entry(ChatEntry::user("hello"));
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the minimap arrow is Yellow (focus_accent).
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let (x, y) = arrow_cell_position(&layout);
    let buffer = terminal.backend().buffer();
    let cell = buffer.cell((x, y)).expect("minimap arrow cell");
    assert_eq!(cell.symbol(), ">");
    assert_eq!(cell.fg, Color::Yellow);
}

#[rstest::rstest]
#[tokio::test]
async fn minimap_arrow_is_darkgray_when_input_scope() {
    // Given a TuiApp rendered with Input scope and one chat entry.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .push(FocusScope::Input);
    app.core
        .state
        .write_test_no_cap()
        .active_session_mut()
        .push_entry(ChatEntry::user("hello"));
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the minimap arrow is DarkGray (border_unfocused).
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let (x, y) = arrow_cell_position(&layout);
    let buffer = terminal.backend().buffer();
    let cell = buffer.cell((x, y)).expect("minimap arrow cell");
    assert_eq!(cell.fg, Color::DarkGray);
}

#[rstest::rstest]
#[tokio::test]
async fn gutter_area_is_not_selectable() {
    // Given a TuiApp rendered in Chat tab with a 80x24 terminal.
    let mut app = render_test_app().await;
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then clicking in the gutter (first column of content area) is not selectable.
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let content = layout.content;
    let found = app
        .selectable_rects
        .find_for_position(content.x, content.y + 1);
    assert!(found.is_none(), "gutter area should not be selectable");
}

#[rstest::rstest]
#[tokio::test]
async fn cwd_input_popup_renders_and_is_selectable() {
    // Given a TuiApp rendered with CwdInput scope.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .push(FocusScope::CwdInput);
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the cwd popup rect is registered as selectable.
    let popup_rect = jinn_domain::feat::cwd_input::render::cwd_input_popup_rect(frame_area(80, 24));
    let probe = app
        .selectable_rects
        .find_for_position(popup_rect.x + 1, popup_rect.y + 1);
    assert!(probe.is_some(), "cwd input popup rect should be selectable");
    assert_eq!(probe.unwrap(), popup_rect);
}

/// Column index of the chat-mode vertical border for an 80-wide terminal
/// with the default sidebar width (30). main(48) | minimap(1) | border(1) | sidebar(30).
const CHAT_BORDER_X_80: u16 = 49;

/// Enters the Dashboard tab by swapping the base scope, then renders once,
/// returning the terminal so the test can inspect its buffer.
async fn render_in_dashboard(
    width: u16,
    height: u16,
) -> ratatui::Terminal<ratatui::backend::TestBackend> {
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    let (mut terminal, _area) = setup_term(width, height);
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();
    terminal
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_renders_no_vertical_border_or_sidebar_gap() {
    // Given a TuiApp rendered in the Dashboard tab.
    let terminal = render_in_dashboard(80, 24).await;

    // When inspecting the column where the chat layout draws the sidebar border.
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let buffer = terminal.backend().buffer();

    // Then the border glyph (│) is absent at the chat border column for
    // every content row — the dashboard owns the full width.
    for y in layout.content.y..(layout.content.y + layout.content.height) {
        let cell = buffer.cell((CHAT_BORDER_X_80, y)).expect("content cell");
        assert_ne!(
            cell.symbol(),
            "\u{2502}",
            "dashboard must not draw the chat sidebar border at column {CHAT_BORDER_X_80}",
        );
    }
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_renders_no_status_bar() {
    // Given a TuiApp rendered in the Dashboard tab.
    let terminal = render_in_dashboard(80, 24).await;
    let buffer = terminal.backend().buffer();

    // When scanning every cell for the status bar's signature glyphs.
    let width = 80;
    let height = 24;
    let status_bar_glyphs = ["\u{21BB}", "\u{2191}", "\u{2193}"];
    let mut found: Vec<String> = vec![];
    for y in 0..height {
        for x in 0..width {
            let sym = buffer.cell((x, y)).expect("cell").symbol();
            if status_bar_glyphs.contains(&sym) {
                found.push(format!("({x},{y})={sym}"));
            }
        }
    }

    // Then none of the status bar glyphs appear anywhere on the dashboard.
    assert!(
        found.is_empty(),
        "status bar glyphs found in dashboard: {}",
        found.join(", ")
    );
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_content_fills_full_width() {
    // Given a TuiApp rendered in the Dashboard tab.
    let terminal = render_in_dashboard(80, 24).await;
    let buffer = terminal.backend().buffer();

    // When reading the rightmost column of the tab-bar row.
    // The tab bar renders "Chat" and "Dashboard" labels; the highlighted
    // "Dashboard" tab must reach the rightmost column (no sidebar reserved).
    let rightmost = buffer.cell((79, 0)).expect("rightmost tab-bar cell");

    // Then the rightmost column is the default background reset cell, confirming
    // the tab bar spans the full width (a status-bar glyph or sidebar content
    // would instead occupy it).
    assert_eq!(
        rightmost.symbol(),
        " ",
        "rightmost column should be reset/blank, not sidebar or border content",
    );
}

/// Writes into the dashboard slice cell through the app registry.
fn write_dashboard(app: &crate::TuiApp, f: impl FnOnce(&mut jinn_domain::feat::dashboard::DashboardState)) {
    let cell: jinn_domain::common::slices::TypedCell<jinn_domain::feat::dashboard::DashboardState> = app
        .services
        .slices
        .reader(&jinn_domain::feat::dashboard::dashboard_slot())
        .expect("test builder registers the dashboard slot");
    cell.update(f);
}

/// Collects the entire terminal buffer into a single string for substring
/// assertions.
fn buffer_string(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_tab_shows_actor_name_and_lifecycle() {
    // Given a dashboard with one running actor.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    write_dashboard(&app, |d| {
        d.mark_running("discord", Some("Discord bot".to_owned()));
    });
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal.draw(|frame| app.render(frame)).unwrap();

    // Then the buffer contains "discord" and "Running".
    let buf_str = buffer_string(&terminal);
    assert!(buf_str.contains("discord"), "dashboard should show name");
    // And the lifecycle column reads "Running".
    assert!(
        buf_str.contains("Running"),
        "dashboard should show lifecycle"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_tab_shows_status_message_for_discord() {
    // Given a dashboard with discord in a connected state.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    write_dashboard(&app, |d| {
        d.mark_running("discord", None);
        d.set_status_message("discord", Some("Connected".to_owned()));
    });
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal.draw(|frame| app.render(frame)).unwrap();

    // Then the buffer contains "Connected".
    let buf_str = buffer_string(&terminal);
    assert!(
        buf_str.contains("Connected"),
        "dashboard should show status message"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_tab_shows_empty_placeholder_when_no_actors() {
    // Given a dashboard cell with no actor rows.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    write_dashboard(&app, jinn_domain::feat::dashboard::DashboardState::clear);
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal.draw(|frame| app.render(frame)).unwrap();

    // Then the buffer contains the placeholder.
    let buf_str = buffer_string(&terminal);
    assert!(
        buf_str.contains("No services"),
        "empty dashboard should show placeholder"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_tab_shows_selection_marker_on_selected_entry() {
    // Given a dashboard with two actors, second selected.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    write_dashboard(&app, |d| {
        d.mark_running("alpha", None);
        d.mark_running("beta", None);
        d.select_next(); // select beta (index 1)
    });
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal.draw(|frame| app.render(frame)).unwrap();

    // Then the buffer contains the selection marker ▸.
    let buf_str = buffer_string(&terminal);
    assert!(buf_str.contains('▸'), "selected entry should have marker");
}

#[rstest::rstest]
#[tokio::test]
async fn dashboard_tab_has_no_em_dash_separator() {
    // Given a dashboard with an actor that has a description.
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    write_dashboard(&app, |d| {
        d.mark_running("discord", Some("Discord bot".to_owned()));
    });
    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal.draw(|frame| app.render(frame)).unwrap();

    // Then the buffer contains no em-dash characters.
    let buf_str = buffer_string(&terminal);
    assert!(
        !buf_str.contains('\u{2014}'),
        "dashboard should not contain em-dashes"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn chat_layout_still_draws_vertical_border_for_sidebar() {
    // Given a TuiApp rendered in the default Chat tab (sidebar width 30).
    let mut app = render_test_app().await;
    let (mut terminal, _area) = setup_term(80, 24);
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // When reading the cell at the chat border column on a content row.
    let layout = AppLayout::new(frame_area(80, 24), 1, 12, 30);
    let buffer = terminal.backend().buffer();
    let cell = buffer
        .cell((layout.border.x, layout.content.y + 1))
        .expect("chat border cell");

    // Then the vertical border glyph (│) is drawn — chat rendering is unchanged.
    assert_eq!(
        cell.symbol(),
        "\u{2502}",
        "chat tab must still render the sidebar border (regression guard)",
    );
}

#[rstest::rstest]
#[tokio::test]
async fn mcp_inspector_renders_server_list_and_logs_pane() {
    // Given the MCP server inspector open with one server selected + running.
    let mut app = render_test_app().await;
    {
        use jinn_domain::feat::mcp::picker_entry::McpServerEntry;
        use jinn_domain::feat::mcp_actor::protocol::McpConnectionStatus;
        use jinn_domain::feat::theme::default_theme;
        use jinn_domain::feat::ui::picker_states::PickerExt;
        let mut w = app.core.state.write_test_no_cap();
        // Seed the active session's live data sources so the per-frame refresh
        // produces the right preview.
        w.active_session_mut()
            .set_mcp_server_status("excalimate", McpConnectionStatus::Running);
        w.active_session_mut()
            .set_mcp_server_stderr("excalimate", "hello from stderr".to_owned());
        let entry = McpServerEntry::new(
            "excalimate".to_owned(),
            "npx @excalimate/mcp-server".to_owned(),
            true,
            default_theme(),
        );
        w.frontend.mcp_server_picker_mut().set_items(vec![entry]);
        w.frontend
            .scope_stack
            .push(jinn_domain::FocusScope::Picker {
                kind: jinn_domain::PickerKind::McpServer,
            });
    }

    let (mut terminal, _area) = setup_term(100, 30);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the buffer mentions the server name, the logs badge, and the stderr tail.
    let buf = terminal.backend().buffer();
    let rendered: String = buf
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        rendered.contains("excalimate"),
        "server list shows the server name"
    );
    assert!(
        rendered.contains("running"),
        "logs pane shows the status badge"
    );
    assert!(
        rendered.contains("hello from stderr"),
        "logs pane shows the stderr tail"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn mcp_inspector_tools_pane_renders_tool_names() {
    // Given the MCP inspector open in Tools mode with one advertised tool.
    let mut app = render_test_app().await;
    {
        use jinn_domain::feat::mcp::picker_entry::{McpPreviewMode, McpServerEntry};
        use jinn_domain::feat::theme::default_theme;
        use jinn_domain::feat::ui::picker_states::PickerExt;
        let mut w = app.core.state.write_test_no_cap();
        // Seed a tool definition so the per-frame refresh surfaces it in tools mode.
        let session_id = w.active_session().session_id().clone();
        let mut defs = std::collections::BTreeMap::new();
        defs.insert(
            "mcp__excalimate__create_scene".to_owned(),
            jinn_domain::ToolDefinition {
                name: "mcp__excalimate__create_scene".to_owned(),
                description: "Create a scene".to_owned(),
                parameters: serde_json::Value::Object(serde_json::Map::new()),
                prompt_snippet: None,
                prompt_guidelines: Vec::new(),
                server_tool_type: None,
            },
        );
        w.context.session_tool_definitions.insert(session_id, defs);
        // Entry starts in Logs mode; flip to Tools so the rendered pane shows tools.
        let mut entry = McpServerEntry::new(
            "excalimate".to_owned(),
            "npx @excalimate/mcp-server".to_owned(),
            true,
            default_theme(),
        );
        entry.preview_mode = McpPreviewMode::Tools;
        w.frontend.mcp_server_picker_mut().set_items(vec![entry]);
        w.frontend
            .scope_stack
            .push(jinn_domain::FocusScope::Picker {
                kind: jinn_domain::PickerKind::McpServer,
            });
    }

    let (mut terminal, _area) = setup_term(100, 30);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the tools pane shows the advertised tool name.
    let buf = terminal.backend().buffer();
    let rendered: String = buf
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        rendered.contains("create_scene"),
        "tools pane shows the tool name"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn which_key_help_renders_above_the_terminal_overlay() {
    // Given an app with the terminal overlay open in view mode and the
    // which-key help activated (as if `?` had been pressed).
    let mut app = render_test_app().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(jinn_domain::FocusScope::TerminalView);
    app.which_key.active = true;

    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the help popup's title survives — the overlay did not paint
    // over it.
    let buf = terminal.backend().buffer();
    let rendered: String = buf
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        rendered.contains("Shortcuts"),
        "which-key help must render above the terminal overlay, got: {rendered}"
    );
}

#[rstest::rstest]
#[tokio::test]
async fn which_key_help_renders_in_base_scopes() {
    // Given a plain chat-scope app with the which-key help activated.
    let mut app = render_test_app().await;
    app.which_key.active = true;

    let (mut terminal, _area) = setup_term(80, 24);

    // When rendering.
    terminal
        .draw(|frame| {
            app.render(frame);
        })
        .unwrap();

    // Then the help popup still renders (the reordering regressed nothing).
    let buf = terminal.backend().buffer();
    let rendered: String = buf
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        rendered.contains("Shortcuts"),
        "which-key help must render in base scopes"
    );
}
