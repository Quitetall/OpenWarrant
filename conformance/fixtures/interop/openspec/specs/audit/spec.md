# Audit Specification

## Purpose

What the system records about who did what.
## Requirements
### Requirement: Audit trail

The system SHALL record every sign-in, with the account and the time.

#### Scenario: Sign-in recorded

- **WHEN** a user signs in
- **THEN** one entry names the account and the time
