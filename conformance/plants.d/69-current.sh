# shellcheck shell=bash
# OW-WAR-0113 — two projections from atoms by relation (OW-ADR-0022).
#
#   OBL-001  currency is derived; a written currency and a `supersedes` cycle
#            are refused; OW-WAR-0073's signature stands.
#   OBL-002  CURRENT.md is current by construction; HISTORY.md holds the rest.
#   OBL-003  no signing command is handed to a human unjudged.
#   OBL-004  presets type the authored atoms without answering for the author.
#
# Every mutation happens on a scratch program with a key generated here and a
# throwaway ssh-agent holding it (as 96-batch.sh does). This corpus is only
# read. Each positive claim is paired with a refusal, and each refusal is
# checked for WHICH rule fired.

echo "== current by relation, the master document, judged acts, presets (OW-WAR-0113) =="

cu_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cu_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

PLANT_ROOT=$(scratch_corpus CU)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
CU_TMP=$(mktemp -d)
CU_R="$PLANT_ROOT"
cu_commit() { git -C "$CU_R" add -A >/dev/null 2>&1; git -C "$CU_R" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }

# The register: one human, a key made here. Nothing is the owner's.
ssh-keygen -q -t ed25519 -N "" -C plant -f "$CU_TMP/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$CU_TMP/id_plant.pub")" > "$CU_R/docs/authority/allowed_signers"
cat > "$CU_R/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/69-current.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while these plants run."
ssh_principal = "plant"
ROLES

W1=CU-WAR-0001
W2=CU-WAR-0002
W3=CU-WAR-0003
uuid_of() { grep -m1 '^uuid' "$CU_R/docs/warrants/$1/manifest.toml" | cut -d'"' -f2; }

# CU-WAR-0002 supersedes CU-WAR-0001; CU-WAR-0003 is an unrelated draft.
"$WAR" --root "$CU_R" new "The successor" --preset feature >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war new --preset feature\n' >&2; exit 9; }
"$WAR" --root "$CU_R" new "Another draft" --preset fix >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war new --preset fix\n' >&2; exit 9; }
[[ -d "$CU_R/docs/warrants/$W3" ]] || { printf 'PLANT SETUP FAILED: %s missing\n' "$W3" >&2; exit 9; }
printf '\n[[supersedes]]\nref = "war://%s"\nreason = "The plant replaces the scaffold."\nadopts = []\n' "$(uuid_of $W1)" >> "$CU_R/docs/warrants/$W2/manifest.toml"
"$WAR" --root "$CU_R" compile >/dev/null 2>&1
cu_commit "register, a successor, a draft"

# ---------------------------------------------------------------- OBL-001 --
# Before the successor is authorized its relation is on record and NOT in
# force: CU-WAR-0001 still reads current. The refusal half of "an authorized
# successor supersedes".
CU_OUT=$("$WAR" --root "$CU_R" check 2>&1)
if grep -qE "PASS relations.currency .*$W2: declares .supersedes. → $W1, not in force" <<<"$CU_OUT" \
    && ! grep -qE "PASS relations.currency .*$W1: superseded" <<<"$CU_OUT"; then
    cu_ok "an unauthorized successor" "$W1 still current; the relation is not in force"
else
    cu_fail "an unauthorized successor" "$(grep 'relations.currency' <<<"$CU_OUT" | head -2 | tr '\n' '|')"
fi

CU_OLD_SOCK=${SSH_AUTH_SOCK:-}
CU_OLD_PID=${SSH_AGENT_PID:-}
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$CU_TMP/id_plant" 2>/dev/null
CU_SIGN=$("$WAR" --root "$CU_R" sign "$W2" --ssh-sign --as "Plant Signer" </dev/null 2>&1)
[[ -f "$CU_R/docs/warrants/$W2/authorization.toml" ]] \
    || { printf 'PLANT SETUP FAILED: %s was not authorized\n%s\n' "$W2" "$CU_SIGN" >&2; ssh-agent -k >/dev/null 2>&1; exit 9; }
"$WAR" --root "$CU_R" compile >/dev/null 2>&1
cu_commit "the successor is authorized"

CU_OUT=$("$WAR" --root "$CU_R" check 2>&1)
if grep -qE "PASS relations.currency .*$W1: superseded, derived from $W2" <<<"$CU_OUT" \
    && ! grep -q '^currency' "$CU_R/docs/warrants/$W1/manifest.toml" \
    && ! grep -q 'relations.currency-authored' <<<"$CU_OUT"; then
    cu_ok "currency derived from the relation" "$W1 superseded by $W2, no field written"
else
    cu_fail "currency derived from the relation" "$(grep 'relations.currency' <<<"$CU_OUT" | head -3 | tr '\n' '|')"
fi

plant "a written currency is refused" "relations.currency-authored" "$W1: the manifest writes .currency = \"superseded\"" 2 \
    "sed -i 's/^profile = /currency = \"superseded\"\nprofile = /' docs/warrants/$W1/manifest.toml; assert_present 'currency = \"superseded\"' docs/warrants/$W1/manifest.toml"

plant "a supersedes cycle is refused" "relations.currency-cycle" "$W1 → $W2 → $W1" 2 \
    "printf '\n[[supersedes]]\nref = \"war://$(uuid_of $W2)\"\nreason = \"a cycle\"\nadopts = []\n' >> docs/warrants/$W1/manifest.toml; assert_present 'reason = \"a cycle\"' docs/warrants/$W1/manifest.toml"

# On this corpus: OW-WAR-0073's manifest carries no written currency, the
# derivation says superseded, and its revision-1 signature stands.
REPO_OUT=$("$WAR" check 2>&1)
if ! grep -q '^currency' docs/warrants/OW-WAR-0073/manifest.toml \
    && grep -qE 'PASS relations.currency .*OW-WAR-0073: superseded, derived from OW-WAR-0112' <<<"$REPO_OUT" \
    && ! grep -qE 'relations.currency-authored' <<<"$REPO_OUT"; then
    cu_ok "OW-WAR-0073 reads superseded" "derived from OW-WAR-0112; no field in its manifest"
else
    cu_fail "OW-WAR-0073 reads superseded" "$(grep -E 'OW-WAR-0073' <<<"$REPO_OUT" | grep -E 'currency' | head -2 | tr '\n' '|')"
fi
DRY_ALL=$("$WAR" sign --all --dry-run 2>&1)
AUTH_0073=$("$WAR" authorize OW-WAR-0073 2>&1)
if ! grep -E 'authorize.no-amendment' <<<"$DRY_ALL" | grep -q 'OW-WAR-0073' \
    && grep -q 'contract_digest = "691f51ce' <<<"$AUTH_0073"; then
    cu_ok "OW-WAR-0073's signature stands" "no authorize.no-amendment; contract 691f51ce…"
else
    cu_fail "OW-WAR-0073's signature stands" "$(grep -E 'OW-WAR-0073' <<<"$DRY_ALL" | head -2 | tr '\n' '|') $(grep contract_digest <<<"$AUTH_0073")"
fi

# ---------------------------------------------------------------- OBL-002 --
CUR="$CU_R/docs/generated/CURRENT.md"
W1_INTENT_LINE=$(sed -n '/^---$/,/^---$/!p' "$CU_R/docs/warrants/$W1/atoms/10-intent.md" | grep -m1 -E '^[A-Za-z]{4}')
# Named once as a subject: the lineage line. The only other place a replaced
# alias may appear is a table row of a CURRENT record that names it — here
# the queue, which still lists the scaffold's own authorization.
if [[ -f "$CUR" ]] && grep -qxF -- "- $W1 → $W2" "$CUR" \
    && [[ $(grep -v '^|' "$CUR" | grep -o "$W1" | wc -l) -eq 1 ]] \
    && ! grep -qE "^#+ .*$W1" "$CUR" \
    && [[ -n "$W1_INTENT_LINE" ]] && ! grep -qF -- "$W1_INTENT_LINE" "$CUR"; then
    cu_ok "a replaced subject is one line" "$W1 once, as lineage; none of its text"
else
    cu_fail "a replaced subject is one line" "$(grep -v '^|' "$CUR" | grep "$W1" | head -3 | tr '\n' '|')"
fi

# history = false (the default for a new program): no HISTORY.md, no drift.
if [[ ! -e "$CU_R/docs/generated/HISTORY.md" ]] \
    && ! "$WAR" --root "$CU_R" check --generated 2>&1 | grep -q 'HISTORY.md'; then
    cu_ok "history off writes no history" "no HISTORY.md, nothing compared"
else
    cu_fail "history off writes no history" "HISTORY.md exists or was compared"
fi

plant "a hand edit to CURRENT.md is drift" "generated.drift" "CURRENT.md" 2 \
    "printf 'claimed by hand\n' >> docs/generated/CURRENT.md; assert_present 'claimed by hand' docs/generated/CURRENT.md" \
    --generated

# history = true: HISTORY.md holds the replaced subject's text, and a hand
# edit to it is drift too.
if grep -q '^history = ' "$CU_R/openwarrant.toml"; then
    sed -i 's/^history = .*$/history = true/' "$CU_R/openwarrant.toml"
else
    sed -i 's/^verify_drift = true$/verify_drift = true\nhistory = true/' "$CU_R/openwarrant.toml"
fi
grep -q '^history = true' "$CU_R/openwarrant.toml" || { printf 'PLANT SETUP FAILED: history key not set\n' >&2; exit 9; }
"$WAR" --root "$CU_R" compile >/dev/null 2>&1
cu_commit "history on"
HIS="$CU_R/docs/generated/HISTORY.md"
if [[ -f "$HIS" ]] && grep -qF -- "$W1_INTENT_LINE" "$HIS" && grep -qF -- "- $W1 → $W2" "$HIS"; then
    cu_ok "the history holds what was replaced" "$W1's intent verbatim, and its lineage"
else
    cu_fail "the history holds what was replaced" "HISTORY.md missing or without $W1's text"
fi
plant "a hand edit to HISTORY.md is drift" "generated.drift" "HISTORY.md" 2 \
    "printf 'claimed by hand\n' >> docs/generated/HISTORY.md; assert_present 'claimed by hand' docs/generated/HISTORY.md" \
    --generated

# On this corpus (read only): every atom of every current Warrant appears in
# CURRENT.md verbatim; OW-WAR-0073 is one lineage line outside those atoms;
# none of 0073's paragraphs appears except inside a current atom that carries
# it; HISTORY.md holds 0073's intent verbatim.
if python3 - <<'PY'
import pathlib, re, sys, tomllib
root = pathlib.Path(".")
cur = (root / "docs/generated/CURRENT.md").read_text()
his = (root / "docs/generated/HISTORY.md").read_text()
def body(text):
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end >= 0:
            return text[end + 5:].lstrip("\n")
    return text
replaced = set(re.findall(r"^- (\S+) → ", cur.split("## Replaced", 1)[1].split("\n## ", 1)[0], re.M))
assert "OW-WAR-0073" in replaced, "0073 not under Replaced"
current_bodies = []
for m in sorted(root.glob("docs/warrants/*/manifest.toml")):
    man = tomllib.loads(m.read_text())
    alias = man["local_alias"]
    for a in man.get("atoms", []):
        b = body((m.parent / a["path"]).resolve().read_text())
        if alias in replaced:
            continue
        assert b in cur, f"{alias}: {a['path']} not verbatim in CURRENT.md"
        current_bodies.append(b)
# Accepted ADRs are atoms too, rendered verbatim under *Decisions*.
for adr in sorted(root.glob("docs/adr/atoms/*.md")):
    t = adr.read_text()
    if "\nstatus: accepted\n" in t.split("\n---\n", 1)[0] + "\n":
        current_bodies.append(body(t).rstrip())
rest = cur
for b in sorted(set(current_bodies), key=len, reverse=True):
    rest = rest.replace(b, "")
# Named as a subject exactly once — the lineage line. Outside verbatim atoms
# the only other mentions allowed are table rows of CURRENT records that name
# the path (OW-WAR-0113's own D-005, and who governs that path now).
prose = "\n".join(l for l in rest.splitlines() if not l.startswith("|"))
assert prose.count("OW-WAR-0073") == 1, [l for l in prose.splitlines() if "OW-WAR-0073" in l]
assert "- OW-WAR-0073 → OW-WAR-0112" in prose
assert not re.search(r"^#+ .*OW-WAR-0073", rest, re.M), "0073 has a heading"
assert all("docs/warrants/OW-WAR-0073/" in l for l in rest.splitlines() if l.startswith("|") and "OW-WAR-0073" in l)
old = root / "docs/warrants/OW-WAR-0073"
for a in tomllib.loads((old / "manifest.toml").read_text())["atoms"]:
    for para in body((old / a["path"]).read_text()).split("\n\n"):
        para = para.strip()
        if len(para) < 40 or para not in cur:
            continue
        assert any(para in b for b in current_bodies), f"0073 text outside a current atom: {para[:60]!r}"
intent = body((old / "atoms/10-intent.md").read_text())
assert intent in his, "0073's intent not verbatim in HISTORY.md"
PY
then
    cu_ok "CURRENT.md on this corpus" "every current atom verbatim; 0073 one lineage line"
else
    cu_fail "CURRENT.md on this corpus" "see the assertion above"
fi
# The same verbatim test must be able to fail: an atom body the document does
# not carry is not found.
if ! grep -qF -- "an atom this corpus never wrote $$" docs/generated/CURRENT.md; then
    cu_ok "the verbatim test can fail" "a body CURRENT.md lacks is not found"
else
    cu_fail "the verbatim test can fail" "found text nobody wrote"
fi
if grep -qE 'PASS generated.drift .*CURRENT.md matches' <<<"$("$WAR" check --generated 2>&1)"; then
    cu_ok "this corpus's CURRENT.md is fresh" "generated.drift passes"
else
    cu_fail "this corpus's CURRENT.md is fresh" "no generated.drift pass for CURRENT.md"
fi

# The four pointing documents name CURRENT.md as the first thing to read;
# CONTRIBUTING.md, which was not asked to, does not — the test can fail.
points_first() { python3 - "$1" <<'PY'
import sys
lines = open(sys.argv[1]).read().splitlines()
for i, l in enumerate(lines):
    if "docs/generated/CURRENT.md" in l:
        window = " ".join(lines[max(0, i - 2):i + 2]).lower()
        sys.exit(0 if "first" in window else 1)
sys.exit(1)
PY
}
CU_MISSING=""
for doc in README.md QUICKSTART.md AGENTS.md docs/TUI.md; do
    points_first "$doc" || CU_MISSING="$CU_MISSING $doc"
done
if [[ -z "$CU_MISSING" ]] && ! points_first CONTRIBUTING.md; then
    cu_ok "four documents point at CURRENT.md" "README, QUICKSTART, AGENTS, TUI: read it first"
else
    cu_fail "four documents point at CURRENT.md" "missing:$CU_MISSING"
fi
TUI_SRC=crates/openwarrant-cli/src/tui/mod.rs
if [[ $(grep -n 'current::CURRENT_PATH' "$TUI_SRC" | head -1 | cut -d: -f1) -lt $(grep -n 'for name in \["AGENTS.md"' "$TUI_SRC" | cut -d: -f1) ]]; then
    cu_ok "the app's Help lists it first" "CURRENT.md before every other document"
else
    cu_fail "the app's Help lists it first" "load_docs order"
fi

# ---------------------------------------------------------------- OBL-003 --
# CU-WAR-0001 and CU-WAR-0003 await authorization and would record. Moving
# the authorized CU-WAR-0002's contract makes its revision 2 refusable
# (`authorize.no-amendment`), and it must then sort after both recordable
# acts though its alias sits between them.
printf '\nThe problem, answered after signing.\n' >> "$CU_R/docs/warrants/$W2/atoms/10-intent.md"
cu_commit "the successor's contract moves"
CU_BEFORE=$(git -C "$CU_R" status --porcelain | sort)
NEXT_JSON=$("$WAR" --root "$CU_R" --json next 2>&1)
NEXT_TXT=$("$WAR" --root "$CU_R" next 2>&1)
CU_AFTER=$(git -C "$CU_R" status --porcelain | sort)
if python3 -c '
import json, sys
d = json.loads(sys.argv[1]); r = d.get("result", d)
acts = [a for a in r["actions"] if a["command"].startswith("war sign")]
assert acts, "no signing act"
for a in acts:
    j = a.get("judged") or {}
    assert j.get("verdict") in ("would_record", "would_refuse"), a
    if j["verdict"] == "would_refuse":
        assert j.get("rule"), a
order = [(a["warrant"], a["judged"]["verdict"]) for a in acts]
refused = [w for w, v in order if v == "would_refuse"]
assert refused == ["CU-WAR-0002"], order
assert any(a["judged"].get("rule") == "authorize.no-amendment" for a in acts if a["warrant"] == "CU-WAR-0002"), order
first_refused = [v for _, v in order].index("would_refuse")
assert all(v == "would_refuse" for _, v in order[first_refused:]), order
assert "CU-WAR-0003" in [w for w, _ in order[:first_refused]], order
' "$NEXT_JSON" \
    && grep -E "war sign $W2 +\[would refuse: authorize.no-amendment\]" <<<"$NEXT_TXT" >/dev/null; then
    cu_ok "every signing act is judged" "the refusable one last, its rule beside the command"
else
    cu_fail "every signing act is judged" "$(grep -E 'war sign' <<<"$NEXT_TXT" | tr '\n' '|')"
fi
if [[ "$CU_BEFORE" == "$CU_AFTER" ]] && ! grep -q 'ssh-keygen\|SSH_AUTH_SOCK' <<<"$NEXT_JSON$NEXT_TXT" \
    && [[ -z "$(ls -d /tmp/war-dry-run-* 2>/dev/null)" ]]; then
    cu_ok "war next writes nothing" "tree unchanged, no draft left, no key named"
else
    cu_fail "war next writes nothing" "tree moved, a draft survived, or ssh was named"
fi
# The judged queue is what CURRENT.md carries under *Awaiting a human*.
"$WAR" --root "$CU_R" compile >/dev/null 2>&1
if grep -E "^\| authorize \| $W2 \| .war sign $W2. \| would refuse: authorize.no-amendment \|" "$CUR" >/dev/null; then
    cu_ok "CURRENT.md carries the verdict" "$W2: would refuse, by rule"
else
    cu_fail "CURRENT.md carries the verdict" "$(grep -A8 '## Awaiting a human' "$CUR" | tr '\n' '|')"
fi
git -C "$CU_R" reset --hard -q HEAD~1

# ---------------------------------------------------------------- OBL-004 --
# `war new --preset` writes one atom per role the profile requires, each
# heading with the question it asks.
PRE_DIR="$CU_R/docs/warrants/$W3"
PRE_MISSING=""
for f in 10-intent.md 20-basis.md 40-work-order.md 45-milestones.yaml 60-assurance.md; do
    [[ -f "$PRE_DIR/atoms/$f" ]] || PRE_MISSING="$PRE_MISSING $f"
    [[ "$f" == *.md ]] && ! grep -q '^<!-- required -->$' "$PRE_DIR/atoms/$f" && PRE_MISSING="$PRE_MISSING $f(no question)"
    grep -q 'TODO' "$PRE_DIR/atoms/$f" 2>/dev/null && PRE_MISSING="$PRE_MISSING $f(TODO)"
done
if [[ -z "$PRE_MISSING" ]] && [[ $(grep -c 'path = "atoms/' "$PRE_DIR/manifest.toml") -eq 5 ]]; then
    cu_ok "a preset writes typed atoms" "five roles, each heading asks its question"
else
    cu_fail "a preset writes typed atoms" "$PRE_MISSING"
fi

# Without --preset: the presets are named and no TODO skeleton is written.
# (The contract's refusal here is NOT delivered; see OW-WAR-0113's report.)
NEW_OUT=$("$WAR" --root "$CU_R" new "x" 2>&1)
NEW_ALIAS=$(grep -oE 'CU-WAR-[0-9]{4}' <<<"$NEW_OUT" | head -1)
if grep -q 'feature, fix, decision' <<<"$NEW_OUT" && [[ -n "$NEW_ALIAS" ]] \
    && ! grep -rq 'TODO' "$CU_R/docs/warrants/$NEW_ALIAS/atoms/"; then
    cu_ok "war new without a preset" "names feature, fix, decision; no TODO"
else
    cu_fail "war new without a preset" "$NEW_OUT"
fi
git -C "$CU_R" clean -fdq
plant_cmd "an unknown preset is refused" "new.unknown-preset" "feature, fix, decision" 1 ":" new "x" --preset nonsense
plant_cmd "a preset's profile is its own" "new.preset-profile" "decision" 1 ":" new "x" --preset decision --profile delivery

plant "an unanswered heading warns a draft" "WARN atom.preset-unanswered" "$W3.*Problem" 0 ":" "$W3"

# Answered required headings and a deleted optional one pass.
PRE_OUT=$(cd "$CU_R" && python3 -c "import re,sys; p='docs/warrants/$W3/atoms/10-intent.md'; t=open(p).read(); t=t.replace('## Problem\n<!-- required -->\n','## Problem\n<!-- required -->\nThe parser drops a key.\n').replace('## Desired Outcome\n<!-- required -->\n','## Desired Outcome\n<!-- required -->\nIt refuses instead.\n'); t=t[:t.index('## Non-goals')]; open(p,'w').write(t)") && PRE_OUT=$("$WAR" --root "$CU_R" check "$W3" 2>&1)
if ! grep -q "atom.preset-unanswered .*10-intent.md" <<<"$PRE_OUT" && grep -q "atom.preset-unanswered .*20-basis.md" <<<"$PRE_OUT"; then
    cu_ok "an answered atom is not named" "10-intent.md passes; 20-basis.md still asks"
else
    cu_fail "an answered atom is not named" "$(grep 'preset-unanswered' <<<"$PRE_OUT" | head -3 | tr '\n' '|')"
fi
git -C "$CU_R" checkout -q -- .

# Once an authorization is on record, the same unanswered heading is an error.
"$WAR" --root "$CU_R" sign "$W3" --ssh-sign --as "Plant Signer" </dev/null >/dev/null 2>&1
if [[ -f "$PRE_DIR/authorization.toml" ]]; then
    cu_commit "the draft is authorized unanswered"
    plant "an unanswered heading, once signed" "ERROR atom.preset-unanswered" "$W3.*Problem" 2 ":"
else
    cu_fail "an unanswered heading, once signed" "setup: $W3 could not be authorized"
fi

# A role no projection renders is refused by name.
plant "an unprojected role is refused" "atom.role-unprojected" "ext.notes" 2 \
    "printf '\n[[atoms]]\nordinal = 70\nrole = \"ext.notes\"\npath = \"atoms/70-notes.md\"\nrequired = false\n' >> docs/warrants/$W3/manifest.toml; printf -- '---\nschema: oh.war/atom/v1\nwarrant_uuid: %s\nrole: ext.notes\njurisdiction: authored\norder: 70\nclassification: internal\n---\n\n# Notes\n' \"$(uuid_of $W3)\" > docs/warrants/$W3/atoms/70-notes.md; assert_present 'ext.notes' docs/warrants/$W3/manifest.toml"

ssh-agent -k >/dev/null 2>&1
if [[ -n "$CU_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$CU_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
if [[ -n "$CU_OLD_PID" ]]; then export SSH_AGENT_PID="$CU_OLD_PID"; else unset SSH_AGENT_PID; fi
rm -rf "$CU_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
