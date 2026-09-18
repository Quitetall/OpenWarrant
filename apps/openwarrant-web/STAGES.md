# Internal stage execution (in development)

A Warrant describes one reviewable outcome. The workflow may split implementation
into explicit stages. These dispatch units do not replace milestone acceptance
records, governing source documents or independent assurance obligations.

The workflow-local planner accepts this configuration shape:

```json
{
  "schema": "oh.war/execution-stage-plan/v1",
  "stages": {
    "api": {
      "title": "Implement API behavior",
      "outcome": "Required responses and refusal cases pass.",
      "dependencies": [],
      "checks": [["/trusted/check-api"]]
    },
    "ui": {
      "title": "Connect UI",
      "outcome": "The UI displays the specified API responses.",
      "dependencies": ["api"],
      "checks": [["/trusted/check-ui"]]
    }
  }
}
```

This is app-owned configuration, not a new canonical document format. The SDK
continues to own document parsing. Source constraints remain binding on every
stage; a stage outcome cannot authorize work outside the Warrant.

Validation requires 1–64 stages, nonempty bounded titles/outcomes, explicit check
commands and an acyclic graph of known stage identities. Duplicate dependencies,
unsupported fields and unsupported versions refuse. Commands use the same bounded
argv validator as ordinary execution. Validation does not execute commands.

The planner propagates a blocking question through dependent stages and keeps
independent stages ready. A running writer prevents another stage from acquiring
the same Warrant worktree. A new block against the running stage reports
stop-required; the planner cannot itself stop or fence a process. Contradictory
completion/writer observations refuse instead of creating a ready result.

Current library implementation provides validation, read-only planning and actual
check execution against an exact clean Git revision. The
existing execution configuration remains v1 and does not yet accept or dispatch
this plan. Dispatch integration must bind exact plan/source revisions, serialize
writers, preserve stage evidence, rerun affected checks after shared-worktree
changes and aggregate final Warrant completion without treating one stage as the
whole outcome. Hotline resume must handle checkpoint movement caused by legitimate
independent stages without silently accepting unrelated edits. These checks remain
required before the stage gap can be closed.

`completed_from_evidence` validates a current stage checkpoint against exact source,
plan and code revision. It removes successors whose prerequisite checks are absent
or failed, even if their own checks passed. A boolean false is not exit code zero.
The checkpoint producer runs actual checks on the unchanged result revision; the
future dispatch integration must bind that producer to trusted current inputs; accepting arbitrary browser-submitted records is not supported.

Checkpoint checks run in dependency order. A failed prerequisite skips checks for
its dependent stages; independent checks continue. `skipped_stages` records direct
unmet prerequisites. A skip is not an observed check failure or successful check.
