Feature: Strategy Switching
  Bus-level scenarios for switching prompt assembly strategies per session.

  Scenario: Default strategy is passthrough
    Given a fresh bus with all handlers
    Then the active strategy should be "passthrough"

  Scenario: PromptStrategySwitched event updates active strategy
    Given a fresh bus with all handlers
    When I submit a PromptStrategySwitched event with strategy "sliding_window"
    Then the active strategy should be "sliding_window"

  Scenario: Switch back to passthrough via event
    Given a fresh bus with all handlers
    When I submit a PromptStrategySwitched event with strategy "sliding_window"
    Then the active strategy should be "sliding_window"
    When I submit a PromptStrategySwitched event with strategy "passthrough"
    Then the active strategy should be "passthrough"

  Scenario: Strategy switch does not affect history
    Given a fresh bus with all handlers
    And the session is idle
    When I submit EnqueueUserMessage with text "hello"
    Then the chat history should contain 1 entry
    When I submit a PromptStrategySwitched event with strategy "sliding_window"
    Then the active strategy should be "sliding_window"
    And the chat history should contain 1 entry
