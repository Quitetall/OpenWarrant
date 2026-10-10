# Protected runtime host identities

The SDK can now resolve repository and collector identities from an operator-owned `runtime-host.json` inside an existing protected authority store. `LoadedEnrollment::load_host` checks the actual execution UID and canonical workspace path, authenticates the already-activated signed enrollment, and returns an opaque `HostEnrollment`. Its `acquire_verifier` method uses those protected identities instead of caller-supplied principal labels. It records no activation, signature, verification verdict or assurance.

Local configuration example (experimental host settings, not a standard document or authority response):

```json
{
  "schema": "oh.war/runtime-host-config/v1-draft.1",
  "repository": "example",
  "repository_root": "/srv/example/checkout",
  "collector": "configured-collector",
  "execution_uid": 1001
}
```

The existing protected store must independently name the execution account. The configuration must be a regular, bounded, operator-owned file, with no symlink or executor write access. Its schema is explicit; duplicate fields, unknown fields, unsupported schemas and missing configuration do not silently select another identity. Before and after native execution, the retained host bytes are checked again alongside the existing authority/enrollment checks. A configuration change refuses the next launch or return of its result. These surrounding observations do not provide an atomic revocation fence.

## Observed controls

Rust 1.97.1 public signature tests passed (four tests; the two namespace entries are intentionally ignored in the ordinary invocation). The explicit namespace test then passed with a real operator UID 0 and unprivileged execution UID 1, software fixture keys and a sealed native executable. The executor could not overwrite the host configuration. Wrong workspace and wrong execution UID were refused. Changing the host UID mapping while preserving the signed collector activation refused the next launch. The original missing-method RED observation and final namespace output are retained alongside the exact source hashes.

This is performer evidence. It does not establish a production human act, independent verification, phase acceptance or launch sandboxing. Native input custody, trusted policy selection, actual CLI/workflow wiring, provider confinement/spend observations and independent qualification remain. Callers using the older explicit-identity SDK constructors remain responsible for authenticating their supplied identities.

Final collector-loading/signature regression: seven passed, two intentionally ignored; the protected namespace case was run separately and passed. CLI all-feature/all-target Clippy with warnings denied, format checking and diff whitespace checking passed. Full protected-main CI and independent qualification remain.
