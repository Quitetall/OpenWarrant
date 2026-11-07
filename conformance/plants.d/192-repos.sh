# shellcheck shell=bash
# Work across repositories (t-3c16c W6): two real repositories, HPR the lower
# layer and a submodule of WMD, with an agent working each side. Each claim
# beside the input it refuses:
#   a clause reads through the pin, and the pin's bytes, not the checkout's;
#   a commit the checkout lacks reads UNKNOWN, and local reads go on;
#   an open dependency holds its item out of ready, and says why in one line;
#   done on origin reads `done upstream, not pinned here` once fetched;
#   a pin bump releases it, and only a bump does;
#   a Warrant cannot close over what is not pinned, and a Ticket warns.

echo "== repos: work across repositories =="
RP_TMP=$(mktemp -d)
[[ -d "${RP_TMP:-}" ]] || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$RP_TMP") # The battery's own cleanup removes it.
rp_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rp_git() { git -c user.email=plant@invalid -c user.name=plant -c protocol.file.allow=always "$@"; }
hpr() { OPENWARRANT_ACTOR=hpr-agent "$WAR" --root "$RP_TMP/hpr" "$@"; }
wmd() { local who=$1; shift; OPENWARRANT_ACTOR=$who "$WAR" --root "$RP_TMP/wmd" "$@"; }
rp_id() { sed -n 's/.*"id": "\(t-[0-9a-f]*\)".*/\1/p' | head -1; }
# A ticket's state as `view tickets` reads it. `war done` hands an Order to a
# person and exits 0, so "not closed" is read here, not from an exit code.
rp_state() { local side=$1 id=$2; "$side" --json view tickets 2>/dev/null | python3 -c 'import json,sys
print(next((t["state"] for t in json.load(sys.stdin)["result"]["tickets"] if t["id"]==sys.argv[1]), "missing"))' "$id"; }

rp_git init -q --bare -b main "$RP_TMP/hpr.git" \
    || { printf 'PLANT SETUP FAILED: bare\n' >&2; exit 9; }
rp_git clone -q "$RP_TMP/hpr.git" "$RP_TMP/hpr" 2>/dev/null \
    || { printf 'PLANT SETUP FAILED: clone\n' >&2; exit 9; }
rp_git -C "$RP_TMP/hpr" checkout -q -b main 2>/dev/null
hpr init --namespace HP >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init hpr\n' >&2; exit 9; }
hpr plan types add interfaces >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: types add\n' >&2; exit 9; }
mkdir -p "$RP_TMP/hpr/docs/records/interfaces"
cat > "$RP_TMP/hpr/docs/records/interfaces/physics.md" <<'DOC'
---
format: openwarrant/1.0
profile: interface
version: 1.0.0
owner: hpr
parties: hpr, wmd
summary: "What the engine relies on"
---
# Physics interface

## CL-step · clause — A step advances the world by exactly dt
status: agreed
verify: test
check: true

`World::step(dt)` advances simulated time by `dt`.

## CL-mass · clause — Mass is in kilograms
status: agreed
verify: inspection

Mass is in kilograms.

## CL-handle · clause — Handles are stable across a step
status: tbd
verify: test
owner: hpr
due: 2026-11-20

Whether a removed handle may be reused is not decided.
DOC
RP_FEATURE=$(hpr --json create "The physics feature" --body "Steps." 2>/dev/null | rp_id)
[[ -n "$RP_FEATURE" ]] || { printf 'PLANT SETUP FAILED: no HPR ticket\n' >&2; exit 9; }
rp_git -C "$RP_TMP/hpr" add -A && rp_git -C "$RP_TMP/hpr" commit -qm start \
    && rp_git -C "$RP_TMP/hpr" push -q origin main \
    || { printf 'PLANT SETUP FAILED: hpr push\n' >&2; exit 9; }

mkdir -p "$RP_TMP/wmd"
rp_git -C "$RP_TMP/wmd" init -q -b main
wmd owner init --namespace WM >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init wmd\n' >&2; exit 9; }
rp_git -C "$RP_TMP/wmd" submodule add -q "$RP_TMP/hpr.git" vendor/hpr \
    || { printf 'PLANT SETUP FAILED: submodule\n' >&2; exit 9; }
printf '\n[repos.hpr]\npath = "vendor/hpr"\nurl = "https://example.invalid/hpr.git"\n' >> "$RP_TMP/wmd/openwarrant.toml"
rp_git -C "$RP_TMP/wmd" add -A && rp_git -C "$RP_TMP/wmd" commit -qm start \
    || { printf 'PLANT SETUP FAILED: wmd commit\n' >&2; exit 9; }

# ---- reads through the pin --------------------------------------------------

RP_PIN=$(git -C "$RP_TMP/wmd/vendor/hpr" rev-parse HEAD)
sed -i 's/advances simulated time/UNCOMMITTED WORDS/' "$RP_TMP/wmd/vendor/hpr/docs/records/interfaces/physics.md"
RP_STATUS=$(wmd a status 2>&1); RP_STATUS_RC=$?
RP_READ=$(wmd a context hpr:CL-step 2>&1); RP_READ_RC=$?
RP_PRIME=$(wmd a view prime "hpr:$RP_FEATURE" 2>&1); RP_PRIME_RC=$?
git -C "$RP_TMP/wmd/vendor/hpr" checkout -q -- docs
if [[ $RP_READ_RC -eq 0 && $RP_PRIME_RC -eq 0 && $RP_STATUS_RC -eq 0 ]] \
    && grep -q "Repos: hpr @ ${RP_PIN:0:12} (submodule)" <<<"$RP_STATUS" \
    && grep -q 'advances simulated time' <<<"$RP_READ" \
    && ! grep -q 'UNCOMMITTED' <<<"$RP_READ" && ! grep -q 'not decided' <<<"$RP_READ" \
    && grep -q 'The physics feature' <<<"$RP_PRIME"; then
    rp_ok "a read comes through the pin" "the clause alone, from the commit, not the checkout"
else
    rp_fail "a read comes through the pin" "status $RP_STATUS_RC read $RP_READ_RC prime $RP_PRIME_RC: $(tr "\n" "|" <<<"$RP_STATUS$RP_READ$RP_PRIME")"
fi

RP_CFG=$(cat "$RP_TMP/wmd/openwarrant.toml")
printf 'pin = "0123456789012345678901234567890123456789"\n' >> "$RP_TMP/wmd/openwarrant.toml"
RP_MISSING=$(wmd a context hpr:CL-step 2>&1); RP_MISSING_RC=$?
RP_LOCAL=$(wmd a view prime 2>&1); RP_LOCAL_RC=$?
printf '%s\n' "$RP_CFG" > "$RP_TMP/wmd/openwarrant.toml"
RP_NOPE=$(wmd a context nope:CL-step 2>&1); RP_NOPE_RC=$?
if [[ $RP_MISSING_RC -eq 2 && $RP_NOPE_RC -eq 2 && $RP_LOCAL_RC -eq 0 ]] \
    && grep -q 'UNKNOWN (repos.unknown)' <<<"$RP_MISSING" \
    && ! grep -q 'advances simulated' <<<"$RP_MISSING" \
    && grep -q 'UNKNOWN (repos.unknown)' <<<"$RP_NOPE"; then
    rp_ok "a missing commit reads UNKNOWN" "and a repository nothing declares; local reads go on"
else
    rp_fail "a missing commit reads UNKNOWN" "missing $RP_MISSING_RC nope $RP_NOPE_RC local $RP_LOCAL_RC: $RP_MISSING"
fi

# ---- done upstream, not pinned here -----------------------------------------

RP_ENGINE=$(wmd owner --json create "The engine feature" --depends-on "hpr:$RP_FEATURE" 2>/dev/null | rp_id)
RP_TESTED=$(wmd owner --json create "The tested engine feature" --depends-on "hpr:$RP_FEATURE" --test true 2>/dev/null | rp_id)
rp_git -C "$RP_TMP/wmd" add -A && rp_git -C "$RP_TMP/wmd" commit -qm wait
[[ -n "$RP_ENGINE" && -n "$RP_TESTED" ]] || { printf 'PLANT SETUP FAILED: no WMD tickets\n' >&2; exit 9; }
RP_READY=$(wmd a view ready 2>&1)
RP_HELD=$(wmd a view prime 2>&1)
if ! grep -q "$RP_ENGINE" <<<"$RP_READY" && grep -q "$RP_ENGINE.*held: hpr:$RP_FEATURE" <<<"$RP_HELD"; then
    rp_ok "an open dependency holds an item" "out of ready, one line in prime"
else
    rp_fail "an open dependency holds an item" "$RP_READY | $RP_HELD"
fi

# HPR's agent finishes and publishes. WMD has fetched nothing.
hpr start "$RP_FEATURE" >/dev/null 2>&1
hpr done "$RP_FEATURE" >/dev/null 2>&1
rp_git -C "$RP_TMP/hpr" add -A && rp_git -C "$RP_TMP/hpr" commit -qm done \
    && rp_git -C "$RP_TMP/hpr" push -q origin main
RP_BEFORE=$(wmd a view prime 2>&1)
rp_git -C "$RP_TMP/wmd/vendor/hpr" fetch -q origin
RP_AFTER=$(wmd a view prime 2>&1)
RP_READY=$(wmd a view ready 2>&1)
if ! grep -q 'done upstream' <<<"$RP_BEFORE" \
    && grep -q "$RP_ENGINE.*done upstream, not pinned here" <<<"$RP_AFTER" \
    && ! grep -q "$RP_ENGINE" <<<"$RP_READY"; then
    rp_ok "done upstream, not pinned here" "once fetched, in one line, and still out of ready"
else
    rp_fail "done upstream, not pinned here" "$RP_AFTER"
fi

wmd a start "$RP_TESTED" >/dev/null 2>&1
RP_REFUSED=$(wmd a done "$RP_TESTED" 2>&1); RP_REFUSED_RC=$?
wmd b start "$RP_ENGINE" >/dev/null 2>&1
RP_WARNED=$(wmd b done "$RP_ENGINE" 2>&1); RP_WARNED_RC=$?
if [[ $RP_REFUSED_RC -ne 0 && $RP_WARNED_RC -eq 0 ]] \
    && grep -q 'done.dependency' <<<"$RP_REFUSED" && grep -q 'nothing was written' <<<"$RP_REFUSED" \
    && grep -q 'warning (done.dependency)' <<<"$RP_WARNED"; then
    rp_ok "a Warrant cannot close unpinned" "refused, nothing written; a Ticket warns and closes"
else
    rp_fail "a Warrant cannot close unpinned" "$RP_REFUSED_RC $RP_REFUSED | $RP_WARNED_RC $RP_WARNED"
fi

rp_git -C "$RP_TMP/wmd" add -A && rp_git -C "$RP_TMP/wmd" commit -qm closed
rp_git -C "$RP_TMP/wmd/vendor/hpr" checkout -q origin/main
RP_UNCOMMITTED=$(wmd owner view prime 2>&1)
rp_git -C "$RP_TMP/wmd" add vendor/hpr && rp_git -C "$RP_TMP/wmd" commit -qm "move the pin"
RP_MOVED=$(wmd owner view prime 2>&1)
# The item it depends on changed since it started, so it is told first.
RP_UNSEEN=$(wmd a done "$RP_TESTED" 2>&1); RP_UNSEEN_RC=$?
wmd a start "$RP_TESTED" >/dev/null 2>&1
RP_DONE=$(wmd a done "$RP_TESTED" 2>&1); RP_DONE_RC=$?
if grep -q 'done upstream' <<<"$RP_UNCOMMITTED" && ! grep -q 'held:' <<<"$RP_MOVED" && [[ $RP_DONE_RC -eq 0 ]] \
    && [[ $RP_UNSEEN_RC -ne 0 ]] && grep -q 'done.unseen' <<<"$RP_UNSEEN"; then
    rp_ok "a pin bump releases the item" "only once committed; seen, the Warrant then closes"
else
    rp_fail "a pin bump releases the item" "$RP_UNSEEN_RC $RP_DONE_RC $RP_DONE | $RP_UNSEEN | $RP_MOVED"
fi


# ---- interfaces that act ----------------------------------------------------

RP_CHECK=$(wmd owner --verbose check 2>&1)
RP_CLAUSES=$(wmd owner --json check --clauses 2>&1); RP_CLAUSES_RC=$?
if grep -q '1 tbd, 0 tbr' <<<"$RP_CHECK" \
    && grep -Pzoq '"clause": "hpr:CL-step",\s+"source": "[^"]*",\s+"result": "PASS"' <<<"$RP_CLAUSES" \
    && grep -Pzoq '"clause": "hpr:CL-mass",\s+"source": "[^"]*",\s+"result": "UNKNOWN"' <<<"$RP_CLAUSES" \
    && [[ $RP_CLAUSES_RC -eq 2 ]]; then
    rp_ok "clauses counted, run, or UNKNOWN" "a clause with no check is never a pass"
else
    rp_fail "clauses counted, run, or UNKNOWN" "$RP_CLAUSES_RC: $(head -c 600 <<<"$RP_CLAUSES")"
fi

RP_TBD=$(wmd owner --json create "Needs the handle rule" --depends-on hpr:CL-handle 2>/dev/null | rp_id)
rp_git -C "$RP_TMP/wmd" add -A && rp_git -C "$RP_TMP/wmd" commit -qm tbd
RP_READY=$(wmd a view ready 2>&1)
RP_HELD=$(wmd a view prime 2>&1)
if ! grep -q "$RP_TBD" <<<"$RP_READY" && grep -q "$RP_TBD.*held: hpr:CL-handle: tbd (owner hpr, due 2026-11-20)" <<<"$RP_HELD"; then
    rp_ok "a tbd clause holds what names it" "out of ready, with its owner and date"
else
    rp_fail "a tbd clause holds what names it" "$RP_HELD"
fi

wmd owner check --clauses --record >/dev/null 2>&1
rp_git -C "$RP_TMP/wmd" add -A && rp_git -C "$RP_TMP/wmd" commit -qm conform
RP_LINE=$(wmd owner status 2>&1 | grep '^Interfaces:')

# ---- notices across the boundary, and a change request ----------------------

RP_STEP=$(wmd owner --json create "Uses a step" --depends-on hpr:CL-step --test true 2>/dev/null | rp_id)
RP_MASS=$(wmd owner --json create "Uses mass" --depends-on hpr:CL-mass 2>/dev/null | rp_id)
rp_git -C "$RP_TMP/wmd" add -A && rp_git -C "$RP_TMP/wmd" commit -qm more
wmd alice start "$RP_STEP" >/dev/null 2>&1
wmd bob start "$RP_MASS" >/dev/null 2>&1

RP_IMPACT=$(wmd owner plan impact hpr:CL-step 2>&1)
RP_BREAK=$(hpr --json create "Change what a step means" --change breaking --clause CL-step 2>/dev/null | rp_id)
RP_REFUSED=$(hpr start "$RP_BREAK" 2>&1; hpr done "$RP_BREAK" 2>&1); RP_REFUSED_RC=$?
RP_NO_CLASS=$(hpr create "Unclassed" --type change-request 2>&1); RP_NO_CLASS_RC=$?
if grep -q "here: $RP_STEP (alice) reaches hpr:CL-step" <<<"$RP_IMPACT" && ! grep -q "$RP_MASS" <<<"$RP_IMPACT" \
    && [[ $RP_NO_CLASS_RC -ne 0 ]] && [[ "$(rp_state hpr "$RP_BREAK")" != done ]] \
    && grep -q 'waits on a person' <<<"$RP_REFUSED" && grep -q 'war sign' <<<"$RP_REFUSED" \
    && grep -q 'change.class' <<<"$RP_NO_CLASS"; then
    rp_ok "a breaking change waits on people" "impact names the claim; done names what they sign"
else
    rp_fail "a breaking change waits on people" "$RP_REFUSED_RC $RP_NO_CLASS_RC $RP_IMPACT | $RP_REFUSED | $RP_NO_CLASS"
fi

sed -i 's/advances simulated time/advances time by one step/' "$RP_TMP/hpr/docs/records/interfaces/physics.md"
rp_git -C "$RP_TMP/hpr" add -A && rp_git -C "$RP_TMP/hpr" commit -qm "change a clause" \
    && rp_git -C "$RP_TMP/hpr" push -q origin main
RP_UNFETCHED=$(wmd alice note "$RP_STEP" working 2>&1)
rp_git -C "$RP_TMP/wmd/vendor/hpr" fetch -q origin
RP_BOB=$(wmd bob note "$RP_MASS" working 2>&1)
RP_ONCE=$(wmd alice note "$RP_STEP" again 2>&1)
RP_TWICE=$(wmd alice note "$RP_STEP" more 2>&1)
if ! grep -q '^notice: ' <<<"$RP_UNFETCHED" && ! grep -q '^notice: ' <<<"$RP_BOB" \
    && [[ $(grep -c '^notice: hpr:CL-step changed upstream' <<<"$RP_ONCE") -eq 1 ]] \
    && ! grep -q '^notice: ' <<<"$RP_TWICE"; then
    rp_ok "a changed clause reaches one side" "once fetched, once; who uses another clause hears nothing"
else
    rp_fail "a changed clause reaches one side" "$RP_UNFETCHED | $RP_BOB | $RP_ONCE | $RP_TWICE"
fi

RP_UNSEEN=$(wmd alice done "$RP_STEP" 2>&1); RP_UNSEEN_RC=$?
wmd alice start "$RP_STEP" >/dev/null 2>&1
RP_BEHIND=$(wmd alice done "$RP_STEP" 2>&1); RP_BEHIND_RC=$?
rp_git -C "$RP_TMP/wmd/vendor/hpr" checkout -q origin/main
rp_git -C "$RP_TMP/wmd" add vendor/hpr && rp_git -C "$RP_TMP/wmd" commit -qm "move the pin again"
RP_QUIET=$(wmd bob note "$RP_MASS" quiet 2>&1)
RP_DRIFT=$(wmd owner status 2>&1 | grep '^Interfaces:')
RP_CLOSED=$(wmd alice done "$RP_STEP" 2>&1); RP_CLOSED_RC=$?
if [[ $RP_UNSEEN_RC -ne 0 && $RP_BEHIND_RC -ne 0 && $RP_CLOSED_RC -eq 0 ]] \
    && grep -q 'done.unseen' <<<"$RP_UNSEEN" && grep -q 'done.behind' <<<"$RP_BEHIND" \
    && ! grep -q '^notice: ' <<<"$RP_QUIET"; then
    rp_ok "a Warrant holds across the boundary" "unseen, then behind; the pin bump closes it"
else
    rp_fail "a Warrant holds across the boundary" "$RP_UNSEEN | $RP_BEHIND | $RP_CLOSED"
fi

if grep -q 'Interfaces: 1 conform' <<<"$RP_LINE" && grep -q 'Interfaces: drift: hpr:' <<<"$RP_DRIFT"; then
    rp_ok "drift is one line" "conforming, then not, once the pin holds a new revision"
else
    rp_fail "drift is one line" "$RP_LINE | $RP_DRIFT"
fi
