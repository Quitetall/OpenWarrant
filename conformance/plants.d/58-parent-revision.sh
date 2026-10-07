# shellcheck shell=bash
# OW-WAR-0123 — a child cites the exact revision of its parent, and a moved
# parent is a finding (RQ-023, §20.2, §91.5 test 29).
#
# Nothing here mutates this repository. The corpus plants run on a full local
# clone of HEAD (the retained history an older revision's digest comes from)
# and on a `--depth 1` clone (history that cannot answer); the draft-parent
# plants run in a scratch program (`PR`). Each assertion reads `war --json
# check` and binds severity, rule and detail to ONE diagnostic, so a finding
# from the neighbouring parent rule cannot stand in for the one planted — and
# every claim is paired with the refusal that shows the control discriminates.
#
# OW-WAR-0001 is authorized at revision 2 (f29c7d95…); revision 1's digest
# (ab7e2df7…) lives only in history. OW-WAR-0002 is authorized, so a wrong
# revision there is the U-001 (b) warning; an unauthorized child gets the
# error.

echo "== parent revision (OW-WAR-0123) =="
PR_REV1="ab7e2df770a6c895e55e0c5990b0ad33a18f5dbebe653e6389fbdc30ca1c648a"
PR_REV2="f29c7d95baeb90a7bbad57a26b07846c724482c66a1e9ed7a2670af8d00b776d"
PR_ZERO="0000000000000000000000000000000000000000000000000000000000000000"
PR_CHILD="docs/warrants/OW-WAR-0002/manifest.toml"

PR_FULL=$(mktemp -d) && PR_SHALLOW=$(mktemp -d) \
    || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$PR_FULL" "$PR_SHALLOW")
git clone -q "$REPO_ROOT" "$PR_FULL/r" 2>/dev/null \
    || { printf 'PLANT SETUP FAILED: full clone of %s\n' "$REPO_ROOT" >&2; exit 9; }
git clone -q --depth 1 "file://$REPO_ROOT" "$PR_SHALLOW/r" 2>/dev/null \
    || { printf 'PLANT SETUP FAILED: shallow clone of %s\n' "$REPO_ROOT" >&2; exit 9; }
[[ "$(git -C "$PR_FULL/r" rev-parse --is-shallow-repository)" == false ]] \
    || { printf 'PLANT SETUP FAILED: the full clone is shallow\n' >&2; exit 9; }
[[ "$(git -C "$PR_SHALLOW/r" rev-parse --is-shallow-repository)" == true ]] \
    || { printf 'PLANT SETUP FAILED: the --depth 1 clone is not shallow\n' >&2; exit 9; }
# The plants below name OW-WAR-0001's two revisions. If the parent moves, the
# plants must be rewritten, not scored against a corpus they no longer describe.
grep -q "^revision = 2$" "$PR_FULL/r/docs/warrants/OW-WAR-0001/authorization.toml" \
    && grep -q "^contract_digest = \"$PR_REV2\"$" "$PR_FULL/r/docs/warrants/OW-WAR-0001/authorization.toml" \
    || { printf 'PLANT SETUP FAILED: OW-WAR-0001 is no longer at revision 2 (%s)\n' "$PR_REV2" >&2; exit 9; }

# pr_count <severity> <rule> [detail...] < json  ->  diagnostics with that
# severity and rule and every detail in their file or message; -1 when the
# output is not a report, which neither pr_expect nor pr_none accepts.
PR_COUNT_PY='
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
pr_count() {
    local json="$1"
    shift
    python3 -c "$PR_COUNT_PY" "$@" <<<"$json"
}

# pr_expect <name> <want-count|+> <json> <severity> <rule> [detail...]
# `+` is "at least one".
pr_expect() {
    local name="$1" want="$2" out="$3"
    shift 3
    local n ok=0
    n=$(pr_count "$out" "$@")
    if [[ "$want" == "+" ]]; then
        [[ "$n" =~ ^[1-9][0-9]*$ ]] && ok=1
    else
        [[ "$n" == "$want" ]] && ok=1
    fi
    if [[ $ok == 1 ]]; then
        printf 'ok    %-34s %s %s ×%s\n' "$name" "$1" "$2" "$n"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted %s %s [%s] ×%s; got ×%s\n' "$name" "$1" "$2" "${*:3}" "$want" "$n"; FAILED=$((FAILED + 1))
    fi
}

# pr_none <name> <json> <severity> <rule> [detail...]  ->  the refusal's half.
pr_none() {
    local name="$1" out="$2"
    shift 2
    local n
    n=$(pr_count "$out" "$@")
    if [[ "$n" == 0 ]]; then
        printf 'ok    %-34s no %s %s\n' "$name" "$1" "$2"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s %s %s time(s)\n' "$name" "$n" "$1" "$2"; FAILED=$((FAILED + 1))
    fi
}

# pr_exit <name> <status> <zero|nonzero>
pr_exit() {
    if { [[ "$3" == zero ]] && [[ "$2" -eq 0 ]]; } || { [[ "$3" == nonzero ]] && [[ "$2" -ne 0 ]]; }; then
        printf 'ok    %-34s exit %s\n' "$1" "$2"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$1" "$2" "$3"; FAILED=$((FAILED + 1))
    fi
}

# pr_cite <root> <manifest> <revision> <digest>: rewrite the child's citation,
# refusing a mutation that did not land.
pr_cite() {
    local f="$1/$2"
    sed -i "s/^contract_revision = .*/contract_revision = $3/; s/^contract_digest = .*/contract_digest = \"sha256:$4\"/" "$f"
    if ! grep -qx "contract_revision = $3" "$f" || ! grep -qx "contract_digest = \"sha256:$4\"" "$f"; then
        printf 'PLANT MUTATION WAS A NO-OP: the citation in %s is not revision %s at %s\n' "$f" "$3" "$4" >&2
        exit 9
    fi
}

pr_check() { "$WAR" --root "$1" --json check "${@:2}" 2>/dev/null; }

F="$PR_FULL/r"
S="$PR_SHALLOW/r"

# ── OBL-006: the committed corpus, as the owner chose (U-001 (b)) ──────────
PR_OUT=$(pr_check "$F")
for c in OW-WAR-0002 OW-WAR-0003 OW-WAR-0004 OW-WAR-0005; do
    pr_expect "corpus: $c named as revision 2" 1 "$PR_OUT" warn relations.parent-revision "$c: parent OW-WAR-0001 is cited as revision 1" "digest of revision 2"
done
PR_OTHER=$(python3 -c '
import json, sys
d = json.loads(sys.stdin.read())["diagnostics"]
kids = ("OW-WAR-0002:", "OW-WAR-0003:", "OW-WAR-0004:", "OW-WAR-0005:")
print(sum(1 for x in d if x["rule"].startswith("relations.parent-") and x["rule"] != "relations.parent-source"
          and x["severity"] != "pass" and not x["message"].startswith(kids)))
' <<<"$PR_OUT")
if [[ "$PR_OTHER" == 0 ]]; then
    printf 'ok    %-34s no parent finding elsewhere\n' "corpus: only the four children"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s parent finding(s) on other Warrants\n' "corpus: only the four children" "$PR_OTHER"; FAILED=$((FAILED + 1))
fi
pr_none "corpus: no parent error" "$PR_OUT" error relations.parent-revision
pr_none "corpus: no parent-digest error" "$PR_OUT" error relations.parent-digest

# ── OBL-001: a revision the parent never had ───────────────────────────────
corpus_reset "$F"
pr_cite "$F" "$PR_CHILD" 7 "$PR_REV2"
PR_OUT=$(pr_check "$F" OW-WAR-0002); PR_STATUS=$?
pr_expect "revision 7 refused" 1 "$PR_OUT" error relations.parent-revision "OW-WAR-0002:" "revision 7, which does not exist" "latest authorized revision is 2"
pr_exit "revision 7 exits non-zero" "$PR_STATUS" nonzero
pr_none "revision 7 is not passed" "$PR_OUT" pass relations.parent-revision "OW-WAR-0002:"

# ── OBL-003: an exact older citation passes, and the move is reported ──────
corpus_reset "$F"
pr_cite "$F" "$PR_CHILD" 1 "$PR_REV1"
PR_OUT=$(pr_check "$F" OW-WAR-0002)
pr_expect "exact revision 1 passes" 1 "$PR_OUT" pass relations.parent-revision "OW-WAR-0002:" "cited at revision 1, at that revision's digest"
pr_expect "exact revision 1: parent moved" 1 "$PR_OUT" warn relations.parent-moved "OW-WAR-0002:" "from revision 1 to revision 2"
pr_none "exact revision 1: no revision error" "$PR_OUT" error relations.parent-revision
pr_none "exact revision 1: no digest error" "$PR_OUT" error relations.parent-digest
pr_none "exact revision 1: not UNKNOWN (full)" "$PR_OUT" unknown relations.parent-revision

corpus_reset "$F"
pr_cite "$F" "$PR_CHILD" 2 "$PR_REV2"
PR_OUT=$(pr_check "$F" OW-WAR-0002)
pr_expect "exact revision 2 passes" 1 "$PR_OUT" pass relations.parent-revision "OW-WAR-0002:" "cited at revision 2, at that revision's digest"
pr_none "exact revision 2: no moved warning" "$PR_OUT" warn relations.parent-moved
pr_none "exact revision 2: no revision error" "$PR_OUT" error relations.parent-revision
pr_none "exact revision 2: no revision warning" "$PR_OUT" warn relations.parent-revision
pr_none "exact revision 2: no digest error" "$PR_OUT" error relations.parent-digest
pr_none "revision 2 exact: 7 not reported" "$PR_OUT" error relations.parent-revision "does not exist"

# ── OBL-002: another revision's digest is named as that revision ───────────
corpus_reset "$F"
pr_cite "$F" "$PR_CHILD" 2 "$PR_REV1"
PR_OUT=$(pr_check "$F" OW-WAR-0002)
pr_expect "rev 2 at rev 1's digest (signed)" 1 "$PR_OUT" warn relations.parent-revision "OW-WAR-0002:" "cited as revision 2" "which is the digest of revision 1" "is an amendment of OW-WAR-0002"
pr_none "rev 2 at rev 1's digest: no pass" "$PR_OUT" pass relations.parent-revision "OW-WAR-0002:"
pr_none "rev 2 at rev 1's digest: no digest pass" "$PR_OUT" pass relations.parent-digest "OW-WAR-0002:"

corpus_reset "$F"
pr_cite "$F" "$PR_CHILD" 1 "$PR_ZERO"
PR_OUT=$(pr_check "$F" OW-WAR-0002); PR_STATUS=$?
pr_expect "a digest of no revision" 1 "$PR_OUT" error relations.parent-digest "OW-WAR-0002:" "sha256:$PR_ZERO, which is the digest of no authorized revision"
pr_exit "a digest of no revision exits non-zero" "$PR_STATUS" nonzero
pr_none "no revision: not named as a revision" "$PR_OUT" warn relations.parent-revision "OW-WAR-0002:"
pr_none "no revision: not passed" "$PR_OUT" pass relations.parent-revision "OW-WAR-0002:"

# An UNAUTHORIZED child with the same mismatch is an error: its fix is an edit.
corpus_reset "$F"
PR_NEW_OUT=$("$WAR" --root "$F" new "Plant: unauthorized child" --parent OW-WAR-0001 2>&1); PR_STATUS=$?
PR_NEW=$(grep -oE 'OW-WAR-[0-9]{4}' <<<"$PR_NEW_OUT" | head -1)
[[ $PR_STATUS -eq 0 && -n "$PR_NEW" ]] \
    || { printf 'PLANT SETUP FAILED: war new --parent in the clone\n%s\n' "$PR_NEW_OUT" >&2; exit 9; }
pr_cite "$F" "docs/warrants/$PR_NEW/manifest.toml" 2 "$PR_REV1"
PR_OUT=$(pr_check "$F" "$PR_NEW"); PR_STATUS=$?
pr_expect "rev 2 at rev 1's digest (unsigned)" 1 "$PR_OUT" error relations.parent-revision "$PR_NEW:" "cited as revision 2" "which is the digest of revision 1"
pr_none "unsigned: not softened to a warning" "$PR_OUT" warn relations.parent-revision "$PR_NEW:"
pr_exit "unsigned mismatch exits non-zero" "$PR_STATUS" nonzero

# ── OBL-004: history that cannot answer is UNKNOWN ─────────────────────────
pr_cite "$S" "$PR_CHILD" 1 "$PR_REV1"
PR_OUT=$(pr_check "$S" OW-WAR-0002); PR_STATUS=$?
pr_expect "shallow: revision 1 is UNKNOWN" 1 "$PR_OUT" unknown relations.parent-revision "OW-WAR-0002:" "revision 1" "shallow repository"
pr_none "shallow: not PASS" "$PR_OUT" pass relations.parent-revision "OW-WAR-0002:"
pr_none "shallow: not ERROR (revision)" "$PR_OUT" error relations.parent-revision
pr_none "shallow: not ERROR (digest)" "$PR_OUT" error relations.parent-digest
pr_none "shallow: no moved claim" "$PR_OUT" warn relations.parent-moved
pr_exit "shallow: UNKNOWN blocks readiness" "$PR_STATUS" nonzero
corpus_reset "$S"
# The same shallow clone, unmutated: revision 2's digest is on disk, so the
# committed children are answered without history — not UNKNOWN.
PR_OUT=$(pr_check "$S")
pr_expect "shallow: corpus still answered" 4 "$PR_OUT" warn relations.parent-revision "digest of revision 2"
pr_none "shallow: corpus has no UNKNOWN" "$PR_OUT" unknown relations.parent-revision

# An unauthorized edit of the parent is still caught for a child resting on
# its latest content (the 00-corpus.sh plant holds it for the live corpus).
corpus_reset "$F"
printf '\nAn edit to the parent that its children were never re-authorized against.\n' >> "$F/docs/warrants/OW-WAR-0001/atoms/10-intent.md"
PR_OUT=$(pr_check "$F" OW-WAR-0002)
pr_expect "parent edited: still caught" 1 "$PR_OUT" error relations.parent-digest "OW-WAR-0002:" "no longer the one it was authorized against"
corpus_reset "$F"
pr_cite "$F" "$PR_CHILD" 2 "$PR_REV2"
printf '\nAn edit to the parent that its children were never re-authorized against.\n' >> "$F/docs/warrants/OW-WAR-0001/atoms/10-intent.md"
PR_OUT=$(pr_check "$F" OW-WAR-0002)
pr_expect "parent edited: exact rev 2 caught" 1 "$PR_OUT" error relations.parent-digest "OW-WAR-0002:" "no longer the one it was authorized against"

# ── OBL-005: war new --parent writes the exact citation, refuses the rest ──
corpus_reset "$F"
PR_UUID=$(sed -n 's/^uuid = "\(.*\)"$/\1/p' "$F/docs/warrants/OW-WAR-0001/manifest.toml")
PR_NEW_OUT=$("$WAR" --root "$F" new "Plant: exact child" --parent OW-WAR-0001 2>&1); PR_STATUS=$?
PR_NEW=$(grep -oE 'OW-WAR-[0-9]{4}' <<<"$PR_NEW_OUT" | head -1)
PR_M="$F/docs/warrants/$PR_NEW/manifest.toml"
if [[ $PR_STATUS -eq 0 && -n "$PR_NEW" && -f "$PR_M" ]] \
    && grep -qx "ref = \"war://$PR_UUID\"" "$PR_M" \
    && grep -qx "contract_revision = 2" "$PR_M" \
    && grep -qx "contract_digest = \"sha256:$PR_REV2\"" "$PR_M"; then
    printf 'ok    %-34s %s cites revision 2 at %s\n' "new --parent writes the citation" "$PR_NEW" "${PR_REV2:0:8}…"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, %s\n' "new --parent writes the citation" "$PR_STATUS" "$PR_NEW_OUT"; FAILED=$((FAILED + 1))
fi
PR_OUT=$(pr_check "$F" "$PR_NEW")
pr_expect "new --parent child passes" 1 "$PR_OUT" pass relations.parent-revision "$PR_NEW:" "cited at revision 2"
pr_none "new --parent child: no finding" "$PR_OUT" error relations.parent-revision
pr_none "new --parent child: no warning" "$PR_OUT" warn relations.parent-revision

corpus_reset "$F"
PR_BEFORE=$(ls "$F/docs/warrants" | wc -l)
PR_NEW_OUT=$("$WAR" --root "$F" new "Plant: orphan" --parent OW-WAR-9999 2>&1); PR_STATUS=$?
PR_AFTER=$(ls "$F/docs/warrants" | wc -l)
if [[ $PR_STATUS -ne 0 && "$PR_BEFORE" == "$PR_AFTER" && -z "$(git -C "$F" status --porcelain)" ]] \
    && grep -q 'no Warrant "OW-WAR-9999"' <<<"$PR_NEW_OUT"; then
    printf 'ok    %-34s refused (exit %s), nothing created\n' "new --parent OW-WAR-9999" "$PR_STATUS"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, %s → %s dirs; %s\n' "new --parent OW-WAR-9999" "$PR_STATUS" "$PR_BEFORE" "$PR_AFTER" "$PR_NEW_OUT"; FAILED=$((FAILED + 1))
fi

# A draft parent, in a program nobody owns: PR-WAR-0001 awaits authorization.
PR_P=$(scratch_corpus PR)
[[ -f "$PR_P/docs/warrants/PR-WAR-0001/manifest.toml" ]] \
    || { printf 'PLANT SETUP FAILED: no PR-WAR-0001 in the scratch program\n' >&2; exit 9; }
[[ ! -e "$PR_P/docs/warrants/PR-WAR-0001/authorization.toml" ]] \
    || { printf 'PLANT SETUP FAILED: PR-WAR-0001 is authorized\n' >&2; exit 9; }
PR_BEFORE=$(ls "$PR_P/docs/warrants" | wc -l)
PR_NEW_OUT=$("$WAR" --root "$PR_P" new "Plant: child of a draft" --parent PR-WAR-0001 2>&1); PR_STATUS=$?
PR_AFTER=$(ls "$PR_P/docs/warrants" | wc -l)
if [[ $PR_STATUS -ne 0 && "$PR_BEFORE" == "$PR_AFTER" && -z "$(git -C "$PR_P" status --porcelain)" ]] \
    && grep -q 'has no authorized revision' <<<"$PR_NEW_OUT"; then
    printf 'ok    %-34s refused (exit %s), nothing created\n' "new --parent <a draft>" "$PR_STATUS"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, %s → %s dirs; %s\n' "new --parent <a draft>" "$PR_STATUS" "$PR_BEFORE" "$PR_AFTER" "$PR_NEW_OUT"; FAILED=$((FAILED + 1))
fi
# The same refusal, for a draft beside the authorized parent in the clone.
PR_DRAFT_OUT=$("$WAR" --root "$F" new "Plant: a draft" 2>&1)
PR_DRAFT=$(grep -oE 'OW-WAR-[0-9]{4}' <<<"$PR_DRAFT_OUT" | head -1)
PR_BEFORE=$(ls "$F/docs/warrants" | wc -l)
PR_NEW_OUT=$("$WAR" --root "$F" new "Plant: child of a draft" --parent "$PR_DRAFT" 2>&1); PR_STATUS=$?
PR_AFTER=$(ls "$F/docs/warrants" | wc -l)
if [[ -n "$PR_DRAFT" && $PR_STATUS -ne 0 && "$PR_BEFORE" == "$PR_AFTER" ]] \
    && grep -q 'has no authorized revision' <<<"$PR_NEW_OUT"; then
    printf 'ok    %-34s refused (exit %s), nothing created\n' "new --parent <a corpus draft>" "$PR_STATUS"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, %s → %s dirs; %s\n' "new --parent <a corpus draft>" "$PR_STATUS" "$PR_BEFORE" "$PR_AFTER" "$PR_NEW_OUT"; FAILED=$((FAILED + 1))
fi

# A hand-written citation of a draft (U-002, kept): it has one revision, 1.
PR_KID_OUT=$("$WAR" --root "$PR_P" new "Plant: hand-cited child" 2>&1)
PR_KID=$(grep -oE 'PR-WAR-[0-9]{4}' <<<"$PR_KID_OUT" | head -1)
PR_PUUID=$(sed -n 's/^uuid = "\(.*\)"$/\1/p' "$PR_P/docs/warrants/PR-WAR-0001/manifest.toml")
[[ -n "$PR_KID" && -n "$PR_PUUID" ]] \
    || { printf 'PLANT SETUP FAILED: a second Warrant in the scratch program\n%s\n' "$PR_KID_OUT" >&2; exit 9; }
printf '\n[[parents]]\nref = "war://%s"\ncontract_revision = 1\n' "$PR_PUUID" >> "$PR_P/docs/warrants/$PR_KID/manifest.toml"
PR_OUT=$(pr_check "$PR_P" "$PR_KID")
PR_DRAFT_DIGEST=$(grep -oE 'contract_digest = \\"sha256:[0-9a-f]{64}' <<<"$PR_OUT" | head -1 | grep -oE '[0-9a-f]{64}')
[[ -n "$PR_DRAFT_DIGEST" ]] \
    || { printf 'PLANT SETUP FAILED: no digest offered for the draft parent\n' >&2; exit 9; }
printf 'contract_digest = "sha256:%s"\n' "$PR_DRAFT_DIGEST" >> "$PR_P/docs/warrants/$PR_KID/manifest.toml"
PR_OUT=$(pr_check "$PR_P" "$PR_KID")
pr_expect "draft parent, revision 1: passes" 1 "$PR_OUT" pass relations.parent-digest "$PR_KID:"
pr_none "draft parent, revision 1: no error" "$PR_OUT" error relations.parent-revision
sed -i 's/^contract_revision = 1$/contract_revision = 2/' "$PR_P/docs/warrants/$PR_KID/manifest.toml"
grep -qx 'contract_revision = 2' "$PR_P/docs/warrants/$PR_KID/manifest.toml" \
    || { printf 'PLANT MUTATION WAS A NO-OP: revision 2 of a draft\n' >&2; exit 9; }
PR_OUT=$(pr_check "$PR_P" "$PR_KID")
pr_expect "draft parent, revision 2: refused" 1 "$PR_OUT" error relations.parent-revision "$PR_KID:" "revision 2, which does not" "can only be revision 1"

corpus_gone "$PR_P"
corpus_gone "$PR_FULL"
corpus_gone "$PR_SHALLOW"
unset F S PR_OUT PR_STATUS PR_NEW PR_NEW_OUT PR_M PR_P PR_KID PR_DRAFT
