# shellcheck shell=bash
# OW-WAR-0145 — the battery as an askable gate: gate://ops.conformance.plants@1.1.0
# runs the battery in a disposable clone of HEAD and touches nothing of the
# working tree. Exercised with a FAKE battery on a scratch corpus: the real
# battery inside the battery would recurse, and that refusal is a plant too.

echo "== the isolated battery gate (OW-WAR-0145) =="
PLANT_ROOT=$(scratch_corpus IB)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
IB_TMP=$(mktemp -d)
IB_GATE=ops.conformance.plants@1.1.0
mkdir -p "$PLANT_ROOT/conformance" "$PLANT_ROOT/docs/gates"
command cp conformance/plant-isolated.sh "$PLANT_ROOT/conformance/"
command cp "docs/gates/$IB_GATE.yaml" "$PLANT_ROOT/docs/gates/"
# The fake battery prints the bytes of a tracked file (so the plant can see
# which tree it read) and exits with IB_EXIT.
cat > "$PLANT_ROOT/conformance/fake-battery.sh" <<'FAKE'
#!/usr/bin/env bash
echo "fake battery sees: $(cat conformance/probe.txt)"
exit "${IB_EXIT:-0}"
FAKE
printf 'committed\n' > "$PLANT_ROOT/conformance/probe.txt"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "the isolated battery and a fake one" >/dev/null 2>&1
IB_HEAD=$(git -C "$PLANT_ROOT" rev-parse HEAD)
# Uncommitted state the gate must neither test nor touch.
printf 'uncommitted\n' > "$PLANT_ROOT/conformance/probe.txt"
printf 'marker\n' > "$PLANT_ROOT/uncommitted-marker.txt"
# Everything but docs/receipts/, where `war` itself writes the §44.6 receipt
# of the run (gitignored in a real repository): that is the recorder
# recording, not the gate mutating.
ib_tree() { (cd "$PLANT_ROOT" && { git rev-parse HEAD; git diff; git diff --cached; find . \( -path ./.git -o -path ./docs/receipts \) -prune -o -type f -print0 | sort -z | xargs -0 sha256sum; sha256sum .git/index; } | sha256sum); }
ib_run() { # IB_EXIT → output of `war gate --run` in the scratch corpus
    # -u OPENWARRANT_IN_BATTERY: this plant runs inside a battery (lib.sh
    # exports it, and so does the gate itself); the run under test is the
    # outermost one, and must not be refused as nested.
    (cd "$PLANT_ROOT" && env -u OPENWARRANT_IN_BATTERY TMPDIR="$IB_TMP" IB_EXIT="$1" OPENWARRANT_ISOLATED_PLANT_SH=conformance/fake-battery.sh \
        "$WAR_ABS" --root "$PLANT_ROOT" gate --run --gate "$IB_GATE" 2>&1)
}
WAR_ABS=$(realpath "$WAR")
ib_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ib_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# OBL-001: askable, passes, names HEAD, reads HEAD's bytes, touches nothing.
IB_BEFORE=$(ib_tree)
IB_OUT=$(ib_run 0)
IB_AFTER=$(ib_tree)
if grep -q 'askability askable · execution completed · verdict pass' <<<"$IB_OUT"; then
    ib_ok "the battery gate is askable" "ran and passed"
else
    ib_fail "the battery gate is askable" "$(grep -E 'gate-run' <<<"$IB_OUT" | head -3 | tr '\n' '|')"
fi
if [[ "$IB_BEFORE" == "$IB_AFTER" ]]; then
    ib_ok "the working tree is untouched" "files, index, HEAD and the uncommitted edit identical"
else
    ib_fail "the working tree is untouched" "the tree moved during the run"
fi
if [[ -z "$(ls -A "$IB_TMP" 2>/dev/null)" ]]; then
    ib_ok "the clone is removed" "nothing left under the temp directory"
else
    ib_fail "the clone is removed" "left behind: $(ls "$IB_TMP" | tr '\n' ' ')"
fi
# The gate's captured output lands in the run record; read it from there when
# the report does not print it.
IB_SEEN=$(grep -rh 'fake battery sees\|tested commit' "$PLANT_ROOT/docs/receipts" "$IB_TMP" 2>/dev/null; grep -h 'fake battery sees\|tested commit' <<<"$IB_OUT")
IB_SEEN="$IB_SEEN$(find "$PLANT_ROOT" -name '*stdout*' -newer "$PLANT_ROOT/uncommitted-marker.txt" -exec cat {} + 2>/dev/null)"
if grep -q 'fake battery sees: committed' <<<"$IB_SEEN" && grep -q "tested commit $IB_HEAD" <<<"$IB_SEEN" \
    && ! grep -q 'fake battery sees: uncommitted' <<<"$IB_SEEN"; then
    ib_ok "it tests HEAD, not the working tree" "committed bytes, tested commit $IB_HEAD"
else
    ib_fail "it tests HEAD, not the working tree" "$(head -c 200 <<<"$IB_SEEN" | tr '\n' '|')"
fi

# Refusal: a failing battery fails the gate.
IB_OUT=$(ib_run 1)
if grep -q 'ERROR gate-run.fail .*askability askable · execution completed · verdict fail' <<<"$IB_OUT"; then
    ib_ok "a failing battery fails the gate" "verdict fail"
else
    ib_fail "a failing battery fails the gate" "$(grep -E 'gate-run' <<<"$IB_OUT" | head -3 | tr '\n' '|')"
fi

# OBL-002: a battery inside the battery refuses instead of recursing.
IB_OUT=$(cd "$PLANT_ROOT" && env TMPDIR="$IB_TMP" OPENWARRANT_IN_BATTERY=1 bash conformance/plant-isolated.sh 2>&1); IB_STATUS=$?
if [[ $IB_STATUS -eq 2 ]] && grep -q 'plant-isolated.refused-nested' <<<"$IB_OUT" && [[ -z "$(ls -A "$IB_TMP" 2>/dev/null)" ]]; then
    ib_ok "a nested battery refuses" "exit 2, refused-nested, no clone made"
else
    ib_fail "a nested battery refuses" "exit $IB_STATUS: $(head -c 150 <<<"$IB_OUT")"
fi

# OBL-003: registered in this repository as non-mutating; @1.0.0 untouched.
IB_CHECK=$("$WAR" check 2>&1)
if grep -q "PASS gate.registered .*$IB_GATE" <<<"$IB_CHECK" && grep -q 'mutating: "false"' "docs/gates/$IB_GATE.yaml" \
    && grep -q 'mutating: "true"' docs/gates/ops.conformance.plants@1.0.0.yaml; then
    ib_ok "1.1.0 registered, 1.0.0 untouched" "1.1.0 non-mutating; 1.0.0 still mutating"
else
    ib_fail "1.1.0 registered, 1.0.0 untouched" "$(grep 'ops.conformance.plants' <<<"$IB_CHECK" | head -2 | tr '\n' '|')"
fi

# OBL-003, the rest: @1.0.0's bytes are the ones it had before OW-WAR-0145
# delivered @1.1.0 (the parent of the commit that added it), and `war check`
# on this repository reports no error. The refusal: one byte more is not seen
# as the same file, so the comparison can fail.
IB_OLD=docs/gates/ops.conformance.plants@1.0.0.yaml
IB_ADDED=$(git log --diff-filter=A --format=%H -- "docs/gates/$IB_GATE.yaml" | tail -1)
IB_BEFORE=$( [[ -n "$IB_ADDED" ]] && git show "$IB_ADDED^:$IB_OLD" 2>/dev/null | sha256sum | cut -d' ' -f1)
IB_NOW=$(sha256sum "$IB_OLD" | cut -d' ' -f1)
if [[ -z "$IB_ADDED" || -z "$IB_BEFORE" ]]; then
    ib_fail "1.0.0 is byte-identical" "UNKNOWN: no commit adds $IB_GATE.yaml in this history"
elif [[ "$IB_NOW" == "$IB_BEFORE" ]]; then
    ib_ok "1.0.0 is byte-identical" "sha256:${IB_NOW:0:12}, as before ${IB_ADDED:0:8}"
else
    ib_fail "1.0.0 is byte-identical" "sha256:${IB_NOW:0:12} now, sha256:${IB_BEFORE:0:12} before ${IB_ADDED:0:8}"
fi
IB_PLUS=$( { cat "$IB_OLD"; printf 'x'; } | sha256sum | cut -d' ' -f1)
if [[ "$IB_PLUS" != "$IB_BEFORE" ]]; then
    ib_ok "a changed 1.0.0 is seen" "one byte more is a different digest"
else
    ib_fail "a changed 1.0.0 is seen" "the comparison could not fail"
fi
if grep -qE '^[0-9]+ pass · [0-9]+ warn · [0-9]+ unknown · 0 error ' <<<"$IB_CHECK" && ! grep -q '^ERROR ' <<<"$IB_CHECK"; then
    ib_ok "war check reports no error" "$(grep -oE '[0-9]+ pass · .* error' <<<"$IB_CHECK" | head -1)"
else
    ib_fail "war check reports no error" "$(grep -E '^ERROR ' <<<"$IB_CHECK" | head -2 | cut -c1-160 | tr '\n' '|')"
fi

command rm -rf "$IB_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT IB_TMP IB_OUT IB_SEEN
