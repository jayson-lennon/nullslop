Feature: Provider Switch
  Handler-level scenarios for switching the active LLM provider.

  Scenario: Valid switch updates active provider
    Given a fresh bus with all handlers
    And services with an ollama provider
    When I submit ProviderSwitch with provider "ollama/llama3"
    Then the active provider should be "ollama/llama3"

  Scenario: Valid switch emits ProviderSwitched event
    Given a fresh bus with all handlers
    And services with an ollama provider
    When I submit ProviderSwitch with provider "ollama/llama3"
    Then a ProviderSwitched event should have been submitted

  Scenario: Rejects unknown provider
    Given a fresh bus with all handlers
    When I submit ProviderSwitch with provider "nonexistent"
    Then the active provider should be "__no_provider__"
    And the chat history should contain 1 entry
    And chat history entry 1 should be a System message

  Scenario: Rejects unavailable provider
    Given a fresh bus with all handlers
    And services with an unavailable provider
    When I submit ProviderSwitch with provider "openrouter/gpt-4"
    Then the active provider should be "__no_provider__"
    And the chat history should contain 1 entry
    And chat history entry 1 should be a System message

  Scenario: Handles remote model not in static registry
    Given a fresh bus with all handlers
    And services with an ollama provider
    When I submit ProviderSwitch with provider "ollama/mistral"
    Then the active provider should be "ollama/mistral"

  Scenario: Rejects unknown remote provider
    Given a fresh bus with all handlers
    When I submit ProviderSwitch with provider "nonexistent/some-model"
    Then the active provider should be "__no_provider__"
    And the chat history should contain 1 entry
    And chat history entry 1 should be a System message
