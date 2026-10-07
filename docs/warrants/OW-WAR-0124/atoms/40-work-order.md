---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f0f-7e31-bed2-efca03dd781e
role: work_order
jurisdiction: authored
order: 40
classification: internal
---


# Work Order

## Deliverables

1. `crates/openwarrant-core/src/config.rs`: an optional `[adoption]`
   table with `baseline` (a full commit id). Absent when unset, and never
   serialized when absent, so existing `openwarrant.toml` files load and
   write unchanged.
2. `crates/openwarrant-cli/src/init/mod.rs` and `crates/openwarrant-cli/src/lib.rs`:
   - when `HEAD` resolves to a commit, `war init` records the baseline
     (HEAD, or `--baseline <commit>` resolved to a full id);
   - `--baseline` naming something that is not a commit in this history
     is refused before anything is written;
   - with no commits, no `[adoption]` table and no change in output;
   - the adopt Warrant's Basis names the baseline and says what it means;
   - an ADR directory found under `docs/adr`, `doc/adr` or `adr` produces
     one line naming the `war migrate` command. Nothing is run.
   - (AM-001, the owner's decision of 2026-09-24) `war init` and `war init
     --program`, run where the directory is not inside a git work tree,
     run `git init` there first and say so in one line. Inside an existing
     work tree nothing is initialized: no nested repository. With git not
     installed, init continues and says that identity and journal history
     cannot be checked until the directory is a git repository
     (OW-WAR-0119's `identity.changed` reads UNKNOWN there, by design).
3. `crates/openwarrant-cli/templates/adopt/20-basis.md`: a
   `## Adoption baseline` section with `{{baseline}}`, and the sentence
   that nothing before it is claimed, owned or verified. Rendered only
   when a baseline exists.
4. `crates/openwarrant-cli/src/init/guided.rs`:
   - `Facts` gains the commit count, the proposed baseline and any ADR
     directory found;
   - after the Program step, a Baseline step when there are commits: it
     shows the count and the commit and asks to confirm or name another;
   - the answer becomes an `Effect` that writes `[adoption]` once;
   - no authority step, question or file changes.
5. `crates/openwarrant-cli/src/telemetry.rs`:
   - with a baseline, `untracked_candidates` reads `<baseline>..HEAD`;
   - a Warrant citation matches `<namespace>-WAR-` for the repository's
     namespace, or `war://`;
   - the report states the baseline it used, or "all history".
6. `docs/ADOPTING.md`: the brownfield path, each step one command, the
   human steps marked, and what the baseline does and does not claim.
7. `conformance/plants.d/59-adoption.sh`, on a scratch repository with
   three commits and namespace `ACME`:
   - `war init --program Acme --namespace ACME` records HEAD as the
     baseline, and `war check` exits 0;
   - telemetry lists 0 candidates; one new commit without a citation
     makes 1; a commit whose subject cites `ACME-WAR-0001` adds none; a
     commit citing only `OW-WAR-0001` is a candidate;
   - `war pins` lists no path committed before the baseline;
   - `--baseline 0000000` is refused, with no `openwarrant.toml` written;
   - a scratch repository with `docs/adr/0001-x.md` gets the
     `war migrate` line, and no import artifact is written;
   - a repository with no commits gets no `[adoption]` table.

## Frozen Surfaces

- The authority steps of `init/guided.rs`: the Signer and KeyLoaded
  questions, the written `roles.toml` and `allowed_signers`, and the rule
  that an existing authority file is never written.
- `war migrate` and its artifact.
- The scripted scaffold on a repository with no commits (`99-init.sh`,
  `75-init-program.sh`).
- Every record schema and the schema pack version.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any change that would touch an authority file or question;
- any record, rather than configuration, for the baseline;
- any change to this repository's telemetry artifact.

## Rollback

Remove the `[adoption]` field, the Baseline step, `--baseline` and the
template section; restore `untracked_candidates`. An `openwarrant.toml`
that already records a baseline still loads, because unknown tables are
ignored by the reader as it stands.
