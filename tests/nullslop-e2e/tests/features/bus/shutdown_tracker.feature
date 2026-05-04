Feature: Shutdown Tracker
  Handler-level scenarios for tracking actor lifecycle during shutdown.

  Scenario: Tracks starting actor
    Given a fresh bus with all handlers
    When I submit ActorStarting with name "actor-a"
    Then the shutdown tracker should have 1 pending actor
    And the shutdown tracker pending actors should include "actor-a"

  Scenario: Completes on last shutdown
    Given a fresh bus with all handlers
    And actor "actor-a" is tracked for shutdown
    And shutdown is active
    When I submit ActorShutdownCompleted with name "actor-a"
    Then a ProceedWithShutdown command should have been submitted

  Scenario: Ignores unknown completion
    Given a fresh bus with all handlers
    And actor "actor-a" is tracked for shutdown
    And shutdown is active
    When I submit ActorShutdownCompleted with name "unknown"
    Then no ProceedWithShutdown command should have been submitted
    And the shutdown tracker should have 1 pending actor
    And the shutdown tracker pending actors should include "actor-a"

  Scenario: Not complete until shutdown is active
    Given a fresh bus with all handlers
    And actor "actor-a" is tracked for shutdown
    When I submit ActorShutdownCompleted with name "actor-a"
    Then no ProceedWithShutdown command should have been submitted
