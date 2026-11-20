# shellcheck shell=bash
# A claim ref that moves on the remote while another clone reads it (plant 102
# under load 127, 2026-11-14: `git cat-file <sha>: bad file`, read UNKNOWN).
# Reading a remote claim lists the ref, then fetches it by name when this
# clone lacks its commit; a fetch by name brings the ref as it is then. A git
# shim on PATH moves the ref between the two, as a renewal, a reclaim or a
# release by the holder's clone would. Each claim beside its refusal.
#
# Accepted: moved once (renewed), the claim is refused by name, naming the
#   holder (work.claimed-by-other), never UNKNOWN; released in between, the
#   ref is read as absent and the claim lands.
# Refused: a ref that moves on every read is UNKNOWN
#   (work.claim-remote-unreachable, naming that it moved on each read), and
#   nothing is claimed: the remote ref is still the holder's.

echo "== a claim ref that moves while it is read =="
CM_WAR="$REPO_ROOT/${WAR#./}"
cm_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cm_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
CM_T=$(mktemp -d)
SCRATCH_CORPORA+=("$CM_T")
CM_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID -u OPENCODE_SESSION_ID -u OPENCODE
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID -u CLAUDECODE -u OPENWARRANT_ACTOR
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
cm_g() { "${CM_ENV[@]}" git "$@" >/dev/null 2>&1; }
CM_GIT=$(command -v git)
CM_S="$CM_T/seed" CM_1="$CM_T/one" CM_2="$CM_T/two" CM_BARE="$CM_T/bare.git"
mkdir -p "$CM_S"
cm_g -C "$CM_S" init -q -b main
(cd "$CM_S" && "${CM_ENV[@]}" "$CM_WAR" init --namespace CM >/dev/null 2>&1)
printf '\n[claims]\nremote = "origin"\nclaim_lease_minutes = 600\n' >> "$CM_S/openwarrant.toml"
CM_IDS=$(cd "$CM_S" && "${CM_ENV[@]}" "$CM_WAR" --json create "Moving ref" --item one --item two --item three 2>/dev/null \
    | python3 -c 'import json,sys; v=json.load(sys.stdin)["result"]; print(*[v["id"]+"/"+i["id"] for i in v["items"]])')
read -r CM_A CM_B CM_C <<<"$CM_IDS"
cm_g -C "$CM_S" add -A
cm_g -C "$CM_S" commit -qm seed
cm_g clone -q --bare "$CM_S" "$CM_BARE"
cm_g clone -q "file://$CM_BARE" "$CM_1"
cm_g clone -q "file://$CM_BARE" "$CM_2"
if [[ -z "$CM_C" || ! -d "$CM_2/.git" ]]; then
    printf 'PLANT SETUP FAILED: two clones and three items (%s)\n' "$CM_IDS" >&2; exit 9
fi
# holder claims each item in clone one.
for it in "$CM_A" "$CM_B" "$CM_C"; do
    (cd "$CM_1" && "${CM_ENV[@]}" OPENWARRANT_ACTOR=holder "$CM_WAR" claim "$it" >/dev/null 2>&1) \
        || { printf 'PLANT SETUP FAILED: holder claims %s\n' "$it" >&2; exit 9; }
done
cm_ref() { echo "refs/openwarrant/claims/${1%%/*}--${1##*/}"; }
# The shim moves the ref on the bare remote as the holder's clone would:
# `once` rewrites it to a new commit carrying the same claim (a renewal) just
# before the reading clone's first fetch of it; `release` deletes it there;
# `always` rewrites it right after every `ls-remote` that lists it, so every
# commit listed is gone before it can be fetched. Then the real git runs.
cm_shim() { # <dir> <mode> <ref>
    mkdir -p "$1"
    cat > "$1/git" <<SH
#!/bin/bash
cm_move() {
    old=\$("$CM_GIT" --git-dir="$CM_BARE" rev-parse "$3" 2>/dev/null) || return
    n=\$(cat "$1/n" 2>/dev/null || echo 0); n=\$((n + 1)); echo \$n > "$1/n"
    new=\$("$CM_GIT" --git-dir="$CM_BARE" log -1 --format=%B "\$old" | GIT_COMMITTER_DATE="2001-01-01T00:00:\$(printf %02d \$n)Z" GIT_AUTHOR_DATE="2001-01-01T00:00:\$(printf %02d \$n)Z" "$CM_GIT" --git-dir="$CM_BARE" commit-tree "\$old^{tree}")
    "$CM_GIT" --git-dir="$CM_BARE" update-ref "$3" "\$new" "\$old"
}
case "$2" in
always)
    if [[ " \$* " == *" ls-remote "* && " \$* " == *" $3"* ]]; then
        "$CM_GIT" "\$@"; rc=\$?; touch "$1/done"; cm_move; exit \$rc
    fi ;;
*)
    if [[ " \$* " == *" fetch "* && " \$* " == *" $3"* && ! -e "$1/done" ]]; then
        touch "$1/done"
        if [[ "$2" == release ]]; then
            "$CM_GIT" --git-dir="$CM_BARE" update-ref -d "$3"
        else
            cm_move
        fi
    fi ;;
esac
exec "$CM_GIT" "\$@"
SH
    chmod +x "$1/git"
}
cm_claim() { # <shim dir> <item>: claim as heir in clone two through the shim
    (cd "$CM_2" && PATH="$1:$PATH" "${CM_ENV[@]}" OPENWARRANT_ACTOR=heir timeout 120 "$CM_WAR" claim "$2" \
        >"$CM_T/out" 2>"$CM_T/err" </dev/null)
}
cm_holder() { "$CM_GIT" --git-dir="$CM_BARE" log -1 --format=%B "$(cm_ref "$1")" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["actor"])' 2>/dev/null; }

# --- moved once (renewed): refused by name, never UNKNOWN ------------------------------
cm_shim "$CM_T/s1" once "$(cm_ref "$CM_A")"
cm_claim "$CM_T/s1" "$CM_A"; CM_RC=$?
CM_ERR=$(cat "$CM_T/out" "$CM_T/err")
if [[ $CM_RC == 2 && -e "$CM_T/s1/done" ]] && grep -q 'work.claimed-by-other' <<<"$CM_ERR" \
    && grep -q 'holder' <<<"$CM_ERR" && ! grep -q 'bad file\|claim-remote-unreachable' <<<"$CM_ERR"; then
    cm_ok "a renewed ref is read as it is" "moved between ls-remote and fetch: refused by name, holder"
else
    cm_fail "a renewed ref is read as it is" "exit $CM_RC moved $([[ -e "$CM_T/s1/done" ]] && echo yes || echo no): $(head -c 300 <<<"$CM_ERR")"
fi

# --- released in between: absent, and the claim lands ----------------------------------
cm_shim "$CM_T/s2" release "$(cm_ref "$CM_B")"
cm_claim "$CM_T/s2" "$CM_B"; CM_RC=$?
CM_ERR=$(cat "$CM_T/out" "$CM_T/err")
if [[ $CM_RC == 0 && -e "$CM_T/s2/done" && "$(cm_holder "$CM_B")" == heir ]]; then
    cm_ok "a released ref is read as absent" "the claim lands; the remote names heir"
else
    cm_fail "a released ref is read as absent" "exit $CM_RC holder '$(cm_holder "$CM_B")': $(head -c 300 <<<"$CM_ERR")"
fi

# --- the refusal: moved on every read stays UNKNOWN, nothing claimed -------------------
cm_shim "$CM_T/s3" always "$(cm_ref "$CM_C")"
cm_claim "$CM_T/s3" "$CM_C"; CM_RC=$?
CM_ERR=$(cat "$CM_T/out" "$CM_T/err")
if [[ $CM_RC != 0 ]] && grep -q 'work.claim-remote-unreachable' <<<"$CM_ERR" \
    && grep -q 'moved on the remote on each of 3 reads' <<<"$CM_ERR" && [[ "$(cm_holder "$CM_C")" == holder ]]; then
    cm_ok "a ref moving on every read" "UNKNOWN, naming that it moved; the remote still names holder"
else
    cm_fail "a ref moving on every read" "exit $CM_RC holder '$(cm_holder "$CM_C")': $(head -c 300 <<<"$CM_ERR")"
fi
