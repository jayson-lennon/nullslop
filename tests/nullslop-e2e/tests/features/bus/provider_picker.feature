Feature: Provider Picker
  Handler-level scenarios for filter input, navigation, confirmation, and provider switching.

  Scenario: Insert char updates picker filter
    Given a fresh bus with all handlers
    When I submit PickerInsertChar with "o"
    Then the picker filter should be "o"

  Scenario: Backspace removes from filter
    Given a fresh bus with all handlers
    When I submit PickerInsertChar with "o"
    And I submit PickerInsertChar with "l"
    And I submit PickerBackspace
    Then the picker filter should be "o"

  Scenario: Move up decrements selection
    Given a fresh bus with all handlers
    And services with an ollama provider
    And the picker selection is 1
    When I submit PickerMoveUp
    Then the picker selection should be 0

  Scenario: Move down increments selection
    Given a fresh bus with all handlers
    And services with an ollama provider
    When I submit PickerMoveDown
    Then the picker selection should be 0

  Scenario: Confirm submits provider switch and closes
    Given a fresh bus with all handlers
    And services with an ollama provider
    And the app is in Picker mode
    When I submit PickerConfirm
    Then the active provider should be "ollama/llama3"
    And the mode should be Normal

  Scenario: Confirm ignores unavailable provider
    Given a fresh bus with all handlers
    And services with an unavailable provider
    And the app is in Picker mode
    When I submit PickerConfirm
    Then the active provider should be "__no_provider__"
    And the mode should be Picker

  Scenario: Move cursor left decrements cursor
    Given a fresh bus with all handlers
    When I submit PickerInsertChar with "a"
    And I submit PickerInsertChar with "b"
    And I submit PickerMoveCursorLeft
    Then the picker cursor position should be 1

  Scenario: Move cursor right increments cursor
    Given a fresh bus with all handlers
    When I submit PickerInsertChar with "a"
    And I submit PickerInsertChar with "b"
    And I submit PickerMoveCursorLeft
    And I submit PickerMoveCursorLeft
    And I submit PickerMoveCursorRight
    Then the picker cursor position should be 1
