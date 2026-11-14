# shellcheck shell=bash
# `war create --part-of roadmap/<phase>`: new work stands under a phase of the
# Roadmap master (docs/work/roadmap/roadmap.md), in either ticket layout. Each
# claim is paired with its refusal.
#
# Accepted: the work's document says `part_of: roadmap/PH-1`; `war check`
# passes; `war admin compile` lists it in docs/ROADMAP.md under that phase's
# heading, not under "Unphased"; `--part-of` naming a ticket still makes it
# that ticket's child.
# Refused, nothing written: a phase the Roadmap does not declare
# (`ticket.part-of-phase`, naming the phases it has); and, with no Roadmap,
# any phase (`ticket.part-of-phase`, saying where the Roadmap goes).

echo "== create --part-of roadmap/<phase> =="
PP_WAR="$REPO_ROOT/${WAR#./}"
pp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
pp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
PP_R=$(scratch_corpus PP)
[[ -d "${PP_R:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
PP_T=$(mktemp -d)
pp_war() {
    (cd "$PP_R" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID \
        -u KATANA_SESSION_ID -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
        OPENWARRANT_ACTOR=plant timeout 120 "$PP_WAR" "$@" </dev/null >"$PP_T/out" 2>"$PP_T/err")
}
pp_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"].get("id",""))' "$PP_T/out" 2>/dev/null; }
# What create writes: every ticket document, either layout.
pp_docs() { { find "$PP_R/docs/tickets" -maxdepth 1 -name 't-*.md'; find "$PP_R/docs/work" -mindepth 2 -maxdepth 2 -name '[tw]-*.md'; } 2>/dev/null | wc -l; }

# Refused first: no Roadmap, so no phase.
PP_N0=$(pp_docs)
pp_war --json create "Ship the export" --part-of roadmap/PH-1 --item one; PP_RC=$?
if [[ $PP_RC -ne 0 ]] && grep -q 'ticket.part-of-phase' "$PP_T/out" "$PP_T/err" \
    && grep -q 'there is no Roadmap' "$PP_T/out" "$PP_T/err" && [[ $(pp_docs) -eq $PP_N0 ]]; then
    pp_ok "no Roadmap, no phase" "ticket.part-of-phase; it says where the Roadmap goes; nothing written"
else
    pp_fail "no Roadmap, no phase" "exit $PP_RC, $(pp_docs) document(s) (had $PP_N0): $(head -c 200 "$PP_T/err")"
fi

mkdir -p "$PP_R/docs/work/roadmap"
printf -- '---\nformat: openwarrant/1.0\nprofile: roadmap\nalias: roadmap\nuuid: 01a1b788-533f-76ca-8adf-73e0ddb95601\nsummary: The plan, by phase.\n---\n# Roadmap\n\n## PH-1 · phase — First things\n\n## PH-2 · phase — Later things\n' \
    > "$PP_R/docs/work/roadmap/roadmap.md"
git -C "$PP_R" add -A >/dev/null 2>&1
git -C "$PP_R" -c user.email=plant@invalid -c user.name=plant commit -qm roadmap >/dev/null 2>&1

pp_war --json create "Ship the export" --part-of roadmap/PH-1 --item one; PP_RC=$?
PP_ID=$(pp_id)
PP_DOC=$(grep -l "^alias: $PP_ID\$" "$PP_R"/docs/tickets/*.md "$PP_R"/docs/work/*/*.md 2>/dev/null | head -1)
pp_war check; PP_CHECK=$?
pp_war admin compile >/dev/null 2>&1
PP_UNDER=$(awk '/^## First things/{f=1;next} /^## /{f=0} f' "$PP_R/docs/ROADMAP.md" 2>/dev/null)
if [[ $PP_RC -eq 0 && -n "$PP_ID" && -n "$PP_DOC" ]] && grep -qx 'part_of: "\?roadmap/PH-1"\?' "$PP_DOC" \
    && [[ $PP_CHECK -eq 0 && "$PP_UNDER" == *"$PP_ID"* ]]; then
    pp_ok "work stands under its phase" "$PP_ID part_of roadmap/PH-1; check passes; ROADMAP.md lists it under First things"
else
    pp_fail "work stands under its phase" "exit $PP_RC id '$PP_ID' doc '${PP_DOC#"$PP_R"/}' check $PP_CHECK; under PH-1: '$(tr '\n' '|' <<<"$PP_UNDER" | head -c 200)'"
fi

PP_N1=$(pp_docs)
pp_war --json create "Ship the import" --part-of roadmap/PH-9 --item one; PP_RC=$?
if [[ $PP_RC -ne 0 ]] && grep -q 'ticket.part-of-phase' "$PP_T/out" "$PP_T/err" \
    && grep -q 'roadmap/PH-1 (First things), roadmap/PH-2 (Later things)' "$PP_T/out" "$PP_T/err" \
    && [[ $(pp_docs) -eq $PP_N1 ]]; then
    pp_ok "an undeclared phase is refused" "ticket.part-of-phase names PH-1 and PH-2; nothing written"
else
    pp_fail "an undeclared phase is refused" "exit $PP_RC, $(pp_docs) document(s) (had $PP_N1): $(head -c 300 "$PP_T/out")"
fi

# A ticket is still a parent: --part-of names an epic as before.
pp_war --json create "Ship the parts" --part-of "$PP_ID" --item one; PP_RC=$?
PP_CHILD=$(pp_id)
PP_CDOC=$(grep -l "^alias: $PP_CHILD\$" "$PP_R"/docs/tickets/*.md "$PP_R"/docs/work/*/*.md 2>/dev/null | head -1)
if [[ $PP_RC -eq 0 && -n "$PP_CDOC" ]] && grep -qx "part_of: \"\?$PP_ID\"\?" "$PP_CDOC"; then
    pp_ok "a ticket is still a parent" "$PP_CHILD part_of $PP_ID"
else
    pp_fail "a ticket is still a parent" "exit $PP_RC: $(head -c 200 "$PP_T/err")"
fi

[[ -n "$PP_T" && "$PP_T" == */tmp.* ]] && /usr/bin/rm -rf "$PP_T"
corpus_gone "$PP_R"
unset PP_WAR PP_R PP_T PP_N0 PP_N1 PP_RC PP_ID PP_DOC PP_CHECK PP_UNDER PP_CHILD PP_CDOC
unset -f pp_ok pp_fail pp_war pp_id pp_docs
