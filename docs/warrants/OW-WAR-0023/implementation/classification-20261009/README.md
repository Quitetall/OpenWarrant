# Legacy Dispatch source classification

Unverified OW-WAR-0023 continuation. The selector had assigned `internal` to every
source, including atoms with different explicit labels. It now preserves each
selected atom's restricted Markdown classification scalar. Section views inherit
the full source's label. Artifacts and glossary declarations are preserved too;
unlabelled/non-Markdown/unfetched sources remain unclassified. Malformed declared
metadata refuses rather than silently becoming internal.

No classification lattice is inferred. A common explicit label is recorded only
when every included source has that label. Otherwise effective classification is
empty with an unresolved policy entry; no policy digest or authority is invented.
Historical packets and signed records remain unchanged.

The public offline SDK and CLI can check disclosure of one selected source under
an independently supplied exact label allowlist. Checks use retained full source
bytes, including section provenance, and reject an unlisted/unknown/mismatched
label. This does not qualify a whole package or establish the caller's authority,
provider-policy join, execution readiness, semantic closure or harness isolation.
It is a primitive that a provider/workflow must invoke under its own trusted
actor/runtime/provider/tool/destination policy before disclosure.

## Observed checks

Rust 1.97.1. Ten portable-Dispatch integration tests pass, including actual source
repository deletion, section/whole source/artifact/glossary labels, missing labels,
opaque case/wildcard denial and a forged manifest with all local hashes recomputed.
A separate parser test checks missing, binary, malformed and duplicate declarations.
All-target CLI Clippy passes with warnings denied. The public CLI plant checks
actual offline allow/deny/unknown behavior. Logs are beside this document.

Full hosted repository gate, provider integration, independent qualification and
human acceptance remain separate. This report awards no assurance or closure.
