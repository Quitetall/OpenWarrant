# OW129 dispatch budget scope

The implemented scope has current performer evidence: **15 CLI controls and one
compiler boundary test passed**. The exact source and binary identities, bounds,
logs and historical delivery comparison are in [observation.json](observation.json).
This is an implementation audit. It does not award assurance or resolve the Warrant.

| Required behavior | Current observation |
| --- | --- |
| OBL-001: compiler owns the estimate and refusal | CLI comparison absent; compiler comparison present. Public dispatch carries estimate, budget and method. Compiler test covers in budget, exactly at budget, one token over, missing token inputs, the largest items and digest binding. |
| OBL-002: refuse before an actor starts | Over-budget `perform` leaves no marker, Dispatch or compiled journal event; its in-budget control starts. `--all` runs only the admitted stage and names the refused stage. Over-budget service `run` leaves no receipt; its in-budget control mints one. |
| OBL-003: check a packet before bundling | Missing tokens and an over-budget edited packet each receive the named refusal and leave no bundle; an unedited packet is bundled. |

The CLI control checks the unchanged token-account formula against selected source
bytes. It does **not** execute the historical CLI and compare its packet to the
current CLI. The estimate remains bytes divided by four, rounded up; no accuracy
claim is made against a model tokenizer. All actors in these controls are fixtures.
The retained debug binary has a dirty build identity; its exact hash is recorded.

## What still prevents closure

The read-only `war sign resolve OW-WAR-0129 --dry-run --json` returned **exit 2**:
6 PASS, 2 WARN and 8 UNKNOWN, with zero ERROR. Its actual report is
[closure.json](closure.json). Current runtime-stage receipts match the basis, but
artifact digests, required obligation dispositions, admissible required gates,
unknowns/blockers, required judgments, independence and risk authority do not all
establish closure.

The two warning lines identify D-001 and D-002 as moved under OW133's later
ownership. These historical records stay intact. The manifest comparison records
all four old delivery hashes beside today's source hashes; it does not repin them,
infer producers from Git, accept a replacement or change an independent finding.

Next: reconcile these historical deliveries under the applicable authority, obtain
an admissible exact-subject registered gate result and independent reassessment,
then present human acceptance. The implementation report stays **in progress**
until those scope and history obligations are settled.
