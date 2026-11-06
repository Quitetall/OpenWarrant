# shellcheck shell=bash
# OW-WAR-0148 M2 (OBL-003, OBL-005) — a document's type chooses its
# capabilities, and a new manifest pins its profile file (OW-ADR-0031).
#
# One scratch program (CP) with the committed delivery and decision profile
# files, and a test profile `lab` that extends delivery and selects only the
# working form's capabilities (structure, links, claims), in contract form so
# a Warrant of it sits in docs/warrants/ beside the others:
#   CP-WAR-0001  the scaffold's delivery Warrant, made before profiles/ existed:
#                its manifest has no `profile_digest`;
#   CP-WAR-0002  a decision Warrant from `war new --preset decision`;
#   CP-WAR-0003  a `lab` Warrant from `war new --profile lab`.
#
# Accepted: a new manifest pins its profile file's sha256; a decision Warrant
# reaches `would_satisfy` with requirement 12 named "not applicable: no
# `stages` capability"; a manifest without the pin checks byte for byte the
# same whatever its profile file says.
# Refused: `verification` without `evidence` (profile.capability-prerequisite)
# and an unknown capability (profile.capability-unknown); a kind without
# `authorization` is refused `war sign --dry-run` and `war authorize` by name,
# and `war next` and `war sign --list` offer it nothing; a kind without
# `resolution` is refused `war resolve`; requirement 12 never reads met, in
# the dry run or in `status --json`; one byte changed in a pinned profile is
# `profile.pin-drift`, a warning unsigned and an error once authorized.
#
# The authorization is signed by a throwaway key in a throwaway ssh-agent
# asserted to hold that key and nothing else, with the caller's agent unset.

echo "== capabilities in profile data (OW-WAR-0148 M2) =="
CP_ROOT=$(scratch_corpus CP)
[[ -d "${CP_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
CP_TMP=$(mktemp -d)
CP_WAR="$REPO_ROOT/${WAR#./}"
CP_D=CP-WAR-0002
CP_L=CP-WAR-0003

if ! WAR="$CP_WAR" D="$CP_ROOT" T="$CP_TMP" R="$REPO_ROOT" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$CP_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w() { "$WAR" --root . "$@"; }
mkdir -p profiles
cp "$R/profiles/delivery.toml" "$R/profiles/decision.toml" profiles/
cat > profiles/lab.toml <<'LAB'
schema = "oh.war/profile/v1"
name = "lab"
extends = "delivery"
approved = false
note = "A test kind: the working form's capabilities, in contract form."
capabilities = ["structure", "links", "claims"]
LAB
sed -e 's/^gate_id: .*/gate_id: "plant.tree"/' "$R/docs/gates/ops.echo@1.0.0.yaml" > docs/gates/plant.tree@1.0.0.yaml
grep -q '^argv: \["true"\]' docs/gates/plant.tree@1.0.0.yaml
w new --preset decision "A decision" >/dev/null
w new --profile lab "A lab note" >/dev/null
test -d docs/warrants/CP-WAR-0002 && test -d docs/warrants/CP-WAR-0003
dw=docs/warrants/CP-WAR-0002
python3 - "$dw/atoms/60-assurance.md" <<'PY'
import sys
p = sys.argv[1]
open(p, "w").write(open(p).read().split("## Acceptance Obligations")[0] + """## Acceptance Obligations

### OBL-001 — the decision is recorded
- **scope:** one file in this scratch program.
- **gate:** `gate://plant.tree@1.0.0`
- **evidence:** the file's bytes.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can this pass with the file wrong? Yes; it is a plant.

- **outcome:** no_counterexample
""")
PY
mkdir -p src
printf 'decided\n' > src/decision.txt
cat > "$dw/deliverables.toml" <<'DELIV'
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "the decision's file"
kind = "file"
target_ref = "src/decision.txt"
required = true
content_addressed = false
provenance_required = true
obligation_refs = ["OBL-001"]
DELIV
printf 'schema = "oh.war/rationale/v1"\n' > "$dw/rationale.toml"
cp "$R/conformance/fixtures/verifier/establishes-all.sh" "$T/verifier.sh"
grep -q '^verifier_argv = ' openwarrant.toml
sed -i "s|^verifier_argv = .*|verifier_argv = [\"bash\", \"$T/verifier.sh\"]|" openwarrant.toml
sed -i "s|^verifier_timeout_secs = .*|verifier_timeout_secs = 120|" openwarrant.toml
ssh-keygen -q -t ed25519 -N "" -C plant -f "$T/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$T/id_plant.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/73-capabilities.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "plant"
ROLES
w compile >/dev/null
g add -A; g commit -qm "a decision, a lab note, and their profiles"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$CP_TMP/setup.log" >&2
    exit 9
fi

cp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
cp_war()  {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
        GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid \
        GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid \
        "$CP_WAR" --root "$CP_ROOT" "$@" </dev/null
}
cp_git()  { git -C "$CP_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
cp_sha()  { sha256sum "$1" | cut -d' ' -f1; }
cp_line() { grep -m1 -E "$1" <<<"$2"; }
# One batch of whatever awaits, from a throwaway agent holding the plant key
# and nothing else, killed after.
cp_sign() {
    local old=${SSH_AUTH_SOCK:-} held want out
    unset SSH_AUTH_SOCK SSH_AGENT_PID
    eval "$(ssh-agent -s)" >/dev/null
    ssh-add -q "$CP_TMP/id_plant" 2>/dev/null
    want=$(ssh-keygen -lf "$CP_TMP/id_plant.pub" | awk '{print $2}')
    held=$(ssh-add -l 2>/dev/null)
    if [[ $(grep -c . <<<"$held") -ne 1 || "$(awk '{print $2}' <<<"$held")" != "$want" ]]; then
        printf 'PLANT SETUP FAILED: the agent holds another key\n' >&2
        ssh-agent -k >/dev/null 2>&1
        exit 9
    fi
    out=$("$CP_WAR" --root "$CP_ROOT" sign --batch --ssh-sign --as "Plant Signer" </dev/null 2>&1)
    ssh-agent -k >/dev/null 2>&1 || true
    if [[ -n "$old" ]]; then export SSH_AUTH_SOCK="$old"; else unset SSH_AUTH_SOCK; fi
    unset SSH_AGENT_PID
    printf '%s' "$out"
}

# 1. Profile files are checked: a prerequisite missing, a name outside the set.
printf 'schema = "oh.war/profile/v1"\nname = "nogate"\nextends = "delivery"\napproved = false\ncapabilities = ["structure", "verification"]\n' \
    > "$CP_ROOT/profiles/nogate.toml"
CP_OUT=$(cp_war check 2>&1); CP_STATUS=$?
if [[ $CP_STATUS -ne 0 ]] && cp_line 'profile\.capability-prerequisite' "$CP_OUT" >/dev/null \
    && cp_line 'selects `verification` without `evidence`' "$CP_OUT" >/dev/null; then
    cp_ok "verification without evidence" "refused: profile.capability-prerequisite, naming both"
else
    cp_fail "verification without evidence" "exit $CP_STATUS: $(head -c 300 <<<"$CP_OUT")"
fi
printf 'schema = "oh.war/profile/v1"\nname = "nogate"\nextends = "delivery"\napproved = false\ncapabilities = ["structure", "telepathy"]\n' \
    > "$CP_ROOT/profiles/nogate.toml"
CP_OUT=$(cp_war check 2>&1); CP_STATUS=$?
if [[ $CP_STATUS -ne 0 ]] && cp_line 'profile\.capability-unknown' "$CP_OUT" >/dev/null \
    && cp_line '"telepathy"' "$CP_OUT" >/dev/null; then
    cp_ok "an unknown capability" "refused: profile.capability-unknown, naming it"
else
    cp_fail "an unknown capability" "exit $CP_STATUS: $(head -c 300 <<<"$CP_OUT")"
fi
command rm -f "$CP_ROOT/profiles/nogate.toml"
CP_OUT=$(cp_war check "$CP_L" 2>&1); CP_STATUS=$?
if cp_line 'capability\.not-applicable .*profile lab does not select .*`authorization`' "$CP_OUT" >/dev/null \
    && ! cp_line 'profile\.capability' "$CP_OUT" >/dev/null; then
    cp_ok "a narrowed kind is admitted" "lab: its absent capabilities named, not passed (exit $CP_STATUS)"
else
    cp_fail "a narrowed kind is admitted" "exit $CP_STATUS: $(head -c 300 <<<"$CP_OUT")"
fi

# 2. The pin. A new manifest carries the sha256 of the profile file.
CP_PIN_D=$(sed -n 's/^profile_digest = "\(.*\)"$/\1/p' "$CP_ROOT/docs/warrants/$CP_D/manifest.toml")
CP_PIN_L=$(sed -n 's/^profile_digest = "\(.*\)"$/\1/p' "$CP_ROOT/docs/warrants/$CP_L/manifest.toml")
if [[ "$CP_PIN_D" == "sha256:$(cp_sha "$CP_ROOT/profiles/decision.toml")" \
    && "$CP_PIN_L" == "sha256:$(cp_sha "$CP_ROOT/profiles/lab.toml")" ]] \
    && ! grep -q '^profile_digest' "$CP_ROOT/docs/warrants/CP-WAR-0001/manifest.toml"; then
    cp_ok "a new manifest pins its profile" "$CP_D: $CP_PIN_D; the scaffold's older manifest has none"
else
    cp_fail "a new manifest pins its profile" "decision '$CP_PIN_D', lab '$CP_PIN_L'"
fi
CP_OUT=$(cp_war check "$CP_D" 2>&1)
if cp_line "^PASS profile\.pinned .*$CP_D" "$CP_OUT" >/dev/null && ! cp_line 'profile\.pin-drift' "$CP_OUT" >/dev/null; then
    cp_ok "the pinned file checks clean" "profile.pinned"
else
    cp_fail "the pinned file checks clean" "$(head -c 300 <<<"$CP_OUT")"
fi
# One byte, before authorization: a warning naming the profile and both.
CP_BASE1=$(cp_war check CP-WAR-0001 2>&1)
printf '#' >> "$CP_ROOT/profiles/decision.toml"
printf '#' >> "$CP_ROOT/profiles/delivery.toml"
CP_NOW=$(cp_sha "$CP_ROOT/profiles/decision.toml")
CP_OUT=$(cp_war check "$CP_D" 2>&1)
CP_BASE2=$(cp_war check CP-WAR-0001 2>&1)
if cp_line "^WARN +profile\.pin-drift .*$CP_D: profile decision was pinned at $CP_PIN_D and profiles/decision.toml is now sha256:$CP_NOW" "$CP_OUT" >/dev/null; then
    cp_ok "unsigned drift is a warning" "profile.pin-drift names decision and both digests"
else
    cp_fail "unsigned drift is a warning" "$(cp_line 'pin-drift' "$CP_OUT")"
fi
# A manifest without the pin checks exactly as before, whatever the file says.
if [[ -n "$CP_BASE1" && "$CP_BASE1" == "$CP_BASE2" ]] && ! cp_line 'profile\.pin' "$CP_BASE2" >/dev/null; then
    cp_ok "no pin, no change" "CP-WAR-0001's check is byte-identical across the profile edit"
else
    cp_fail "no pin, no change" "$(diff <(printf '%s\n' "$CP_BASE1") <(printf '%s\n' "$CP_BASE2") | head -5)"
fi
cp_git checkout -q -- profiles/

# 3. A kind without `authorization`: offered nothing, refused by name.
CP_NEXT=$(cp_war next 2>&1)
CP_LIST=$(cp_war sign --list 2>&1)
CP_SIGN=$(cp_war sign "$CP_L" --dry-run 2>&1); CP_SIGN_STATUS=$?
CP_AUTH=$(cp_war authorize "$CP_L" 2>&1); CP_AUTH_STATUS=$?
if cp_line "$CP_D .*authorize" "$CP_NEXT" >/dev/null && cp_line "$CP_D .*authorize" "$CP_LIST" >/dev/null \
    && ! cp_line "$CP_L" "$CP_NEXT" >/dev/null && ! cp_line "$CP_L" "$CP_LIST" >/dev/null; then
    cp_ok "next offers lab nothing" "the decision's authorize is offered; $CP_L appears in neither next nor sign --list"
else
    cp_fail "next offers lab nothing" "$(cp_line "$CP_L" "$CP_NEXT$CP_LIST")"
fi
if [[ $CP_SIGN_STATUS -ne 0 && $CP_AUTH_STATUS -ne 0 ]] \
    && cp_line "^ERROR +capability\.absent .*$CP_L: profile lab does not select the \`authorization\` capability" "$CP_SIGN" >/dev/null \
    && cp_line "capability\.absent: $CP_L: .*\`authorization\`" "$CP_AUTH" >/dev/null; then
    cp_ok "sign and authorize refuse lab" "capability.absent, naming \`authorization\`"
else
    cp_fail "sign and authorize refuse lab" "sign $CP_SIGN_STATUS: $(head -c 200 <<<"$CP_SIGN"); authorize $CP_AUTH_STATUS: $(head -c 200 <<<"$CP_AUTH")"
fi
CP_RES=$(cp_war resolve --dry-run "$CP_L" 2>&1); CP_RES_STATUS=$?
CP_REQ=$(cp_war resolve "$CP_L" 2>&1); CP_REQ_STATUS=$?
CP_VER=$(cp_war verify "$CP_L" --performer claude 2>&1); CP_VER_STATUS=$?
if [[ $CP_RES_STATUS -ne 0 && $CP_REQ_STATUS -ne 0 && $CP_VER_STATUS -ne 0 ]] \
    && cp_line "capability\.absent .*\`resolution\`" "$CP_RES" >/dev/null \
    && cp_line "capability\.absent: .*\`resolution\`" "$CP_REQ" >/dev/null \
    && cp_line "capability\.absent: .*\`verification\`" "$CP_VER" >/dev/null \
    && ! cp_line 'requirement-met' "$CP_RES" >/dev/null; then
    cp_ok "resolve and verify refuse lab" "capability.absent for \`resolution\` and \`verification\`; no requirement read met"
else
    cp_fail "resolve and verify refuse lab" "$CP_RES_STATUS/$CP_REQ_STATUS/$CP_VER_STATUS: $(head -c 300 <<<"$CP_RES$CP_REQ$CP_VER")"
fi

# 4. The decision, authorized by the throwaway signer, then taken to its
#    sign-off unattended: requirement 12 is not applicable, never met.
cp_sign >/dev/null
[[ -f "$CP_ROOT/docs/warrants/$CP_D/authorization.toml" ]] || { printf 'PLANT SETUP FAILED: the decision was not authorized\n' >&2; exit 9; }
CP_OUT=$(cp_war check "$CP_D" 2>&1)
if cp_line "^PASS profile\.pinned .*$CP_D" "$CP_OUT" >/dev/null; then
    cp_ok "authorized, the pin still holds" "profile.pinned"
else
    cp_fail "authorized, the pin still holds" "$(cp_line 'profile\.pin' "$CP_OUT")"
fi
printf '#' >> "$CP_ROOT/profiles/decision.toml"
CP_NOW=$(cp_sha "$CP_ROOT/profiles/decision.toml")
CP_OUT=$(cp_war check "$CP_D" 2>&1); CP_STATUS=$?
if [[ $CP_STATUS -ne 0 ]] \
    && cp_line "^ERROR +profile\.pin-drift .*$CP_D: profile decision was pinned at $CP_PIN_D and profiles/decision.toml is now sha256:$CP_NOW" "$CP_OUT" >/dev/null; then
    cp_ok "signed drift is an error" "profile.pin-drift: the signature covered the type as pinned"
else
    cp_fail "signed drift is an error" "exit $CP_STATUS: $(cp_line 'pin-drift' "$CP_OUT")"
fi
cp_git checkout -q -- profiles/

cp_git add -A >/dev/null 2>&1
cp_git commit -qm "the decision is authorized" >/dev/null 2>&1
CP_PREP=$(cp_war prepare "$CP_D" 2>&1); CP_PREP_STATUS=$?
CP_RES=$(cp_war resolve --dry-run "$CP_D" 2>&1)
CP_RUNG=$(cp_war status --json 2>/dev/null | python3 -c '
import json, sys
w = [w for w in json.load(sys.stdin)["result"]["warrants"] if w["alias"] == sys.argv[1]][0]
c = w["checks"]
print(w["rung"], c["runtime_receipts_match_the_basis"], ",".join(c.get("not_applicable", [])), len(w["unmet"]))
' "$CP_D" 2>&1)
if [[ "$CP_RUNG" == "would_satisfy False runtime receipts match the basis 0" ]] \
    && cp_line "^PASS resolution\.requirements .*$CP_D: all 12 applicable §56\.1 requirements are met; 1 not applicable" "$CP_RES" >/dev/null \
    && cp_line "resolution\.requirement-not-applicable .*runtime receipts match the basis — not applicable: no \`stages\` capability" "$CP_RES" >/dev/null; then
    cp_ok "a decision reaches would_satisfy" "requirement 12 not applicable: no \`stages\` capability, named"
else
    cp_fail "a decision reaches would_satisfy" "rung '$CP_RUNG'; prepare $CP_PREP_STATUS: $(cp_line '^(ERROR|UNKNOWN|PASS +prepare)' "$CP_PREP"); $(cp_line 'resolution\.requirement' "$CP_RES")"
fi
# The refusal beside it: requirement 12 is never read met for want of
# `stages` — its boolean is false and the dry run names it, not passes it.
if ! cp_line 'requirement-met .*runtime receipts' "$CP_RES" >/dev/null \
    && [[ "${CP_RUNG#would_satisfy }" == False* ]]; then
    cp_ok "an absent capability is never met" "requirement 12 reads false and not applicable, never met"
else
    cp_fail "an absent capability is never met" "$CP_RUNG; $(cp_line 'runtime receipts' "$CP_RES")"
fi

command rm -rf "$CP_TMP"
corpus_gone "$CP_ROOT"
unset CP_ROOT CP_TMP CP_WAR CP_D CP_L
