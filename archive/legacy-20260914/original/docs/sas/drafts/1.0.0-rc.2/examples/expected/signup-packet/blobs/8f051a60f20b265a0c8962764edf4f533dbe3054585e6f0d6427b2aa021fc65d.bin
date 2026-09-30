+++
schema = "oh.war/document/1.0.0-rc.2"
kind = "warrant"
id = "example:signup"
revision = 1
title = "Remove the company-name field from signup"
state = "proposed"
scope = { subsystems = ["identity"], paths = ["src/signup.rs"] }

[[context]]
id = "activation-rule"
target = "architecture.md#decision"
required = true
when = { stage = ["implementation", "verification"], subsystem = ["identity"] }

[[context]]
id = "required-cases"
target = "acceptance.json"
required = true

[[context]]
id = "design-history"
target = "unavailable-notes.txt"
required = false
+++

<!-- ow:unit outcome binding -->
## Desired outcome

Users can request an account with email and password without entering a company name.

<!-- ow:unit scope binding -->
## Scope

Change signup validation and the signup form. Keep account activation and existing
accounts compatible. Payment, login, and account deletion are outside this work.

<!-- ow:unit context binding -->
## Governing context

Use the activation decision and its definition dependency. This is an illustrative
proposal; this example does not assert an accepted ADR or grant execution permission.

<!-- ow:unit constraints binding -->
## Constraints

Email verification is required before account activation. Removing a form field
must not remove that condition. Existing accepted account records remain valid.

<!-- ow:unit acceptance binding -->
## Acceptance expectations

Exercise the cases in acceptance.json. Show that omitting company name succeeds,
invalid email fails, and an unverified account cannot activate. A listed case is
an expectation, not evidence that a test ran.

<!-- ow:unit outputs binding -->
## Required outputs

Return the changed code revision, case results, and any remaining limitations.

<!-- ow:unit stop binding -->
## Stopping conditions

Stop affected work if activation behavior must change or required expectations
cannot be met. Report the proposed revision and question; do not claim acceptance.

<!-- ow:unit approach background -->
## Suggested approach

Locate the field validator first. The implementer may choose another internal
approach within the stated outcome and constraints.
