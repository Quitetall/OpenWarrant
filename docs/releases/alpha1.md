# OpenWarrant 1.0.0-alpha.1 — usable prototype preview

This preview provides the `war` CLI, offline SDK operations, readable Warrant
records, agent instructions, terminal review and offline/live progress views.
Prompt-authorized coding and record keeping do not require SSH signing. Explicit
repository access rules and verified-start requirements still apply.

## Install

Download the archive for Linux x86-64 or macOS Apple Silicon and its `.sha256`
file from this release. Verify the checksum before extracting:

```sh
sha256sum -c openwarrant-v1.0.0-alpha.1-linux-x86_64.tar.gz.sha256
# macOS: shasum -a 256 -c openwarrant-v1.0.0-alpha.1-darwin-arm64.tar.gz.sha256
mkdir openwarrant-alpha1
tar -xzf openwarrant-v1.0.0-alpha.1-linux-x86_64.tar.gz -C openwarrant-alpha1
./openwarrant-alpha1/bin/war --version
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

For requested legacy approvals, use `war console` or `war sign APP-WAR-0001`.
Terminal confirmation is not cryptographic human presence. Secure signing is
separate from ordinary prototype work.

## Limits

This alpha is not Stable 1.0, SAS acceptance, Verified qualification or permission
to deploy. Legacy governance commands remain available and enforce their own
requirements. Existing signed records remain unchanged. A complete guided setup
wizard, all external agent/provider integrations, and final release qualification
remain unfinished. Experimental archive APIs may change. No crates.io packages
are published by this preview workflow. Candidate manifests intentionally retain
`qualified: false` and `promotable: false` for stable-release qualification.
