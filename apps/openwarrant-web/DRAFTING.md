# Configured agent drafting

Add `--drafting-config /private/path/drafting.json` to the reference server.
The optional adapter can use a local model, a cloud endpoint or a connected agent.
The app does not choose providers, hold their keys, or claim their model quality.
Keep credentials in the harness's private environment, never context or argv.

```json
{
  "schema": "oh.war/drafting-config/v1",
  "argv": ["/absolute/path/to/trusted-drafting-adapter"],
  "backend": "your provider/model/version label",
  "sandbox": "read-only-repository",
  "cost_mode": "unknown",
  "spend_limit_usd": 10,
  "timeout_seconds": 300,
  "context": [
    {"path": ".claude/skills/war-spec/SKILL.md", "sha256": "REPLACE_WITH_64_HEX_SHA256", "role": "skill"}
  ]
}
```

This example refuses dispatch: unknown cost cannot meet its hard cap. An owner
may explicitly use `null` for an uncapped policy, retaining unknown cost, or
`free` only when the harness makes no paid calls. Free is an owner assertion;
this app does not meter paid providers. The default user budget remains $10.
The backend label is also an owner assertion, not observed model provenance.

Exactly one context entry identifies the drafting skill. Add relevant exact
rules and references as `role: context`. Paths are repository-relative and reject
symlinks/traversal. Each file is at most 64 KiB; total context at most 128 KiB.
Digest mismatch refuses dispatch. The same pinned inputs are rechecked before
accepting output. This is context delivery, not semantic selection or compilation.
LAMU or another provider can prepare the context ahead of this adapter.

## Process contract

The adapter gets one JSON request on stdin (`oh.war/drafting-request/v1`) with
request ID, prompt, `skill: war spec`, pinned context bodies and limits. It must
follow the supplied skill, name unresolved decisions, and return only:

```json
{
  "schema": "oh.war/drafting-result/v1",
  "fields": {
    "title": "One reviewable outcome",
    "outcome": "Required behavior and positive/refusal acceptance expectations",
    "scope": "Permitted changes, constraints and non-goals",
    "context": "Exact sources, prerequisites and unresolved decisions"
  }
}
```

Every field must be nonempty and at most 16,000 UTF-8 bytes. Extra authority or
qualification fields refuse. The real SDK authors/validates a revision-one draft
from these fields. Generated Markdown is previewed, not automatically saved.
Model reasoning, invented approval and executable callbacks are not part of this
protocol. Process stdout/stderr is bounded at 1 MiB each; raw streams are not
retained in drafting history. Configured wall time bounds the process transport.

The trusted harness must enforce read-only repository access and protect control
storage. This configuration assertion and a process group are not a sandbox.
The adapter owns containment of descendants and remote jobs. Unknown execution
blocks replacement; this slice has no operator bypass or recovery command.

## API and browser

All routes use the existing session token and local-origin checks:

- `POST /api/drafting`: `{request_id: UUID, prompt: string}` starts one request.
- `GET /api/drafting`: lists retained attempts.
- `GET /api/drafting/{request_id}`: reads one attempt.

Reuse a request ID only for the same prompt and configuration. Repeating it returns
its recorded state without another launch; conflicting reuse refuses. One attempt
runs at a time. A restart during execution reports unknown instead of assuming it
ended. History uses immutable checksummed initial/final records, capped at 32
attempts; checksums detect accidental damage and are not authentication.

In the webapp, Generate draft starts the configured adapter. Ready results offer
Use as new editable draft. Unsaved editor changes prevent replacement. Inspect,
edit, and Save draft through the ordinary authoring route. Saving never starts a
coding agent or awards assurance. `saved: false` in an attempt means generation
itself did not save; later authoring records are separate.

## Evidence bounds

Synthetic adapters test real HTTP, subprocess, SDK and storage behavior. They do
not prove a real model follows the skill, a cloud budget is measured, or a harness
sandbox works. Actual local/cloud adapter observations, browser qualification and
remaining Phase 3 evidence must be retained separately before release.
