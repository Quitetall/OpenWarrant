# shellcheck shell=bash
# OW-WAR-0122 — the document grammar (docs/GRAMMAR.md), bound to the tool.
#
# docs/GRAMMAR.md says, construct by construct, what `war` accepts and what it
# refuses and by which rule. A grammar document nobody re-reads drifts from
# the tool (R-001), so every row with a plant id (`G-…`) is exercised here,
# and the drift control at the end fails when the document cites a rule id or
# a plant id that nothing below produced.
#
# On a scratch program, never this repository: the header plants break an
# atom's header, and the atoms of this corpus are bound by signatures. The
# scaffold's GR-WAR-0001 is well-formed by construction, so each refusal below
# is the one planted and not one the corpus already had.
#
# §62's example is read from the SAS in this repository at run time, not
# copied here: the plant tests the example the specification actually prints.

echo "== the document grammar (OW-WAR-0122) =="

PLANT_ROOT=$(scratch_corpus GR)
# Sourced outside plant.sh, `scratch_corpus` is undefined and PLANT_ROOT is
# empty, and every mutation below would land on this repository. Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }

GR_W=docs/warrants/GR-WAR-0001
GR_UUID=$(sed -n 's/^uuid = "\(.*\)"$/\1/p' "$PLANT_ROOT/$GR_W/manifest.toml")
[[ -n "$GR_UUID" ]] || { printf 'PLANT SETUP FAILED: no uuid in %s/manifest.toml\n' "$GR_W" >&2; exit 9; }
# Another Warrant's identity, for the copied-in atom: this Warrant's own.
GR_OTHER_UUID=01a0d04c-5ee5-7ba2-9dc3-b9c50dcc6ba1

# §62's example, verbatim: the ```markdown block under "## 62. Atom source format".
GR_EXAMPLE=$(mktemp)
awk '/^## 62\. Atom source format/ {in62=1; next}
     in62 && /^## / {exit}
     in62 && /^```markdown$/ {grab=1; next}
     grab && /^```$/ {exit}
     grab {print}' docs/sas/WAR_Software_Architecture_Specification.md > "$GR_EXAMPLE"
grep -q '^holder:$' "$GR_EXAMPLE" && grep -q '^  kind: git$' "$GR_EXAMPLE" \
    || { printf 'PLANT SETUP FAILED: §62 example not found, or it no longer nests `holder`\n' >&2; exit 9; }

# Every `war` output and every plant id this file produced, for the drift control.
GR_SEEN=$(mktemp)
GR_RAN=()

gr_record() { printf '%s\n' "$2" >> "$GR_SEEN"; GR_RAN+=("${1%% *}"); }

# grammar <name> <rule> <detail> <exit> <mutation> <war args...>
#
# plant_cmd's assertions — the exit, the rule, the detail that pins WHICH
# violation — with the output kept, so the drift control can ask whether a
# rule the document cites was ever produced.
grammar() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"
    shift 5

    plant_restore
    plant_mutate "$mutate" || { FAILED=$((FAILED + 1)); return; }

    local out status
    out="$(plant_war "$@" 2>&1)"
    status=$?
    plant_restore
    gr_record "$name" "$out"

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but rule %s never fired\n' "$name" "$status" "$rule"
        printf '      (rejected for the wrong reason — the failure §92 warns about)\n'
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s rule %s fired but not for %s\n' "$name" "$rule" "$detail"
        printf '      (right rule, wrong violation — still the wrong reason)\n'
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

# grammar_accept <name> <must-not-fire> <mutation> <war args...>
#
# A positive control: the construct is accepted (exit 0) and the rule that
# would refuse it did not fire. Without these, a reader that refused
# everything would pass every plant above.
grammar_accept() {
    local name="$1" rule="$2" mutate="$3"
    shift 3

    plant_restore
    plant_mutate "$mutate" || { FAILED=$((FAILED + 1)); return; }

    local out status
    out="$(plant_war "$@" 2>&1)"
    status=$?
    plant_restore
    gr_record "$name" "$out"

    if [[ "$status" -ne 0 ]] || grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exit %s; %s\n' "$name" "$status" "$(grep -m1 -E '^(ERROR|UNKNOWN)' <<<"$out")"
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s accepted, no %s\n' "$name" "$rule"
    PASSED=$((PASSED + 1))
}

GR_INTENT=$GR_W/atoms/10-intent.md
GR_BASIS=$GR_W/atoms/20-basis.md
GR_MAN=$GR_W/manifest.toml
GR_MS=$GR_W/atoms/45-milestones.yaml
GR_ASSUR=$GR_W/atoms/60-assurance.md
GR_WO=$GR_W/atoms/40-work-order.md

# ---------------------------------------------------------------------------
# The atom header against its manifest entry (OBL-001).
# ---------------------------------------------------------------------------

grammar_accept "G-H0 an untouched scaffold" "atom.header" ":" check

grammar "G-H1 header role changed" "atom.header" \
    "header's \`role\` is \"intent\", but the manifest's role is \"basis\"" 2 \
    "sed -i 's/^role: basis\$/role: intent/' $GR_BASIS; assert_present 'role: intent' $GR_BASIS" check

grammar "G-H2 header warrant_uuid changed" "atom.header" \
    "header's \`warrant_uuid\` is \"$GR_OTHER_UUID\", but the manifest's \`uuid\` is \"$GR_UUID\"" 2 \
    "sed -i 's/^warrant_uuid: .*/warrant_uuid: $GR_OTHER_UUID/' $GR_BASIS; assert_present '$GR_OTHER_UUID' $GR_BASIS" check

grammar "G-H3 header order changed" "atom.header" \
    "header's \`order\` is \"21\", but the manifest's ordinal is \"20\"" 2 \
    "sed -i 's/^order: 20\$/order: 21/' $GR_BASIS; assert_present 'order: 21' $GR_BASIS" check

grammar "G-H4 header schema removed" "atom.header" \
    "the header has no \`schema\`" 2 \
    "sed -i '/^schema: oh.war\/atom\/v1\$/d' $GR_BASIS; assert_gone 'schema: oh.war/atom/v1' $GR_BASIS" check

# `jurisdiction` is the one header key the tool read before atom.header, and
# it is checked by value, not against the manifest (§13, §16.1).
grammar "G-J1 an unknown jurisdiction" "atom.unknown-jurisdiction" "declares jurisdiction \"whatever\"" 2 \
    "sed -i 's/^jurisdiction: authored\$/jurisdiction: whatever/' $GR_INTENT; assert_present 'jurisdiction: whatever' $GR_INTENT" check
grammar "G-J2 intent declared bound" "atom.jurisdiction-mismatch" "has role \`intent\`, which §16.1 places under \`authored\`" 2 \
    "sed -i 's/^jurisdiction: authored\$/jurisdiction: bound/' $GR_INTENT; assert_present 'jurisdiction: bound' $GR_INTENT" check
grammar_accept "G-J3 no jurisdiction at all" "atom.header" \
    "sed -i '/^jurisdiction: authored\$/d' $GR_INTENT; assert_gone 'jurisdiction:' $GR_INTENT" check

# ---------------------------------------------------------------------------
# The frontmatter reader (OW-ADR-0002), construct by construct (OBL-003).
# Each replaces the scaffold's `classification: internal` line.
# ---------------------------------------------------------------------------

gr_header_plant() {
    local id="$1" what="$2" replacement="$3" detail="$4"
    grammar "$id $what" "atom.frontmatter" "$detail" 2 \
        "sed -i 's/^classification: internal\$/$replacement/' $GR_INTENT; assert_gone 'classification: internal' $GR_INTENT" check
}
gr_header_plant G-R1 "an anchor"          'classification: \&c internal'           "a YAML anchor is not supported"
gr_header_plant G-R2 "an alias"           'classification: *c'                     "a YAML alias is not supported"
gr_header_plant G-R3 "a tag"              'classification: !!str internal'         "a YAML tag is not supported"
gr_header_plant G-R4 "a flow sequence"    'classification: [internal]'             "a flow sequence is not supported"
gr_header_plant G-R5 "a flow mapping"     'classification: {level: internal}'      "a flow mapping is not supported"
gr_header_plant G-R6 "a block scalar"     'classification: |\n  internal'          "a block scalar is not supported"
gr_header_plant G-R7 "a folded scalar"    'classification: >\n  internal'          "a folded block scalar is not supported"
gr_header_plant G-R8 "a duplicate key"    'classification: secret\nclassification: public' \
    "duplicate key \"classification\""

grammar "G-R9 no opening fence" "atom.frontmatter" "does not begin with a \`---\` frontmatter fence" 2 \
    "sed -i '1d' $GR_INTENT; [[ \$(head -1 $GR_INTENT) == 'schema: oh.war/atom/v1' ]] || { echo 'PLANT MUTATION WAS A NO-OP: the fence is still there' >&2; plant_restore; exit 9; }" check

# §62's own example, verbatim, as the intent atom: refused for the nested
# `holder:` mapping, and for nothing else first (line 8 is `  kind: git`).
grammar "G-R10 §62 example verbatim" "atom.frontmatter" \
    "line 8: an indented (nested) mapping is not supported" 2 \
    "cp '$GR_EXAMPLE' $GR_INTENT; assert_present 'holder:' $GR_INTENT" check

# The same example with `holder` removed: the reader accepts it, and the only
# refusal is atom.header naming the Warrant the example belongs to — which is
# the rule working, not the reader.
plant_restore
grep -v -e '^holder:$' -e '^  kind: git$' "$GR_EXAMPLE" > "$PLANT_ROOT/$GR_INTENT"
assert_gone 'holder:' "$PLANT_ROOT/$GR_INTENT"
GR_OUT=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
GR_STATUS=$?
plant_restore
# The only errors are the header's, for warrant_uuid: `atom.header` here
# and OW-WAR-0119's `identity.atom-mismatch`, which names the same fact.
gr_record "G-P1 §62 example minus holder" "$GR_OUT"
if [[ $GR_STATUS -eq 2 ]] && ! grep -q 'atom.frontmatter' <<<"$GR_OUT" \
    && grep -q "atom.header .*header's \`warrant_uuid\` is \"019c8f2d-7b4d-7c41-9cb7-2636e5f582ea\"" <<<"$GR_OUT" \
    && [[ $(grep '^ERROR' <<<"$GR_OUT" | grep -vcE '^ERROR (atom\.header|identity\.atom-mismatch) ') -eq 0 ]]; then
    printf 'ok    %-34s read; only atom.header, for warrant_uuid\n' "G-P1 §62 example minus holder"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; %s\n' "G-P1 §62 example minus holder" "$GR_STATUS" "$(grep -m1 '^ERROR' <<<"$GR_OUT")"
    FAILED=$((FAILED + 1))
fi

# ...and with this Warrant's identity in place of the example's, it passes:
# `atom_uuid`, `jurisdiction` and `classification` are kept, not refused.
grammar_accept "G-P2 §62 example, own uuid" "atom.frontmatter" \
    "grep -v -e '^holder:\$' -e '^  kind: git\$' '$GR_EXAMPLE' | sed 's/^warrant_uuid: .*/warrant_uuid: $GR_UUID/' > $GR_INTENT; assert_present 'atom_uuid:' $GR_INTENT; assert_gone 'holder:' $GR_INTENT" \
    check

# A namespaced key passes, and survives compilation: the IR binds the atom's
# exact bytes (§62.2), so the digest it records is the digest of bytes that
# still carry the key.
plant_restore
sed -i 's/^classification: internal$/classification: internal\nx.note: kept/' "$PLANT_ROOT/$GR_INTENT"
assert_present 'x.note: kept' "$PLANT_ROOT/$GR_INTENT"
GR_OUT=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
GR_STATUS=$?
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
GR_GEN=$("$WAR" --root "$PLANT_ROOT" check --generated 2>&1)
GR_GEN_STATUS=$?
GR_WANT=$(sha256sum "$PLANT_ROOT/$GR_INTENT" | cut -d' ' -f1)
GR_GOT=$(python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
print("".join(a["atom_source_digest"] for a in d["source_and_composition"]["atoms"] if a["source"] == "atoms/10-intent.md"))
' "$PLANT_ROOT/$GR_W/generated/WAR.json" 2>/dev/null)
plant_restore
gr_record "G-P3 a namespaced key" "$GR_OUT"
if [[ $GR_STATUS -eq 0 && $GR_GEN_STATUS -eq 0 && -n "$GR_WANT" && "$GR_GOT" == "$GR_WANT" ]]; then
    printf 'ok    %-34s passes; the IR digest binds its bytes\n' "G-P3 a namespaced key"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s check %s, --generated %s, IR %s vs file %s\n' "G-P3 a namespaced key" \
        "$GR_STATUS" "$GR_GEN_STATUS" "${GR_GOT:-none}" "$GR_WANT"
    FAILED=$((FAILED + 1))
fi

# ---------------------------------------------------------------------------
# The manifest and roles (§61, §16).
# ---------------------------------------------------------------------------

grammar "G-M1 a duplicate ordinal" "manifest.invalid" "duplicate atom ordinal 10" 2 \
    "sed -i '0,/^ordinal = 20\$/s//ordinal = 10/' $GR_MAN; assert_gone 'ordinal = 20' $GR_MAN" check

grammar "G-M2 a declared atom is missing" "atom.missing" "declared at ordinal 20" 2 \
    "command rm -f $GR_BASIS; assert_gone_file $GR_BASIS" check

grammar_accept "G-M3 an unknown manifest key" "manifest.invalid" \
    "sed -i 's/^profile = \"delivery\"\$/profile = \"delivery\"\nflavour = \"vanilla\"/' $GR_MAN; assert_present 'flavour' $GR_MAN" check

# gr_role_atom <ordinal> <role> <required>: one more atom, header and all.
# A function the mutation calls, run by plant_mutate inside the scratch.
gr_role_atom() {
    printf '\n[[atoms]]\nordinal = %s\nrole = "%s"\npath = "atoms/%s-x.md"\nrequired = %s\n' \
        "$1" "$2" "$1" "$3" >> "$GR_MAN"
    printf -- '---\nschema: oh.war/atom/v1\nwarrant_uuid: %s\nrole: %s\norder: %s\n---\n\n# X\n' \
        "$GR_UUID" "$2" "$1" > "$GR_W/atoms/$1-x.md"
    assert_present "role = \"$2\"" "$GR_MAN"
}

grammar "G-M4 an unknown required role" "manifest.invalid" "declares role \"hypothesis\"" 2 \
    "gr_role_atom 70 hypothesis true" check
grammar "G-M5 unknown optional plain role" "manifest.invalid" "declares role \"hypothesis\"" 2 \
    "gr_role_atom 70 hypothesis false" check
grammar "G-M6 a namespaced role, required" "manifest.invalid" "declares role \"x.review\"" 2 \
    "gr_role_atom 70 x.review true" check
grammar "G-M7 §16.1 role name decisions" "manifest.invalid" "declares role \"decisions\"" 2 \
    "gr_role_atom 30 decisions false" check

# A namespaced optional role is accepted and carried into the canonical IR.
plant_restore
plant_mutate "gr_role_atom 70 x.review false"
GR_OUT=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
GR_STATUS=$?
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
GR_ROLE=$(python3 -c '
import json, sys
d = json.load(open(sys.argv[1]))
print(" ".join(a["role"] for a in d["source_and_composition"]["atoms"]))
' "$PLANT_ROOT/$GR_W/generated/WAR.json" 2>/dev/null)
plant_restore
gr_record "G-M8 a namespaced role, optional" "$GR_OUT"
if [[ $GR_STATUS -eq 0 ]] && grep -qw 'x.review' <<<"$GR_ROLE"; then
    printf 'ok    %-34s accepted, and in the IR\n' "G-M8 a namespaced role, optional"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; IR roles: %s\n' "G-M8 a namespaced role, optional" "$GR_STATUS" "${GR_ROLE:-none}"
    FAILED=$((FAILED + 1))
fi

grammar "G-M9 an atom bound by ref" "atom.bound-unresolvable" "cannot be resolved offline" 2 \
    "printf '\n[[atoms]]\nordinal = 30\nrole = \"adr\"\nref = \"adr://019c0000-0000-7000-8000-000000000000\"\nrequired = false\n' >> $GR_MAN; assert_present 'ref = ' $GR_MAN" check

# ---------------------------------------------------------------------------
# Structured atoms (§62.1; OW-ADR-0003).
# ---------------------------------------------------------------------------

grammar "G-S1 an anchor in milestones" "milestones.invalid" "a YAML anchor is not supported" 2 \
    "sed -i 's/^  - id: \"M1\"\$/  - id: \&m \"M1\"/' $GR_MS; assert_present '&m' $GR_MS" check

grammar "G-S2 a mapping nested too deep" "milestones.invalid" "a mapping nested below a sequence item" 2 \
    "sed -i '0,/^    title: .*/s//    title:\n      text: nested/' $GR_MS; assert_present 'text: nested' $GR_MS" check

grammar "G-S3 duplicate key in milestones" "milestones.invalid" "duplicate key \"schema\"" 2 \
    "sed -i 's/^schema: \"oh.war\/milestones\/v1\"\$/schema: \"oh.war\/milestones\/v1\"\nschema: \"x\"/' $GR_MS; assert_present 'schema: \"x\"' $GR_MS" check

# §62.1 permits canonical JSON; the tool reads every milestones atom with the
# YAML-subset reader, whatever its extension.
grammar "G-S4 canonical JSON milestones" "milestones.invalid" "line 1: expected \`key: value\`" 2 \
    "command rm -f $GR_MS; printf '{\"milestones\":[{\"id\":\"M1\",\"obligation_refs\":[\"OBL-001\"],\"title\":\"t\"}],\"schema\":\"oh.war/milestones/v1\",\"stages\":[]}' > $GR_W/atoms/45-milestones.json; sed -i 's/45-milestones.yaml/45-milestones.json/' $GR_MAN; assert_present '45-milestones.json' $GR_MAN" check

# ---------------------------------------------------------------------------
# Markdown body headings, as a stage's context_sections selects them.
# ---------------------------------------------------------------------------

# gr_section <heading>: STAGE-001 selects 40-work-order.md#<heading>, and the
# work order gains a fenced block holding a `## Fenced` line.
gr_section() {
    sed -i "0,/^    responsibility_tier: \"T2\"\$/s//    responsibility_tier: \"T2\"\n    executor_ref: \"agent:\/\/plant\"\n    context_sections: [\"40-work-order.md#$1\"]/" "$GR_MS"
    printf '\n```\n## Fenced\n```\n' >> "$GR_WO"
    assert_present "40-work-order.md#$1" "$GR_MS"
}

grammar_accept "G-B1 a heading, selected by text" "dispatch.section-missing" \
    "gr_section Deliverables" dispatch GR-WAR-0001 STAGE-001 --prototype
grammar "G-B2 a heading inside a fence" "dispatch.section-missing" "has no section \"Fenced\"" 2 \
    "gr_section Fenced" dispatch GR-WAR-0001 STAGE-001 --prototype
grammar "G-B3 a heading in another case" "dispatch.section-missing" "has no section \"deliverables\"" 2 \
    "gr_section deliverables" dispatch GR-WAR-0001 STAGE-001 --prototype

# ---------------------------------------------------------------------------
# The obligation block and the gate citation.
# ---------------------------------------------------------------------------

grammar "G-O1 an obligation with no scope" "obligations.invalid" "\"OBL-001\" declares no scope" 2 \
    "sed -i '0,/^- \*\*scope:\*\*.*/{/^- \*\*scope:\*\*/d}' $GR_ASSUR; assert_gone 'revisions/0.1.0.toml\` and this' $GR_ASSUR" check

grammar "G-O2 obligation with no evidence" "obligations.invalid" "\"OBL-001\" declares no evidence" 2 \
    "sed -i '0,/^- \*\*evidence:\*\*.*/{/^- \*\*evidence:\*\*/d}' $GR_ASSUR; assert_gone 'reports 0.1.0 accepted' $GR_ASSUR" check

grammar "G-O3 a duplicate obligation id" "obligations.invalid" "duplicate obligation id \"OBL-001\"" 2 \
    "sed -i 's/^### OBL-002 /### OBL-001 /' $GR_ASSUR; assert_gone '### OBL-002' $GR_ASSUR" check

grammar_accept "G-O4 a hyphen for the em dash" "obligations.invalid" \
    "sed -i 's/^### OBL-001 — /### OBL-001 - /' $GR_ASSUR; assert_present '### OBL-001 - ' $GR_ASSUR" check

grammar "G-G1 a gate not in the registry" "gate.unresolved" "gate://no.such.gate@9.9.9, which is not in the registry" 2 \
    "sed -i '0,/gate:\/\/software.repo.war-check@1.0.0/s//gate:\/\/no.such.gate@9.9.9/' $GR_ASSUR; assert_present 'no.such.gate' $GR_ASSUR" check

# §105 recommends gate://<gate-id>/<version>; the tool reads only `@`.
grammar "G-G2 §105 gate URI form" "gate.unresolved" "malformed gate URI \"gate://software.repo.war-check/1.0.0\"" 2 \
    "sed -i '0,/gate:\/\/software.repo.war-check@1.0.0/s//gate:\/\/software.repo.war-check\/1.0.0/' $GR_ASSUR; assert_present 'war-check/1.0.0' $GR_ASSUR" check

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
command rm -f "$GR_EXAMPLE"

# ---------------------------------------------------------------------------
# The drift control (OBL-003): the document cites nothing no plant produced.
#
# A rule is cited as rule `x.y`, a plant as `G-…`; that convention is stated
# in docs/GRAMMAR.md. Then the same question, asked of a copy of the document
# with a rule nothing produced: a control that never fails is not a control.
# ---------------------------------------------------------------------------

gr_uncited() { # <document> -> each cited rule or plant id nothing above produced
    local r
    grep -oE 'rule `[a-z][a-z0-9_-]*(\.[a-z0-9_-]+)+`' "$1" | sed 's/^rule `//; s/`$//' | sort -u \
        | while read -r r; do grep -Fqw -- "$r" "$GR_SEEN" || printf 'rule %s\n' "$r"; done
    grep -oE '`G-[A-Z][0-9]+`' "$1" | tr -d '`' | sort -u \
        | while read -r r; do printf '%s\n' "${GR_RAN[@]}" | grep -Fqx -- "$r" || printf 'plant %s\n' "$r"; done
}

GR_DOC=docs/GRAMMAR.md
GR_RAN+=("G-D1" "G-D2")
GR_CITED=$(grep -oE 'rule `[a-z][a-z0-9_-]*(\.[a-z0-9_-]+)+`' "$GR_DOC" 2>/dev/null | sort -u | wc -l)
GR_MISSING=$(gr_uncited "$GR_DOC" 2>/dev/null)
if [[ -f "$GR_DOC" && "$GR_CITED" -gt 0 && -z "$GR_MISSING" ]]; then
    printf 'ok    %-34s %s rule id(s), every one produced\n' "G-D1 the document cites no drift" "$GR_CITED"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s cited; unproduced: %s\n' "G-D1 the document cites no drift" "$GR_CITED" "$(tr '\n' ' ' <<<"$GR_MISSING")"
    FAILED=$((FAILED + 1))
fi

GR_COPY=$(mktemp)
cp "$GR_DOC" "$GR_COPY" 2>/dev/null
printf '\n| planted | none | refuse | refuse, rule `grammar.planted-drift` | `G-Z9` |\n' >> "$GR_COPY"
GR_MISSING=$(gr_uncited "$GR_COPY")
command rm -f "$GR_COPY"
if grep -qx 'rule grammar.planted-drift' <<<"$GR_MISSING" && grep -qx 'plant G-Z9' <<<"$GR_MISSING"; then
    printf 'ok    %-34s a planted citation is reported\n' "G-D2 the drift control refuses"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s reported: %s\n' "G-D2 the drift control refuses" "$(tr '\n' ' ' <<<"$GR_MISSING")"
    FAILED=$((FAILED + 1))
fi

command rm -f "$GR_SEEN"
