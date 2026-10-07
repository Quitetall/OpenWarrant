# Warrant sign-off: verification and qualification

Ordinary completed work can stay unverified. The common Verified mark needs
its versioned baseline, independent evidence and a person's secure acceptance
of the exact result. A signature alone does not prove behaviour. A late review
keeps the actual chronology and does not satisfy a criterion that asks for an
act before the work.

For a Warrant, request independent verification with
`war verify <alias> --performer <actor> --bundle`. Give the verifier the exact
contract, the candidate code or workspace, protected fixtures and retained
evidence. It records only what it can establish. Record its answer with
`war verify <alias> --response <file>`; the actor and independence fields stay
as the verifier wrote them. A verifier that was unavailable means UNKNOWN,
not PASS.

`war resolve --dry-run <alias>` lists what close-out still lacks. Drafting a
resolution or correction request is separate from a person signing it; see
the [sign-off loop](loop.md). Original signatures and signed revisions stay as
they are.

When `war sign` fails, run `war doctor`. Its signing probes sign nothing: they
check `ssh-keygen`, the agent behind `SSH_AUTH_SOCK` and the keys it holds,
`docs/authority/roles.toml` and `docs/authority/allowed_signers`, and say what
is missing. At a terminal, `war doctor --fix-signing` offers to repair each
finding. A signing failure blocks only the sign-off, not your work.

For prototypes, keep tests and results without presenting a legacy resolution
as the new completion model. New SDK and workflow acts are supported where
the installed tool exposes them. An agent's attestation, where a person's
policy allows one, uses the agent's own identity and key; it does not award
the common mark or stand in for a person's acceptance, and the tool records a
person's signature only from that person's key.
