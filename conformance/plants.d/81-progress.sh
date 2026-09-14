# shellcheck shell=bash
# The progress bundle (slice D5): copied bytes with a manifest, verifiable,
# refused into a non-empty directory, and drift in a copied file is named.

PB_TMP=$(mktemp -d)
PB_OUT="$PB_TMP/bundle"

plant_cmd "the progress bundle is exported" "progress.exported" "bundle sha256" 0 \
    "true" \
    export --progress "$PB_OUT"
if [[ -f "$PB_OUT/MANIFEST.json" && -f "$PB_OUT/CORPUS_STATUS.json" && -f "$PB_OUT/NORMATIVE.json" ]] && ls "$PB_OUT"/warrants/*/WAR.json >/dev/null 2>&1; then
    printf 'ok    %-34s manifest, projections and per-Warrant WAR.json present\n' "the bundle has its files"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s files missing under %s\n' "the bundle has its files" "$PB_OUT"
    FAILED=$((FAILED + 1))
fi

plant_cmd "the bundle verifies against its manifest" "progress.verified" "match" 0 \
    "true" \
    export --verify-progress "$PB_OUT"

plant_cmd "a non-empty directory is refused" "progress.not-empty" "empty" 2 \
    "true" \
    export --progress "$PB_OUT"

plant_cmd "a byte moved in the bundle is named" "progress.file-drift" "CORPUS_PENDING.json" 2 \
    "printf ' ' >> \"$PB_OUT/CORPUS_PENDING.json\"" \
    export --verify-progress "$PB_OUT"

rm -rf "$PB_TMP"
