# Gate-evidence recorder attribution

`war evidence record <alias> --as agent://codex` records who attaches the observed
gate receipt. The MCP tool `war_evidence_record` accepts the same optional `actor`
field. Use the actual submitting agent's name. The value is claimed metadata,
not authentication of a process, key holder or model.

The recorder must be a non-empty `agent://` identifier, at most 256 bytes, with
ASCII letters, digits, `-`, `.`, `_` and `/`. Empty path segments and `.` or `..`
segments refuse. Human identifiers, control characters and malformed values
refuse before any gate or journal write. This field cannot represent a human
signature, authorize work or supply independent verification.

Omitting the recorder retains the legacy configured-performer label. The public
four-argument Rust `evidence::record` entry point also retains that behavior;
`evidence::record_as` supplies the optional recorder. No old journal event,
contract, receipt format or digest algorithm is rewritten.

The protected performer identity used by self-verification and authority checks
is unchanged. Retrying an admissible receipt under the same recorder replays
without another write. A different recorder cannot overwrite the original
attachment: the existing conflict rule remains in force. A new collector name
does not make the submitting performer an independent verifier.

The synthetic public controls in `tests/evidence_recorder.rs` exercise actual
CLI recording, actor preservation, equivalent retry, conflict, invalid-input
no-write behavior, unchanged configuration and refusal of independent closure.
Their gate executes `true` only to exercise transport. It proves no SDK feature,
production identity, qualified gate or human acceptance.
