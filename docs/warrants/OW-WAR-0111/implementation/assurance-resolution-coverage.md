# Assurance and resolution preservation coverage

The local exporter now retains assurance source pointers, judgment records,
verification records, resolution records and their selected historical versions.
Explicit assurance atoms outside the Warrant directory are included. Known record
schemas must parse; malformed records remain preserved but category coverage stays
unavailable. Verdicts, independence claims and standing are not reissued or upgraded.

A selected current/history inventory with no resolution record reports that bounded
absence. Inspection recomputes category coverage from retained sources. It also
checks absent claims for previously derived artifact, contract and audit categories;
changing a label to absent cannot bypass their observed coverage requirements.
Unknown external/provider records are not inferred from local absence.

Validation: preservation unit tests, thirteen integration tests and all-target CLI
Clippy passed with Rust 1.97.1. Refusal cases include malformed resolution and
verification records, history-free absence claims, and forged artifact absence.
Real OW-WAR-0030 export with explicit origin/records/round3 history and offline
inspection passed. Assurance coverage is retained; no resolution record is present
in that selected inventory. ADR context and runtime receipt references remain
unavailable. No Warrant resolution or independent assurance was issued.
