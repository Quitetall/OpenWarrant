## MODIFIED Requirements

### Requirement: Session expiry

The system SHALL end a session after 30 minutes without a request, or after 15 minutes when the second factor was skipped.

#### Scenario: Skipped second factor

- **WHEN** a user signs in without the second factor
- **THEN** the session ends after 15 minutes without a request

## REMOVED Requirements

### Requirement: Remember me

**Reason**: a remembered device would skip the second factor.

**Migration**: users sign in again, with the second factor.
