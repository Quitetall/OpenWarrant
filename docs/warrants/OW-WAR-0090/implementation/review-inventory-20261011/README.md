# Exact release-review inputs

Unverified performer work. The inventory source subject is
`5af8d25107c3c5e0b2efe3b27c2b6c36ae9dc82d`; its committed generator bytes match
the generator SHA-256 in the snapshot. Later evidence files do not change that
frozen subject. No human act, independent verdict or release qualification is recorded.

Run the primitive with:

```bash
python3 conformance/release/review_inventory.py --revision HEAD --output /path/to/new-review.json
```

The snapshot freezes committed bytes rather than the current working files. It
retains all Git source entries and the exact attributed work reports, hashes their
referenced evidence, and lists missing evidence and Warrant directories without a
work report. It does not interpret source hashes or completed reports as assurance.
This is a private performer artifact, not a new standard or signed review manifest.

## Observed result

[Observation](observation.json) identifies original file hashes and retained
uncompressed copies. The [frozen snapshot](review-subject-5af8d251.json.gz) contains
10,891 source entries, 88 work reports, 340 referenced evidence files, no missing
evidence, and 57 Warrant directories without a work report. The reported states
remain 68 completed, 16 in progress and four blocked. Absent work reports do not
establish completion, failure or resolution; assess their actual records separately.

Five public-command tests pass. They exercise repeatable committed-byte selection,
changed/untracked working files, missing evidence, duplicate report fields, unsafe
references, mismatched identity/schema, absent optional evidence, unreported
Warrants, preserved existing snapshots, and refused committed symlink inputs.
The original four-test planted runner passed. The full release-tool suite passed
22 tests before the final optional-evidence fix; the final five-test suite covers
that fix. Shell syntax and diff checks passed. Regenerated corpus checks report
2,028 PASS, 630 WARN, zero ERROR/UNKNOWN; this does not establish runtime readiness.

The first refusal run reused a filename between subcases, so an earlier accepted
output contaminated later no-output assertions. The corrected isolated run exposed
two actual omissions: duplicate fields and wrong schema. Both were repaired. The
first positive run refused because the command did not yet exist. The optional
evidence control subsequently reproduced a valid absent field being refused; it
now preserves that source without inventing an evidence list. All original logs
are retained, including those unsuccessful attempts.

## Still required

Independent review, actual required provider/operator/user observations, all four
phase exits, reconciliation of all remaining Warrant scopes, an eligible signed
final review subject and exact owner publication permission remain open. No study
participants are available. This inventory cannot substitute for those observations
or cryptographically establish binary/source linkage.
