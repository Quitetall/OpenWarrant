# shellcheck shell=bash
# OW-WAR-0064 — the correction act (OW-ADR-0012).
#
# Target: OW-WAR-0061, which is RESOLVED and whose D-001 is
# docs/roadmap/PHASE1_EXIT.md — under docs/roadmap/, which `restore` covers. A
# Warrant whose pinned deliverable lives under crates/ cannot be the target:
# `restore` does not check out crates/, so a plant there would leave the working
# tree dirty.
#
# Every ingest plant writes a correction record under docs/warrants/OW-WAR-0061/
# corrections/ and a journal line. The journal is tracked and restored by
# checkout; the corrections directory is NOT tracked today and is removed by
# `restore_corrections` below. That removal is safe ONLY while no real
# correction exists there — the guard below refuses to run otherwise.
#
# The target was OW-WAR-0062 until 2026-09-13, when the owner signed a genuine
# correction of its D-001 and the guard stopped the battery rather than deleting
# it. That is the guard working: the header said "the day it gains a genuine
# correction this file must move to another target", and this is the move. (The
# `rm -f` that once deleted a real resolution is why this is a guard and not a
# comment.)

TARGET_ALIAS="OW-WAR-0061"
TARGET_DID="D-001"
TARGET_FILE="docs/roadmap/PHASE1_EXIT.md"
CORR_DIR="docs/warrants/$TARGET_ALIAS/corrections"

if [[ -e "$CORR_DIR" ]]; then
    echo "REFUSING: $CORR_DIR exists; these plants remove that directory and would" >&2
    echo "delete a real correction. Retarget the OW-WAR-0064 plants to another Warrant." >&2
    exit 1
fi

restore_corrections() {
    rm -rf "$CORR_DIR"
}

# Append a comment to the pinned file: still valid TOML, different bytes.
drift_target() {
    printf '# planted by conformance/plants.d/64-corrections.sh\n' >> "$TARGET_FILE"
    assert_present 'planted by conformance/plants.d/64-corrections.sh' "$TARGET_FILE"
}

recorded_digest() {
    # The digest deliverables.toml records for D-001.
    awk -v id="$TARGET_DID" '
        $0 ~ "^id = \"" id "\"" { in_block = 1 }
        in_block && /^content_digest = / { gsub(/"/, "", $3); print $3; exit }
    ' "docs/warrants/$TARGET_ALIAS/deliverables.toml"
}

current_digest() {
    printf 'sha256:%s' "$(sha256sum "$TARGET_FILE" | cut -c1-64)"
}

# write_response <out> <actor> <superseded> <new> [effective_time]
write_response() {
    local out="$1" actor="$2" superseded="$3" new="$4" when="${5:-2026-09-11T12:00:00Z}"
    cat > "$out" <<EOF
schema = "oh.war/correction-response/v1"
warrant = "$TARGET_ALIAS"
deliverable_id = "$TARGET_DID"
superseded_digest = "$superseded"
new_digest = "$new"
reason = "planted: the revision record gained a trailing comment"
kind = "added-refusal"
corrected_by = "$actor"
acting_role = "authorizer"
effective_time = "$when"
EOF
}

RESP=/tmp/openwarrant-plant-correction.toml

# 1. Drift with no correction record is still the error it always was.
plant "a resolved deliverable drifted with no correction" "deliverable.digest-drift" "war correct" 2 \
    "drift_target" \
    "$TARGET_ALIAS"

# 2. An agent signing a correction is refused by kind, and nothing is written.
plant_cmd "an agent signing a correction" "correction.agent" "SHALL NOT correct" 2 \
    "drift_target; write_response $RESP claude \$(recorded_digest) \$(current_digest)" \
    correct "$TARGET_ALIAS" "$TARGET_DID" --response "$RESP"
[[ -e "$CORR_DIR" ]] && { echo "FAIL  an agent's refused correction left $CORR_DIR" ; FAILED=$((FAILED + 1)); restore_corrections; }

# 3. A response whose new digest is not the file's bytes corrects nothing.
plant_cmd "a correction whose new digest is not the file" "correction.stale" "corrects nothing" 2 \
    "drift_target; write_response $RESP 'Brian Lam' \$(recorded_digest) sha256:$(printf '0%.0s' {1..64})" \
    correct "$TARGET_ALIAS" "$TARGET_DID" --response "$RESP"

# 4. A response superseding a digest that was never on record supersedes nothing.
plant_cmd "a correction superseding a digest never delivered" "correction.superseded-mismatch" "supersedes nothing" 2 \
    "drift_target; write_response $RESP 'Brian Lam' sha256:$(printf '1%.0s' {1..64}) \$(current_digest)" \
    correct "$TARGET_ALIAS" "$TARGET_DID" --response "$RESP"

# 4b. The same lie as a RECORD on disk, not a response: war check refuses it by name.
plant "a correction record superseding a digest never delivered" "correction.superseded-never-delivered" "supersedes nothing" 2 \
    "drift_target; mkdir -p $CORR_DIR; write_response $RESP 'Brian Lam' sha256:$(printf '1%.0s' {1..64}) \$(current_digest);
     printf 'schema = \"oh.war/correction/v1\"\nwarrant = \"$TARGET_ALIAS\"\n\n[correction]\nid = \"01a06a12-0aa2-7503-b589-67cf75905be4\"\ndeliverable_id = \"$TARGET_DID\"\ntarget_ref = \"$TARGET_FILE\"\nsequence = 1\nsuperseded_digest = \"sha256:%s\"\nnew_digest = \"%s\"\nreason = \"planted\"\nkind = \"added-refusal\"\nauthorized_by_ref = \"person://Brian Lam\"\nacting_role_ref = \"role://authorizer\"\neffective_at = \"2026-09-11T12:00:00Z\"\nrecorded_at = \"2026-09-11T12:00:01Z\"\n' \"$(printf '1%.0s' {1..64})\" \"\$(current_digest)\" > $CORR_DIR/$TARGET_DID-1.toml" \
    "$TARGET_ALIAS"
restore_corrections

# 5. A valid human correction is ingested; then the record is edited: refused as edited.
plant "a correction edited after authorization" "correction.edited" "edited after authorization" 2 \
    "drift_target; write_response $RESP 'Brian Lam' \$(recorded_digest) \$(current_digest);
     \$WAR correct $TARGET_ALIAS $TARGET_DID --response $RESP > /dev/null 2>&1 || { echo 'setup: valid correction was refused' >&2; exit 9; };
     sed -i 's|^reason = .*|reason = \"reworded after the fact\"|' $CORR_DIR/$TARGET_DID-1.toml;
     assert_present 'reworded after the fact' $CORR_DIR/$TARGET_DID-1.toml" \
    "$TARGET_ALIAS"
restore_corrections

# 6. A second change applied by EDITING correction 1 instead of adding correction 2.
plant "a second correction applied as an edit to the first" "correction.edited" "second correction is a second file" 2 \
    "drift_target; write_response $RESP 'Brian Lam' \$(recorded_digest) \$(current_digest);
     \$WAR correct $TARGET_ALIAS $TARGET_DID --response $RESP > /dev/null 2>&1 || { echo 'setup: valid correction was refused' >&2; exit 9; };
     printf '# planted again\n' >> $TARGET_FILE;
     sed -i \"s|^new_digest = .*|new_digest = \\\"\$(current_digest)\\\"|\" $CORR_DIR/$TARGET_DID-1.toml;
     assert_present 'planted again' $TARGET_FILE" \
    "$TARGET_ALIAS"
restore_corrections

# 7. Positive: after a valid correction, the check passes and names the supersession.
plant "a corrected deliverable passes and names the superseded digest" "deliverable.corrected" "superseded" 0 \
    "drift_target; write_response $RESP 'Brian Lam' \$(recorded_digest) \$(current_digest);
     \$WAR correct $TARGET_ALIAS $TARGET_DID --response $RESP > /dev/null 2>&1 || { echo 'setup: valid correction was refused' >&2; exit 9; }" \
    "$TARGET_ALIAS"
restore_corrections

# 8. OBL-005: the superseded digest cannot be removed from a record.
plant "a correction record with its superseded digest removed" "correction.malformed" "superseded_digest" 2 \
    "drift_target; write_response $RESP 'Brian Lam' \$(recorded_digest) \$(current_digest);
     \$WAR correct $TARGET_ALIAS $TARGET_DID --response $RESP > /dev/null 2>&1 || { echo 'setup: valid correction was refused' >&2; exit 9; };
     sed -i '/^superseded_digest = /d' $CORR_DIR/$TARGET_DID-1.toml;
     assert_gone superseded_digest $CORR_DIR/$TARGET_DID-1.toml" \
    "$TARGET_ALIAS"
restore_corrections

# 8b. OBL-005: `war show` prints the superseded digest after a correction.
plant_cmd "war show names the superseded digest" "Corrections" "superseded" 0 \
    "drift_target; write_response $RESP 'Brian Lam' \$(recorded_digest) \$(current_digest);
     \$WAR correct $TARGET_ALIAS $TARGET_DID --response $RESP > /dev/null 2>&1 || { echo 'setup: valid correction was refused' >&2; exit 9; }" \
    show "$TARGET_ALIAS" --view status
restore_corrections

# 9. Before resolution the ordinary remedy applies: a correction is refused.
plant_cmd "a correction on an unresolved warrant" "correction.not-resolved" "regenerate deliverables.toml" 2 \
    "printf '# planted\n' >> conformance/plants.d/00-corpus.sh; TARGET_ALIAS=OW-WAR-0063 TARGET_DID=D-003 write_response $RESP 'Brian Lam' sha256:$(printf '2%.0s' {1..64}) sha256:$(printf '3%.0s' {1..64})" \
    correct OW-WAR-0063 D-003 --response "$RESP"
git checkout -- conformance/plants.d/00-corpus.sh 2>/dev/null || true

rm -f "$RESP"
