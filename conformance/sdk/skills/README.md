# Fixed skill artifact acceptance

These cases test the **artifact boundary**, not model skill discovery, stochastic
output quality, token savings or human effort. They do not install instructions,
execute work or grant authority. Current candidate scope: S08 / SDK-07.

`artifacts.json` contains agent-authored SDK inputs for discovery questions, a
Warrant, stage context, an unresolved decision map, an ADR and a migrated draft.
The driver authors and validates real RC.3 bytes through `war sdk`, resolves exact
units, and rejects removed constraints, repeated settled questions and false
maturity fields. The stage context accompanies `stages.json`, an actual legacy
milestone proposal; it does not replace that executable graph. A separate public
milestone parser test observes dependency-cycle refusal. The legacy proposal
validator alone checks a narrower boundary and does not catch that cycle.
A supplied verification record retains candidate, performer/verifier identities,
`not-established` disposition and missing trust; candidate mutation refuses. Detailed
self-verification and human-spoof controls are exercised by the SDK record suite.
Migration authoring here is a new document boundary; byte-preserving exchange and
lineage are exercised by the legacy SDK suite, not claimed from a path in prose.

`context_entry.py` is a pure reference adapter for fixed edit proposals. It accepts
explicit host/source bytes, digests, scope and resolved/approved target facts.
It returns proposed bytes and identities, never writes, scans or authenticates.
Tests preserve unrelated UTF-8 host content and nested scope, reuse an unchanged
pointer, remove only its unchanged owned block, and refuse modified entries,
missing sources, stale bytes, conflicts and unapproved resolved targets. An input
without a final newline refuses instead of changing existing bytes. A real host
must resolve symlinks, supply trusted target permission and recheck inputs before
writing; these fixtures cannot establish those external facts.

```sh
python3 conformance/sdk/skills/check_artifacts.py --war target/debug/war
python3 -m unittest discover -s conformance/sdk/skills -p 'test_*.py'
cargo test -p openwarrant-core --test skill_artifacts
```

Actual harness invocation, host installation, workflow execution, real-user
friction and token comparisons remain later observations. No artificial user count
or model quality score is derived from these fixed cases.
