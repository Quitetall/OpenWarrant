# shellcheck shell=bash
# OW-WAR-0148 M17 (decisions 21-23; docs/SCORE.md): the compliance score.
#
# On a scratch program (SC) with a ticket, three commits (one citing it), and a
# done tick:
# - the score is deterministic: `war admin score --json` twice, and once in
#   a fresh clone of the same commit, give the same bytes; `war status --json`
#   carries the same score as `compliance`; the published weights sum to 1000.
#   Refused: a statement whose score is moved by one point (`--verify`
#   recomputes every number from its counts).
# - an unmeasurable dimension reads UNKNOWN and earns 0: `prs` (no merged PR)
#   is `unknown` with no counts and 0 points, and the score is exactly the
#   sum of the measured points; a shallow clone reads `commits`, `prs` and
#   `ledger` UNKNOWN and scores no higher than the full clone. Refused: a
#   statement in which the UNKNOWN `prs` claims its 150 points
#   (score.statement, exit 2).
# - the in-toto output has its documented shape: `_type` Statement v1, one
#   subject whose `digest.gitCommit` is HEAD, the work-score predicateType,
#   the seven dimensions in order; `--verify` passes it. Refused: the same
#   statement with another predicateType, and with a sha256 subject digest.

echo "== the compliance score: deterministic, UNKNOWN earns 0, in-toto shape (M17) =="
SC_TMP=$(mktemp -d)
SC_ROOT=$(scratch_corpus SC)
[[ -d "${SC_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
SC_WAR="$REPO_ROOT/${WAR#./}"
sc_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sc_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$SC_WAR" --root "${SC_AT:-$SC_ROOT}" "$@" </dev/null; }
sc_git() { git -C "$SC_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false "$@"; }
sc_py() { python3 -c "import json, sys; v = json.load(open(sys.argv[1])); print($1)" "$2" 2>&1; }

SC_ID=$(sc_war --json create "Fix the login redirect" --item "Read the cookie first" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])' 2>/dev/null)
mkdir -p "$SC_ROOT/src"
printf 'fn redirect() {}\n' > "$SC_ROOT/src/redirect.rs"
sc_git add -A >/dev/null 2>&1
sc_git commit -qm "Read the cookie before the redirect" -m "Warrant: $SC_ID" >/dev/null 2>&1
printf 'fn other() {}\n' > "$SC_ROOT/src/other.rs"
sc_git add -A >/dev/null 2>&1
sc_git commit -qm "An uncited change" >/dev/null 2>&1
sc_war claim "$SC_ID" >/dev/null 2>&1
sc_war done "$SC_ID" >/dev/null 2>&1
sc_git add -A >/dev/null 2>&1
sc_git commit -qm "Done" >/dev/null 2>&1
[[ -n "$SC_ID" ]] || { printf 'PLANT SETUP FAILED: no ticket\n' >&2; exit 9; }

# --- deterministic --------------------------------------------------------------------------
sc_war --json admin score > "$SC_TMP/a.json" 2>/dev/null
sc_war --json admin score > "$SC_TMP/b.json" 2>/dev/null
git clone -q --no-hardlinks "$SC_ROOT" "$SC_TMP/clone" >/dev/null 2>&1
SC_AT="$SC_TMP/clone" sc_war --json admin score > "$SC_TMP/c.json" 2>/dev/null
sc_war --json status > "$SC_TMP/status.json" 2>/dev/null
sc_war --json admin score --weights > "$SC_TMP/weights.json" 2>/dev/null
SC_A=$(sc_py 'json.dumps(v["result"], sort_keys=True)' "$SC_TMP/a.json")
SC_B=$(sc_py 'json.dumps(v["result"], sort_keys=True)' "$SC_TMP/b.json")
SC_C=$(sc_py 'json.dumps(v["result"], sort_keys=True)' "$SC_TMP/c.json")
SC_S=$(sc_py 'json.dumps(v["result"]["compliance"], sort_keys=True)' "$SC_TMP/status.json")
SC_SCORE=$(sc_py 'v["result"]["score"]' "$SC_TMP/a.json")
SC_COMMITS=$(sc_py '"%s/%s %s" % (*[(d["numerator"], d["denominator"], d["points"]) for d in v["result"]["dimensions"] if d["id"] == "commits"][0],)' "$SC_TMP/a.json")
SC_W=$(sc_py '"%s %s %s" % (v["result"]["schema"], v["result"]["total"], len(v["result"]["dimensions"]))' "$SC_TMP/weights.json")
if [[ -n "$SC_A" && "$SC_A" == "$SC_B" && "$SC_A" == "$SC_C" && "$SC_A" == "$SC_S" \
    && "$SC_COMMITS" == "1/4 50" && "$SC_W" == "oh.war/score-weights/v1 1000 7" ]]; then
    sc_ok "the score is deterministic" "score $SC_SCORE twice, in a fresh clone and in war status; commits 1/4 = 50; weights v1 sum 1000"
else
    sc_fail "the score is deterministic" "a/b/clone/status equal: $([[ "$SC_A" == "$SC_B" ]] && echo y)/$([[ "$SC_A" == "$SC_C" ]] && echo y)/$([[ "$SC_A" == "$SC_S" ]] && echo y); commits '$SC_COMMITS'; weights '$SC_W'"
fi

# --- UNKNOWN earns nothing --------------------------------------------------------------------
SC_PRS=$(sc_py '"%s %s %s" % (*[(d["status"], d["points"], "numerator" in d) for d in v["result"]["dimensions"] if d["id"] == "prs"][0],)' "$SC_TMP/a.json")
SC_SUM=$(sc_py 'sum(d["points"] for d in v["result"]["dimensions"]) == v["result"]["score"]' "$SC_TMP/a.json")
git clone -q --depth 1 "file://$SC_ROOT" "$SC_TMP/shallow" >/dev/null 2>&1
SC_AT="$SC_TMP/shallow" sc_war --json admin score > "$SC_TMP/shallow.json" 2>/dev/null
SC_SH=$(sc_py '" ".join(d["status"] for d in v["result"]["dimensions"][:3]) + " %d" % v["result"]["score"]' "$SC_TMP/shallow.json")
SC_SH_SCORE=${SC_SH##* }
if [[ "$SC_PRS" == "unknown 0 False" && "$SC_SUM" == "True" \
    && "$SC_SH" == "unknown unknown unknown "* && "$SC_SH_SCORE" =~ ^[0-9]+$ && $SC_SH_SCORE -le $SC_SCORE ]]; then
    sc_ok "an unmeasurable dimension is UNKNOWN" "prs: unknown, 0 points, no counts; the score is the measured sum; a shallow clone scores $SC_SH_SCORE <= $SC_SCORE"
else
    sc_fail "an unmeasurable dimension is UNKNOWN" "prs '$SC_PRS', sum '$SC_SUM', shallow '$SC_SH'"
fi

# --- the in-toto statement --------------------------------------------------------------------
sc_war admin score --in-toto > "$SC_TMP/st.json" 2>/dev/null
SC_HEAD=$(git -C "$SC_ROOT" rev-parse HEAD)
SC_SHAPE=$(python3 - "$SC_TMP/st.json" "$SC_HEAD" <<'PY' 2>&1
import json, sys
v = json.load(open(sys.argv[1]))
ok = (v["_type"] == "https://in-toto.io/Statement/v1"
      and len(v["subject"]) == 1 and v["subject"][0]["digest"] == {"gitCommit": sys.argv[2]}
      and v["subject"][0]["name"]
      and v["predicateType"] == "https://openwarrant.dev/attestation/work-score/v1"
      and v["predicate"]["weights"] == "oh.war/score-weights/v1"
      and [d["id"] for d in v["predicate"]["dimensions"]]
          == ["commits", "prs", "ledger", "documents", "tested", "verified", "approved"]
      and v["predicate"]["score"] == sum(d["points"] for d in v["predicate"]["dimensions"]))
print("shape" if ok else "wrong")
PY
)
SC_V_OUT=$(sc_war admin score --verify "$SC_TMP/st.json" 2>&1); SC_V_RC=$?
sc_tamper() { # name python-expression-on-v
    python3 - "$SC_TMP/st.json" "$SC_TMP/$1.json" "$2" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
exec(sys.argv[3])
json.dump(v, open(sys.argv[2], "w"))
PY
    sc_war --json admin score --verify "$SC_TMP/$1.json" > "$SC_TMP/$1.out" 2>/dev/null
    echo "$? $(sc_py '",".join(d["rule"] for d in v["diagnostics"]) + " " + " | ".join(d["message"] for d in v["diagnostics"])' "$SC_TMP/$1.out")"
}
SC_T_UNKNOWN=$(sc_tamper unknown 'd = [d for d in v["predicate"]["dimensions"] if d["id"] == "prs"][0]; d["points"] = 150; v["predicate"]["score"] += 150')
SC_T_ONE=$(sc_tamper one 'v["predicate"]["score"] += 1')
SC_T_TYPE=$(sc_tamper ptype 'v["predicateType"] = "https://slsa.dev/provenance/v1"')
SC_T_DIGEST=$(sc_tamper digest 'v["subject"][0]["digest"] = {"sha256": "00" * 32}')
if [[ "$SC_SHAPE" == "shape" && $SC_V_RC -eq 0 ]] && line_has -F 'score.statement' -E '^PASS' <<<"$SC_V_OUT"; then
    sc_ok "the in-toto statement has its shape" "Statement v1, subject gitCommit = HEAD, work-score predicate, 7 dimensions; --verify passes"
else
    sc_fail "the in-toto statement has its shape" "shape '$SC_SHAPE', verify exit $SC_V_RC: $SC_V_OUT"
fi
if [[ "$SC_T_UNKNOWN" == "2 score.statement"*"UNKNOWN yet claims"* && "$SC_T_ONE" == "2 score.statement"* \
    && "$SC_T_TYPE" == "2 score.statement"*"predicateType"* && "$SC_T_DIGEST" == "2 score.statement"*"gitCommit"* ]]; then
    sc_ok "a tampered statement is refused" "UNKNOWN prs claiming 150, a score one point off, another predicateType, a sha256 subject: each score.statement, exit 2"
else
    sc_fail "a tampered statement is refused" "unknown '$SC_T_UNKNOWN'; one '$SC_T_ONE'; type '$SC_T_TYPE'; digest '$SC_T_DIGEST'"
fi

command rm -rf "$SC_TMP" "$SC_ROOT"
