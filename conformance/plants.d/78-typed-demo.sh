# shellcheck shell=bash
# OW-WAR-0148 M5 (OBL-007, OBL-011) — the password-reset demonstration: one
# set of records, every output that reads them, and what a change affects.
#
# One scratch program (DM) with the committed profiles (delivery, decision,
# ticket, and the document types prd, architecture, test-plan, agent-packet),
# this repository's password-reset area (docs/records/password-reset: its
# record atoms and documents.toml), and a two-phase roadmap record. From the
# one set of records:
#   - a ticket, `war create --implements REQ-pr1 --implements CON-pr1`: one
#     item per record, its text the record's own first sentence;
#   - a Warrant draft, DM-WAR-0002 (`war new`), whose basis names REQ-pr1,
#     CON-pr1 and DEC-pr1 and whose OBL-001 `evaluates` REQ-pr1 pinned at its
#     revision now, placed in DM-PHASE-2 (`war roadmap assign`);
#   - the roadmap view (`war roadmap`): DM-WAR-0002 in DM-PHASE-2;
#   - the declared projections `war compile` writes: PRD, architecture, test
#     plan and the agent packet for REQ-pr1.
#
# Accepted: every output above exists and names the records; after REQ-pr1
# changes, `war impact REQ-pr1` names every affected output — both ticket
# items, the Warrant, OBL-001 (stale, bound to the old revision), DM-PHASE-2,
# and exactly the declared projections that select REQ-pr1 — and `war
# compile` then moves exactly those projections' bytes and nothing else.
# Refused, each by name: `--implements` naming no record (nothing created),
# and `war impact` on an id that is no record. The architecture view, which
# names REQ-pr1 only by id, is neither named nor moved.

echo "== the typed-records demonstration (OW-WAR-0148 M5) =="
DM_ROOT=$(scratch_corpus DM)
[[ -d "${DM_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
DM_TMP=$(mktemp -d)
DM_WAR="$REPO_ROOT/${WAR#./}"
DM_AREA="docs/records/password-reset"
DM_GEN="$DM_AREA/generated"

if ! WAR="$DM_WAR" D="$DM_ROOT" T="$DM_TMP" R="$REPO_ROOT" AREA="$DM_AREA" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$DM_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w() { "$WAR" --root . "$@" </dev/null; }
mkdir -p profiles "$AREA" docs/roadmap/atoms
for p in delivery decision ticket prd architecture test-plan agent-packet; do
    cp "$R/profiles/$p.toml" profiles/
done
for f in 10-records.md 20-product.md 30-architecture.md documents.toml; do
    cp "$R/$AREA/$f" "$AREA/$f"
done
cat > docs/roadmap/roadmap.toml <<'TOML'
schema = "oh.war/roadmap/v1"
uuid = "01a0cd26-0000-7000-8000-0000000000d1"
program = "Plant Corpus DM"
prefix = "DM"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
TOML
cat > docs/roadmap/atoms/20-phases.yaml <<'YAML'
schema: "oh.war/roadmap-phases/v1"

phases:
  - id: "DM-PHASE-1"
    title: "Adopt"
    exit: "it is adopted"
    depends_on: []
  - id: "DM-PHASE-2"
    title: "Password reset"
    exit: "a user resets a forgotten password without support"
    depends_on: ["DM-PHASE-1"]
YAML
w model --json > "$T/m0.json"
python3 - "$T/m0.json" > "$T/rev0" <<'PY'
import json, sys
print([r["revision"] for r in json.load(open(sys.argv[1]))["result"]["records"] if r["id"] == "REQ-pr1"][0])
PY
# The Warrant draft: names the records in its basis, evaluates REQ-pr1.
w new "Expire password reset tokens" > /dev/null
test -d docs/warrants/DM-WAR-0002
w roadmap assign DM-WAR-0002 DM-PHASE-2/reset > /dev/null
python3 - docs/warrants/DM-WAR-0002/atoms/60-assurance.md docs/warrants/DM-WAR-0002/atoms/20-basis.md "$(cat "$T/rev0")" <<'PY'
import sys
a, b, rev = sys.argv[1:]
s = open(a).read()
head = "## Gate Adequacy\n"
assert s.count(head) == 1, "no Gate Adequacy heading"
obl = ("### OBL-001 — a reset token expires when REQ-pr1 says\n\n"
       "- **evaluates:** REQ-pr1@" + rev + "\n"
       "- **scope:** the reset endpoint's token check, over the tokens its tests issue.\n"
       "- **gate:** `gate://software.repo.war-check@1.0.0`\n"
       "- **evidence:** a token used just inside the window is accepted; just past it, refused.\n\n")
open(a, "w").write(s.replace(head, obl + head))
s = open(b).read()
head = "## Decisions relied on\n"
assert s.count(head) == 1, "no Decisions heading"
open(b, "w").write(s.replace(head, "Implements REQ-pr1, within CON-pr1 and DEC-pr1 (docs/records/password-reset/).\n\n" + head))
PY
# The ticket: one item per record it implements.
w --json create "Expire reset tokens" --type feature --implements REQ-pr1 --implements CON-pr1 > "$T/ticket.json"
w compile > /dev/null
g add -A; g commit -qm "password-reset records, a ticket, a Warrant draft in a phase, the projections"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$DM_TMP/setup.log" >&2
    exit 9
fi

dm_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
dm_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
dm_war()  { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$DM_WAR" --root "$DM_ROOT" "$@" </dev/null; }
dm_git()  { git -C "$DM_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
dm_has()  { grep -qF -- "$1" <<<"$2"; }
dm_line() { grep -qE -- "$1" <<<"$2"; }
DM_T=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$DM_TMP/ticket.json" 2>/dev/null)
DM_I1=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["items"][0]["id"])' "$DM_TMP/ticket.json" 2>/dev/null)
DM_I2=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["items"][1]["id"])' "$DM_TMP/ticket.json" 2>/dev/null)
DM_REV0=$(cat "$DM_TMP/rev0")

# 1. One set of records, every output. The ticket's items carry the records'
#    own sentences and implement them; the Warrant draft names them and sits
#    in DM-PHASE-2 on the roadmap view; the four declared projections are
#    compiled and current. Refused: `--implements` naming no record, nothing
#    created.
DM_SHOW=$(dm_war show "$DM_T" 2>/dev/null)
DM_RM=$(dm_war roadmap --json 2>/dev/null | python3 -c '
import json, sys
for p in json.load(sys.stdin)["result"]["phases"]:
    print(p["id"], ",".join(p["members"]))
' 2>&1)
DM_WARMD=$(cat "$DM_ROOT/docs/warrants/DM-WAR-0002/generated/WAR.md" 2>/dev/null)
DM_GENS=$(cd "$DM_ROOT/$DM_GEN" 2>/dev/null && ls | tr '\n' ' ')
DM_CG=$(dm_war check --generated 2>&1); DM_SG=$?
DM_N0=$(find "$DM_ROOT/docs/tickets" -name manifest.toml | wc -l)
DM_ERR=$(dm_war create "Nothing behind it" --implements REQ-nope 2>&1 >/dev/null); DM_SE=$?
DM_N1=$(find "$DM_ROOT/docs/tickets" -name manifest.toml | wc -l)
if [[ -n "$DM_T" && $DM_SG -eq 0 && "$DM_GENS" == "agent-packet.md architecture.md prd.md test-plan.md " \
    && $DM_SE -eq 2 && "$DM_N0" == "$DM_N1" ]] \
    && dm_has "- [ ] A reset token expires 15 minutes after issue (implements REQ-pr1) ($DM_I1)" "$DM_SHOW" \
    && dm_has "- [ ] Tokens are single-use and stored hashed (implements CON-pr1) ($DM_I2)" "$DM_SHOW" \
    && dm_line '^DM-PHASE-2 DM-WAR-0002$' "$DM_RM" \
    && dm_has "Implements REQ-pr1, within CON-pr1 and DEC-pr1" "$DM_WARMD" \
    && dm_has 'ticket.record-unknown' "$DM_ERR" && dm_has 'REQ-nope' "$DM_ERR"; then
    dm_ok "one set of records, every output" "ticket $DM_T (items implement REQ-pr1, CON-pr1), DM-WAR-0002 in DM-PHASE-2, prd/architecture/test-plan/agent-packet current; --implements REQ-nope refused"
else
    dm_fail "one set of records, every output" "ticket '$DM_T'; generated '$DM_GENS' check $DM_SG; roadmap '$DM_RM'; refusal $DM_SE '$DM_ERR' ($DM_N0/$DM_N1); show: $(head -c 300 <<<"$DM_SHOW")"
fi

# 2. REQ-pr1 changes from 15 to 10 minutes. `war impact REQ-pr1` names every
#    affected output: both ticket items (one directly, one through CON-pr1),
#    the Warrant draft, OBL-001 stale against the old revision, DM-PHASE-2,
#    and the declared projections that select REQ-pr1.
sed -i 's/A reset token expires 15 minutes after issue\./A reset token expires 10 minutes after issue./' "$DM_ROOT/$DM_AREA/10-records.md"
DM_IMP=$(dm_war impact REQ-pr1 --json 2>/dev/null | python3 -c '
import json, sys
i = json.load(sys.stdin)["result"]
for a in i["affected"]:
    print("affected", a["id"], a["via"]["kind"], a["via"]["to"])
for d in i["documents"]:
    print("doc", d["id"], d["kind"])
for e in i["evaluations"]:
    print("eval", e["obligation"], e["reads"], e.get("bound_revision", "-"))
for p in i["phases"]:
    print("phase", p["id"], ",".join(p["via"]))
print("declared", " ".join(sorted(p["path"].rsplit("/", 1)[1] for p in i["projections"] if p["scope"] == "declared")))
for p in i["projections"]:
    print("proj", p["scope"], p["path"])
' 2>&1)
DM_MISS=""
for want in \
    "^affected $DM_T/$DM_I1 implements REQ-pr1\$" \
    "^affected $DM_T/$DM_I2 implements CON-pr1\$" \
    "^affected DM-WAR-0002/OBL-001 evaluates REQ-pr1\$" \
    "^doc $DM_T ticket\$" \
    "^doc DM-WAR-0002 warrant\$" \
    "^eval DM-WAR-0002/OBL-001 stale $DM_REV0\$" \
    "^phase DM-PHASE-2 DM-WAR-0002\$" \
    "^declared agent-packet\.md prd\.md test-plan\.md\$" \
    "^proj warrant docs/warrants/DM-WAR-0002/generated/WAR\.md\$"; do
    dm_line "$want" "$DM_IMP" || DM_MISS="$DM_MISS [$want]"
done
if [[ -z "$DM_MISS" ]] && ! dm_line '/generated/architecture\.md' "$DM_IMP"; then
    dm_ok "impact names every affected output" "items $DM_I1 and $DM_I2, DM-WAR-0002 and OBL-001 (stale), DM-PHASE-2, prd/test-plan/agent-packet; architecture not named"
else
    dm_fail "impact names every affected output" "missing:$DM_MISS; got: $(tr '\n' ';' <<<"$DM_IMP" | head -c 700)"
fi

# 3. `war compile` moves exactly the projections impact named as selecting
#    REQ-pr1, and nothing else: the architecture view, the Warrant's own
#    views and the ticket keep their bytes. Refused: impact on no record.
dm_war compile >/dev/null 2>&1; DM_SC=$?
DM_MOVED=$(dm_git status --porcelain | awk '{print $2}' | sort | tr '\n' ' ')
DM_WANT="$DM_AREA/10-records.md $DM_GEN/agent-packet.md $DM_GEN/prd.md $DM_GEN/test-plan.md "
DM_CG2=$(dm_war check --generated 2>&1); DM_SG2=$?
DM_U=$(dm_war impact REQ-nope 2>&1); DM_SU=$?
if [[ $DM_SC -eq 0 && "$DM_MOVED" == "$DM_WANT" && $DM_SG2 -eq 0 && $DM_SU -ne 0 ]] \
    && dm_has "A reset token expires 10 minutes after issue." "$(cat "$DM_ROOT/$DM_GEN/prd.md")" \
    && dm_line 'impact\.unknown-record +REQ-nope is not a record of this corpus' "$DM_U"; then
    dm_ok "a change moves exactly its outputs" "compile moved prd, test-plan, agent-packet (and the record); architecture, WAR.md, the ticket unchanged; impact REQ-nope refused"
else
    dm_fail "a change moves exactly its outputs" "compile $DM_SC; moved '$DM_MOVED' want '$DM_WANT'; check $DM_SG2 $(grep -m1 '^ERROR' <<<"$DM_CG2"); unknown $DM_SU"
fi

command rm -rf "$DM_TMP"
corpus_gone "$DM_ROOT"
unset DM_ROOT DM_TMP DM_WAR DM_AREA DM_GEN DM_T DM_I1 DM_I2 DM_REV0 DM_SHOW DM_RM DM_WARMD DM_GENS DM_CG \
    DM_SG DM_N0 DM_N1 DM_ERR DM_SE DM_IMP DM_MISS DM_MOVED DM_WANT DM_CG2 DM_SG2 DM_SC DM_U DM_SU
