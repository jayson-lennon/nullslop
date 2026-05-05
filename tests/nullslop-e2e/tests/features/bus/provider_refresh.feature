Feature: Provider Refresh
  Handler-level scenarios for model refresh: command handling, cache loading, and summary messages.

  Scenario: RefreshModels pushes system message to active session
    Given a fresh bus with all handlers
    When I submit RefreshModels
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a System message with text "Refreshing model list..."

  Scenario: ModelsRefreshed sets last refreshed timestamp
    Given a fresh bus with all handlers
    When I submit a ModelsRefreshed event with no results or errors
    Then the last refreshed at timestamp should be set

  Scenario: ModelsRefreshed posts summary message
    Given a fresh bus with all handlers
    When I submit a ModelsRefreshed event with 2 providers and 3 models
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a System message with text "Models refreshed: 2 providers, 3 models"

  Scenario: ModelsRefreshed includes errors in summary
    Given a fresh bus with all handlers
    When I submit a ModelsRefreshed event with 1 provider and 1 model and errors
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a System message with text "Models refreshed: 1 providers, 1 models (errors: lmstudio)"

  Scenario: ModelsRefreshed shows no models when empty
    Given a fresh bus with all handlers
    When I submit a ModelsRefreshed event with no results or errors
    Then the chat history should contain 1 entry
    And chat history entry 1 should be a System message with text "No models discovered."
