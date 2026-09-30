# The loop, with the stops marked

Legacy transport reference. These commands retain their existing authority rules;
use [execution](execution.md) for prompt-only unverified work.

Every step is a `war` command. A human signs at two points; nothing you do
substitutes for either.

```bash
war next                                   # whose act is next: read this first
war new "What this work accomplishes"      # docs/warrants/<NS>-WAR-NNNN/
# edit the atoms (see AGENTS.md "Writing the atoms")
war check <alias>                          # deterministic; no agent, no network
war compile && war check --generated       # projections written; no drift
war authorize <alias>                      # the REQUEST a human signs
#   ── STOP. Human: `war sign <alias> --ssh-sign` (one dialog). ──
war pins --resolved-only                   # before editing: what a resolution pins
# deliver; declare each file in deliverables.toml
war evidence record <alias>                # run the cited gates; mint §44.6 receipts
war verify <alias> --performer <you>       # request for an INDEPENDENT verifier
# hand it to something that is not you; ingest what comes back:
war verify <alias> --response <file>
war resolve --dry-run <alias>              # the thirteen §56.1 requirements, honestly
war resolve <alias>                        # the REQUEST
#   ── STOP. Human: `war sign <alias> --ssh-sign`. ──
```

`--json` on any command gives one `oh.war/report/v1` envelope: `diagnostics[]`
(each with a `rule`), `verdict`, `exit_code`, and a command-specific `result`.
`war next --json` never hands an agent a signing act.

A pinned file of a RESOLVED Warrant may only change through the correction
act: `war correct <alias> <D-id>` emits the request; a human signs it with
`war sign <alias>/<D-id> --kind behaviour-change|added-refusal --meaning "…"`.
Edit first, then request; the tool refuses when nothing drifted.

## Reading the specification

Never load the whole SAS (tens of thousands of tokens). `docs/sas/generated/
NORMATIVE.md` is every binding sentence with its section, compiled by `war
compile` and drift-checked; cite a section as `§47.2` and read the document
only when the sentence's reasoning matters.

## Context per stage

A stage in `45-milestones.yaml` may declare what its Dispatch carries:
`context_sections: ["40-work-order.md#Deliverables"]`, `context_atoms`,
`context_artifacts` (repository paths), `context_external` (URIs, recorded
and never fetched). `war dispatch <alias> <stage> --emit-context ctx.json`
shows what was selected and what was omitted, each with its reason; a section
that does not exist is refused with the headings that do.
