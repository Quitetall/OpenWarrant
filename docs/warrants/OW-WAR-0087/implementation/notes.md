# OW-WAR-0087 implementation

The offline `war sdk --request <file|->` command exposes 33 public SDK operations.
It runs outside a repository and emits one existing report envelope. Requests use
explicit bytes and typed helper fields. Source editing returns new source text;
optional output publishes a new JSON result file without replacing old files.
Legacy Warrant commands and `war document review` retain their meanings.

The CLI profile and full operation mapping are in
`conformance/sdk/cli/README.md`. Its file-backed driver has 71 cases, with success
and a named refusal for every operation. Eleven public CLI tests additionally compare
parser, author/edit, canonical digest, record/readiness and preservation outcomes
with direct SDK calls. They exercise file/stdin input, fresh output, existing-file
and symlink refusal, cancellation before publication, malformed arguments, nested
JSON duplicates/unknown fields/positional objects and admission limits. A bounded
writer unit test observes serialization refusal. Clippy passes with warnings denied.

Assumptions are never authenticated by this command. Conditional eligibility is
not an assurance mark. Missing human acceptance or established checks remains
unknown; a changed subject refuses. Provider responses are checked as supplied
bytes without calling the provider; integrity does not establish semantic coverage.
Workflow helpers neither dispatch nor stop agents, change storage nor issue human
signatures. No completion, permission or qualification follows from exit zero.

Independent specification review reproduced all 71 cases and passed the bounded
slice. Independent standards review found an ignored nested requirement field;
the fix rejects unknown variant fields and any ignored nested object member.
Earlier review found duplicate JSON keys; recursive bounded decoding now refuses
them before typed decoding. Both have regression coverage. Explicit null in known optional SDK fields stays
valid; unknown null fields still refuse, with a separate regression. No independent
assurance disposition or human acceptance is claimed.

The API is an explicit input shell, not an interactive workflow. Its request cap
is 4 MiB, 65,536 JSON values and depth 64, plus existing SDK limits. The envelope
cap is 16 MiB. Atomic no-clobber publication preserves existing bytes on failures;
an abrupt kill may leave an owned temporary file or a fully published new output.
This is not a claim of crash durability or measured universal memory bounds.

Base: `fba4eba73965bab67eed0d69a0643ff2f15f34de` (OW84). OW85 depends on OW87's CLI
parity, so OW87 runs first. Full repository gate and commit review remain pending
until their observed results are attached. Work is not yet reported complete.
