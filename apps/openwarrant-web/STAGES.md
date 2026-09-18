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

Execution configuration v2 requires `stage_plan` in each Warrant policy. V1
remains unchanged. A staged `/api/runs` or `/api/admission` request includes exact
`warrant_id`, `source_sha256`, and `stage`. Omitting or inventing the stage refuses.
The harness receives execution-request/v3 with the selected stage, full stage plan
and prior completed stage identities. Results retain the ordinary v1 result shape;
its `completed` value refers to the selected stage. The controller decides whether
the full Warrant is complete.

Dispatch serializes writers in one Warrant worktree and checks stage dependencies
against retained evidence on its exact current clean revision. Every successful
stage reruns checks for earlier completed stages. Only completion of every stage
plus the Warrant checks emits the Warrant completion signal. Stage attempts share
the Warrant time budget; repair limits apply per stage. Question continuations keep
the existing cumulative budget rules. A question must name the selected stage and
may name only configured affected stages.

Stage selection is available through the authenticated API and browser execution
controls. GET `/api/stages/<warrant-id>` reports configured stages and advisory
readiness without creating attempts. Browser selection requires an explicit stage
choice. Safe hotline resume after independent stages advance a question's
checkpoint remains unfinished. The existing exact-checkpoint resume refusal remains;
no old answer silently authorizes a changed checkpoint.

`completed_from_evidence` validates a current stage checkpoint against exact source,
plan and code revision. It removes successors whose prerequisite checks are absent
or failed, even if their own checks passed. A boolean false is not exit code zero.
The checkpoint producer runs actual checks on the unchanged result revision; dispatch binds that producer to configured policy and current Git inputs; accepting arbitrary browser-submitted records is not supported.

Checkpoint checks run in dependency order. A failed prerequisite skips checks for
its dependent stages; independent checks continue. `skipped_stages` records direct
unmet prerequisites. A skip is not an observed check failure or successful check.

GET `/api/hotline/<attempt-id>/checkpoint` provides a read-only checkpoint review.
Every dispatch records its input Git revision. Movement from the original question
checkpoint must trace uniquely through stopped independent-stage results with the
same source, policy, execution configuration and worktree, and passing current
stage evidence. Dirty state, unknown/running writers, unrelated commits, missing
lineage or an already resumed question refuse. The preview explicitly requires
answer reconfirmation when the checkpoint changed; it does not authorize resume.
Authenticated reconfirmation storage and its browser/resume integration remain open.
