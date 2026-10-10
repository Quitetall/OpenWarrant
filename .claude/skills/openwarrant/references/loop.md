# Warrant sign-off: the loop

For a Warrant whose type has a sign-off step. Ordinary work and tickets skip
all of this; for implementation, see [execution](execution.md).

A person signs at two points, and the tool accepts nothing else in their
place.

```bash
war next                                   # what is ready, and whose step it is
war plan new "What this work accomplishes" # docs/warrants/<NS>-WAR-NNNN/
# fill in the atoms (AGENTS.md, "Writing the atoms")
war check <alias>                          # deterministic; no agent, no network
war admin compile && war check --generated # views written; no drift
war sign authorize <alias>                 # drafts the approval request
#   a person signs: `war sign <alias> --ssh-sign` (one dialog)
war admin pins --resolved-only             # the files closed Warrants pin
# deliver; declare each file in deliverables.toml
war evidence record <alias>                # run the cited checks; record receipts
war evidence verify <alias> --performer <you> # the request for an independent verifier
# hand it to something that is not you, then record what comes back:
war evidence verify <alias> --response <file>
war sign resolve --dry-run <alias>         # what is still missing before close-out
war sign resolve <alias>                   # drafts the close-out request
#   a person signs: `war sign <alias> --ssh-sign`
```

`--json` on any command gives one `oh.war/report/v1` envelope: `diagnostics[]`
(each with a `rule`), `verdict`, `exit_code`, and a command-specific `result`.
`war next --json` gives every signing step to a person.

A file a closed Warrant pins changes through the correction act: edit it,
`war sign correct <alias> <D-id>` drafts the request, and a person signs it with
`war sign <alias>/<D-id> --kind behaviour-change|added-refusal --meaning "..."`.
The tool refuses a correction when nothing drifted.

If `war sign` fails, `war admin doctor` checks the signing setup without signing
anything and says what to fix. A signing failure blocks only the sign-off,
not your work.

## Reading the specification

The whole SAS is tens of thousands of tokens. `docs/sas/generated/
NORMATIVE.md` is every binding sentence with its section, compiled by `war
compile` and drift-checked; cite a section as `§47.2` and read the document
only when the sentence's reasoning matters.

## Context per stage

A stage in `45-milestones.yaml` may declare what its Dispatch carries:
`context_sections: ["40-work-order.md#Deliverables"]`, `context_atoms`,
`context_artifacts` (repository paths), `context_external` (URIs, recorded
and not fetched). `war admin dispatch <alias> <stage> --emit-context ctx.json`
shows what was selected and what was omitted, each with its reason; a section
that does not exist is refused with the headings that do.
