# shellcheck shell=bash
# OW-WAR-0148 M18 (decision 27): untyped documents count. On a scratch
# program (UD):
# - An untyped document lowers coverage: a new docs/notes.md adds one to
#   `document_coverage.total` and `untyped` in `war status --json`, and is a
#   record `doc:docs/notes.md` of type `document`, state `untyped`, in
#   `war plan model`. A file under a generated/ directory is not indexed.
# - Adopting it raises coverage: `war plan type docs/notes.md release` adds
#   one to `typed` and takes one from `untyped`; the model then carries it as
#   a `release`, state `adopted`; docs/types.toml maps its path to its type.
# Refused, each by rule and writing nothing (docs/types.toml and the
# document's bytes unchanged, coverage unchanged):
# - a file that does not exist, `types.file-unknown`;
# - a type the program does not have, `types.type-unknown`;
# - a store's type (`spec`), `types.type-not-adoptable`;
# - a file a type already reads (the SAS document), `types.file-governed`.
# Through all of it, docs/notes.md keeps its bytes.

echo "== untyped documents count, adoption raises coverage (M18) =="
UD_TMP=$(mktemp -d)
UD_ROOT=$(scratch_corpus UD)
[[ -d "${UD_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
UD_WAR="$REPO_ROOT/${WAR#./}"
ud_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ud_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ud_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$UD_WAR" --root "$UD_ROOT" "$@" </dev/null; }
# ud_cov: "typed untyped total" from `war status --json`.
ud_cov() {
    ud_war --json status 2>/dev/null | python3 -c 'import json,sys; c=json.load(sys.stdin)["result"]["document_coverage"]; print(c["typed"], c["untyped"], c["total"])' 2>&1
}
# ud_doc <id>: "<type> <state value> <provenance>" of a model record, or nothing.
ud_doc() {
    ud_war --json plan model > "$UD_TMP/model.json" 2>/dev/null
    python3 - "$UD_TMP/model.json" "$1" <<'PY' 2>&1
import json, sys
m = json.load(open(sys.argv[1]))["result"]
r = next((r for r in m["records"] if r["id"] == sys.argv[2]), None)
s = next((s for s in m["states"] if s["record"] == sys.argv[2] and s["kind"] == "document"), None)
if r and s:
    print(r["type"], s["value"], s["provenance"])
PY
}
ud_sum() { sha256sum "$UD_ROOT/docs/notes.md" 2>/dev/null | cut -d' ' -f1; }

# --- an untyped document lowers coverage ----------------------------------------------------
read -r UD_T0 UD_U0 UD_N0 <<<"$(ud_cov)"
printf '# Notes\n\nWhat we decided at the offsite, not yet anyone'"'"'s record.\n' > "$UD_ROOT/docs/notes.md"
mkdir -p "$UD_ROOT/docs/sas/generated"
printf '# A projection\n' > "$UD_ROOT/docs/sas/generated/NOT_INDEXED.md"
UD_SUM=$(ud_sum)
read -r UD_T1 UD_U1 UD_N1 <<<"$(ud_cov)"
UD_REC1=$(ud_doc doc:docs/notes.md)
UD_GEN=$(ud_doc doc:docs/sas/generated/NOT_INDEXED.md)
if [[ -n $UD_N0 && $UD_T1 -eq $UD_T0 && $UD_U1 -eq $((UD_U0 + 1)) && $UD_N1 -eq $((UD_N0 + 1)) \
    && $UD_REC1 == "document untyped computed" && -z $UD_GEN ]]; then
    ud_ok "an untyped document lowers coverage" "typed/total $UD_T0/$UD_N0 -> $UD_T1/$UD_N1; doc:docs/notes.md untyped; generated/ not indexed"
else
    ud_fail "an untyped document lowers coverage" "before $UD_T0 $UD_U0 $UD_N0, after $UD_T1 $UD_U1 $UD_N1; record '$UD_REC1'; generated '$UD_GEN'"
fi

# --- refusals write nothing --------------------------------------------------------------------
UD_BAD=""
ud_refused() {
    local rule=$1 out rc; shift
    out=$(ud_war plan type "$@" 2>&1); rc=$?
    if [[ $rc -eq 0 ]] || ! grep -q "refused ($rule)" <<<"$out"; then
        UD_BAD="$UD_BAD [$*: exit $rc, $(head -1 <<<"$out")]"
    fi
}
UD_SASDOC=$(cd "$UD_ROOT" && find docs/sas -maxdepth 1 -name '*.md' | head -1)
ud_refused types.file-unknown docs/no-such-notes.md release
ud_refused types.type-unknown docs/notes.md no-such-type
ud_refused types.type-not-adoptable docs/notes.md spec
ud_refused types.file-governed "$UD_SASDOC" release
read -r UD_T2 UD_U2 UD_N2 <<<"$(ud_cov)"
if [[ -z $UD_BAD && -n $UD_SASDOC && ! -e "$UD_ROOT/docs/types.toml" && $(ud_sum) == "$UD_SUM" \
    && "$UD_T2 $UD_U2 $UD_N2" == "$UD_T1 $UD_U1 $UD_N1" ]]; then
    ud_ok "a refused adoption writes nothing" "no file, no type, a store's type, a governed file: refused by rule; coverage and bytes unchanged"
else
    ud_fail "a refused adoption writes nothing" "${UD_BAD:-refused as wanted}; types.toml $([[ -e "$UD_ROOT/docs/types.toml" ]] && echo WRITTEN || echo absent); coverage $UD_T2 $UD_U2 $UD_N2"
fi

# --- adopting raises coverage, and the document keeps its bytes ---------------------------------
ud_war plan type docs/notes.md release > "$UD_TMP/adopt" 2>&1; UD_ARC=$?
read -r UD_T3 UD_U3 UD_N3 <<<"$(ud_cov)"
UD_REC3=$(ud_doc doc:docs/notes.md)
if [[ $UD_ARC -eq 0 && $UD_T3 -eq $((UD_T1 + 1)) && $UD_U3 -eq $((UD_U1 - 1)) && $UD_N3 -eq $UD_N1 \
    && $UD_REC3 == "release adopted authored" && $(ud_sum) == "$UD_SUM" ]] \
    && grep -q '^path = "docs/notes.md"' "$UD_ROOT/docs/types.toml" \
    && grep -q '^type = "release"' "$UD_ROOT/docs/types.toml"; then
    ud_ok "adopting a document raises coverage" "typed/total $UD_T1/$UD_N1 -> $UD_T3/$UD_N3; a release, adopted; its bytes unchanged"
else
    ud_fail "adopting a document raises coverage" "exit $UD_ARC; $UD_T3 $UD_U3 $UD_N3; record '$UD_REC3'; bytes $([[ $(ud_sum) == "$UD_SUM" ]] && echo same || echo CHANGED)"
fi

command rm -rf "$UD_TMP"
