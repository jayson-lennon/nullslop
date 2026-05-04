Feature: Headless Script Execution
  Simulating headless script execution through the keymap pipeline.

  Scenario: Running a quit script sets should_quit
    Given a new app
    When I run the headless script "q"
    Then the app should quit

  Scenario: Running an empty script makes no state changes
    Given a new app
    When I run an empty headless script
    Then the app should not quit
    And the chat history should contain 0 entry
