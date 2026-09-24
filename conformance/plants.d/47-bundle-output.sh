# shellcheck shell=bash
# OW-WAR-0146 — a verification bundle carries what each gate printed.
#
# A scratch program records one software.repo.war-check run through `war
# evidence record`, then bundles it. The blind verifier decides from the
# bundle alone (OW-WAR-0117); an output left out is evidence it cannot see.

echo "== bundled gate output (OW-WAR-0146) =="
PLANT_ROOT=$(scratch_corpus BO)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
BO_W="$PLANT_ROOT/docs/warrants/BO-WAR-0001"
BO_GATE=software.repo.war-check@1.0.0
BO_OUT="$BO_W/gate-runs/software_repo_war-check_1_0_0.stdout.txt"
mkdir -p "$PLANT_ROOT/docs/gates"
command cp "docs/gates/$BO_GATE.yaml" "$PLANT_ROOT/docs/gates/"
# The gate runs ./target/debug/war: the scratch shares this build.
ln -s "$(readlink -f target)" "$PLANT_ROOT/target"
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a gate to record" >/dev/null 2>&1
"$WAR" --root "$PLANT_ROOT" evidence record BO-WAR-0001 >/dev/null 2>&1
bo_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
bo_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
bo_bundle() { # → path of the bundle just written
    command rm -f "$BO_W"/verifications/bundle-*.json
    "$WAR" --root "$PLANT_ROOT" verify BO-WAR-0001 --performer claude --bundle >/dev/null 2>&1
    ls "$BO_W"/verifications/bundle-*.json 2>/dev/null | head -1
}
bo_field() { # bundle, python expression over run → printed value
    python3 -c "
import json, sys
b = json.load(open(sys.argv[1]))
run = next(r for r in b['gate_runs'] if r['gate'] == sys.argv[2])
print($2)" "$1" "$BO_GATE"
}

# OBL-001: the output travels with the run, whole-file digest, tail text.
[[ -s "$BO_OUT" ]] || { printf 'PLANT SETUP FAILED: no recorded stdout at %s\n' "$BO_OUT" >&2; exit 9; }
BO_B=$(bo_bundle)
BO_SHA=$(sha256sum "$BO_OUT" | cut -c1-64)
if [[ -n "$BO_B" ]] && [[ "$(bo_field "$BO_B" "run['stdout']['sha256']")" == "$BO_SHA" ]] \
    && [[ "$(bo_field "$BO_B" "run['stdout']['captured']")" == "True" ]] \
    && [[ "$(bo_field "$BO_B" "run['stdout']['text'] == open('$BO_OUT').read()[-len(run['stdout']['text']):]")" == "True" ]]; then
    bo_ok "the gate's stdout is in the bundle" "captured, text is the file's, digest $BO_SHA"
else
    bo_fail "the gate's stdout is in the bundle" "bundle ${BO_B:-not written}"
fi
# A cap below the file's size: truncated, the digest still of the whole file.
# Inside the scaffold's existing [verify] table: a second table is a TOML
# error, not a smaller cap.
python3 - "$PLANT_ROOT/openwarrant.toml" <<'PY'
import re, sys
p = sys.argv[1]; t = open(p).read()
t = re.sub(r"(?m)^max_excerpt_bytes = .*\n", "", t)
t = re.sub(r"(?m)^\[verify\]$", "[verify]\nmax_excerpt_bytes = 64", t, count=1) if "[verify]" in t else t + "\n[verify]\nmax_excerpt_bytes = 64\n"
open(p, "w").write(t)
PY
grep -q '^max_excerpt_bytes = 64' "$PLANT_ROOT/openwarrant.toml" || { printf 'PLANT SETUP FAILED: could not set max_excerpt_bytes\n' >&2; exit 9; }
BO_B=$(bo_bundle)
if [[ "$(bo_field "$BO_B" "run['stdout']['truncated']")" == "True" ]] \
    && [[ "$(bo_field "$BO_B" "len(run['stdout']['text'].encode())")" -le 64 ]] \
    && [[ "$(bo_field "$BO_B" "run['stdout']['sha256']")" == "$BO_SHA" ]]; then
    bo_ok "a long output is cut, not re-digested" "truncated at 64 bytes, whole-file digest"
else
    bo_fail "a long output is cut, not re-digested" "$(bo_field "$BO_B" "run['stdout']" 2>&1 | head -c 200)"
fi
git -C "$PLANT_ROOT" checkout -q -- openwarrant.toml
# Refusal: an absent capture is `captured: false` with no text at all.
command mv "$BO_OUT" "$BO_OUT.away"
BO_B=$(bo_bundle)
command mv "$BO_OUT.away" "$BO_OUT"
if [[ "$(bo_field "$BO_B" "run['stdout']['captured']")" == "False" ]] \
    && [[ "$(bo_field "$BO_B" "'text' in run['stdout']")" == "False" ]]; then
    bo_ok "a missing output is not silence" "captured false, no text"
else
    bo_fail "a missing output is not silence" "$(bo_field "$BO_B" "run['stdout']" 2>&1 | head -c 200)"
fi

# OBL-002: no stdout digest in the receipt, so no mismatch is claimed.
BO_B=$(bo_bundle)
if [[ "$(bo_field "$BO_B" "'stdout_digest' in (run['receipt'] or {})")" == "False" ]] \
    && [[ "$(bo_field "$BO_B" "'mismatch' in run['stdout']")" == "False" ]]; then
    bo_ok "no digest recorded, no mismatch" "the receipt records none; the field is absent"
else
    bo_fail "no digest recorded, no mismatch" "$(bo_field "$BO_B" "run['stdout'].get('mismatch')" 2>&1)"
fi

# OBL-003: deterministic, and still ingested by a verifier.
command rm -f "$BO_W"/verifications/bundle-*.json
"$WAR" --root "$PLANT_ROOT" verify BO-WAR-0001 --performer claude --bundle >/dev/null 2>&1
"$WAR" --root "$PLANT_ROOT" verify BO-WAR-0001 --performer claude --bundle >/dev/null 2>&1
BO_N=$(ls "$BO_W"/verifications/bundle-*.json 2>/dev/null | wc -l)
command cp -r conformance/fixtures "$PLANT_ROOT/conformance-fixtures"
sed -i "s|^verifier_argv = .*|verifier_argv = [\"bash\", \"$PLANT_ROOT/conformance-fixtures/verifier/establishes-all.sh\"]|" "$PLANT_ROOT/openwarrant.toml"
BO_RUN=$("$WAR" --root "$PLANT_ROOT" verify BO-WAR-0001 --performer claude --run 2>&1)
if [[ "$BO_N" -eq 1 ]] && grep -q 'verify.recorded' <<<"$BO_RUN"; then
    bo_ok "one digest, and the verifier ingests" "one bundle file after two runs; verify.recorded"
else
    bo_fail "one digest, and the verifier ingests" "$BO_N bundle file(s); $(grep -E '^(ERROR|WARN)' <<<"$BO_RUN" | head -1)"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT BO_B
