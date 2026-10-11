# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Format 1.0 (OW-ADR-0032): the hand-written normative schemas under
# schemas/format/ and their registry.
#
# Accepted: every frozen record of docs/COMPATIBILITY.md has a registered
# schema (sas-revision and amendment included); every schema under
# schemas/oh.war/ and schemas/format/ has its $id in schemas/format/registry.json;
# each hand-written schema accepts the repository's own records (SAS
# revisions, amendments, profiles, type packs), the examples of
# docs/format/FORMAT-1.0.md and its valid fixtures.
# Refused: a frozen id with no schema, an unregistered $id and a registered
# file that is gone, each by name (planted in memory); for every hand-written
# schema at least one instance, each for the reason its fixture names
# (doctored copies of real records are held in memory, never written).
# Read only: nothing in the tree moves.

echo "== format schemas and their registry (OW-ADR-0032) =="
if ! python3 -c 'import jsonschema, referencing, tomllib' 2>/dev/null; then
    printf 'PLANT SETUP FAILED: python3 cannot import jsonschema, referencing and tomllib; the format schema check cannot run\n' >&2
    exit 9
fi
FS_OUT=$(python3 "$REPO_ROOT/conformance/schemas/format_schemas.py" --root "$REPO_ROOT" 2>&1)
FS_CODE=$?
FS_SUMMARY=$(printf '%s\n' "$FS_OUT" | tail -n 1)
if [[ $FS_CODE -eq 0 ]]; then
    printf 'ok    %-34s %s\n' "format schemas and registry" "$FS_SUMMARY"
    PASSED=$((PASSED + 1))
else
    printf '%s\n' "$FS_OUT" | grep -E '^(FAIL|UNKNOWN)' >&2
    printf 'FAIL  %-34s exit %s: %s\n' "format schemas and registry" "$FS_CODE" "$FS_SUMMARY"
    FAILED=$((FAILED + 1))
fi
unset FS_OUT FS_CODE FS_SUMMARY
