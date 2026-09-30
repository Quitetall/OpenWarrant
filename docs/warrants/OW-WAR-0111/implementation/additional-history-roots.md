# Explicit additional history roots

`war archive export ALIAS OUTPUT --history --history-ref REF` adds a local history
root. Repeat the option up to sixteen times. Each ref resolves to a commit before
capture; the archive records sorted additional commit identities. Default remains
HEAD-only. Selection retains the combined 256-commit, record and byte limits.
Missing refs refuse; no fetch, checkout, active authority or signature change occurs.
Shared-atom selection and artifact version lookup use the same explicit roots.
Older history descriptors without additional roots remain supported.

Validation: thirteen preservation integration tests and all-target CLI Clippy pass
with Rust 1.97.1. A test proves an unmerged record is absent by default, included
only with its selected branch, inspectable after source removal, and accompanied
by the pinned extra root. Unknown refs refuse without creating output.

Recovery: twelve retained original-document snapshots did not contain the missing
OW-WAR-0030 OBL-003 verification bytes. Search across local refs found exact digest
8eb3041554c8471f70c79096cb5b7520dcb2f6344a58dbcf1c3196b715ab236f
at commit 3de9e1f53c459088a1c672d01e7ce424276fc5da on
refs/remotes/origin/records/round3, outside current HEAD ancestry. Explicit export
with that ref recovered the bytes. Its audit category now has retained coverage;
four other categories remain unavailable. Offline inspection passed. The search,
archive and pinned-root observation are retained beside this note.

No current verification record was replaced. Conflicting historical states remain
historical data; inclusion does not select authority or accept an outcome. OW111
remains incomplete; full category and provider qualification still apply.
