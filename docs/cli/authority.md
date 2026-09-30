# Signed authority changes

Candidate SDK and reference workflow, OW-WAR-0096. Agent edits are proposals.
Only a signed transition checked against previous trusted authority changes the
new store. This does not migrate the legacy `war sign`/roles.toml path automatically.

## Prepare, review, approve

Use a dedicated operator/broker environment with a trusted installed `war` binary.
The execution agent must not control that account, its environment, the authority
store, the verifier, SSH signing socket or private keys. Store directory must be
owned by the operator, mode0700, with ancestors not writable by the execution
account or group/other. `/usr/bin/ssh-keygen` must be a trusted system executable.
Do not expose unrestricted sudo or a service that accepts arbitrary store paths.

An agent can draft canonical JSON without editing TOML manually:

```sh
war authority draft --repository example --principal owner \
  --public-key owner.pub --role authority-admin --role authority-recovery \
  --emit baseline.json --json
```

This returns a revision digest and grants nothing. The operator independently
checks the baseline, key fingerprint and repository identity. In the protected
operator environment, initialize an empty store using that reviewed digest:

```sh
war authority bootstrap --store /var/lib/openwarrant/example \
  --revision baseline.json --expected-digest sha256:REVIEWED_DIGEST \
  --agent-uid 1000 --legacy-dir /path/to/repository/docs/authority
```

Create the protected directory first through normal administrator provisioning.
`--legacy-dir` is optional; when used it retains exact roles.toml and allowed_signers
bytes without interpreting them as new grants. Bootstrap refuses an existing store
and does not contact a model, infer a root key, or sign a human acceptance record.
Keep an independently protected backup; recovery keys must be configured in advance.

Export current state for an agent, then draft a replacement principal's exact roles:

```sh
war authority status --store /var/lib/openwarrant/example --emit current.json
war authority draft --current current.json --principal reviewer \
  --public-key reviewer.pub --role warrant-review --emit next.json
war authority propose --current current.json --next next.json --emit change.json
```

`draft --current` increments the sequence. It replaces the named principal's role
set; include every role that should remain. `--remove` removes a principal. At least
one administrator must remain. Repository identity cannot change in a transition.
The proposal output shows before/after principals and exact subject digests.

One operator command reviews, signs and activates the proposal:

```sh
war authority approve --store /var/lib/openwarrant/example --proposal change.json \
  --principal owner --key /protected/owner.pub --activate
```

The command prints the exact change before invoking OpenSSH. Configure key custody
and confirmation outside the execution agent, for example a protected SSH agent
with per-signature confirmation. A signature alone cannot establish human presence;
this adapter reports `human_review_established: false`. The application must obtain
actual human-control evidence before claiming a human assurance mark.

For separate signing and activation, use `approve --current current.json ...
--emit approval.sig`, then `activate --store ... --proposal change.json
--signature owner=approval.sig`. `check --current ... --proposal ... --signature ...`
checks against caller-supplied state without installing anything. No unattended
signing fallback is added. A stale proposal must be redrafted and approved again.

## Current-state use and recovery

`allows --store ... --principal reviewer --role warrant-review --expected-head
sha256:HEAD` refuses missing grants and stale heads. Caller identity is a separate
trusted fact: passing a principal name does not authenticate it. The workflow must
bind the authenticated actor and current decision to the protected action; it must
not fall back to repository roles when this check fails. This reference CLI does
not reroute existing legacy privileged commands or award verification marks.

A recovery proposal uses `propose --operation recover`; a key already holding
`authority-recovery` must sign. Recovery is a forward transition, not an unsigned
reset or replacement of trusted roots. Losing all administrator and recovery keys
requires explicit new trust bootstrap outside this chain; it cannot preserve the
claim of uninterrupted authority. Revocations affect future acts; historical
transitions retain their original key sets and signatures.

`history --store ... --emit history.json` exports bootstrap, captured legacy bytes
and signed transitions. Exports are audit data, not installation authorization.
The store uses a nonblocking exclusive lock and atomically replaces one synced
snapshot holding both history and head. On an interrupted/uncertain activation,
read `status` before retrying. Orphan `pending-*` files are never loaded as state;
an operator may inspect and remove them while no activation is running.

## Isolation and testing

Normal mode refuses configured execution UID equal to operator UID. Every store
command needs `--unprotected-test-store` when using a disposable same-account store.
Test mode cannot later be presented as normal protected mode. All outputs retain
`isolation_enforced: false`: POSIX directory checks cannot prove host account/ACL,
sudo, binary custody or deployment facts. The trusted host must establish those.

Linux reference runner:

```sh
scripts/authority-agent-sandbox.sh /dedicated/task-worktree -- /usr/bin/your-agent
python3 scripts/check-authority-sandbox.py
```

Install the runner and bwrap outside agent write access. Task workspace must contain
only task data, never authority state or signing material. The runner exposes that
workspace and read-only runtime libraries, clears environment, isolates processes
and network, and omits host home/signing sockets. Models needing network require a
separately designed broker; this example grants no network. macOS store/signature
code uses Unix APIs, but this Linux sandbox is not macOS containment evidence.

SDK: `openwarrant_core::authority_transition`. Candidate schema, canonicalization,
limits and domains: [ADR](../sas/drafts/1.0.0-rc.3/authority-transition.adr.md).
Current limits:64KiB records,128 principals,32 roles per principal,4096 transitions,
8MiB retained store. Capacity exhaustion refuses rather than deleting history.

## Activation observations

New activations atomically retain a receipt alongside history and head: proposal
digest, previous/new heads, principals whose prior-key signatures verified, process
operator UID and observed Unix seconds. Status/history expose these receipts.
They identify the verified keys and local process; they do not identify a human
reviewer or prove physical presence. `activation_time_authenticated` remains false:
a local wall clock is an observation, not an independently authenticated timestamp.

Older snapshots without receipts remain readable. `missing_activation_receipts`
reports the gap; the CLI never backfills invented actors or times. Receipt subject
mismatches refuse loading. The protected store retains observations; exports do not
cryptographically authenticate these receipt fields or transfer trust. Earlier CLI
versions may refuse snapshots containing the new optional receipt map.
