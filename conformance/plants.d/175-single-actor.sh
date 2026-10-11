# shellcheck shell=bash
# Issue #132 item 3: `[independence] single_actor = true`, a repository that
# declares one actor holds every role (a solo owner working through agents).
#
# On a scratch program (SA) with one directory Warrant (basic assurance):
# - with all nine §46.1 dimensions `false` and no declaration, `war check`
#   WARNs independence.insufficient (the baseline the declaration answers).
# - with `single_actor = true` the WARN is gone and the fact is said once, as
#   a NOT CHECKED note naming the declaration; `war status --json` reports
#   independence as declared single_actor, independent false. Refused: any
#   independence.* PASS (the declaration never reads as a minimum met), and
#   any movement of the compliance score: `war admin score --json` is the same
#   bytes with and without the declaration, so it earns nothing.
# - `single_actor = true` beside a dimension written `true` is refused by name:
#   ERROR independence.single-actor-contradicted naming the dimension, exit 2,
#   and the independence.insufficient WARN is NOT suppressed.

echo "== single_actor: the gap said once, never read as verified (issue #132) =="
SA_TMP=$(mktemp -d)
SA_ROOT=$(scratch_corpus SA)
[[ -d "${SA_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
SA_WAR="$REPO_ROOT/${WAR#./}"
sa_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sa_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sa_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$SA_WAR" --root "$SA_ROOT" "$@" </dev/null; }
sa_git() { git -C "$SA_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false "$@"; }
sa_py() { python3 -c "import json, sys; v = json.load(open(sys.argv[1])); print($1)" "$2" 2>&1; }
# sa_config <extra lines for [independence]>: the committed config plus the table.
sa_config() {
    cp "$SA_TMP/base.toml" "$SA_ROOT/openwarrant.toml"
    printf '\n[independence]\n%s\n' "$1" >> "$SA_ROOT/openwarrant.toml"
}
SA_NONE='performer_transcript_blind = false
performer_rationale_blind = false
separate_writable_workspace = false
cannot_modify_subject_artifacts = false
cannot_modify_gate_definition = false
cannot_modify_gate_fixtures = false
separate_context_compilation = false
distinct_model_required = false
distinct_human_required = false'

sa_war new "A basic Warrant to count" >/dev/null 2>&1
sa_war compile >/dev/null 2>&1
sa_git add -A >/dev/null 2>&1
sa_git commit -qm "A Warrant" >/dev/null 2>&1
cp "$SA_ROOT/openwarrant.toml" "$SA_TMP/base.toml"
# The independence findings of a check: rules and severities, then notes.
SA_FIND='" ".join(sorted(d["severity"] + ":" + d["rule"] for d in v["diagnostics"] if d["rule"].startswith("independence."))) + " | " + " ".join(n for n in v["notes"] if n.startswith("independence"))'

# --- the baseline: nine false, undeclared -------------------------------------------------
sa_config "$SA_NONE"
sa_war --json check > "$SA_TMP/none.json" 2>/dev/null
sa_war --json admin score > "$SA_TMP/score-none.json" 2>/dev/null
SA_NONE_F=$(sa_py "$SA_FIND" "$SA_TMP/none.json")

# --- declared: the WARN goes, the note comes, nothing passes, the score holds ------------
sa_config "$SA_NONE
single_actor = true"
sa_war --json check > "$SA_TMP/single.json" 2>/dev/null; SA_SINGLE_RC=$?
sa_war --json admin score > "$SA_TMP/score-single.json" 2>/dev/null
sa_war --json status > "$SA_TMP/status.json" 2>/dev/null
SA_SINGLE_F=$(sa_py "$SA_FIND" "$SA_TMP/single.json")
SA_STATUS=$(sa_py 'json.dumps(v["result"].get("independence"), sort_keys=True)' "$SA_TMP/status.json")
SA_S0=$(sa_py 'json.dumps(v["result"], sort_keys=True)' "$SA_TMP/score-none.json")
SA_S1=$(sa_py 'json.dumps(v["result"], sort_keys=True)' "$SA_TMP/score-single.json")
SA_VERIFIED=$(sa_py '[d["points"] for d in v["result"]["dimensions"] if d["id"] == "verified"][0]' "$SA_TMP/score-single.json")
# single_actor alone (every dimension unwritten) is the same declaration.
sa_config "single_actor = true"
sa_war --json check > "$SA_TMP/alone.json" 2>/dev/null
SA_ALONE_F=$(sa_py "$SA_FIND" "$SA_TMP/alone.json")

if [[ "$SA_NONE_F" == "warn:independence.insufficient | " \
    && $SA_SINGLE_RC -eq 0 && "$SA_SINGLE_F" == " | independence — not evaluated per Warrant"*"single_actor = true"*"unmet for "[0-9]*" basic Warrant(s)"* \
    && "$SA_ALONE_F" == "$SA_SINGLE_F" \
    && "$SA_STATUS" == '{"declared": "single_actor", "independent": false}' ]]; then
    sa_ok "single_actor says the gap once" "undeclared: WARN insufficient; declared: no independence.* line, one note; status: declared, independent false"
else
    sa_fail "single_actor says the gap once" "none '$SA_NONE_F'; single (exit $SA_SINGLE_RC) '$SA_SINGLE_F'; alone '$SA_ALONE_F'; status '$SA_STATUS'"
fi
if [[ -n "$SA_S0" && "$SA_S0" == "$SA_S1" && "$SA_SINGLE_F" != *"pass:"* && "$SA_VERIFIED" == "0" ]]; then
    sa_ok "single_actor earns nothing" "no independence.* PASS; the score is the same bytes declared or not; verified earns 0"
else
    sa_fail "single_actor earns nothing" "score equal: $([[ "$SA_S0" == "$SA_S1" ]] && echo y || echo n); findings '$SA_SINGLE_F'; verified '$SA_VERIFIED'"
fi

# --- contradicted: refused by name, the WARN stays ---------------------------------------
sa_config "${SA_NONE/performer_transcript_blind = false/performer_transcript_blind = true}
single_actor = true"
sa_war --json check > "$SA_TMP/contra.json" 2>/dev/null; SA_CONTRA_RC=$?
SA_CONTRA_F=$(sa_py "$SA_FIND" "$SA_TMP/contra.json")
SA_CONTRA_M=$(sa_py '" ".join(d["message"] for d in v["diagnostics"] if d["rule"] == "independence.single-actor-contradicted")' "$SA_TMP/contra.json")
if [[ $SA_CONTRA_RC -eq 2 && "$SA_CONTRA_F" == "error:independence.single-actor-contradicted warn:independence.insufficient | " \
    && "$SA_CONTRA_M" == *"performer_transcript_blind"* ]]; then
    sa_ok "a contradicted single_actor is refused" "ERROR independence.single-actor-contradicted names performer_transcript_blind, exit 2; the WARN is not suppressed"
else
    sa_fail "a contradicted single_actor is refused" "exit $SA_CONTRA_RC; findings '$SA_CONTRA_F'; message '$SA_CONTRA_M'"
fi

command rm -rf "$SA_TMP" "$SA_ROOT"
