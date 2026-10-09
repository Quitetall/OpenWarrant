# Source-detached provider capture query

Candidate unverified continuation of OW-WAR-0111 and OW-WAR-0149. The query now inventories supported retained runtime captures and reconnects their exact Dispatch, compile event, Warrant, contract, stage and attempt to source bytes in the archive. It reads no provider service or original repository. It does not select a current worker or establish receipt eligibility, native authentication, authority or assurance. Saved native verdicts are ignored.

Missing retained source and unsupported versions remain explicit query gaps. Observed altered content, contradictory bindings and duplicate compile events refuse. An incomplete archive still cannot pass the whole-project import coverage gate. No archive encoding, hash domain, signed source or retained evidence was changed.

## Actual observation

The merged cookbook cbb33663 with BLUT engine 2502a4dd, built with the previously recorded explicit engine override, returned actual job `20261009-131242-620252367` as done with no PID through its local jobs interface. The raw payload is a status observation, not the missing native dispatch-bound receipt protocol. That job predates the prototype Dispatch used for this collector demonstration. The import truthfully reports `runtime.verifier-unavailable` and UNKNOWN native eligibility.

A disposable checkout of OpenWarrant main 99c30f50 produced the actual OW-WAR-0047/STAGE-003 Dispatch and compile event. Its capture and sources were exported before removing the entire disposable checkout. Querying the unchanged archive with that checkout absent reconnects both the exact Dispatch source and the reconstructed contract/stage. Native verification stays UNKNOWN and execution coverage and assurance stay false. The source archive contains 113 records and incomplete whole-project history coverage. It is retained compressed for source-detached query reproduction, not represented as a complete import or qualified native run. `source-archive-identity.json` identifies both raw bytes and the existing archive digest.

## SAS reconstruction defect

The first real query reconnected the Dispatch but could not reconstruct its contract. The IR stores SAS identities as `sha256:<hex>` while the compiler input expects bare hex. The historical/current graph reader had supplied the prefixed form to that input, causing a second prefix and a different contract digest. The reader now checks and unwraps that identity before using the existing lowering interface. The failing observation is retained; neither the archive nor a signed SAS pin was rewritten.

## Bounded checks

Focused runtime-capture controls cover source deletion, a move to an empty directory, unknown native support, missing archived Dispatch, a contradictory attempt with all local hashes recomputed, unsupported capture versions, and forged saved PASS/assurance flags. A SAS-pinned lowering round-trip checks the reader defect and malformed digest refusals. Existing preservation integration tests and all-target CLI Clippy run separately. Rust 1.97.1; raw logs are beside this file once complete. Full hosted gate and independent/human acceptance remain separate.

Reproduction: decompress `source-archive.json.gz` to a new file, then run the candidate `war archive runtime-basis <file> --json` without the original source checkout. Do not expect `war archive import` to accept its deliberately incomplete whole-project coverage.
