---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ef7-7862-8205-72d16ebcaf9f
role: work_order
jurisdiction: authored
order: 40
classification: internal
---


# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/check.rs`, the parent citation check. For
   each `[[parents]]` entry citing revision r at digest d, of a parent in
   this repository:
   - parent unauthorized: r must be 1 and d is compared with the current
     compile, as today;
   - parent authorized at revision R: r > R is an error,
     `relations.parent-revision`, "revision r does not exist";
   - d equals revision r's digest: pass. If r < R, also a warning,
     `relations.parent-moved`, naming R;
   - d equals the digest of another revision r' of the parent:
     `relations.parent-revision`, naming r', error or warning per U-001;
   - d equals no revision's digest: error, `relations.parent-digest`;
   - d equals the latest authorized digest and the parent's working
     contract compiles to something else: error,
     `relations.parent-digest`, with today's wording;
   - a digest that needs history the repository does not have: UNKNOWN,
     `relations.parent-revision`, saying why.
2. `crates/openwarrant-cli/src/new.rs` and `crates/openwarrant-cli/src/lib.rs`:
   `war new <title> --parent <alias>`.
   - Writes `[[parents]]` with `ref = "war://<uuid>"`, the parent's
     latest authorized revision and `contract_digest = "sha256:<digest>"`.
   - Refuses an alias that does not exist, and a parent with no
     `authorization.toml`, writing nothing.
   - A new function; `new::run`'s signature is unchanged, so its callers
     (`init`, `plan`) do not move.
3. `conformance/plants.d/58-parent-revision.sh`, on the real corpus with
   restore, and on a shallow clone:
   - OW-WAR-0002 cites revision 7: error, `relations.parent-revision`;
   - OW-WAR-0002 cites revision 1 at `ab7e2df7…`: pass, and
     `relations.parent-moved` names revision 2;
   - OW-WAR-0002 cites revision 2 at `f29c7d95…`: pass, no moved warning;
   - OW-WAR-0002 cites revision 2 at `ab7e2df7…`: the finding names
     revision 1;
   - a digest of all zeros: error, `relations.parent-digest`;
   - in a `git clone --depth 1` copy, the revision 1 citation is UNKNOWN,
     not pass and not error;
   - `war new --parent OW-WAR-0001` writes revision 2 at `f29c7d95…`;
     `--parent OW-WAR-9999` and `--parent <a draft>` are refused with
     nothing written.

## Frozen Surfaces

- `oh.war/manifest/v1` and `ParentRef`'s fields.
- Every signed manifest, including OW-WAR-0002 to 0005.
- `contract_history.rs`'s reading rules and limits.
- The existing plant in `00-corpus.sh`.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- the severity question (U-001) if it is not answered before authorization;
- any finding on a Warrant other than OW-WAR-0002 to 0005;
- any need to change `00-corpus.sh` or a signed manifest.

## Rollback

Restore the previous `relations.parent-digest` block and remove
`--parent`. No record is written by this work, so nothing else moves.
