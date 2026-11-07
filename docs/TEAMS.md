# Teams: review assignment and a queue per person

`docs/authority/roles.toml` already names more than one person, and `war sign`
already computes who is eligible for each pending act. This page is about the
two things a team needs beyond that: saying **who** reviews a given Warrant,
and each person seeing **what is waiting on them** (OW-WAR-0137).

## The one rule: an assignment narrows, it never grants

`roles.toml` stays the only source of who may sign at all (§27.4). An
assignment picks, for one Warrant, a subset of the people the register
already allows. It can remove signers from an act. It cannot add one:

- an assigned actor who does not hold the role is refused, not granted it;
- an agent named to resolve is refused by kind (§27.2), whatever the file says;
- the performer named as its own verifier is refused (§51.2).

If code ever puts someone on an eligible list because an assignment named
them, that is a defect.

## Assigning

Write `docs/warrants/<alias>/assignment.toml` before the Warrant is
authorized:

```toml
schema = "oh.war/assignment/v1"
verify = ["Ada"]
resolve = ["Ben"]
```

Either key may be left out; the act it names is then not narrowed. One or more
actors per act. Unknown keys (`authorize = [...]`, a misspelling) are refused
rather than read as "no assignment": the authorizer is who signs the
assignment, so authorization itself cannot be assigned.

`war authorize <alias>` lists the assignment and its digest beside the
deliverable set, and `war sign` shows it on the screen being signed and echoes
the digest into the response. **The human who authorizes signs who reviews.**

At ingest:

| what happened | refused as |
|---|---|
| the response signs a different assignment than the file now holds | `authorize.stale-assignment` |
| the Warrant has an assignment and the response echoes none | `authorize.assignment-unsigned` |
| an assigned actor lacks the role, or has no entry | `assignment.role-missing` |
| an agent is named to resolve | `assignment.agent-prohibited` |
| the performer is named to verify (or resolve) | `assignment.self-verify` (`assignment.self-act`) |

Nothing is written when any of these fire. `war sign <alias> --dry-run` shows
the same refusals without a key.

## After authorization

The digest the authorizer signed travels in the signed response under
`docs/authority/responses/`, and ingest copies it into the authorization's
journal event. The signed response is what is believed; the journal is read
only when no response is on disk. From then on the file is compared with it:

- **edited, added, or removed** after signing: `assignment.moved`. Nobody may
  resolve or verify until the file is restored, or the change is amended and
  re-authorized (§31). A removal is refused like an edit — deleting the file
  would otherwise put every eligible signer back, which is a widening.
- **an assignee loses the role** in `roles.toml`: `assignment.role-revoked`.
  Nobody else is substituted; restore the role or re-assign and re-authorize.

`war check` reports both by name: `assignment.moved` (and
`assignment.malformed`) as errors, `assignment.role-revoked` as a warning, and
`assignment.signed` when the file is the one signed.

A Warrant with no `assignment.toml` behaves exactly as before.

## Who signs

On an assigned resolve act, `war sign <alias>` with no `--as` picks the
assigned resolver; `--dry-run` reports `sign.signer` naming them. An eligible
resolver who is not assigned is refused `sign.not-assigned`. `war sign --list`
marks an assigned act `[assigned: …]`, or the rule that blocks it. A
hand-written `war resolve <alias> --response` naming anyone else is refused
`resolution.not-assigned` before anything is written.

On an assigned verify act, `war verify <alias> --response <file>` records a
verdict only from the assigned verifier (`verify.not-assigned` otherwise), and
only while that actor holds `verifier` in `roles.toml` (`verify.role-missing`).
Refused verdicts are not written under `verifications/`.

## The personal queue

```
war sign --list --as Ada          # the acts Ada may sign now, assigned first
war inbox --as Ada                # the same inbox, filtered to Ada
war --json sign --list --as Ada   # the same list: target, act, eligible, assigned
```

Both read the same functions as the shared `war sign --list` and `war inbox`
and filter them, so a queue can only ever be a subset of what the register and
the assignments allow. Questions stay in every inbox: answering one is no
role's act. `--as` names whose queue to show; it signs nothing and proves
nobody's identity.

## Sharing

The queue is the repository, shared through git. There is no lock, server or
notification. Two people signing the same act in two clones write the same
response path and record, so the expectation (OW-WAR-0137's assumption A-001,
not yet measured) is that git reports a conflict when the second one merges.

## What assignment does not do

- It does not say who a person **is**. A name is not a login. `--ssh-sign`
  checks the key for authorize and resolve; a verifier's verdict is not yet
  signed, so on the verify path the assignment is enforced against the name
  in the response (OW-WAR-0138).
- It does not change who may sign by kind (§27.2). Agents stay refused.
- It does not claim two-person review. Assigning Ada and Ben records who was
  chosen; only the records of who actually signed and verified say who did.
  `war show <alias>`, `war status <alias>` and each Warrant's `review` in
  `war status --json` list who authorized, verified and resolved, and count
  the distinct humans among them (§27.4). None of them says "reviewed".

See `docs/THREAT_MODEL.md` entry 14.
