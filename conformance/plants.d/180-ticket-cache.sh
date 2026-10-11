# shellcheck shell=bash
# The ticket cache (crates/openwarrant-cli/src/ticket/cache.rs): `war next`,
# `war view ready`, `war view tickets`, `war status` and `war claim` answer
# unchanged tickets from a local cache under git's common directory. It may
# only make them faster; every plant here pairs "it was used" with "where it
# could be wrong, it was not trusted".
#
# On a scratch program (TC) with four tickets, settled past the cache's 2 s:
# - warm: OPENWARRANT_CACHE_TRACE says every ticket and the listing came from
#   the cache, and the answer is the same bytes as with OPENWARRANT_NO_CACHE=1
#   (which writes and reads nothing). Nothing of it is in `git status`.
# - an in-place edit of a manifest, same size, same inode, its mtime put back
#   with `touch -d` to the nanosecond: the edit is seen (ctime moved), the old
#   title is gone, and the trace names the one re-read.
# - a new ticket directory made by hand (no `war create`) is listed: the
#   listing is re-read when the directory changes.
# - each cache file corrupted (a byte flipped, truncated, replaced): ignored,
#   the answer still the disabled one, every ticket re-read, and rebuilt.
# - a cache written by another build (the same binary copied elsewhere, so
#   another identity): ignored.
# - `war check`, `war admin host --export` and a hosted `war admin host` run
#   never open it (no trace line), and the hosted request carries none of it.

echo "== ticket cache: used when unchanged, never trusted when it could be wrong =="
TC_TMP=$(mktemp -d)
TC_ROOT=$(scratch_corpus TC)
[[ -d "${TC_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
TC_WAR="$REPO_ROOT/${WAR#./}"
TC_CACHE="$TC_ROOT/.git/openwarrant/cache"
tc_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# The cache's switches come from TC_ENV alone, never from the caller's
# environment.
TC_ENV=()
tc_war() {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_NO_CACHE -u OPENWARRANT_CACHE_TRACE \
        OPENWARRANT_ACTOR=plant-tc ${TC_ENV[@]+"${TC_ENV[@]}"} "$TC_WAR" --root "$TC_ROOT" "$@" </dev/null
}
# tc_traced <out-prefix> <args...>: stdout to <prefix>.out, stderr without the
# trace line to <prefix>.err, the trace line to <prefix>.trace.
tc_traced() {
    local p="$TC_TMP/$1"; shift
    TC_ENV+=(OPENWARRANT_CACHE_TRACE=1)
    tc_war "$@" >"$p.out" 2>"$p.raw"
    echo "exit $?" >>"$p.out"
    TC_ENV=()
    grep -v '^ticket cache:' "$p.raw" >"$p.err"
    grep '^ticket cache:' "$p.raw" >"$p.trace"
}
# tc_off: tc_traced with the cache disabled; its trace must stay empty.
tc_off() {
    TC_ENV=(OPENWARRANT_NO_CACHE=1)
    tc_traced "$@"
}
tc_same() { cmp -s "$TC_TMP/$1.out" "$TC_TMP/$2.out" && cmp -s "$TC_TMP/$1.err" "$TC_TMP/$2.err"; }
tc_trace() { cat "$TC_TMP/$1.trace"; }

for t in Alpha Bravo Charlie Delta; do
    tc_war create "$t ticket" --item "first of $t" --item "second of $t" >/dev/null 2>&1
done
git -C "$TC_ROOT" add -A >/dev/null 2>&1
git -C "$TC_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false commit -qm tickets >/dev/null 2>&1
sleep 3

# --- warm: used, and the same bytes as disabled ---------------------------------------------
tc_off off1 --json next
TC_OFF_FILES=$(find "$TC_CACHE" -type f 2>/dev/null | wc -l)
tc_traced cold --json next
tc_traced warm --json next
tc_off offr view ready
tc_traced warmr view ready
TC_STATUS=$(git -C "$TC_ROOT" status --porcelain --untracked-files=all)
if [[ "$TC_OFF_FILES" == 0 && -z "$(tc_trace off1)$(tc_trace offr)" ]] && tc_same off1 cold && tc_same off1 warm && tc_same offr warmr \
    && [[ "$(tc_trace warm)" == "ticket cache: 4 of 4 ticket(s) from the cache, 0 read from their files; 0 shard(s) written; the listing from the cache" \
    && "$(tc_trace warmr)" == "ticket cache: 4 of 4 ticket(s) from the cache"* \
    && -z "$TC_STATUS" ]]; then
    tc_ok "a warm cache answers" "4 of 4 tickets and the listing from the cache; next --json and view ready byte-identical to OPENWARRANT_NO_CACHE=1, which wrote nothing; git status clean"
else
    tc_fail "a warm cache answers" "disabled wrote $TC_OFF_FILES file(s), traced '$(tc_trace off1)'; warm trace '$(tc_trace warm)'; ready trace '$(tc_trace warmr)'; same: $(tc_same off1 warm && echo y || echo n)/$(tc_same offr warmr && echo y || echo n); status '$TC_STATUS'"
fi

# --- an in-place edit with its mtime put back ---------------------------------------------
TC_MAN=$(grep -l 'Alpha ticket' "$TC_ROOT"/docs/tickets/*/manifest.toml | head -1)
TC_STAT0=$(stat -c '%s %i %y' "$TC_MAN")
TC_MTIME=$(stat -c '%y' "$TC_MAN")
python3 - "$TC_MAN" <<'PY'
import sys
p = sys.argv[1]
b = open(p, 'rb').read().replace(b'Alpha ticket', b'Omega ticket')
with open(p, 'r+b') as f:   # in place: same inode, same size, no truncation
    f.write(b)
PY
touch -d "$TC_MTIME" "$TC_MAN"
TC_STAT1=$(stat -c '%s %i %y' "$TC_MAN")
tc_off offe --json view ready
tc_traced edit --json view ready
if [[ "$TC_STAT0" == "$TC_STAT1" ]] && tc_same offe edit \
    && grep -q 'Omega ticket' "$TC_TMP/edit.out" && ! grep -q 'Alpha ticket' "$TC_TMP/edit.out" \
    && [[ "$(tc_trace edit)" == "ticket cache: 3 of 4 ticket(s) from the cache, 1 read from their files"* ]]; then
    tc_ok "an in-place edit is seen" "same size, inode and mtime ($TC_STAT1) after touch -d; the new title is listed, the old one never; 1 ticket re-read"
else
    tc_fail "an in-place edit is seen" "stat '$TC_STAT0' -> '$TC_STAT1'; same as disabled: $(tc_same offe edit && echo y || echo n); trace '$(tc_trace edit)'"
fi

# --- a ticket directory made by hand -------------------------------------------------------
TC_SRC=$(dirname "$TC_MAN")
TC_NEW="$TC_ROOT/docs/tickets/t-0000aa"
cp -r "$TC_SRC" "$TC_NEW"
python3 - "$TC_NEW/manifest.toml" "$(basename "$TC_SRC")" <<'PY'
import sys
p, old = sys.argv[1], sys.argv[2]
t = open(p).read().replace(f'id = "{old}"', 'id = "t-0000aa"').replace('Omega ticket', 'Hand ticket')
open(p, 'w').write(t)
PY
tc_off offn --json view ready
tc_traced handmade --json view ready
if tc_same offn handmade && grep -q 'Hand ticket' "$TC_TMP/handmade.out" \
    && [[ "$(tc_trace handmade)" == *"the listing from the directory" ]]; then
    tc_ok "a hand-made ticket is listed" "the directory changed, so the listing was re-read: the new ticket is in view ready, byte-identical to disabled"
else
    tc_fail "a hand-made ticket is listed" "same as disabled: $(tc_same offn handmade && echo y || echo n); trace '$(tc_trace handmade)'"
fi
command rm -rf "$TC_NEW"
sleep 3
tc_traced settle --json next

# --- corrupt, truncated, replaced: never trusted, rebuilt ---------------------------------
tc_off offc --json next
TC_BAD=""
for how in flip truncate replace; do
    for f in "$TC_CACHE"/*.bin; do
        python3 - "$f" "$how" <<'PY'
import sys
p, how = sys.argv[1], sys.argv[2]
b = bytearray(open(p, 'rb').read())
if how == 'flip':
    b[len(b) // 2] ^= 1
elif how == 'truncate':
    b = b[:-1]
else:
    b = bytearray(b'not a cache file')
open(p, 'wb').write(bytes(b))
PY
    done
    tc_traced "bad-$how" --json next
    tc_traced "re-$how" --json next
    if ! tc_same offc "bad-$how" \
        || [[ "$(tc_trace "bad-$how")" != "ticket cache: 0 of 4 ticket(s) from the cache, 4 read from their files"*"the listing from the directory" ]] \
        || ! tc_same offc "re-$how" \
        || [[ "$(tc_trace "re-$how")" != "ticket cache: 4 of 4 ticket(s) from the cache"* ]]; then
        TC_BAD="$TC_BAD $how: '$(tc_trace "bad-$how")' then '$(tc_trace "re-$how")';"
    fi
done
if [[ -z "$TC_BAD" ]]; then
    tc_ok "a damaged cache is never trusted" "a flipped byte, a truncation, a replaced file: each ignored (0 of 4 from it, the answer the disabled one), then rebuilt (4 of 4)"
else
    tc_fail "a damaged cache is never trusted" "$TC_BAD"
fi

# --- another build's cache ------------------------------------------------------------------
cp "$TC_WAR" "$TC_TMP/other-war"
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_NO_CACHE OPENWARRANT_CACHE_TRACE=1 OPENWARRANT_ACTOR=plant-tc \
    "$TC_TMP/other-war" --root "$TC_ROOT" --json next </dev/null >"$TC_TMP/other.out" 2>"$TC_TMP/other.raw"
echo "exit $?" >>"$TC_TMP/other.out"
grep -v '^ticket cache:' "$TC_TMP/other.raw" >"$TC_TMP/other.err"
grep '^ticket cache:' "$TC_TMP/other.raw" >"$TC_TMP/other.trace"
if tc_same offc other && [[ "$(tc_trace other)" == "ticket cache: 0 of 4 ticket(s) from the cache"* ]]; then
    tc_ok "another build's cache is ignored" "the same binary at another path is another identity: 0 of 4 from what this one wrote, the same answer"
else
    tc_fail "another build's cache is ignored" "trace '$(tc_trace other)'; same: $(tc_same offc other && echo y || echo n)"
fi

# --- what never opens it: check, host --export, a hosted run --------------------------------
tc_traced check check
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_NO_CACHE OPENWARRANT_CACHE_TRACE=1 "$TC_WAR" --root "$TC_ROOT" host --export \
    </dev/null >"$TC_TMP/request.json" 2>"$TC_TMP/export.raw"
grep '^ticket cache:' "$TC_TMP/export.raw" >"$TC_TMP/export.trace"
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_NO_CACHE OPENWARRANT_CACHE_TRACE=1 "$TC_WAR" host \
    <"$TC_TMP/request.json" >"$TC_TMP/hosted.json" 2>"$TC_TMP/hosted.raw"
grep '^ticket cache:' "$TC_TMP/hosted.raw" >"$TC_TMP/hosted.trace"
# Members of the exported basis, how many are cache files, and whether the
# hosted run built a model from it.
TC_MEMBERS=$(python3 -c '
import json, sys
r = json.load(open(sys.argv[1]))
ms = [m["path"] for m in r["basis"]["members"]]
h = json.load(open(sys.argv[2]))
print(len(ms), sum(1 for m in ms if "openwarrant/cache" in m or m.startswith(".git/")), "model" if h.get("model") else "no-model")' \
    "$TC_TMP/request.json" "$TC_TMP/hosted.json" 2>&1)
if [[ -z "$(tc_trace check)" && -z "$(tc_trace export)" && -z "$(tc_trace hosted)" \
    && "$TC_MEMBERS" =~ ^[1-9][0-9]*\ 0\ model$ ]]; then
    tc_ok "judging and hosted runs never use it" "war check, host --export and a hosted run: no trace line; the exported basis (members, cache files, hosted result: $TC_MEMBERS) holds no cache file"
else
    tc_fail "judging and hosted runs never use it" "check '$(tc_trace check)'; export '$(tc_trace export)'; hosted '$(tc_trace hosted)'; members '$TC_MEMBERS'"
fi

command rm -rf "$TC_TMP" "$TC_ROOT"
