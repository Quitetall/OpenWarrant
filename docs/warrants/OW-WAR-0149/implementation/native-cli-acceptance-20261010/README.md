# Native CLI acceptance checkpoint

Unverified implementation under OW149; no project acceptance or independent verdict.

Full native preparation and ingestion passed with actual Katana and BLUT CPU
software fixture binaries under separate operator and executor UIDs. Each uses
all thirteen required checks, protected native receipts, observed gate evidence
and a separately executed synthetic review protocol. These fixture reviewers
and public test keys do not establish actual human review or code quality.

The `war resolve` command validates the exact bounded signed response and its
adjacent signature, requires protected native policy, rejects mismatched capture
selections and performs a no-write signed dry run before recording. The legacy
signed response also supports its existing no-write ingestion mode. The legacy
runtime request remains unsigned and cannot substitute for a native response.

Controls refused signed snapshot, policy and locator changes, extra gate refs,
missing selections, explicit selection mismatch, changed delivered bytes and
replay. The stored signer is the trusted principal, distinct from its actor label.
Katana, BLUT and normal protected namespace tests each passed. Final formatting
and all-target/all-feature CLI Clippy passed.

Retained failures show the missing path type import, the original argument
parser conflict and a fixture temporary-directory permission failure. They were
corrected; final reruns passed. `observations.json` identifies the tested source
revision and hashes every retained log.

Secure human signing integration, portable native schemas, retained Dispatch
identity mapping and independent production qualification remain. OW95 has no
participants; no sessions, acceptance or signatures were invented. Source
rechecks and target compare-and-write are not an atomic fence over all inputs.
