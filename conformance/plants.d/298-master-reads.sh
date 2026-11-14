# shellcheck shell=bash
# A ticket kept as a master is read no more often than one in docs/tickets/
# (alpha.8, the read path): which file of docs/work/<id>/ is the document, and
# that it is a ticket's, is settled from one listing of the directory, and a
# document the ticket cache holds unchanged is not opened to say so. Each
# claim is paired with its refusal. Observed with strace; without it nothing
# is established, and the plant says UNKNOWN.
#
# Accepted: a warm `war view ready` over three masters opens none of their
# documents and lists each directory once; `war create` beside them opens none
# either (its duplicate check lists the work orders that are no ticket, and
# the tickets it has read are not opened again to learn they are none).
# Refused: a master edited in place, its mtime put back, is opened and its new
# title listed; a second document claiming the id is opened too, and the
# ticket is not listed (format.document-duplicate).

plant_layout work "counts the opens of docs/work/ masters; docs/tickets/ has no directory per ticket"
echo "== masters: read once, and not at all when cached =="
MR_WAR="$REPO_ROOT/${WAR#./}"
mr_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
mr_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
if ! command -v strace >/dev/null 2>&1; then
    printf 'UNKNOWN %-32s strace is not installed; the reads are not observed\n' "masters are not read when cached"
else
    MR_T=$(mktemp -d)
    MR_R=$(scratch_corpus MR)
    [[ -d "${MR_R:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
    MR_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
        -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID -u OPENWARRANT_NO_CACHE OPENWARRANT_ACTOR=plant)
    mr_war() { (cd "$MR_R" && "${MR_ENV[@]}" timeout 120 "$MR_WAR" "$@" </dev/null); }
    # mr_traced <args...>: the run's stdout in $MR_T/out; each open of a file
    # or directory under docs/work/, one per line, in $MR_T/opens.
    mr_traced() {
        (cd "$MR_R" && "${MR_ENV[@]}" timeout 120 strace -f -qq -e trace=openat,open -o "$MR_T/trace" \
            "$MR_WAR" "$@" </dev/null >"$MR_T/out" 2>"$MR_T/err")
        grep -o '"[^"]*docs/work/[^"]*"' "$MR_T/trace" | tr -d '"' | sed "s|^$MR_R/||" >"$MR_T/opens"
    }
    mr_docs() { grep -c '^docs/work/w-[0-9a-f]*/w-[^/]*\.md$' "$MR_T/opens"; }
    mr_dirs() { grep -c '^docs/work/w-[0-9a-f]*$' "$MR_T/opens"; }
    for MR_N in Alpha Bravo Charlie; do mr_war create "$MR_N master" --item "one of $MR_N" >/dev/null 2>&1; done
    git -C "$MR_R" add -A >/dev/null 2>&1
    git -C "$MR_R" -c user.email=plant@invalid -c user.name=plant commit -qm masters >/dev/null 2>&1
    # An entry is written only for a file settled for a moment (the cache's own rule).
    sleep 3
    mr_war view ready >/dev/null 2>&1; mr_war view ready >/dev/null 2>&1
    mr_traced view ready
    MR_WARM_DOCS=$(mr_docs); MR_WARM_DIRS=$(mr_dirs); MR_LISTED=$(grep -c ' master' "$MR_T/out")
    if [[ $MR_WARM_DOCS -eq 0 && $MR_WARM_DIRS -eq 3 && $MR_LISTED -ge 3 ]]; then
        mr_ok "a warm read opens no master" "three masters listed; each directory listed once, no document opened"
    else
        mr_fail "a warm read opens no master" "$MR_WARM_DOCS document open(s), $MR_WARM_DIRS directory open(s), $MR_LISTED listed: $(head -c 200 "$MR_T/err")"
    fi

    mr_traced create "Delta master" --item "one of Delta"; MR_RC=$?
    # Its own new document aside: the three that were there.
    MR_CREATE_DOCS=$(grep -c '^docs/work/w-[0-9a-f]*/w-[^/]*-\(alpha\|bravo\|charlie\)-master\.md$' "$MR_T/opens")
    if [[ $MR_RC -eq 0 && $MR_CREATE_DOCS -eq 0 ]]; then
        mr_ok "create opens no other master" "the duplicate check and what is next read none of the three"
    else
        mr_fail "create opens no other master" "exit $MR_RC, $MR_CREATE_DOCS open(s) of the three: $(head -c 200 "$MR_T/err")"
    fi

    # Refused: an in-place edit with its mtime put back is read, and seen.
    MR_DOC=$(grep -l '^# Alpha master' "$MR_R"/docs/work/*/*.md | head -1)
    MR_MTIME=$(stat -c '%y' "$MR_DOC")
    python3 - "$MR_DOC" <<'PY'
import sys
p = sys.argv[1]
b = open(p, "rb").read().replace(b"Alpha master", b"Omega master")
with open(p, "r+b") as f:
    f.write(b)
PY
    touch -d "$MR_MTIME" "$MR_DOC"
    mr_traced view ready
    MR_EDIT_DOCS=$(mr_docs)
    if [[ $MR_EDIT_DOCS -ge 1 ]] && grep -q 'Omega master' "$MR_T/out" && ! grep -q 'Alpha master' "$MR_T/out"; then
        mr_ok "an edited master is read" "same size and mtime; its document is opened and the new title listed"
    else
        mr_fail "an edited master is read" "$MR_EDIT_DOCS document open(s); $(grep -c 'Omega master' "$MR_T/out") Omega, $(grep -c 'Alpha master' "$MR_T/out") Alpha"
    fi

    # Refused: a second document claiming the id; neither is worked.
    command cp "$MR_DOC" "${MR_DOC%.md}-copy.md"
    mr_traced view ready
    MR_DUP_DOCS=$(mr_docs)
    MR_CHECK=$(mr_war check 2>&1)
    command rm -f "${MR_DOC%.md}-copy.md"
    if [[ $MR_DUP_DOCS -ge 2 ]] && ! grep -q 'Omega master' "$MR_T/out" && grep -q 'Bravo master' "$MR_T/out" \
        && grep -q 'format.document-duplicate' <<<"$MR_CHECK"; then
        mr_ok "a second claimant is read, refused" "both files opened; the id is not listed, the others are; check says format.document-duplicate"
    else
        mr_fail "a second claimant is read, refused" "$MR_DUP_DOCS open(s); Omega listed $(grep -c 'Omega master' "$MR_T/out"), Bravo $(grep -c 'Bravo master' "$MR_T/out"); $(grep -m1 'document-duplicate' <<<"$MR_CHECK" | cut -c1-160)"
    fi
    [[ -n "$MR_T" && "$MR_T" == */tmp.* ]] && /usr/bin/rm -rf "$MR_T"
    corpus_gone "$MR_R"
    unset MR_T MR_R MR_ENV MR_N MR_WARM_DOCS MR_WARM_DIRS MR_LISTED MR_RC MR_CREATE_DOCS MR_DOC MR_MTIME MR_EDIT_DOCS \
        MR_DUP_DOCS MR_CHECK
fi
unset MR_WAR
