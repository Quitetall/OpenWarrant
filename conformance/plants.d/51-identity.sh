# shellcheck shell=bash
# OW-WAR-0119 — identity is the UUIDv7, and no alias stands in for it.
#
# Every refusal is planted in a scratch program (`ID`), never in this
# repository; the corpus is only READ, for the positive run and for the
# warnings OBL-004 and OBL-005 say it must list. Each plant asserts the
# severity, the rule and the detail on ONE diagnostic, from `war check
# --json`, so a finding from a neighbouring rule cannot stand in for it.

echo "== identity (OW-WAR-0119) =="
PLANT_ROOT=$(scratch_corpus ID)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ID_W="$PLANT_ROOT/docs/warrants/ID-WAR-0001"
ID_UUID=$(sed -n 's/^uuid = "\(.*\)"$/\1/p' "$ID_W/manifest.toml")
[[ -n "$ID_UUID" ]] || { printf 'PLANT SETUP FAILED: no uuid in %s\n' "$ID_W/manifest.toml" >&2; exit 9; }
# Another UUIDv7: the last hex digit moved, the version nibble untouched.
ID_OTHER="${ID_UUID%?}$([[ "${ID_UUID: -1}" == 0 ]] && echo 1 || echo 0)"
ID_V4="7c1f7f7e-4b1a-4d2a-9d3e-2b8f0a6c5e11"
ID_TMP=$(mktemp -d)

# id_count <json> <severity> <rule> [detail...]  ->  how many diagnostics carry
# that severity and rule and every detail string in their file or message.
# The JSON goes in on stdin: the corpus's report is larger than an argv may be.
# Unparseable output counts -1, which neither id_expect nor id_none accepts.
ID_COUNT_PY='
import json, sys
severity, rule, *details = sys.argv[1:]
try:
    diagnostics = json.loads(sys.stdin.read())["diagnostics"]
except (ValueError, KeyError):
    print(-1)
    sys.exit()
print(sum(
    1 for d in diagnostics
    if d["severity"] == severity and d["rule"] == rule
    and all(x in (d.get("file") or "") + " " + d["message"] for x in details)
))
'
id_count() {
    local json="$1"
    shift
    python3 -c "$ID_COUNT_PY" "$@" <<<"$json"
}

# id_expect <name> <exit> <want-exit> <json> <severity> <rule> [detail...]
id_expect() {
    local name="$1" status="$2" want="$3" out="$4"
    shift 4
    local n
    n=$(id_count "$out" "$@")
    if [[ "$status" == "$want" && "$n" =~ ^[1-9][0-9]*$ ]]; then
        printf 'ok    %-34s %s %s (exit %s)\n' "$name" "$1" "$2" "$status"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted %s %s [%s] exit %s; got %s such, exit %s\n' "$name" "$1" "$2" "${*:3}" "$want" "$n" "$status"; FAILED=$((FAILED + 1))
    fi
}

# id_none <name> <json> <severity> <rule>  ->  the refusal's other half.
id_none() {
    local n
    n=$(id_count "$2" "$3" "$4")
    if [[ "$n" == 0 ]]; then
        printf 'ok    %-34s no %s %s\n' "$1" "$3" "$4"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s %s %s time(s)\n' "$1" "$n" "$3" "$4"; FAILED=$((FAILED + 1))
    fi
}

id_check() { "$WAR" --root "${1:-$PLANT_ROOT}" --json check 2>/dev/null; }

id_adr() { # <file> <uuid> <alias> <governs>
    cat > "$PLANT_ROOT/docs/adr/atoms/$1" <<ADR
---
adr_uuid: $2
local_alias: $3
status: accepted
governs:
  - "$4"
---

# $3: a plant
ADR
}

# ── Positive: the clean program, and this repository's corpus ───────────────
ID_OUT=$(id_check); ID_STATUS=$?
if [[ "$ID_STATUS" -eq 0 ]] && [[ $(id_count "$ID_OUT" pass identity.changed) =~ ^[1-9] ]]; then
    printf 'ok    %-34s exit 0, identity rules ran\n' "the clean program passes"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s\n' "the clean program passes" "$ID_STATUS"; FAILED=$((FAILED + 1))
fi
for rule in duplicate-uuid atom-mismatch changed alias-ref adr-not-v7; do
    id_none "clean program: no $rule" "$ID_OUT" error "identity.$rule"
done

ID_CORPUS=$("$WAR" --json check 2>/dev/null)
for rule in duplicate-uuid atom-mismatch changed alias-ref adr-not-v7; do
    id_none "corpus: no $rule error" "$ID_CORPUS" error "identity.$rule"
done

# ── OBL-001 — one UUID names one Warrant, and one ADR ───────────────────────
cp -r "$ID_W" "$PLANT_ROOT/docs/warrants/ID-WAR-0002"
sed -i 's/^local_alias = "ID-WAR-0001"/local_alias = "ID-WAR-0002"/' "$PLANT_ROOT/docs/warrants/ID-WAR-0002/manifest.toml"
assert_present 'local_alias = "ID-WAR-0002"' "$PLANT_ROOT/docs/warrants/ID-WAR-0002/manifest.toml"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "a copied Warrant keeps its UUID" "$ID_STATUS" 2 "$ID_OUT" error identity.duplicate-uuid "$ID_UUID" ID-WAR-0001 ID-WAR-0002
corpus_reset "$PLANT_ROOT"

mkdir -p "$PLANT_ROOT/docs/adr/atoms"
id_adr ID-ADR-0901-a.md "$ID_UUID" ID-ADR-0901 "war://$ID_UUID"
id_adr ID-ADR-0902-b.md "$ID_UUID" ID-ADR-0902 "war://$ID_UUID"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "two ADRs with one adr_uuid" "$ID_STATUS" 2 "$ID_OUT" error identity.duplicate-uuid ID-ADR-0901 ID-ADR-0902
id_adr ID-ADR-0902-b.md "$ID_OTHER" ID-ADR-0902 "war://$ID_UUID"
ID_OUT=$(id_check)
id_none "two ADRs, two adr_uuids" "$ID_OUT" error identity.duplicate-uuid
corpus_reset "$PLANT_ROOT"

# ── OBL-002 — an atom belongs to its manifest's Warrant ─────────────────────
sed -i "s/^warrant_uuid: $ID_UUID\$/warrant_uuid: $ID_OTHER/" "$ID_W/atoms/10-intent.md"
assert_present "warrant_uuid: $ID_OTHER" "$ID_W/atoms/10-intent.md"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "an atom names another Warrant" "$ID_STATUS" 2 "$ID_OUT" error identity.atom-mismatch atoms/10-intent.md "$ID_OTHER"
sed -i "s/^warrant_uuid: $ID_OTHER\$/warrant_uuid: $ID_UUID/" "$ID_W/atoms/10-intent.md"
ID_OUT=$(id_check)
id_none "the atom restored" "$ID_OUT" error identity.atom-mismatch
corpus_reset "$PLANT_ROOT"

# ── OBL-003 — a committed UUID does not change ──────────────────────────────
sed -i "s/^uuid = \"$ID_UUID\"/uuid = \"$ID_OTHER\"/" "$ID_W/manifest.toml"
assert_present "uuid = \"$ID_OTHER\"" "$ID_W/manifest.toml"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "a committed UUID replaced" "$ID_STATUS" 2 "$ID_OUT" error identity.changed "$ID_UUID" "$ID_OTHER"
# The same edit where git cannot answer: a copy of the program with no .git.
cp -r "$PLANT_ROOT" "$ID_TMP/nogit" && command rm -rf "$ID_TMP/nogit/.git"
ID_OUT=$(id_check "$ID_TMP/nogit"); ID_STATUS=$?
id_expect "no git repository: unknown" "$ID_STATUS" 2 "$ID_OUT" unknown identity.changed "not inside a git repository"
id_none "no git repository: never a pass" "$ID_OUT" pass identity.changed
corpus_reset "$PLANT_ROOT"

"$WAR" --root "$PLANT_ROOT" new "A Warrant nobody has committed" >/dev/null 2>&1
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "an uncommitted Warrant is new" "$ID_STATUS" 0 "$ID_OUT" pass identity.changed "1 new"
id_none "an uncommitted Warrant: no change" "$ID_OUT" error identity.changed
corpus_reset "$PLANT_ROOT"

# ── OBL-004 — an alias is refused where an identity is required ─────────────
"$WAR" --root "$PLANT_ROOT" new "A Warrant that names another" >/dev/null 2>&1
ID_W2="$PLANT_ROOT/docs/warrants/ID-WAR-0002"
[[ -f "$ID_W2/manifest.toml" ]] || { printf 'PLANT SETUP FAILED: war new made no ID-WAR-0002\n' >&2; exit 9; }
cp "$ID_W2/manifest.toml" "$ID_TMP/manifest.toml"
printf '\n[[supersedes]]\nref = "war://ID-WAR-0001"\nreason = "a plant"\n' >> "$ID_W2/manifest.toml"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "[[supersedes]] by alias" "$ID_STATUS" 2 "$ID_OUT" error identity.alias-ref "[[supersedes]]" "war://$ID_UUID"
cp "$ID_TMP/manifest.toml" "$ID_W2/manifest.toml"
printf '\n[[supersedes]]\nref = "war://%s"\nreason = "a plant"\n' "$ID_UUID" >> "$ID_W2/manifest.toml"
ID_OUT=$(id_check)
id_none "[[supersedes]] by UUID" "$ID_OUT" error identity.alias-ref
cp "$ID_TMP/manifest.toml" "$ID_W2/manifest.toml"
printf '\n[[parents]]\nref = "war://ID-WAR-0001"\ncontract_revision = 1\n' >> "$ID_W2/manifest.toml"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "[[parents]] by alias" "$ID_STATUS" 2 "$ID_OUT" error identity.alias-ref "[[parents]]" "war://$ID_UUID"
cp "$ID_TMP/manifest.toml" "$ID_W2/manifest.toml"
printf '\n[[parents]]\nref = "war://%s"\ncontract_revision = 1\n' "$ID_UUID" >> "$ID_W2/manifest.toml"
ID_OUT=$(id_check)
id_none "[[parents]] by UUID" "$ID_OUT" error identity.alias-ref
corpus_reset "$PLANT_ROOT"

mkdir -p "$PLANT_ROOT/docs/adr/atoms"
id_adr ID-ADR-0901-a.md "$ID_OTHER" ID-ADR-0901 "war://ID-WAR-0001"
ID_OUT=$(id_check)
id_expect "an ADR governs by alias" 0 0 "$ID_OUT" warn identity.alias-ref ID-ADR-0901 "war://$ID_UUID"
id_none "an ADR governs by alias: no error" "$ID_OUT" error identity.alias-ref
corpus_reset "$PLANT_ROOT"

# The six alias-form `governs` in this corpus, listed as warnings — counted
# from the ADR atoms themselves, not from the tool that is being tested.
ID_GOVERNS=$(grep -lE '^  - "?war://[A-Z]+-WAR-[0-9]+' docs/adr/atoms/*.md | sort | tr '\n' ' ')
ID_LISTED=$(python3 -c '
import json, sys
d = json.loads(sys.stdin.read())["diagnostics"]
print(" ".join(sorted({x["file"] for x in d if x["rule"] == "identity.alias-ref" and x["severity"] == "warn"})) + " ")
' <<<"$ID_CORPUS")
if [[ -n "${ID_GOVERNS// }" && "$ID_LISTED" == "$ID_GOVERNS" ]]; then
    printf 'ok    %-34s %s ADR(s) warned\n' "corpus: alias-form governs listed" "$(wc -w <<<"$ID_GOVERNS")"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s atoms: %s; warned: %s\n' "corpus: alias-form governs listed" "$ID_GOVERNS" "$ID_LISTED"; FAILED=$((FAILED + 1))
fi

# §49.2's stage identity, as `war blut` reports it on a Warrant of this
# corpus (read only; nothing is emitted). `blut.rs`'s unit test asserts the
# same thing by parsing it; this is the binary saying it.
ID_BLUT_UUID=$(sed -n 's/^uuid = "\(.*\)"$/\1/p' docs/warrants/OW-WAR-0047/manifest.toml)
ID_OUT=$("$WAR" blut OW-WAR-0047 2>&1)
if grep -q "blut.lowered.*stage identity war://$ID_BLUT_UUID" <<<"$ID_OUT" && ! grep -q 'war://OW-WAR-0047' <<<"$ID_OUT"; then
    printf 'ok    %-34s war://%s\n' "blut names the stage by UUID" "$ID_BLUT_UUID"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "blut names the stage by UUID" "$(grep -E 'blut\.|war://' <<<"$ID_OUT" | head -2 | tr '\n' '|')"; FAILED=$((FAILED + 1))
fi

# ── The v4 manifest UUID is still refused by the rule that always did ───────
sed -i "s/^uuid = \"$ID_UUID\"/uuid = \"$ID_V4\"/" "$ID_W/manifest.toml"
assert_present "uuid = \"$ID_V4\"" "$ID_W/manifest.toml"
ID_OUT=$(id_check); ID_STATUS=$?
id_expect "a v4 manifest UUID" "$ID_STATUS" 2 "$ID_OUT" error manifest.invalid "UUIDv7"
corpus_reset "$PLANT_ROOT"

# ── OBL-005 — ADR identities are reported, never rewritten ──────────────────
# The v4 set is computed here from the atoms, independently of the rule.
ID_V4_ADRS=$(python3 -c '
import glob, re, uuid
out = []
for f in sorted(glob.glob("docs/adr/atoms/*.md")):
    m = re.search(r"^adr_uuid:\s*(\S+)", open(f).read(), re.M)
    if m and uuid.UUID(m.group(1)).version != 7:
        out.append(f)
print(" ".join(out) + " ")
')
ID_WARNED=$(python3 -c '
import json, sys
d = json.loads(sys.stdin.read())["diagnostics"]
w = [x["file"] for x in d if x["rule"] == "identity.adr-not-v7" and x["severity"] == "warn"]
print(" ".join(sorted(w)) + (" " if len(w) == len(set(w)) else " DUPLICATED"))
' <<<"$ID_CORPUS")
if [[ -n "${ID_V4_ADRS// }" && "$ID_WARNED" == "$ID_V4_ADRS" ]]; then
    printf 'ok    %-34s %s v4 ADR(s), once each, no v7\n' "corpus: adr-not-v7 one by one" "$(wc -w <<<"$ID_V4_ADRS")"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s v4: %s; warned: %s\n' "corpus: adr-not-v7 one by one" "$ID_V4_ADRS" "$ID_WARNED"; FAILED=$((FAILED + 1))
fi
if git -C "$REPO_ROOT" diff --quiet HEAD -- docs/adr/; then
    printf 'ok    %-34s git diff over docs/adr/ is empty\n' "no ADR identity rewritten"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s docs/adr/ differs from HEAD\n' "no ADR identity rewritten"; FAILED=$((FAILED + 1))
fi

command rm -rf "$ID_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
