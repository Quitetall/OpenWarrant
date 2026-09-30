# Remove the company-name field from signup

Role: implementation. Stage: implementation.
Action: Produce candidate artifacts and evidence-backed claims within the contract.
Stop: Candidate ready for independent verification, or affected work is blocked.
Coverage: declared-inputs-and-dependencies. Readiness: blocked.
Reference fixture only; no execution permission or accepted result.

## Task brief

- outcome: signup.md#outcome (exact text below).
- scope: signup.md#scope (exact text below).
- context: signup.md#context (exact text below).
- constraints: signup.md#constraints (exact text below).
- outputs: signup.md#outputs (exact text below).
- stop: signup.md#stop (exact text below).

## Binding context

Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit acceptance

## Acceptance expectations

Exercise the cases in acceptance.json. Show that omitting company name succeeds,
invalid email fails, and an unverified account cannot activate. A listed case is
an expectation, not evidence that a test ran.


Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit constraints

## Constraints

Email verification is required before account activation. Removing a form field
must not remove that condition. Existing accepted account records remain valid.


Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit context

## Governing context

Use the activation decision and its definition dependency. This is an illustrative
proposal; this example does not assert an accepted ADR or grant execution permission.


Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit outcome

## Desired outcome

Users can request an account with email and password without entering a company name.


Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit outputs

## Required outputs

Return the changed code revision, case results, and any remaining limitations.


Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit scope

## Scope

Change signup validation and the signup form. Keep account activation and existing
accounts compatible. Payment, login, and account deletion are outside this work.


Source: signup.md | sha256:8f051a60f20b265a0c8962764edf4f533dbe3054585e6f0d6427b2aa021fc65d | unit stop

## Stopping conditions

Stop affected work if activation behavior must change or required expectations
cannot be met. Report the proposed revision and question; do not claim acceptance.


Source: acceptance.json | sha256:c0654f491b78a4b66e5a9ce9236d49662446114a5bb5efebb87c13ffe0ef0f9b | unit *

[Required fixture bytes](blobs/c0654f491b78a4b66e5a9ce9236d49662446114a5bb5efebb87c13ffe0ef0f9b.bin), 505 bytes, application/json. These cases have not run.

Source: architecture.md | sha256:c90c4ab63cf754ace400953f34441b6eada14a967cf40122b08880159cdb22b5 | unit decision

## Activation requirement

Email verification is required before account activation.


Source: architecture.md | sha256:c90c4ab63cf754ace400953f34441b6eada14a967cf40122b08880159cdb22b5 | unit inactive

## Inactive account definition

An inactive account can receive a verification message but cannot use member-only
functions. Creating an account does not activate it.


## References

- architecture.md#session-rule: FALSE; supplied=true; selected=false; src/signup.rs does not match src/session/**.
- signup.md#activation-rule: TRUE; supplied=true; selected=true; stage and subsystem match.
- signup.md#design-history: TRUE; supplied=false; selected=false; optional source unavailable.
- signup.md#required-cases: TRUE; supplied=true; selected=true; unconditional required input.

## Diagnostics

- readiness-blocked: Illustrative proposal: no trusted execution permission or accepted architecture record supplied.
- target-missing: Optional design history is unavailable.
