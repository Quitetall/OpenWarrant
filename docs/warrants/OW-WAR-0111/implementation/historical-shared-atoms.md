# Shared atoms at historical Warrant revisions

History capture now reads each retained manifest and captures its explicit atom
references from that same Git commit. Shared ADR bodies outside the Warrant tree
are stored with that commit's records. Current working-tree bytes cannot substitute
for missing historical content. Missing sources, bound URIs without a resolver,
non-regular Git entries and archive bounds refuse capture.

Validation: twelve preservation integration tests passed (Rust 1.97.1), including
two historical shared bodies, a third uncommitted current body, source removal,
and refusal when a historical source is missing but a current file exists.
All-target CLI Clippy with warnings denied passed before the final fixture-only
parent-directory repair. Real OW-WAR-0003 history export succeeded and retained
20 shared ADR records. Its compressed archive is retained here.

Scope: commits selected by Warrant-path history reachable from pinned HEAD.
Commits changing only an external shared atom are not yet added to that selection.
This does not prove complete ADR history or independently authenticate Git records.
OW-WAR-0111 remains incomplete; category completeness is not upgraded.
