# W113 decision preset defect

The public `war plan new --preset decision` command created a local
`atoms/30-decision.md` with role adr and jurisdiction authored. The checker
refused it as `atom.jurisdiction-mismatch`: the role's jurisdiction is bound.
Changing only that header in the synthetic fixture then refused
`atom.generated-as-source`, because a bound ADR must be referenced from outside
the Warrant's own editable directory. A valid proposed ADR in the configured
ADR corpus, bound by the manifest, passes the same structural check.

The original command output and both refusal observations are preserved here.
The modified demo remains synthetic; no real architecture choice, approval or
signature was created. Its original generated structure was sequestered inside
the fixture archive, not erased.

Source inspection finds `new::write_preset` hard-coding authored jurisdiction
and `manifest_template` putting every core atom below atoms/. This is a writer
defect, not a reason to weaken role/jurisdiction checks. Existing resolved pins
for new.rs are historical and name OW113/D-031 as their current governor;
the old bytes and records remain untouched.

W113's earlier unverified completion report is reopened for this actual
implementation defect. Its selected bounded malformed-role criterion and
historical evidence are unchanged. The fix must create a proposed ADR with its
own minted identity in the configured ADR corpus and bind it without inventing
acceptance or exposing a cross-branch alias collision.

## Implemented correction, unverified

The writer now mints a proposed ADR with its own UUID in the configured ADR
corpus. Its UUID-derived alias avoids a shared next-number race; the Warrant
binds its relative source path and does not create a local editable ADR atom.
TOML string serialization preserves quoted titles and configured paths. No
choice is inferred and proposed does not mean accepted.

Three public CLI controls pass: distinct external proposed ADRs without
overwriting the prior source, quoted titles/paths, and rationale parsing without
resolving an unknown. The parser rejects unknown epistemic statuses and missing
resolution requirements; checking leaves source bytes unchanged. A valid unknown
is a valid draft, not evidence that its question was answered.

All 10 existing scaffold unit tests, eight existing check CLI tests and
all-target Clippy with warnings denied pass. The initial Namespace formatting
compile failure and an unrelated fixture setup failure remain in the logs;
neither counts as the intended red observation. The corrected repository corpus
check has zero errors and explicitly parses 88 rationale records. Exact-head
protected CI and qualification remain pending.
