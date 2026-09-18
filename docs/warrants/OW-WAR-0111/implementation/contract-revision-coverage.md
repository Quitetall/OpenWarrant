# Retained contract revision coverage

The local exporter now parses retained current and historical authorization records,
checks supported schema and contract digest syntax, and reconciles each revision's
predecessor digest against the preceding numbered revision. Missing history or
predecessors remain unavailable with exact source paths. A consistent retained set
lists its source records, amendments and history inventory as coverage evidence.
Source records are preserved unchanged. No approval, signature, authorization or
contract acceptance is manufactured.

Offline inspection recomputes retained contract coverage and refuses a forged
retained claim whose pointers or prerequisites disagree with the retained set.
This establishes local retained-record coverage, not signature validity, original
contract-digest recomputation, or completeness of external provider inventories.
Other refs and unavailable history remain outside the declared capture boundary.

Validation: predecessor-chain unit test passed, including missing history, missing
predecessor, correct predecessor and wrong revision number. Twelve preservation
integration tests passed, including forged contract-coverage refusal. All-target
CLI Clippy passed before the final added refusal assertion (Rust 1.97.1).
Real OW-WAR-0030 export and offline inspection passed with retained contract
coverage. Five other required categories remain unavailable; OW111 stays open.
