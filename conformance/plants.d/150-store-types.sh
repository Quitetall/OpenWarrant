# shellcheck shell=bash
# OW-WAR-0148 M18 (decision 25): the roadmap, the specification and ADRs are
# ordinary types (`roadmap`, `spec`, `adr`), each read as one store's
# encoding, unchanged, with its `roadmap.*`, `sas.*` and `adr.*` rules gated
# on the capabilities the type selects.
#
# On a scratch program (ST) with a roadmap record (a proposed revision), a
# SAS with a proposed revision, and two ADRs, one superseding the other:
# - The differential: `war plan roadmap --json`, `war sign sas status
#   --json`, `war check --json` and the store records of `war plan model`
#   are byte-identical read through the built-in types and through this
#   repository's profiles/roadmap.toml, spec.toml and adr.toml.
#   Refused: the same comparison tells a doctored byte apart. One byte
#   planted in the SAS document is `sas.digest-drift`; in the roadmap's
#   phases atom, the atoms are no longer the proposed revision; in an ADR,
#   its model revision moves. Restored, the outputs are the clean ones.
# - The rules are the type's: with `spec` narrowed to structure and links,
#   the doctored SAS byte is no `sas.digest-drift` (nothing holds the
#   document to a revision); with acceptance selected, it is. A store type
#   selecting `verification` is refused when the program opens,
#   `profile.capabilities`.
# - The model carries the stores' records with their relations: ST-ROADMAP
#   and its phases (part_of, depends_on), ST-SAS and its sections (part_of),
#   and the ADRs with the one `supersedes` and its `superseded` state.

echo "== store types: roadmap, spec and adr read unchanged (M18) =="
ST_TMP=$(mktemp -d)
ST_ROOT=$(scratch_corpus ST)
[[ -d "${ST_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
ST_WAR="$REPO_ROOT/${WAR#./}"
st_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
st_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
st_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$ST_WAR" --root "$ST_ROOT" "$@" </dev/null; }

# A roadmap record whose phases include the scaffold's own ST-PHASE-1.
mkdir -p "$ST_ROOT/docs/roadmap/atoms" "$ST_ROOT/docs/adr/atoms"
cat > "$ST_ROOT/docs/roadmap/roadmap.toml" <<'EOF'
schema = "oh.war/roadmap/v1"
uuid = "01a11589-0000-7150-8000-000000000150"
program = "Store Plant"
prefix = "ST"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
EOF
printf '# Intent\n\nA plant program, built in two phases.\n' > "$ST_ROOT/docs/roadmap/atoms/10-intent.md"
cat > "$ST_ROOT/docs/roadmap/atoms/20-phases.yaml" <<'EOF'
schema: "oh.war/roadmap-phases/v1"

phases:
  - id: "ST-PHASE-1"
    title: "Adopt"
    outcome: "The program is under war."
    exit: "every Warrant names a phase."
    depends_on: []

  - id: "ST-PHASE-2"
    title: "Grow"
    outcome: "More work."
    exit: "the second phase's exit Warrant resolves."
    depends_on: ["ST-PHASE-1"]
EOF
st_adr() {
    printf -- '---\nschema: oh.war/atom/v1\nadr_uuid: %s\nlocal_alias: %s\nrole: adr\njurisdiction: bound\nstatus: %s\n%s---\n\n# ADR %s: %s\n\n## Status\n\n%s.\n' \
        "$1" "$2" "$3" "$4" "$2" "$5" "$3" > "$ST_ROOT/docs/adr/atoms/$2-plant.md"
}
st_adr 01a11589-0000-7150-8000-0000000000a1 ST-ADR-0001 superseded "" "The first decision"
st_adr 01a11589-0000-7150-8000-0000000000a2 ST-ADR-0002 accepted \
    $'supersedes:\n  - "adr://01a11589-0000-7150-8000-0000000000a1"\n' "The second decision"
st_war plan roadmap propose --note "plant" >/dev/null 2>&1
st_war sign sas propose 0.1.0 >/dev/null 2>&1
git -C "$ST_ROOT" add -A >/dev/null 2>&1
git -C "$ST_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "stores" >/dev/null 2>&1
ST_SAS=$(cd "$ST_ROOT/docs/sas" && find . -maxdepth 1 -name '*.md' -printf '%f\n' | head -1)
[[ -n $ST_SAS && -f "$ST_ROOT/docs/roadmap/revisions/1.toml" && -f "$ST_ROOT/docs/sas/revisions/0.1.0.toml" ]] \
    || { printf 'PLANT SETUP FAILED: no SAS document or no proposed revisions in %s\n' "$ST_ROOT" >&2; exit 9; }

# st_take <dir>: the four readings, each to a file in <dir>.
st_take() {
    mkdir -p "$1"
    st_war --json plan roadmap > "$1/roadmap" 2>&1
    st_war --json sign sas status > "$1/sas" 2>&1
    st_war --json check > "$1/check" 2>&1
    st_war --json plan model > "$ST_TMP/model.json" 2>/dev/null
    python3 - "$ST_TMP/model.json" > "$1/model" 2>&1 <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))["result"]
types = {"roadmap", "phase", "spec", "section", "requirement", "adr"}
ids = {r["id"] for r in m["records"] if r["type"] in types}
for r in m["records"]:
    if r["id"] in ids:
        print("R", r["id"], r["type"], r["source"], r["revision"], r.get("governed_by", ""))
for r in m["relations"]:
    if r["from"] in ids or r["to"] in ids:
        print("L", r["from"], r["kind"], r["to"])
for s in m["states"]:
    if s["record"] in ids:
        print("S", s["record"], s["kind"], s["value"], s["provenance"])
PY
}
# st_same <a> <b>: every reading identical.
st_same() {
    local f
    for f in roadmap sas check model; do cmp -s "$1/$f" "$2/$f" || return 1; done
}
st_profiles() { mkdir -p "$ST_ROOT/profiles"; cp "$REPO_ROOT/profiles/roadmap.toml" "$REPO_ROOT/profiles/spec.toml" "$REPO_ROOT/profiles/adr.toml" "$ST_ROOT/profiles/"; }

# --- the differential: built in, then declared ------------------------------------
st_take "$ST_TMP/builtin"
st_profiles
st_take "$ST_TMP/declared"
ST_LINES=$(wc -l < "$ST_TMP/declared/model")
if st_same "$ST_TMP/builtin" "$ST_TMP/declared" && [[ $ST_LINES -gt 10 ]] \
    && grep -q '"command":"roadmap"\|"command": "roadmap"' "$ST_TMP/declared/roadmap"; then
    st_ok "stores read the same, built in or declared" "roadmap, sas status, check --json and $ST_LINES model lines byte-identical"
else
    st_fail "stores read the same, built in or declared" "$(for f in roadmap sas check model; do cmp -s "$ST_TMP/builtin/$f" "$ST_TMP/declared/$f" || printf '%s differs; ' "$f"; done)$ST_LINES model lines"
fi

# The comparison has teeth: a doctored byte in each store is told apart, by
# the store's own rule, and restoring it reads clean again.
st_doctor() { sed -i "0,/$2/s/$2/$3/" "$ST_ROOT/$1"; }
st_doctor "docs/sas/$ST_SAS" "a" "A"
st_take "$ST_TMP/sas-doctored"
ST_DRIFT=$(grep -c '"rule": *"sas.digest-drift"' "$ST_TMP/sas-doctored/check")
git -C "$ST_ROOT" checkout -q -- docs/sas
st_doctor "docs/roadmap/atoms/20-phases.yaml" "More work" "More Work"
st_take "$ST_TMP/roadmap-doctored"
git -C "$ST_ROOT" checkout -q -- docs/roadmap
st_doctor "docs/adr/atoms/ST-ADR-0002-plant.md" "second" "Second"
st_take "$ST_TMP/adr-doctored"
git -C "$ST_ROOT" checkout -q -- docs/adr
st_take "$ST_TMP/restored"
if [[ $ST_DRIFT -ge 1 ]] && ! cmp -s "$ST_TMP/declared/check" "$ST_TMP/sas-doctored/check" \
    && ! cmp -s "$ST_TMP/declared/roadmap" "$ST_TMP/roadmap-doctored/roadmap" \
    && ! cmp -s "$ST_TMP/declared/model" "$ST_TMP/adr-doctored/model" \
    && st_same "$ST_TMP/declared" "$ST_TMP/restored"; then
    st_ok "a doctored byte is refused" "SAS byte: sas.digest-drift; roadmap and ADR bytes: told apart; restored: identical"
else
    st_fail "a doctored byte is refused" "digest-drift $ST_DRIFT; check/roadmap/model differ: $(cmp -s "$ST_TMP/declared/check" "$ST_TMP/sas-doctored/check" && echo no || echo yes)/$(cmp -s "$ST_TMP/declared/roadmap" "$ST_TMP/roadmap-doctored/roadmap" && echo no || echo yes)/$(cmp -s "$ST_TMP/declared/model" "$ST_TMP/adr-doctored/model" && echo no || echo yes); restored same: $(st_same "$ST_TMP/declared" "$ST_TMP/restored" && echo yes || echo no)"
fi

# --- the rules are the type's -------------------------------------------------------
st_doctor "docs/sas/$ST_SAS" "a" "A"
sed -i 's/^capabilities = \["structure", "links", "acceptance"\]$/capabilities = ["structure", "links"]/' "$ST_ROOT/profiles/spec.toml"
ST_NARROW=$(st_war --json check 2>&1)
cp "$REPO_ROOT/profiles/spec.toml" "$ST_ROOT/profiles/spec.toml"
ST_FULL=$(st_war --json check 2>&1)
sed -i 's/^capabilities = .*/capabilities = ["structure", "verification"]/' "$ST_ROOT/profiles/spec.toml"
ST_WIDE=$(st_war --json check 2>&1); ST_WIDE_RC=$?
git -C "$ST_ROOT" checkout -q -- docs/sas
cp "$REPO_ROOT/profiles/spec.toml" "$ST_ROOT/profiles/spec.toml"
if ! grep -q 'sas.digest-drift' <<<"$ST_NARROW" && grep -q '"rule": *"sas.digest-drift"' <<<"$ST_FULL" \
    && [[ $ST_WIDE_RC -ne 0 ]] && grep -q 'profile.capabilities' <<<"$ST_WIDE"; then
    st_ok "store rules are gated on the type" "no acceptance: no digest-drift; acceptance: digest-drift; verification: profile.capabilities"
else
    st_fail "store rules are gated on the type" "narrow has drift: $(grep -c 'sas.digest-drift' <<<"$ST_NARROW"); full: $(grep -c 'sas.digest-drift' <<<"$ST_FULL"); wide exit $ST_WIDE_RC"
fi

# --- the model carries the stores' records and relations ------------------------------
ST_WANT=(
    'R ST-ROADMAP roadmap docs/roadmap/roadmap.toml'
    'L ST-PHASE-2 depends_on ST-PHASE-1'
    'L ST-PHASE-1 part_of ST-ROADMAP'
    'L ST-ADR-0002 supersedes ST-ADR-0001'
    'S ST-ADR-0001 computed superseded computed'
    'S ST-ADR-0002 status accepted authored'
)
ST_MISS=""
for w in "${ST_WANT[@]}"; do grep -qF -- "$w" "$ST_TMP/declared/model" || ST_MISS="$ST_MISS [$w]"; done
ST_SECTIONS=$(grep -c '^R ST-SAS-[0-9.]* section ' "$ST_TMP/declared/model")
ST_SPEC=$(grep -c '^R [A-Z]*-SAS spec ' "$ST_TMP/declared/model")
if [[ -z $ST_MISS && $ST_SECTIONS -ge 1 && $ST_SPEC -eq 1 ]]; then
    st_ok "the model carries the stores" "roadmap, phases, $ST_SECTIONS sections, ADRs; part_of, depends_on, supersedes"
else
    st_fail "the model carries the stores" "missing:$ST_MISS; sections $ST_SECTIONS, spec $ST_SPEC"
fi

command rm -rf "$ST_TMP"
