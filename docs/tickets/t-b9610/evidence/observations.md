# Stale verifier reproduction

Performer diagnostic, not an independent verdict. The only established dispositions in this reproduction are explicitly synthetic responses inside a disposable copied inbox fixture. No production verification was created.

Binary: `/mnt/2tb/war-sha2-optimized`, built from cbe9a8a8 sources. Fixture: `conformance/fixtures/inbox/repository`, IX-WAR-0003.

1. Emit the request for fixture-performer.
2. Ingest an unbound v1 fixture response for both declared obligations from fixture-independent-verifier, a distinct synthetic service.
3. `prepare IX-WAR-0003 --dry-run --json` reports verification current, 2/2 established.
4. Change the actual OBL-001 requirement in the assurance atom, preserving its id and all other sources.
5. Emit a new request and run the same preparation command. The new requirement appears in the request, and exact authorized Contract Revision becomes unmet, but verification is still reported current, 2/2.
6. Ingest the unchanged old response again. It exits zero.

This demonstrates stale verification reuse and old-response acceptance. Other resolution gates still block this fixture; this is not a claim that unauthorized resolution passed.

The request, response envelope and persisted verification have no checked reviewed-subject binding. `prepare::verdicts` reads `resolve::assess`; assessment checks actor independence and nonempty evidence but does not compare a reviewed contract/artifact snapshot. The repair must bind actual independent review inputs and reject stale late responses, not mint a fresh review binding when an arbitrary old response is imported. Preserve old verdict history and classify unavailable bindings as UNKNOWN.

Core `verification.rs` remains an active resolved pin. Use the CLI/admissibility seam and the existing canonical verification-bundle domain; request a correction if a core change becomes necessary. No replacement canonicalization or invented provider seal.
