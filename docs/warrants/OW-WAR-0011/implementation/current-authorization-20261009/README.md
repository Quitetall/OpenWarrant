# Current authorization check in partial Preflight

Source: `8dd895ea954f75bfe6d52d7976178b1c53cce02e`. Implementation evidence only; OW11 remains in progress.

The baseline public CLI left a cryptographically signed synthetic authorization unexamined. An independent `ssh-keygen -Y verify` of the existing fixture succeeded; the expected Preflight pass assertion failed. No new key or human signature was created.

Preflight now checks the exact current contract, supported record identity/state, recorded human authorizer/role/meaning/time, current configured authorizer grant, and an exactly matching typed authorization response with a verified signature. A standing-class authorization follows its existing re-derived class coverage path. The exact-payload helper is used by Preflight; other existing signature consumers retain their prior semantics.

Nine public CLI tests passed, including absent signatures, altered recorded meaning, removed authorizer role, unavailable configured store without legacy fallback, source changes, invalid graph and read-only controls. All 41 checks and six dimensions remain. Runtime and side-effect authority are not inferred. This does not persist a receipt, start work, activate authority or confer assurance. Legacy trust files are not execution isolation.

The first shared-cache build waited until its 300-second deadline; it is UNKNOWN, not a failing test or a passing suite. Reusing the existing project target cache produced the nine-test pass. All-target CLI Clippy passed on Rust 1.97.1. Full hosted gate, remaining live actor/gate/context/authority observations, the human-only checker wording correction, independent qualification and acceptance remain separate.

The added standing-policy control initially observed a false pass: ordinary-response fallback bypassed the claimed class. The consumer now calls class coverage directly; an unavailable re-derivation stays UNKNOWN. The nine-test final suite and lint checks passed. The earlier eight-test source/results remain retained with their original source boundary.

The broader local CLI unit invocation reached its 300-second deadline with the Bonsai pilot unfinished; 349 individual test passes were observed. Its overall result is UNKNOWN. Projection generation also reached its first deadline during local load; a sequential retry completed. These outcomes do not substitute for the full hosted gate.
