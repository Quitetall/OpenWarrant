# shellcheck shell=bash
# OW-WAR-0140 — profiles as data, and the contractor Work Order beside an
# unchanged technical core.
#
# A scratch program adopts profiles/contractor.toml and writes the
# NON-BINDING fixture (conformance/fixtures/contractor/work-order.sh). Every
# check pairs what is admitted with what is refused. The fixture is not a
# contract, not a legal instrument, and not legal, financial or quality advice.
#
# OBL-005 (the Phase 10 Exit) is not planted here: it needs the legal,
# finance and QMS decisions SAS §98 names, which no plant can supply.

echo "== contractor profile (OW-WAR-0140) =="
PLANT_ROOT=$(scratch_corpus CT)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ct_ok() { printf 'ok    %-44s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ct_fail() { printf 'FAIL  %-44s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# Run the real copy block on a tiny repository, including on same-device CI.
if python3 conformance/controls/contractor-copy.py; then
    ct_ok "frozen-module copy controls" "all modules preserved; Git objects not hard-linked"
else
    ct_fail "frozen-module copy controls" "copy lost a module or hard-linked an object"
fi
ct_check() { "$WAR" --root "$PLANT_ROOT" check 2>&1; }
ct_commit() {
    git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
    git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1
}

# The register: one human who may accept, one agent granted resolver anyway,
# one human without resolver, and the performer, a human holding resolver.
cat >"$PLANT_ROOT/docs/authority/roles.toml" <<'TOML'
[[assignment]]
actor = "Quinn Accepter"
actor_kind = "human"
roles = ["resolver"]
assigned_by = "plant"
effective_time = "2026-09-25T00:00:00Z"

[[assignment]]
actor = "Ada Agent"
actor_kind = "agent"
roles = ["resolver"]
assigned_by = "plant"
effective_time = "2026-09-25T00:00:00Z"

[[assignment]]
actor = "Norm Human"
actor_kind = "human"
roles = ["verifier"]
assigned_by = "plant"
effective_time = "2026-09-25T00:00:00Z"

[[assignment]]
actor = "Pat Performer"
actor_kind = "human"
roles = ["performer", "resolver"]
assigned_by = "plant"
effective_time = "2026-09-25T00:00:00Z"
TOML
python3 - "$PLANT_ROOT/openwarrant.toml" <<'PY'
import sys
p = sys.argv[1]; t = open(p).read()
t = t.replace("[project]\n", "[project]\nperformer = \"Pat Performer\"\n", 1)
open(p, "w").write(t)
PY
grep -q '^performer = "Pat Performer"' "$PLANT_ROOT/openwarrant.toml" || { printf 'PLANT SETUP FAILED: could not set the performer\n' >&2; exit 9; }
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
ct_commit "a register"
CT_BASE_IR=$(cat "$PLANT_ROOT"/docs/warrants/CT-WAR-0001/generated/WAR.json 2>/dev/null)
[[ -n "$CT_BASE_IR" ]] || { printf 'PLANT SETUP FAILED: the scaffold Warrant did not compile\n' >&2; exit 9; }

# --- OBL-001: the seam changes nothing for the existing profiles ----------
# The two core profiles as data: adopting them moves no digest.
mkdir -p "$PLANT_ROOT/profiles"
command cp profiles/delivery.toml profiles/decision.toml "$PLANT_ROOT/profiles/"
CT_OUT=$("$WAR" --root "$PLANT_ROOT" compile 2>&1); CT_STATUS=$?
if [[ $CT_STATUS -eq 0 ]] && [[ "$(cat "$PLANT_ROOT"/docs/warrants/CT-WAR-0001/generated/WAR.json)" == "$CT_BASE_IR" ]]; then
    ct_ok "core profiles as data move no digest" "CT-WAR-0001's IR is byte-identical with profiles/{delivery,decision}.toml"
else
    ct_fail "core profiles as data move no digest" "compile exit $CT_STATUS: $(tail -1 <<<"$CT_OUT")"
fi
# Refusal: a program cannot loosen a core profile by editing its file.
sed -i '/"assurance",/d' "$PLANT_ROOT/profiles/delivery.toml"
CT_OUT=$(ct_check); CT_STATUS=$?
command cp profiles/delivery.toml "$PLANT_ROOT/profiles/delivery.toml"
if [[ $CT_STATUS -ne 0 ]] && grep -q 'profile.invalid: profiles/delivery.toml: redefines core profile delivery' <<<"$CT_OUT"; then
    ct_ok "a loosened core profile is refused" "profile.invalid, redefines core profile delivery (exit $CT_STATUS)"
else
    ct_fail "a loosened core profile is refused" "exit $CT_STATUS: $(head -2 <<<"$CT_OUT")"
fi
# The corpus half: the seam commit rewrote no Warrant's compiled IR.
# The seam is the commit that added profiles/delivery.toml — found by what
# it did, not by its subject, which a squash merge renames. Its own
# Warrant's IR moves with its amendment; no OTHER Warrant's may.
CT_SEAM=$(git log --diff-filter=A --format=%H -- profiles/delivery.toml 2>/dev/null | tail -1)
if [[ -z "$CT_SEAM" ]]; then
    ct_fail "the seam rewrote no committed IR" "no commit added profiles/delivery.toml"
elif CT_IRS=$(git diff --name-only "$CT_SEAM^" "$CT_SEAM" -- 'docs/warrants/*/generated/WAR.json' | grep -v '^docs/warrants/OW-WAR-0140/') ; [[ -z "$CT_IRS" ]]; then
    ct_ok "the seam rewrote no committed IR" "no generated/WAR.json changed in ${CT_SEAM:0:12}"
else
    ct_fail "the seam rewrote no committed IR" "changed: $(head -3 <<<"$CT_IRS" | tr '\n' ' ')"
fi
ct_commit "core profiles"

# --- OBL-002: missing a contractor role fails closed; nothing else loosens
command cp profiles/contractor.toml "$PLANT_ROOT/profiles/"
CT_DIR=$(bash conformance/fixtures/contractor/work-order.sh "$WAR" "$PLANT_ROOT" "Quinn Accepter" 2>&1 | tail -1)
[[ -f "${CT_DIR:-/nonexistent}/manifest.toml" ]] || { printf 'PLANT SETUP FAILED: the contractor fixture wrote no Warrant: %s\n' "$CT_DIR" >&2; exit 9; }
CT_ALIAS=$(basename "$CT_DIR")
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
ct_commit "the contractor fixture"
CT_MANIFEST="$CT_DIR/manifest.toml"
CT_OUT=$(ct_check)
if grep -q "manifest.valid .*$CT_ALIAS" <<<"$CT_OUT" && [[ $(grep -c '^\[\[atoms\]\]' "$CT_MANIFEST") -eq 9 ]] \
    && grep -q '^profile = "contractor"' "$CT_MANIFEST"; then
    ct_ok "a contractor Work Order validates" "$CT_ALIAS: nine atoms, profile contractor"
else
    ct_fail "a contractor Work Order validates" "$(grep -E "^(ERROR|FAIL).*$CT_ALIAS" <<<"$CT_OUT" | head -1)"
fi
ct_manifest_refused() { # <name> <python edit over t> <expected text>
    python3 - "$CT_MANIFEST" "$2" <<'PY'
import re, sys
p = sys.argv[1]; t = open(p).read()
exec(sys.argv[2])
open(p, "w").write(t)
PY
    local out
    out=$(ct_check)
    git -C "$PLANT_ROOT" checkout -q -- .
    if grep -q "^ERROR manifest.invalid" <<<"$out" && grep -qF "$3" <<<"$out"; then
        ct_ok "$1" "manifest.invalid: $3"
    else
        ct_fail "$1" "$(grep -E '^(ERROR|WARN) manifest' <<<"$out" | head -1)"
    fi
}
ct_manifest_refused "without contractor.acceptance: refused" \
    't = re.sub(r"\[\[atoms\]\]\nordinal = 73\n[^\[]*", "", t)' \
    "profile contractor requires a contractor.acceptance atom"
ct_manifest_refused "an unknown profile: refused" \
    't = t.replace("profile = \"contractor\"", "profile = \"experiment\"")' \
    'unknown profile "experiment"'
# The same manifest in a program without the definition: unknown.
command mv "$PLANT_ROOT/profiles/contractor.toml" "$PLANT_ROOT/contractor.away"
CT_OUT=$(ct_check)
command mv "$PLANT_ROOT/contractor.away" "$PLANT_ROOT/profiles/contractor.toml"
if grep -q "^ERROR manifest.invalid" <<<"$CT_OUT" && grep -qF 'unknown profile "contractor"' <<<"$CT_OUT"; then
    ct_ok "without profiles/contractor.toml: unknown" 'manifest.invalid: unknown profile "contractor"'
else
    ct_fail "without profiles/contractor.toml: unknown" "$(grep -E '^(ERROR|WARN) manifest' <<<"$CT_OUT" | head -1)"
fi
# Elsewhere nothing loosens: a delivery Warrant, in this same program, with
# contractor.terms REQUIRED is still an unknown required role...
CT_DELIVERY="$PLANT_ROOT/docs/warrants/CT-WAR-0001/manifest.toml"
command cp "$CT_DIR/atoms/72-contractor-terms.md" "$PLANT_ROOT/docs/warrants/CT-WAR-0001/atoms/72-terms.md"
printf '\n[[atoms]]\nordinal = 72\nrole = "contractor.terms"\npath = "atoms/72-terms.md"\nrequired = true\n' >>"$CT_DELIVERY"
CT_OUT=$(ct_check)
if grep -q '^ERROR manifest.invalid' <<<"$CT_OUT" && grep -qF 'declares role "contractor.terms", which is neither a core role' <<<"$CT_OUT"; then
    ct_ok "required contractor.terms on delivery: refused" "UnknownRequiredRole, as before the registry"
else
    ct_fail "required contractor.terms on delivery: refused" "$(grep -E '^(ERROR|WARN) manifest' <<<"$CT_OUT" | head -1)"
fi
# ...and optional, it is preserved as it always was.
sed -i '$ s/required = true/required = false/' "$CT_DELIVERY"
CT_OUT=$(ct_check)
git -C "$PLANT_ROOT" checkout -q -- . && git -C "$PLANT_ROOT" clean -fdq
if grep -q 'manifest.valid .*CT-WAR-0001' <<<"$CT_OUT" && ! grep -q '^ERROR manifest.invalid' <<<"$CT_OUT"; then
    ct_ok "optional contractor.terms on delivery: kept" "manifest.valid"
else
    ct_fail "optional contractor.terms on delivery: kept" "$(grep -E '^(ERROR|WARN) manifest' <<<"$CT_OUT" | head -1)"
fi

# --- OBL-003: the technical core is unchanged; terms live in the profile --
CT_IR_C=$(mktemp); CT_IR_D=$(mktemp)
command cp "$CT_DIR/generated/WAR.json" "$CT_IR_C"
python3 - "$CT_MANIFEST" <<'PY'
import re, sys
p = sys.argv[1]; t = open(p).read()
t = t.replace('profile = "contractor"', 'profile = "delivery"')
t = re.sub(r'\[\[atoms\]\]\nordinal = 7\d\nrole = "contractor\.[a-z]+"\n[^\[]*', '', t)
open(p, "w").write(t)
PY
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
command cp "$CT_DIR/generated/WAR.json" "$CT_IR_D"
git -C "$PLANT_ROOT" checkout -q -- .
# Equal after removing exactly what the obligation names.
ct_ir_equal() { # <contractor IR> <delivery IR>
    python3 - "$1" "$2" <<'PY'
import json, sys
def strip(ir):
    ir["identity"].pop("profile")
    ir["format_basis"].pop("profile_schema_id")
    sc = ir["source_and_composition"]
    sc.pop("manifest_digest")
    sc["atoms"] = [a for a in sc["atoms"] if not a["role"].startswith("contractor.")]
    for k in ("composition_revision_digest", "workspace_basis_digest"):
        ir["integrity"].pop(k)
    return ir
c, d = (strip(json.load(open(f))) for f in sys.argv[1:3])
sys.exit(0 if c == d else 1)
PY
}
ct_profile_of() { python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["identity"]["profile"])' "$1"; }
if [[ $(ct_profile_of "$CT_IR_C") == contractor && $(ct_profile_of "$CT_IR_D") == delivery ]] \
    && ct_ir_equal "$CT_IR_C" "$CT_IR_D"; then
    ct_ok "contractor IR = delivery IR, less the named" "profile, schema id, contractor.* entries, their digests"
else
    ct_fail "contractor IR = delivery IR, less the named" "profiles $(ct_profile_of "$CT_IR_C") and $(ct_profile_of "$CT_IR_D"); some other field differs"
fi
# Control: the comparison is not vacuous. A difference it was not told to
# remove — here the title — fails it.
python3 - "$CT_IR_D" <<'PY2'
import json, sys
ir = json.load(open(sys.argv[1])); ir["identity"]["title"] = "another title"
json.dump(ir, open(sys.argv[1] + ".t", "w"))
PY2
if ! ct_ir_equal "$CT_IR_C" "$CT_IR_D.t"; then
    ct_ok "any other IR difference fails the comparison" "a changed title is caught"
else
    ct_fail "any other IR difference fails the comparison" "a changed title compared equal"
fi
# No party, term, invoice or payment token reaches either IR...
if ! grep -qE 'FIXTURE-|compensation|kf://' "$CT_IR_C" "$CT_IR_D"; then
    ct_ok "no term string in the core IR" "no FIXTURE-*, compensation or kf:// in either IR"
else
    ct_fail "no term string in the core IR" "$(grep -ohE 'FIXTURE-[A-Z0-9-]+|compensation|kf://[^\"]*' "$CT_IR_C" "$CT_IR_D" | head -1)"
fi
# ...and the probe finds one when one is there.
python3 - "$CT_MANIFEST" <<'PY'
import sys
p = sys.argv[1]; t = open(p).read()
t = t.replace('title = "', 'title = "FIXTURE-WO-0001 ', 1)
open(p, "w").write(t)
PY
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
if grep -q 'FIXTURE-WO-0001' "$CT_DIR/generated/WAR.json"; then
    ct_ok "a term copied into a core field is found" "FIXTURE-WO-0001 in identity.title"
else
    ct_fail "a term copied into a core field is found" "the probe missed it"
fi
git -C "$PLANT_ROOT" checkout -q -- .
command rm -f "$CT_IR_C" "$CT_IR_D" "$CT_IR_D.t"

# The frozen modules: the seam changed none of them, and touched only
# role.rs and manifest.rs in the core and compiler. (Not "unchanged since":
# a later Warrant may amend the core under its own authority; the claim is
# about what the profile seam did.)
CT_FROZEN=(
    crates/openwarrant-core/src/lifecycle.rs crates/openwarrant-core/src/state.rs
    crates/openwarrant-core/src/contract.rs crates/openwarrant-core/src/obligation.rs
    crates/openwarrant-core/src/verification.rs crates/openwarrant-core/src/independence.rs
    crates/openwarrant-core/src/resolution.rs crates/openwarrant-core/src/authority.rs
    crates/openwarrant-core/src/deliverable.rs crates/openwarrant-core/src/gate.rs
    crates/openwarrant-core/src/gate_run.rs
    crates/openwarrant-compiler/src/ir.rs crates/openwarrant-compiler/src/canonical.rs
    crates/openwarrant-compiler/src/digest.rs
)
ct_frozen_clean() { # <repo> <rev> -> 0 when every frozen module equals <rev>'s
    git -C "$1" diff --quiet "$2" -- "${CT_FROZEN[@]}"
}
CT_SEAM_CORE=$( [[ -n "$CT_SEAM" ]] && git show --name-only --format= "$CT_SEAM" -- crates/openwarrant-core crates/openwarrant-compiler | sort | tr '\n' ' ')
if [[ -n "$CT_SEAM" ]] && git diff --quiet "$CT_SEAM^" "$CT_SEAM" -- "${CT_FROZEN[@]}" \
    && [[ "$CT_SEAM_CORE" == "crates/openwarrant-core/src/manifest.rs crates/openwarrant-core/src/role.rs " ]]; then
    ct_ok "the seam left the frozen core alone" "${#CT_FROZEN[@]} modules unchanged by it; it touched role.rs, manifest.rs"
else
    ct_fail "the seam left the frozen core alone" "seam ${CT_SEAM:-missing}; core files it touched: $CT_SEAM_CORE; $(git diff --stat "${CT_SEAM:-HEAD}^" "${CT_SEAM:-HEAD}" -- "${CT_FROZEN[@]}" | tail -1)"
fi
# Refusal: an `invoice` field on a core struct, in a copy of this tree.
CT_COPY=$(mktemp -d)
git clone -q --local --no-hardlinks --no-checkout "$REPO_ROOT" "$CT_COPY/r" 2>/dev/null \
    && git -C "$CT_COPY/r" checkout -q HEAD -- "${CT_FROZEN[@]}" \
    || { printf 'PLANT SETUP FAILED: could not copy the frozen modules\n' >&2; exit 9; }
sed -i '/^pub struct Identity {/a\    pub invoice: String,' "$CT_COPY/r/crates/openwarrant-compiler/src/ir.rs"
grep -q 'pub invoice: String' "$CT_COPY/r/crates/openwarrant-compiler/src/ir.rs" || { printf 'PLANT SETUP FAILED: the invoice field was not planted\n' >&2; exit 9; }
if ! ct_frozen_clean "$CT_COPY/r" HEAD; then
    ct_ok "an invoice field on a core struct: caught" "ir.rs Identity differs from the committed module"
else
    ct_fail "an invoice field on a core struct: caught" "the frozen-module diff stayed empty"
fi
command rm -rf "$CT_COPY"

# --- OBL-004: acceptance is the existing human resolution act --------------
ct_acceptance() { # <name> <actor> <expected severity> <expected text>
    sed -i "s/^acceptance_authority: .*/acceptance_authority: $2/" "$CT_DIR/atoms/73-contractor-acceptance.md"
    local out line
    out=$(ct_check)
    git -C "$PLANT_ROOT" checkout -q -- .
    line=$(grep -E "^[A-Z]+ +profile.acceptance-authority" <<<"$out" | head -1)
    if [[ "$line" == "$3 "* ]] && grep -qF "$4" <<<"$line"; then
        ct_ok "$1" "$3: $4"
    else
        ct_fail "$1" "${line:-no profile.acceptance-authority diagnostic}"
    fi
}
ct_acceptance "a human holding resolver may accept" "Quinn Accepter" PASS "is a human holding resolver"
ct_acceptance "an agent is refused by kind" "Ada Agent" ERROR "is agent-kind"
ct_acceptance "no resolver: refused by name" "Norm Human" ERROR 'does not hold `resolver`'
ct_acceptance "the performer is refused (§27.2)" "Pat Performer" ERROR "is the performer"
ct_acceptance "an actor the register lacks: refused" "Nobody Known" ERROR "has no assignment"
# No actor role was added: the register's roles are §27's six.
ct_roles() { # <authority.rs> -> the ActorRole variants, one line
    awk '/^pub enum ActorRole \{/ { on = 1; next } on && /^\}/ { exit } on' "$1" \
        | grep -oE '^    [A-Z][A-Za-z]*,' | tr -d ' ,' | tr '\n' ' '
}
CT_SIX="Performer Verifier Authorizer Resolver RiskAcceptor Judge "
if [[ "$(ct_roles crates/openwarrant-core/src/authority.rs)" == "$CT_SIX" ]]; then
    ct_ok "no new ActorRole variant" "$CT_SIX"
else
    ct_fail "no new ActorRole variant" "$(ct_roles crates/openwarrant-core/src/authority.rs)"
fi
CT_PLANTED=$(mktemp)
sed '/^pub enum ActorRole {/a\    Acceptor,' crates/openwarrant-core/src/authority.rs >"$CT_PLANTED"
if [[ "$(ct_roles "$CT_PLANTED")" != "$CT_SIX" ]]; then
    ct_ok "a planted Acceptor role is seen" "$(ct_roles "$CT_PLANTED")"
else
    ct_fail "a planted Acceptor role is seen" "the source check missed it"
fi
command rm -f "$CT_PLANTED"
# A kf:// invoice reference is UNKNOWN: not PASS, not ERROR (Law 15, U-004).
CT_OUT=$(ct_check)
CT_INV=$(grep -E '^[A-Z]+ +profile.reference .*kf://finance/invoices/FIXTURE-INV-0001' <<<"$CT_OUT")
if [[ $(wc -l <<<"$CT_INV") -eq 1 && "$CT_INV" == "UNKNOWN "* ]]; then
    ct_ok "a kf:// invoice reference is UNKNOWN" "no Knowledge Fabric reachable"
else
    ct_fail "a kf:// invoice reference is UNKNOWN" "${CT_INV:-no diagnostic for it}"
fi
# Control: the check resolves what it can — a war:// reference to a Warrant
# here passes — and a reference role citing nothing is named.
CT_UUID=$(grep -oE '^uuid = "[^"]+"' "$PLANT_ROOT/docs/warrants/CT-WAR-0001/manifest.toml" | cut -d'"' -f2)
printf -- '- related Warrant: war://%s\n' "$CT_UUID" >>"$CT_DIR/atoms/74-contractor-commercial.md"
CT_OUT=$(ct_check)
git -C "$PLANT_ROOT" checkout -q -- .
if grep -qE "^PASS +profile.reference .*war://$CT_UUID" <<<"$CT_OUT"; then
    ct_ok "a war:// reference here resolves" "PASS, not a blanket UNKNOWN"
else
    ct_fail "a war:// reference here resolves" "$(grep "war://" <<<"$CT_OUT" | head -1)"
fi
sed -i '/:\/\//d' "$CT_DIR/atoms/74-contractor-commercial.md"
CT_OUT=$(ct_check)
git -C "$PLANT_ROOT" checkout -q -- .
if grep -qE '^(WARN|ERROR) +profile.reference-missing .*contractor.commercial cites no reference' <<<"$CT_OUT"; then
    ct_ok "a reference role citing nothing is named" "profile.reference-missing (§22.3: link, not copy)"
else
    ct_fail "a reference role citing nothing is named" "$(grep 'profile.reference' <<<"$CT_OUT" | head -1)"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT CT_DIR CT_ALIAS CT_MANIFEST CT_OUT CT_STATUS CT_SEAM CT_BASE_IR
