---
schema: oh.war/atom/v1
warrant_uuid: 01a060de-a1c9-77f2-962a-74ca52797a34
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `.github/workflows/pages.yml`: on push to `main` touching the three
   generated files (and `workflow_dispatch`), assemble `_site/` from
   `docs/warrants/generated/CORPUS_STATUS.{html,json,md}` with the HTML also
   as `index.html`; assert every uploaded file's sha256 equals the committed
   file's; upload and deploy with SHA-pinned actions; `contents: read`,
   `pages: write`, `id-token: write`; a timeout; a concurrency group.
2. `cargo xtask gate` step `workflows`: every `uses:` in
   `.github/workflows/*.yml` pinned to a 40-hex SHA; `pages.yml` names
   `docs/warrants/generated/CORPUS_STATUS.html` and compares digests. Five
   unit tests, including the tag-pinned refusal and the real workflow.
3. Pages enabled on the repository (`build_type = workflow`) and the first
   deploy run, with its job URL and the served page's digest recorded in the
   basis when it happens.

## Frozen Surfaces

`CORPUS_STATUS.{html,json,md}` — published as they are, byte for byte. The
eight existing gate steps.

## Premade Instructions

- Do not render anything in the workflow. Copy and compare.
- Do not add a tool that writes into `docs/warrants/generated/` from CI.
- Pin by SHA; keep the tag in a comment.

## Autonomy and Escalation

Tier T2. Escalate if enabling Pages requires a repository setting the API
will not change.

## Rollback

Delete the workflow and disable Pages. The committed page is unchanged.
