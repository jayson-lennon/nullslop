Feature: Multi-turn tool loop
  The LLM actor should keep looping when it produces tool calls.
  When tool results come back, it starts a new stream. This continues
  until the LLM responds without tool calls.

  Scenario: LLM calls a tool then finishes with text
    Given a fresh actor world with the tool loop fake
    When I submit SendToLlmProvider with the tool loop trigger
    Then the chat history should contain at least 1 entries
    And the session should be idle
