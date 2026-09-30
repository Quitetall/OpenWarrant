# Security policy

## Reporting a vulnerability

Report privately via GitHub's **[Security Advisories](https://github.com/Quitetall/OpenWarrant/security/advisories/new)**
on this repository. Do not open a public issue for anything security-relevant.

Include the exact input, the command, the output, and the toolchain version
(`rustc --version`). A reproducer is worth more than a description.

You will get an acknowledgement. This is a small project with one maintainer, so
please do not expect a same-day response; if you have heard nothing in two weeks,
send a reminder through the same channel.

## Supported versions

Pre-1.0. Only the tip of `main` is supported. There are no backports, and there
is no version whose wire format is stable enough to promise compatibility for.

## What counts as a vulnerability here

OpenWarrant reads untrusted files and produces documents that people rely on to
be true. The interesting failures are therefore about **false claims**, not only
about memory safety:

- **Manufacturing a false PASS.** Any input for which `war check` reports
  well-formed while a rule it claims to enforce is violated. This is the most
  serious class in the project.
- **Digest collision or ambiguity.** Two semantically different Warrants
  producing the same contract digest, or a digest that changes when it should
  not (or fails to change when it should).
- **Canonicalization divergence.** Output disagreeing with RFC 8785 on any
  input, which would make cross-system verification silently wrong.
- **Parser exploitation.** Resource exhaustion or unexpected behaviour from a
  crafted manifest or atom. The frontmatter reader is deliberately a restricted
  subset rather than a YAML parser (OW-ADR-0002) specifically to keep this
  surface small; a way around that restriction is a finding.
- **Path escape.** Any manifest path that reads or writes outside the repository.

`#![forbid(unsafe_code)]` is set on every library crate, so memory-safety issues
would have to come from a dependency.

## What does not count

- A `war check` verdict you disagree with on interpretation, absent a SAS
  citation. Open an issue.
- Anything requiring an attacker to already have write access to the repository.
  A repository you can edit is a repository you control.
- Denial of service from a genuinely enormous but well-formed input.
