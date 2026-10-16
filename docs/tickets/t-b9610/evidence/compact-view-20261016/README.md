# Compact model input emission and local transport

An opt-in --bundle --compact emitter now retains distinct v1 executable model
views and exact canonical v2 archives. Paired archive copies make offline
input delivery possible. The index points to large subject metadata instead
of repeating it. Projected requests identify their source schema without
claiming to be complete frozen requests.

On Rust 1.97.1, all 56 schema-enabled verification CLI regressions pass. The
new public control checks exact archives, source/contract preservation,
unchanged subject, deterministic replay, immutable collision refusal and
retention of ineligible over-budget inputs. The missing compact flag was
observed before implementation. CLI Clippy over all targets passes.

All 11 local transport controls pass. The initial compact call was refused
before the reader supported that input. It now sends only the view, preserves
the archived subject in a v3 response and binds only the actual input pair.
Refusal controls cover missing/wrong references, changed sibling/canonical
archives, links, over-budget/ineligible inputs and a preview with a toggled
flag, all before provider contact or review-log creation. Existing v2 controls
remain. Optional empty subject maps are displayed explicitly by TOML; they
have the same typed meaning as omitted default-empty JSON maps.

A composed control used the real compiler with a synthetic HTTP backend and
observed a bound v3 response. This is transport evidence, not a model review,
source qualification or assurance. No paid or production inference occurred.
Rust ingestion still must validate actual view/archive/current-source,
performer, coverage, independence and journal/storage bindings. Compact CLI
run/ingestion remains disabled; no production verdict or human act was written.

Raw tests and compiler-client control:
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/compact-view-20261016/`.
