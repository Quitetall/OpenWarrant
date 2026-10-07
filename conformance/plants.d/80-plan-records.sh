# shellcheck shell=bash
# OW-WAR-0148 M7 — from a sentence to typed records (OW-ADR-0031).
#
# One scratch program (PL) with the committed delivery, decision and ticket
# profiles and this repository's password-reset records
# (docs/records/password-reset/10-records.md), and a fixture drafter
# (conformance/fixtures/drafter/records-proposal.sh) whose
# oh.war/records-proposal/v1 holds five password-reset-by-email records — an
# outcome, two requirements (one implementing the existing OUT-pr1 and
# depending on REQ-pr1), a constraint and a decision — and a ticket of three
# items, two implementing those records.
#
# Accepted: `war plan --records` emits an oh.war/records-request/v1 naming
# the profile's record types and relation kinds and the records already
# there, and writes nothing; `--draft --reviewed --apply` writes one record
# atom and a ticket, which `war check` reads well-formed, `war model` shows
# (the records, their relations, and each item's `implements`), and `war
# impact` walks; `war create --draft --records` does the same from one
# sentence.
# Refused, each by rule, the tree byte-identical after: a proposal with an
# undeclared type, an unknown target and an id the corpus has
# (conformance/fixtures/drafter/records-faulty.sh: record.type-undeclared,
# record.relation-target-unknown, record.duplicate-id, all named at once),
# through plan and through create; `--apply` without `--reviewed`
# (plan.review-required); no drafter configured (plan.no-drafter,
# ticket.no-drafter); and applying the same proposal again
# (record.duplicate-id, once per record).

echo "== from a sentence to typed records (OW-WAR-0148 M7) =="
PL_ROOT=$(scratch_corpus PL)
[[ -d "${PL_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
PL_TMP=$(mktemp -d)
PL_WAR="$REPO_ROOT/${WAR#./}"
PL_FX="$REPO_ROOT/conformance/fixtures"

if ! WAR="$PL_WAR" D="$PL_ROOT" R="$REPO_ROOT" FX="$PL_FX" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$PL_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
mkdir -p profiles docs/records/password-reset
cp "$R/profiles/delivery.toml" "$R/profiles/decision.toml" "$R/profiles/ticket.toml" profiles/
cp "$R/docs/records/password-reset/10-records.md" docs/records/password-reset/
grep -q '^drafter_argv = ' openwarrant.toml
sed -i "s|^drafter_argv = .*|drafter_argv = [\"bash\", \"$FX/drafter/records-proposal.sh\"]|" openwarrant.toml
sed -i "s|^drafter_timeout_secs = .*|drafter_timeout_secs = 120|" openwarrant.toml
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "password-reset records; the records drafter"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$PL_TMP/setup.log" >&2
    exit 9
fi

pl_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
pl_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
pl_war()  {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
        "$PL_WAR" --root "$PL_ROOT" "$@" </dev/null
}
pl_git()  { git -C "$PL_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
pl_line() { grep -m1 -E -- "$1" <<<"$2"; }
# Every file under the root but .git, ignored ones included, by content.
pl_tree() {
    (cd "$PL_ROOT" && find . -path ./.git -prune -o -type f -print0 | sort -z | xargs -0 sha256sum) | sha256sum
}
pl_reset() { pl_git checkout -q -- .; pl_git clean -fdxq; }
pl_drafter() {  # <fixture script> | "" for none
    if [[ -n "$1" ]]; then
        sed -i "s|^drafter_argv = .*|drafter_argv = [\"bash\", \"$PL_FX/drafter/$1\"]|" "$PL_ROOT/openwarrant.toml"
    else
        sed -i 's|^drafter_argv = .*|drafter_argv = []|' "$PL_ROOT/openwarrant.toml"
    fi
}
# The rules of an envelope's error diagnostics, sorted, then its exit code.
pl_rules() {
    python3 -c '
import json, sys
v = json.load(sys.stdin)
print(" ".join(sorted(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")), v["exit_code"])
' 2>&1
}
PL_SENT="Add password reset by email"

# 1. The request: the profile's types and kinds, the records already there,
#    and nothing written.
PL_T0=$(pl_tree)
PL_REQ=$(pl_war --json plan "$PL_SENT" --records 2>/dev/null | python3 -c '
import json, sys
r = json.load(sys.stdin)["result"]
print(r["api_version"], ",".join(r["record_types"]), ",".join(r["relation_kinds"]),
      ",".join("/".join(x) for x in r["required_relations"]), ",".join(r["item_relation_kinds"]),
      ",".join(sorted(e["id"] for e in r["existing_records"])))
' 2>&1)
if [[ "$PL_REQ" == "oh.war/records-request/v1 constraint,decision,option,outcome,requirement part_of,depends_on,implements,constrains,evaluates,supersedes,selected_over requirement/implements/outcome implements CON-pr1,DEC-pr1,OPT-pr1,OPT-pr2,OUT-pr1,REQ-pr1" \
    && "$(pl_tree)" == "$PL_T0" ]]; then
    pl_ok "the records request" "records-request/v1: delivery's 5 types, 7 kinds, the required relation, implements for items, the 6 records there; nothing written"
else
    pl_fail "the records request" "'$PL_REQ'"
fi

# 2. A faulty proposal: an undeclared type, an unknown target and an id the
#    corpus already has, each named by rule at once, through plan and
#    through create; the tree byte-identical.
pl_drafter records-faulty.sh
PL_T1=$(pl_tree)
PL_FP=$(pl_war --json plan "$PL_SENT" --records --draft 2>/dev/null)
PL_FPR=$(pl_rules <<<"$PL_FP")
PL_FPT=$(pl_tree)
PL_FC=$(pl_war --json create "$PL_SENT" --draft --records 2>/dev/null)
PL_FCR=$(pl_rules <<<"$PL_FC")
PL_FCT=$(pl_tree)
PL_FH=$(pl_war plan "$PL_SENT" --records --draft 2>&1)
PL_MISS=""
for want in \
    '^ERROR +record\.type-undeclared .*RISK-prf1 · risk: profile delivery declares no record type `risk`' \
    '^ERROR +record\.relation-target-unknown .*REQ-prf1 implements OUT-nope: OUT-nope is no record of this corpus' \
    '^ERROR +record\.duplicate-id .*REQ-pr1 is already declared at docs/records/password-reset/10-records\.md:'; do
    pl_line "$want" "$PL_FH" >/dev/null || PL_MISS="$PL_MISS [$want]"
done
PL_FWANT="record.duplicate-id record.relation-required record.relation-target-unknown record.type-undeclared 2"
if [[ "$PL_FPR" == "$PL_FWANT" && "$PL_FCR" == "$PL_FWANT" && -z "$PL_MISS" \
    && "$PL_FPT" == "$PL_T1" && "$PL_FCT" == "$PL_T1" ]]; then
    pl_ok "a faulty proposal is refused" "type-undeclared, relation-target-unknown, duplicate-id (and the requirement left implementing nothing) named at once, by plan and by create; tree byte-identical"
else
    pl_fail "a faulty proposal is refused" "plan '$PL_FPR', create '$PL_FCR'; missing:$PL_MISS; tree $( [[ "$PL_FPT" == "$PL_T1" && "$PL_FCT" == "$PL_T1" ]] && echo same || echo changed)"
fi
pl_drafter records-proposal.sh

# 3. --apply without --reviewed is refused before the drafter is asked, and
#    so is a proposal file applied unreviewed; nothing written.
PL_T2=$(pl_tree)
PL_U1=$(pl_war plan "$PL_SENT" --records --draft --apply 2>&1); PL_U1S=$?
PL_U2=$(pl_war plan --records --proposal "$PL_FX/proposals/records-password-reset.json" --apply 2>&1); PL_U2S=$?
if [[ $PL_U1S -ne 0 && $PL_U2S -ne 0 && "$(pl_tree)" == "$PL_T2" ]] \
    && pl_line '^ERROR +plan\.review-required .*a person reads the proposal first' "$PL_U1" >/dev/null \
    && pl_line '^ERROR +plan\.review-required ' "$PL_U2" >/dev/null; then
    pl_ok "unreviewed --apply is refused" "plan.review-required, from the drafter and from a file; tree byte-identical"
else
    pl_fail "unreviewed --apply is refused" "exit $PL_U1S/$PL_U2S: $(pl_line 'plan\.' "$PL_U1") | $(pl_line 'plan\.' "$PL_U2")"
fi

# 4. No drafter configured: refused by name, by plan and by create.
pl_drafter ""
PL_T3=$(pl_tree)
PL_N1=$(pl_war plan "$PL_SENT" --records --draft --reviewed --apply 2>&1); PL_N1S=$?
PL_N2=$(pl_war create "$PL_SENT" --draft --records 2>&1); PL_N2S=$?
PL_N3=$(pl_tree)
pl_drafter records-proposal.sh
if [[ $PL_N1S -ne 0 && $PL_N2S -ne 0 && "$PL_N3" == "$PL_T3" ]] \
    && pl_line '^ERROR +plan\.no-drafter .*\[plan\] drafter_argv` is not set; nothing was invented' "$PL_N1" >/dev/null \
    && pl_line '^refused \(ticket\.no-drafter\)' "$PL_N2" >/dev/null; then
    pl_ok "no drafter is refused by name" "plan.no-drafter and ticket.no-drafter; tree byte-identical"
else
    pl_fail "no drafter is refused by name" "exit $PL_N1S/$PL_N2S: $(pl_line 'no-drafter' "$PL_N1") | $(pl_line 'no-drafter' "$PL_N2")"
fi

# 5. Accepted: the drafter's proposal, reviewed and applied, is one record
#    atom and one ticket. `war check` reads the records well-formed; `war
#    model` carries the records, their relations and each item's
#    implements; `war impact REQ-pre1` reaches the constraint, the decision
#    and the item.
PL_A=$(pl_war --json plan "$PL_SENT" --records --draft --reviewed --apply 2>/dev/null)
PL_AJ=$(python3 -c '
import json, sys
v = json.load(sys.stdin)
r = v["result"]
print(v["exit_code"], r["schema"], r["file"], r["ticket"], " ".join("%s:%s" % (x["id"], x["type"]) for x in r["records"]),
      " ".join("%s>%s" % (i["id"], "+".join(i["implements"]) or "-") for i in r["items"]))
' <<<"$PL_A" 2>&1)
PL_TK=$(awk '{print $4}' <<<"$PL_AJ")
PL_REC="docs/records/password-reset-email/10-password-reset-by-email.md"
PL_NEW=$(pl_git status --porcelain --untracked-files=all | sed -n 's/^?? //p' | sed "s|docs/tickets/$PL_TK/.*|docs/tickets/$PL_TK/|" | sort -u | tr '\n' ' ')
PL_CHK=$(pl_war check 2>&1); PL_CS=$?
PL_REL=$(pl_war model --json 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)["result"]
ids = {"OUT-pre1", "REQ-pre1", "REQ-pre2", "CON-pre1", "DEC-pre1"}
recs = sorted("%s:%s" % (r["id"], r["type"]) for r in m["records"] if r["id"] in ids)
rels = sorted("%s>%s>%s" % (r["from"].split("/")[-1] if r["from"].startswith("t-") else r["from"], r["kind"], r["to"])
              for r in m["relations"] if (r["from"] in ids or r["from"].startswith("t-")) and r["kind"] != "part_of")
print(" ".join(recs), "|", " ".join(rels))
' 2>&1)
PL_I1=$(awk '{print $10}' <<<"$PL_AJ" | cut -d'>' -f1)
PL_I2=$(awk '{print $11}' <<<"$PL_AJ" | cut -d'>' -f1)
PL_WREL="CON-pre1:constraint DEC-pre1:decision OUT-pre1:outcome REQ-pre1:requirement REQ-pre2:requirement | CON-pre1>constrains>REQ-pre1 DEC-pre1>constrains>REQ-pre1 REQ-pre1>implements>OUT-pre1 REQ-pre2>depends_on>REQ-pr1 REQ-pre2>implements>OUT-pr1 $PL_I1>implements>CON-pre1 $PL_I1>implements>REQ-pre1 $PL_I2>implements>REQ-pre2"
PL_WREL_SORTED="$(tr ' ' '\n' <<<"${PL_WREL#*| }" | sort | tr '\n' ' ')"
PL_IMP=$(pl_war impact REQ-pre1 --json 2>/dev/null | python3 -c '
import json, sys
i = json.load(sys.stdin)["result"]
print(" ".join(sorted("%s>%s" % (a["id"].split("/")[-1], a["via"]["kind"]) for a in i["affected"])))
' 2>&1)
if [[ "$PL_AJ" == "0 oh.war/records-applied/v1 $PL_REC t-"*" OUT-pre1:outcome REQ-pre1:requirement REQ-pre2:requirement CON-pre1:constraint DEC-pre1:decision i-"*">REQ-pre1+CON-pre1 i-"*">REQ-pre2 i-"*">-" \
    && "$PL_NEW" == "docs/records/password-reset-email/10-password-reset-by-email.md docs/tickets/$PL_TK/ " \
    && $PL_CS -eq 0 && "${PL_REL%% |*}" == "${PL_WREL%% |*}" \
    && "$(tr ' ' '\n' <<<"${PL_REL#*| }" | sort | tr '\n' ' ')" == "$PL_WREL_SORTED" \
    && "$PL_IMP" == "CON-pre1>constrains DEC-pre1>constrains $PL_I1>implements" ]] \
    && pl_line '^PASS records\.well-formed .*11 record\(s\) in 2 record atom\(s\)' "$PL_CHK" >/dev/null; then
    pl_ok "a proposal becomes records + ticket" "$PL_REC and $PL_TK; check well-formed (11 records); model: 5 records, 8 relations incl. items implements; impact REQ-pre1 reaches CON, DEC and the item"
else
    pl_fail "a proposal becomes records + ticket" "applied '$PL_AJ'; new '$PL_NEW'; check $PL_CS $(pl_line 'record' "$PL_CHK"); model '$PL_REL'; impact '$PL_IMP'"
fi

# 6. Applying it again writes nothing: every record is a duplicate id, named
#    with where it already is.
PL_T4=$(pl_tree)
PL_AG=$(pl_war plan "$PL_SENT" --records --draft --reviewed --apply 2>&1); PL_AGS=$?
PL_DUPS=$(grep -c -E '^ERROR +record\.duplicate-id .*is already declared at docs/records/password-reset-email/10-password-reset-by-email\.md:' <<<"$PL_AG")
if [[ $PL_AGS -ne 0 && "$PL_DUPS" == 5 && "$(pl_tree)" == "$PL_T4" ]]; then
    pl_ok "applying twice is refused" "record.duplicate-id x5, each naming its first declaration; tree byte-identical"
else
    pl_fail "applying twice is refused" "exit $PL_AGS, $PL_DUPS duplicate-id; $(pl_line 'ERROR' "$PL_AG")"
fi

# 7. `war create --draft --records`: the ticket and the records it
#    implements from one sentence, in the area named.
pl_reset
PL_C=$(pl_war --json create "$PL_SENT" --draft --records --area email-reset 2>/dev/null)
PL_CJ=$(python3 -c '
import json, sys
v = json.load(sys.stdin)
r = v["result"]
print(v["exit_code"], r["file"], len(r["records"]), len(r["items"]))
' <<<"$PL_C" 2>&1)
PL_CK=$(pl_war check 2>&1); PL_CKS=$?
PL_CM=$(pl_war model --json 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)["result"]
print(len([r for r in m["relations"] if r["from"].startswith("t-") and r["kind"] == "implements"]))
' 2>&1)
pl_reset
if [[ "$PL_CJ" == "0 docs/records/email-reset/10-password-reset-by-email.md 5 3" && $PL_CKS -eq 0 && "$PL_CM" == 3 ]] \
    && pl_line '^PASS records\.well-formed .*11 record\(s\) in 2 record atom\(s\)' "$PL_CK" >/dev/null; then
    pl_ok "create --draft --records" "5 records in docs/records/email-reset/ and a 3-item ticket, 3 items-implements edges in the model; check well-formed"
else
    pl_fail "create --draft --records" "'$PL_CJ'; check $PL_CKS; implements $PL_CM"
fi

command rm -rf "$PL_TMP"
corpus_gone "$PL_ROOT"
unset PL_ROOT PL_TMP PL_WAR PL_FX PL_SENT PL_TK PL_REC
