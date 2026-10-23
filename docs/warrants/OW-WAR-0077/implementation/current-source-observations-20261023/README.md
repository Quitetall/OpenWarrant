# Current SDK source observations — 2026-10-23

The sealed CLI built from clean `c7e46541f7bbcbda465a19b23e344a62e7ce1102`
passed all 19 source cases and unseen-input/output-preservation controls: 65 actual
public SDK calls. `wire.jsonl` retains each request, stdout, stderr and exit;
`summary.json` binds that wire and the executable. Original working-candidate
observations remain in the adjacent earlier directory.

The real LAMU provider at `2572894f28ce76c2c7e6c6c9ae6be51a7f400fa1` also
passed all five source integration tests using this checkout's current SDK.
A temporary Cargo path override selected `openwarrant-core` from the exact current
checkout; raw Cargo metadata and commands are retained, with source hashes and
all test output. This is a compatibility experiment, not an update to the
provider's published SDK pin or profile. Its original lock was restored after
the run. No provider code or OpenWarrant dependency was adopted or changed.

These are Linux performer observations, not independent dispositions, human
acceptance or release qualification. Sequential filesystem rechecks are not an
atomic multi-file snapshot. The supplied-source SDK still performs no source
acquisition and grants no semantic assurance or authority.
