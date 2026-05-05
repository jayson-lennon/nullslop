Feature: Chat Log
  Handler-level scenarios for pushing chat entries, emitting events, and scrolling.

  Scenario: Push user entry adds to history
    Given a fresh bus with all handlers
    When I submit PushChatEntry with a User message "hello"
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a User message with text "hello"

  Scenario: Push entry emits ChatEntrySubmitted event
    Given a fresh bus with all handlers
    When I submit PushChatEntry with a User message "hello"
    Then a ChatEntrySubmitted event should have been submitted

  Scenario: Push actor entry adds to history
    Given a fresh bus with all handlers
    When I submit PushChatEntry with an Actor message from "echo" with text "HELLO"
    Then the chat history should contain 1 entry
    And chat history entry 1 should be an Actor message from "echo" with text "HELLO"

  Scenario: Scroll up from bottom resolves to concrete offset
    Given a fresh bus with all handlers
    And the session has 20 history entries
    And the last max offset is 100
    When I submit ScrollUp
    Then the scroll offset should be 90

  Scenario: Scroll down from known offset increments
    Given a fresh bus with all handlers
    And the session has 1 history entries
    And the last max offset is 100
    And the scroll offset is at the top
    When I submit ScrollDown
    Then the scroll offset should be 10
