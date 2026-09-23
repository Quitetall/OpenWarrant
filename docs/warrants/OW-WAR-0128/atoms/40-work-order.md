---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f6e-7d90-8eed-cf0953b77657
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `docs/integrations/kf-compiler.md`: the interface.
   - **Who invokes whom**, as Q-001 and Q-002 were answered.
   - **Request table**: one row per §81.1 input (protocol, Source Holder
     snapshots, manifest, supplied source bytes, bound records, Workspace
     Basis, policies, requested targets, schema pack). Each is carried in
     v1, deferred with a reason, or refused.
   - **Result table**: one row per §81.2 field (canonical IR, semantic
     digest, dependency digest, diagnostics, unresolved refs, omitted
     subgraphs, source maps, conversion loss, compiled views), the same way.
   - **Process contract**: stdin and stdout, exit codes, size bounds. For
     each refusal: the input that triggers it, the error code, and the exit
     status. At least: a path given instead of bytes; an unknown protocol
     version; input over the bound; a request needing the network.
   - **Pins** the caller records (§83.2).
   - **Digests**: each named with algorithm and domain (§65).
   - **Examples**: one request and its result; one refused request and its
     error. Each is a fenced JSON block that parses.
   - **Status column**: each OpenWarrant element names the code it comes
     from, or says "to build"; each KF element cites a KF file at a commit,
     or sits in "Open questions for KF".
2. `docs/adr/atoms/OW-ADR-0027-kf-compiler-interface.md`, status
   `proposed`, `governs` this Warrant. It records the answers to Q-001 to
   Q-003, the protocol names, and the options not chosen with reasons.

## Frozen Surfaces

- All code, the schema pack, and the CLI.
- `oh.war/report/v1` and every existing record schema.
- Knowledge Fabric.

## Autonomy and Escalation

Tier T2. The performer drafts only after Q-001 to Q-003 are answered.
Escalate rather than decide:

- any §81 field whose disposition the answers do not settle;
- any KF behaviour the performer cannot cite;
- any change to an existing OpenWarrant document shape.

## Rollback

Delete the two files. Nothing else changes.
