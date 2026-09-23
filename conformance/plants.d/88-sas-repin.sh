# shellcheck shell=bash
# The circular pin (OW-ADR-0016, slice B4): an amendment's `sas_revision`
# re-pins a Warrant ahead of its authorization, the contract digest moves, and
# the next authorization revision is what `war sign --list` shows. A re-pin
# to a revision nobody recorded, or to one that lacks a row the Warrant
# implements, is refused by name; a re-pin after resolution is reported stale.

REPIN_ALIAS=OW-WAR-0047   # authorized, unresolved, implements rows 1.0.0 still has
REPIN_RESOLVED=OW-WAR-0062
REPIN_DIR=docs/warrants/$REPIN_ALIAS

# An amendment that carries only what the record needs, plus the pin.
repin_amendment() {
    local dir="$1" version="$2" stage="$3" milestone="$4"
    mkdir -p "$dir/amendments"
    cat > "$dir/amendments/AM-901.yaml" <<YAML
schema: "oh.war/amendment/v1"

id: "AM-901"
band: "manual_revision"
reason: "Re-pin to SAS $version (planted by conformance/plants.d/88-sas-repin.sh)."
governing_adr_or_policy: "adr://OW-ADR-0016"
artifact_admissibility: "remain_admissible"
restart_or_repair_instruction: "Continue; the Basis names a newer SAS revision."
re_preflight_required: "false"
authorizer: "QuiteTall"
effective_time: "2026-09-12"
sas_revision: "$version"
predecessor_sas_revision: "0.1.0-draft.3"

semantic_diff:
  - element: "basis"
    before: "pinned to the authorization's SAS revision"
    after: "pinned to SAS $version"

affected_stages: ["$stage"]
affected_milestones: ["$milestone"]
YAML
}

# A recorded revision nobody proposed through the tool: 1.0.0's record minus
# the row OW-WAR-0047 implements, so the re-pin has a requirement to lose.
fake_revision() {
    python3 - <<'PY'
import re, pathlib
src = pathlib.Path("docs/sas/revisions/1.0.0.toml").read_text()
rq = re.search(r'sas://(WAR-SAS-RQ-\d+)', pathlib.Path("docs/warrants/OW-WAR-0047/manifest.toml").read_text()).group(1)
out = src.replace('version = "1.0.0"', 'version = "9.9.9"').replace('predecessor = "0.1.0-draft.3"', 'predecessor = "1.0.0"')
out = re.sub(rf'^{rq} = .*\n', '', out, flags=re.M)
pathlib.Path("docs/sas/revisions/9.9.9.toml").write_text(out)
PY
}

plant "a re-pin to an unrecorded revision" "sas.pin-unknown" "7.7.7" 2 \
    "repin_amendment $REPIN_DIR 7.7.7 STAGE-001 M1" \
    "$REPIN_ALIAS"

plant "a re-pin that loses an implemented row" "sas.repin-unknown-requirement" "9.9.9" 2 \
    "fake_revision; repin_amendment $REPIN_DIR 9.9.9 STAGE-001 M1" \
    "$REPIN_ALIAS"

# A resolved Warrant re-pinned: the resolution is stale, never moved.
plant "a re-pin after resolution is stale" "resolution.stale" "" 2 \
    "repin_amendment docs/warrants/$REPIN_RESOLVED 1.0.0 STAGE-001 M1" \
    "$REPIN_RESOLVED"

# Positive: a sound re-pin moves the contract digest, and the pending act is
# the NEXT authorization revision — a human's, with the amendment beside it.
#
# Which number that is comes from the record, not from this file. It was
# hard-coded as "rev 2" until OW-WAR-0047 gained a real amendment of its own and
# the answer became 3; a plant that names a revision names a moment in the
# corpus, and the corpus moves.
REPIN_NOW=$(grep -m1 '^revision = ' "$REPIN_DIR/authorization.toml" | tr -dc '0-9')
REPIN_NEXT=$((REPIN_NOW + 1))
plant_cmd "a sound re-pin asks for authorization rev $REPIN_NEXT" "rev $REPIN_NEXT" "authorize" 0 \
    "repin_amendment $REPIN_DIR 1.0.0 STAGE-001 M1" \
    sign --list

# ---------------------------------------------------------------------------
# `war sas repin` (OW-WAR-0112 M5): the tool writes the amendment the plants
# above fabricate by hand. Each plant removes what the command wrote — the
# battery's restore knows AM-901, not the ids the tool numbers.
echo "== sas repin (OW-WAR-0112) =="
REPIN_TOOL_ID=$(printf 'AM-%03d' $(( $(ls "$REPIN_DIR/amendments" 2>/dev/null | grep -c '^AM-[0-9]*\.yaml$') + 1 )))

# One Warrant: the amendment lands and the queue asks for revision N+1.
plant_cmd "repin writes the amendment; rev $REPIN_NEXT pending" "rev $REPIN_NEXT" "authorize" 0 \
    "\"$WAR\" sas repin $REPIN_ALIAS >/dev/null 2>&1; assert_present 'predecessor_sas_revision' '$REPIN_DIR/amendments/$REPIN_TOOL_ID.yaml'" \
    sign --list
rm -f "$REPIN_DIR/amendments/$REPIN_TOOL_ID.yaml"

# The amendment the tool writes is the one `check` reads: it names the
# latest revision and the pin it moved from, and no `sas.repin-*` fires.
REPIN_LATEST=$(ls docs/sas/revisions/*.toml | xargs -n1 basename | sed 's/\.toml$//' | sort -V | tail -1)
"$WAR" sas repin "$REPIN_ALIAS" >/dev/null 2>&1
REPIN_CHK=$("$WAR" check "$REPIN_ALIAS" 2>&1)
if grep -q "sas_revision: \"$REPIN_LATEST\"" "$REPIN_DIR/amendments/$REPIN_TOOL_ID.yaml" \
    && ! grep -q 'sas.repin-unknown-requirement\|sas.pin-unknown' <<<"$REPIN_CHK" \
    && ! grep -q "sas.pin-superseded .*$REPIN_ALIAS" <<<"$REPIN_CHK"; then
    printf 'ok    %-34s pinned to %s, the warning gone\n' "the written re-pin checks clean" "$REPIN_LATEST"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "the written re-pin checks clean" "$(grep -E 'sas\.' <<<"$REPIN_CHK" | grep -v '^PASS' | head -2 | tr '\n' '|')"
    FAILED=$((FAILED + 1))
fi
rm -f "$REPIN_DIR/amendments/$REPIN_TOOL_ID.yaml"
plant_restore

# A resolved Warrant named outright: refused by name, nothing written.
REPIN_OUT=$("$WAR" sas repin "$REPIN_RESOLVED" 2>&1)
REPIN_STATUS=$?
if [[ $REPIN_STATUS -eq 2 ]] && grep -q 'sas.repin-resolved' <<<"$REPIN_OUT" && [[ ! -d "docs/warrants/$REPIN_RESOLVED/amendments" ]]; then
    printf 'ok    %-34s refused, nothing written\n' "repin of a resolved Warrant"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; amendments dir: %s\n' "repin of a resolved Warrant" "$REPIN_STATUS" "$([[ -d docs/warrants/$REPIN_RESOLVED/amendments ]] && echo present || echo absent)"
    FAILED=$((FAILED + 1))
fi
plant_restore

# `--all --dry-run`: every candidate named, nothing written, tree unchanged.
REPIN_BEFORE=$(git status --porcelain | sort)
REPIN_DRY=$("$WAR" sas repin --all --dry-run 2>&1)
REPIN_AFTER=$(git status --porcelain | sort)
REPIN_N=$(grep -c 'sas.repin-would-write' <<<"$REPIN_DRY")
if [[ "$REPIN_BEFORE" == "$REPIN_AFTER" ]] && [[ $REPIN_N -gt 0 ]] && ! grep -q 'sas.repin-written' <<<"$REPIN_DRY"; then
    printf 'ok    %-34s %s candidate(s), nothing written\n' "repin --all --dry-run" "$REPIN_N"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s candidate(s); tree moved: %s\n' "repin --all --dry-run" "$REPIN_N" "$([[ "$REPIN_BEFORE" == "$REPIN_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi
