# shellcheck shell=bash
# t-3c16c W3, step 6 — `war done` holds a change and a widening to the item's level.
#
# One scratch program on `main`, with a throwaway key for what only a person does.
# Each behaviour is paired with its refusal.
#
# Ticket: an unseen change warns and closes; a widening changes nothing.
# Warrant: refused until seen; refused while its workspace lacks the change; then
# the trigger runs on the new state and closes, even a trigger `war done` alone would
# not have run; a trigger failing on the new state does not close; a widening is
# flagged by name and journalled, and a resumed start does not start it away.
# Order: an unseen change is refused as on a Warrant; an unchanged Order is not
# refused; a widening is refused by name, writes nothing, emits what a person would
# sign, and an agent cannot sign it; a human's new revision lets it close; a change
# to what was signed needs another, and one to an unsigned Order does not.
# Unknown: a corrupt notice, a corrupt manifest and the switch answer UNKNOWN on a
# Warrant and warn on a Ticket; an unreadable level answers UNKNOWN. Unaffected: an
# item nothing reached closes with nothing said. Every refusal is exit 2 and leaves
# the tree and the journal as they were.

echo "== war done, by level (t-3c16c W3 step 6) =="
DE_ROOT=$(scratch_corpus DE)
[[ -d "${DE_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
DE_WAR="$REPO_ROOT/${WAR#./}"
DE_T=$(mktemp -d)
de_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
de_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
de_g() { git -C "$DE_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
# de_as <actor> <checkout> <war args...>: stdout to $DE_T/out, stderr to $DE_T/err.
de_as() {
    local actor="$1" at="$2"; shift 2
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR="$actor" OPENWARRANT_SESSION_ID=plant-189 \
        "$DE_WAR" --root "$at" "$@" >"$DE_T/out" 2>"$DE_T/err" </dev/null
}
de_field() { python3 -c 'import json,sys; v=json.load(open(sys.argv[1]))
for k in sys.argv[2].split("."): v=v[int(k)] if k.isdigit() else v[k]
print(v)' "$DE_T/out" "$1"; }
de_create() { de_as owner "$DE_ROOT" --json create "$@" && de_field result.id; }
de_commit() { de_g add -A; de_g commit -qm "$1"; }
# Start on a worktree of its own; the worktree's path.
de_start() { de_as "$1" "$DE_ROOT" --json start --worktree --print "$2" && de_field result.worktree; }
de_merge() { git -C "$1" -c user.email=p@i -c user.name=p merge -q --no-edit main >/dev/null 2>&1; }
de_commit_wt() { git -C "$1" -c user.email=p@i -c user.name=p commit -qam "$2"; }
de_state() { (cd "$DE_ROOT" && git status --porcelain; cat "$1"/docs/tickets/*.md "$1"/docs/tickets/*/journal.jsonl 2>/dev/null) | sha256sum; }
de_says() { grep -qF -- "$1" "$DE_T/err"; }
de_open() { grep -qE -- "^- \[ \] " "$1"; }

de_g branch -M main
mkdir -p "$DE_ROOT/docs" "$DE_ROOT/src"
printf '# Plan\n\nOLD\n' >"$DE_ROOT/docs/plan.md"
printf 'plain\n' >"$DE_ROOT/src/a.txt"
ssh-keygen -q -t ed25519 -N "" -C plant-ada -f "$DE_T/id_ada" </dev/null \
    || { printf 'PLANT SETUP FAILED: ssh-keygen\n' >&2; exit 9; }
DE_PUB=$(cut -d' ' -f1,2 "$DE_T/id_ada.pub")
printf 'ada namespaces="oh.war/response,oh.war/dsse" %s\n' "$DE_PUB" >"$DE_ROOT/docs/authority/allowed_signers"
cat >"$DE_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Ada"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/189-done-enforce.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "ada"
ROLES
printf '\n[roles.roster]\nada = "maintain"\n' >>"$DE_ROOT/openwarrant.toml"

# A person's act, in an agent holding one throwaway key and nothing else.
de_human() {
    local at="$1"; shift
    (
        eval "$(ssh-agent -s)" >/dev/null
        trap 'ssh-agent -k >/dev/null 2>&1' EXIT
        ssh-add -q "$DE_T/id_ada" 2>/dev/null
        [[ "$(ssh-add -L 2>/dev/null)" == "$DE_PUB"* && $(ssh-add -L | wc -l) == 1 ]] \
            || { echo 'PLANT: the agent holds the wrong keys' >&2; exit 97; }
        OPENWARRANT_ACTOR=Ada "$DE_WAR" --root "$at" "$@" </dev/null
    ) >"$DE_T/human.out" 2>"$DE_T/human.err"
}

DE_TICKET=$(de_create "A ticket from the plan" --body "Follow docs/plan.md.")
DE_ONE=$(de_create "A Warrant from the plan" --body "Follow docs/plan.md." --test 'grep -q NEW docs/plan.md')
DE_FAILS=$(de_create "A Warrant the new plan breaks" --body "Follow docs/plan.md." --test 'grep -q OLD docs/plan.md')
DE_LIGHT=$(de_create --legacy "A light Warrant" --body "Follow docs/plan.md.")
de_as owner "$DE_ROOT" plan add "$DE_LIGHT" --test 'grep -q OLD docs/plan.md'
DE_ALONE=$(de_create "Nothing reaches this ticket")
DE_ALONE_W=$(de_create "Nothing reaches this Warrant" --test true)
DE_ORDER_CHANGED=$(de_create "An Order from the plan" --body "Follow docs/plan.md." -i "the release")
de_as owner "$DE_ROOT" --json view show "$DE_ORDER_CHANGED"
DE_OC_I=$(de_field result.items.0.id)
de_as owner "$DE_ROOT" plan add "$DE_ORDER_CHANGED/$DE_OC_I" --min signed
de_commit 'the items and the plan'

DE_WT=$(de_start tess "$DE_TICKET")
DE_W1=$(de_start wes "$DE_ONE")
DE_W2=$(de_start fay "$DE_FAILS")
DE_W3=$(de_start lee "$DE_LIGHT")
DE_W4=$(de_start ned "$DE_ALONE")
DE_W5=$(de_start nia "$DE_ALONE_W")
DE_W6=$(de_start olga "$DE_ORDER_CHANGED")
for d in "$DE_WT" "$DE_W1" "$DE_W2" "$DE_W3" "$DE_W4" "$DE_W5" "$DE_W6"; do
    [[ -d "${d:-}" ]] || { printf 'PLANT SETUP FAILED: no worktree: %s\n' "$(head -c 300 "$DE_T/err")" >&2; exit 9; }
done

# ---- the change, on main ------------------------------------------------------------
sed -i 's/^OLD$/NEW/' "$DE_ROOT/docs/plan.md"
de_commit 'amend the plan'

# ---- unaffected: nothing heard, nothing said ----------------------------------------
de_as ned "$DE_W4" done "$DE_ALONE" --note done; DE_RC=$?
DE_ERR=$(cat "$DE_T/err")
de_as nia "$DE_W5" done "$DE_ALONE_W" --note done; DE_RC2=$?
DE_ERR2=$(cat "$DE_T/err")
if [[ $DE_RC == 0 && -z $DE_ERR && $DE_RC2 == 0 && -z $DE_ERR2 ]]; then
    de_ok "an unaffected item closes" "a Ticket and a Warrant, exit 0 and nothing on stderr"
else
    de_fail "an unaffected item closes" "$DE_RC [$DE_ERR] $DE_RC2 [$DE_ERR2]"
fi

# ---- Ticket: warns and closes --------------------------------------------------------
de_as tess "$DE_WT" done "$DE_TICKET" --note done; DE_RC=$?
if [[ $DE_RC == 0 ]] && de_says 'warning (done.unseen):' && de_says 'docs/plan.md changed on main' \
    && grep -q "^done $DE_TICKET" "$DE_T/out"; then
    de_ok "Ticket: unseen warns and closes" "$(grep -F done.unseen "$DE_T/err")"
else
    de_fail "Ticket: unseen warns and closes" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi

# ---- Warrant: refused until seen ------------------------------------------------------
DE_BEFORE=$(de_state "$DE_W1")
de_as wes "$DE_W1" done "$DE_ONE" --note done; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'refused (done.unseen):' && de_says "war start $DE_ONE" \
    && [[ $(de_state "$DE_W1") == "$DE_BEFORE" ]] && ! grep -q '^done ' "$DE_T/out"; then
    de_ok "Warrant: refused until seen" "$(grep -F 'refused (done.unseen)' "$DE_T/err")"
else
    de_fail "Warrant: refused until seen" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi
de_as wes "$DE_W1" start "$DE_ONE"
if grep -q '^+NEW' "$DE_T/out"; then de_ok "start records it seen" "prints the hunk"; else de_fail "start records it seen" "$(head -3 "$DE_T/out" | tr '\n' '|')"; fi

# ---- seen, and not in the workspace: its trigger would say nothing ---------------------
DE_BEFORE=$(de_state "$DE_W1")
de_as wes "$DE_W1" done "$DE_ONE" --note done; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'refused (done.behind):' && [[ $(de_state "$DE_W1") == "$DE_BEFORE" ]]; then
    de_ok "Warrant: seen but not held" "$(grep -F 'done.behind' "$DE_T/err")"
else
    de_fail "Warrant: seen but not held" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi

# ---- then its trigger runs on the new state --------------------------------------------
de_merge "$DE_W1"
de_as wes "$DE_W1" done "$DE_ONE" --note done; DE_RC=$?
if [[ $DE_RC == 0 ]] && grep -q 'observed' "$DE_T/out"; then
    de_ok "Warrant: then its trigger runs" "$(grep observed "$DE_T/out" | head -1)"
else
    de_fail "Warrant: then its trigger runs" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi

# ---- a trigger that fails on the new state does not close ---------------------------------
de_as fay "$DE_W2" start "$DE_FAILS"
de_merge "$DE_W2"
de_as fay "$DE_W2" done "$DE_FAILS" --note done; DE_RC=$?
if [[ $DE_RC == 2 ]] && ! grep -q '^done ' "$DE_T/out" && de_says 'not ticked'; then
    de_ok "a failing trigger does not close" "$(head -1 "$DE_T/err" | cut -c1-110)"
else
    de_fail "a failing trigger does not close" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi

# ---- a trigger plain `war done` would not run still runs ------------------------------------
de_as lee "$DE_W3" start "$DE_LIGHT"
de_merge "$DE_W3"
de_as lee "$DE_W3" done "$DE_LIGHT" --note done; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'ticket.check-failed'; then
    de_ok "seen forces the trigger" "a light Warrant's test ran with no --check, and failed"
else
    de_fail "seen forces the trigger" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi

# ---- Order: the same ---------------------------------------------------------------------
de_as olga "$DE_W6" done "$DE_ORDER_CHANGED"; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'refused (done.unseen):'; then
    de_ok "Order: refused until seen" "$(grep -F 'done.unseen' "$DE_T/err" | cut -c1-110)"
else
    de_fail "Order: refused until seen" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi

# ---- a widening: a Ticket hears nothing, a Warrant is flagged ------------------------------------
DE_TK=$(de_create "A scoped ticket" --path 'src/**' --tool Read)
DE_WD=$(de_create "A scoped Warrant" --path 'src/**' --tool Read --command 'cargo test' --test true)
DE_SAME=$(de_create "A scoped Warrant left alone" --path 'src/**' --tool Read --test true)
DE_NARROW=$(de_create "A narrowed Warrant" --path 'src/**' --tool Read --tool Grep --test true)
de_commit 'scoped items'
de_as sam "$DE_ROOT" start "$DE_TK"
sed -i 's/^  - "\{0,1\}Read"\{0,1\}$/  - Read\n  - Bash/' "$DE_ROOT/docs/tickets/$DE_TK.md"
de_as sam "$DE_ROOT" done "$DE_TK" --note done; DE_RC=$?
if [[ $DE_RC == 0 ]] && ! grep -q 'done\.' "$DE_T/err"; then
    de_ok "Ticket: a widening changes nothing" "exit 0, nothing said"
else
    de_fail "Ticket: a widening changes nothing" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi
de_as sam "$DE_ROOT" start "$DE_WD"
sed -i 's/^  - "\{0,1\}Read"\{0,1\}$/  - Read\n  - Bash/;s/^  - "\{0,1\}src\/\*\*"\{0,1\}$/  - "src\/**"\n  - "docs\/**"/' "$DE_ROOT/docs/tickets/$DE_WD.md"
de_as sam "$DE_ROOT" start "$DE_WD"
de_as sam "$DE_ROOT" done "$DE_WD" --note done; DE_RC=$?
DE_J=$(grep -h '"work.scope_widened"' "$DE_ROOT"/docs/tickets/*"$DE_WD"*/journal.jsonl "$DE_ROOT"/docs/tickets/.journal/"$DE_WD"* 2>/dev/null | head -1)
if [[ -z $DE_J ]]; then DE_J=$(grep -rhs 'work.scope_widened' "$DE_ROOT/docs" | grep -F "$DE_WD" | head -1); fi
if [[ $DE_RC == 0 ]] && de_says 'warning (done.widened):' && de_says 'x.tools + Bash' && de_says 'paths + docs/**' \
    && [[ $DE_J == *'x.tools + Bash'* ]]; then
    de_ok "Warrant: a widening is flagged" "$(grep -F 'done.widened' "$DE_T/err")"
else
    de_fail "Warrant: a widening is flagged" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|') journal[$DE_J]"
fi
de_as sam "$DE_ROOT" start "$DE_SAME"
de_as sam "$DE_ROOT" done "$DE_SAME" --note done; DE_RC=$?
if [[ $DE_RC == 0 ]] && ! grep -q 'done\.' "$DE_T/err"; then
    de_ok "an unchanged Warrant is not flagged" "exit 0, nothing said"
else
    de_fail "an unchanged Warrant is not flagged" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi
de_as sam "$DE_ROOT" start "$DE_NARROW"
sed -i '/^  - "\{0,1\}Grep"\{0,1\}$/d' "$DE_ROOT/docs/tickets/$DE_NARROW.md"
de_as sam "$DE_ROOT" done "$DE_NARROW" --note done; DE_RC=$?
if [[ $DE_RC == 0 ]] && ! grep -q 'done\.' "$DE_T/err"; then
    de_ok "a narrowing is not a widening" "exit 0, nothing said"
else
    de_fail "a narrowing is not a widening" "exit $DE_RC: $(head -3 "$DE_T/err" | tr '\n' '|')"
fi
de_commit 'tick'

# ---- Order: an unchanged one is not refused, a widening is -------------------------------------
de_order() { # title, as many items as asked; the first is the one a person signs
    local title="$1"; shift
    local id item args=()
    for item in "$@"; do args+=(-i "$item"); done
    id=$(de_create "$title" --path 'src/**' --tool Read "${args[@]}") || return 1
    de_as owner "$DE_ROOT" --json view show "$id"
    de_as owner "$DE_ROOT" plan add "$id/$(de_field result.items.0.id)" --min signed
    printf '%s' "$id"
}
# An item by its words: a milestone moves the item it holds.
de_item() {
    de_as owner "$DE_ROOT" --json view show "$1"
    python3 -c 'import json,sys
print(next(i["id"] for i in json.load(open(sys.argv[1]))["result"]["items"] if i["text"] == sys.argv[2]))' "$DE_T/out" "$2"
}
DE_O1=$(de_order "An unchanged Order" "the release" "the notes")
DE_O1A=$(de_item "$DE_O1" "the release"); DE_O1B=$(de_item "$DE_O1" "the notes")
DE_O2=$(de_order "A widened Order" "the release" "the notes")
DE_O2B=$(de_item "$DE_O2" "the notes")
DE_O3=$(de_order "An amended Order" "the release" "the notes" "the rest")
DE_O3B=$(de_item "$DE_O3" "the notes"); DE_O3C=$(de_item "$DE_O3" "the rest")
DE_O4=$(de_order "An Order nobody signed yet" "the release" "the notes")
DE_O4B=$(de_item "$DE_O4" "the notes")
de_commit 'orders'

de_as oren "$DE_ROOT" start "$DE_O1"
de_as oren "$DE_ROOT" done "$DE_O1/$DE_O1B" --note done; DE_RC=$?
DE_ERR=$(cat "$DE_T/err")
de_as oren "$DE_ROOT" done "$DE_O1/$DE_O1A" --note done; DE_RC2=$?
if [[ $DE_RC == 0 && $DE_ERR != *done.* && $DE_RC2 == 2 ]] && de_says 'ticket.tick-below-minimum' && ! de_says 'done.order'; then
    de_ok "an unchanged Order is not refused" "its free item closes; its signed item still takes a person"
else
    de_fail "an unchanged Order is not refused" "$DE_RC [$DE_ERR] $DE_RC2: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi

de_as oren "$DE_ROOT" start "$DE_O2"
sed -i 's/^  - "\{0,1\}Read"\{0,1\}$/  - Read\n  - Bash/' "$DE_ROOT/docs/tickets/$DE_O2.md"
de_as oren "$DE_ROOT" start "$DE_O2"
DE_BEFORE=$(de_state "$DE_ROOT")
de_as oren "$DE_ROOT" --json done "$DE_O2/$DE_O2B" --note done; DE_RC=$?
DE_CMD=$(de_field result.command 2>/dev/null); DE_DIGEST=$(de_field result.digest 2>/dev/null)
DE_RULE=$(de_field diagnostics.0.rule 2>/dev/null)
de_as oren "$DE_ROOT" done "$DE_O2/$DE_O2B" --note done; DE_RC2=$?
if [[ $DE_RC == 2 && $DE_RC2 == 2 && $DE_RULE == done.order-widened ]] && de_says 'x.tools + Bash' \
    && [[ $DE_CMD == "war sign approve $DE_O2 --ssh-sign" && $DE_DIGEST == sha256:* ]] \
    && [[ $(de_state "$DE_ROOT") == "$DE_BEFORE" ]]; then
    de_ok "Order: a widening is refused" "$(grep -F 'done.order-widened' "$DE_T/err")"
    de_ok "the refusal emits what is signed" "$DE_CMD over $DE_DIGEST"
else
    de_fail "Order: a widening is refused" "$DE_RC/$DE_RC2 $DE_RULE [$DE_CMD] $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_as oren "$DE_ROOT" sign approve "$DE_O2" --ssh-sign --as Ada; DE_RC=$?
if [[ $DE_RC != 0 ]] && [[ ! -e "$DE_ROOT/docs/authority/responses/$DE_O2.approve.response.toml.sig" ]]; then
    de_ok "an agent cannot sign it" "with no key in reach: exit $DE_RC, $(head -1 "$DE_T/err" | cut -c1-80)"
else
    de_fail "an agent cannot sign it" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_human "$DE_ROOT" sign approve "$DE_O2" --ssh-sign --as Ada; DE_RC=$?
if [[ $DE_RC != 0 ]]; then de_fail "a human signs a revision" "exit $DE_RC: $(head -3 "$DE_T/human.err" | tr '\n' '|')"; fi
de_as oren "$DE_ROOT" done "$DE_O2/$DE_O2B" --note done; DE_RC=$?
if [[ $DE_RC == 0 ]] && ! de_says 'done.'; then
    de_ok "a new signed revision lets it close" "the same widening, signed: exit 0"
else
    de_fail "a new signed revision lets it close" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi

# ---- Order: a change to what was signed takes a new signature; to what was not, nothing ----------
de_human "$DE_ROOT" sign approve "$DE_O3" --ssh-sign --as Ada; DE_RC=$?
[[ $DE_RC == 0 ]] || de_fail "a human signs the plan" "exit $DE_RC: $(head -3 "$DE_T/human.err" | tr '\n' '|')"
de_as oren "$DE_ROOT" start "$DE_O3"
de_as oren "$DE_ROOT" done "$DE_O3/$DE_O3B"; DE_RC=$?
sed -i 's/the rest/the rest, amended/' "$DE_ROOT/docs/tickets/$DE_O3.md"
DE_BEFORE=$(de_state "$DE_ROOT")
de_as oren "$DE_ROOT" done "$DE_O3/$DE_O3C"; DE_RC2=$?
if [[ $DE_RC == 0 && $DE_RC2 == 2 ]] && de_says 'refused (done.order-amended):' && [[ $(de_state "$DE_ROOT") == "$DE_BEFORE" ]]; then
    de_ok "a signed Order amended is refused" "$(grep -F 'done.order-amended' "$DE_T/err" | cut -c1-120)"
else
    de_fail "a signed Order amended is refused" "$DE_RC/$DE_RC2: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_human "$DE_ROOT" sign approve "$DE_O3" --ssh-sign --as Ada; DE_RC=$?
de_as oren "$DE_ROOT" done "$DE_O3/$DE_O3C"; DE_RC2=$?
if [[ $DE_RC == 0 && $DE_RC2 == 0 ]]; then
    de_ok "its new revision closes it" "signed again: exit 0"
else
    de_fail "its new revision closes it" "$DE_RC/$DE_RC2: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_as oren "$DE_ROOT" start "$DE_O4"
sed -i 's/the notes/the notes, amended/' "$DE_ROOT/docs/tickets/$DE_O4.md"
de_as oren "$DE_ROOT" done "$DE_O4/$DE_O4B"; DE_RC=$?
if [[ $DE_RC == 0 ]] && ! de_says 'done.'; then
    de_ok "an unsigned Order may be amended" "nothing signed yet: exit 0"
else
    de_fail "an unsigned Order may be amended" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_commit 'orders ticked'

# ---- unknown stays unknown --------------------------------------------------------------------
DE_UW=$(de_create "A Warrant whose standing goes unread" --test true)
DE_UK=$(de_create "A ticket whose standing goes unread")
DE_UM=$(de_create "A Warrant whose manifest goes unread" --test true)
DE_US=$(de_create "A Warrant done with the switch on" --test true)
DE_UL=$(de_create --legacy "A light Warrant of unknown level" -i "one")
de_commit 'for the unknowns'
mkdir -p "$DE_ROOT/docs/tickets/$DE_UL/atoms/30-checks.md"
for id in "$DE_UW" "$DE_UK" "$DE_UM" "$DE_US"; do de_as uma "$DE_ROOT" start "$id"; done
DE_N="$DE_ROOT/.git/openwarrant/notices"
mkdir -p "$DE_N"
printf '{ not json' >"$DE_N/garbage.json"
de_as uma "$DE_ROOT" done "$DE_UW"; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'UNKNOWN (done.seen-unknown):' && ! grep -q '^done ' "$DE_T/out"; then
    de_ok "Warrant: unread is UNKNOWN" "$(grep -F 'done.seen-unknown' "$DE_T/err" | cut -c1-120)"
else
    de_fail "Warrant: unread is UNKNOWN" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_as uma "$DE_ROOT" done "$DE_UK"; DE_RC=$?
if [[ $DE_RC == 0 ]] && de_says 'warning (done.seen-unknown):' && grep -q "^done $DE_UK" "$DE_T/out"; then
    de_ok "Ticket: unread warns and closes" "$(grep -F 'done.seen-unknown' "$DE_T/err" | cut -c1-120)"
else
    de_fail "Ticket: unread warns and closes" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
/usr/bin/rm -f "$DE_N/garbage.json"
de_as uma "$DE_ROOT" done "$DE_UW"; DE_RC=$?
if [[ $DE_RC == 0 ]]; then de_ok "put right, it closes" "the notice gone: exit 0"; else de_fail "put right, it closes" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"; fi
printf '{ not json' >"$DE_ROOT/.git/openwarrant/claims/given/$DE_UM.json"
de_as uma "$DE_ROOT" done "$DE_UM"; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'UNKNOWN (done.seen-unknown):'; then
    de_ok "a corrupt manifest is UNKNOWN" "not read as nothing recorded"
else
    de_fail "a corrupt manifest is UNKNOWN" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
OPENWARRANT_NO_NOTICES=1 de_as uma "$DE_ROOT" done "$DE_US"; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'UNKNOWN (done.seen-unknown):'; then
    de_ok "the switch is no way past" "on a started Warrant: UNKNOWN"
else
    de_fail "the switch is no way past" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi
de_as uma "$DE_ROOT" admin claim "$DE_UL"
de_as uma "$DE_ROOT" done "$DE_UL"; DE_RC=$?
if [[ $DE_RC == 2 ]] && de_says 'UNKNOWN (done.level-unknown):'; then
    de_ok "an unread level is UNKNOWN" "$(grep -F 'done.level-unknown' "$DE_T/err" | cut -c1-120)"
else
    de_fail "an unread level is UNKNOWN" "exit $DE_RC: $(head -2 "$DE_T/err" | tr '\n' '|')"
fi

/usr/bin/rm -rf "$DE_T"
