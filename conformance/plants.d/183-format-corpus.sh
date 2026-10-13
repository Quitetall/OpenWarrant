# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Format 1.0's conformance corpus (docs/format/FORMAT-1.0.md section 13.1;
# conformance/format/) through war:
# - war passes every case except those known-war.txt lists, and every listed
#   case still fails (a settled divergence must leave the list); every rule
#   id of section 11.2 is expected by some case.
# - Refused: the same run over a copy of the corpus in which one case's
#   expected refusal names another line, and in which one accept case's
#   expected model has a changed content_digest, each fails by that case's
#   name. The copy is in a temporary directory; nothing in the tree moves.

echo "== Format 1.0 conformance corpus (FORMAT-1.0 section 13.1) =="
FC_WAR="$REPO_ROOT/${WAR#./}"
FC_DIR="$REPO_ROOT/conformance/format"
fc_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

FC_OUT=$(python3 "$FC_DIR/run.py" --reader war --war "$FC_WAR" --known "$FC_DIR/known-war.txt" 2>&1)
FC_RC=$?
FC_SUMMARY=$(printf '%s\n' "$FC_OUT" | tail -n 1)
if [[ $FC_RC -eq 0 && "$FC_SUMMARY" == "war: "*" passed, 0 failed, "* ]]; then
    fc_ok "war passes the corpus" "$FC_SUMMARY"
else
    printf '%s\n' "$FC_OUT" | grep -E '^FAIL' >&2
    fc_fail "war passes the corpus" "exit $FC_RC: $FC_SUMMARY"
fi

# --- the runner is seen to fail on a changed expectation ---------------------------------
FC_TMP=$(mktemp -d)
mkdir -p "$FC_TMP/conformance"
cp -R "$FC_DIR" "$FC_TMP/conformance/format"
FC_REF="$FC_TMP/conformance/format/111-key-unknown-frontmatter/expected/refusal"
FC_MOD="$FC_TMP/conformance/format/006-records-document/expected/model.json"
if [[ -f "$FC_REF" && -f "$FC_MOD" ]]; then
    sed -i 's/:7$/:8/' "$FC_REF"
    python3 - "$FC_MOD" <<'PY'
import json, sys
p = sys.argv[1]
m = json.load(open(p))
m["records"][0]["content_digest"] = "sha256:" + "0" * 64
open(p, "w").write(json.dumps(m, sort_keys=True, separators=(",", ":")) + "\n")
PY
    FC_BAD=$(python3 "$FC_TMP/conformance/format/run.py" --reader war --war "$FC_WAR" \
        --known "$FC_TMP/conformance/format/known-war.txt" 2>&1)
    FC_BAD_RC=$?
    if [[ $FC_BAD_RC -eq 1 ]] \
        && grep -q '^FAIL  111-key-unknown-frontmatter: ' <<<"$FC_BAD" \
        && grep -q '^FAIL  006-records-document: records: ' <<<"$FC_BAD" \
        && [[ $(grep -c '^FAIL' <<<"$FC_BAD") -eq 2 ]]; then
        fc_ok "a changed expectation fails" "111 (refusal line) and 006 (content_digest) fail by name; nothing else"
    else
        fc_fail "a changed expectation fails" "exit $FC_BAD_RC: $(grep '^FAIL' <<<"$FC_BAD" | head -3 | tr '\n' ' ')"
    fi
else
    fc_fail "a changed expectation fails" "the cases the control edits are missing"
fi
rm -rf "$FC_TMP"
unset FC_WAR FC_DIR FC_OUT FC_RC FC_SUMMARY FC_TMP FC_REF FC_MOD FC_BAD FC_BAD_RC
