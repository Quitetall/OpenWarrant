# shellcheck shell=bash
# OW-WAR-0148 M18: ROADMAP.md is the roadmap type's declared projection. On a
# scratch program (RP) with a roadmap record of two phases, the second after
# the first, and the scaffold's adopt Warrant naming the first:
# - `war admin compile` writes docs/roadmap/generated/ROADMAP.md: the phases,
#   each exit, the member Warrant, achievement and what each needs first;
#   its bytes are `war plan render roadmap`'s, and `war check --generated`
#   passes it (`projection.drift`).
# Refused:
# - a hand-edit to ROADMAP.md is `projection.drift`, an error naming the
#   file, and `war admin compile` puts the rendering back;
# - a change to the phases atom without recompiling is drift too: the file
#   is a projection of the record, never a second source.

echo "== ROADMAP.md: the roadmap type's projection (M18) =="
RP_TMP=$(mktemp -d)
RP_ROOT=$(scratch_corpus RP)
[[ -d "${RP_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
RP_WAR="$REPO_ROOT/${WAR#./}"
rp_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rp_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$RP_WAR" --root "$RP_ROOT" "$@" </dev/null; }
RP_FILE="$RP_ROOT/docs/roadmap/generated/ROADMAP.md"

mkdir -p "$RP_ROOT/docs/roadmap/atoms"
cat > "$RP_ROOT/docs/roadmap/roadmap.toml" <<'EOF'
schema = "oh.war/roadmap/v1"
uuid = "01a11589-0000-7151-8000-000000000151"
program = "Roadmap Plant"
prefix = "RP"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
EOF
cat > "$RP_ROOT/docs/roadmap/atoms/20-phases.yaml" <<'EOF'
schema: "oh.war/roadmap-phases/v1"

phases:
  - id: "RP-PHASE-1"
    title: "Adopt the tool"
    outcome: "The program is under war."
    exit: "the adopt Warrant resolves."
    depends_on: []

  - id: "RP-PHASE-2"
    title: "Ship the release"
    outcome: "A release ships."
    exit: "the release Warrant resolves."
    depends_on: ["RP-PHASE-1"]
EOF
RP_ADOPT=$(cd "$RP_ROOT/docs/warrants" && find . -maxdepth 1 -name 'RP-WAR-*' -printf '%f\n' | sort | head -1)
[[ "$RP_ADOPT" =~ ^RP-WAR-[0-9]{4}$ ]] || { printf 'PLANT SETUP FAILED: no adopt Warrant in %s\n' "$RP_ROOT" >&2; exit 9; }

# --- rendered, compiled, drift-checked ------------------------------------------------
rp_war admin compile >/dev/null 2>&1
rp_war admin compile >/dev/null 2>&1
rp_war plan render roadmap > "$RP_TMP/rendered" 2>"$RP_TMP/render.err"
rp_war check --generated > "$RP_TMP/check" 2>&1; RP_RC=$?
RP_MISS=""
for w in '# Roadmap Plant: roadmap' '| RP-PHASE-2 | Ship the release |' '| RP-PHASE-1 |' \
    '- **Exit:** the adopt Warrant resolves.' "- **Warrants:** 1: $RP_ADOPT" '- **Achieved:** ' \
    '## Sources'; do
    [[ -f $RP_FILE ]] && grep -qF -- "$w" "$RP_FILE" || RP_MISS="$RP_MISS [$w]"
done
RP_AFTER=$(grep -F '| RP-PHASE-2 |' "$RP_FILE" 2>/dev/null)
if [[ -z $RP_MISS && $RP_RC -eq 0 && $RP_AFTER == *"| RP-PHASE-1 |"* ]] && cmp -s "$RP_FILE" "$RP_TMP/rendered" \
    && grep -q 'PASS projection.drift .*docs/roadmap/generated/ROADMAP.md matches' "$RP_TMP/check"; then
    rp_ok "ROADMAP.md is compiled" "phases, exits, members, achievement; = war plan render roadmap; check --generated passes"
else
    rp_fail "ROADMAP.md is compiled" "missing:$RP_MISS; check exit $RP_RC; render $(cmp -s "$RP_FILE" "$RP_TMP/rendered" && echo same || echo differs)"
fi

# --- a hand-edit is drift, and compile puts the rendering back -------------------------
cp "$RP_FILE" "$RP_TMP/clean"
printf '\nRP-PHASE-2 is done. (hand-edited)\n' >> "$RP_FILE"
rp_war check --generated > "$RP_TMP/edited" 2>&1; RP_ERC=$?
rp_war admin compile >/dev/null 2>&1
rp_war check --generated > "$RP_TMP/again" 2>&1; RP_ARC=$?
if [[ $RP_ERC -ne 0 && $RP_ARC -eq 0 ]] \
    && grep -q '^ERROR projection.drift .*docs/roadmap/generated/ROADMAP.md' "$RP_TMP/edited" \
    && cmp -s "$RP_FILE" "$RP_TMP/clean"; then
    rp_ok "a hand-edited ROADMAP.md is refused" "projection.drift (exit $RP_ERC); compile restores it, check exits 0"
else
    rp_fail "a hand-edited ROADMAP.md is refused" "edited exit $RP_ERC, after compile exit $RP_ARC; restored: $(cmp -s "$RP_FILE" "$RP_TMP/clean" && echo yes || echo no)"
fi

# --- the record moves, the file does not: drift until recompiled ----------------------
sed -i 's/Ship the release/Ship the first release/' "$RP_ROOT/docs/roadmap/atoms/20-phases.yaml"
rp_war check --generated > "$RP_TMP/moved" 2>&1; RP_MRC=$?
rp_war admin compile >/dev/null 2>&1
if [[ $RP_MRC -ne 0 ]] && grep -q '^ERROR projection.drift .*ROADMAP.md' "$RP_TMP/moved" \
    && grep -qF '| RP-PHASE-2 | Ship the first release |' "$RP_FILE"; then
    rp_ok "ROADMAP.md follows the record" "an atom edit is drift until compile, which renders the new title"
else
    rp_fail "ROADMAP.md follows the record" "exit $RP_MRC after the atom edit"
fi

command rm -rf "$RP_TMP"
