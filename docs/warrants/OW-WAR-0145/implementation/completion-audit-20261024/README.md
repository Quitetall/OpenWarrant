# W145 implementation completion, unverified

The scoped implementation is complete on the checked revision. The previous
in-progress report mixed implementation status with the separate requirements
for qualification and resolution. Those requirements remain open.

[Protected CI 40073655945](https://github.com/Quitetall/OpenWarrant/actions/runs/40073655945/job/120812291596)
passed at `5a5f7ce6f3b0f6758c7971f6f4ef34ea42011508`, now in main through
`b54d55bff6b82263ba12825b49337a70fddb56eb`. Each declared implementation file
and the frozen 1.0.0 definition match that tested revision exactly. Source
hashes are in [the observation](observation.json); the original output is in
[the CI excerpt](ci-excerpt.log).

| Required implementation behavior | Observed control |
| --- | --- |
| OBL-001: askable isolated run | fake battery ran and passed; files, index, HEAD and uncommitted edits stayed identical; clone removed |
| OBL-001: refusal and tested subject | failing battery failed the gate; only committed HEAD bytes were visible; output named the tested commit |
| OBL-002: nested-run refusal | exit 2, named refusal, no clone made |
| OBL-003: version boundary | 1.1.0 registered as non-mutating; 1.0.0 matched its historical digest; one-byte mutation was detected; record check had zero errors |

The index-preparation success and refusal controls also passed. Source inspection
confirms clone cleanup traps, the target-directory link, exported cache path and
status propagation. This is the Warrant's bounded fake-battery scope. It does
not establish build-cache isolation or macOS behavior.

The earlier closure dry run and its nine UNKNOWN requirements are preserved.
No signatures, independent dispositions, active authority or delivery pins were
changed. This report is performer evidence. It does not establish signed
criterion acceptance, gate qualification, Verified status or legacy resolution.
