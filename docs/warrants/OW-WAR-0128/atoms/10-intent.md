---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f6e-7d90-8eed-cf0953b77657
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The product spec names the Knowledge Fabric Compiler as the first workflow
integration: "Knowledge Fabric Compiler → OpenWarrant compiler → Knowledge
Fabric Compiler → consuming workflow". It also says the interface, "the
responsibility for each input and output", and the acceptance evidence
"remain to be designed" (`docs/design/openwarrant-product-spec.md`).

The SAS gives a shape and no contract:

- §81: an *illustrative* `compile(request) -> result`, with nine request
  inputs (§81.1) and nine result fields (§81.2);
- §83.1: KF SHOULD invoke an exact pinned OpenWarrant binary through
  canonical JSON; §83.2 lists the pins; §83.3 the sandbox (no network, no
  database credentials, no ambient source discovery, bounded I/O, supplied
  bytes only).

Neither side implements it:

- OpenWarrant has no request or result type. `war compile` discovers a
  repository and writes projections into it, which §83.3 rules out.
  `war sdk` is an offline JSON-in, JSON-out shell, but none of its
  operations is a §81 compile.
- Knowledge Fabric (commit `3d3c871e`) runs a *document* compiler over
  `kf-document-v1` to Liminal, and v1.0 ships none (KF ADRs 0002, 0010). No
  KF code starts an OpenWarrant process.
- KF ADR 0019 says KF records what OpenWarrant computed — contract digest,
  Compilation Basis, canonical IR — does not recompute, and never stores
  authored atoms. That reads as "the repository compiles". The product
  spec reads as "KF supplies the inputs". The two have not been reconciled.

## Desired Outcome

An interface both owners can accept or reject, written so an implementation
Warrant on each side can test against it:

- who invokes whom, as the owners decide (Q-001, Q-002);
- the request document: each §81.1 input carried in v1, deferred with a
  reason, or refused;
- the result document: each §81.2 field, the same way;
- the process contract: stdin and stdout, exit codes, size bounds, and the
  refusals §83.3 implies, each with the input that triggers it;
- the §83.2 pins the caller records;
- every digest named with its algorithm and domain (§65, RQ-081);
- one worked request and result, and one refused request and its error, as
  JSON that parses;
- each OpenWarrant element traced to existing code or marked "to build";
  each KF element cited at a commit or listed as a question for KF's owner.

A proposed ADR records the decision and the rejected alternatives.

## Non-goals

- Implementing either side. No code, schema-pack or CLI change.
- Generated TypeScript types (§83.4; OW-WAR-0110).
- The Liminal adapter (§82.2) and any KF change.
- Registration and authority (OW-WAR-0029, OW-WAR-0126).
