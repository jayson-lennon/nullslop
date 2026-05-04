Feature: TUI Application
  End-to-end scenarios exercising keystroke → keymap → command → bus → handler → state.

  Scenario: App starts in Normal mode
    Given a new app
    Then the which-key scope should be Normal
    And which-key should be inactive

  Scenario: Pressing 'i' in Normal mode enters Input mode
    Given a new app
    When the user presses i
    Then the mode should be Input

  Scenario: Pressing 'q' in Normal mode quits
    Given a new app
    When the user presses q
    Then the app should quit

  Scenario: Submitting a message from Input mode
    Given a new app
    And the app is in Input mode
    And the input buffer contains "hello"
    And the active provider is set
    When the user presses enter
    Then the chat history should contain 1 entry
    And the input buffer should be empty
    And the chat history entry 1 should be a user message with text "hello"

  Scenario: Pressing Esc in Input mode returns to Normal
    Given a new app
    And the app is in Input mode
    When the user presses esc
    Then the mode should be Normal

  Scenario: Toggle which-key popup
    Given a new app
    When the app routes the ToggleWhichKey command
    Then which-key should be active

  Scenario: Pushing a chat entry from an actor
    Given a new app
    When the app routes the PushChatEntry command with an actor message from "nullslop-echo" with text "HELLO"
    Then the chat history should contain 1 entry
    And the chat history entry 1 should be an actor message from "nullslop-echo" with text "HELLO"

  Scenario: Shift+Enter inserts a newline in Input mode
    Given a new app
    And the app is in Input mode
    And the input buffer contains "hello"
    When the user presses enter with shift
    Then the input buffer should be "hello\n"
    And the chat history should contain 0 entry

  Scenario: Ctrl+Enter inserts a newline in Input mode
    Given a new app
    And the app is in Input mode
    And the input buffer contains "hello"
    When the user presses enter with ctrl
    Then the input buffer should be "hello\n"
    And the chat history should contain 0 entry
