# Two-axis review example: W113 decision scaffold

This is a bounded **performer inspection**, not an independent verification.
Base: `08ae6c2084399f22ea64a86bfafc0d018f3fbfb9`. Candidate: `fd80d261bb763cd9292aa91c9902a53ce58180a2`.
The [machine-readable observation](observation.json) pins the source hashes,
separate axes and actual verification request. Later report edits are outside
this reviewed candidate.

## Standards axis — PASS within the stated bound

I inspected the diff in new.rs and check.rs and its three public CLI controls.
The scaffold keeps ADRs bound outside the Warrant, mints an independent UUID,
and uses an opaque alias instead of taking a shared sequence number. TOML
serialization preserves quoted values. The checker reuses the existing rationale
parser and preserves UNKNOWN for an unavailable source; parsing an unresolved
question does not answer it. No dependency or signed contract was changed.
No actionable finding was established in this bounded inspection. This does
not assess every W113 deliverable or failure mode.

The [W113 correction report](../../../OW-WAR-0113/implementation/decision-preset-audit-20261024/README.md)
retains the actual failing controls, three passing CLI controls, ten scaffold
unit tests, eight check CLI tests and warnings-denied Clippy. I checked those
retained observations and the current source; I did not rerun the full protected
gate here. Candidate CI was still pending when this report was written.

## Obligations axis — UNKNOWN; independent execution NOT RUN

The public tool emitted an `oh.war/verification-request/v2` for W113 with the
real performer `agent://codex` and all four obligations. Its original report
is retained at the absolute path and SHA-256 in observation.json, rather than
copying its 1.4 MB payload into this example. It binds current deliverable bytes,
contract and gate records; the new.rs and check.rs digests match this candidate.
Historical gate records inside that request are inputs, not a claim that the
new candidate ran those gates.

No verifier was invoked, no response was received or ingested, and no independent
disposition, signature or completion mark was created. Mandatory model reviews
are suspended by owner direction; this example does not start one. A later
verifier must receive executable candidate access and the applicable evidence,
and must assess the exact subject it actually reviewed. Regenerate the request
if its bound sources change.

The owner permits either `manifest.invalid` or `atom.role-unprojected` for the
bounded malformed bare-role refusal. That direction does not remove rejection,
projection or extension controls and does not amend the signed criterion itself.

W68 still needs the owner's review of first recorded runs and independent
qualification. This example records the process and the missing observation;
it does not clear those gates.
