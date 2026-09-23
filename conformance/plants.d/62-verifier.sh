# shellcheck shell=bash
# OW-WAR-0117 — the blind verifier: the model chooses verdicts and nothing
# else; anything unclear is not_established; it holds no tool; independence
# is claimed only where it is true.
#
# A fake `claude` (CLAUDE_BIN) records the argv it was given and answers from
# a mode, so no plant here calls a model. The real CLI's behaviour under the
# same flags is recorded in docs/VERIFICATION.md, not asserted here.

echo "== the blind verifier (OW-WAR-0117) =="
PLANT_ROOT=$(scratch_corpus VF)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
VF_TMP=$(mktemp -d)
VF_WRAP=$(realpath tools/verifier/claude-verifier.sh)
cat > "$VF_TMP/claude" <<'FAKE'
#!/usr/bin/env bash
if [[ "${1:-}" == "--version" ]]; then echo "fake-claude 0.0"; exit 0; fi
printf '%s\n' "$@" > "$VF_LOG/argv"
pwd > "$VF_LOG/cwd"
bundle=$(cat)
case "$VF_MODE" in
    fail) echo "boom" >&2; exit 1 ;;
    garbage) echo "I think it is probably fine." ;;
    *) python3 - "$VF_MODE" "$bundle" <<'PY'
import json, sys
mode, bundle = sys.argv[1], json.loads(sys.argv[2])
obls = [o["id"] for o in bundle["request"]["obligations"]]
if mode == "skip":
    obls = obls[1:]
out = []
for i, o in enumerate(obls):
    d = "refuted" if i == 1 else "established"
    if mode == "probably":
        d = "probably"
    v = {"obligation": o, "disposition": d, "evidence": f"fixture evidence for {o}"}
    if mode == "claim":
        v.update({"verifier": {"actor": "the-performer", "kind": "human"},
                  "independence": {k: False for k in ["performer_transcript_blind", "cannot_modify_subject_artifacts"]},
                  "distinct_human_required": True})
    out.append(v)
doc = {"verdicts": out}
if mode == "claim":
    doc["actor"] = "the-performer"
print(json.dumps(doc))
PY
    ;;
esac
FAKE
chmod +x "$VF_TMP/claude"
"$WAR" --root "$PLANT_ROOT" verify VF-WAR-0001 --performer claude --bundle >/dev/null 2>&1
VF_BUNDLE=$(ls "$PLANT_ROOT"/docs/warrants/VF-WAR-0001/verifications/bundle-*.json | head -1)
VF_OBLS=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["request"]["obligations"]))' "$VF_BUNDLE")
vf() { # mode, extra env... → stdout of the wrapper; exit in VF_STATUS
    local mode=$1; shift
    VF_OUT=$(env CLAUDE_BIN="$VF_TMP/claude" VF_LOG="$VF_TMP" VF_MODE="$mode" "$@" "$VF_WRAP" "$VF_BUNDLE" 2>"$VF_TMP/stderr"); VF_STATUS=$?
}
vf_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
vf_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
vf_count() { grep -c "^disposition = \"$1\"" <<<"$VF_OUT"; }
vf_flags() { grep -E '^(actor|kind|performer_|separate_|cannot_|distinct_)' <<<"$VF_OUT" | sort | uniq -c; }

# OBL-001: verdicts pass through; nothing the model says about who it is or
# what independence it has reaches the response.
vf good; VF_GOOD=$VF_OUT; VF_GOOD_FLAGS=$(vf_flags)
if [[ $VF_STATUS -eq 0 && $VF_OBLS -ge 2 ]] && [[ $(vf_count established) -eq $((VF_OBLS - 1)) && $(vf_count refuted) -eq 1 ]] \
    && grep -q 'fixture evidence for OBL-001' <<<"$VF_OUT"; then
    vf_ok "verdicts pass through" "$((VF_OBLS - 1)) established, 1 refuted, evidence kept"
else
    vf_fail "verdicts pass through" "exit $VF_STATUS, $VF_OBLS obligation(s): $(head -c 200 <<<"$VF_OUT")"
fi
vf claim
if [[ $VF_STATUS -eq 0 ]] && [[ "$(vf_flags)" == "$VF_GOOD_FLAGS" ]] && ! grep -q 'the-performer' <<<"$VF_OUT" \
    && [[ "$(grep '^disposition' <<<"$VF_OUT")" == "$(grep '^disposition' <<<"$VF_GOOD")" ]]; then
    vf_ok "the model cannot name itself" "identity and flags identical with and without its claims"
else
    vf_fail "the model cannot name itself" "$(diff <(echo "$VF_GOOD_FLAGS") <(vf_flags) | head -3 | tr '\n' '|')"
fi

# OBL-002: unclear answers settle nothing.
for VF_M in garbage skip probably; do
    vf "$VF_M"
    case $VF_M in
        skip) VF_WANT=1 ;;               # the skipped first obligation only
        *) VF_WANT=$VF_OBLS ;;           # every obligation
    esac
    if [[ $VF_STATUS -eq 0 && $(vf_count not_established) -ge $VF_WANT ]] \
        && { [[ $VF_M != skip ]] || grep -q 'no usable verdict' <<<"$VF_OUT"; }; then
        vf_ok "an answer that is $VF_M" "$(vf_count not_established) not_established"
    else
        vf_fail "an answer that is $VF_M" "exit $VF_STATUS: $(grep '^disposition' <<<"$VF_OUT" | tr '\n' ' ')"
    fi
done
vf probably
if ! grep -q '^disposition = "probably"' <<<"$VF_OUT"; then
    vf_ok "an unknown disposition is dropped" "no \"probably\" in the response"
else
    vf_fail "an unknown disposition is dropped" "\"probably\" reached the response"
fi

# A failing verifier makes `war verify --run` write nothing.
# The scaffold already has a [verify] table; name the wrapper in it.
sed -i "s|^verifier_argv = .*|verifier_argv = [\"$VF_WRAP\"]|" "$PLANT_ROOT/openwarrant.toml"
grep -q "^verifier_argv = \[\"$VF_WRAP\"\]" "$PLANT_ROOT/openwarrant.toml" || { printf 'PLANT SETUP FAILED: no verifier_argv line in the scaffold\n' >&2; exit 9; }
VF_BEFORE=$(find "$PLANT_ROOT/docs" -type f -name '*.toml' | sort | xargs sha256sum | sha256sum)
VF_RUN=$(CLAUDE_BIN="$VF_TMP/claude" VF_LOG="$VF_TMP" VF_MODE=fail "$WAR" --root "$PLANT_ROOT" verify VF-WAR-0001 --performer claude --run 2>&1); VF_STATUS=$?
VF_AFTER=$(find "$PLANT_ROOT/docs" -type f -name '*.toml' | sort | xargs sha256sum | sha256sum)
if [[ $VF_STATUS -ne 0 ]] && grep -q 'verify.verifier-failed' <<<"$VF_RUN" && [[ "$VF_BEFORE" == "$VF_AFTER" ]]; then
    vf_ok "a failing verifier records nothing" "verify.verifier-failed (exit $VF_STATUS), no record moved"
else
    vf_fail "a failing verifier records nothing" "exit $VF_STATUS, records $([[ "$VF_BEFORE" == "$VF_AFTER" ]] && echo same || echo moved): $(grep -E '^ERROR' <<<"$VF_RUN" | head -1)"
fi
# And the positive end to end: the same seam ingests a good answer.
VF_RUN=$(CLAUDE_BIN="$VF_TMP/claude" VF_LOG="$VF_TMP" VF_MODE=good "$WAR" --root "$PLANT_ROOT" verify VF-WAR-0001 --performer claude --run 2>&1); VF_STATUS=$?
if [[ -f "$PLANT_ROOT/docs/warrants/VF-WAR-0001/verifications/OBL-001.toml" ]] \
    && grep -q 'claude-verifier' "$PLANT_ROOT/docs/warrants/VF-WAR-0001/verifications/OBL-001.toml"; then
    vf_ok "war verify --run ingests it" "OBL-001 recorded under claude-verifier"
else
    vf_fail "war verify --run ingests it" "exit $VF_STATUS: $(grep -E '^(ERROR|WARN)' <<<"$VF_RUN" | head -2 | tr '\n' '|')"
fi

# OBL-003: no tool, no MCP, no session, no project context. Each flag is
# checked by name, so removing one fails here.
vf good
VF_ARGV=$(cat "$VF_TMP/argv")
VF_MISSING=""
for VF_F in --no-session-persistence --strict-mcp-config --restricted; do
    grep -qx -- "$VF_F" <<<"$VF_ARGV" || VF_MISSING="$VF_MISSING $VF_F"
done
grep -A1 -x -- '--tools' <<<"$VF_ARGV" | sed -n 2p | grep -qx '' || VF_MISSING="$VF_MISSING --tools-empty"
VF_DENY=$(grep -A1 -x -- '--disallowedTools' <<<"$VF_ARGV" | sed -n 2p)
for VF_T in Bash Read Write Edit; do
    grep -q "\\b$VF_T\\b" <<<"$VF_DENY" || VF_MISSING="$VF_MISSING deny:$VF_T"
done
case "$(cat "$VF_TMP/cwd")" in "$REPO_ROOT"*|"$PLANT_ROOT"*) VF_MISSING="$VF_MISSING cwd-in-a-repository" ;; esac
if [[ -z "$VF_MISSING" ]]; then
    vf_ok "the verifier holds no tool" "--tools '', no MCP, restricted, no session, empty cwd"
else
    vf_fail "the verifier holds no tool" "missing:$VF_MISSING"
fi

# OBL-004: distinct_model is claimed only when checked; no human, ever.
vf good; VF_A=$(grep -c '^distinct_model_required = true' <<<"$VF_OUT")
vf good CLAUDE_PERFORMER_MODEL=claude-sonnet-5; VF_B=$(grep -c '^distinct_model_required = true' <<<"$VF_OUT")
vf good CLAUDE_PERFORMER_MODEL=claude-opus-5-5; VF_C=$(grep -c '^distinct_model_required = true' <<<"$VF_OUT")
VF_H=$(grep -c '^distinct_human_required = true' <<<"$VF_OUT")
if [[ $VF_A -eq 0 && $VF_B -eq 0 && $VF_C -eq $VF_OBLS && $VF_H -eq 0 ]]; then
    vf_ok "independence only where true" "distinct model: unset no, same no, different yes; human never"
else
    vf_fail "independence only where true" "unset $VF_A, same $VF_B, different $VF_C of $VF_OBLS, human $VF_H"
fi

command rm -rf "$VF_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
