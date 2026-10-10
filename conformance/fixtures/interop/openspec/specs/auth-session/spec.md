# Auth Session Specification

## Purpose

How long a signed-in session lasts, and what ends it.
## Requirements
### Requirement: Session expiry

The system SHALL end a session after 30 minutes without a request.

#### Scenario: Idle session

- **WHEN** a signed-in user makes no request for 30 minutes
- **THEN** the next request is answered as signed out

### Requirement: Remember me

The system SHALL keep a session for 30 days when the user asks to be remembered.

#### Scenario: Remembered device

- **WHEN** a user signs in with "remember me" ticked
- **THEN** the session survives a browser restart
