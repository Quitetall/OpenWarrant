# BLUT plan check — OW-WAR-0047's lowered PlanSpec, 2026-09-03

Binary: /mnt/4tb/LamQuant/training/cookbooks/lamquant/target/release/lqt (blut 0.2.0-alpha.1+23cd95ac5e18), built from training/engine at 23cd95ac; `plan check` landed in BLUT at 7b60d21e.
Command: war blut OW-WAR-0047 --verify <lqt> --emit evidence/OW-WAR-0047.planspec.json
Recorded by the performer; the verdict lines below are BLUT's own output, verbatim.

```
PASS blut.ports-mapped                  OW-WAR-0047: 3 port(s) map to kinds BLUT declares at 33b3e047fd8e
PASS blut.lowered                       OW-WAR-0047: lowered 2 stage(s) against a pinned registry (blut@33b3e047fd8e)
PASS blut.emitted                       OW-WAR-0047: PlanSpec written to /tmp/claude-1000/-mnt-4tb-LamQuant/a9f30445-ae55-49c5-bbc6-6949c830237e/scratchpad/0047.planspec.json
PASS blut.accepted                      OW-WAR-0047: BLUT accepted the lowering (fingerprint b69b886e4c9f, 8 recipes registered, exit 0) — reported by /mnt/4tb/LamQuant/training/cookbooks/lamquant/target/release/lqt

4 pass · 0 warn · 0 unknown · 0 error   (worst: PASS)

NOT CHECKED:
  · Verdict obtained from /mnt/4tb/LamQuant/training/cookbooks/lamquant/target/release/lqt — a separate program this repository does not control, whose refusal messages it cannot author. That is what makes it `authoritative_external` (§40.2) rather than a self-report (§51.3).

It is NOT an execution. OW-WAR-0047's OBL-001 asks for status, artifact and lineage receipts from a real BLUT run; a typecheck produces none of those, so this does not discharge it.

Note that every stage this repository names is a `STAGE-NNN` identifier that no cookbook compiles in, so a refusal naming an unknown stage is the expected and correct answer, not a defect in the lowering.

{
  "name": "OW-WAR-0047",
  "nodes": [
    {
      "stage": "materialize_dataset_path",
      "args": {
        "path": "conformance/fixtures/ow-war-0047-corpus.jsonl"
      }
    },
    {
      "stage": "filter_dataset",
      "args": {
        "min_turns": 2,
        "drop_errors": true
      }
    }
  ],
  "edges": [
    [
      0,
      1
    ]
  ],
  "version": 1
}

WELL-FORMED (record only — Preflight is not implemented, and `war check` does not run gates)
```
