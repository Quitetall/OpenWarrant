# Effective authority-reader access — performer evidence

## Reproduced defect

The namespace worker runs with real/effective UID 1 and effective `CAP_DAC_OVERRIDE`. It writes an operator-owned read-only probe while `access(WRITE_OK)`, using real identity, denies write. Before repair, active collector loading reports `accepted=true` and the refusal assertion fails (exit 101). The failure log is retained.

## Repair and observations

Authority-store readers and collector loading now share a permission observation using `accessat(CWD, path, WRITE_OK, EACCESS)`. Normal reader guards apply it to every store/state ancestor for the actual execution account; test-store guards also use effective permissions. Unexpected observation errors produce unavailable results, not a false read-only observation.

The same capable worker now observes `accepted=false`, with the precise writable-executor refusal. Ordinary capability-free UID 1 still loads valid active enrollment and refuses direct writes, changed selection, account mismatch and signed revocation. Existing sealed-image path-replacement and missing-account UNKNOWN controls remain intact. Three store controls and four crypto controls pass; the namespace fixture is explicitly run after its default ignored listing. All 445 CLI unit regressions pass. Strict all-target/all-feature CLI Clippy passes.

A broad unit invocation was interrupted while waiting for the build lock, before test execution. Its log is retained as an unavailable observation, not a test verdict; the run is restarted after the binary build completes.

No host account, authority store or actual key is changed. The writable probe and capabilities exist only in the disposable user/mount namespace. No human acceptance, independent verdict, deployment or participant qualification is issued.
