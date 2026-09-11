# Quickstart — from an empty directory to a resolved Warrant

Every step is one command. Two of them are a human's and only a human's; they
are marked. Times are what the tool takes; the human steps take as long as
reading takes.

## 0. Install

```bash
cargo install --path crates/openwarrant-cli     # or a release tarball; `war --version`
```

## 1. Scaffold the program (agent or human, seconds)

```bash
mkdir demo && cd demo && git init
war init --program "Demo" --namespace DM
```

You now have `openwarrant.toml`, `AGENTS.md`, `docs/sas/Demo_SAS.md` (three
sections the tool reads, everything else yours to write), the authority
examples, the `war check` gate, and `DM-WAR-0001 — Adopt OpenWarrant in Demo`
with real atoms. `war check` exits 0.

## 2. Say who may sign (human, once)

```bash
cp docs/authority/roles.toml.example docs/authority/roles.toml          # edit: your name, your roles
cp docs/authority/allowed_signers.example docs/authority/allowed_signers  # edit: your key
ssh-add -c ~/.ssh/id_ed25519      # -c: every signature asks you. Test Deny before you trust Allow.
```

No tool writes these two files. Read the comments in the examples; the one
thing `war` cannot check is that the key was loaded with `-c`.

## 3. Accept the SAS

```bash
war sas propose 0.1.0            # agent: records the document's digest and its §106 rows
war sign 0.1.0 --ssh-sign        # HUMAN: one dialog
```

## 4. Authorize the first Warrant

```bash
war check && war compile && war check --generated
war authorize DM-WAR-0001        # agent: the request, showing what a signature would mean
war sign DM-WAR-0001 --ssh-sign  # HUMAN: one dialog; an attestation is written beside the record
```

## 5. Deliver, record evidence, get it verified by someone else

```bash
war pins --resolved-only         # what you may not edit (nothing yet)
# do the work; declare each file in docs/warrants/DM-WAR-0001/deliverables.toml
war evidence record DM-WAR-0001  # runs the cited gate, mints a §44.6 receipt
war verify DM-WAR-0001 --performer claude > request.toml
# hand request.toml to a SEPARATE context — another session, another model, a person
war verify DM-WAR-0001 --response verdicts.toml
```

`war verify --response` refuses a verdict whose verifier is the performer and
writes nothing.

## 6. Resolve

```bash
war resolve --dry-run DM-WAR-0001   # the thirteen §56.1 requirements, honestly
war resolve DM-WAR-0001             # agent: the request
war sign DM-WAR-0001 --ssh-sign     # HUMAN: one dialog
war status                          # DM-WAR-0001: resolved / RQ-001: satisfied
```

At every step `war next` says whose act comes next. It never hands an agent a
signature.

## From an agent harness

Claude Code: add this repository as a plugin (`claude plugin marketplace add
<path>`, `/plugin install openwarrant@openwarrant`) — the skill, the MCP server
and two hooks (a pin guard on edits, a stop check on `war check`). Any other
harness: `war mcp` over stdio; `war mcp --describe` lists the tools and what
is deliberately not one.

## Worked examples

`docs/EXAMPLES/` — a code Warrant walked end to end from this repository's own
records.
