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
RP_DONE=$(wmd a done "$RP_TESTED" 2>&1); RP_DONE_RC=$?
if grep -q 'done upstream' <<<"$RP_UNCOMMITTED" && ! grep -q 'held:' <<<"$RP_MOVED" && [[ $RP_DONE_RC -eq 0 ]]; then
    rp_ok "a pin bump releases the item" "only once committed; the Warrant then closes"
else
    rp_fail "a pin bump releases the item" "$RP_DONE_RC $RP_DONE | $RP_MOVED"
fi

