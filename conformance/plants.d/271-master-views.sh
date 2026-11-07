# shellcheck shell=bash
# The views of the masters (FORMAT-1.0 §4.3, revision 4; views.rs), each claim
# beside its refusal:
#
#   a repository with no docs/work/ gets no master views from `war compile`;
#   with masters, a document gets its page (tier and type), an open work order
#   is listed in docs/ROADMAP.md under the phase it is part_of, a done one in
#   docs/HISTORY.md and not in the Roadmap;
#   a number in docs/generated/numbers.tsv is shown as #N; an id it lacks is
#   shown as itself, never given a number;
#   the pages are views: a pull request that carries one is refused by
#   conformance/views.py (`views.toml` lists docs/pages/**).

echo "== master views (FORMAT-1.0 §4.3) =="
MV_ROOT=$(scratch_corpus MV)
[[ -d "${MV_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
mv_war()  { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID \
                -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
                "$REPO_ROOT/${WAR#./}" --root "$MV_ROOT" "$@"; }
mv_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
mv_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
MV_N=0
mv_uuid() { MV_N=$((MV_N + 1)); printf '01a1b788-533f-76ca-8adf-%012x' "$((0x73e0ddb95500 + MV_N))"; }
# mv_doc <id> <file> <profile> <frontmatter extra or ""> <body>
mv_doc() {
    mkdir -p "$MV_ROOT/docs/work/$1"
    {
        printf -- '---\nformat: openwarrant/1.0\nprofile: %s\nalias: %s\nuuid: %s\n' "$3" "$1" "$(mv_uuid)"
        printf 'summary: Views plant item %s.\n' "$1"
        [[ -n "$4" ]] && printf '%s\n' "$4"
        printf -- '---\n# Title of %s\n\n%s\n' "$1" "$5"
    } > "$MV_ROOT/docs/work/$1/$2"
}

# ---- 1. no masters: no master views ----
mv_war compile >/dev/null 2>&1
if [[ ! -e "$MV_ROOT/docs/ROADMAP.md" && ! -d "$MV_ROOT/docs/pages" ]]; then
    mv_ok "no masters, no master views" "compile wrote no ROADMAP.md or pages"
else
    mv_fail "no masters, no master views" "$(ls "$MV_ROOT/docs" | tr '\n' ' ')"
fi

# ---- 2. masters: a page per document, open work by phase, closed work ----
mv_doc roadmap roadmap.md roadmap "" "## PH-1 · phase — first things"
mv_doc d-5e21b d-5e21b-cache.md design "type: design" "## Approach
Keyed by digest."
mv_doc w-0a0a1 w-0a0a1-open-one.md ticket "part_of: roadmap/PH-1" "## Checklist
- [ ] Still to do (i-d01)"
mv_doc w-0b0b2 w-0b0b2-done-one.md ticket "" "## Checklist
- [x] Did it (i-d02) — done by plant, 2026-11-07"
mkdir -p "$MV_ROOT/docs/generated"
printf 'id\tnumber\tfirst_seen\nw-0a0a1\t7\tabc\n' > "$MV_ROOT/docs/generated/numbers.tsv"
out=$(mv_war compile 2>&1); st=$?
page="$MV_ROOT/docs/pages/d-5e21b.md"
road=$(cat "$MV_ROOT/docs/ROADMAP.md" 2>/dev/null)
hist=$(cat "$MV_ROOT/docs/HISTORY.md" 2>/dev/null)
if [[ $st -eq 0 && -f "$page" ]] && grep -q 'Design (tier 3) · type design' "$page" && grep -q 'Keyed by digest' "$page"; then
    mv_ok "a page per document" "docs/pages/d-5e21b.md, Design tier 3"
else
    mv_fail "a page per document" "exit $st: $(head -c 300 <<<"$out")"
fi
if [[ -z "$(ls "$MV_ROOT/docs/pages" | grep -v '^d-5e21b.md$')" ]]; then
    mv_ok "no page for a work order" "only the document has one"
else
    mv_fail "no page for a work order" "$(ls "$MV_ROOT/docs/pages" | tr '\n' ' ')"
fi
sect=$(awk '/^## first things/{f=1;next} /^## /{f=0} f' <<<"$road")
if [[ "$sect" == *"#7 \`w-0a0a1\`"* && "$road" != *"w-0b0b2"* ]]; then
    mv_ok "open work under its phase" "#7 w-0a0a1 under PH-1; the done one is not there"
else
    mv_fail "open work under its phase" "$(tr '\n' '|' <<<"$road" | head -c 400)"
fi
if [[ "$hist" == *"\`w-0b0b2\`"* && "$hist" != *"#"*"\`w-0b0b2\`"* && "$hist" != *"w-0a0a1"* ]]; then
    mv_ok "closed work in HISTORY, unnumbered" "w-0b0b2 shown as its id"
else
    mv_fail "closed work in HISTORY, unnumbered" "$(tr '\n' '|' <<<"$hist" | head -c 400)"
fi
if [[ "$road" != *"d-5e21b"* && "$hist" != *"d-5e21b"* ]]; then
    mv_ok "a document is no work order" "d-5e21b in neither list"
else
    mv_fail "a document is no work order" "listed as work"
fi

# ---- 3. the pages are views: a pull request carrying one is refused ----
git -C "$MV_ROOT" add -A >/dev/null 2>&1
git -C "$MV_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgsign=false commit -qm "views plant" >/dev/null 2>&1
mv_base=$(git -C "$MV_ROOT" rev-parse HEAD~1 2>/dev/null)
pr=$(cd "$MV_ROOT" && python3 "$REPO_ROOT/conformance/views.py" pr --base "$mv_base" 2>&1); st=$?
if [[ $st -ne 0 && "$pr" == *"docs/pages/d-5e21b.md"* ]]; then
    mv_ok "a page in a PR is refused" "views.py names docs/pages/d-5e21b.md"
else
    mv_fail "a page in a PR is refused" "exit $st: $(tr '\n' '|' <<<"$pr" | head -c 300)"
fi
