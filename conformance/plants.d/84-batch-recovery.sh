# shellcheck shell=bash
# OW-WAR-0130: recovery input does not authorize arbitrary rollback writes.
echo "== batch recovery input =="
PLANT_ROOT=$(scratch_corpus BR)
BR_TMP=$(mktemp -d)
mkdir -p "$BR_TMP/outside" "$PLANT_ROOT/docs/authority/batches"
printf 'unrelated bytes' > "$BR_TMP/outside/keep.txt"
BR_MARKER="$PLANT_ROOT/docs/authority/batches/B-unsafe.recording"
for BR_CASE in truncated external traversal; do
    python3 - "$BR_MARKER" "$BR_TMP/outside" "$BR_CASE" <<'PYMARKER'
import json, pathlib, sys
p, outside, case = sys.argv[1:]
if case == "truncated":
    pathlib.Path(p).write_text("{truncated")
else:
    root = outside if case == "external" else "../../" + pathlib.Path(outside).relative_to("/").as_posix()
    pathlib.Path(p).write_text(json.dumps({"schema":"oh.war/batch-recording/v1", "batch_id":"B-unsafe", "roots":[{"dir":root,"copy":None}], "paths":[]}))
PYMARKER
    BR_BEFORE=$(sha256sum "$BR_MARKER" | cut -d' ' -f1)
    BR_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch --recover B-unsafe --json 2>&1); BR_STATUS=$?
    BR_AFTER=$(sha256sum "$BR_MARKER" 2>/dev/null | cut -d' ' -f1)
    if [[ "$BR_STATUS" -eq 2 && "$BR_BEFORE" == "$BR_AFTER" ]] \
        && [[ "$(cat "$BR_TMP/outside/keep.txt" 2>/dev/null)" == "unrelated bytes" ]] \
        && grep -Eq 'batch.recover-(unreadable|failed)' <<<"$BR_OUT"; then
        printf 'ok    recovery %-24s marker and unrelated files retained\n' "$BR_CASE"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  recovery %-24s exit %s: %s\n' "$BR_CASE" "$BR_STATUS" "$BR_OUT"
        FAILED=$((FAILED + 1))
    fi
done
command rm -rf "$BR_TMP"
unset PLANT_ROOT
