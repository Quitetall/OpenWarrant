# shellcheck shell=bash
# FORMAT-1.0 11.1: a work document `part_of` another counts as an item of it,
# so a parent with one child done and another open is `in_progress`, not
# `open`, wherever `war` shows its state. Each claim has its refusal.
# - one child done of two: the parent is in_progress in `view warrants`,
#   `view prime <parent>` and `view show <parent>`.
# - refusal: no child done or claimed: the parent stays open.
# - all children done: the parent's own items still open, so in_progress,
#   never done; with its own item done too, done.

echo "== a parent's state counts its part_of children (FORMAT-1.0 11.1) =="
PS_TMP=$(mktemp -d)
PS_WAR="$REPO_ROOT/${WAR#./}"
ps_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ps_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ps_war() {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID \
        -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
        OPENWARRANT_ACTOR=plant timeout 120 "$PS_WAR" --root "$PS_TMP/repo" "$@" </dev/null
}
ps_id() { grep -o 'w-[0-9a-f]*\|t-[0-9a-f]*' | head -1; }
mkdir -p "$PS_TMP/repo"
git -C "$PS_TMP/repo" init -q .
ps_war init >/dev/null 2>&1
PS_P=$(ps_war create "Parent work" --item "own item" 2>&1 | grep -o 'docs/[a-z]*/[tw]-[0-9a-f]*' | head -1 | sed 's|.*/||')
PS_C1=$(ps_war create "Child one" --part-of "$PS_P" --item one 2>&1 | grep -o 'docs/[a-z]*/[tw]-[0-9a-f]*' | head -1 | sed 's|.*/||')
PS_C2=$(ps_war create "Child two" --part-of "$PS_P" --item two 2>&1 | grep -o 'docs/[a-z]*/[tw]-[0-9a-f]*' | head -1 | sed 's|.*/||')
ps_state() { ps_war view warrants 2>&1 | awk -v id="$PS_P" '$1==id {print ($2=="in" ? "in_progress" : $2)}'; }
ps_show()  { ps_war view show "$PS_P" 2>&1 | sed -n 3p | awk '{print ($1=="in" ? "in_progress" : $1)}'; }
ps_prime() { ps_war view prime "$PS_P" 2>&1 | sed -n 3p | awk '{print ($1=="in" ? "in_progress" : $1)}'; }

if [[ -z $PS_P || -z $PS_C1 || -z $PS_C2 ]]; then
    ps_fail "setup" "parent '$PS_P' children '$PS_C1' '$PS_C2'"
else
    # Refusal first: nothing done or claimed.
    if [[ $(ps_state) == open && $(ps_show) == open && $(ps_prime) == open ]]; then
        ps_ok "no child done: parent open" "$PS_P open in warrants, show and prime"
    else
        ps_fail "no child done: parent open" "warrants '$(ps_state)' show '$(ps_show)' prime '$(ps_prime)'"
    fi
    ps_war done "$PS_C1" >/dev/null 2>&1
    if [[ $(ps_state) == in_progress && $(ps_show) == in_progress && $(ps_prime) == in_progress ]]; then
        ps_ok "one child done: in_progress" "$PS_P in_progress in warrants, show and prime"
    else
        ps_fail "one child done: in_progress" "warrants '$(ps_state)' show '$(ps_show)' prime '$(ps_prime)'"
    fi
    ps_war done "$PS_C2" >/dev/null 2>&1
    if [[ $(ps_state) == in_progress && $(ps_show) == in_progress ]]; then
        ps_ok "children done, own item open" "$PS_P in_progress, not done"
    else
        ps_fail "children done, own item open" "warrants '$(ps_state)' show '$(ps_show)'"
    fi
    ps_war done "$PS_P" >/dev/null 2>&1
    if [[ $(ps_state) == done && $(ps_show) == done ]]; then
        ps_ok "everything done: done" "$PS_P done"
    else
        ps_fail "everything done: done" "warrants '$(ps_state)' show '$(ps_show)'"
    fi
fi
[[ -n "$PS_TMP" && "$PS_TMP" == */tmp.* ]] && /usr/bin/rm -rf "$PS_TMP"
unset PS_TMP PS_WAR PS_P PS_C1 PS_C2
unset -f ps_ok ps_fail ps_war ps_id ps_state ps_show ps_prime
