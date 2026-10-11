---
format: openwarrant/1.0
profile: delivery
alias: OW-WAR-0146
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
summary: A verification bundle carries what each gate printed.
x.note: reviewed on paper
com.example.cost-centre: '4410'
---
# A verification bundle carries what each gate printed

## Intent
The blind verifier decides from the bundle alone, so the bundle carries
what each gate printed.

## Basis
The verification bundle format, and the gate runner's output files.

## Work order
Deliverables: the bundle writer. Frozen: the gate runner.

## Milestones
## M-1 · milestone — the bundle carries output

## Assurance
## OBL-001 · obligation — a bundled gate run carries its output
scope: `war verify --bundle` on a scratch Warrant with one recorded run.
  No claim about bundles with several runs.
gate: gate://ops.conformance.plants@1.1.0
part_of M-1
evidence:
  - the run entry's `stdout.sha256` equals the file's sha256;
  - with `max_excerpt_bytes` below the file size, `truncated` is true.
refusal: with the stdout file removed, `text` is absent, never "".

The bundle is read by a verifier who saw nothing else.
