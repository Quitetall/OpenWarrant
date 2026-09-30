# The projection contract

What `docs/warrants/generated/CORPUS_STATUS.json` and its siblings promise to
anything that reads them — the static app on Pages today, a Knowledge Fabric
projection or any other tracker tomorrow. Every field names the SAS section it
derives from. **Additive only after 1.0**: a field may be added with a
default; none is renamed, removed or re-typed under the same schema id.

The files, all compiled by `war compile` and drift-checked by `war check
--generated`, all canonical JSON (RFC 8785) where JSON:

| file | schema | what |
|---|---|---|
| `CORPUS_STATUS.json` | `oh.war/corpus-status/v1` | every Warrant, Objective and Requirement with its rung and the records behind it |
| `CORPUS_TIMELINE.json` | `oh.war/corpus-timeline/v1` | every journal event, sorted, with a per-day histogram |
| `CORPUS_PENDING.json` | `oh.war/corpus-pending/v1` | the human acts awaiting a signature, with the command for each |
| `docs/sas/generated/NORMATIVE.json` | `oh.war/sas-normative/v1` | every binding sentence of the SAS with its section |

Nothing here is a percentage. Every number is a count of named things, and
the names are in the same file.

## `oh.war/corpus-status/v1`

Top level:

| field | derives from | meaning |
|---|---|---|
| `provenance`, `provenance_note` | §24 | `derived` — states come from the records' shape; the note says what is and is not journalled |
| `release` | §101, §34.3 | the latest SAS revision, its digest, and the requirement ladder counts |
| `objectives[]` | §98 | one per phase: its Warrants, its ladder, whether its Exit is recorded, blocked or open |
| `warrants[]` | below | |
| `requirements[]` | §106, §34.3 | one per row: status (`satisfied` / `in_progress` / `claimed` / `unaddressed` / `superseded`) and the Warrants that link to it |
| `next_actionable[]`, `nothing_actionable` | §33 | the unblocked stages of the lowest unachieved Objective, or why there are none |
| `caveats[]` | — | what this projection does not read, in words |
| `repository_url` | `openwarrant.toml [project] repository_url` | optional; where the records live |
| `generated_by.war_version` | — | the binary that compiled this |

Each `warrants[]` entry:

| field | derives from | meaning |
|---|---|---|
| `alias`, `uuid`, `title`, `profile`, `assurance_level` | §12, §16 | identity |
| `validity` | §16.4 | `valid`, or `invalid` with the reason |
| `rung` | §34.3 | `invalid` → `draft` → `ready_to_resolve` → `would_satisfy` → `resolved` |
| `state` | §24 | recorded from the journal when it exists; derived and labelled otherwise |
| `checks` | §56.1 | the thirteen requirements, each true or false |
| `unmet[]` | §56.1 | the names of the false ones |
| `would_resolve_satisfied` | §38.6 | whether the admissible verifications would carry a `satisfied` resolution |
| `contract_revision`, `contract_digest` | §28 | the authorized revision and the digest the Warrant compiles to now |
| `resolution` | §56.2 | the record, with `binds_current_contract` (a resolution of an earlier revision derives nothing) |
| `obligations[]` | §38, §46 | `id`, `statement`, `scope`, the `gate://` cited, `disposition` (`established` / `refuted` / `not_established` / `undispositioned`), the verifier and its kind |
| `deliverables[]` | §37.2, OW-ADR-0012 | `id`, `title`, `target_ref`, `required`, `digest` (`verified` / `corrected` / `drift` / `target_unreadable` / `not_content_addressed`), `recorded_digest`, `corrections` (chain length) |
| `gate_runs[]` | §44.6 | `gate`, `run_id`, `verdict`, `class` (`admissible` / `stale_binding` / `receipt_invalid` / `inadmissible`), `why`, `receipt_ref` |
| `amendments[]` | §31 | `id`, `reason`, `changes[]` as `(element, before, after)` |
| `unknowns[]` | §36.2, §36.3 | every rationale assumption that is not a verified fact — residual risks, blocking unknowns — as `id`, `statement`, `epistemic_status`, `mentions[]` (external systems the statement names, read from its words; a reading, not a declaration) |
| `unestablished[]`, `blocking_unknowns[]` | §38.6, §36.3 | obligation ids without an admissible `established`; assumptions with `blocking_unknown` status |
| `milestones[]` | §33 | each with `reached` (`evidenced` / `unblocked` / `blocked`), its stages and obligations |
| `journal_ref`, `authorization_ref`, `resolution_ref` | — | repository-relative paths of the records, when present |
| `roadmap[]`, `implements[]` | §98, §106 | the refs into the SAS |

`digest` and `class` are computed by the same functions `war check` and
`war resolve --dry-run` use, so the page and the checker cannot disagree.

## `oh.war/corpus-timeline/v1`

`events[]` sorted by `(occurred_at, warrant, id)`, each `{occurred_at,
warrant, id, event_type, class, actor_ref, payload}` where `payload` is the
recorded payload parsed as JSON when it is JSON and the string otherwise;
`days[]` as `{date, events, by_type}`; `by_type`; `warrants` (how many
journals were read); `malformed_timestamps` (events kept but bucketed under
the day `malformed-timestamp`).

## `oh.war/corpus-pending/v1`

`acts[]` as `{warrant, action, command, why}` — `action` is `authorize`,
`resolve`, `correct` or `accept`; `command` is `war sign <target>` verbatim —
and `count`. Derived exactly as `war next` derives its human actions.

## Knowledge Fabric mapping (SAS §84)

For a KF projection or any relational tracker, the records §84 recommends map
onto these fields:

| §84 record | source |
|---|---|
| `warrant` | `warrants[]` (alias, uuid, title, profile, assurance_level, rung, state) |
| `warrant_contract_revision` | `warrants[].contract_revision`, `contract_digest`, `amendments[]` |
| `warrant_milestone`, `warrant_stage` | `warrants[].milestones[]` and its `stages[]` |
| `warrant_acceptance_obligation` | `warrants[].obligations[]` |
| `warrant_gate_binding` | `warrants[].obligations[].gate` |
| `warrant_runtime_receipt_reference` | `warrants[].gate_runs[]` (`receipt_ref`) |
| `warrant_blocker` | `warrants[].unknowns[]` where `epistemic_status = blocking_unknown` |
| `warrant_evidence` | `warrants[].gate_runs[]` and `deliverables[]` |
| `warrant_judgment` | `warrants[].obligations[]` (`disposition`, `verifier`) |
| `warrant_resolution` | `warrants[].resolution` |
| journal (§24, §80) | `CORPUS_TIMELINE.json` `events[]` |

`warrant_submission`, `warrant_deviation`, `warrant_observation`,
`warrant_inference` and `warrant_resolution_dispute` have no projection field
yet; a tracker that needs them reads the records under `docs/warrants/`.

## Stability

Schema ids are frozen at `v1`. A new field arrives with `#[serde(default)]`
on the Rust side and is optional on the wire. A change that cannot be
additive is a `v2` schema beside `v1`, never a silent redefinition. The
projection's own tests assert this: a `v1` document written before a field
existed still parses after it.
