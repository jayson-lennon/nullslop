Feature: Message Queue
  Handler-level scenarios for message queue lifecycle: enqueue, dispatch, stream completion, and cancel.

  Scenario: Enqueue when idle dispatches immediately
    Given a fresh bus with all handlers
    And the active provider is "test"
    And the session is idle
    When I submit EnqueueUserMessage with text "hello"
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a User message with text "hello"
    And the session should be sending
    And a SendToLlmProvider command should have been submitted

  Scenario: Enqueue when busy queues message
    Given a fresh bus with all handlers
    And the session is sending
    When I submit EnqueueUserMessage with text "queued msg"
    Then the chat history should contain 0 entry
    And the message queue should contain 1 message
    And message queue entry 1 should be "queued msg"

  Scenario: Stream completed dispatches next from queue
    Given a fresh bus with all handlers
    And the active provider is "test"
    And the session is sending
    And the session has queued message "next msg"
    When I submit StreamCompleted with reason Finished
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a User message with text "next msg"
    And the message queue should be empty
    And the session should be sending
    And a SendToLlmProvider command should have been submitted

  Scenario: Stream completed canceled does not dispatch
    Given a fresh bus with all handlers
    And the session is idle
    When I submit StreamCompleted with reason Canceled
    Then the chat history should contain 0 entry
    And no commands should be pending

  Scenario: Cancel stream drains queue and restores input
    Given a fresh bus with all handlers
    And the session is streaming
    And the session has queued message "first"
    And the session has queued message "second"
    When I submit CancelStream
    Then the message queue should be empty
    And the session should not be streaming
    And a SetChatInputText command should have been submitted with text "first\nsecond"

  Scenario: Stream completed dispatches all queued messages at once
    Given a fresh bus with all handlers
    And the active provider is "test"
    And the session is sending
    And the session has queued message "msg 1"
    And the session has queued message "msg 2"
    And the session has queued message "msg 3"
    When I submit StreamCompleted with reason Finished
    Then the chat history should contain 3 entry
    And chat history entry 1 should be a User message with text "msg 1"
    And chat history entry 2 should be a User message with text "msg 2"
    And chat history entry 3 should be a User message with text "msg 3"
    And the message queue should be empty
    And the session should be sending
    And exactly 1 SendToLlmProvider command should have been submitted

  Scenario: Stream completed with empty queue does not dispatch
    Given a fresh bus with all handlers
    And the session is sending
    When I submit StreamCompleted with reason Finished
    Then the chat history should contain 0 entry
    And the session should be idle
    And no commands should be pending

  Scenario: Set chat input text replaces input buffer
    Given a fresh bus with all handlers
    And the session is idle
    When I submit SetChatInputText with "restored text"
    Then the input buffer should be "restored text"

  Scenario: Enqueue with no provider dispatches to LLM
    Given a fresh bus with all handlers
    And the session is idle
    When I submit EnqueueUserMessage with text "hello"
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a User message with text "hello"
    And the session should be sending
    And a SendToLlmProvider command should have been submitted
