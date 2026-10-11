# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Format 1.0 (docs/format/FORMAT-1.0.md, OW-ADR-0032) through the CLI, on a
# scratch program (FF); nothing in this repository moves:
# - `war new --format 1.0` writes one file, `<alias>.md`; `war check` reads it
#   with PASS format.well-formed and `war model` carries its obligation with
#   its text and a content_digest.
# - an unknown undotted frontmatter key is refused by format.key-unknown at
#   its path:line, exit 2, and the document adds no record; the same key
#   dotted is carried in `ext`, exit 0, with the same findings.
# - `requires` naming an extension war does not understand, and a newer
#   major, read UNKNOWN (never PASS format.well-formed), exit 2, no record.

echo "== Format 1.0: one-file Warrant, unknown names, UNKNOWN (FORMAT-1.0) =="
FF_TMP=$(mktemp -d)
FF_ROOT=$(scratch_corpus FF)
[[ -d "${FF_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
FF_WAR="$REPO_ROOT/${WAR#./}"
ff_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ff_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ff_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$FF_WAR" --root "$FF_ROOT" "$@" </dev/null; }
ff_py() { python3 -c "import json, sys; v = json.load(open(sys.argv[1])); print($1)" "$2" 2>&1; }
FF_RULES='" ".join(sorted(d["severity"] + ":" + d["rule"] + "@" + (d.get("file") or "") for d in v["diagnostics"] if d["rule"].startswith("format.")))'

ff_war new "Format plant: one file" --format 1.0 > "$FF_TMP/new.txt" 2>&1
FF_DOC=$(grep -oE 'docs/warrants/FF-WAR-[0-9]{4}\.md' "$FF_TMP/new.txt" | head -1)
FF_ALIAS=$(basename "${FF_DOC:-x.md}" .md)
ff_war --json check > "$FF_TMP/clean.json" 2>/dev/null; FF_CLEAN_RC=$?
ff_war --json model > "$FF_TMP/model.json" 2>/dev/null
FF_CLEAN=$(ff_py "$FF_RULES" "$FF_TMP/clean.json")
FF_OBL=$(ff_py "[(r['title'], r['fields']['refusal'][:8], r['content_digest'][:7]) for r in v['result']['records'] if r['id'] == '$FF_ALIAS/OBL-001']" "$FF_TMP/model.json")
if [[ -n "$FF_DOC" && -f "$FF_ROOT/$FF_DOC" && $FF_CLEAN_RC -eq 0 && "$FF_CLEAN" == "pass:format.well-formed@" \
    && "$FF_OBL" == "[('what is checked', 'an input', 'sha256:')]" ]]; then
    ff_ok "war new --format 1.0 reads" "$FF_DOC: PASS format.well-formed; the model carries OBL-001's text and content_digest"
else
    ff_fail "war new --format 1.0 reads" "doc '$FF_DOC' exit $FF_CLEAN_RC; findings '$FF_CLEAN'; obligation '$FF_OBL'"
fi

# --- an unknown undotted key: refused; dotted: carried ----------------------------------
cp "$FF_ROOT/$FF_DOC" "$FF_TMP/doc.md"
ff_with() { python3 - "$FF_TMP/doc.md" "$FF_ROOT/$FF_DOC" "$1" "$2" <<'PY'
import sys
src, dst, insert, swap = sys.argv[1:5]
t = open(src).read()
t = t.replace("assurance_level: basic\n", "assurance_level: basic\n" + insert.replace("\\n", "\n"), 1)
if swap:
    t = t.replace("openwarrant/1.0", swap, 1)
open(dst, "w").write(t)
PY
}
ff_with 'sumary: a typo\n' ''
ff_war --json check > "$FF_TMP/typo.json" 2>/dev/null; FF_TYPO_RC=$?
ff_war --json model > "$FF_TMP/typo-model.json" 2>/dev/null
FF_TYPO=$(ff_py "$FF_RULES" "$FF_TMP/typo.json")
FF_TYPO_N=$(ff_py "len([r for r in v['result']['records'] if r['id'].startswith('$FF_ALIAS')])" "$FF_TMP/typo-model.json")
ff_with 'x.note: carried\n' ''
ff_war --json check > "$FF_TMP/dotted.json" 2>/dev/null; FF_DOT_RC=$?
ff_war --json model > "$FF_TMP/dotted-model.json" 2>/dev/null
FF_DOT=$(ff_py "$FF_RULES" "$FF_TMP/dotted.json")
FF_EXT=$(ff_py "[r['ext'] for r in v['result']['records'] if r['id'] == '$FF_ALIAS']" "$FF_TMP/dotted-model.json")
if [[ $FF_TYPO_RC -eq 2 && "$FF_TYPO" == "error:format.key-unknown@$FF_DOC:8" && "$FF_TYPO_N" == "0" ]]; then
    ff_ok "an unknown key is refused" "ERROR format.key-unknown at $FF_DOC:8, exit 2, no record"
else
    ff_fail "an unknown key is refused" "exit $FF_TYPO_RC; findings '$FF_TYPO'; records '$FF_TYPO_N'"
fi
if [[ $FF_DOT_RC -eq 0 && "$FF_DOT" == "$FF_CLEAN" && "$FF_EXT" == "[{'x.note': 'carried'}]" ]]; then
    ff_ok "a dotted key is carried, inert" "ext {x.note}; exit 0; the same format findings as without it"
else
    ff_fail "a dotted key is carried, inert" "exit $FF_DOT_RC; findings '$FF_DOT'; ext '$FF_EXT'"
fi

# --- not understood: UNKNOWN, never PASS --------------------------------------------------
for FF_CASE in "requires-unknown|requires:\n  - com.example.risk\n|" "newer-major||openwarrant/2.0"; do
    IFS='|' read -r FF_RULE FF_INS FF_SWAP <<<"$FF_CASE"
    ff_with "$FF_INS" "$FF_SWAP"
    ff_war --json check > "$FF_TMP/u.json" 2>/dev/null; FF_U_RC=$?
    ff_war --json model > "$FF_TMP/u-model.json" 2>/dev/null
    FF_U=$(ff_py "$FF_RULES" "$FF_TMP/u.json")
    FF_U_N=$(ff_py "len([r for r in v['result']['records'] if r['id'].startswith('$FF_ALIAS')])" "$FF_TMP/u-model.json")
    if [[ $FF_U_RC -eq 2 && "$FF_U" == "unknown:format.$FF_RULE@$FF_DOC:"* && "$FF_U" != *"pass:"* && "$FF_U_N" == "0" ]]; then
        ff_ok "$FF_RULE reads UNKNOWN" "UNKNOWN format.$FF_RULE, exit 2, no PASS, no record"
    else
        ff_fail "$FF_RULE reads UNKNOWN" "exit $FF_U_RC; findings '$FF_U'; records '$FF_U_N'"
    fi
done

command rm -rf "$FF_TMP" "$FF_ROOT"
unset FF_TMP FF_ROOT FF_WAR FF_DOC FF_ALIAS FF_RULES FF_CLEAN FF_CLEAN_RC FF_OBL FF_TYPO FF_TYPO_RC FF_TYPO_N \
    FF_DOT FF_DOT_RC FF_EXT FF_CASE FF_RULE FF_INS FF_SWAP FF_U FF_U_RC FF_U_N
