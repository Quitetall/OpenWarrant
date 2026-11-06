# shellcheck shell=bash
# M16 (decision 13): each `##` section of CLAUDE.md and AGENTS.md is a record
# `md:<file>#<slug>` of type `instruction`, read without editing the file,
# with a revision over its own byte span. Records and Warrants cite one
# (`constrains md:CLAUDE.md#testing`, a basis naming it), and `war impact`
# follows the citations. The managed block is no section.
#
# On a scratch program (IR) with the delivery profile's record vocabulary:
# - `war model` lists both CLAUDE.md sections as `instruction` records and the
#   relation citing one; the block added by `war agents-md --block` adds no
#   record and moves no revision.
# - Editing one section moves its revision and no other.
# - `war impact md:CLAUDE.md#testing` names the record that constrains it and
#   the Warrant whose basis cites it.
# Refusals:
# - a relation to a section that does not exist is kept and warned,
#   record.relation-target-unknown in `war check` and
#   model.relation-target-unknown in `war model`, never dropped;
# - `war impact` on a section that does not exist is impact.unknown-record;
# - "see md:CLAUDE.md for the rules" (no `#`) is prose, not a relation.

echo "== CLAUDE.md sections as records (M16) =="
IR_TMP=$(mktemp -d)
IR_ROOT=$(scratch_corpus IR)
# Sourced outside the battery, `scratch_corpus` is undefined and the edits
# below would land in this repository. Refuse.
[[ -d "${IR_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
IR_WAR="$REPO_ROOT/${WAR#./}"
ir_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ir_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ir_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$IR_WAR" --root "$IR_ROOT" "$@"; }
# ir_model <file>: `war model --json` into <file>.
ir_model() { ir_war model --json > "$1" 2>/dev/null; }
# ir_rev <model> <id>: the revision of record <id>, or nothing.
ir_rev() {
    python3 -c 'import json,sys; m=json.load(open(sys.argv[1]))["result"]; print(next((r["revision"] for r in m["records"] if r["id"]==sys.argv[2]), ""))' "$1" "$2" 2>/dev/null
}

mkdir -p "$IR_ROOT/profiles" "$IR_ROOT/docs/records/rules"
cp "$REPO_ROOT/profiles/delivery.toml" "$REPO_ROOT/profiles/decision.toml" "$REPO_ROOT/profiles/ticket.toml" "$IR_ROOT/profiles/"
printf '# House rules\n\n## Testing\nRun `make test` before you push.\n\n## Style\nTabs, not spaces.\n' > "$IR_ROOT/CLAUDE.md"
cat > "$IR_ROOT/docs/records/rules/10-rules.md" <<'EOF'
---
schema: oh.war/records/v1
profile: delivery
---
# Rules this program keeps

## CON-ir1 · constraint
constrains md:CLAUDE.md#testing

The suite passes before a merge. See md:CLAUDE.md for the rest.
EOF
printf '\nThis plan follows md:CLAUDE.md#testing.\n' >> "$IR_ROOT/docs/warrants/IR-WAR-0001/atoms/20-basis.md"

# --- sections are records --------------------------------------------------------
ir_model "$IR_TMP/a.json"
IR_SEEN=$(python3 - "$IR_TMP/a.json" <<'PY' 2>&1
import json, sys
m = json.load(open(sys.argv[1]))["result"]
recs = sorted("%s:%s" % (r["id"], r["type"]) for r in m["records"] if r["source"] == "CLAUDE.md")
rels = sorted("%s %s %s" % (r["from"], r["kind"], r["to"]) for r in m["relations"] if r["to"].startswith("md:"))
print(" ".join(recs)); print(" | ".join(rels))
PY
)
IR_WANT=$'md:CLAUDE.md#style:instruction md:CLAUDE.md#testing:instruction\nCON-ir1 constrains md:CLAUDE.md#testing'
if [[ $IR_SEEN == "$IR_WANT" ]]; then
    ir_ok "CLAUDE.md sections are records" "two instruction records and the one relation citing them; prose is no relation"
else
    ir_fail "CLAUDE.md sections are records" "$(tr '\n' '|' <<<"$IR_SEEN")"
fi

# The block is no record and moves no revision.
IR_T0=$(ir_rev "$IR_TMP/a.json" 'md:CLAUDE.md#testing')
IR_S0=$(ir_rev "$IR_TMP/a.json" 'md:CLAUDE.md#style')
ir_war agents-md --block >/dev/null 2>&1
ir_model "$IR_TMP/b.json"
IR_N=$(python3 -c 'import json,sys; print(sum(1 for r in json.load(open(sys.argv[1]))["result"]["records"] if r["source"]=="CLAUDE.md"))' "$IR_TMP/b.json" 2>&1)
if [[ -n $IR_T0 && $IR_N == 2 && $(ir_rev "$IR_TMP/b.json" 'md:CLAUDE.md#testing') == "$IR_T0" \
    && $(ir_rev "$IR_TMP/b.json" 'md:CLAUDE.md#style') == "$IR_S0" ]] && grep -qxF '<!-- openwarrant:begin -->' "$IR_ROOT/CLAUDE.md"; then
    ir_ok "the block is no record" "added to CLAUDE.md: still 2 sections, neither revision moved"
else
    ir_fail "the block is no record" "$IR_N CLAUDE.md records; testing $IR_T0 -> $(ir_rev "$IR_TMP/b.json" 'md:CLAUDE.md#testing')"
fi

# --- one edit, one revision --------------------------------------------------------
sed -i 's/Tabs, not spaces./Spaces, four of them./' "$IR_ROOT/CLAUDE.md"
ir_model "$IR_TMP/c.json"
IR_T1=$(ir_rev "$IR_TMP/c.json" 'md:CLAUDE.md#testing')
IR_S1=$(ir_rev "$IR_TMP/c.json" 'md:CLAUDE.md#style')
if [[ -n $IR_S1 && $IR_T1 == "$IR_T0" && $IR_S1 != "$IR_S0" ]]; then
    ir_ok "an edit moves one section's revision" "style ${IR_S0:7:12} -> ${IR_S1:7:12}; testing unchanged"
else
    ir_fail "an edit moves one section's revision" "testing $IR_T0 -> $IR_T1; style $IR_S0 -> $IR_S1"
fi

# --- impact names its citers -------------------------------------------------------
IR_IMPACT=$(ir_war impact 'md:CLAUDE.md#testing' --json 2>/dev/null)
IR_CITERS=$(python3 -c '
import json, sys
r = json.loads(sys.argv[1])["result"]
print(" ".join(a["id"] for a in r["affected"]))
print(" ".join("%s:%s" % (d["id"], d["kind"]) for d in r["documents"]))
' "$IR_IMPACT" 2>&1)
if [[ $(head -1 <<<"$IR_CITERS") == "CON-ir1" ]] \
    && grep -q 'IR-WAR-0001:warrant' <<<"$IR_CITERS" && grep -q 'CLAUDE.md:instruction' <<<"$IR_CITERS"; then
    ir_ok "impact names what cites a section" "CON-ir1 (constrains), IR-WAR-0001 (its basis cites it), CLAUDE.md"
else
    ir_fail "impact names what cites a section" "$(tr '\n' '|' <<<"$IR_CITERS")"
fi

# --- refusals ------------------------------------------------------------------------
sed -i 's/^constrains md:CLAUDE.md#testing$/constrains md:CLAUDE.md#testing, md:CLAUDE.md#no-such-section/' \
    "$IR_ROOT/docs/records/rules/10-rules.md"
IR_CHECK=$(ir_war check 2>&1)
ir_model "$IR_TMP/d.json"
IR_KEPT=$(python3 - "$IR_TMP/d.json" <<'PY' 2>&1
import json, sys
m = json.load(open(sys.argv[1]))["result"]
t = "md:CLAUDE.md#no-such-section"
print("edges", sum(1 for r in m["relations"] if r["to"] == t),
      "diagnostics", sum(1 for d in m["diagnostics"] if d["rule"] == "model.relation-target-unknown" and t in d["message"]))
PY
)
if grep -q 'WARN record.relation-target-unknown .*md:CLAUDE.md#no-such-section' <<<"$IR_CHECK" \
    && grep -q 'docs/records/rules/10-rules.md:8' <<<"$IR_CHECK" && [[ $IR_KEPT == "edges 1 diagnostics 1" ]]; then
    ir_ok "a missing section is a diagnostic" "record.relation-target-unknown at 10-rules.md:8; the edge is kept ($IR_KEPT)"
else
    ir_fail "a missing section is a diagnostic" "$IR_KEPT; $(grep -E 'record\.' <<<"$IR_CHECK" | head -2 | tr '\n' '|')"
fi
IR_OUT=$(ir_war impact 'md:CLAUDE.md#no-such-section' 2>&1)
IR_RC=$?
if [[ $IR_RC -ne 0 ]] && grep -q 'impact.unknown-record' <<<"$IR_OUT"; then
    ir_ok "impact refuses a missing section" "impact.unknown-record, exit $IR_RC"
else
    ir_fail "impact refuses a missing section" "exit $IR_RC: $(head -2 <<<"$IR_OUT" | tr '\n' '|')"
fi

corpus_gone "$IR_ROOT"
command rm -rf "$IR_TMP"
unset IR_TMP IR_ROOT IR_WAR IR_SEEN IR_WANT IR_T0 IR_S0 IR_T1 IR_S1 IR_N IR_IMPACT IR_CITERS \
    IR_CHECK IR_KEPT IR_OUT IR_RC
unset -f ir_ok ir_fail ir_war ir_model ir_rev
