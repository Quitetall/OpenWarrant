# bonsai qualification-plants unit test fails under concurrent load: scope_findings[0] was not outside-warrant.txt

## Notes

- **2026-10-07 02:18 UTC, claude:** 2026-10-06, merged M4+M8 tree 80d905a4: failed once in cargo test --workspace (bonsai.rs:1221, assert scope_findings[0].path == outside-warrant.txt) while two other builders' test runs and builds loaded the machine (load ~33); passed alone and in two further full runs (341/341). Fixture dirs are pid+counter unique and none were left in /tmp, so not stale-dir reuse; the fixture runs git worktree add against the shared repository, which concurrent test runs in other worktrees also do. Unproven.
