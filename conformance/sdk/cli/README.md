# Offline SDK CLI profile

`war sdk --request request.json` runs one explicit SDK operation. Use `--request -`
for standard input. It works outside an OpenWarrant repository. It always returns
one `oh.war/report/v1` JSON envelope; `--json` is accepted but not required.
`--help` remains ordinary command help. Existing `war document review` and legacy
Warrant commands keep their meanings.

```sh
printf '%s\n' '{"schema":"oh.war/sdk-request/v1","operation":"raw-digest","bytes_hex":"616263"}' |
  war sdk --request -
```

Request profile `oh.war/sdk-request/v1` is a candidate CLI transport profile. It
wraps existing versioned SDK documents and records without renaming their schemas
or digest domains. Requests use the fields below; complete runnable examples and
named refusals are in `cases.json`. No fixture ID is sent to the implementation.

| Operation | Required fields; optional fields in parentheses | Public SDK operation |
| --- | --- | --- |
| `parse`, `validate` | `source`, `dialect` | `parse_document`, `validate_document` |
| `unit` | `source`, `dialect`, `unit` | `Document::unit` |
| `author` | `metadata`, `units`; (`crlf`, `wrap`) | `author_document` |
| `edit` | `source`, `dialect`, `edits` | `edit_document` |
| `raw-digest` | `bytes_hex` | `raw_digest` |
| `digest` | `domain`, `payload` | `structured_digest` |
| `condition` | `condition` | `validate_condition` |
| `source-describe` | `path`, `bytes_hex`, `holder`, `metadata`; (`dialect`) | `describe_source` |
| `source-codec`, `reference-codec` | `payload` | source/reference decode and encode |
| `sources-check` | `descriptors`, `files`; (`reference`) | `check_sources`, `check_reference` |
| `packet-decode`, `packet-encode` | `payload` | packet decode/encode |
| `packet-check` | `payload`, `files`, `entry` | `check_packet_integrity` |
| `package-check` | `files` | `check_package` |
| `adapter-prepare` | `basis`, `files`, `provider`, `required` | `prepare` |
| `adapter-check` | previous fields plus `response` | `prepare`, `validate_response` |
| `records` | `records`, `subject`; (`assumptions`) | `check_records` |
| `readiness` | previous fields plus `conditions`, `action`, `stage`; (`policies`) | `evaluate_readiness` |
| `assurance` | `records`, `subject`, `verification_ref`, `acceptance_ref`, `conditions`; (`assumptions`, `contract`, `policies`) | `evaluate_assurance` |
| `agent-act` | `payload` | `decode_agent_act` |
| `workflow-decode` | `payload` | `decode_workflow_record` |
| `handoff` | `stop`, `overview`, `handoff`, `assumptions`, `word`, `style` | `render_handoff` |
| `replay` | `previous`, `incoming` | `check_replay` |
| `resume` | `change`, `assumptions` | `evaluate_resume` |
| `availability` | `record`, `assumptions` | `evaluate_availability` |
| `batch-acceptance` | `manifest`, `member`, `assumptions` | `batch_acceptance` |
| `schedule` | `scopes`, `dependencies`, `facts` | `evaluate_schedule` |
| `difficulty` | `estimate` | `check_estimate` |
| `legacy-import` | `payload` | `import`, `export` |
| `legacy-capture` | `dialect`, `adapter`, `entry`, `files` | `capture`, `export` |
| `successor` | `predecessor`, `successor`, `paths` | `check_successor` |

## Encoding

- `source` is exact UTF-8 Markdown. Document dialect is explicit `rc2` or `rc3`.
  Legacy capture uses its existing `legacy-warrant-v1`, `rc2` or `rc3` dialect.
- Embedded record/package descriptor `payload`, `records`, `contract`, workflow
  and preservation inputs are JSON **strings**, preserving their exact bytes.
  The `digest` operation alone takes an arbitrary JSON **value** as `payload`.
- `metadata` for authoring is an array of `[key, JSON value]` pairs so duplicate
  field refusal remains observable. Units are `{id, kind, text}`, with kind
  `binding` or `background`. Edits are `{kind:"metadata", key, value}` (null
  removes a field), or `{kind:"unit", id, text}`. Authored text returns in
  `result.source`; the CLI does not overwrite a source document.
- `files` is an array of `{path, hex}`. Hex must be lowercase, with two characters
  per byte. For source/packet/adapter input blobs, `path` is the raw digest key
  (`sha256:...`). For complete packages and preservation inventories it is the
  exact relative filename. Duplicate keys refuse. No named path is fetched.
- `digest.domain` is `basis`, `contract` or `package`, selecting the existing
  domain. `condition` accepts an object and checks syntax only.
- `basis`, `provider`, `subject`, descriptors/references and helper facts use
  their SDK field names. Enum variants use kebab-case; struct fields use
  snake_case. `Timing` is `"any"` or `{"before": milliseconds}`; requirements
  are objects such as `{"passing-check":{"record_id":"...","check_digest":"..."}}`.
  Omitted assumptions/policies are empty. Optional scalar fields may be absent
  or null. Unknown fields, duplicate JSON keys and positional arrays for named
  structs refuse, including nested fields. Duplicate set members refuse.
- `adapter-check.response` contains `provider`, `basis_digest`, `files` and
  optional `semantic_evidence`. The request is rebuilt and validated from its
  basis and bytes. No serialized private validated handle is trusted.

## Results and authority

Exit 0 means the requested evaluation ran. It does **not** mean that the work is
ready, eligible, authenticated or complete. Inspect `result.evaluation` and its
findings. UNKNOWN and unmet evaluations remain successful queries with their true
standing. Malformed, conflicting, unsupported, over-limit inputs and I/O failures
return exit 1 with a diagnostic, no result, and no published output file.
`parse` may return a syntactically parsed but invalid document; `validate` refuses
invalid documents. Diagnostics retain SDK code, message and byte span where given.

Assumptions and policy facts are supplied for conditional evaluation. The CLI
does not authenticate them, verify signatures, establish independent observations,
grant permission or issue an assurance mark. Even conditional `eligible` and
`established` results report this limitation. Agent acts remain unauthenticated.
Package integrity is separate from semantic coverage. Provider semantic evidence
is a reported provider claim, not locally established coverage. Adapter operations
perform no provider call. Scheduling neither dispatches work nor grants access.
Handoff text remains inside a conditional result; the CLI never emits a bare
completion word or updates the tracker. It never signs, stops workers or deletes
retained history. The workflow layer owns these actions.

## Explicit I/O and bounds

The only automatic read is the named request file or stdin. Embedded source paths,
URLs, references and commands are data. No repository discovery, network, model,
provider or signing invocation occurs. Requests are at most 4 MiB, at most 65,536
JSON value nodes and depth 64. Fixed SDK defaults additionally limit each operation.
The result envelope, including its newline, is at most 16 MiB. Oversize output is
refused while serialized. These are admission bounds, not a universal peak-memory
or runtime guarantee.

`--output fresh.json` publishes the **same JSON envelope** written to stdout, only
on success. It requires an existing parent and a new destination. A same-directory
exclusive temporary file and atomic hard-link publication prevent replacement of
existing files or symlinks. Failure or interruption never overwrites old bytes.
An abrupt process kill may leave an owned `.ow-sdk-*` temporary file, or a fully
published new result if publication already happened; it never leaves a partial
published file. No fsync-of-directory crash durability is promised. A broken stdout
can occur after successful file publication; callers must inspect the explicit file
before retrying. There is no output overwrite mode.

## Reproduce

```sh
cargo test -p openwarrant-cli --test sdk_cli
cargo test -p openwarrant-cli sdk::wire::tests
```

The first command is the direct OW87 acceptance driver: real noninteractive CLI,
file-backed success/refusal requests, public SDK comparisons, malformed nested
JSON, limits, input file/stdin, new output, prior-byte preservation, symlink refusal
and cancellation before publication. It does not launch a provider or a human
signer. The full repository gate retains the earlier SDK and legacy command tests.
