# Proposal

## Why

An account protected by a password alone is taken over when the password leaks. A second factor stops that.

## What Changes

- Add a time-based one-time code (TOTP) as a second factor, enrolled from the account page.
- Shorten a session to 15 minutes when the second factor is skipped.
- Remove "remember me": a remembered device would skip the second factor.

## Capabilities

### New Capabilities

- `two-factor`: enrolment and checking of a TOTP second factor.

### Modified Capabilities

- `auth-session`: shorter sessions without a second factor; no remembered sessions.

## Impact

- Login flow and the account page.
- No new dependency beyond a TOTP library.
