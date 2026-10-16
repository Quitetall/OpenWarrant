# Compact verification against an immutable Git candidate

The existing acceptance battery now includes an actual new in-scope input change that its prior v2 verification cannot clear. A captured compact index and its retained archive supply a synthetic v3 response through the public CLI; no newer request or digest is invented by the fixture helper. The immutable Git reader accepts that exact reviewed candidate without changing the original resolution.

Adding a newline to its retained view makes the new candidate lose qualification, while reading the earlier exact Git commit still reports it re-verified. This pairs the positive control with refusal and proves the candidate reader does not borrow mutable checkout view bytes. All three new cases passed on the first run. The broader run reported 30 passed and one failed: its initial current-corpus generated-projection check detected drift after source changes. That failure remains in the raw log; the final run must pass after the source/projection update. No production verdict, resolution or human act was recorded; signing uses only the battery's own disposable corpus and throwaway key.

Raw logs and exact runner: `/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/compact-candidate-20261016/`.

The final acceptance run passed all 31 controls, including the current-corpus generated check. The source change correctly moved OW-WAR-0134/D-005's displayed delivery status from verified to drift; compiling the updated sources preserved that real status instead of refreshing its old digest. No delivery provenance was transferred or rewritten. The fixture helper's legacy v2 path also passed the existing controls. The first failed run remains retained beside the final log.
