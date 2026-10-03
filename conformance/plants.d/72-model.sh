# shellcheck shell=bash
# OW-WAR-0148 OBL-001 / OBL-002 — `war model`, the compiled corpus as one
# document (`oh.war/model/v1`), on a scratch program (MD) holding two
# Warrants (one a child and successor of the other), a deliverable, a
# question, the roadmap the scaffold writes, and a ticket with a blocker that
# was promoted into a Warrant.
#
# Accepted: every record kind and every relation kind is named; two runs over
# the same tree are byte-identical with the same basis_digest; the output
# validates against schemas/oh.war/model/v1.json.
# Refused: a parent naming no Warrant of the corpus is a diagnostic and the
# edge is kept, not dropped; a model with one field added fails the schema;
# and the comparison the M1 differential uses (`cmp`) reports a doctored
# output — one byte changed — as different.

echo "== compiled model (OW-WAR-0148) =="
MD_TMP=$(mktemp -d)
MD_ROOT=$(scratch_corpus MD)
# Sourced outside the battery, `scratch_corpus` is undefined and the edits
# below would land in this repository. Refuse.
[[ -d "${MD_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
MD_WAR="$REPO_ROOT/${WAR#./}"
md_ok()      { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
md_fail()    { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
md_unknown() { printf 'UNKNOWN %-32s %s\n' "$1" "$2"; }
md_war()     { "$MD_WAR" --root "$MD_ROOT" "$@"; }

# The program: a ticket with a blocker, promoted; a question; a deliverable;
# the promoted Warrant names MD-WAR-0001 as parent and supersedes it.
MD_T=$(md_war create "Plant ticket" -i first -i second 2>/dev/null | awk 'NR==1{print $1}')
MD_I1=$(sed -n 's/^- \[ \] first (\(i-[0-9a-f]*\))$/\1/p' "$MD_ROOT/docs/tickets/$MD_T/atoms/"*checklist* 2>/dev/null)
md_war add "$MD_T" third --after "$MD_I1" >/dev/null 2>&1
md_war promote "$MD_T" >/dev/null 2>&1
md_war ask MD-WAR-0001 STAGE-001 "Which way?" >/dev/null 2>&1
MD_U1=$(sed -n 's/^uuid = "\(.*\)"/\1/p' "$MD_ROOT/docs/warrants/MD-WAR-0001/manifest.toml")
cat >> "$MD_ROOT/docs/warrants/MD-WAR-0002/manifest.toml" <<EOF

[[parents]]
contract_revision = 1
ref = "war://$MD_U1"

[[supersedes]]
ref = "war://$MD_U1"
reason = "plant"
adopts = []
EOF
cat > "$MD_ROOT/docs/warrants/MD-WAR-0001/deliverables.toml" <<'EOF'
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "A plant file"
kind = "file"
target_ref = "README.md"
required = true
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]
EOF

md_war model --json > "$MD_TMP/a.json" 2>"$MD_TMP/a.err"
MD_STATUS=$?
md_war model --json > "$MD_TMP/b.json" 2>/dev/null

# Every kind, by name. The model's own words, read by a JSON parser.
MD_KINDS=$(python3 - "$MD_TMP/a.json" <<'PY' 2>&1
import json, sys
m = json.load(open(sys.argv[1]))["result"]
print("types", " ".join(sorted({r["type"] for r in m["records"]})))
print("kinds", " ".join(sorted({r["kind"] for r in m["relations"]})))
print("diagnostics", len(m["diagnostics"]))
PY
)
MD_WANT_TYPES="deliverable item obligation phase question requirement stage ticket warrant"
MD_WANT_KINDS="depends_on implements parent part_of promoted_to roadmap supersedes"
if [[ $MD_STATUS -eq 0 ]] && grep -qx "types $MD_WANT_TYPES" <<<"$MD_KINDS"; then
    md_ok "model names every record kind" "$MD_WANT_TYPES"
else
    md_fail "model names every record kind" "exit $MD_STATUS; $(head -c 400 <<<"$MD_KINDS") $(head -c 200 "$MD_TMP/a.err")"
fi
if grep -qx "kinds $MD_WANT_KINDS" <<<"$MD_KINDS"; then
    md_ok "model names every relation kind" "$MD_WANT_KINDS"
else
    md_fail "model names every relation kind" "$(head -c 400 <<<"$MD_KINDS")"
fi
if grep -qx "diagnostics 0" <<<"$MD_KINDS"; then
    md_ok "a resolvable program: no diagnostic" "every relation names a record"
else
    md_fail "a resolvable program: no diagnostic" "$(head -c 400 <<<"$MD_KINDS")"
fi

# Determinism: same tree, same bytes, same basis digest.
md_basis() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["basis_digest"])' "$1" 2>/dev/null; }
MD_DA=$(md_basis "$MD_TMP/a.json")
MD_DB=$(md_basis "$MD_TMP/b.json")
if [[ -s "$MD_TMP/a.json" ]] && cmp -s "$MD_TMP/a.json" "$MD_TMP/b.json" && [[ -n "$MD_DA" && "$MD_DA" == "$MD_DB" ]]; then
    md_ok "two runs are byte-identical" "basis $MD_DA"
else
    md_fail "two runs are byte-identical" "basis '$MD_DA' vs '$MD_DB'"
fi

# The differential's own refusal: one byte changed is a difference.
cp "$MD_TMP/a.json" "$MD_TMP/doctored.json"
printf 'x' | dd of="$MD_TMP/doctored.json" bs=1 seek=40 conv=notrunc status=none
if ! cmp -s "$MD_TMP/a.json" "$MD_TMP/doctored.json"; then
    md_ok "a doctored output is a difference" "one byte at offset 40, reported by cmp"
else
    md_fail "a doctored output is a difference" "cmp read the doctored bytes as equal"
fi

# Schema: the model validates; one added field does not.
MD_SCHEMA="$REPO_ROOT/schemas/oh.war/model/v1.json"
MD_VALID=$(python3 - "$MD_SCHEMA" "$MD_TMP/a.json" <<'PY' 2>&1
import json, sys
try:
    import jsonschema
except ImportError:
    print("UNAVAILABLE"); sys.exit(0)
schema = json.load(open(sys.argv[1]))
m = json.load(open(sys.argv[2]))["result"]
def ok(doc):
    try:
        jsonschema.validate(doc, schema); return "valid"
    except jsonschema.ValidationError as e:
        return "invalid"
bad = dict(m); bad["planted"] = True
print("model", ok(m)); print("planted", ok(bad))
PY
)
if grep -qx UNAVAILABLE <<<"$MD_VALID"; then
    md_unknown "the schema validates the model" "python3 jsonschema is not installed, so validation was not asked"
elif grep -qx "model valid" <<<"$MD_VALID" && grep -qx "planted invalid" <<<"$MD_VALID"; then
    md_ok "the schema validates the model" "and refuses one planted field"
else
    md_fail "the schema validates the model" "$(head -c 400 <<<"$MD_VALID")"
fi

# Refusal: a parent naming no Warrant of the corpus is reported, not dropped.
sed -i "s|^ref = \"war://$MD_U1\"$|ref = \"war://01a0ffff-0000-7000-8000-000000000000\"|" \
    "$MD_ROOT/docs/warrants/MD-WAR-0002/manifest.toml"
md_war model --json > "$MD_TMP/c.json" 2>/dev/null
MD_UNK=$(python3 - "$MD_TMP/c.json" <<'PY' 2>&1
import json, sys
m = json.load(open(sys.argv[1]))["result"]
t = "war://01a0ffff-0000-7000-8000-000000000000"
edges = [r for r in m["relations"] if r["to"] == t]
diags = [d for d in m["diagnostics"] if d["rule"] == "model.relation-target-unknown" and t in d["message"]]
print("edges", len(edges), "diagnostics", len(diags))
PY
)
if grep -q "edges [12] diagnostics [12]" <<<"$MD_UNK"; then
    md_ok "an unknown target is a diagnostic" "model.relation-target-unknown; the edge is kept ($MD_UNK)"
else
    md_fail "an unknown target is a diagnostic" "$MD_UNK"
fi

corpus_gone "$MD_ROOT"
rm -rf "$MD_TMP"
