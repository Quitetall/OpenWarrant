# Declared artifact byte capture

Export now reads current and captured historical `deliverables.toml` records and
builds `__ow_archive__/artifacts.json`. Each claim retains its declaration source,
id, target, expected digest and either a retained content-addressed record or an
explicit unavailable reason. Source declarations remain byte-preserved.

For local file deliverables with an exact SHA-256 identity, export uses the
no-follow bounded reader and retains bytes only when their digest matches the
declared version. Content-addressed blobs are deduplicated within the archive.
A newer file with a different digest is never substituted for the old artifact.
Non-file resolvers, missing identities and absent historical versions remain
unavailable. No network resolver or authority operation runs during this capture.

Inventory checks compare every claim against retained source declarations and
refuse duplicate/empty ids, unknown deliverable kinds, omitted claims, mismatched
source fields, missing retained blobs and digest changes. Recomputing an outer
archive record hash does not bypass the declaration-to-blob identity check.

The local test preserves a binary artifact after original docs and target file
are removed. It also observes refusal for changed bytes, removed inventory claims
and unknown required record kinds, plus explicit unavailability for a newer file
whose bytes differ from the declared artifact version.

This is collection of observed declared artifacts. Global artifact coverage stays
unavailable until required historical versions and provider/external inventories
are reconciled. The archive is still incomplete; collection is not independent
verification or acceptance of the delivered code.

## Recovery from retained Git history

With `--history`, missing or different local files now trigger a bounded lookup
of regular-file versions reachable from the same pinned HEAD used for Warrant
history capture. The declared SHA-256 selects a version. Lookup never checks out
files, follows symlinks, guesses line-ending conversions, or fetches missing Git
objects. More than 256 relevant commits refuses; candidates larger than the
remaining archive budget cannot be retained.

Recovered claims record the observed HEAD, matching commit and Git blob ids.
Inspection checks that the origin's HEAD matches the captured history descriptor;
these ids remain provenance observations, not independent authentication of Git
ancestry. Artifact bytes are independently checked against their declared SHA-256.
Repeated content is stored once, and claim count is bounded by the archive record
limit. Missing versions remain unavailable rather than being replaced by current
bytes.

The test commits two binary versions and declarations, leaves a third uncommitted
version, and captures both declared versions. Removing the current path produces
byte-identical archive output. Removing the original Git database and docs still
permits reconstruction. A rewritten origin HEAD refuses. Git-normalized text
bytes that were never stored as blobs remain unavailable; the tool does not claim
to restore bytes that Git discarded.

## Git process isolation

A regression reproduced inherited `GIT_DIR` overriding the selected repository
and making its available history appear unavailable. History subprocesses now
remove inherited Git directory, worktree, common-directory, object-directory,
alternate-object-directory, index, shallow-file and namespace overrides. The
selected checkout still supplies its own Git metadata and configuration.

The regression injects conflicting locations and requires byte-identical archive
output compared with the normal run. This protects repository selection; it does
not authenticate the selected repository or bypass missing-history refusals.
