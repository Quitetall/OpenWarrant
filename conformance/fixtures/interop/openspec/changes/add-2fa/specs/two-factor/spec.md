## ADDED Requirements

### Requirement: TOTP second factor

The system SHALL ask an enrolled user for a valid time-based one-time code after the password.

#### Scenario: Valid code

- **WHEN** an enrolled user enters the password and a valid code
- **THEN** the user is signed in

#### Scenario: Wrong code

- **WHEN** an enrolled user enters a wrong code
- **THEN** the user is not signed in
