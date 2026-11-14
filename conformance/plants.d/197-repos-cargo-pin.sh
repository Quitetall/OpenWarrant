# shellcheck shell=bash
# Work across repositories when the pin is a Cargo git dependency (t-3c16c W6).
# Two throwaway repositories and a real `cargo generate-lockfile`, with HPR a
# sibling checkout and WMD's Cargo.lock the only thing that says which commit it
# has. Each claim beside the input it refuses:
#   the pin is the commit Cargo locked, and a url nothing locks reads UNKNOWN;
#   a read comes through that commit;
#   an open item holds what depends on it; done upstream says so, once HPR has it;
#   a lock moved but not committed releases nothing, and a committed one does;
#   `--type change-request` lists change requests, and a typo is still refused;
#   a clause with no `CL-` prefix is watched, a word that is no clause is not;
#   a dependency on an item hears when it changes, once, and its neighbour does not.

echo "== repos: a Cargo.lock pin, end to end =="
CP_TMP=$(mktemp -d)
[[ -d "${CP_TMP:-}" ]] || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$CP_TMP") # The battery's own cleanup removes it.
cp_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
cp_git() { git -c user.email=plant@invalid -c user.name=plant -c protocol.file.allow=always "$@"; }
# The clock of this repository's toolchain, and a Cargo home of its own: nothing
# here reaches ~/.cargo or the network, the dependency is a file:// url.
cp_cargo() { (cd "$REPO_ROOT" && CARGO_HOME="$CP_TMP/cargo" timeout 60 cargo "$@"); }
hpr() { OPENWARRANT_ACTOR=hpr-agent "$WAR" --root "$CP_TMP/hpr" "$@"; }
wmd() { local who=$1; shift; OPENWARRANT_ACTOR=$who "$WAR" --root "$CP_TMP/wmd" "$@"; }
cp_id() { sed -n 's/.*"id": "\([tw]-[0-9a-f]*\)".*/\1/p' | head -1; }
CP_URL="file://$CP_TMP/hpr.git"

cp_git init -q --bare -b main "$CP_TMP/hpr.git" || { printf 'PLANT SETUP FAILED: bare\n' >&2; exit 9; }
cp_git clone -q "$CP_TMP/hpr.git" "$CP_TMP/hpr" 2>/dev/null || { printf 'PLANT SETUP FAILED: clone\n' >&2; exit 9; }
cp_git -C "$CP_TMP/hpr" checkout -q -b main 2>/dev/null
hpr init --namespace HP >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init hpr\n' >&2; exit 9; }
hpr plan types add interfaces >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: types add\n' >&2; exit 9; }
mkdir -p "$CP_TMP/hpr/src" "$CP_TMP/hpr/docs/records/interfaces"
printf '[package]\nname = "hpr"\nversion = "0.1.0"\nedition = "2021"\n' > "$CP_TMP/hpr/Cargo.toml"
: > "$CP_TMP/hpr/src/lib.rs"
cat > "$CP_TMP/hpr/docs/records/interfaces/physics.md" <<'DOC'
---
format: openwarrant/1.0
profile: interface
version: 1.0.0
owner: hpr
parties: hpr, wmd
summary: "What the engine relies on"
---
# Physics interface

## PHYS-step · clause — A step advances the world by exactly dt
status: agreed
verify: inspection

A step advances simulated time by `dt`.

## PHYS-mass · clause — Mass is in kilograms
status: agreed
verify: inspection

Mass is in kilograms.
DOC
CP_FEATURE=$(hpr --json create "The physics feature" --body "Steps." 2>/dev/null | cp_id)
CP_OTHER=$(hpr --json create "Something else" --body "Beside it." 2>/dev/null | cp_id)
[[ -n "$CP_FEATURE" && -n "$CP_OTHER" ]] || { printf 'PLANT SETUP FAILED: no HPR tickets\n' >&2; exit 9; }
cp_git -C "$CP_TMP/hpr" add -A && cp_git -C "$CP_TMP/hpr" commit -qm start \
    && cp_git -C "$CP_TMP/hpr" push -q origin main \
    || { printf 'PLANT SETUP FAILED: hpr push\n' >&2; exit 9; }

mkdir -p "$CP_TMP/wmd/src"
cp_git -C "$CP_TMP/wmd" init -q -b main
wmd owner init --namespace WM >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init wmd\n' >&2; exit 9; }
printf '[package]\nname = "wmd"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nhpr = { git = "%s" }\n' \
    "$CP_URL" > "$CP_TMP/wmd/Cargo.toml"
: > "$CP_TMP/wmd/src/lib.rs"
cp_cargo generate-lockfile -q --manifest-path "$CP_TMP/wmd/Cargo.toml" \
    || { printf 'PLANT SETUP FAILED: cargo generate-lockfile\n' >&2; exit 9; }
grep -q 'source = "git+' "$CP_TMP/wmd/Cargo.lock" || { printf 'PLANT SETUP FAILED: no git source in the lock\n' >&2; exit 9; }
printf '\n[repos.hpr]\npath = "../hpr"\nurl = "%s"\n' "$CP_URL" >> "$CP_TMP/wmd/openwarrant.toml"
cp_git -C "$CP_TMP/wmd" add -A && cp_git -C "$CP_TMP/wmd" commit -qm start \
    || { printf 'PLANT SETUP FAILED: wmd commit\n' >&2; exit 9; }

# ---- the pin is what Cargo locked --------------------------------------------
CP_PIN=$(git -C "$CP_TMP/hpr" rev-parse HEAD)
CP_STATUS=$(wmd a status 2>&1)
CP_READ=$(wmd a context hpr:PHYS-step 2>&1); CP_READ_RC=$?
if grep -q "Repos: hpr @ ${CP_PIN:0:12} (Cargo.lock)" <<<"$CP_STATUS" && [[ $CP_READ_RC -eq 0 ]] \
    && grep -q 'advances simulated time' <<<"$CP_READ" && ! grep -q 'kilograms' <<<"$CP_READ" \
    && grep -q "#$CP_PIN" "$CP_TMP/wmd/Cargo.lock"; then
    cp_ok "the pin is what Cargo locked" "the commit in Cargo.lock; the clause alone, read from it"
else
    cp_fail "the pin is what Cargo locked" "$CP_READ_RC: $(tr '\n' '|' <<<"$CP_STATUS$CP_READ" | head -c 300)"
fi

CP_CFG=$(cat "$CP_TMP/wmd/openwarrant.toml")
printf '\n[repos.other]\npath = "../hpr"\nurl = "https://example.invalid/other.git"\n' >> "$CP_TMP/wmd/openwarrant.toml"
CP_OTHERS=$(wmd a status 2>&1); CP_LOCAL=$(wmd a view prime 2>&1); CP_LOCAL_RC=$?
printf '%s\n' "$CP_CFG" > "$CP_TMP/wmd/openwarrant.toml"
if grep -q 'other UNKNOWN' <<<"$CP_OTHERS" && [[ $CP_LOCAL_RC -eq 0 ]]; then
    cp_ok "a url nothing locks reads UNKNOWN" "local reads go on"
else
    cp_fail "a url nothing locks reads UNKNOWN" "$CP_LOCAL_RC: $(grep -m1 Repos <<<"$CP_OTHERS")"
fi

# ---- done upstream, then pinned ----------------------------------------------
CP_ENGINE=$(wmd owner --json create "The engine feature" --depends-on "hpr:$CP_FEATURE" 2>/dev/null | cp_id)
CP_NEXT=$(wmd owner --json create "Beside the feature" --depends-on "hpr:$CP_OTHER" 2>/dev/null | cp_id)
CP_CITES=$(wmd owner --json create "Cites a clause" --body "Built on hpr:PHYS-mass, and hpr:no-such-clause is a mere word." 2>/dev/null | cp_id)
CP_STEPS=$(wmd owner --json create "Cites its neighbour" --body "Built on hpr:PHYS-step." 2>/dev/null | cp_id)
[[ -n "$CP_ENGINE" && -n "$CP_NEXT" && -n "$CP_CITES" && -n "$CP_STEPS" ]] || { printf 'PLANT SETUP FAILED: no WMD tickets\n' >&2; exit 9; }
cp_git -C "$CP_TMP/wmd" add -A && cp_git -C "$CP_TMP/wmd" commit -qm wait
CP_READY=$(wmd a view ready 2>&1); CP_HELD=$(wmd a view prime 2>&1)
if ! grep -q "$CP_ENGINE" <<<"$CP_READY" && grep -q "$CP_ENGINE.*held: hpr:$CP_FEATURE" <<<"$CP_HELD"; then
    cp_ok "an open item holds what needs it" "out of ready, one line in prime"
else
    cp_fail "an open item holds what needs it" "$CP_HELD"
fi
wmd carol start "$CP_ENGINE" >/dev/null 2>&1
wmd dave start "$CP_NEXT" >/dev/null 2>&1
wmd erin start "$CP_CITES" >/dev/null 2>&1
wmd frank start "$CP_STEPS" >/dev/null 2>&1

hpr start "$CP_FEATURE" >/dev/null 2>&1
hpr done "$CP_FEATURE" >/dev/null 2>&1
sed -i 's/^Mass is in kilograms\./Mass is in grams./' "$CP_TMP/hpr/docs/records/interfaces/physics.md"
cp_git -C "$CP_TMP/hpr" add -A && cp_git -C "$CP_TMP/hpr" commit -qm done \
    && cp_git -C "$CP_TMP/hpr" push -q origin main
CP_TIP=$(git -C "$CP_TMP/hpr" rev-parse HEAD)
CP_AFTER=$(wmd owner view prime 2>&1)
if grep -q "$CP_ENGINE.*done upstream, not pinned here (pin ${CP_PIN:0:12}, upstream ${CP_TIP:0:12})" <<<"$CP_AFTER"; then
    cp_ok "done upstream, not pinned here" "the locked commit against what HPR has"
else
    cp_fail "done upstream, not pinned here" "$CP_AFTER"
fi

# ---- notices: an item, and a clause with no CL- prefix -------------------------
CP_ITEM=$(wmd carol note "$CP_ENGINE" working 2>&1)
CP_ITEM2=$(wmd carol note "$CP_ENGINE" again 2>&1)
CP_NEIGHBOUR=$(wmd dave note "$CP_NEXT" working 2>&1)
CP_CLAUSE=$(wmd erin note "$CP_CITES" working 2>&1)
CP_BESIDE=$(wmd frank note "$CP_STEPS" working 2>&1)
if [[ $(grep -c "^notice: hpr:$CP_FEATURE changed upstream" <<<"$CP_ITEM") -eq 1 ]] \
    && ! grep -q '^notice: ' <<<"$CP_ITEM2" && ! grep -q '^notice: ' <<<"$CP_NEIGHBOUR"; then
    cp_ok "a changed item reaches its dependent" "once; who depends on the item beside it hears nothing"
else
    cp_fail "a changed item reaches its dependent" "$CP_ITEM | $CP_ITEM2 | $CP_NEIGHBOUR"
fi
if [[ $(grep -c '^notice: hpr:PHYS-mass changed upstream' <<<"$CP_CLAUSE") -eq 1 ]] \
    && ! grep -q 'no-such-clause' <<<"$CP_CLAUSE" && ! grep -q '^notice: ' <<<"$CP_BESIDE"; then
    cp_ok "a clause needs no CL- prefix" "what the document declares; a word it does not, unwatched"
else
    cp_fail "a clause needs no CL- prefix" "$CP_CLAUSE | $CP_BESIDE"
fi
CP_SEEN=$(wmd carol start "$CP_ENGINE" 2>&1)
if grep -q "hpr:$CP_FEATURE" <<<"$CP_SEEN"; then
    cp_ok "war start shows what changed" "hpr:$CP_FEATURE, and it counts as seen"
else
    cp_fail "war start shows what changed" "$CP_SEEN"
fi

cp_cargo update -q --manifest-path "$CP_TMP/wmd/Cargo.toml" -p hpr \
    || { printf 'PLANT SETUP FAILED: cargo update\n' >&2; exit 9; }
grep -q "#$CP_TIP" "$CP_TMP/wmd/Cargo.lock" || { printf 'PLANT SETUP FAILED: the lock did not move\n' >&2; exit 9; }
CP_UNCOMMITTED=$(wmd owner view prime 2>&1)
cp_git -C "$CP_TMP/wmd" add Cargo.lock && cp_git -C "$CP_TMP/wmd" commit -qm "move the pin"
CP_MOVED=$(wmd owner view prime 2>&1); CP_STATUS=$(wmd a status 2>&1)
CP_MASS=$(wmd a context hpr:PHYS-mass 2>&1)
if grep -q 'done upstream' <<<"$CP_UNCOMMITTED" && ! grep -q "$CP_ENGINE.*held:" <<<"$CP_MOVED" \
    && grep -q "Repos: hpr @ ${CP_TIP:0:12} (Cargo.lock)" <<<"$CP_STATUS" && grep -q 'in grams' <<<"$CP_MASS"; then
    cp_ok "a lock bump releases the item" "only once committed; the read moves with it"
else
    cp_fail "a lock bump releases the item" "$(grep "$CP_ENGINE" <<<"$CP_MOVED") | $(grep -m1 Repos <<<"$CP_STATUS")"
fi

# ---- the list a change request belongs in --------------------------------------
CP_CR=$(hpr --json create "Say what a kilogram is" --change compatible --clause PHYS-mass --test true 2>/dev/null | cp_id)
CP_LIST=$(hpr view tickets --type change-request 2>&1); CP_LIST_RC=$?
CP_TYPO=$(hpr view tickets --type change-requests 2>&1); CP_TYPO_RC=$?
if [[ $CP_LIST_RC -eq 0 && -n "$CP_CR" ]] && grep -q "$CP_CR" <<<"$CP_LIST" && ! grep -q "$CP_OTHER" <<<"$CP_LIST" \
    && grep -q 'ticket.filter-type-unknown' <<<"$CP_TYPO"; then
    cp_ok "--type change-request lists them" "that one alone; a typo is still refused"
else
    cp_fail "--type change-request lists them" "$CP_LIST_RC $CP_TYPO_RC: $CP_LIST | $CP_TYPO"
fi
