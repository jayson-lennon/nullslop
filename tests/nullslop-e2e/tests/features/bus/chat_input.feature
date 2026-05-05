Feature: Chat Input Box
  Handler-level scenarios for typing, deleting, cursor movement, submit, clear, interrupt, and mode transitions.

  Scenario: Insert char appends to buffer
    Given a fresh bus with all handlers
    When I submit InsertChar with "x"
    Then the input buffer should be "x"
    And the cursor position should be 1

  Scenario: Delete grapheme removes last
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit InsertChar with "b"
    And I submit DeleteGrapheme
    Then the input buffer should be "a"
    And the cursor position should be 1

  Scenario: Delete grapheme forward removes at cursor
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit InsertChar with "b"
    And I submit MoveCursorLeft
    And I submit DeleteGraphemeForward
    Then the input buffer should be "a"
    And the cursor position should be 1

  Scenario: Delete grapheme forward at end is noop
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit DeleteGraphemeForward
    Then the input buffer should be "a"

  Scenario: Submit message adds entry and clears buffer
    Given a fresh bus with all handlers
    And the active provider is "test"
    And the input buffer contains "hello"
    When I submit SubmitMessage
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a User message with text "hello"
    And the input buffer should be empty
    And the cursor position should be 0

  Scenario: Submit message ignores empty buffer
    Given a fresh bus with all handlers
    When I submit SubmitMessage
    Then the chat history should contain 0 entry
    And no commands should be pending
    And the input buffer should be ""

  Scenario: Submit message requests prompt assembly
    Given a fresh bus with all handlers
    And the active provider is "test"
    And the input buffer contains "hello"
    When I submit SubmitMessage
    Then an AssemblePrompt command should have been submitted

  Scenario: Clear empties buffer
    Given a fresh bus with all handlers
    And the input buffer contains "some text"
    When I submit Clear
    Then the input buffer should be empty
    And the cursor position should be 0

  Scenario: Set mode changes app mode
    Given a fresh bus with all handlers
    When I submit SetMode Input
    Then the mode should be Input

  Scenario: Move cursor left decrements position
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit InsertChar with "b"
    And I submit MoveCursorLeft
    Then the cursor position should be 1
    And the input buffer should be "ab"

  Scenario: Move cursor right increments position
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit InsertChar with "b"
    And I submit MoveCursorLeft
    And I submit MoveCursorRight
    Then the cursor position should be 2

  Scenario: Move cursor to start sets zero
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit InsertChar with "b"
    And I submit InsertChar with "c"
    And I submit MoveCursorToStart
    Then the cursor position should be 0

  Scenario: Move cursor to end sets count
    Given a fresh bus with all handlers
    When I submit InsertChar with "a"
    And I submit InsertChar with "b"
    And I submit InsertChar with "c"
    And I submit MoveCursorToStart
    And I submit MoveCursorToEnd
    Then the cursor position should be 3

  Scenario: Move cursor word left skips word
    Given a fresh bus with all handlers
    When I submit InsertChar with "h"
    And I submit InsertChar with "e"
    And I submit InsertChar with "l"
    And I submit InsertChar with "l"
    And I submit InsertChar with "o"
    And I submit InsertChar with " "
    And I submit InsertChar with "w"
    And I submit InsertChar with "o"
    And I submit InsertChar with "r"
    And I submit InsertChar with "l"
    And I submit InsertChar with "d"
    And I submit MoveCursorWordLeft
    Then the cursor position should be 6

  Scenario: Move cursor word right skips word
    Given a fresh bus with all handlers
    When I submit InsertChar with "h"
    And I submit InsertChar with "e"
    And I submit InsertChar with "l"
    And I submit InsertChar with "l"
    And I submit InsertChar with "o"
    And I submit InsertChar with " "
    And I submit InsertChar with "w"
    And I submit InsertChar with "o"
    And I submit InsertChar with "r"
    And I submit InsertChar with "l"
    And I submit InsertChar with "d"
    And I submit MoveCursorToStart
    And I submit MoveCursorWordRight
    Then the cursor position should be 6

  Scenario: Move cursor up moves to previous line
    Given a fresh bus with all handlers
    And the input buffer contains "ab\ncd"
    When I submit MoveCursorUp
    Then the cursor row should be 0 and column should be 2

  Scenario: Move cursor down moves to next line
    Given a fresh bus with all handlers
    And the input buffer contains "ab\ncd"
    When I submit MoveCursorToStart
    And I submit MoveCursorDown
    Then the cursor row should be 1 and column should be 0

  Scenario: Move cursor up noop on first line
    Given a fresh bus with all handlers
    And the input buffer contains "hello"
    When I submit MoveCursorToStart
    And I submit MoveCursorUp
    Then the cursor position should be 0

  Scenario: Move cursor down noop on last line
    Given a fresh bus with all handlers
    And the input buffer contains "hello"
    When I submit MoveCursorDown
    Then the cursor position should be 5

  Scenario: Insert newline adds to buffer
    Given a fresh bus with all handlers
    And the input buffer contains "hello"
    When I submit InsertChar with "\n"
    Then the input buffer should be "hello\n"
    And the cursor position should be 6

  Scenario: Set mode from Input to Normal cancels when not idle
    Given a fresh bus with all handlers
    And the app is in Input mode
    And the session is sending
    When I submit SetMode Normal
    Then a CancelStream command should have been submitted
    And the mode should be Normal

  Scenario: Set mode from Input to Normal no cancel when idle
    Given a fresh bus with all handlers
    And the app is in Input mode
    And the session is idle
    When I submit SetMode Normal
    Then no CancelStream command should have been submitted
    And the mode should be Normal

  Scenario: Interrupt clears buffer when non-empty
    Given a fresh bus with all handlers
    And the input buffer contains "hello"
    When I submit Interrupt
    Then the input buffer should be empty
    And the cursor position should be 0

  Scenario: Interrupt quits when buffer empty
    Given a fresh bus with all handlers
    When I submit Interrupt
    Then the app should quit
