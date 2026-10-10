# shellcheck shell=bash
# OW-WAR-0148 M17 (decision 24; docs/LEDGER.md): the file ledger.
#
# On a scratch program (LG), each claim paired with a refusal:
# - the three writer modes: a `Ledger: <path>: <why>` trailer speaks for its
#   file, the body's first paragraph for a commit without one, and the
#   deterministic fallback is the cited Warrant's title (or the subject).
#   Agent mode asks: `war admin ledger record` without `--why` writes nothing
#   and is refused (ledger.why-needed, exit 2); with `--why` it records the
#   line; `war done` in agent mode prints the prompt.
# - pruning keeps exactly N entries plus a summary: five commits to one file
#   with `keep = 3` leave three entries and one `earlier` line counting two,
#   while `git log` still lists all five. Refused: `keep = 0` (ledger.config).
# - the JSONL is regenerated and never committed: `war admin compile` writes
#   .openwarrant/ledger.jsonl (one line per file), git ignores it (`git
#   check-ignore`), and `war view prime` points to it. Refused: a commit that
#   force-adds it fails `war check` (ledger.committed).
# - a malformed atom line is refused by `war check` (ledger.atom).

echo "== the file ledger: writer modes, pruning, a JSONL never committed (M17) =="
LG_TMP=$(mktemp -d)
LG_ROOT=$(scratch_corpus LG)
[[ -d "${LG_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
LG_WAR="$REPO_ROOT/${WAR#./}"
lg_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
lg_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
lg_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$LG_WAR" --root "$LG_ROOT" "$@" </dev/null; }
lg_git() { git -C "$LG_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false "$@"; }
lg_commit() { # file content message...
    mkdir -p "$(dirname "$LG_ROOT/$1")"
    printf '%s\n' "$2" > "$LG_ROOT/$1"
    lg_git add "$1" >/dev/null 2>&1
    shift 2
    local args=()
    for m in "$@"; do args+=(-m "$m"); done
    lg_git commit -q "${args[@]}" >/dev/null 2>&1
}
lg_line() { grep -F -- "$2" "$LG_ROOT/docs/ledger/$1.md" 2>/dev/null | head -n1; }

LG_ID=$(lg_war --json create "Fix the login redirect" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])' 2>/dev/null)
lg_git add -A >/dev/null 2>&1
lg_git commit -qm "Ticket" >/dev/null 2>&1

# --- the writer modes -------------------------------------------------------------------------
mkdir -p "$LG_ROOT/src"
printf 'a\n' > "$LG_ROOT/src/a.rs"; printf 'b\n' > "$LG_ROOT/src/b.rs"
lg_git add -A >/dev/null 2>&1
lg_git commit -q -m "Touch a and b" -m "Ledger: src/a.rs: the redirect read the session first" -m "Warrant: $LG_ID" >/dev/null 2>&1
lg_war admin ledger record >/dev/null 2>&1
lg_commit src/c.rs c "Add c" "The cookie domain comes from config now."
lg_war admin ledger record >/dev/null 2>&1
lg_commit src/d.rs d "Add d" "Warrant: $LG_ID"
lg_war admin ledger record >/dev/null 2>&1
LG_A=$(lg_line src/a.rs "(commit): ")
LG_B=$(lg_line src/b.rs " $LG_ID (")
LG_C=$(lg_line src/c.rs "(commit): ")
LG_D=$(lg_line src/d.rs "(deterministic): ")
if [[ "$LG_A" == *" $LG_ID (commit): the redirect read the session first" \
    && "$LG_B" == *"(deterministic): Fix the login redirect" \
    && "$LG_C" == *"- (commit): The cookie domain comes from config now." \
    && "$LG_D" == *" $LG_ID (deterministic): Fix the login redirect" ]]; then
    lg_ok "the commit and deterministic modes" "a per-file Ledger: trailer, the body, and the Warrant's title as fallback"
else
    lg_fail "the commit and deterministic modes" "a '$LG_A'; b '$LG_B'; c '$LG_C'; d '$LG_D'"
fi
cp "$LG_ROOT/openwarrant.toml" "$LG_TMP/orig.toml"
printf '\n[ledger]\nmode = "agent"\n' >> "$LG_ROOT/openwarrant.toml"
lg_commit src/e.rs e "Add e"
LG_ASK=$(lg_war admin ledger record 2>&1); LG_ASK_RC=$?
LG_ASK_FILE=$([[ -f "$LG_ROOT/docs/ledger/src/e.rs.md" ]] && echo written || echo none)
lg_war admin ledger record HEAD --why "e holds the empty-URL case" >/dev/null 2>&1
LG_E=$(lg_line src/e.rs "(agent): ")
lg_war claim "$LG_ID" >/dev/null 2>&1
LG_DONE=$(lg_war done "$LG_ID" 2>&1)
if [[ $LG_ASK_RC -eq 2 && "$LG_ASK_FILE" == "none" && "$LG_E" == *"(agent): e holds the empty-URL case" ]] \
    && line_has -F 'ledger.why-needed' -F 'war admin ledger record --why' <<<"$LG_ASK" \
    && line_has -F 'Ledger:' -F "war admin ledger record --why \"...\" --warrant $LG_ID" <<<"$LG_DONE"; then
    lg_ok "agent mode asks, then records" "no --why: ledger.why-needed, exit 2, nothing written; --why recorded; war done prints the prompt"
else
    lg_fail "agent mode asks, then records" "exit $LG_ASK_RC ($LG_ASK_FILE): $LG_ASK; e '$LG_E'; done: $LG_DONE"
fi
cat "$LG_TMP/orig.toml" > "$LG_ROOT/openwarrant.toml"
lg_git add -A >/dev/null 2>&1
lg_git commit -qm "Record" >/dev/null 2>&1

# --- pruning ----------------------------------------------------------------------------------
for i in 1 2 3 4 5; do
    lg_commit src/p.rs "p$i" "Change p $i" "Reason $i for p."
    lg_war admin ledger record >/dev/null 2>&1
done
LG_N=$(grep -c '^- 20' "$LG_ROOT/docs/ledger/src/p.rs.md")
LG_E5=$(lg_line src/p.rs "- earlier: ")
LG_LOG=$(git -C "$LG_ROOT" log --oneline -- src/p.rs | wc -l)
printf '\n[ledger]\nkeep = 0\n' >> "$LG_ROOT/openwarrant.toml"
LG_ZERO=$(lg_war check 2>&1); LG_ZERO_RC=$?
cat "$LG_TMP/orig.toml" > "$LG_ROOT/openwarrant.toml"
if [[ "$LG_N" == "3" && "$LG_E5" == "- earlier: 2 change(s), "*"Reason 1 for p. | Reason 2 for p."* && "$LG_LOG" == "5" ]]; then
    lg_ok "pruning keeps N plus a summary" "5 changes, keep 3: 3 entries + earlier (2 change(s)); git log keeps all 5"
else
    lg_fail "pruning keeps N plus a summary" "entries $LG_N; earlier '$LG_E5'; git log $LG_LOG"
fi
if [[ $LG_ZERO_RC -ne 0 ]] && line_has -F 'ledger.config' -F 'keep = 0' <<<"$LG_ZERO"; then
    lg_ok "keep = 0 is refused" "ledger.config: an atom keeps at least 1 entry"
else
    lg_fail "keep = 0 is refused" "exit $LG_ZERO_RC: $(grep -m1 ledger <<<"$LG_ZERO")"
fi

# --- the JSONL --------------------------------------------------------------------------------
lg_war admin compile >/dev/null 2>&1
LG_JSONL="$LG_ROOT/.openwarrant/ledger.jsonl"
LG_LINES=$(wc -l < "$LG_JSONL" 2>/dev/null)
LG_ATOMS=$(find "$LG_ROOT/docs/ledger" -name '*.md' | wc -l)
LG_PARSE=$(python3 -c 'import json,sys; ls=[json.loads(l) for l in open(sys.argv[1])]; print(all(l["schema"]=="oh.war/ledger-file/v1" for l in ls), [l["path"] for l in ls] == sorted(l["path"] for l in ls))' "$LG_JSONL" 2>&1)
LG_IGN=$(git -C "$LG_ROOT" check-ignore .openwarrant/ledger.jsonl 2>&1)
LG_PORCELAIN=$(git -C "$LG_ROOT" status --porcelain --untracked-files=all -- .openwarrant)
LG_PRIME=$(lg_war view prime 2>&1)
command rm -f "$LG_JSONL"
lg_war admin compile >/dev/null 2>&1
LG_AGAIN=$(wc -l < "$LG_JSONL" 2>/dev/null)
if [[ -n "$LG_LINES" && "$LG_LINES" == "$LG_ATOMS" && "$LG_AGAIN" == "$LG_LINES" && "$LG_PARSE" == "True True" \
    && "$LG_IGN" == ".openwarrant/ledger.jsonl" && -z "$LG_PORCELAIN" ]] \
    && line_has -F '.openwarrant/ledger.jsonl' -F 'in one go' <<<"$LG_PRIME"; then
    lg_ok "the JSONL is regenerated, ignored" "$LG_LINES line(s), one per atom, by path; regenerated after removal; git check-ignore; prime points to it"
else
    lg_fail "the JSONL is regenerated, ignored" "lines $LG_LINES/$LG_ATOMS, again $LG_AGAIN, parse '$LG_PARSE', ignore '$LG_IGN', status '$LG_PORCELAIN'"
fi
lg_git add -A >/dev/null 2>&1
lg_git commit -qm "Ledger and config" >/dev/null 2>&1
LG_CLEAN=$(lg_war check 2>&1); LG_CLEAN_RC=$?
lg_git add -f .openwarrant/ledger.jsonl >/dev/null 2>&1
lg_git commit -qm "Commit the JSONL" >/dev/null 2>&1
LG_COMMITTED=$(lg_war check 2>&1); LG_COMMITTED_RC=$?
if [[ $LG_COMMITTED_RC -ne 0 ]] && ! line_has -F 'ledger.' -E '^ERROR' <<<"$LG_CLEAN" \
    && line_has -F 'ledger.committed' -E '^ERROR' <<<"$LG_COMMITTED"; then
    lg_ok "a committed JSONL is refused" "war check: ERROR ledger.committed once it is tracked (none before), exit $LG_COMMITTED_RC"
else
    lg_fail "a committed JSONL is refused" "before exit $LG_CLEAN_RC; after exit $LG_COMMITTED_RC: $(grep -m1 ledger <<<"$LG_COMMITTED")"
fi
lg_git rm -q --cached .openwarrant/ledger.jsonl >/dev/null 2>&1
lg_git commit -qm "Untrack the JSONL" >/dev/null 2>&1

# --- a malformed atom ---------------------------------------------------------------------------
printf -- '- yesterday - - (rumour): hand-written\n' >> "$LG_ROOT/docs/ledger/src/a.rs.md"
LG_BAD=$(lg_war check 2>&1); LG_BAD_RC=$?
if [[ $LG_BAD_RC -ne 0 ]] && line_has -F 'ledger.atom' -E '^ERROR' <<<"$LG_BAD"; then
    lg_ok "a malformed atom is refused" "war check: ERROR ledger.atom, by line"
else
    lg_fail "a malformed atom is refused" "exit $LG_BAD_RC: $(grep -m1 ledger <<<"$LG_BAD")"
fi

command rm -rf "$LG_TMP" "$LG_ROOT"
