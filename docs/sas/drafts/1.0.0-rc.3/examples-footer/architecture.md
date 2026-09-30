<!-- ow:unit context binding -->
# Keep verification before account activation

## Decision context

This illustrative architecture decision separates account creation from activation.
Its source state is proposed; no actual acceptance record accompanies the example.

<!-- ow:unit decision binding -->
## Activation requirement

Email verification is required before account activation.

<!-- ow:unit inactive binding -->
## Inactive account definition

An inactive account can receive a verification message but cannot use member-only
functions. Creating an account does not activate it.

<!-- ow:unit consequences binding -->
## Consequences

Signup can collect fewer fields while preserving the activation boundary.

<!-- ow:unit session binding -->
## Session changes

A change to session creation must keep inactive accounts out of member-only sessions.

<!-- ow:metadata -->
```toml
schema = "oh.war/document/1.0.0-rc.3"
kind = "adr"
id = "example:activation-decision"
revision = 3
title = "Keep verification before account activation"
state = "proposed"

[[dependencies]]
unit = "decision"
target = "#inactive"

[[context]]
id = "session-rule"
target = "architecture.md#session"
required = true
when = { path = ["src/session/**"] }
```
<!-- /ow:metadata -->
