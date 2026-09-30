# Qualification and legacy verification

Ordinary completed work may remain unverified. The common Verified mark requires
its versioned baseline, independent evidence and secure human acceptance of the
exact result. A signature alone does not prove behavior. A late review preserves
actual chronology and cannot satisfy a criterion requiring a pre-work act.

For legacy records, request independent verification with
`war verify <alias> --performer <actor> --bundle`. Give an independent verifier
exact contract, candidate code/workspace, protected fixtures and retained evidence.
It records only observations it can establish. Receive its response through
`war verify <alias> --response <file>`; preserve the actual actor and independence
fields. An unavailable verifier means UNKNOWN, not an invented PASS.

`war resolve --dry-run <alias>` lists legacy resolution gaps. Requesting resolution
or correction is separate from the human signing it. Read [legacy loop](loop.md)
for those historical acts. Preserve original signatures and immutable revisions.

For unverified prototypes, retain tests and results without claiming that legacy
resolution represents the new completion model. New SDK/workflow acts are supported
only when the installed tool exposes them. Agent attestation, where human policy
permits it, uses the agent's own identity/key; it cannot award the common mark or
replace a secure human acceptance. Never invoke a human signing key as an agent.
