# shellcheck shell=bash
# t-3c16c W3, steps 1 to 5 — live updates to running agents on one machine.
#
# One scratch program on `main` and two worktrees, each with one claim: Alice's
# ticket names docs/plan.md, Bob's does not. Everything below is a claim paired
# with its refusal.
#
# Accepted: a change on main reaches the affected agent, as one line on its next
# `war` call, once; `war start` prints only the changes and records them seen;
# a later change is heard again; `war view prime` keeps the line until seen; a
# plain uncommitted edit on main counts; a relation and overlapping paths reach
# a claim; `war plan impact` names the claims it reaches; `--json` carries the
# line in `notes`, so stdout stays one envelope.
# Refused: the unaffected agent hears nothing, and neither does an agent with
# the same name in another worktree; the second call is silent; a change on a
# side branch is nobody's news; an actor's own edit is not news; a claim whose
# branch holds a copy newer than main hears nothing; nothing reached hears
# nothing; a broken notices directory never fails a read, and a write only
# warns; a hosted or recording run leaves a waiting notice where it lies and
# prints nothing; delivery changes no tracked file; `OPENWARRANT_NO_NOTICES`
# turns it off; `war done` leaves no manifest and no notice behind.

echo "== live updates: who hears of a change on main (t-3c16c W3) =="
LU_ROOT=$(scratch_corpus LU)
[[ -d "${LU_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
LU_WAR="$REPO_ROOT/${WAR#./}"
lu_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
lu_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
LU_T=$(mktemp -d)
lu_g() { git -C "$LU_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
# lu_as <actor> <checkout> <war args...>: stdout and stderr to $LU_T/out, $LU_T/err.
lu_as() {
    local actor="$1" at="$2"; shift 2
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR="$actor" \
        "$LU_WAR" --root "$at" "$@" >"$LU_T/out" 2>"$LU_T/err" </dev/null
}
lu_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$1"; }
lu_create() { lu_as owner "$LU_ROOT" --json create "$@"; lu_id "$LU_T/out"; }
lu_heard() { grep -c '^notice: ' "$LU_T/err"; }
lu_notices() { find "$LU_ROOT/.git/openwarrant/notices" -maxdepth 1 -name '*.json' 2>/dev/null | wc -l; }
LU_N="$LU_ROOT/.git/openwarrant/notices"

lu_g branch -M main
mkdir -p "$LU_ROOT/docs"
printf '# Plan\n\n## One\n\nfirst\n\n## Two\n\nsecond\n' >"$LU_ROOT/docs/plan.md"
printf '# Other\n\nnobody cites this\n' >"$LU_ROOT/docs/other.md"
LU_EPIC=$(lu_create "The programme")
LU_A=$(lu_create "Alice works from the plan" --body "Follow docs/plan.md for this.")
LU_B=$(lu_create "Bob works elsewhere" --body "Nothing cited here.")
LU_C=$(lu_create "Carol is under the programme" --part-of "$LU_EPIC")
LU_P=$(lu_create "Dan touches the parser" --path 'src/parser/**')
LU_Q=$(lu_create "Queued parser notes" --path 'src/parser/lexer.rs')
lu_g add -A; lu_g commit -qm 'tickets and a plan'

lu_as alice "$LU_ROOT" --json start --worktree --print "$LU_A"
LU_WA=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["worktree"])' "$LU_T/out")
lu_as bob "$LU_ROOT" --json start --worktree --print "$LU_B"
LU_WB=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["worktree"])' "$LU_T/out")
if [[ ! -d "${LU_WA:-}" || ! -d "${LU_WB:-}" ]]; then
    printf 'PLANT SETUP FAILED: no worktrees: %s\n' "$(head -c 300 "$LU_T/err")" >&2; exit 9
fi

# --- step 1: what each was given stays beside its claim -------------------------
LU_KEPT=$(python3 - "$LU_ROOT/.git/openwarrant/claims" "$LU_A" <<'PY' 2>&1
import json, sys
k = json.load(open(f"{sys.argv[1]}/given/{sys.argv[2]}.json"))
w = {x["source"]: x["reach"] for x in k["watched"]}
print(k["schema"], any(e["id"] == sys.argv[2] for e in k["source_manifest"]), w.get("docs/plan.md"), "docs/other.md" in w)
PY
)
if [[ $LU_KEPT == "oh.war/claim-manifest/v1 True relation False" ]]; then
    lu_ok "start keeps the manifest" "beside the claim; the cited plan watched, the uncited document not"
else
    lu_fail "start keeps the manifest" "$LU_KEPT"
fi

# --- impact names the running work it reaches, and only that ---------------------
lu_as owner "$LU_ROOT" plan impact "$LU_A" --json
LU_IMP=$(python3 -c 'import json,sys; print(sorted((c["ticket"],c["via"]) for c in json.load(open(sys.argv[1]))["result"].get("claims",[])))' "$LU_T/out" 2>&1)
if [[ $LU_IMP == "[('$LU_A', 'manifest')]" ]]; then
    lu_ok "impact includes claims" "$LU_A by its manifest; Bob's claim is not listed"
else
    lu_fail "impact includes claims" "$LU_IMP"
fi

# --- step 2-5: a change on main, committed, with no `war` on main ------------------
LU_BEFORE=$(git -C "$LU_WA" status --porcelain)
sed -i 's/^second$/SECOND/' "$LU_ROOT/docs/plan.md"
lu_g add -A; lu_g commit -qm 'amend the plan'
lu_as bob "$LU_WB" note "$LU_B" "still going"
LU_BOB=$(lu_heard)
lu_as alice "$LU_WA" view ready
LU_ALICE1=$(lu_heard); LU_LINE=$(grep '^notice: ' "$LU_T/err")
lu_as alice "$LU_WA" view ready
LU_ALICE2=$(lu_heard)
lu_as bob "$LU_WB" view ready
LU_BOB2=$(lu_heard)
if [[ $LU_ALICE1 == 1 && $LU_LINE == *"docs/plan.md"*"changed on main"*"war start $LU_A"* ]]; then
    lu_ok "the affected agent hears" "$LU_LINE"
else
    lu_fail "the affected agent hears" "$LU_ALICE1: $LU_LINE"
fi
if [[ $LU_ALICE2 == 0 ]]; then lu_ok "it hears once" "the second call is silent"; else lu_fail "it hears once" "$LU_ALICE2 lines"; fi
if [[ $LU_BOB == 0 && $LU_BOB2 == 0 ]]; then lu_ok "the unaffected agent hears nothing" "in its write and in its read"; else lu_fail "the unaffected agent hears nothing" "$LU_BOB $LU_BOB2"; fi
if [[ $(git -C "$LU_WA" status --porcelain) == "$LU_BEFORE" ]]; then
    lu_ok "delivery changes no tracked file" "the worktree's status is unmoved"
else
    lu_fail "delivery changes no tracked file" "$(git -C "$LU_WA" status --porcelain | head -3 | tr '\n' '|')"
fi

# --- prime keeps it until seen, and start records it as seen -----------------------
lu_as alice "$LU_WA" view prime
if grep -q '^notice: .*docs/plan.md' "$LU_T/out"; then lu_ok "prime shows what is unseen" "one line"; else lu_fail "prime shows what is unseen" "$(tail -2 "$LU_T/out" | tr '\n' '|')"; fi
lu_as alice "$LU_WA" start "$LU_A"
if grep -q '^-SECOND\|^+SECOND' "$LU_T/out" && grep -q '^changed since you started' "$LU_T/out" \
    && ! grep -q 'Close it in the same command' "$LU_T/out"; then
    lu_ok "start prints only what changed" "the hunk, and no brief"
else
    lu_fail "start prints only what changed" "$(head -5 "$LU_T/out" | tr '\n' '|')"
fi
lu_as alice "$LU_WA" start "$LU_A"
LU_S2=$(grep -c '^changed since' "$LU_T/out")
LU_BRIEF=$(grep -c 'Close it in the same command' "$LU_T/out")
lu_as alice "$LU_WA" view prime
LU_P2=$(grep -c '^notice: ' "$LU_T/out")
if [[ $LU_S2 == 0 && $LU_BRIEF == 1 && $LU_P2 == 0 ]]; then
    lu_ok "seen stays seen" "the next start is the brief; prime is quiet"
else
    lu_fail "seen stays seen" "$LU_S2 $LU_BRIEF $LU_P2"
fi

# --- a later change is news again; a plain edit on main counts ---------------------
sed -i 's/^first$/FIRST/' "$LU_ROOT/docs/plan.md"
lu_as owner "$LU_ROOT" check --changed
lu_as alice "$LU_WA" view ready
LU_LINE=$(grep '^notice: ' "$LU_T/err")
if [[ $LU_LINE == *"docs/plan.md"*"uncommitted on main"* ]]; then
    lu_ok "a plain edit on main counts" "$LU_LINE"
else
    lu_fail "a plain edit on main counts" "[$LU_LINE] $(head -2 "$LU_T/err" | tr '\n' '|')"
fi
lu_as alice "$LU_WA" start "$LU_A"
lu_g add -A; lu_g commit -qm 'keep the edit'

# --- the JSON reader gets it in the envelope -------------------------------------
sed -i 's/^FIRST$/FIRST AGAIN/' "$LU_ROOT/docs/plan.md"
lu_g add -A; lu_g commit -qm 'amend again'
lu_as bob "$LU_WB" note "$LU_B" "another"
lu_as alice "$LU_WA" --json view ready
LU_NOTE=$(python3 -c 'import json,sys; print([n for n in json.load(open(sys.argv[1]))["notes"] if n.startswith("notice: ")])' "$LU_T/out" 2>&1)
if [[ $LU_NOTE == *"docs/plan.md"* && $(lu_heard) == 0 ]]; then
    lu_ok "--json carries it in notes" "stdout is one envelope; stderr is quiet"
else
    lu_fail "--json carries it in notes" "$LU_NOTE / $(head -2 "$LU_T/err")"
fi
lu_as alice "$LU_WA" start "$LU_A"

# --- several changes are one line --------------------------------------------------
sed -i 's/^# Plan$/# The plan/' "$LU_ROOT/docs/plan.md"
sed -i 's/^## Checklist$/## Checklist\n/' "$LU_ROOT/docs/tickets/$LU_A.md"
printf '\nA line the owner added.\n' >>"$LU_ROOT/docs/tickets/$LU_A.md"
lu_g add -A; lu_g commit -qm 'two files'
lu_as bob "$LU_WB" note "$LU_B" "third"
lu_as alice "$LU_WA" view ready
LU_LINE=$(grep '^notice: ' "$LU_T/err")
if [[ $(lu_heard) == 1 && $LU_LINE == "notice: 2 files changed"* ]]; then
    lu_ok "two files are one line" "$LU_LINE"
else
    lu_fail "two files are one line" "$(lu_heard): $LU_LINE"
fi
lu_as alice "$LU_WA" start "$LU_A"

# --- refused: a side branch is nobody's news ---------------------------------------
LU_COUNT=$(lu_notices)
sed -i 's/^# Plan$/# Plan, by Bob/;s/^# The plan$/# The plan, by Bob/' "$LU_WB/docs/plan.md"
git -C "$LU_WB" -c user.email=p@i -c user.name=p commit -qam 'bob amends on his branch'
lu_as bob "$LU_WB" note "$LU_B" "fourth"
lu_as alice "$LU_WA" view ready
if [[ $(lu_notices) == "$LU_COUNT" && $(lu_heard) == 0 ]]; then
    lu_ok "a side branch is nobody's news" "Bob's commit posted nothing"
else
    lu_fail "a side branch is nobody's news" "$(lu_notices) $(grep notice "$LU_T/err")"
fi

# --- refused: an actor's own edit is not news -------------------------------------
printf 'mine\n' >>"$LU_WA/docs/plan.md"
lu_as alice "$LU_WA" note "$LU_A" "my own edit"
lu_as alice "$LU_WA" view ready
if [[ $(lu_notices) == "$LU_COUNT" && $(lu_heard) == 0 ]]; then
    lu_ok "an own edit is not news" "nothing posted"
else
    lu_fail "an own edit is not news" "$(lu_notices) $(grep notice "$LU_T/err")"
fi
git -C "$LU_WA" checkout -q docs/plan.md

# --- reached by a relation, and by overlapping paths; the rest refused -------------
lu_as carol "$LU_ROOT" --json start --worktree --print "$LU_C"
LU_WC=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["worktree"])' "$LU_T/out")
lu_as dan "$LU_ROOT" --json start --worktree --print "$LU_P"
LU_WD=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["worktree"])' "$LU_T/out")
printf '\nThe programme moved.\n' >>"$LU_ROOT/docs/tickets/$LU_EPIC.md"
printf '\nThe queued notes moved.\n' >>"$LU_ROOT/docs/tickets/$LU_Q.md"
printf 'still nobody\n' >>"$LU_ROOT/docs/other.md"
lu_g add -A; lu_g commit -qm 'the programme, the notes, an uncited document'
lu_as owner "$LU_ROOT" note "$LU_EPIC" "said so"
lu_as carol "$LU_WC" view ready; LU_CAROL=$(grep '^notice: ' "$LU_T/err")
lu_as dan "$LU_WD" view ready; LU_DAN=$(grep '^notice: ' "$LU_T/err")
lu_as alice "$LU_WA" view ready; LU_AL=$(lu_heard)
lu_as bob "$LU_WB" view ready; LU_BO=$(lu_heard)
if [[ $LU_CAROL == *"$LU_EPIC"*"changed on main"* ]]; then lu_ok "a relation reaches a claim" "$LU_CAROL"; else lu_fail "a relation reaches a claim" "[$LU_CAROL]"; fi
if [[ $LU_DAN == *"$LU_Q"*"changed on main"* ]]; then lu_ok "overlapping paths reach a claim" "$LU_DAN"; else lu_fail "overlapping paths reach a claim" "[$LU_DAN]"; fi
if [[ $LU_AL == 0 && $LU_BO == 0 ]]; then lu_ok "nothing reached hears nothing" "Alice and Bob, of an uncited document"; else lu_fail "nothing reached hears nothing" "$LU_AL $LU_BO"; fi

# --- refused: a branch holding a newer copy is not behind ---------------------------
lu_g worktree add -q -b ahead "$LU_T/ahead" main
printf 'ahead of main\n' >>"$LU_T/ahead/docs/plan.md"
git -C "$LU_T/ahead" -c user.email=p@i -c user.name=p commit -qam 'ahead'
env -u SSH_AUTH_SOCK OPENWARRANT_ACTOR=erin "$LU_WAR" --root "$LU_ROOT" --json create "Erin is ahead" --body "Follow docs/plan.md." >"$LU_T/out" 2>/dev/null
LU_E=$(lu_id "$LU_T/out")
lu_g add -A; lu_g commit -qm 'erin'
git -C "$LU_T/ahead" -c user.email=p@i -c user.name=p merge -q --no-edit main >/dev/null 2>&1
lu_as erin "$LU_T/ahead" start "$LU_E"
printf 'unrelated tip move\n' >>"$LU_ROOT/docs/other.md"
lu_g add -A; lu_g commit -qm 'the tip moves, not the plan'
lu_as owner "$LU_ROOT" note "$LU_EPIC" "tick"
lu_as erin "$LU_T/ahead" view ready
if [[ $(lu_heard) == 0 ]]; then
    lu_ok "a newer copy is not behind" "main's older plan is no news to the branch ahead of it"
else
    lu_fail "a newer copy is not behind" "$(grep notice "$LU_T/err")"
fi

# --- refused: another agent of the same name, in another worktree, hears nothing ----
lu_g worktree add -q -b same "$LU_T/same" main
sed -i 's/^## One$/## One, renamed/' "$LU_ROOT/docs/plan.md"
lu_g add -A; lu_g commit -qm 'plan again'
lu_as bob "$LU_WB" note "$LU_B" "fifth"
lu_as alice "$LU_T/same" view ready
LU_ELSE=$(lu_heard)
lu_as alice "$LU_WA" view ready
if [[ $LU_ELSE == 0 && $(lu_heard) == 1 ]]; then
    lu_ok "same name, other worktree: silent" "only the claim's workspace hears"
else
    lu_fail "same name, other worktree: silent" "elsewhere $LU_ELSE, here $(lu_heard)"
fi

# --- refused: a notice problem never fails a read, and a write only warns ----------
lu_as alice "$LU_WA" view ready; cp "$LU_T/out" "$LU_T/good"
mv "$LU_N" "$LU_T/notices.saved"
printf 'not a directory\n' >"$LU_N"
lu_as alice "$LU_WA" view ready; LU_RC=$?
LU_SAME=$(cmp -s "$LU_T/out" "$LU_T/good" && echo same)
LU_ERR=$(cat "$LU_T/err")
sed -i 's/^## Two$/## Two, renamed/' "$LU_ROOT/docs/plan.md"
lu_g add -A; lu_g commit -qm 'plan under a broken directory'
lu_as bob "$LU_WB" note "$LU_B" "sixth"; LU_WC_RC=$?
LU_WARN=$(cat "$LU_T/err")
/usr/bin/rm -f "$LU_N"
mv "$LU_T/notices.saved" "$LU_N"
if [[ $LU_RC == 0 && $LU_SAME == same && -z $LU_ERR ]]; then
    lu_ok "a broken directory fails no read" "exit 0, the same stdout, nothing on stderr"
else
    lu_fail "a broken directory fails no read" "exit $LU_RC [$LU_SAME] $LU_ERR"
fi
if [[ $LU_WC_RC == 0 && $LU_WARN == *"notices.unavailable"* ]]; then
    lu_ok "a broken directory warns a write" "exit 0, one warning"
else
    lu_fail "a broken directory warns a write" "exit $LU_WC_RC [$LU_WARN]"
fi
printf '{ not json' >"$LU_N/garbage.json"
lu_as alice "$LU_WA" view ready; LU_RC=$?
/usr/bin/rm -f "$LU_N/garbage.json"
if [[ $LU_RC == 0 ]]; then lu_ok "a corrupt notice fails no read" "ignored"; else lu_fail "a corrupt notice fails no read" "exit $LU_RC: $(head -2 "$LU_T/err")"; fi

# --- refused: the switch, and hosted or recording runs ------------------------------
lu_list() { find "$LU_N" -maxdepth 1 -name '*.json' | sort; }
LU_OLD=$(lu_list)
sed -i 's/^## Two, renamed$/## Two, once more/' "$LU_ROOT/docs/plan.md"
lu_g add -A; lu_g commit -qm 'plan, one more'
lu_as bob "$LU_WB" note "$LU_B" "seventh"
LU_NOW=$(lu_list)
LU_WAITING=$(comm -13 <(printf '%s\n' "$LU_OLD") <(printf '%s\n' "$LU_NOW") | grep -c .)
OPENWARRANT_NO_NOTICES=1 lu_as alice "$LU_WA" view ready
LU_OFF=$(lu_heard)
"$LU_WAR" admin host --export --root "$LU_WA" >"$LU_T/req.json" 2>"$LU_T/host.err" </dev/null
LU_EXPORT=$?
(cd "$LU_WA" && OPENWARRANT_ACTOR=alice "$LU_WAR" admin host <"$LU_T/req.json" >"$LU_T/res.json" 2>"$LU_T/host2.err")
LU_HOST=$?
LU_AFTER=$(lu_list)
if [[ $LU_WAITING -ge 1 && $LU_OFF == 0 ]]; then lu_ok "the switch turns it off" "a waiting notice is left waiting"; else lu_fail "the switch turns it off" "waiting $LU_WAITING heard $LU_OFF"; fi
# A hosted run in a linked worktree answers not_established (exit 2): it has no
# `.git` directory in its basis. That is its own answer, and no failure here.
if [[ $LU_EXPORT == 0 && ( $LU_HOST == 0 || $LU_HOST == 2 ) && $LU_AFTER == "$LU_NOW" ]] && ! grep -q notice "$LU_T/host.err" "$LU_T/host2.err"; then
    lu_ok "a hosted run hears nothing" "export and host left the notice where it lay"
else
    lu_fail "a hosted run hears nothing" "export $LU_EXPORT host $LU_HOST: $(head -2 "$LU_T/host.err" "$LU_T/host2.err" | tr '\n' '|')"
fi

"$LU_WAR" admin host --export --root "$LU_ROOT" >"$LU_T/req2.json" 2>/dev/null </dev/null
if [[ $(grep -c 'claims/given/t-' "$LU_T/req2.json") == 0 ]]; then
    lu_ok "an export never lists a manifest" "what a claim was given is no member of a basis"
else
    lu_fail "an export never lists a manifest" "$(grep -o 'claims/given/[^"]*' "$LU_T/req2.json" | head -2 | tr '\n' ' ')"
fi

# --- done leaves nothing behind ------------------------------------------------------
lu_as alice "$LU_WA" done "$LU_A" --note "finished"; LU_RC=$?
LU_LEFT=$(find "$LU_ROOT/.git/openwarrant/claims" -name "$LU_A*" | wc -l)
LU_DEAD=$(grep -l "\"$LU_A\"" "$LU_N"/*.json "$LU_N"/delivered/*.json 2>/dev/null | wc -l)
lu_as owner "$LU_ROOT" view ready
LU_DEAD2=$(grep -l "\"$LU_A\"" "$LU_N"/*.json 2>/dev/null | wc -l)
if [[ $LU_RC == 0 && $LU_LEFT == 0 ]]; then
    lu_ok "done leaves no manifest" "the lock and its manifest are gone ($LU_DEAD notice(s) to sweep, $LU_DEAD2 after a read)"
else
    lu_fail "done leaves no manifest" "exit $LU_RC, $LU_LEFT left: $(head -2 "$LU_T/err" | tr '\n' '|')"
fi

/usr/bin/rm -rf "$LU_T"
