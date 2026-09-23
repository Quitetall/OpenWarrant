# shellcheck shell=bash
# OW-WAR-0112 — ownership (OW-ADR-0021) and the dry run.
#
# Every plant here runs on a scratch program or reads this corpus without
# mutating it. The dry-run plants come first because they are what an agent
# reaches for before asking a human to sign; the ownership plants follow in
# M1 phase B, once the drift rule itself lands in check.rs.

PLANT_ROOT=$(scratch_corpus DR)
# Sourced outside plant.sh, `scratch_corpus` is undefined and PLANT_ROOT is
# empty, and every `git -C "$PLANT_ROOT"` below would then act on the real
# repository — it did once, committing a working tree as "plant". Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
W1=DR-WAR-0001
# A scaffold ships the authority EXAMPLES only — nobody is eligible to sign,
# and a dry run rightly stops at `sign.who`. The plant needs an eligible
# human on the register, so the example becomes the register here. This is a
# throwaway program: the write-once rule for these files is about this
# repository's, not a scratch's.
cp "$PLANT_ROOT/docs/authority/roles.toml.example" "$PLANT_ROOT/docs/authority/roles.toml"
cp "$PLANT_ROOT/docs/authority/allowed_signers.example" "$PLANT_ROOT/docs/authority/allowed_signers"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register" >/dev/null 2>&1

# A fresh scaffold's adopt Warrant awaits authorization. A dry run must say
# it WOULD record, and must leave no trace: no response, no draft, no journal
# line, no attestation, and a tree byte-identical to before.
DR_BEFORE=$(git -C "$PLANT_ROOT" status --porcelain | sort)
# The example register names two humans, and `sign` rightly refuses to pick
# one; the plant picks, as an operator would.
DR_OUT=$("$WAR" --root "$PLANT_ROOT" sign "$W1" --dry-run --as your-name-here 2>&1)
DR_STATUS=$?
DR_AFTER=$(git -C "$PLANT_ROOT" status --porcelain | sort)
if [[ $DR_STATUS -eq 0 ]] && grep -q 'authorize.would-record' <<<"$DR_OUT" \
    && grep -q 'sign.would-record' <<<"$DR_OUT" \
    && [[ "$DR_BEFORE" == "$DR_AFTER" ]] \
    && [[ ! -d "$PLANT_ROOT/docs/authority/responses" || -z "$(ls -A "$PLANT_ROOT/docs/authority/responses" 2>/dev/null)" ]]; then
    printf 'ok    %-34s would record, wrote nothing\n' "dry run of an authorization"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; tree moved: %s\n' "dry run of an authorization" "$DR_STATUS" \
        "$([[ "$DR_BEFORE" == "$DR_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# The dry run must exercise the SAME refusals the real ingest does, and must
# never say an act was recorded. Over this corpus's whole queue: every act
# ends as `would-record` or `would-refuse` (or `needs-decision`, the sweep's
# own word for an act waiting on a flag), nothing ends as `<act>.recorded`,
# and a refusal is preceded by the ingest rule it came from. Not pinned to
# an alias: the queue moves, and a plant pinned to a moment in it breaks the
# night the owner signs (lib.sh, `scratch_warrant`).
DR2_BEFORE=$(git status --porcelain -- docs/ | sort)
DR2_OUT=$("$WAR" sign --all --dry-run 2>&1)
DR2_AFTER=$(git status --porcelain -- docs/ | sort)
DR2_JUDGED=$(grep -cE 'sign\.(would-record|would-refuse)' <<<"$DR2_OUT")
DR2_RECORDED=$(grep -cE '(authorize|resolution|correction)\.recorded|attest\.emitted' <<<"$DR2_OUT")
if [[ "$DR2_JUDGED" -gt 0 && "$DR2_RECORDED" -eq 0 ]] \
    && [[ "$DR2_BEFORE" == "$DR2_AFTER" ]] \
    && ! ls docs/authority/responses/*.draft.toml >/dev/null 2>&1; then
    printf 'ok    %-34s %s act(s) judged, none recorded, nothing written\n' "dry run names the real refusal" "$DR2_JUDGED"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s judged %s, recorded %s, tree moved: %s\n' "dry run names the real refusal" "$DR2_JUDGED" "$DR2_RECORDED" \
        "$([[ "$DR2_BEFORE" == "$DR2_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# --all --dry-run over a real queue: many acts, still nothing written, no
# draft left in a temp directory either.
DR3_BEFORE=$(git status --porcelain | sort)
"$WAR" sign --all --dry-run >/dev/null 2>&1
DR3_AFTER=$(git status --porcelain | sort)
if [[ "$DR3_BEFORE" == "$DR3_AFTER" ]] && [[ -z "$(ls -d /tmp/war-dry-run-* 2>/dev/null)" ]]; then
    printf 'ok    %-34s a whole queue judged, nothing written\n' "dry run of every pending act"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s tree moved or a temp draft survived\n' "dry run of every pending act"
    FAILED=$((FAILED + 1))
fi

# A dry run holds no key: the code path must never reach ssh-keygen.
if ! grep -q 'ssh-keygen\|SSH_AUTH_SOCK' <<<"$DR_OUT$DR2_OUT"; then
    printf 'ok    %-34s no ssh in the transcript\n' "dry run reaches no key"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s ssh mentioned\n' "dry run reaches no key"
    FAILED=$((FAILED + 1))
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT

# ---------------------------------------------------------------------------
# Ownership (OW-ADR-0021), on THIS corpus. OW-WAR-0005 resolved with a pin on
# check.rs; OW-WAR-0112 later declared it and was authorized. Read-only
# plants grep one `war check`; mutating ones go through plant()/restore.
echo "== ownership (OW-ADR-0021) =="
OWN_OUT=$("$WAR" check 2>&1)

# A later owner makes the older pin historical: a PASS naming both Warrants,
# and no drift or correction error for that path against the old one.
if grep -q 'deliverable.superseded-by .*OW-WAR-0005: D-001 pinned crates/openwarrant-cli/src/check.rs .*OW-WAR-0112/D-' <<<"$OWN_OUT" \
    && ! grep -qE '^ERROR .*OW-WAR-0005: D-001' <<<"$OWN_OUT"; then
    printf 'ok    %-34s OW-WAR-0005/D-001 historical under OW-WAR-0112\n' "later owner makes the pin historical"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no superseded-by for OW-WAR-0005/D-001, or it still errors\n' "later owner makes the pin historical"
    FAILED=$((FAILED + 1))
fi

# `war pins` says the same thing the check does, from the same index.
if "$WAR" --json pins 2>/dev/null | python3 -c '
import sys, json
d = json.load(sys.stdin); r = d.get("result", d)
rows = {(p["warrant"], p["path"]): p for p in r["pins"]}
old = rows[("OW-WAR-0005", "crates/openwarrant-cli/src/check.rs")]
new = rows[("OW-WAR-0112", "crates/openwarrant-cli/src/check.rs")]
sys.exit(0 if old["historical"] and not new["historical"] else 1)
'; then
    printf 'ok    %-34s 0005 historical, 0112 current\n' "pins agree with the check"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s pins and check disagree on check.rs\n' "pins agree with the check"
    FAILED=$((FAILED + 1))
fi

# A resolved Warrant under an older SAS is history, not a re-pin debt.
if grep -q 'sas.pin-historical .*OW-WAR-0062' <<<"$OWN_OUT" && ! grep -q 'sas.pin-superseded .*OW-WAR-0062' <<<"$OWN_OUT"; then
    printf 'ok    %-34s resolved Warrant asked for nothing\n' "old SAS pin on a resolved Warrant"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s OW-WAR-0062 still warns sas.pin-superseded, or no pin-historical\n' "old SAS pin on a resolved Warrant"
    FAILED=$((FAILED + 1))
fi

# A correction against a historical pin is refused by name and writes nothing:
# there is nothing to correct, the newer owner answers for the bytes.
CH_BEFORE=$(git status --porcelain | sort)
CH_OUT=$("$WAR" correct OW-WAR-0005 D-001 2>&1)
CH_AFTER=$(git status --porcelain | sort)
if grep -q 'correction.historical' <<<"$CH_OUT" && [[ "$CH_BEFORE" == "$CH_AFTER" ]]; then
    printf 'ok    %-34s refused, nothing written\n' "correction of a historical pin"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; tree moved: %s\n' "correction of a historical pin" "$?" \
        "$([[ "$CH_BEFORE" == "$CH_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# A path added to deliverables.toml AFTER the signature is declared but not
# owned: a warning that says so, and no ownership of the new path.
plant_restore
printf '\n[[deliverable]]\nid = "D-999"\ntitle = "added after signing"\nkind = "file"\ntarget_ref = "docs/roadmap/PHASE1_EXIT.md"\nrequired = false\ncontent_addressed = false\nprovenance_required = false\n' \
    >> docs/warrants/OW-WAR-0112/deliverables.toml
assert_present 'D-999' docs/warrants/OW-WAR-0112/deliverables.toml
UA_OUT=$("$WAR" check 2>&1)
plant_restore
if grep -q 'deliverable.undeclared-at-authorization .*OW-WAR-0112: D-999' <<<"$UA_OUT" \
    && ! grep -q 'superseded-by .*OW-WAR-0061: D-001' <<<"$UA_OUT"; then
    printf 'ok    %-34s declared, not owned; OW-WAR-0061 keeps its pin\n' "path added after signing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no undeclared-at-authorization for D-999, or 0061 went historical\n' "path added after signing"
    FAILED=$((FAILED + 1))
fi

# A signed-for deliverable deleted from the declaration is an error: a claim
# on a path is not withdrawn by removing the line.
plant "a signed deliverable removed from the declaration" "deliverable.declared-then-removed" \
    "OW-WAR-0112: the authorization signed for D-001" 2 \
    "python3 -c \"
import pathlib, re
p = pathlib.Path('docs/warrants/OW-WAR-0112/deliverables.toml')
t = p.read_text()
t2 = re.sub(r'\[\[deliverable\]\]\nid = \\\"D-001\\\"\n(?:.*\n)*?(?=\[\[deliverable\]\])', '', t, count=1)
assert t2 != t
p.write_text(t2)
\"; ! grep -q 'id = \"D-001\"' docs/warrants/OW-WAR-0112/deliverables.toml"

# The `owned` set lives under the authorization's attestation: editing it is
# record tampering, and `war attest --verify` says so before any ownership is
# computed from it.
plant_restore
sed -i 's#docs/adr/atoms/OW-ADR-0021-pin-ownership.md#docs/adr/atoms/OW-ADR-0021-x.md#' docs/warrants/OW-WAR-0112/authorization.toml
assert_present 'OW-ADR-0021-x.md' docs/warrants/OW-WAR-0112/authorization.toml
OE_OUT=$("$WAR" attest OW-WAR-0112 --verify 2>&1)
OE_STATUS=$?
plant_restore
if [[ $OE_STATUS -eq 2 ]] && grep -q 'attest.subject-drift .*OW-WAR-0112/authorization.toml' <<<"$OE_OUT"; then
    printf 'ok    %-34s attest.subject-drift (exit 2)\n' "owned set edited after signing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, subject-drift on authorization.toml not reported\n' "owned set edited after signing" "$OE_STATUS"
    FAILED=$((FAILED + 1))
fi
