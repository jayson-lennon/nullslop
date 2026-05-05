Feature: Dashboard Selection
  Handler-level scenarios for navigating the dashboard actor list with j/k and gg/G.

  Scenario: Select down moves to next actor
    Given a fresh bus with all handlers
    And the dashboard has actors "echo" and "llm"
    When I submit DashboardSelectDown
    Then the dashboard selected index should be 1

  Scenario: Select down clamps at last actor
    Given a fresh bus with all handlers
    And the dashboard has actors "echo" and "llm"
    When I submit DashboardSelectDown
    And I submit DashboardSelectDown
    Then the dashboard selected index should be 1

  Scenario: Select up moves to previous actor
    Given a fresh bus with all handlers
    And the dashboard has actors "echo", "llm", and "ctx"
    When I submit DashboardSelectDown
    And I submit DashboardSelectDown
    And I submit DashboardSelectUp
    Then the dashboard selected index should be 1

  Scenario: Select up clamps at first actor
    Given a fresh bus with all handlers
    And the dashboard has actors "echo" and "llm"
    When I submit DashboardSelectUp
    Then the dashboard selected index should be 0

  Scenario: Select first jumps to index zero
    Given a fresh bus with all handlers
    And the dashboard has actors "echo", "llm", and "ctx"
    When I submit DashboardSelectDown
    And I submit DashboardSelectDown
    And I submit DashboardSelectFirst
    Then the dashboard selected index should be 0

  Scenario: Select last jumps to last actor
    Given a fresh bus with all handlers
    And the dashboard has actors "echo", "llm", and "ctx"
    When I submit DashboardSelectLast
    Then the dashboard selected index should be 2

  Scenario: Select down noop with no actors
    Given a fresh bus with all handlers
    When I submit DashboardSelectDown
    Then the dashboard selected index should be 0

  Scenario: Select first noop with no actors
    Given a fresh bus with all handlers
    When I submit DashboardSelectFirst
    Then the dashboard selected index should be 0

  Scenario: Select last noop with no actors
    Given a fresh bus with all handlers
    When I submit DashboardSelectLast
    Then the dashboard selected index should be 0
