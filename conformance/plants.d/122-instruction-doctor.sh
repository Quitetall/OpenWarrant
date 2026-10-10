# shellcheck shell=bash
# M16 (decision 13): `war doctor` reports the managed block in the root
# AGENTS.md and CLAUDE.md, and changes nothing.
#
# - In a repository `war init` just made, AGENTS.md's block is this war's:
#   doctor.agents-block, and no stale or missing finding.
# - A block stamped by an older war is doctor.agents-block-stale, naming the
#   file and the older version; `war agents-md --block` clears it.
# - A CLAUDE.md with no block is doctor.agents-block-missing.
# Refusals:
# - A block stamped by a NEWER war is not called stale (that is
#   install.version-skew, which doctor also says): `--block` from an older
#   binary would be a downgrade.
# - Doctor writes nothing: the tree is byte-identical before and after.

echo "== doctor flags a stale or missing block (M16) =="
ID_TMP=$(mktemp -d)
ID_WAR="$REPO_ROOT/${WAR#./}"
id_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
id_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
id_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$ID_WAR" --root "$ID_ROOT" "$@"; }
# id_rules <doctor --json output>: the doctor.agents-block* and skew rules,
# each as rule@file, sorted.
id_rules() {
    python3 -c '
import json, sys
v = json.loads(sys.argv[1])
print(" ".join(sorted("%s@%s" % (d["rule"], (d["file"] or "").split(":")[0]) for d in v["diagnostics"]
                      if d["rule"].startswith("doctor.agents-block") or d["rule"] == "install.version-skew")))
' "$1" 2>&1
}
id_snapshot() { find "$ID_ROOT" -path "$ID_ROOT/.git" -prune -o -type f -print0 | sort -z | xargs -0 sha256sum; }

ID_ROOT="$ID_TMP/repo"
mkdir -p "$ID_ROOT"
git -C "$ID_ROOT" init -q .
id_war init >/dev/null 2>&1
ID_VER=$("$ID_WAR" --version 2>/dev/null | awk '{print $2}')

# --- fresh: current ---------------------------------------------------------------
ID_BEFORE=$(id_snapshot)
ID_RULES=$(id_rules "$(id_war doctor --json 2>/dev/null)")
ID_AFTER=$(id_snapshot)
if [[ $ID_RULES == "doctor.agents-block@" && -n $ID_VER && "$ID_BEFORE" == "$ID_AFTER" ]]; then
    id_ok "a fresh init's block is current" "doctor.agents-block for AGENTS.md (war $ID_VER); doctor wrote nothing"
else
    id_fail "a fresh init's block is current" "rules: $ID_RULES; tree moved: $([[ "$ID_BEFORE" == "$ID_AFTER" ]] && echo no || echo yes)"
fi

# --- an older stamp: stale; --block clears it ---------------------------------------
sed -i 's/written by war [^ ]* -->/written by war 0.0.1 -->/' "$ID_ROOT/AGENTS.md"
ID_OUT=$(id_war doctor 2>&1)
ID_RULES=$(id_rules "$(id_war doctor --json 2>/dev/null)")
id_war agents-md --block >/dev/null 2>&1
ID_RULES2=$(id_rules "$(id_war doctor --json 2>/dev/null)")
if [[ $ID_RULES == "doctor.agents-block-stale@AGENTS.md" && $ID_RULES2 == "doctor.agents-block@" ]] \
    && grep -q 'written by war 0.0.1' <<<"$ID_OUT"; then
    id_ok "doctor flags a stale block" "doctor.agents-block-stale names AGENTS.md and war 0.0.1; --block clears it"
else
    id_fail "doctor flags a stale block" "stale: $ID_RULES; after --block: $ID_RULES2"
fi

# --- a newer stamp is skew, not stale -------------------------------------------------
sed -i 's/written by war [^ ]* -->/written by war 99.0.0 -->/' "$ID_ROOT/AGENTS.md"
ID_RULES=$(id_rules "$(id_war doctor --json 2>/dev/null)")
if [[ $ID_RULES == "install.version-skew@AGENTS.md" ]]; then
    id_ok "a newer block is skew, not stale" "install.version-skew only; no doctor.agents-block-stale"
else
    id_fail "a newer block is skew, not stale" "$ID_RULES"
fi
id_war agents-md --block >/dev/null 2>&1

# --- a CLAUDE.md without the block: missing ---------------------------------------
printf '# Mine\n\n## Testing\nmake test\n' > "$ID_ROOT/CLAUDE.md"
ID_RULES=$(id_rules "$(id_war doctor --json 2>/dev/null)")
if [[ $ID_RULES == "doctor.agents-block-missing@CLAUDE.md doctor.agents-block@" ]]; then
    id_ok "doctor flags a missing block" "doctor.agents-block-missing names CLAUDE.md; AGENTS.md current"
else
    id_fail "doctor flags a missing block" "$ID_RULES"
fi

command rm -rf "$ID_TMP"
unset ID_TMP ID_WAR ID_ROOT ID_VER ID_BEFORE ID_AFTER ID_RULES ID_RULES2 ID_OUT
unset -f id_ok id_fail id_war id_rules id_snapshot
