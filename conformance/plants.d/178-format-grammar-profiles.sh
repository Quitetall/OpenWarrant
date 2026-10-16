# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Format 1.0 draft 3's grammar and profile data through the CLI
# (docs/format/FORMAT-1.0.md sections 3.4, 4.5, 7.5), on a scratch program
# (FG); nothing in this repository moves. Each claim beside the refusal it
# was seen to make:
# - an ASCII record heading (`## REQ-1 | requirement -- t`) reads as the
#   Unicode one: same id, type, title and content_digest, PASS
#   format.well-formed; a heading mixing the spellings is refused
#   format.heading at its path:line, exit 2.
# - SPEC.md and a part `spec/<area>.md` read as records of type `spec`; a
#   SPEC.md without `version` is refused format.field-missing at SPEC.md:1,
#   and a second root (profile spec, not SPEC.md, no part_of) is refused
#   format.document-duplicate.
# - a declared field's shape: `priority: must` reads, `priority: urgent` is
#   refused format.field-shape at its line.
# - a profile file: a valid `profiles/runbook.toml` governs its document;
#   one with an unknown key is refused profile.invalid at its path:1 and its
#   document reads UNKNOWN format.profile-unknown, never PASS.

echo "== Format 1.0 draft 3: ASCII headings, specifications, profile data (FORMAT-1.0) =="
FG_TMP=$(mktemp -d)
FG_ROOT=$(scratch_corpus FG)
[[ -d "${FG_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
FG_WAR="$REPO_ROOT/${WAR#./}"
fg_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fg_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
fg_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$FG_WAR" --root "$FG_ROOT" "$@" </dev/null; }
fg_py() { python3 -c "import json, sys; v = json.load(open(sys.argv[1])); print($1)" "$2" 2>&1; }
FG_RULES='" ".join(sorted(d["severity"] + ":" + d["rule"] + "@" + (d.get("file") or "") for d in v["diagnostics"] if d["rule"].split(".")[0] in ("format", "profile")))'
# fg_write <path> <text>: the text with \n expanded, {DOT} as U+00B7 MIDDLE DOT and
# {DASH} as U+2014 EM DASH (the plant's own bytes stay ASCII).
fg_write() { mkdir -p "$(dirname "$FG_ROOT/$1")"; python3 -c 'import sys; open(sys.argv[1], "w", encoding="utf-8").write(sys.argv[2].replace("\\n", "\n").replace("{DOT}", "\u00b7").replace("{DASH}", "\u2014"))' "$FG_ROOT/$1" "$2"; }
fg_check() { fg_war --json check > "$FG_TMP/$1.json" 2>/dev/null; echo $?; }
fg_digest() { fg_war --json model > "$FG_TMP/m.json" 2>/dev/null; fg_py "[(r['type'], r['title'], r['content_digest']) for r in v['result']['records'] if r['id'] == '$1']" "$FG_TMP/m.json"; }
fg_clean() { command rm -rf "$FG_ROOT/SPEC.md" "$FG_ROOT/spec" "$FG_ROOT/docs/records/fg" "$FG_ROOT/profiles/runbook.toml"; }

FG_HEAD='---\nformat: openwarrant/1.0\nprofile: prd\n---\n# Notes\n\n'
FG_BODY='statement: A reset link works once.\n'

# --- an ASCII heading is the Unicode record; a mixed one is refused ----------------------
fg_write docs/records/fg/a.md "${FG_HEAD}## REQ-fg1 {DOT} requirement {DASH} a link works once\n${FG_BODY}"
FG_UNI_RC=$(fg_check uni); FG_UNI=$(fg_digest REQ-fg1)
fg_write docs/records/fg/a.md "${FG_HEAD}## REQ-fg1 | requirement -- a link works once\n${FG_BODY}"
FG_ASC_RC=$(fg_check asc); FG_ASC=$(fg_digest REQ-fg1); FG_ASC_F=$(fg_py "$FG_RULES" "$FG_TMP/asc.json")
fg_write docs/records/fg/a.md "${FG_HEAD}## REQ-fg1 | requirement {DASH} a link works once\n${FG_BODY}"
FG_MIX_RC=$(fg_check mix); FG_MIX=$(fg_py "$FG_RULES" "$FG_TMP/mix.json")
if [[ $FG_UNI_RC -eq 0 && $FG_ASC_RC -eq 0 && "$FG_ASC" == "$FG_UNI" && "$FG_ASC" == "[('requirement', 'a link works once', 'sha256:"* \
    && "$FG_ASC_F" == "pass:format.well-formed@" ]]; then
    fg_ok "an ASCII heading is the record" "## REQ-fg1 | requirement -- ...: same type, title and content_digest; PASS"
else
    fg_fail "an ASCII heading is the record" "exit $FG_UNI_RC/$FG_ASC_RC; unicode '$FG_UNI' ascii '$FG_ASC' findings '$FG_ASC_F'"
fi
if [[ $FG_MIX_RC -eq 2 && "$FG_MIX" == "error:format.heading@docs/records/fg/a.md:7" ]]; then
    fg_ok "a mixed heading is refused" "ERROR format.heading at docs/records/fg/a.md:7, exit 2"
else
    fg_fail "a mixed heading is refused" "exit $FG_MIX_RC; findings '$FG_MIX'"
fi
fg_clean

# --- the specification -------------------------------------------------------------------
FG_SPEC='---\nformat: openwarrant/1.0\nprofile: spec\nversion: 0.3.0\nsummary: Password reset.\n---\n# Account service\n\n## Purpose\nAccess without support.\n\n## Scope\nIn: email. Out: SMS.\n\n## REQ-reset-1 {DOT} requirement {DASH} a reset link works once\nstatement: A reset link MUST be accepted at most once.\npriority: must\n'
fg_write SPEC.md "$FG_SPEC"
fg_write spec/email.md '---\nformat: openwarrant/1.0\nprofile: spec\npart_of: doc:SPEC.md\n---\n# Email\n\n## REQ-email-1 {DOT} requirement {DASH} the email names the expiry\nstatement: The reset email MUST state when the link expires.\n'
FG_SP_RC=$(fg_check spec); FG_SP=$(fg_py "$FG_RULES" "$FG_TMP/spec.json")
fg_war --json model > "$FG_TMP/sm.json" 2>/dev/null
FG_SP_M=$(fg_py "sorted((r['id'], r['type']) for r in v['result']['records'] if r['source'] in ('SPEC.md', 'spec/email.md') and r['type'] in ('spec', 'requirement'))" "$FG_TMP/sm.json")
if [[ $FG_SP_RC -eq 0 && "$FG_SP" == "pass:format.well-formed@" \
    && "$FG_SP_M" == "[('REQ-email-1', 'requirement'), ('REQ-reset-1', 'requirement'), ('doc:SPEC.md', 'spec'), ('doc:spec/email.md', 'spec')]" ]]; then
    fg_ok "a specification reads" "doc:SPEC.md and doc:spec/email.md of type spec; their requirements global; PASS"
else
    fg_fail "a specification reads" "exit $FG_SP_RC; findings '$FG_SP'; records '$FG_SP_M'"
fi
fg_write SPEC.md "${FG_SPEC/version: 0.3.0\\n/}"
FG_NV_RC=$(fg_check nover); FG_NV=$(fg_py "$FG_RULES" "$FG_TMP/nover.json")
fg_write SPEC.md "$FG_SPEC"
fg_write spec/other.md '---\nformat: openwarrant/1.0\nprofile: spec\n---\n# Other\n'
FG_DUP_RC=$(fg_check dup); FG_DUP=$(fg_py "$FG_RULES" "$FG_TMP/dup.json")
command rm -f "$FG_ROOT/spec/other.md"
if [[ $FG_NV_RC -eq 2 && "$FG_NV" == "error:format.field-missing@SPEC.md:1 warn:format.target-unknown@spec/email.md:4" \
    && $FG_DUP_RC -eq 2 && "$FG_DUP" == "error:format.document-duplicate@spec/other.md:1" ]]; then
    fg_ok "a specification's refusals" "no version: format.field-missing SPEC.md:1; a second root: format.document-duplicate"
else
    fg_fail "a specification's refusals" "exit $FG_NV_RC '$FG_NV'; exit $FG_DUP_RC '$FG_DUP'"
fi

# --- a declared field's shape --------------------------------------------------------------
fg_write SPEC.md "${FG_SPEC/priority: must/priority: urgent}"
FG_SH_RC=$(fg_check shape); FG_SH=$(fg_py "$FG_RULES" "$FG_TMP/shape.json")
if [[ $FG_SP_RC -eq 0 && $FG_SH_RC -eq 2 && "$FG_SH" == "error:format.field-shape@SPEC.md:17 warn:format.target-unknown@spec/email.md:4" ]]; then
    fg_ok "a field has its shape" "priority: must reads; priority: urgent is format.field-shape at SPEC.md:17"
else
    fg_fail "a field has its shape" "exit $FG_SH_RC; findings '$FG_SH'"
fi
fg_clean

# --- profile data: a profile file governs; a bad one is profile.invalid --------------------
FG_PROFILE='schema = "oh.war/profile/v1"\nname = "runbook"\nform = "document"\n[sections]\nrequired = ["Steps"]\n'
fg_write profiles/runbook.toml "$FG_PROFILE"
fg_write docs/records/fg/r.md '---\nformat: openwarrant/1.0\nprofile: runbook\n---\n# Restart\n\n## Steps\nRestart it.\n'
FG_PV_RC=$(fg_check pv); FG_PV=$(fg_py "$FG_RULES" "$FG_TMP/pv.json")
fg_write docs/records/fg/r.md '---\nformat: openwarrant/1.0\nprofile: runbook\n---\n# Restart\n\n## Notes\nRestart it.\n'
FG_PS_RC=$(fg_check ps); FG_PS=$(fg_py "$FG_RULES" "$FG_TMP/ps.json")
fg_write profiles/runbook.toml "${FG_PROFILE}colour = \"red\"\n"
FG_PI_RC=$(fg_check pi); FG_PI=$(fg_py "$FG_RULES" "$FG_TMP/pi.json")
if [[ $FG_PV_RC -eq 0 && "$FG_PV" == "pass:format.well-formed@" \
    && $FG_PS_RC -eq 2 && "$FG_PS" == "error:format.section-missing@docs/records/fg/r.md:1" ]]; then
    fg_ok "a profile file governs" "profiles/runbook.toml requires Steps: present PASS, absent format.section-missing"
else
    fg_fail "a profile file governs" "exit $FG_PV_RC '$FG_PV'; exit $FG_PS_RC '$FG_PS'"
fi
if [[ $FG_PI_RC -eq 2 && "$FG_PI" == "error:profile.invalid@profiles/runbook.toml:1 unknown:format.profile-unknown@docs/records/fg/r.md:1" ]]; then
    fg_ok "a bad profile is profile.invalid" "unknown key: profile.invalid; its document UNKNOWN format.profile-unknown, no PASS"
else
    fg_fail "a bad profile is profile.invalid" "exit $FG_PI_RC; findings '$FG_PI'"
fi
fg_clean

command rm -rf "$FG_TMP" "$FG_ROOT"
unset FG_TMP FG_ROOT FG_WAR FG_RULES FG_HEAD FG_BODY FG_UNI FG_UNI_RC FG_ASC FG_ASC_RC FG_ASC_F FG_MIX FG_MIX_RC \
    FG_SPEC FG_SP FG_SP_RC FG_SP_M FG_NV FG_NV_RC FG_DUP FG_DUP_RC FG_SH FG_SH_RC FG_PROFILE FG_PV FG_PV_RC \
    FG_PS FG_PS_RC FG_PI FG_PI_RC
