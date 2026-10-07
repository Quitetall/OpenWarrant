# shellcheck shell=bash
# OW-WAR-0148 M6 — projection targets: one set of records, many documents
# (OW-ADR-0031).
#
# One scratch program (PJ) with the committed profiles (delivery, decision,
# ticket, and the document types prd, architecture, test-plan, agent-packet)
# and this repository's password-reset area (docs/records/password-reset: its
# record atoms and documents.toml). PJ-WAR-0001's OBL-001 `evaluates` REQ-pr1,
# pinned at the revision it has when the corpus is built.
#
# Accepted: `war compile` renders the four declared documents from the same
# records, and `war render` gives the same bytes and writes nothing; every
# line of a rendering traces to a record id and revision (or to the template
# or the document); changing REQ-pr1 changes exactly the projections that
# select it, and `war impact REQ-pr1` names exactly those.
# Refused, each by rule: a projection naming an undeclared record type, an
# undeclared relation kind or a namespaced one, and a document type selecting
# an authority capability (profile.projection, profile.capabilities); a
# hand-edit to a generated projection, and a file no document produces
# (projection.drift); an agent packet over its budget (projection.over-budget),
# with nothing written or truncated; an unknown projection (projection.unknown).

echo "== projection targets (OW-WAR-0148 M6) =="
PJ_ROOT=$(scratch_corpus PJ)
[[ -d "${PJ_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
PJ_TMP=$(mktemp -d)
PJ_WAR="$REPO_ROOT/${WAR#./}"
PJ_AREA="docs/records/password-reset"
PJ_GEN="$PJ_AREA/generated"

if ! WAR="$PJ_WAR" D="$PJ_ROOT" T="$PJ_TMP" R="$REPO_ROOT" AREA="$PJ_AREA" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$PJ_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w() { "$WAR" --root . "$@"; }
mkdir -p profiles "$AREA"
for p in delivery decision ticket prd architecture test-plan agent-packet; do
    cp "$R/profiles/$p.toml" profiles/
done
for f in 10-records.md 20-product.md 30-architecture.md documents.toml; do
    cp "$R/$AREA/$f" "$AREA/$f"
done
w model --json > "$T/m0.json"
python3 - "$T/m0.json" > "$T/rev0" <<'PY'
import json, sys
print([r["revision"] for r in json.load(open(sys.argv[1]))["result"]["records"] if r["id"] == "REQ-pr1"][0])
PY
python3 - docs/warrants/PJ-WAR-0001/atoms/60-assurance.md "$(cat "$T/rev0")" <<'PY'
import sys
p, rev = sys.argv[1:]
s = open(p).read()
head = "### OBL-001 — the SAS is accepted and pinned\n"
assert s.count(head) == 1, "no OBL-001 heading"
s = s.replace(head, head + "- **evaluates:** REQ-pr1@" + rev + "\n")
open(p, "w").write(s)
PY
w compile >/dev/null
g add -A; g commit -qm "password-reset records, documents and their projections"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$PJ_TMP/setup.log" >&2
    exit 9
fi

pj_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
pj_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
pj_war()  { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$PJ_WAR" --root "$PJ_ROOT" "$@" </dev/null; }
pj_git()  { git -C "$PJ_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
pj_line() { grep -m1 -E -- "$1" <<<"$2"; }
pj_has()  { grep -qF -- "$1" <<<"$2"; }
pj_reset() { pj_git checkout -q -- .; pj_git clean -qfd -- docs profiles; }
pj_changed() {  # the generated files git sees as changed, by name, sorted
    pj_git status --porcelain -- "$PJ_GEN" | awk '{print $2}' | sed 's|.*/||' | sort | tr '\n' ' '
}

# 1. Four documents from one set of records. `war compile` wrote them; each
#    says what its type says; `war render` gives the same bytes and writes
#    nothing. Refused: a projection no document type declares.
PJ_PRD=$(cat "$PJ_ROOT/$PJ_GEN/prd.md" 2>/dev/null)
PJ_ARC=$(cat "$PJ_ROOT/$PJ_GEN/architecture.md" 2>/dev/null)
PJ_TST=$(cat "$PJ_ROOT/$PJ_GEN/test-plan.md" 2>/dev/null)
PJ_PKT=$(cat "$PJ_ROOT/$PJ_GEN/agent-packet.md" 2>/dev/null)
pj_war render prd --of password-reset/prd > "$PJ_TMP/prd.out" 2>"$PJ_TMP/prd.err"; PJ_S1=$?
pj_war render prd > "$PJ_TMP/prd.default" 2>/dev/null; PJ_S2=$?
PJ_DIRTY=$(pj_git status --porcelain)
PJ_UNK=$(pj_war render roadmap-poster --of REQ-pr1 2>&1 >"$PJ_TMP/unk.out"); PJ_S3=$?
PJ_BAD=""
pj_has "> A user who forgot their password regains access without support." "$PJ_PRD" || PJ_BAD="$PJ_BAD prd:outcome"
pj_has "- **REQ-pr1.** A reset token expires 15 minutes after issue. *Implements OUT-pr1; constrained by CON-pr1.*" "$PJ_PRD" || PJ_BAD="$PJ_BAD prd:requirement"
pj_has "- **NG-pr1.** Recovering an account" "$PJ_PRD" || PJ_BAD="$PJ_BAD prd:non-goal"
pj_has "**Alternatives considered**" "$PJ_ARC" || PJ_BAD="$PJ_BAD arch:alternatives"
pj_has "- **OPT-pr1.** Random opaque tokens" "$PJ_ARC" || PJ_BAD="$PJ_BAD arch:option"
pj_has "| IF-pr2 | \`POST /password-reset/confirm\`" "$PJ_ARC" || PJ_BAD="$PJ_BAD arch:interface"
pj_has "| REQ-pr1 | requirement | A reset token expires 15 minutes after issue. | PJ-WAR-0001/OBL-001 |" "$PJ_TST" || PJ_BAD="$PJ_BAD test:coverage"
pj_has "judged REQ-pr1 at its current revision" "$PJ_TST" || PJ_BAD="$PJ_BAD test:pin"
pj_has "## Provenance" "$PJ_PKT" || PJ_BAD="$PJ_BAD packet:provenance"
pj_has "\`requirement · docs/records/password-reset/10-records.md:18 · revision " "$PJ_PKT" || PJ_BAD="$PJ_BAD packet:line"
if [[ -z "$PJ_BAD" && $PJ_S1 -eq 0 && $PJ_S2 -eq 0 && -z "$PJ_DIRTY" ]] \
    && cmp -s "$PJ_TMP/prd.out" "$PJ_ROOT/$PJ_GEN/prd.md" \
    && cmp -s "$PJ_TMP/prd.default" "$PJ_ROOT/$PJ_GEN/prd.md" \
    && [[ $PJ_S3 -ne 0 && ! -s "$PJ_TMP/unk.out" ]] && pj_line 'projection\.unknown.*roadmap-poster is no projection' "$PJ_UNK" >/dev/null; then
    pj_ok "four documents from one set" "prd, architecture, test-plan and agent-packet compiled; render = compiled bytes, writes nothing; an unknown projection refused"
else
    pj_fail "four documents from one set" "missing:$PJ_BAD; render $PJ_S1/$PJ_S2; dirty '$PJ_DIRTY'; unknown $PJ_S3: $PJ_UNK"
fi

# 2. Every line traces to a record id and revision, or to the template or the
#    document. The test plan selects REQ-pr1 at its model revision; the
#    architecture view names REQ-pr1 only by id: mentioned, not selected.
PJ_REV0=$(cat "$PJ_TMP/rev0")
PJ_TRACE=$(pj_war render test-plan --of password-reset/test-plan --json 2>/dev/null \
    | python3 -c '
import json, sys
p = json.load(sys.stdin)["result"]
lines = p["content"].count("\n")
nxt = 1
for t in p["trace"]:
    assert t["start_line"] == nxt and t["end_line"] >= t["start_line"], t
    assert t["origin"] in ("record", "template", "document"), t
    nxt = t["end_line"] + 1
assert nxt == lines + 1, (nxt, lines)
req = [t for t in p["trace"] if t["id"] == "REQ-pr1"]
print("covered", lines, "REQ-pr1", req[0]["revision"], " ".join(s["id"] for s in p["selects"]))
' 2>&1)
PJ_ARCJ=$(pj_war render architecture --of password-reset/architecture --json 2>/dev/null \
    | python3 -c '
import json, sys
p = json.load(sys.stdin)["result"]
print("selects-REQ" if any(s["id"] == "REQ-pr1" for s in p["selects"]) else "not-selected", "mentions", " ".join(p["mentions"]))
' 2>&1)
if [[ "$PJ_TRACE" == "covered "*" REQ-pr1 $PJ_REV0 REQ-pr1 CON-pr1 PJ-WAR-0001/OBL-001" \
    && "$PJ_ARCJ" == "not-selected mentions REQ-pr1" ]]; then
    pj_ok "every line traces to its record" "test plan: lines covered, REQ-pr1 at its model revision; architecture mentions REQ-pr1, selects it not"
else
    pj_fail "every line traces to its record" "test-plan '$PJ_TRACE'; architecture '$PJ_ARCJ'"
fi

# 3. Change REQ-pr1. Exactly the projections that select it change (PRD, test
#    plan, agent packet), and `war impact REQ-pr1` names exactly those; the
#    architecture view, which names REQ-pr1 only by id, does not move and is
#    not named. The test plan says OBL-001 judged an earlier revision.
sed -i 's/A reset token expires 15 minutes after issue\./A reset token expires 10 minutes after issue./' "$PJ_ROOT/$PJ_AREA/10-records.md"
pj_war compile >/dev/null 2>&1; PJ_S=$?
PJ_MOVED=$(pj_changed)
PJ_IMP=$(pj_war impact REQ-pr1 --json 2>/dev/null | python3 -c '
import json, sys
i = json.load(sys.stdin)["result"]
print(" ".join(sorted(p["path"].rsplit("/", 1)[1] for p in i["projections"] if p["scope"] == "declared")))
' 2>&1)
PJ_STALE=$(cat "$PJ_ROOT/$PJ_GEN/test-plan.md")
PJ_GENCHK=$(pj_war check --generated 2>&1)
if [[ $PJ_S -eq 0 && "$PJ_MOVED" == "agent-packet.md prd.md test-plan.md " \
    && "$PJ_IMP" == "agent-packet.md prd.md test-plan.md" ]] \
    && pj_has "judged REQ-pr1 at an earlier revision (" "$PJ_STALE" \
    && ! pj_line '^ERROR' "$PJ_GENCHK" >/dev/null; then
    pj_ok "a change moves exactly its projections" "REQ-pr1 changed: prd, test-plan, agent-packet moved and impact names them; architecture neither"
else
    pj_fail "a change moves exactly its projections" "compile $PJ_S; moved '$PJ_MOVED'; impact '$PJ_IMP'; $(pj_line '^ERROR' "$PJ_GENCHK")"
fi
pj_reset

# 4. A projection that names an undeclared record type, an undeclared or
#    namespaced relation kind, and a document type that selects authority
#    are each refused by rule, before anything is read. The shipped types
#    are accepted.
PJ_OK=$(pj_war check 2>&1); PJ_SOK=$?
PJ_BAD=""
pj_profile() {  # <label> <rule> <message ERE> <python replace: old> <new>
    local out rc
    python3 - "$PJ_ROOT/profiles/prd.toml" "$4" "$5" <<'PY'
import sys
p, old, new = sys.argv[1:]
s = open(p).read()
assert s.count(old) >= 1, old
open(p, "w").write(s.replace(old, new, 1))
PY
    out=$(pj_war check 2>&1); rc=$?
    if [[ $rc -eq 0 ]] || ! pj_line "$2: profiles/prd\.toml: .*$3" "$out" >/dev/null; then
        PJ_BAD="$PJ_BAD $1(exit $rc: $(pj_line 'profile\.' "$out"))"
    fi
    pj_reset
}
pj_profile undeclared-type 'profile\.projection' 'record type `risk` is not declared by this profile' \
    'types = ["non_goal"]' 'types = ["risk"]'
pj_profile undeclared-kind 'profile\.projection' '`in:supersedes` names the relation kind `supersedes`, which this profile does not allow' \
    'annotate = ["out:constrains"]' 'annotate = ["in:supersedes"]'
pj_profile namespaced-kind 'profile\.projection' 'walks the namespaced kind `x\.mentions`' \
    'annotate = ["out:constrains"]' 'annotate = ["out:x.mentions"]'
pj_profile authority 'profile\.capabilities' '`authorization` is not among them' \
    'capabilities = ["structure", "links"]' 'capabilities = ["structure", "links", "authorization"]'
if [[ -z "$PJ_BAD" && $PJ_SOK -eq 0 ]] \
    && pj_line '^PASS +documents\.well-formed +4 document\(s\) in 1 area\(s\)' "$PJ_OK" >/dev/null; then
    pj_ok "undeclared types and kinds refused" "risk, in:supersedes, x.mentions refused (profile.projection); authorization refused (profile.capabilities); the shipped four accepted"
else
    pj_fail "undeclared types and kinds refused" "shipped: exit $PJ_SOK $(pj_line 'documents\.' "$PJ_OK");$PJ_BAD"
fi

# 5. A hand-edit to a generated projection is drift, and so is a file no
#    document produces; the committed projections pass.
PJ_CLEAN=$(pj_war check --generated 2>&1)
printf '\nAlso supports magic links.\n' >> "$PJ_ROOT/$PJ_GEN/prd.md"
PJ_EDIT=$(pj_war check --generated 2>&1); PJ_SE=$?
pj_reset
printf '# Notes\n' > "$PJ_ROOT/$PJ_GEN/notes.md"
PJ_ORPH=$(pj_war check --generated 2>&1); PJ_SO=$?
pj_reset
if pj_line '^PASS +projection\.drift +docs/records/password-reset/generated/prd\.md matches a fresh rendering' "$PJ_CLEAN" >/dev/null \
    && [[ $PJ_SE -ne 0 && $PJ_SO -ne 0 ]] \
    && pj_line '^ERROR +projection\.drift +the committed docs/records/password-reset/generated/prd\.md differs .*edited by hand' "$PJ_EDIT" >/dev/null \
    && pj_line '^ERROR +projection\.drift +docs/records/password-reset/generated/notes\.md is produced by no declared document' "$PJ_ORPH" >/dev/null; then
    pj_ok "a hand-edit is drift" "prd.md edited: projection.drift; a stray notes.md: projection.drift; committed: pass"
else
    pj_fail "a hand-edit is drift" "clean $(pj_line 'projection\.' "$PJ_CLEAN"); edit $PJ_SE $(pj_line 'projection\.' "$PJ_EDIT"); orphan $PJ_SO $(pj_line 'notes' "$PJ_ORPH")"
fi

# 6. An agent packet over its budget is refused by name: `war compile` writes
#    nothing for it (nothing truncated), `war render` prints nothing, `war
#    check --generated` says why. Within budget it renders.
python3 - "$PJ_ROOT/$PJ_AREA/documents.toml" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = 'roots = ["REQ-pr1"]\n'
assert s.count(old) == 1
open(p, "w").write(s.replace(old, old + "max_bytes = 600\n"))
PY
PJ_COMP=$(pj_war compile 2>&1); PJ_SC=$?
PJ_UNTOUCHED=$(pj_changed)
pj_war render agent-packet --of password-reset/agent-packet > "$PJ_TMP/pkt.out" 2>"$PJ_TMP/pkt.err"; PJ_SR=$?
PJ_RERR=$(cat "$PJ_TMP/pkt.err")
PJ_CG=$(pj_war check --generated 2>&1)
pj_war render agent-packet --of password-reset/agent-packet --max-bytes 100000 > "$PJ_TMP/pkt.big" 2>/dev/null; PJ_SB=$?
pj_reset
if [[ $PJ_SC -ne 0 && -z "$PJ_UNTOUCHED" && $PJ_SR -ne 0 && ! -s "$PJ_TMP/pkt.out" && $PJ_SB -eq 0 ]] \
    && pj_line 'projection\.over-budget: password-reset/agent-packet: .* over its budget of 600 bytes; nothing is truncated' "$PJ_COMP" >/dev/null \
    && pj_line 'projection\.over-budget +projection agent-packet of password-reset/agent-packet is [0-9]+ bytes, over its budget of 600 bytes.*cost the most: [A-Za-z0-9/-]+ \([0-9]+ bytes\)' "$PJ_RERR" >/dev/null \
    && pj_line '^ERROR +projection\.compile +projection\.over-budget: password-reset/agent-packet: ' "$PJ_CG" >/dev/null \
    && cmp -s "$PJ_TMP/pkt.big" "$PJ_ROOT/$PJ_GEN/agent-packet.md"; then
    pj_ok "an agent packet over budget is refused" "600 bytes: compile and render refuse by name, nothing written or truncated; within budget it renders"
else
    pj_fail "an agent packet over budget is refused" "compile $PJ_SC '$(pj_line 'over-budget' "$PJ_COMP")'; changed '$PJ_UNTOUCHED'; render $PJ_SR '$PJ_RERR'; check '$(pj_line 'projection\.' "$PJ_CG")'; within $PJ_SB"
fi

command rm -rf "$PJ_TMP"
unset PJ_ROOT PJ_TMP PJ_WAR PJ_AREA PJ_GEN PJ_PRD PJ_ARC PJ_TST PJ_PKT PJ_S PJ_S1 PJ_S2 PJ_S3 \
    PJ_DIRTY PJ_UNK PJ_BAD PJ_REV0 PJ_TRACE PJ_ARCJ PJ_MOVED PJ_IMP PJ_STALE PJ_GENCHK PJ_OK \
    PJ_SOK PJ_CLEAN PJ_EDIT PJ_SE PJ_ORPH PJ_SO PJ_COMP PJ_SC PJ_UNTOUCHED PJ_SR PJ_RERR PJ_CG PJ_SB
