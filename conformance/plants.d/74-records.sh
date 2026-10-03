# shellcheck shell=bash
# OW-WAR-0148 M3 (OBL-006, OBL-007) — records carry identity and their own
# revision; relations are typed by the profile; `war impact` names what a
# change to one record affects (OW-ADR-0031).
#
# One scratch program (RP) with the committed delivery, decision and ticket
# profiles and this repository's password-reset records
# (docs/records/password-reset/10-records.md):
#   RP-WAR-0001  the scaffold's Warrant, placed in RP-PHASE-1. Its basis names
#                REQ-pr1, CON-pr1 and DEC-pr1; its OBL-001 `evaluates`
#                REQ-pr1 pinned at the revision it had when the Warrant was
#                authorized (by a throwaway signer) and verified (by the
#                fixture verifier), so OBL-001's verdict is `established`.
#   a ticket     whose first item `implements REQ-pr1`.
#
# Accepted: the records parse, each with its type and relations; editing one
# record moves that record's revision and no other; a namespaced kind is
# carried in the model and changes no check, status or readiness; after
# REQ-pr1 changes, `war impact REQ-pr1` names the implementing item, the
# Warrant naming it, OBL-001 with its verdict bound to the old revision
# (stale, still recorded), the phase and the generated views; nothing the
# verification recorded moves.
# Refused, each by rule: a duplicate id (record.duplicate-id), a type the
# profile does not declare (record.type-undeclared, kernel types included), a
# core kind it does not allow (record.relation-undeclared), a word that is no
# kind (record.relation-kind-unknown), a requirement implementing no outcome
# (record.relation-required); an unknown target is a warning with the edge
# kept (record.relation-target-unknown, model.relation-target-unknown); and
# `war impact` on an unknown id (impact.unknown-record).

echo "== records and typed relations (OW-WAR-0148 M3) =="
RP_ROOT=$(scratch_corpus RP)
[[ -d "${RP_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
RP_TMP=$(mktemp -d)
RP_WAR="$REPO_ROOT/${WAR#./}"
RP_REC="docs/records/password-reset/10-records.md"

if ! WAR="$RP_WAR" D="$RP_ROOT" T="$RP_TMP" R="$REPO_ROOT" REC="$RP_REC" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$RP_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w() { "$WAR" --root . "$@"; }
mkdir -p profiles docs/records/password-reset
cp "$R/profiles/delivery.toml" "$R/profiles/decision.toml" "$R/profiles/ticket.toml" profiles/
cp "$R/$REC" "$REC"
sed -e 's/^gate_id: .*/gate_id: "plant.tree"/' "$R/docs/gates/ops.echo@1.0.0.yaml" > docs/gates/plant.tree@1.0.0.yaml
grep -q '^argv: \["true"\]' docs/gates/plant.tree@1.0.0.yaml
w model --json > "$T/m0.json"
python3 - "$T/m0.json" > "$T/rev0" <<'PY'
import json, sys
print([r["revision"] for r in json.load(open(sys.argv[1]))["result"]["records"] if r["id"] == "REQ-pr1"][0])
PY
wd=docs/warrants/RP-WAR-0001
python3 - "$wd/atoms/60-assurance.md" "$(cat "$T/rev0")" <<'PY'
import sys
p, rev = sys.argv[1:]
s = open(p).read()
assert s.count("gate://software.repo.war-check@1.0.0") == 2
s = s.replace("gate://software.repo.war-check@1.0.0", "gate://plant.tree@1.0.0")
head = "### OBL-001 — the SAS is accepted and pinned\n"
assert s.count(head) == 1
s = s.replace(head, head + "- **evaluates:** REQ-pr1@" + rev + "\n")
open(p, "w").write(s)
PY
printf '\n## Records\n\n- REQ-pr1, CON-pr1 and DEC-pr1 (docs/records/password-reset/).\n' >> "$wd/atoms/20-basis.md"
w create "Add password reset" -i "Expire tokens after issue (implements REQ-pr1)" -i "Write the reset page" > "$T/ticket"
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
assigned_by = "conformance/plants.d/74-records.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "plant"
ROLES
w compile >/dev/null
g add -A; g commit -qm "password-reset records, a ticket, a Warrant that evaluates REQ-pr1"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$RP_TMP/setup.log" >&2
    exit 9
fi

rp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rp_war()  {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
        GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid \
        GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid \
        "$RP_WAR" --root "$RP_ROOT" "$@" </dev/null
}
rp_git()  { git -C "$RP_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
rp_line() { grep -m1 -E -- "$1" <<<"$2"; }
rp_reset() { rp_git checkout -q -- docs/records; command rm -f "$RP_ROOT"/docs/records/password-reset/2*.md; }
# The model's records as "id type revision" lines, sorted.
rp_revs() {
    rp_war model --json 2>/dev/null | python3 -c '
import json, sys
for r in json.load(sys.stdin)["result"]["records"]:
    if r["source"].startswith("docs/records/"):
        print(r["id"], r["type"], r["revision"])
' 2>&1
}
# One batch of whatever awaits, from a throwaway agent holding the plant key
# and nothing else, killed after.
rp_sign() {
    local old=${SSH_AUTH_SOCK:-} held want out
    unset SSH_AUTH_SOCK SSH_AGENT_PID
    eval "$(ssh-agent -s)" >/dev/null
    ssh-add -q "$RP_TMP/id_plant" 2>/dev/null
    want=$(ssh-keygen -lf "$RP_TMP/id_plant.pub" | awk '{print $2}')
    held=$(ssh-add -l 2>/dev/null)
    if [[ $(grep -c . <<<"$held") -ne 1 || "$(awk '{print $2}' <<<"$held")" != "$want" ]]; then
        printf 'PLANT SETUP FAILED: the agent holds another key\n' >&2
        ssh-agent -k >/dev/null 2>&1
        exit 9
    fi
    out=$("$RP_WAR" --root "$RP_ROOT" sign --batch --ssh-sign --as "Plant Signer" </dev/null 2>&1)
    ssh-agent -k >/dev/null 2>&1 || true
    if [[ -n "$old" ]]; then export SSH_AUTH_SOCK="$old"; else unset SSH_AUTH_SOCK; fi
    unset SSH_AGENT_PID
    printf '%s' "$out"
}
RP_REV0=$(cat "$RP_TMP/rev0")
RP_T=$(awk 'NR==1{print $1}' "$RP_TMP/ticket")
RP_I=$(sed -n 's/^- \[ \] Expire tokens after issue (implements REQ-pr1) (\(i-[0-9a-f]*\))$/\1/p' \
    "$RP_ROOT/docs/tickets/$RP_T/atoms/15-checklist.md")
[[ -n "$RP_T" && -n "$RP_I" ]] || { printf 'PLANT SETUP FAILED: no ticket item (%s)\n' "$(cat "$RP_TMP/ticket")" >&2; exit 9; }

# 1. The records parse: every heading a record of its type, every relation
#    line a relation, and `war check` names them well-formed.
RP_REVS=$(rp_revs)
RP_CHK=$(rp_war check 2>&1); RP_S=$?
RP_REL=$(rp_war model --json 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)["result"]
print(" ".join(sorted("%s>%s>%s" % (r["from"], r["kind"], r["to"]) for r in m["relations"]
    if r["from"] in ("REQ-pr1", "CON-pr1", "DEC-pr1") or r["kind"] in ("evaluates",) or r["from"].startswith("t-"))))
' 2>&1)
RP_WANT_REL="CON-pr1>constrains>REQ-pr1 DEC-pr1>constrains>REQ-pr1 DEC-pr1>selected_over>OPT-pr1 DEC-pr1>selected_over>OPT-pr2 REQ-pr1>implements>OUT-pr1 RP-WAR-0001/OBL-001>evaluates>REQ-pr1 $RP_T/$RP_I>implements>REQ-pr1 $RP_T/$RP_I>part_of>$RP_T"
RP_TYPES=$(awk '{print $1":"$2}' <<<"$RP_REVS" | sort | tr '\n' ' ')
if [[ $RP_S -eq 0 && "$RP_TYPES" == "CON-pr1:constraint DEC-pr1:decision OPT-pr1:option OPT-pr2:option OUT-pr1:outcome REQ-pr1:requirement " ]] \
    && [[ "$(tr ' ' '\n' <<<"$RP_REL" | grep -v '>part_of>' | sort | tr '\n' ' ')" == "$(tr ' ' '\n' <<<"$RP_WANT_REL" | grep -v '>part_of>' | sort | tr '\n' ' ')" ]] \
    && rp_line '^PASS records\.well-formed .*6 record\(s\) in 1 record atom\(s\), 7 core relation\(s\)' "$RP_CHK" >/dev/null; then
    rp_ok "the password-reset records parse" "6 records by type; implements, constrains, selected_over, evaluates and an item's implements as relations"
else
    rp_fail "the password-reset records parse" "check $RP_S; types '$RP_TYPES'; relations '$RP_REL'; $(rp_line 'records\.|record\.' "$RP_CHK")"
fi

# 2. One edit, one revision. REQ-pr1's text changes: its revision moves and
#    no other record's does. The title above the first record is no record's:
#    editing it moves none.
sed -i 's/A reset token expires 15 minutes after issue\./A reset token expires 10 minutes after issue./' "$RP_ROOT/$RP_REC"
RP_REVS2=$(rp_revs)
RP_MOVED=$(diff <(printf '%s\n' "$RP_REVS") <(printf '%s\n' "$RP_REVS2") | sed -n 's/^> \([^ ]*\) .*/\1/p' | tr '\n' ' ')
sed -i 's/^# Password reset$/# Password reset, retitled/' "$RP_ROOT/$RP_REC"
RP_REVS3=$(rp_revs)
rp_reset
if [[ "$RP_MOVED" == "REQ-pr1 " && -n "$RP_REVS2" && "$RP_REVS2" == "$RP_REVS3" ]]; then
    rp_ok "one edit moves one revision" "REQ-pr1's revision moved and no other; the title moved none"
else
    rp_fail "one edit moves one revision" "moved '$RP_MOVED'; title edit $( [[ "$RP_REVS2" == "$RP_REVS3" ]] && echo inert || echo moved)"
fi

# 3. Refused by rule: a duplicate id, an undeclared type, a kernel type, an
#    undeclared core kind, a word that is no kind, a requirement implementing
#    no outcome. Each in its own file beside the records; each checked alone.
RP_BAD=""; RP_NBAD=0
rp_refused() {  # <label> <rule> <message ERE> <record atom body>
    local out rc
    printf -- '---\nschema: oh.war/records/v1\nprofile: delivery\n---\n%s' "$4" \
        > "$RP_ROOT/docs/records/password-reset/20-plant.md"
    out=$(rp_war check 2>&1); rc=$?
    RP_NBAD=$((RP_NBAD + 1))
    if [[ $rc -eq 0 ]] || ! rp_line "^ERROR +$2 .*$3" "$out" >/dev/null; then
        RP_BAD="$RP_BAD $1(exit $rc: $(rp_line 'record\.' "$out"))"
    fi
    rp_reset
}
rp_refused duplicate-id 'record\.duplicate-id' 'REQ-pr1 is already declared at docs/records/password-reset/10-records\.md:' \
    $'## REQ-pr1 · requirement\nimplements OUT-pr1\n\nAgain.\n'
rp_refused undeclared-type 'record\.type-undeclared' 'RISK-pr1 · risk: profile delivery declares no record type `risk`' \
    $'## RISK-pr1 · risk\nA token leaks.\n'
rp_refused kernel-type 'record\.type-undeclared' '`item` is a kernel type' \
    $'## ITEM-pr1 · item\nNot here.\n'
rp_refused undeclared-kind 'record\.relation-undeclared' '`causes` is a core relation kind profile delivery does not allow' \
    $'## CON-pr2 · constraint\ncauses REQ-pr1\n'
rp_refused unknown-kind 'record\.relation-kind-unknown' '`mentions` is not a relation kind' \
    $'## CON-pr3 · constraint\nmentions REQ-pr1\n'
rp_refused required-relation 'record\.relation-required' 'REQ-pr9 · requirement: profile delivery requires every requirement to `implements` a record of type `outcome`' \
    $'## REQ-pr9 · requirement\nconstrains CON-pr1\n'
RP_OK_OUT=$(rp_war check 2>&1); RP_OK_S=$?
if [[ $RP_NBAD -eq 6 && -z "$RP_BAD" && $RP_OK_S -eq 0 ]] && ! rp_line '^ERROR +record\.' "$RP_OK_OUT" >/dev/null; then
    rp_ok "each undeclared thing refused" "6 by rule: duplicate id, undeclared type, kernel type, undeclared core kind, no kind, missing required relation; the records alone pass"
else
    rp_fail "each undeclared thing refused" "$RP_NBAD tried; not refused as named:$RP_BAD; alone: exit $RP_OK_S"
fi

# 4. A namespaced kind is carried and inert. OPT-pr1 gains `x.mentions` to
#    REQ-pr1 and to a target that exists nowhere: check, status and next
#    read byte for byte as before; the model carries both edges; impact does
#    not walk them.
RP_C0=$(rp_war check 2>&1); RP_ST0=$(rp_war status --json 2>&1); RP_N0=$(rp_war next 2>&1)
sed -i 's/^Random opaque tokens stored server-side, looked up on every use\.$/x.mentions REQ-pr1, NOPE-404\n\n&/' "$RP_ROOT/$RP_REC"
RP_C1=$(rp_war check 2>&1); RP_ST1=$(rp_war status --json 2>&1); RP_N1=$(rp_war next 2>&1)
RP_XM=$(rp_war model --json 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)["result"]
print(" ".join(sorted(r["to"] for r in m["relations"] if r["from"] == "OPT-pr1" and r["kind"] == "x.mentions")))
' 2>&1)
RP_IMP=$(rp_war impact REQ-pr1 --json 2>/dev/null | python3 -c '
import json, sys
i = json.load(sys.stdin)["result"]
print(" ".join(sorted(a["id"] for a in i["affected"])), "|", " ".join(sorted(r["from"] + ">" + r["kind"] for r in i["inert"])))
' 2>&1)
if [[ -n "$RP_C0" && "$RP_C0" == "$RP_C1" && "$RP_ST0" == "$RP_ST1" && "$RP_N0" == "$RP_N1" \
    && "$RP_XM" == "NOPE-404 REQ-pr1" && "$RP_IMP" == *"| OPT-pr1>x.mentions" && "$RP_IMP" != *"OPT-pr1 "*"|"* ]]; then
    rp_ok "a namespaced kind is inert" "carried (OPT-pr1 x.mentions REQ-pr1, NOPE-404); check, status, next unchanged; impact lists it inert, unwalked"
else
    rp_fail "a namespaced kind is inert" "check $( [[ "$RP_C0" == "$RP_C1" ]] && echo same || echo differs), status $( [[ "$RP_ST0" == "$RP_ST1" ]] && echo same || echo differs), next $( [[ "$RP_N0" == "$RP_N1" ]] && echo same || echo differs); model '$RP_XM'; impact '$RP_IMP'"
fi

# 5. The same edge with a core kind drives the check: an unknown target is a
#    warning naming it, the edge kept in the model, never dropped.
sed -i 's/^x\.mentions REQ-pr1, NOPE-404$/constrains NOPE-404/' "$RP_ROOT/$RP_REC"
RP_C2=$(rp_war check 2>&1); RP_S2=$?
RP_MD=$(rp_war model --json 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)["result"]
kept = any(r["from"] == "OPT-pr1" and r["kind"] == "constrains" and r["to"] == "NOPE-404" for r in m["relations"])
diag = [d["rule"] for d in m["diagnostics"] if "NOPE-404" in d["message"]]
print(kept, ",".join(diag))
' 2>&1)
rp_reset
if [[ $RP_S2 -eq 0 && "$RP_C2" != "$RP_C0" && "$RP_MD" == "True model.relation-target-unknown" ]] \
    && rp_line '^WARN +record\.relation-target-unknown .*OPT-pr1 constrains NOPE-404: NOPE-404 is not a record of this corpus; the relation is kept' "$RP_C2" >/dev/null; then
    rp_ok "an unknown target is a diagnostic" "record.relation-target-unknown (a warning) and model.relation-target-unknown; the edge kept"
else
    rp_fail "an unknown target is a diagnostic" "check $RP_S2: $(rp_line 'NOPE-404' "$RP_C2"); model '$RP_MD'"
fi

# 6. The Warrant is authorized by the throwaway signer and verified by the
#    fixture verifier: OBL-001's verdict is recorded against REQ-pr1 as it
#    was. Before any change, impact reads it current.
rp_sign >/dev/null
[[ -f "$RP_ROOT/docs/warrants/RP-WAR-0001/authorization.toml" ]] || { printf 'PLANT SETUP FAILED: RP-WAR-0001 was not authorized\n' >&2; exit 9; }
rp_git add -A >/dev/null 2>&1; rp_git commit -qm "authorized" >/dev/null 2>&1
RP_PREP=$(rp_war prepare RP-WAR-0001 2>&1)
rp_git add -A >/dev/null 2>&1; rp_git commit -qm "verified" >/dev/null 2>&1
rp_impact() {
    rp_war impact REQ-pr1 --json 2>/dev/null | python3 -c '
import json, sys
i = json.load(sys.stdin)["result"]
for e in i["evaluations"]:
    print("eval", e["obligation"], e["verdict"], e["reads"], e.get("bound_revision", "-"), e["current_revision"])
for a in i["affected"]:
    print("affected", a["id"], a["via"]["kind"])
for d in i["documents"]:
    for b in d["because"]:
        print("doc", d["id"], b)
for p in i["phases"]:
    print("phase", p["id"], ",".join(p["via"]))
for p in i["projections"]:
    print("proj", p["path"])
' 2>&1
}
RP_I0=$(rp_impact)
if rp_line "^eval RP-WAR-0001/OBL-001 established current $RP_REV0 $RP_REV0\$" "$RP_I0" >/dev/null; then
    rp_ok "a verdict on this revision is current" "OBL-001 established, bound to REQ-pr1's revision now"
else
    rp_fail "a verdict on this revision is current" "$(rp_line '^eval' "$RP_I0"); prepare: $(rp_line '(prepared|ERROR)' "$RP_PREP")"
fi

# 7. REQ-pr1 changes from 15 to 10 minutes. Impact names the item that
#    implements it, the Warrant whose basis names it, OBL-001 with its verdict
#    still recorded and bound to the old revision (stale), the phase, and the
#    Warrant's generated views. Nothing the verification wrote moves.
sed -i 's/A reset token expires 15 minutes after issue\./A reset token expires 10 minutes after issue./' "$RP_ROOT/$RP_REC"
RP_I1=$(rp_impact)
RP_H1=$(rp_war impact REQ-pr1 2>&1); RP_HS=$?
RP_REV1=$(rp_revs | awk '$1 == "REQ-pr1" {print $3}')
RP_DIRTY=$(git -C "$RP_ROOT" status --porcelain | tr '\n' ' ')
RP_MISS=""
for want in \
    "^affected $RP_T/$RP_I implements\$" \
    "^affected RP-WAR-0001/OBL-001 evaluates\$" \
    "^doc RP-WAR-0001 names REQ-pr1 in atoms/20-basis\.md\$" \
    "^doc $RP_T holds $RP_T/$RP_I\$" \
    "^eval RP-WAR-0001/OBL-001 established stale $RP_REV0 $RP_REV1\$" \
    "^phase RP-PHASE-1 RP-WAR-0001\$" \
    "^proj docs/warrants/RP-WAR-0001/generated/WAR\.md\$" \
    "^proj docs/warrants/generated/CORPUS_STATUS\.json\$"; do
    rp_line "$want" "$RP_I1" >/dev/null || RP_MISS="$RP_MISS [$want]"
done
if [[ -z "$RP_MISS" && "$RP_REV1" != "$RP_REV0" && $RP_HS -eq 0 && "$RP_DIRTY" == " M $RP_REC " ]] \
    && rp_line '^WARN +impact\.evaluation-stale .*The verdict \(established\) stays recorded' "$RP_H1" >/dev/null; then
    rp_ok "impact names what a change affects" "the item, the Warrant's basis, OBL-001 established but stale, RP-PHASE-1, WAR.md and the corpus views; only the record file changed"
else
    rp_fail "impact names what a change affects" "missing:$RP_MISS; dirty '$RP_DIRTY'; human exit $RP_HS"
fi

# 8. An id that is no record is refused by name.
RP_U=$(rp_war impact REQ-nope 2>&1); RP_US=$?
RP_UJ=$(rp_war impact REQ-nope --json 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print(v["exit_code"], v.get("result"), [d["rule"] for d in v["diagnostics"]])
' 2>&1)
if [[ $RP_US -ne 0 ]] && rp_line '^ERROR +impact\.unknown-record +REQ-nope is not a record of this corpus' "$RP_U" >/dev/null \
    && [[ "$RP_UJ" == *"None ['impact.unknown-record']" && "$RP_UJ" != 0* ]]; then
    rp_ok "impact on an unknown id" "refused: impact.unknown-record, naming REQ-nope; no result"
else
    rp_fail "impact on an unknown id" "exit $RP_US: $(head -c 200 <<<"$RP_U"); json '$RP_UJ'"
fi

command rm -rf "$RP_TMP"
corpus_gone "$RP_ROOT"
unset RP_ROOT RP_TMP RP_WAR RP_REC RP_REV0 RP_T RP_I
