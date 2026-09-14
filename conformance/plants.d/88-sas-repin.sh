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
