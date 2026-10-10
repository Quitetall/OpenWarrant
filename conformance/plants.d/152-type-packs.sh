# shellcheck shell=bash
# OW-WAR-0148 M18 (decision 26): about ten core types ship in the box, and the
# rest as versioned packs in the same profile format anyone can write.
# On a scratch program (TP), which has no profiles/ and no packs/ of its own:
# - `war plan types` lists the core types built in: release, incident and
#   exit-report beside roadmap, spec, adr, delivery, decision and ticket.
# - `war plan types add ops --dry-run` writes nothing; `war plan types add
#   ops` copies the pack's four profiles into profiles/ and records the pack
#   in docs/types.toml; `war plan types` names each `pack ops 1.0.0`, and
#   `war check` passes `types.well-formed`. The same pack again writes
#   nothing.
# Refused, each writing nothing at all (the tree is byte-identical after):
# - a pack whose profile selects a capability outside the closed set,
#   `profile.capability-unknown`;
# - a pack whose type is named like a type the program already has (the
#   built-in `incident`; an installed `runbook`), `types.collision`;
# - a pack whose document type selects `verification`, `profile.capabilities`;
# - a pack that is not there, `types.pack-unknown`.

echo "== type packs: install, and refuse what the closed set refuses (M18) =="
TP_TMP=$(mktemp -d)
TP_ROOT=$(scratch_corpus TP)
[[ -d "${TP_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
TP_WAR="$REPO_ROOT/${WAR#./}"
tp_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
tp_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$TP_WAR" --root "$TP_ROOT" "$@" </dev/null; }
# tp_tree: every path and content digest under the program, .git aside.
tp_tree() { (cd "$TP_ROOT" && find . -path ./.git -prune -o -type f -print0 | sort -z | xargs -0 sha256sum); }

# --- the core types are in the box ---------------------------------------------------------
tp_war plan types > "$TP_TMP/types0" 2>&1
TP_MISS=""
for t in release incident exit-report roadmap spec adr delivery decision ticket; do
    grep -q "^$t  *[a-z]*  *built in " "$TP_TMP/types0" || TP_MISS="$TP_MISS $t"
done
if [[ -z $TP_MISS && ! -d "$TP_ROOT/profiles" ]]; then
    tp_ok "core types ship in the box" "release, incident, exit-report, roadmap, spec, adr, delivery, decision, ticket: built in"
else
    tp_fail "core types ship in the box" "not listed as built in:$TP_MISS"
fi

# --- a pack installs, once ---------------------------------------------------------------------
tp_tree > "$TP_TMP/before"
tp_war plan types add ops --dry-run > "$TP_TMP/dry" 2>&1; TP_DRC=$?
tp_tree > "$TP_TMP/after-dry"
tp_war plan types add ops > "$TP_TMP/add" 2>&1; TP_ARC=$?
tp_war plan types > "$TP_TMP/types1" 2>&1
tp_war check > "$TP_TMP/check" 2>&1
TP_N=$(grep -c ' pack ops 1.0.0 ' "$TP_TMP/types1")
tp_tree > "$TP_TMP/installed"
tp_war plan types add ops > "$TP_TMP/again" 2>&1; TP_GRC=$?
tp_tree > "$TP_TMP/after-again"
if [[ $TP_DRC -eq 0 && $TP_ARC -eq 0 && $TP_GRC -eq 0 && $TP_N -eq 4 ]] \
    && cmp -s "$TP_TMP/before" "$TP_TMP/after-dry" && cmp -s "$TP_TMP/installed" "$TP_TMP/after-again" \
    && [[ -f "$TP_ROOT/profiles/runbook.toml" && -f "$TP_ROOT/profiles/incident-review.toml" ]] \
    && grep -q '^name = "ops"' "$TP_ROOT/docs/types.toml" \
    && grep -q 'PASS types.well-formed .*1 pack' "$TP_TMP/check" \
    && grep -q 'installed already' "$TP_TMP/again"; then
    tp_ok "a pack installs, once" "--dry-run writes nothing; 4 types from pack ops 1.0.0; again writes nothing"
else
    tp_fail "a pack installs, once" "dry exit $TP_DRC, add exit $TP_ARC, again exit $TP_GRC, $TP_N listed; $(tr '\n' ' ' < "$TP_TMP/add")"
fi

# --- refusals, each writing nothing --------------------------------------------------------------
# tp_pack <dir> <file> <profile body>: a one-profile pack.
tp_pack() {
    mkdir -p "$TP_TMP/$1"
    printf 'schema = "oh.war/pack/v1"\nname = "%s"\nversion = "0.1.0"\nprofiles = ["%s"]\n' "$1" "$2" > "$TP_TMP/$1/pack.toml"
    printf '%s\n' "$3" > "$TP_TMP/$1/$2"
}
tp_pack shiny shiny.toml $'schema = "oh.war/profile/v1"\nname = "shiny"\nform = "document"\ncapabilities = ["structure", "telepathy"]'
tp_pack clash incident.toml $'schema = "oh.war/profile/v1"\nname = "incident"\nform = "document"'
tp_pack twice runbook.toml $'schema = "oh.war/profile/v1"\nname = "runbook"\nform = "document"'
tp_pack proof proof.toml $'schema = "oh.war/profile/v1"\nname = "proof"\nform = "document"\ncapabilities = ["structure", "verification"]'
tp_tree > "$TP_TMP/held"
TP_BAD=""
tp_refused() {
    local rule=$1 pack=$2 out rc
    out=$(tp_war plan types add "$pack" 2>&1); rc=$?
    if [[ $rc -eq 0 ]] || ! grep -q "refused ($rule)" <<<"$out"; then
        TP_BAD="$TP_BAD [$pack: exit $rc, $(head -1 <<<"$out")]"
    fi
}
tp_refused profile.capability-unknown "$TP_TMP/shiny"
tp_refused types.collision "$TP_TMP/clash"
tp_refused types.collision "$TP_TMP/twice"
tp_refused profile.capabilities "$TP_TMP/proof"
tp_refused types.pack-unknown "$TP_TMP/nowhere"
tp_tree > "$TP_TMP/after-refusals"
if [[ -z $TP_BAD ]] && cmp -s "$TP_TMP/held" "$TP_TMP/after-refusals"; then
    tp_ok "a refused pack installs nothing" "unknown capability, two collisions, verification, no pack: refused by rule; tree unchanged"
else
    tp_fail "a refused pack installs nothing" "${TP_BAD:-refused as wanted}; tree $(cmp -s "$TP_TMP/held" "$TP_TMP/after-refusals" && echo unchanged || echo CHANGED)"
fi

command rm -rf "$TP_TMP"
