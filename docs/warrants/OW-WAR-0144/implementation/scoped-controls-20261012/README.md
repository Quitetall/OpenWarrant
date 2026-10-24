# OW-WAR-0144 scoped runtime observations

The existing authorization-retention implementation passes six feature controls plus the positive corpus check (7/0 total). A throwaway fixture key authorizes two revisions of a synthetic Warrant. The superseded authorization survives byte-for-byte beside revision 2; its original attestation verifies with attest.subject-archived. Removing those retained bytes produces the expected attest.subject-drift. A conflicting archive filename refuses authorize.retire-collision while preserving the old authorization. The original assertion did not compare the whole Warrant snapshot; the [later audit](../retention-audit-20261024/README.md) corrects that evidence gap. A dry run writes nothing, and the retained file does not become another active authorization.

Only fixture keys and actors were used. No real authorization, authority configuration or signing key was modified. These are performer observations over this Linux build, not an independent verdict or human acceptance.

The resolution dry run reports six PASS, one WARN and eight UNKNOWN. D-001 (authorize.rs) has later OW148 ownership; exact artifacts, independent dispositions, admissible gates, unknowns, blockers, judgments, independence and risk authority remain unestablished. Existing pins, contracts, signatures and dispositions remain intact. OW144 remains in progress.
