# OpenWarrant 1.0.0-alpha.2 — authority is enforced on the read path

**Replace 1.0.0-alpha.1.** In that build an authority record was believed for
its contents: a `docs/warrants/<alias>/authorization.toml` written by hand,
saying `authorized` and naming anyone as the authorizer, passed `war check` with
zero errors and satisfied §56.1's first resolution requirement. Nothing asked
who signed it. Demonstrated against the published alpha.1 binary on 2026-09-19
with a record whose `meaning` field read "FORGED BY AN AGENT. No human saw
this".

In this build every authorization, resolution, correction and SAS acceptance is
read through one question: is there a response for THAT act, over THAT digest,
whose `.sig` `ssh-keygen -Y verify` accepts as the principal
`docs/authority/roles.toml` binds to the actor, and is that actor `human` there?
A record without one is `ERROR authority.unsigned`, resolution requirement 1 is
unmet, and `war dispatch` refuses to compile a packet for it. A missing
`ssh-keygen` is `authority.verify-unavailable` — never a pass.

What did not change: an agent still drafts every record, request, reason and
batch. The human act is one signature over bytes already prepared, and
`war sign --all --ssh-sign` does a whole backlog in one pass. Prototyping
before authorization stays available and now says so — `--prototype` on
`dispatch`, `run` and `perform` stamps `prototype://unauthorized` into the
packet.

If you have records made under alpha.1, `war sign --list` offers each as an
ordinary pending act that repeats what the record already says; nothing is
rewritten and nothing is back-dated.

This preview provides the `war` CLI, offline SDK operations, readable Warrant
records, agent instructions, terminal review and offline/live progress views.
Prompt-authorized coding and record keeping do not require SSH signing. Explicit
repository access rules and verified-start requirements still apply.

## Install

Download the archive for Linux x86-64 or macOS Apple Silicon and its `.sha256`
file from this release. Verify the checksum before extracting:

```sh
sha256sum -c openwarrant-v1.0.0-alpha.2-linux-x86_64.tar.gz.sha256
# macOS: shasum -a 256 -c openwarrant-v1.0.0-alpha.2-darwin-arm64.tar.gz.sha256
mkdir openwarrant-alpha2
tar -xzf openwarrant-v1.0.0-alpha.2-linux-x86_64.tar.gz -C openwarrant-alpha2
./openwarrant-alpha2/bin/war --version
```

Use the absolute path to that `bin/war`, or add its directory to PATH. Check
`command -v war` so an older global install does not take precedence. Keep the
bundle intact. Extract `sdk-source.tar` into a separate source directory for
SDK source, skills, examples and full documentation. No global install is required.

## Start once per repository

```sh
war init --namespace APP --name "My project"
war new "Describe one outcome"
war compile
war overview --serve
```

Open the local URL printed by the last command. `war overview --html` writes an
offline snapshot. Existing repositories already containing `openwarrant.toml` do
not need another initialization. Existing AGENTS.md is preserved; integrate the
prototype-first guidance rather than overwriting custom instructions.

Ask your connected agent to implement the work, run checks and record its result
as unverified. The commands above create and display records; they do not start
an autonomous coding agent. `war document draft --help` exposes interactive and
noninteractive document authoring; `war sdk --help` exposes offline SDK operations.

For approvals, use `war console` or `war sign APP-WAR-0001 --ssh-sign`. Terminal
confirmation is not cryptographic human presence: only an ssh-signed act
verifies on the read path, so a tty-signed record still reports
`authority.unsigned`. Load the signing key with `ssh-add -c` — `war` cannot
check that you did, and without it any process holding `SSH_AUTH_SOCK` can sign
silently (`docs/THREAT_MODEL.md`, entries 1 and 11).

## Limits

This alpha is not Stable 1.0, SAS acceptance, Verified qualification or permission
to deploy. Legacy governance commands remain available and enforce their own
requirements. Existing signed records remain unchanged. A complete guided setup
wizard, all external agent/provider integrations, and final release qualification
remain unfinished. Experimental archive APIs may change. No crates.io packages
are published by this preview workflow. Candidate manifests intentionally retain
`qualified: false` and `promotable: false` for stable-release qualification.

Authority enforcement is new in this preview and the corpus it was measured on
is this repository's own: 174 verified signatures, 320 planted violations each
rejected by its intended control, `cargo xtask gate` green in 14 steps. It has
not been run against anyone else's corpus.
