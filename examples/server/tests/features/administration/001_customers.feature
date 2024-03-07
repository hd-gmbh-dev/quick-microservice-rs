@customers
Feature: 001 -- Administrate -- create customers
  Scenario Outline: Unauthenticated user is not able to create customers
    Given Without user
    When creating Customer with the name '<name>'
    Then the response should contain the error extension code 403
    Examples:
      | name    |
      | cust001 |
      | cust002 |
      | cust003 |
      | cust004 |
  Scenario Outline: Admin user is able to create customers with initial users
    Given Admin user
    When creating Customer with the name '<name>'
    Then created Customer has name '<name>'
    And creating Customer with the name '<name>' again
    Then the response should contain the error extension code 409
    And the response should contain the error extension type 'Customer'
    And the response should contain the error extension field 'name'
    Examples:
      | name    |
      | cust001 |
      | cust002 |
      | cust003 |
      | cust004 |
