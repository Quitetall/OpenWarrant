# shellcheck shell=bash
# The first field report on alpha.7 (2026-11-13): a project with a roadmap,
# a spec, tickets tied to phases and one single-file delivery Warrant in
# docs/warrants/. Each claim beside its refusal:
#   a single-file Warrant's check runs by alias and by id, its runs kept in its
#   own state; a failing check fails there, and a name nothing has is refused;
#   it is a member of the phase it is `part_of`, and without the relation
#   `war check` says so at the document, never at a manifest it has none of;
#   `war check run` twice with no edit stales nothing, nor does a ticket
#   created meanwhile; an edit to a source file does;
#   `war start` prints the records its task packet selects and nothing else,
#   and a record too long for the brief is listed with the cut said;
#   work waits on the phases its phase depends on, by name, and `war start`
#   on it is refused; once that phase holds no open work it is ready;
#   `part_of` a phase shows its title, and an undeclared one reads unknown;
#   a ticket's test is one check and one line; `war done <ticket>` raises an
#   item ticked at claimed before the test existed, and a failing test raises
#   nothing; `war view show` prints the document's own path;
#   a default actor name is said on `war start`; a named actor adds nothing.

echo "== the alpha.7 field report (alpha.8) =="
for v in OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID \
         CLAUDE_CODE_SESSION_ID OPENWARRANT_ACTOR; do unset "$v"; done
FR_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$FR_T")
FR_R="$FR_T/repo"
FR_WAR="$REPO_ROOT/${WAR#./}"
fr_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fr_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# fr <args>: war as nobody in particular, under no harness.
fr() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u CLAUDECODE -u CODEX_SANDBOX \
           -u OPENCODE -u GEMINI_CLI -u CURSOR_AGENT OPENWARRANT_NO_PROJECTS=1 \
           "$FR_WAR" --root "$FR_R" "$@" </dev/null; }
fr_ids() { python3 -c 'import json,sys; v=json.load(sys.stdin)["result"]; print(v["id"], *[i["id"] for i in v.get("items", [])])'; }
fr_commit() {
    git -C "$FR_R" add -A >/dev/null 2>&1
    git -C "$FR_R" -c user.email=plant@invalid -c user.name=plant -c commit.gpgsign=false commit -qm "$1" >/dev/null 2>&1
}
# fr_link <ticket> <phase> [<requirement>]: the relations an author writes in the frontmatter.
fr_link() {
    local add="part_of: \"$2\""
    [[ -n "${3:-}" ]] && add="$add\nimplements:\n  - $3"
    sed -i "s/^priority: .*/$add\n&/" "$FR_R/docs/tickets/$1.md"
}
fr_tail() { tail -4 <<<"$1" | cut -c1-160 | tr '\n' '|'; }

mkdir -p "$FR_R" && git -C "$FR_R" init -q . || { printf 'PLANT SETUP FAILED: git init\n' >&2; exit 9; }
fr init --namespace PP >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init\n' >&2; exit 9; }
mkdir -p "$FR_R/docs/roadmap/atoms"
cat > "$FR_R/docs/roadmap/roadmap.toml" <<'TOML'
schema = "oh.war/roadmap/v1"
uuid = "01a0cd3a-0a24-74cb-9a4b-5a83dfe420f9"
program = "PP"
prefix = "PP"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
TOML
printf '# Intent\n\nPP, in phases.\n' > "$FR_R/docs/roadmap/atoms/10-intent.md"
cat > "$FR_R/docs/roadmap/atoms/20-phases.yaml" <<'YAML'
schema: "oh.war/roadmap-phases/v1"

tiers: []

phases:
  - id: "PP-PHASE-3"
    title: "Workspace"
    outcome: "the workspace builds"
    exit: "workspace tests pass"
    depends_on: []
    open: []

  - id: "PP-PHASE-4"
    title: "Web"
    outcome: "the web app"
    exit: "ci-web passes"
    depends_on: ["PP-PHASE-3"]
    open: []

  - id: "PP-PHASE-5"
    title: "Release"
    outcome: "released"
    exit: "tagged"
    depends_on: ["PP-PHASE-4"]
    open: []
YAML
{
    printf -- '---\nformat: openwarrant/1.0\nprofile: spec\nuuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01\nversion: 0.1.0\nsummary: PP.\n---\n# PP\n\n'
    printf '## Purpose\nPreamble no ticket needs: PURPOSE-MARK.\n\n## Scope\nIn: everything.\n\n'
    printf '## REQ-ws-1 · requirement — the workspace builds\nstatement: The workspace MUST build. REQ-MARK\n\n'
    printf '## CON-ws-1 · constraint — no datastore\nconstrains REQ-ws-1\nstatement: It MUST NOT add a datastore. CON-MARK\n\n'
    printf '## DEC-ws-1 · decision — one crate\nselected_over OPT-ws-1\nconstrains REQ-ws-1\n\nOne crate. DEC-MARK\n\n'
    printf '## OPT-ws-1 · option — many crates\n\nRejected. OPT-MARK\n\n'
    printf '## REQ-long-1 · requirement — a long one\nstatement: It MUST be short. REQ-LONG-MARK\n\n'
    printf '## CON-long-1 · constraint — a long constraint\nconstrains REQ-long-1\nstatement: %sCON-LONG-MARK\n' "$(printf 'Long prose. %.0s' $(seq 1 700))"
} > "$FR_R/SPEC.md"
printf 'test "$(cat ok)" = yes\n' > "$FR_R/ok.sh"; printf 'yes\n' > "$FR_R/ok"; printf 'source\n' > "$FR_R/src.txt"
fr plan new "Workspace builds" --format 1.0 >/dev/null 2>&1
FR_DOC="$FR_R/docs/warrants/PP-WAR-0001.md"
[[ -f "$FR_DOC" ]] || { printf 'PLANT SETUP FAILED: plan new --format 1.0 wrote no %s\n' "$FR_DOC" >&2; exit 9; }
sed -i 's/^summary: .*/part_of: "PP-PHASE-3"\n&/' "$FR_DOC"
printf '\n## CHK-workspace · check — workspace tests pass\nrun: sh ok.sh\non: manual\nevaluates OBL-001\n' >> "$FR_DOC"
read -r FR_T3 FR_I1 FR_I2 FR_I3 <<<"$(fr --json create "Phase three work" --item first --item second --item third | fr_ids)"
read -r FR_T4 FR_J1 <<<"$(fr --json create "Phase four work" --item web | fr_ids)"
read -r FR_T5 _ <<<"$(fr --json create "Phase five work" --item tag | fr_ids)"
read -r FR_TL _ <<<"$(fr --json create "Long record" --item one | fr_ids)"
read -r FR_T9 _ <<<"$(fr --json create "Lost phase" --item one | fr_ids)"
[[ -n "$FR_T3" && -n "$FR_T4" && -n "$FR_T5" && -n "$FR_TL" && -n "$FR_T9" ]] \
    || { printf 'PLANT SETUP FAILED: tickets not created\n' >&2; exit 9; }
fr_link "$FR_T3" PP-PHASE-3 REQ-ws-1; fr_link "$FR_T4" PP-PHASE-4; fr_link "$FR_T5" PP-PHASE-5
fr_link "$FR_TL" PP-PHASE-3 REQ-long-1; fr_link "$FR_T9" PP-PHASE-9
fr plan add "$FR_T4/$FR_J1" --test "sh ok.sh" --name web >/dev/null 2>&1
fr_commit "the project"

# ---- group 1: a single-file Warrant is a work document like any other -------------
FR_BAD=""
for fr_target in PP-WAR-0001 CHK-workspace PP-WAR-0001/CHK-workspace; do
    FR_OUT=$(fr check run "$fr_target" 2>&1); FR_S=$?
    [[ $FR_S -eq 0 ]] && grep -q 'PP-WAR-0001/CHK-workspace PASS' <<<"$FR_OUT" || FR_BAD="$FR_BAD $fr_target($FR_S)"
done
if [[ -z "$FR_BAD" && -f "$FR_R/docs/warrants/PP-WAR-0001/journal.jsonl" && ! -e "$FR_R/docs/tickets/PP-WAR-0001" ]]; then
    fr_ok "a single-file Warrant's check runs" "by alias, by id and by both; runs kept in docs/warrants/PP-WAR-0001/"
else
    fr_fail "a single-file Warrant's check runs" "refused or misplaced:$FR_BAD"
fi
printf 'no\n' > "$FR_R/ok"
FR_OUT=$(fr check run PP-WAR-0001 2>&1); FR_S=$?
FR_OUT2=$(fr check run CHK-absent 2>&1); FR_S2=$?
printf 'yes\n' > "$FR_R/ok"
if [[ $FR_S -ne 0 && $FR_S2 -eq 2 ]] && grep -q 'check.failed' <<<"$FR_OUT" && grep -q 'check.nothing' <<<"$FR_OUT2"; then
    fr_ok "its failing check fails" "exit $FR_S check.failed; an unknown name is check.nothing"
else
    fr_fail "its failing check fails" "exit $FR_S/$FR_S2: $(fr_tail "$FR_OUT")"
fi
FR_CHECK=$(fr check 2>&1)
FR_MEMBERS=$(fr --json plan roadmap 2>/dev/null | python3 -c '
import json, sys
for p in json.load(sys.stdin)["result"]["phases"]:
    print(p["id"], ",".join(p["members"]) or "-")' | tr '\n' ' ')
if [[ "$FR_MEMBERS" == "PP-PHASE-3 PP-WAR-0001 PP-PHASE-4 - PP-PHASE-5 - " ]] \
    && ! grep -q 'roadmap.unassigned\|corpus.empty' <<<"$FR_CHECK"; then
    fr_ok "part_of a phase is membership" "PP-PHASE-3 lists PP-WAR-0001; check has no corpus.empty or unassigned"
else
    fr_fail "part_of a phase is membership" "members '$FR_MEMBERS': $(grep 'roadmap.unassigned\|corpus.empty' <<<"$FR_CHECK" | cut -c1-200)"
fi
sed -i '/^part_of: "PP-PHASE-3"$/d' "$FR_DOC"
FR_CHECK=$(fr check 2>&1)
git -C "$FR_R" checkout -q -- docs/warrants/PP-WAR-0001.md
if grep -q 'roadmap.unassigned' <<<"$FR_CHECK" && grep -q 'docs/warrants/PP-WAR-0001.md' <<<"$FR_CHECK" \
    && ! grep -q 'PP-WAR-0001/manifest.toml' <<<"$FR_CHECK"; then
    fr_ok "no relation, no membership" "roadmap.unassigned at docs/warrants/PP-WAR-0001.md"
else
    fr_fail "no relation, no membership" "$(grep -A1 'roadmap.unassigned' <<<"$FR_CHECK" | cut -c1-200 | tr '\n' '|')"
fi

# ---- group 2: work waits on the phases before its own ---------------------------
FR_NEXT=$(fr view next 2>&1); FR_PRIME=$(fr view prime 2>&1)
if grep -q "$FR_T3" <<<"$FR_NEXT" && ! grep -q "$FR_T4\|$FR_T5" <<<"$FR_NEXT" \
    && grep -q "$FR_T4 .*waits on PP-PHASE-3" <<<"$FR_PRIME" && grep -q "$FR_T5 .*waits on PP-PHASE-3, PP-PHASE-4" <<<"$FR_PRIME"; then
    fr_ok "a later phase's work is not ready" "phase 4 waits on PP-PHASE-3; phase 5 on both, by name"
else
    fr_fail "a later phase's work is not ready" "next: $(fr_tail "$FR_NEXT") prime: $(grep 'waits on' <<<"$FR_PRIME" | tr '\n' '|')"
fi
FR_OUT=$(fr start "$FR_T4" 2>&1); FR_S=$?
if [[ $FR_S -ne 0 ]] && grep -q 'ticket.blocked' <<<"$FR_OUT" && grep -q 'PP-PHASE-3' <<<"$FR_OUT"; then
    fr_ok "taking it by name is refused" "ticket.blocked, naming PP-PHASE-3"
else
    fr_fail "taking it by name is refused" "exit $FR_S: $(fr_tail "$FR_OUT")"
fi
FR_SHOW=$(fr view show "$FR_T4" 2>&1); FR_SHOW9=$(fr view show "$FR_T9" 2>&1)
if grep -qF 'part of PP-PHASE-4 (Web)' <<<"$FR_SHOW" && grep -qF 'part of PP-PHASE-9 (unknown)' <<<"$FR_SHOW9" \
    && grep -qF "\`docs/tickets/$FR_T4.md\`" <<<"$FR_SHOW" && ! grep -q 'atoms/30-checks.md' <<<"$FR_SHOW"; then
    fr_ok "part_of a phase shows its title" "PP-PHASE-4 (Web); an undeclared phase is (unknown); the path is the document's"
else
    fr_fail "part_of a phase shows its title" "$(grep 'part of\|docs/tickets\|30-checks' <<<"$FR_SHOW$FR_SHOW9" | cut -c1-160 | tr '\n' '|')"
fi

# ---- group 3: start carries what the work rests on, and says a default name -----
FR_OUT=$(fr start "$FR_T3" 2>&1); FR_S=$?
if [[ $FR_S -eq 0 ]] && grep -q 'REQ-MARK' <<<"$FR_OUT" && grep -q 'CON-MARK' <<<"$FR_OUT" && grep -q 'DEC-MARK' <<<"$FR_OUT" \
    && grep -q '^> REQ-ws-1 · requirement · SPEC.md:' <<<"$FR_OUT" \
    && ! grep -q 'OPT-MARK\|PURPOSE-MARK\|Ordinary coding needs no Warrant' <<<"$FR_OUT"; then
    fr_ok "start prints the linked records" "REQ, CON and DEC whole ($(wc -c <<<"$FR_OUT") bytes); no option, preamble or AGENTS.md"
else
    fr_fail "start prints the linked records" "exit $FR_S: $(grep -c MARK <<<"$FR_OUT") marks: $(fr_tail "$FR_OUT")"
fi
if grep -qF 'acting as claude, the default name: `--as <name>` or OPENWARRANT_ACTOR' <<<"$FR_OUT"; then
    fr_ok "a default actor is said" "one line on war start"
else
    fr_fail "a default actor is said" "$(fr_tail "$FR_OUT")"
fi
FR_OUT=$(OPENWARRANT_ACTOR=ada fr start "$FR_TL" 2>&1); FR_S=$?
if [[ $FR_S -eq 0 ]] && ! grep -q 'the default name' <<<"$FR_OUT" && grep -q 'claimed by ada' <<<"$FR_OUT"; then
    fr_ok "a named actor adds no line" "claimed by ada, nothing more"
else
    fr_fail "a named actor adds no line" "exit $FR_S: $(fr_tail "$FR_OUT")"
fi
if grep -q 'REQ-LONG-MARK' <<<"$FR_OUT" && ! grep -q 'CON-LONG-MARK' <<<"$FR_OUT" \
    && grep -qF 'Not printed (this brief holds 6144 bytes)' <<<"$FR_OUT" \
    && grep -q '^- CON-long-1 · constraint · SPEC.md:' <<<"$FR_OUT" && [[ $(wc -c <<<"$FR_OUT") -lt 6656 ]]; then
    fr_ok "a record over the brief is listed" "the cut is said, and the brief is $(wc -c <<<"$FR_OUT") bytes"
else
    fr_fail "a record over the brief is listed" "$(wc -c <<<"$FR_OUT") bytes: $(grep 'Not printed\|^- CON' <<<"$FR_OUT" | tr '\n' '|')"
fi
OPENWARRANT_ACTOR=ada fr done "$FR_TL" >/dev/null 2>&1

# ---- group 4: one check for the ticket, and what was ticked before it -------------
fr done "$FR_I1" >/dev/null 2>&1
fr plan add "$FR_T3" --test "sh ok.sh" --name late >/dev/null 2>&1
printf 'no\n' > "$FR_R/ok"
FR_OUT=$(fr done "$FR_T3" --check 2>&1); FR_S=$?
FR_SHOW=$(fr view show "$FR_T3" 2>&1)
printf 'yes\n' > "$FR_R/ok"
if [[ $FR_S -ne 0 ]] && grep -qF -- "- [x] (claimed) first ($FR_I1)" <<<"$FR_SHOW" && grep -qF -- "- [ ] second ($FR_I2)" <<<"$FR_SHOW"; then
    fr_ok "a failing test raises nothing" "exit $FR_S; the earlier tick stays claimed and nothing is ticked"
else
    fr_fail "a failing test raises nothing" "exit $FR_S: $(grep '^- \[' <<<"$FR_SHOW" | tr '\n' '|')"
fi
FR_OUT=$(fr done "$FR_T3" --check 2>&1); FR_S=$?
FR_SHOW=$(fr view show "$FR_T3" 2>&1)
if [[ $FR_S -eq 0 ]] && grep -qF "raised from claimed to observed: $FR_I1" <<<"$FR_OUT" && ! grep -q '(claimed)' <<<"$FR_SHOW" \
    && [[ $(grep -c '^## CHK-late' "$FR_R/docs/tickets/$FR_T3.md") == 1 && $(grep -c '^- test late:' <<<"$FR_SHOW") == 1 ]] \
    && grep -qF "(for $FR_I1, $FR_I2, $FR_I3)" <<<"$FR_SHOW"; then
    fr_ok "done --check raises the earlier tick" "one check, one line for three items; $FR_I1 raised to observed"
else
    fr_fail "done --check raises the earlier tick" "exit $FR_S: $(fr_tail "$FR_OUT") $(grep 'test late\|claimed' <<<"$FR_SHOW" | tr '\n' '|')"
fi
FR_NEXT=$(fr view next 2>&1)
if grep -q "$FR_T4" <<<"$FR_NEXT" && ! grep -q "$FR_T5" <<<"$FR_NEXT"; then
    fr_ok "a finished phase releases the next" "phase 4's work is ready; phase 5's still waits"
else
    fr_fail "a finished phase releases the next" "$(fr_tail "$FR_NEXT")"
fi

# ---- group 5: an observation is stale only when source changes --------------------
fr_commit "phase three done"
FR_BAD=""
for fr_round in 1 2; do
    FR_OUT=$(fr check run 2>&1); FR_S=$?
    [[ $FR_S -eq 0 ]] && ! grep -q 'source-stale\| stale by ' <<<"$FR_OUT" || FR_BAD="$FR_BAD round $fr_round($FR_S)"
done
fr create "Tracked meanwhile" --item one >/dev/null 2>&1
FR_OUT=$(fr check evaluate 2>&1)
grep -q 'source-stale' <<<"$FR_OUT" && FR_BAD="$FR_BAD after-create"
if [[ -z "$FR_BAD" ]]; then
    fr_ok "its own receipts stale nothing" "two runs with no edit, then a ticket created: no check.source-stale"
else
    fr_fail "its own receipts stale nothing" "stale at:$FR_BAD: $(grep 'stale' <<<"$FR_OUT" | cut -c1-200 | tr '\n' '|')"
fi
printf 'changed\n' > "$FR_R/src.txt"
FR_OUT=$(fr check evaluate 2>&1)
if grep -q 'check.source-stale' <<<"$FR_OUT"; then
    fr_ok "a source edit does stale it" "check.source-stale after src.txt changed"
else
    fr_fail "a source edit does stale it" "$(fr_tail "$FR_OUT")"
fi
