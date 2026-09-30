# `war preflight`

Assess legacy Warrant readiness without changing records or running work:

```sh
war preflight OW-WAR-0011
war preflight OW-WAR-0011 --json
```

The command lists all 41 checks in SAS §32, grouped into contract, context,
graph, runtime, gates and authority. Each check reports `pass`, `fail` or
`unknown`, with its observation. A dimension reports the worst of its checks:
`fail` takes precedence over `unknown`, then `pass`. Unobserved checks remain
`unknown`; they are never treated as optional or successful.

This is a **partial local assessment**, not full Preflight qualification. The
command validates local manifests, loads atoms, parses milestone graphs and
compares a freshly compiled contract digest with a recorded commitment. That
comparison does not authenticate its authorizer. Local Git or filesystem access
cannot establish the eventual actor's runtime, provider, secrets, network path,
protected gate environment or side-effect authority. Those checks remain unknown.

Core `PreflightReceipt::readiness()` determines readiness from the observations.
Any failed or unknown check produces `not_ready` and a reported `blocked`
execution condition. Exit status is 2 while not ready. No receipt is ingested;
no lifecycle phase, execution condition or permission is persisted. This does
not add a universal start gate to prompt-only prototype work. Explicit action
gates and repository permissions still govern that work.

JSON uses the existing `oh.war/report/v1` envelope. Its `result` contains the
candidate `oh.war/preflight-report/v1` view: Warrant alias, optional compiled
contract digest, readiness, execution condition, `state_persisted: false`,
meaning, limits and six dimensions with named checks. This read-only result is
not an addition to the frozen schema pack or a signed Preflight receipt.

## Remaining OW-WAR-0011 scope

The live actor path and complete gate/authority observations are not implemented.
OW-WAR-0011 therefore remains partial, as its recorded basis requires. The old
`war check` success text still says Preflight is not implemented. Its source is
pinned by resolved OW-WAR-0005/D-002 and needs a human-signed correction before
editing. `war check` still proves record well-formedness, not execution readiness
or successful gates; that distinction must remain after any wording correction.

Tests in `crates/openwarrant-cli/tests/preflight_cli.rs` cover the public command:
all dimensions, unknown runtime refusal, stale commitment refusal, missing atom
and cyclic graph refusal, conservative multiple-graph aggregation, and absence
of repository writes. These fixtures do not establish a live execution path.
