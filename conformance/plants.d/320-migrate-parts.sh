# shellcheck shell=bash
# `war admin migrate` carries a directory ticket's tests, KPIs and milestones
# (its `atoms/30-checks.md`) into the master as the records `war plan add`
# writes into a document, when that is exact; otherwise the ticket stays
# where it is (`migrate.parts`). Each claim is paired with its refusal.
#
# Accepted: a ticket with a whole-Warrant test, an item's test, a KPI with a
# target and a milestone with a minimum moves; the master holds CHK-whole
# (evaluating the Warrant), CHK-only-one (its item), CHK-speed (kpi, max,
# target) and M-<item> (min observed), and no checks atom is left; `war check`
# of it passes; the item's test still decides its tick (`done --check` fails, then
# passes once the file it tests exists).
# Refused, each named and left byte for byte: an atom with a line that does
# not read, and a test for an item the ticket does not have.

echo "== war admin migrate: a ticket's parts =="
MP_WAR="$REPO_ROOT/${WAR#./}"
mp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
mp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
MP_R=$(scratch_corpus MP)
[[ -d "${MP_R:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
MP_T=$(mktemp -d)
mp_war() {
    (cd "$MP_R" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID \
        -u KATANA_SESSION_ID -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
        OPENWARRANT_ACTOR=plant timeout 120 "$MP_WAR" "$@" </dev/null >"$MP_T/out" 2>"$MP_T/err")
}
mp_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$MP_T/out" 2>/dev/null; }
mp_items() { sed -n 's/.*(\(i-[0-9a-f]*\)).*/\1/p' "$MP_R/docs/tickets/$1/atoms/15-checklist.md"; }
mp_sum() { (cd "$MP_R/docs/tickets/$1" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum); }

# Three directory tickets: one whose parts convert, two whose parts do not.
mp_war --json create --legacy "Parts that convert" --item one --item two; MP_A=$(mp_id)
mp_war --json create --legacy "A line that does not read" --item one; MP_B=$(mp_id)
mp_war --json create --legacy "A test of no item here" --item one; MP_C=$(mp_id)
read -r MP_A1 MP_A2 < <(mp_items "$MP_A" | tr '\n' ' ')
mp_war plan add "$MP_A" --test true --name whole
mp_war plan add "$MP_A/$MP_A1" --test "test -f gate.txt" --name only-one
mp_war plan add "$MP_A" --kpi speed --cmd "echo 3" --direction max --target 2
mp_war plan add "$MP_A/$MP_A2" --min observed
mp_war plan add "$MP_B" --test true --name fine
printf -- '- this line is no part\n' >>"$MP_R/docs/tickets/$MP_B/atoms/30-checks.md"
mp_war plan add "$MP_C" --test true --name elsewhere
sed -i 's/^- elsewhere: `true`$/- elsewhere: `true` (for i-0000)/' "$MP_R/docs/tickets/$MP_C/atoms/30-checks.md"
if [[ -z "$MP_A1" || -z "$MP_A2" ]] || ! grep -q '^- i-.* · min observed$' "$MP_R/docs/tickets/$MP_A/atoms/30-checks.md" \
    || ! grep -q '(for i-0000)' "$MP_R/docs/tickets/$MP_C/atoms/30-checks.md"; then
    printf 'PLANT SETUP FAILED: three directory tickets with parts: %s\n' "$(head -c 300 "$MP_T/err")" >&2; exit 9
fi
git -C "$MP_R" add -A >/dev/null 2>&1
git -C "$MP_R" -c user.email=plant@invalid -c user.name=plant commit -qm parts >/dev/null 2>&1
MP_B0=$(mp_sum "$MP_B"); MP_C0=$(mp_sum "$MP_C")

mp_war admin migrate; MP_RC=$?
command cp "$MP_T/out" "$MP_T/migrate.out"
MP_DOC=$(ls "$MP_R/docs/work/$MP_A"/*.md 2>/dev/null | head -1)
mp_rec() { awk -v h="$1" 'index($0, h) == 1 {f=1; print; next} /^## /{f=0} f' "$MP_DOC"; }
MP_WHOLE=$(mp_rec "## CHK-whole · check"); MP_ONE=$(mp_rec "## CHK-only-one · check")
MP_SPEED=$(mp_rec "## CHK-speed · kpi"); MP_MILE=$(mp_rec "## M-$MP_A2 · milestone")
if [[ $MP_RC -eq 0 && -n "$MP_DOC" && ! -e "$MP_R/docs/work/$MP_A/atoms/30-checks.md" && ! -e "$MP_R/docs/tickets/$MP_A" ]] \
    && grep -qx "evaluates $MP_A" <<<"$MP_WHOLE" && grep -qx 'run: true' <<<"$MP_WHOLE" \
    && grep -qx "evaluates $MP_A1" <<<"$MP_ONE" && grep -qx 'run: test -f gate.txt' <<<"$MP_ONE" \
    && grep -qx 'direction: max' <<<"$MP_SPEED" && grep -qx 'target: 2.0' <<<"$MP_SPEED" \
    && grep -qx 'min: observed' <<<"$MP_MILE" && grep -q "($MP_A2)" <<<"$MP_MILE"; then
    mp_ok "parts become records" "a whole test, an item's test, a KPI and a milestone; no checks atom left"
else
    mp_fail "parts become records" "exit $MP_RC doc '${MP_DOC#"$MP_R"/}': $(tr '\n' '|' <"$MP_T/migrate.out" | head -c 300)"
fi

mp_war check "$MP_A"; MP_CHECK=$?
mp_war admin claim "$MP_A/$MP_A1"
mp_war done "$MP_A/$MP_A1" --check; MP_D1=$?
MP_D1_ERR=$(cat "$MP_T/err")
: >"$MP_R/gate.txt"
mp_war done "$MP_A/$MP_A1" --check; MP_D2=$?
if [[ $MP_CHECK -eq 0 && $MP_D1 -ne 0 && "$MP_D1_ERR" == *check.failed* && $MP_D2 -eq 0 ]] \
    && grep -q "^- \[x\] one ($MP_A1).*\[observed\]" "$MP_DOC"; then
    mp_ok "the item's test still decides" "check passes; done --check fails without gate.txt, ticks at observed with it"
else
    mp_fail "the item's test still decides" "check $MP_CHECK, first done $MP_D1 ($(head -c 160 <<<"$MP_D1_ERR")), second $MP_D2"
fi

if grep -q "^$MP_B  left in place (migrate.parts): " "$MP_T/migrate.out" && [[ "$(mp_sum "$MP_B")" == "$MP_B0" ]] \
    && [[ ! -e "$MP_R/docs/work/$MP_B" ]]; then
    mp_ok "an unreadable part line stays" "migrate.parts names it; its files byte for byte"
else
    mp_fail "an unreadable part line stays" "$(grep "^$MP_B" "$MP_T/migrate.out" | head -c 200); moved: $([[ -e "$MP_R/docs/work/$MP_B" ]] && echo yes || echo no)"
fi
if grep -q "^$MP_C  left in place (migrate.parts): .*do not convert exactly" "$MP_T/migrate.out" \
    && [[ "$(mp_sum "$MP_C")" == "$MP_C0" ]] && [[ ! -e "$MP_R/docs/work/$MP_C" ]]; then
    mp_ok "a part not carried exactly stays" "a test of an item the ticket lacks: migrate.parts; its files byte for byte"
else
    mp_fail "a part not carried exactly stays" "$(grep "^$MP_C" "$MP_T/migrate.out" | head -c 200); moved: $([[ -e "$MP_R/docs/work/$MP_C" ]] && echo yes || echo no)"
fi

[[ -n "$MP_T" && "$MP_T" == */tmp.* ]] && /usr/bin/rm -rf "$MP_T"
corpus_gone "$MP_R"
unset MP_WAR MP_R MP_T MP_A MP_B MP_C MP_A1 MP_A2 MP_B0 MP_C0 MP_RC MP_DOC MP_WHOLE MP_ONE MP_SPEED MP_MILE \
    MP_CHECK MP_D1 MP_D1_ERR MP_D2
unset -f mp_ok mp_fail mp_war mp_id mp_items mp_sum mp_rec
