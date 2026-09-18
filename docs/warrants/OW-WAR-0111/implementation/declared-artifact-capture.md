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
