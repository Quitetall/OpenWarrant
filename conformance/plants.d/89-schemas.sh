# shellcheck shell=bash
# The JSON Schema pack (OW-WAR-0032): generated from the record types,
# drift-checked like every projection. A hand-edited schema is the same defect
# class as a hand-edited parent, and the check names the file. The generator
# is behind the `schema` cargo feature, so these plants build it once, apart
# from ./target/debug/war (battery_tool, lib.sh; t-d052), and run that copy.
# It is built from the committed tree before any mutation, so what it checks
# is the files on disk against the types, never a pack a plant planted and
# a rebuild then embedded.

SCHEMAS_TMP=$(mktemp -d)
if battery_tool "$SCHEMAS_TMP" war -p openwarrant-cli --features schema; then
    SCHEMAS_RUN=("$SCHEMAS_TMP/war" schemas --check)
else
    SCHEMAS_RUN=(sh -c 'echo "error: the schema-featured war did not build (battery_tool)"; exit 125')
fi

schemas_expect() {
    local name="$1" want_exit="$2" want="$3" mutate="$4"
    restore
    eval "$mutate"
    local out status
    out=$("${SCHEMAS_RUN[@]}" 2>&1)
    status=$?
    restore
    if [[ $status -eq $want_exit ]] && grep -Fq -- "$want" <<< "$out"; then
        printf 'ok    %-34s exit %s %s\n' "$name" "$status" "$want"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted exit %s %s; got exit %s:\n%s\n' "$name" "$want_exit" "$want" "$status" "$(grep -E '^(ERROR|error)' <<< "$out" | head -3)"
        FAILED=$((FAILED + 1))
    fi
}

schemas_expect "the committed pack matches the types" 0 "schemas.current" "true"
schemas_expect "a hand-edited schema is drift, by file" 2 "schemas/oh.war/manifest/v1.json" \
    "sed -i 's/\"description\"/\"descriptionX\"/' schemas/oh.war/manifest/v1.json; assert_present descriptionX schemas/oh.war/manifest/v1.json"
schemas_expect "a missing schema is named" 2 "schemas.missing" "rm schemas/oh.war/correction/v1.json"
schemas_expect "a relabelled pack is drift" 2 "schemas/pack.json" \
    "sed -i 's/\"transitive_digest\":\"[0-9a-f]*\"/\"transitive_digest\":\"0000\"/' schemas/pack.json; assert_present '\"0000\"' schemas/pack.json"
# Candidate publication has its own controls; it is not in the active pack.
schemas_expect "a candidate schema edit is drift" 2 "schemas/oh.war/verification/v2.json" \
    "sed -i 's/oh.war\/verification\/v2/oh.war\/verification\/v9/' schemas/oh.war/verification/v2.json; assert_present 'oh.war/verification/v9' schemas/oh.war/verification/v2.json"
schemas_expect "a missing candidate is named" 2 "schemas.missing" "rm schemas/oh.war/verification/v2.json"
rm -rf "$SCHEMAS_TMP"
unset SCHEMAS_TMP SCHEMAS_RUN
